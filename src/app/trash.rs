//! The Trash view, the delete confirmations and the subtask dialogs ("complete the subtasks too?", "make subtask of…").

use super::calendar_view::fit_text;
use super::dialogs::{destructive_footer, footer, modal, Dialog};
use super::widgets::{self, Icon};
use super::App;
use crate::fmt;
use crate::i18n;
use crate::model::{Data, TrashEntry};
use crate::theme::{self, Btn, Pal};
use eframe::egui::{self, Align, CursorIcon, Key, Layout, Modifiers, Rect, RichText, ScrollArea, Sense, Ui, pos2, vec2};

/// Enter on a destructive confirmation means Cancel, never the destructive action.
fn enter_pressed(ctx: &egui::Context) -> bool {
    ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter))
}

/// A one-line title inside a modal, cut with an ellipsis when it is too long.
fn title_line(ui: &mut Ui, text: &str, p: &Pal) {
    ui.add(egui::Label::new(RichText::new(text).font(theme::medium(15.0)).color(p.text)).truncate());
}

impl App {
    // ───────────────────────── confirmations ─────────────────────────

    pub(super) fn confirm_delete_task(&mut self, ctx: &egui::Context, p: &Pal, id: String) -> Option<Dialog> {
        let t = self.s();
        let task = self.data.task(&id).cloned()?;
        let subs = self.data.subtasks_of(&id).len();
        let days = self.cfg.trash_days;
        let (mut cancel, mut confirm) = (enter_pressed(ctx), false);
        let backdrop = modal(ctx, p, "del-task", 440.0, |ui: &mut Ui| {
            let q = if task.parent_id.is_some() { t.delete_subtask_q } else { t.delete_task_q };
            ui.label(RichText::new(q).font(theme::medium(18.0)).color(p.text));
            ui.add_space(10.0);
            title_line(ui, &task.title, p);
            if subs > 0 {
                ui.add_space(2.0);
                theme::hint(ui, &i18n::plural(self.lang, subs, &t.subtasks_plural), p);
            }
            ui.add_space(8.0);
            theme::hint(ui, &t.trash_note.replace("{n}", &days.to_string()), p);
            ui.add_space(24.0);
            let f = destructive_footer(ui, p, t.cancel, t.delete);
            cancel |= f.cancel;
            confirm |= f.confirm;
        });
        if confirm {
            self.delete_task(ctx, &id);
            return None;
        }
        if cancel || backdrop {
            return None;
        }
        Some(Dialog::ConfirmDeleteTask { id })
    }

    pub(super) fn confirm_complete(&mut self, ctx: &egui::Context, p: &Pal, id: String) -> Option<Dialog> {
        let t = self.s();
        let task = self.data.task(&id).cloned()?;
        let open = self.data.open_subtask_count(&id);
        if open == 0 {
            return None;
        }
        let (mut close, mut only, mut all) = (false, false, false);
        let backdrop = modal(ctx, p, "complete-parent", 460.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.complete_parent_q).font(theme::medium(18.0)).color(p.text));
            ui.add_space(10.0);
            title_line(ui, &task.title, p);
            ui.add_space(2.0);
            theme::hint(ui, &i18n::plural(self.lang, open, &t.open_subtasks_plural), p);
            ui.add_space(24.0);
            let f = footer(ui, self, p, None, &[t.cancel, t.only_the_task], t.complete_all, true);
            all |= f.primary;
            close |= f.secondary == Some(0);
            only |= f.secondary == Some(1);
        });
        if all || only {
            let now = self.now;
            if all {
                self.data.complete_with_subtasks(&id, now);
            } else {
                self.data.complete_only(&id, now);
            }
            self.touch_data(ctx);
            return None;
        }
        if close || backdrop {
            return None;
        }
        Some(Dialog::ConfirmComplete { id })
    }

    pub(super) fn confirm_purge(&mut self, ctx: &egui::Context, p: &Pal, id: String) -> Option<Dialog> {
        let t = self.s();
        let entry = self.data.trash.iter().find(|e| e.id == id).cloned()?;
        let (mut cancel, mut confirm) = (enter_pressed(ctx), false);
        let backdrop = modal(ctx, p, "purge", 440.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.delete_forever_q).font(theme::medium(18.0)).color(p.text));
            ui.add_space(10.0);
            title_line(ui, &entry.root().title, p);
            if entry.subtask_count() > 0 {
                ui.add_space(2.0);
                theme::hint(ui, &i18n::plural(self.lang, entry.subtask_count(), &t.subtasks_plural), p);
            }
            ui.add_space(8.0);
            theme::hint(ui, t.cannot_undo, p);
            ui.add_space(24.0);
            let f = destructive_footer(ui, p, t.cancel, t.delete_forever);
            cancel |= f.cancel;
            confirm |= f.confirm;
        });
        if confirm {
            self.data.delete_forever(&id);
            self.touch_data(ctx);
            return None;
        }
        if cancel || backdrop {
            return None;
        }
        Some(Dialog::ConfirmPurge { id })
    }

    pub(super) fn confirm_empty_trash(&mut self, ctx: &egui::Context, p: &Pal) -> Option<Dialog> {
        let t = self.s();
        let n = self.data.trash.len();
        if n == 0 {
            return None;
        }
        let (mut cancel, mut confirm) = (enter_pressed(ctx), false);
        let backdrop = modal(ctx, p, "empty-trash", 440.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.empty_trash_q).font(theme::medium(18.0)).color(p.text));
            ui.add_space(10.0);
            title_line(ui, &i18n::plural(self.lang, n, &t.items_plural), p);
            ui.add_space(8.0);
            theme::hint(ui, t.cannot_undo, p);
            ui.add_space(24.0);
            let f = destructive_footer(ui, p, t.cancel, t.empty_trash);
            cancel |= f.cancel;
            confirm |= f.confirm;
        });
        if confirm {
            self.data.empty_trash();
            self.touch_data(ctx);
            return None;
        }
        if cancel || backdrop {
            return None;
        }
        Some(Dialog::ConfirmEmptyTrash)
    }

    /// "Make subtask of…": a searchable list of the tasks that can be the parent.
    pub(super) fn pick_parent(&mut self, ctx: &egui::Context, p: &Pal, id: String, mut query: String, focus: bool) -> Option<Dialog> {
        let t = self.s();
        self.data.task(&id)?;
        let q = query.trim().to_lowercase();
        let cands: Vec<(String, String)> = self
            .data
            .tasks
            .iter()
            .filter(|c| c.parent_id.is_none() && self.data.can_make_subtask(&id, &c.id).is_ok())
            .filter(|c| q.is_empty() || c.title.to_lowercase().contains(&q))
            .take(60)
            .map(|c| {
                let place = match c.project_id.as_deref().and_then(|pid| self.project_name(pid)) {
                    Some(n) => n.to_string(),
                    None => format!("{} · {}", self.profile_name(c.profile), t.inbox),
                };
                (c.id.clone(), format!("{}\u{1f}{}", c.title, place))
            })
            .collect();
        let mut chosen: Option<String> = None;
        let mut close = false;
        let backdrop = modal(ctx, p, "pick-parent", 480.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.make_subtask).font(theme::medium(18.0)).color(p.text));
            ui.add_space(10.0);
            let r = widgets::input(ui, &mut query, t.pick_parent_hint, ui.available_width());
            if focus {
                r.request_focus();
            }
            ui.add_space(8.0);
            if cands.is_empty() {
                theme::hint(ui, t.no_parent_candidates, p);
            } else {
                ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
                    for (cid, label) in &cands {
                        let (title, place) = label.split_once('\u{1f}').unwrap_or((label, ""));
                        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
                        ui.painter().rect_filled(rect, theme::R_CONTROL as f32, theme::alpha(p.raised, (theme::fade(ui, resp.id.with("hover"), resp.hovered()) * 255.0) as u8));
                        let pg = ui.painter().layout_no_wrap(place.to_string(), theme::reg(12.0), p.muted);
                        let pw = pg.size().x.min(160.0);
                        ui.painter().galley(pos2(rect.right() - 12.0 - pg.size().x, rect.center().y - pg.size().y / 2.0), pg, p.muted);
                        let g = fit_text(ui.painter(), title, theme::reg(14.0), p.text, rect.width() - 36.0 - pw, None);
                        ui.painter().galley(pos2(rect.left() + 12.0, rect.center().y - g.size().y / 2.0), g, p.text);
                        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                            chosen = Some(cid.clone());
                        }
                    }
                });
            }
            ui.add_space(24.0);
            let f = footer(ui, self, p, None, &[], t.cancel, true);
            close |= f.primary;
        });
        if let Some(parent) = chosen {
            self.make_subtask(ctx, &id, &parent);
            return None;
        }
        if close || backdrop {
            return None;
        }
        Some(Dialog::PickParent { id, query, focus: false })
    }

    // ───────────────────────── the Trash view ─────────────────────────

    pub(super) fn trash_view(&mut self, ui: &mut Ui, p: &Pal) {
        let t = self.s();
        let mut restore: Option<String> = None;
        let mut purge: Option<String> = None;
        let mut empty = false;
        let days = self.cfg.trash_days;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(t.trash).font(theme::medium(24.0)).color(p.text));
                theme::hint(ui, &t.trash_hint.replace("{n}", &days.to_string()), p);
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::btn_enabled(ui, t.empty_trash, Btn::Secondary, !self.data.trash.is_empty(), p).clicked() {
                    empty = true;
                }
            });
        });
        ui.add_space(16.0);
        if self.data.trash.is_empty() {
            ui.add_space(60.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(t.trash_empty).font(theme::medium(18.0)).color(p.text));
                ui.add_space(4.0);
                theme::hint(ui, t.trash_empty_hint, p);
            });
        } else {
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for e in self.data.trash_newest_first() {
                    self.trash_row(ui, p, e, &mut restore, &mut purge);
                }
                ui.add_space(8.0);
            });
        }
        if empty {
            self.dialog = Some(Dialog::ConfirmEmptyTrash);
        }
        if let Some(id) = purge {
            self.dialog = Some(Dialog::ConfirmPurge { id });
        }
        if let Some(id) = restore {
            let ctx = ui.ctx().clone();
            self.restore_trashed(&ctx, &id);
        }
    }

    /// Where a deleted task came from: its parent, its project, or the Inbox of its profile.
    fn trash_origin(&self, e: &TrashEntry) -> String {
        let t = self.s();
        if let Some(parent) = &e.parent_title {
            return t.trash_subtask_of.replace("{parent}", parent);
        }
        match &e.project_name {
            Some(name) => name.clone(),
            None => format!("{} · {}", self.profile_name(e.root().profile), t.inbox),
        }
    }

    fn trash_row(&self, ui: &mut Ui, p: &Pal, e: &TrashEntry, restore: &mut Option<String>, purge: &mut Option<String>) {
        let t = self.s();
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::hover());
        let hov = theme::fade(ui, resp.id.with("hover"), resp.hovered());
        ui.painter().rect(rect, theme::R_CONTROL as f32, theme::mix(p.card, p.raised, hov), egui::Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
        let cy = rect.center().y;
        ui.painter().rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 10.0), vec2(3.0, rect.height() - 20.0)), 2.0, self.color_of(e.root().profile));

        // Right side: the buttons, then how long is left.
        let mut rx = rect.right() - 10.0;
        let del = Rect::from_center_size(pos2(rx - 14.0, cy), vec2(28.0, 28.0));
        let dresp = ui.interact(del, ui.scope_id().with(("purge", &e.id)), Sense::click());
        if dresp.hovered() {
            ui.painter().rect_filled(del, theme::R_CONTROL as f32, p.raised_hi);
        }
        widgets::icon(ui.painter(), Icon::Trash, del.center(), 16.0, if dresp.hovered() { p.negative } else { p.muted });
        if dresp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(t.delete_forever).clicked() {
            *purge = Some(e.id.clone());
        }
        rx = del.left() - 8.0;
        let rw = ui.painter().layout_no_wrap(t.restore.to_string(), theme::medium(14.0), p.text).size().x + 24.0;
        let rrect = Rect::from_min_size(pos2(rx - rw, cy - 15.0), vec2(rw, 30.0));
        if theme::button_at(ui, rrect, ui.scope_id().with(("restore", &e.id)), t.restore, Btn::Secondary, p).clicked() {
            *restore = Some(e.id.clone());
        }
        rx = rrect.left() - 16.0;
        let left = Data::days_left(e, self.now, self.cfg.trash_days);
        let when = if left == 0 { t.deletes_soon.to_string() } else { i18n::plural(self.lang, left as usize, &t.deletes_in_plural) };
        let g = ui.painter().layout_no_wrap(when, theme::reg(12.0), p.muted);
        rx -= g.size().x;
        ui.painter().galley(pos2(rx, cy - g.size().y / 2.0), g, p.muted);
        rx -= 16.0;

        // Left side: the title, then where it came from, when it was deleted and how many subtasks went with it.
        let x = rect.left() + 18.0;
        let g = fit_text(ui.painter(), &e.root().title, theme::reg(14.0), p.text, rx - x, None);
        ui.painter().galley(pos2(x, cy - 18.0), g, p.text);
        let mut day = super::today_view::date_label(self, e.deleted_at.date());
        // "Deleted yesterday 19:43": the relative words read better in lower case inside a sentence.
        if (self.today - e.deleted_at.date()).num_days().abs() <= 1 {
            day = day.chars().next().map(|c| c.to_lowercase().collect::<String>() + &day[c.len_utf8()..]).unwrap_or(day);
        }
        let mut sub = format!("{} · {}", self.trash_origin(e), t.deleted_when.replace("{when}", &format!("{day} {}", fmt::time_text(e.deleted_at.time(), self.h24()))));
        if e.subtask_count() > 0 {
            sub = format!("{sub} · {}", i18n::plural(self.lang, e.subtask_count(), &t.subtasks_plural));
        }
        let g = fit_text(ui.painter(), &sub, theme::reg(12.0), p.muted, rx - x, None);
        ui.painter().galley(pos2(x, cy + 2.0), g, p.muted);
        ui.add_space(4.0);
    }
}
