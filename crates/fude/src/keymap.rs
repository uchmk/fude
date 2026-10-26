//! The keymap: `defaults/keymap.toml` with the user's
//! `<config>/uchmk/fude/keymap.toml` laid over it by `ito-keymap` (a
//! section per layer, each with `keymap`, `prepend_keymap` and
//! `append_keymap`; the first binding that matches wins). The keys every
//! uchmk app has, and the shelf's; the writing band's own keys are the
//! band's.

use ito_keys::Key;

pub use ito_keymap::resolve;

pub type Binding = ito_keymap::Binding<Act>;
pub type Match<'a> = ito_keymap::Match<'a, Act>;

const DEFAULTS: &str = include_str!("../defaults/keymap.toml");

/// The command that does nothing: a built-in key taken away.
pub const NOOP: &str = "noop";

/// Keys read before the keymap is (`main.rs`): bound in it, they never run.
const FIXED: [&str; 8] = ["<Esc>", "<F12>", "<C-S-p>", "<C-=>", "<C-+>", "<C-;>", "<C-->", "<C-0>"];

/// What a key in the `app` layer does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Open the settings, or close them.
    Settings,
    /// Open the key list, or close it.
    Help,
    /// Open the shelf, or close it.
    Shelf,
    /// The palette on the ways to export the clean copy.
    Export,
    /// The palette on the ways to print it.
    Print,
    Quit,
    /// Does nothing: a key taken away from a built-in operation.
    Noop,
}

/// The command a `run` word names, for the keymap's reader.
pub fn parse(s: &str) -> Result<Act, String> {
    let words: Vec<&str> = s.split_whitespace().collect();
    match words.as_slice() {
        ["settings"] => Ok(Act::Settings),
        ["help"] => Ok(Act::Help),
        ["shelf"] => Ok(Act::Shelf),
        ["export"] => Ok(Act::Export),
        ["print"] => Ok(Act::Print),
        ["quit"] => Ok(Act::Quit),
        [w] if *w == NOOP => Ok(Act::Noop),
        _ => Err(format!("`{s}` is not a command")),
    }
}

pub(crate) const SPEC: ito_keymap::Spec = ito_keymap::Spec { layers: &[ito_keymap::Layer::new("app")], ignored: &[], defaults: DEFAULTS };

#[derive(Default, Debug)]
pub struct Keymap {
    pub app: Vec<Binding>,
}

impl Keymap {
    /// The built-in keymap with the user's file (its text, if there is one)
    /// laid over it. Problems come back as warnings; a bad binding, or one
    /// on a key read before the keymap, is skipped.
    pub fn load(user: Option<&str>) -> (Self, Vec<String>) {
        let mut command = |s: &str, warn: &mut Vec<String>| match parse(s) {
            Ok(a) => Some(a),
            Err(e) => {
                warn.push(format!("keymap: {e}"));
                None
            }
        };
        let files: Vec<(&str, &str)> = user.map(|t| ("keymap.toml", t)).into_iter().collect();
        let mut km = ito_keymap::load(&SPEC, &files, &mut command);
        let mut warnings = std::mem::take(&mut km.warnings);
        let mut app = km.take("app");
        app.retain(|b| match Self::fixed(&b.on) {
            Some(k) => {
                warnings.push(format!("{}: [app] `{}` is fixed and cannot be bound", b.from, k.human()));
                false
            }
            None => true,
        });
        (Self { app }, warnings)
    }

    /// The key that runs `act`, spelled for a reader (`Ctrl+,`): the first
    /// binding that is just that command.
    pub fn hint(&self, act: Act) -> Option<String> {
        let b = self.app.iter().find(|b| b.run.len() == 1 && b.run[0] == act)?;
        Some(ito_keys::human_seq(&b.on))
    }

    /// Every layer's bindings, by name in [`SPEC`]'s order: what the Keys
    /// page lists.
    pub fn layers(&self) -> Vec<(String, Vec<Binding>)> {
        vec![("app".to_owned(), self.app.clone())]
    }

    /// The built-in defaults alone, layer by layer, read once.
    pub fn built_in_layers() -> &'static [(String, Vec<Binding>)] {
        static BUILT_IN_LAYERS: std::sync::OnceLock<Vec<(String, Vec<Binding>)>> = std::sync::OnceLock::new();
        BUILT_IN_LAYERS.get_or_init(|| Self::load(None).0.layers())
    }

    /// What `key` does, if it is bound to one command.
    pub fn act(&self, key: Key) -> Option<Act> {
        match resolve(&self.app, &[key]) {
            Match::Exact(b) => b.run.first().copied(),
            _ => None,
        }
    }

    /// The first of `keys` that fude reads before the keymap (Esc, F12, the
    /// palette, the scale): bound, it never runs.
    pub fn fixed(keys: &[Key]) -> Option<&Key> {
        keys.iter().find(|k| FIXED.iter().any(|f| Key::parse(f).as_ref() == Some(*k)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> Key {
        Key::parse(s).unwrap()
    }

    #[test]
    fn the_defaults_parse_without_a_warning() {
        let (km, warn) = Keymap::load(None);
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(km.act(key("<C-,>")), Some(Act::Settings));
        assert_eq!(km.act(key("<F1>")), Some(Act::Help));
        assert_eq!(km.act(key("<C-q>")), Some(Act::Quit));
        assert_eq!(km.act(key("<C-o>")), Some(Act::Shelf));
        assert_eq!(km.act(key("<C-e>")), Some(Act::Export));
        assert_eq!(km.act(key("<C-p>")), Some(Act::Print));
        assert_eq!(km.act(key("a")), None, "a plain letter is the text's");
        assert_eq!(km.hint(Act::Settings).as_deref(), Some("Ctrl+,"));
    }

    #[test]
    fn a_prepended_key_wins_and_noop_takes_a_default_away() {
        let user = r#"
            [app]
            prepend_keymap = [
                { on = "<C-k>", run = "help" },
                { on = "<C-q>", run = "noop" },
            ]
        "#;
        let (km, warn) = Keymap::load(Some(user));
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(km.act(key("<C-k>")), Some(Act::Help));
        assert_eq!(km.act(key("<C-q>")), Some(Act::Noop));
        assert_eq!(km.hint(Act::Help).as_deref(), Some("Ctrl+K"));
    }

    #[test]
    fn a_command_that_is_not_one_and_a_fixed_key_are_skipped_with_a_warning() {
        let user = r#"
            [app]
            prepend_keymap = [
                { on = "<C-k>", run = "frobnicate" },
                { on = "<Esc>", run = "quit" },
            ]
        "#;
        let (km, warn) = Keymap::load(Some(user));
        assert_eq!(warn.len(), 2, "{warn:?}");
        assert!(warn[1].contains("`Esc` is fixed"), "{warn:?}");
        assert_eq!(km.act(key("<C-k>")), None);
        assert_eq!(km.act(key("<Esc>")), None);
        assert_eq!(Keymap::fixed(&[key("<C-0>")]), Some(&key("<C-0>")));
        assert_eq!(Keymap::fixed(&[key("<C-,>")]), None);
    }

    #[test]
    fn the_keys_egui_sends_are_the_keys_written() {
        use egui::{Key as K, Modifiers as M};
        let ctrl = M { ctrl: true, command: true, ..M::NONE };
        let (km, _) = Keymap::load(None);
        let at = |k, m| ito_keys::from_egui(k, &m).and_then(|k| km.act(k));
        assert_eq!(at(K::Comma, ctrl), Some(Act::Settings));
        assert_eq!(at(K::F1, M::NONE), Some(Act::Help));
        assert_eq!(at(K::Q, ctrl), Some(Act::Quit));
        assert_eq!(at(K::O, ctrl), Some(Act::Shelf));
        assert_eq!(at(K::E, ctrl), Some(Act::Export));
        assert_eq!(at(K::P, ctrl), Some(Act::Print));
    }
}
