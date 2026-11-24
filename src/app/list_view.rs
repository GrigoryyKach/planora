//! Inbox and project views: project cards, filters, sorting and task rows with quick actions.

use super::calendar_view::fit_text;
use super::widgets::{self, Icon};
use super::{App, DateTarget, Dialog, Picker};
use crate::config::{Nav, SortMode};
use crate::fmt;
use crate::model::calendar::{end_of_week, is_overdue};
use crate::model::{Priority, Profile, Status, Task};
use crate::theme::{self, Btn, Pal};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use eframe::egui::{
    self, Align, Align2, CursorIcon, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

#[derive(Clone)]
pub enum Scope {
    Inbox(Profile),
    Project(String),
}

enum RowAct {
    Open(String),
    Cycle(String),
    Status(String, Option<Status>),
    Prio(String, Option<Priority>),
    Deadline(String, Option<NaiveDate>),
    Sched(String, Option<NaiveDate>),
    Pick(String, bool, Pos2),
    MoveTo(String, Profile, Option<String>),
    Dup(String),
    Del(String),
    Swap(String, String),
    ToggleExpand(String),
    AddSub(String),
    MakeSub(String),
    ToTask(String),
}

/// What a parent row shows about its subtasks.
#[derive(Clone, Copy)]
struct Group {
    done: usize,
    total: usize,
    expanded: bool,
}

/// Row geometry: a subtask row is indented and a little smaller than a task row.
const ROW_H_TASK: f32 = 40.0;
const ROW_H_SUB: f32 = 34.0;
const SUB_INDENT: f32 = 28.0;

fn next_week_start(today: NaiveDate, ws: Weekday) -> NaiveDate {
    let since = (7 + today.weekday().num_days_from_monday() as i64 - ws.num_days_from_monday() as i64) % 7;
    today + Duration::days(7 - since)
}

impl App {
    fn scope_matches(&self, scope: &Scope, t: &Task) -> bool {
        match scope {
            Scope::Inbox(p) => t.profile == *p && t.project_id.is_none(),
            Scope::Project(id) => t.project_id.as_deref() == Some(id),
        }
    }

    /// Is any list filter active (so subtasks that match are shown even under a collapsed parent)?
    fn filtering(&self) -> bool {
        let f = &self.filter;
        !f.text.trim().is_empty() || f.status.iter().any(|b| *b) || f.priority.iter().any(|b| *b) || f.deadline.iter().any(|b| *b)
    }

    /// The subtasks of a parent that a list shows: all of them, or the ones that pass the filters.
    pub(super) fn sub_rows(&self, parent: &str) -> Vec<&Task> {
        self.data.subtasks_of(parent).into_iter().filter(|x| self.passes_filter(x)).collect()
    }

    /// The top-level tasks a list shows: scope, active filters and sort order (also used by the PDF export).
    /// A parent is listed when it passes the filters or when one of its subtasks does.
    pub(super) fn list_rows(&self, scope: &Scope) -> Vec<&Task> {
        let mut rows: Vec<&Task> = self
            .data
            .tasks
            .iter()
            .filter(|x| x.parent_id.is_none() && self.scope_matches(scope, x))
            .filter(|x| self.passes_filter(x) || (self.data.has_subtasks(&x.id) && !self.sub_rows(&x.id).is_empty()))
            .collect();
        match self.cfg.sort {
            SortMode::Manual => {}
            SortMode::Deadline => rows.sort_by_key(|x| (x.deadline.is_none(), x.deadline)),
            SortMode::Priority => rows.sort_by_key(|x| std::cmp::Reverse(x.priority)),
            SortMode::Created => rows.sort_by_key(|x| std::cmp::Reverse(x.created)),
        }
        rows
    }

    fn passes_filter(&self, t: &Task) -> bool {
        let f = &self.filter;
        if self.cfg.hide_done && t.is_closed() {
            return false;
        }
        let q = f.text.trim().to_lowercase();
        if !q.is_empty() && !t.title.to_lowercase().contains(&q) && !t.description.to_lowercase().contains(&q) {
            return false;
        }
        if f.status.iter().any(|b| *b) {
            let i = match t.status {
                None => 0,
                Some(Status::Todo) => 1,
                Some(Status::InProgress) => 2,
                Some(Status::Done) => 3,
                Some(Status::Cancelled) => 4,
            };
            if !f.status[i] {
                return false;
            }
        }
        if f.priority.iter().any(|b| *b) {
            let i = match t.priority {
                None => 0,
                Some(Priority::Low) => 1,
                Some(Priority::Medium) => 2,
                Some(Priority::High) => 3,
                Some(Priority::Urgent) => 4,
            };
            if !f.priority[i] {
                return false;
            }
        }
        let today = self.today;
        if f.deadline.iter().any(|b| *b) {
            let ok = (f.deadline[0] && is_overdue(t, self.now))
                || (f.deadline[1] && t.deadline == Some(today))
                || (f.deadline[2] && t.deadline.is_some_and(|d| d >= today && d <= end_of_week(today, self.week_start())))
                || (f.deadline[3] && t.deadline.is_none());
            return ok;
        }
        true
    }

    pub(super) fn open_picker(&mut self, target: DateTarget, anchor: Pos2, current: Option<NaiveDate>) {
        let base = current.unwrap_or(self.today);
        self.picker = Some(Picker { target, anchor, month: (base.year(), base.month()), current });
    }

    pub(super) fn list_view(&mut self, ui: &mut Ui, p: &Pal, scope: Scope) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        let project = match &scope {
            Scope::Project(id) => match self.data.project(id) {
                Some(pr) => Some(pr.clone()),
                None => {
                    self.go(&ctx, Nav::Today);
                    return;
                }
            },
            Scope::Inbox(_) => None,
        };
        let profile = match &scope {
            Scope::Inbox(p) => *p,
            Scope::Project(_) => project.as_ref().map(|x| x.profile).unwrap_or(Profile::Personal),
        };

        // Header.
        ui.horizontal(|ui| {
            let (dot, title) = match &project {
                Some(pr) => (widgets::rgb(pr.color), pr.name.clone()),
                None => (self.color_of(profile), format!("{} · {}", self.profile_name(profile), t.inbox)),
            };
            let (r, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
            ui.painter().rect_filled(r, 3.0, dot);
            ui.add(egui::Label::new(RichText::new(title).font(theme::medium(24.0)).color(p.text)).truncate());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| self.header_right(ui, p, false, Some(super::export::ExportTarget::Scope(scope.clone()))));
        });
        ui.add_space(16.0);

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // Project card, or cards of all projects in the inbox.
            match &project {
                Some(pr) => {
                    if let Some(id) = self.project_card(ui, p, &pr.id, true) {
                        let _ = id;
                    }
                }
                None => {
                    let ids: Vec<String> = self.data.projects.iter().filter(|x| x.profile == profile && !x.archived).map(|x| x.id.clone()).collect();
                    if !ids.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                            for id in &ids {
                                if let Some(open) = self.project_card(ui, p, id, false) {
                                    self.go(&ctx, Nav::Project(open));
                                }
                            }
                        });
                        ui.add_space(16.0);
                    }
                }
            }

            if project.is_some() {
                ui.add_space(8.0);
            }
            self.quick_add_row(ui, p, profile, project.as_ref().map(|x| x.id.clone()));
            ui.add_space(12.0);
            self.filter_bar(ui, p);
            ui.add_space(8.0);

            // Rows.
            let rows = self.list_rows(&scope);
            if rows.is_empty() {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(t.no_tasks).font(theme::medium(16.0)).color(p.text));
                    ui.add_space(2.0);
                    theme::hint(ui, t.no_tasks_hint, p);
                });
                return;
            }
            let ids: Vec<String> = rows.iter().map(|x| x.id.clone()).collect();
            let manual = self.cfg.sort == SortMode::Manual;
            let filtering = self.filtering();
            let mut act: Option<RowAct> = None;
            self.column_header(ui, p);
            for (i, task) in rows.iter().enumerate() {
                let prev = if manual && i > 0 { Some(ids[i - 1].clone()) } else { None };
                let next = if manual { ids.get(i + 1).cloned() } else { None };
                let all_subs = self.data.subtasks_of(&task.id);
                let group = (!all_subs.is_empty()).then(|| {
                    let (done, total) = self.data.subtask_progress(&task.id);
                    Group { done, total, expanded: self.is_expanded(&task.id) || filtering }
                });
                if let Some(a) = self.task_row(ui, p, task, prev, next, group, false) {
                    act = Some(a);
                }
                if group.is_some_and(|g| g.expanded) {
                    let subs = self.sub_rows(&task.id);
                    let sub_ids: Vec<String> = subs.iter().map(|x| x.id.clone()).collect();
                    for (j, sub) in subs.iter().enumerate() {
                        let prev = if j > 0 { Some(sub_ids[j - 1].clone()) } else { None };
                        let next = sub_ids.get(j + 1).cloned();
                        if let Some(a) = self.task_row(ui, p, sub, prev, next, None, true) {
                            act = Some(a);
                        }
                    }
                }
            }
            ui.add_space(10.0);
            if let Some(a) = act {
                self.apply_row_act(&ctx, a);
            }
        });
    }

    fn apply_row_act(&mut self, ctx: &egui::Context, a: RowAct) {
        match a {
            RowAct::Open(id) => self.open_editor(&id),
            RowAct::Cycle(id) => self.cycle_status(ctx, &id),
            RowAct::Status(id, s) => self.set_status(ctx, &id, s),
            RowAct::Prio(id, p) => self.set_priority(ctx, &id, p),
            RowAct::Deadline(id, d) => self.edit_task(ctx, &id, |t| t.deadline = d),
            RowAct::Sched(id, d) => self.edit_task(ctx, &id, |t| t.scheduled = d),
            RowAct::Pick(id, deadline, pos) => {
                let cur = self.data.task(&id).and_then(|t| if deadline { t.deadline } else { t.scheduled });
                let target = if deadline { DateTarget::Deadline(id) } else { DateTarget::Scheduled(id) };
                self.open_picker(target, pos, cur);
            }
            RowAct::MoveTo(id, profile, project) => self.move_task_to(ctx, &id, profile, project),
            RowAct::Dup(id) => self.duplicate_task(ctx, &id),
            RowAct::Del(id) => self.ask_delete(&id),
            RowAct::Swap(a, b) => self.move_task(ctx, &a, &b),
            RowAct::ToggleExpand(id) => self.toggle_expanded(ctx, &id),
            RowAct::AddSub(id) => {
                self.open_editor(&id);
                if let Some(ed) = &mut self.editor {
                    ed.focus_sub = true;
                }
            }
            RowAct::MakeSub(id) => self.dialog = Some(Dialog::PickParent { id, query: String::new(), focus: true }),
            RowAct::ToTask(id) => self.convert_to_task(ctx, &id),
        }
    }

    /// A project card with progress. Returns the project id when the (compact) card is clicked.
    fn project_card(&mut self, ui: &mut Ui, p: &Pal, id: &str, wide: bool) -> Option<String> {
        let t = self.s();
        let pr = self.data.project(id)?.clone();
        // Top-level tasks only: a parent stands for its subtasks, so counting both would count the same work twice.
        let tasks: Vec<&Task> = self.data.tasks.iter().filter(|x| x.parent_id.is_none() && x.project_id.as_deref() == Some(id)).collect();
        let total = tasks.iter().filter(|x| x.status != Some(Status::Cancelled)).count();
        let done = tasks.iter().filter(|x| x.status == Some(Status::Done)).count();
        let share = if total == 0 { 0.0 } else { done as f32 / total as f32 };
        let next = tasks.iter().filter(|x| !x.is_closed()).filter_map(|x| x.deadline).min();
        let color = widgets::rgb(pr.color);
        let size = if wide { vec2(ui.available_width(), 96.0) } else { vec2(228.0, 92.0) };
        let (rect, resp) = ui.allocate_exact_size(size, if wide { Sense::hover() } else { Sense::click() });
        let hov = theme::fade(ui, resp.id.with("hover"), resp.hovered() && !wide);
        ui.painter().rect(rect, theme::R_CARD as f32, theme::mix(p.card, p.raised, hov), Stroke::new(1.0, theme::mix(p.line, p.line_hi, hov)), StrokeKind::Inside);
        ui.painter().rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 16.0), vec2(4.0, rect.height() - 32.0)), 2.0, color);
        let x = rect.left() + 20.0;
        let g = fit_text(ui.painter(), &pr.name, theme::medium(15.0), p.text, rect.width() - 40.0, None);
        ui.painter().galley(pos2(x, rect.top() + 12.0), g, p.text);
        let mut y = rect.top() + 38.0;
        if wide && !pr.description.is_empty() {
            let g = fit_text(ui.painter(), &pr.description, theme::reg(12.0), p.muted, rect.width() - 200.0, None);
            ui.painter().galley(pos2(x, y - 4.0), g, p.muted);
            y += 18.0;
        }
        // Progress bar.
        let bar = Rect::from_min_size(pos2(x, y + 4.0), vec2((rect.width() - 40.0 - if wide { 160.0 } else { 0.0 }).max(60.0), 4.0));
        ui.painter().rect_filled(bar, 2.0, p.line_hi);
        if share > 0.0 {
            ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * share, bar.height())), 2.0, color);
        }
        let stats = t.project_progress.replace("{done}", &done.to_string()).replace("{total}", &total.to_string());
        ui.painter().text(pos2(x, bar.bottom() + 8.0), Align2::LEFT_TOP, stats, theme::reg(12.0), p.muted);
        if let Some(d) = next {
            let od = d < self.today;
            let label = t.next_deadline.replace("{date}", &fmt::date_short(self.lang, d));
            ui.painter().text(
                pos2(bar.right(), bar.bottom() + 8.0),
                Align2::RIGHT_TOP,
                label,
                theme::reg(12.0),
                if od { p.negative } else { p.muted },
            );
        }
        if wide {
            // Edit button in the card corner.
            let br = Rect::from_min_size(pos2(rect.right() - 148.0, rect.top() + 16.0), vec2(132.0, Btn::Secondary.height()));
            let resp = theme::button_at(ui, br, ui.scope_id().with("edit-project"), t.edit_project, Btn::Secondary, p);
            if resp.clicked() {
                self.dialog = Some(Dialog::project_edit(&pr));
            }
            let _ = resp;
            return None;
        }
        resp.on_hover_cursor(CursorIcon::PointingHand).clicked().then(|| pr.id.clone())
    }

    fn quick_add_row(&mut self, ui: &mut Ui, p: &Pal, profile: Profile, project: Option<String>) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        let id = ui.scope_id().with("inline-add");
        let mut text: String = ctx.data_mut(|d| d.get_temp(id).unwrap_or_default());
        let mut added = false;
        ui.horizontal(|ui| {
            let w = ui.available_width() - 100.0;
            let resp = widgets::input(ui, &mut text, t.add_task_hint, w);
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let click = widgets::btn(ui, t.add, Btn::Secondary, p).clicked();
            if (enter || click) && !text.trim().is_empty() {
                added = true;
                if enter {
                    resp.request_focus();
                }
            }
        });
        if added {
            let title = std::mem::take(&mut text);
            self.add_task(&ctx, profile, project, &title, None);
        }
        ctx.data_mut(|d| d.insert_temp(id, text));
    }

    fn filter_bar(&mut self, ui: &mut Ui, p: &Pal) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        // Wrapped: on a narrow window (side panel open) the controls flow onto a second line instead of overflowing.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            let mut q = std::mem::take(&mut self.filter.text);
            widgets::input(ui, &mut q, t.filter_hint, 220.0);
            self.filter.text = q;
            ui.add_space(6.0);
            let all = t.all;
            widgets::multi_select(ui, "f-status", t.status_label, all, t.clear, &[t.status_none, t.status_todo, t.status_progress, t.status_done, t.status_cancelled], &mut self.filter.status, p);
            widgets::multi_select(ui, "f-priority", t.priority_label, all, t.clear, &[t.priority_none, t.prio_low, t.prio_medium, t.prio_high, t.prio_urgent], &mut self.filter.priority, p);
            widgets::multi_select(ui, "f-deadline", t.deadline_label, all, t.clear, &[t.dl_overdue, t.dl_today, t.dl_week, t.dl_none], &mut self.filter.deadline, p);
            ui.add_space(6.0);
            // The sort label and its dropdown stay together when the row wraps.
            let sort_w = ui.painter().layout_no_wrap(t.sort_by.to_string(), theme::reg(13.0), p.text2).size().x + 8.0 + 130.0 + 16.0;
            ui.allocate_ui_with_layout(vec2(sort_w, super::ROW_H), Layout::left_to_right(Align::Center), |ui| {
                let mut s = self.cfg.sort;
                ui.label(RichText::new(t.sort_by).font(theme::reg(13.0)).color(p.text2));
                let names = [(SortMode::Manual, t.sort_manual), (SortMode::Deadline, t.sort_deadline), (SortMode::Priority, t.sort_priority), (SortMode::Created, t.sort_created)];
                let cur = names.iter().find(|n| n.0 == s).map(|n| n.1).unwrap_or("");
                widgets::combo("sort", cur, 130.0).show_ui(ui, |ui| {
                    for (m, n) in names {
                        if ui.selectable_label(s == m, RichText::new(n).font(theme::reg(13.5))).clicked() {
                            s = m;
                        }
                    }
                });
                if s != self.cfg.sort {
                    self.cfg.sort = s;
                    self.touch_cfg(&ctx);
                }
            });
            self.hide_done_toggle(ui, p, false);
            if self.filter.status.iter().any(|b| *b) || self.filter.priority.iter().any(|b| *b) || self.filter.deadline.iter().any(|b| *b) || !self.filter.text.is_empty() {
                if widgets::btn(ui, t.clear_filters, Btn::Link, p).clicked() {
                    self.filter = Default::default();
                }
            }
        });
    }

    fn column_header(&self, ui: &mut Ui, p: &Pal) {
        let t = self.s();
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
        let cols = columns(rect.width());
        let y = rect.center().y;
        let f = theme::medium(12.0);
        let right = rect.right() - 16.0;
        let st = ui.painter();
        st.text(pos2(rect.left() + 52.0, y), Align2::LEFT_CENTER, t.col_title, f.clone(), p.muted);
        st.text(pos2(right - cols.prio_x, y), Align2::LEFT_CENTER, t.col_priority, f.clone(), p.muted);
        st.text(pos2(right - cols.deadline_x, y), Align2::LEFT_CENTER, t.col_deadline, f.clone(), p.muted);
        st.text(pos2(right - cols.sched_x, y), Align2::LEFT_CENTER, t.col_scheduled, f, p.muted);
    }

    #[allow(clippy::too_many_lines)]
    fn task_row(&self, ui: &mut Ui, p: &Pal, task: &Task, prev: Option<String>, next: Option<String>, group: Option<Group>, sub: bool) -> Option<RowAct> {
        let t = self.s();
        let mut act: Option<RowAct> = None;
        let row_h = if sub { ROW_H_SUB } else { ROW_H_TASK };
        let (full, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
        // A subtask row is the same design, indented and a little smaller, joined to its parent by a thin elbow.
        let rect = if sub { Rect::from_min_max(pos2(full.left() + SUB_INDENT, full.top()), full.max) } else { full };
        if sub {
            let x = full.left() + SUB_INDENT / 2.0;
            ui.painter().vline(x, full.top() - 4.0..=rect.center().y, Stroke::new(1.0, p.line_hi));
            ui.painter().hline(x..=rect.left(), rect.center().y, Stroke::new(1.0, p.line_hi));
        }
        let hov = theme::fade(ui, resp.id.with("hover"), resp.hovered());
        let selected = self.selected.as_deref() == Some(&task.id) || self.editor.as_ref().is_some_and(|e| e.id == task.id);
        let occ = crate::model::calendar::current_occurrence(task, self.today);
        let done = match (task.recurrence.is_some(), occ) {
            (true, Some(o)) => task.done_dates.contains(&o) || task.is_closed(),
            _ => task.is_closed(),
        };
        let muted = done || task.status == Some(Status::Cancelled);
        // Selection is a raised surface with an outline; hover is the same surface fading in.
        let fill = if selected { p.raised } else { theme::mix(p.card, p.raised, hov) };
        ui.painter().rect(rect, theme::R_CONTROL as f32, fill, Stroke::new(1.0, if selected { p.line_hi } else { p.line }), StrokeKind::Inside);
        let cy = rect.center().y;
        let cols = columns(rect.width());
        let right = rect.right() - 16.0;

        // Chevron of a parent: shows or hides its subtasks (collapsed by default, the choice is saved).
        let mut chevron_hit = false;
        if let Some(g) = group {
            let cr = Rect::from_center_size(pos2(rect.left() + 8.0, cy), vec2(14.0, 28.0));
            let cresp = ui.interact(cr, ui.scope_id().with(("chevron", &task.id)), Sense::click());
            let col = theme::mix(p.muted, p.text, theme::fade(ui, cresp.id.with("hover"), cresp.hovered()));
            theme::icon_chevron(ui.painter(), cr.center(), col, g.expanded);
            if cresp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(if g.expanded { t.hide_subtasks } else { t.show_subtasks }).clicked() {
                act = Some(RowAct::ToggleExpand(task.id.clone()));
                chevron_hit = true;
            }
        }

        // Status button (separate hit area on top of the row).
        let sr = Rect::from_center_size(pos2(rect.left() + 29.0, cy), vec2(26.0, 26.0));
        let sresp = ui.interact(sr, ui.scope_id().with(("status", &task.id)), Sense::click());
        if sresp.hovered() {
            ui.painter().circle_filled(sr.center(), 13.0, p.raised_hi);
        }
        widgets::status_mark(ui.painter(), sr.center(), task.status, done, p);
        if sresp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(t.click_status).clicked() {
            act = Some(RowAct::Cycle(task.id.clone()));
        } else if resp.clicked() && !chevron_hit {
            act = Some(RowAct::Open(task.id.clone()));
        }

        // Title, and the progress pill of a parent ("2/5" subtasks done) right after it.
        let x = rect.left() + 52.0;
        let tcol = if muted { p.muted } else { p.text };
        let strike = muted.then(|| Stroke::new(1.0, p.muted));
        let title_right = right - cols.sched_x - 8.0;
        let pill = group.map(|g| {
            let text = format!("{}/{}", g.done, g.total);
            let gl = ui.painter().layout_no_wrap(text, theme::mono(11.5), if g.total > 0 && g.done == g.total { p.positive } else { p.text2 });
            (gl.size().x + 14.0, gl)
        });
        let pill_w = pill.as_ref().map_or(0.0, |(w, _)| w + 8.0);
        let font = if sub { theme::reg(13.5) } else { theme::reg(14.0) };
        let g = fit_text(ui.painter(), &task.title, font, tcol, title_right - x - pill_w, strike);
        let title_w = g.size().x;
        ui.painter().galley(pos2(x, cy - g.size().y / 2.0), g, tcol);
        if let (Some((w, gl)), Some(grp)) = (pill, group) {
            let pr = Rect::from_min_size(pos2(x + title_w + 8.0, cy - 10.0), vec2(w, 20.0));
            let presp = ui.interact(pr, ui.scope_id().with(("pill", &task.id)), Sense::click());
            let fill = if presp.hovered() { p.raised_hi } else { p.raised };
            ui.painter().rect_filled(pr, 10.0, fill);
            ui.painter().galley(pos2(pr.left() + 7.0, pr.center().y - gl.size().y / 2.0), gl, p.text2);
            if presp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(if grp.expanded { t.hide_subtasks } else { t.show_subtasks }).clicked() {
                act = Some(RowAct::ToggleExpand(task.id.clone()));
            }
        }

        // Columns.
        let font = theme::reg(12.0);
        if let Some(pr) = task.priority {
            ui.painter().circle_filled(pos2(right - cols.prio_x + 4.0, cy), 4.0, widgets::priority_color(pr));
            if cols.show_prio_text {
                let name = match pr {
                    Priority::Low => t.prio_low,
                    Priority::Medium => t.prio_medium,
                    Priority::High => t.prio_high,
                    Priority::Urgent => t.prio_urgent,
                };
                ui.painter().text(pos2(right - cols.prio_x + 14.0, cy), Align2::LEFT_CENTER, name, font.clone(), p.muted);
            }
        }
        if let Some(d) = task.deadline {
            let od = is_overdue(task, self.now);
            let col = if od { p.negative } else if muted { p.muted } else { p.text };
            widgets::icon(ui.painter(), Icon::Flag, pos2(right - cols.deadline_x + 5.0, cy), 12.0, if od { p.negative } else { p.muted });
            let mut dl = super::today_view::date_label(self, d);
            if let Some(tm) = task.deadline_time {
                dl = format!("{dl} {}", fmt::time_text(tm, self.h24()));
            }
            ui.painter().text(pos2(right - cols.deadline_x + 16.0, cy), Align2::LEFT_CENTER, dl, font.clone(), col);
        }
        if let Some(d) = task.scheduled {
            let mut s = super::today_view::date_label(self, d);
            if let Some(tm) = task.time {
                s = format!("{s} {}", fmt::time_text(tm, self.h24()));
            }
            widgets::icon(ui.painter(), if task.time.is_some() { Icon::Clock } else { Icon::Calendar }, pos2(right - cols.sched_x + 5.0, cy), 12.0, p.muted);
            ui.painter().text(pos2(right - cols.sched_x + 16.0, cy), Align2::LEFT_CENTER, s, font.clone(), if muted { p.muted } else { p.text });
        }
        // Checklist, recurrence and link marks, right-aligned.
        let mut mx = right;
        if !task.links.is_empty() {
            widgets::icon(ui.painter(), Icon::Link, pos2(mx - 8.0, cy), 15.0, p.muted);
            mx -= 24.0;
        }
        if task.recurrence.is_some() {
            widgets::icon(ui.painter(), Icon::Repeat, pos2(mx - 8.0, cy), 14.0, p.muted);
            mx -= 22.0;
        }
        let (cd, ct) = task.progress();
        if ct > 0 {
            let s = format!("{cd}/{ct}");
            let g = ui.painter().layout_no_wrap(s, theme::mono(12.0), if cd == ct { p.positive } else { p.muted });
            let w = g.size().x;
            ui.painter().galley(pos2(mx - w, cy - g.size().y / 2.0), g, p.muted);
        }

        // Context menu with the quick actions.
        let id = task.id.clone();
        let today = self.today;
        resp.context_menu(|ui| {
            ui.set_min_width(190.0);
            ui.menu_button(t.menu_status, |ui| {
                for s in std::iter::once(None).chain(Status::ALL.iter().copied().map(Some)) {
                    let name = status_name(t, s);
                    if ui.selectable_label(task.status == s, name).clicked() {
                        act = Some(RowAct::Status(id.clone(), s));
                        ui.close();
                    }
                }
            });
            ui.menu_button(t.menu_priority, |ui| {
                for pr in std::iter::once(None).chain(Priority::ALL.iter().copied().map(Some)) {
                    let name = match pr {
                        None => t.priority_none,
                        Some(Priority::Low) => t.prio_low,
                        Some(Priority::Medium) => t.prio_medium,
                        Some(Priority::High) => t.prio_high,
                        Some(Priority::Urgent) => t.prio_urgent,
                    };
                    if ui.selectable_label(task.priority == pr, name).clicked() {
                        act = Some(RowAct::Prio(id.clone(), pr));
                        ui.close();
                    }
                }
            });
            for (deadline, label) in [(true, t.menu_deadline), (false, t.menu_schedule)] {
                ui.menu_button(label, |ui| {
                    let mk = |d: Option<NaiveDate>| if deadline { RowAct::Deadline(id.clone(), d) } else { RowAct::Sched(id.clone(), d) };
                    if ui.button(t.today).clicked() {
                        act = Some(mk(Some(today)));
                        ui.close();
                    }
                    if ui.button(t.tomorrow).clicked() {
                        act = Some(mk(Some(today + Duration::days(1))));
                        ui.close();
                    }
                    if ui.button(t.next_week).clicked() {
                        act = Some(mk(Some(next_week_start(today, self.week_start()))));
                        ui.close();
                    }
                    if ui.button(t.pick_date).clicked() {
                        let pos = ui.ctx().input(|i| i.pointer.latest_pos()).unwrap_or(pos2(300.0, 300.0));
                        act = Some(RowAct::Pick(id.clone(), deadline, pos));
                        ui.close();
                    }
                    if ui.button(t.clear).clicked() {
                        act = Some(mk(None));
                        ui.close();
                    }
                });
            }
            // A subtask lives where its parent lives, so only a top-level task can be moved.
            if !sub {
                ui.menu_button(t.move_to, |ui| {
                    for profile in Profile::ALL {
                        let pname = self.profile_name(profile);
                        if ui.button(format!("{pname} · {}", t.inbox)).clicked() {
                            act = Some(RowAct::MoveTo(id.clone(), profile, None));
                            ui.close();
                        }
                        for pr in self.data.projects.iter().filter(|x| x.profile == profile && !x.archived) {
                            if ui.button(format!("{pname} · {}", pr.name)).clicked() {
                                act = Some(RowAct::MoveTo(id.clone(), profile, Some(pr.id.clone())));
                                ui.close();
                            }
                        }
                    }
                });
                if ui.button(t.add_subtask).clicked() {
                    act = Some(RowAct::AddSub(id.clone()));
                    ui.close();
                }
                let blocked = if task.recurrence.is_some() {
                    Some(t.no_subtask_recurring)
                } else if self.data.has_subtasks(&task.id) {
                    Some(t.no_subtask_has_subtasks)
                } else {
                    None
                };
                let r = ui.add_enabled(blocked.is_none(), egui::Button::new(t.make_subtask));
                if let Some(why) = blocked {
                    r.on_disabled_hover_text(why);
                } else if r.clicked() {
                    act = Some(RowAct::MakeSub(id.clone()));
                    ui.close();
                }
            } else if ui.button(t.convert_to_task).clicked() {
                act = Some(RowAct::ToTask(id.clone()));
                ui.close();
            }
            ui.separator();
            if let Some(pv) = &prev {
                if ui.button(t.move_up).clicked() {
                    act = Some(RowAct::Swap(id.clone(), pv.clone()));
                    ui.close();
                }
            }
            if let Some(nx) = &next {
                if ui.button(t.move_down).clicked() {
                    act = Some(RowAct::Swap(id.clone(), nx.clone()));
                    ui.close();
                }
            }
            if ui.button(t.duplicate).clicked() {
                act = Some(RowAct::Dup(id.clone()));
                ui.close();
            }
            if ui.button(RichText::new(t.delete).color(p.negative)).clicked() {
                act = Some(RowAct::Del(id.clone()));
                ui.close();
            }
        });
        ui.add_space(if sub { 3.0 } else { 4.0 });
        act
    }
}

pub fn status_name(t: &crate::strings::S, s: Option<Status>) -> &'static str {
    match s {
        None => t.status_none,
        Some(Status::Todo) => t.status_todo,
        Some(Status::InProgress) => t.status_progress,
        Some(Status::Done) => t.status_done,
        Some(Status::Cancelled) => t.status_cancelled,
    }
}

struct Cols {
    prio_x: f32,
    deadline_x: f32,
    sched_x: f32,
    show_prio_text: bool,
}

/// Column offsets measured from the right edge of the row.
fn columns(width: f32) -> Cols {
    let show_prio_text = width > 760.0;
    let prio_w = if show_prio_text { 78.0 } else { 20.0 };
    // Reserve room on the far right for the checklist, recurrence and link marks.
    let marks = 110.0;
    let prio_x = marks + prio_w;
    let deadline_x = prio_x + 96.0;
    let sched_x = deadline_x + 142.0;
    Cols { prio_x, deadline_x, sched_x, show_prio_text }
}
