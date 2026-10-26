//! Notebooks: one folder each in `Documents\fude`, holding the pieces written
//! in it as `YYYY-MM-DD_HHMM.txt` (named by the local time a piece was begun;
//! UTF-8, LF, a paragraph to a line) and `fude.toml`, with the notebook's own
//! settings (`[note]`) and how long each piece took (`[pieces.<stem>]`). A
//! `.txt` put there by hand is a piece too, with no record. The disk work is
//! done on the [`Worker`]'s thread; the UI keeps what it last read.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::SystemTime;

use chrono::NaiveDateTime;
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
    /// How long it took, in milliseconds; `None` with no record.
    pub ms: Option<u64>,
    pub modified: Option<SystemTime>,
}

impl Piece {
    pub fn new(stem: &str, text: &str, ms: Option<u64>) -> Piece {
        Piece { stem: stem.into(), text: text.into(), chars: chars(text), ms, modified: Some(SystemTime::now()) }
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

/// The notebook in `dir`, its pieces read.
pub fn read_note(dir: &Path) -> Note {
    let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (toml, warn) = match read_toml(dir) {
        Ok(t) => (t, None),
        Err(e) => (toml::Table::new(), Some(e)),
    };
    let records = toml.get("pieces").and_then(|v| v.as_table());
    let ms = |stem: &str| {
        let n = records?.get(stem)?.get("ms")?.as_integer()?;
        u64::try_from(n).ok()
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
            pieces.push(Piece { stem: stem.into(), chars: chars(&text), text, ms: ms(stem), modified: modified(&path) });
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

/// Put `text` into the piece `stem` in `dir` (the folder made if need be),
/// written only when it differs from what is there; nothing written for an
/// empty piece that has no file yet. `ms` goes into `fude.toml` when it
/// changes it. Whether the piece's file was written.
pub fn bind(dir: &Path, stem: &str, text: &str, ms: Option<u64>) -> Result<bool, String> {
    let path = dir.join(format!("{stem}.txt"));
    let had = match fs::read(&path) {
        Ok(b) => Some(normalize(&String::from_utf8_lossy(&b))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let wrote = match had {
        None if text.is_empty() => return Ok(false),
        Some(h) if h == text => false,
        _ => {
            ito_common::write_atomic(&path, body(text))?;
            true
        }
    };
    if let Some(ms) = ms.filter(|_| bare(stem)) {
        record(dir, stem, ms)?;
    }
    Ok(wrote)
}

/// `[pieces.<stem>] ms` in `dir`'s `fude.toml`, written when it changes it.
fn record(dir: &Path, stem: &str, ms: u64) -> Result<(), String> {
    let _one = TOML_ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let path = dir.join(FILE);
    let at = |e: String| format!("{}: {e}", path.display());
    let text = ito_common::read_or_empty(&path)?;
    let new = ito_common::set_key(&text, Some(&format!("pieces.{stem}")), "ms", &ms.to_string()).map_err(at)?;
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
        ms: Option<u64>,
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
                        Job::Bind { dir, stem, text, ms } => match bind(&dir, &stem, &text, ms) {
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
        // An empty piece with no file: nothing is made.
        assert_eq!(bind(&dir, "2026-10-27_0905", "", Some(5)), Ok(false));
        assert!(!dir.exists());
        assert_eq!(bind(&dir, "2026-10-27_0905", "一行目\n二行目", Some(65_000)), Ok(true));
        assert_eq!(fs::read_to_string(dir.join("2026-10-27_0905.txt")).unwrap(), "一行目\n二行目\n");
        // The same again: the file is left, the time goes in.
        assert_eq!(bind(&dir, "2026-10-27_0905", "一行目\n二行目", Some(70_000)), Ok(false));
        // A piece put there by hand, with CRLF and a BOM, and one not named by its time.
        fs::write(dir.join("2026-10-26_2200.TXT"), "\u{feff}手で\r\n\r\n置いた\r\n").unwrap();
        fs::write(dir.join("my memo.txt"), "メモ").unwrap();
        fs::write(dir.join("notes.md"), "not a piece").unwrap();
        // Hand-written CRLF read as it is: binding the same text leaves the file alone.
        assert_eq!(bind(&dir, "2026-10-26_2200", "手で\n置いた", None), Ok(false));
        assert_eq!(bind(&dir, "my memo", "メモ。", Some(1)), Ok(true));
        let toml = fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!toml.contains("memo") && toml.contains("[pieces.2026-10-27_0905]"), "{toml}");

        let notes = scan(&root);
        assert_eq!(notes.len(), 1);
        let n = &notes[0];
        assert_eq!(n.name, "旅");
        assert!(n.warn.is_none());
        let stems: Vec<&str> = n.pieces.iter().map(|p| p.stem.as_str()).collect();
        assert_eq!(stems, ["2026-10-26_2200", "2026-10-27_0905", "my memo"]);
        assert_eq!(n.pieces[1].ms, Some(70_000));
        assert_eq!(n.pieces[1].chars, 6);
        assert_eq!(n.pieces[0].text, "手で\n置いた");
        assert_eq!(n.pieces[0].ms, None);
        assert_eq!(n.pieces[2].text, "メモ。");
        // Emptied: the file stays, empty.
        assert_eq!(bind(&dir, "my memo", "", None), Ok(true));
        assert_eq!(fs::read_to_string(dir.join("my memo.txt")).unwrap(), "");
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
        bind(&root.join("一"), "2026-10-27_0905", "新しい", None).unwrap();
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
        bind(&root.join("三"), "a", "字", None).unwrap();
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
        w.send(Job::Bind { dir: dir.clone(), stem: "2026-10-27_0905".into(), text: "書いた".into(), ms: Some(3) });
        w.send(Job::Rename { from: root.join("none"), to: root.join("x") });
        w.finish();
        let done: Vec<Done> = std::iter::from_fn(|| w.done()).collect();
        assert!(matches!(&done[0], Done::Note(n) if n.pieces.len() == 1 && n.pieces[0].ms == Some(3)), "{done:?}");
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
