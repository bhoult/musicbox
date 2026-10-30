//! The equalizer's bars: the last 4096 frames the speakers played through a
//! Hann window and an FFT, gathered into log-spaced bands, in decibels
//! tilted up 3 dB an octave (music's highs sit far below its bass), each
//! bar falling slowly after a rise and a peak marker held above it.

use std::time::Instant;

use crate::tap::Tap;

/// The analysis frame (85 ms at 48 kHz: 11.7 Hz bins).
const N: usize = 4096;
/// The bands' edges, Hz.
const LO: f32 = 35.0;
const HI: f32 = 16000.0;
/// The decibels a bar spans (full height at `TOP`), and the tilt's pivot.
const FLOOR_DB: f32 = -72.0;
const TOP_DB: f32 = -12.0;
const TILT_DB: f32 = 3.0;
const PIVOT: f32 = 1000.0;
/// A bar's fall, heights a second; a peak marker's hold (s) and fall (heights a second, gaining speed).
const FALL: f32 = 1.4;
const HOLD: f32 = 0.6;
const PEAK_FALL: f32 = 2.5;

/// Radix-2 FFT, in place, of one fixed power-of-two size (fleece's `audio.rs`).
struct Fft {
    rev: Vec<u32>,
    cos: Vec<f32>,
    sin: Vec<f32>,
}

impl Fft {
    fn new(n: usize) -> Fft {
        assert!(n.is_power_of_two() && n >= 2);
        let bits = n.trailing_zeros();
        let rev = (0..n as u32).map(|i| i.reverse_bits() >> (32 - bits)).collect();
        let a = |k: usize| -2.0 * std::f64::consts::PI * k as f64 / n as f64;
        let cos = (0..n / 2).map(|k| a(k).cos() as f32).collect();
        let sin = (0..n / 2).map(|k| a(k).sin() as f32).collect();
        Fft { rev, cos, sin }
    }

    /// The forward transform (e^-2πi kn/N) of `re` + i `im`.
    fn run(&self, re: &mut [f32], im: &mut [f32]) {
        let n = self.rev.len();
        for i in 0..n {
            let j = self.rev[i] as usize;
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let (half, step) = (len / 2, n / len);
            for s in (0..n).step_by(len) {
                for k in 0..half {
                    let (c, si) = (self.cos[k * step], self.sin[k * step]);
                    let (a, b) = (s + k, s + k + half);
                    let tr = re[b] * c - im[b] * si;
                    let ti = re[b] * si + im[b] * c;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            len *= 2;
        }
    }
}

/// Where `f` (Hz) falls across the bars, 0 (left) to 1 (right).
pub fn position(f: f32) -> f32 {
    (f / LO).ln() / (HI / LO).ln()
}

pub struct Spectrum {
    fft: Fft,
    window: Vec<f32>,
    rate: f32,
    re: Vec<f32>,
    im: Vec<f32>,
    /// |X|^2 to a sine's mean square (0 dB: a full-scale sine)
    scale: f32,
    /// each bar's height (0..1), its peak marker, and how long the marker has held
    pub bars: Vec<f32>,
    pub peaks: Vec<f32>,
    held: Vec<f32>,
    last: Option<Instant>,
}

impl Spectrum {
    pub fn new(rate: u32) -> Spectrum {
        let window: Vec<f32> =
            (0..N).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / N as f32).cos()).collect();
        let wsum: f32 = window.iter().sum();
        Spectrum {
            fft: Fft::new(N),
            window,
            rate: rate as f32,
            re: vec![0.0; N],
            im: vec![0.0; N],
            // a sine of amplitude A peaks at A * wsum / 2 in its bin; its mean square is A^2 / 2
            scale: 2.0 / (wsum * wsum),
            bars: Vec::new(),
            peaks: Vec::new(),
            held: Vec::new(),
            last: None,
        }
    }

    /// Analyses what is playing now into `n` bars.
    pub fn update(&mut self, tap: &Tap, n: usize) {
        let mut x = vec![0.0f32; N];
        tap.window(&mut x);
        self.analyse(&x, n);
    }

    /// `x` (N frames, mono) into `n` bars, falling from the last ones by the time since.
    pub fn analyse(&mut self, x: &[f32], n: usize) {
        let n = n.max(1);
        let dt = self.last.map_or(0.0, |t| t.elapsed().as_secs_f32().min(0.25));
        self.last = Some(Instant::now());
        if self.bars.len() != n {
            self.bars = vec![0.0; n];
            self.peaks = vec![0.0; n];
            self.held = vec![0.0; n];
        }
        for i in 0..N {
            self.re[i] = x.get(i).copied().unwrap_or(0.0) * self.window[i];
            self.im[i] = 0.0;
        }
        self.fft.run(&mut self.re, &mut self.im);
        let hz = self.rate / N as f32;
        let power = |k: usize| self.re[k] * self.re[k] + self.im[k] * self.im[k];
        for b in 0..n {
            let f0 = LO * (HI / LO).powf(b as f32 / n as f32);
            let f1 = LO * (HI / LO).powf((b + 1) as f32 / n as f32);
            let (k0, k1) = ((f0 / hz).round() as usize, (f1 / hz).round() as usize);
            // the band's loudest bin; a band narrower than a bin reads the
            // bins either side of its middle, interpolated
            let p = if k1 > k0 {
                (k0..k1.min(N / 2)).map(power).fold(0.0f32, f32::max)
            } else {
                let c = (f0 * f1).sqrt() / hz;
                let (k, t) = (c.floor() as usize, c.fract());
                power(k) * (1.0 - t) + power((k + 1).min(N / 2)) * t
            };
            let fc = (f0 * f1).sqrt();
            let db = 10.0 * (p * self.scale).max(1e-12).log10() + TILT_DB * (fc / PIVOT).log2();
            let h = ((db - FLOOR_DB) / (TOP_DB - FLOOR_DB)).clamp(0.0, 1.0);
            self.bars[b] = h.max(self.bars[b] - FALL * dt);
            if self.bars[b] >= self.peaks[b] {
                (self.peaks[b], self.held[b]) = (self.bars[b], 0.0);
            } else {
                self.held[b] += dt;
                if self.held[b] > HOLD {
                    let t = self.held[b] - HOLD;
                    self.peaks[b] = (self.peaks[b] - PEAK_FALL * t * dt).max(self.bars[b]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sine lights the bar holding its frequency, and no bar far from it.
    #[test]
    fn a_sine_lights_its_own_band() {
        let mut s = Spectrum::new(48000);
        let f = 440.0;
        let x: Vec<f32> = (0..N).map(|i| 0.5 * (2.0 * std::f32::consts::PI * f * i as f32 / 48000.0).sin()).collect();
        s.analyse(&x, 32);
        let band = |hz: f32| ((hz / LO).ln() / (HI / LO).ln() * 32.0) as usize;
        let at = band(f);
        assert!(s.bars[at] > 0.6, "{:?}", s.bars);
        for (b, &h) in s.bars.iter().enumerate() {
            if b.abs_diff(at) > 3 {
                assert!(h < 0.25, "bar {b}: {h} ({:?})", s.bars);
            }
        }
        // silence: the bars fall, the peaks hold a while
        s.analyse(&[0.0; N], 32);
        assert!(s.peaks[at] > 0.6);
    }
}
