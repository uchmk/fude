//! Notebooks: one folder each in `Documents\fude`, holding the pieces written
//! in it as `YYYY-MM-DD_HHMM.txt` (named by the local time a piece was begun;
//! UTF-8, LF, a paragraph to a line) and `fude.toml`, with the notebook's own
//! settings (`[note]`) and how each piece was written (`[pieces.<stem>]`: its
//! time, topic, pace and days). A `.txt` put there by hand is a piece too,
//! with no record. The disk work is
//! done on the [`Worker`]'s thread; the UI keeps what it last read.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::SystemTime;

use chrono::{NaiveDate, NaiveDateTime};
use ito_common::text::natural_cmp;

/// The notebook's settings and records, in its folder.
pub const FILE: &str = "fude.toml";

/// A piece's name: the local time it was begun, to the minute.
const STAMP: &str = "%Y-%m-%d_%H%M";

/// The settings screen and the worker both write `fude.toml`; one at a time,
/// each reading what the one before wrote.
pub static TOML_ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// Where the notebooks are: `fude` in the documents folder.
pub fn root() -> Option<PathBuf> {
    ito_common::documents_dir().map(|d| d.join("fude"))
}

/// One piece as it was read: its text with a paragraph to a line.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// The file's name without `.txt`.
    pub stem: String,
    pub text: String,
    /// Letters, the paragraph breaks not counted.
    pub chars: usize,
    /// How it was written; `None` for a piece fude did not time.
    pub rec: Option<Record>,
    pub modified: Option<SystemTime>,
}

/// How a piece was written, `[pieces.<stem>]` in `fude.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Record {
    /// How long it took, in milliseconds.
    pub ms: u64,
    /// The topic it was written to.
    pub topic: Option<String>,
    /// The letters at the end of each [`band::PACE`] of its time.
    ///
    /// [`band::PACE`]: crate::band::PACE
    pub pace: Vec<u32>,
    /// What it grew by on each day it was written on, oldest first.
    pub days: Vec<Day>,
}

/// What a piece grew by on one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Day {
    pub day: NaiveDate,
    pub chars: u32,
    pub ms: u64,
}

impl Piece {
    pub fn new(stem: &str, text: &str, rec: Option<Record>) -> Piece {
        Piece { stem: stem.into(), text: text.into(), chars: chars(text), rec, modified: Some(SystemTime::now()) }
    }

    /// How long it took, in milliseconds.
    pub fn ms(&self) -> Option<u64> {
        self.rec.as_ref().map(|r| r.ms)
    }

    /// What it grew by on each day. A record from before the days were kept
    /// counts as all written on the day the piece was begun.
    pub fn days(&self) -> Vec<Day> {
        match &self.rec {
            Some(r) if r.days.is_empty() => self
                .begun()
                .map(|b| Day { day: b.date(), chars: self.chars as u32, ms: r.ms })
                .filter(|d| d.chars > 0 || d.ms > 0)
                .into_iter()
                .collect(),
            Some(r) => r.days.clone(),
            None => Vec::new(),
        }
    }

    /// When it was begun: by its name, or else when it was last changed.
    pub fn begun(&self) -> Option<NaiveDateTime> {
        begun(&self.stem).or_else(|| self.modified.map(|m| chrono::DateTime::<chrono::Local>::from(m).naive_local()))
    }

    /// Its first paragraph, cut at `max` letters with "…".
    pub fn opening(&self, max: usize) -> String {
        let first = self.text.lines().next().unwrap_or("");
        if first.chars().count() <= max {
            first.to_owned()
        } else {
            first.chars().take(max).chain(['…']).collect()
        }
    }
}

/// One notebook as it was read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    pub name: String,
    pub dir: PathBuf,
    /// `fude.toml`'s `[note]`.
    pub table: toml::Table,
    /// Oldest first, by their names.
    pub pieces: Vec<Piece>,
    /// Why `fude.toml` could not be read.
    pub warn: Option<String>,
    /// The newest change in it: the folder's or a piece's.
    pub modified: Option<SystemTime>,
}

impl Note {
    /// A notebook with nothing in it yet.
    pub fn empty(name: &str, dir: PathBuf) -> Note {
        Note { name: name.into(), dir, modified: Some(SystemTime::now()), ..Default::default() }
    }

    pub fn piece(&self, stem: &str) -> Option<&Piece> {
        self.pieces.iter().find(|p| p.stem == stem)
    }

    /// The piece bound just now, in place of what was read of it.
    pub fn put(&mut self, piece: Piece) {
        self.modified = piece.modified.max(self.modified);
        match self.pieces.iter_mut().find(|p| p.stem == piece.stem) {
            Some(p) => *p = piece,
            None => {
                self.pieces.push(piece);
                sort_pieces(&mut self.pieces);
            }
        }
    }
}

/// `days` with `chars` letters and `ms` more on `day`.
pub fn add_day(days: &mut Vec<Day>, day: NaiveDate, chars: u32, ms: u64) {
    if chars == 0 && ms == 0 {
        return;
    }
    match days.iter_mut().find(|d| d.day == day) {
        Some(d) => {
            d.chars += chars;
            d.ms += ms;
        }
        None => {
            days.push(Day { day, chars, ms });
            days.sort_by_key(|d| d.day);
        }
    }
}

/// The days of a piece that was `had` and is now `chars` letters long,
/// having taken `ms`: what it grew by goes to `today`. Letters put in it by
/// hand were never timed, so they are not counted.
pub fn grown(had: Option<&Piece>, chars: usize, ms: u64, today: NaiveDate) -> Vec<Day> {
    let mut days = had.map(Piece::days).unwrap_or_default();
    let (c, m) = had.map_or((0, 0), |p| (p.chars, p.ms().unwrap_or(0)));
    add_day(&mut days, today, chars.saturating_sub(c) as u32, ms.saturating_sub(m));
    days
}

/// Letters, the paragraph breaks not counted.
pub fn chars(text: &str) -> usize {
    text.chars().filter(|&c| c != '\n').count()
}

/// When a piece named `stem` was begun, if its name says.
pub fn begun(stem: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(stem.get(..15)?, STAMP).ok()
}

/// A file's text as a piece has it: no BOM, LF, tabs as spaces, and no empty
/// paragraphs.
pub fn normalize(raw: &str) -> String {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', " ")
        .split('\n')
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// What a piece's file holds: its text and a last line break.
fn body(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!("{text}\n")
    }
}

fn sort_pieces(pieces: &mut [Piece]) {
    pieces.sort_by(|a, b| natural_cmp(&a.stem, &b.stem, true));
}

/// The newest changed first, then by name.
pub fn sort_notes(notes: &mut [Note]) {
    notes.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| natural_cmp(&a.name, &b.name, false)));
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// A folder's name that is a notebook (not hidden).
fn shown(name: &str) -> bool {
    !name.starts_with('.')
}

/// Every notebook in `root`, the newest changed first. None when there is
/// no such folder.
pub fn scan(root: &Path) -> Vec<Note> {
    let Ok(entries) = fs::read_dir(root) else { return Vec::new() };
    let mut notes: Vec<Note> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| e.file_name().to_str().is_some_and(shown))
        .map(|e| read_note(&e.path()))
        .collect();
    sort_notes(&mut notes);
    notes
}

/// The name of the notebook in `root` changed last: where writing goes on.
pub fn newest(root: &Path) -> Option<String> {
    let entries = fs::read_dir(root).ok()?;
    entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| Some((e.file_name().into_string().ok()?, modified(&e.path()))))
        .filter(|(n, _)| shown(n))
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| natural_cmp(&b.0, &a.0, false)))
        .map(|(n, _)| n)
}

/// `fude.toml` in `dir`, as a table; empty when there is none.
fn read_toml(dir: &Path) -> Result<toml::Table, String> {
    let path = dir.join(FILE);
    let text = ito_common::read_or_empty(&path)?;
    text.parse::<toml::Table>().map_err(|e| format!("{}: {e}", path.display()))
}

/// The notebook's own settings, `[note]` in its `fude.toml`.
pub fn read_table(dir: &Path) -> Result<toml::Table, String> {
    Ok(note_table(&read_toml(dir)?))
}

fn note_table(t: &toml::Table) -> toml::Table {
    t.get("note").and_then(|v| v.as_table()).cloned().unwrap_or_default()
}

/// The record of the piece `stem` in `fude.toml`, read as `t`.
fn record_in(t: &toml::Table, stem: &str) -> Option<Record> {
    let r = t.get("pieces")?.get(stem)?.as_table()?;
    let ms = u64::try_from(r.get("ms")?.as_integer()?).ok()?;
    let topic = r.get("topic").and_then(|v| v.as_str()).map(str::to_owned);
    let list = |k: &str| r.get(k).and_then(|v| v.as_array()).into_iter().flatten();
    let pace = list("pace").filter_map(|v| u32::try_from(v.as_integer()?).ok()).collect();
    let days = list("days").filter_map(|v| day_in(v.as_table()?)).collect();
    Some(Record { ms, topic, pace, days })
}

/// One of a record's `days`: `{ day = 2026-10-27, chars = 120, ms = 65000 }`.
fn day_in(t: &toml::Table) -> Option<Day> {
    let day = match t.get("day")? {
        toml::Value::Datetime(d) => {
            let d = d.date?;
            NaiveDate::from_ymd_opt(d.year.into(), d.month.into(), d.day.into())?
        }
        toml::Value::String(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?,
        _ => return None,
    };
    let chars = u32::try_from(t.get("chars")?.as_integer()?).ok()?;
    let ms = u64::try_from(t.get("ms")?.as_integer()?).ok()?;
    Some(Day { day, chars, ms })
}

/// The notebook in `dir`, its pieces read.
pub fn read_note(dir: &Path) -> Note {
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (toml, warn) = match read_toml(dir) {
        Ok(t) => (t, None),
        Err(e) => (toml::Table::new(), Some(e)),
    };
    let mut pieces = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let path = e.path();
            let txt = path.extension().is_some_and(|x| x.eq_ignore_ascii_case("txt"));
            if !txt || !e.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            let Ok(bytes) = fs::read(&path) else { continue };
            let text = normalize(&String::from_utf8_lossy(&bytes));
            pieces.push(Piece { stem: stem.into(), chars: chars(&text), text, rec: record_in(&toml, stem), modified: modified(&path) });
        }
    }
    sort_pieces(&mut pieces);
    let newest = pieces.iter().filter_map(|p| p.modified).max();
    Note { name, dir: dir.into(), table: note_table(&toml), pieces, warn, modified: newest.max(modified(dir)) }
}

/// The name for a piece begun at `at`; one already `taken` gets `_2`, `_3`…
pub fn stem_at(at: NaiveDateTime, taken: impl Fn(&str) -> bool) -> String {
    let base = at.format(STAMP).to_string();
    if !taken(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}_{n}")).find(|s| !taken(s)).unwrap_or(base)
}

/// A name `fude.toml` can hold a record under as it is.
fn bare(stem: &str) -> bool {
    !stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// What is wrong with a notebook's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadName {
    Empty,
    /// A letter or a name the file system will not take.
    Unfit,
}

/// `name` made fit for a folder (trimmed), or why it cannot be.
pub fn valid_name(name: &str) -> Result<String, BadName> {
    let name = name.trim();
    if name.is_empty() {
        return Err(BadName::Empty);
    }
    let unfit = name.chars().any(|c| c.is_control() || r#"\/:*?"<>|"#.contains(c))
        || name.starts_with('.')
        || name.ends_with('.')
        || reserved(name);
    if unfit {
        return Err(BadName::Unfit);
    }
    Ok(name.to_owned())
}

/// The names Windows keeps for devices, with or without an extension.
fn reserved(name: &str) -> bool {
    let head = name.split('.').next().unwrap_or(name).trim_end().to_ascii_uppercase();
    matches!(head.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((head.starts_with("COM") || head.starts_with("LPT"))
            && head.len() == 4
            && head.as_bytes()[3].is_ascii_digit()
            && head.as_bytes()[3] != b'0')
}

/// The file of the piece `stem` in `dir`: `stem.txt`, or one put there by
/// hand with `.txt` in another case, which [`read_note`] reads as a piece too
/// and a file system that minds the case does not find as `stem.txt`.
pub fn piece_file(dir: &Path, stem: &str) -> PathBuf {
    let path = dir.join(format!("{stem}.txt"));
    if path.exists() {
        return path;
    }
    let same =
        |p: &Path| p.file_stem().is_some_and(|s| s == stem) && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("txt")) && p.is_file();
    fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).find(|p| same(p)).unwrap_or(path)
}

/// Put `text` into the piece `stem` in `dir` (the folder made if need be),
/// written only when it differs from what is there; nothing written for an
/// empty piece that has no file yet. `rec` goes into `fude.toml` when it
/// changes it, its days counted again from what is on the disk with what the
/// piece grew by going to `today`. Whether the piece's file was written.
pub fn bind(dir: &Path, stem: &str, text: &str, rec: Option<&Record>, today: NaiveDate) -> Result<bool, String> {
    let path = piece_file(dir, stem);
    let had = match fs::read(&path) {
        Ok(b) => Some(normalize(&String::from_utf8_lossy(&b))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let had_chars = had.as_deref().map(chars);
    let wrote = match had {
        None if text.is_empty() => return Ok(false),
        Some(h) if h == text => false,
        _ => {
            ito_common::write_atomic(&path, body(text))?;
            true
        }
    };
    if let Some(rec) = rec.filter(|_| bare(stem)) {
        record(dir, stem, rec, had_chars, chars(text), today)?;
    }
    Ok(wrote)
}

/// `[pieces.<stem>]` in `dir`'s `fude.toml`, written when it changes it: the
/// time, topic and pace of `rec`, and the days on the disk with what the
/// piece grew by from `had` letters to `chars` going to `today`.
fn record(dir: &Path, stem: &str, rec: &Record, had: Option<usize>, chars: usize, today: NaiveDate) -> Result<(), String> {
    let _one = TOML_ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let path = dir.join(FILE);
    let at = |e: String| format!("{}: {e}", path.display());
    let text = ito_common::read_or_empty(&path)?;
    let old = text.parse::<toml::Table>().map_err(|e| at(e.to_string()))?;
    let old = record_in(&old, stem);
    let was = (had.is_some() || old.is_some()).then(|| Piece {
        stem: stem.into(),
        text: String::new(),
        chars: had.unwrap_or(0),
        rec: old,
        modified: None,
    });
    let days = grown(was.as_ref(), chars, rec.ms, today);
    let table = format!("pieces.{stem}");
    let set = |text: &str, key: &str, value: &str| ito_common::set_key(text, Some(&table), key, value).map_err(at);
    let mut new = set(&text, "ms", &rec.ms.to_string())?;
    if let Some(t) = &rec.topic {
        new = set(&new, "topic", &ito_common::quote(t))?;
    }
    if !rec.pace.is_empty() {
        let pace: Vec<String> = rec.pace.iter().map(u32::to_string).collect();
        new = set(&new, "pace", &format!("[{}]", pace.join(", ")))?;
    }
    if !days.is_empty() {
        let days: Vec<String> =
            days.iter().map(|d| format!("{{ day = {}, chars = {}, ms = {} }}", d.day.format("%Y-%m-%d"), d.chars, d.ms)).collect();
        new = set(&new, "days", &format!("[{}]", days.join(", ")))?;
    }
    if new != text {
        ito_common::write_atomic(&path, new)?;
    }
    Ok(())
}

/// A new notebook's folder, with `table` as its own settings when there are
/// any (those of the notebook it was made from).
pub fn make(dir: &Path, table: &toml::Table) -> Result<(), String> {
    if dir.exists() {
        return Err(format!("{}: {}", dir.display(), std::io::Error::from(std::io::ErrorKind::AlreadyExists)));
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if table.is_empty() {
        return Ok(());
    }
    let mut t = toml::Table::new();
    t.insert("note".into(), toml::Value::Table(table.clone()));
    let text = toml::to_string(&t).map_err(|e| e.to_string())?;
    ito_common::write_atomic(&dir.join(FILE), text)
}

/// A notebook's folder renamed; not over one that is there.
pub fn rename(from: &Path, to: &Path) -> Result<(), String> {
    // Only the case changed: the same folder on Windows.
    let same = from.to_string_lossy().to_lowercase() == to.to_string_lossy().to_lowercase();
    if to.exists() && !same {
        return Err(format!("{}: {}", to.display(), std::io::Error::from(std::io::ErrorKind::AlreadyExists)));
    }
    fs::rename(from, to).map_err(|e| format!("{} → {}: {e}", from.display(), to.display()))
}

/// The disk work the UI asks for.
#[derive(Debug)]
pub enum Job {
    Bind {
        dir: PathBuf,
        stem: String,
        text: String,
        rec: Option<Record>,
    },
    Make {
        dir: PathBuf,
        table: toml::Table,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    /// Read every notebook again.
    Scan,
}

/// What came of a [`Job`].
#[derive(Debug)]
pub enum Done {
    /// Every notebook, read again (after a scan, a notebook made or one
    /// renamed).
    Scanned(Vec<Note>),
    /// One notebook, read again after a piece was bound in it.
    Note(Note),
    Failed(String),
}

/// The thread that does the [`Job`]s in the order they come, in `root`.
pub struct Worker {
    tx: Option<Sender<Job>>,
    rx: Receiver<Done>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(root: PathBuf, wake: impl Fn() + Send + 'static) -> Worker {
        let (tx, jobs) = channel::<Job>();
        let (back, rx) = channel();
        let thread = std::thread::Builder::new()
            .name("fude-notebook".into())
            .spawn(move || {
                let send = |d: Done| {
                    let _ = back.send(d);
                    wake();
                };
                while let Ok(job) = jobs.recv() {
                    match job {
                        Job::Bind { dir, stem, text, rec } => match bind(&dir, &stem, &text, rec.as_ref(), today()) {
                            Ok(_) => send(Done::Note(read_note(&dir))),
                            Err(e) => send(Done::Failed(e)),
                        },
                        Job::Make { dir, table } => {
                            if let Err(e) = make(&dir, &table) {
                                send(Done::Failed(e));
                            }
                            send(Done::Scanned(scan(&root)));
                        }
                        Job::Rename { from, to } => {
                            if let Err(e) = rename(&from, &to) {
                                send(Done::Failed(e));
                            }
                            send(Done::Scanned(scan(&root)));
                        }
                        Job::Scan => send(Done::Scanned(scan(&root))),
                    }
                }
            })
            .ok();
        Worker { tx: Some(tx), rx, thread }
    }

    pub fn send(&self, job: Job) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(job);
        }
    }

    /// What the thread has done since the last call.
    pub fn done(&self) -> Option<Done> {
        self.rx.try_recv().ok()
    }

    /// Wait for what was asked to be done (on the way out).
    pub fn finish(&mut self) {
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The local date: the day what is written now counts on.
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// The piece `by` places from `at` among `stems` (oldest first), where `at`
/// is `None` for a piece with no name yet (after the last). `None` at either
/// end.
pub fn flip<'a>(stems: &[&'a str], at: Option<&str>, by: isize) -> Option<&'a str> {
    let mut all: Vec<&str> = stems.to_vec();
    if let Some(a) = at.filter(|a| !all.contains(a)) {
        all.push(a);
    }
    all.sort_by(|a, b| natural_cmp(a, b, true));
    let here = match at {
        Some(a) => all.iter().position(|s| *s == a)? as isize,
        None => all.len() as isize,
    };
    let to = usize::try_from(here + by).ok()?;
    let s = *all.get(to)?;
    stems.iter().find(|x| **x == s).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fude-notebook-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn timed(ms: u64) -> Record {
        Record { ms, ..Record::default() }
    }

    #[test]
    fn a_piece_is_named_by_when_it_was_begun() {
        assert_eq!(stem_at(at("2026-10-27 09:05"), |_| false), "2026-10-27_0905");
        let taken = ["2026-10-27_0905", "2026-10-27_0905_2"];
        assert_eq!(stem_at(at("2026-10-27 09:05"), |s| taken.contains(&s)), "2026-10-27_0905_3");
        assert_eq!(begun("2026-10-27_0905_3"), Some(at("2026-10-27 09:05")));
        assert_eq!(begun("2026-10-27_0905"), Some(at("2026-10-27 09:05")));
        assert_eq!(begun("memo"), None);
        assert_eq!(begun("2026-13-27_0905"), None);
    }

    #[test]
    fn a_file_is_read_as_paragraphs() {
        assert_eq!(normalize("\u{feff}一\r\n\r\n\r\n二\t三\r\n"), "一\n二 三");
        assert_eq!(normalize("\n\n"), "");
        assert_eq!(chars("一\n二三"), 3);
        let p = Piece::new("s", "あいうえお\nかき", None);
        assert_eq!(p.opening(3), "あいう…");
        assert_eq!(p.opening(5), "あいうえお");
    }

    #[test]
    fn names_the_file_system_will_not_take_are_turned_down() {
        assert_eq!(valid_name("  旅の記録 "), Ok("旅の記録".into()));
        assert_eq!(valid_name(" "), Err(BadName::Empty));
        for bad in ["a/b", "a\\b", "a:b", "a?", "a*", "\"a\"", "<a>", "a|b", ".", "..", ".hidden", "a.", "CON", "nul.txt", "com1", "LPT9"] {
            assert_eq!(valid_name(bad), Err(BadName::Unfit), "{bad}");
        }
        for good in ["COM0", "console", "コン", "a.b"] {
            assert!(valid_name(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn a_piece_is_bound_with_its_time_and_read_back() {
        let root = scratch("bind");
        let dir = root.join("旅");
        let d = day("2026-10-27");
        // An empty piece with no file: nothing is made.
        assert_eq!(bind(&dir, "2026-10-27_0905", "", Some(&timed(5)), d), Ok(false));
        assert!(!dir.exists());
        assert_eq!(bind(&dir, "2026-10-27_0905", "一行目\n二行目", Some(&timed(65_000)), d), Ok(true));
        assert_eq!(fs::read_to_string(dir.join("2026-10-27_0905.txt")).unwrap(), "一行目\n二行目\n");
        // The same again: the file is left, the time goes in.
        assert_eq!(bind(&dir, "2026-10-27_0905", "一行目\n二行目", Some(&timed(70_000)), d), Ok(false));
        // A piece put there by hand, with CRLF and a BOM, and one not named by its time.
        fs::write(dir.join("2026-10-26_2200.TXT"), "\u{feff}手で\r\n\r\n置いた\r\n").unwrap();
        fs::write(dir.join("my memo.txt"), "メモ").unwrap();
        fs::write(dir.join("notes.md"), "not a piece").unwrap();
        // Hand-written CRLF read as it is: binding the same text leaves the file alone.
        assert_eq!(bind(&dir, "2026-10-26_2200", "手で\n置いた", None, d), Ok(false));
        // Changed, it is written into that file, not a new `.txt` beside it.
        assert_eq!(bind(&dir, "2026-10-26_2200", "手で\n置いた。", None, d), Ok(true));
        assert_eq!(bind(&dir, "my memo", "メモ。", Some(&timed(1)), d), Ok(true));
        let toml = fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!toml.contains("memo") && toml.contains("[pieces.2026-10-27_0905]"), "{toml}");

        let notes = scan(&root);
        assert_eq!(notes.len(), 1);
        let n = &notes[0];
        assert_eq!(n.name, "旅");
        assert!(n.warn.is_none());
        let stems: Vec<&str> = n.pieces.iter().map(|p| p.stem.as_str()).collect();
        assert_eq!(stems, ["2026-10-26_2200", "2026-10-27_0905", "my memo"]);
        assert_eq!(n.pieces[1].ms(), Some(70_000));
        assert_eq!(n.pieces[1].chars, 6);
        assert_eq!(n.pieces[1].days(), [Day { day: d, chars: 6, ms: 70_000 }]);
        assert_eq!(n.pieces[0].text, "手で\n置いた。");
        assert_eq!(n.pieces[0].ms(), None);
        assert_eq!(n.pieces[2].text, "メモ。");
        // Emptied: the file stays, empty.
        assert_eq!(bind(&dir, "my memo", "", None, d), Ok(true));
        assert_eq!(fs::read_to_string(dir.join("my memo.txt")).unwrap(), "");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_record_keeps_the_topic_the_pace_and_what_each_day_added() {
        let root = scratch("record");
        let dir = root.join("旅");
        let stem = "2026-10-26_2200";
        let rec = |ms, pace: &[u32]| Record { ms, topic: Some("朝の\"音\"".into()), pace: pace.to_vec(), days: Vec::new() };
        bind(&dir, stem, "あいう", Some(&rec(10_000, &[2, 3])), day("2026-10-26")).unwrap();
        // Two more letters and 20 seconds the next day; a day bound again adds to itself.
        bind(&dir, stem, "あいう\nえお", Some(&rec(25_000, &[2, 3, 3, 4, 5])), day("2026-10-27")).unwrap();
        bind(&dir, stem, "あいう\nえおか", Some(&rec(30_000, &[2, 3, 3, 4, 5, 6])), day("2026-10-27")).unwrap();
        // Letters taken out are not counted against a day.
        bind(&dir, stem, "あ", Some(&rec(31_000, &[2, 3, 3, 4, 5, 6])), day("2026-10-28")).unwrap();
        let toml = fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(toml.contains("days = [{ day = 2026-10-26, chars = 3, ms = 10000 }"), "{toml}");
        let p = &read_note(&dir).pieces[0];
        let r = p.rec.as_ref().unwrap();
        assert_eq!(r.topic.as_deref(), Some("朝の\"音\""));
        assert_eq!(r.pace, [2, 3, 3, 4, 5, 6]);
        assert_eq!(
            p.days(),
            [
                Day { day: day("2026-10-26"), chars: 3, ms: 10_000 },
                Day { day: day("2026-10-27"), chars: 3, ms: 20_000 },
                Day { day: day("2026-10-28"), chars: 0, ms: 1_000 },
            ]
        );
        // A record from before the days were kept: all on the day it was begun, and
        // what is written on it later goes on the day it is.
        fs::write(dir.join(FILE), "[pieces.2026-10-26_2200]\nms = 9000\n").unwrap();
        let p = &read_note(&dir).pieces[0];
        assert_eq!(p.days(), [Day { day: day("2026-10-26"), chars: 1, ms: 9_000 }]);
        assert_eq!(
            grown(Some(p), 3, 12_000, day("2026-10-29")),
            [Day { day: day("2026-10-26"), chars: 1, ms: 9_000 }, Day { day: day("2026-10-29"), chars: 2, ms: 3_000 }]
        );
        bind(&dir, stem, "あいう", Some(&timed(12_000)), day("2026-10-29")).unwrap();
        assert_eq!(read_note(&dir).pieces[0].days(), grown(Some(p), 3, 12_000, day("2026-10-29")));
        // A day written by hand as a string reads too.
        fs::write(dir.join(FILE), "[pieces.2026-10-26_2200]\nms = 9000\ndays = [{ day = \"2026-10-30\", chars = 4, ms = 5 }]\n").unwrap();
        assert_eq!(read_note(&dir).pieces[0].days(), [Day { day: day("2026-10-30"), chars: 4, ms: 5 }]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn notebooks_are_made_renamed_and_read_newest_first() {
        let root = scratch("make");
        let mut table = toml::Table::new();
        table.insert("paper".into(), "ruled".into());
        make(&root.join("一"), &table).unwrap();
        assert!(make(&root.join("一"), &table).is_err());
        make(&root.join("二"), &toml::Table::new()).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        fs::write(root.join("loose.txt"), "not in a notebook").unwrap();
        bind(&root.join("一"), "2026-10-27_0905", "新しい", None, day("2026-10-27")).unwrap();
        let notes = scan(&root);
        let names: Vec<&str> = notes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["一", "二"]);
        assert_eq!(notes[0].table.get("paper").and_then(|v| v.as_str()), Some("ruled"));
        assert!(notes[1].table.is_empty());
        assert_eq!(newest(&root).as_deref(), Some("一"));
        assert_eq!(read_table(&root.join("一")).unwrap(), table);

        assert!(rename(&root.join("二"), &root.join("一")).is_err());
        rename(&root.join("二"), &root.join("三")).unwrap();
        rename(&root.join("三"), &root.join("三")).unwrap();
        let names: Vec<String> = scan(&root).into_iter().map(|n| n.name).collect();
        assert!(names.contains(&"三".to_owned()) && !names.contains(&"二".to_owned()), "{names:?}");

        // A broken fude.toml is said, and the pieces are still read.
        fs::write(root.join("三").join(FILE), "[note\n").unwrap();
        bind(&root.join("三"), "a", "字", None, day("2026-10-27")).unwrap();
        let n = read_note(&root.join("三"));
        assert!(n.warn.is_some() && n.pieces.len() == 1);
        assert_eq!(scan(&root.join("none")), Vec::new());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_worker_binds_and_reads_back() {
        let root = scratch("worker");
        let mut w = Worker::start(root.clone(), || {});
        let dir = root.join("ノート");
        w.send(Job::Bind { dir: dir.clone(), stem: "2026-10-27_0905".into(), text: "書いた".into(), rec: Some(timed(3)) });
        w.send(Job::Rename { from: root.join("none"), to: root.join("x") });
        w.finish();
        let done: Vec<Done> = std::iter::from_fn(|| w.done()).collect();
        assert!(matches!(&done[0], Done::Note(n) if n.pieces.len() == 1 && n.pieces[0].ms() == Some(3)), "{done:?}");
        assert!(matches!(&done[1], Done::Failed(_)), "{done:?}");
        assert!(matches!(&done[2], Done::Scanned(n) if n.len() == 1), "{done:?}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pieces_flip_in_the_order_they_were_begun() {
        let stems = ["2026-10-27_0905", "2026-10-26_2200", "2026-10-27_0905_2"];
        assert_eq!(flip(&stems, Some("2026-10-27_0905"), 1), Some("2026-10-27_0905_2"));
        assert_eq!(flip(&stems, Some("2026-10-27_0905"), -1), Some("2026-10-26_2200"));
        assert_eq!(flip(&stems, Some("2026-10-26_2200"), -1), None);
        assert_eq!(flip(&stems, Some("2026-10-27_0905_2"), 1), None);
        // Not bound yet: after the last.
        assert_eq!(flip(&stems, Some("2026-10-28_1000"), -1), Some("2026-10-27_0905_2"));
        assert_eq!(flip(&stems, Some("2026-10-28_1000"), 1), None);
        assert_eq!(flip(&stems, None, -1), Some("2026-10-27_0905_2"));
        assert_eq!(flip(&stems, None, 1), None);
        assert_eq!(flip(&[], None, -1), None);
    }
}
