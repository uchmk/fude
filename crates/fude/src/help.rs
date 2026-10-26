//! The key list (F1 by default): the keymap's own keys as they are now and
//! the keys fude reads before the keymap, drawn by `ito-help` in a card over
//! the writing.

use egui::{pos2, vec2, Align2, CornerRadius, FontId, Rect, Stroke, StrokeKind, Ui};

use crate::keymap::NOOP;
use crate::App;

/// A row's height, and the narrowest a column may be.
const LINE: f32 = 20.0;
const COLUMN: f32 = 360.0;

impl App {
    /// The key list as titled groups, to be dealt into columns.
    pub fn help_groups(&self) -> Vec<ito_help::Group> {
        let t = &self.table;
        let tr = |k: &'static str| t.tr(k).to_owned();
        let rows = ito_keymap::help_lines(&self.keymap.app, NOOP)
            .into_iter()
            .map(|l| {
                // A person's own binding without a description shows its command.
                let desc = if l.desc.is_empty() { l.run.to_owned() } else { t.tr_desc(l.desc).to_owned() };
                (l.keys, desc)
            })
            .collect();
        // Read before the keymap (keymap.rs `FIXED`), so not in it.
        let fixed = vec![
            ("<Esc>".to_owned(), tr("help.esc")),
            ("<C-S-p>".to_owned(), tr("help.palette")),
            ("<C-=> / <C-->".to_owned(), tr("help.zoom")),
            ("<C-0>".to_owned(), tr("help.zoom_reset")),
            ("<F12>".to_owned(), tr("help.bug")),
        ];
        vec![ito_help::Group { title: tr("help.keys"), rows }, ito_help::Group { title: tr("help.fixed"), rows: fixed }]
    }

    /// The arrows, the pages, Home and End scroll the list; whether `key`
    /// was one of them.
    pub fn help_scroll_key(&mut self, key: egui::Key) -> bool {
        let Some(d) = self.help_drawn.get() else { return false };
        let page = d.visible.saturating_sub(1).max(1) as i64;
        self.help_scroll = match key {
            egui::Key::ArrowDown => ito_help::scrolled(d.scroll, 1, d.max),
            egui::Key::ArrowUp => ito_help::scrolled(d.scroll, -1, d.max),
            egui::Key::PageDown => ito_help::scrolled(d.scroll, page, d.max),
            egui::Key::PageUp => ito_help::scrolled(d.scroll, -page, d.max),
            egui::Key::Home => 0,
            egui::Key::End => d.max,
            _ => return false,
        };
        true
    }
}

/// The list in a card in the middle of `r`.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    let w = r.width().min(900.0);
    let cols = ito_help::columns_for(w - 40.0, COLUMN, 2);
    let columns = ito_help::deal(&app.help_groups(), cols);
    let tall = columns.iter().map(Vec::len).max().unwrap_or(0);
    let plan = ito_help::Plan { columns, scale: 1.0, scrolls: false };
    let need = 50.0 + tall as f32 * LINE + 36.0;
    let card = Rect::from_center_size(r.center(), vec2(w, need.min(r.height() - 24.0).max(120.0)));
    p.rect_filled(card, CornerRadius::same(10), c.raised());
    p.rect_stroke(card, CornerRadius::same(10), Stroke::new(1.0, c.border_strong()), StrokeKind::Inside);
    let t = &app.table;
    let tr = |k: &'static str| t.tr(k);
    p.text(pos2(card.left() + 20.0, card.top() + 18.0), Align2::LEFT_TOP, tr("help.title"), FontId::proportional(16.0), c.fg);
    let body = Rect::from_min_max(pos2(card.left() + 20.0, card.top() + 50.0), pos2(card.right() - 20.0, card.bottom() - 36.0));
    // Ctrl+wheel is the scale.
    let wheel = ui.input(|i| if i.modifiers.command { 0.0 } else { i.smooth_scroll_delta.y });
    if wheel != 0.0 {
        if let Some(d) = app.help_drawn.get() {
            app.help_scroll = ito_help::scrolled(d.scroll, (-wheel / LINE).round() as i64, d.max);
        }
    }
    let style = ito_help::Style {
        font: FontId::monospace(12.0),
        row_h: LINE,
        head: c.done,
        keys: c.fg,
        desc: c.dim,
        warn: c.err,
        dim: c.dim,
        bar: c.border_strong(),
    };
    let drawn = ito_help::draw_plan(&p, body, &plan, app.help_scroll, &style);
    app.help_drawn.set(Some(drawn));
    p.text(pos2(card.left() + 20.0, card.bottom() - 22.0), Align2::LEFT_CENTER, tr("help.close"), FontId::proportional(11.5), c.dim);
}

#[cfg(test)]
mod tests {
    use crate::prefs::Prefs;
    use crate::App;

    /// The keymap's keys, translated, then the ones read before it.
    #[test]
    fn the_list_has_the_keymap_and_the_fixed_keys() {
        let mut a = App::with(Prefs::empty());
        let (t, _) = crate::i18n::table("ja");
        a.table = t;
        let groups = a.help_groups();
        assert_eq!(groups.len(), 2);
        assert!(groups[0].rows.contains(&("<C-,>".to_owned(), "設定を開く".to_owned())), "{:?}", groups[0].rows);
        assert!(groups[0].rows.iter().any(|(k, _)| k == "<F1>"));
        assert!(groups[1].rows.iter().any(|(k, _)| k == "<Esc>"));
        assert!(groups[1].rows.iter().all(|(_, d)| !d.is_empty() && !d.starts_with("help.")), "{:?}", groups[1].rows);
    }

    /// The keys scroll only once the list has been drawn, and stay inside it.
    #[test]
    fn the_keys_scroll_the_list() {
        let mut a = App::with(Prefs::empty());
        assert!(!a.help_scroll_key(egui::Key::ArrowDown), "nothing drawn yet");
        a.help_drawn.set(Some(ito_help::Drawn { visible: 5, max: 3, scroll: 0, above: 0, below: 3 }));
        assert!(a.help_scroll_key(egui::Key::PageDown));
        assert_eq!(a.help_scroll, 3);
        assert!(a.help_scroll_key(egui::Key::Home));
        assert_eq!(a.help_scroll, 0);
        assert!(!a.help_scroll_key(egui::Key::A));
    }
}
