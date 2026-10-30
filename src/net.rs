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
//!   band's noise amplitude (its RMS).
//!
//! The sound, sample by sample, the frames' values ramped between them:
//! sum over k of a_k sin(k phi), phi the note's phase; plus, for each band,
//! its amplitude times white noise through that band's band-pass (a state
//! variable filter at `band_hz(b)`, Q `BAND_Q`), scaled to an RMS of 1.

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
}

impl Net {
    /// An `MBN1` file: the magic, then u32s (inputs, harmonics, bands,
    /// layers), then each layer's u32s (inputs, outputs) and its f32s
    /// (weights, outputs rows of inputs; then biases); little-endian.
    pub fn parse(b: &[u8], rel: f32) -> Result<Net, String> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], String> {
            let s = b.get(at..at + n).ok_or("network: cut short")?;
            at += n;
            Ok(s)
        };
        if take(4)? != MAGIC {
            return Err("not a musicbox network".into());
        }
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
        if want != 1 + HARMS + BANDS {
            return Err(format!("network: {want} out"));
        }
        Ok(Net { layers, rel })
    }

    /// The controls at `t` seconds into a note at `pitch` (fractional),
    /// velocity `vel`: each harmonic's amplitude (none at or past `NYQ`) and
    /// each band's.
    pub fn frame(&self, pitch: f32, vel: f32, t: f32, h: &mut [f32; HARMS], n: &mut [f32; BANDS]) {
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
        self.bp = *bands();
        self.seed = seed | 1;
        self.h1 = [0.0; HARMS];
        self.n1 = [0.0; BANDS];
        // the first frame at once, with nothing to ramp from
        self.left = usize::MAX;
    }

    /// The note at `pitch` into `out` (overwritten, the same both sides).
    pub fn fill(&mut self, pitch: f32, out: &mut [[f32; 2]]) {
        let Some(net) = get(self.net) else {
            out.fill([0.0; 2]);
            return;
        };
        let f0 = 440.0 * ((pitch - 69.0) / 12.0).exp2();
        let dphi = f0 / SR;
        let mut i = 0;
        while i < out.len() {
            if self.left == usize::MAX || self.left == 0 {
                let first = self.left == usize::MAX;
                let (mut h, mut n) = ([0.0; HARMS], [0.0; BANDS]);
                net.frame(pitch, self.vel, self.t, &mut h, &mut n);
                if first {
                    // the frame at the note's start, held through its first frame
                    self.h1 = h;
                    self.n1 = n;
                    self.t += FRAME as f32 / SR;
                    net.frame(pitch, self.vel, self.t, &mut h, &mut n);
                }
                self.h0 = self.h1;
                self.n0 = self.n1;
                self.h1 = h;
                self.n1 = n;
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
                self.phase += dphi;
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

/// How many networks are loaded.
pub fn count() -> usize {
    nets().map_or(0, |n| n.len())
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
