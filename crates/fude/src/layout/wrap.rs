//! A sheet that is not a grid (ruled, graph, plain) filled: lines broken
//! where the paper runs out, Latin words kept whole and broken at their
//! spaces, no line opened with a stop or closed with an opening bracket.
//! What the mock leaves to the browser (`renderWrap`).

use std::collections::VecDeque;

use super::{indented, Doc, Letter, Token};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A letter as it is set: upright down the page.
    Char,
    /// A space between words: where a line breaks.
    Space,
    /// Latin letters kept together; on their side down the page.
    Word,
    /// One or two digits across the column (縦中横).
    Tcy,
    /// A Latin letter or a digit upright in a square (graph paper, down the
    /// page).
    Upright,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub kind: Kind,
    pub t: Token,
    /// How far it runs along the line, in em.
    pub w: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    /// The indent before the first piece, in em.
    pub lead: f32,
    pub pieces: Vec<Piece>,
}

#[derive(Clone, Copy, Debug)]
pub struct Opt {
    pub vertical: bool,
    /// Graph paper: down the page, Latin letters stand in the squares.
    pub graph: bool,
    pub indent: bool,
}

fn alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

/// What a Latin word is made of.
fn wordy(c: char) -> bool {
    c != ' ' && (c.is_ascii_graphic() || ('\u{A0}'..='\u{24F}').contains(&c))
}

/// What never opens a line.
pub fn no_start(c: char) -> bool {
    matches!(
        c,
        '、' | '。'
            | '，'
            | '．'
            | '・'
            | '：'
            | '；'
            | '！'
            | '？'
            | '」'
            | '』'
            | '）'
            | '〕'
            | '】'
            | '〉'
            | '》'
            | 'ー'
            | '.'
            | ','
            | ':'
            | ';'
            | '!'
            | '?'
            | ')'
            | ']'
            | '}'
    ) || crate::vertical::small_kana(c)
}

/// What never closes a line.
pub fn no_end(c: char) -> bool {
    matches!(c, '「' | '『' | '（' | '〔' | '【' | '〈' | '《' | '(' | '[' | '{')
}

fn first(p: &Piece) -> char {
    p.t.s.chars().next().unwrap_or(' ')
}

/// A paragraph's letters as what a line is filled with; `adv` is how far a
/// run of letters set across goes, in em.
pub fn pieces(letters: &[Letter], o: &Opt, adv: &dyn Fn(&str) -> f32) -> Vec<Piece> {
    let same = |a: &Letter, b: &Letter| a.dot == b.dot && a.ruby == b.ruby;
    let upright = |c: char| o.vertical && o.graph && alnum(c);
    // Down the page: how many digits from `i` go across the column, one or
    // two with no Latin letter or digit by them.
    let tcy = |i: usize| -> usize {
        if !o.vertical || !letters[i].c.is_ascii_digit() {
            return 0;
        }
        let n = letters[i..].iter().take_while(|l| l.c.is_ascii_digit() && same(l, &letters[i])).count();
        let before = i > 0 && alnum(letters[i - 1].c);
        let after = letters.get(i + n).is_some_and(|l| alnum(l.c));
        if n <= 2 && !before && !after {
            n
        } else {
            0
        }
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < letters.len() {
        let l = &letters[i];
        let mut t = Token::of(l);
        let n = tcy(i);
        let (kind, w) = if l.c == ' ' {
            (Kind::Space, adv(" "))
        } else if n > 0 {
            letters[i + 1..i + n].iter().for_each(|d| t.push(d));
            (Kind::Tcy, 1.0)
        } else if upright(l.c) {
            (Kind::Upright, 1.0)
        } else if wordy(l.c) {
            let more =
                letters[i + 1..].iter().enumerate().take_while(|&(k, d)| wordy(d.c) && same(d, l) && tcy(i + 1 + k) == 0 && !upright(d.c));
            more.for_each(|(_, d)| t.push(d));
            (Kind::Word, adv(&t.s))
        } else {
            (Kind::Char, if o.vertical { 1.0 } else { adv(&t.s) })
        };
        i += t.s.chars().count();
        out.push(Piece { kind, t, w });
    }
    out
}

/// A word too long for a line, a piece a letter.
fn split(p: &Piece, adv: &dyn Fn(&str) -> f32) -> Vec<Piece> {
    let t = &p.t;
    let one = |(k, c): (usize, char)| {
        let s = c.to_string();
        let w = adv(&s);
        Piece { kind: p.kind, t: Token { s, at: t.at + k, end: t.at + k + 1, dot: t.dot, ruby: t.ruby }, w }
    };
    t.s.chars().enumerate().map(one).collect()
}

/// A paragraph's pieces broken into lines of `room` em, the first opened by
/// `lead`.
pub fn rows(pieces: Vec<Piece>, room: f32, lead: f32, adv: &dyn Fn(&str) -> f32) -> Vec<Row> {
    let mut out: Vec<Row> = Vec::new();
    let mut cur = Row { lead, pieces: Vec::new() };
    let mut used = lead;
    let mut queue: VecDeque<Piece> = pieces.into();
    while let Some(p) = queue.pop_front() {
        // A space a line breaks at is not carried to the next.
        if cur.pieces.is_empty() && p.kind == Kind::Space && !out.is_empty() {
            continue;
        }
        if used + p.w <= room + 0.001 {
            used += p.w;
            cur.pieces.push(p);
            continue;
        }
        if cur.pieces.is_empty() {
            if p.kind == Kind::Word && p.t.s.chars().nth(1).is_some() {
                split(&p, adv).into_iter().rev().for_each(|q| queue.push_front(q));
                continue;
            }
            // Too long for a line even alone.
            cur.pieces.push(p);
        } else {
            if p.kind != Kind::Space {
                queue.push_front(p);
            }
            // A stop goes down with what it follows; an opening bracket goes
            // down with what it opens.
            while cur.pieces.len() > 1 && queue.front().is_some_and(|q| no_start(first(q))) {
                queue.push_front(cur.pieces.pop().unwrap());
            }
            while cur.pieces.len() > 1 && cur.pieces.last().is_some_and(|q| no_end(first(q))) {
                queue.push_front(cur.pieces.pop().unwrap());
            }
            while cur.pieces.last().is_some_and(|q| q.kind == Kind::Space) {
                cur.pieces.pop();
            }
        }
        out.push(std::mem::take(&mut cur));
        used = 0.0;
    }
    if !cur.pieces.is_empty() || out.is_empty() {
        out.push(cur);
    }
    out
}

/// Each paragraph's lines.
pub fn layout(doc: &Doc, o: &Opt, room: f32, adv: &dyn Fn(&str) -> f32) -> Vec<Vec<Row>> {
    let lines = |p: &Vec<Letter>| {
        let lead = if indented(p, o.indent) { 1.0 } else { 0.0 };
        rows(pieces(p, o, adv), room.max(1.0), lead, adv)
    };
    doc.paras.iter().map(lines).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Latin letter half an em, anything else one.
    fn adv(s: &str) -> f32 {
        s.chars().map(|c| if c.is_ascii() { 0.5 } else { 1.0 }).sum()
    }

    fn across(indent: bool) -> Opt {
        Opt { vertical: false, graph: false, indent }
    }

    /// Each line's pieces joined, `_` before an indented one.
    fn shown(text: &str, o: &Opt, room: f32) -> Vec<String> {
        let rows = layout(&Doc::parse(text), o, room, &adv);
        let line = |r: &Row| {
            let s: String = r.pieces.iter().map(|p| p.t.s.as_str()).collect();
            if r.lead > 0.0 {
                format!("_{s}")
            } else {
                s
            }
        };
        rows.iter().flatten().map(line).collect()
    }

    fn kinds(text: &str, o: &Opt) -> Vec<(String, Kind)> {
        let doc = Doc::parse(text);
        pieces(&doc.paras[0], o, &adv).into_iter().map(|p| (p.t.s, p.kind)).collect()
    }

    #[test]
    fn latin_words_break_at_their_spaces_or_letter_by_letter_when_too_long() {
        assert_eq!(shown("This is a pen.", &across(true), 4.0), ["This is", "a pen."]);
        assert_eq!(shown("abcdefghij", &across(true), 2.0), ["abcd", "efgh", "ij"]);
        assert_eq!(shown("ab  cd", &across(true), 1.0), ["ab", "cd"]);
        // The pieces of a split word keep their place in the text.
        let rows = layout(&Doc::parse("abcdefghij"), &across(true), 2.0, &adv);
        assert_eq!((rows[0][1].pieces[0].t.at, rows[0][1].pieces[0].t.s.as_str()), (4, "e"));
    }

    #[test]
    fn no_line_opens_with_a_stop_or_closes_with_an_opening_bracket() {
        assert_eq!(shown("あいうえお。", &across(false), 5.0), ["あいうえ", "お。"]);
        assert_eq!(shown("あいうえおっ", &across(false), 5.0), ["あいうえ", "おっ"]);
        assert_eq!(shown("あいう「え」", &across(false), 4.0), ["あいう", "「え」"]);
        assert_eq!(shown("あいうえお\n「か」", &across(true), 3.0), ["_あい", "うえお", "「か」"]);
    }

    #[test]
    fn down_the_page_digits_go_across_and_latin_lies_or_stands_on_graph_paper() {
        use Kind::*;
        let down = Opt { vertical: true, graph: false, indent: false };
        let k = |s: &str, kind| (s.to_string(), kind);
        assert_eq!(
            kinds("12時とPC、123(12)", &down),
            [k("12", Tcy), k("時", Char), k("と", Char), k("PC", Word), k("、", Char), k("123(", Word), k("12", Tcy), k(")", Word)]
        );
        let graph = Opt { graph: true, ..down };
        assert_eq!(
            kinds("PC 123", &graph),
            [k("P", Upright), k("C", Upright), k(" ", Space), k("1", Upright), k("2", Upright), k("3", Upright)]
        );
        assert_eq!(kinds("PC", &across(false)), [k("PC", Word)]);
    }
}
