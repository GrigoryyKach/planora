//! Modal dialogs, the day panel, the date picker popup and the undo toast.

use super::widgets::{self, Icon};
use super::{id_of, App, PALETTE};
use crate::config::Config;
use crate::model::reminders::Reminder;
use crate::model::{Data, Profile, Project};
use crate::theme::{self, Pal};
use chrono::NaiveDate;
use eframe::egui::{self, Align2, Color32, CursorIcon, Frame, Margin, Order, Sense, Stroke, Ui, vec2};
use std::path::PathBuf;

pub struct ProjectForm {
    pub id: Option<String>,
    pub profile: Profile,
    pub name: String,
    pub description: String,
    pub color: u32,
    pub archived: bool,
    pub focus: bool,
}

pub enum Dialog {
    QuickAdd { title: String, profile: Profile, project: Option<String>, date: Option<NaiveDate>, focus: bool },
    Search { query: String, focus: bool },
    Project(ProjectForm),
    ConfirmDeleteProject { id: String, move_to_inbox: bool },
    ConfirmImport(Box<Data>),
    ConfirmRestore(PathBuf),
    Missed(Vec<Reminder>),
    Corrupt(PathBuf),
    PdfSaved(PathBuf),
}

impl Dialog {
    pub fn project_new(profile: Profile, _cfg: &Config) -> Dialog {
        Dialog::Project(ProjectForm {
            id: None,
            profile,
            name: String::new(),
            description: String::new(),
            color: if profile == Profile::Personal { PALETTE[0] } else { PALETTE[4] },
            archived: false,
            focus: true,
        })
    }

    pub fn project_edit(p: &Project) -> Dialog {
        Dialog::Project(ProjectForm {
            id: Some(p.id.clone()),
            profile: p.profile,
            name: p.name.clone(),
            description: p.description.clone(),
            color: p.color,
            archived: p.archived,
            focus: true,
        })
    }
}

/// Dimmed backdrop plus a centred card. Returns `true` when the backdrop was clicked.
pub(super) fn modal(ctx: &egui::Context, p: &Pal, name: &str, width: f32, add: impl FnOnce(&mut Ui)) -> bool {
    // egui keeps each Area's layer position by id, and clicking the backdrop raises it above everything
    // that is shown later under the same id. A generation number that changes on every opening gives the
    // backdrop and the card fresh ids, so a reopened modal always has the card on top of the backdrop.
    let name = &format!("{name}-{}", super::modal_generation(ctx));
    let screen = ctx.content_rect();
    let dim = egui::Area::new(id_of(&format!("{name}-dim"))).order(Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let r = ui.allocate_rect(screen, Sense::click());
        ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(if p.dark { 150 } else { 90 }));
        r
    });
    egui::Area::new(id_of(name)).order(Order::Foreground).anchor(Align2::CENTER_CENTER, vec2(0.0, -10.0)).show(ctx, |ui| {
        Frame::new()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(18)
            .inner_margin(Margin::symmetric(24, 20))
            .shadow(theme::shadow(16, 48, Color32::from_black_alpha(90)))
            .show(ui, |ui| {
                ui.set_width(width);
                add(ui);
            });
    });
    dim.inner.clicked()
}

pub(super) fn primary(ui: &mut Ui, app: &App, label: &str, w: f32, enabled: bool, p: &Pal) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 38.0), Sense::hover());
    theme::gradient_button(ui, rect, ui.scope_id().with(("primary", label)), label, None, enabled, &app.tex, p).clicked() && enabled
}


pub(super) fn color_swatches(ui: &mut Ui, color: &mut u32, p: &Pal) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for c in PALETTE {
            let (r, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
            ui.painter().circle_filled(r.center(), 11.0, widgets::rgb(c));
            if *color == c {
                ui.painter().circle_stroke(r.center(), 13.5, Stroke::new(2.0, p.text));
                widgets::icon(ui.painter(), Icon::Check, r.center(), 13.0, widgets::on_color(widgets::rgb(c)));
            }
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                *color = c;
            }
        }
    });
}

/// Opens a folder in Explorer.
pub fn open_folder(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::process::Command::new("explorer.exe").arg(dir).spawn();
}
