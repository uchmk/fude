//! The clean copy out of the window: the piece on the band, or every piece
//! of its notebook, as PDF or PNG in the notebook's `export` folder, or to
//! the printer. The pages are drawn as SVG here on the UI thread, since they
//! are measured in egui's fonts; a thread of their own makes the files, which
//! takes a while (the system's fonts are read at the first).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;

use ito_notices::Kind;

use crate::{svg, App, Screen};

/// What the pages are made into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum What {
    Pdf,
    Png,
    /// A PDF, handed to the printer.
    Print,
}

/// How much of the notebook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whole {
    /// The piece on the band.
    Piece,
    /// Every piece in the notebook, oldest first, each on pages of its own.
    Note,
}

/// The folder in the notebook's that the files go to.
pub const FOLDER: &str = "export";

/// A PNG is this many times the page: 288 dpi.
const SCALE: f32 = 3.0;

/// How long the tools row says the files are being made, at most.
const BUSY: f64 = 60.0;

#[derive(Debug)]
pub struct Job {
    what: What,
    pages: Vec<String>,
    dir: PathBuf,
    /// The files' names without their numbers and extensions.
    name: String,
}

/// What came of a [`Job`].
#[derive(Debug)]
pub enum Done {
    Wrote {
        what: What,
        files: Vec<PathBuf>,
    },
    /// The PDF to print was opened in its viewer instead, which has no way
    /// to be told to print (Edge and Chrome): the person prints from there.
    Opened(PathBuf),
    Failed(String),
}

/// The thread that makes the files, in the order asked for.
pub struct Worker {
    tx: Option<Sender<Job>>,
    rx: Receiver<Done>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(wake: impl Fn() + Send + 'static) -> Worker {
        let (tx, jobs) = channel::<Job>();
        let (back, rx) = channel();
        let thread = std::thread::Builder::new()
            .name("fude-export".into())
            .spawn(move || {
                let fonts = ito_export::Fonts::system();
                while let Ok(job) = jobs.recv() {
                    let done = match make(&job, &fonts).and_then(|files| hand(job.what, files)) {
                        Ok(done) => done,
                        Err(e) => Done::Failed(e),
                    };
                    let _ = back.send(done);
                    wake();
                }
            })
            .ok();
        Worker { tx: Some(tx), rx, thread }
    }

    fn send(&self, job: Job) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(job);
        }
    }

    /// What the thread has done since the last call.
    fn done(&self) -> Option<Done> {
        self.rx.try_recv().ok()
    }

    /// Wait for the files still being made (on the way out).
    pub fn finish(&mut self) {
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The files of `job`: `<name>.pdf`, or `<name>-01.png` and on (pages left
/// from a longer copy before are taken away).
fn make(job: &Job, fonts: &ito_export::Fonts) -> Result<Vec<PathBuf>, String> {
    std::fs::create_dir_all(&job.dir).map_err(|e| format!("{}: {e}", job.dir.display()))?;
    match job.what {
        What::Png => {
            let png = |n: usize| job.dir.join(format!("{}-{n:02}.png", job.name));
            let mut files = Vec::new();
            for (n, page) in job.pages.iter().enumerate() {
                let out = png(n + 1);
                ito_export::png(page, SCALE, fonts, &out)?;
                files.push(out);
            }
            for n in job.pages.len() + 1.. {
                if std::fs::remove_file(png(n)).is_err() {
                    break;
                }
            }
            Ok(files)
        }
        What::Pdf | What::Print => {
            let out = job.dir.join(format!("{}.pdf", job.name));
            ito_export::pdf(&job.pages, fonts, &out)?;
            Ok(vec![out])
        }
    }
}

/// What came of `files`, made as `what`: a print's PDF goes to the printer.
fn hand(what: What, files: Vec<PathBuf>) -> Result<Done, String> {
    match files.first() {
        Some(pdf) if what == What::Print => match ito_export::print(pdf)? {
            ito_export::Printed::Sent => Ok(Done::Wrote { what, files }),
            ito_export::Printed::Opened => Ok(Done::Opened(pdf.clone())),
        },
        _ => Ok(Done::Wrote { what, files }),
    }
}

/// `export\<file>`, as the tools row names it.
fn shown(file: &Path) -> String {
    Path::new(FOLDER).join(file.file_name().unwrap_or_default()).display().to_string()
}

impl App {
    /// The clean copy of the piece or of its notebook out as `what`, on the
    /// paper the clean copy is on (or would open on). A script's print is
    /// only the PDF.
    pub fn export(&mut self, ctx: &egui::Context, what: What, whole: Whole) {
        let time = ctx.input(|i| i.time);
        self.bind_piece();
        let config = self.prefs.config;
        let i = self.ensure_note();
        let note = &self.notes[i];
        let texts: Vec<String> = match whole {
            Whole::Piece => vec![self.band.text()],
            Whole::Note => note.pieces.iter().map(|p| p.text.clone()).collect(),
        };
        let sheet = self.clean.sheet_of(&config, &texts.join("\n"), self.screen == Screen::Clean);
        let pages: Vec<String> = texts.iter().flat_map(|t| svg::pages(ctx, &sheet, &config, t)).collect();
        let name = match (whole, &self.piece) {
            (Whole::Piece, Some(stem)) => stem.clone(),
            _ => note.name.clone(),
        };
        let dir = note.dir.join(FOLDER);
        let t = &self.table;
        if pages.is_empty() {
            let words = t.tr("export.empty").to_owned();
            return self.tell(Kind::Info, words, time);
        }
        if self.root.is_none() {
            let words = t.tr("export.no_folder").to_owned();
            return self.tell(Kind::Warn, words, time);
        }
        let what = if what == What::Print && self.script.is_some() { What::Pdf } else { what };
        let busy = t.tr(if what == What::Print { "export.printing" } else { "export.busy" }).to_owned();
        if self.screen == Screen::Clean {
            self.clean.said = Some((Kind::Info, busy, time + BUSY));
        }
        let worker = self.export.get_or_insert_with(|| {
            let ctx = ctx.clone();
            Worker::start(move || ctx.request_repaint())
        });
        worker.send(Job { what, pages, dir, name });
    }

    /// What the export thread made since the last frame. The folder opens
    /// on the files written (not for a script, nor what went to print).
    pub fn follow_exports(&mut self, ctx: &egui::Context) {
        let time = ctx.input(|i| i.time);
        while let Some(done) = self.export.as_ref().and_then(Worker::done) {
            let t = &self.table;
            match done {
                Done::Wrote { what, files } => {
                    let Some(first) = files.first() else { continue };
                    let words = match what {
                        What::Pdf => t.trf("export.pdf", &[&shown(first)]),
                        What::Png => t.trf("export.png", &[&files.len().to_string(), &shown(first)]),
                        What::Print => t.trf("export.printed", &[&shown(first)]),
                    };
                    let opened = match first.parent() {
                        Some(dir) if what != What::Print && self.script.is_none() => crate::prefs::open_path(dir),
                        _ => Ok(()),
                    };
                    self.tell(Kind::Ok, words, time);
                    if let Err(e) = opened {
                        self.say(Kind::Warn, &e);
                    }
                }
                Done::Opened(pdf) => {
                    let words = t.trf("export.opened", &[&shown(&pdf)]);
                    self.tell(Kind::Info, words, time);
                }
                Done::Failed(e) => {
                    let words = t.trf("export.failed", &[&e]);
                    self.say(Kind::Err, &words);
                    if self.screen == Screen::Clean {
                        self.clean.say(Kind::Err, words, time);
                    }
                }
            }
        }
    }

    /// The palette on the ways out (Ctrl+E) or to the printer (Ctrl+P).
    pub fn choose_export(&mut self, print: bool) {
        let mut view = ito_palette::View::new();
        view.query = self.table.tr(if print { "palette.print_query" } else { "palette.export_query" }).to_owned();
        self.palette = Some(view);
    }

    /// The tools row says it on the clean copy; elsewhere, a notice does.
    fn tell(&mut self, kind: Kind, words: String, time: f64) {
        if self.screen == Screen::Clean {
            self.clean.say(kind, words, time);
        } else {
            self.say(kind, &words);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(what: What, pages: usize, dir: &Path) -> Job {
        let page =
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40"><rect width="80" height="40" fill="#ffffff"/></svg>"##;
        Job { what, pages: vec![page.to_owned(); pages], dir: dir.to_owned(), name: "2026-10-27_0930".into() }
    }

    #[test]
    fn the_files_are_made_in_the_folder_and_old_pages_go() {
        let dir = std::env::temp_dir().join(format!("fude-export-{}", std::process::id())).join(FOLDER);
        let fonts = ito_export::Fonts::empty();
        let files = make(&job(What::Png, 3, &dir), &fonts).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["2026-10-27_0930-01.png", "2026-10-27_0930-02.png", "2026-10-27_0930-03.png"]);
        assert!(files.iter().all(|f| f.is_file()));
        make(&job(What::Png, 1, &dir), &fonts).unwrap();
        assert!(dir.join("2026-10-27_0930-01.png").is_file() && !dir.join("2026-10-27_0930-02.png").exists());
        let pdf = make(&job(What::Pdf, 2, &dir), &fonts).unwrap();
        assert_eq!(pdf, [dir.join("2026-10-27_0930.pdf")]);
        assert!(std::fs::read(&pdf[0]).unwrap().starts_with(b"%PDF"));
        assert_eq!(shown(&pdf[0]), Path::new("export").join("2026-10-27_0930.pdf").display().to_string());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn only_a_print_goes_to_the_printer() {
        let missing = std::env::temp_dir().join(format!("fude-export-missing-{}.pdf", std::process::id()));
        assert!(matches!(hand(What::Pdf, vec![missing.clone()]), Ok(Done::Wrote { what: What::Pdf, .. })));
        assert!(matches!(hand(What::Print, Vec::new()), Ok(Done::Wrote { what: What::Print, .. })));
        assert!(hand(What::Print, vec![missing]).unwrap_err().contains("no such file"));
    }

    #[test]
    fn no_pages_is_no_file() {
        let dir = std::env::temp_dir().join(format!("fude-export-none-{}", std::process::id()));
        assert!(make(&job(What::Pdf, 0, &dir), &ito_export::Fonts::empty()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
