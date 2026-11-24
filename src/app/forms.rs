//! Dialog dispatch and the form dialogs: quick add, search, project, delete project.

use super::dialogs::{color_swatches, footer, modal, Dialog, ProjectForm};
use super::widgets::{self, Icon};
use super::{App, DateTarget};
use crate::config::Nav;
use crate::fmt;
use crate::model::Profile;
use crate::theme::{self, Btn, Pal};
use chrono::NaiveDate;
use eframe::egui::{self, CursorIcon, Rect, RichText, ScrollArea, Sense, Ui, pos2, vec2};

impl App {
    pub(super) fn dialog_ui(&mut self, ctx: &egui::Context, p: &Pal) {
        let Some(d) = self.dialog.take() else { return };
        let next = match d {
            Dialog::QuickAdd { title, profile, project, date, focus } => self.quick_add(ctx, p, title, profile, project, date, focus),
            Dialog::Search { query, focus } => self.search(ctx, p, query, focus),
            Dialog::Project(f) => self.project_dialog(ctx, p, f),
            Dialog::ConfirmDeleteProject { id, move_to_inbox } => self.confirm_delete_project(ctx, p, id, move_to_inbox),
            Dialog::ConfirmDeleteTask { id } => self.confirm_delete_task(ctx, p, id),
            Dialog::ConfirmComplete { id } => self.confirm_complete(ctx, p, id),
            Dialog::ConfirmPurge { id } => self.confirm_purge(ctx, p, id),
            Dialog::ConfirmEmptyTrash => self.confirm_empty_trash(ctx, p),
            Dialog::PickParent { id, query, focus } => self.pick_parent(ctx, p, id, query, focus),
            Dialog::ConfirmImport(data) => self.confirm_replace(ctx, p, *data, None),
            Dialog::ConfirmRestore(path) => self.confirm_restore(ctx, p, path),
            Dialog::Missed(list) => self.missed_dialog(ctx, p, list),
            Dialog::Corrupt(path) => self.corrupt_dialog(ctx, p, path),
            Dialog::PdfSaved(path) => self.pdf_saved_dialog(ctx, p, path),
        };
        // An action inside may already have opened another dialog; that one wins.
        if self.dialog.is_none() {
            self.dialog = next;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn quick_add(
        &mut self,
        ctx: &egui::Context,
        p: &Pal,
        mut title: String,
        mut profile: Profile,
        mut project: Option<String>,
        date: Option<NaiveDate>,
        focus: bool,
    ) -> Option<Dialog> {
        let t = self.s();
        let (mut close, mut submit) = (false, false);
        let mut pick: Option<Rect> = None;
        let mut date = date;
        let backdrop = modal(ctx, p, "quick-add", 440.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.new_task).font(theme::medium(18.0)).color(p.text));
            ui.add_space(16.0);
            let r = widgets::input(ui, &mut title, t.title_hint, ui.available_width());
            if focus {
                r.request_focus();
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let colors = [Some(self.color_of(Profile::Personal)), Some(self.color_of(Profile::Work))];
                let sel = if profile == Profile::Personal { 0 } else { 1 };
                if let Some(i) = widgets::segmented(ui, "qa-profile", &[t.personal, t.work], sel, &colors, p) {
                    profile = Profile::ALL[i];
                    if project.as_deref().and_then(|id| self.data.project(id)).is_some_and(|pr| pr.profile != profile) {
                        project = None;
                    }
                }
                let current = project.as_deref().and_then(|id| self.project_name(id)).unwrap_or(t.inbox).to_string();
                widgets::combo("qa-project", &current, ui.available_width())
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(project.is_none(), t.inbox).clicked() {
                            project = None;
                        }
                        for pr in self.data.projects.iter().filter(|x| x.profile == profile && !x.archived) {
                            if ui.selectable_label(project.as_deref() == Some(&pr.id), &pr.name).clicked() {
                                project = Some(pr.id.clone());
                            }
                        }
                    });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t.date_label).font(theme::reg(14.0)).color(p.text2));
                let text = date.map(|d| fmt::date_full(self.lang, d)).unwrap_or_else(|| t.set_date.to_string());
                let resp = widgets::btn(ui, &text, Btn::Secondary, p);
                if resp.clicked() {
                    pick = Some(resp.rect);
                }
                if date.is_some() && widgets::icon_button(ui, Icon::Close, 26.0, p).clicked() {
                    date = None;
                }
            });
            ui.add_space(24.0);
            let f = footer(ui, self, p, None, &[t.cancel], t.add, !title.trim().is_empty());
            submit |= f.primary;
            close |= f.secondary.is_some();
        });
        if let Some(r) = pick {
            // Keep the typed text while the picker is open.
            self.dialog = Some(Dialog::QuickAdd { title: title.clone(), profile, project: project.clone(), date, focus: false });
            self.open_picker(DateTarget::QuickAdd, super::below(r), date);
            return None;
        }
        if submit && !title.trim().is_empty() {
            let pid = project.clone();
            self.add_task(ctx, profile, project, &title, date);
            if date.is_none() && matches!(self.cfg.nav, Nav::Today | Nav::Calendar | Nav::Settings) {
                // A task without a date only shows up in lists; tell the user where it went.
                let place = match pid.as_deref().and_then(|id| self.project_name(id)) {
                    Some(n) => n.to_string(),
                    None => format!("{} · {}", self.profile_name(profile), t.inbox),
                };
                self.notify(ctx, t.added_to.replace("{where}", &place));
            }
            return None;
        }
        if close || backdrop {
            return None;
        }
        Some(Dialog::QuickAdd { title, profile, project, date, focus: false })
    }

    fn search(&mut self, ctx: &egui::Context, p: &Pal, mut query: String, focus: bool) -> Option<Dialog> {
        let t = self.s();
        let mut open: Option<String> = None;
        let q = query.trim().to_lowercase();
        let mut hits: Vec<&crate::model::Task> = self.data.search(&query);
        hits.sort_by_key(|x| x.is_closed());
        let total = hits.len();
        hits.truncate(40);
        let mut enter = false;
        let backdrop = modal(ctx, p, "search", 560.0, |ui: &mut Ui| {
            let start = ui.cursor().min;
            ui.horizontal(|ui| {
                ui.add_space(26.0);
                let r = widgets::input(ui, &mut query, t.search_hint, ui.available_width());
                widgets::icon(ui.painter(), Icon::Search, pos2(start.x + 12.0, r.rect.center().y), 17.0, p.muted);
                if focus {
                    r.request_focus();
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    enter = true;
                }
            });
            ui.add_space(8.0);
            if q.is_empty() {
                theme::hint(ui, t.search_prompt, p);
            } else if hits.is_empty() {
                theme::hint(ui, t.search_none, p);
            } else {
                // Compact with few results, grows with them up to ~60% of the window, then scrolls.
                let max_h = (ctx.content_rect().height() * 0.6 - 120.0).max(160.0);
                let h = (hits.len() as f32 * 42.0 + 4.0).min(max_h);
                ScrollArea::vertical().max_height(h).min_scrolled_height(h).auto_shrink([false, true]).show(ui, |ui| {
                    for task in &hits {
                        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                        ui.painter().rect_filled(rect, theme::R_CONTROL as f32, theme::alpha(p.raised, (theme::fade(ui, resp.id.with("hover"), resp.hovered()) * 255.0) as u8));
                        widgets::status_mark(ui.painter(), pos2(rect.left() + 18.0, rect.center().y), task.status, task.is_closed(), p);
                        ui.painter().circle_filled(pos2(rect.right() - 14.0, rect.center().y), 4.0, self.color_of(task.profile));
                        let name = task.project_id.as_deref().and_then(|id| self.project_name(id)).unwrap_or("");
                        let mut rx = rect.right() - 30.0;
                        if !name.is_empty() {
                            let g = ui.painter().layout_no_wrap(name.to_string(), theme::reg(12.0), p.muted);
                            rx -= g.size().x;
                            ui.painter().galley(pos2(rx, rect.center().y - g.size().y / 2.0), g, p.muted);
                            rx -= 12.0;
                        }
                        let col = if task.is_closed() { p.muted } else { p.text };
                        let g = super::calendar_view::fit_text(ui.painter(), &self.display_title(task), theme::reg(14.0), col, rx - rect.left() - 44.0, None);
                        ui.painter().galley(pos2(rect.left() + 38.0, rect.center().y - g.size().y / 2.0), g, col);
                        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            open = Some(task.id.clone());
                        }
                    }
                });
                if total > hits.len() {
                    theme::hint(ui, &t.more.replace("{n}", &(total - hits.len()).to_string()), p);
                }
            }
        });
        if enter && open.is_none() {
            open = hits.first().map(|x| x.id.clone());
        }
        if let Some(id) = open {
            self.open_editor(&id);
            return None;
        }
        if backdrop {
            return None;
        }
        Some(Dialog::Search { query, focus: false })
    }

    fn project_dialog(&mut self, ctx: &egui::Context, p: &Pal, mut f: ProjectForm) -> Option<Dialog> {
        let t = self.s();
        let (mut close, mut save, mut delete) = (false, false, false);
        let editing = f.id.is_some();
        let backdrop = modal(ctx, p, "project", 440.0, |ui: &mut Ui| {
            ui.label(RichText::new(if editing { t.edit_project } else { t.new_project }).font(theme::medium(18.0)).color(p.text));
            ui.add_space(10.0);
            if !editing {
                let colors = [Some(self.color_of(Profile::Personal)), Some(self.color_of(Profile::Work))];
                let sel = if f.profile == Profile::Personal { 0 } else { 1 };
                if let Some(i) = widgets::segmented(ui, "pf-profile", &[t.personal, t.work], sel, &colors, p) {
                    f.profile = Profile::ALL[i];
                }
                ui.add_space(8.0);
            }
            let r = widgets::input(ui, &mut f.name, t.project_name_hint, ui.available_width());
            if f.focus {
                r.request_focus();
            }
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                save = true;
            }
            ui.add_space(6.0);
            widgets::input(ui, &mut f.description, t.project_desc_hint, ui.available_width());
            ui.add_space(20.0);
            theme::section(ui, t.color, p);
            ui.add_space(8.0);
            color_swatches(ui, &mut f.color, p);
            if editing {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    theme::switch(ui, &mut f.archived, p);
                    ui.label(RichText::new(t.archived_label).font(theme::reg(14.0)).color(p.text));
                });
            }
            ui.add_space(24.0);
            let out = footer(ui, self, p, editing.then_some(t.delete_project), &[t.cancel], t.save, !f.name.trim().is_empty());
            delete |= out.left;
            save |= out.primary;
            close |= out.secondary.is_some();
        });
        if delete {
            if let Some(id) = f.id.clone() {
                self.dialog = Some(Dialog::ConfirmDeleteProject { id, move_to_inbox: true });
            }
            return None;
        }
        if save && !f.name.trim().is_empty() {
            match &f.id {
                Some(id) => {
                    if let Some(pr) = self.data.projects.iter_mut().find(|x| x.id == *id) {
                        pr.name = f.name.trim().to_string();
                        pr.description = f.description.trim().to_string();
                        pr.color = f.color;
                        pr.archived = f.archived;
                    }
                    if f.archived && self.cfg.nav == Nav::Project(id.clone()) {
                        self.go(ctx, Nav::Today);
                    }
                    self.touch_data(ctx);
                }
                None => {
                    let id = self.new_project(ctx, f.profile, &f.name, &f.description, f.color);
                    self.go(ctx, Nav::Project(id));
                }
            }
            return None;
        }
        if close || backdrop {
            return None;
        }
        f.focus = false;
        Some(Dialog::Project(f))
    }

    fn confirm_delete_project(&mut self, ctx: &egui::Context, p: &Pal, id: String, mut to_inbox: bool) -> Option<Dialog> {
        let t = self.s();
        let pr = self.data.project(&id).cloned()?;
        let count = self.data.tasks.iter().filter(|x| x.project_id.as_deref() == Some(&id)).count();
        let days = self.cfg.trash_days;
        let (mut close, mut confirm) = (false, false);
        let backdrop = modal(ctx, p, "del-project", 440.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.delete_project_q.replace("{name}", &pr.name)).font(theme::medium(18.0)).color(p.text));
            ui.add_space(6.0);
            theme::hint(ui, &crate::i18n::plural(self.lang, count, &t.tasks_plural), p);
            if count > 0 && !to_inbox {
                ui.add_space(2.0);
                theme::hint(ui, &t.trash_note_many.replace("{n}", &days.to_string()), p);
            }
            if count > 0 {
                ui.add_space(10.0);
                if ui.radio(to_inbox, RichText::new(t.move_to_inbox).font(theme::reg(14.0))).clicked() {
                    to_inbox = true;
                }
                if ui.radio(!to_inbox, RichText::new(t.delete_tasks_too).font(theme::reg(14.0))).clicked() {
                    to_inbox = false;
                }
            }
            ui.add_space(24.0);
            let f = footer(ui, self, p, None, &[t.cancel], t.delete, true);
            confirm |= f.primary;
            close |= f.secondary.is_some();
        });
        if confirm {
            if to_inbox {
                for task in self.data.tasks.iter_mut().filter(|x| x.project_id.as_deref() == Some(&id)) {
                    task.project_id = None;
                }
            } else {
                // The tasks go to the Trash like any deleted task (their project is gone, so a restore puts them in the Inbox).
                self.data.trash_project_tasks(&id, self.now);
            }
            self.data.projects.retain(|x| x.id != id);
            if self.cfg.nav == Nav::Project(id.clone()) {
                self.go(ctx, Nav::Inbox(pr.profile));
            }
            if self.editor.as_ref().is_some_and(|e| self.data.task(&e.id).is_none()) {
                self.editor = None;
            }
            self.touch_data(ctx);
            return None;
        }
        if close || backdrop {
            return None;
        }
        Some(Dialog::ConfirmDeleteProject { id, move_to_inbox: to_inbox })
    }
}
