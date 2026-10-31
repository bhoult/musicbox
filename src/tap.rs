//! What the player hands the displays: the last second or so of the sound it
//! made (for the equalizer), where in it the speakers are, and what is
//! playing. The audio thread writes without ever waiting on a reader.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

/// Frames kept (1.4 s at 48 kHz), a power of two.
const RING: usize = 1 << 16;

/// What is playing, as the displays show it.
#[derive(Clone, Debug, Default)]
pub struct Now {
    /// the music (a `music::mix_id`)
    pub id: usize,
    /// pieces begun since the start, this one included
    pub piece: u64,
    pub bpm: f64,
    /// the tonic and the mode (empty for the gamelan, whose tunings have neither)
    pub key: &'static str,
    pub mode: &'static str,
    /// the meter, and the piece's form or style where the genre has them ("6/8 jig")
    pub meter: &'static str,
    /// the section (intro, build, main, breakdown, outro), its bar and length
    pub section: &'static str,
    pub bar: u32,
    pub bars: u32,
    /// stopped (else playing)
    pub stopped: bool,
    /// seconds into the piece, the length asked of each piece (0: its own),
    /// and the piece's own, as planned
    pub elapsed: f64,
    pub length: f64,
    pub planned: f64,
    /// the seed of the engine playing, and which of its pieces this is (as
    /// `--export` takes them: the browser's export)
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub seed: u64,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub song: usize,
    /// how the piece playing is made (the browser's buttons)
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub opts: crate::music::Opts,
    /// the song playing, as its id gives it (once it has begun), and the
    /// songs before and after it in the history (back and forward)
    pub sid: Option<crate::music::SongId>,
    pub back: Option<crate::music::SongId>,
    pub fwd: Option<crate::music::SongId>,
}

pub struct Tap {
    /// interleaved stereo, f32 bits
    ring: Box<[AtomicU32]>,
    /// frames written, and when the last were (ns after `base`)
    written: AtomicU64,
    stamp: AtomicU64,
    base: Instant,
    /// frames between the newest written and the one the speakers play
    latency: u64,
    rate: u32,
    now: Mutex<Now>,
}

impl Tap {
    pub fn new(rate: u32, latency: u64) -> Tap {
        Tap {
            ring: (0..2 * RING).map(|_| AtomicU32::new(0)).collect(),
            written: AtomicU64::new(0),
            stamp: AtomicU64::new(0),
            base: Instant::now(),
            latency,
            rate,
            now: Mutex::new(Now::default()),
        }
    }

    /// The audio thread: frames just made (interleaved stereo), and what is
    /// playing (dropped if a display holds the lock just then).
    pub fn push(&self, buf: &[f32], now: impl FnOnce() -> Now) {
        let w = self.written.load(Ordering::Relaxed);
        for (k, f) in buf.chunks_exact(2).enumerate() {
            let i = 2 * ((w as usize + k) % RING);
            self.ring[i].store(f[0].to_bits(), Ordering::Relaxed);
            self.ring[i + 1].store(f[1].to_bits(), Ordering::Relaxed);
        }
        self.stamp.store(self.base.elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.written.store(w + (buf.len() / 2) as u64, Ordering::Release);
        if let Ok(mut g) = self.now.try_lock() {
            *g = now();
        }
    }

    pub fn now(&self) -> Now {
        self.now.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The frame the speakers are playing, about: the newest written less the
    /// latency, moving on in real time between writes.
    pub fn playing(&self) -> u64 {
        let w = self.written.load(Ordering::Acquire);
        let since = (self.base.elapsed().as_nanos() as u64).saturating_sub(self.stamp.load(Ordering::Relaxed));
        let ahead = (since as f64 * 1e-9 * self.rate as f64) as u64;
        (w + ahead.min(self.latency)).saturating_sub(self.latency)
    }

    /// The `out.len()` frames up to the one playing, mixed to mono (zeros
    /// before the start).
    pub fn window(&self, out: &mut [f32]) {
        let end = self.playing();
        let n = out.len().min(RING / 2) as u64;
        for (k, o) in out.iter_mut().enumerate() {
            let f = (end + k as u64).checked_sub(n);
            *o = f.map_or(0.0, |f| {
                let i = 2 * (f as usize % RING);
                0.5 * (f32::from_bits(self.ring[i].load(Ordering::Relaxed))
                    + f32::from_bits(self.ring[i + 1].load(Ordering::Relaxed)))
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_ends_where_the_speakers_are() {
        let tap = Tap::new(48000, 100);
        let buf: Vec<f32> = (0..2000).flat_map(|i| [i as f32, i as f32]).collect();
        tap.push(&buf, Now::default);
        // just written: playing 100 frames back
        let mut w = vec![0.0; 10];
        tap.window(&mut w);
        let p = tap.playing();
        assert!((1900..=2000).contains(&p), "{p}");
        assert_eq!(w[9], (p - 1) as f32);
        assert_eq!(w[0], (p - 10) as f32);
        // before the start: zeros
        let mut long = vec![1.0; 4000];
        tap.window(&mut long);
        assert_eq!(long[0], 0.0);
    }
}
