//! The clean copy on its paper: a manuscript grid in pages, or a ruled,
//! graph or plain sheet the lines wrap on; across the page or down it. Ruby
//! over the letters, a dot by each emphasised one, and where a click puts
//! the cursor; in the margin, how fast each line went and where the hand
//! stopped. The mock's `renderGenko` and `renderWrap` and their CSS.

use std::f32::consts::FRAC_PI_2;

use egui::epaint::{Shadow, TextShape};
use egui::{
    pos2, vec2, Color32, CursorIcon, FontFamily, FontId, Painter, Pos2, Rect, ScrollArea, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::config::{Config, Dir, Face, Hang, Paper, Rule};
use crate::fonts;
use crate::layout::{self, genko, wrap, Doc, Token};
use crate::pace::Pace;
use crate::vertical::{self, Turn};
use crate::write::middle;

/// Room around the paper.
const PAD: f32 = 24.0;
/// Between pages.
const GAP: f32 = 28.0;
/// A sheet's letters, the line they are set on, and its margins (at the
/// sides, and above and below).
const FONT: f32 = 20.0;
const PITCH: f32 = 40.0;
const MARGIN: Vec2 = vec2(44.0, 36.0);

/// The paper the clean copy is on, as the tools row sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet {
    /// Never `Auto`.
    pub paper: Paper,
    pub dir: Dir,
    pub cols: u32,
    pub rows: u32,
    /// Letters to a line on a sheet; 0 follows the window.
    pub wrap: u32,
}

impl Sheet {
    /// The paper the settings open the clean copy on: `Auto` is a grid, or a
    /// ruled sheet across the page when the text is mostly Latin.
    pub fn choose(config: &Config, text: &str) -> Sheet {
        let (paper, dir) = match config.paper {
            Paper::Auto if layout::is_latin(text) => (Paper::Ruled, Dir::Horizontal),
            Paper::Auto => (Paper::Genko, config.paper_dir),
            p => (p, config.paper_dir),
        };
        Sheet { paper, dir, cols: config.cols, rows: config.rows, wrap: config.wrap }
    }
}

/// What a frame of the clean copy came to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    /// Where a click put the cursor.
    pub clicked: Option<usize>,
    /// Sheets of manuscript paper (none on a sheet), the lines filled, and
    /// the paragraphs.
    pub pages: usize,
    pub lines: usize,
    pub paras: usize,
}

/// The clean copy of `text` on `sheet`, filling what is left of `ui`;
/// `empty` is said on a sheet with nothing on it. `fresh` scrolls to where
/// the text begins: the top, and the right of a vertical one. With a
/// `pace`, the margin shows how fast each line went.
pub fn draw(ui: &mut Ui, sheet: &Sheet, config: &Config, text: &str, empty: &str, fresh: bool, pace: Option<&Pace>) -> Drawn {
    let doc = Doc::parse(text);
    let end = text.chars().count();
    if sheet.paper == Paper::Genko {
        grid(ui, sheet, config, &doc, end, fresh, pace)
    } else {
        lined(ui, sheet, config, &doc, end, empty, fresh, pace)
    }
}

/// The family the clean copy is set in.
fn family(ctx: &egui::Context, config: &Config) -> FontFamily {
    match config.face {
        Face::Mincho => fonts::mincho(ctx),
        Face::Gothic => FontFamily::Proportional,
    }
}

/// What the paper is drawn in.
struct Ink {
    paper: Color32,
    ink: Color32,
    rule: Color32,
    dim: Color32,
    /// The dot where the hand stopped.
    stop: Color32,
}

impl Ink {
    fn of(rule: Rule) -> Ink {
        let c = ito_theme::colors();
        let rgb = |light: [u8; 3], dark: [u8; 3]| {
            let [r, g, b] = if c.light { light } else { dark };
            Color32::from_rgb(r, g, b)
        };
        let rule = match rule {
            Rule::Shu => rgb([0xd0, 0x50, 0x3f], [0xb8, 0x65, 0x5a]),
            Rule::Midori => rgb([0x3c, 0x8a, 0x56], [0x5f, 0x9c, 0x74]),
            Rule::Ao => rgb([0x6f, 0x9f, 0xd0], [0x5a, 0x7f, 0xa6]),
            Rule::Sumi => rgb([0x8a, 0x90, 0x9b], [0x5b, 0x62, 0x72]),
        };
        Ink {
            paper: rgb([0xff, 0xfd, 0xf8], [0x1d, 0x20, 0x27]),
            ink: rgb([0x1d, 0x21, 0x29], [0xdf, 0xe3, 0xea]),
            rule,
            dim: c.dim,
            stop: c.wait,
        }
    }

    /// A sheet of paper laid on the room.
    fn paper(&self, painter: &Painter, r: Rect) {
        let light = ito_theme::colors().light;
        let shadow = Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if light { 40 } else { 110 }) };
        painter.add(shadow.as_shape(r, 3.0));
        painter.rect_filled(r, 3.0, self.paper);
    }
}

/// How letters are set.
struct Pen<'a> {
    painter: &'a Painter,
    family: FontFamily,
    color: Color32,
    /// How far down a laid out letter the middle of an ideograph falls, as a
    /// share of its size.
    mid: f32,
    has: &'a dyn Fn(char) -> bool,
    vertical: bool,
}

impl<'a> Pen<'a> {
    fn new(painter: &'a Painter, family: FontFamily, color: Color32, has: &'a dyn Fn(char) -> bool, vertical: bool) -> Pen<'a> {
        let mid = middle(painter, &FontId::new(40.0, family.clone())) / 40.0;
        Pen { painter, family, color, mid, has, vertical }
    }

    fn font(&self, size: f32) -> FontId {
        FontId::new(size, self.family.clone())
    }

    /// `s` across the page, its middle at `at`, made smaller if wider than
    /// `fit`.
    fn across(&self, s: &str, at: Pos2, size: f32, fit: f32) {
        let mut size = size;
        let mut g = self.painter.layout_no_wrap(s.into(), self.font(size), self.color);
        if g.size().x > fit {
            size *= fit / g.size().x;
            g = self.painter.layout_no_wrap(s.into(), self.font(size), self.color);
        }
        self.painter.galley(pos2(at.x - g.size().x / 2.0, at.y - self.mid * size), g, self.color);
    }

    /// A letter standing in its square, moved by these shares of its size.
    fn upright(&self, ch: char, at: Pos2, size: f32, dx: f32, dy: f32) {
        let g = self.painter.layout_no_wrap(ch.to_string(), self.font(size), self.color);
        self.painter.galley(pos2(at.x - g.size().x / 2.0 + dx * size, at.y - self.mid * size + dy * size), g, self.color);
    }

    /// `s` on its side down the page, its middle at `at`: a quarter turn
    /// clockwise about its top left, which then sits at the right.
    fn side(&self, s: &str, at: Pos2, size: f32) {
        let g = self.painter.layout_no_wrap(s.into(), self.font(size), self.color);
        let top = at.y - g.size().x / 2.0;
        self.painter.add(TextShape::new(pos2(at.x + self.mid * size, top), g, self.color).with_angle(FRAC_PI_2));
    }

    /// A letter as it stands in a line, the middle of its square at `at`.
    fn letter(&self, ch: char, at: Pos2, size: f32) {
        if !self.vertical {
            return self.across(&ch.to_string(), at, size, f32::INFINITY);
        }
        match vertical::glyph(ch, self.has) {
            (g, Turn::Upright) => self.upright(g, at, size, 0.0, 0.0),
            (g, Turn::Shift(dx, dy)) => self.upright(g, at, size, dx, dy),
            (g, Turn::Quarter) => self.side(&g.to_string(), at, size),
        }
    }

    /// What fills a square `cell` wide: a letter, two of them (`12`, `PC`
    /// across the page, `。」`), or none.
    fn cell(&self, s: &str, at: Pos2, size: f32, cell: f32) {
        let chars: Vec<char> = s.chars().collect();
        match chars[..] {
            [] => {}
            [a, b] if matches!(a, '。' | '、') && matches!(b, '」' | '』' | '）') => {
                // Both in the one square; across the page the bracket moves
                // over to the right of the stop.
                self.letter(a, at, size);
                self.letter(b, at + vec2(if self.vertical { 0.0 } else { cell * 0.38 }, 0.0), size);
            }
            // Digits across the column (縦中横), a Latin letter upright.
            _ if chars.iter().all(char::is_ascii_alphanumeric) => self.across(s, at, size, if self.vertical { size } else { cell * 0.86 }),
            [c] => self.letter(c, at, size),
            _ => self.across(s, at, size, cell),
        }
    }

    /// Ruby along the line from `a` to `b`, spread over it, or past both
    /// ends if it is longer.
    fn ruby(&self, text: &str, a: Pos2, b: Pos2, size: f32) {
        let n = text.chars().count();
        if n == 0 {
            return;
        }
        let along = b - a;
        let step = (along.length() / n as f32).max(size);
        let dir = along.normalized();
        let mid = a + along / 2.0;
        for (i, ch) in text.chars().enumerate() {
            let k = i as f32 - (n as f32 - 1.0) / 2.0;
            self.letter(ch, mid + dir * (k * step), size);
        }
    }
}

/// Where a token went: its square, or its run along the line.
struct Placed<'t> {
    t: &'t Token,
    r: Rect,
}

/// Where in the text each ruby's letters begin.
fn ruby_starts(doc: &Doc) -> Vec<usize> {
    let mut out = vec![0; doc.rubies.len()];
    for l in doc.paras.iter().flatten().rev() {
        if let Some(r) = l.ruby {
            out[r] = l.at;
        }
    }
    out
}

/// The ruby and the dots of one line, set at `side` across it: the x of a
/// vertical line, the y of a horizontal one.
fn marks(pen: &Pen, doc: &Doc, starts: &[usize], placed: &[Placed], side: f32, ruby: f32, dot: f32) {
    let at = |p: &Placed, share: f32| {
        if pen.vertical {
            pos2(side, p.r.top() + p.r.height() * share)
        } else {
            pos2(p.r.left() + p.r.width() * share, side)
        }
    };
    for p in placed.iter().filter(|p| p.t.dot) {
        let n = p.t.s.chars().count();
        for k in 0..n {
            pen.painter.circle_filled(at(p, (k as f32 + 0.5) / n as f32), dot, pen.color);
        }
    }
    let mut i = 0;
    while i < placed.len() {
        let Some(r) = placed[i].t.ruby else {
            i += 1;
            continue;
        };
        let n = placed[i..].iter().take_while(|p| p.t.ruby == Some(r)).count();
        let (first, last) = (&placed[i], &placed[i + n - 1]);
        let text = layout::ruby_part(&doc.rubies[r], first.t.at - starts[r], last.t.end - first.t.at);
        pen.ruby(&text, at(first, 0.0), at(last, 1.0), ruby);
        i += n;
    }
}

/// Positions `from` plus whole steps that fall in `lo..hi`.
fn every(from: f32, step: f32, lo: f32, hi: f32) -> impl Iterator<Item = f32> {
    let k0 = ((lo - from) / step).ceil() as i32;
    let k1 = ((hi - from) / step).ceil() as i32;
    (k0..k1).map(move |k| from + step * k as f32)
}

/// A room `size` big, scrolled if the window is smaller, the paper in its
/// middle across; `paint` draws it from its top left and says where a click
/// on each spot puts the cursor. A click anywhere else puts it at `end`.
fn scrolled(
    ui: &mut Ui,
    size: Vec2,
    from_right: bool,
    fresh: bool,
    end: usize,
    paint: impl FnOnce(&Painter, Pos2) -> Vec<(Rect, usize)>,
) -> Option<usize> {
    let avail = ui.available_size();
    let mut area = ScrollArea::both().auto_shrink(false).id_salt("fude-clean");
    if fresh {
        area = area.scroll_offset(vec2(if from_right { (size.x - avail.x).max(0.0) } else { 0.0 }, 0.0));
    }
    area.show(ui, |ui| {
        let room = size.max(avail);
        let (rect, response) = ui.allocate_exact_size(room, Sense::click());
        let painter = ui.painter_at(rect);
        let spots = paint(&painter, rect.min + vec2((room.x - size.x) / 2.0, 0.0));
        let spot = |p: Pos2| spots.iter().find(|s| s.0.contains(p)).map(|s| s.1);
        if response.hover_pos().and_then(spot).is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }
        response.clicked().then(|| response.interact_pointer_pos().and_then(spot).unwrap_or(end))
    })
    .inner
}

/// A thin rule across (`along_x`) or down, crisp on the pixels.
fn rule(painter: &Painter, along_x: bool, at: f32, range: (f32, f32), color: Color32) {
    let at = painter.round_to_pixel_center(at);
    let stroke = Stroke::new(1.0, color);
    if along_x {
        painter.hline(range.0..=range.1, at, stroke);
    } else {
        painter.vline(at, range.0..=range.1, stroke);
    }
}

/// Manuscript paper: `rows` lines of `cols` squares to a page, the pages
/// side by side as many as fit, then on down.
fn grid(ui: &mut Ui, sheet: &Sheet, config: &Config, doc: &Doc, end: usize, fresh: bool, pace: Option<&Pace>) -> Drawn {
    let vertical = sheet.dir == Dir::Vertical;
    let (cols, rows) = (sheet.cols.max(1) as usize, sheet.rows.max(1) as usize);
    let lines = genko::lines(doc, &genko::Opt { cols, indent: config.indent, hang: config.hang, merge: config.merge, vertical });
    let pages = lines.len().div_ceil(rows).max(1);
    // A pillar down the middle of a vertical page, with the page's number.
    let pillar = vertical && rows % 2 == 0 && rows >= 12;
    let avail = ui.available_size();
    let (h, w) = (avail.y - 2.0 * PAD, avail.x - 2.0 * PAD);
    let (cf, rf) = (cols as f32, rows as f32);
    let fit =
        if vertical { ((h - 8.0) / (cf + 2.6)).min(w / (rf * 1.3 + 3.4)) } else { ((h - 8.0) / (rf * 1.3 + 2.8)).min(w / (cf + 2.6)) };
    let s = fit.floor().clamp(14.0, 30.0);
    let g = s * 0.3;
    let grid = if vertical {
        vec2(rf * s + (rf - 1.0) * g + if pillar { 1.1 * s + g } else { 0.0 }, cf * s)
    } else {
        vec2(cf * s, rf * s + (rf - 1.0) * g)
    };
    let page = grid + if vertical { vec2(1.8 * s, 2.35 * s) } else { vec2(2.3 * s, 2.5 * s) };
    let per_row = (((w + GAP) / (page.x + GAP)).floor().max(1.0) as usize).min(pages);
    let page_rows = pages.div_ceil(per_row);
    let size = vec2(per_row as f32 * (page.x + GAP) - GAP, page_rows as f32 * (page.y + GAP) - GAP) + Vec2::splat(2.0 * PAD);
    let ink = Ink::of(config.rule);
    let ctx = ui.ctx().clone();
    let family = family(&ctx, config);
    let probe = FontId::new(s, family.clone());
    let has = |ch: char| ctx.fonts_mut(|f| f.has_glyph(&probe, ch));
    let starts = ruby_starts(doc);
    let tuck = config.hang == Hang::In;
    let paint = |painter: &Painter, origin: Pos2| {
        let pen = Pen::new(painter, family.clone(), ink.ink, &has, vertical);
        let mut spots = Vec::new();
        for p in 0..pages {
            let (row, k) = (p / per_row, p % per_row);
            let in_row = if row + 1 == page_rows { pages - row * per_row } else { per_row };
            let row_w = in_row as f32 * (page.x + GAP) - GAP;
            let left = origin.x + PAD + (size.x - 2.0 * PAD - row_w) / 2.0;
            // Vertical pages are read from the right.
            let idx = if vertical { in_row - 1 - k } else { k };
            let at = pos2(left + idx as f32 * (page.x + GAP), origin.y + PAD + row as f32 * (page.y + GAP));
            let page_r = Rect::from_min_size(at, page);
            if !painter.clip_rect().intersects(page_r) {
                continue;
            }
            ink.paper(painter, page_r);
            let grid_r = Rect::from_min_size(at + vec2(0.9 * s, 0.9 * s), grid);
            painter.rect_stroke(grid_r.expand(0.14 * s + 0.75), 0.0, Stroke::new(1.5, ink.rule), StrokeKind::Middle);
            for r in 0..rows {
                let line_r = if vertical {
                    let right = grid_r.right() - r as f32 * (s + g) - if pillar && r >= rows / 2 { 1.1 * s + g } else { 0.0 };
                    Rect::from_min_max(pos2(right - s, grid_r.top()), pos2(right, grid_r.bottom()))
                } else {
                    let top = grid_r.top() + r as f32 * (s + g);
                    Rect::from_min_max(pos2(grid_r.left(), top), pos2(grid_r.right(), top + s))
                };
                if pillar && r == rows / 2 {
                    let right = grid_r.right() - r as f32 * (s + g);
                    hashira(&pen, &ink, Rect::from_min_max(pos2(right - 1.1 * s, grid_r.top()), pos2(right, grid_r.bottom())), s, p + 1);
                }
                let ln = lines.get(p * rows + r);
                spots.extend(line(&pen, &ink, doc, &starts, ln, line_r, s, cols, tuck, end, pace));
            }
            if !pillar {
                let font = FontId::proportional(11.0);
                let no = painter.layout_no_wrap(format!("{} / {pages}", p + 1), font, ink.dim);
                painter.galley(pos2(page_r.center().x - no.size().x / 2.0, page_r.bottom() - 0.4 * s - no.size().y), no, ink.dim);
            }
        }
        spots
    };
    let clicked = scrolled(ui, size, vertical, fresh, end, paint);
    Drawn { clicked, pages, lines: lines.len(), paras: doc.paras.len() }
}

/// One line of the grid in `r`, its rules and what fills it; the spots a
/// click puts the cursor by.
#[allow(clippy::too_many_arguments)]
fn line(
    pen: &Pen,
    ink: &Ink,
    doc: &Doc,
    starts: &[usize],
    ln: Option<&genko::Line>,
    r: Rect,
    s: f32,
    cols: usize,
    tuck: bool,
    end: usize,
    pace: Option<&Pace>,
) -> Vec<(Rect, usize)> {
    let vertical = pen.vertical;
    let painter = pen.painter;
    let fs = s * 0.72;
    if vertical {
        rule(painter, false, r.left(), (r.top(), r.bottom()), ink.rule);
        rule(painter, false, r.right(), (r.top(), r.bottom()), ink.rule);
    } else {
        rule(painter, true, r.top(), (r.left(), r.right()), ink.rule);
        rule(painter, true, r.bottom(), (r.left(), r.right()), ink.rule);
    }
    let cell = |c: usize| {
        if vertical {
            Rect::from_min_size(pos2(r.left(), r.top() + c as f32 * s), vec2(s, s))
        } else {
            Rect::from_min_size(pos2(r.left() + c as f32 * s, r.top()), vec2(s, s))
        }
    };
    for c in 1..cols {
        let rc = cell(c);
        if vertical {
            rule(painter, true, rc.top(), (r.left(), r.right()), ink.rule);
        } else {
            rule(painter, false, rc.left(), (r.top(), r.bottom()), ink.rule);
        }
    }
    let Some(ln) = ln else {
        return vec![(r, end)];
    };
    let mut spots = Vec::new();
    let mut placed = Vec::new();
    let last_end = ln.cells.last().map_or(end, |t| t.end);
    for c in 0..cols {
        let rc = cell(c);
        let Some(t) = ln.cells.get(c) else {
            spots.push((rc, last_end));
            continue;
        };
        spots.push((rc, if t.s.is_empty() { t.at } else { t.end }));
        match &ln.hang {
            // The stop that would open the next line shares the last square
            // with the letter before it.
            Some(h) if tuck && c + 1 == cols => {
                let shift = if vertical { vec2(0.0, -0.1 * s) } else { vec2(-0.1 * s, 0.0) };
                pen.cell(&t.s, rc.center() + shift, fs * 0.92, s);
                let small = fs * 0.6;
                let at = if vertical {
                    pos2(rc.right() - 0.216 * s, rc.bottom() - 0.0864 * s)
                } else {
                    pos2(rc.right() - 0.0648 * s, rc.bottom() - 0.216 * s)
                };
                pen.cell(&h.s, at, small, small);
                spots.last_mut().unwrap().1 = h.end;
                placed.push(Placed { t, r: rc });
                placed.push(Placed { t: h, r: Rect::from_center_size(at, Vec2::splat(small)) });
                continue;
            }
            _ => pen.cell(&t.s, rc.center(), fs, s),
        }
        if !t.s.is_empty() {
            placed.push(Placed { t, r: rc });
        }
    }
    if let (Some(h), false) = (&ln.hang, tuck) {
        // Past the end of the line, outside the grid.
        let rc = if vertical {
            Rect::from_min_size(pos2(r.left(), r.bottom() + 1.0), vec2(s, s))
        } else {
            Rect::from_min_size(pos2(r.right() + 1.0, r.top()), vec2(s, s))
        };
        pen.cell(&h.s, rc.center(), fs, s);
        spots.push((rc, h.end));
        placed.push(Placed { t: h, r: rc });
    }
    let g = s * 0.3;
    let side = if vertical { r.right() + g / 2.0 } else { r.top() - g / 2.0 };
    marks(pen, doc, starts, &placed, side, s * 0.28, fs * 0.09);
    if let Some(pace) = pace {
        // Over the grid or before it; the stops on the rule after the line,
        // away from the ruby.
        let (from, most, side) = if vertical {
            (pos2(r.center().x, r.top() - 0.25 * s), vec2(0.0, -0.55 * s), r.left())
        } else {
            (pos2(r.left() - 0.25 * s, r.center().y), vec2(-0.55 * s, 0.0), r.bottom())
        };
        speed(pen, ink, pace, &placed, from, most, side);
    }
    spots
}

/// How fast the line of `placed` went: a bar in the margin from `from`, as
/// long as `most` at twice the piece's own speed; and a dot along `side`
/// after each letter the hand stopped at.
fn speed(pen: &Pen, ink: &Ink, pace: &Pace, placed: &[Placed], from: Pos2, most: Vec2, side: f32) {
    let (Some(first), Some(last)) = (placed.first(), placed.last()) else {
        return;
    };
    let (at, end) = (first.t.at, last.t.end);
    if let Some(share) = pace.share(at, end) {
        // Even the slowest line shows a stub.
        let share = share.max(2.0 / most.length());
        pen.painter.line_segment([from, from + most * share], Stroke::new(3.0, ink.dim.gamma_multiply(0.7)));
    }
    for &i in pace.stops().iter().filter(|&&i| at < i && i <= end) {
        if let Some(p) = placed.iter().rev().find(|p| p.t.at < i) {
            let c = if pen.vertical { pos2(side, p.r.bottom()) } else { pos2(p.r.right(), side) };
            pen.painter.circle_filled(c, 2.2, ink.stop);
        }
    }
}

/// The pillar of a vertical page: the fishtail and the page's number.
fn hashira(pen: &Pen, ink: &Ink, r: Rect, s: f32, page: usize) {
    let painter = pen.painter;
    rule(painter, false, r.left(), (r.top(), r.bottom()), ink.rule);
    rule(painter, false, r.right(), (r.top(), r.bottom()), ink.rule);
    let (w, h) = (0.66 * s, 0.5 * s);
    let o = pos2(r.center().x - w / 2.0, r.top() + 2.2 * s);
    // The fishtail is not convex: its two halves are.
    for half in [[(0.0, 0.0), (0.5, 0.0), (0.5, 0.48), (0.0, 1.0)], [(0.5, 0.0), (1.0, 0.0), (1.0, 1.0), (0.5, 0.48)]] {
        let points = half.iter().map(|&(x, y)| o + vec2(x * w, y * h)).collect();
        painter.add(Shape::convex_polygon(points, ink.rule, Stroke::NONE));
    }
    let size = 0.46 * s;
    let number = Pen { painter, family: pen.family.clone(), color: ink.dim, mid: pen.mid, has: pen.has, vertical: false };
    number.across(&page.to_string(), pos2(r.center().x, r.bottom() - 0.6 * s - size / 2.0), size, s);
}

/// A ruled, graph or plain sheet: the lines broken where it runs out, one
/// long sheet.
#[allow(clippy::too_many_arguments)]
fn lined(ui: &mut Ui, sheet: &Sheet, config: &Config, doc: &Doc, end: usize, empty: &str, fresh: bool, pace: Option<&Pace>) -> Drawn {
    let vertical = sheet.dir == Dir::Vertical;
    let graph = sheet.paper == Paper::Graph;
    let avail = ui.available_size();
    let ctx = ui.ctx().clone();
    let family = family(&ctx, config);
    let font = FontId::new(FONT, family.clone());
    let measure = ui.painter().clone();
    let adv = |s: &str| measure.layout_no_wrap(s.into(), font.clone(), Color32::WHITE).size().x / FONT;
    let room = if sheet.wrap > 0 {
        sheet.wrap as f32
    } else if vertical {
        (avail.y - 2.0 * PAD - 2.0 * MARGIN.y) / FONT
    } else {
        (avail.x - 2.0 * PAD - 2.0 * MARGIN.x).min(40.0 * FONT) / FONT
    }
    .max(1.0);
    let paras = wrap::layout(doc, &wrap::Opt { vertical, graph, indent: config.indent }, room, &adv);
    let rows: Vec<&wrap::Row> = paras.iter().flatten().collect();
    let n = rows.len().max(1) as f32;
    let paper = if vertical {
        vec2(n * PITCH + 2.0 * MARGIN.x, room * FONT + 2.0 * MARGIN.y)
    } else {
        vec2(room * FONT + 2.0 * MARGIN.x, n * PITCH + 2.0 * MARGIN.y)
    };
    let size = paper + Vec2::splat(2.0 * PAD);
    let ink = Ink::of(config.rule);
    let has = |ch: char| ctx.fonts_mut(|f| f.has_glyph(&font, ch));
    let starts = ruby_starts(doc);
    let paint = |painter: &Painter, origin: Pos2| {
        let pen = Pen::new(painter, family.clone(), ink.ink, &has, vertical);
        let sheet_r = Rect::from_min_size(origin + Vec2::splat(PAD), paper);
        ink.paper(painter, sheet_r);
        let text_r = sheet_r.shrink2(MARGIN);
        let (xs, ys) = ((sheet_r.left(), sheet_r.right()), (sheet_r.top(), sheet_r.bottom()));
        match (sheet.paper, vertical) {
            (Paper::Ruled, false) => {
                every(text_r.top() + PITCH - 1.0, PITCH, ys.0, ys.1).for_each(|y| rule(painter, true, y, xs, ink.rule.gamma_multiply(0.55)))
            }
            (Paper::Ruled, true) => {
                every(text_r.right(), PITCH, xs.0, xs.1).for_each(|x| rule(painter, false, x, ys, ink.rule.gamma_multiply(0.55)))
            }
            (Paper::Graph, _) => {
                let (from_x, from_y) = if vertical { (text_r.right() - 11.0, text_r.top()) } else { (text_r.left(), text_r.top() + 10.0) };
                let col = ink.rule.gamma_multiply(0.32);
                every(from_x, FONT, xs.0, xs.1).for_each(|x| rule(painter, false, x, ys, col));
                every(from_y, FONT, ys.0, ys.1).for_each(|y| rule(painter, true, y, xs, col));
            }
            _ => {}
        }
        if rows.is_empty() {
            let note = Pen::new(painter, FontFamily::Proportional, ink.dim, &has, vertical);
            if vertical {
                let x = text_r.right() - PITCH / 2.0;
                for (i, ch) in empty.chars().enumerate() {
                    note.letter(ch, pos2(x, text_r.top() + 13.0 * (i as f32 + 0.5)), 13.0);
                }
            } else {
                let g = painter.layout_no_wrap(empty.into(), FontId::proportional(13.0), ink.dim);
                painter.galley(pos2(text_r.left(), text_r.top() + (PITCH - g.size().y) / 2.0), g, ink.dim);
            }
            return vec![(sheet_r, end)];
        }
        let mut spots = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let band = if vertical {
                let right = text_r.right() - i as f32 * PITCH;
                Rect::from_min_max(pos2(right - PITCH, sheet_r.top()), pos2(right, sheet_r.bottom()))
            } else {
                let top = text_r.top() + i as f32 * PITCH;
                Rect::from_min_max(pos2(sheet_r.left(), top), pos2(sheet_r.right(), top + PITCH))
            };
            if !painter.clip_rect().intersects(band) {
                continue;
            }
            spots.extend(row_of(&pen, &ink, doc, &starts, row, band, text_r, pace));
        }
        spots
    };
    let clicked = scrolled(ui, size, vertical, fresh, end, paint);
    Drawn { clicked, pages: 0, lines: rows.len(), paras: doc.paras.len() }
}

/// One line of a sheet in its `band` (the sheet's width or height), its
/// letters from the start of `text_r`; the spots a click puts the cursor by.
#[allow(clippy::too_many_arguments)]
fn row_of(
    pen: &Pen,
    ink: &Ink,
    doc: &Doc,
    starts: &[usize],
    row: &wrap::Row,
    band: Rect,
    text_r: Rect,
    pace: Option<&Pace>,
) -> Vec<(Rect, usize)> {
    let vertical = pen.vertical;
    let mut at = if vertical { text_r.top() } else { text_r.left() } + row.lead * FONT;
    let mut spots = Vec::new();
    let mut placed = Vec::new();
    // The run of a piece along the line, across the whole band.
    let run = |from: f32, to: f32| {
        if vertical {
            Rect::from_min_max(pos2(band.left(), from), pos2(band.right(), to))
        } else {
            Rect::from_min_max(pos2(from, band.top()), pos2(to, band.bottom()))
        }
    };
    let first = row.pieces.first().map_or(0, |p| p.t.at);
    spots.push((run(f32::NEG_INFINITY, at), first));
    for p in &row.pieces {
        let len = p.w * FONT;
        let r = run(at, at + len);
        let mid = r.center();
        match (p.kind, vertical) {
            (wrap::Kind::Space, _) => {}
            (wrap::Kind::Char, true) => pen.letter(p.t.s.chars().next().unwrap_or(' '), mid, FONT),
            (wrap::Kind::Word, true) => pen.side(&p.t.s, mid, FONT),
            (_, true) => pen.across(&p.t.s, mid, FONT, FONT),
            (_, false) => pen.across(&p.t.s, mid, FONT, f32::INFINITY),
        }
        // A word is clicked letter by letter.
        let n = p.t.s.chars().count().max(1);
        for k in 0..n {
            let step = len / n as f32;
            spots.push((
                run(at + step * k as f32, at + step * (k + 1) as f32),
                if p.kind == wrap::Kind::Word { p.t.at + k + 1 } else { p.t.end },
            ));
        }
        if p.kind != wrap::Kind::Space {
            placed.push(Placed {
                t: &p.t,
                r: if vertical { r.shrink2(vec2((PITCH - FONT) / 2.0, 0.0)) } else { r.shrink2(vec2(0.0, (PITCH - FONT) / 2.0)) },
            });
        }
        at += len;
    }
    let last = row.pieces.last().map_or(first, |p| p.t.end);
    spots.push((run(at, f32::INFINITY), last));
    let side = if vertical { band.center().x + 15.0 } else { band.top() + 5.0 };
    marks(pen, doc, starts, &placed, side, 10.0, FONT * 0.09);
    if let Some(pace) = pace {
        let (from, most, side) = if vertical {
            (pos2(band.center().x, text_r.top() - 6.0), vec2(0.0, -26.0), band.center().x - 15.0)
        } else {
            (pos2(text_r.left() - 6.0, band.center().y), vec2(-32.0, 0.0), band.bottom() - 5.0)
        };
        speed(pen, ink, pace, &placed, from, most, side);
    }
    spots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_paper_is_a_grid_unless_the_text_is_latin() {
        let config = Config::default();
        assert_eq!(config.paper, Paper::Auto);
        let s = Sheet::choose(&config, "吾輩は猫である。");
        assert_eq!((s.paper, s.dir), (Paper::Genko, config.paper_dir));
        let s = Sheet::choose(&config, "This is a pen.");
        assert_eq!((s.paper, s.dir), (Paper::Ruled, Dir::Horizontal));
        let config = Config { paper: Paper::Graph, paper_dir: Dir::Vertical, ..Config::default() };
        let s = Sheet::choose(&config, "This is a pen.");
        assert_eq!((s.paper, s.dir), (Paper::Graph, Dir::Vertical));
    }

    #[test]
    fn rules_fall_on_the_steps_inside_the_range() {
        assert_eq!(every(10.0, 20.0, 0.0, 70.0).collect::<Vec<_>>(), [10.0, 30.0, 50.0]);
        assert_eq!(every(50.0, 20.0, 0.0, 70.0).collect::<Vec<_>>(), [10.0, 30.0, 50.0]);
        assert_eq!(every(0.0, 20.0, 0.0, 40.0).collect::<Vec<_>>(), [0.0, 20.0]);
    }
}
