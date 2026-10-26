# Testing

The checks a person (or an agent with a screen) makes. Each section is a table of `# | Do | Expect`; a test whose doc
comment names a row (`/// 2.1: …` or `TESTING.md 2.1`) takes it off the list, and `cargo run --example make-testcheck`
writes what is left to `TESTING-CHECKS.md`.

## 1. Start

| # | Do | Expect |
| --- | --- | --- |
| 1.1 | Run `fude` | A window of 1280x800 opens with the title "fude"; the title bar and the taskbar show the icon, a vermilion square with a brush's stroke in it |
| 1.2 | Move and resize the window, close it and start it again | The window comes back at the same place and size |
| 1.3 | Press `F12` | The browser opens the repository's new issue form with the version and OS filled in |
| 1.4 | Start it with no `Documents\fude` folder | `Documents\fude\ノート\` is made and the band takes letters at once, with nothing to answer first |

## 2. Writing

| # | Do | Expect |
| --- | --- | --- |
| 2.1 | Type `きょうははれ` with the IME on, convert it and confirm | The letters being converted stand at the input point on the band; the input point stays where it is and the letters flow away from it |
| 2.2 | Press `Left` twice, type `X`, then `Home` and `End` | The input point goes back by a letter each time, `X` goes in there, and `Home` / `End` take it to the ends |
| 2.3 | Write `一。二、三` and press `Ctrl+Left` and `Ctrl+Right` | The input point jumps to just after each punctuation mark |
| 2.4 | Press `Enter` before the first letter, then twice after a letter | No paragraph is made before the first letter, and two `Enter`s make one break, not an empty paragraph |
| 2.5 | Paste text with a tab and three blank lines in it | The tab becomes a space and the blank lines one paragraph break |
| 2.6 | Settings, Writing: set the direction to vertical | The band stands upright and `Up` / `Down` move the input point |
| 2.7 | Type for a while without touching the mouse, then move it | The title bar fades while typing and comes back when the pointer moves |
| 2.8 | Type a few letters, close the window and start it again | The letters come back and the input point is after them |
| 2.9 | Settings, This notebook: turn on "no erasing", then press `Backspace`, `Delete` and `Left` | None of them does anything, and typing still goes on |

## 3. Limits

| # | Do | Expect |
| --- | --- | --- |
| 3.1 | Settings, This notebook: set the letter limit to 10, then type 12 letters | At the 10th the band says the limit is reached, a small bell rings once, and the screen stays as it is; the 11th and 12th go in |
| 3.2 | Set a time limit of 1 minute, with the clock starting at the first letter; wait, then type | The clock stands at 0:00 until the first letter, and the band tells of the limit a minute after it |
| 3.3 | Past a limit, press `Esc`, then `Enter` | The writing goes on, and the limit is raised or lifted as the Writing page says |

## 4. The clean copy

| # | Do | Expect |
| --- | --- | --- |
| 4.1 | Write two paragraphs and press `Esc` | The clean copy shows them on 20×20 manuscript paper written vertically in mincho, with the column, the fishtail and the page number |
| 4.2 | Write `「PCー」`, then pick graph paper, vertical, in the tools row | The brackets and the long mark take their upright forms, and `P` and `C` go one to a square |
| 4.3 | Write paragraphs beginning `今`, `「` and `A` | Only the one beginning `今` is indented by one square |
| 4.4 | Write a line that fills the paper's line so that `。` falls at the head of the next | `。` goes in the last square of the line before, or hangs below the paper as the This notebook page says; `。」` share one square |
| 4.5 | Write `｜漢字《かんじ》` and `《《強調》》` | The reading sits small beside `漢字`, and dots beside `強調`; none of the marks shows |
| 4.6 | Change the paper, the direction and the squares in the tools row | The clean copy is drawn again at once in each |
| 4.7 | Press `Ctrl+C` and paste into Notepad | The text comes with a full-width space at the head of each indented paragraph, and the tools row says how many letters were copied |
| 4.8 | Click a letter in the clean copy | The writing comes back with the input point at that letter |
| 4.9 | On the clean copy, type a letter | The writing comes back with that letter added at the end |
| 4.10 | Write a few lines at different speeds, stopping once for 40 seconds, and press `Esc` | Beside each line is a line as long as it was fast, and a dot where the hand stopped; Settings, Writing can turn them off, and they are not in a copy or an export |
| 4.11 | Write two pieces, then on the clean copy press `Left` and `Right` | The pieces turn over one by one (vertically, `Left` is the newer one), and at the end the band says there is no more |

## 5. Notebooks and the shelf

| # | Do | Expect |
| --- | --- | --- |
| 5.1 | Write a piece, press `Esc`, and open `Documents\fude\ノート\` | A `YYYY-MM-DD_HHMM.txt` named by when the piece was begun holds it, in UTF-8 with LF, one paragraph a line |
| 5.2 | Press `Ctrl+N` | The piece is bound to its file and a new one begins at once, with nothing asked |
| 5.3 | Press `Ctrl+O` | The shelf shows the notebooks on the left and the notebook's pieces on the right, newest first, with their letters, time and letters a minute |
| 5.4 | On the shelf, press `N`, type `日記` and `Enter` | `Documents\fude\日記\` is made and the new notebook is picked |
| 5.5 | Pick a notebook, press `F2`, rename it, and `Enter` | The folder is renamed with it; a name with `\` or one already there is refused with the reason |
| 5.6 | Pick a piece and press `Enter` | Its clean copy opens |
| 5.7 | Put a `.txt` of your own in the notebook's folder and open the shelf | It is listed as a piece and opens as one |
| 5.8 | On the clean copy, press `Ctrl+S` | The piece's file is written at once |

## 6. Topics and the record

| # | Do | Expect |
| --- | --- | --- |
| 6.1 | Settings, This notebook: add three topics, choose "in order", then press `Ctrl+N` three times | Each new piece shows the next topic faintly above the input point, fainter still once typing starts; the topics are in the notebook's `fude.toml` |
| 6.2 | Choose "by lot", then "none", and start new pieces | The topic is drawn at random, and then none shows |
| 6.3 | Write on two days running and open the shelf | Above it: today's letters, the days in a row and the letters a minute over the last 7 days; under the clean copy, today's letters and the days |

## 7. Sounds

| # | Do | Expect |
| --- | --- | --- |
| 7.1 | Settings, Writing: pick the typewriter, then type, erase, press `Enter` and the arrows | A key sound for each letter and each erasing, a heavier one for `Enter`, and none for the arrows; picking it plays it once |
| 7.2 | Pick the pencil, then type fast | A softer sound for each letter, and a run of keys does not run into one sound |
| 7.3 | Turn the volume down to 0, then up | The sounds go quiet and come back |
| 7.4 | Leave the window a minute after the last sound | The sound device is let go (the volume mixer no longer lists fude) |

## 8. Settings

| # | Do | Expect |
| --- | --- | --- |
| 8.1 | Press `Ctrl+,` | The settings open on General, Theme, Writing, This notebook, Keys, Advanced and About, in the theme and the language the other uchmk apps use |
| 8.2 | On Writing, change the letter size, the input point and what the band shows | Each takes effect at once and is written to `config.toml` |
| 8.3 | On This notebook, change the paper and the limits | They are written to `[note]` in the notebook's `fude.toml`, and another notebook keeps its own |
| 8.4 | Change the theme in another uchmk app | fude follows it without a restart |

## 9. Keys, the help and the palette

| # | Do | Expect |
| --- | --- | --- |
| 9.1 | Press `F1` on the writing, then on the clean copy | The key list shows the keymap's keys, then the writing's, the clean copy's, the shelf's and the fixed ones; closed on the clean copy, it goes back there |
| 9.2 | Press `Ctrl+Shift+P` and type `設定` | The palette lists the settings, the writing's page and this notebook's page; each opens the settings on its page |
| 9.3 | Write `[[app.keymap]]` with `on = "<F2>"` and `run = "help"` in `keymap.toml`, then press `F2` | The key list opens without a restart; a misspelt command is told of in a notification |
| 9.4 | Press `Ctrl+=`, `Ctrl+-` and `Ctrl+0` | Everything grows, shrinks and comes back to its size |
| 9.5 | Press `Ctrl+Q` | fude closes, and the piece is bound to its file first |

## 10. Export

| # | Do | Expect |
| --- | --- | --- |
| 10.1 | On the clean copy, press `Ctrl+E`, then `Enter` | `export\<piece>.pdf` is written in the notebook's folder and the folder opens; the PDF shows the clean copy on A4 in mincho, as the window shows it |
| 10.2 | Pick "Export: PNG (the whole notebook)" in the palette, then export a shorter copy the same way | A PNG is written to each page of every piece; no page of the longer copy is left behind |
| 10.3 | Press `Ctrl+P`, then `Enter` | The piece goes to the printer, and the tools row says so |
