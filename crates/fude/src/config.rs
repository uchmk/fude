//! fude's own settings, config.toml in fude's folder (`app_dir`): `[write]`
//! is how the band looks and counts, `[note]` the limits and the no-erase
//! mode (the defaults of a notebook's own fude.toml, once there are
//! notebooks). A value that is not understood is said and left as it was.

use std::path::{Path, PathBuf};

use crate::band::Limits;

/// An enum written as one of a few words.
macro_rules! words {
    ($(#[$m:meta])* $t:ident { $first:ident = $fw:literal $(, $v:ident = $w:literal)* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub enum $t {
            #[default]
            $first,
            $($v),*
        }

        impl $t {
            pub const ALL: &[($t, &str)] = &[($t::$first, $fw) $(, ($t::$v, $w))*];

            pub fn word(self) -> &'static str {
                Self::ALL.iter().find(|(v, _)| *v == self).map_or("", |(_, w)| w)
            }

            fn parse(w: &str) -> Option<Self> {
                Self::ALL.iter().find(|(_, x)| *x == w).map(|(v, _)| *v)
            }
        }
    };
}

words! {
    /// Which way the band runs.
    Dir { Horizontal = "horizontal", Vertical = "vertical" }
}

words! {
    /// Where the input point stands along the band: the middle, nearer its
    /// start (left, or the top of a vertical band) or nearer its end.
    Point { Center = "center", Start = "start", End = "end" }
}

words! {
    /// What the hint under the input point and the bars at the edge show.
    Show { Both = "both", Chars = "chars", Time = "time", Nothing = "none" }
}

words! {
    /// When the clock starts: at the first letter, or as the screen opens.
    TimeStart { First = "first", Open = "open" }
}

words! {
    /// Writing on past a limit, after the clean copy: the limit that was met
    /// grows by as much again, or there are no limits any more.
    OnContinue { Add = "add", Free = "free" }
}

words! {
    /// The paper of the clean copy: a manuscript grid, a ruled or a graph
    /// sheet, or a plain one; `Auto` is the grid, or ruled across the page
    /// for Latin text.
    Paper { Auto = "auto", Genko = "genko", Ruled = "ruled", Graph = "graph", Plain = "plain" }
}

words! {
    /// The colour the paper is ruled in: vermilion, green, blue or ink.
    Rule { Shu = "shu", Midori = "midori", Ao = "ao", Sumi = "sumi" }
}

words! {
    /// A stop that would open a line of the grid: in the last cell with what
    /// it follows, past the last cell, or at the line's head after all.
    Hang { In = "in", Out = "out", Off = "off" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub font_size: u32,
    pub dir: Dir,
    pub point: Point,
    pub show: Show,
    pub time_start: TimeStart,
    pub on_continue: OnContinue,
    /// Letters; 0 is none.
    pub char_limit: u32,
    /// Minutes; 0 is none.
    pub time_limit: u32,
    pub no_erase: bool,
    pub paper: Paper,
    /// Which way the clean copy is written.
    pub paper_dir: Dir,
    pub rule: Rule,
    /// The grid: letters to a line, lines to a sheet.
    pub cols: u32,
    pub rows: u32,
    /// A paragraph opens a letter in.
    pub indent: bool,
    pub hang: Hang,
    /// `。」` in one cell.
    pub merge: bool,
    /// Letters to a line of a sheet that is not a grid; 0 is as the window
    /// fits.
    pub wrap: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_size: 40,
            dir: Dir::default(),
            point: Point::default(),
            show: Show::default(),
            time_start: TimeStart::default(),
            on_continue: OnContinue::default(),
            char_limit: 0,
            time_limit: 0,
            no_erase: false,
            paper: Paper::default(),
            paper_dir: Dir::Vertical,
            rule: Rule::default(),
            cols: 20,
            rows: 20,
            indent: true,
            hang: Hang::default(),
            merge: true,
            wrap: 0,
        }
    }
}

pub const FONT_SIZE: (u32, u32) = (12, 120);
pub const CHAR_LIMIT: (u32, u32) = (0, 100_000);
pub const TIME_LIMIT: (u32, u32) = (0, 600);
pub const COLS: (u32, u32) = (5, 40);
pub const ROWS: (u32, u32) = (2, 40);
pub const WRAP: (u32, u32) = (0, 80);

/// config.toml in `base`'s fude folder.
pub fn path(base: &Path) -> PathBuf {
    ito_common::app_dir_in(base, "fude").join("config.toml")
}

/// The settings in the file at `path` (none is the defaults) and what in it
/// was not understood. Call it off the UI thread.
pub fn read(path: &Path) -> Result<(Config, Vec<String>), String> {
    let text = ito_common::read_or_empty(path)?;
    parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn parse(text: &str) -> Result<(Config, Vec<String>), String> {
    let doc: toml::Table = text.parse().map_err(|e: toml::de::Error| e.message().to_owned())?;
    let mut c = Config::default();
    let mut warn = Vec::new();
    for (name, item) in &doc {
        match (name.as_str(), item) {
            ("write" | "note", toml::Value::Table(t)) => {
                for (key, value) in t {
                    if let Err(e) = c.apply(name, key, value) {
                        warn.push(format!("config.toml: {name}.{key}: {e}"));
                    }
                }
            }
            _ => warn.push(format!("config.toml: `{name}` is not a setting of fude")),
        }
    }
    Ok((c, warn))
}

impl Config {
    pub fn limits(&self) -> Limits {
        Limits { chars: self.char_limit, minutes: self.time_limit }
    }

    /// `table.key = value`, as the file or the settings screen says it. A
    /// number out of its range is taken at the nearest end, and said.
    pub fn apply(&mut self, table: &str, key: &str, value: &toml::Value) -> Result<(), String> {
        fn number(v: &toml::Value, (lo, hi): (u32, u32), to: &mut u32) -> Result<(), String> {
            let n = v.as_integer().ok_or_else(|| format!("{v} is not a number"))?;
            *to = n.clamp(i64::from(lo), i64::from(hi)) as u32;
            if (i64::from(lo)..=i64::from(hi)).contains(&n) {
                Ok(())
            } else {
                Err(format!("{n} is not between {lo} and {hi}; {to} is used"))
            }
        }
        fn word<T: Copy>(v: &toml::Value, parse: fn(&str) -> Option<T>, all: &[(T, &str)], to: &mut T) -> Result<(), String> {
            match v.as_str().and_then(parse) {
                Some(w) => {
                    *to = w;
                    Ok(())
                }
                None => Err(format!("{v} is not one of {}", all.iter().map(|(_, w)| format!("\"{w}\"")).collect::<Vec<_>>().join(", "))),
            }
        }
        fn switch(v: &toml::Value, to: &mut bool) -> Result<(), String> {
            *to = v.as_bool().ok_or_else(|| format!("{v} is not true or false"))?;
            Ok(())
        }
        match (table, key) {
            ("write", "font_size") => number(value, FONT_SIZE, &mut self.font_size),
            ("write", "dir") => word(value, Dir::parse, Dir::ALL, &mut self.dir),
            ("write", "point") => word(value, Point::parse, Point::ALL, &mut self.point),
            ("write", "show") => word(value, Show::parse, Show::ALL, &mut self.show),
            ("write", "time_start") => word(value, TimeStart::parse, TimeStart::ALL, &mut self.time_start),
            ("write", "on_continue") => word(value, OnContinue::parse, OnContinue::ALL, &mut self.on_continue),
            ("note", "char_limit") => number(value, CHAR_LIMIT, &mut self.char_limit),
            ("note", "time_limit") => number(value, TIME_LIMIT, &mut self.time_limit),
            ("note", "no_erase") => switch(value, &mut self.no_erase),
            ("note", "paper") => word(value, Paper::parse, Paper::ALL, &mut self.paper),
            ("note", "dir") => word(value, Dir::parse, Dir::ALL, &mut self.paper_dir),
            ("note", "rule") => word(value, Rule::parse, Rule::ALL, &mut self.rule),
            ("note", "cols") => number(value, COLS, &mut self.cols),
            ("note", "rows") => number(value, ROWS, &mut self.rows),
            ("note", "indent") => switch(value, &mut self.indent),
            ("note", "hang") => word(value, Hang::parse, Hang::ALL, &mut self.hang),
            ("note", "merge") => switch(value, &mut self.merge),
            ("note", "wrap") => number(value, WRAP, &mut self.wrap),
            _ => Err("not a setting of fude".into()),
        }
    }
}

/// `value` as TOML text, for `set_key`.
pub fn toml_text(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => ito_common::quote(s),
        v => v.to_string(),
    }
}

/// Write `table.key = value` into the file at `path`, keeping the rest of it
/// (comments and all). Written only when that changes it; off the UI thread.
pub fn write(path: &Path, table: &str, key: &str, value: &toml::Value) -> Result<(), String> {
    let at = |e: String| format!("{}: {e}", path.display());
    let text = ito_common::read_or_empty(path).map_err(at)?;
    let new = ito_common::set_key(&text, Some(table), key, &toml_text(value)).map_err(at)?;
    if new != text {
        ito_common::write_atomic(path, &new).map_err(at)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_the_defaults() {
        assert_eq!(parse("").unwrap(), (Config::default(), vec![]));
        let c = Config::default();
        assert_eq!((c.font_size, c.paper, c.paper_dir, c.cols, c.rows), (40, Paper::Auto, Dir::Vertical, 20, 20));
        assert!(c.indent && c.merge);
    }

    #[test]
    fn the_file_is_read_and_what_is_wrong_is_said_and_left() {
        let text = r#"
[write]
font_size = 200
dir = "vertical"
point = "middle"
show = "chars"
time_start = "open"
on_continue = "free"
colour = "red"

[note]
char_limit = 400
time_limit = 15
no_erase = true
paper = "graph"
dir = "horizontal"
rule = "ao"
cols = 4
rows = 10
indent = false
hang = "out"
merge = "yes"
wrap = 30
"#;
        let (c, warn) = parse(text).unwrap();
        assert_eq!(c.font_size, 120);
        assert_eq!((c.dir, c.point, c.show), (Dir::Vertical, Point::Center, Show::Chars));
        assert_eq!((c.time_start, c.on_continue), (TimeStart::Open, OnContinue::Free));
        assert_eq!(c.limits(), Limits { chars: 400, minutes: 15 });
        assert!(c.no_erase);
        assert_eq!((c.paper, c.paper_dir, c.rule), (Paper::Graph, Dir::Horizontal, Rule::Ao));
        assert_eq!((c.cols, c.rows, c.wrap), (5, 10, 30));
        assert_eq!((c.indent, c.hang, c.merge), (false, Hang::Out, true));
        assert_eq!(warn.len(), 5, "{warn:?}");
        let said = |a: &str, b: &str| warn.iter().any(|w| w.contains(a) && w.contains(b));
        assert!(said("font_size", "120 is used") && said("point", "\"start\"") && said("colour", "not a setting"), "{warn:?}");
        assert!(parse("[write").is_err());
        assert_eq!(parse("size = 3").unwrap().1.len(), 1);
    }

    #[test]
    fn a_setting_is_written_into_the_file_and_read_back() {
        let dir = std::env::temp_dir().join(format!("fude-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = path(&dir);
        write(&path, "write", "dir", &toml::Value::String("vertical".into())).unwrap();
        write(&path, "note", "char_limit", &toml::Value::Integer(400)).unwrap();
        write(&path, "note", "no_erase", &toml::Value::Boolean(true)).unwrap();
        let (c, warn) = read(&path).unwrap();
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!((c.dir, c.char_limit, c.no_erase), (Dir::Vertical, 400, true));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
