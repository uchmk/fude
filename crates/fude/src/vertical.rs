//! How a letter stands in a vertical line. egui sets type horizontally and
//! knows no `vert` feature, so the vertical forms are picked here: the
//! punctuation and brackets have their own code points (the presentation
//! forms the Mincho faces carry), Latin letters and dashes lie on their side,
//! and small kana move up to the right, as a browser sets `vertical-rl`.

/// How a letter is drawn in the column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Turn {
    /// As it is, one em down the column.
    Upright,
    /// A quarter turn clockwise; it takes its own width down the column.
    Quarter,
    /// Upright, moved by this much of the size to the right and down.
    Shift(f32, f32),
}

/// The vertical presentation form of `c`, when Unicode has one.
fn form(c: char) -> Option<char> {
    Some(match c {
        '，' => '\u{FE10}',
        '、' => '\u{FE11}',
        '。' => '\u{FE12}',
        '：' => '\u{FE13}',
        '；' => '\u{FE14}',
        '！' => '\u{FE15}',
        '？' => '\u{FE16}',
        '〖' => '\u{FE17}',
        '〗' => '\u{FE18}',
        '…' => '\u{FE19}',
        '‥' => '\u{FE30}',
        '—' => '\u{FE31}',
        '–' => '\u{FE32}',
        '＿' => '\u{FE33}',
        '（' => '\u{FE35}',
        '）' => '\u{FE36}',
        '｛' => '\u{FE37}',
        '｝' => '\u{FE38}',
        '〔' => '\u{FE39}',
        '〕' => '\u{FE3A}',
        '【' => '\u{FE3B}',
        '】' => '\u{FE3C}',
        '《' => '\u{FE3D}',
        '》' => '\u{FE3E}',
        '〈' => '\u{FE3F}',
        '〉' => '\u{FE40}',
        '「' => '\u{FE41}',
        '」' => '\u{FE42}',
        '『' => '\u{FE43}',
        '』' => '\u{FE44}',
        '［' => '\u{FE47}',
        '］' => '\u{FE48}',
        _ => return None,
    })
}

fn small_kana(c: char) -> bool {
    "ぁぃぅぇぉっゃゅょゎゕゖァィゥェォッャュョヮヵヶ".contains(c)
}

/// Lies on its side in a vertical line: Latin and the other narrow scripts,
/// the half-width forms, and the long marks that run along the line.
fn sideways(c: char) -> bool {
    let narrow = (c as u32) < 0x1100 && c != '\u{a7}' && c != '\u{b6}' && c != '\u{d7}' && c != '\u{f7}';
    let half = ('\u{FF61}'..='\u{FFDC}').contains(&c) || ('\u{FFE8}'..='\u{FFEE}').contains(&c);
    narrow || half || matches!(c, 'ー' | '〜' | '～' | '―' | '－' | '…' | '‥' | '—' | '–' | '＿' | '→' | '←' | '＝' | '〰')
}

/// What to draw for `c` in a vertical line and how; `has` says whether the
/// face has a letter, so a missing form falls back on a turned or moved
/// letter instead of a box.
pub fn glyph(c: char, has: impl Fn(char) -> bool) -> (char, Turn) {
    if let Some(f) = form(c).filter(|&f| has(f)) {
        return (f, Turn::Upright);
    }
    if matches!(c, '、' | '。' | '，' | '．') {
        // Set at the bottom left of their square; vertical wants the top right.
        return (c, Turn::Shift(0.6, -0.6));
    }
    if small_kana(c) {
        return (c, Turn::Shift(0.1, -0.12));
    }
    if form(c).is_some() && !matches!(c, '！' | '？' | '：' | '；') {
        // A bracket or a dash the face has no vertical form of.
        return (c, Turn::Quarter);
    }
    if sideways(c) {
        return (c, Turn::Quarter);
    }
    (c, Turn::Upright)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(_: char) -> bool {
        true
    }

    fn none(_: char) -> bool {
        false
    }

    #[test]
    fn punctuation_and_brackets_take_their_vertical_forms() {
        assert_eq!(glyph('、', all), ('\u{FE11}', Turn::Upright));
        assert_eq!(glyph('。', all), ('\u{FE12}', Turn::Upright));
        assert_eq!(glyph('「', all), ('\u{FE41}', Turn::Upright));
        assert_eq!(glyph('』', all), ('\u{FE44}', Turn::Upright));
        assert_eq!(glyph('（', all), ('\u{FE35}', Turn::Upright));
        assert_eq!(glyph('…', all), ('\u{FE19}', Turn::Upright));
    }

    #[test]
    fn a_face_without_the_forms_turns_or_moves_the_letter() {
        assert_eq!(glyph('、', none), ('、', Turn::Shift(0.6, -0.6)));
        assert_eq!(glyph('「', none), ('「', Turn::Quarter));
        assert_eq!(glyph('…', none), ('…', Turn::Quarter));
        assert_eq!(glyph('！', none), ('！', Turn::Upright));
    }

    #[test]
    fn latin_and_long_marks_lie_on_their_side() {
        for c in ['P', 'c', '1', ' ', 'é', 'ｱ', 'ー', '〜', '―'] {
            assert_eq!(glyph(c, all), (c, Turn::Quarter), "{c:?}");
        }
        // Full-width letters and kanji stand up.
        for c in ['Ｐ', '１', '漢', 'あ', 'ア', '¶'] {
            assert_eq!(glyph(c, all), (c, Turn::Upright), "{c:?}");
        }
    }

    #[test]
    fn small_kana_move_up_to_the_right() {
        for c in ['っ', 'ゃ', 'ッ', 'ョ', 'ヶ'] {
            assert!(matches!(glyph(c, all), (_, Turn::Shift(x, y)) if x > 0.0 && y < 0.0), "{c:?}");
        }
        assert_eq!(glyph('つ', all), ('つ', Turn::Upright));
    }
}
