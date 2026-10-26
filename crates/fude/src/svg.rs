//! The clean copy as pages of SVG, for ito-export to make into PNG or PDF:
//! drawn as the window draws it ([`draw::print`]), its letters measured in
//! egui's fonts and set by name in the system's.

use std::cell::RefCell;
use std::fmt::Write as _;

use egui::{Color32, Context, FontFamily, FontId, LayerId, Painter, Pos2, Rect, Vec2};

use crate::config::Config;
use crate::draw::{self, Sheet, Surface};
use crate::write::sink;

/// The faces the clean copy is set in, as the system names them.
const MINCHO: &str = "'Yu Mincho','MS Mincho','Hiragino Mincho ProN','Noto Serif CJK JP','IPAMincho',serif";
const GOTHIC: &str = "'Meiryo','Yu Gothic','MS Gothic','Hiragino Sans','Noto Sans CJK JP','IPAGothic',sans-serif";

/// The pages of the clean copy of `text` on `sheet`; none when it is empty.
pub fn pages(ctx: &Context, sheet: &Sheet, config: &Config, text: &str) -> Vec<String> {
    let measure = Painter::new(ctx.clone(), LayerId::background(), Rect::EVERYTHING);
    draw::print(ctx, sheet, config, text, |size, k| Svg::new(measure.clone(), size, k)).into_iter().map(Svg::finish).collect()
}

/// A page being drawn.
struct Svg {
    measure: Painter,
    body: RefCell<String>,
    /// How far below the middle of an ideograph its baseline is, as a share
    /// of its size, for each family.
    sinks: RefCell<Vec<(FontFamily, f32)>>,
    size: Vec2,
    /// What is drawn is this many times as big on the page.
    k: f32,
}

impl Svg {
    fn new(measure: Painter, size: Vec2, k: f32) -> Svg {
        Svg { measure, body: RefCell::default(), sinks: RefCell::default(), size, k }
    }

    /// The page: white, and what was drawn on it.
    fn finish(self) -> String {
        let (w, h) = (num(self.size.x), num(self.size.y));
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="#ffffff"/><g transform="scale({})">{}</g></svg>"##,
            num(self.k),
            self.body.into_inner()
        )
    }

    fn put(&self, s: std::fmt::Arguments) {
        let _ = self.body.borrow_mut().write_fmt(s);
    }
}

impl Surface for Svg {
    fn width(&self, s: &str, font: &FontId) -> f32 {
        self.measure.layout_no_wrap(s.into(), font.clone(), Color32::WHITE).size().x
    }

    fn text(&self, s: &str, at: Pos2, font: &FontId, color: Color32, turned: bool) {
        let sink = draw::once(&self.sinks, &font.family, || sink(&self.measure, &FontId::new(40.0, font.family.clone())) / 40.0);
        let (x, y) = (num(at.x), num(at.y));
        let turn = if turned { format!(r#" transform="rotate(90 {x} {y})""#) } else { String::new() };
        self.put(format_args!(
            r#"<text x="{x}" y="{}" font-size="{}" font-family="{}"{}{turn} text-anchor="middle">{}</text>"#,
            num(at.y + sink * font.size),
            num(font.size),
            face(&font.family),
            paint("fill", color),
            escape(s)
        ));
    }

    fn rule(&self, along_x: bool, at: f32, range: (f32, f32), color: Color32) {
        let (a, b) =
            if along_x { (Pos2::new(range.0, at), Pos2::new(range.1, at)) } else { (Pos2::new(at, range.0), Pos2::new(at, range.1)) };
        self.line(a, b, 1.0, color);
    }

    fn frame(&self, r: Rect, width: f32, color: Color32) {
        self.put(format_args!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="none" stroke-width="{}"{}/>"#,
            num(r.left()),
            num(r.top()),
            num(r.width()),
            num(r.height()),
            num(width),
            paint("stroke", color)
        ));
    }

    fn dot(&self, c: Pos2, r: f32, color: Color32) {
        self.put(format_args!(r#"<circle cx="{}" cy="{}" r="{}"{}/>"#, num(c.x), num(c.y), num(r), paint("fill", color)));
    }

    fn fill(&self, points: Vec<Pos2>, color: Color32) {
        let points: Vec<String> = points.iter().map(|p| format!("{},{}", num(p.x), num(p.y))).collect();
        self.put(format_args!(r#"<polygon points="{}"{}/>"#, points.join(" "), paint("fill", color)));
    }

    fn line(&self, a: Pos2, b: Pos2, width: f32, color: Color32) {
        self.put(format_args!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke-width="{}"{}/>"#,
            num(a.x),
            num(a.y),
            num(b.x),
            num(b.y),
            num(width),
            paint("stroke", color)
        ));
    }

    /// The page is the paper.
    fn sheet(&self, _r: Rect, _color: Color32) {}

    fn visible(&self, _r: Rect) -> bool {
        true
    }
}

/// A length, short.
fn num(x: f32) -> String {
    let s = format!("{x:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

/// ` fill="#rrggbb"`, and its opacity when it shows through.
fn paint(what: &str, c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let mut s = format!(r##" {what}="#{r:02x}{g:02x}{b:02x}""##);
    if a < 255 {
        let _ = write!(s, r#" {what}-opacity="{}""#, num(a as f32 / 255.0));
    }
    s
}

/// The faces of `family`: the mincho is the only one named.
fn face(family: &FontFamily) -> &'static str {
    match family {
        FontFamily::Name(_) => MINCHO,
        _ => GOTHIC,
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Dir, Paper};

    /// `f` within a frame, as the app makes the pages.
    fn in_frame<R>(f: impl FnOnce(&Context) -> R) -> R {
        let ctx = Context::default();
        let (mut f, mut made) = (Some(f), None);
        let mut out = ctx.run_ui(Default::default(), |ui| made = f.take().map(|f| f(ui.ctx())));
        out.textures_delta.clear();
        made.unwrap()
    }

    fn sheet(paper: Paper, dir: Dir) -> Sheet {
        Sheet { paper, dir, cols: 20, rows: 20, wrap: 0 }
    }

    #[test]
    fn a_grid_is_a_page_of_a4_to_each_of_its_sheets() {
        in_frame(a_grid_on);
    }

    fn a_grid_on(ctx: &Context) {
        let config = Config::default();
        assert!(pages(ctx, &sheet(Paper::Genko, Dir::Vertical), &config, " \n").is_empty());
        let one = pages(ctx, &sheet(Paper::Genko, Dir::Vertical), &config, "吾輩は猫である。");
        assert_eq!(one.len(), 1);
        // A vertical grid of 20 by 20 is wider than tall: A4 turned.
        assert!(one[0].starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1123" height="794""#), "{}", &one[0][..120]);
        assert!(one[0].contains(">吾</text>"));
        let long = "あ".repeat(500);
        assert_eq!(pages(ctx, &sheet(Paper::Genko, Dir::Vertical), &config, &long).len(), 2);
        let across = pages(ctx, &sheet(Paper::Genko, Dir::Horizontal), &config, &long);
        assert_eq!(across.len(), 2);
        assert!(across[0].contains(r#"width="794" height="1123""#));
    }

    #[test]
    fn a_sheet_runs_on_over_pages_of_a4_standing() {
        in_frame(a_sheet_on);
    }

    fn a_sheet_on(ctx: &Context) {
        let config = Config::default();
        let text = vec!["This is a pen."; 200].join("\n");
        let p = pages(ctx, &sheet(Paper::Ruled, Dir::Horizontal), &config, &text);
        assert!(p.len() > 2, "{}", p.len());
        assert!(p.iter().all(|s| s.contains(r#"width="794" height="1123""#)));
        assert!(p[0].contains("<line") && p[0].contains("This"));
        // Lines longer than the page holds draw it smaller.
        let narrow = Sheet { wrap: 60, ..sheet(Paper::Plain, Dir::Horizontal) };
        let p = pages(ctx, &narrow, &config, &"word ".repeat(100));
        assert!(!p[0].contains(r#"scale(1)"#), "{}", &p[0][..200]);
    }

    #[test]
    fn the_text_is_escaped_and_the_colours_are_plain() {
        assert_eq!(escape(r#"<a & "b" 'c'>"#), "&lt;a &amp; &quot;b&quot; &apos;c&apos;&gt;");
        assert_eq!(paint("fill", Color32::from_rgb(0x1d, 0x21, 0x29)), r##" fill="#1d2129""##);
        assert!(paint("stroke", Color32::from_rgb(200, 80, 60).gamma_multiply(0.5)).contains(r#"stroke-opacity="0.5""#));
        assert_eq!((num(1.0), num(0.126), num(-0.001), num(12.5)), ("1".into(), "0.13".into(), "0".into(), "12.5".into()));
    }

    #[test]
    fn the_pages_make_a_pdf_and_images() {
        let p = in_frame(|ctx| pages(ctx, &sheet(Paper::Genko, Dir::Vertical), &Config::default(), "吾輩は猫である。名前はまだ無い。"));
        let fonts = ito_export::Fonts::empty();
        let pdf = ito_export::pdf_bytes(&p, &fonts).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        let png = ito_export::png_bytes(&p[0], 0.5, &fonts).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }
}
