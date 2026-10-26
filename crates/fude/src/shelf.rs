//! The shelf (Ctrl+O): the notebooks down the left and the pieces of the one
//! picked on the right, newest first. Enter opens a notebook, a piece's clean
//! copy, or a new piece; N makes a notebook and F2 renames one. Also what
//! fude keeps of the notebooks: the one written in, the piece on the band
//! and its topic, and binding it into its file.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use egui::text::{LayoutJob, TextWrapping};
use egui::{pos2, vec2, Color32, Event, FontId, Galley, Key, Modifiers, Rect, Response, Sense, Stroke, Ui, UiBuilder};
use ito_notices::Kind;
use ito_prefs::Look;
use ito_theme::Colors;
use serde::{Deserialize, Serialize};

use crate::band::{self, Band, Hit, Kept};
use crate::config::{OnContinue, Topic};
use crate::notebook::{self, BadName, Done, Job, Note, Piece, Record};
use crate::{clean, stats, App, Screen};

/// The notebooks' column, as wide as the settings' pages.
const SIDE: f32 = 260.0;

/// How long Enter and Esc are not the shelf's after it is shown or a name
/// is written (egui reports Enter on two frames in a row), in seconds.
const HOLD: f64 = 0.2;

/// The field a notebook is named in.
const NAME_FIELD: &str = "shelf-name";

/// What draft.toml keeps: the notebook written in, the piece on the band
/// (none until it has a letter), its topic, and the band. A draft from
/// before there were notebooks is a bare band, and reads as one in no
/// notebook.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Draft {
    pub note: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub piece: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(flatten)]
    pub band: Kept,
}

/// The shelf's own state.
pub struct Shelf {
    /// Where Esc goes back to: the writing or the clean copy.
    pub from: Screen,
    /// The notebook whose pieces are picked among; none while the
    /// notebooks are.
    pub open: Option<String>,
    /// The row picked: a notebook, or in an open one 0 for a new piece and
    /// then its pieces, newest first.
    pub sel: usize,
    naming: Option<Naming>,
    /// Enter and Esc are not the shelf's until then, in egui's time.
    hold: f64,
    /// The shelf was on the screen in the last frame.
    shown: bool,
    /// The next draw brings the row picked into view.
    scroll: bool,
}

impl Default for Shelf {
    fn default() -> Self {
        Shelf { from: Screen::Write, open: None, sel: 0, naming: None, hold: f64::NEG_INFINITY, shown: false, scroll: false }
    }
}

/// A notebook being named.
struct Naming {
    /// The notebook renamed; none for a new one.
    renaming: Option<String>,
    /// Why the name written was refused.
    err: Option<String>,
    /// The field is to take the keys.
    focus: bool,
}

/// What a click or a name asks of the shelf.
enum Go {
    Pick(usize),
    NewPiece(String),
    Piece(String, String),
    NewNote,
    Rename,
    Named(String),
    Unname,
}

impl App {
    /// The folder of the notebook `name`.
    fn dir_of(&self, name: &str) -> PathBuf {
        match &self.root {
            Some(r) => r.join(name),
            None => PathBuf::from(name),
        }
    }

    /// The notebook written in, in the list (put there when the disk has
    /// not shown it yet).
    pub fn ensure_note(&mut self) -> usize {
        if let Some(i) = self.notes.iter().position(|n| n.name == self.note) {
            return i;
        }
        let dir = self.dir_of(&self.note);
        self.notes.push(Note { table: self.prefs.note_table(), ..Note::empty(&self.note, dir) });
        notebook::sort_notes(&mut self.notes);
        self.notes.iter().position(|n| n.name == self.note).unwrap_or(0)
    }

    /// What draft.toml is to keep now.
    pub fn draft_now(&self) -> Draft {
        Draft { note: self.note.clone(), piece: self.piece.clone(), topic: self.topic.clone(), band: self.band.kept(Instant::now()) }
    }

    /// At the start: the notebook and the piece of the draft, or else the
    /// notebook written in last (or a new one), its settings in force, and
    /// the thread that reads and writes the notebooks.
    pub fn start_notes(&mut self, draft: Option<Draft>, wake: impl Fn() + Send + 'static) {
        let draft = draft.unwrap_or_default();
        let asked = notebook::valid_name(&draft.note).ok();
        let name = asked
            .clone()
            .filter(|a| self.dir_of(a).is_dir())
            .or_else(|| self.root.as_deref().and_then(notebook::newest))
            .or_else(|| asked.clone())
            .unwrap_or_else(|| self.table.tr("note.default").to_owned());
        let dir = self.dir_of(&name);
        match &self.root {
            Some(_) => self.prefs.set_note(Some(dir.join(notebook::FILE)), notebook::read_table(&dir)),
            None => self.prefs.set_note(None, Ok(toml::Table::new())),
        }
        // A draft of another notebook goes on as a new piece in this one.
        let same = asked.as_ref() == Some(&name);
        self.piece = draft.piece.filter(|_| same);
        self.topic = draft.topic.filter(|_| same);
        self.band = Band::from_kept(draft.band, self.prefs.config.limits());
        self.note = name;
        if let Some(root) = self.root.clone() {
            let w = notebook::Worker::start(root, wake);
            w.send(if dir.is_dir() { Job::Scan } else { Job::Make { dir, table: toml::Table::new() } });
            self.worker = Some(w);
        }
        self.ensure_note();
        if self.topic.is_none() && self.piece.is_none() && self.band.count() == 0 {
            self.choose_topic();
        }
    }

    /// The topic of a new piece, from the notebook's list: the one after the
    /// topic shown last (or else the newest piece's), or one by lot.
    fn choose_topic(&mut self) {
        let list = self.prefs.topics();
        let i = self.ensure_note();
        let last = self
            .topic
            .take()
            .filter(|t| list.contains(t))
            .or_else(|| self.notes[i].pieces.iter().rev().find_map(|p| p.rec.as_ref()?.topic.clone()));
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        self.topic = self.prefs.config.topic.pick(&list, last.as_deref(), seed);
    }

    /// The topics or how they are picked changed: a piece not begun yet
    /// keeps its topic while it is still in the list, or else takes one
    /// again.
    pub fn retopic(&mut self) {
        if self.piece.is_some() || self.band.count() > 0 {
            return;
        }
        let list = self.prefs.topics();
        let keep = self.prefs.config.topic != Topic::Off && self.topic.as_ref().is_some_and(|t| list.contains(t));
        if !keep {
            self.choose_topic();
        }
    }

    /// Into the notebook `name`: its own settings in force.
    fn enter_note(&mut self, name: &str) {
        let table = self.prefs.note_table();
        if let Some(n) = self.notes.iter_mut().find(|n| n.name == self.note) {
            n.table = table;
        }
        self.note = name.to_owned();
        let i = self.ensure_note();
        let n = &self.notes[i];
        let read = match &n.warn {
            Some(w) => Err(w.clone()),
            None => Ok(n.table.clone()),
        };
        let path = self.root.as_ref().map(|_| n.dir.join(notebook::FILE));
        self.prefs.set_note(path, read);
    }

    /// The piece on the band is named once it has a letter: by the time it
    /// was begun.
    fn name_piece(&mut self) {
        if self.piece.is_some() || self.band.count() == 0 {
            return;
        }
        let i = self.ensure_note();
        let (n, on_disk) = (&self.notes[i], self.root.is_some());
        let taken = |s: &str| n.piece(s).is_some() || (on_disk && n.dir.join(format!("{s}.txt")).exists());
        self.piece = Some(notebook::stem_at(chrono::Local::now().naive_local(), taken));
    }

    /// The piece on the band into its file (on the worker's thread), when it
    /// changed since it last was: leaving the band, Ctrl+S, another piece,
    /// the end.
    pub fn bind_piece(&mut self) {
        self.name_piece();
        let Some(stem) = self.piece.clone() else { return };
        let i = self.ensure_note();
        let text = notebook::normalize(&self.band.text());
        let had = self.notes[i].piece(&stem);
        let now = Instant::now();
        let rec = match self.band.started() {
            true => {
                let ms = self.band.elapsed(now).as_millis() as u64;
                let days = notebook::grown(had, notebook::chars(&text), ms, notebook::today());
                Some(Record { ms, topic: self.topic.clone(), pace: self.band.pace(now), days })
            }
            false => had.and_then(|p| p.rec.clone()),
        };
        if had.map_or(text.is_empty(), |p| p.text == text && p.ms() == rec.as_ref().map(|r| r.ms)) {
            return;
        }
        let n = &mut self.notes[i];
        n.put(Piece::new(&stem, &text, rec.clone()));
        n.modified = Some(SystemTime::now());
        let dir = n.dir.clone();
        notebook::sort_notes(&mut self.notes);
        if let Some(w) = &self.worker {
            w.send(Job::Bind { dir, stem, text, rec });
        }
    }

    /// An empty band for a new piece, with a topic, on the writing. The
    /// emptied draft is written at once.
    fn fresh_piece(&mut self, time: f64) {
        self.band.reset();
        self.piece = None;
        self.choose_topic();
        self.preedit.clear();
        self.draft_due = Some(time);
        // The clock starts again as the band comes back on the screen.
        self.on_band = false;
        self.screen = Screen::Write;
    }

    /// Ctrl+N: what is written is bound into its piece, and a new one is
    /// begun at once.
    pub fn new_piece(&mut self, time: f64) {
        self.bind_piece();
        self.fresh_piece(time);
    }

    /// A new piece in the notebook `name`.
    fn open_note(&mut self, name: &str, time: f64) {
        self.bind_piece();
        if name != self.note {
            self.enter_note(name);
        }
        self.fresh_piece(time);
    }

    /// The piece `stem` of the notebook `name` onto the band, its clean copy
    /// shown. A limit it reached is raised, as writing on past it would.
    fn open_piece(&mut self, name: &str, stem: &str, time: f64) {
        if name == self.note && self.piece.as_deref() == Some(stem) {
            self.open_clean();
            return;
        }
        self.bind_piece();
        if name != self.note {
            self.enter_note(name);
        }
        let i = self.ensure_note();
        let Some(p) = self.notes[i].piece(stem).cloned() else { return };
        let limits = self.prefs.config.limits();
        let ms = p.ms().unwrap_or(0);
        let mut hit = Vec::new();
        if limits.chars > 0 && p.chars >= limits.chars as usize {
            hit.push(Hit::Chars);
        }
        if limits.minutes > 0 && ms >= u64::from(limits.minutes) * 60_000 {
            hit.push(Hit::Time);
        }
        let started = p.rec.is_some();
        let rec = p.rec.unwrap_or_default();
        let kept = Kept { text: p.text, hit, started, elapsed_ms: ms, pace: rec.pace, ..Kept::default() };
        self.topic = rec.topic;
        self.band = Band::from_kept(kept, limits);
        self.band.go_on(Instant::now(), self.prefs.config.on_continue == OnContinue::Free);
        self.piece = Some(p.stem);
        self.preedit.clear();
        self.on_band = false;
        self.open_clean();
        self.draft_due = Some(time);
    }

    /// Ctrl+S: the piece into its file now; the clean copy's tools row says
    /// so.
    pub fn save_piece(&mut self, time: f64) {
        self.bind_piece();
        let t = &self.table;
        let words = match &self.piece {
            Some(stem) => t.trf("clean.saved", &[&format!("{stem}.txt")]),
            None => t.tr("clean.save_empty").to_owned(),
        };
        self.clean.said = Some((words, time + clean::SAID));
    }

    /// ←→ on the clean copy: the piece `by` places on in this notebook (1
    /// is newer); at either end the tools row says so.
    pub fn flip_piece(&mut self, by: isize, time: f64) {
        let i = self.ensure_note();
        let to = {
            let stems: Vec<&str> = self.notes[i].pieces.iter().map(|p| p.stem.as_str()).collect();
            notebook::flip(&stems, self.piece.as_deref(), by).map(str::to_owned)
        };
        match to {
            Some(stem) => {
                let name = self.note.clone();
                self.open_piece(&name, &stem, time);
            }
            None => {
                let k = if by < 0 { "clean.first" } else { "clean.last" };
                self.clean.said = Some((self.table.tr(k).to_owned(), time + clean::SAID));
            }
        }
    }

    /// What the worker did since the last frame: the notebooks as read
    /// again, and what failed.
    pub fn follow_notes(&mut self) {
        let mut changed = false;
        while let Some(done) = self.worker.as_ref().and_then(|w| w.done()) {
            match done {
                Done::Scanned(notes) => self.notes = notes,
                Done::Note(n) => match self.notes.iter_mut().find(|m| m.dir == n.dir) {
                    Some(m) => *m = n,
                    None => self.notes.push(n),
                },
                Done::Failed(e) => {
                    self.say(Kind::Err, &e);
                    continue;
                }
            }
            changed = true;
        }
        if changed {
            notebook::sort_notes(&mut self.notes);
            self.ensure_note();
        }
    }

    /// Ctrl+O: the shelf, open on the notebook written in at the piece on
    /// the band; again, back to where it was opened from.
    pub fn toggle_shelf(&mut self) {
        match self.screen {
            Screen::Shelf => self.screen = self.shelf.from,
            Screen::Settings | Screen::Help if self.back == Screen::Shelf => self.screen = Screen::Shelf,
            _ => {
                self.shelf.from = if matches!(self.screen, Screen::Write | Screen::Clean) { self.screen } else { self.back };
                self.bind_piece();
                if let Some(w) = &self.worker {
                    w.send(Job::Scan);
                }
                let i = self.ensure_note();
                let n = &self.notes[i];
                let at = self.piece.as_ref().and_then(|s| n.pieces.iter().rev().position(|p| &p.stem == s));
                self.shelf.sel = at.map_or(0, |at| at + 1);
                self.shelf.open = Some(self.note.clone());
                self.shelf.scroll = true;
                self.shelf.naming = None;
                self.screen = Screen::Shelf;
            }
        }
    }

    /// The rows the keys move among.
    fn shelf_rows(&self) -> usize {
        match &self.shelf.open {
            None => self.notes.len(),
            Some(name) => 1 + self.notes.iter().find(|n| &n.name == name).map_or(0, |n| n.pieces.len()),
        }
    }

    /// The notebook F2 renames: the one open, or the one picked.
    fn shelf_target(&self) -> Option<String> {
        self.shelf.open.clone().or_else(|| self.notes.get(self.shelf.sel).map(|n| n.name.clone()))
    }

    /// The shelf's keys, read before anything else sees them: ↑↓ Home End
    /// pick, Enter (→ on a notebook) opens, ← goes back to the notebooks,
    /// Esc closes, N makes a notebook and F2 renames one. While a name is
    /// written the field has them.
    pub fn shelf_keys(&mut self, ctx: &egui::Context, time: f64) {
        let on = self.screen == Screen::Shelf;
        let was = std::mem::replace(&mut self.shelf.shown, on);
        if !on || self.palette.is_some() || self.bell_open || egui::Popup::is_any_open(ctx) {
            return;
        }
        if !was {
            self.shelf.hold = time + HOLD;
        }
        if time < self.shelf.hold {
            ctx.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Enter);
                i.consume_key(Modifiers::NONE, Key::Escape);
            });
        }
        if self.shelf.naming.is_some() {
            return;
        }
        let key = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        let last = self.shelf_rows().saturating_sub(1);
        let sel = self.shelf.sel.min(last);
        let to = if key(Key::ArrowDown) {
            Some(sel + 1)
        } else if key(Key::ArrowUp) {
            Some(sel.saturating_sub(1))
        } else if key(Key::Home) {
            Some(0)
        } else if key(Key::End) {
            Some(last)
        } else {
            None
        };
        if let Some(to) = to {
            (self.shelf.sel, self.shelf.scroll) = (to.min(last), true);
        } else if key(Key::Enter) || (self.shelf.open.is_none() && key(Key::ArrowRight)) {
            self.shelf_enter(time);
        } else if self.shelf.open.is_some() && key(Key::ArrowLeft) {
            let open = self.shelf.open.take();
            self.shelf.sel = open.and_then(|o| self.notes.iter().position(|n| n.name == o)).unwrap_or(0);
            self.shelf.scroll = true;
        } else if key(Key::Escape) {
            self.screen = self.shelf.from;
        } else if key(Key::N) {
            self.start_naming(None);
        } else if key(Key::F2) {
            if let Some(name) = self.shelf_target() {
                self.start_naming(Some(name));
            }
        } else {
            return;
        }
        if self.shelf.naming.is_some() {
            // The letter of the key that opened the field is not its first.
            ctx.input_mut(|i| i.events.retain(|e| !matches!(e, Event::Text(_))));
        }
    }

    /// Enter on the row picked.
    fn shelf_enter(&mut self, time: f64) {
        let sel = self.shelf.sel;
        match self.shelf.open.clone() {
            None => {
                if let Some(n) = self.notes.get(sel) {
                    (self.shelf.open, self.shelf.sel, self.shelf.scroll) = (Some(n.name.clone()), 0, true);
                }
            }
            Some(name) if sel == 0 => self.open_note(&name, time),
            Some(name) => {
                let n = self.notes.iter().find(|n| n.name == name);
                if let Some(stem) = n.and_then(|n| n.pieces.iter().rev().nth(sel - 1)).map(|p| p.stem.clone()) {
                    self.open_piece(&name, &stem, time);
                }
            }
        }
    }

    /// The name written for a new notebook or the one renamed, or why it is
    /// refused.
    fn name_note(&mut self, text: &str, time: f64) -> Result<(), String> {
        let name = notebook::valid_name(text).map_err(|e| {
            let k = if e == BadName::Empty { "shelf.empty_name" } else { "shelf.bad_name" };
            self.table.tr(k).to_owned()
        })?;
        let renaming = self.shelf.naming.as_ref().and_then(|n| n.renaming.clone());
        let lower = name.to_lowercase();
        if self.notes.iter().any(|n| Some(&n.name) != renaming.as_ref() && n.name.to_lowercase() == lower) {
            return Err(self.table.trf("shelf.taken", &[&name]));
        }
        let dir = self.dir_of(&name);
        match renaming {
            // A new notebook takes the settings of the one written in.
            None => {
                let table = self.prefs.note_table();
                self.notes.push(Note { table: table.clone(), modified: Some(SystemTime::now()), ..Note::empty(&name, dir.clone()) });
                notebook::sort_notes(&mut self.notes);
                if let Some(w) = &self.worker {
                    w.send(Job::Make { dir, table });
                }
            }
            Some(old) if old != name => {
                let from = self.dir_of(&old);
                if let Some(n) = self.notes.iter_mut().find(|n| n.name == old) {
                    (n.name, n.dir) = (name.clone(), dir.clone());
                }
                if self.note == old {
                    self.note = name.clone();
                    let table = self.prefs.note_table();
                    self.prefs.set_note(self.root.as_ref().map(|_| dir.join(notebook::FILE)), Ok(table));
                    self.draft_due = Some(time);
                }
                if let Some(w) = &self.worker {
                    w.send(Job::Rename { from, to: dir });
                }
            }
            Some(_) => {}
        }
        (self.shelf.open, self.shelf.sel, self.shelf.scroll) = (Some(name), 0, true);
        Ok(())
    }

    fn start_naming(&mut self, renaming: Option<String>) {
        self.shelf.naming = Some(Naming { renaming, err: None, focus: true });
        self.prefs.drafts.remove(NAME_FIELD);
    }

    fn stop_naming(&mut self, time: f64) {
        self.shelf.naming = None;
        self.shelf.hold = time + HOLD;
        self.prefs.drafts.remove(NAME_FIELD);
    }

    fn shelf_go(&mut self, go: Go, time: f64) {
        match go {
            Go::Pick(i) => {
                if let Some(n) = self.notes.get(i) {
                    (self.shelf.open, self.shelf.sel) = (Some(n.name.clone()), 0);
                }
            }
            Go::NewPiece(name) => self.open_note(&name, time),
            Go::Piece(name, stem) => self.open_piece(&name, &stem, time),
            Go::NewNote => self.start_naming(None),
            Go::Rename => {
                if let Some(name) = self.shelf_target() {
                    self.start_naming(Some(name));
                }
            }
            Go::Named(text) => match self.name_note(&text, time) {
                Ok(()) => self.stop_naming(time),
                Err(e) => {
                    if let Some(n) = &mut self.shelf.naming {
                        (n.err, n.focus) = (Some(e), true);
                    }
                }
            },
            Go::Unname => self.stop_naming(time),
        }
    }
}

/// The shelf filling `ui`. `under` is the key list or the settings over it:
/// drawn, but not to be clicked.
pub fn show(ui: &mut Ui, app: &mut App, under: bool) {
    let ctx = ui.ctx().clone();
    let time = ctx.input(|i| i.time);
    if under {
        ui.disable();
    }
    let l = Look { c: ito_theme::colors(), q: "" };
    let c = l.c;
    let z = |size: f32| ito_theme::zoomed(&ctx, size);
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, c.bg);
    let side = Rect::from_min_max(rect.min, pos2(rect.left() + SIDE, rect.bottom()));
    ui.painter().rect_filled(side, 0.0, c.side);
    ui.painter().vline(side.right() - 0.5, side.y_range(), Stroke::new(1.0, c.border));
    let mut go = None;

    let mut nav = ui.new_child(UiBuilder::new().max_rect(side.shrink2(vec2(10.0, 16.0))));
    nav.label(egui::RichText::new(app.table.tr("shelf.title")).size(z(15.0)).color(c.strong()));
    nav.add_space(z(8.0));
    let rows: Vec<(String, usize)> = app.notes.iter().map(|n| (n.name.clone(), n.pieces.len())).collect();
    let renaming = app.shelf.naming.as_ref().map(|n| n.renaming.clone());
    let room = (nav.available_height() - z(84.0)).max(z(64.0));
    egui::ScrollArea::vertical().id_salt("shelf-notes").max_height(room).auto_shrink([false, true]).show(&mut nav, |ui| {
        for (i, (name, count)) in rows.iter().enumerate() {
            if renaming.as_ref().is_some_and(|r| r.as_deref() == Some(name.as_str())) {
                go = name_field(ui, app, name).or(go.take());
                continue;
            }
            let on = match &app.shelf.open {
                None => i == app.shelf.sel,
                Some(o) => o == name,
            };
            let (r, resp) = row(ui, z(32.0), on, c);
            let p = ui.painter();
            let n = p.layout_no_wrap(count.to_string(), FontId::proportional(z(11.0)), c.faint());
            let right = r.right() - 10.0 - n.size().x;
            p.galley(pos2(right, r.center().y - n.size().y / 2.0), n, c.faint());
            let here = *name == app.note;
            let width = right - 8.0 - (r.left() + 10.0) - if here { 12.0 } else { 0.0 };
            let g = line(ui, name, z(13.0), if on { c.strong() } else { c.dim }, width);
            let w = g.size().x;
            p.galley(pos2(r.left() + 10.0, r.center().y - g.size().y / 2.0), g, c.dim);
            if here {
                p.circle_filled(pos2(r.left() + 10.0 + w + 8.0, r.center().y), 3.0, c.run);
            }
            if resp.clicked() {
                go = Some(Go::Pick(i));
            }
            if on && app.shelf.scroll && app.shelf.open.is_none() {
                resp.scroll_to_me(None);
            }
        }
        if renaming == Some(None) {
            go = name_field(ui, app, "").or(go.take());
        }
    });
    if let Some(e) = app.shelf.naming.as_ref().and_then(|n| n.err.as_deref()) {
        ito_prefs::status(&mut nav, e, c.err);
    }
    nav.add_space(z(10.0));
    nav.horizontal_wrapped(|ui| {
        if ito_prefs::keycap(ui, l, "N", false).clicked() | ito_prefs::button(ui, l, app.table.tr("shelf.new")) {
            go = Some(Go::NewNote);
        }
        if ito_prefs::keycap(ui, l, "F2", false).clicked() | ito_prefs::button(ui, l, app.table.tr("shelf.rename")) {
            go = Some(Go::Rename);
        }
    });

    let mut page = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_max(pos2(side.right(), rect.top()), rect.max)));
    let shown = match &app.shelf.open {
        Some(o) => app.notes.iter().position(|n| &n.name == o),
        None => (app.shelf.sel < app.notes.len()).then_some(app.shelf.sel),
    };
    egui::ScrollArea::vertical().id_salt("shelf-pieces").auto_shrink([false, false]).show(&mut page, |ui| {
        let margin = egui::Margin { left: 48, right: 48, top: 28, bottom: 40 };
        egui::Frame::NONE.inner_margin(margin).show(ui, |ui| {
            ui.set_max_width(820.0);
            record(ui, app, l);
            ui.add_space(z(22.0));
            if let Some(i) = shown {
                if let Some(g) = pieces(ui, app, i, l) {
                    go = Some(g);
                }
            }
        });
    });
    app.shelf.scroll = false;
    if let Some(go) = go.filter(|_| !under) {
        app.shelf_go(go, time);
    }
}

/// The record over the pieces, of all the notebooks: the letters today,
/// the days written on in a row and the letters to a minute this week.
fn record(ui: &mut Ui, app: &App, l: Look) {
    let c = l.c;
    let t = &app.table;
    let ctx = ui.ctx().clone();
    let z = |size: f32| ito_theme::zoomed(&ctx, size);
    let s = stats::notes(&app.notes, notebook::today());
    let rate = s.per_minute.map_or_else(|| "—".to_owned(), |n| t.trf("record.chars", &[&n.to_string()]));
    let cells = [
        (t.tr("record.today_label"), t.trf("record.chars", &[&s.today.to_string()])),
        (t.tr("record.streak_label"), t.trf("record.days", &[&s.streak.to_string()])),
        (t.tr("record.rate_label"), rate),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = z(36.0);
        for (label, value) in cells {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = z(2.0);
                ui.label(egui::RichText::new(label).size(z(11.5)).color(c.faint()));
                ui.label(egui::RichText::new(value).size(z(18.0)).color(c.fg));
            });
        }
    });
}

/// The pieces of the `i`th notebook, newest first, under a row for a new
/// one.
fn pieces(ui: &mut Ui, app: &App, i: usize, l: Look) -> Option<Go> {
    let c = l.c;
    let t = &app.table;
    let ctx = ui.ctx().clone();
    let z = |size: f32| ito_theme::zoomed(&ctx, size);
    let n = &app.notes[i];
    let mut go = None;
    ui.label(egui::RichText::new(&n.name).size(z(24.0)).color(c.strong()));
    let total: usize = n.pieces.iter().map(|p| p.chars).sum();
    ui.label(egui::RichText::new(t.trf("shelf.lead", &[&n.pieces.len().to_string(), &total.to_string()])).size(z(13.0)).color(c.dim));
    if let Some(w) = &n.warn {
        ito_prefs::status(ui, w, c.err);
    }
    ui.add_space(z(14.0));
    let picking = app.shelf.open.as_ref() == Some(&n.name);
    let scroll = app.shelf.scroll && picking;

    let on = picking && app.shelf.sel == 0;
    let (r, resp) = row(ui, z(40.0), on, c);
    let g = line(ui, &format!("+ {}", t.tr("shelf.write_new")), z(14.0), if on { c.strong() } else { c.run }, r.width() - 24.0);
    ui.painter().galley(pos2(r.left() + 12.0, r.center().y - g.size().y / 2.0), g, c.run);
    if resp.clicked() {
        go = Some(Go::NewPiece(n.name.clone()));
    }
    if on && scroll {
        resp.scroll_to_me(None);
    }

    for (k, p) in n.pieces.iter().rev().enumerate() {
        let on = picking && app.shelf.sel == k + 1;
        let (r, resp) = row(ui, z(54.0), on, c);
        let paint = ui.painter();
        let mut info = t.trf("shelf.chars", &[&p.chars.to_string()]);
        if let Some(ms) = p.ms() {
            info = format!("{info} · {}", band::fmt(Duration::from_millis(ms), false));
            if let Some(n) = stats::per_minute(p.chars as u32, ms) {
                info = format!("{info} · {}", t.trf("record.rate", &[&n.to_string()]));
            }
        }
        let info = paint.layout_no_wrap(info, FontId::proportional(z(11.5)), c.faint());
        let top = r.top() + 9.0;
        let right = r.right() - 12.0 - info.size().x;
        paint.galley(pos2(right, top), info, c.faint());
        let when = p.begun().map_or_else(|| p.stem.clone(), |b| b.format("%Y-%m-%d %H:%M").to_string());
        let date = line(ui, &when, z(11.5), c.dim, right - 8.0 - (r.left() + 12.0));
        let date_w = date.size().x;
        paint.galley(pos2(r.left() + 12.0, top), date, c.dim);
        if n.name == app.note && app.piece.as_ref() == Some(&p.stem) {
            paint.circle_filled(pos2(r.left() + 12.0 + date_w + 8.0, top + z(11.5) * 0.6), 3.0, c.run);
        }
        let words = line(ui, &p.opening(40), z(14.0), if on { c.strong() } else { c.fg }, r.width() - 24.0);
        paint.galley(pos2(r.left() + 12.0, r.bottom() - 9.0 - words.size().y), words, c.fg);
        if resp.clicked() {
            go = Some(Go::Piece(n.name.clone(), p.stem.clone()));
        }
        if on && scroll {
            resp.scroll_to_me(None);
        }
    }
    if n.pieces.is_empty() {
        ui.add_space(z(8.0));
        ui.label(egui::RichText::new(t.tr("shelf.empty")).size(z(13.0)).color(c.faint()));
    }
    ui.add_space(z(18.0));
    ui.label(egui::RichText::new(t.tr("shelf.keys")).size(z(11.5)).color(c.faint()));
    go
}

/// A row to click, lit when picked or under the pointer.
fn row(ui: &mut Ui, height: f32, on: bool, c: Colors) -> (Rect, Response) {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let fill = if on {
        c.chosen()
    } else if resp.hovered() {
        c.hover()
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(r, 6.0, fill);
    (r, resp)
}

/// One line of `text`, cut with … at `width`.
fn line(ui: &Ui, text: &str, size: f32, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), FontId::proportional(size), color);
    job.wrap = TextWrapping::truncate_at_width(width.max(1.0));
    ui.painter().layout_job(job)
}

/// The field a notebook's name is written in, `now` to begin with.
fn name_field(ui: &mut Ui, app: &mut App, now: &str) -> Option<Go> {
    let id = ito_prefs::field_id(NAME_FIELD);
    let want = app.shelf.naming.as_ref().is_some_and(|n| n.focus);
    if want {
        ui.memory_mut(|m| m.request_focus(id));
    }
    let hint = app.table.tr("shelf.name_hint").to_owned();
    let width = ui.available_width();
    let got = ito_prefs::field(ui, &mut app.prefs.drafts, NAME_FIELD, now, &hint, width);
    let has = ui.memory(|m| m.has_focus(id));
    if has {
        if let Some(n) = &mut app.shelf.naming {
            n.focus = false;
        }
    }
    match got {
        Some(t) => Some(Go::Named(t)),
        None if !has && !want => Some(Go::Unname),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The draft keeps the notebook and the piece beside the band.
    #[test]
    fn a_draft_goes_round() {
        let d = Draft {
            note: "ノート".into(),
            piece: Some("2026-10-27_0905".into()),
            topic: Some("朝の音".into()),
            band: Kept {
                text: "書きかけ".into(),
                back: 1,
                hit: vec![Hit::Time],
                started: true,
                elapsed_ms: 10_500,
                pace: vec![1, 3],
                ..Kept::default()
            },
        };
        let text = toml::to_string(&d).unwrap();
        assert!(text.contains("note = \"ノート\"") && text.contains("topic = \"朝の音\""), "{text}");
        assert_eq!(toml::from_str::<Draft>(&text).unwrap(), d);
        let none = Draft { piece: None, topic: None, ..d };
        let text = toml::to_string(&none).unwrap();
        assert!(!text.contains("piece") && !text.contains("topic"), "{text}");
    }

    /// A draft from before there were notebooks is a band in none.
    #[test]
    fn an_old_draft_reads() {
        let k = Kept { text: "前の".into(), started: true, elapsed_ms: 42, ..Kept::default() };
        let d: Draft = toml::from_str(&toml::to_string(&k).unwrap()).unwrap();
        assert_eq!(d, Draft { note: String::new(), piece: None, topic: None, band: k });
    }
}
