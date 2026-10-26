//! fude: a one-line editor for writing, and a clean copy of what was written.
//!
//! What is wired here: the window (its place kept, its own title bar with the
//! bell and the notice list), common.toml's theme, language and scale, the
//! keymap, the settings screen, the key list, the palette, `--keys`,
//! `--version` / `--help`, and F12 (the bug report). The writing band, the
//! clean copy and the notebooks come next (TODO.md).

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod fonts;
mod help;
mod i18n;
mod icon;
mod keymap;
mod prefs;
mod settings;

use std::cell::Cell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ito_common::window::{Saved, Tracker};
use ito_keyscript::{Do, Runner, Step};
use ito_notices::Kind;

use crate::keymap::{Act, Keymap};
use crate::prefs::Prefs;

/// The window's icon, in pixels square.
const ICON_PX: u32 = 128;

const REPO: &str = "https://github.com/uchmk/fude";

/// The most notices the bell keeps; the oldest go first.
const NOTICES: usize = 100;

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
    /// The writing (the band comes in 0.3.0).
    Write,
    Settings,
    /// The key list, over the writing.
    Help,
}

impl Screen {
    /// The name `--keys` reports.
    fn name(self) -> &'static str {
        match self {
            Screen::Write => "write",
            Screen::Settings => "settings",
            Screen::Help => "help",
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
    tracker: Option<Tracker>,
    /// The `--keys` script, and what it asked of the frame loop.
    script: Option<Runner>,
    todo: Vec<Do>,
}

/// What the palette can do.
#[derive(Clone, Copy)]
enum Command {
    Settings,
    Help,
    ReportBug,
    ResetScale,
    Quit,
}

impl App {
    /// fude on `prefs`, on the writing. The keymap's problems are notices.
    fn with(prefs: Prefs) -> Self {
        let lang_asked = prefs.common.language.clone();
        let lang = i18n::resolve(lang_asked.as_deref());
        let (table, errors) = i18n::table(&lang);
        let (keymap, warnings) = Keymap::load(prefs.keymap.as_deref());
        let mut app = Self {
            table,
            lang,
            lang_asked,
            prefs,
            keymap,
            screen: Screen::Write,
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
            script: None,
            todo: Vec::new(),
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
        format!(
            "screen: {}\nlang: {}\nscale: {scale}%\npalette: {}\nnotices: {}\nsettings_page: {}\n",
            self.screen.name(),
            self.lang,
            self.palette.is_some(),
            self.notices.len(),
            self.prefs.state.page,
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
        let now: Vec<String> = self.prefs.common_warn.iter().chain(&self.prefs.theme_warn).chain(&self.prefs.theme_err).cloned().collect();
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
        let toggle = |now: Screen, to: Screen| if now == to { Screen::Write } else { to };
        match act {
            Act::Settings => self.screen = toggle(self.screen, Screen::Settings),
            Act::Help => {
                self.screen = toggle(self.screen, Screen::Help);
                self.help_scroll = 0;
            }
            Act::Quit => self.quit = true,
            Act::Noop => {}
        }
    }

    /// On the key list: Esc closes it, and the arrows, the pages, Home and
    /// End scroll it.
    fn help_keys(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Help || self.palette.is_some() || self.bell_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.screen = Screen::Write;
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
            ito_palette::Entry::new(tr("palette.settings"), hint(Act::Settings), Command::Settings),
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
            Some(ito_palette::Answer::Pick(Command::Settings)) => self.run(Act::Settings),
            Some(ito_palette::Answer::Pick(Command::Help)) => self.run(Act::Help),
            Some(ito_palette::Answer::Pick(Command::ReportBug)) => self.report_bug(ctx),
            Some(ito_palette::Answer::Pick(Command::ResetScale)) => self.scale_to(ctx, ito_common::ScaleTo::Reset),
            Some(ito_palette::Answer::Pick(Command::Quit)) => self.quit = true,
            Some(_) => {}
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
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from);
        let context = ito_bugreport::Context { home: home.as_deref(), ..Default::default() };
        let url = ito_bugreport::url(&spec, &[("context", ito_bugreport::context(&context))]);
        ctx.open_url(egui::OpenUrl::new_tab(url));
    }

    /// Where the writing band goes: for now, the keys that open the rest.
    fn write_screen(&self, ui: &mut egui::Ui) {
        let c = ito_theme::colors();
        let none = || "—".to_owned();
        let settings = self.keymap.hint(Act::Settings).unwrap_or_else(none);
        let help = self.keymap.hint(Act::Help).unwrap_or_else(none);
        let t = &self.table;
        let text = t.trf("write.placeholder", &[&settings, &help]);
        let r = ui.max_rect();
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(15.0), c.dim);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(t) = &mut self.tracker {
            t.follow(&ctx);
        }
        self.script(&ctx);
        self.follow_prefs(&ctx);
        // While the Keys page takes a key, every key is the key.
        if !self.prefs.keys_edit.busy() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F12)) {
                self.report_bug(&ctx);
            }
            self.rescale(&ctx);
            self.keymap_keys(&ctx);
            self.help_keys(&ctx);
        }
        egui::Panel::top("titlebar")
            .exact_size(ito_titlebar::HEIGHT)
            .frame(egui::Frame::NONE.fill(ito_theme::colors().bg))
            .show(ui, |ui| self.title_bar(ui));
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(ito_theme::colors().bg)).show(ui, |ui| match self.screen {
            Screen::Settings => settings::show(ui, self),
            Screen::Write => self.write_screen(ui),
            Screen::Help => {
                self.write_screen(ui);
                let r = ui.max_rect();
                help::show(ui, self, r);
            }
        });
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

    /// The window's place is written as it closes, and a window that closes
    /// while a script runs says so in the done file.
    fn on_exit(&mut self) {
        if let Some(t) = &mut self.tracker {
            t.close(None);
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
    let viewport = egui::ViewportBuilder::default()
        .with_title("fude")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([720.0, 420.0])
        .with_icon(std::sync::Arc::new(egui::IconData { rgba: icon::pixels(ICON_PX), width: ICON_PX, height: ICON_PX }));
    // The bar is the app's own (the Mac keeps its traffic lights over it).
    let viewport = ito_titlebar::frameless(viewport);
    let options = eframe::NativeOptions { viewport: saved.viewport(viewport), ..Default::default() };
    eframe::run_native(
        "fude",
        options,
        Box::new(move |cc| {
            saved.put(cc);
            let mut prefs = Prefs::load(ito_common::base_dir());
            fonts::load_in_background(cc.egui_ctx.clone());
            prefs.watch(&cc.egui_ctx);
            prefs.apply_theme(&cc.egui_ctx);
            // The scale is common.toml's, so egui's own Ctrl+= / Ctrl+- stay out of it.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            cc.egui_ctx.set_zoom_factor(prefs.common.scale_or_one());
            let mut app = App::with(prefs);
            app.tracker = saved.tracker();
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

    /// One frame of `a` with `events`, as eframe would run it.
    fn frame(ctx: &egui::Context, a: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0))),
            events,
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
}
