//! Notification-area icon (StatusNotifier / AppIndicator) with the Open / New task / Today / Exit menu.
//!
//! `tray-icon` needs a GTK main loop, which eframe does not run, so the icon lives on its own GTK thread and the UI
//! talks to it through a channel. The AppIndicator protocol has no click events: the icon only opens its menu.
//! GNOME needs the "AppIndicator and KStatusNotifierItem Support" extension; without a tray host `Tray::new`
//! returns `None` and the window then quits on close instead of hiding.

use crate::bus::{self, Cmd};
use crate::i18n::Lang;
use crate::platform;
use crate::strings;
use crate::support;
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

enum Msg {
    Lang(Lang),
    Remove,
}

pub struct Tray {
    tx: Sender<Msg>,
}

/// Is a StatusNotifier host running? Without one the icon would be invisible.
fn tray_host_present() -> bool {
    let out = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.NameHasOwner",
            "org.kde.StatusNotifierWatcher",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).contains("true"),
        // gdbus is missing or the bus is unreachable: assume the best.
        _ => true,
    }
}

impl Tray {
    pub fn new(lang: Lang, icon_png: &[u8]) -> Option<Tray> {
        let (tx, rx) = channel::<Msg>();
        let (ready_tx, ready_rx) = channel::<bool>();
        let png = icon_png.to_vec();
        std::thread::Builder::new().name("planora-tray".into()).spawn(move || run(lang, png, rx, ready_tx)).ok()?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(true) => Some(Tray { tx }),
            _ => None,
        }
    }

    pub fn set_lang(&self, lang: Lang) {
        let _ = self.tx.send(Msg::Lang(lang));
    }

    pub fn remove(&self) {
        let _ = self.tx.send(Msg::Remove);
    }
}

fn run(lang: Lang, icon_png: Vec<u8>, rx: std::sync::mpsc::Receiver<Msg>, ready: Sender<bool>) {
    let built = (|| {
        gtk::init().ok()?;
        let data = eframe::icon_data::from_png_bytes(&icon_png).ok()?;
        let icon = Icon::from_rgba(data.rgba, data.width, data.height).ok()?;
        let t = strings::TABLE.get(lang);
        let open = MenuItem::new(t.tray_open, true, None);
        let new_task = MenuItem::new(t.tray_new, true, None);
        let today = MenuItem::new(t.tray_today, true, None);
        let support_item = MenuItem::new(support::tray_label(lang), true, None);
        let exit = MenuItem::new(t.tray_exit, true, None);
        let menu = Menu::new();
        menu.append_items(&[&open, &new_task, &today, &PredefinedMenuItem::separator(), &support_item, &exit]).ok()?;
        let tray = TrayIconBuilder::new().with_icon(icon).with_tooltip("Planora").with_menu(Box::new(menu)).build().ok()?;
        Some((tray, open, new_task, today, support_item, exit))
    })();
    let Some((tray, open, new_task, today, support_item, exit)) = built else {
        let _ = ready.send(false);
        return;
    };
    if !tray_host_present() {
        let _ = ready.send(false);
        return;
    }

    let (open_id, new_id, today_id, support_id, exit_id) = (open.id().clone(), new_task.id().clone(), today.id().clone(), support_item.id().clone(), exit.id().clone());
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        if e.id == open_id {
            platform::show_main_window();
            bus::push(Cmd::Show);
        } else if e.id == new_id {
            platform::show_main_window();
            bus::push(Cmd::NewTask);
        } else if e.id == today_id {
            platform::show_main_window();
            bus::push(Cmd::GoToday);
        } else if e.id == support_id {
            support::open();
        } else if e.id == exit_id {
            bus::push(Cmd::Quit);
            // If the hidden window does not wake up to handle the command, leave anyway.
            std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(1500));
                if bus::pending() {
                    std::process::exit(0);
                }
            });
        }
    }));
    let _ = ready.send(true);

    gtk::glib::timeout_add_local(Duration::from_millis(200), move || {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Msg::Lang(lang) => {
                    let t = strings::TABLE.get(lang);
                    open.set_text(t.tray_open);
                    new_task.set_text(t.tray_new);
                    today.set_text(t.tray_today);
                    support_item.set_text(support::tray_label(lang));
                    exit.set_text(t.tray_exit);
                }
                Msg::Remove => {
                    let _ = tray.set_visible(false);
                    gtk::main_quit();
                    return gtk::glib::ControlFlow::Break;
                }
            }
        }
        gtk::glib::ControlFlow::Continue
    });
    gtk::main();
}
