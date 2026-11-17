//! Minimal A4 task-list PDF: a header, then one line per task ("checkbox, time, title"), page numbers.
//!
//! Pure Rust (`printpdf`, builds on GNU and MSVC). Noto Sans (SIL OFL, see `assets/fonts/OFL.txt`)
//! is embedded, and printpdf subsets it, so Latin and Cyrillic text render correctly.

use printpdf::{
    Color, Line, LinePoint, Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, Rgb, TextItem,
};

const REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 20.0;
const BODY_SIZE: f32 = 11.5;
const LINE_H: f32 = 6.4;
/// Millimetres per point.
const MM: f32 = 25.4 / 72.0;

pub struct Row {
    pub done: bool,
    pub time: Option<String>,
    pub title: String,
}

pub struct Doc {
    pub title: String,
    pub subtitle: String,
    pub rows: Vec<Row>,
    /// "Page {n} of {total}" in the interface language.
    pub page_label: String,
    /// Shown when `rows` is empty.
    pub empty_label: String,
}

enum Item {
    Text { x: f32, y: f32, size: f32, bold: bool, grey: f32, text: String },
    Line { pts: Vec<(f32, f32)>, closed: bool, width: f32, grey: f32 },
}

struct Fonts {
    regular: ParsedFont,
    bold: ParsedFont,
}

impl Fonts {
    /// Text width in millimetres.
    fn width(&self, text: &str, size: f32, bold: bool) -> f32 {
        let f = if bold { &self.bold } else { &self.regular };
        let upm = f.units_per_em.max(1) as f32;
        text.chars()
            .map(|c| f.lookup_glyph_index(c as u32).and_then(|g| f.get_glyph_width(g)).unwrap_or((upm * 0.5) as u16) as f32)
            .sum::<f32>()
            / upm
            * size
            * MM
    }

    /// Greedy word wrap (long words are split by characters).
    fn wrap(&self, text: &str, size: f32, max: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut cur = String::new();
        for word in text.split_whitespace() {
            let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
            if self.width(&cand, size, false) <= max {
                cur = cand;
                continue;
            }
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let mut piece = String::new();
            for ch in word.chars() {
                let mut next = piece.clone();
                next.push(ch);
                if self.width(&next, size, false) > max && !piece.is_empty() {
                    lines.push(std::mem::take(&mut piece));
                }
                piece.push(ch);
            }
            cur = piece;
        }
        if !cur.is_empty() || lines.is_empty() {
            lines.push(cur);
        }
        lines
    }
}

fn pt(mm: f32) -> Pt {
    Pt(mm / MM)
}

/// Renders the document. Returns the PDF bytes.
pub fn render(doc: &Doc) -> Result<Vec<u8>, String> {
    let mut warn = Vec::new();
    let fonts = Fonts {
        regular: ParsedFont::from_bytes(REGULAR, 0, &mut warn).ok_or("regular font")?,
        bold: ParsedFont::from_bytes(BOLD, 0, &mut warn).ok_or("bold font")?,
    };

    // Layout: pages of items, y measured from the top of the page in millimetres.
    let mut pages: Vec<Vec<Item>> = vec![Vec::new()];
    let mut y = MARGIN + 7.0;
    let text_w = PAGE_W - 2.0 * MARGIN;
    pages[0].push(Item::Text { x: MARGIN, y, size: 20.0, bold: true, grey: 0.1, text: doc.title.clone() });
    y += 7.5;
    if !doc.subtitle.is_empty() {
        pages[0].push(Item::Text { x: MARGIN, y, size: 11.0, bold: false, grey: 0.4, text: doc.subtitle.clone() });
        y += 4.5;
    }
    pages[0].push(Item::Line { pts: vec![(MARGIN, y), (PAGE_W - MARGIN, y)], closed: false, width: 0.6, grey: 0.75 });
    y += 9.0;
    let bottom = PAGE_H - MARGIN - 6.0;
    let box_size = 3.6;
    if doc.rows.is_empty() {
        pages[0].push(Item::Text { x: MARGIN, y, size: BODY_SIZE, bold: false, grey: 0.45, text: doc.empty_label.clone() });
    }
    for row in &doc.rows {
        let indent = box_size + 3.0;
        let time_w = row.time.as_ref().map(|t| fonts.width(t, BODY_SIZE, true) + 3.0).unwrap_or(0.0);
        let first_max = text_w - indent - time_w;
        let lines = fonts.wrap(&row.title, BODY_SIZE, first_max.max(20.0));
        let height = LINE_H * lines.len() as f32;
        if y + height > bottom {
            pages.push(Vec::new());
            y = MARGIN + 7.0;
        }
        let page = pages.last_mut().expect("page");
        let grey = if row.done { 0.55 } else { 0.1 };
        // Checkbox, with a tick when done.
        let (bx, by) = (MARGIN, y - box_size + 0.6);
        page.push(Item::Line {
            pts: vec![(bx, by), (bx + box_size, by), (bx + box_size, by + box_size), (bx, by + box_size)],
            closed: true,
            width: 0.7,
            grey: 0.35,
        });
        if row.done {
            page.push(Item::Line {
                pts: vec![(bx + 0.7, by + box_size * 0.55), (bx + box_size * 0.42, by + box_size - 0.8), (bx + box_size - 0.5, by + 0.6)],
                closed: false,
                width: 1.0,
                grey: 0.2,
            });
        }
        let mut x = MARGIN + indent;
        if let Some(t) = &row.time {
            page.push(Item::Text { x, y, size: BODY_SIZE, bold: true, grey: 0.35, text: t.clone() });
            x += time_w;
        }
        for (i, l) in lines.iter().enumerate() {
            let lx = if i == 0 { x } else { MARGIN + indent + time_w };
            page.push(Item::Text { x: lx, y: y + LINE_H * i as f32, size: BODY_SIZE, bold: false, grey, text: l.clone() });
        }
        y += height + 1.2;
    }

    // Footers need the page count, so they are added last.
    let total = pages.len();
    for (i, page) in pages.iter_mut().enumerate() {
        let label = doc.page_label.replace("{n}", &(i + 1).to_string()).replace("{total}", &total.to_string());
        let w = fonts.width(&label, 9.0, false);
        let fy = PAGE_H - MARGIN + 2.0;
        page.push(Item::Text { x: PAGE_W - MARGIN - w, y: fy, size: 9.0, bold: false, grey: 0.5, text: label });
        page.push(Item::Text { x: MARGIN, y: fy, size: 9.0, bold: false, grey: 0.5, text: "Planora".into() });
    }

    let mut pdf = PdfDocument::new(&doc.title);
    let regular = pdf.add_font(&fonts.regular);
    let bold = pdf.add_font(&fonts.bold);
    let grey = |g: f32| Color::Rgb(Rgb { r: g, g, b: g, icc_profile: None });
    let mut out_pages = Vec::new();
    for items in &pages {
        let mut ops: Vec<Op> = Vec::new();
        for it in items {
            match it {
                Item::Text { x, y, size, bold: b, grey: g, text } => {
                    ops.push(Op::StartTextSection);
                    ops.push(Op::SetTextCursor { pos: Point { x: pt(*x), y: pt(PAGE_H - *y) } });
                    ops.push(Op::SetFont { font: PdfFontHandle::External(if *b { bold.clone() } else { regular.clone() }), size: Pt(*size) });
                    ops.push(Op::SetFillColor { col: grey(*g) });
                    ops.push(Op::ShowText { items: vec![TextItem::Text(text.clone())] });
                    ops.push(Op::EndTextSection);
                }
                Item::Line { pts, closed, width, grey: g } => {
                    ops.push(Op::SetOutlineColor { col: grey(*g) });
                    ops.push(Op::SetOutlineThickness { pt: Pt(*width / MM) });
                    ops.push(Op::DrawLine {
                        line: Line {
                            points: pts.iter().map(|(x, y)| LinePoint { p: Point { x: pt(*x), y: pt(PAGE_H - *y) }, bezier: false }).collect(),
                            is_closed: *closed,
                        },
                    });
                }
            }
        }
        out_pages.push(PdfPage::new(Mm(PAGE_W), Mm(PAGE_H), ops));
    }
    let bytes = pdf.with_pages(out_pages).save(&PdfSaveOptions::default(), &mut Vec::new());
    if bytes.is_empty() {
        return Err("empty pdf".into());
    }
    Ok(bytes)
}

/// Makes a string safe to use as a Windows file name.
pub fn sanitize_file_name(s: &str) -> String {
    let cleaned: String = s.chars().map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { '_' } else { c }).collect();
    let t = cleaned.trim().trim_end_matches('.').trim();
    if t.is_empty() { "Planora".into() } else { t.chars().take(120).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(n: usize) -> Doc {
        let mut rows = vec![
            Row { done: false, time: Some("09:30".into()), title: "Call the dentist".into() },
            Row { done: true, time: None, title: "Купить молоко и хлеб, позвонить маме".into() },
            Row { done: false, time: Some("18:00".into()), title: "Підготувати звіт про проєкт".into() },
        ];
        for i in 0..n {
            rows.push(Row { done: i % 3 == 0, time: (i % 5 == 0).then(|| "12:15".to_string()), title: format!("Task number {i}: a reasonably long title that may need to wrap onto a second line when the page is narrow, Завдання {i}") });
        }
        Doc { title: "Проект «Ремонт» / Home".into(), subtitle: "Personal".into(), rows, page_label: "Page {n} of {total}".into(), empty_label: "No tasks".into() }
    }

    #[test]
    fn renders_cyrillic_and_latin() {
        let bytes = render(&doc(0)).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 5_000);
        let parsed = PdfDocument::parse(&bytes, &printpdf::PdfParseOptions::default(), &mut Vec::new()).unwrap();
        assert_eq!(parsed.pages.len(), 1);
    }

    #[test]
    fn hundred_items_break_pages() {
        let bytes = render(&doc(100)).unwrap();
        let parsed = PdfDocument::parse(&bytes, &printpdf::PdfParseOptions::default(), &mut Vec::new()).unwrap();
        assert!(parsed.pages.len() >= 3, "pages: {}", parsed.pages.len());
        // The font is subset: much smaller than the two bundled fonts together.
        assert!(bytes.len() < 700_000, "size {}", bytes.len());
    }

    #[test]
    fn empty_list_still_renders() {
        let mut d = doc(0);
        d.rows.clear();
        assert!(render(&d).is_ok());
    }

    #[test]
    fn file_names_are_sanitised() {
        assert_eq!(sanitize_file_name("a/b:c*?"), "a_b_c__");
        assert_eq!(sanitize_file_name("  "), "Planora");
        assert_eq!(sanitize_file_name("Ремонт."), "Ремонт");
    }
}
