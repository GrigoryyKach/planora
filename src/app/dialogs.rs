//! Modal dialogs, the day panel, the date picker popup and the undo toast.

use super::widgets::{self, Icon};
use super::{id_of, App, PALETTE};
use crate::config::Config;
use crate::model::reminders::Reminder;
use crate::model::{Data, Profile, Project};
use crate::theme::{self, Pal};
use chrono::NaiveDate;
use eframe::egui::{self, Align2, Color32, CursorIcon, Frame, Margin, Order, Rect, Sense, Stroke, Ui, vec2};
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
            .stroke(Stroke::new(1.0, p.line_hi))
            .corner_radius(theme::R_WINDOW)
            .inner_margin(Margin::same(24))
            .shadow(theme::shadow_for(p))
            .show(ui, |ui| {
                ui.set_width(width);
                add(ui);
            });
    });
    dim.inner.clicked()
}

/// What was clicked in a modal footer.
#[derive(Default)]
pub(super) struct FooterOut {
    pub left: bool,
    /// Index into the `secondary` labels.
    pub secondary: Option<usize>,
    pub primary: bool,
}

/// The one footer every modal uses: all buttons are 36 px tall with the same minimum width, the accent
/// button is rightmost, the secondary buttons sit to its left, an optional secondary button sits at the far left.
pub(super) fn footer(ui: &mut Ui, _app: &App, p: &Pal, left: Option<&str>, secondary: &[&str], primary: &str, enabled: bool) -> FooterOut {
    const H: f32 = 36.0;
    const GAP: f32 = 8.0;
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), H), Sense::hover());
    let painter = ui.painter().clone();
    let width = |label: &str| (painter.layout_no_wrap(label.to_string(), theme::medium(14.0), p.text).size().x + 40.0).max(112.0);
    let mut out = FooterOut::default();
    let mut x = row.right();
    let w = width(primary);
    x -= w;
    let rect = Rect::from_min_size(egui::pos2(x, row.top()), vec2(w, H));
    let id = ui.scope_id().with(("footer-primary", primary));
    if enabled {
        if theme::button_at(ui, rect, id, primary, theme::Btn::Primary, p).clicked() {
            out.primary = true;
        }
    } else {
        // Disabled: a flat outline with muted text, no hover and no click.
        ui.interact(rect, id, Sense::hover());
        painter.rect(rect, theme::R_CONTROL as f32, p.raised, Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
        painter.text(rect.center(), Align2::CENTER_CENTER, primary, theme::medium(14.0), p.muted);
    }
    for (i, label) in secondary.iter().enumerate().rev() {
        let w = width(label);
        x -= GAP + w;
        let rect = Rect::from_min_size(egui::pos2(x, row.top()), vec2(w, H));
        if theme::button_at(ui, rect, ui.scope_id().with(("footer-secondary", i, *label)), label, theme::Btn::Secondary, p).clicked() {
            out.secondary = Some(i);
        }
    }
    if let Some(label) = left {
        let rect = Rect::from_min_size(row.min, vec2(width(label), H));
        if theme::button_at(ui, rect, ui.scope_id().with(("footer-left", label)), label, theme::Btn::Secondary, p).clicked() {
            out.left = true;
        }
    }
    out
}


pub(super) fn color_swatches(ui: &mut Ui, color: &mut u32, p: &Pal) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for c in PALETTE {
            let (r, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
            ui.painter().circle_filled(r.center(), 11.0, widgets::rgb(c));
            if *color == c {
                ui.painter().circle_stroke(r.center(), 13.0, Stroke::new(1.5, p.text));
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
    let _ = std::process::Command::new(if cfg!(windows) { "explorer.exe" } else { "xdg-open" }).arg(dir).spawn();
}
