//! How fast a piece was written, read back from its pace (the letters at the
//! end of each [`PACE`] of its time): how fast each line of the clean copy
//! went, and where the hand stopped. The clean copy draws these in its
//! margin; they are for the screen only.
//!
//! The pace is the count over time, so a letter is taken to be written when
//! the count first reached it. Erasing and writing between letters blur
//! this, which is fine for a thin line.
//!
//! [`PACE`]: crate::band::PACE

/// The count standing still this many paces in a row (30 s) is a stop.
pub const STOP: usize = 6;

/// A piece's pace, ready to be asked about its lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Pace {
    /// The letters (the paragraph breaks not counted) before each letter of
    /// the text, and at its end.
    before: Vec<u32>,
    pace: Vec<u32>,
    /// Letters to a pace over the whole piece.
    avg: f32,
    /// The text's indexes where the hand stopped.
    stops: Vec<usize>,
}

impl Pace {
    /// The pace of `text`; none when it was not kept or there is nothing
    /// written.
    pub fn new(text: &str, pace: &[u32]) -> Option<Pace> {
        let mut before = Vec::with_capacity(text.len() + 1);
        let mut n = 0;
        for c in text.chars() {
            before.push(n);
            n += u32::from(c != '\n');
        }
        before.push(n);
        if pace.is_empty() || n == 0 {
            return None;
        }
        let avg = n as f32 / pace.len() as f32;
        let mut p = Pace { before, pace: pace.to_vec(), avg, stops: Vec::new() };
        p.stops = p.find_stops();
        Some(p)
    }

    /// How fast the letters from `at` to `end` (the text's indexes) were
    /// written, from 0 to 1, where twice the piece's own speed is 1; none
    /// when there is no letter there.
    pub fn share(&self, at: usize, end: usize) -> Option<f32> {
        let last = self.before.len() - 1;
        let (a, b) = (self.before[at.min(last)], self.before[end.min(last)]);
        if b <= a {
            return None;
        }
        let first = self.pace.iter().position(|&c| c > a).unwrap_or(self.pace.len());
        let done = self.pace.iter().position(|&c| c >= b).unwrap_or(self.pace.len());
        let speed = (b - a) as f32 / (done.saturating_sub(first) + 1) as f32;
        Some((speed / (2.0 * self.avg)).min(1.0))
    }

    /// The text's indexes where the hand stopped for [`STOP`] paces or more
    /// and then wrote on, in order.
    pub fn stops(&self) -> &[usize] {
        &self.stops
    }

    fn find_stops(&self) -> Vec<usize> {
        let total = *self.before.last().unwrap_or(&0);
        let mut stops = Vec::new();
        let mut k = 0;
        while k < self.pace.len() {
            let n = self.pace[k];
            let run = self.pace[k..].iter().take_while(|&&c| c == n).count();
            let later = self.pace[k + run..].iter().copied().chain([total]).max().unwrap_or(0);
            if run >= STOP && n > 0 && n <= total && later > n {
                stops.push(self.before.partition_point(|&c| c < n));
            }
            k += run;
        }
        stops.sort_unstable();
        stops.dedup();
        stops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_kept_is_no_pace() {
        assert_eq!(Pace::new("あいう", &[]), None);
        assert_eq!(Pace::new("", &[0, 0]), None);
        assert_eq!(Pace::new("\n", &[0]), None);
    }

    #[test]
    fn a_line_is_as_fast_as_its_letters_came() {
        // Ten letters in the first pace, then two in each of the next five.
        let text = "あいうえおかきくけこ\nさしすせそたちつてと";
        let p = Pace::new(text, &[10, 12, 14, 16, 18, 20]).unwrap();
        let fast = p.share(0, 10).unwrap();
        let slow = p.share(11, 21).unwrap();
        assert_eq!(fast, 1.0);
        assert!(slow < 0.5 && slow > 0.0, "{slow}");
        // The break between them is no letter.
        assert_eq!(p.share(10, 11), None);
        assert_eq!(p.share(3, 3), None);
        // Past the end is the end.
        assert_eq!(p.share(11, 99), p.share(11, 21));
    }

    #[test]
    fn letters_after_the_last_pace_count_in_the_one_going() {
        let p = Pace::new("あいうえ", &[2]).unwrap();
        assert!(p.share(2, 4).is_some());
    }

    #[test]
    fn a_stop_is_half_a_minute_and_then_more() {
        // Three letters, still for seven paces, then two more.
        let p = Pace::new("あい\nうえお", &[2, 3, 3, 3, 3, 3, 3, 3, 5]).unwrap();
        // After う, which is at 3 in the text (the break counted).
        assert_eq!(p.stops(), [4]);
        // Still for five paces is no stop.
        let p = Pace::new("あいうえお", &[3, 3, 3, 3, 3, 5]).unwrap();
        assert!(p.stops().is_empty());
        // Still at the end is not a stop: nothing came after.
        let p = Pace::new("あいう", &[3, 3, 3, 3, 3, 3, 3]).unwrap();
        assert!(p.stops().is_empty());
        // Nor is still before the first letter.
        let p = Pace::new("あい", &[0, 0, 0, 0, 0, 0, 0, 2]).unwrap();
        assert!(p.stops().is_empty());
        // Letters typed after the last pace end the stop too.
        let p = Pace::new("あいうえ", &[2, 2, 2, 2, 2, 2]).unwrap();
        assert_eq!(p.stops(), [2]);
    }
}
