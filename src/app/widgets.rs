//! Small drawing helpers: icons, status and priority marks, segmented controls, dropdowns, buttons.

use crate::model::{Priority, Status};
use crate::theme::{self, Btn, Pal};
use eframe::egui::{
    self, epaint::PathStroke, Align2, Color32, CursorIcon, Margin, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

/// Urgent priority dot. The dot colours are the one place colour is semantic in the lists.
const URGENT: Color32 = theme::hex(0xe5484d);

pub fn priority_color(p: Priority) -> Color32 {
    match p {
        Priority::Low => theme::hex(0x5b9cf5),
        Priority::Medium => theme::hex(0xe0ac2b),
        Priority::High => theme::hex(0xf2803a),
        Priority::Urgent => URGENT,
    }
}

pub fn rgb(c: u32) -> Color32 {
    theme::hex(c)
}

/// Text colour that reads well on top of `bg`.
pub fn on_color(bg: Color32) -> Color32 {
    let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if l > 150.0 { theme::hex(0x141415) } else { Color32::WHITE }
}

/// Fill of the selected row on a surface that is not a card: raised on dark, the card white on light
/// (on light the sidebar and the raised step are the same grey, so the white card marks the selection).
pub fn selected_fill(p: &Pal) -> Color32 {
    if p.dark { p.raised } else { p.card }
}

/// Hover fill that fades in, for rows drawn by hand.
pub fn hover_fill(ui: &Ui, id: egui::Id, hovered: bool, p: &Pal) -> Color32 {
    theme::alpha(selected_fill(p), (theme::fade(ui, id.with("hover"), hovered) * 255.0) as u8)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Gear,
    Plus,
    Search,
    Left,
    Right,
    Close,
    Flag,
    Link,
    Trash,
    Check,
    Calendar,
    Today,
    Inbox,
    Repeat,
    Clock,
}

/// Draws a line icon centred at `c` within a `size`×`size` box.
pub fn icon(p: &Painter, ic: Icon, c: Pos2, size: f32, col: Color32) {
    let s = size / 2.0;
    let st = Stroke::new(1.5, col);
    let line = |a: Pos2, b: Pos2| p.line_segment([a, b], st);
    match ic {
        Icon::Gear => {
            p.circle_stroke(c, s * 0.5, st);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                line(c + d * (s * 0.68), c + d * (s * 0.98));
            }
            p.circle_stroke(c, s * 0.78, Stroke::new(st.width, col));
        }
        Icon::Plus => {
            line(c - vec2(s * 0.7, 0.0), c + vec2(s * 0.7, 0.0));
            line(c - vec2(0.0, s * 0.7), c + vec2(0.0, s * 0.7));
        }
        Icon::Search => {
            let o = c - vec2(s * 0.15, s * 0.15);
            p.circle_stroke(o, s * 0.55, st);
            line(o + vec2(s * 0.4, s * 0.4), c + vec2(s * 0.8, s * 0.8));
        }
        Icon::Left => {
            line(c + vec2(s * 0.25, -s * 0.6), c + vec2(-s * 0.3, 0.0));
            line(c + vec2(-s * 0.3, 0.0), c + vec2(s * 0.25, s * 0.6));
        }
        Icon::Right => {
            line(c + vec2(-s * 0.25, -s * 0.6), c + vec2(s * 0.3, 0.0));
            line(c + vec2(s * 0.3, 0.0), c + vec2(-s * 0.25, s * 0.6));
        }
        Icon::Close => {
            line(c + vec2(-s * 0.5, -s * 0.5), c + vec2(s * 0.5, s * 0.5));
            line(c + vec2(-s * 0.5, s * 0.5), c + vec2(s * 0.5, -s * 0.5));
        }
        Icon::Flag => {
            line(c + vec2(-s * 0.55, -s * 0.8), c + vec2(-s * 0.55, s * 0.8));
            p.add(Shape::convex_polygon(
                vec![c + vec2(-s * 0.55, -s * 0.75), c + vec2(s * 0.7, -s * 0.35), c + vec2(-s * 0.55, s * 0.1)],
                col,
                Stroke::NONE,
            ));
        }
        Icon::Link => {
            let r1 = Rect::from_center_size(c + vec2(-s * 0.3, s * 0.0), vec2(s * 1.1, s * 0.6));
            let r2 = Rect::from_center_size(c + vec2(s * 0.3, s * 0.0), vec2(s * 1.1, s * 0.6));
            p.rect_stroke(r1, s * 0.3, st, StrokeKind::Inside);
            p.rect_stroke(r2, s * 0.3, st, StrokeKind::Inside);
        }
        Icon::Trash => {
            line(c + vec2(-s * 0.7, -s * 0.5), c + vec2(s * 0.7, -s * 0.5));
            line(c + vec2(-s * 0.2, -s * 0.5), c + vec2(-s * 0.2, -s * 0.8));
            line(c + vec2(s * 0.2, -s * 0.5), c + vec2(s * 0.2, -s * 0.8));
            line(c + vec2(-s * 0.2, -s * 0.8), c + vec2(s * 0.2, -s * 0.8));
            p.rect_stroke(Rect::from_min_max(c + vec2(-s * 0.5, -s * 0.5), c + vec2(s * 0.5, s * 0.85)), 2.0, st, StrokeKind::Inside);
        }
        Icon::Check => {
            line(c + vec2(-s * 0.55, 0.0), c + vec2(-s * 0.15, s * 0.45));
            line(c + vec2(-s * 0.15, s * 0.45), c + vec2(s * 0.6, -s * 0.45));
        }
        Icon::Calendar => {
            let r = Rect::from_center_size(c + vec2(0.0, s * 0.1), vec2(s * 1.6, s * 1.45));
            p.rect_stroke(r, 3.0, st, StrokeKind::Inside);
            line(pos2(r.left(), r.top() + s * 0.45), pos2(r.right(), r.top() + s * 0.45));
            line(pos2(r.left() + s * 0.45, r.top() - s * 0.2), pos2(r.left() + s * 0.45, r.top() + s * 0.2));
            line(pos2(r.right() - s * 0.45, r.top() - s * 0.2), pos2(r.right() - s * 0.45, r.top() + s * 0.2));
        }
        Icon::Today => {
            let r = Rect::from_center_size(c + vec2(0.0, s * 0.1), vec2(s * 1.6, s * 1.45));
            p.rect_stroke(r, 3.0, st, StrokeKind::Inside);
            line(pos2(r.left(), r.top() + s * 0.45), pos2(r.right(), r.top() + s * 0.45));
            p.circle_filled(r.center() + vec2(0.0, s * 0.25), s * 0.2, col);
        }
        Icon::Inbox => {
            let r = Rect::from_center_size(c, vec2(s * 1.6, s * 1.3));
            p.rect_stroke(r, 3.0, st, StrokeKind::Inside);
            line(pos2(r.left(), c.y + s * 0.1), pos2(r.left() + s * 0.5, c.y + s * 0.1));
            line(pos2(r.left() + s * 0.5, c.y + s * 0.1), pos2(r.left() + s * 0.6, c.y + s * 0.35));
            line(pos2(r.left() + s * 0.6, c.y + s * 0.35), pos2(r.right() - s * 0.6, c.y + s * 0.35));
            line(pos2(r.right() - s * 0.6, c.y + s * 0.35), pos2(r.right() - s * 0.5, c.y + s * 0.1));
            line(pos2(r.right() - s * 0.5, c.y + s * 0.1), pos2(r.right(), c.y + s * 0.1));
        }
        Icon::Repeat => {
            let n = 14;
            let pts: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let a = -0.6 + i as f32 / n as f32 * 4.9;
                    c + vec2(a.cos(), a.sin()) * s * 0.7
                })
                .collect();
            p.add(Shape::line(pts.clone(), PathStroke::new(st.width, col)));
            let e = *pts.last().unwrap();
            p.add(Shape::convex_polygon(vec![e + vec2(-s * 0.3, -s * 0.35), e + vec2(s * 0.35, -s * 0.2), e + vec2(0.0, s * 0.35)], col, Stroke::NONE));
        }
        Icon::Clock => {
            p.circle_stroke(c, s * 0.8, st);
            line(c, c + vec2(0.0, -s * 0.5));
            line(c, c + vec2(s * 0.35, s * 0.2));
        }
    }
}

/// Square icon button; a raised fill fades in on hover.
pub fn icon_button(ui: &mut Ui, ic: Icon, size: f32, p: &Pal) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let t = theme::fade(ui, resp.id.with("hover"), resp.hovered());
    ui.painter().rect_filled(rect, theme::R_CONTROL, theme::alpha(p.raised, (t * 255.0) as u8));
    icon(ui.painter(), ic, rect.center(), size * 0.55, theme::mix(p.muted, p.text, t));
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Status mark: a ring (todo), a half ring in the accent (in progress), a green check (done), a red crossed
/// ring (cancelled). Cancelled keeps its own mark even though the task counts as closed.
pub fn status_mark(painter: &Painter, c: Pos2, status: Option<Status>, done: bool, p: &Pal) {
    let status = if done && status != Some(Status::Cancelled) { Some(Status::Done) } else { status };
    match status {
        None | Some(Status::Todo) => {
            let col = if status.is_some() { p.muted } else { theme::mix(p.muted, p.card, 0.5) };
            painter.circle_stroke(c, 8.0, Stroke::new(1.5, col));
        }
        Some(Status::InProgress) => {
            painter.circle_stroke(c, 8.0, Stroke::new(1.5, p.accent));
            let pts: Vec<Pos2> = (0..=10)
                .map(|i| {
                    let a = std::f32::consts::FRAC_PI_2 + i as f32 / 10.0 * std::f32::consts::PI;
                    c + vec2(a.cos(), -a.sin()) * 4.2
                })
                .collect();
            let mut poly = pts;
            poly.push(c + vec2(0.0, -4.2));
            painter.add(Shape::convex_polygon(poly, p.accent, Stroke::NONE));
        }
        Some(Status::Done) => {
            painter.circle_filled(c, 8.5, p.positive);
            icon(painter, Icon::Check, c, 14.0, on_color(p.positive));
        }
        Some(Status::Cancelled) => {
            painter.circle_stroke(c, 8.0, Stroke::new(1.5, p.negative));
            icon(painter, Icon::Close, c, 11.0, p.negative);
        }
    }
}

/// Height shared by every control that sits in a row (segmented control, buttons, inputs).
pub const ROW_BTN_H: f32 = 34.0;

/// Segmented control (like All, Personal, Work), 34 high like every other control in a row. Items may carry a
/// colour dot (a profile). Returns the picked index.
pub fn segmented(ui: &mut Ui, id: &str, labels: &[&str], selected: usize, colors: &[Option<Color32>], p: &Pal) -> Option<usize> {
    let font = theme::medium(13.0);
    let widths: Vec<f32> = labels.iter().map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), p.text).size().x + 28.0).collect();
    let total: f32 = widths.iter().sum::<f32>() + 8.0;
    let (rect, _) = ui.allocate_exact_size(vec2(total, ROW_BTN_H), Sense::hover());
    ui.painter().rect(rect, 10.0, p.card, Stroke::new(1.0, p.line), StrokeKind::Inside);
    let mut x = rect.left() + 4.0;
    let mut picked = None;
    for (i, l) in labels.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, rect.top() + 4.0), vec2(widths[i], ROW_BTN_H - 8.0));
        x += widths[i];
        let resp = ui.interact(r, ui.scope_id().with((id, i)), Sense::click());
        let on = i == selected;
        let t = theme::fade(ui, resp.id.with("hover"), resp.hovered() && !on);
        if on {
            ui.painter().rect(r, 6.0, p.raised, Stroke::new(1.0, p.line), StrokeKind::Inside);
            ui.painter().hline(r.x_range().shrink(6.0), r.top() + 0.5, Stroke::new(1.0, p.top_light()));
        } else if t > 0.0 {
            ui.painter().rect_filled(r, 6.0, theme::alpha(p.raised, (t * 160.0) as u8));
        }
        let col = if on { p.text } else { theme::mix(p.text2, p.text, t) };
        let mut tx = r.center().x;
        if let Some(Some(dot)) = colors.get(i) {
            let g = ui.painter().layout_no_wrap(l.to_string(), font.clone(), col);
            let w = g.size().x + 14.0;
            tx = r.center().x + 7.0;
            ui.painter().circle_filled(pos2(r.center().x - w / 2.0 + 4.0, r.center().y), 4.0, *dot);
        }
        ui.painter().text(pos2(tx, r.center().y), Align2::CENTER_CENTER, l, font.clone(), col);
        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            picked = Some(i);
        }
    }
    picked
}

/// Row of selectable chips (radius 8, not pills). `selected` is the index or `None`. Returns `Some(new)` when
/// clicked; clicking the selected chip clears it when `clearable`. The chosen chip is a raised surface with an
/// outline; a colour dot (priority) is the only colour.
pub fn pills(ui: &mut Ui, id: &str, labels: &[&str], colors: &[Option<Color32>], selected: Option<usize>, clearable: bool, p: &Pal) -> Option<Option<usize>> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        for (i, l) in labels.iter().enumerate() {
            let on = selected == Some(i);
            let dot_col = colors.get(i).copied().flatten();
            let font = theme::reg(13.0);
            let g = ui.painter().layout_no_wrap(l.to_string(), font.clone(), p.text);
            let dot = if dot_col.is_some() { 14.0 } else { 0.0 };
            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 24.0 + dot, 28.0), Sense::click());
            let t = theme::fade(ui, resp.id.with("hover"), resp.hovered() && !on);
            let fill = if on { p.raised } else { theme::alpha(p.raised, (t * 255.0) as u8) };
            ui.painter().rect(r, 8.0, fill, Stroke::new(1.0, if on { p.line_hi } else { p.line }), StrokeKind::Inside);
            let mut x = r.left() + 12.0;
            if let Some(c) = dot_col {
                ui.painter().circle_filled(pos2(x + 3.0, r.center().y), 4.0, c);
                x += dot;
            }
            ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, l, font, if on { p.text } else { theme::mix(p.text2, p.text, t) });
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                let _ = id;
                out = Some(if on && clearable { None } else { Some(i) });
            }
        }
    });
    out
}

/// Single-line text input in the suite style.
pub fn input(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    ui.add(
        egui::TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(width)
            .margin(Margin::symmetric(12, 8))
            .font(theme::reg(14.0)),
    )
}

/// The Planora mark: a month page with a ring on the day, in the accent colour. Flat, no tile.
pub fn mark_glyph(painter: &Painter, c: Pos2, col: Color32) {
    let st = Stroke::new(1.5, col);
    let r = Rect::from_center_size(c + vec2(0.0, 1.0), vec2(20.0, 18.0));
    painter.rect_stroke(r, 4.0, st, StrokeKind::Inside);
    painter.line_segment([pos2(r.left(), r.top() + 5.5), pos2(r.right(), r.top() + 5.5)], st);
    painter.line_segment([pos2(r.left() + 5.5, r.top() - 2.5), pos2(r.left() + 5.5, r.top() + 1.5)], st);
    painter.line_segment([pos2(r.right() - 5.5, r.top() - 2.5), pos2(r.right() - 5.5, r.top() + 1.5)], st);
    painter.circle_stroke(r.center() + vec2(0.0, 3.0), 3.2, st);
}

// ───────────────────────── buttons with unique ids ─────────────────────────

/// `theme::button` derives its id from the label alone, so two buttons with the same text in one frame
/// ("Set date" twice, "Add" in the list and in the side panel) share one id: hovering one highlights both
/// and clicks get lost. This wrapper salts the id with the button's position, so every button is independent.
pub fn btn(ui: &mut Ui, label: &str, kind: Btn, p: &Pal) -> Response {
    let pos = ui.cursor().min;
    ui.push_id(("btn-at", pos.x as i32, pos.y as i32), |ui| theme::button(ui, label, kind, p)).inner
}

/// Like [`btn`], but can be shown disabled (muted, no hover, no click).
pub fn btn_enabled(ui: &mut Ui, label: &str, kind: Btn, enabled: bool, p: &Pal) -> Response {
    if enabled {
        return btn(ui, label, kind, p);
    }
    let font = theme::medium(14.0);
    let g = ui.painter().layout_no_wrap(label.to_string(), font.clone(), p.muted);
    let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 24.0, kind.height()), Sense::hover());
    ui.painter().rect(rect, theme::R_CONTROL as f32, Color32::TRANSPARENT, Stroke::new(1.0, p.line), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, p.muted);
    resp
}

// ───────────────────────── dropdowns ─────────────────────────

/// Paints the dropdown chevron, vertically centred on the text row.
fn chevron(ui: &Ui, rect: Rect, color: Color32) {
    theme::icon_chevron(ui.painter(), rect.center() + vec2(0.0, CHEVRON_NUDGE), color, true);
}

/// Pixels to move the chevron so it looks level with the text.
const CHEVRON_NUDGE: f32 = 0.0;

/// A dropdown button in the suite style: text on the left, a chevron centred on the same row.
pub fn combo(id: &str, text: &str, width: f32) -> egui::ComboBox<'static> {
    egui::ComboBox::from_id_salt(id.to_string())
        .width(width)
        .selected_text(egui::RichText::new(text.to_string()).font(theme::reg(14.0)))
        .icon(|ui, rect, visuals, _open| chevron(ui, rect, visuals.fg_stroke.color))
}

/// A checkbox row for multi-select popups.
pub fn check_row(ui: &mut Ui, label: &str, checked: bool, p: &Pal) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(190.0), 32.0), Sense::click());
    ui.painter().rect_filled(rect, 6.0, theme::alpha(p.raised, (theme::fade(ui, resp.id.with("hover"), resp.hovered()) * 255.0) as u8));
    let b = Rect::from_center_size(pos2(rect.left() + 18.0, rect.center().y), vec2(16.0, 16.0));
    check_box(ui.painter(), b, checked, p);
    ui.painter().text(pos2(rect.left() + 34.0, rect.center().y), Align2::LEFT_CENTER, label, theme::reg(14.0), p.text);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// A checkbox: accent fill and a check when on, a muted outline when off. Radius 4.
pub fn check_box(painter: &Painter, b: Rect, on: bool, p: &Pal) {
    if on {
        painter.rect_filled(b, 4.0, p.accent);
        icon(painter, Icon::Check, b.center(), 12.0, p.on_accent);
    } else {
        painter.rect_stroke(b, 4.0, Stroke::new(1.5, p.muted), StrokeKind::Inside);
    }
}

/// A multi-select dropdown ("Status: All", "Status: 2", "Status: Done") with checkboxes and a Clear entry.
/// Returns `true` when the selection changed.
#[allow(clippy::too_many_arguments)]
pub fn multi_select(ui: &mut Ui, id: &str, title: &str, all: &str, clear: &str, options: &[&str], flags: &mut [bool], p: &Pal) -> bool {
    let n = flags.iter().filter(|b| **b).count();
    let value = match n {
        0 => all.to_string(),
        1 => flags.iter().position(|b| *b).and_then(|i| options.get(i)).map(|s| s.to_string()).unwrap_or_default(),
        _ => n.to_string(),
    };
    let mut changed = false;
    combo(id, &format!("{title}: {value}"), 160.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            ui.set_min_width(190.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            for (i, o) in options.iter().enumerate() {
                if check_row(ui, o, flags[i], p).clicked() {
                    flags[i] = !flags[i];
                    changed = true;
                }
            }
            ui.add_space(4.0);
            theme::hairline(ui, p);
            ui.add_space(4.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width().max(190.0), 28.0), Sense::click());
            let t = theme::fade(ui, resp.id.with("hover"), resp.hovered() && n > 0);
            let col = theme::mix(p.muted, p.accent, t);
            ui.painter().text(pos2(rect.left() + 12.0, rect.center().y), Align2::LEFT_CENTER, clear, theme::medium(12.0), col);
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() && n > 0 {
                flags.iter_mut().for_each(|f| *f = false);
                changed = true;
            }
        });
    changed
}
