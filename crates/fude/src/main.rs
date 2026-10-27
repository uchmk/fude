//! fude: a one-line editor for writing, and a clean copy of what was written.
//!
//! What is wired here: the window (its place kept and its level as
//! common.toml's `[window]` says, its own title bar with the bell and the
//! notice list), common.toml's theme, language and scale, the keymap, the
//! settings screen, the key list, the palette, `--keys`, `--version` /
//! `--help`, F12 (the bug report), the writing band with its draft, the clean
//! copy Esc opens, and the notebooks on the shelf (Ctrl+O).

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod band;
mod clean;
mod config;
mod draw;
mod export;
mod fonts;
mod help;
mod i18n;
mod icon;
mod keymap;
mod layout;
mod notebook;
mod notify;
mod pace;
mod prefs;
mod settings;
mod shelf;
mod sound;
mod stats;
mod status;
mod svg;
mod vertical;
mod write;

use std::cell::Cell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Instant;

use ito_common::window::{Saved, Tracker};
use ito_keyscript::{Do, Runner, Step};
use ito_notices::Kind;

use crate::band::{Band, Edit, Hit};
use crate::config::{Dir, Paper, TimeStart};
use crate::export::{What, Whole};
use crate::keymap::{Act, Keymap};
use crate::prefs::Prefs;
use crate::sound::{Cue, Sound};

/// The window's icon, in pixels square.
const ICON_PX: u32 = 128;

const REPO: &str = "https://github.com/uchmk/fude";

/// The most notices the bell keeps; the oldest go first.
const NOTICES: usize = 100;

/// The draft is written this long after the last edit, in seconds.
const DRAFT_AFTER: f64 = 1.0;

/// The pointer moving this far, in points, brings the bar back after typing.
const STIR: f32 = 6.0;

const USAGE: &str = "\
fude — a one-line editor for writing

usage: fude [--keys SCRIPT] [--version | --help]

  --keys SCRIPT  press these keys once the window is up, as kura's --keys
                 (`<C-,>`, `<F1>`, `<Wait:200>`, `<State:name>`, `<Quit>`);
                 FUDE_KEYS_DONE names the file that says how it ended
";

/// What fills the window under the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// The writing band.
    Write,
    /// What was written, set on paper (Esc on the writing).
    Clean,
    Settings,
    /// The key list, over the writing.
    Help,
    /// The notebooks and their pieces (Ctrl+O).
    Shelf,
}

impl Screen {
    /// The name `--keys` reports.
    fn name(self) -> &'static str {
        match self {
            Screen::Write => "write",
            Screen::Clean => "clean",
            Screen::Settings => "settings",
            Screen::Help => "help",
            Screen::Shelf => "shelf",
        }
    }
}

pub struct App {
    table: ito_i18n::Table,
    /// The language in force, and common.toml's `language` it came from.
    lang: String,
    lang_asked: Option<String>,
    prefs: Prefs,
    keymap: Keymap,
    screen: Screen,
    /// Where the settings and the key list go back to: the writing, the
    /// clean copy or the shelf.
    back: Screen,
    clean: clean::Clean,
    /// The key list's first row shown, and what its last draw found out.
    help_scroll: usize,
    help_drawn: Cell<Option<ito_help::Drawn>>,
    /// What the bell lists, oldest first. `()`: no notice leads anywhere yet.
    notices: Vec<ito_notices::Notice<()>>,
    next_id: u64,
    /// The list under the bell is open, and where the bell was last drawn.
    bell_open: bool,
    bell_at: egui::Rect,
    /// The command palette while it is open (Ctrl+Shift+P, or the bar's box).
    palette: Option<ito_palette::View>,
    /// The settings' warnings already told, so each is told once.
    warned: Vec<String>,
    /// The window is to close at the end of this frame.
    quit: bool,
    /// Writes the window's place while common.toml's `[window]
    /// remember_place` is on (none for a script's window).
    tracker: Option<Tracker>,
    /// The `[window] always_on_top` last sent to the window.
    on_top: bool,
    /// Whether fude starts at sign-in, as the OS said (none until asked),
    /// and the answer on its way.
    startup: Option<bool>,
    startup_rx: Option<std::sync::mpsc::Receiver<(bool, Option<String>)>>,
    /// The `--keys` script, and what it asked of the frame loop.
    script: Option<Runner>,
    todo: Vec<Do>,
    /// The line being written, and what the input method is composing at
    /// its cursor.
    band: Band,
    preedit: String,
    /// The last edit or change of the composition, in egui's time.
    typed_at: f64,
    /// Typing hushes the bar until the pointer moves; where it was then.
    calm: bool,
    calm_at: Option<egui::Pos2>,
    /// draft.toml in the state folder (none for a script), and when it is
    /// next to be written, in egui's time.
    draft: Option<ito_common::kept::Writer>,
    draft_due: Option<f64>,
    /// The band was on the screen in the last frame.
    on_band: bool,
    /// The notebooks' folder (none: they are kept in memory, as for a
    /// script), the notebooks as last read, the one written in, the piece on
    /// the band (none until it has a letter), and the thread that reads and
    /// writes them.
    root: Option<PathBuf>,
    notes: Vec<notebook::Note>,
    note: String,
    piece: Option<String>,
    worker: Option<notebook::Worker>,
    shelf: shelf::Shelf,
    /// The topic shown over the band, of the piece on it.
    topic: Option<String>,
    /// The keys' sounds and the bell; none for a script, which is quiet.
    sound: Option<Sound>,
    /// The thread that makes the exported files, from the first export.
    export: Option<export::Worker>,
}

/// What the palette can do.
#[derive(Clone, Copy)]
enum Command {
    Clean,
    Write,
    Settings,
    /// The settings, on one of their pages.
    Page(usize),
    Paper(Paper),
    FlipDir,
    Copy,
    New,
    Shelf,
    Save,
    Help,
    ReportBug,
    ResetScale,
    Quit,
    Export(export::What, export::Whole),
}

impl App {
    /// fude on `prefs`, on the writing. The keymap's problems are notices.
    fn with(prefs: Prefs) -> Self {
        let lang_asked = prefs.common.language.clone();
        let lang = i18n::resolve(lang_asked.as_deref());
        let (table, errors) = i18n::table(&lang);
        let (keymap, warnings) = Keymap::load(prefs.keymap.as_deref());
        let mut band = Band::default();
        band.limits = prefs.config.limits();
        let mut app = Self {
            table,
            lang,
            lang_asked,
            prefs,
            keymap,
            screen: Screen::Write,
            back: Screen::Write,
            clean: clean::Clean::default(),
            help_scroll: 0,
            help_drawn: Cell::new(None),
            notices: Vec::new(),
            next_id: 0,
            bell_open: false,
            bell_at: egui::Rect::NOTHING,
            palette: None,
            warned: Vec::new(),
            quit: false,
            tracker: None,
            on_top: false,
            startup: None,
            startup_rx: None,
            script: None,
            todo: Vec::new(),
            band,
            preedit: String::new(),
            typed_at: f64::NEG_INFINITY,
            calm: false,
            calm_at: None,
            draft: None,
            draft_due: None,
            on_band: false,
            root: None,
            notes: Vec::new(),
            note: String::new(),
            piece: None,
            worker: None,
            shelf: shelf::Shelf::default(),
            topic: None,
            sound: None,
            export: None,
        };
        for w in errors.iter().chain(&warnings) {
            app.say(Kind::Warn, w);
        }
        app
    }

    /// A notice under the bell.
    pub fn say(&mut self, kind: Kind, text: &str) {
        self.next_id += 1;
        self.notices.push(ito_notices::Notice {
            id: self.next_id,
            at: std::time::SystemTime::now(),
            kind,
            title: text.to_owned(),
            note: None,
            go: None,
            read: false,
        });
        if self.notices.len() > NOTICES {
            self.notices.remove(0);
        }
    }

    /// What `--keys` writes for `<State:name>` and at the end.
    fn report(&self) -> String {
        let scale = (self.prefs.common.scale_or_one() * 100.0).round();
        let hit: Vec<&str> = self.band.hit().iter().map(|h| if *h == Hit::Chars { "chars" } else { "time" }).collect();
        format!(
            "screen: {}\nlang: {}\nscale: {scale}%\npalette: {}\nnotices: {}\nsettings_page: {}\nchars: {}\nback: {}\nhit: {}\npaper: {} {}\nnote: {}\npiece: {}\ntext: {:?}\n",
            self.screen.name(),
            self.lang,
            self.palette.is_some(),
            self.notices.len(),
            self.prefs.state.page,
            self.band.count(),
            self.band.back(),
            hit.join(" "),
            self.clean.sheet.paper.word(),
            self.clean.sheet.dir.word(),
            self.note,
            self.piece.as_deref().unwrap_or(""),
            self.band.text(),
        )
    }

    fn write_state(&self, path: &Path, tail: &str) {
        if let Err(e) = std::fs::write(path, format!("{}{tail}", self.report())) {
            ito_console::err(&format!("fude --keys: {}: {e}\n", path.display()));
        }
    }

    /// The script's next keys go in through `raw_input_hook`; what they ask
    /// of the app is done here.
    fn script(&mut self, ctx: &egui::Context) {
        let Some(shot) = self.script.as_mut().map(|s| s.shot(ctx)) else { return };
        if let Some(e) = shot {
            self.say(Kind::Err, &e);
        }
        for d in std::mem::take(&mut self.todo) {
            let Some(script) = &self.script else { return };
            match d {
                Do::State(name) => self.write_state(&script.dir.join(format!("{name}.txt")), ""),
                Do::Quit => {
                    self.quit = true;
                    // `<Quit>` finishes the run itself, so `on_exit` finds it over and writes nothing.
                    if let Some(done) = &script.done {
                        self.write_state(done, &ito_keyscript::quit_report(script.left()));
                    }
                }
                // The names are alphanumerics and `- _ .` only, so they are safe inside TOML quotes.
                Do::Theme(name) => {
                    let change = ito_common::CommonChange { table: None, key: "theme", value: format!("\"{name}\"") };
                    self.prefs.common_change(vec![change], "no config folder");
                }
                Do::Lang(code) => {
                    let change = ito_common::CommonChange { table: None, key: "language", value: format!("\"{code}\"") };
                    self.prefs.common_change(vec![change], "no config folder");
                }
                // fude defines no readings of its own.
                Do::Read { .. } => {}
                Do::Done => {
                    if let Some(done) = &script.done {
                        self.write_state(done, "keys: done\n");
                    }
                }
            }
        }
        if let Some(due) = self.script.as_ref().and_then(Runner::due) {
            ctx.request_repaint_after(due);
        }
    }

    /// What the watcher read since the last frame: the theme, the language,
    /// the keymap and the scale follow the files, and what went wrong is a
    /// notice.
    fn follow_prefs(&mut self, ctx: &egui::Context) {
        self.prefs.pump();
        self.prefs.apply_theme(ctx);
        if self.prefs.common.language != self.lang_asked {
            self.lang_asked = self.prefs.common.language.clone();
            let lang = i18n::resolve(self.lang_asked.as_deref());
            if lang != self.lang {
                let (table, errors) = i18n::table(&lang);
                (self.table, self.lang) = (table, lang);
                for e in errors {
                    self.say(Kind::Warn, &e);
                }
            }
        }
        match self.prefs.keymap_new.take() {
            Some(None) => {
                let (keymap, warnings) = Keymap::load(self.prefs.keymap.as_deref());
                self.keymap = keymap;
                for w in warnings {
                    self.say(Kind::Warn, &w);
                }
            }
            Some(Some(e)) => self.say(Kind::Warn, &e),
            None => {}
        }
        if let Some(e) = self.prefs.failed.take() {
            self.say(Kind::Err, &e);
        }
        // The files' warnings last while the files are as they are: each is
        // told once, and again if it comes back after going.
        let now: Vec<String> = self
            .prefs
            .common_warn
            .iter()
            .chain(&self.prefs.theme_warn)
            .chain(&self.prefs.theme_err)
            .chain(&self.prefs.config_warn)
            .cloned()
            .collect();
        for w in now.iter().filter(|w| !self.warned.contains(w)).cloned().collect::<Vec<_>>() {
            self.say(Kind::Warn, &w);
        }
        self.warned = now;
        let scale = self.prefs.common.scale_or_one();
        if (ctx.zoom_factor() - scale).abs() > f32::EPSILON {
            ctx.set_zoom_factor(scale);
        }
    }

    /// Ctrl+wheel and Ctrl+= / Ctrl+- / Ctrl+0: the whole window's scale, one
    /// step at a time. It is applied now and written to common.toml off the UI
    /// thread, so the other apps at the same desk follow within 2 seconds.
    fn rescale(&mut self, ctx: &egui::Context) {
        use ito_common::ScaleTo;
        let wheel = ctx.input(|i| if i.modifiers.command { i.smooth_scroll_delta.y } else { 0.0 });
        let to = if wheel > 0.0 {
            Some(ScaleTo::In)
        } else if wheel < 0.0 {
            Some(ScaleTo::Out)
        } else {
            let key = |k| ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, k));
            if key(egui::Key::Equals) || key(egui::Key::Plus) {
                Some(ScaleTo::In)
            } else if key(egui::Key::Minus) {
                Some(ScaleTo::Out)
            } else if key(egui::Key::Num0) {
                Some(ScaleTo::Reset)
            } else {
                None
            }
        };
        if let Some(to) = to {
            self.scale_to(ctx, to);
        }
    }

    fn scale_to(&mut self, ctx: &egui::Context, to: ito_common::ScaleTo) {
        let next = ito_common::scale_step(self.prefs.common.scale_or_one(), to);
        if next != self.prefs.common.scale_or_one() {
            self.prefs.set_scale(next);
            ctx.set_zoom_factor(next);
        }
        // Said at a limit too, where the scale stays as it is.
        ito_statusbar::scale_changed(ctx, next);
    }

    /// The keys the keymap binds, taken out of the frame's events before the
    /// screens see them. Not while the Keys page is taking keys, nor while
    /// the palette or the notices have them.
    fn keymap_keys(&mut self, ctx: &egui::Context) {
        if self.prefs.keys_edit.busy() || self.palette.is_some() || self.bell_open {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut used = Vec::new();
        for (n, e) in events.iter().enumerate() {
            let egui::Event::Key { key, pressed: true, modifiers, .. } = e else { continue };
            let Some(act) = ito_keys::from_egui(*key, modifiers).and_then(|k| self.keymap.act(k)) else { continue };
            self.run(act);
            used.push(n);
        }
        if !used.is_empty() {
            ctx.input_mut(|i| {
                let mut n = 0;
                i.events.retain(|_| {
                    n += 1;
                    !used.contains(&(n - 1))
                });
            });
        }
    }

    fn run(&mut self, act: Act) {
        match act {
            Act::Settings => self.toggle(Screen::Settings),
            Act::Help => {
                self.toggle(Screen::Help);
                self.help_scroll = 0;
            }
            Act::Quit => self.quit = true,
            Act::Shelf => self.toggle_shelf(),
            Act::Export => self.choose_export(false),
            Act::Print => self.choose_export(true),
            Act::Noop => {}
        }
    }

    /// `to`, or back to where it was opened from if it is already shown.
    fn toggle(&mut self, to: Screen) {
        if self.screen == to {
            self.screen = self.back;
            return;
        }
        if matches!(self.screen, Screen::Write | Screen::Clean | Screen::Shelf) {
            self.back = self.screen;
        }
        self.screen = to;
    }

    /// On the key list: Esc closes it, and the arrows, the pages, Home and
    /// End scroll it.
    fn help_keys(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Help || self.palette.is_some() || self.bell_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.screen = self.back;
            return;
        }
        use egui::Key as K;
        for k in [K::ArrowDown, K::ArrowUp, K::PageDown, K::PageUp, K::Home, K::End] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k)) {
                self.help_scroll_key(k);
            }
        }
    }

    /// The window's own bar: the name, the palette box, the bell and the
    /// caption buttons. The bell opens the list of notices.
    fn title_bar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let unread = ito_notices::unread(&self.notices);
        let t = &self.table;
        let tr = |k: &'static str| t.tr(k);
        let bell = t.trf("title.bell", &[&unread.to_string()]);
        let spec = ito_titlebar::Spec {
            own: !cfg!(target_os = "macos"),
            left: if cfg!(target_os = "macos") { 76.0 } else { 0.0 },
            maximized,
            unread,
            any_error: ito_notices::any_error(&self.notices),
            centre: true,
            right_min: 0.0,
            words: ito_titlebar::Words {
                search: tr("title.search"),
                search_key: "Ctrl+Shift+P",
                bell: &bell,
                minimize: tr("title.minimize"),
                maximize: tr("title.maximize"),
                restore: tr("title.restore"),
                close: tr("title.close"),
            },
        };
        let name = tr("app.title").to_owned();
        let out = ito_titlebar::show(
            ui,
            &spec,
            |ui, rect| {
                let galley = ui.painter().layout_no_wrap(name, egui::FontId::proportional(13.0), ito_theme::colors().fg);
                let x = rect.left() + 12.0;
                let right = x + galley.size().x;
                ui.painter().galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, egui::Color32::WHITE);
                right
            },
            |_, _| {},
        );
        if let Some(r) = out.bell_rect {
            self.bell_at = r;
        }
        if out.bell {
            self.bell_open = !self.bell_open;
        }
        if out.search {
            self.palette = Some(ito_palette::View::new());
        }
        if let Some(op) = out.window {
            ito_titlebar::apply(&ctx, op, maximized);
        }
    }

    fn commands(&self) -> Vec<ito_palette::Entry<Command>> {
        let t = &self.table;
        let tr = |k: &'static str| t.tr(k);
        let hint = |a| self.keymap.hint(a).unwrap_or_default();
        vec![
            ito_palette::Entry::new(tr("palette.clean"), "Esc", Command::Clean),
            ito_palette::Entry::new(tr("palette.write"), "Esc", Command::Write),
            ito_palette::Entry::new(tr("palette.settings"), hint(Act::Settings), Command::Settings),
            ito_palette::Entry::new(tr("palette.write_settings"), "", Command::Page(settings::WRITE)),
            ito_palette::Entry::new(tr("palette.clean_settings"), "", Command::Page(settings::NOTE)),
            ito_palette::Entry::new(tr("palette.paper_genko"), "", Command::Paper(Paper::Genko)),
            ito_palette::Entry::new(tr("palette.paper_ruled"), "", Command::Paper(Paper::Ruled)),
            ito_palette::Entry::new(tr("palette.paper_graph"), "", Command::Paper(Paper::Graph)),
            ito_palette::Entry::new(tr("palette.paper_plain"), "", Command::Paper(Paper::Plain)),
            ito_palette::Entry::new(tr("palette.flip_dir"), "", Command::FlipDir),
            ito_palette::Entry::new(tr("palette.copy"), "Ctrl+C", Command::Copy),
            ito_palette::Entry::new(tr("palette.new"), "Ctrl+N", Command::New),
            ito_palette::Entry::new(tr("palette.shelf"), hint(Act::Shelf), Command::Shelf),
            ito_palette::Entry::new(tr("palette.save"), "Ctrl+S", Command::Save),
            ito_palette::Entry::new(tr("palette.export_pdf"), hint(Act::Export), Command::Export(What::Pdf, Whole::Piece)),
            ito_palette::Entry::new(tr("palette.export_png"), "", Command::Export(What::Png, Whole::Piece)),
            ito_palette::Entry::new(tr("palette.export_note_pdf"), "", Command::Export(What::Pdf, Whole::Note)),
            ito_palette::Entry::new(tr("palette.export_note_png"), "", Command::Export(What::Png, Whole::Note)),
            ito_palette::Entry::new(tr("palette.print"), hint(Act::Print), Command::Export(What::Print, Whole::Piece)),
            ito_palette::Entry::new(tr("palette.print_note"), "", Command::Export(What::Print, Whole::Note)),
            ito_palette::Entry::new(tr("palette.help"), hint(Act::Help), Command::Help),
            ito_palette::Entry::new(tr("palette.report_bug"), "F12", Command::ReportBug),
            ito_palette::Entry::new(tr("palette.reset_scale"), "Ctrl+0", Command::ResetScale),
            ito_palette::Entry::new(tr("palette.quit"), hint(Act::Quit), Command::Quit),
        ]
    }

    /// Ctrl+Shift+P opens the palette; while it is open it takes the keys.
    fn command_palette(&mut self, ctx: &egui::Context) {
        let open = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P));
        if open && self.palette.is_none() && !self.prefs.keys_edit.busy() {
            self.palette = Some(ito_palette::View::new());
        }
        let Some(mut view) = self.palette.take() else { return };
        let c = ito_theme::colors();
        let style = ito_palette::Style {
            panel: c.raised(),
            border: c.border,
            text: c.fg,
            dim: c.dim,
            selected_bg: c.chosen(),
            selected_text: c.strong(),
            hit: c.wait,
            accent: c.done,
        };
        let t = &self.table;
        let tr = |k: &'static str| t.tr(k);
        let words = ito_palette::Words { hint: tr("palette.hint"), none: tr("palette.none"), header: "", footer: "" };
        match ito_palette::show(ctx, &mut view, &self.commands(), &style, &words) {
            None => self.palette = Some(view),
            Some(ito_palette::Answer::Pick(c)) => self.command(ctx, c),
            Some(_) => {}
        }
    }

    /// What a command picked in the palette does.
    fn command(&mut self, ctx: &egui::Context, c: Command) {
        let time = ctx.input(|i| i.time);
        // The clean copy's own commands open it first.
        let on_clean = |app: &mut App| {
            if app.screen != Screen::Clean {
                app.open_clean();
            }
        };
        match c {
            Command::Clean => self.open_clean(),
            Command::Write if self.screen == Screen::Clean || self.back == Screen::Clean => self.write_on(),
            Command::Write => self.screen = Screen::Write,
            Command::Settings => self.run(Act::Settings),
            Command::Page(page) => {
                self.prefs.state.page = page;
                if self.screen != Screen::Settings {
                    self.toggle(Screen::Settings);
                }
            }
            Command::Paper(p) => {
                on_clean(self);
                self.clean.set_sheet(draw::Sheet { paper: p, ..self.clean.sheet });
            }
            Command::FlipDir => {
                on_clean(self);
                let dir = if self.clean.sheet.dir == Dir::Vertical { Dir::Horizontal } else { Dir::Vertical };
                self.clean.set_sheet(draw::Sheet { dir, ..self.clean.sheet });
            }
            Command::Copy => {
                on_clean(self);
                self.copy(ctx, time);
            }
            Command::New => self.new_piece(time),
            Command::Shelf => self.toggle_shelf(),
            Command::Save => self.save_piece(time),
            Command::Help => self.run(Act::Help),
            Command::ReportBug => self.report_bug(ctx),
            Command::ResetScale => self.scale_to(ctx, ito_common::ScaleTo::Reset),
            Command::Quit => self.quit = true,
            Command::Export(what, whole) => self.export(ctx, what, whole),
        }
    }

    /// The list under the bell, while it is open.
    fn notice_list(&mut self, ctx: &egui::Context) {
        if !self.bell_open {
            return;
        }
        let t = &self.table;
        let tr = |k: &'static str| t.tr(k);
        let title = t.trf("notices.title", &[&ito_notices::unread(&self.notices).to_string()]);
        let words = ito_notices::Words {
            title: &title,
            mark_read: tr("notices.mark_read"),
            empty: tr("notices.empty"),
            ago: tr("notices.ago"),
            kinds: [tr("notices.ok"), tr("notices.info"), tr("notices.warn"), tr("notices.err"), tr("notices.wait")],
        };
        match ito_notices::show(ctx, self.bell_at, &self.notices, &words) {
            Some(ito_notices::Action::Open(_, id)) => {
                self.notices.iter_mut().filter(|n| n.id == id).for_each(|n| n.read = true);
                self.bell_open = false;
            }
            Some(ito_notices::Action::ReadAll) => self.notices.iter_mut().for_each(|n| n.read = true),
            Some(ito_notices::Action::Close) => self.bell_open = false,
            None => {}
        }
    }

    /// F12: the browser opens the issue form filled with the version, the OS
    /// and what the app knows.
    pub fn report_bug(&self, ctx: &egui::Context) {
        let spec = ito_bugreport::Spec { app: "fude", version: env!("CARGO_PKG_VERSION"), repo: REPO, template: "bug_report.yml" };
        let context = bug_context(self.prefs.base.as_deref(), ctx.pixels_per_point());
        let url = ito_bugreport::url(&spec, &[("context", context)]);
        ctx.open_url(egui::OpenUrl::new_tab(url));
    }

    /// The band takes the keys and the input method: on the writing, and on
    /// the clean copy, whose first letter writes on.
    /// Not while the palette, the notices or the Keys page have them.
    fn typing_here(&self) -> bool {
        matches!(self.screen, Screen::Write | Screen::Clean) && self.palette.is_none() && !self.bell_open && !self.prefs.keys_edit.busy()
    }

    /// The input method at the band's input point, set the way the band runs.
    fn ime(&self) -> ito_ime::Ime {
        let orientation = match self.prefs.config.dir {
            Dir::Horizontal => ito_ime::Orientation::Horizontal,
            Dir::Vertical => ito_ime::Orientation::Vertical,
        };
        ito_ime::Ime::new(egui::Id::new("fude-band")).orientation(orientation)
    }

    /// The clock runs only while the band is on the screen, and limits
    /// changed in the settings are the band's at once. Leaving the band
    /// binds the piece and writes the draft.
    fn follow_band(&mut self, now: Instant, time: f64) {
        let on = self.screen == Screen::Write;
        let limits = self.prefs.config.limits();
        if on == self.on_band && limits == self.band.limits {
            return;
        }
        self.band.pause(now);
        self.band.limits = limits;
        if on {
            self.band.enter(now, self.prefs.config.time_start == TimeStart::Open);
        } else {
            self.bind_piece();
            self.draft_due = Some(time);
        }
        self.on_band = on;
    }

    /// Typing hushes the bar; the pointer moving brings it back.
    fn follow_calm(&mut self, ctx: &egui::Context) {
        if !self.calm {
            return;
        }
        let at = ctx.input(|i| i.pointer.latest_pos());
        let stirred = match (at, self.calm_at) {
            (Some(a), Some(b)) => a.distance(b) > STIR,
            (a, b) => a.is_some() != b.is_some(),
        };
        if stirred || self.screen != Screen::Write {
            self.calm = false;
        }
    }

    /// draft.toml, once the edits have rested a moment. The file is written
    /// off the UI thread.
    fn keep_draft(&mut self, ctx: &egui::Context, time: f64) {
        let Some(due) = self.draft_due else { return };
        if time < due {
            ctx.request_repaint_after_secs((due - time) as f32);
            return;
        }
        self.draft_due = None;
        if let Some(w) = &self.draft {
            w.save_toml(&self.draft_now());
        }
    }

    /// `cue` sounds, a letter's and a paragraph's as the settings say.
    pub fn sound(&mut self, cue: Cue, now: Instant) {
        let c = &self.prefs.config;
        if let Some(s) = &mut self.sound {
            s.play(cue, c.sound, c.volume, now);
        }
    }

    /// The band: what is typed, committed and pasted goes onto it when
    /// `typing`, and it is drawn. Each frame of typing makes one sound, and
    /// a limit reached rings the bell.
    fn write_screen(&mut self, ui: &mut egui::Ui, typing: bool) {
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let now = Instant::now();
        let time = ctx.input(|i| i.time);
        let config = self.prefs.config;
        let hits = self.band.hit().len();
        let mut cue = None;
        if typing {
            let f = self.ime().frame(ui, write::caret_at(rect, &config), self.band.cursor() as u64);
            let preedit = if f.composing { f.preedit } else { String::new() };
            let mut typed = preedit != self.preedit;
            if typed {
                cue = Some(Cue::Letter);
            }
            if !preedit.is_empty() && !self.band.started() && config.time_start == TimeStart::First {
                self.band.start(now);
            }
            self.preedit = preedit;
            // The input method's commits come as `Ime::Commit` alone, so the
            // events are read here rather than the frame's commits.
            let events = ctx.input(|i| i.events.clone());
            let mut edited = false;
            for e in events.iter().filter_map(|e| write::edit_of(e, config.dir)) {
                // Moving along the band is quiet.
                let sounds = match e {
                    Edit::Paragraph => Some(Cue::Paragraph),
                    Edit::Text(_) | Edit::Backspace | Edit::Delete => Some(Cue::Letter),
                    _ => None,
                };
                if self.band.edit(e, now, config.no_erase) {
                    edited = true;
                    cue = cue.max(sounds);
                }
            }
            if edited {
                self.draft_due = Some(time + DRAFT_AFTER);
            }
            typed |= edited;
            if typed {
                self.typed_at = time;
                self.calm = true;
                self.calm_at = ctx.input(|i| i.pointer.latest_pos());
            }
        } else {
            self.preedit.clear();
        }
        if self.band.running() {
            self.band.tick(now);
        }
        if let Some(cue) = cue {
            self.sound(cue, now);
        }
        let reached = self.band.hit().len() > hits;
        if config.bell && reached {
            self.sound(Cue::Bell, now);
        }
        // Behind another window, a limit reached is told the ways `[notify]`
        // picks; a script tells nobody.
        if reached && self.script.is_none() && ctx.input(|i| i.viewport().focused) == Some(false) {
            let body = write::hit_words(&self.band, &self.table);
            notify::tell(&self.prefs.notify, self.table.tr("write.hit_title"), &body, self.table.tr("write.hit_open"));
        }
        let settings_key = self.keymap.hint(Act::Settings).unwrap_or_else(|| "—".to_owned());
        let view = write::View {
            band: &self.band,
            config: &config,
            preedit: &self.preedit,
            now,
            typed_at: self.typed_at,
            settings_key: &settings_key,
            table: &self.table,
            topic: self.topic.as_deref(),
        };
        write::draw(ui, rect, &view);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let window = self.prefs.common.window_or_default();
        if let Some(t) = self.tracker.as_mut().filter(|_| window.remember_place) {
            t.follow(&ctx);
        }
        if window.always_on_top != self.on_top {
            self.on_top = window.always_on_top;
            let level = if self.on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
        }
        self.script(&ctx);
        // A click on a limit's notification only brings the window forward:
        // the writing is the one place it leads to.
        ito_prefs::take_clicks();
        self.follow_prefs(&ctx);
        self.follow_notes();
        self.follow_exports(ui.ctx());
        let typing = self.typing_here();
        // Enter, Backspace and the arrows are the input method's while it composes.
        if typing {
            self.ime().swallow_keys(&ctx);
        }
        // While the Keys page takes a key, every key is the key.
        if !self.prefs.keys_edit.busy() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F12)) {
                self.report_bug(&ctx);
            }
            self.rescale(&ctx);
            self.keymap_keys(&ctx);
            self.help_keys(&ctx);
        }
        let (now, time) = (Instant::now(), ctx.input(|i| i.time));
        self.clean_keys(&ctx, time);
        self.shelf_keys(&ctx, time);
        self.follow_band(now, time);
        self.follow_calm(&ctx);
        let hush = ctx.animate_bool_with_time(egui::Id::new("fude-calm"), self.calm, 0.3);
        egui::Panel::top("titlebar").exact_size(ito_titlebar::HEIGHT).frame(egui::Frame::NONE.fill(ito_theme::colors().bg)).show(
            ui,
            |ui| {
                ui.multiply_opacity(1.0 - 0.85 * hush);
                self.title_bar(ui);
            },
        );
        if self.screen != Screen::Settings {
            egui::Panel::bottom("fude-status")
                .exact_size(ito_statusbar::HEIGHT)
                .frame(egui::Frame::NONE)
                .show_separator_line(false)
                .show(ui, |ui| status::show(ui, self, 1.0 - 0.85 * hush));
        }
        // The screen may have changed under the keys just now.
        let typing = typing && self.typing_here();
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(ito_theme::colors().bg)).show(ui, |ui| match self.screen {
            Screen::Settings => settings::show(ui, self),
            Screen::Write => self.write_screen(ui, typing),
            Screen::Clean => clean::show(ui, self, false),
            Screen::Shelf => shelf::show(ui, self, false),
            Screen::Help => {
                if self.back == Screen::Clean {
                    clean::show(ui, self, true);
                } else if self.back == Screen::Shelf {
                    shelf::show(ui, self, true);
                } else {
                    self.write_screen(ui, false);
                }
                let r = ui.max_rect();
                help::show(ui, self, r);
            }
        });
        self.keep_draft(&ctx, time);
        if let Some(due) = self.sound.as_mut().and_then(|s| s.rest(Instant::now())) {
            ctx.request_repaint_after(due);
        }
        self.notice_list(&ctx);
        self.command_palette(&ctx);
        if self.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // Last, so the edges of the frameless window win over what is under them.
        if !cfg!(target_os = "macos") {
            ito_titlebar::resize_edges(&ctx);
        }
    }

    /// The window's place, the piece and the draft are written as it
    /// closes, and a window that closes while a script runs says so in the
    /// done file.
    fn on_exit(&mut self) {
        if let Some(t) = self.tracker.as_mut().filter(|_| self.prefs.common.window_or_default().remember_place) {
            t.close(None);
        }
        self.bind_piece();
        if let Some(w) = &self.draft {
            let saved = toml::to_string(&self.draft_now()).map_err(|e| e.to_string()).and_then(|text| w.save_now(text));
            if let Err(e) = saved {
                ito_console::err(&format!("fude: the draft: {e}\n"));
            }
        }
        // The worker binds what is still to be bound before it ends.
        if let Some(w) = &mut self.worker {
            w.finish();
        }
        if let Some(w) = &mut self.export {
            w.finish();
        }
        let Some(script) = &mut self.script else { return };
        if !script.running() {
            return;
        }
        let left = script.left();
        script.finish();
        let Some(done) = script.done.clone() else { return };
        let tail = if self.quit { ito_keyscript::quit_report(left) } else { ito_keyscript::closed_report(left) };
        self.write_state(&done, &tail);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some(script) = &mut self.script {
            if let Some(d) = script.input(ctx, raw) {
                self.todo.push(d);
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        ito_theme::colors().bg.to_normalized_gamma_f32()
    }
}

/// The report's `context` field: the scale, and the config files fude read
/// from the `uchmk` folder `base` (the ones there), named without the home.
fn bug_context(base: Option<&Path>, scale: f32) -> String {
    let home = ito_common::home_dir();
    let files: Vec<PathBuf> = base
        .map(|b| [ito_common::common_path(b), prefs::keymap_path(b), config::path(b)])
        .into_iter()
        .flatten()
        .filter(|p| p.is_file())
        .collect();
    ito_bugreport::context(&ito_bugreport::Context {
        home: home.as_deref(),
        scale: Some(scale),
        config: &files,
        anchors: &["uchmk", "fude"],
        ..Default::default()
    })
}

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Cli {
    Version,
    Help,
    /// Open the window, with a script to play into it.
    Open(Option<Vec<Step>>),
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut script = None;
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--keys") => {
                let s = args.next().ok_or("--keys needs a script")?;
                let s = s.to_str().ok_or("--keys: the script is not text")?;
                script = Some(ito_keyscript::parse(s).map_err(|e| format!("--keys: {e}"))?);
            }
            Some("--version" | "-V") => return Ok(Cli::Version),
            Some("--help" | "-h") => return Ok(Cli::Help),
            _ => return Err(format!("unknown argument `{}`", a.to_string_lossy())),
        }
    }
    Ok(Cli::Open(script))
}

/// Puts a line where whoever typed the command is looking, on standard error
/// when `err`. A release build on Windows has no console of its own;
/// `ito_console` writes to the console it was started from, or to the pipe
/// `fude.com` hands on, and lets go of a write that fails.
fn say(text: &str, err: bool) {
    let line = format!("{text}\n");
    match err {
        true => ito_console::err(&line),
        false => ito_console::out(&line),
    }
}

fn main() -> eframe::Result {
    let script = match parse_args(std::env::args_os().skip(1)) {
        Ok(Cli::Version) => {
            say(&ito_bugreport::version_line("fude", env!("CARGO_PKG_VERSION")), false);
            return Ok(());
        }
        Ok(Cli::Help) => {
            say(USAGE, false);
            return Ok(());
        }
        Ok(Cli::Open(script)) => script,
        Err(e) => {
            say(&format!("fude: {e}\n\n{USAGE}"), true);
            std::process::exit(2);
        }
    };
    let done = std::env::var_os("FUDE_KEYS_DONE").filter(|v| !v.is_empty()).map(PathBuf::from);
    if let (Some(_), Some(done)) = (&script, &done) {
        ito_keyscript::install_panic_report(done.clone());
    }
    // A script plays on a window that keeps no place, and leaves none.
    let saved = if script.is_none() { Saved::load(ito_common::state_dir("fude")) } else { Saved::load(None) };
    // Told not to keep its place, the window opens as new, and is followed
    // again once that is turned back on.
    let first = ito_common::base_dir().and_then(|b| ito_common::Common::read(&b).ok()).unwrap_or_default().0.window_or_default();
    let opened = if first.remember_place { saved.clone() } else { Saved::load(None) };
    // Nor does it go on from the draft, or write one.
    let draft =
        ito_common::state_dir("fude").filter(|_| script.is_none()).map(|d| ito_common::kept::Writer::new(d.join("draft.toml"), true));
    let viewport = egui::ViewportBuilder::default()
        .with_title("fude")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([720.0, 420.0])
        .with_icon(std::sync::Arc::new(egui::IconData { rgba: icon::pixels(ICON_PX), width: ICON_PX, height: ICON_PX }));
    // The bar is the app's own (the Mac keeps its traffic lights over it).
    let viewport = ito_titlebar::frameless(viewport);
    let viewport = opened.viewport(viewport);
    let viewport = if first.always_on_top { viewport.with_always_on_top() } else { viewport };
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "fude",
        options,
        Box::new(move |cc| {
            opened.put(cc);
            let mut prefs = Prefs::load(ito_common::base_dir());
            ito_prefs::notify_window(&cc.egui_ctx);
            fonts::load_in_background(cc.egui_ctx.clone());
            prefs.watch(&cc.egui_ctx);
            prefs.apply_theme(&cc.egui_ctx);
            // The scale is common.toml's, so egui's own Ctrl+= / Ctrl+- stay out of it.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            cc.egui_ctx.set_zoom_factor(prefs.common.scale_or_one());
            let mut app = App::with(prefs);
            app.tracker = saved.tracker();
            app.on_top = first.always_on_top;
            // The writing goes on where it was left, in the notebook it was
            // in. A script keeps its notebooks in memory, unless it is given
            // a documents folder of its own.
            let kept = draft.as_ref().and_then(|w| ito_common::kept::read::<shelf::Draft>(w.path()));
            let own_docs = std::env::var_os("UCHMK_DOCUMENTS_DIR").is_some();
            app.root = notebook::root().filter(|_| script.is_none() || own_docs);
            let ctx = cc.egui_ctx.clone();
            app.start_notes(kept, move || ctx.request_repaint());
            app.draft = draft;
            app.sound = script.is_none().then(Sound::default);
            app.script = script.map(|steps| {
                let mut r = Runner::new(steps, done);
                r.watch(cc.egui_ctx.clone());
                r
            });
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Result<Cli, String> {
        parse_args(a.iter().map(OsString::from))
    }

    #[test]
    fn the_command_line_reads() {
        assert_eq!(args(&[]), Ok(Cli::Open(None)));
        assert_eq!(args(&["--version"]), Ok(Cli::Version));
        assert_eq!(args(&["-h"]), Ok(Cli::Help));
        assert!(matches!(args(&["--keys", "<C-,><Quit>"]), Ok(Cli::Open(Some(s))) if s.len() == 2));
        assert!(args(&["--keys"]).unwrap_err().contains("needs a script"));
        assert!(args(&["--frob"]).unwrap_err().contains("`--frob`"));
        assert!(args(&["notes.txt"]).is_err(), "no file is opened from the command line yet");
    }

    /// The report names the config files that are there, from `uchmk` down.
    #[test]
    fn the_report_names_the_config_files_read() {
        let base = std::env::temp_dir().join(format!("fude-bug-{}", std::process::id())).join("uchmk");
        std::fs::create_dir_all(base.join("fude")).unwrap();
        assert!(bug_context(Some(&base), 1.25).ends_with("Scale: 125%\nConfig: defaults only"));
        std::fs::write(ito_common::common_path(&base), "").unwrap();
        std::fs::write(config::path(&base), "").unwrap();
        let sep = std::path::MAIN_SEPARATOR;
        let said = bug_context(Some(&base), 1.0);
        assert!(said.ends_with(&format!("Config: uchmk{sep}common.toml, uchmk{sep}fude{sep}config.toml")), "{said}");
        assert!(bug_context(None, 1.0).ends_with("Config: defaults only"));
        let _ = std::fs::remove_dir_all(base.parent().unwrap());
    }

    /// One frame of `a` with `events`, as eframe would run it.
    fn frame(ctx: &egui::Context, a: &mut App, events: Vec<egui::Event>) {
        frame_at(ctx, a, events, None);
    }

    /// One frame at `time` in seconds (egui's own step when none).
    fn frame_at(ctx: &egui::Context, a: &mut App, events: Vec<egui::Event>, time: Option<f64>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0))),
            events,
            time,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| eframe::App::ui(a, ui, &mut eframe::Frame::_new_kittest()));
        out.textures_delta.clear();
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]
    }

    /// The keymap's keys open the screens and close them again, Esc closes
    /// the key list, and the report says where the window is.
    #[test]
    fn the_keys_open_and_close_the_screens() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        let ctrl = egui::Modifiers { ctrl: true, command: true, ..Default::default() };
        frame(&ctx, &mut a, key(egui::Key::F1, egui::Modifiers::NONE));
        assert_eq!(a.screen, Screen::Help);
        frame(&ctx, &mut a, key(egui::Key::Escape, egui::Modifiers::NONE));
        assert_eq!(a.screen, Screen::Write);
        frame(&ctx, &mut a, key(egui::Key::Comma, ctrl));
        assert_eq!(a.screen, Screen::Settings);
        assert!(a.report().contains("screen: settings\n"), "{}", a.report());
        frame(&ctx, &mut a, key(egui::Key::Comma, ctrl));
        assert_eq!(a.screen, Screen::Write);
        frame(&ctx, &mut a, key(egui::Key::Q, ctrl));
        assert!(a.quit);
    }

    /// While the palette is open the keymap's keys are its, not the
    /// screens'.
    #[test]
    fn the_palette_keeps_the_keys() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        a.palette = Some(ito_palette::View::new());
        frame(&ctx, &mut a, key(egui::Key::F1, egui::Modifiers::NONE));
        assert_eq!(a.screen, Screen::Write);
    }

    /// The palette opens the settings on the writing's page or the
    /// notebook's, and every command it lists has its words.
    #[test]
    fn the_palette_opens_a_settings_page() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        let commands = a.commands();
        assert!(commands.iter().all(|e| !e.title.is_empty() && !e.title.starts_with("palette.")));
        a.command(&ctx, Command::Page(settings::WRITE));
        assert_eq!((a.screen, a.prefs.state.page), (Screen::Settings, settings::WRITE));
        a.command(&ctx, Command::Page(settings::NOTE));
        assert_eq!((a.screen, a.prefs.state.page), (Screen::Settings, settings::NOTE));
    }

    /// A keymap.toml read again remakes the keys; one with a mistake says so.
    #[test]
    fn a_keymap_read_again_is_followed() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        a.prefs.keymap = Some("[app]\nprepend_keymap = [{ on = \"<C-k>\", run = \"frob\" }, { on = \"<F2>\", run = \"help\" }]".into());
        a.prefs.keymap_new = Some(None);
        frame(&ctx, &mut a, Vec::new());
        assert_eq!(a.keymap.act(ito_keys::Key::parse("<F2>").unwrap()), Some(Act::Help));
        assert_eq!(a.notices.len(), 1, "{:?}", a.notices.iter().map(|n| &n.title).collect::<Vec<_>>());
        assert_eq!(a.notices[0].kind, Kind::Warn);
    }

    /// Letters, the input method's commits and pastes go onto the band;
    /// Enter ends a paragraph (not at the start), ← goes back to fix a
    /// letter, and the report says where the cursor is. The no-erase mode
    /// keeps what is written.
    #[test]
    fn typing_goes_onto_the_band() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        let none = egui::Modifiers::NONE;
        frame(&ctx, &mut a, key(egui::Key::Enter, none));
        assert_eq!(a.band.text(), "", "no paragraph before a letter");
        frame(&ctx, &mut a, vec![egui::Event::Text("ab".into()), egui::Event::Ime(egui::ImeEvent::Commit("かな".into()))]);
        frame(&ctx, &mut a, key(egui::Key::Enter, none));
        frame(&ctx, &mut a, vec![egui::Event::Paste("x\ty".into())]);
        assert_eq!(a.band.text(), "abかな\nx y", "a tab pasted is a space");
        frame(&ctx, &mut a, key(egui::Key::ArrowLeft, none));
        frame(&ctx, &mut a, key(egui::Key::Backspace, none));
        assert_eq!(a.band.text(), "abかな\nxy");
        let report = a.report();
        assert!(report.contains("back: 1\n") && report.contains("text: \"abかな\\nxy\"\n"), "{report}");
        a.prefs.config.no_erase = true;
        frame(&ctx, &mut a, key(egui::Key::Backspace, none));
        frame(&ctx, &mut a, key(egui::Key::ArrowLeft, none));
        assert_eq!((a.band.text().as_str(), a.band.back()), ("abかな\nxy", 1), "nothing is erased or moved");
    }

    /// The band is written to draft.toml once the edits rest, and the
    /// draft read back makes the same band.
    #[test]
    fn the_draft_is_written_and_goes_on() {
        let dir = std::env::temp_dir().join(format!("fude-draft-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        a.draft = Some(ito_common::kept::Writer::new(dir.join("draft.toml"), true));
        frame_at(&ctx, &mut a, vec![egui::Event::Text("書きかけ".into())], Some(1.0));
        frame_at(&ctx, &mut a, key(egui::Key::ArrowLeft, egui::Modifiers::NONE), Some(1.2));
        assert!(a.draft_due.is_some(), "not yet: the edits have not rested");
        frame_at(&ctx, &mut a, Vec::new(), Some(1.2 + DRAFT_AFTER));
        assert!(a.draft_due.is_none());
        let path = dir.join("draft.toml");
        let mut kept = None;
        for _ in 0..500 {
            kept = ito_common::kept::read::<shelf::Draft>(&path).map(|d| d.band).filter(|k| k.back == 1);
            if kept.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let kept = kept.expect("draft.toml is written");
        let again = Band::from_kept(kept, a.prefs.config.limits());
        assert_eq!((again.text(), again.back()), ("書きかけ".to_owned(), 1));
        assert!(again.started() && !again.running(), "the clock stands until the band is shown");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Esc shows the clean copy and Enter writes on; a letter typed on it
    /// writes on too, onto the band. The key list over it goes back to it.
    #[test]
    fn the_clean_copy_opens_and_writes_on() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        let none = egui::Modifiers::NONE;
        frame(&ctx, &mut a, vec![egui::Event::Text("あいう".into())]);
        frame(&ctx, &mut a, key(egui::Key::Escape, none));
        assert_eq!(a.screen, Screen::Clean);
        assert!(a.report().contains("screen: clean\n"), "{}", a.report());
        frame(&ctx, &mut a, key(egui::Key::Enter, none));
        assert_eq!(a.screen, Screen::Write);
        assert_eq!(a.band.text(), "あいう", "the Enter that wrote on is no paragraph");
        frame(&ctx, &mut a, key(egui::Key::Escape, none));
        frame(&ctx, &mut a, vec![egui::Event::Text("え".into())]);
        assert_eq!(a.screen, Screen::Write);
        assert_eq!(a.band.text(), "あいうえ", "the letter that wrote on is on the band");
        frame(&ctx, &mut a, key(egui::Key::Escape, none));
        frame(&ctx, &mut a, key(egui::Key::F1, none));
        assert_eq!(a.screen, Screen::Help);
        frame(&ctx, &mut a, key(egui::Key::Escape, none));
        assert_eq!(a.screen, Screen::Clean, "the key list goes back to the clean copy");
    }

    /// Ctrl+E on the clean copy offers the ways out, and Enter takes the
    /// first: the piece as a PDF in its notebook's export folder.
    #[test]
    fn the_clean_copy_is_exported() {
        let dir = std::env::temp_dir().join(format!("fude-export-app-{}", std::process::id()));
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        a.root = Some(dir.clone());
        // As under --keys, the folder is not opened.
        a.script = Some(Runner::new(Vec::new(), None));
        let (ctrl, none) = (egui::Modifiers { ctrl: true, command: true, ..Default::default() }, egui::Modifiers::NONE);
        frame(&ctx, &mut a, vec![egui::Event::Text("吾輩は猫である。".into())]);
        frame(&ctx, &mut a, key(egui::Key::Escape, none));
        let stem = a.piece.clone().expect("leaving the band names the piece");
        frame(&ctx, &mut a, key(egui::Key::E, ctrl));
        assert_eq!(a.palette.as_ref().map(|p| p.query.as_str()), Some(a.table.tr("palette.export_query")));
        frame(&ctx, &mut a, key(egui::Key::Enter, none));
        assert!(a.palette.is_none());
        let pdf = a.notes[0].dir.join(export::FOLDER).join(format!("{stem}.pdf"));
        let said = |a: &App| a.clean.said.as_ref().map(|s| s.1.clone()).unwrap_or_default();
        let wrote = a.table.trf("export.pdf", &[&Path::new(export::FOLDER).join(format!("{stem}.pdf")).display().to_string()]);
        for _ in 0..600 {
            if said(&a) == wrote {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            frame(&ctx, &mut a, Vec::new());
        }
        assert_eq!(said(&a), wrote);
        assert_eq!(a.clean.said.as_ref().map(|s| s.0), Some(ito_notices::Kind::Ok), "what was written is said as done");
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
        if let Some(w) = &mut a.export {
            w.finish();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Ctrl+N binds what is written into its piece and starts a new one at
    /// once, on the writing.
    #[test]
    fn a_new_piece_starts_at_once() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        let ctrl = egui::Modifiers { ctrl: true, command: true, ..Default::default() };
        frame(&ctx, &mut a, vec![egui::Event::Text("書いた".into())]);
        frame(&ctx, &mut a, key(egui::Key::Escape, egui::Modifiers::NONE));
        assert_eq!(a.screen, Screen::Clean);
        let stem = a.piece.clone().expect("leaving the band names the piece");
        frame(&ctx, &mut a, key(egui::Key::N, ctrl));
        assert_eq!((a.screen, a.band.text().as_str(), a.piece.as_deref()), (Screen::Write, "", None));
        assert_eq!(a.notes[0].piece(&stem).map(|p| p.text.as_str()), Some("書いた"), "the piece is kept");
        frame(&ctx, &mut a, key(egui::Key::N, ctrl));
        assert_eq!(a.notes[0].pieces.len(), 1, "an empty band is no piece");
    }

    /// Ctrl+O opens the shelf on the notebook written in; Esc goes back,
    /// and a piece picked there opens on its clean copy.
    #[test]
    fn the_shelf_opens_and_closes() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        let (ctrl, none) = (egui::Modifiers { ctrl: true, command: true, ..Default::default() }, egui::Modifiers::NONE);
        frame_at(&ctx, &mut a, vec![egui::Event::Text("一篇め".into())], Some(1.0));
        frame_at(&ctx, &mut a, key(egui::Key::N, ctrl), Some(1.1));
        frame_at(&ctx, &mut a, vec![egui::Event::Text("二篇め".into())], Some(1.2));
        frame_at(&ctx, &mut a, key(egui::Key::O, ctrl), Some(2.0));
        assert_eq!(a.screen, Screen::Shelf);
        assert!(a.report().contains("screen: shelf\n"), "{}", a.report());
        assert_eq!((a.shelf.open.as_deref(), a.shelf.sel), (Some(""), 1), "at the piece on the band");
        frame_at(&ctx, &mut a, key(egui::Key::Escape, none), Some(2.05));
        assert_eq!(a.screen, Screen::Shelf, "not the Esc of the frame it opened in");
        frame_at(&ctx, &mut a, key(egui::Key::Escape, none), Some(2.5));
        assert_eq!(a.screen, Screen::Write);
        frame_at(&ctx, &mut a, key(egui::Key::O, ctrl), Some(3.0));
        frame_at(&ctx, &mut a, key(egui::Key::ArrowDown, none), Some(3.5));
        frame_at(&ctx, &mut a, key(egui::Key::Enter, none), Some(3.6));
        assert_eq!((a.screen, a.band.text().as_str()), (Screen::Clean, "一篇め"));
        frame_at(&ctx, &mut a, key(egui::Key::ArrowLeft, none), Some(3.7));
        assert_eq!(a.band.text(), "二篇め", "on vertical paper, ← is the newer piece");
        frame_at(&ctx, &mut a, key(egui::Key::ArrowRight, none), Some(3.8));
        assert_eq!(a.band.text(), "一篇め");
    }

    /// A limit met, then the clean copy and writing on: the limit grows, so
    /// the band goes on without it.
    #[test]
    fn writing_on_past_a_limit_raises_it() {
        let ctx = egui::Context::default();
        let mut a = App::with(Prefs::empty());
        a.prefs.config.char_limit = 3;
        frame(&ctx, &mut a, Vec::new());
        frame(&ctx, &mut a, vec![egui::Event::Text("あいう".into())]);
        assert_eq!(a.band.hit(), [band::Hit::Chars]);
        frame(&ctx, &mut a, key(egui::Key::Escape, egui::Modifiers::NONE));
        frame(&ctx, &mut a, key(egui::Key::Enter, egui::Modifiers::NONE));
        assert!(a.band.hit().is_empty(), "{:?}", a.band.hit());
        frame(&ctx, &mut a, vec![egui::Event::Text("え".into())]);
        assert!(a.band.hit().is_empty(), "the limit grew past 4");
    }
}
