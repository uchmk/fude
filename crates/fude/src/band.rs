//! The writing band's text, cursor and clock, without egui: what was typed,
//! where the cursor stands, the limits and when they were reached. The mock's
//! "writing session" (`fude-mock/index.html`), moved over as it is.
//!
//! Every edit is at the cursor; with the cursor at the end (`back` 0) it is
//! plain typing. The point on the screen never moves: the text slides past it.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// A limit the writing reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Hit {
    Chars,
    Time,
}

/// The limits set: 0 is none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    pub chars: u32,
    pub minutes: u32,
}

/// One change at the cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// Typed, committed by the input method, or pasted.
    Text(String),
    /// Enter: the paragraph ends here.
    Paragraph,
    Backspace,
    Delete,
    /// An arrow along the band: one letter back (-1) or on (1).
    Step(isize),
    /// An arrow with Ctrl: to the next point just past punctuation.
    Jump(isize),
    Home,
    End,
}

impl Edit {
    /// What the no-erase mode stops: all but typing and new paragraphs.
    pub fn erases(&self) -> bool {
        !matches!(self, Edit::Text(_) | Edit::Paragraph)
    }
}

/// Where Ctrl+arrows stop: just after a run of these (or a paragraph's end).
fn stop(c: char) -> bool {
    matches!(c, '、' | '。' | '，' | '．' | '！' | '？' | '!' | '?' | '」' | '』' | '）' | '\n')
}

/// Letters, the paragraph breaks not counted.
pub fn count(t: &[char]) -> usize {
    t.iter().filter(|&&c| c != '\n').count()
}

/// `m:ss`; `up` rounds a part second up, for what is left.
pub fn fmt(d: Duration, up: bool) -> String {
    let s = if up { d.as_millis().div_ceil(1000) } else { d.as_millis() / 1000 };
    format!("{}:{:02}", s / 60, s % 60)
}

fn minutes(m: u32) -> Duration {
    Duration::from_secs(u64::from(m) * 60)
}

/// The pace is taken every this long of the time counted: the letters there
/// are then.
pub const PACE: Duration = Duration::from_secs(5);

/// The pace is taken for six hours at most.
const PACE_MAX: usize = 4320;

/// How many [`PACE`]s `d` holds.
fn paces(d: Duration) -> usize {
    ((d.as_millis() / PACE.as_millis()) as usize).min(PACE_MAX)
}

/// What a draft keeps of the band, and what it is made again from.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kept {
    pub text: String,
    /// Letters after the cursor.
    pub back: usize,
    pub hit: Vec<Hit>,
    pub started: bool,
    /// The time counted, in milliseconds.
    pub elapsed_ms: u64,
    /// What going on after a limit added to the limits.
    pub add_chars: u32,
    pub add_ms: u64,
    /// Going on after a limit with none.
    pub free: bool,
    /// The letters at the end of each [`PACE`] of the time counted.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pace: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct Band {
    text: Vec<char>,
    /// Letters after the cursor: 0 is at the end, the usual case.
    back: usize,
    /// The limits reached, in the order they were.
    hit: Vec<Hit>,
    started: bool,
    /// The clock runs since then; `None` while it stands.
    since: Option<Instant>,
    /// What the clock counted before `since`.
    before: Duration,
    add_chars: u32,
    add_time: Duration,
    free: bool,
    /// The letters at the end of each [`PACE`] of the time counted, taken as
    /// the time passes [`Band::take_pace`] by.
    pace: Vec<u32>,
    /// Begun before the pace was taken: none is taken of it.
    paceless: bool,
    pub limits: Limits,
}

impl Band {
    pub fn text(&self) -> String {
        self.text.iter().collect()
    }

    /// The letters before the cursor and after it.
    pub fn split(&self) -> (&[char], &[char]) {
        self.text.split_at(self.cursor())
    }

    pub fn cursor(&self) -> usize {
        self.text.len() - self.back
    }

    pub fn back(&self) -> usize {
        self.back
    }

    pub fn hit(&self) -> &[Hit] {
        &self.hit
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn running(&self) -> bool {
        self.since.is_some()
    }

    pub fn count(&self) -> usize {
        count(&self.text)
    }

    /// The letters between the cursor and the end.
    pub fn rest(&self) -> usize {
        count(self.split().1)
    }

    /// The letter limit in force, 0 for none.
    pub fn char_cap(&self) -> u32 {
        if !self.free && self.limits.chars > 0 {
            self.limits.chars + self.add_chars
        } else {
            0
        }
    }

    /// The time limit in force, zero for none.
    pub fn time_cap(&self) -> Duration {
        if !self.free && self.limits.minutes > 0 {
            minutes(self.limits.minutes) + self.add_time
        } else {
            Duration::ZERO
        }
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        self.before + self.since.map_or(Duration::ZERO, |t| now.saturating_duration_since(t))
    }

    pub fn start(&mut self, now: Instant) {
        self.started = true;
        if self.since.is_none() {
            self.since = Some(now);
        }
    }

    /// The pace up to `now`: each [`PACE`] ended since it was last taken
    /// gets the letters there are, so it is taken before every edit.
    fn take_pace(&mut self, now: Instant) {
        let due = paces(self.elapsed(now));
        if !self.paceless && self.pace.len() < due {
            let n = self.count() as u32;
            self.pace.resize(due, n);
        }
    }

    /// The pace up to `now` (see [`Kept::pace`]); empty for a piece begun
    /// before it was taken.
    pub fn pace(&self, now: Instant) -> Vec<u32> {
        let mut p = self.pace.clone();
        let due = paces(self.elapsed(now));
        if !self.paceless && p.len() < due {
            p.resize(due, self.count() as u32);
        }
        p
    }

    /// The clock stands while the band is not on the screen.
    pub fn pause(&mut self, now: Instant) {
        self.take_pace(now);
        if let Some(t) = self.since.take() {
            self.before += now.saturating_duration_since(t);
        }
    }

    /// Back on the band: the clock goes on if it had started (or, with
    /// `from_open`, counts from the screen opening), and the band of a limit
    /// raised or dropped since goes.
    pub fn enter(&mut self, now: Instant, from_open: bool) {
        if self.started || from_open {
            self.start(now);
        }
        let (cc, tc, n, el) = (self.char_cap(), self.time_cap(), self.count(), self.elapsed(now));
        self.hit.retain(|h| match h {
            Hit::Chars => cc > 0 && n >= cc as usize,
            Hit::Time => !tc.is_zero() && el >= tc,
        });
    }

    /// `e` at the cursor; whether the text or the cursor changed. In the
    /// no-erase mode only typing and new paragraphs do anything.
    pub fn edit(&mut self, e: Edit, now: Instant, no_erase: bool) -> bool {
        if no_erase && e.erases() {
            return false;
        }
        self.take_pace(now);
        let n = self.text.len() as isize;
        let at = self.cursor() as isize;
        match e {
            Edit::Text(s) => self.commit(&s, now),
            Edit::Paragraph => self.paragraph(),
            Edit::Backspace => self.backspace(),
            Edit::Delete => self.delete(),
            Edit::Step(d) => self.move_to(at + d),
            Edit::Jump(d) => self.jump(d),
            Edit::Home => self.move_to(0),
            Edit::End => self.move_to(n),
        }
    }

    /// `a` before the cursor and `b` after it, with no empty paragraph left
    /// behind.
    fn rejoin(&mut self, a: Vec<char>, mut b: Vec<char>) {
        if b.first() == Some(&'\n') && a.last().is_none_or(|&c| c == '\n') {
            b.remove(0);
        }
        self.back = b.len();
        self.text = a;
        self.text.extend(b);
    }

    fn parts(&self) -> (Vec<char>, Vec<char>) {
        let (a, b) = self.split();
        (a.to_vec(), b.to_vec())
    }

    fn commit(&mut self, s: &str, now: Instant) -> bool {
        if s.is_empty() {
            return false;
        }
        let s = s.replace("\r\n", "\n").replace('\r', "\n");
        self.start(now);
        let (mut a, b) = self.parts();
        // A paste's blank lines are one break, as Enter would make them.
        for c in s.chars() {
            if c != '\n' || a.last().is_some_and(|&p| p != '\n') {
                a.push(c);
            }
        }
        self.rejoin(a, b);
        let cc = self.char_cap();
        if cc > 0 && self.count() >= cc as usize {
            self.reach(Hit::Chars);
        }
        true
    }

    fn paragraph(&mut self) -> bool {
        let (mut a, b) = self.parts();
        if a.last().is_none_or(|&c| c == '\n') || b.first() == Some(&'\n') {
            return false;
        }
        a.push('\n');
        self.rejoin(a, b);
        true
    }

    fn backspace(&mut self) -> bool {
        let (mut a, b) = self.parts();
        if a.pop().is_none() {
            return false;
        }
        self.rejoin(a, b);
        true
    }

    fn delete(&mut self) -> bool {
        let (a, mut b) = self.parts();
        if b.is_empty() {
            return false;
        }
        b.remove(0);
        self.rejoin(a, b);
        true
    }

    /// The cursor before the text's `i`th letter (a click on the clean copy).
    pub fn set_cursor(&mut self, i: usize) -> bool {
        self.move_to(i as isize)
    }

    /// A new piece: nothing written and the clock at zero, the limits kept.
    pub fn reset(&mut self) {
        *self = Band { limits: self.limits, ..Default::default() };
    }

    fn move_to(&mut self, i: isize) -> bool {
        let n = self.text.len();
        let back = n - i.clamp(0, n as isize) as usize;
        let moved = back != self.back;
        self.back = back;
        moved
    }

    fn jump(&mut self, d: isize) -> bool {
        let t = &self.text;
        let n = t.len() as isize;
        let at = |q: isize| q == 0 || q == n || (stop(t[q as usize - 1]) && !stop(t[q as usize]));
        let mut p = self.cursor() as isize;
        loop {
            p += d;
            if !(p > 0 && p < n && !at(p)) {
                break;
            }
        }
        self.move_to(p)
    }

    /// A limit reached: whether it is news.
    fn reach(&mut self, h: Hit) -> bool {
        if self.hit.contains(&h) {
            return false;
        }
        self.hit.push(h);
        true
    }

    /// The time limit, looked at while the clock runs: whether it was just
    /// reached.
    pub fn tick(&mut self, now: Instant) -> bool {
        self.take_pace(now);
        let tc = self.time_cap();
        !tc.is_zero() && self.elapsed(now) >= tc && self.reach(Hit::Time)
    }

    /// Each limit reached, raised by its own step until it is past where the
    /// writing already is: what goes on to add to the letters and the time.
    /// The clean copy shows it, and [`Band::go_on`] takes it.
    pub fn raised(&self, now: Instant) -> (u32, Duration) {
        let (mut add_c, mut add_t) = (self.add_chars, self.add_time);
        let (n, el) = (self.count(), self.elapsed(now));
        let (c, t) = (self.limits.chars, minutes(self.limits.minutes));
        if self.hit.contains(&Hit::Chars) && c > 0 {
            while n >= (c + add_c) as usize {
                add_c += c;
            }
        }
        if self.hit.contains(&Hit::Time) && !t.is_zero() {
            while el >= t + add_t {
                add_t += t;
            }
        }
        (add_c, add_t)
    }

    /// Writing on after a limit raises it (or, with `free`, drops it), so its
    /// band does not come straight back. Coming back from the clean copy
    /// does it.
    pub fn go_on(&mut self, now: Instant, free: bool) {
        if !self.hit.is_empty() {
            if free {
                self.free = true;
            } else {
                (self.add_chars, self.add_time) = self.raised(now);
            }
        }
        self.hit.clear();
    }

    /// What a draft keeps.
    pub fn kept(&self, now: Instant) -> Kept {
        Kept {
            text: self.text(),
            back: self.back,
            hit: self.hit.clone(),
            started: self.started,
            elapsed_ms: self.elapsed(now).as_millis() as u64,
            add_chars: self.add_chars,
            add_ms: self.add_time.as_millis() as u64,
            free: self.free,
            pace: self.pace(now),
        }
    }

    /// A band again from its draft, with the clock standing.
    pub fn from_kept(k: Kept, limits: Limits) -> Band {
        let text: Vec<char> = k.text.chars().collect();
        let paceless = k.pace.is_empty() && paces(Duration::from_millis(k.elapsed_ms)) > 0;
        Band {
            back: k.back.min(text.len()),
            text,
            hit: k.hit,
            started: k.started,
            since: None,
            before: Duration::from_millis(k.elapsed_ms),
            add_chars: k.add_chars,
            add_time: Duration::from_millis(k.add_ms),
            free: k.free,
            pace: k.pace,
            paceless,
            limits,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band(limits: Limits) -> (Band, Instant) {
        (Band { limits, ..Default::default() }, Instant::now())
    }

    fn typed(b: &mut Band, now: Instant, s: &str) {
        assert!(b.edit(Edit::Text(s.into()), now, false));
    }

    /// The text with `|` at the cursor.
    fn shown(b: &Band) -> String {
        let (a, c) = b.split();
        format!("{}|{}", a.iter().collect::<String>(), c.iter().collect::<String>())
    }

    #[test]
    fn typing_goes_in_at_the_cursor_and_the_arrows_move_it() {
        let (mut b, now) = band(Limits::default());
        typed(&mut b, now, "今日は晴れ");
        assert_eq!(shown(&b), "今日は晴れ|");
        assert!(b.edit(Edit::Step(-2), now, false));
        assert_eq!(shown(&b), "今日は|晴れ");
        typed(&mut b, now, "よく");
        assert_eq!(shown(&b), "今日はよく|晴れ");
        assert_eq!(b.rest(), 2);
        assert!(b.edit(Edit::Backspace, now, false));
        assert!(b.edit(Edit::Delete, now, false));
        assert_eq!(shown(&b), "今日はよ|れ");
        assert!(b.edit(Edit::Home, now, false));
        assert!(!b.edit(Edit::Step(-1), now, false), "nothing before the start");
        assert!(!b.edit(Edit::Backspace, now, false));
        assert!(b.edit(Edit::End, now, false));
        assert_eq!(shown(&b), "今日はよれ|");
        assert!(!b.edit(Edit::Step(1), now, false));
    }

    /// 2.4: no paragraph before the first letter, and none left empty.
    #[test]
    fn enter_ends_a_paragraph_once_and_none_is_left_empty() {
        let (mut b, now) = band(Limits::default());
        assert!(!b.edit(Edit::Paragraph, now, false), "nothing to end yet");
        typed(&mut b, now, "一つ。");
        assert!(b.edit(Edit::Paragraph, now, false));
        assert!(!b.edit(Edit::Paragraph, now, false), "no empty paragraph");
        typed(&mut b, now, "二つ");
        assert_eq!(b.text(), "一つ。\n二つ");
        assert_eq!(b.count(), 5, "the break is not a letter");
        // Backspace over the start of the paragraph joins it to the one before.
        assert!(b.edit(Edit::Step(-2), now, false));
        assert!(b.edit(Edit::Backspace, now, false));
        assert_eq!(shown(&b), "一つ。|二つ");
        // At a paragraph's start, deleting the letter before drops the break
        // that would be left at the front.
        let (mut b, now) = band(Limits::default());
        typed(&mut b, now, "あ\nい");
        assert!(b.edit(Edit::Step(-2), now, false));
        assert!(b.edit(Edit::Backspace, now, false));
        assert_eq!(shown(&b), "|い");
        // Enter just before a break does nothing.
        let (mut b, now) = band(Limits::default());
        typed(&mut b, now, "あ\nい");
        assert!(b.edit(Edit::Step(-2), now, false));
        assert!(!b.edit(Edit::Paragraph, now, false));
    }

    #[test]
    fn a_paste_keeps_its_paragraphs() {
        let (mut b, now) = band(Limits::default());
        typed(&mut b, now, "一\r\n二\r三");
        assert_eq!(b.text(), "一\n二\n三");
        // Blank lines close up, and nothing starts with a break.
        let (mut b, now) = band(Limits::default());
        typed(&mut b, now, "\n\n一\r\n\r\n\n二\n");
        assert_eq!(b.text(), "一\n二\n");
    }

    /// 2.3: Ctrl and the arrows stop just after each punctuation mark.
    #[test]
    fn ctrl_arrows_stop_after_punctuation() {
        let (mut b, now) = band(Limits::default());
        typed(&mut b, now, "雨。「晴れ」と言った。\n次");
        assert!(b.edit(Edit::Jump(-1), now, false));
        assert_eq!(shown(&b), "雨。「晴れ」と言った。\n|次");
        assert!(b.edit(Edit::Jump(-1), now, false));
        assert_eq!(shown(&b), "雨。「晴れ」|と言った。\n次");
        assert!(b.edit(Edit::Jump(-1), now, false));
        assert_eq!(shown(&b), "雨。|「晴れ」と言った。\n次");
        assert!(b.edit(Edit::Jump(-1), now, false));
        assert_eq!(shown(&b), "|雨。「晴れ」と言った。\n次");
        assert!(!b.edit(Edit::Jump(-1), now, false));
        assert!(b.edit(Edit::Jump(1), now, false));
        assert_eq!(shown(&b), "雨。|「晴れ」と言った。\n次");
    }

    #[test]
    fn no_erase_lets_only_typing_through() {
        let (mut b, now) = band(Limits::default());
        assert!(b.edit(Edit::Text("あい".into()), now, true));
        assert!(b.edit(Edit::Paragraph, now, true));
        for e in [Edit::Backspace, Edit::Delete, Edit::Step(-1), Edit::Jump(-1), Edit::Home, Edit::End] {
            assert!(!b.edit(e.clone(), now, true), "{e:?}");
        }
        assert_eq!(shown(&b), "あい\n|");
    }

    #[test]
    fn the_clock_starts_at_the_first_letter_and_stands_off_the_band() {
        let (mut b, t0) = band(Limits::default());
        b.enter(t0, false);
        assert!(!b.running(), "it waits for the first letter");
        let t1 = t0 + Duration::from_secs(3);
        typed(&mut b, t1, "あ");
        assert!(b.running());
        let t2 = t1 + Duration::from_secs(10);
        assert_eq!(b.elapsed(t2), Duration::from_secs(10));
        b.pause(t2);
        assert_eq!(b.elapsed(t2 + Duration::from_secs(60)), Duration::from_secs(10));
        b.enter(t2 + Duration::from_secs(60), false);
        assert_eq!(b.elapsed(t2 + Duration::from_secs(65)), Duration::from_secs(15));
        // Counting from the screen opening, it runs before a letter is typed.
        let (mut b, t0) = band(Limits::default());
        b.enter(t0, true);
        assert!(b.running() && b.started());
    }

    #[test]
    fn a_limit_is_reached_and_writing_on_raises_it() {
        let (mut b, t0) = band(Limits { chars: 5, minutes: 1 });
        typed(&mut b, t0, "あいう\nえ");
        assert!(b.hit().is_empty());
        typed(&mut b, t0, "おか");
        assert_eq!(b.hit(), [Hit::Chars]);
        let t1 = t0 + Duration::from_secs(59);
        assert!(!b.tick(t1));
        let t2 = t0 + Duration::from_secs(130);
        assert!(b.tick(t2));
        assert!(!b.tick(t2), "told once");
        assert_eq!(b.hit(), [Hit::Chars, Hit::Time]);
        // 6 letters: 5 + 5; 2:10: 1 + 1 + 1 minutes.
        assert_eq!(b.raised(t2), (5, Duration::from_secs(120)));
        b.go_on(t2, false);
        assert!(b.hit().is_empty());
        assert_eq!((b.char_cap(), b.time_cap()), (10, Duration::from_secs(180)));
        // Going on free drops them.
        b.edit(Edit::Text("きくけこさ".into()), t2, false);
        assert_eq!(b.hit(), [Hit::Chars]);
        b.go_on(t2, true);
        assert_eq!((b.char_cap(), b.time_cap()), (0, Duration::ZERO));
        assert!(!b.tick(t2 + Duration::from_secs(3600)));
    }

    #[test]
    fn a_limit_raised_since_lets_its_band_go() {
        let (mut b, t0) = band(Limits { chars: 3, minutes: 0 });
        typed(&mut b, t0, "あいう");
        assert_eq!(b.hit(), [Hit::Chars]);
        b.pause(t0);
        b.limits.chars = 400;
        b.enter(t0, false);
        assert!(b.hit().is_empty());
    }

    #[test]
    fn the_time_reads_as_minutes_and_seconds() {
        assert_eq!(fmt(Duration::from_millis(65_400), false), "1:05");
        assert_eq!(fmt(Duration::from_millis(65_400), true), "1:06");
        assert_eq!(fmt(Duration::from_secs(300), true), "5:00");
        assert_eq!(fmt(Duration::ZERO, false), "0:00");
    }

    #[test]
    fn a_draft_makes_the_same_band() {
        let (mut b, t0) = band(Limits { chars: 3, minutes: 5 });
        typed(&mut b, t0, "あいう\nえお");
        b.edit(Edit::Step(-1), t0, false);
        let k = b.kept(t0 + Duration::from_millis(4200));
        assert_eq!(k.elapsed_ms, 4200);
        let text = toml::to_string(&k).unwrap();
        let again = Band::from_kept(toml::from_str(&text).unwrap(), b.limits);
        assert_eq!(shown(&again), "あいう\nえ|お");
        assert_eq!(again.hit(), [Hit::Chars]);
        assert!(again.started() && !again.running(), "it stands until the band is shown");
        assert_eq!(again.elapsed(Instant::now()), Duration::from_millis(4200));
        // A draft from somewhere else does not put the cursor past the end.
        let k = Kept { text: "あ".into(), back: 9, ..Default::default() };
        assert_eq!(Band::from_kept(k, Limits::default()).cursor(), 0);
    }

    #[test]
    fn the_pace_is_the_letters_every_five_seconds_of_the_time_counted() {
        let (mut b, t0) = band(Limits::default());
        let s = Duration::from_secs;
        typed(&mut b, t0, "あい");
        typed(&mut b, t0 + s(7), "う");
        // 0:05 had 2; at 0:12 there were 3 by then.
        b.edit(Edit::Backspace, t0 + s(12), false);
        assert_eq!(b.pace(t0 + s(12)), [2, 3]);
        b.pause(t0 + s(14));
        // Off the band the time does not count.
        assert_eq!(b.pace(t0 + s(100)), [2, 3]);
        b.enter(t0 + s(100), false);
        assert_eq!(b.pace(t0 + s(102)), [2, 3, 2]);
        let k = b.kept(t0 + s(102));
        assert_eq!(k.pace, [2, 3, 2]);
        assert_eq!(Band::from_kept(k, Limits::default()).pace(t0), [2, 3, 2]);
        // A piece timed before the pace was taken gets none.
        let old = Band::from_kept(Kept { text: "前の".into(), started: true, elapsed_ms: 65_000, ..Kept::default() }, Limits::default());
        assert!(old.pace(t0).is_empty());
        let mut b = Band::from_kept(Kept { started: true, elapsed_ms: 4_000, ..Kept::default() }, Limits::default());
        b.enter(t0, false);
        typed(&mut b, t0 + s(2), "あ");
        // The 4 seconds before count: at 0:05 there were none.
        assert_eq!(b.pace(t0 + s(2)), [0]);
        b.reset();
        assert!(b.pace(t0 + s(9)).is_empty());
    }

    #[test]
    fn a_click_puts_the_cursor_and_a_new_piece_keeps_only_the_limits() {
        let (mut b, t0) = band(Limits { chars: 3, minutes: 5 });
        typed(
            &mut b,
            t0,
            "あいう
えお",
        );
        assert!(b.set_cursor(2) && !b.set_cursor(2));
        assert_eq!(
            shown(&b),
            "あい|う
えお"
        );
        b.set_cursor(99);
        assert_eq!(b.back(), 0);
        b.reset();
        assert_eq!((b.text(), b.hit().len(), b.started(), b.running()), (String::new(), 0, false, false));
        assert_eq!(b.elapsed(t0 + Duration::from_secs(9)), Duration::ZERO);
        assert_eq!(b.limits, Limits { chars: 3, minutes: 5 });
    }
}
