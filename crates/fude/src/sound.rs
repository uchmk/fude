//! The sounds of writing, made here in code rather than recorded (nothing to
//! license): a typewriter's key or a pencil's stroke for each letter, a
//! heavier one for a new paragraph, and a small bell when a limit is
//! reached. ito's player plays them. It is opened at the first sound and
//! closed after a minute of quiet, since an output left open keeps the
//! machine from sleeping.

use std::f32::consts::TAU;
use std::time::{Duration, Instant};

use ito_notify::player::{Player, Sample, RATE};

use crate::config::Tick;

/// What is to sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cue {
    /// A letter typed or erased, or a key of the input method's.
    Letter,
    /// Enter: a new paragraph.
    Paragraph,
    /// A limit reached.
    Bell,
}

/// Quiet this long closes the output.
const IDLE: Duration = Duration::from_secs(60);

/// An output refused is asked for again after this long.
const RETRY: Duration = Duration::from_secs(60);

/// A letter's sounds, taken in turn so that a run of keys is not one sound
/// over and over.
const TURNS: u64 = 4;

/// The loudest a sound is made; the volume scales it.
const PEAK: f32 = 0.9;

/// The player while it is open, and the sounds once made.
#[derive(Default)]
pub struct Sound {
    player: Option<Player>,
    /// The last sound, or when the output was opened.
    last: Option<Instant>,
    refused: Option<Instant>,
    sounds: Option<Sounds>,
    turn: usize,
}

struct Sounds {
    typewriter: Vec<Sample>,
    carriage: Sample,
    pencil: Vec<Sample>,
    dash: Sample,
    bell: Sample,
}

impl Sounds {
    fn make() -> Sounds {
        let sample = |pcm: Vec<f32>| Sample::from_pcm(&pcm, RATE);
        Sounds {
            typewriter: (0..TURNS).map(|k| sample(key(k, 1.0))).collect(),
            carriage: sample(carriage(TURNS)),
            pencil: (0..TURNS).map(|k| sample(pencil(k, 0.05 + 0.006 * k as f32))).collect(),
            dash: sample(pencil(TURNS, 0.14)),
            bell: sample(bell()),
        }
    }
}

impl Sound {
    /// Sound `cue` at `volume` (0–100), a letter's and a paragraph's as
    /// `kind` (none: nothing). The output is opened if it is not.
    pub fn play(&mut self, cue: Cue, kind: Tick, volume: u32, now: Instant) {
        if volume == 0 || (cue != Cue::Bell && kind == Tick::Off) {
            return;
        }
        let sounds = self.sounds.get_or_insert_with(Sounds::make);
        self.turn = (self.turn + 1) % TURNS as usize;
        let sample = match (cue, kind) {
            (Cue::Bell, _) => &sounds.bell,
            (Cue::Paragraph, Tick::Pencil) => &sounds.dash,
            (Cue::Paragraph, _) => &sounds.carriage,
            (Cue::Letter, Tick::Pencil) => &sounds.pencil[self.turn],
            (Cue::Letter, _) => &sounds.typewriter[self.turn],
        };
        if self.player.is_none() && self.refused.is_none_or(|t| now.duration_since(t) >= RETRY) {
            match Player::new() {
                Ok(p) => self.player = Some(p),
                Err(_) => self.refused = Some(now),
            }
        }
        if let Some(p) = &self.player {
            p.play(sample, volume.min(100) as f32 / 100.0);
            self.last = Some(now);
        }
    }

    /// Close the output after a minute of quiet; while it is open, how long
    /// until it is to be looked at again.
    pub fn rest(&mut self, now: Instant) -> Option<Duration> {
        self.player.as_ref()?;
        let quiet = self.last.map_or(IDLE, |t| now.duration_since(t));
        if quiet >= IDLE {
            self.player = None;
            return None;
        }
        Some(IDLE - quiet)
    }
}

/// A little noise of its own, the same each run.
struct Noise(u64);

impl Noise {
    fn new(seed: u64) -> Noise {
        Noise(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }

    /// -1.0..1.0.
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 40) as f32 / (1u64 << 23) as f32 - 1.0
    }
}

fn frames(secs: f32) -> usize {
    (secs * RATE as f32) as usize
}

/// `pcm` brought to `peak` at its loudest.
fn level(mut pcm: Vec<f32>, peak: f32) -> Vec<f32> {
    let most = pcm.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    if most > 0.0 {
        pcm.iter_mut().for_each(|x| *x *= peak / most);
    }
    pcm
}

/// A typewriter's key, `low` times its pitch: the type bar's strike on the
/// platen, a short ring of metal and the body's thud.
fn key(seed: u64, low: f32) -> Vec<f32> {
    let mut n = Noise::new(seed);
    let pitch = (1.0 + 0.05 * n.next()) / low;
    let pcm = (0..frames(0.055))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let strike = n.next() * (-t / 0.0012).exp();
            let ring = (TAU * 2300.0 * pitch * t).sin() * (-t / 0.005).exp();
            let thud = (TAU * 150.0 * pitch * t).sin() * (-t / 0.016).exp();
            0.8 * strike + 0.3 * ring + 0.6 * thud
        })
        .collect();
    level(pcm, PEAK)
}

/// The carriage going back for a new paragraph: the ratchet's clicks, then
/// the stop.
fn carriage(seed: u64) -> Vec<f32> {
    let mut pcm = vec![0.0; frames(0.17)];
    let parts = [(0.0, 0.35, 1.0), (0.024, 0.3, 1.0), (0.048, 0.27, 1.0), (0.09, 1.0, 1.4)];
    for (k, (at, gain, low)) in parts.into_iter().enumerate() {
        let from = frames(at);
        for (o, x) in pcm[from..].iter_mut().zip(key(seed + k as u64, low)) {
            *o += gain * x;
        }
    }
    level(pcm, PEAK)
}

/// A pencil on paper for `secs`: grains of graphite in a band of noise,
/// coming in quickly and dying away.
fn pencil(seed: u64, secs: f32) -> Vec<f32> {
    let mut n = Noise::new(seed + 101);
    let len = frames(secs);
    let grain_len = frames(0.0015).max(1);
    let (mut low, mut high, mut last) = (0.0f32, 0.0f32, 0.0f32);
    let mut grain = 1.0;
    let pcm = (0..len)
        .map(|i| {
            if i % grain_len == 0 {
                grain = 0.35 + 0.65 * n.next().abs();
            }
            // About 1.5 kHz to 6 kHz.
            low += 0.57 * (n.next() - low);
            high = 0.82 * (high + low - last);
            last = low;
            let t = i as f32 / len as f32;
            let env = (t / 0.08).min(1.0) * (1.0 - t).powf(1.5);
            high * grain * env
        })
        .collect();
    level(pcm, PEAK * 0.8)
}

/// A small bell, struck once: its partials, the high ones dying first.
fn bell() -> Vec<f32> {
    let partials = [(1.0, 1.0, 0.6), (2.0, 0.3, 0.35), (2.76, 0.22, 0.25), (5.4, 0.08, 0.1)];
    let pcm = (0..frames(1.4))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let attack = (t / 0.003).min(1.0);
            attack * partials.iter().map(|(f, a, d)| a * (TAU * 1046.5 * f * t).sin() * (-t / d).exp()).sum::<f32>()
        })
        .collect();
    level(pcm, PEAK * 0.7)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loudest(pcm: &[f32]) -> f32 {
        pcm.iter().fold(0.0, |m, x| m.max(x.abs()))
    }

    #[test]
    fn the_sounds_are_short_and_in_range() {
        for (pcm, most) in [(key(0, 1.0), 0.08), (carriage(4), 0.2), (pencil(0, 0.05), 0.08), (pencil(4, 0.14), 0.2), (bell(), 1.5)] {
            let secs = pcm.len() as f32 / RATE as f32;
            assert!(secs > 0.03 && secs <= most, "{secs}");
            assert!(pcm.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
            assert!(loudest(&pcm) > 0.5, "not silent");
        }
        // A key ends quiet: no click at its end.
        let k = key(1, 1.0);
        assert!(loudest(&k[k.len() - 50..]) < 0.05);
    }

    #[test]
    fn a_run_of_keys_is_not_one_sound() {
        assert_ne!(key(0, 1.0), key(1, 1.0));
        assert_ne!(pencil(0, 0.05), pencil(1, 0.05));
        // And the same each run.
        assert_eq!(key(2, 1.0), key(2, 1.0));
    }

    #[test]
    fn nothing_sounds_without_a_kind_or_a_volume() {
        let mut s = Sound::default();
        let now = Instant::now();
        s.play(Cue::Letter, Tick::Off, 50, now);
        s.play(Cue::Bell, Tick::Pencil, 0, now);
        assert!(s.player.is_none() && s.refused.is_none() && s.sounds.is_none());
        assert_eq!(s.rest(now), None);
    }
}
