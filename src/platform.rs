//! Windows glue: single instance, showing and hiding the main window, and the autostart registry value.

use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, GetCurrentProcessId, SetEvent, WaitForSingleObject, INFINITE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
    SetForegroundWindow, ShowWindow, ASFW_ANY, SW_HIDE, SW_RESTORE, SW_SHOW,
};

pub const WINDOW_TITLE: &str = "Planora";
const MUTEX_NAME: &str = "Local\\Planora.SingleInstance";
const EVENT_NAME: &str = "Local\\Planora.ShowWindow";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Claims the single-instance mutex. Returns `false` when another Planora already runs;
/// in that case it has been asked to show its window.
pub fn claim_single_instance() -> bool {
    unsafe {
        let m = CreateMutexW(std::ptr::null(), 0, wide(MUTEX_NAME).as_ptr());
        if m.is_null() {
            return true; // cannot tell; do not block startup
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(m);
            // We were just started by the user, so we may hand the foreground right to the other instance.
            AllowSetForegroundWindow(ASFW_ANY);
            let ev = CreateEventW(std::ptr::null(), 0, 0, wide(EVENT_NAME).as_ptr());
            if !ev.is_null() {
                SetEvent(ev);
                CloseHandle(ev);
            }
            return false;
        }
        // The handle stays open for the lifetime of the process on purpose.
        true
    }
}

/// Background thread: whenever a second launch signals the event, shows the main window and calls `on_show`.
pub fn watch_for_second_launch(on_show: impl Fn() + Send + 'static) {
    std::thread::spawn(move || unsafe {
        let ev = CreateEventW(std::ptr::null(), 0, 0, wide(EVENT_NAME).as_ptr());
        if ev.is_null() {
            return;
        }
        loop {
            if WaitForSingleObject(ev, INFINITE) != 0 {
                return;
            }
            show_main_window();
            on_show();
        }
    });
}

static HWND_CACHE: AtomicIsize = AtomicIsize::new(0);

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> i32 {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() {
        let mut buf = [0u16; 64];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if n > 0 && String::from_utf16_lossy(&buf[..n as usize]) == WINDOW_TITLE {
            *(lparam as *mut isize) = hwnd as isize;
            return 0;
        }
    }
    1
}

/// The main window of this process (hidden windows included).
pub fn main_window() -> Option<HWND> {
    let cached = HWND_CACHE.load(Ordering::Relaxed);
    if cached != 0 {
        return Some(cached as HWND);
    }
    let mut found: isize = 0;
    unsafe { EnumWindows(Some(enum_cb), &mut found as *mut isize as LPARAM) };
    if found != 0 {
        HWND_CACHE.store(found, Ordering::Relaxed);
        Some(found as HWND)
    } else {
        None
    }
}

pub fn show_main_window() {
    if let Some(h) = main_window() {
        unsafe {
            if IsIconic(h) != 0 {
                ShowWindow(h, SW_RESTORE);
            } else {
                ShowWindow(h, SW_SHOW);
            }
            SetForegroundWindow(h);
        }
    }
}

pub fn hide_main_window() {
    if let Some(h) = main_window() {
        unsafe { ShowWindow(h, SW_HIDE) };
    }
}

/// Is the main window currently on screen (not hidden in the tray)?
pub fn main_window_visible() -> bool {
    main_window().is_none_or(|h| unsafe { IsWindowVisible(h) != 0 })
}

// ───────────────────────── autostart ─────────────────────────

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE: &str = "Planora";

/// The real registry state, not a remembered setting.
pub fn autostart_enabled() -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_VALUE))
        .is_ok_and(|v| !v.is_empty())
}

/// Adds or removes `"<exe>" --minimized` in HKCU\...\Run. Returns whether the operation succeeded.
pub fn set_autostart(on: bool) -> bool {
    use winreg::{enums::{HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_READ}, RegKey};
    let Ok((key, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey_with_flags(RUN_KEY, KEY_SET_VALUE | KEY_READ) else {
        return false;
    };
    if on {
        let Ok(exe) = std::env::current_exe() else { return false };
        key.set_value(RUN_VALUE, &format!("\"{}\" --minimized", exe.display())).is_ok()
    } else {
        match key.delete_value(RUN_VALUE) {
            Ok(()) => true,
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        }
    }
}
