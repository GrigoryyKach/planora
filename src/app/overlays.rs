//! Overlays: replace-data confirmation, missed reminders, corrupted data notice, day panel,
//! date picker popup and the toast.

use super::dialogs::{footer, modal, open_folder, Dialog};
use super::widgets::{self, Icon};
use super::{App, DateTarget};
use crate::config::Nav;
use crate::fmt;
use crate::model::calendar::{self, items_in_range};
use crate::model::reminders::Reminder;
use crate::model::storage;
use crate::model::Data;
use crate::theme::{self, Btn, Pal};
use chrono::{Datelike, NaiveDate};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, Frame, Id, Layout, Margin, Order, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2,
    vec2,
};
use std::path::PathBuf;

impl App {
    /// Confirms replacing everything with `data` (import or restore).
    pub(super) fn confirm_replace(&mut self, ctx: &egui::Context, p: &Pal, data: Data, from: Option<&str>) -> Option<Dialog> {
        let t = self.s();
        let (mut close, mut confirm) = (false, false);
        let backdrop = modal(ctx, p, "replace", 460.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.replace_q).font(theme::bold(17.0)).color(p.text));
            ui.add_space(6.0);
            if let Some(f) = from {
                theme::hint(ui, f, p);
            }
            theme::hint(
                ui,
                &format!(
                    "{} · {}",
                    crate::i18n::plural(self.lang, data.tasks.len(), &t.tasks_plural),
                    crate::i18n::plural(self.lang, data.projects.len(), &t.projects_plural)
                ),
                p,
            );
            ui.add_space(4.0);
            theme::hint(ui, t.replace_hint, p);
            ui.add_space(14.0);
            let f = footer(ui, self, p, None, &[t.cancel], t.replace, true);
            confirm |= f.primary;
            close |= f.secondary.is_some();
        });
        if confirm {
            self.replace_data(ctx, data);
            return None;
        }
        if close || backdrop {
            return None;
        }
        Some(Dialog::ConfirmImport(Box::new(data)))
    }

    pub(super) fn confirm_restore(&mut self, ctx: &egui::Context, p: &Pal, path: PathBuf) -> Option<Dialog> {
        let t = self.s();
        let parsed = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| storage::parse(&s));
        match parsed {
            Ok(data) => {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                match self.confirm_replace(ctx, p, data, Some(&name)) {
                    // Still waiting: keep the restore dialog (it re-reads the file each frame).
                    Some(Dialog::ConfirmImport(_)) => Some(Dialog::ConfirmRestore(path)),
                    other => other,
                }
            }
            Err(_) => {
                self.notify(ctx, t.restore_failed.to_string());
                None
            }
        }
    }

    /// Replaces all data. The current data is first copied to `backups/before-<time>.json`.
    pub fn replace_data(&mut self, ctx: &egui::Context, data: Data) {
        let stamp = self.now.format("%Y%m%d-%H%M%S").to_string();
        if let Ok(json) = serde_json::to_vec_pretty(&self.data) {
            let _ = storage::write_atomic(&self.dir.join(storage::BACKUP_DIR).join(format!("before-{stamp}.json")), &json);
        }
        self.data = data;
        self.editor = None;
        self.selected = None;
        if let Nav::Project(id) = &self.cfg.nav {
            if self.data.project(id).is_none() {
                self.cfg.nav = Nav::Today;
            }
        }
        self.touch_data(ctx);
        self.flush();
        let msg = self.s().data_replaced.to_string();
        self.notify(ctx, msg);
    }

    pub(super) fn missed_dialog(&mut self, ctx: &egui::Context, p: &Pal, list: Vec<Reminder>) -> Option<Dialog> {
        let t = self.s();
        let mut close = false;
        let mut open: Option<String> = None;
        let backdrop = modal(ctx, p, "missed", 480.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.missed_title).font(theme::bold(18.0)).color(p.text));
            ui.add_space(2.0);
            theme::hint(ui, t.missed_hint, p);
            ui.add_space(8.0);
            ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
                for r in &list {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, 9.0, p.hover);
                    }
                    let when = format!("{} {}", fmt::date_short(self.lang, r.event.date()), fmt::time_text(r.event.time(), self.h24()));
                    let g = ui.painter().layout_no_wrap(when, theme::semi(13.0), p.accent);
                    ui.painter().galley(pos2(rect.left() + 10.0, rect.center().y - g.size().y / 2.0), g, p.accent);
                    let g = super::calendar_view::fit_text(ui.painter(), &r.title, theme::reg(14.0), p.text, rect.width() - 150.0, None);
                    ui.painter().galley(pos2(rect.left() + 130.0, rect.center().y - g.size().y / 2.0), g, p.text);
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        open = Some(r.task_id.clone());
                    }
                }
            });
            ui.add_space(12.0);
            close |= footer(ui, self, p, None, &[], t.dismiss, true).primary;
        });
        if let Some(id) = open {
            self.open_editor(&id);
            return None;
        }
        if close || backdrop {
            return None;
        }
        Some(Dialog::Missed(list))
    }

    pub(super) fn corrupt_dialog(&mut self, ctx: &egui::Context, p: &Pal, path: PathBuf) -> Option<Dialog> {
        let t = self.s();
        let mut close = false;
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let backdrop = modal(ctx, p, "corrupt", 500.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.corrupt_title).font(theme::bold(18.0)).color(p.text));
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new(t.corrupt_text.replace("{file}", &name)).font(theme::reg(14.0)).color(p.text)).wrap());
            ui.add_space(14.0);
            let f = footer(ui, self, p, None, &[t.open_data_folder], t.ok, true);
            close |= f.primary;
            if f.secondary.is_some() {
                open_folder(&self.dir);
            }
        });
        if close || backdrop {
            return None;
        }
        Some(Dialog::Corrupt(path))
    }

    // ───────────────────────── day panel ─────────────────────────

    pub(super) fn day_panel_ui(&mut self, ctx: &egui::Context, p: &Pal) {
        let Some(day) = self.day_panel else { return };
        let t = self.s();
        let items = items_in_range(&self.data.tasks, day, day, self.cfg.filter, self.cfg.hide_done, self.now).remove(&day).unwrap_or_default();
        let mut close = false;
        let mut add = false;
        let mut export = false;
        let mut action = None;
        let backdrop = modal(ctx, p, "day-panel", 520.0, |ui: &mut Ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(fmt::date_long(self.lang, day)).font(theme::bold(18.0)).color(p.text));
                    if day == self.today {
                        ui.add_space(-4.0);
                        theme::hint(ui, t.today, p);
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::icon_button(ui, Icon::Close, 30.0, p).clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(8.0);
            ScrollArea::vertical().max_height(340.0).auto_shrink([false, true]).show(ui, |ui| {
                if items.is_empty() {
                    theme::hint(ui, t.nothing_on_day, p);
                } else {
                    action = self.item_list(ui, p, &items, false);
                }
            });
            ui.add_space(10.0);
            let f = footer(ui, self, p, None, &[t.export_pdf], t.add_on_day, true);
            add |= f.primary;
            export |= f.secondary.is_some();
        });
        if export {
            self.export_pdf(ctx, super::export::ExportTarget::Day(day));
        }
        if add {
            self.day_panel = None;
            self.open_quick_add(Some(day));
            return;
        }
        if action.is_some() {
            let is_open = matches!(action, Some(super::today_view::ItemAction::Open(_)));
            self.apply_item_action(ctx, action);
            if is_open {
                self.day_panel = None;
            }
        }
        if close || backdrop {
            self.day_panel = None;
        }
    }

    // ───────────────────────── date picker ─────────────────────────

    pub(super) fn apply_date(&mut self, ctx: &egui::Context, target: &DateTarget, d: Option<NaiveDate>) {
        match target {
            DateTarget::Deadline(id) => self.edit_task(ctx, id, |t| t.deadline = d),
            DateTarget::Scheduled(id) => self.edit_task(ctx, id, |t| t.scheduled = d),
            DateTarget::Until(id) => self.edit_task(ctx, id, |t| {
                if let Some(r) = &mut t.recurrence {
                    r.until = d;
                }
            }),
            DateTarget::QuickAdd => {
                if let Some(Dialog::QuickAdd { date, .. }) = &mut self.dialog {
                    *date = d;
                }
            }
        }
    }

    pub(super) fn picker_ui(&mut self, ctx: &egui::Context, p: &Pal) {
        let Some(mut pk) = self.picker.take() else { return };
        let t = self.s();
        let grid = calendar::month_grid(pk.month.0, pk.month.1, self.week_start());
        let mut chosen: Option<Option<NaiveDate>> = None;
        let screen = ctx.content_rect();
        let size = vec2(266.0, 296.0);
        let pos = pos2(pk.anchor.x.min(screen.right() - size.x - 8.0).max(8.0), pk.anchor.y.min(screen.bottom() - size.y - 8.0).max(8.0));
        let area = egui::Area::new(Id::unique("date-picker")).order(Order::Tooltip).fixed_pos(pos).show(ctx, |ui| {
            Frame::new()
                .fill(p.card)
                .stroke(Stroke::new(1.0, p.line))
                .corner_radius(14)
                .inner_margin(Margin::same(12))
                .shadow(theme::shadow(10, 30, Color32::from_black_alpha(80)))
                .show(ui, |ui| {
                    ui.set_width(size.x - 24.0);
                    ui.horizontal(|ui| {
                        if widgets::icon_button(ui, Icon::Left, 28.0, p).clicked() {
                            pk.month = calendar::shift_month(pk.month.0, pk.month.1, -1);
                        }
                        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width() - 36.0, 28.0), Sense::hover());
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, fmt::month_title(self.lang, pk.month.0, pk.month.1), theme::semi(14.0), p.text);
                        if widgets::icon_button(ui, Icon::Right, 28.0, p).clicked() {
                            pk.month = calendar::shift_month(pk.month.0, pk.month.1, 1);
                        }
                    });
                    ui.add_space(4.0);
                    let cell = ui.available_width() / 7.0;
                    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
                    for (i, wd) in calendar::weekday_order(self.week_start()).iter().enumerate() {
                        ui.painter().text(
                            pos2(head.left() + cell * (i as f32 + 0.5), head.center().y),
                            Align2::CENTER_CENTER,
                            fmt::weekday_short(self.lang, *wd),
                            theme::semi(11.0),
                            p.muted,
                        );
                    }
                    let (body, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0 * 30.0), Sense::hover());
                    for (i, d) in grid.iter().enumerate() {
                        let r = Rect::from_min_size(pos2(body.left() + cell * (i % 7) as f32, body.top() + 30.0 * (i / 7) as f32), vec2(cell, 30.0)).shrink(1.0);
                        let resp = ui.interact(r, Id::unique(("pk-day", i)), Sense::click());
                        let sel = pk.current == Some(*d);
                        let today = *d == self.today;
                        if sel {
                            ui.painter().rect_filled(r, 8.0, p.accent);
                        } else if resp.hovered() {
                            ui.painter().rect_filled(r, 8.0, p.hover);
                        } else if today {
                            ui.painter().rect_stroke(r, 8.0, Stroke::new(1.2, p.accent), StrokeKind::Inside);
                        }
                        let in_month = d.month() == pk.month.1;
                        let col = if sel {
                            Color32::WHITE
                        } else if in_month {
                            p.text
                        } else {
                            p.muted.gamma_multiply(0.55)
                        };
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, d.day().to_string(), theme::reg(13.0), col);
                        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            chosen = Some(Some(*d));
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if widgets::btn(ui, t.today_btn, Btn::Soft, p).clicked() {
                            chosen = Some(Some(self.today));
                        }
                        if widgets::btn(ui, t.clear, Btn::Ghost, p).clicked() {
                            chosen = Some(None);
                        }
                    });
                });
        });
        let outside = ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|pp| !area.response.rect.contains(pp));
        if let Some(c) = chosen {
            let target = pk.target.clone();
            self.apply_date(ctx, &target, c);
            return;
        }
        if !outside {
            self.picker = Some(pk);
        }
    }

    pub(super) fn toast_ui(&mut self, ctx: &egui::Context, _p: &Pal) {
        let Some(toast) = &self.toast else { return };
        let t = self.s();
        let text = toast.text.clone();
        let has_undo = toast.undo.is_some();
        let until = toast.until;
        let mut undo = false;
        let mut dismiss = false;
        let dark = Pal::dark();
        egui::Area::new(Id::unique("toast")).order(Order::Tooltip).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -22.0)).show(ctx, |ui| {
            Frame::new()
                .fill(theme::hex(0x2b303a))
                .corner_radius(12)
                .inner_margin(Margin::symmetric(16, 10))
                .shadow(theme::shadow(8, 24, Color32::from_black_alpha(70)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&text).font(theme::reg(14.0)).color(Color32::WHITE));
                        if has_undo {
                            ui.add_space(8.0);
                            if widgets::btn(ui, t.undo, Btn::Link, &dark).clicked() {
                                undo = true;
                            }
                        }
                        if widgets::icon_button(ui, Icon::Close, 22.0, &dark).clicked() {
                            dismiss = true;
                        }
                    });
                });
        });
        if undo {
            self.undo(ctx);
        } else if dismiss {
            self.toast = None;
        } else if until < f64::MAX {
            let now = ctx.input(|i| i.time);
            ctx.request_repaint_after(std::time::Duration::from_millis(((until - now).max(0.05) * 1000.0) as u64 + 30));
        }
    }
}
