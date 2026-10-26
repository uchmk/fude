//! The settings screen (Ctrl+,): `ito-prefs` draws the nav and the cards,
//! and the rows common to every uchmk app (the language, the scale, the
//! theme) write common.toml. fude's own keys are on the Keys page, written
//! to keymap.toml. The writing's own settings (the limits, the time, the
//! paper) come with the band and the clean copy.

use egui::{vec2, FontId, Sense, Stroke, StrokeKind, Ui};
use ito_common::{themes_dir, CommonChange};
use ito_i18n::Table;
use ito_prefs::{section, KeyWords, MoreWords, Page, Words};

use crate::keymap::Keymap;
use crate::{App, Screen};

pub const GENERAL: usize = 0;
pub const THEME: usize = 1;
pub const KEYS: usize = 2;
pub const ADVANCED: usize = 3;
pub const ABOUT: usize = 4;

/// The pages fude has besides General and About, which every app has.
const HAS: [Page; 3] = [Page::Theme, Page::Keys, Page::Advanced];

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
    ]
    .into_iter()
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
    let pages: Vec<_> = ito_prefs::default_order(&HAS).into_iter().map(|p| p.words(more)).collect();
    let t = &app.table;
    let tr = |k: &'static str| t.tr(k);
    let index = index(w, more, kw, t);
    let nav = ito_prefs::Nav { pages: &pages, index: &index, words: w, file: "common.toml" };
    let layers = app.keymap.layers();
    let state_dir = ito_common::state_dir("fude");
    let mut asked: Vec<CommonChange> = Vec::new();
    let mut key_edits = Vec::new();
    let mut advanced = Vec::new();
    let mut bug = false;
    let prefs = &mut app.prefs;
    let keys_edit = &mut prefs.keys_edit;
    let (common, themes, base) = (&prefs.common, &prefs.themes, prefs.base.as_deref());
    let out = ito_prefs::show(ui, ito_theme::colors(), &mut prefs.state, &nav, keys, |ui, page, l| match page {
        GENERAL => section(ui, l, tr("settings.display"), |ui| {
            let now = common.language.as_deref().unwrap_or("auto");
            asked.extend(ito_prefs::language_row(ui, l, w, now, ""));
            ito_prefs::sep(ui, l);
            asked.extend(ito_prefs::scale_row(ui, l, w, common.scale_or_one(), ""));
        }),
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
        app.screen = Screen::Write;
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
    use egui::{pos2, Rect};

    #[test]
    fn pages_follow_ito_default_order() {
        let ours = [GENERAL, THEME, KEYS, ADVANCED, ABOUT];
        assert!(ours.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(ito_prefs::default_order(&HAS), [Page::General, Page::Theme, Page::Keys, Page::Advanced, Page::About]);
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
        }
    }
}
