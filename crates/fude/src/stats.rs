//! The record of writing, summed from the days the pieces grew on: the
//! letters written today, the days written on in a row, and the letters to a
//! minute over the last week. The shelf shows it over the pieces and the
//! clean copy under the paper.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};

use crate::notebook::{Day, Note};

/// The days the letters to a minute are taken over, today among them.
pub const WEEK: u64 = 7;

/// Less time than this is too little to say a speed by (a minute).
const ENOUGH_MS: u64 = 60_000;

/// The record as of a day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Letters written today.
    pub today: u32,
    /// Days written on in a row, up to today or, while today has nothing
    /// yet, up to yesterday.
    pub streak: u32,
    /// Letters to a minute over the last [`WEEK`] days; none on too little
    /// time.
    pub per_minute: Option<u32>,
}

/// The record of `days` (of any pieces, in any order) as of `today`.
pub fn of(days: impl IntoIterator<Item = Day>, today: NaiveDate) -> Stats {
    let mut by: BTreeMap<NaiveDate, (u32, u64)> = BTreeMap::new();
    for d in days {
        let e = by.entry(d.day).or_default();
        e.0 += d.chars;
        e.1 += d.ms;
    }
    let wrote = |day: NaiveDate| by.get(&day).is_some_and(|e| e.0 > 0);
    let mut day = if wrote(today) { Some(today) } else { today.checked_sub_days(Days::new(1)) };
    let mut streak = 0;
    while let Some(d) = day.filter(|&d| wrote(d)) {
        streak += 1;
        day = d.checked_sub_days(Days::new(1));
    }
    let from = today.checked_sub_days(Days::new(WEEK - 1)).unwrap_or(NaiveDate::MIN);
    let (chars, ms) = by.range(from..=today).fold((0, 0), |(c, m), (_, e)| (c + e.0, m + e.1));
    Stats { today: by.get(&today).map_or(0, |e| e.0), streak, per_minute: per_minute(chars, ms) }
}

/// The record of all the notebooks' pieces as of `today`.
pub fn notes(notes: &[Note], today: NaiveDate) -> Stats {
    of(notes.iter().flat_map(|n| &n.pieces).flat_map(|p| p.days()), today)
}

/// `chars` letters in `ms` as letters to a minute; none on too little time.
pub fn per_minute(chars: u32, ms: u64) -> Option<u32> {
    (ms >= ENOUGH_MS).then(|| (chars as u64 * 60_000 / ms) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str, chars: u32, ms: u64) -> Day {
        Day { day: s.parse().unwrap(), chars, ms }
    }

    #[test]
    fn today_and_the_days_in_a_row() {
        let today: NaiveDate = "2026-10-27".parse().unwrap();
        let days = [
            day("2026-10-27", 120, 60_000),
            day("2026-10-26", 300, 120_000),
            day("2026-10-27", 30, 30_000),
            day("2026-10-25", 10, 5_000),
            // A day of only erasing breaks the row.
            day("2026-10-24", 0, 9_000),
            day("2026-10-23", 50, 10_000),
        ];
        let s = of(days, today);
        assert_eq!(s.today, 150);
        assert_eq!(s.streak, 3);
        // 510 letters in 4 minutes 54 seconds, the 23rd's too.
        assert_eq!(s.per_minute, Some(510 * 60_000 / 234_000));
    }

    #[test]
    fn the_row_waits_for_today() {
        let days = [day("2026-10-26", 300, 120_000), day("2026-10-25", 10, 5_000)];
        let s = of(days, "2026-10-27".parse().unwrap());
        assert_eq!((s.today, s.streak), (0, 2));
        // A day missed ends it.
        let s = of(days, "2026-10-28".parse().unwrap());
        assert_eq!(s.streak, 0);
    }

    #[test]
    fn a_week_back_and_enough_time() {
        let today: NaiveDate = "2026-10-27".parse().unwrap();
        // Eight days back is out of the week.
        let s = of([day("2026-10-19", 600, 600_000), day("2026-10-27", 20, 10_000)], today);
        assert_eq!(s.per_minute, None);
        assert_eq!(of([], today), Stats::default());
        assert_eq!(per_minute(90, 60_000), Some(90));
        assert_eq!(per_minute(90, 59_999), None);
    }
}
