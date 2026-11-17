//! The Today view and the shared item rows (also used by the day panel).

use super::calendar_view::fit_text;
use super::widgets::{self, Icon};
use super::App;
use crate::fmt;
use crate::model::calendar::{items_in_range, overdue_items, Item, ItemKind};
use crate::theme::{self, Pal};
use chrono::{Datelike, Duration, NaiveDate};
use eframe::egui::{self, Align, Align2, CursorIcon, Layout, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub enum ItemAction {
    Open(String),
    Toggle(String, NaiveDate),
}

impl App {
    pub(super) fn today_view(&mut self, ui: &mut Ui, p: &Pal) {
        let t = self.s();
        let today = self.today;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(t.today).font(theme::bold(24.0)).color(p.text));
                ui.add_space(-6.0);
                theme::hint(ui, &fmt::date_long(self.lang, today), p);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| self.header_right(ui, p, true));
        });
        ui.add_space(12.0);

        let filter = self.cfg.filter;
        let hide = self.cfg.hide_done;
        let overdue = overdue_items(&self.data.tasks, today, filter);
        let todays = items_in_range(&self.data.tasks, today, today, filter, hide, today).remove(&today).unwrap_or_default();
        let soon_map = items_in_range(&self.data.tasks, today + Duration::days(1), today + Duration::days(7), filter, hide, today);
        let soon: Vec<Item> = soon_map.into_values().flatten().collect();

        let mut action = None;
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if overdue.is_empty() && todays.is_empty() && soon.is_empty() {
                ui.add_space(60.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(t.nothing_today).font(theme::semi(18.0)).color(p.text));
                    ui.add_space(4.0);
                    theme::hint(ui, t.nothing_today_hint, p);
                });
                return;
            }
            if !overdue.is_empty() {
                section_title(ui, &format!("{} · {}", t.overdue, overdue.len()), Some(widgets::RED), p);
                action = action.take().or(self.item_list(ui, p, &overdue, true));
                ui.add_space(14.0);
            }
            section_title(ui, &format!("{} · {}", t.today, todays.len()), None, p);
            if todays.is_empty() {
                theme::hint(ui, t.nothing_today, p);
            } else {
                action = action.take().or(self.item_list(ui, p, &todays, false));
            }
            if !soon.is_empty() {
                ui.add_space(14.0);
                section_title(ui, t.coming_up, None, p);
                action = action.take().or(self.item_list(ui, p, &soon, true));
            }
            ui.add_space(8.0);
        });
        self.apply_item_action(ui.ctx(), action);
    }

    pub(super) fn apply_item_action(&mut self, ctx: &egui::Context, action: Option<ItemAction>) {
        match action {
            Some(ItemAction::Open(id)) => self.open_editor(&id),
            Some(ItemAction::Toggle(id, occ)) => self.toggle_done(ctx, &id, Some(occ)),
            None => {}
        }
    }

    /// Rows for calendar items. `show_date` adds the date (relative for overdue items).
    pub(super) fn item_list(&self, ui: &mut Ui, p: &Pal, items: &[Item], show_date: bool) -> Option<ItemAction> {
        let mut action = None;
        for (i, item) in items.iter().enumerate() {
            let Some(task) = self.data.task(&item.task_id) else { continue };
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
            let hov = resp.hovered();
            ui.painter().rect(rect, 10.0, if hov { theme::mix(p.card, p.accent, 0.05) } else { p.card }, Stroke::new(1.0, p.line), StrokeKind::Inside);
            let pc = self.color_of(task.profile);
            ui.painter().rect_filled(egui::Rect::from_min_size(rect.min + vec2(0.0, 8.0), vec2(3.0, rect.height() - 16.0)), 2.0, pc);

            let sr = egui::Rect::from_center_size(pos2(rect.left() + 26.0, rect.center().y), vec2(26.0, 26.0));
            let sresp = ui.interact(sr, ui.scope_id().with(("irow-status", i, &item.task_id)), Sense::click());
            if sresp.hovered() {
                ui.painter().circle_filled(sr.center(), 13.0, p.hover);
            }
            widgets::status_mark(ui.painter(), sr.center(), task.status, item.done, p);
            if sresp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                action = Some(ItemAction::Toggle(item.task_id.clone(), item.occ));
            } else if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                action = Some(ItemAction::Open(item.task_id.clone()));
            }

            let mut x = rect.left() + 48.0;
            let cy = rect.center().y;
            if item.kind == ItemKind::Deadline {
                widgets::icon(ui.painter(), Icon::Flag, pos2(x + 5.0, cy), 13.0, if item.overdue { widgets::RED } else { p.muted });
                x += 20.0;
            }
            if let Some(tm) = item.time {
                let s = fmt::time_text(tm, self.h24());
                let g = ui.painter().layout_no_wrap(s, theme::semi(13.0), p.accent);
                let w = g.size().x;
                ui.painter().galley(pos2(x, cy - g.size().y / 2.0), g, p.accent);
                x += w + 10.0;
            }
            // Right side: date, project.
            let mut rx = rect.right() - 14.0;
            if show_date {
                let s = date_label(self, item.date);
                let col = if item.overdue && !item.done { widgets::RED } else { p.muted };
                let g = ui.painter().layout_no_wrap(s, theme::reg(12.5), col);
                rx -= g.size().x;
                ui.painter().galley(pos2(rx, cy - g.size().y / 2.0), g, col);
                rx -= 14.0;
            }
            if let Some(name) = task.project_id.as_deref().and_then(|id| self.project_name(id)) {
                let g = fit_text(ui.painter(), name, theme::reg(12.0), p.muted, 150.0, None);
                rx -= g.size().x;
                ui.painter().galley(pos2(rx, cy - g.size().y / 2.0), g, p.muted);
                rx -= 14.0;
            }
            let col = if item.done { p.muted } else if item.overdue { widgets::RED } else { p.text };
            let strike = item.done.then(|| Stroke::new(1.0, p.muted));
            let g = fit_text(ui.painter(), &task.title, theme::reg(14.0), col, rx - x, strike);
            ui.painter().galley(pos2(x, cy - g.size().y / 2.0), g, col);
            ui.add_space(4.0);
        }
        action
    }
}

fn section_title(ui: &mut Ui, text: &str, color: Option<egui::Color32>, p: &Pal) {
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &text.to_uppercase(),
            0.0,
            egui::text::TextFormat { font_id: theme::semi(11.5), color: color.unwrap_or(p.muted), extra_letter_spacing: 0.8, ..Default::default() },
        );
        ui.label(job);
    });
    ui.add_space(4.0);
}

/// "Yesterday", "Today", "Tomorrow", or "Nov 20".
pub fn date_label(app: &App, d: NaiveDate) -> String {
    let t = app.s();
    match (d - app.today).num_days() {
        -1 => t.yesterday.to_string(),
        0 => t.today.to_string(),
        1 => t.tomorrow.to_string(),
        _ if d.year() == app.today.year() => fmt::date_short(app.lang, d),
        _ => fmt::date_full(app.lang, d),
    }
}

#[allow(dead_code)]
fn _unused(_: Align2) {}
