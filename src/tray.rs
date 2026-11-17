//! Notification-area icon with the Open / New task / Today / Exit menu.

use crate::bus::{self, Cmd};
use crate::i18n::Lang;
use crate::platform;
use crate::strings;
use crate::support;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct Tray {
    icon: TrayIcon,
    open: MenuItem,
    new_task: MenuItem,
    today: MenuItem,
    support: MenuItem,
    exit: MenuItem,
}

impl Tray {
    pub fn new(lang: Lang, icon_png: &[u8]) -> Option<Tray> {
        let data = eframe::icon_data::from_png_bytes(icon_png).ok()?;
        let icon = Icon::from_rgba(data.rgba, data.width, data.height).ok()?;
        let t = strings::TABLE.get(lang);
        let open = MenuItem::new(t.tray_open, true, None);
        let new_task = MenuItem::new(t.tray_new, true, None);
        let today = MenuItem::new(t.tray_today, true, None);
        let support = MenuItem::new(support::tray_label(lang), true, None);
        let exit = MenuItem::new(t.tray_exit, true, None);
        let menu = Menu::new();
        menu.append_items(&[&open, &new_task, &today, &PredefinedMenuItem::separator(), &support, &exit]).ok()?;
        let icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("Planora")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .ok()?;

        let (open_id, new_id, today_id, exit_id) = (open.id().clone(), new_task.id().clone(), today.id().clone(), exit.id().clone());
        let support_id = support.id().clone();
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
                    std::thread::sleep(std::time::Duration::from_millis(1500));
                    if bus::pending() {
                        std::process::exit(0);
                    }
                });
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                platform::show_main_window();
                bus::push(Cmd::Show);
            }
        }));
        Some(Tray { icon, open, new_task, today, support, exit })
    }

    pub fn set_lang(&self, lang: Lang) {
        let t = strings::TABLE.get(lang);
        self.open.set_text(t.tray_open);
        self.new_task.set_text(t.tray_new);
        self.today.set_text(t.tray_today);
        self.support.set_text(support::tray_label(lang));
        self.exit.set_text(t.tray_exit);
    }

    pub fn remove(&self) {
        let _ = self.icon.set_visible(false);
    }
}
