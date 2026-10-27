//! The status bar along the window's foot (`ito-statusbar`): the screen,
//! the letters and the time against their limits, the paper the clean copy
//! opens on, the keys of the screen, the settings and, at the right end, the
//! clock as common.toml sets it, on every screen. It fades with the title
//! bar while letters come.

use std::time::Instant;

use ito_statusbar::{divider, dot, item, link};

use crate::config::{Dir, Paper};
use crate::keymap::Act;
use crate::{write, App, Screen};

/// The bar in `ui` (the bottom panel), at `opacity`.
pub fn show(ui: &mut egui::Ui, app: &mut App, opacity: f32) {
    let c = ito_theme::colors();
    let t = &app.table;
    let clock = app.prefs.common.clock_or(None);
    let spec = ito_statusbar::Spec { clock: &clock, lang: &app.lang, opacity };
    let config = app.prefs.config;
    let (mut to_clean, mut to_settings) = (false, false);
    let left = |ui: &mut egui::Ui| {
        let (name, keys) = match app.screen {
            Screen::Write => {
                let back = if config.dir == Dir::Vertical { "↑↓" } else { "←→" };
                ("status.write", t.trf("status.keys_write", &[back]))
            }
            Screen::Clean => ("status.clean", t.tr("status.keys_clean").to_owned()),
            Screen::Shelf => ("status.shelf", t.tr("status.keys_close").to_owned()),
            Screen::Help => ("status.help", t.tr("status.keys_close").to_owned()),
            Screen::Settings => ("status.settings_screen", t.tr("status.keys_settings").to_owned()),
        };
        dot(ui, c.blue);
        item(ui, t.tr(name), c.fg);
        let b = &app.band;
        let counts = write::counts(b, config.show, Instant::now(), t);
        if !counts.is_empty() {
            divider(ui);
            let last = counts.len() - 1;
            for (i, (words, warn)) in counts.into_iter().enumerate() {
                let stopped = app.screen == Screen::Write && i == last && b.started() && !b.running();
                let words = if stopped { words + t.tr("status.stopped") } else { words };
                item(ui, &words, if warn { c.wait } else { c.dim });
            }
        }
        // The paper the clean copy is on, or opens on: on the clean copy (or
        // the key list over it) it only says so, elsewhere it opens it.
        divider(ui);
        let shown = app.screen == Screen::Clean || (app.screen == Screen::Help && app.back == Screen::Clean);
        let sheet = app.clean.sheet_of(&config, &b.text(), shown);
        let paper = match sheet.paper {
            Paper::Ruled => "clean.ruled",
            Paper::Graph => "clean.graph",
            Paper::Plain => "clean.plain",
            Paper::Genko | Paper::Auto => "clean.genko",
        };
        let dir = t.tr(if sheet.dir == Dir::Vertical { "clean.vertical" } else { "clean.horizontal" });
        let size = match (sheet.paper, sheet.wrap) {
            (Paper::Genko, _) => format!("{}×{}", sheet.cols, sheet.rows),
            (_, 0) => t.tr("clean.auto").to_owned(),
            (_, w) => t.trf("clean.to_chars", &[&w.to_string()]),
        };
        let form = t.trf("status.form", &[t.tr(paper), dir, &size]);
        if shown {
            item(ui, &form, c.dim);
        } else {
            let r = ui.scope(|ui| link(ui, &form));
            to_clean = r.inner;
            r.response.on_hover_text(t.tr("status.form_hover"));
        }
        divider(ui);
        let mut keys = ito_keys::human_text(&keys).into_owned();
        if app.screen != Screen::Help {
            if let Some(k) = app.keymap.hint(Act::Help) {
                keys = format!("{keys} · {}", t.trf("status.help_key", &[&k]));
            }
        }
        item(ui, &keys, c.dim);
    };
    // On the settings the link would only close them, which Esc says.
    let on_settings = app.screen == Screen::Settings;
    let right = |ui: &mut egui::Ui| {
        if on_settings {
            return;
        }
        let r = ui.scope(|ui| link(ui, t.tr("status.settings")));
        to_settings = r.inner;
        if let Some(k) = app.keymap.hint(Act::Settings) {
            r.response.on_hover_text(k);
        }
    };
    ito_statusbar::show(ui, &spec, left, right);
    if to_clean {
        app.open_clean();
    }
    if to_settings {
        app.run(Act::Settings);
    }
}
