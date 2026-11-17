//! Month calendar with task chips, day panel and drag-to-reschedule.

use super::widgets::{self, Icon};
use super::{App, Drag};
use crate::fmt;
use crate::model::calendar::{self, items_in_range, Item, ItemKind};
use crate::model::{Profile, ProfileFilter};
use crate::theme::{self, Btn, Pal};
use chrono::{Datelike, NaiveDate};
use eframe::egui::{
    self, text::LayoutJob, text::TextFormat, text::TextWrapping, Align, Align2, Color32, CursorIcon, Layout, Order, Rect, RichText, Sense,
    Stroke, StrokeKind, Ui, pos2, vec2,
};
use std::sync::Arc;

const CHIP_H: f32 = 19.0;
const CHIP_GAP: f32 = 2.0;

/// Single-row galley cut to `max_width` with an ellipsis.
pub fn fit_text(painter: &egui::Painter, text: &str, font: egui::FontId, color: Color32, max_width: f32, strike: Option<Stroke>) -> Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    job.append(text, 0.0, TextFormat { font_id: font, color, strikethrough: strike.unwrap_or(Stroke::NONE), ..Default::default() });
    job.wrap = TextWrapping { max_width: max_width.max(1.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    painter.layout_job(job)
}

/// Paints a chip into `rect` and returns nothing; the caller owns the interaction.
#[allow(clippy::too_many_arguments)]
pub fn paint_chip(
    ui: &Ui,
    rect: Rect,
    item: &Item,
    title: &str,
    profile_color: Color32,
    time_text: Option<&str>,
    hovered: bool,
    p: &Pal,
) {
    let painter = ui.painter();
    let base = theme::mix(p.card, profile_color, if p.dark { 0.20 } else { 0.14 });
    let fill = if hovered { theme::mix(base, profile_color, 0.12) } else { base };
    painter.rect_filled(rect, 5.0, fill);
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(3.0, rect.height())), 2.0, profile_color);
    let mut x = rect.left() + 8.0;
    let cy = rect.center().y;
    let col = if item.done {
        p.muted
    } else if item.overdue {
        widgets::RED
    } else {
        p.text
    };
    if item.kind == ItemKind::Deadline {
        widgets::icon(painter, Icon::Flag, pos2(x + 4.0, cy), 11.0, if item.overdue && !item.done { widgets::RED } else { p.muted });
        x += 13.0;
    }
    if let Some(tt) = time_text {
        let g = painter.layout_no_wrap(tt.to_string(), theme::semi(11.0), if item.done { p.muted } else { p.muted });
        let w = g.size().x;
        painter.galley(pos2(x, cy - g.size().y / 2.0), g, p.muted);
        x += w + 5.0;
    }
    let strike = item.done.then(|| Stroke::new(1.0, p.muted));
    let g = fit_text(painter, title, theme::reg(12.0), col, rect.right() - x - 5.0, strike);
    painter.galley(pos2(x, cy - g.size().y / 2.0), g, col);
}

impl App {
    /// Right side of the page header: profile filter, hide done, search and new task.
    pub(super) fn header_right(&mut self, ui: &mut Ui, p: &Pal, with_filter: bool, export: Option<super::export::ExportTarget>) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        // With the editor open there is little room: the button shrinks to "+" and labels go.
        let narrow = ui.available_width() < 560.0;
        let rect = ui.allocate_exact_size(vec2(if narrow { 38.0 } else { 132.0 }, 36.0), Sense::hover()).0;
        let label = if narrow { "+" } else { t.new_task };
        if theme::gradient_button(ui, rect, ui.scope_id().with("new-task"), label, None, true, &self.tex, p).on_hover_text(format!("{} (Ctrl+N)", t.new_task)).clicked() {
            self.open_quick_add(None);
        }
        if let Some(target) = export {
            if theme::button(ui, t.export_pdf, Btn::Ghost, p).clicked() {
                self.export_pdf(&ctx, target);
            }
        }
        if widgets::icon_button(ui, Icon::Search, 34.0, p).on_hover_text(format!("{} (Ctrl+F)", t.search)).clicked() {
            self.dialog = Some(super::Dialog::Search { query: String::new(), focus: true });
        }
        if with_filter {
            let labels = [t.all, t.personal, t.work];
            let sel = match self.cfg.filter {
                ProfileFilter::All => 0,
                ProfileFilter::Personal => 1,
                ProfileFilter::Work => 2,
            };
            let colors = [None, Some(self.color_of(Profile::Personal)), Some(self.color_of(Profile::Work))];
            if let Some(i) = widgets::segmented(ui, "pf", &labels, sel, &colors, p) {
                self.cfg.filter = [ProfileFilter::All, ProfileFilter::Personal, ProfileFilter::Work][i];
                self.touch_cfg(&ctx);
            }
        }
        if with_filter {
            self.hide_done_toggle(ui, p, narrow);
        }
    }

    pub(super) fn hide_done_toggle(&mut self, ui: &mut Ui, p: &Pal, narrow: bool) {
        let ctx = ui.ctx().clone();
        let mut hide = self.cfg.hide_done;
        if theme::switch(ui, &mut hide, p).on_hover_text(self.s().hide_done).changed() {
            self.cfg.hide_done = hide;
            self.touch_cfg(&ctx);
        }
        if !narrow {
            ui.label(RichText::new(self.s().hide_done).font(theme::reg(13.0)).color(p.muted));
        }
    }

    pub(super) fn calendar_view(&mut self, ui: &mut Ui, p: &Pal) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        let (year, month) = self.cal;
        ui.horizontal(|ui| {
            ui.label(RichText::new(fmt::month_title(self.lang, year, month)).font(theme::bold(24.0)).color(p.text));
            ui.add_space(8.0);
            if widgets::icon_button(ui, Icon::Left, 32.0, p).on_hover_text(format!("{} (PgUp)", t.prev_month)).clicked() {
                self.cal = calendar::shift_month(year, month, -1);
            }
            if theme::button(ui, t.today_btn, Btn::Ghost, p).clicked() {
                self.cal = (self.today.year(), self.today.month());
            }
            if widgets::icon_button(ui, Icon::Right, 32.0, p).on_hover_text(format!("{} (PgDn)", t.next_month)).clicked() {
                self.cal = calendar::shift_month(year, month, 1);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| self.header_right(ui, p, true, None));
        });
        ui.add_space(10.0);

        let (year, month) = self.cal;
        let grid = calendar::month_grid(year, month, self.week_start());
        let items = items_in_range(&self.data.tasks, grid[0], grid[41], self.cfg.filter, self.cfg.hide_done, self.today);

        // Weekday header.
        let area = ui.available_rect_before_wrap();
        let gap = 6.0;
        let cell_w = (area.width() - gap * 6.0) / 7.0;
        let head_h = 22.0;
        for (i, wd) in calendar::weekday_order(self.week_start()).iter().enumerate() {
            let x = area.left() + i as f32 * (cell_w + gap);
            let weekend = matches!(wd, chrono::Weekday::Sat | chrono::Weekday::Sun);
            ui.painter().text(
                pos2(x + cell_w / 2.0, area.top() + head_h / 2.0),
                Align2::CENTER_CENTER,
                fmt::weekday_short(self.lang, *wd),
                theme::semi(12.0),
                if weekend { p.accent2 } else { p.muted },
            );
        }
        let grid_top = area.top() + head_h + 4.0;
        let cell_h = ((area.bottom() - grid_top - gap * 5.0) / 6.0).max(76.0);
        ui.allocate_rect(Rect::from_min_max(area.min, pos2(area.right(), grid_top + 6.0 * cell_h + 5.0 * gap)), Sense::hover());

        let pointer = ctx.input(|i| i.pointer.latest_pos());
        let mut hover_cell: Option<NaiveDate> = None;
        let mut open_day: Option<NaiveDate> = None;
        let mut open_task: Option<String> = None;
        let mut start_drag: Option<Drag> = None;
        let dragging = self.drag.is_some();

        for (idx, day) in grid.iter().enumerate() {
            let (row, col) = (idx / 7, idx % 7);
            let rect = Rect::from_min_size(
                pos2(area.left() + col as f32 * (cell_w + gap), grid_top + row as f32 * (cell_h + gap)),
                vec2(cell_w, cell_h),
            );
            let in_month = day.month() == month;
            let is_today = *day == self.today;
            let weekend = matches!(day.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun);
            if pointer.is_some_and(|pp| rect.expand(gap / 2.0).contains(pp)) {
                hover_cell = Some(*day);
            }
            let cell_resp = ui.interact(rect, ui.scope_id().with(("cell", idx)), Sense::click());
            let mut fill = if weekend { theme::mix(p.card, p.accent2, 0.05) } else { p.card };
            if !in_month {
                fill = theme::mix(fill, p.bg, 0.55);
            }
            if cell_resp.hovered() && !dragging {
                fill = theme::mix(fill, p.accent, 0.06);
            }
            let is_drop = dragging && hover_cell == Some(*day);
            let stroke = if is_drop {
                Stroke::new(2.0, p.accent)
            } else if is_today {
                Stroke::new(1.5, p.accent)
            } else {
                Stroke::new(1.0, p.line)
            };
            ui.painter().rect(rect, 10.0, fill, stroke, StrokeKind::Inside);
            if cell_resp.clicked() {
                open_day = Some(*day);
            }

            // Day number.
            let num_pos = pos2(rect.left() + 10.0, rect.top() + 14.0);
            if is_today {
                ui.painter().circle_filled(num_pos + vec2(5.0, 0.0), 11.0, p.accent);
                ui.painter().text(num_pos + vec2(5.0, 0.0), Align2::CENTER_CENTER, day.day().to_string(), theme::bold(12.5), Color32::WHITE);
            } else {
                let c = if in_month { p.text } else { p.muted.gamma_multiply(0.6) };
                ui.painter().text(num_pos + vec2(5.0, 0.0), Align2::CENTER_CENTER, day.day().to_string(), theme::semi(12.5), c);
            }

            // Chips.
            let list: &[Item] = items.get(day).map(|v| v.as_slice()).unwrap_or(&[]);
            if list.is_empty() {
                continue;
            }
            let avail_h = rect.height() - 30.0 - 4.0;
            let max = ((avail_h + CHIP_GAP) / (CHIP_H + CHIP_GAP)).floor().max(1.0) as usize;
            let (shown, more) = if list.len() > max { (max.saturating_sub(1).max(1), list.len() - max.saturating_sub(1).max(1)) } else { (list.len(), 0) };
            let mut y = rect.top() + 30.0;
            for (k, item) in list.iter().take(shown).enumerate() {
                let Some(task) = self.data.task(&item.task_id) else { continue };
                let chip = Rect::from_min_size(pos2(rect.left() + 6.0, y), vec2(rect.width() - 12.0, CHIP_H));
                y += CHIP_H + CHIP_GAP;
                let resp = ui.interact(chip, ui.scope_id().with(("chip", idx, k)), Sense::click_and_drag());
                let ttxt = item.time.map(|tt| fmt::time_text(tt, self.h24()));
                let is_dragged = self.drag.as_ref().is_some_and(|d| d.task_id == item.task_id && d.kind == item.kind);
                let pc = self.color_of(task.profile);
                if is_dragged {
                    ui.painter().rect_stroke(chip, 5.0, Stroke::new(1.0, p.line), StrokeKind::Inside);
                } else {
                    paint_chip(ui, chip, item, &task.title, pc, ttxt.as_deref(), resp.hovered(), p);
                }
                if resp.clicked() {
                    open_task = Some(item.task_id.clone());
                }
                if resp.drag_started() {
                    start_drag = Some(Drag { task_id: item.task_id.clone(), kind: item.kind, from: item.date });
                }
                if resp.dragged() || resp.hovered() {
                    ctx.set_cursor_icon(if resp.dragged() { CursorIcon::Grabbing } else { CursorIcon::PointingHand });
                }
                if !resp.dragged() {
                    let tip = match &ttxt {
                        Some(tt) => format!("{tt}  {}", task.title),
                        None => task.title.clone(),
                    };
                    resp.on_hover_text(tip);
                }
            }
            if more > 0 {
                let r = Rect::from_min_size(pos2(rect.left() + 6.0, y), vec2(rect.width() - 12.0, CHIP_H));
                let resp = ui.interact(r, ui.scope_id().with(("more", idx)), Sense::click());
                let col = if resp.hovered() { p.accent } else { p.muted };
                ui.painter().text(pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, t.more.replace("{n}", &more.to_string()), theme::semi(11.5), col);
                if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    open_day = Some(*day);
                }
            }
        }

        if let Some(d) = start_drag {
            self.drag = Some(d);
        }
        // Drop.
        if ctx.input(|i| i.pointer.any_released()) {
            if let (Some(drag), Some(target)) = (self.drag.take(), hover_cell) {
                if target != drag.from {
                    let now = self.now;
                    if let Some(task) = self.data.task_mut(&drag.task_id) {
                        calendar::move_item(task, drag.kind, drag.from, target, now);
                        self.touch_data(&ctx);
                    }
                }
            }
        }
        if let Some(id) = open_task {
            self.open_editor(&id);
        } else if let Some(d) = open_day {
            self.day_panel = Some(d);
        }
    }

    /// The chip that follows the pointer while dragging.
    pub(super) fn drag_ghost(&mut self, ctx: &egui::Context, p: &Pal) {
        let Some(drag) = &self.drag else { return };
        let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) else { return };
        let Some(task) = self.data.task(&drag.task_id) else { return };
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        let item = Item {
            task_id: task.id.clone(),
            kind: drag.kind,
            date: drag.from,
            occ: drag.from,
            time: None,
            done: false,
            overdue: false,
            profile: task.profile,
        };
        let title = task.title.clone();
        let pc = self.color_of(task.profile);
        egui::Area::new(egui::Id::unique("drag-ghost")).order(Order::Tooltip).fixed_pos(pos + vec2(10.0, 6.0)).interactable(false).show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(160.0, CHIP_H + 2.0), Sense::hover());
            ui.painter().rect_filled(rect.expand(1.0), 6.0, Color32::from_black_alpha(60));
            ui.painter().rect_filled(rect, 5.0, p.card);
            paint_chip(ui, rect, &item, &title, pc, None, true, p);
        });
    }
}
