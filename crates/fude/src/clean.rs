//! The clean copy, which Esc on the writing opens: what was written, set on
//! its paper, under a banner that says why it is shown and a row that
//! changes the paper, over a line with the record and how fast it went. Enter or a letter writes on, a click on a letter puts
//! the cursor there, Ctrl+N starts a new piece, Ctrl+S binds this one and
//! ←→ turn to the pieces around it. The mock's `#clean`.

use std::time::Instant;

use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, vec2, Align, Event, FontId, Key, Layout, Modifiers, Sense, Stroke, Ui};
use ito_prefs::Look;

use crate::band::{self, Band, Hit};
use crate::config::{Config, Dir, OnContinue, Paper, COLS, ROWS, WRAP};
use crate::draw::{self, Drawn, Sheet};
use crate::pace::Pace;
use crate::{layout, notebook, stats, write, App, Screen};

/// How long the tools row says what a copy did, in seconds.
pub const SAID: f64 = 2.5;

/// The letters to a line the tools row offers on a sheet; 0 follows the
/// window.
const WRAPS: [u32; 4] = [0, 20, 30, 40];

/// The grids the tools row offers: letters to a line, lines to a sheet.
const GRIDS: [(u32, u32); 2] = [(20, 20), (20, 10)];

/// The clean copy's own state: the paper as the tools row set it, and what
/// the tools row says.
pub struct Clean {
    pub sheet: Sheet,
    /// The tools row changed the paper; it stays so while the settings do.
    tweaked: bool,
    /// The next draw scrolls to where the text begins.
    fresh: bool,
    /// What the tools row says in place of the counts, and until when (in
    /// egui's time).
    pub said: Option<(String, f64)>,
    /// The counts the last draw came to.
    last: Drawn,
    /// The settings the paper was chosen from.
    seen: Config,
}

impl Default for Clean {
    fn default() -> Self {
        Clean::open(&Config::default(), "")
    }
}

impl Clean {
    /// The clean copy of `text` on the paper the settings choose.
    pub fn open(config: &Config, text: &str) -> Clean {
        Clean { sheet: Sheet::choose(config, text), tweaked: false, fresh: true, said: None, last: Drawn::default(), seen: *config }
    }

    /// The tools row or the palette put the copy on `sheet`; it stays on it
    /// while the settings do.
    pub fn set_sheet(&mut self, sheet: Sheet) {
        if sheet != self.sheet {
            (self.sheet, self.tweaked, self.fresh) = (sheet, true, true);
        }
    }
}

impl App {
    /// Esc on the writing: the clean copy, on the paper the tools row left
    /// it on, or else the paper the settings choose.
    pub fn open_clean(&mut self) {
        let config = self.prefs.config;
        if self.clean.tweaked && self.clean.seen == config {
            (self.clean.fresh, self.clean.said) = (true, None);
        } else {
            self.clean = Clean::open(&config, &self.band.text());
        }
        self.screen = Screen::Clean;
    }

    /// Back to the writing. A limit that was reached is raised (or dropped,
    /// as the settings say), so its banner does not come straight back.
    pub fn write_on(&mut self) {
        self.band.go_on(Instant::now(), self.prefs.config.on_continue == OnContinue::Free);
        self.screen = Screen::Write;
    }

    /// Ctrl+C: the text as the clean copy has it, paragraphs opened with a
    /// full-width space when they are indented. The tools row says so.
    pub fn copy(&mut self, ctx: &egui::Context, time: f64) {
        let text = self.band.text();
        let t = &self.table;
        let words = if text.is_empty() {
            t.tr("clean.copy_empty").to_owned()
        } else {
            let indent = self.prefs.config.indent;
            ctx.copy_text(layout::plain_text(&text, indent));
            let n = layout::count(&text).to_string();
            if indent {
                t.trf("clean.copied", &[&n])
            } else {
                t.trf("clean.copied_plain", &[&n])
            }
        };
        self.clean.said = Some((words, time + SAID));
    }

    /// The keys of the clean copy, read before anything else sees them. On
    /// the writing, Esc opens it; on it, Esc or Enter writes on, a letter
    /// writes on and goes onto the band in the same frame, Ctrl+C copies and
    /// ←→ turn to the pieces around it (← is the newer one on a vertical
    /// sheet, which is read right to left). On both, Ctrl+N starts a new
    /// piece and Ctrl+S binds this one.
    pub fn clean_keys(&mut self, ctx: &egui::Context, time: f64) {
        if self.palette.is_some() || self.bell_open || self.prefs.keys_edit.busy() || egui::Popup::is_any_open(ctx) {
            return;
        }
        let key = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
        match self.screen {
            Screen::Write => {
                if key(Modifiers::NONE, Key::Escape) {
                    self.open_clean();
                } else if key(Modifiers::COMMAND, Key::N) {
                    self.new_piece(time);
                } else if key(Modifiers::COMMAND, Key::S) {
                    self.save_piece(time);
                }
            }
            Screen::Clean => {
                // egui may send Ctrl+C both as `Copy` and as the key; it is one copy.
                let copy = ctx.input_mut(|i| {
                    let had = i.events.iter().any(|e| matches!(e, Event::Copy));
                    i.events.retain(|e| !matches!(e, Event::Copy));
                    had
                }) | key(Modifiers::COMMAND, Key::C);
                if copy {
                    self.copy(ctx, time);
                }
                let newer = if self.clean.sheet.dir == Dir::Vertical { -1 } else { 1 };
                if key(Modifiers::COMMAND, Key::N) {
                    self.new_piece(time);
                } else if key(Modifiers::COMMAND, Key::S) {
                    self.save_piece(time);
                } else if key(Modifiers::NONE, Key::ArrowRight) {
                    self.flip_piece(newer, time);
                } else if key(Modifiers::NONE, Key::ArrowLeft) {
                    self.flip_piece(-newer, time);
                } else if key(Modifiers::NONE, Key::Escape) || key(Modifiers::NONE, Key::Enter) {
                    self.write_on();
                } else if ctx.input(|i| i.events.iter().any(writes)) {
                    // Left in the frame's input: the band takes them below.
                    self.write_on();
                }
            }
            Screen::Settings | Screen::Help | Screen::Shelf => {}
        }
    }
}

/// An event that starts writing again on the clean copy: a letter, a paste,
/// or the input method beginning to compose.
fn writes(e: &Event) -> bool {
    match e {
        Event::Text(t) | Event::Paste(t) => t.chars().any(|c| !c.is_control()),
        Event::Ime(egui::ImeEvent::Preedit { text: t, .. } | egui::ImeEvent::Commit(t)) => !t.is_empty(),
        _ => false,
    }
}

/// The clean copy filling `ui`: the banner, the tools row, the paper and
/// the line under it.
/// `under` is the key list over it: drawn, but not to be clicked.
pub fn show(ui: &mut Ui, app: &mut App, under: bool) {
    let ctx = ui.ctx().clone();
    let time = ctx.input(|i| i.time);
    let config = app.prefs.config;
    let text = app.band.text();
    if config != app.clean.seen {
        app.clean = Clean::open(&config, &text);
    }
    if under {
        ui.disable();
    } else if app.typing_here() {
        // The window lets the input method compose here too; its first
        // letters bring the writing back (`clean_keys`).
        app.ime().frame(ui, write::caret_at(ui.max_rect(), &config), app.band.cursor() as u64);
    }
    let l = Look { c: ito_theme::colors(), q: "" };
    if let Some(go) = banner(ui, app, l) {
        match go {
            Go::WriteOn => app.write_on(),
            Go::New => app.new_piece(time),
        }
    }
    if tools(ui, app, l, time) {
        app.write_on();
    }
    // Flipping loads a piece onto the band, so the band's pace is the one
    // of the piece shown.
    let pace = if config.pace { Pace::new(&text, &app.band.pace(Instant::now())) } else { None };
    foot(ui, app, l, pace.is_some());
    let empty = app.table.tr("clean.empty").to_owned();
    let drawn = draw::draw(ui, &app.clean.sheet, &config, &text, &empty, app.clean.fresh, pace.as_ref());
    app.clean.fresh = false;
    let counts = Drawn { clicked: None, ..drawn };
    if counts != app.clean.last {
        // The tools row was drawn with the counts of the frame before.
        app.clean.last = counts;
        ctx.request_repaint();
    }
    if let Some(i) = drawn.clicked.filter(|_| app.screen == Screen::Clean) {
        app.band.set_cursor(i);
        app.write_on();
    }
}

/// The line under the paper: how fast this piece went, the letters today
/// and the days in a row, and with the pace in the margin what its marks
/// are.
fn foot(ui: &mut Ui, app: &App, l: Look, paced: bool) {
    let c = l.c;
    let t = &app.table;
    let b = &app.band;
    let s = stats::notes(&app.notes, notebook::today());
    let mut parts = Vec::new();
    if let Some(n) = stats::per_minute(b.count() as u32, b.elapsed(Instant::now()).as_millis() as u64) {
        parts.push(t.trf("record.piece_rate", &[&n.to_string()]));
    }
    parts.push(t.trf("record.today", &[&s.today.to_string()]));
    if s.streak > 0 {
        parts.push(t.trf("record.streak", &[&s.streak.to_string()]));
    }
    let frame = egui::Frame::NONE.fill(c.side).inner_margin(egui::Margin::symmetric(16, 6));
    egui::Panel::bottom("fude-clean-foot").frame(frame).show_separator_line(false).show(ui, |ui| {
        let r = ui.max_rect();
        ui.painter().hline(r.x_range().expand(16.0), r.top() - 5.5, Stroke::new(1.0, c.border));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ito_prefs::status(ui, &parts.join(" · "), c.dim);
            if paced {
                ui.add_space(16.0);
                let h = ito_theme::zoomed(ui.ctx(), 12.5);
                let (r, _) = ui.allocate_exact_size(vec2(18.0, h), Sense::hover());
                let bar = Stroke::new(3.0, c.dim.gamma_multiply(0.7));
                ui.painter().line_segment([pos2(r.left(), r.center().y), pos2(r.right(), r.center().y)], bar);
                ito_prefs::status(ui, t.tr("record.pace"), c.faint());
                let (r, _) = ui.allocate_exact_size(vec2(6.0, h), Sense::hover());
                ui.painter().circle_filled(r.center(), 2.2, c.wait);
                ito_prefs::status(ui, t.tr("record.stop"), c.faint());
            }
        });
    });
}

/// What the banner's buttons ask for.
enum Go {
    WriteOn,
    New,
}

/// The band across the top that says why the clean copy is shown: a limit
/// reached, or Esc.
fn banner(ui: &mut Ui, app: &App, l: Look) -> Option<Go> {
    let ctx = ui.ctx().clone();
    let c = l.c;
    let t = &app.table;
    let tr = |k: &'static str| t.tr(k).to_owned();
    let b = &app.band;
    let now = Instant::now();
    let count = b.count().to_string();
    let (tint, dot, words, note) = if !b.hit().is_empty() {
        let took = t.trf("clean.took", &[&band::fmt(b.elapsed(now), false), &count]);
        (ito_theme::mix(c.panel, c.wait, 0.11), c.wait, write::hit_words(b, t), format!("{took} {}", going_on(b, app, now)))
    } else {
        (c.panel, c.done, tr("clean.so_far"), t.trf("clean.so_far_note", &[&count, &band::fmt(b.elapsed(now), false)]))
    };
    let h = ito_theme::zoomed(&ctx, 44.0);
    let frame = egui::Frame::NONE.fill(tint).inner_margin(egui::Margin::symmetric(16, 0));
    let mut go = None;
    egui::Panel::top("fude-clean-banner").exact_size(h).frame(frame).show_separator_line(false).show(ui, |ui| {
        let r = ui.max_rect();
        ui.painter().hline(r.x_range().expand(16.0), r.bottom() - 0.5, Stroke::new(1.0, c.border));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ito_prefs::button(ui, l, &tr("clean.anew")) {
                go = Some(Go::New);
            }
            if keycap(ui, l, "Esc") | ito_prefs::button(ui, l, &tr("clean.go_on")) {
                go = Some(Go::WriteOn);
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let (dot_at, _) = ui.allocate_exact_size(vec2(14.0, h), Sense::hover());
                ui.painter().circle_filled(pos2(dot_at.left() + 4.0, dot_at.center().y), 4.0, dot);
                let font = FontId::proportional(ito_theme::zoomed(&ctx, 13.0));
                let format = |color| TextFormat { font_id: font.clone(), color, valign: Align::Center, ..Default::default() };
                let mut job = LayoutJob::default();
                job.append(&words, 0.0, format(c.strong()));
                job.append(&note, font.size * 0.8, format(c.dim));
                ui.add(egui::Label::new(job).truncate());
            });
        });
    });
    go
}

/// What writing on past the limits will do, after a limit was reached.
fn going_on(b: &Band, app: &App, now: Instant) -> String {
    let t = &app.table;
    if app.prefs.config.on_continue == OnContinue::Free {
        return t.tr("clean.go_free").to_owned();
    }
    // The limits reached, as they will be.
    let (add_c, add_t) = b.raised(now);
    let to: Vec<String> = b
        .hit()
        .iter()
        .map(|h| match h {
            Hit::Chars => t.trf("clean.to_chars", &[&(b.limits.chars + add_c).to_string()]),
            Hit::Time => {
                let minutes = ((f64::from(b.limits.minutes) * 60.0 + add_t.as_secs_f64()) / 60.0).round();
                t.trf("clean.to_minutes", &[&minutes.to_string()])
            }
        })
        .collect();
    t.trf("clean.go_raised", &[&to.join(t.tr("write.hit_join"))])
}

/// A key's cap, which does what its key does when clicked.
fn keycap(ui: &mut Ui, l: Look, words: &str) -> bool {
    ito_prefs::keycap(ui, l, words, false).clicked()
}

/// The row under the banner that changes the paper for this clean copy
/// (the settings stay as they are), with the counts and the copy button:
/// whether "write on" was clicked.
fn tools(ui: &mut Ui, app: &mut App, l: Look, time: f64) -> bool {
    let ctx = ui.ctx().clone();
    let c = l.c;
    let t = &app.table;
    let tr = |k: &'static str| t.tr(k).to_owned();
    let sheet = app.clean.sheet;
    let mut to = sheet;
    let (mut copy, mut write_on) = (false, false);
    let frame = egui::Frame::NONE.fill(c.side).inner_margin(egui::Margin::symmetric(16, 8));
    egui::Panel::top("fude-clean-tools").frame(frame).show_separator_line(false).show(ui, |ui| {
        let r = ui.max_rect();
        ui.painter().hline(r.x_range().expand(16.0), r.bottom() + 7.5, Stroke::new(1.0, c.border));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let label = |ui: &mut Ui, k: &'static str| ito_prefs::status(ui, t.tr(k), c.dim);
            label(ui, "clean.note");
            let papers = [
                (Paper::Genko, tr("clean.genko")),
                (Paper::Ruled, tr("clean.ruled")),
                (Paper::Graph, tr("clean.graph")),
                (Paper::Plain, tr("clean.plain")),
            ];
            let papers: Vec<(Paper, &str)> = papers.iter().map(|(p, w)| (*p, w.as_str())).collect();
            if let Some(p) = ito_prefs::select(ui, "clean-paper", sheet.paper, &papers) {
                to.paper = p;
            }
            let dirs = [(Dir::Vertical, tr("clean.vertical")), (Dir::Horizontal, tr("clean.horizontal"))];
            let dirs: Vec<(Dir, &str)> = dirs.iter().map(|(d, w)| (*d, w.as_str())).collect();
            if let Some(d) = ito_prefs::select(ui, "clean-dir", sheet.dir, &dirs) {
                to.dir = d;
            }
            if sheet.paper == Paper::Genko {
                label(ui, "clean.grid");
                let mut grids = GRIDS.to_vec();
                let own = (sheet.cols.clamp(COLS.0, COLS.1), sheet.rows.clamp(ROWS.0, ROWS.1));
                if !grids.contains(&own) {
                    grids.push(own);
                }
                let names: Vec<String> = grids.iter().map(|(c, r)| format!("{c}×{r}")).collect();
                let grids: Vec<((u32, u32), &str)> = grids.iter().zip(&names).map(|(g, n)| (*g, n.as_str())).collect();
                if let Some((c, r)) = ito_prefs::select(ui, "clean-grid", own, &grids) {
                    (to.cols, to.rows) = (c, r);
                }
            } else {
                label(ui, "clean.line");
                let mut wraps = WRAPS.to_vec();
                let own = sheet.wrap.clamp(WRAP.0, WRAP.1);
                if !wraps.contains(&own) {
                    wraps.push(own);
                    wraps.sort_unstable();
                }
                let names: Vec<String> = wraps.iter().map(|w| if *w == 0 { tr("clean.auto") } else { w.to_string() }).collect();
                let wraps: Vec<(u32, &str)> = wraps.iter().zip(&names).map(|(w, n)| (*w, n.as_str())).collect();
                if let Some(w) = ito_prefs::select(ui, "clean-wrap", own, &wraps) {
                    to.wrap = w;
                }
            }
            let said = app.clean.said.as_ref().filter(|(_, until)| time < *until);
            let (words, color) = match said {
                Some((w, until)) => {
                    ctx.request_repaint_after_secs((until - time) as f32);
                    (w.clone(), c.done)
                }
                None => (stats(app, &sheet), c.dim),
            };
            ito_prefs::status(ui, &words, color);
            copy = keycap(ui, l, "Ctrl+C") | ito_prefs::button(ui, l, &tr("clean.copy"));
            write_on = keycap(ui, l, "Esc") | ito_prefs::button(ui, l, &tr("clean.go_on"));
        });
    });
    app.clean.set_sheet(to);
    if copy {
        app.copy(&ctx, time);
    }
    write_on
}

/// The counts after the paper: "412 字 · 3:05 · 原稿用紙 2 枚（21 行）".
fn stats(app: &App, sheet: &Sheet) -> String {
    let t = &app.table;
    let b = &app.band;
    let d = app.clean.last;
    let info = if sheet.paper == Paper::Genko {
        t.trf("clean.genko_info", &[&d.pages.to_string(), &d.lines.to_string()])
    } else {
        let name = match sheet.paper {
            Paper::Graph => "clean.graph",
            Paper::Plain => "clean.plain",
            _ => "clean.ruled",
        };
        let wrap = if sheet.wrap == 0 { t.tr("clean.wrap_auto").to_owned() } else { t.trf("clean.wrap_at", &[&sheet.wrap.to_string()]) };
        t.trf("clean.sheet_info", &[t.tr(name), &wrap, &d.paras.to_string()])
    };
    t.trf("clean.stats", &[&b.count().to_string(), &band::fmt(b.elapsed(Instant::now()), false), &info])
}
