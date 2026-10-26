//! The writing band on the screen: the line typed so far sliding past a
//! fixed input point, the caret, the input method's composition at it, the
//! hint by it, the topic over it, the band that says a limit was reached and
//! the bars along the bottom. What the keys and the input method send becomes an [`Edit`] here.
//! The mock's `#write` (`fude-mock/index.html`), drawn with egui.

use std::f32::consts::FRAC_PI_2;
use std::time::{Duration, Instant};

use egui::epaint::TextShape;
use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{pos2, vec2, Align, Color32, Event, FontId, Key, Painter, Pos2, Rect, Stroke};

use crate::band::{self, Band, Edit, Hit};
use crate::config::{Config, Dir, Point, Show};
use crate::fonts;
use crate::vertical::{self, Turn};

/// How long a letter keeps the caret steady and the hint quiet, in seconds.
const TYPING: f64 = 0.9;

/// The caret's blink: shown for the first half of this, in seconds.
const BLINK: f64 = 1.06;

/// The line fades out towards the window's edge: how much of it shows at
/// these shares of the way from the edge to the input point.
const MASK: [(f32, f32); 4] = [(0.0, 0.0), (0.3, 0.16), (0.82, 1.0), (1.0, 1.0)];

/// What the band is drawn from.
pub struct View<'a> {
    pub band: &'a Band,
    pub config: &'a Config,
    /// What the input method is composing, drawn at the cursor.
    pub preedit: &'a str,
    pub now: Instant,
    /// The last edit, in egui's time.
    pub typed_at: f64,
    /// The key that opens the settings, for the hint.
    pub settings_key: &'a str,
    pub table: &'a ito_i18n::Table,
    /// The piece's topic, shown over the input point.
    pub topic: Option<&'a str>,
}

/// The input point: where the letters before the cursor end.
fn point(rect: Rect, config: &Config) -> Pos2 {
    let f = match config.point {
        Point::Center => 0.5,
        Point::Start => 0.3,
        Point::End => 0.7,
    };
    match config.dir {
        Dir::Horizontal => pos2(rect.left() + rect.width() * f, rect.center().y),
        Dir::Vertical => pos2(rect.center().x, rect.top() + rect.height() * f),
    }
}

/// The caret, just past the input point; the input method's window opens by
/// it.
pub fn caret_at(rect: Rect, config: &Config) -> Rect {
    let p = point(rect, config);
    let half = config.font_size as f32 * 0.575;
    match config.dir {
        Dir::Horizontal => Rect::from_min_max(pos2(p.x + 2.0, p.y - half), pos2(p.x + 4.0, p.y + half)),
        Dir::Vertical => Rect::from_min_max(pos2(p.x - half, p.y + 2.0), pos2(p.x + half, p.y + 4.0)),
    }
}

/// The edit `e` asks for, if any. The arrows along the band move the cursor:
/// left and right on a horizontal band, up and down on a vertical one.
pub fn edit_of(e: &Event, dir: Dir) -> Option<Edit> {
    let text = |s: String| (!s.is_empty()).then_some(Edit::Text(s));
    match e {
        Event::Text(s) => text(s.chars().filter(|c| !c.is_control()).collect()),
        Event::Paste(s) => {
            text(s.chars().map(|c| if c == '\t' { ' ' } else { c }).filter(|&c| !c.is_control() || c == '\n' || c == '\r').collect())
        }
        Event::Ime(egui::ImeEvent::Commit(s)) => text(s.clone()),
        Event::Key { key, pressed: true, modifiers, .. } if !modifiers.alt => {
            let (back, on) = match dir {
                Dir::Horizontal => (Key::ArrowLeft, Key::ArrowRight),
                Dir::Vertical => (Key::ArrowUp, Key::ArrowDown),
            };
            let step = |d| if modifiers.command { Edit::Jump(d) } else { Edit::Step(d) };
            match *key {
                Key::Enter if !modifiers.command => Some(Edit::Paragraph),
                Key::Backspace if !modifiers.command => Some(Edit::Backspace),
                Key::Delete if !modifiers.command => Some(Edit::Delete),
                Key::Home => Some(Edit::Home),
                Key::End => Some(Edit::End),
                k if k == back => Some(step(-1)),
                k if k == on => Some(step(1)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether a letter came just now: the caret holds still and the hint goes
/// quiet.
fn typing(time: f64, typed_at: f64) -> bool {
    time - typed_at < TYPING
}

/// What a line is set in.
struct Faces {
    letter: FontId,
    /// The small ¶ at a paragraph's end.
    mark: FontId,
    ink: Color32,
}

/// The band, the hint, the caret, the limit's band and the bars, in `rect`.
pub fn draw(ui: &egui::Ui, rect: Rect, v: &View) {
    let ctx = ui.ctx();
    let c = ito_theme::colors();
    let fs = v.config.font_size as f32;
    let f = Faces { letter: FontId::new(fs, fonts::mincho(ctx)), mark: FontId::proportional(fs * 0.55), ink: c.strong() };
    let p = point(rect, v.config);
    let painter = ui.painter().with_clip_rect(rect);
    let (before, after) = v.band.split();
    // Enough letters to fill the room even if every one is narrow.
    let fit = |room: f32| ((room.max(0.0) / (fs * 0.25)) as usize + 2).min(600);
    let past =
        &before[before.len().saturating_sub(fit(if v.config.dir == Dir::Horizontal { p.x - rect.left() } else { p.y - rect.top() }))..];
    match v.config.dir {
        Dir::Horizontal => {
            let next = &after[..after.len().min(fit(rect.right() - p.x))];
            let (job, tail) = line(&f, past, v.preedit);
            let g = painter.layout_job(job);
            painter.galley(pos2(p.x - tail - g.size().x, p.y - g.size().y / 2.0), g, f.ink);
            let g = painter.layout_job(line(&f, next, "").0);
            painter.galley(pos2(p.x + 5.0, p.y - g.size().y / 2.0), g, f.ink);
            let (top, bottom) = (p.y - fs * 1.2, p.y + fs * 1.2);
            fade(&painter, Rect::from_min_max(pos2(rect.left(), top), pos2(p.x, bottom)), true, true, c.bg);
            fade(&painter, Rect::from_min_max(pos2(p.x + 5.0, top), pos2(rect.right(), bottom)), true, false, c.bg);
        }
        Dir::Vertical => {
            let next = &after[..after.len().min(fit(rect.bottom() - p.y))];
            let has = |g: char| ctx.fonts_mut(|fonts| fonts.has_glyph(&f.letter, g));
            let mut up = pieces(past, false, &has);
            up.extend(pieces(&v.preedit.chars().collect::<Vec<_>>(), true, &has));
            column(&painter, &f, &up, p, rect.top(), true);
            column(&painter, &f, &pieces(next, false, &has), pos2(p.x, p.y + 5.0), rect.bottom(), false);
            let (left, right) = (p.x - fs * 1.2, p.x + fs * 1.2);
            fade(&painter, Rect::from_min_max(pos2(left, rect.top()), pos2(right, p.y)), false, true, c.bg);
            fade(&painter, Rect::from_min_max(pos2(left, p.y + 5.0), pos2(right, rect.bottom())), false, false, c.bg);
        }
    }
    let time = ctx.input(|i| i.time);
    hint(ui, &painter, p, v, typing(time, v.typed_at));
    topic(ui, &painter, rect, p, v, typing(time, v.typed_at));
    caret(ui, &painter, caret_at(rect, v.config), time, v.typed_at);
    bars(ui, &painter, rect, v);
    banner(ui, rect, v);
    if v.band.running() {
        let ms = v.band.elapsed(v.now).as_millis() as u64;
        ctx.request_repaint_after(Duration::from_millis(1000 - ms % 1000));
    }
}

/// A horizontal line of `chars`, each paragraph's end a small ¶, then
/// `preedit` underlined. Also the room to leave after it: a ¶ keeps a little
/// space on both sides, even at the end.
fn line(f: &Faces, chars: &[char], preedit: &str) -> (LayoutJob, f32) {
    let c = ito_theme::colors();
    let plain = TextFormat { font_id: f.letter.clone(), color: f.ink, ..Default::default() };
    let pil = TextFormat { font_id: f.mark.clone(), color: c.faint(), valign: Align::Center, ..Default::default() };
    let pad = f.mark.size * 0.45;
    let mut job = LayoutJob::default();
    // The room a ¶ just before leaves.
    let mut lead = 0.0;
    let mut runs = chars.split(|&ch| ch == '\n').peekable();
    while let Some(run) = runs.next() {
        if !run.is_empty() {
            job.append(&run.iter().collect::<String>(), lead, plain.clone());
            lead = 0.0;
        }
        if runs.peek().is_some() {
            job.append("¶", lead + pad, pil.clone());
            lead = pad;
        }
    }
    if !preedit.is_empty() {
        job.append(preedit, lead, TextFormat { color: c.fg, underline: Stroke::new(2.0, c.blue), ..plain });
        lead = 0.0;
    }
    (job, lead)
}

/// One step down a vertical line.
enum Piece {
    /// A letter standing up, moved by these shares of the size to the right
    /// and down.
    Upright(char, f32, f32),
    /// Letters lying on their side, as many as come in a row.
    Side(String),
    /// A paragraph's end.
    Mark,
}

/// `chars` as they stand in a vertical line; `preedit` marks them as the
/// input method's.
fn pieces(chars: &[char], preedit: bool, has: &impl Fn(char) -> bool) -> Vec<(Piece, bool)> {
    let mut out: Vec<(Piece, bool)> = Vec::new();
    for &ch in chars {
        if ch == '\n' {
            out.push((Piece::Mark, preedit));
            continue;
        }
        match vertical::glyph(ch, has) {
            (g, Turn::Quarter) => match out.last_mut() {
                Some((Piece::Side(s), _)) => s.push(g),
                _ => out.push((Piece::Side(g.to_string()), preedit)),
            },
            (g, Turn::Upright) => out.push((Piece::Upright(g, 0.0, 0.0), preedit)),
            (g, Turn::Shift(dx, dy)) => out.push((Piece::Upright(g, dx, dy), preedit)),
        }
    }
    out
}

/// A vertical line of `pieces` centred on `at.x`: going `up` from `at.y` (the
/// last piece nearest) until past `end`, or down from it.
fn column(painter: &Painter, f: &Faces, pieces: &[(Piece, bool)], at: Pos2, end: f32, up: bool) {
    let c = ito_theme::colors();
    let fs = f.letter.size;
    let mid = middle(painter, &f.letter);
    let order: Vec<&(Piece, bool)> = if up { pieces.iter().rev().collect() } else { pieces.iter().collect() };
    let mut y = at.y;
    for (piece, preedit) in order {
        if (up && y < end) || (!up && y > end) {
            break;
        }
        let color = if *preedit { c.fg } else { f.ink };
        let g = match piece {
            Piece::Upright(ch, ..) => painter.layout_no_wrap(ch.to_string(), f.letter.clone(), color),
            Piece::Side(s) => painter.layout_no_wrap(s.clone(), f.letter.clone(), color),
            Piece::Mark => painter.layout_no_wrap("¶".into(), f.mark.clone(), c.faint()),
        };
        let size = g.size();
        let adv = match piece {
            Piece::Upright(..) => fs,
            Piece::Side(_) => size.x,
            Piece::Mark => f.mark.size * 1.9,
        };
        let top = if up { y - adv } else { y };
        y = if up { top } else { y + adv };
        match piece {
            Piece::Upright(_, dx, dy) => painter.galley(pos2(at.x - size.x / 2.0 + dx * fs, top + fs / 2.0 - mid + dy * fs), g, color),
            // A quarter turn clockwise about its top left, which then sits at
            // the right, its middle on the line.
            Piece::Side(_) => {
                painter.add(TextShape::new(pos2(at.x + mid, top), g, color).with_angle(FRAC_PI_2));
            }
            Piece::Mark => painter.galley(pos2(at.x - size.x / 2.0, top + (adv - size.y) / 2.0), g, c.faint()),
        }
        if *preedit {
            painter.vline(at.x + fs * 0.62, top..=top + adv, Stroke::new(2.0, c.blue));
        }
    }
}

/// How far down from the top of a laid out letter the middle of an ideograph
/// falls. A Mincho line is taller than its size and more of it is above the
/// letters, so its box's own middle would set them too high.
pub fn middle(painter: &Painter, font: &FontId) -> f32 {
    let g = painter.layout_no_wrap("国".into(), font.clone(), Color32::WHITE);
    let row = &g.rows[0];
    match row.glyphs.first() {
        Some(gl) if !gl.uv_rect.is_nothing() => row.pos.y + gl.pos.y + gl.uv_rect.offset.y + gl.uv_rect.size.y / 2.0,
        _ => g.size().y / 2.0,
    }
}

/// How far below the middle of an ideograph in `font` its baseline is: where
/// a renderer that sets text on its baseline is to put it.
pub fn sink(painter: &Painter, font: &FontId) -> f32 {
    let g = painter.layout_no_wrap("国".into(), font.clone(), Color32::WHITE);
    match g.rows[0].glyphs.first() {
        Some(gl) if !gl.uv_rect.is_nothing() => -(gl.uv_rect.offset.y + gl.uv_rect.size.y / 2.0),
        _ => font.size * 0.38,
    }
}

/// The background laid over `r` thinning out along the line (`horizontal`
/// or not), from opaque at its far end to clear by the input point:
/// `far_first` when the far end is the left or the top.
fn fade(painter: &Painter, r: Rect, horizontal: bool, far_first: bool, bg: Color32) {
    let mut mesh = egui::Mesh::default();
    for (i, &(t, m)) in MASK.iter().enumerate() {
        let t = if far_first { t } else { 1.0 - t };
        let col = bg.gamma_multiply(1.0 - m);
        let (a, b) = if horizontal {
            let x = r.left() + r.width() * t;
            (pos2(x, r.top()), pos2(x, r.bottom()))
        } else {
            let y = r.top() + r.height() * t;
            (pos2(r.left(), y), pos2(r.right(), y))
        };
        mesh.colored_vertex(a, col);
        mesh.colored_vertex(b, col);
        if i > 0 {
            let k = 2 * i as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k, k + 1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// How a part of the hint is said.
#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Dim,
    Warn,
    /// The cursor is back in the text.
    Mid,
    /// A key, set as a key cap right after the part before.
    Key,
}

/// The hint by the input point: the letters and the time against their
/// limits, or before the first letter what the limits are and where to set
/// them. It goes quiet while letters come.
fn hint(ui: &egui::Ui, painter: &Painter, p: Pos2, v: &View, busy: bool) {
    let ctx = ui.ctx();
    let c = ito_theme::colors();
    let t = v.table;
    let tr = |k: &'static str| t.tr(k).to_owned();
    let trf = |k: &'static str, a: &[&str]| t.trf(k, a);
    let b = v.band;
    let (n, cc, tc, el) = (b.count(), b.char_cap(), b.time_cap(), b.elapsed(v.now));
    let show = v.config.show;
    let mut parts: Vec<(String, Tone)> = Vec::new();
    if !b.started() {
        parts.push((if cc > 0 { trf("write.hint_chars_of", &["0", &cc.to_string()]) } else { trf("write.hint_chars", &["0"]) }, Tone::Dim));
        if !tc.is_zero() {
            parts.push((band::fmt(tc, true), Tone::Dim));
        }
        parts.push((if tc.is_zero() { tr("write.hint_time_counts") } else { tr("write.hint_time_moves") }, Tone::Dim));
        parts.push((tr("write.hint_limits"), Tone::Dim));
        parts.push((v.settings_key.to_owned(), Tone::Key));
    } else if show != Show::Nothing {
        if show != Show::Time {
            let n_s = n.to_string();
            parts.push(if cc > 0 {
                let near = n as f64 >= f64::from(cc) * 0.9;
                (trf("write.hint_chars_of", &[&n_s, &cc.to_string()]), if near { Tone::Warn } else { Tone::Dim })
            } else {
                (trf("write.hint_chars", &[&n_s]), Tone::Dim)
            });
        }
        if show != Show::Chars {
            parts.push(if tc.is_zero() {
                (band::fmt(el, false), Tone::Dim)
            } else if el > tc {
                (trf("write.hint_over", &[&band::fmt(tc, false), &band::fmt(el - tc, false)]), Tone::Warn)
            } else {
                let left = tc - el;
                (trf("write.hint_left", &[&band::fmt(left, true)]), if left <= Duration::from_secs(30) { Tone::Warn } else { Tone::Dim })
            });
        }
    }
    if b.back() > 0 {
        let rest = b.rest();
        parts.insert(0, (if rest > 0 { trf("write.hint_back", &[&rest.to_string()]) } else { tr("write.hint_end") }, Tone::Mid));
    }
    if parts.is_empty() {
        return;
    }
    let font = FontId::proportional(ito_theme::zoomed(ctx, 12.5));
    let small = FontId::proportional(ito_theme::zoomed(ctx, 10.5));
    let gap = font.size * 0.55;
    let format = |font_id: &FontId, color| TextFormat { font_id: font_id.clone(), color, valign: Align::Center, ..Default::default() };
    let mut job = LayoutJob::default();
    for (i, (s, tone)) in parts.iter().enumerate() {
        if *tone == Tone::Key {
            job.append(&format!(" {s} "), font.size * 0.35, TextFormat { background: c.raised(), ..format(&small, c.dim) });
            continue;
        }
        if i > 0 {
            job.append("·", gap, format(&font, c.faint()));
        }
        let color = match tone {
            Tone::Warn => c.wait,
            Tone::Mid => c.blue,
            _ => c.dim,
        };
        job.append(s, if i > 0 { gap } else { 0.0 }, format(&font, color));
    }
    let g = painter.layout_job(job);
    let fs = v.config.font_size as f32;
    let at = match v.config.dir {
        Dir::Horizontal => pos2(p.x - g.size().x / 2.0, p.y + fs * 1.05),
        Dir::Vertical => pos2(p.x + fs * 1.1, p.y - g.size().y / 2.0),
    };
    let mut painter = painter.clone();
    painter.multiply_opacity(1.0 - 0.45 * ctx.animate_bool_with_time(egui::Id::new("fude-hint-quiet"), busy, 0.3));
    painter.galley(at, g, c.dim);
}

/// The piece's topic, faint over the input point (left of it on a vertical
/// band), cut with … where the band runs out. It goes quieter while letters
/// come.
fn topic(ui: &egui::Ui, painter: &Painter, rect: Rect, p: Pos2, v: &View, busy: bool) {
    let Some(topic) = v.topic else {
        return;
    };
    let ctx = ui.ctx();
    let c = ito_theme::colors();
    let fs = v.config.font_size as f32;
    let width = match v.config.dir {
        Dir::Horizontal => rect.width() * 0.6,
        Dir::Vertical => p.x - fs * 1.1 - rect.left() - 16.0,
    };
    let font = FontId::proportional(ito_theme::zoomed(ctx, 14.0));
    let mut job = LayoutJob::simple_singleline(v.table.trf("write.topic", &[topic]), font, c.faint());
    job.wrap = TextWrapping::truncate_at_width(width.max(1.0));
    let g = painter.layout_job(job);
    let at = match v.config.dir {
        Dir::Horizontal => pos2(p.x - g.size().x / 2.0, p.y - fs * 1.05 - g.size().y),
        Dir::Vertical => pos2(p.x - fs * 1.1 - g.size().x, p.y - g.size().y / 2.0),
    };
    let mut painter = painter.clone();
    painter.multiply_opacity(1.0 - 0.6 * ctx.animate_bool_with_time(egui::Id::new("fude-topic-quiet"), busy, 0.3));
    painter.galley(at, g, c.faint());
}

/// The caret at `r`: steady while letters come, then blinking. With the
/// window in the background it stands still and asks for no frames.
fn caret(ui: &egui::Ui, painter: &Painter, r: Rect, time: f64, typed_at: f64) {
    let ctx = ui.ctx();
    let focused = ctx.input(|i| i.focused);
    let busy = typing(time, typed_at);
    let base = if typed_at.is_finite() { typed_at + TYPING } else { 0.0 };
    let phase = (time - base).rem_euclid(BLINK);
    let half = BLINK / 2.0;
    if !focused || busy || phase < half {
        painter.rect_filled(r, 0.0, ito_theme::colors().blue);
    }
    if focused {
        let next = if busy {
            base - time
        } else if phase < half {
            half - phase
        } else {
            BLINK - phase
        };
        ctx.request_repaint_after_secs(next as f32);
    }
}

/// The bars along the bottom: the letters against their limit and, below
/// them, the time against its own.
fn bars(ui: &egui::Ui, painter: &Painter, rect: Rect, v: &View) {
    let ctx = ui.ctx();
    let c = ito_theme::colors();
    let b = v.band;
    let show = v.config.show;
    let (cc, tc) = (b.char_cap(), b.time_cap());
    let time =
        (!tc.is_zero() && matches!(show, Show::Both | Show::Time)).then(|| (b.elapsed(v.now).as_secs_f32() / tc.as_secs_f32()).min(1.0));
    let chars = (cc > 0 && matches!(show, Show::Both | Show::Chars)).then(|| (b.count() as f32 / cc as f32).min(1.0));
    let mut bottom = rect.bottom();
    for (name, frac, color) in [("fude-bar-time", time, c.wait.gamma_multiply(0.6)), ("fude-bar-chars", chars, c.blue.gamma_multiply(0.75))]
    {
        let Some(frac) = frac else { continue };
        let w = rect.width() * ctx.animate_value_with_time(egui::Id::new(name), frac, 0.3);
        painter.rect_filled(Rect::from_min_size(pos2(rect.left(), bottom - 2.0), vec2(w, 2.0)), 0.0, color);
        bottom -= 4.0;
    }
}

/// What was reached, as the banners say it: "400 字に達しました・15 分たちました".
pub fn hit_words(b: &Band, t: &ito_i18n::Table) -> String {
    let mins = (b.time_cap().as_secs_f64() / 60.0).round().to_string();
    let words: Vec<String> = b
        .hit()
        .iter()
        .map(|h| match h {
            Hit::Chars => t.trf("write.hit_chars", &[&b.char_cap().to_string()]),
            Hit::Time => t.trf("write.hit_time", &[&mins]),
        })
        .collect();
    words.join(t.tr("write.hit_join"))
}

/// The band that slides down from the top when a limit is reached. The
/// writing goes on under it.
fn banner(ui: &egui::Ui, rect: Rect, v: &View) {
    let ctx = ui.ctx();
    let c = ito_theme::colors();
    let t = v.table;
    let tr = |k: &'static str| t.tr(k).to_owned();
    let b = v.band;
    let id = egui::Id::new("fude-limit-band");
    let on = !b.hit().is_empty();
    // The words stay while it slides away.
    let words = if on {
        let words = hit_words(b, t);
        ctx.data_mut(|d| d.insert_temp(id.with("words"), words.clone()));
        words
    } else {
        ctx.data(|d| d.get_temp::<String>(id.with("words"))).unwrap_or_default()
    };
    let shown = ctx.animate_bool_with_time_and_easing(id, on, 0.25, egui::emath::easing::cubic_out);
    if shown <= 0.0 {
        return;
    }
    let h = ito_theme::zoomed(ctx, 34.0);
    let r = Rect::from_min_size(pos2(rect.left(), rect.top() - (1.0 - shown) * h), vec2(rect.width(), h));
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(r, 0.0, ito_theme::mix(c.panel, c.wait, 0.11));
    painter.hline(r.x_range(), r.bottom() - 0.5, Stroke::new(1.0, c.border));
    painter.circle_filled(pos2(r.left() + 20.0, r.center().y), 4.0, c.wait);
    let font = FontId::proportional(ito_theme::zoomed(ctx, 13.0));
    let format = |color| TextFormat { font_id: font.clone(), color, valign: Align::Center, ..Default::default() };
    let mut job = LayoutJob::default();
    job.append(&words, 0.0, format(c.strong()));
    job.append(&tr("write.go_on"), font.size * 0.8, format(c.dim));
    let g = painter.layout_job(job);
    painter.galley(pos2(r.left() + 34.0, r.center().y - g.size().y / 2.0), g, c.dim);
}

#[cfg(test)]
mod tests {
    use egui::Modifiers;

    use super::*;

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
    }

    #[test]
    fn letters_pastes_and_the_input_method_become_text() {
        let h = Dir::Horizontal;
        assert_eq!(edit_of(&Event::Text("あ\u{7}".into()), h), Some(Edit::Text("あ".into())));
        assert_eq!(edit_of(&Event::Text("\u{7}".into()), h), None);
        assert_eq!(edit_of(&Event::Paste("\ta\u{7}\r\nb".into()), h), Some(Edit::Text(" a\r\nb".into())));
        assert_eq!(edit_of(&Event::Ime(egui::ImeEvent::Commit("漢字".into())), h), Some(Edit::Text("漢字".into())));
        assert_eq!(edit_of(&Event::Ime(egui::ImeEvent::Commit(String::new())), h), None);
    }

    #[test]
    fn the_arrows_along_the_band_move_the_cursor() {
        let none = Modifiers::NONE;
        let ctrl = Modifiers { ctrl: true, command: true, ..Default::default() };
        let h = Dir::Horizontal;
        let v = Dir::Vertical;
        assert_eq!(edit_of(&key(Key::ArrowLeft, none), h), Some(Edit::Step(-1)));
        assert_eq!(edit_of(&key(Key::ArrowRight, ctrl), h), Some(Edit::Jump(1)));
        assert_eq!(edit_of(&key(Key::ArrowUp, none), h), None);
        assert_eq!(edit_of(&key(Key::ArrowUp, none), v), Some(Edit::Step(-1)));
        assert_eq!(edit_of(&key(Key::ArrowDown, ctrl), v), Some(Edit::Jump(1)));
        assert_eq!(edit_of(&key(Key::ArrowLeft, none), v), None);
        assert_eq!(edit_of(&key(Key::Enter, none), h), Some(Edit::Paragraph));
        assert_eq!(edit_of(&key(Key::Enter, ctrl), h), None);
        assert_eq!(edit_of(&key(Key::Backspace, none), h), Some(Edit::Backspace));
        assert_eq!(edit_of(&key(Key::End, none), v), Some(Edit::End));
        assert_eq!(edit_of(&key(Key::Home, Modifiers::ALT), v), None);
    }

    #[test]
    fn a_paragraph_mark_keeps_room_after_it() {
        let f = Faces { letter: FontId::proportional(40.0), mark: FontId::proportional(22.0), ink: Color32::WHITE };
        let (job, tail) = line(&f, &['あ', '\n', 'い'], "う");
        assert_eq!(job.text, "あ¶いう");
        assert_eq!(tail, 0.0);
        let (job, tail) = line(&f, &['あ', '\n'], "");
        assert_eq!(job.text, "あ¶");
        assert!(tail > 0.0);
    }
}
