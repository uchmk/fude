# Testing

The checks a person (or an agent with a screen) makes. Each section is a table of `# | Do | Expect`; a test whose doc
comment names a row (`/// 2.1: …` or `TESTING.md 2.1`) takes it off the list, and `cargo run --example make-testcheck`
writes what is left to `TESTING-CHECKS.md`.

## 1. Start

| # | Do | Expect |
| --- | --- | --- |
| 1.1 | Run `fude` | A window of 1280x800 opens with the title "fude" |
| 1.2 | Move and resize the window, close it and start it again | The window comes back at the same place and size |
| 1.3 | Press `F12` | The browser opens the repository's new issue form with the version and OS filled in |

## 2. Export

| # | Do | Expect |
| --- | --- | --- |
| 2.1 | On the clean copy, press `Ctrl+E`, then `Enter` | `export\<piece>.pdf` is written in the notebook's folder and the folder opens; the PDF shows the clean copy on A4 in mincho, as the window shows it |
| 2.2 | Pick "Export: PNG (the whole notebook)" in the palette, then export a shorter copy the same way | A PNG is written to each page of every piece; no page of the longer copy is left behind |
| 2.3 | Press `Ctrl+P`, then `Enter` | The piece goes to the printer, and the tools row says so |
