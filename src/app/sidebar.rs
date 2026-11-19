//! Left sidebar: the main navigation. Today, Calendar, the two profiles with their projects, Settings.
//! It can collapse to icons; projects can be reordered by dragging.

use super::widgets::{self, Icon};
use super::{App, Dialog};
use crate::config::Nav;
use crate::model::calendar::{items_in_range, overdue_items};
use crate::model::{Profile, ProfileFilter};
use crate::theme::{self, Pal};
use eframe::egui::{self, Align2, Color32, CursorIcon, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

struct Row<'a> {
    icon: Option<Icon>,
    dot: Option<Color32>,
    label: &'a str,
    /// (overdue, open): overdue is shown in red before the count.
    counts: (usize, usize),
    selected: bool,
    collapsed: bool,
    draggable: bool,
}

/// One navigation row. Returns the response (clicked = navigate).
fn nav_row(ui: &mut Ui, p: &Pal, r: Row) -> egui::Response {
    let sense = if r.draggable { Sense::click_and_drag() } else { Sense::click() };
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), sense);
    // The selected row is a raised surface with a hairline, never an accent wash.
    if r.selected {
        ui.painter().rect(rect, theme::R_CONTROL as f32, widgets::selected_fill(p), Stroke::new(1.0, p.line_hi), StrokeKind::Inside);
        ui.painter().hline(rect.x_range().shrink(8.0), rect.top() + 0.5, Stroke::new(1.0, p.top_light()));
    } else {
        ui.painter().rect_filled(rect, theme::R_CONTROL as f32, widgets::hover_fill(ui, resp.id, resp.hovered(), p));
    }
    let col = if r.selected { p.text } else { p.text2 };
    let icon_col = if r.selected { p.text } else { p.muted };
    if r.collapsed {
        let c = rect.center();
        if let Some(ic) = r.icon {
            widgets::icon(ui.painter(), ic, c, 18.0, icon_col);
        } else if let Some(d) = r.dot {
            ui.painter().circle_filled(c, 5.0, d);
        }
        if r.counts.0 > 0 {
            ui.painter().circle_filled(pos2(rect.right() - 8.0, rect.top() + 8.0), 3.0, p.negative);
        }
        return resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(r.label);
    }
    // Icons and dots share one 16 px column; labels start on one line.
    let mut x = rect.left() + 12.0;
    if let Some(ic) = r.icon {
        widgets::icon(ui.painter(), ic, pos2(x + 8.0, rect.center().y), 16.0, icon_col);
    }
    if let Some(d) = r.dot {
        ui.painter().circle_filled(pos2(x + 8.0, rect.center().y), 4.0, d);
    }
    if r.icon.is_some() || r.dot.is_some() {
        x += 24.0;
    }
    // Counts are quiet mono text, right-aligned: [overdue] [open]. Only the overdue count has a colour.
    let mut rx = rect.right() - 12.0;
    if r.counts.1 > 0 {
        let g = ui.painter().layout_no_wrap(r.counts.1.to_string(), theme::mono(12.0), p.muted);
        rx -= g.size().x;
        ui.painter().galley(pos2(rx, rect.center().y - g.size().y / 2.0), g, p.muted);
        rx -= 8.0;
    }
    if r.counts.0 > 0 {
        let g = ui.painter().layout_no_wrap(r.counts.0.to_string(), theme::mono(12.0), p.negative);
        rx -= g.size().x;
        ui.painter().galley(pos2(rx, rect.center().y - g.size().y / 2.0), g, p.negative);
        rx -= 8.0;
    }
    let font = if r.selected { theme::medium(14.0) } else { theme::reg(14.0) };
    let clip = Rect::from_min_max(pos2(x, rect.top()), pos2(rx.max(x + 20.0), rect.bottom()));
    ui.painter().with_clip_rect(clip).text(pos2(x, rect.center().y), Align2::LEFT_CENTER, r.label, font, col);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

impl App {
    pub(super) fn sidebar(&mut self, ui: &mut Ui, p: &Pal) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        let collapsed = self.cfg.sidebar_collapsed;
        let mut toggle = false;
        if collapsed {
            let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
            widgets::mark_glyph(ui.painter(), row.center(), p.accent);
            if resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(t.expand_sidebar).clicked() {
                toggle = true;
            }
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let (mark, _) = ui.allocate_exact_size(vec2(28.0, 36.0), Sense::hover());
                widgets::mark_glyph(ui.painter(), mark.center(), p.accent);
                ui.label(RichText::new("Planora").font(theme::medium(18.0)).color(p.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::icon_button(ui, Icon::Left, 28.0, p).on_hover_text(t.collapse_sidebar).clicked() {
                        toggle = true;
                    }
                });
            });
        }
        ui.add_space(16.0);

        let bottom_h = 44.0;
        let h = (ui.available_height() - bottom_h).max(80.0);
        let mut nav: Option<Nav> = None;
        let mut dialog: Option<Dialog> = None;
        let mut archive_toggle: Option<(String, bool)> = None;
        let mut delete_project: Option<String> = None;
        let mut proj_rows: Vec<(String, Profile, Rect)> = Vec::new();
        let mut started_drag: Option<String> = None;
        ScrollArea::vertical().max_height(h).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let cur = self.cfg.nav.clone();
            let today_count = items_in_range(&self.data.tasks, self.today, self.today, ProfileFilter::All, true, self.now)
                .values()
                .map(|v| v.len())
                .sum::<usize>();
            let overdue = overdue_items(&self.data.tasks, self.now, ProfileFilter::All).len();
            let row = |icon, label, counts, selected| Row { icon: Some(icon), dot: None, label, counts, selected, collapsed, draggable: false };
            if nav_row(ui, p, row(Icon::Today, t.today, (overdue, today_count), cur == Nav::Today)).clicked() {
                nav = Some(Nav::Today);
            }
            if nav_row(ui, p, row(Icon::Calendar, t.calendar, (0, 0), cur == Nav::Calendar)).clicked() {
                nav = Some(Nav::Calendar);
            }
            for profile in Profile::ALL {
                ui.add_space(if collapsed { 8.0 } else { 20.0 });
                let c = self.color_of(profile);
                if collapsed {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), Sense::hover());
                    ui.painter().rect_filled(Rect::from_center_size(r.center(), vec2(22.0, 4.0)), 2.0, c);
                } else {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                        ui.painter().rect_filled(r, 2.0, c);
                        theme::section(ui, self.profile_name(profile), p);
                    });
                }
                ui.add_space(4.0);
                let inbox = Nav::Inbox(profile);
                let counts = (self.overdue_count(profile, None), self.open_count(profile, None));
                let r = Row { icon: Some(Icon::Inbox), dot: None, label: t.inbox, counts, selected: cur == inbox, collapsed, draggable: false };
                if nav_row(ui, p, r).clicked() {
                    nav = Some(inbox);
                }
                let mut archived_exists = false;
                for pr in self.data.projects.iter().filter(|x| x.profile == profile) {
                    if pr.archived {
                        archived_exists = true;
                        if !self.show_archived {
                            continue;
                        }
                    }
                    let this = Nav::Project(pr.id.clone());
                    let label = if pr.archived { format!("{} ({})", pr.name, t.archived_short) } else { pr.name.clone() };
                    let counts = (self.overdue_count(profile, Some(&pr.id)), self.open_count(profile, Some(&pr.id)));
                    let r = Row {
                        icon: None,
                        dot: Some(widgets::rgb(pr.color)),
                        label: &label,
                        counts,
                        selected: cur == this,
                        collapsed,
                        draggable: true,
                    };
                    let resp = nav_row(ui, p, r);
                    proj_rows.push((pr.id.clone(), profile, resp.rect));
                    if resp.drag_started() {
                        started_drag = Some(pr.id.clone());
                    }
                    if resp.clicked() {
                        nav = Some(this);
                    }
                    resp.context_menu(|ui| {
                        if ui.button(t.edit_project).clicked() {
                            dialog = Some(Dialog::project_edit(pr));
                            ui.close();
                        }
                        let (label, flag) = if pr.archived { (t.unarchive, false) } else { (t.archive, true) };
                        if ui.button(label).clicked() {
                            archive_toggle = Some((pr.id.clone(), flag));
                            ui.close();
                        }
                        if ui.button(t.delete_project).clicked() {
                            delete_project = Some(pr.id.clone());
                            ui.close();
                        }
                    });
                }
                // "+ New project" under every profile.
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                ui.painter().rect_filled(rect, theme::R_CONTROL as f32, widgets::hover_fill(ui, resp.id, resp.hovered(), p));
                let col = theme::mix(p.muted, p.text, theme::fade(ui, resp.id.with("col"), resp.hovered()));
                if collapsed {
                    widgets::icon(ui.painter(), Icon::Plus, rect.center(), 16.0, col);
                } else {
                    widgets::icon(ui.painter(), Icon::Plus, pos2(rect.left() + 20.0, rect.center().y), 16.0, col);
                    ui.painter().text(pos2(rect.left() + 36.0, rect.center().y), Align2::LEFT_CENTER, t.new_project, theme::reg(14.0), col);
                }
                if resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(t.new_project).clicked() {
                    dialog = Some(Dialog::project_new(profile, &self.cfg));
                }
                if archived_exists && !collapsed {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let label = if self.show_archived { t.hide_archived } else { t.show_archived_label };
                        if widgets::btn(ui, label, theme::Btn::Link, p).clicked() {
                            self.show_archived = !self.show_archived;
                        }
                    });
                }
            }
        });
        if started_drag.is_some() {
            self.sidebar_drag = started_drag;
        }
        self.sidebar_reorder(&ctx, p, &proj_rows);
        if toggle {
            self.cfg.sidebar_collapsed = !collapsed;
            self.touch_cfg(&ctx);
        }
        if let Some(n) = nav {
            self.go(&ctx, n);
        }
        if let Some(d) = dialog {
            self.dialog = Some(d);
        }
        if let Some((id, archived)) = archive_toggle {
            if let Some(pr) = self.data.projects.iter_mut().find(|x| x.id == id) {
                pr.archived = archived;
            }
            if archived && self.cfg.nav == Nav::Project(id.clone()) {
                self.go(&ctx, Nav::Today);
            }
            self.touch_data(&ctx);
        }
        if let Some(id) = delete_project {
            self.dialog = Some(Dialog::ConfirmDeleteProject { id, move_to_inbox: true });
        }

        ui.add_space(8.0);
        let cur = self.cfg.nav == Nav::Settings;
        let r = Row { icon: Some(Icon::Gear), dot: None, label: t.settings, counts: (0, 0), selected: cur, collapsed, draggable: false };
        if nav_row(ui, p, r).clicked() {
            self.go(&ctx, Nav::Settings);
        }
    }

    /// Drag a project to another row of the same profile to reorder; shows an insertion line meanwhile.
    fn sidebar_reorder(&mut self, ctx: &egui::Context, p: &Pal, rows: &[(String, Profile, Rect)]) {
        let Some(drag_id) = self.sidebar_drag.clone() else { return };
        let Some(&(_, drag_profile, _)) = rows.iter().find(|r| r.0 == drag_id) else {
            self.sidebar_drag = None;
            return;
        };
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        let pos = ctx.input(|i| i.pointer.latest_pos());
        let target = pos.and_then(|pp| rows.iter().find(|r| r.1 == drag_profile && r.0 != drag_id && r.2.contains(pp)));
        if let Some((tid, _, rect)) = target {
            let from = self.data.projects.iter().position(|x| x.id == drag_id);
            let to = self.data.projects.iter().position(|x| x.id == *tid);
            if let (Some(from), Some(to)) = (from, to) {
                let y = if from < to { rect.bottom() + 1.0 } else { rect.top() - 1.0 };
                let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("sidebar-drop"));
                ctx.layer_painter(layer).hline(rect.x_range(), y, Stroke::new(2.0, p.accent));
                if ctx.input(|i| i.pointer.any_released()) {
                    let item = self.data.projects.remove(from);
                    self.data.projects.insert(to, item);
                    self.touch_data(ctx);
                }
            }
        }
        if ctx.input(|i| i.pointer.any_released()) {
            self.sidebar_drag = None;
        }
    }
}
