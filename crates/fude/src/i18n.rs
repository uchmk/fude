//! The app's own words, in English and Japanese. The language is the one in
//! common.toml (`auto`: the OS's), shared by every uchmk app; the settings
//! screen's own words are `ito_prefs::Words`.

use ito_i18n::Table;

const TABLES: &[(&str, &str)] = &[("en", include_str!("../lang/en.toml")), ("ja", include_str!("../lang/ja.toml"))];

/// The code to use: common.toml's `language`, else the OS's, else English.
pub fn resolve(common: Option<&str>) -> String {
    ito_i18n::resolve(None, common, ito_common::os_language().as_deref(), &["en", "ja"])
}

/// The table for `code`, and what in the files does not read.
pub fn table(code: &str) -> (Table, Vec<String>) {
    Table::new(TABLES, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    use ito_i18n::check;

    /// Both files read, and Japanese has every English word with the same
    /// holes to fill.
    #[test]
    fn the_tables_read_and_agree() {
        for code in ["en", "ja"] {
            let (_, errs) = table(code);
            assert!(errs.is_empty(), "{code}: {errs:?}");
        }
        assert_eq!(check::translations(TABLES), Vec::<String>::new());
    }

    /// Each built-in key's description has its Japanese.
    #[test]
    fn the_keymap_descriptions_are_translated() {
        let (km, _) = crate::keymap::Keymap::load(None);
        let descs: Vec<&str> = km.app.iter().map(|b| b.desc.as_str()).filter(|d| !d.is_empty()).collect();
        assert!(!descs.is_empty());
        assert_eq!(check::key_descriptions(TABLES, &descs), Vec::<String>::new());
    }

    /// Every word the source asks for is in en.toml.
    #[test]
    fn the_source_asks_for_words_that_are_there() {
        let (en, _) = table("en");
        let src = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        assert_eq!(check::source_keys(src, &en, &["i18n.rs"]), Vec::<String>::new());
    }
}
