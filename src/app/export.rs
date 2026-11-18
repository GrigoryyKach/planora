//! "Export PDF": a plain list of task titles for a project, the inbox or a single day.

use super::dialogs::Dialog;
use super::list_view::Scope;
use super::App;
use crate::fmt;
use crate::model::calendar::{items_in_range, ItemKind};
use crate::model::ProfileFilter;
use crate::pdf;
use crate::theme::{self, Btn, Pal};
use chrono::NaiveDate;
use eframe::egui::{self, Align, Layout, RichText, Ui};
use std::path::PathBuf;

#[derive(Clone)]
pub enum ExportTarget {
    Scope(Scope),
    Day(NaiveDate),
}

impl App {
    fn filter_name(&self) -> &'static str {
        let t = self.s();
        match self.cfg.filter {
            ProfileFilter::All => t.all,
            ProfileFilter::Personal => t.personal,
            ProfileFilter::Work => t.work,
        }
    }

    /// Builds the document for a target, respecting the active filters. Also returns a file name stem.
    pub(super) fn pdf_doc(&self, target: &ExportTarget) -> (pdf::Doc, String) {
        let t = self.s();
        let (title, subtitle, rows, file) = match target {
            ExportTarget::Scope(scope) => {
                let (title, subtitle) = match scope {
                    Scope::Project(id) => {
                        let pr = self.data.project(id);
                        (pr.map(|x| x.name.clone()).unwrap_or_default(), pr.map(|x| self.profile_name(x.profile).to_string()).unwrap_or_default())
                    }
                    Scope::Inbox(p) => (t.inbox.to_string(), self.profile_name(*p).to_string()),
                };
                let rows: Vec<pdf::Row> = self
                    .list_rows(scope)
                    .into_iter()
                    .map(|x| pdf::Row {
                        done: x.is_closed(),
                        time: x.scheduled.and(x.time).map(|tm| fmt::time_text(tm, self.h24())),
                        title: x.title.clone(),
                    })
                    .collect();
                let file = match scope {
                    Scope::Inbox(p) => format!("{} - {}", t.inbox, self.profile_name(*p)),
                    Scope::Project(_) => title.clone(),
                };
                (title, subtitle, rows, file)
            }
            ExportTarget::Day(d) => {
                let items = items_in_range(&self.data.tasks, *d, *d, self.cfg.filter, self.cfg.hide_done, self.today).remove(d).unwrap_or_default();
                let rows = items
                    .iter()
                    .filter_map(|i| {
                        let task = self.data.task(&i.task_id)?;
                        let title = if i.kind == ItemKind::Deadline { format!("{} ({})", task.title, t.pdf_deadline) } else { task.title.clone() };
                        Some(pdf::Row { done: i.done, time: i.time.map(|tm| fmt::time_text(tm, self.h24())), title })
                    })
                    .collect();
                let sub = t.pdf_profile.replace("{p}", self.filter_name());
                (fmt::date_long(self.lang, *d), sub, rows, d.format("%Y-%m-%d").to_string())
            }
        };
        let doc = pdf::Doc { title, subtitle, rows, page_label: t.pdf_page.to_string(), empty_label: t.no_tasks.to_string() };
        (doc, file)
    }

    pub(super) fn export_pdf(&mut self, ctx: &egui::Context, target: ExportTarget) {
        let t = self.s();
        let (doc, file) = self.pdf_doc(&target);
        let name = format!("Planora - {}.pdf", pdf::sanitize_file_name(&file));
        let Some(path) = rfd::FileDialog::new().set_file_name(name).add_filter("PDF", &["pdf"]).save_file() else { return };
        match pdf::render(&doc).and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string())) {
            Ok(()) => self.dialog = Some(Dialog::PdfSaved(path)),
            Err(_) => self.notify(ctx, t.pdf_failed.to_string()),
        }
    }

    pub(super) fn pdf_saved_dialog(&mut self, ctx: &egui::Context, p: &Pal, path: PathBuf) -> Option<Dialog> {
        let t = self.s();
        let mut close = false;
        let backdrop = super::dialogs::modal(ctx, p, "pdf-saved", 480.0, |ui: &mut Ui| {
            ui.label(RichText::new(t.pdf_saved).font(theme::bold(18.0)).color(p.text));
            ui.add_space(4.0);
            ui.add(egui::Label::new(RichText::new(path.display().to_string()).font(theme::reg(12.5)).color(p.muted)).wrap());
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if super::dialogs::primary(ui, self, t.open, 110.0, true, p) {
                    let _ = std::process::Command::new(if cfg!(windows) { "explorer.exe" } else { "xdg-open" }).arg(&path).spawn();
                    close = true;
                }
                if theme::button(ui, t.show_in_folder, Btn::Ghost, p).clicked() {
                    #[cfg(windows)]
                    let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{}", path.display())).spawn();
                    // No portable "select this file" on Linux: open the folder that holds it.
                    #[cfg(not(windows))]
                    if let Some(dir) = path.parent() {
                        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
                    }
                    close = true;
                }
                if theme::button(ui, t.close, Btn::Ghost, p).clicked() {
                    close = true;
                }
            });
        });
        if close || backdrop {
            return None;
        }
        Some(Dialog::PdfSaved(path))
    }
}
