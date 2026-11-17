//! The task editor side panel. Changes are applied to the task immediately (autosave is debounced).

use super::list_view::status_name;
use super::widgets::{self, Icon};
use super::{below, App, DateTarget};
use crate::fmt;
use crate::model::{CheckItem, Link, Priority, Profile, RecKind, Recurrence, Remind, Status};
use crate::theme::{self, Btn, Pal};
use chrono::NaiveTime;
use eframe::egui::{self, Align, Align2, Color32, CursorIcon, Layout, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

pub struct Editor {
    pub id: String,
    time_buf: String,
    time_synced: Option<NaiveTime>,
    time_init: bool,
    new_link: String,
    new_check: String,
    focus_title: bool,
}

impl Editor {
    pub fn new(id: String) -> Self {
        Editor { id, time_buf: String::new(), time_synced: None, time_init: false, new_link: String::new(), new_check: String::new(), focus_title: true }
    }
}

/// Adds `https://` when the user typed a bare address.
pub fn normalize_url(s: &str) -> String {
    let s = s.trim();
    if s.contains("://") || s.starts_with("mailto:") { s.to_string() } else { format!("https://{s}") }
}

/// A usable link: a scheme and a host with a dot (or localhost), no spaces.
pub fn valid_url(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.contains(char::is_whitespace) {
        return false;
    }
    if s.starts_with("mailto:") {
        return s.contains('@');
    }
    let n = normalize_url(s);
    let Some((scheme, rest)) = n.split_once("://") else { return false };
    if !matches!(scheme, "http" | "https" | "ftp") {
        return false;
    }
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    !host.is_empty() && (host.contains('.') || host.starts_with("localhost"))
}

impl App {
    pub(super) fn editor_panel(&mut self, ui: &mut Ui, p: &Pal) {
        let ctx = ui.ctx().clone();
        let t = self.s();
        let Some(mut ed) = self.editor.take() else { return };
        let Some(orig) = self.data.task(&ed.id).cloned() else { return };
        let mut task = orig.clone();
        let mut close = false;
        let mut delete = false;

        if !ed.time_init || ed.time_synced != task.time {
            ed.time_buf = task.time.map(|x| fmt::time_text(x, self.h24())).unwrap_or_default();
            ed.time_synced = task.time;
            ed.time_init = true;
        }

        ui.horizontal(|ui| {
            theme::caps(ui, t.task, p);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::Close, 30.0, p).on_hover_text(format!("{} (Esc)", t.close)).clicked() {
                    close = true;
                }
                if widgets::icon_button(ui, Icon::Trash, 30.0, p).on_hover_text(t.delete).clicked() {
                    delete = true;
                }
            });
        });
        ui.add_space(6.0);

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(ui.available_width() - 6.0);
            let w = ui.available_width();

            // Title.
            let r = ui.add(
                egui::TextEdit::singleline(&mut task.title)
                    .hint_text(t.title_hint)
                    .desired_width(w)
                    .margin(egui::Margin::symmetric(12, 10))
                    .font(theme::semi(16.0)),
            );
            if ed.focus_title && task.title.is_empty() {
                r.request_focus();
            }
            ed.focus_title = false;
            ui.add_space(8.0);

            // Profile and project.
            ui.horizontal(|ui| {
                let labels = [t.personal, t.work];
                let colors = [Some(self.color_of(Profile::Personal)), Some(self.color_of(Profile::Work))];
                let sel = if task.profile == Profile::Personal { 0 } else { 1 };
                if let Some(i) = widgets::segmented(ui, "ed-profile", &labels, sel, &colors, p) {
                    task.profile = Profile::ALL[i];
                    if task.project_id.as_deref().and_then(|id| self.data.project(id)).is_some_and(|pr| pr.profile != task.profile) {
                        task.project_id = None;
                    }
                }
                let current = task.project_id.as_deref().and_then(|id| self.project_name(id)).unwrap_or(t.inbox).to_string();
                egui::ComboBox::from_id_salt("ed-project").selected_text(RichText::new(current).font(theme::reg(13.5))).width(ui.available_width() - 4.0).show_ui(ui, |ui| {
                    if ui.selectable_label(task.project_id.is_none(), t.inbox).clicked() {
                        task.project_id = None;
                    }
                    for pr in self.data.projects.iter().filter(|x| x.profile == task.profile && !x.archived) {
                        if ui.selectable_label(task.project_id.as_deref() == Some(&pr.id), &pr.name).clicked() {
                            task.project_id = Some(pr.id.clone());
                        }
                    }
                });
            });

            // Status and priority.
            ui.add_space(12.0);
            theme::caps(ui, t.status_label, p);
            ui.add_space(2.0);
            let labels: Vec<&str> = Status::ALL.iter().map(|s| status_name(t, Some(*s))).collect();
            let sel = task.status.and_then(|s| Status::ALL.iter().position(|x| *x == s));
            if let Some(n) = widgets::pills(ui, "ed-status", &labels, &[], sel, true, p) {
                crate::model::calendar::set_status(&mut task, n.map(|i| Status::ALL[i]), self.now);
            }
            ui.add_space(8.0);
            theme::caps(ui, t.priority_label, p);
            ui.add_space(2.0);
            let labels = [t.prio_low, t.prio_medium, t.prio_high, t.prio_urgent];
            let colors: Vec<Option<Color32>> = Priority::ALL.iter().map(|x| Some(widgets::priority_color(*x))).collect();
            let sel = task.priority.and_then(|s| Priority::ALL.iter().position(|x| *x == s));
            if let Some(n) = widgets::pills(ui, "ed-prio", &labels, &colors, sel, true, p) {
                task.priority = n.map(|i| Priority::ALL[i]);
            }

            // Dates.
            ui.add_space(12.0);
            theme::caps(ui, t.when, p);
            ui.add_space(2.0);
            let h24 = self.h24();
            self.date_row(ui, p, t.scheduled_label, &mut task.scheduled, DateTarget::Scheduled(task.id.clone()), |ui, task_time_enabled| {
                if task_time_enabled {
                    let bad = !ed.time_buf.trim().is_empty() && fmt::parse_time(&ed.time_buf).is_none();
                    let hint = if h24 { "HH:MM" } else { "h:MM AM" };
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut ed.time_buf)
                            .hint_text(hint)
                            .desired_width(80.0)
                            .margin(egui::Margin::symmetric(9, 6))
                            .font(theme::reg(13.5)),
                    );
                    if bad {
                        ui.painter().rect_stroke(resp.rect, 5.0, Stroke::new(1.5, widgets::RED), StrokeKind::Outside);
                    }
                    if resp.changed() {
                        if ed.time_buf.trim().is_empty() {
                            task.time = None;
                            ed.time_synced = None;
                        } else if let Some(tm) = fmt::parse_time(&ed.time_buf) {
                            task.time = Some(tm);
                            ed.time_synced = Some(tm);
                        }
                    }
                    if resp.lost_focus() {
                        ed.time_buf = task.time.map(|x| fmt::time_text(x, h24)).unwrap_or_default();
                    }
                }
            });
            self.date_row(ui, p, t.deadline_label, &mut task.deadline, DateTarget::Deadline(task.id.clone()), |_, _| {});

            // Recurrence.
            if task.anchor().is_some() {
                ui.add_space(10.0);
                theme::caps(ui, t.repeat, p);
                ui.add_space(2.0);
                let labels = [t.rec_none, t.rec_daily, t.rec_weekdays, t.rec_weekly, t.rec_monthly, t.rec_days];
                let sel = match task.recurrence {
                    None => 0,
                    Some(r) => 1 + RecKind::ALL.iter().position(|k| *k == r.kind).unwrap_or(0),
                };
                if let Some(Some(i)) = widgets::pills(ui, "ed-rec", &labels, &[], Some(sel), false, p) {
                    task.recurrence = if i == 0 {
                        None
                    } else {
                        let old = task.recurrence;
                        Some(Recurrence { kind: RecKind::ALL[i - 1], interval: old.map_or(2, |o| o.interval.max(1)), until: old.and_then(|o| o.until) })
                    };
                }
                if let Some(mut rec) = task.recurrence {
                    if rec.kind == RecKind::Days {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(t.every).font(theme::reg(13.5)).color(p.text));
                            let mut n = rec.interval as i32;
                            ui.add(egui::DragValue::new(&mut n).range(1..=365).speed(0.2));
                            rec.interval = n.max(1) as u32;
                            ui.label(RichText::new(t.days_word).font(theme::reg(13.5)).color(p.text));
                        });
                    }
                    self.date_row(ui, p, t.until, &mut rec.until, DateTarget::Until(task.id.clone()), |_, _| {});
                    task.recurrence = Some(rec);
                    theme::hint(ui, t.rec_hint, p);
                }
            }

            // Reminder.
            if task.time.is_some() && task.scheduled.is_some() {
                ui.add_space(10.0);
                theme::caps(ui, t.reminder, p);
                ui.add_space(2.0);
                let def = t.remind_default.replace("{n}", &self.cfg.default_remind.to_string());
                let labels = [def.as_str(), t.remind_off, t.remind_at, "5", "10", "15", "30"];
                let mins = ["5", "10", "15", "30"];
                let label_texts: Vec<String> = labels
                    .iter()
                    .map(|l| if mins.contains(l) { t.remind_min.replace("{n}", l) } else { l.to_string() })
                    .collect();
                let refs: Vec<&str> = label_texts.iter().map(|s| s.as_str()).collect();
                let sel = match task.remind {
                    Remind::Default => 0,
                    Remind::Off => 1,
                    Remind::Min(0) => 2,
                    Remind::Min(5) => 3,
                    Remind::Min(10) => 4,
                    Remind::Min(15) => 5,
                    Remind::Min(30) => 6,
                    Remind::Min(_) => 99,
                };
                if let Some(Some(i)) = widgets::pills(ui, "ed-remind", &refs, &[], Some(sel), false, p) {
                    task.remind = match i {
                        0 => Remind::Default,
                        1 => Remind::Off,
                        2 => Remind::Min(0),
                        3 => Remind::Min(5),
                        4 => Remind::Min(10),
                        5 => Remind::Min(15),
                        _ => Remind::Min(30),
                    };
                }
            }

            // Description.
            ui.add_space(12.0);
            theme::caps(ui, t.description, p);
            ui.add_space(2.0);
            ui.add(
                egui::TextEdit::multiline(&mut task.description)
                    .hint_text(t.description_hint)
                    .desired_width(w)
                    .desired_rows(4)
                    .margin(egui::Margin::symmetric(12, 9))
                    .font(theme::reg(14.0)),
            );

            // Links.
            ui.add_space(12.0);
            theme::caps(ui, t.links, p);
            ui.add_space(2.0);
            let mut remove_link = None;
            for (i, l) in task.links.iter_mut().enumerate() {
                let bad = !l.url.trim().is_empty() && !valid_url(&l.url);
                ui.horizontal(|ui| {
                    let r = widgets::input(ui, &mut l.url, "https://", (w - 150.0).max(80.0));
                    if bad {
                        ui.painter().rect_stroke(r.rect, 5.0, Stroke::new(1.5, widgets::RED), StrokeKind::Outside);
                        r.on_hover_text(t.bad_url);
                    }
                    if !bad && !l.url.trim().is_empty() {
                        if theme::button(ui, t.open, Btn::Small, p).clicked() {
                            ctx.open_url(egui::OpenUrl::new_tab(normalize_url(&l.url)));
                        }
                        if theme::button(ui, t.copy, Btn::Small, p).clicked() {
                            ctx.copy_text(normalize_url(&l.url));
                        }
                    }
                    if widgets::icon_button(ui, Icon::Close, 24.0, p).clicked() {
                        remove_link = Some(i);
                    }
                });
                widgets::input(ui, &mut l.title, t.link_title_hint, w - 4.0);
                ui.add_space(4.0);
            }
            if let Some(i) = remove_link {
                task.links.remove(i);
            }
            ui.horizontal(|ui| {
                let r = widgets::input(ui, &mut ed.new_link, t.add_link_hint, w - 70.0);
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (theme::button(ui, t.add, Btn::Soft, p).clicked() || enter) && !ed.new_link.trim().is_empty() {
                    task.links.push(Link { url: std::mem::take(&mut ed.new_link).trim().to_string(), title: String::new() });
                }
            });

            // Checklist.
            ui.add_space(12.0);
            let (cd, ct) = task.progress();
            ui.horizontal(|ui| {
                theme::caps(ui, t.checklist, p);
                if ct > 0 {
                    ui.label(RichText::new(format!("{cd}/{ct}")).font(theme::semi(12.0)).color(p.muted));
                }
            });
            ui.add_space(2.0);
            let mut remove_item = None;
            for (i, c) in task.checklist.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                    let box_r = egui::Rect::from_center_size(r.center(), vec2(17.0, 17.0));
                    if c.done {
                        ui.painter().rect_filled(box_r, 5.0, p.accent);
                        widgets::icon(ui.painter(), Icon::Check, box_r.center(), 12.0, Color32::WHITE);
                    } else {
                        ui.painter().rect_stroke(box_r, 5.0, Stroke::new(1.5, p.muted), StrokeKind::Inside);
                    }
                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                        c.done = !c.done;
                    }
                    let col = if c.done { p.muted } else { p.text };
                    ui.add(
                        egui::TextEdit::singleline(&mut c.text)
                            .desired_width(w - 80.0)
                            .frame(egui::Frame::NONE)
                            .text_color(col)
                            .font(theme::reg(14.0)),
                    );
                    if widgets::icon_button(ui, Icon::Close, 24.0, p).clicked() {
                        remove_item = Some(i);
                    }
                });
            }
            if let Some(i) = remove_item {
                task.checklist.remove(i);
            }
            ui.horizontal(|ui| {
                let r = widgets::input(ui, &mut ed.new_check, t.add_item_hint, w - 70.0);
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (theme::button(ui, t.add, Btn::Soft, p).clicked() || enter) && !ed.new_check.trim().is_empty() {
                    task.checklist.push(CheckItem { text: std::mem::take(&mut ed.new_check).trim().to_string(), done: false });
                    if enter {
                        r.request_focus();
                    }
                }
            });

            ui.add_space(16.0);
            theme::hint(
                ui,
                &t.created_updated
                    .replace("{c}", &format!("{} {}", fmt::date_full(self.lang, task.created.date()), fmt::time_text(task.created.time(), self.h24())))
                    .replace("{u}", &format!("{} {}", fmt::date_full(self.lang, task.updated.date()), fmt::time_text(task.updated.time(), self.h24()))),
                p,
            );
            ui.add_space(8.0);
        });

        if task != orig {
            let id = task.id.clone();
            let now = self.now;
            let new = task;
            self.edit_task(&ctx, &id, move |old| {
                *old = new;
                old.updated = now;
            });
        }
        if delete {
            let id = ed.id.clone();
            self.delete_task(&ctx, &id);
        } else if close {
            self.flush();
        } else {
            self.editor = Some(ed);
        }
    }

    /// A labelled row: label, date button (opens the picker), clear button, and optional extras (time field).
    #[allow(clippy::too_many_arguments)]
    fn date_row(
        &mut self,
        ui: &mut Ui,
        p: &Pal,
        label: &str,
        value: &mut Option<chrono::NaiveDate>,
        target: DateTarget,
        extra: impl FnOnce(&mut Ui, bool),
    ) {
        let t = self.s();
        ui.horizontal(|ui| {
            ui.add_sized(vec2(84.0, 28.0), egui::Label::new(RichText::new(label).font(theme::reg(13.5)).color(p.muted)));
            let text = match value {
                Some(d) => fmt::date_full(self.lang, *d),
                None => t.set_date.to_string(),
            };
            let resp = theme::button(ui, &text, if value.is_some() { Btn::Soft } else { Btn::Ghost }, p);
            if resp.clicked() {
                self.open_picker(target.clone(), below(resp.rect), *value);
            }
            let is_sched = matches!(target, DateTarget::Scheduled(_));
            if value.is_some() {
                if is_sched {
                    extra(ui, true);
                }
                if widgets::icon_button(ui, Icon::Close, 26.0, p).on_hover_text(t.clear).clicked() {
                    *value = None;
                }
            }
        });
        ui.add_space(2.0);
        let _ = Align2::CENTER_CENTER;
        let _ = pos2(0.0, 0.0);
    }
}
