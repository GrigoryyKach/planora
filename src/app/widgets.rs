//! Small drawing helpers: icons, status and priority marks, segmented controls, cards.

use crate::model::{Priority, Status};
use crate::theme::{self, Pal};
use eframe::egui::{
    self, epaint::PathStroke, Align2, Color32, CursorIcon, Margin, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};

pub const RED: Color32 = theme::hex(0xe5484d);
pub const GREEN: Color32 = theme::hex(0x2fb26a);

pub fn priority_color(p: Priority) -> Color32 {
    match p {
        Priority::Low => theme::hex(0x5b9cf5),
        Priority::Medium => theme::hex(0xe0ac2b),
        Priority::High => theme::hex(0xf2803a),
        Priority::Urgent => RED,
    }
}

pub fn rgb(c: u32) -> Color32 {
    theme::hex(c)
}

/// Text colour that reads well on top of `bg`.
pub fn on_color(bg: Color32) -> Color32 {
    let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if l > 170.0 { theme::hex(0x1b1f27) } else { Color32::WHITE }
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
    let st = Stroke::new((size / 11.0).max(1.4), col);
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

/// Square icon button with a hover background.
pub fn icon_button(ui: &mut Ui, ic: Icon, size: f32, p: &Pal) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hov = resp.hovered();
    if hov {
        ui.painter().rect_filled(rect, 8.0, p.hover);
    }
    icon(ui.painter(), ic, rect.center(), size * 0.55, if hov { p.text } else { p.muted });
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// Status mark: a ring (todo), a half ring (in progress), a green check (done), a crossed ring (cancelled).
pub fn status_mark(painter: &Painter, c: Pos2, status: Option<Status>, done: bool, p: &Pal) {
    let status = if done { Some(Status::Done) } else { status };
    match status {
        None | Some(Status::Todo) => {
            painter.circle_stroke(c, 8.0, Stroke::new(1.6, if status.is_some() { p.muted } else { p.track }));
        }
        Some(Status::InProgress) => {
            painter.circle_stroke(c, 8.0, Stroke::new(1.6, p.accent));
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
            painter.circle_filled(c, 8.5, GREEN);
            icon(painter, Icon::Check, c, 14.0, Color32::WHITE);
        }
        Some(Status::Cancelled) => {
            painter.circle_stroke(c, 8.0, Stroke::new(1.6, p.muted));
            icon(painter, Icon::Close, c, 11.0, p.muted);
        }
    }
}

/// Segmented control (like All · Personal · Work). Returns the picked index.
pub fn segmented(ui: &mut Ui, id: &str, labels: &[&str], selected: usize, colors: &[Option<Color32>], p: &Pal) -> Option<usize> {
    let font = theme::semi(13.0);
    let widths: Vec<f32> = labels.iter().map(|l| ui.painter().layout_no_wrap(l.to_string(), font.clone(), p.text).size().x + 28.0).collect();
    let total: f32 = widths.iter().sum::<f32>() + 6.0;
    let (rect, _) = ui.allocate_exact_size(vec2(total, 34.0), Sense::hover());
    ui.painter().rect(rect, 10.0, p.card, Stroke::new(1.0, p.line), StrokeKind::Inside);
    let mut x = rect.left() + 3.0;
    let mut picked = None;
    for (i, l) in labels.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, rect.top() + 3.0), vec2(widths[i], 28.0));
        x += widths[i];
        let resp = ui.interact(r, ui.scope_id().with((id, i)), Sense::click());
        let on = i == selected;
        if on {
            ui.painter().rect_filled(r, 8.0, p.accent_soft);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 8.0, p.hover);
        }
        let col = if on { p.accent } else if resp.hovered() { p.text } else { p.muted };
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

/// Row of selectable pills. `selected` is the index or `None`. Returns `Some(new)` when clicked;
/// clicking the selected pill clears it when `clearable`.
pub fn pills(ui: &mut Ui, id: &str, labels: &[&str], colors: &[Option<Color32>], selected: Option<usize>, clearable: bool, p: &Pal) -> Option<Option<usize>> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for (i, l) in labels.iter().enumerate() {
            let on = selected == Some(i);
            let col = colors.get(i).copied().flatten().unwrap_or(p.accent);
            let font = theme::semi(12.5);
            let g = ui.painter().layout_no_wrap(l.to_string(), font.clone(), p.text);
            let dot = if colors.get(i).copied().flatten().is_some() { 14.0 } else { 0.0 };
            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 22.0 + dot, 28.0), Sense::click());
            let fill = if on { theme::mix(p.card, col, 0.2) } else if resp.hovered() { p.hover } else { Color32::TRANSPARENT };
            ui.painter().rect(r, 14.0, fill, Stroke::new(1.0, if on { col } else { p.line }), StrokeKind::Inside);
            let mut x = r.left() + 11.0;
            if dot > 0.0 {
                ui.painter().circle_filled(pos2(x + 3.0, r.center().y), 4.0, col);
                x += dot;
            }
            ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, l, font, if on { p.text } else { p.muted });
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
            .margin(Margin::symmetric(11, 8))
            .font(theme::reg(14.0)),
    )
}


/// The app icon at `size` px (the texture is 512 px and filtered, so it stays crisp when scaled down).
pub fn logo_image(ui: &mut Ui, tex: &egui::TextureHandle, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    ui.painter().image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    resp
}
