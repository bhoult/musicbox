//! The network's instruments: for each recorded instrument it has learned
//! (tools/neural/voices.py), a small network that plays a note from its
//! pitch, its velocity and the time since it began, 250 times a second: how
//! loud the note is, how that loudness is shared among its harmonics, and how
//! loud its noise (breath, bow, buzz) is in each of `BANDS` bands. Oscillators
//! and filtered noise make that into sound. A few hundred kilobytes in place
//! of an instrument's recordings, and every pitch and velocity between them.
//!
//! What a network computes, as voices.py trains it (`MBN1` files):
//! - in: (pitch - 60) / 24, velocity (0-1), and ln(1 + t / 0.01) / ln(1 + 3 / 0.01)
//!   with t the seconds since the note began, at most 3 (a held note's tone
//!   is steady by then);
//! - layers: dense, ReLU between them, none after the last;
//! - out: ln of the harmonics' total amplitude, `HARMS` logits (a softmax over
//!   the harmonics under 20 kHz shares the amplitude out), and ln of each
//!   band's noise amplitude (its RMS); an `MBN2` file's two more, its
//!   vibrato: ln of the depth (cents, at most `VIB_MAX`), and the rate
//!   (4 + 4 sigmoid(y) Hz).
//!
//! The sound, sample by sample, the frames' values ramped between them:
//! sum over k of a_k sin(k phi), phi the note's phase (its rate f0 times
//! 2^(depth sin(2 pi psi) / 1200), psi the vibrato's phase, both from 0 at
//! the note's start); plus, for each band, its amplitude times white noise
//! through that band's band-pass (a state variable filter at `band_hz(b)`,
//! Q `BAND_Q`), scaled to an RMS of 1.

use std::sync::OnceLock;

/// Samples a frame: 250 frames a second at 48 kHz.
pub const FRAME: usize = 192;
pub const HARMS: usize = 48;
pub const BANDS: usize = 16;
const SR: f32 = 48000.0;
const T_MAX: f32 = 3.0;
const T_KNEE: f32 = 0.01;
/// Harmonics at or above this are left out (and their share given to the rest).
const NYQ: f32 = 20000.0;
pub const BAND_Q: f32 = 1.5;
const MAGIC: &[u8; 4] = b"MBN1";
const MAGIC_VIB: &[u8; 4] = b"MBN2";
/// The deepest vibrato a network may ask for (cents).
const VIB_MAX: f32 = 150.0;

/// The centre of noise band `b`: 80 Hz to 16 kHz, evenly in octaves.
pub fn band_hz(b: usize) -> f32 {
    80.0 * 200f32.powf(b as f32 / (BANDS - 1) as f32)
}

struct Layer {
    inp: usize,
    out: usize,
    /// out rows of inp weights
    w: Vec<f32>,
    b: Vec<f32>,
}

/// One instrument's network.
pub struct Net {
    layers: Vec<Layer>,
    /// the instrument's own release (seconds to -60 dB), as its recordings have it
    pub rel: f32,
    /// whether it says its vibrato (`MBN2`)
    vib: bool,
}

impl Net {
    /// An `MBN1` (or `MBN2`) file: the magic, then u32s (inputs, harmonics,
    /// bands, layers), then each layer's u32s (inputs, outputs) and its f32s
    /// (weights, outputs rows of inputs; then biases); little-endian.
    pub fn parse(b: &[u8], rel: f32) -> Result<Net, String> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], String> {
            let s = b.get(at..at + n).ok_or("network: cut short")?;
            at += n;
            Ok(s)
        };
        let vib = match take(4)? {
            m if m == MAGIC => false,
            m if m == MAGIC_VIB => true,
            _ => return Err("not a musicbox network".into()),
        };
        let mut u = || -> Result<usize, String> { Ok(u32::from_le_bytes(take(4)?.try_into().expect("4")) as usize) };
        let (inp, harms, bands, n) = (u()?, u()?, u()?, u()?);
        if (inp, harms, bands) != (3, HARMS, BANDS) || !(1..=8).contains(&n) {
            return Err(format!("network: {inp} in, {harms} harmonics, {bands} bands, {n} layers: not this kind"));
        }
        let mut shapes = Vec::new();
        let mut layers = Vec::new();
        for _ in 0..n {
            let (i, o) = (u()?, u()?);
            shapes.push((i, o));
            if i == 0 || o == 0 || i > 1024 || o > 1024 {
                return Err(format!("network: a layer {i} by {o}"));
            }
            layers.push(Layer { inp: i, out: o, w: Vec::new(), b: Vec::new() });
        }
        let mut f = |n: usize| -> Result<Vec<f32>, String> {
            Ok(take(4 * n)?.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().expect("4"))).collect())
        };
        let mut want = 3;
        for l in layers.iter_mut() {
            if l.inp != want {
                return Err(format!("network: a layer takes {} from {want}", l.inp));
            }
            l.w = f(l.inp * l.out)?;
            l.b = f(l.out)?;
            want = l.out;
        }
        if want != 1 + HARMS + BANDS + 2 * vib as usize {
            return Err(format!("network: {want} out"));
        }
        Ok(Net { layers, rel, vib })
    }

    /// The controls at `t` seconds into a note at `pitch` (fractional),
    /// velocity `vel`: each harmonic's amplitude (none at or past `NYQ`),
    /// each band's, and the vibrato (its depth in cents, its rate in Hz;
    /// none from an `MBN1`).
    pub fn frame(&self, pitch: f32, vel: f32, t: f32, h: &mut [f32; HARMS], n: &mut [f32; BANDS], v: &mut [f32; 2]) {
        let mut x: Vec<f32> =
            vec![(pitch - 60.0) / 24.0, vel, (1.0 + t.clamp(0.0, T_MAX) / T_KNEE).ln() / (1.0 + T_MAX / T_KNEE).ln()];
        let last = self.layers.len() - 1;
        for (k, l) in self.layers.iter().enumerate() {
            let mut y = l.b.clone();
            for (o, yo) in y.iter_mut().enumerate() {
                let row = &l.w[o * l.inp..(o + 1) * l.inp];
                *yo += row.iter().zip(&x).map(|(w, x)| w * x).sum::<f32>();
                if k < last {
                    *yo = yo.max(0.0);
                }
            }
            x = y;
        }
        let f0 = 440.0 * ((pitch - 69.0) / 12.0).exp2();
        let amp = x[0].min(4.0).exp();
        let top = ((NYQ / f0.max(1.0)).ceil() as usize).clamp(1, HARMS + 1) - 1;
        let logits = &x[1..1 + HARMS];
        let most = logits[..top.max(1)].iter().copied().fold(f32::MIN, f32::max);
        let mut sum = 0.0;
        for k in 0..HARMS {
            h[k] = if k < top { (logits[k] - most).exp() } else { 0.0 };
            sum += h[k];
        }
        for v in h.iter_mut() {
            *v *= amp / sum.max(1e-12);
        }
        for (b, v) in n.iter_mut().enumerate() {
            *v = x[1 + HARMS + b].min(4.0).exp();
        }
        *v = if self.vib {
            let y = &x[1 + HARMS + BANDS..];
            [y[0].min(VIB_MAX.ln()).exp(), 4.0 + 4.0 / (1.0 + (-y[1]).exp())]
        } else {
            [0.0, 5.0]
        };
    }
}

/// A band-pass (a state variable filter's, topology-preserving).
#[derive(Clone, Copy, Default)]
struct Bp {
    a1: f32,
    a2: f32,
    a3: f32,
    ic1: f32,
    ic2: f32,
    /// its output's RMS for the noise below, made 1
    norm: f32,
}

impl Bp {
    fn new(hz: f32) -> Bp {
        let g = (std::f32::consts::PI * hz / SR).tan();
        let k = 1.0 / BAND_Q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        Bp { a1, a2: g * a1, a3: g * g * a1, ic1: 0.0, ic2: 0.0, norm: 1.0 }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        v1
    }
}

/// White noise in [-1, 1) (xorshift32, as voices.py has it).
#[inline]
fn white(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    *s as i32 as f32 * (1.0 / 2_147_483_648.0)
}

/// The bands, each filter's RMS for unit noise measured once (2^16 samples
/// after 2^12 to settle) and made 1.
fn bands() -> &'static [Bp; BANDS] {
    static B: OnceLock<[Bp; BANDS]> = OnceLock::new();
    B.get_or_init(|| {
        std::array::from_fn(|b| {
            let mut f = Bp::new(band_hz(b));
            let mut s = 0x2545_F491u32;
            for _ in 0..1 << 12 {
                f.tick(white(&mut s));
            }
            let sum: f64 = (0..1 << 16).map(|_| (f.tick(white(&mut s)) as f64).powi(2)).sum();
            Bp { ic1: 0.0, ic2: 0.0, norm: 1.0 / ((sum / 65536.0).sqrt() as f32).max(1e-9), ..Bp::new(band_hz(b)) }
        })
    })
}

/// A note played by a network.
#[derive(Clone, Copy)]
pub struct NetVoice {
    /// which network (an index into `nets`), and whether it sounds
    pub net: u16,
    pub on: bool,
    vel: f32,
    /// seconds into the note at the next frame, and samples left of this one
    t: f32,
    left: usize,
    /// the controls at this frame's start and its end
    h0: [f32; HARMS],
    h1: [f32; HARMS],
    n0: [f32; BANDS],
    n1: [f32; BANDS],
    /// the vibrato (depth in cents, rate) at the frame's start and end, its phase
    v0: [f32; 2],
    v1: [f32; 2],
    vph: f32,
    phase: f32,
    bp: [Bp; BANDS],
    seed: u32,
}

pub const NET_VOICE: NetVoice = NetVoice {
    net: 0,
    on: false,
    vel: 0.0,
    t: 0.0,
    left: 0,
    h0: [0.0; HARMS],
    h1: [0.0; HARMS],
    n0: [0.0; BANDS],
    n1: [0.0; BANDS],
    v0: [0.0; 2],
    v1: [0.0; 2],
    vph: 0.0,
    phase: 0.0,
    bp: [Bp { a1: 0.0, a2: 0.0, a3: 0.0, ic1: 0.0, ic2: 0.0, norm: 0.0 }; BANDS],
    seed: 1,
};

impl NetVoice {
    /// Starts network `k` on a note (`seed`: its noise).
    pub fn start(&mut self, k: u16, vel: f32, seed: u32) {
        self.net = k;
        self.on = true;
        self.vel = vel;
        self.t = 0.0;
        self.left = 0;
        self.phase = 0.0;
        self.vph = 0.0;
        self.bp = *bands();
        self.seed = seed | 1;
        self.h1 = [0.0; HARMS];
        self.n1 = [0.0; BANDS];
        // the first frame at once, with nothing to ramp from
        self.left = usize::MAX;
    }

    /// The note at `pitch` into `out` (overwritten, the same both sides).
    pub fn fill(&mut self, pitch: f32, out: &mut [[f32; 2]]) {
        match get(self.net) {
            Some(net) => self.fill_by(net, pitch, out),
            None => out.fill([0.0; 2]),
        }
    }
}

impl NetVoice {
    /// `fill`, by `net`.
    fn fill_by(&mut self, net: &Net, pitch: f32, out: &mut [[f32; 2]]) {
        let f0 = 440.0 * ((pitch - 69.0) / 12.0).exp2();
        let dphi = f0 / SR;
        let mut i = 0;
        while i < out.len() {
            if self.left == usize::MAX || self.left == 0 {
                let first = self.left == usize::MAX;
                let (mut h, mut n, mut v) = ([0.0; HARMS], [0.0; BANDS], [0.0; 2]);
                net.frame(pitch, self.vel, self.t, &mut h, &mut n, &mut v);
                if first {
                    // the frame at the note's start, held through its first frame
                    self.h1 = h;
                    self.n1 = n;
                    self.v1 = v;
                    self.t += FRAME as f32 / SR;
                    net.frame(pitch, self.vel, self.t, &mut h, &mut n, &mut v);
                }
                self.h0 = self.h1;
                self.n0 = self.n1;
                self.v0 = self.v1;
                self.h1 = h;
                self.n1 = n;
                self.v1 = v;
                self.t += FRAME as f32 / SR;
                self.left = FRAME;
            }
            let m = self.left.min(out.len() - i);
            for o in out[i..i + m].iter_mut() {
                let r = (FRAME - self.left) as f32 / FRAME as f32;
                self.left -= 1;
                // sin(k phi) for every k from sin and cos of phi
                let (s1, c1) = (std::f32::consts::TAU * self.phase).sin_cos();
                let (mut sp, mut s) = (0.0f32, s1);
                let mut x = 0.0;
                for k in 0..HARMS {
                    let a = self.h0[k] + (self.h1[k] - self.h0[k]) * r;
                    if a == 0.0 && self.h0[k] == 0.0 {
                        break;
                    }
                    x += a * s;
                    let next = 2.0 * c1 * s - sp;
                    sp = s;
                    s = next;
                }
                let w = white(&mut self.seed);
                for (b, f) in self.bp.iter_mut().enumerate() {
                    let a = self.n0[b] + (self.n1[b] - self.n0[b]) * r;
                    x += a * f.norm * f.tick(w);
                }
                // the vibrato: 2^(cents / 1200), to second order
                let depth = self.v0[0] + (self.v1[0] - self.v0[0]) * r;
                let mut step = dphi;
                if depth > 0.0 {
                    self.vph += (self.v0[1] + (self.v1[1] - self.v0[1]) * r) / SR;
                    self.vph -= self.vph.floor();
                    let y = depth * (std::f32::consts::TAU * self.vph).sin() * (std::f32::consts::LN_2 / 1200.0);
                    step *= 1.0 + y + 0.5 * y * y;
                }
                self.phase += step;
                self.phase -= self.phase.floor();
                *o = [x, x];
            }
            i += m;
        }
    }
}

/// The networks loaded, and which bank instrument each plays (by id).
#[derive(Default)]
pub struct Nets {
    nets: Vec<Net>,
    by_id: Vec<(u16, u16)>,
}

impl Nets {
    /// `net` playing the bank's instrument `id`.
    pub fn add(&mut self, id: u16, net: Net) {
        self.by_id.retain(|&(i, _)| i != id);
        self.nets.push(net);
        self.by_id.push((id, self.nets.len() as u16 - 1));
    }

    pub fn len(&self) -> usize {
        self.nets.len()
    }
}

/// A line of a networks' index (index.tsv): its kind ("multi" or "gm"),
/// the instrument (or preset), the articulation (empty for a preset), and
/// the network's file. None for a heading, a comment or a blank line.
pub fn index_line(line: &str) -> Option<(&str, &str, &str, &str)> {
    let f: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
    match f.as_slice() {
        [kind, name, art, file] if matches!(*kind, "multi" | "gm") => Some((kind, name, art, file)),
        _ => None,
    }
}

/// Where the networks are: `MUSICBOX_MODELS`, else
/// ~/.local/share/musicbox/models, else the source tree's models/.
#[cfg(not(target_arch = "wasm32"))]
pub fn default_dir() -> std::path::PathBuf {
    use std::path::{Path, PathBuf};
    if let Some(d) = std::env::var_os("MUSICBOX_MODELS") {
        return PathBuf::from(d);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let own = Path::new(&home).join(".local/share/musicbox/models");
    if own.is_dir() { own } else { Path::new(env!("CARGO_MANIFEST_DIR")).join("models") }
}

/// The networks in `dir` for the instruments `bank` has, installed: how
/// many, and a word on each that did not load.
#[cfg(not(target_arch = "wasm32"))]
pub fn load(dir: &std::path::Path, bank: &crate::samples::Bank) -> (usize, Vec<String>) {
    let mut nets = Nets::default();
    let mut skipped = Vec::new();
    let Ok(text) = std::fs::read_to_string(dir.join("index.tsv")) else {
        return (0, skipped);
    };
    for (kind, name, art, file) in text.lines().filter_map(index_line) {
        let id = if kind == "multi" { bank.multi(name, art) } else { bank.gm(name) };
        let Some(id) = id else {
            skipped.push(format!("{kind} {name} {art}: not in the samples"));
            continue;
        };
        match std::fs::read(dir.join(file)).map_err(|e| e.to_string()).and_then(|b| Net::parse(&b, bank.release(id))) {
            Ok(n) => nets.add(id, n),
            Err(e) => skipped.push(format!("{file}: {e}")),
        }
    }
    let n = nets.len();
    install(nets);
    (n, skipped)
}

#[cfg(not(target_arch = "wasm32"))]
static NETS: OnceLock<Nets> = OnceLock::new();

/// Makes `nets` the ones voices play (the first installed stays).
#[cfg(not(target_arch = "wasm32"))]
pub fn install(nets: Nets) {
    let _ = NETS.set(nets);
}

#[cfg(not(target_arch = "wasm32"))]
fn nets() -> Option<&'static Nets> {
    NETS.get()
}

/// The browser's: added to as they come, on the worker's one thread.
#[cfg(target_arch = "wasm32")]
struct WebNets(std::cell::UnsafeCell<Nets>);

// SAFETY: wasm32-unknown-unknown runs one thread
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WebNets {}

#[cfg(target_arch = "wasm32")]
static WEB: WebNets = WebNets(std::cell::UnsafeCell::new(Nets { nets: Vec::new(), by_id: Vec::new() }));

#[cfg(target_arch = "wasm32")]
fn nets() -> Option<&'static Nets> {
    // SAFETY: one thread; changed only between calls into the module (`web`)
    Some(unsafe { &*WEB.0.get() })
}

/// The browser's networks, to add to.
///
/// # Safety
/// Only between calls into the module that play or compose.
#[cfg(target_arch = "wasm32")]
pub unsafe fn web() -> &'static mut Nets {
    // SAFETY: the caller's
    unsafe { &mut *WEB.0.get() }
}

/// The network that plays bank instrument `id`, if one does (its index).
pub fn find(id: u16) -> Option<u16> {
    nets()?.by_id.iter().find(|x| x.0 == id).map(|x| x.1)
}

fn get(k: u16) -> Option<&'static Net> {
    nets()?.nets.get(k as usize)
}

/// Network `k`'s release (seconds to -60 dB).
pub fn rel(k: u16) -> f32 {
    get(k).map_or(0.2, |n| n.rel)
}

/// How many networks are loaded (in the browser's window, where none are:
/// how many the page has to fetch when they are chosen).
pub fn count() -> usize {
    nets().map_or(0, |n| n.len()).max(LISTED.load(std::sync::atomic::Ordering::Relaxed))
}

/// The networks the page lists (models/index.tsv), fetched when chosen.
static LISTED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(target_arch = "wasm32")]
pub fn listed(n: usize) {
    LISTED.store(n, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A network of one layer, its weights nothing: the fundamental alone
    /// at amplitude `a`, and band `b` at `n` (the rest of the noise none).
    fn flat(a: f32, b: usize, n: f32) -> Vec<u8> {
        let mut v = MAGIC.to_vec();
        let out = 1 + HARMS + BANDS;
        for x in [3, HARMS, BANDS, 1, 3, out] {
            v.extend_from_slice(&(x as u32).to_le_bytes());
        }
        v.extend((0..3 * out).flat_map(|_| 0f32.to_le_bytes()));
        let mut bias = vec![a.ln()];
        bias.extend((0..HARMS).map(|k| if k == 0 { 10.0 } else { -30.0 }));
        bias.extend((0..BANDS).map(|k| if k == b { n.ln() } else { -30.0 }));
        v.extend(bias.iter().flat_map(|x| x.to_le_bytes()));
        v
    }

    #[test]
    fn a_network_with_vibrato_wavers_as_it_says() {
        // the fundamental at 0.1, vibrato 50 cents (ln 50) at 6 Hz (sigmoid 0.5)
        let mut v = flat(0.1, 0, 1e-9);
        v[..4].copy_from_slice(MAGIC_VIB);
        let out = 1 + HARMS + BANDS;
        // the one layer 3 by out, made 3 by out + 2: its outputs' count, weights, biases
        let at = 4 + 4 * 4 + 4;
        v[at..at + 4].copy_from_slice(&((out + 2) as u32).to_le_bytes());
        let w_end = at + 4 + 4 * 3 * out;
        v.splice(w_end..w_end, (0..3 * 2).flat_map(|_| 0f32.to_le_bytes()));
        v.extend(50f32.ln().to_le_bytes());
        v.extend(0f32.to_le_bytes());
        let net = Net::parse(&v, 0.2).expect("parse");
        let (mut h, mut n, mut vib) = ([0.0; HARMS], [0.0; BANDS], [0.0; 2]);
        net.frame(69.0, 0.8, 1.0, &mut h, &mut n, &mut vib);
        assert!((vib[0] - 50.0).abs() < 0.01 && (vib[1] - 6.0).abs() < 0.01, "{vib:?}");
        let mut nets = Nets::default();
        nets.add(3, net);
        let k = nets.len() as u16 - 1;
        let mut voice = NET_VOICE;
        // (its own table, not installed: the other test installs one)
        voice.start(k, 0.8, 9);
        let mut buf = vec![[0.0f32; 2]; 48000];
        let net = &nets.nets[k as usize];
        // the oscillator's frequency through a second, by its periods
        let (mut hh, mut nn, mut vv) = ([0.0; HARMS], [0.0; BANDS], [0.0; 2]);
        net.frame(69.0, 0.8, 0.0, &mut hh, &mut nn, &mut vv);
        voice.h0 = hh;
        voice.h1 = hh;
        voice.v0 = vv;
        voice.v1 = vv;
        voice.left = FRAME;
        let mut x = Vec::new();
        for o in buf.iter_mut() {
            // (one sample at a time, the controls held: the vibrato alone)
            if voice.left == 0 {
                voice.left = FRAME;
            }
            let mut one = [[0.0f32; 2]; 1];
            voice.fill_by(net, 69.0, &mut one);
            *o = one[0];
            x.push(one[0][0]);
        }
        let ups: Vec<usize> = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).collect();
        let f: Vec<f32> = ups.windows(2).map(|w| 48000.0 / (w[1] - w[0]) as f32).collect();
        let (lo, hi) = (f.iter().copied().fold(f32::MAX, f32::min), f.iter().copied().fold(0.0, f32::max));
        let cents = 1200.0 * (hi / lo).log2();
        assert!((cents - 100.0).abs() < 15.0, "{cents:.0} cents from lowest to highest, not 100");
    }

    fn rms(x: &[[f32; 2]]) -> f32 {
        (x.iter().map(|s| s[0] * s[0]).sum::<f32>() / x.len() as f32).sqrt()
    }

    #[test]
    fn a_network_plays_what_it_says() {
        let net = Net::parse(&flat(0.1, 0, 1e-9), 0.3).expect("parse");
        let mut nets = Nets::default();
        nets.add(7, net);
        nets.add(9, Net::parse(&flat(1e-9, 6, 0.05), 0.3).expect("parse"));
        install(nets);
        assert_eq!(find(7), Some(0));
        assert_eq!(rel(0), 0.3);
        let mut v = NET_VOICE;
        v.start(0, 0.8, 5);
        let mut out = vec![[0.0f32; 2]; 24000];
        for c in out.chunks_mut(64) {
            v.fill(69.0, c);
        }
        let ups = (1..out.len()).filter(|&i| out[i - 1][0] < 0.0 && out[i][0] >= 0.0).count();
        assert!((ups as f32 - 220.0).abs() <= 1.0, "{ups} cycles in half a second, not 220");
        let r = rms(&out);
        assert!((r - 0.1 / 2f32.sqrt()).abs() < 0.002, "RMS {r}");
        // a band of noise at its amplitude
        let mut v = NET_VOICE;
        v.start(1, 0.8, 5);
        for c in out.chunks_mut(64) {
            v.fill(69.0, c);
        }
        let r = rms(&out[4800..]);
        assert!((r - 0.05).abs() < 0.005, "noise RMS {r}");
        // a malformed file refused
        assert!(Net::parse(&flat(0.1, 0, 0.1)[..40], 0.2).is_err());
    }
}
