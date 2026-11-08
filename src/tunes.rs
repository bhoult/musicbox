//! Tunes by a network: a small recurrent network (a GRU) that learned the
//! dance tunes of O'Neill's Music of Ireland (1903) as the O'Neill's Project
//! transcribed them (GPL-2.0-or-later; tools/neural/tunes.py, credits in
//! tools/neural/TUNES.md). It writes a part (eight bars) a note at a time:
//! how long the next note is, then its pitch, from the notes before it, where
//! it falls in the bar and the part, the chord under it, the tune's type and
//! mode. The genre's rules still choose everything else: the chords, the
//! form, the key, the tempo, the band.
//!
//! What a network sees and says (`MBT1` files, as tunes.py trains them),
//! for each note in turn:
//! - in (`FEATS`): the tune type (`TYPES`), the mode (ionian, dorian,
//!   mixolydian, aeolian), the part (A 0, B 1), the bar (0-7), the 16th it
//!   starts on (0-15), the chord there (its root, semitones above the tonic,
//!   and whether it is minor), the note before it (its pitch, `PITCHES`, and
//!   length, `DURS`; none for the first);
//! - out: the length's logits (1 to `DURS` 16ths), then the pitch's
//!   (semitones above the tonic, `LOW` to `LOW + PITCHES - 2`, or a rest),
//!   the pitch head seeing the length chosen.
//!
//! The arithmetic is plain sums and products, with its own exp, so a seed
//! writes the same tune on any machine (the desktop's and the browser's).

pub const TYPES: usize = 6;
pub const DURS: usize = 12;
pub const PITCHES: usize = 40;
pub const LOW: i32 = -12;
pub const REST: usize = PITCHES - 1;
pub const FEATS: usize = TYPES + 4 + 1 + 8 + 16 + 13 + PITCHES + DURS;
const MAGIC: &[u8; 4] = b"MBT1";

/// The tune types the network knows: a reel, a jig, a hornpipe, a 2/4
/// march, a 6/8 march, a slow air (or planxty, 3/4).
pub const T_REEL: usize = 0;
pub const T_JIG: usize = 1;
pub const T_HORNPIPE: usize = 2;
pub const T_MARCH: usize = 3;
pub const T_MARCH68: usize = 4;
pub const T_AIR: usize = 5;

/// e^x, the same everywhere: range reduction by ln 2, a degree-6
/// polynomial on what is left, then the power of two put back by hand.
pub fn exp(x: f32) -> f32 {
    let x = x.clamp(-87.0, 88.0);
    let k = (x * std::f32::consts::LOG2_E + 0.5).floor();
    let r = x - k * 0.693_145_75 - k * 1.428_606_8e-6;
    let p = 1.0 + r * (1.0 + r * (0.5 + r * (1.0 / 6.0 + r * (1.0 / 24.0 + r * (1.0 / 120.0 + r * (1.0 / 720.0))))));
    p * f32::from_bits(((k as i32 + 127) as u32) << 23)
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + exp(-x))
}

fn tanh(x: f32) -> f32 {
    2.0 * sigmoid(2.0 * x) - 1.0
}

/// A dense layer: rows of weights, one a output, then the biases.
struct Dense {
    inp: usize,
    w: Vec<f32>,
    b: Vec<f32>,
}

impl Dense {
    fn apply(&self, x: &[f32], y: &mut [f32]) {
        for (o, yo) in y.iter_mut().enumerate() {
            let row = &self.w[o * self.inp..(o + 1) * self.inp];
            *yo = self.b[o] + row.iter().zip(x).map(|(w, x)| w * x).sum::<f32>();
        }
    }
}

/// A GRU layer (PyTorch's: the gates' rows reset, update, new).
struct Gru {
    hid: usize,
    ih: Dense,
    hh: Dense,
}

impl Gru {
    fn step(&self, x: &[f32], h: &mut [f32]) {
        let n = self.hid;
        let (mut a, mut b) = (vec![0.0; 3 * n], vec![0.0; 3 * n]);
        self.ih.apply(x, &mut a);
        self.hh.apply(h, &mut b);
        for k in 0..n {
            let r = sigmoid(a[k] + b[k]);
            let z = sigmoid(a[n + k] + b[n + k]);
            let c = tanh(a[2 * n + k] + r * b[2 * n + k]);
            h[k] = (1.0 - z) * c + z * h[k];
        }
    }
}

/// A tune network.
pub struct TuneNet {
    layers: Vec<Gru>,
    dur: Dense,
    pitch: Dense,
}

impl TuneNet {
    /// An `MBT1` file: the magic; u32s (features, hidden size, layers);
    /// each layer's input weights (3 hidden rows of its inputs), hidden
    /// weights (3 hidden rows of hidden), input biases, hidden biases; the
    /// length head (`DURS` rows of hidden, biases); the pitch head
    /// (`PITCHES` rows of hidden + `DURS`, biases); f32s, little-endian.
    pub fn parse(b: &[u8]) -> Result<TuneNet, String> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], String> {
            let s = b.get(at..at + n).ok_or("tune network: cut short")?;
            at += n;
            Ok(s)
        };
        if take(4)? != MAGIC {
            return Err("not a musicbox tune network".into());
        }
        let mut words = [0usize; 3];
        for w in words.iter_mut() {
            *w = u32::from_le_bytes(take(4)?.try_into().expect("4")) as usize;
        }
        let [feats, hid, n] = words;
        if feats != FEATS || !(1..=1024).contains(&hid) || !(1..=4).contains(&n) {
            return Err(format!("tune network: {feats} features, {hid} hidden, {n} layers: not this kind"));
        }
        let mut f = |len: usize| -> Result<Vec<f32>, String> {
            Ok(take(4 * len)?.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().expect("4"))).collect())
        };
        let mut layers = Vec::new();
        for l in 0..n {
            let inp = if l == 0 { feats } else { hid };
            let (wi, wh) = (f(3 * hid * inp)?, f(3 * hid * hid)?);
            let (bi, bh) = (f(3 * hid)?, f(3 * hid)?);
            layers.push(Gru { hid, ih: Dense { inp, w: wi, b: bi }, hh: Dense { inp: hid, w: wh, b: bh } });
        }
        let dur = Dense { inp: hid, w: f(DURS * hid)?, b: f(DURS)? };
        let pitch = Dense { inp: hid + DURS, w: f(PITCHES * (hid + DURS))?, b: f(PITCHES)? };
        Ok(TuneNet { layers, dur, pitch })
    }

    /// A note's step: the layers' states `h` on from the features `x`;
    /// the length's logits.
    fn step(&self, h: &mut [Vec<f32>], x: &[f32]) -> Vec<f32> {
        let mut inp = x.to_vec();
        for (l, g) in self.layers.iter().enumerate() {
            g.step(&inp, &mut h[l]);
            inp = h[l].clone();
        }
        let mut dl = vec![0.0; DURS];
        self.dur.apply(&h[h.len() - 1], &mut dl);
        dl
    }

    /// The pitch's logits, the length `d` (0-based) chosen.
    fn pitch_logits(&self, h: &[Vec<f32>], d: usize) -> Vec<f32> {
        let mut ph = h[h.len() - 1].clone();
        ph.extend((0..DURS).map(|k| if k == d { 1.0 } else { 0.0 }));
        let mut pl = vec![0.0; PITCHES];
        self.pitch.apply(&ph, &mut pl);
        pl
    }

    /// Eight bars of a tune of type `ty` (`bar` 16ths a bar), its mode
    /// (0 ionian, 1 dorian, 2 mixolydian, 3 aeolian) and part (0 A, 1 B), over
    /// `chords` (two a bar: root above the tonic, minor or not), its pitches
    /// within `lo..=hi`: (start in 16ths, length, pitch; None a rest). `u`
    /// gives the dice (each in [0, 1)); `heat` how freely it chooses (1: as it
    /// learned).
    #[allow(clippy::too_many_arguments)]
    pub fn part(
        &self,
        ty: usize,
        bar: u16,
        mode: usize,
        part: usize,
        chords: &[(i32, bool)],
        lo: i32,
        hi: i32,
        heat: f32,
        mut u: impl FnMut() -> f32,
    ) -> Vec<(u16, u16, Option<i32>)> {
        let hid = self.layers[0].hid;
        let mut h = vec![vec![0.0f32; hid]; self.layers.len()];
        let mut x = vec![0.0f32; FEATS];
        let mut out = Vec::new();
        let (mut at, end) = (0u16, 8 * bar);
        let mut prev: Option<(usize, usize)> = None;
        while at < end {
            let (b, s) = (at / bar, at % bar);
            let (root, minor) = chords
                .get((b as usize * 2 + (2 * s / bar) as usize).min(chords.len().max(1) - 1))
                .copied()
                .unwrap_or((0, false));
            features(&mut x, ty, mode, part, b as usize, s as usize, root, minor, prev);
            // the length: none past the bar's end
            let dl = self.step(&mut h, &x);
            let room = (bar - s) as usize;
            let d = pick(&dl, heat, &mut u, |k| k < room);
            // the pitch, knowing the length: within the range (a rest only in an air)
            let pl = self.pitch_logits(&h, d);
            let p = pick(&pl, heat, &mut u, |k| {
                if k == REST {
                    ty == T_AIR
                } else {
                    let v = LOW + k as i32;
                    v >= lo && v <= hi
                }
            });
            out.push((at, d as u16 + 1, (p != REST).then_some(LOW + p as i32)));
            prev = Some((p, d));
            at += d as u16 + 1;
        }
        out
    }
}

/// A note's features (`FEATS`, as the module's head lists them) into `x`.
#[allow(clippy::too_many_arguments)]
pub fn features(
    x: &mut [f32],
    ty: usize,
    mode: usize,
    part: usize,
    bar: usize,
    step: usize,
    root: i32,
    minor: bool,
    prev: Option<(usize, usize)>,
) {
    x.fill(0.0);
    let mut o = 0;
    for (k, n) in [(ty, TYPES), (mode, 4)] {
        x[o + k.min(n - 1)] = 1.0;
        o += n;
    }
    x[o] = part.min(1) as f32;
    o += 1;
    for (k, n) in [(bar, 8), (step, 16), (root.rem_euclid(12) as usize, 12)] {
        x[o + k.min(n - 1)] = 1.0;
        o += n;
    }
    x[o] = minor as u8 as f32;
    o += 1;
    if let Some((p, d)) = prev {
        x[o + p.min(PITCHES - 1)] = 1.0;
        x[o + PITCHES + d.min(DURS - 1)] = 1.0;
    }
}

/// An index by `logits` at `heat`, among those `ok` allows (the most likely
/// of them if the dice find none).
fn pick(logits: &[f32], heat: f32, u: &mut impl FnMut() -> f32, ok: impl Fn(usize) -> bool) -> usize {
    let most = (0..logits.len()).filter(|&k| ok(k)).map(|k| logits[k]).fold(f32::MIN, f32::max);
    let w: Vec<f32> =
        (0..logits.len()).map(|k| if ok(k) { exp((logits[k] - most) / heat.max(0.05)) } else { 0.0 }).collect();
    let sum: f32 = w.iter().sum();
    if sum.is_nan() || sum <= 0.0 {
        return (0..logits.len()).find(|&k| ok(k)).unwrap_or(0);
    }
    let mut x = u() * sum;
    for (k, &wk) in w.iter().enumerate() {
        if wk > 0.0 && x < wk {
            return k;
        }
        x -= wk;
    }
    (0..logits.len()).rev().find(|&k| w[k] > 0.0).unwrap_or(0)
}

/// The tune networks loaded: one for celtic (O'Neill's) so far.
#[derive(Default)]
pub struct Tunes {
    pub celtic: Option<TuneNet>,
}

/// The tunes networks in `dir`'s index (index.tsv: "tunes", the genre, an
/// empty articulation, the file), installed: a word on each that did not
/// load.
#[cfg(not(target_arch = "wasm32"))]
pub fn load(dir: &std::path::Path) -> Vec<String> {
    let mut t = Tunes::default();
    let mut skipped = Vec::new();
    if let Ok(text) = std::fs::read_to_string(dir.join("index.tsv")) {
        for line in text.lines() {
            let f: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
            if let ["tunes", genre, _, file] = f.as_slice() {
                match (
                    *genre,
                    std::fs::read(dir.join(file)).map_err(|e| e.to_string()).and_then(|b| TuneNet::parse(&b)),
                ) {
                    ("celtic", Ok(n)) => t.celtic = Some(n),
                    (g, Ok(_)) => skipped.push(format!("{file}: no genre {g} takes tunes from a network")),
                    (_, Err(e)) => skipped.push(format!("{file}: {e}")),
                }
            }
        }
    }
    install(t);
    skipped
}

#[cfg(not(target_arch = "wasm32"))]
static TUNES: std::sync::OnceLock<Tunes> = std::sync::OnceLock::new();

/// Makes `t` the tunes networks the genres use (the first installed stays).
#[cfg(not(target_arch = "wasm32"))]
pub fn install(t: Tunes) {
    let _ = TUNES.set(t);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get() -> Option<&'static Tunes> {
    TUNES.get()
}

/// The browser's, set between calls on the worker's one thread.
#[cfg(target_arch = "wasm32")]
struct WebTunes(std::cell::UnsafeCell<Tunes>);

// SAFETY: wasm32-unknown-unknown runs one thread
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WebTunes {}

#[cfg(target_arch = "wasm32")]
static WEB: WebTunes = WebTunes(std::cell::UnsafeCell::new(Tunes { celtic: None }));

#[cfg(target_arch = "wasm32")]
pub fn get() -> Option<&'static Tunes> {
    // SAFETY: one thread; changed only between calls into the module (`web`)
    Some(unsafe { &*WEB.0.get() })
}

/// The browser's tunes networks, to set.
///
/// # Safety
/// Only between calls into the module that play or compose.
#[cfg(target_arch = "wasm32")]
pub unsafe fn web() -> &'static mut Tunes {
    // SAFETY: the caller's
    unsafe { &mut *WEB.0.get() }
}

/// The celtic tunes network, if one is loaded.
pub fn celtic() -> Option<&'static TuneNet> {
    get()?.celtic.as_ref()
}

/// Whether there is a celtic tunes network: loaded, or (in the browser's
/// window, where none is) one the page lists, to fetch when it is chosen.
pub fn has_celtic() -> bool {
    celtic().is_some() || LISTED.load(std::sync::atomic::Ordering::Relaxed)
}

static LISTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_arch = "wasm32")]
pub fn listed(on: bool) {
    LISTED.store(on, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_exp_is_exp() {
        for k in -400..400 {
            let x = k as f32 * 0.21;
            let (a, b) = (exp(x), x.exp());
            assert!(((a - b) / b).abs() < 2e-6, "exp({x}) {a} against {b}");
        }
        assert!((tanh(0.5) - 0.5f32.tanh()).abs() < 1e-6);
    }

    /// A network of one layer, its weights nothing: the length its biases
    /// favour (a 16th more each), the pitch likewise.
    fn flat(hid: usize, dur: usize, pitch: usize) -> Vec<u8> {
        let mut v = MAGIC.to_vec();
        for x in [FEATS, hid, 1] {
            v.extend_from_slice(&(x as u32).to_le_bytes());
        }
        let zeros = |n: usize| (0..n).flat_map(|_| 0f32.to_le_bytes()).collect::<Vec<u8>>();
        v.extend(zeros(3 * hid * FEATS + 3 * hid * hid + 6 * hid));
        v.extend(zeros(DURS * hid));
        v.extend((0..DURS).flat_map(|k| (if k == dur { 9.0f32 } else { 0.0 }).to_le_bytes()));
        v.extend(zeros(PITCHES * (hid + DURS)));
        v.extend((0..PITCHES).flat_map(|k| (if k == pitch { 9.0f32 } else { 0.0 }).to_le_bytes()));
        v
    }

    /// The shipped network's first steps as PyTorch computed them
    /// (models/tunes_celtic.check, from tools/neural/tunes.py).
    #[test]
    fn the_shipped_network_computes_as_it_was_trained() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models");
        let (Ok(model), Ok(check)) =
            (std::fs::read(dir.join("tunes_celtic.mbt")), std::fs::read_to_string(dir.join("tunes_celtic.check")))
        else {
            return;
        };
        let net = TuneNet::parse(&model).expect("parse");
        let mut h = vec![vec![0.0f32; net.layers[0].hid]; net.layers.len()];
        let mut x = vec![0.0f32; FEATS];
        let nums = |s: &str| s.split_whitespace().map(|v| v.parse::<f32>().expect("a number")).collect::<Vec<f32>>();
        let mut steps = 0;
        for line in check.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('|').collect();
            let a = nums(f[0]);
            let prev = (a[7] >= 0.0).then_some((a[7] as usize, a[8] as usize));
            let u = |k: usize| a[k] as usize;
            features(&mut x, u(0), u(1), u(2), u(3), u(4), a[5] as i32, a[6] != 0.0, prev);
            let close = |got: &[f32], want: &[f32], what: &str| {
                for (k, (g, w)) in got.iter().zip(want).enumerate() {
                    assert!((g - w).abs() < 1e-4 * w.abs().max(1.0), "step {steps} {what} {k}: {g} not {w}");
                }
            };
            close(&net.step(&mut h, &x), &nums(f[1]), "length");
            close(&net.pitch_logits(&h, f[2].trim().parse().expect("index")), &nums(f[3]), "pitch");
            steps += 1;
        }
        assert_eq!(steps, 3);
    }

    #[test]
    fn a_part_fills_eight_bars_in_range() {
        // eighths (a length of 2) on the tonic, a jig's 12 16ths a bar
        let net = TuneNet::parse(&flat(8, 1, (-LOW) as usize)).expect("parse");
        let mut s = 7u64;
        let mut u = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 40) as f32 / (1u64 << 24) as f32
        };
        let chords = vec![(0, false); 16];
        let notes = net.part(T_JIG, 12, 0, 0, &chords, -3, 12, 1.0, &mut u);
        let total: u16 = notes.iter().map(|n| n.1).sum();
        assert_eq!(total, 96, "eight bars");
        assert!(notes.iter().all(|n| (n.0 % 12) + n.1 <= 12), "no note across a bar line");
        assert!(notes.iter().filter(|n| n.2 == Some(0) && n.1 == 2).count() as f32 > 0.8 * notes.len() as f32);
        // the range kept: a tonic outside it is never chosen
        let notes = net.part(T_JIG, 12, 0, 0, &chords, 2, 9, 1.0, &mut u);
        assert!(notes.iter().all(|n| n.2.is_some_and(|p| (2..=9).contains(&p))));
        assert!(TuneNet::parse(&flat(8, 1, 12)[..100]).is_err());
    }
}
