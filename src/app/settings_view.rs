//! The Settings page.

use super::dialogs::{color_swatches, open_folder, Dialog};
use super::widgets;
use super::App;
use crate::config::Nav;
use crate::fmt;
use crate::i18n;
use crate::model::storage;
use crate::model::Profile;
use crate::nimvoro::{self, UpdateState};
use crate::platform;
use crate::theme::{self, Btn, Pal};
use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui};

fn group<R>(ui: &mut Ui, p: &Pal, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(14.0);
    theme::caps(ui, title, p);
    ui.add_space(4.0);
    Frame::new().fill(p.card).stroke(Stroke::new(1.0, p.line)).corner_radius(14).inner_margin(Margin::symmetric(18, 14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    })
    .inner
}

fn switch_row(ui: &mut Ui, p: &Pal, title: &str, hint: &str, value: &mut bool) -> bool {
    let mut changed = false;
    let w = ui.available_width();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(w - 58.0);
            ui.label(RichText::new(title).font(theme::reg(14.0)).color(p.text));
            if !hint.is_empty() {
                ui.add_space(-6.0);
                theme::hint(ui, hint, p);
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            changed = theme::switch(ui, value, p).changed();
        });
    });
    ui.add_space(6.0);
    changed
}

fn combo_row<T: PartialEq + Copy>(ui: &mut Ui, p: &Pal, title: &str, id: &str, current: T, options: &[(T, &str)], width: f32) -> Option<T> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).font(theme::reg(14.0)).color(p.text));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = options.iter().find(|o| o.0 == current).map(|o| o.1).unwrap_or("");
            egui::ComboBox::from_id_salt(id).width(width).selected_text(RichText::new(label).font(theme::reg(13.5))).show_ui(ui, |ui| {
                for (v, name) in options {
                    if ui.selectable_label(*v == current, RichText::new(*name).font(theme::reg(13.5))).clicked() {
                        picked = Some(*v);
                    }
                }
            });
        });
    });
    ui.add_space(6.0);
    picked
}

impl App {
    pub(super) fn settings_view(&mut self, ui: &mut Ui, p: &Pal) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
          // The column keeps its maximum width and sits in the middle of the main area.
          let avail = ui.available_width();
          let col_w = (avail - 12.0).clamp(300.0, 680.0);
          ui.horizontal_top(|ui| {
            ui.add_space(((avail - col_w) / 2.0).max(0.0));
            ui.vertical(|ui| {
            ui.set_width(col_w);
            ui.label(RichText::new(t.settings).font(theme::bold(24.0)).color(p.text));
            ui.add_space(2.0);

            // ── Interface
            group(ui, p, t.group_interface, |ui| {
                let mut lang = self.lang;
                if i18n::language_row(ui, &mut lang, p) {
                    self.lang = lang;
                    self.cfg.language = Some(lang.code().to_string());
                    if let Some(tr) = &self.tray {
                        tr.set_lang(lang);
                    }
                    self.sync_worker();
                    self.touch_cfg(&ctx);
                }
                ui.add_space(6.0);
                let mut mode = self.cfg.theme;
                if i18n::theme_row(ui, &mut mode, self.lang, p) {
                    self.cfg.theme = mode;
                    theme::apply(&ctx, mode);
                    self.touch_cfg(&ctx);
                }
                ui.add_space(6.0);
                if self.lang == i18n::Lang::En {
                    let cur = self.cfg.week_starts_sunday;
                    if let Some(v) = combo_row(ui, p, t.week_start, "weekstart", cur, &[(false, t.monday), (true, t.sunday)], 150.0) {
                        self.cfg.week_starts_sunday = v;
                        self.touch_cfg(&ctx);
                    }
                    let cur = self.cfg.time_24h;
                    if let Some(v) = combo_row(ui, p, t.time_format, "timefmt", cur, &[(true, "24-hour"), (false, "12-hour")], 150.0) {
                        self.cfg.time_24h = v;
                        self.sync_worker();
                        self.touch_cfg(&ctx);
                    }
                } else {
                    theme::hint(ui, t.week_fixed_hint, p);
                }
            });

            // ── Behaviour
            group(ui, p, t.group_behaviour, |ui| {
                let mut auto = self.autostart;
                if switch_row(ui, p, t.autostart, t.autostart_hint, &mut auto) {
                    let ok = platform::set_autostart(auto);
                    self.autostart = platform::autostart_enabled();
                    self.settings_msg = if ok && self.autostart == auto { None } else { Some(t.autostart_failed.to_string()) };
                }
                let mut v = self.cfg.close_to_tray;
                if switch_row(ui, p, t.close_to_tray, t.close_to_tray_hint, &mut v) {
                    self.cfg.close_to_tray = v;
                    self.touch_cfg(&ctx);
                }
                let mut v = self.cfg.notifications;
                if switch_row(ui, p, t.notifications, t.notifications_hint, &mut v) {
                    self.cfg.notifications = v;
                    self.sync_worker();
                    self.touch_cfg(&ctx);
                }
                let cur = self.cfg.default_remind;
                let opts: Vec<(u32, String)> = [0u32, 5, 10, 15, 30, 60]
                    .iter()
                    .map(|m| (*m, if *m == 0 { t.remind_at.to_string() } else { t.remind_min.replace("{n}", &m.to_string()) }))
                    .collect();
                let refs: Vec<(u32, &str)> = opts.iter().map(|(m, s)| (*m, s.as_str())).collect();
                if let Some(v) = combo_row(ui, p, t.default_reminder, "defremind", cur, &refs, 150.0) {
                    self.cfg.default_remind = v;
                    self.sync_worker();
                    self.touch_cfg(&ctx);
                }
                if let Some(m) = &self.settings_msg {
                    ui.label(RichText::new(m).font(theme::reg(12.5)).color(widgets::RED));
                }
            });

            // ── Profiles
            group(ui, p, t.group_profiles, |ui| {
                for profile in Profile::ALL {
                    ui.label(RichText::new(self.profile_name(profile)).font(theme::reg(14.0)).color(p.text));
                    ui.add_space(2.0);
                    let mut c = self.cfg.profile_color(profile);
                    let before = c;
                    color_swatches(ui, &mut c, p);
                    if c != before {
                        match profile {
                            Profile::Personal => self.cfg.personal_color = c,
                            Profile::Work => self.cfg.work_color = c,
                        }
                        self.touch_cfg(&ctx);
                    }
                    ui.add_space(8.0);
                }
            });

            // ── Data
            group(ui, p, t.group_data, |ui| {
                ui.label(RichText::new(self.dir.display().to_string()).font(theme::reg(12.5)).color(p.muted));
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if theme::button(ui, t.open_data_folder, Btn::Soft, p).clicked() {
                        open_folder(&self.dir);
                    }
                    if theme::button(ui, t.export_json, Btn::Soft, p).clicked() {
                        self.export_json(&ctx);
                    }
                    if theme::button(ui, t.import_json, Btn::Soft, p).clicked() {
                        self.import_json();
                    }
                });
                ui.add_space(10.0);
                theme::caps(ui, t.backups, p);
                ui.add_space(2.0);
                theme::hint(ui, t.backups_hint, p);
                ui.add_space(4.0);
                if self.backups.is_empty() {
                    theme::hint(ui, t.no_backups, p);
                }
                let mut restore = None;
                for (day, path) in self.backups.iter().take(14) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(fmt::date_long(self.lang, *day)).font(theme::reg(13.5)).color(p.text));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::button(ui, t.restore, Btn::Small, p).clicked() {
                                restore = Some(path.clone());
                            }
                        });
                    });
                }
                if let Some(path) = restore {
                    self.dialog = Some(Dialog::ConfirmRestore(path));
                }
                if let Some(m) = &self.settings_msg {
                    let _ = m;
                }
            });

            // ── Updates
            let u = nimvoro::strings(self.lang);
            group(ui, p, u.updates, |ui| {
                let mut v = self.cfg.check_updates;
                if switch_row(ui, p, u.check_on_start, u.check_on_start_hint, &mut v) {
                    self.cfg.check_updates = v;
                    self.touch_cfg(&ctx);
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.updater.status_text(self.lang)).font(theme::reg(13.5)).color(p.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let UpdateState::Available { .. } = self.updater.state() {
                            if theme::button(ui, u.update_btn, Btn::Soft, p).clicked() && self.updater.apply() {
                                self.quit(&ctx);
                            }
                        }
                        if theme::button(ui, u.check_now, Btn::Ghost, p).clicked() {
                            let c = ctx.clone();
                            self.updater.check(move || c.request_repaint());
                        }
                    });
                });
            });

            // ── About
            group(ui, p, t.about, |ui| {
                ui.horizontal(|ui| {
                    widgets::logo_image(ui, &self.logo, 40.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Planora").font(theme::semi(15.0)).color(p.text));
                        ui.add_space(-6.0);
                        theme::hint(ui, &format!("{} {}", t.version, env!("CARGO_PKG_VERSION")), p);
                    });
                });
                ui.add_space(6.0);
                if theme::button(ui, crate::REPO_URL, Btn::Link, p).clicked() {
                    ctx.open_url(egui::OpenUrl::new_tab(crate::REPO_URL));
                }
            });
            ui.add_space(16.0);
            });
          });
        });
        if self.cfg.nav != Nav::Settings {
            self.settings_msg = None;
        }
    }

    fn export_json(&mut self, ctx: &egui::Context) {
        let t = self.s();
        let name = format!("planora-{}.json", self.today.format("%Y-%m-%d"));
        if let Some(path) = rfd::FileDialog::new().set_file_name(name).add_filter("JSON", &["json"]).save_file() {
            let ok = serde_json::to_vec_pretty(&self.data).ok().is_some_and(|json| storage::write_atomic(&path, &json).is_ok());
            self.notify(ctx, if ok { t.exported.to_string() } else { t.export_failed.to_string() });
        }
    }

    fn import_json(&mut self) {
        let t = self.s();
        if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
            match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| storage::parse(&s)) {
                Ok(data) => {
                    self.settings_msg = None;
                    self.dialog = Some(Dialog::ConfirmImport(Box::new(data)));
                }
                Err(_) => self.settings_msg = Some(t.import_failed.to_string()),
            }
        }
    }
}
