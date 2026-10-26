//! The settings screen (Ctrl+,): `ito-prefs` draws the nav and the cards,
//! and the rows common to every uchmk app (the language, the scale, the
//! theme) write common.toml. The Writing page is fude's own, written to
//! config.toml: the clock and the band, the same in every notebook and in
//! force at once, and the clean copy's letters and pace. The This notebook
//! page is the open notebook's, written to the fude.toml in its folder: the
//! limits, no erasing, the paper its clean copy opens on, the grid, how a
//! line of it is filled, and the topics. fude's keys are
//! on the Keys page, written to keymap.toml.

use egui::{vec2, FontId, Sense, Stroke, StrokeKind, Ui};
use ito_common::{themes_dir, CommonChange};
use ito_i18n::Table;
use ito_prefs::{keyed_row, section, sep, Drafts, KeyWords, Look, MoreWords, Page, Words};

use crate::config::{self, Config, Dir, Face, Hang, OnContinue, Paper, Point, Rule, Show, TimeStart, Topic};
use crate::keymap::Keymap;
use crate::App;

pub const GENERAL: usize = 0;
pub const THEME: usize = 1;
pub const WRITE: usize = 2;
pub const NOTE: usize = 3;
pub const KEYS: usize = 4;
pub const ADVANCED: usize = 5;
pub const ABOUT: usize = 6;

/// The pages fude has besides General and About, which every app has.
/// `App` is the Writing page and the This notebook page.
const HAS: [Page; 4] = [Page::Theme, Page::App, Page::Keys, Page::Advanced];

/// The Writing page's keys in config.toml, which the search finds too.
const WRITE_KEYS: [&str; 8] =
    ["write.time_start", "write.on_continue", "write.show", "write.point", "write.dir", "write.font_size", "write.face", "write.pace"];

/// The This notebook page's keys in fude.toml.
const NOTE_KEYS: [&str; 14] = [
    "note.char_limit",
    "note.time_limit",
    "note.no_erase",
    "note.paper",
    "note.dir",
    "note.rule",
    "note.cols",
    "note.rows",
    "note.indent",
    "note.hang",
    "note.merge",
    "note.wrap",
    "note.topic",
    "note.topics",
];

/// The field a topic is added in.
const TOPIC_FIELD: &str = "note.topics";

/// What a page of fude's asks for: `table.key` in config.toml, and its value.
type ConfigChange = (&'static str, toml::Value);

/// Stands for "your own" among a limit's presets.
const OWN: u32 = u32::MAX;

/// The rows on each page, for the search.
fn index<'a>(w: &'static Words, more: &'static MoreWords, kw: &'static KeyWords, t: &'a Table) -> Vec<(usize, &'a str)> {
    let tr = |k: &'static str| t.tr(k);
    vec![
        (GENERAL, w.language),
        (GENERAL, w.scale),
        (GENERAL, "language"),
        (GENERAL, "scale"),
        (THEME, w.mode),
        (THEME, w.themes),
        (THEME, w.preview),
        (THEME, "theme"),
        (THEME, "dark_theme"),
        (THEME, "light_theme"),
        (KEYS, "keymap.toml"),
        (KEYS, "prepend_keymap"),
        (KEYS, tr("keys.app")),
        (WRITE, "config.toml"),
        (WRITE, tr("settings.time")),
        (WRITE, tr("settings.time_start")),
        (WRITE, tr("settings.on_continue")),
        (WRITE, tr("settings.screen")),
        (WRITE, tr("settings.show")),
        (WRITE, tr("settings.point")),
        (WRITE, tr("settings.dir")),
        (WRITE, tr("settings.font_size")),
        (WRITE, tr("settings.copy")),
        (WRITE, tr("settings.face")),
        (WRITE, tr("settings.pace")),
        (NOTE, "fude.toml"),
        (NOTE, tr("settings.limits")),
        (NOTE, tr("settings.char_limit")),
        (NOTE, tr("settings.time_limit")),
        (NOTE, tr("settings.no_erase")),
        (NOTE, tr("settings.opened")),
        (NOTE, tr("settings.paper")),
        (NOTE, tr("settings.paper_dir")),
        (NOTE, tr("settings.rule")),
        (NOTE, tr("settings.genko")),
        (NOTE, tr("settings.grid")),
        (NOTE, tr("settings.indent")),
        (NOTE, tr("settings.hang")),
        (NOTE, tr("settings.merge")),
        (NOTE, tr("settings.sheet")),
        (NOTE, tr("settings.wrap")),
        (NOTE, tr("settings.topic_section")),
        (NOTE, tr("settings.topic")),
        (NOTE, tr("settings.topics")),
    ]
    .into_iter()
    .chain(WRITE_KEYS.map(|k| (WRITE, k)))
    .chain(NOTE_KEYS.map(|k| (NOTE, k)))
    .chain(ito_prefs::keys_index(kw).into_iter().map(|t| (KEYS, t)))
    .chain(ito_prefs::advanced_index(more).into_iter().map(|t| (ADVANCED, t)))
    .chain(ito_prefs::about_index(more, w).into_iter().map(|t| (ABOUT, t)))
    .collect()
}

pub fn show(ui: &mut Ui, app: &mut App) {
    let w = Words::of(&app.lang);
    let more = MoreWords::of(&app.lang);
    let kw = KeyWords::of(&app.lang);
    // While a key is being set the page takes every key, Esc included; while
    // the palette or the notices are open, the keys are theirs.
    let others = app.prefs.keys_edit.busy() || app.palette.is_some() || app.bell_open;
    let keys = if others { ito_prefs::Keys::default() } else { ito_prefs::Keys::standard(ui, false) };
    let t = &app.table;
    let tr = |k: &'static str| t.tr(k);
    // The notebook's page is led by its name.
    let name = if app.note.is_empty() { tr("note.default") } else { app.note.as_str() };
    let note_lead = t.trf("settings.clean_lead", &[name]);
    let pages: Vec<_> = ito_prefs::default_order(&HAS)
        .into_iter()
        .flat_map(|p| match p {
            Page::App => vec![(tr("settings.write"), tr("settings.write_lead")), (tr("settings.clean"), note_lead.as_str())],
            _ => vec![p.words(more)],
        })
        .collect();
    let index = index(w, more, kw, t);
    let nav = ito_prefs::Nav { pages: &pages, index: &index, words: w, file: "common.toml" };
    let layers = app.keymap.layers();
    let state_dir = ito_common::state_dir("fude");
    let mut asked: Vec<CommonChange> = Vec::new();
    let mut key_edits = Vec::new();
    let mut advanced = Vec::new();
    let mut bug = false;
    let mut config_asked = Vec::new();
    let topics = app.prefs.topics();
    let prefs = &mut app.prefs;
    let keys_edit = &mut prefs.keys_edit;
    let (config, drafts) = (prefs.config, &mut prefs.drafts);
    let (common, themes, base) = (&prefs.common, &prefs.themes, prefs.base.as_deref());
    let out = ito_prefs::show(ui, ito_theme::colors(), &mut prefs.state, &nav, keys, |ui, page, l| match page {
        GENERAL => section(ui, l, tr("settings.display"), |ui| {
            let now = common.language.as_deref().unwrap_or("auto");
            asked.extend(ito_prefs::language_row(ui, l, w, now, ""));
            ito_prefs::sep(ui, l);
            asked.extend(ito_prefs::scale_row(ui, l, w, common.scale_or_one(), ""));
        }),
        WRITE => config_asked = write_page(ui, l, t, config),
        NOTE => config_asked = note_page(ui, l, t, config, drafts, &topics),
        THEME => {
            let (choice, dark, light) = common.theme_choice(None);
            let note = match base {
                Some(b) => format!("{} {}", tr("settings.theme_note"), themes_dir(b).display()),
                None => tr("settings.no_folder").into(),
            };
            asked.extend(ito_prefs::theme_page(ui, l, w, themes, (&choice, &dark, &light), &note, |ui| {
                preview(ui, tr("settings.preview_text"))
            }));
        }
        KEYS => {
            let layer_name = |_: &str| tr("keys.app").to_owned();
            let desc = |d: &str| t.tr_desc(d).to_owned();
            // Keys fude reads before the keymap: bound, they never run.
            let extra = |_: &str, on: &[ito_keys::Key]| -> Vec<String> {
                Keymap::fixed(on).map(|k| tr("keys.fixed").replace("{}", &k.human())).into_iter().collect()
            };
            let table = ito_prefs::KeyTable {
                now: &layers,
                built_in: Keymap::built_in_layers(),
                noop: crate::keymap::NOOP,
                own: "keymap.toml",
                layer_name: &layer_name,
                tr: &desc,
                extra: &extra,
                unbound: &[],
                sup: false,
            };
            key_edits.extend(ito_prefs::keys_page(ui, l, kw, &table, keys_edit).unwrap_or_default());
        }
        ADVANCED => advanced.extend(ito_prefs::advanced_card(ui, l, more, state_dir.is_some())),
        ABOUT => {
            let system = ito_bugreport::os_line();
            let license = Some(env!("CARGO_PKG_LICENSE")).filter(|x| !x.is_empty());
            let about =
                ito_prefs::About { version: env!("CARGO_PKG_VERSION"), commit: option_env!("FUDE_COMMIT"), system: &system, license };
            bug |= !ito_prefs::about_page(ui, l, more, w, "fude", &about).is_empty();
        }
        _ => {}
    });
    let no_folder = tr("settings.no_folder").to_owned();
    if bug {
        app.report_bug(ui.ctx());
    }
    for a in advanced {
        match a {
            ito_prefs::Advanced::OpenConfigDir => match app.prefs.base.clone() {
                Some(b) => app.prefs.open_dir(b),
                None => app.say(ito_notices::Kind::Warn, &no_folder),
            },
            ito_prefs::Advanced::OpenStateDir => {
                if let Some(d) = state_dir.clone() {
                    app.prefs.open_dir(d);
                }
            }
            ito_prefs::Advanced::Reset => app.prefs.reset(&no_folder),
        }
    }
    if out.close {
        app.screen = app.back;
    }
    if out.open_file {
        app.prefs.open_common(&no_folder);
    }
    // Gone to another page: the keys stop being taken for this one.
    if app.prefs.state.page != KEYS {
        app.prefs.keys_edit.capture = None;
        app.prefs.keys_edit.listening = false;
    }
    app.prefs.keymap_edit(key_edits, &no_folder);
    app.prefs.common_change(asked, &no_folder);
    let retopic = config_asked.iter().any(|(k, _)| k.starts_with("note.topic"));
    for (k, v) in config_asked {
        let (table, key) = k.split_once('.').expect("a key is table.key");
        app.prefs.config_change(table, key, v, &no_folder);
    }
    if retopic {
        app.retopic();
    }
}

/// The Writing page: the clock, the band and the clean copy's letters and
/// pace, the same in every notebook. What it asks for goes into config.toml.
fn write_page(ui: &mut Ui, l: Look, t: &Table, c: Config) -> Vec<ConfigChange> {
    let tr = |k: &'static str| t.tr(k);
    let word = |w: &str| toml::Value::String(w.into());
    let mut asked: Vec<ConfigChange> = Vec::new();
    section(ui, l, tr("settings.time"), |ui| {
        let starts = [(TimeStart::First, tr("settings.time_start_first")), (TimeStart::Open, tr("settings.time_start_open"))];
        let words = (tr("settings.time_start"), tr("settings.time_start_note"));
        if let Some(v) = pick(ui, l, "write.time_start", words, c.time_start, &starts) {
            asked.push(("write.time_start", word(v.word())));
        }
        sep(ui, l);
        let goes = [(OnContinue::Add, tr("settings.on_continue_add")), (OnContinue::Free, tr("settings.on_continue_free"))];
        let words = (tr("settings.on_continue"), tr("settings.on_continue_note"));
        if let Some(v) = pick(ui, l, "write.on_continue", words, c.on_continue, &goes) {
            asked.push(("write.on_continue", word(v.word())));
        }
    });
    section(ui, l, tr("settings.screen"), |ui| {
        let shows = [
            (Show::Both, tr("settings.show_both")),
            (Show::Chars, tr("settings.show_chars")),
            (Show::Time, tr("settings.show_time")),
            (Show::Nothing, tr("settings.show_none")),
        ];
        if let Some(v) = pick(ui, l, "write.show", (tr("settings.show"), tr("settings.show_note")), c.show, &shows) {
            asked.push(("write.show", word(v.word())));
        }
        sep(ui, l);
        let points = [
            (Point::Center, tr("settings.point_center")),
            (Point::Start, tr("settings.point_start")),
            (Point::End, tr("settings.point_end")),
        ];
        if let Some(v) = pick(ui, l, "write.point", (tr("settings.point"), tr("settings.point_note")), c.point, &points) {
            asked.push(("write.point", word(v.word())));
        }
        sep(ui, l);
        let dirs = [(Dir::Horizontal, tr("settings.dir_horizontal")), (Dir::Vertical, tr("settings.dir_vertical"))];
        if let Some(v) = pick(ui, l, "write.dir", (tr("settings.dir"), tr("settings.dir_note")), c.dir, &dirs) {
            asked.push(("write.dir", word(v.word())));
        }
        sep(ui, l);
        // A size from the file that is not in the list is put in it.
        let mut sizes = vec![24, 32, 40, 48, 56, 64];
        if !sizes.contains(&c.font_size) {
            sizes.push(c.font_size);
            sizes.sort_unstable();
        }
        let sizes: Vec<(u32, String)> = sizes.into_iter().map(|n| (n, n.to_string())).collect();
        let sizes: Vec<(u32, &str)> = sizes.iter().map(|(n, w)| (*n, w.as_str())).collect();
        let words = (tr("settings.font_size"), tr("settings.font_size_note"));
        if let Some(n) = pick(ui, l, "write.font_size", words, c.font_size, &sizes) {
            asked.push(("write.font_size", n.into()));
        }
    });
    section(ui, l, tr("settings.copy"), |ui| {
        let faces = [(Face::Mincho, tr("settings.face_mincho")), (Face::Gothic, tr("settings.face_gothic"))];
        if let Some(v) = pick(ui, l, "write.face", (tr("settings.face"), tr("settings.face_note")), c.face, &faces) {
            asked.push(("write.face", word(v.word())));
        }
        sep(ui, l);
        if keyed_row(ui, l, "write.pace", tr("settings.pace"), tr("settings.pace_note"), |ui| ito_prefs::switch(ui, l, c.pace)) {
            asked.push(("write.pace", (!c.pace).into()));
        }
    });
    asked
}

/// The This notebook page: the limits, no erasing, the paper the clean
/// copy opens on, the grid, the sheets that are not a grid, and the topics
/// (`topics` is the notebook's list). What it asks for goes into the
/// notebook's fude.toml (config.toml when none is open; the list then stays
/// in memory).
fn note_page(ui: &mut Ui, l: Look, t: &Table, c: Config, drafts: &mut Drafts, topics: &[String]) -> Vec<ConfigChange> {
    let tr = |k: &'static str| t.tr(k);
    let trf = |k: &'static str, n: u32| t.trf(k, &[&n.to_string()]);
    let word = |w: &str| toml::Value::String(w.into());
    let mut asked: Vec<ConfigChange> = Vec::new();
    section(ui, l, tr("settings.limits"), |ui| {
        let chars = [0, 100, 200, 400, 800, 1200].map(|n| {
            let w = match n {
                0 => tr("settings.none").to_owned(),
                400 => trf("settings.chars_sheet", n),
                _ => trf("settings.chars_n", n),
            };
            (n, w)
        });
        let words = [tr("settings.char_limit"), tr("settings.char_limit_note"), tr("settings.unit_chars"), tr("settings.custom")];
        if let Some(n) = limit_row(ui, l, drafts, "note.char_limit", words, c.char_limit, config::CHAR_LIMIT, &chars) {
            asked.push(("note.char_limit", n.into()));
        }
        sep(ui, l);
        let minutes =
            [0, 1, 3, 5, 10, 15, 30].map(|n| (n, if n == 0 { tr("settings.none").to_owned() } else { trf("settings.minutes_n", n) }));
        let words = [tr("settings.time_limit"), tr("settings.time_limit_note"), tr("settings.unit_minutes"), tr("settings.custom")];
        if let Some(n) = limit_row(ui, l, drafts, "note.time_limit", words, c.time_limit, config::TIME_LIMIT, &minutes) {
            asked.push(("note.time_limit", n.into()));
        }
        sep(ui, l);
        let flip = keyed_row(ui, l, "note.no_erase", tr("settings.no_erase"), tr("settings.no_erase_note"), |ui| {
            ito_prefs::switch(ui, l, c.no_erase)
        });
        if flip {
            asked.push(("note.no_erase", (!c.no_erase).into()));
        }
    });
    section(ui, l, tr("settings.opened"), |ui| {
        let papers = [
            (Paper::Auto, tr("settings.paper_auto")),
            (Paper::Genko, tr("settings.paper_genko")),
            (Paper::Ruled, tr("settings.paper_ruled")),
            (Paper::Graph, tr("settings.paper_graph")),
            (Paper::Plain, tr("settings.paper_plain")),
        ];
        if let Some(v) = pick(ui, l, "note.paper", (tr("settings.paper"), tr("settings.paper_note")), c.paper, &papers) {
            asked.push(("note.paper", word(v.word())));
        }
        sep(ui, l);
        let dirs = [(Dir::Vertical, tr("settings.paper_vertical")), (Dir::Horizontal, tr("settings.paper_horizontal"))];
        if let Some(v) = pick(ui, l, "note.dir", (tr("settings.paper_dir"), tr("settings.paper_dir_note")), c.paper_dir, &dirs) {
            asked.push(("note.dir", word(v.word())));
        }
        sep(ui, l);
        let rules = [
            (Rule::Shu, tr("settings.rule_shu")),
            (Rule::Midori, tr("settings.rule_midori")),
            (Rule::Ao, tr("settings.rule_ao")),
            (Rule::Sumi, tr("settings.rule_sumi")),
        ];
        if let Some(v) = pick(ui, l, "note.rule", (tr("settings.rule"), tr("settings.rule_note")), c.rule, &rules) {
            asked.push(("note.rule", word(v.word())));
        }
    });
    section(ui, l, tr("settings.genko"), |ui| {
        // A grid from the file that is not in the list is put in it.
        let mut grids = vec![(20, 20), (20, 10)];
        if !grids.contains(&(c.cols, c.rows)) {
            grids.push((c.cols, c.rows));
        }
        let grids: Vec<((u32, u32), String)> = grids
            .into_iter()
            .map(|(x, y)| ((x, y), t.trf("settings.grid_n", &[&x.to_string(), &y.to_string(), &(x * y).to_string()])))
            .collect();
        let grids: Vec<((u32, u32), &str)> = grids.iter().map(|(g, w)| (*g, w.as_str())).collect();
        if let Some((x, y)) = pick(ui, l, "note.cols", (tr("settings.grid"), tr("settings.grid_note")), (c.cols, c.rows), &grids) {
            asked.push(("note.cols", x.into()));
            asked.push(("note.rows", y.into()));
        }
        sep(ui, l);
        if keyed_row(ui, l, "note.indent", tr("settings.indent"), tr("settings.indent_note"), |ui| ito_prefs::switch(ui, l, c.indent)) {
            asked.push(("note.indent", (!c.indent).into()));
        }
        sep(ui, l);
        let hangs = [(Hang::In, tr("settings.hang_in")), (Hang::Out, tr("settings.hang_out")), (Hang::Off, tr("settings.hang_off"))];
        if let Some(v) = pick(ui, l, "note.hang", (tr("settings.hang"), tr("settings.hang_note")), c.hang, &hangs) {
            asked.push(("note.hang", word(v.word())));
        }
        sep(ui, l);
        if keyed_row(ui, l, "note.merge", tr("settings.merge"), tr("settings.merge_note"), |ui| ito_prefs::switch(ui, l, c.merge)) {
            asked.push(("note.merge", (!c.merge).into()));
        }
    });
    section(ui, l, tr("settings.sheet"), |ui| {
        let mut wraps = vec![0, 20, 30, 40];
        if !wraps.contains(&c.wrap) {
            wraps.push(c.wrap);
            wraps.sort_unstable();
        }
        let wraps: Vec<(u32, String)> = wraps
            .into_iter()
            .map(|n| (n, if n == 0 { tr("settings.wrap_auto").to_owned() } else { t.trf("settings.chars_n", &[&n.to_string()]) }))
            .collect();
        let wraps: Vec<(u32, &str)> = wraps.iter().map(|(n, w)| (*n, w.as_str())).collect();
        if let Some(n) = pick(ui, l, "note.wrap", (tr("settings.wrap"), tr("settings.wrap_note")), c.wrap, &wraps) {
            asked.push(("note.wrap", n.into()));
        }
    });
    section(ui, l, tr("settings.topic_section"), |ui| {
        let ways = [
            (Topic::Order, tr("settings.topic_order")),
            (Topic::Random, tr("settings.topic_random")),
            (Topic::Off, tr("settings.topic_none")),
        ];
        if let Some(v) = pick(ui, l, "note.topic", (tr("settings.topic"), tr("settings.topic_note")), c.topic, &ways) {
            asked.push(("note.topic", word(v.word())));
        }
        sep(ui, l);
        let added = keyed_row(ui, l, "note.topics", tr("settings.topics"), tr("settings.topics_note"), |ui| {
            ito_prefs::field(ui, drafts, TOPIC_FIELD, "", tr("settings.topic_add"), 220.0)
        });
        // The field empties for the next one.
        if let Some(new) = added.map(|w| w.trim().to_owned()).filter(|w| !w.is_empty()) {
            drafts.remove(TOPIC_FIELD);
            if !topics.contains(&new) {
                let list: Vec<String> = topics.iter().cloned().chain([new]).collect();
                asked.push(("note.topics", config::topics_value(&list)));
            }
        }
        if topics.is_empty() {
            sep(ui, l);
            ito_prefs::status(ui, tr("settings.topics_empty"), l.c.faint());
        }
        for (i, topic) in topics.iter().enumerate() {
            sep(ui, l);
            if ito_prefs::row(ui, l, topic, "", |ui| ito_prefs::button(ui, l, tr("settings.topic_remove"))) {
                let list: Vec<String> = topics.iter().enumerate().filter(|&(k, _)| k != i).map(|(_, w)| w.clone()).collect();
                asked.push(("note.topics", config::topics_value(&list)));
            }
        }
    });
    asked
}

/// A row with a list to pick one of; `key` is its key in config.toml.
fn pick<T: PartialEq + Copy>(
    ui: &mut Ui,
    l: Look,
    key: &'static str,
    (label, note): (&str, &str),
    now: T,
    options: &[(T, &str)],
) -> Option<T> {
    keyed_row(ui, l, key, label, note, |ui| ito_prefs::select(ui, key, now, options))
}

/// A limit's row: its presets and "your own", which brings a field for the
/// number (a number not among the presets shows it too). `words` are the
/// label, the note, the unit and "your own"; a number typed is taken at the
/// nearest end of `range`.
#[allow(clippy::too_many_arguments)]
fn limit_row(
    ui: &mut Ui,
    l: Look,
    drafts: &mut Drafts,
    key: &'static str,
    [label, note, unit, own_word]: [&str; 4],
    now: u32,
    (lo, hi): (u32, u32),
    presets: &[(u32, String)],
) -> Option<u32> {
    let own_id = egui::Id::new(("fude-own-limit", key));
    let own = ui.data(|d| d.get_temp::<bool>(own_id)).unwrap_or(false) || !presets.iter().any(|(n, _)| *n == now);
    let mut options: Vec<(u32, &str)> = presets.iter().map(|(n, w)| (*n, w.as_str())).collect();
    options.push((OWN, own_word));
    keyed_row(ui, l, key, label, note, |ui| {
        let mut out = None;
        if own {
            ito_prefs::status(ui, unit, l.c.dim);
            if let Some(typed) = ito_prefs::field(ui, drafts, key, &now.to_string(), "", 72.0) {
                out = typed.trim().parse::<u64>().ok().map(|n| n.clamp(lo.into(), hi.into()) as u32);
            }
        }
        match ito_prefs::select(ui, key, if own { OWN } else { now }, &options) {
            Some(OWN) => {
                ui.data_mut(|d| d.insert_temp(own_id, true));
                ui.memory_mut(|m| m.request_focus(ito_prefs::field_id(key)));
            }
            Some(n) => {
                ui.data_mut(|d| d.remove_temp::<bool>(own_id));
                out = Some(n);
            }
            None => {}
        }
        out.filter(|n| *n != now)
    })
}

/// fude in the theme in force: a line of text on a rule, with the caret at
/// its end, as the writing band shows it.
fn preview(ui: &mut Ui, sample: &str) {
    let c = ito_theme::colors();
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(420.0), 96.0), Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 8.0, c.bg);
    p.rect_stroke(r, 8.0, Stroke::new(1.0, c.border), StrokeKind::Inside);
    let inner = r.shrink(12.0);
    let base = inner.center().y + 12.0;
    p.hline(inner.x_range(), base, Stroke::new(1.0, c.border));
    let g = p.layout_no_wrap(sample.to_owned(), FontId::proportional(20.0), c.fg);
    let at = egui::pos2(inner.left() + 8.0, base - 6.0 - g.size().y);
    let end = at.x + g.size().x.min(inner.width() - 20.0);
    p.galley(at, g, c.fg);
    p.vline(end + 3.0, (base - 32.0)..=(base - 6.0), Stroke::new(2.0, c.done));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Screen;
    use egui::{pos2, Rect};

    #[test]
    fn pages_follow_ito_default_order() {
        assert_eq!([GENERAL, THEME, WRITE, NOTE, KEYS, ADVANCED, ABOUT], [0, 1, 2, 3, 4, 5, 6]);
        let order = [Page::General, Page::Theme, Page::App, Page::Keys, Page::Advanced, Page::About];
        assert_eq!(ito_prefs::default_order(&HAS), order);
    }

    /// The Keys page takes the keys pressed while a key is being set (Esc
    /// and Enter included, which would close the screen), and writes only
    /// the key set to keymap.toml once Enter sets it.
    #[test]
    fn the_keys_page_takes_a_key_and_writes_it_on_enter() {
        let dir = std::env::temp_dir().join(format!("fude-settings-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut a = App::with(crate::prefs::Prefs::empty());
        a.prefs.base = Some(dir.clone());
        a.prefs.state.page = KEYS;
        a.screen = Screen::Settings;
        let ctx = egui::Context::default();
        let mut time = 0.0;
        let mut frame = |a: &mut App, events: Vec<egui::Event>| {
            time += 0.1;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))),
                time: Some(time),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| show(ui, a));
            });
            out.textures_delta.clear();
        };
        let key = |key, modifiers, pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers };
        let mut press = |a: &mut App, k, m| {
            frame(a, vec![key(k, m, true)]);
            frame(a, vec![key(k, m, false)]);
        };
        let start = |a: &mut App| {
            a.prefs.keys_edit.capture =
                Some(ito_prefs::Capture { layer: "app".into(), run: "help".into(), keys: Vec::new(), replace: None });
        };
        let file = crate::prefs::keymap_path(&dir);
        // Esc gives up the key, and the screen stays open.
        start(&mut a);
        press(&mut a, egui::Key::Escape, egui::Modifiers::NONE);
        assert!(a.prefs.keys_edit.capture.is_none(), "Esc gave the key up");
        assert!(a.screen == Screen::Settings, "the screen stays open");
        // Ctrl+Shift+Enter is a key; nothing is written before Enter.
        start(&mut a);
        let cs = egui::Modifiers { ctrl: true, shift: true, command: true, ..Default::default() };
        press(&mut a, egui::Key::Enter, cs);
        assert!(!file.exists(), "a key pressed is not written yet");
        assert_eq!(a.prefs.keys_edit.capture.as_ref().map(|c| c.keys.len()), Some(1), "{:?}", a.prefs.keys_edit.capture);
        press(&mut a, egui::Key::Enter, egui::Modifiers::NONE);
        assert!(a.screen == Screen::Settings, "Enter set the key and did not close the screen");
        let mut text = String::new();
        for _ in 0..500 {
            text = std::fs::read_to_string(&file).unwrap_or_default();
            if text.contains("help") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(text.contains("help"), "keymap.toml was not written within 5 s");
        assert!(text.contains("<C-S-Enter>") && text.contains(r#"run = "help""#) && text.contains("prepend_keymap"), "{text}");
        assert!(!text.contains("settings") && !text.contains("quit"), "only the key set is written: {text}");
        // Taking keys, then gone to another page: the keys stop being taken.
        start(&mut a);
        a.prefs.keys_edit.listening = true;
        a.prefs.state.page = GENERAL;
        press(&mut a, egui::Key::Escape, egui::Modifiers::NONE);
        assert!(!a.prefs.keys_edit.busy(), "another page takes no keys for the Keys page");
        assert!(a.screen == Screen::Settings, "the Esc that gave the key up did not close the screen");
        // Nothing taken: Esc closes the screen again.
        press(&mut a, egui::Key::Escape, egui::Modifiers::NONE);
        assert!(a.screen == Screen::Write, "Esc with nothing taken closes the settings");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Keys page's words are found by the search, in both languages.
    #[test]
    fn the_search_finds_the_keys_page() {
        for lang in ["en", "ja"] {
            let (t, _) = crate::i18n::table(lang);
            let idx = index(Words::of(lang), MoreWords::of(lang), KeyWords::of(lang), &t);
            assert!(idx.iter().any(|(p, s)| *p == KEYS && *s == "keymap.toml"));
            assert!(idx.iter().any(|(p, s)| *p == KEYS && *s == t.tr("keys.app")), "{lang}");
            assert!(idx.iter().any(|(p, s)| *p == ABOUT && !s.is_empty()));
            assert!(idx.iter().any(|(p, s)| *p == WRITE && *s == t.tr("settings.font_size")), "{lang}");
            assert!(idx.iter().any(|(p, s)| *p == NOTE && *s == t.tr("settings.char_limit")), "{lang}");
            assert!(idx.iter().any(|(p, s)| *p == NOTE && *s == t.tr("settings.paper")), "{lang}");
            assert!(idx.iter().any(|(p, s)| *p == NOTE && *s == "note.paper"), "{lang}");
        }
    }

    /// Every key on the Writing and the This notebook pages is one the
    /// files take, so the search finds the rows by what the file calls them.
    #[test]
    fn the_page_keys_are_settings() {
        let mut c = Config::default();
        let value = |k: &str| match k {
            "note.no_erase" | "note.indent" | "note.merge" | "write.pace" => toml::Value::Boolean(true),
            "note.char_limit" | "note.time_limit" | "write.font_size" | "note.cols" | "note.rows" | "note.wrap" => toml::Value::Integer(20),
            _ => toml::Value::String(String::new()),
        };
        for k in WRITE_KEYS.into_iter().chain(NOTE_KEYS) {
            let (table, key) = k.split_once('.').unwrap();
            let r = c.apply(table, key, &value(k));
            assert!(r.as_ref().map_or_else(|e| !e.contains("not a setting"), |_| true), "{k}: {r:?}");
        }
    }

    /// The Writing and the This notebook pages draw in both languages, a
    /// limit not among the presets included; a change is in force at once
    /// and written to config.toml when no notebook is open.
    #[test]
    fn the_writing_page_draws_and_a_change_is_written() {
        for lang in ["en", "ja"] {
            let mut a = App::with(crate::prefs::Prefs::empty());
            (a.table, _) = crate::i18n::table(lang);
            a.prefs.config.char_limit = 333;
            (a.prefs.config.cols, a.prefs.config.rows, a.prefs.config.wrap) = (24, 12, 33);
            // A notebook with no file keeps its topics in memory.
            a.prefs.config_change("note", "topics", config::topics_value(&["朝の音".into(), "窓".into()]), "");
            assert_eq!(a.prefs.topics(), ["朝の音", "窓"]);
            a.screen = Screen::Settings;
            let ctx = egui::Context::default();
            for page in [WRITE, NOTE] {
                a.prefs.state.page = page;
                for _ in 0..2 {
                    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                        egui::CentralPanel::default().show(ui, |ui| show(ui, &mut a));
                    });
                    out.textures_delta.clear();
                }
            }
            assert_eq!(a.prefs.config.char_limit, 333, "drawing changes nothing");
            assert_eq!((a.prefs.config.cols, a.prefs.config.rows, a.prefs.config.wrap), (24, 12, 33), "drawing changes nothing");
        }
        let dir = std::env::temp_dir().join(format!("fude-settings-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut a = App::with(crate::prefs::Prefs::empty());
        a.prefs.base = Some(dir.clone());
        a.prefs.config_change("note", "char_limit", toml::Value::Integer(400), "");
        assert_eq!(a.prefs.config.limits().chars, 400, "in force at once");
        let file = config::path(&dir);
        let mut text = String::new();
        for _ in 0..500 {
            text = std::fs::read_to_string(&file).unwrap_or_default();
            if text.contains("400") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(text.contains("[note]") && text.contains("char_limit = 400"), "{text}");
        a.prefs.config_change("note", "paper", toml::Value::String("graph".into()), "");
        assert_eq!(a.prefs.config.paper, Paper::Graph, "in force at once");
        for _ in 0..500 {
            text = std::fs::read_to_string(&file).unwrap_or_default();
            if text.contains("graph") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(text.contains(r#"paper = "graph""#) && text.contains("char_limit = 400"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
