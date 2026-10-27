//! `fude env`: what a bug report would otherwise have to ask for, laid out
//! as every uchmk app's `env` (`ito_common::report`). Printed, not opened in
//! a browser, and it never opens a window.

use ito_common::report::{files_rows, finish, presence, row, section, variables, version_rows, Rows};

/// The whole report.
pub fn text() -> String {
    let mut out = String::new();
    section(&mut out, "Fude", &version_rows(env!("CARGO_PKG_VERSION"), &ito_bugreport::os_line()));
    section(&mut out, "Files", &files());
    section(&mut out, "Last run", &last_run());
    section(
        &mut out,
        "Variables",
        &variables(&["UCHMK_CONFIG_DIR", "UCHMK_DOCUMENTS_DIR", "FUDE_STATE_HOME", "FUDE_KEYS_DONE", "WGPU_BACKEND", "LANG"], "(not set)"),
    );
    finish(&mut out);
    out
}

/// The shared folders, fude's own settings and where the notebooks are.
fn files() -> Rows {
    let mut rows = files_rows("fude");
    if let Some(base) = ito_common::base_dir() {
        rows.push(row("config.toml", presence(&crate::config::path(&base))));
        rows.push(row("keymap.toml", presence(&crate::prefs::keymap_path(&base))));
    }
    let notes = crate::notebook::root()
        .map_or_else(|| "none (this OS has no documents folder)".into(), |p| format!("{} ({})", p.display(), presence(&p)));
    rows.push(row("notebooks", notes));
    rows
}

/// What the last window drew with (`ito_common::runinfo`, written by `main`).
fn last_run() -> Rows {
    let Some(run) = ito_common::runinfo::load("fude") else {
        return vec![row("Last run", "no record (fude has not opened a window here)")];
    };
    let mut rows = vec![row("Version", run.version.clone())];
    if let Some([adapter, backend, device]) = run.rendering() {
        rows.push(row("Adapter", adapter));
        rows.push(row("Backend", backend));
        rows.push(row("Device", device));
    }
    if let Some(line) = run.window_line() {
        rows.push(row("Window", line));
    }
    rows
}

#[cfg(test)]
mod tests {
    /// The release checks `fude.com env` gives ten lines or more.
    #[test]
    fn the_report_is_long_enough_and_ends_in_one_newline() {
        let t = super::text();
        assert!(t.lines().count() >= 10, "{t}");
        assert!(t.starts_with("Fude\n") && t.contains("Version") && t.contains("notebooks"), "{t}");
        assert!(t.ends_with('\n') && !t.ends_with("\n\n"));
    }
}
