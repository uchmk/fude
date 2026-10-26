//! The clean copy's text, without egui: the band's text read for its marks
//! (ruby and emphasis), cut into what the paper is filled with, and the
//! paragraphs as they are copied out. The mock's `tokenize`, `indented` and
//! `plainText` (`fude-mock/index.html`); the marks are the usual ones of
//! novels written on the web.
//!
//! - `|漢字《かんじ》` (or `｜`): ruby over the letters between the bar and
//!   the brackets.
//! - `漢字《かんじ》`: ruby over the run of kanji just before the brackets.
//! - `《《強調》》`: a dot by each letter.
//!
//! A mark that does not close is a letter like any other.

pub mod genko;
pub mod wrap;

/// One letter of the clean copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Letter {
    pub c: char,
    /// Where it is in the band's text (a letter's index, the paragraph
    /// breaks counted).
    pub at: usize,
    /// Under emphasis: a dot by it.
    pub dot: bool,
    /// The ruby over it, an index into [`Doc::rubies`].
    pub ruby: Option<usize>,
}

/// Ruby, and how many letters it stands over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ruby {
    pub text: String,
    pub base: usize,
}

/// The band's text, read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doc {
    /// The paragraphs with letters in them.
    pub paras: Vec<Vec<Letter>>,
    pub rubies: Vec<Ruby>,
}

/// What goes into one cell of the paper, or one piece of a line: a letter,
/// two of them sharing a cell (`12`, `。」`), or none (the indent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub s: String,
    /// The text's index of its first letter, and of the one after its last.
    pub at: usize,
    pub end: usize,
    pub dot: bool,
    pub ruby: Option<usize>,
}

impl Token {
    fn of(l: &Letter) -> Token {
        Token { s: l.c.to_string(), at: l.at, end: l.at + 1, dot: l.dot, ruby: l.ruby }
    }

    /// The empty cell a paragraph opens with.
    pub fn indent(at: usize) -> Token {
        Token { s: String::new(), at, end: at, dot: false, ruby: None }
    }

    fn push(&mut self, l: &Letter) {
        self.s.push(l.c);
        self.end = l.at + 1;
    }
}

/// What opens: never the last cell of a line, and no indent before it.
pub fn open(c: char) -> bool {
    matches!(c, '「' | '『' | '（' | '〔' | '【')
}

/// What may hang past the line's end rather than open the next line.
pub fn hang(c: char) -> bool {
    matches!(c, '、' | '。' | '，' | '．' | '」' | '』' | '）' | '〕' | '】')
}

/// What ruby with no bar reaches back over.
pub fn kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}' | '\u{20000}'..='\u{2FFFF}' | '々' | '〆' | 'ヶ' | '〇')
}

/// Mostly Latin letters (by more than half of what is not a space): the clean
/// copy opens on a ruled sheet, not on a manuscript grid.
pub fn is_latin(text: &str) -> bool {
    let all = text.chars().filter(|c| !c.is_whitespace()).count();
    let latin = text.chars().filter(char::is_ascii_alphabetic).count();
    all > 0 && latin * 2 > all
}

/// Letters, the paragraph breaks not counted.
pub fn count(text: &str) -> usize {
    text.chars().filter(|&c| c != '\n').count()
}

/// Whether a paragraph opens a letter in: with `indent` on, unless it opens
/// with a bracket, a Latin letter or a digit.
pub fn indented(letters: &[Letter], indent: bool) -> bool {
    letters.first().is_some_and(|l| indent && !open(l.c) && !l.c.is_ascii_alphanumeric())
}

/// The text as it is copied: a paragraph a line, each indented one with a
/// full-width space, the marks kept.
pub fn plain_text(text: &str, indent: bool) -> String {
    let mut out = String::new();
    for p in text.split('\n').filter(|p| !p.is_empty()) {
        let letters = parse_para(&p.chars().collect::<Vec<_>>(), 0, &mut Vec::new());
        if indented(&letters, indent) {
            out.push('　');
        }
        out.push_str(p);
        out.push('\n');
    }
    out
}

impl Doc {
    pub fn parse(text: &str) -> Doc {
        let mut doc = Doc::default();
        let chars: Vec<char> = text.chars().collect();
        let mut from = 0;
        for p in chars.split(|&c| c == '\n') {
            let letters = parse_para(p, from, &mut doc.rubies);
            if !letters.is_empty() {
                doc.paras.push(letters);
            }
            from += p.len() + 1;
        }
        doc
    }
}

/// Where `what` is next in `p`, from `i`.
fn find(p: &[char], i: usize, what: &[char]) -> Option<usize> {
    (i..p.len().saturating_sub(what.len() - 1)).find(|&j| p[j..].starts_with(what))
}

/// One paragraph's letters (`p` starts at `from` in the text); its ruby goes
/// onto `rubies`.
fn parse_para(p: &[char], from: usize, rubies: &mut Vec<Ruby>) -> Vec<Letter> {
    let mut out: Vec<Letter> = Vec::new();
    let mut i = 0;
    // Where the emphasis that is open closes.
    let mut close = None;
    let mut ruby = |out: &mut Vec<Letter>, start: usize, text: &[char]| {
        let base = out.len() - start;
        for l in &mut out[start..] {
            l.ruby = Some(rubies.len());
        }
        rubies.push(Ruby { text: text.iter().collect(), base });
    };
    while i < p.len() {
        if close == Some(i) {
            close = None;
            i += 2;
            continue;
        }
        if close.is_none() && p[i..].starts_with(&['《', '《']) {
            if let Some(j) = find(p, i + 2, &['》', '》']).filter(|&j| j > i + 2) {
                close = Some(j);
                i += 2;
                continue;
            }
        }
        if close.is_none() && matches!(p[i], '|' | '｜') {
            // The base runs to the brackets, and the ruby in them is not empty.
            let base = p[i + 1..].iter().position(|&c| matches!(c, '|' | '｜' | '《' | '》')).map(|n| i + 1 + n);
            if let Some(b) = base.filter(|&b| b > i + 1 && p[b] == '《') {
                if let Some(e) =
                    p[b + 1..].iter().position(|&c| matches!(c, '《' | '》')).map(|n| b + 1 + n).filter(|&e| e > b + 1 && p[e] == '》')
                {
                    let start = out.len();
                    out.extend((i + 1..b).map(|k| Letter { c: p[k], at: from + k, dot: false, ruby: None }));
                    ruby(&mut out, start, &p[b + 1..e]);
                    i = e + 1;
                    continue;
                }
            }
        }
        if close.is_none() && p[i] == '《' {
            let end = p[i + 1..].iter().position(|&c| matches!(c, '《' | '》')).map(|n| i + 1 + n);
            if let Some(e) = end.filter(|&e| e > i + 1 && p[e] == '》') {
                let start = out.len() - out.iter().rev().take_while(|l| kanji(l.c) && l.ruby.is_none() && !l.dot).count();
                if start < out.len() {
                    ruby(&mut out, start, &p[i + 1..e]);
                    i = e + 1;
                    continue;
                }
            }
        }
        out.push(Letter { c: p[i], at: from + i, dot: close.is_some(), ruby: None });
        i += 1;
    }
    out
}

/// A paragraph's letters as cells: `。」` share one when `merge`, and two of
/// what `pair` takes (digits, or Latin letters too across the page) share
/// one; letters with different marks never do.
pub fn tokens(letters: &[Letter], merge: bool, pair: impl Fn(char) -> bool) -> Vec<Token> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < letters.len() {
        let l = &letters[i];
        let mut t = Token::of(l);
        if let Some(d) = letters.get(i + 1).filter(|d| d.dot == l.dot && d.ruby == l.ruby) {
            let stop = merge && matches!(l.c, '。' | '、') && matches!(d.c, '」' | '』' | '）');
            if stop || (pair(l.c) && pair(d.c)) {
                t.push(d);
                i += 1;
            }
        }
        out.push(t);
        i += 1;
    }
    out
}

/// The part of a ruby over `n` of its letters, from its `from`th: shared out
/// by the letters, so a ruby split over two lines splits where they do.
pub fn ruby_part(ruby: &Ruby, from: usize, n: usize) -> String {
    let chars: Vec<char> = ruby.text.chars().collect();
    let base = ruby.base.max(1);
    let at = |k: usize| (chars.len() * k.min(base) + base / 2) / base;
    chars[at(from)..at(from + n)].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The paragraphs' letters, each `<` ruby-index `>` or `*` (a dot) after
    /// the letter.
    fn shown(doc: &Doc) -> Vec<String> {
        doc.paras
            .iter()
            .map(|p| {
                p.iter()
                    .map(|l| {
                        let mut s = l.c.to_string();
                        if let Some(r) = l.ruby {
                            s += &format!("<{r}>");
                        }
                        if l.dot {
                            s.push('*');
                        }
                        s
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn ruby_and_emphasis_are_read_and_what_does_not_close_is_left() {
        let doc = Doc::parse("昔々、|東京《とうきょう》の漢字《かんじ》と《《強く》》言う。\n\n残《》る「《ルビ》」と|x《》");
        assert_eq!(shown(&doc), ["昔々、東<0>京<0>の漢<1>字<1>と強*く*言う。", "残《》る「《ルビ》」と|x《》"]);
        assert_eq!(doc.rubies, [Ruby { text: "とうきょう".into(), base: 2 }, Ruby { text: "かんじ".into(), base: 2 }]);
        // Where each letter is in the text: the marks and the break counted.
        assert_eq!(doc.paras[0][3].at, 4);
        assert_eq!(doc.paras[1][0].at, "昔々、|東京《とうきょう》の漢字《かんじ》と《《強く》》言う。\n\n".chars().count());
        // The run of kanji stops at what has ruby already, and at a kana.
        let doc = Doc::parse("|東京《とうきょう》都庁《とちょう》ひら仮名《かな》");
        assert_eq!(shown(&doc), ["東<0>京<0>都<1>庁<1>ひら仮<2>名<2>"]);
        assert_eq!(Doc::parse("《《閉じない").paras[0].len(), 6);
        assert!(Doc::parse("\n\n").paras.is_empty());
    }

    #[test]
    fn letters_pair_in_a_cell_when_their_marks_agree() {
        let doc = Doc::parse("「はい。」と12時にPCで|AB《エービー》C");
        let digits = |c: char| c.is_ascii_digit();
        let ss = |t: Vec<Token>| t.into_iter().map(|t| t.s).collect::<Vec<_>>();
        assert_eq!(
            ss(tokens(&doc.paras[0], true, digits)),
            ["「", "は", "い", "。」", "と", "12", "時", "に", "P", "C", "で", "A", "B", "C"]
        );
        assert_eq!(ss(tokens(&doc.paras[0], false, |c: char| c.is_ascii_alphanumeric()))[2..5], ["い", "。", "」"]);
        let across = tokens(&doc.paras[0], true, |c: char| c.is_ascii_alphanumeric());
        assert_eq!(ss(across.clone())[8..], ["PC", "で", "AB", "C"]);
        // A pair's place in the text runs over both letters.
        let pc = &across[8];
        assert_eq!((pc.at, pc.end), (10, 12));
    }

    #[test]
    fn the_copy_indents_paragraphs_and_keeps_the_marks() {
        let text = "書いた。\n「台詞」\nPCの話。\n\n|漢《かん》";
        assert_eq!(plain_text(text, true), "　書いた。\n「台詞」\nPCの話。\n　|漢《かん》\n");
        assert_eq!(plain_text(text, false), "書いた。\n「台詞」\nPCの話。\n|漢《かん》\n");
        assert_eq!(count(text), 19);
        assert!(is_latin("This is a pen. ペン") && !is_latin("ペンです。pen") && !is_latin(""));
    }

    #[test]
    fn ruby_split_over_lines_is_shared_out_by_its_letters() {
        let r = Ruby { text: "とうきょう".into(), base: 2 };
        assert_eq!((ruby_part(&r, 0, 1), ruby_part(&r, 1, 1)), ("とうき".into(), "ょう".into()));
        assert_eq!(ruby_part(&r, 0, 2), "とうきょう");
    }
}
