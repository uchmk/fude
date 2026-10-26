//! The manuscript grid filled: each paragraph a run of lines of `cols` cells,
//! opened with an empty cell, a stop that would open a line hung past the
//! last cell, and no opening bracket left in it. The mock's `layoutGenko`.

use super::{hang, indented, open, tokens, Doc, Token};
use crate::config::Hang;

#[derive(Clone, Copy, Debug)]
pub struct Opt {
    pub cols: usize,
    pub indent: bool,
    pub hang: Hang,
    /// `。」` in one cell.
    pub merge: bool,
    /// Two digits share a cell down the page; across it, two Latin letters
    /// too.
    pub vertical: bool,
}

/// One line of the grid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    pub cells: Vec<Token>,
    /// The stop that hangs past the last cell.
    pub hang: Option<Token>,
}

fn first(t: &Token) -> char {
    t.s.chars().next().unwrap_or(' ')
}

pub fn lines(doc: &Doc, o: &Opt) -> Vec<Line> {
    let pair = |c: char| if o.vertical { c.is_ascii_digit() } else { c.is_ascii_alphanumeric() };
    let mut out = Vec::new();
    for p in &doc.paras {
        let mut cur = Line::default();
        if indented(p, o.indent) {
            cur.cells.push(Token::indent(p[0].at));
        }
        for t in tokens(p, o.merge, pair) {
            if cur.cells.len() >= o.cols {
                if o.hang != Hang::Off && hang(first(&t)) {
                    cur.hang = Some(t);
                    out.push(std::mem::take(&mut cur));
                    continue;
                }
                out.push(std::mem::take(&mut cur));
            }
            if o.cols > 1 && cur.cells.len() == o.cols - 1 && open(first(&t)) {
                out.push(std::mem::take(&mut cur));
            }
            cur.cells.push(t);
        }
        if !cur.cells.is_empty() {
            out.push(cur);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt(cols: usize) -> Opt {
        Opt { cols, indent: true, hang: Hang::In, merge: true, vertical: true }
    }

    /// Each line's cells joined, `_` the indent, and `+` before what hangs.
    fn shown(text: &str, o: &Opt) -> Vec<String> {
        lines(&Doc::parse(text), o)
            .into_iter()
            .map(|l| {
                let mut s: String = l.cells.iter().map(|t| if t.s.is_empty() { "_" } else { &t.s }).collect();
                if let Some(h) = l.hang {
                    s = format!("{s}+{}", h.s);
                }
                s
            })
            .collect()
    }

    #[test]
    fn a_sheet_of_twenty_by_twenty_takes_twenty_to_a_line() {
        let text = "あ".repeat(45);
        let l = lines(&Doc::parse(&text), &opt(20));
        assert_eq!(l.iter().map(|l| l.cells.len()).collect::<Vec<_>>(), [20, 20, 6]);
        assert_eq!((l[0].cells[0].s.as_str(), l[0].cells[1].at), ("", 0));
        // Empty paragraphs take no line; a bracket opens with no indent.
        assert_eq!(shown("あ\n\n「い」", &opt(20)), ["_あ", "「い」"]);
    }

    #[test]
    fn a_stop_hangs_past_the_line_or_opens_the_next() {
        let o = Opt { indent: false, ..opt(5) };
        assert_eq!(shown("あいうえお。かき", &o), ["あいうえお+。", "かき"]);
        assert_eq!(shown("あいうえお。」かき", &o), ["あいうえお+。」", "かき"]);
        assert_eq!(shown("あいうえお。」かき", &Opt { merge: false, ..o }), ["あいうえお+。", "」かき"]);
        assert_eq!(shown("あいうえお。かき", &Opt { hang: Hang::Out, ..o }), ["あいうえお+。", "かき"]);
        assert_eq!(shown("あいうえお。かき", &Opt { hang: Hang::Off, ..o }), ["あいうえお", "。かき"]);
    }

    #[test]
    fn an_opening_bracket_is_never_left_in_the_last_cell() {
        let o = Opt { indent: false, ..opt(5) };
        assert_eq!(shown("あいうえ「か」", &o), ["あいうえ", "「か」"]);
        assert_eq!(shown("「", &Opt { cols: 1, ..o }), ["「"]);
    }

    #[test]
    fn latin_letters_stand_one_to_a_cell_down_the_page_and_two_across() {
        let o = Opt { indent: false, ..opt(20) };
        assert_eq!(lines(&Doc::parse("PCと12時"), &o)[0].cells.len(), 5);
        assert_eq!(shown("PCと12時", &Opt { vertical: false, ..o }), ["PCと12時"]);
        assert_eq!(lines(&Doc::parse("PCと12時"), &Opt { vertical: false, ..o })[0].cells.len(), 4);
    }
}
