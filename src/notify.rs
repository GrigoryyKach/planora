//! Reminders: Windows toast notifications and the background thread that waits for the next due reminder.
//!
//! Toasts use WinRT through `tauri-winrt-notification`. The app registers its own AppUserModelID under
//! HKCU\Software\Classes\AppUserModelId so Windows shows "Planora" and its icon (no tray balloon is used).
//! The thread sleeps until the next reminder (at most 30 s at a time) and uses no CPU otherwise.

use crate::bus::{self, Cmd};
use crate::config::State;
use crate::fmt;
use crate::i18n::Lang;
use crate::model::reminders::{next_reminder, reminders_between, Reminder};
use crate::model::Task;
use crate::strings;
use chrono::{Duration, Local, NaiveDateTime};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

const AUMID: &str = "Planora.App";

static IDENTITY_OK: AtomicBool = AtomicBool::new(false);

/// Windows only shows toasts for an AppUserModelID that belongs to a Start-menu shortcut.
/// This tags (or creates) `Start Menu\Programs\Planora.lnk` with the ID. The NIMVORO launcher creates
/// the same shortcut at install time; we only add the ID. With `PLANORA_DATA_DIR` set (tests, screenshots)
/// a separate `Planora (test).lnk` is used so a real shortcut is never touched.
/// Returns `false` when it failed; toasts then go out under the PowerShell identity instead.
pub fn register_identity() -> bool {
    let ok = tag_shortcut().is_ok();
    IDENTITY_OK.store(ok, Ordering::Relaxed);
    ok
}

/// Path of the Start-menu shortcut used for the toast identity.
pub fn shortcut_path() -> Option<PathBuf> {
    let name = if std::env::var_os("PLANORA_DATA_DIR").is_some() { "Planora (test).lnk" } else { "Planora.lnk" };
    Some(dirs::config_dir()?.join("Microsoft").join("Windows").join("Start Menu").join("Programs").join(name))
}

fn tag_shortcut() -> windows::core::Result<()> {
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READWRITE};
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    let path = shortcut_path().ok_or_else(windows::core::Error::empty)?;
    let exe = std::env::current_exe().map_err(|_| windows::core::Error::empty())?;
    let wide = |p: &std::path::Path| -> Vec<u16> { p.as_os_str().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect() };
    unsafe {
        // The UI toolkit may have initialised COM already; either result is fine.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let file: IPersistFile = link.cast()?;
        let wpath = wide(&path);
        let existed = path.exists();
        if existed {
            file.Load(PCWSTR(wpath.as_ptr()), STGM_READWRITE)?;
        } else {
            let wexe = wide(&exe);
            link.SetPath(PCWSTR(wexe.as_ptr()))?;
            link.SetIconLocation(PCWSTR(wexe.as_ptr()), 0)?;
            let desc = wide(std::path::Path::new("Planora"));
            link.SetDescription(PCWSTR(desc.as_ptr()))?;
        }
        let store: IPropertyStore = link.cast()?;
        if existed {
            if let Ok(v) = store.GetValue(&PKEY_AppUserModel_ID) {
                if v.to_string() == AUMID {
                    return Ok(());
                }
            }
        }
        store.SetValue(&PKEY_AppUserModel_ID, &PROPVARIANT::from(AUMID))?;
        store.Commit()?;
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        file.Save(PCWSTR(wpath.as_ptr()), true)?;
    }
    Ok(())
}

/// Shows one toast. Clicking it opens the task.
pub fn toast(title: &str, body: &str, icon_png: &std::path::Path, task_id: Option<String>) -> Result<(), String> {
    use tauri_winrt_notification::{IconCrop, Toast};
    let id = if IDENTITY_OK.load(Ordering::Relaxed) { AUMID } else { Toast::POWERSHELL_APP_ID };
    let mut t = Toast::new(id).title(title).text1(body).icon(icon_png, IconCrop::Square, "Planora");
    t = t.on_activated(move |_| {
        crate::platform::show_main_window();
        match &task_id {
            Some(id) => bus::push(Cmd::OpenTask(id.clone())),
            None => bus::push(Cmd::Show),
        }
        Ok(())
    });
    t.show().map_err(|e| e.to_string())
}

/// Text under the title: "In 10 min · 14:30" or "Now · 14:30" (with the date when it is not today).
pub fn reminder_body(lang: Lang, r: &Reminder, today: chrono::NaiveDate, h24: bool) -> String {
    let t = strings::TABLE.get(lang);
    let time = fmt::time_text(r.event.time(), h24);
    let when = if r.event.date() == today { time } else { format!("{} {}", fmt::date_short(lang, r.event.date()), time) };
    let lead = if r.before == 0 { t.toast_now.to_string() } else { t.toast_in_min.replace("{n}", &r.before.to_string()) };
    format!("{lead} · {when}")
}

struct Shared {
    tasks: Vec<Task>,
    default_before: u32,
    enabled: bool,
    lang: Lang,
    h24: bool,
    last_tick: NaiveDateTime,
    quit: bool,
}

/// Handle to the reminder thread.
#[derive(Clone)]
pub struct Worker {
    shared: Arc<(Mutex<Shared>, Condvar)>,
}

impl Worker {
    pub fn start(dir: PathBuf, icon_png: PathBuf, lang: Lang) -> Worker {
        let now = Local::now().naive_local();
        let shared = Arc::new((
            Mutex::new(Shared { tasks: Vec::new(), default_before: 10, enabled: true, lang, h24: true, last_tick: now, quit: false }),
            Condvar::new(),
        ));
        let me = shared.clone();
        std::thread::Builder::new()
            .name("planora-reminders".into())
            .spawn(move || run(me, dir, icon_png))
            .ok();
        Worker { shared }
    }

    /// Pushes the current tasks and settings to the thread (call after every change).
    pub fn update(&self, tasks: &[Task], default_before: u32, enabled: bool, lang: Lang, h24: bool) {
        let (lock, cv) = &*self.shared;
        if let Ok(mut s) = lock.lock() {
            s.tasks = tasks.iter().filter(|t| t.time.is_some() && t.scheduled.is_some()).cloned().collect();
            s.default_before = default_before;
            s.enabled = enabled;
            s.lang = lang;
            s.h24 = h24;
        }
        cv.notify_all();
    }

    pub fn stop(&self) {
        let (lock, cv) = &*self.shared;
        if let Ok(mut s) = lock.lock() {
            s.quit = true;
        }
        cv.notify_all();
    }
}

fn run(shared: Arc<(Mutex<Shared>, Condvar)>, dir: PathBuf, icon_png: PathBuf) {
    let (lock, cv) = &*shared;
    let mut last_saved = Local::now().naive_local();
    State { last_tick: Some(last_saved) }.save_to(&dir);
    let Ok(mut guard) = lock.lock() else { return };
    loop {
        if guard.quit {
            State { last_tick: Some(Local::now().naive_local()) }.save_to(&dir);
            return;
        }
        let now = Local::now().naive_local();
        let due = if guard.enabled { reminders_between(&guard.tasks, guard.default_before, guard.last_tick, now) } else { Vec::new() };
        guard.last_tick = now;
        let (lang, h24) = (guard.lang, guard.h24);
        if !due.is_empty() {
            // Fire without holding the lock so the UI is never blocked by WinRT.
            drop(guard);
            for r in &due {
                if let Err(e) = toast(&r.title, &reminder_body(lang, r, now.date(), h24), &icon_png, Some(r.task_id.clone())) {
                    // A failed toast must not be silent: leave a note next to the data.
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("notify.log")) {
                        let _ = writeln!(f, "{} toast failed: {e}", now.format("%Y-%m-%d %H:%M:%S"));
                    }
                }
            }
            State { last_tick: Some(now) }.save_to(&dir);
            last_saved = now;
            guard = match lock.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            continue_after_fire(&mut guard, now);
        } else if now - last_saved > Duration::minutes(2) {
            State { last_tick: Some(now) }.save_to(&dir);
            last_saved = now;
        }
        let wait = match next_reminder(&guard.tasks, guard.default_before, guard.last_tick).filter(|_| guard.enabled) {
            Some(r) => (r.at - now).to_std().unwrap_or_default().clamp(std::time::Duration::from_millis(100), std::time::Duration::from_secs(30)),
            None => std::time::Duration::from_secs(30),
        };
        guard = match cv.wait_timeout(guard, wait) {
            Ok((g, _)) => g,
            Err(_) => return,
        };
    }
}

/// After firing, `last_tick` must not move backwards if the UI pushed an update meanwhile.
fn continue_after_fire(g: &mut Shared, now: NaiveDateTime) {
    if g.last_tick < now {
        g.last_tick = now;
    }
}
