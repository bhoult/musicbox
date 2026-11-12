//! More variety for a genre's songs (`Opts::varied`, a song id's `v`): what
//! the styled genres' pieces draw for themselves when they are written so,
//! and the more they have to draw on. A piece draws its drums' lines (the
//! style's own, a pair written for it here, each plain line maybe another of
//! its kind), how its bass, chords, arpeggio and second voice go (the
//! style's way, or another that suits its instruments), who plays some of
//! its parts, its chords from a longer list, and its form's turns (a long
//! first verse, a half verse, a build or a breath before the last chorus,
//! that chorus a key up). As it plays, a fill takes one of a few shapes, a
//! section's even bars turn the beat a little, a part sits out half a
//! section, a verse or a chorus come round again has a cell of its tune
//! written anew, a hook is not one bar eight times, and the last chorus may
//! open without its drums and double its tune an octave up.
//!
//! All of it is behind the option: a piece made without it draws nothing
//! here, and is the piece it always was.

use super::*;

/// The rows a dozen styles share: a piece written with more variety draws
/// them half as often.
pub(super) const COMMON_ROWS: [&str; 3] =
    ["I V vi IV I V vi IV", "vi IV I V vi IV I V", "i bVI bIII bVII i bVI bIII bVII"];

/// A bar's turn of the beat (`Vary::orn`'s kind): a kick more, a ghost on
/// the snare, the hats resting through the last half beat, the half bar's
/// kick left out.
pub(super) const ORN_KICK: u8 = 1;
pub(super) const ORN_GHOST: u8 = 2;
pub(super) const ORN_REST: u8 = 3;
pub(super) const ORN_LESS: u8 = 4;

/// What a piece written with more variety draws for itself.
#[derive(Clone, Default)]
pub(super) struct Vary {
    /// the drums' lines it plays (A, B), and a mix's styled drummer's
    pub(super) lanes: [Vec<Lane>; 2],
    pub(super) dlanes: [Vec<Lane>; 2],
    /// how its bass, chords, arpeggio and second voice go
    pub(super) ways: Option<(u8, u8, u8, u8)>,
    /// the section's tune, when it is not the piece's as first written: which
    /// of the piece's it stands for, and its notes
    pub(super) tune: Option<(usize, Vec<MNote>)>,
    /// the section's turn of the beat on its even bars (a kind, and which of
    /// its places), the layers sitting out its first and second halves,
    /// whether it opens without its drums, and whether its tune is doubled
    /// an octave up
    pub(super) orn: (u8, u8),
    pub(super) out: [u32; 2],
    pub(super) down: bool,
    pub(super) lift: bool,
    /// how often each part has come
    seen: [u8; 8],
}

/// What a style has more of, for a piece written with more variety: chords
/// (after its own), pairs of drum lines, other ways for its bass, chords,
/// arpeggio and second voice, and other players for some parts (one a
/// piece); whether its kit is a drummer's (ghost notes suit it), whether a
/// riser's build may lead into its last chorus, and whether that chorus may
/// go up a key.
pub(super) struct Extra {
    pub(super) maj: &'static [&'static str],
    pub(super) min: &'static [&'static str],
    pub(super) grooves: &'static [[&'static [Lane]; 2]],
    pub(super) bass: &'static [u8],
    pub(super) comp: &'static [u8],
    pub(super) arp: &'static [u8],
    pub(super) second: &'static [u8],
    pub(super) alts: &'static [(usize, Pal)],
    pub(super) acoustic: bool,
    pub(super) build: bool,
    pub(super) lift: bool,
}

const X: Extra = Extra {
    maj: &[],
    min: &[],
    grooves: &[],
    bass: &[],
    comp: &[],
    arp: &[],
    second: &[],
    alts: &[],
    acoustic: false,
    build: false,
    lift: false,
};

// ---------------------------------------------------------------- lines of a kind

/// Kicks under a backbeat: a verse's, and a chorus's busier ones.
const KICK_A: &[&str] = &[
    "x.......x.......",
    "x.......x.x.....",
    "x.....x.x.......",
    "x......x..x.....",
    "x..x....x.......",
    "x.......x..x....",
    "x.x.....x.......",
    "x.....x...x.....",
    "x..x....x.x.....",
];
const KICK_B: &[&str] = &[
    "x.....x.x.x...x.",
    "x.......x.x.....",
    "x.....x.x.......",
    "x.x...x.x.....x.",
    "x..x..x.x..x....",
    "x......x..x..x..",
    "x.x.....x.x.....",
    "x..x....x.x...x.",
    "x.....x.x..x..x.",
];
/// The backbeat, plain and with ghosts; and accented.
const BACK: &[&str] =
    &["....x.......x...", "....x..o....x...", "....x.......x..o", "....x..o.o..x...", "....x.....o.x..."];
const BACK_X: &[&str] = &["....X.......X...", "....X..o....X...", "....X.......X..o", "....X.....o.X..."];
/// Hats in eighths; in 16ths; at a slow tempo the eighths may be 16ths, at
/// a fast one quarters.
const HAT_8: &[&str] = &["x.-.x.-.x.-.x.-.", "x.x.x.x.x.x.x.x."];
const HAT_16: &[&str] = &["x-x-x-x-x-x-x-x-", "xoxoxoxoxoxoxoxo", "x.-.x.-.x.-.x.-."];
const HAT_SLOW: &[&str] = &["x.-.x.-.x.-.x.-.", "x.x.x.x.x.x.x.x.", "x-x-x-x-x-x-x-x-"];
const HAT_FAST: &[&str] = &["x.-.x.-.x.-.x.-.", "x.x.x.x.x.x.x.x.", "x...x...x...x..."];
const RIDE: &[&str] = &["x.x.x.x.x.x.x.x.", "x.-.x.-.x.-.x.-.", "x...x...x...x..."];
const SHAKE: &[&str] = &["-x-x-x-x-x-x-x-x", "x.x.x.x.x.x.x.x.", "x-x-x-x-x-x-x-x-"];
/// A 12/8's kicks and hats.
const KICK_12: &[&str] =
    &["x.....x.....x.....x.....", "x...........x.....x.....", "x.........x.x...........", "x.....x.....x.........x."];
const HAT_12: &[&str] = &["x...x.x...x.x...x.x...x.", "x.x.x.x.x.x.x.x.x.x.x.x.", "x.....x.....x.....x....."];

/// The line `l` of a style's drums as a piece plays it: one of its kind
/// (when it is a plain one), drawn.
fn kin(r: &mut Rng, st: &Style, l: Lane, side: usize, bpm: f64, acoustic: bool) -> &'static str {
    let (ch, kind, pat) = l;
    let (fam, p): (&[&'static str], f32) = match (st.bs, st.beat, ch) {
        (16, 4, CH_KICK) if KICK_A.contains(&pat) || KICK_B.contains(&pat) => {
            (if side == 0 { KICK_A } else { KICK_B }, 0.7)
        }
        (16, 4, CH_SNARE) if kind == 0 && acoustic && BACK.contains(&pat) => (BACK, 0.4),
        (16, 4, CH_SNARE) if kind == 0 && acoustic && BACK_X.contains(&pat) => (BACK_X, 0.4),
        (16, 4, CH_HAT) if kind == HAT_C && HAT_8.contains(&pat) => {
            let fam = if bpm < 116.0 {
                HAT_SLOW
            } else if bpm >= 150.0 {
                HAT_FAST
            } else {
                HAT_8
            };
            (fam, 0.5)
        }
        (16, 4, CH_HAT) if kind == HAT_C && HAT_16.contains(&pat) && bpm < 132.0 => (HAT_16, 0.4),
        (16, 4, CH_CYM) if kind == 1 && RIDE.contains(&pat) => (RIDE, 0.4),
        (16, 4, CH_PERC) if kind == SHAKER && SHAKE.contains(&pat) => (SHAKE, 0.5),
        (24, 6, CH_KICK) if KICK_12.contains(&pat) => (KICK_12, 0.5),
        (24, 6, CH_HAT | CH_CYM) if HAT_12.contains(&pat) => (HAT_12, 0.4),
        _ => return pat,
    };
    if r.chance(p) { r.pick(fam) } else { pat }
}

/// A style's drum lines for a piece: its own pair or one written for it
/// here, then each plain line maybe another of its kind (the same line in
/// both, the same other; the kick's each its own).
pub(super) fn draw_lanes(r: &mut Rng, st: &Style, bpm: f64) -> [Vec<Lane>; 2] {
    let x = extra(st);
    let k = r.below(1 + x.grooves.len());
    let pair = if k == 0 { st.grooves } else { x.grooves[k - 1] };
    let mut out = [pair[0].to_vec(), pair[1].to_vec()];
    let mut drawn: Vec<(Lane, &'static str)> = Vec::new();
    for (side, lanes) in out.iter_mut().enumerate() {
        for l in lanes.iter_mut() {
            let was = *l;
            l.2 = match drawn.iter().find(|d| d.0 == was && was.0 != CH_KICK) {
                Some(d) => d.1,
                None => kin(r, st, was, side, bpm, x.acoustic),
            };
            drawn.push((was, l.2));
        }
    }
    out
}

/// A section's turn of the beat: none three times in ten.
pub(super) fn draw_orn(r: &mut Rng, acoustic: bool) -> (u8, u8) {
    let kind = [0, ORN_KICK, ORN_GHOST, ORN_REST, ORN_LESS]
        [r.weighted(&[0.3, 0.3, if acoustic { 0.15 } else { 0.0 }, 0.15, 0.1])];
    (kind, r.below(4) as u8)
}

/// Where a turn of the beat falls in a bar of `bs` 16ths, beats of `beat`
/// (the bar's length when it has no place).
pub(super) fn orn_step(orn: (u8, u8), bs: u32, beat: u32) -> u32 {
    let k = orn.1 as usize % 4;
    // (a compound beat's last third; a simple one's second half)
    let (sub, compound) = (if beat == 6 { 2 } else { beat / 2 }, beat == 6);
    let at = match orn.0 {
        ORN_KICK if compound => [bs - sub, bs / 2 - sub, bs - sub, bs / 2 - sub][k],
        ORN_KICK => [bs - sub, bs / 2 - sub, bs / 2 + sub, bs - 1][k],
        ORN_GHOST if compound => [bs / 2 - sub, bs - sub, bs / 2 - sub, bs - sub][k],
        ORN_GHOST => [bs / 2 - 1, bs - 1, bs / 2 + 1, beat - 1][k],
        ORN_LESS => bs / 2,
        _ => bs,
    };
    at.min(bs)
}

/// Whether 16th `j` of a fill `n` long plays, in the fill's shape: 0 and 1
/// as the style has it, 2 each beat's second 16th left out, 3 eighths to
/// the last half beat (two beats long: `sy_drums`), 4 all of it (the snare
/// into the toms).
pub(super) fn fill_steps(shape: u8, j: u32, n: u32, beat: u32) -> bool {
    match shape {
        2 => j % beat != 1,
        3 => j.is_multiple_of(2) || j + beat / 2 >= n,
        _ => true,
    }
}

/// A hook of eight bars that is not one bar eight times: the hook, its
/// answer (the hook moved, its end turned), the hook, the hook's end
/// turned; then the same again to a close on the tonic.
pub(super) fn sy_hook(r: &mut Rng, bs: u32, beat: u32, density: f32, lo: i32, hi: i32) -> Vec<MNote> {
    let h = sy_cell(r, 1, bs, beat, density, lo, hi, false, TU_HOOK);
    let turned = |r: &mut Rng, v: &[MNote]| {
        let mut v = v.to_vec();
        for n in v.iter_mut().rev().take(2) {
            n.deg = (n.deg + r.pick(&[-2, -1, 1, 2])).clamp(lo, hi);
        }
        v
    };
    let by = r.pick(&[1, -1, 2, -2]);
    let ans = turned(r, &moved(&h, by, lo, hi));
    let end = turned(r, &h);
    let mut close = h.clone();
    if let Some(n) = close.last_mut() {
        let mut t = (n.deg as f32 / 7.0).round() as i32 * 7;
        if t > hi {
            t -= 7;
        }
        if t < lo {
            t += 7;
        }
        n.deg = t;
        n.len = (bs - n.step as u32).max(1) as u16;
    }
    let bars = [&h, &ans, &h, &end, &h, &ans, &h, &close];
    let mut v = Vec::new();
    for (k, bar) in bars.iter().enumerate() {
        v.extend(bar.iter().map(|n| MNote { step: n.step + (bs * k as u32) as u16, ..*n }));
    }
    v
}

impl Song {
    /// A style's drum lines as this piece plays them (`var`: A or B;
    /// `guest`: a mix's styled drummer's).
    pub(super) fn sy_lanes(&self, st: &Style, var: usize, guest: bool) -> Vec<Lane> {
        let own = if guest { &self.sy.vary.dlanes } else { &self.sy.vary.lanes };
        if self.opts.varied && !own[var].is_empty() { own[var].clone() } else { st.grooves[var].to_vec() }
    }

    /// The tune `k` of the piece as this section plays it.
    pub(super) fn tune_now(&self, k: usize) -> Vec<MNote> {
        match &self.sy.vary.tune {
            Some((of, v)) if *of == k => v.clone(),
            _ => self.sy.tunes[k].clone(),
        }
    }

    /// The pool of an older genre's progressions: techno's with more for
    /// its mode, written with more variety.
    pub(super) fn pool_now(&self) -> Vec<&'static str> {
        let mut v = pool(self.genre, self.mode).to_vec();
        if self.opts.varied && self.genre == TECHNO {
            v.extend(match self.mode {
                1 => TECH_DORIAN,
                2 => TECH_PHRYGIAN,
                _ => TECH_MORE,
            });
        }
        v
    }

    /// The form's turns: a long first verse (its tune twice) or a longer
    /// intro where bars are short, a half verse the second time, and before
    /// the last choruses a riser's build (a style that has them) or a breath
    /// on the intro's parts (a song with no bridge or solo there).
    pub(super) fn vary_form(&mut self, st: &Style) {
        let x = extra(st);
        let bar = st.bs as f64 / st.beat as f64 * 60.0 / self.bpm;
        let nth = |parts: &[u8], p: u8, n: usize| parts.iter().enumerate().filter(|x| *x.1 == p).nth(n).map(|x| x.0);
        let r = &mut self.rng;
        if let Some(k) = nth(&self.sy.parts, PA_VERSE, 0).filter(|&k| self.plan[k].bars == 8 && bar < 2.1)
            && r.chance(0.2)
        {
            self.plan[k].bars = 16;
        }
        if let Some(k) = nth(&self.sy.parts, PA_VERSE, 1).filter(|&k| self.plan[k].bars == 8)
            && r.chance(0.2)
        {
            self.plan[k].bars = 4;
        }
        if let Some(k) = nth(&self.sy.parts, PA_INTRO, 0).filter(|&k| self.plan[k].bars == 4 && bar < 2.2)
            && r.chance(0.2)
        {
            self.plan[k].bars = 8;
        }
        // the first of the last choruses
        let Some(mut k) = self.sy.parts.iter().rposition(|&p| p == PA_CHORUS) else { return };
        while k > 0 && self.sy.parts[k - 1] == PA_CHORUS {
            k -= 1;
        }
        if k < 3 {
            return;
        }
        let before = self.sy.parts[k - 1];
        if x.build && before != PA_BUILD && r.chance(0.3) {
            let mask = st.layers[PA_VERSE as usize] | L_SNARE | L_HAT;
            self.plan.insert(k, Sec { kind: BUILD, bars: 4, mask, energy: 0.7 });
            self.sy.parts.insert(k, PA_BUILD);
        } else if !x.build && matches!(before, PA_VERSE | PA_CHORUS) && r.chance(0.25) {
            self.plan.insert(k, Sec { kind: BREAK, bars: 4, mask: st.layers[PA_INTRO as usize], energy: 0.45 });
            self.sy.parts.insert(k, PA_BRIDGE);
        }
    }

    /// The piece's own: its drums' lines, its ways, and who plays the parts
    /// the style has others for (its own player a little under half the
    /// time; a part whose instruments already change with the sections
    /// keeps to that).
    pub(super) fn vary_piece(&mut self, st: &Style, s: &mut Synth) {
        let x = extra(st);
        let r = &mut self.rng;
        self.sy.vary.lanes = draw_lanes(r, st, self.bpm);
        let way = |r: &mut Rng, own: u8, more: &[u8]| if more.is_empty() || r.chance(0.5) { own } else { r.pick(more) };
        let ways =
            (way(r, st.bass, x.bass), way(r, st.comp, x.comp), way(r, st.arp, x.arp), way(r, st.second, x.second));
        self.sy.vary.ways = Some(ways);
        for &(ch, pal) in x.alts {
            if self.band.tones.iter().any(|t| t.0 == ch) || r.chance(0.45) {
                continue;
            }
            let orig = s.ch[ch].patch;
            let p = (r.pick(pal).1)(r);
            s.ch[ch].patch = levelled(&orig, note_of(ch), p);
        }
    }

    /// The last choruses a key up, three pieces in ten of a style that does
    /// so (from the first of them, or the second of two), where the plan
    /// moves no key there already.
    pub(super) fn vary_keys(&mut self, st: &Style) {
        if !extra(st).lift || !self.rng.chance(0.3) {
            return;
        }
        let Some(last) = self.sy.parts.iter().rposition(|&p| p == PA_CHORUS) else { return };
        let mut k = last;
        while k > 0 && self.sy.parts[k - 1] == PA_CHORUS {
            k -= 1;
        }
        if k < last && self.rng.bit_chance() {
            k = last;
        }
        if k < 3 || k >= self.lifts.len() || self.lifts[k - 1..].iter().any(|&l| l != self.lifts[k - 1]) {
            return;
        }
        let up = self.rng.pick(&[1, 2, 2]);
        for l in self.lifts[k..].iter_mut() {
            *l += up;
        }
    }

    /// A section's own: a verse's or a chorus's tune come round again with
    /// a cell written anew (or its close; a hook's turns moved), the turn
    /// its even bars give the beat, what sits out its halves (the last
    /// choruses, back from a bridge or a solo, now and then open without
    /// their drums), and the last choruses' tune doubled an octave up.
    pub(super) fn vary_section(&mut self, part: u8) {
        let st = self.style();
        let x = extra(st);
        let sec = self.plan[self.sec];
        let nth = self.sy.vary.seen[part as usize & 7];
        self.sy.vary.seen[part as usize & 7] = nth.saturating_add(1);
        let chorus = matches!(part, PA_CHORUS | PA_DROP);
        let last = self.sy.parts.iter().rposition(|&p| matches!(p, PA_CHORUS | PA_DROP));
        let last_run = chorus
            && last.is_some_and(|l| {
                self.sec <= l && self.sy.parts[self.sec..=l].iter().all(|&p| matches!(p, PA_CHORUS | PA_DROP))
            });
        let before = self.sec.checked_sub(1).map(|k| self.part_of(k));
        let (bs, beat) = (st.bs, st.beat);
        let k = match part {
            PA_CHORUS | PA_DROP => 1,
            PA_BRIDGE | PA_BUILD => 2,
            _ => 0,
        };
        let (lo, hi) = if k == 1 { (st.voice.1 + 2, st.voice.2 + 2) } else { (st.voice.1, st.voice.2) };
        let r = &mut self.rng;
        let mut tune = None;
        let sung = matches!(st.tune, TU_SONG | TU_CLASSIC | TU_LONG | TU_HOOK);
        if nth >= 1 && (part == PA_VERSE || chorus) && sung && r.chance(0.6) {
            let mut v = self.sy.tunes[k].clone();
            if st.tune == TU_HOOK {
                // the hook's turns (its fourth bar, its eighth's lead-in): moved again
                for bar in [3u32, 6] {
                    let at: Vec<usize> = (0..v.len()).filter(|&i| v[i].step as u32 / bs == bar).collect();
                    for &i in at.iter().rev().take(2) {
                        v[i].deg = (v[i].deg + r.pick(&[-2, -1, 1, 2])).clamp(lo, hi);
                    }
                }
            } else {
                // a cell of two bars written anew: the second or the third, or the close
                let c = if r.chance(0.35) { 3 } else { r.pick(&[1u32, 2]) };
                let d = if k == 1 { st.density.1 } else { st.density.0 } * if c == 3 { 0.8 } else { 1.0 };
                let fresh = sy_cell(r, 2, bs, beat, d, lo, hi, c == 3, st.tune);
                let cell = 2 * bs;
                v.retain(|n| n.step as u32 / cell != c);
                v.extend(fresh.iter().map(|n| MNote { step: n.step + (cell * c) as u16, ..*n }));
                v.sort_by_key(|n| n.step);
            }
            tune = Some((k, v));
        }
        let orn = if st.fill != FI_NONE { draw_orn(r, x.acoustic) } else { (0, 0) };
        let (mut out, mut down) = ([0u32; 2], false);
        if sec.kind == MAIN && sec.bars >= 8 {
            // (the chords' parts, keys, pad and arpeggio: one may sit out
            // where another plays on)
            let chordal = (sec.mask & (L_KEYS | L_PAD | L_ARP)).count_ones();
            let opt: Vec<u32> = [L_CTR, L_ARP, L_PAD, L_PERC, L_OHAT]
                .into_iter()
                .filter(|&l| sec.mask & l != 0 && (l & (L_PAD | L_ARP) == 0 || chordal >= 2))
                .collect();
            if !opt.is_empty() && r.chance(0.3) {
                out[0] = r.pick(&opt);
            }
            if !opt.is_empty() && r.chance(0.25) {
                out[1] = r.pick(&opt);
            }
            if part == PA_CHORUS && last_run && matches!(before, Some(PA_BRIDGE | PA_SOLO)) && r.chance(0.3) {
                (out[0], down) = (L_KICK | L_SNARE | L_HAT | L_OHAT | L_PERC, true);
            }
        }
        let lift = last_run && nth >= 1 && st.voice.0 <= 74 && !matches!(st.tune, TU_RIFF | TU_NONE) && r.chance(0.5);
        let v = &mut self.sy.vary;
        (v.tune, v.orn, v.out, v.down, v.lift) = (tune, orn, out, down, lift);
    }

    /// A mix's styled drummer's lines for the piece.
    pub(super) fn vary_guest(&mut self) {
        let Some(g) = self.guest.filter(|&g| is_styled(g)) else { return };
        let list = styles_of(g);
        let st = &list[self.sy.dstyle.min(list.len() - 1)].1;
        self.sy.vary.dlanes = draw_lanes(&mut self.rng, st, self.bpm);
    }

    /// A section's turn of the beat for a mix's styled drummer, when the
    /// writer is not one of the styled genres (which draw their own).
    pub(super) fn vary_guest_section(&mut self) {
        let Some(g) = self.guest.filter(|&g| is_styled(g) && !is_styled(self.genre)) else { return };
        let list = styles_of(g);
        let st = &list[self.sy.dstyle.min(list.len() - 1)].1;
        self.sy.vary.orn = if st.fill != FI_NONE { draw_orn(&mut self.rng, extra(st).acoustic) } else { (0, 0) };
    }
}

// ---------------------------------------------------------------- the older genres'

/// A theme (the western's, starconflict's: bars of `fb` 16ths, pitches in
/// semitones over the tonic) with one of its inner bars (the third, the
/// fourth or the seventh) moved a step or two along the scale of `mode`:
/// a later statement's.
pub(super) fn moved_bar(r: &mut Rng, theme: &[Tn], fb: f32, mode: usize) -> Vec<Tn> {
    let bar = r.pick(&[2usize, 3, 6]);
    let by = r.pick(&[-2, -1, 1, 2]);
    let sc = &MODES[mode];
    let step = |p: i32| {
        // the scale's degree nearest `p`, then `by` degrees on
        let (oct, pc) = (p.div_euclid(12), p.rem_euclid(12));
        let d = (0..7).min_by_key(|&d| (sc[d] - pc).abs()).unwrap_or(0) as i32 + by;
        12 * (oct + d.div_euclid(7)) + sc[d.rem_euclid(7) as usize]
    };
    theme.iter().map(|n| if (n.st / fb) as usize == bar { Tn { p: step(n.p), ..*n } } else { *n }).collect()
}

const TECH_DORIAN: &[&str] = &["i7 IV7 i7 IV7 i7 IV7 bVII bVII", "i9 i9 IV9 IV9 i9 i9 bVII9 bVII9"];
const TECH_PHRYGIAN: &[&str] = &["i7 i7 bII bII i7 i7 bVII bVII", "i7 bII bIII bII i7 bII bVII bVII"];
const TECH_MORE: &[&str] =
    &["i7 i7 bVII bVII bVI bVI bVII bVII", "i9 i9 i9 i9 iv9 iv9 i9 i9", "i7 i7 v7 v7 bVI bVI bVII bVII"];

// ---------------------------------------------------------------- each style's

/// Lines for a pair: (the drum's channel, its kind, a character a step).
macro_rules! lanes {
    ($(($ch:expr, $kind:expr, $pat:expr)),* $(,)?) => { &[$(($ch, $kind, $pat)),*] };
}

const FOUR: &str = "x...x...x...x...";
const BACKBEAT: &str = "....x.......x...";
const OFF: &str = "..x...x...x...x.";
const EIGHTHS: &str = "x.-.x.-.x.-.x.-.";
const SIXTEENTHS: &str = "x-x-x-x-x-x-x-x-";

const E_SONATA: Extra = Extra {
    comp: &[CP_ARP8, CP_BLOCK4],
    alts: &[(CH_LEAD, &[("flute", flute), ("violin", fiddle)]), (CH_KEYS, &[("harpsichord", harpsichord)])],
    ..X
};
const E_MINUET: Extra = Extra {
    min: &["i iv^1 V7 i bVI iv V7 i"],
    second: &[SV_COUNTER, SV_GUIDE],
    alts: &[(CH_LEAD, &[("flute", flute), ("oboe", sw_oboe)])],
    ..X
};
const E_BAROQUE: Extra = Extra {
    min: &["i bVII bVI V i iv V i"],
    bass: &[BS_WALK],
    comp: &[CP_BLOCK4],
    second: &[SV_ANSWER, SV_COUNTER],
    alts: &[(CH_LEAD, &[("oboe", sw_oboe), ("flute", flute)])],
    ..X
};
const E_SYMPHONY: Extra =
    Extra { bass: &[BS_TWO], second: &[SV_GUIDE], alts: &[(CH_KEYS, &[("horns", sw_horns)])], ..X };
const E_WALTZ: Extra = Extra {
    min: &["i i iv iv bVI V7 i i", "i V7 i i iv V7 i i"],
    second: &[SV_GUIDE, SV_OCTAVE],
    alts: &[(CH_LEAD, &[("flute", flute), ("clarinet", clarinet), ("oboe", sw_oboe)])],
    ..X
};
const E_ADAGIO: Extra =
    Extra { second: &[SV_GUIDE], alts: &[(CH_BELL, &[("cellos", sw_cellos), ("clarinet", clarinet)])], ..X };

const ROCK_ALTS: &[(usize, Pal)] = &[
    (CH_KEYS, &[("crunchy guitar", ska_guitar_crunch)]),
    (CH_BASS, &[("fingered bass", ska_bass_flat)]),
    (CH_PAD, &[("Hammond", ska_hammond)]),
];
const E_HARD: Extra = Extra {
    maj: &["I bIII IV I I bIII bVII IV"],
    min: &["i bVII bVI bVII i bVII bVI V"],
    bass: &[BS_ROOT4],
    alts: ROCK_ALTS,
    acoustic: true,
    ..X
};
const E_ANTHEM: Extra = Extra {
    min: &["i bIII bVII iv i bIII bVII bVII", "i iv bVI bVII i iv bVI V"],
    bass: &[BS_ROOT4],
    alts: ROCK_ALTS,
    acoustic: true,
    lift: true,
    ..X
};
const E_BLUES: Extra = Extra {
    maj: &["I7 IV7 I7 I7 IV7 IV7 I7 VI7 II7 V7 I7 V7", "I7 I7 I7 I7 IV7 IV7 I7 I7 V7 V7 I7 V7"],
    min: &[
        "i7 i7 i7 i7 iv7 iv7 i7 i7 bVI7 V7 i7 i7",
        "i7 iv7 i7 i7 iv7 iv7 i7 i7 V7 iv7 i7 V7",
        "i7 i7 i7 i7 iv7 iv7 i7 i7 iv7 V7 i7 V7",
    ],
    grooves: &[[
        lanes![
            (CH_KICK, 0, "x...........x.....x....."),
            (CH_SNARE, 0, "......x...........x....."),
            (CH_HAT, HAT_C, "x...x.x...x.x...x.x...x."),
        ],
        lanes![
            (CH_KICK, 0, "x.....x.....x.....x....."),
            (CH_SNARE, 0, "......X...........X....."),
            (CH_CYM, 1, "x.x.x.x.x.x.x.x.x.x.x.x."),
        ],
    ]],
    bass: &[BS_WALK],
    comp: &[CP_BLOCK4],
    alts: &[(CH_KEYS, &[("crunchy guitar", ska_guitar_crunch)]), (CH_BASS, &[("picked bass", bass_pick_dark)])],
    acoustic: true,
    ..X
};

const E_SOUL: Extra = Extra {
    min: &["i bVII bVI V i bVII bVI V", "i iv i V7 i iv V7 i", "i bVI iv V7 i bVI iv V7"],
    grooves: &[[
        lanes![
            (CH_KICK, 0, "x.....x.x......."),
            (CH_SNARE, 0, BACKBEAT),
            (CH_HAT, HAT_C, EIGHTHS),
            (CH_PERC, SHAKER, BACKBEAT),
        ],
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_SNARE, 0, BACKBEAT),
            (CH_HAT, HAT_C, EIGHTHS),
            (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x."),
        ],
    ]],
    bass: &[BS_ROOT8, BS_WALK],
    comp: &[CP_BLOCK8, CP_GOSPEL],
    second: &[SV_ANSWER],
    alts: &[
        (CH_KEYS, &[("Hammond", ska_hammond), ("electric guitar", elec_clean)]),
        (CH_BASS, &[("upright bass", bass_upright), ("picked bass", bass_pick_dark)]),
    ],
    acoustic: true,
    lift: true,
    ..X
};
const E_SURF: Extra = Extra {
    maj: &["I vi IV V I vi IV V"],
    min: &["i bVII bVI V i bVII bVI V"],
    grooves: &[[
        lanes![(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, "....x.x.....x..."), (CH_HAT, HAT_C, EIGHTHS)],
        lanes![
            (CH_KICK, 0, "x.......x......."),
            (CH_SNARE, 0, "....x.x.....x.x."),
            (CH_PERC, TOM_LO, "x.x.x.x.x.x.x.x."),
        ],
    ]],
    bass: &[BS_ROOT4],
    comp: &[CP_BLOCK8, CP_ARP8],
    alts: &[
        (CH_LEAD, &[("twelve-string", twelve_string), ("tenor sax", sax)]),
        (CH_KEYS, &[("twelve-string", twelve_string)]),
        (CH_PAD, &[("Hammond", ska_hammond)]),
        (CH_BASS, &[("fingered bass", ska_bass_flat)]),
    ],
    acoustic: true,
    ..X
};
const E_BEAT: Extra = Extra {
    min: &["i iv bVII bIII i iv V V", "i bIII bVII iv i bIII V V", "i bVI bIII bVII i bVI V V"],
    grooves: &[[
        lanes![(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, "....x.x.....x..."), (CH_HAT, HAT_C, EIGHTHS)],
        lanes![
            (CH_KICK, 0, "x.....x.x.x....."),
            (CH_SNARE, 0, BACKBEAT),
            (CH_CYM, 1, "x.x.x.x.x.x.x.x."),
            (CH_PERC, SHAKER, BACKBEAT),
        ],
    ]],
    bass: &[BS_ROOT4],
    comp: &[CP_ARP8],
    second: &[SV_ANSWER],
    alts: &[
        (CH_KEYS, &[("clean guitar", elec_clean), ("piano", piano)]),
        (CH_BASS, &[("picked bass", bass_pick_dark), ("fingered bass", ska_bass_flat)]),
        (CH_BELL, &[("organ", ska_farfisa)]),
    ],
    acoustic: true,
    lift: true,
    ..X
};

const E_DISCO: Extra = Extra {
    maj: &["Imaj7 iii7 IVmaj7 V7sus4 Imaj7 iii7 IVmaj7 V7sus4"],
    min: &["i7 bVII bVImaj7 bVII i7 bVII bVImaj7 V7sus4"],
    grooves: &[[
        lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, SIXTEENTHS)],
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_SNARE, 0, BACKBEAT),
            (CH_SNARE, 1, BACKBEAT),
            (CH_HAT, HAT_C, FOUR),
            (CH_HAT, HAT_O, OFF),
            (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x."),
        ],
    ]],
    bass: &[BS_ROOT8, BS_MOTOWN],
    comp: &[CP_STABS],
    second: &[SV_ANSWER],
    alts: &[(CH_KEYS, &[("clavinet", clav), ("Rhodes", ep_rhodes)])],
    lift: true,
    ..X
};
const E_FUNK: Extra = Extra {
    maj: &["I7 I7 IV7 IV7 I7 I7 V7 IV7"],
    min: &["i7 i7 bVII7 bVII7 i7 i7 IV7 IV7"],
    grooves: &[
        [
            lanes![
                (CH_KICK, 0, "x..x..x...x....."),
                (CH_SNARE, 0, "....x..o.o..x..."),
                (CH_HAT, HAT_C, "xoxoxoxoxoxoxoxo"),
            ],
            lanes![
                (CH_KICK, 0, "x..x..x.x.x..x.."),
                (CH_SNARE, 0, "....x..o.o..x..o"),
                (CH_HAT, HAT_C, "xoxoxoxoxoxoxo.o"),
                (CH_HAT, HAT_O, "..............x."),
            ],
        ],
        [
            lanes![(CH_KICK, 0, "x.x......xx....."), (CH_SNARE, 0, "....x.o..o..x..."), (CH_HAT, HAT_C, SIXTEENTHS),],
            lanes![
                (CH_KICK, 0, "x.x...x..xx...x."),
                (CH_SNARE, 0, "....x.o..o..x.o."),
                (CH_HAT, HAT_C, "x-x-x-x-x-x-x.x-"),
                (CH_HAT, HAT_O, "..............x."),
            ],
        ],
    ],
    bass: &[BS_MOTOWN],
    comp: &[CP_STABS],
    second: &[SV_ANSWER],
    alts: &[(CH_PAD, &[("strings", pad_strings)])],
    acoustic: true,
    ..X
};
const E_SOFT: Extra = Extra {
    maj: &["Imaj7 iii7 vi7 IVmaj7 ii7 V7sus4 Imaj7 Imaj7"],
    min: &[
        "i7 iv7 bVII7 bIIImaj7 bVImaj7 iv7 V7sus4 V7",
        "i7 bVII bVImaj7 bVII i7 bVII bVImaj7 V7sus4",
        "i9 bVImaj7 bIIImaj7 bVII i9 bVImaj7 iv7 V7",
    ],
    bass: &[BS_ROOT8, BS_TWO],
    comp: &[CP_BLOCK4, CP_HOLD],
    second: &[SV_ANSWER, SV_OCTAVE],
    alts: &[(CH_BASS, &[("fingered bass", ska_bass_flat)])],
    acoustic: true,
    lift: true,
    ..X
};

const E_SYNTHPOP: Extra = Extra {
    maj: &["I vi IV V I vi IV V"],
    min: &["i bVI bVII i i bVI bVII V", "i bIII bVII bVI i bIII bVII bVII"],
    bass: &[BS_OFFBEAT, BS_DISCO],
    comp: &[CP_BLOCK8, CP_OFFBEAT],
    arp: &[AR_16, AR_PLUCK],
    second: &[SV_ANSWER],
    alts: &[
        (CH_BASS, &[("FM bass", bass_fm)]),
        (CH_KEYS, &[("saw stabs", stab_saw), ("clean guitar", elec_clean)]),
        (CH_ARP, &[("FM bells", bell_fm)]),
    ],
    build: true,
    lift: true,
    ..X
};
const E_BALLAD: Extra = Extra {
    min: &["i bVII bVI V i bVII bVI V", "i iv bVII bIII bVI iv V V", "i bVI iv V i bVI bVII i"],
    bass: &[BS_HOLD, BS_ROOT8],
    comp: &[CP_BLOCK4, CP_HOLD],
    second: &[SV_OCTAVE],
    alts: &[(CH_KEYS, &[("FM piano", glass_fm)]), (CH_BASS, &[("picked bass", bass_pick_dark)])],
    acoustic: true,
    lift: true,
    ..X
};
const E_NEWWAVE: Extra = Extra {
    maj: &["I IV vi V I IV vi V"],
    min: &["i bVI bIII bVII i bVI bIII bVII", "i bIII bVI bVII i bIII bVI V"],
    grooves: &[[
        lanes![(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, EIGHTHS)],
        lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 0, "....X.......X..."), (CH_HAT, HAT_C, SIXTEENTHS)],
    ]],
    bass: &[BS_DISCO],
    comp: &[CP_BLOCK8, CP_STRUM],
    arp: &[AR_16, AR_PLUCK],
    alts: &[(CH_KEYS, &[("FM keys", glass_fm)]), (CH_BASS, &[("FM bass", bass_fm)])],
    ..X
};

const FLOOR_MORE: &[[&[Lane]; 2]] = &[
    [
        lanes![(CH_KICK, 0, FOUR), (CH_HAT, HAT_C, OFF)],
        lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, SIXTEENTHS), (CH_HAT, HAT_O, OFF)],
    ],
    [
        lanes![(CH_KICK, 0, FOUR), (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x.")],
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_SNARE, 0, BACKBEAT),
            (CH_HAT, HAT_O, OFF),
            (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x."),
        ],
    ],
];
const E_EURO: Extra = Extra {
    maj: &["I V vi IV I V vi IV", "vi V IV V vi V IV IV"],
    min: &["i bVII bVI bVII i bVII bVI V"],
    grooves: FLOOR_MORE,
    bass: &[BS_DISCO, BS_ROOT8],
    comp: &[CP_STABS, CP_BLOCK8],
    arp: &[AR_8, AR_PLUCK],
    alts: &[(CH_KEYS, &[("polysynth", hoover)]), (CH_BASS, &[("synth bass", bass_synth)])],
    lift: true,
    ..X
};
const E_GRUNGE: Extra = Extra {
    maj: &["I IV bVII IV I IV bVII bVII"],
    min: &["i iv bVII bIII i iv bVI bVII"],
    bass: &[BS_ROOT4],
    acoustic: true,
    ..X
};
const E_SLOWJAM: Extra = Extra {
    maj: &["Imaj7 iii7 IVmaj7 V7sus4 Imaj7 iii7 ii7 V7"],
    min: &["i9 bVImaj7 bVII7 i9 i9 bVImaj7 iv9 V7", "i7 bVII7 bVImaj7 V7 i7 bVII7 bVImaj7 V7"],
    grooves: &[
        [
            lanes![(CH_KICK, 0, "x.........x....."), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, EIGHTHS)],
            lanes![(CH_KICK, 0, "x......x..x....."), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, SIXTEENTHS)],
        ],
        [
            lanes![(CH_KICK, 0, "x..x......x....."), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, EIGHTHS)],
            lanes![
                (CH_KICK, 0, "x..x...x..x..x.."),
                (CH_SNARE, 0, BACKBEAT),
                (CH_HAT, HAT_C, "x-x-x-x-x-x-x.x-"),
                (CH_HAT, HAT_O, "..............x."),
            ],
        ],
    ],
    bass: &[BS_HOLD],
    comp: &[CP_ARP8, CP_BLOCK4],
    alts: &[(CH_KEYS, &[("bright piano", bright_piano)]), (CH_PAD, &[("warm pad", pad_warm)])],
    lift: true,
    ..X
};

const E_PUNK: Extra = Extra {
    min: &["i bVII bVI bVII i bVII bVI V", "i bIII bVII bVI i bIII bVII bVII", "i iv bVI bVII i iv bVI V"],
    grooves: &[[
        lanes![(CH_KICK, 0, "x.....x.x......."), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, EIGHTHS)],
        lanes![(CH_KICK, 0, "x.....x.x.x...x."), (CH_SNARE, 0, "....X.......X..."), (CH_CYM, 1, "x.x.x.x.x.x.x.x.")],
    ]],
    acoustic: true,
    lift: true,
    ..X
};
const E_RNB: Extra = Extra {
    maj: &["IVmaj7 V7 iii7 vi7 IVmaj7 V7 Imaj7 Imaj7", "Imaj7 vi7 IVmaj7 V7 Imaj7 vi7 IVmaj7 V7"],
    min: &["i bVII bVI V i bVII bVI V"],
    grooves: &[[
        lanes![(CH_KICK, 0, "x..x.....xx....."), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, "xoxoxoxoxoxoxoxo")],
        lanes![
            (CH_KICK, 0, "x..x..x..xx..x.."),
            (CH_SNARE, 0, BACKBEAT),
            (CH_HAT, HAT_C, "xoxoxoxoxoxxxoxo"),
            (CH_PERC, RIM, "..x.......x...x."),
        ],
    ]],
    comp: &[CP_HOLD, CP_ARP8],
    second: &[SV_GUIDE],
    alts: &[(CH_LEAD, &[("FM keys", glass_fm), ("square lead", lead_square)]), (CH_PAD, &[("warm pad", pad_warm)])],
    ..X
};
const E_ELECTRO: Extra = Extra {
    maj: &["IV V vi vi IV V I I", "I vi IV V I vi IV V"],
    min: &["bVI bVII i i bVI bVII bIII bIII", "i bVII bVI bVII i bVII bVI V"],
    grooves: FLOOR_MORE,
    bass: &[BS_ROOT8, BS_DISCO],
    comp: &[CP_OFFBEAT, CP_BLOCK8],
    arp: &[AR_8, AR_PLUCK],
    alts: &[(CH_KEYS, &[("polysynth", hoover), ("FM keys", glass_fm)])],
    lift: true,
    ..X
};

const E_EDM: Extra = Extra {
    maj: &["I V vi IV I V vi IV", "IV I V vi IV I V V"],
    min: &["i bIII bVII bVI i bIII bVII bVI"],
    grooves: &[[
        lanes![(CH_KICK, 0, FOUR), (CH_HAT, HAT_C, OFF)],
        lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 1, BACKBEAT), (CH_HAT, HAT_O, OFF), (CH_PERC, 0, "-x-x-x-x-x-x-x-x"),],
    ]],
    bass: &[BS_ROOT8],
    comp: &[CP_STABS, CP_HOLD],
    arp: &[AR_8, AR_PLUCK],
    alts: &[
        (CH_LEAD, &[("polysynth", hoover), ("FM bells", bell_fm)]),
        (CH_BASS, &[("sub bass", bass_sub)]),
        (CH_ARP, &[("FM bells", bell_fm)]),
    ],
    ..X
};
const E_TROPICAL: Extra = Extra {
    maj: &["I iii vi IV I iii vi V"],
    min: &["i bVII bVI bVII i bVII bVI V", "bVI bVII i i bVI bVII bIII bIII", "i iv bVI bVII i iv bVI V"],
    grooves: &[
        [
            lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 1, BACKBEAT), (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x.")],
            lanes![
                (CH_KICK, 0, FOUR),
                (CH_SNARE, 1, BACKBEAT),
                (CH_HAT, HAT_C, "-x-x-x-x-x-x-x-x"),
                (CH_PERC, TOM_MID, "...x..x....x..x."),
            ],
        ],
        [
            lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 1, "...x..x....x..x."), (CH_HAT, HAT_C, OFF)],
            lanes![
                (CH_KICK, 0, FOUR),
                (CH_SNARE, 1, "...x..x....x..x."),
                (CH_HAT, HAT_C, "-x-x-x-x-x-x-x-x"),
                (CH_PERC, TOM_MID, OFF),
            ],
        ],
    ],
    bass: &[BS_HOLD, BS_OFFBEAT],
    comp: &[CP_STABS, CP_HOLD],
    arp: &[AR_8, AR_SPARSE],
    alts: &[(CH_KEYS, &[("felt piano", piano_felt)])],
    build: true,
    lift: true,
    ..X
};
const E_TRAP: Extra = Extra {
    maj: &["IV V vi vi IV V iii vi", "I I vi vi IV IV V V", "vi V IV IV vi V IV V"],
    min: &["i bVII bVI V i bVII bVI V"],
    grooves: &[
        [
            lanes![
                (CH_KICK, 0, "x.........x....."),
                (CH_SNARE, 0, "........x......."),
                (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x.")
            ],
            lanes![
                (CH_KICK, 0, "x..x....x.x..x.."),
                (CH_SNARE, 0, "........x......."),
                (CH_HAT, HAT_C, "x.x.xxx.x.x.x.xx"),
                (CH_HAT, HAT_O, "..............x."),
            ],
        ],
        [
            lanes![(CH_KICK, 0, "x......x..x....."), (CH_SNARE, 0, "........x......."), (CH_HAT, HAT_C, EIGHTHS)],
            lanes![
                (CH_KICK, 0, "x.....xx..x...x."),
                (CH_SNARE, 0, "........x......."),
                (CH_HAT, HAT_C, "xxx.x.x.xxx.x.x."),
                (CH_HAT, HAT_O, "......x........."),
            ],
        ],
    ],
    comp: &[CP_ARP8],
    second: &[SV_GUIDE],
    alts: &[
        (CH_LEAD, &[("FM keys", glass_fm), ("synth voice", vox_chop), ("saw lead", lead_saw)]),
        (CH_KEYS, &[("piano", piano)]),
        (CH_PAD, &[("warm pad", pad_warm)]),
    ],
    ..X
};
const E_STOMP: Extra = Extra {
    min: &["i bVII bVI bVII i bVII bVI V", "i bIII bVII iv i bIII bVII bVII", "i iv bVI V i iv bVI V"],
    grooves: &[
        [
            lanes![(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 1, BACKBEAT)],
            lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 1, BACKBEAT), (CH_PERC, SHAKER, "-x-x-x-x-x-x-x-x")],
        ],
        [
            lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 1, BACKBEAT)],
            lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 1, "....x.......x.x."), (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x.")],
        ],
    ],
    bass: &[BS_TWO, BS_ROOT8],
    comp: &[CP_ARP8],
    arp: &[AR_PLUCK, AR_SPARSE],
    second: &[SV_OCTAVE],
    alts: &[(CH_BELL, &[("glockenspiel", glockenspiel)])],
    lift: true,
    ..X
};

const E_NUDISCO: Extra = Extra {
    maj: &["IVmaj7 V7 iii7 vi7 IVmaj7 V7 Imaj7 Imaj7"],
    min: &["i7 bVII bVImaj7 bVII i7 bVII bVImaj7 V7sus4", "i7 i7 iv7 iv7 bVImaj7 bVII7 i7 i7"],
    grooves: &[[
        lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 0, BACKBEAT), (CH_HAT, HAT_C, FOUR), (CH_HAT, HAT_O, OFF)],
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_SNARE, 0, "....X.......X..."),
            (CH_HAT, HAT_C, SIXTEENTHS),
            (CH_HAT, HAT_O, "..............x."),
        ],
    ]],
    bass: &[BS_FUNK, BS_ROOT8],
    comp: &[CP_OFFBEAT, CP_BLOCK8],
    arp: &[AR_16, AR_PLUCK],
    alts: &[
        (CH_KEYS, &[("Rhodes", ep_rhodes), ("clean guitar", ska_guitar_clean)]),
        (CH_BASS, &[("synth bass", bass_synth)]),
    ],
    build: true,
    lift: true,
    ..X
};
const E_AMAPIANO: Extra = Extra {
    maj: &["IVmaj7 iii7 ii7 Imaj7 IVmaj7 iii7 ii7 V7", "Imaj7 vi7 ii7 V7 Imaj7 vi7 ii7 V7"],
    min: &["i7 bVImaj7 bVII7 i7 i7 bVImaj7 bVII7 V7"],
    grooves: &[[
        lanes![(CH_KICK, 0, FOUR), (CH_HAT, HAT_C, SIXTEENTHS), (CH_PERC, RIM, "...x..x...x..x..")],
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_HAT, HAT_C, SIXTEENTHS),
            (CH_PERC, RIM, "...x..x....x.x.."),
            (CH_SNARE, 1, "........x......."),
        ],
    ]],
    comp: &[CP_HOLD, CP_BLOCK4],
    second: &[SV_GUIDE],
    alts: &[
        (CH_KEYS, &[("Rhodes", ep_rhodes)]),
        (CH_BELL, &[("marimba", marimba)]),
        (CH_PAD, &[("warm pad", pad_warm)]),
    ],
    ..X
};
const E_AFRO: Extra = Extra {
    maj: &["I vi IV V I vi IV V"],
    min: &["i bVII bVI bVII i bVII bVI V", "i iv bVII bIII i iv bVI V"],
    grooves: &[
        [
            lanes![(CH_KICK, 0, "x..x..x...x....."), (CH_SNARE, 1, BACKBEAT), (CH_HAT, HAT_C, SIXTEENTHS)],
            lanes![
                (CH_KICK, 0, "x..x..x...x....."),
                (CH_SNARE, 1, BACKBEAT),
                (CH_HAT, HAT_C, SIXTEENTHS),
                (CH_PERC, TOM_MID, "..x...x...x..x.."),
            ],
        ],
        [
            lanes![
                (CH_KICK, 0, "x.....x...x...x."),
                (CH_SNARE, 1, "...x..x....x..x."),
                (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x.")
            ],
            lanes![
                (CH_KICK, 0, "x.....x...x...x."),
                (CH_SNARE, 1, "...x..x....x..x."),
                (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x."),
                (CH_PERC, TOM_MID, "...x......x....x"),
            ],
        ],
    ],
    bass: &[BS_ROOT8],
    comp: &[CP_STABS, CP_OFFBEAT],
    second: &[SV_THIRDS],
    alts: &[(CH_KEYS, &[("Rhodes", ep_rhodes), ("piano", piano)]), (CH_BELL, &[("Rhodes", ep_rhodes)])],
    lift: true,
    ..X
};

const E_BENGA: Extra = Extra {
    min: &["i i iv iv V V i i", "i iv V iv i iv V i", "i V iv V i V iv i"],
    grooves: &[[
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_SNARE, 0, BACKBEAT),
            (CH_HAT, HAT_C, SIXTEENTHS),
            (CH_PERC, 0, "-o-o-o-o-o-o-o-o"),
        ],
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_SNARE, 0, "....x..o....x.o."),
            (CH_HAT, HAT_C, SIXTEENTHS),
            (CH_PERC, 0, "-o-o-o-o-o-o-o-o"),
        ],
    ]],
    comp: &[CP_ARP8],
    second: &[SV_ANSWER],
    alts: &[(CH_LEAD, &[("clean guitar", elec_clean)]), (CH_BASS, &[("upright bass", bass_upright)])],
    acoustic: true,
    ..X
};
const E_RUMBA: Extra = Extra {
    maj: &["I I IV V I I IV V"],
    min: &["i i iv V i i iv V", "i V iv V i V iv V", "i iv i V i iv i V"],
    grooves: &[[
        lanes![
            (CH_KICK, 0, "x..x....x..x...."),
            (CH_SNARE, 1, "...x..x...x.x..."),
            (CH_HAT, HAT_C, EIGHTHS),
            (CH_PERC, TOM_MID, OFF),
            (CH_PERC, TOM_LO, "......x.x......."),
        ],
        lanes![
            (CH_KICK, 0, FOUR),
            (CH_SNARE, 0, BACKBEAT),
            (CH_HAT, HAT_C, "xoxoxoxoxoxoxoxo"),
            (CH_PERC, TOM_MID, "...x..x....x..x."),
            (CH_PERC, TOM_LO, "......x.x......."),
        ],
    ]],
    bass: &[BS_ROOT4, BS_BENGA],
    comp: &[CP_ARP8],
    second: &[SV_THIRDS],
    alts: &[
        (CH_LEAD, &[("clean guitar", elec_clean)]),
        (CH_BASS, &[("fingered bass", ska_bass_flat)]),
        (CH_KEYS, &[("clean guitar", elec_clean)]),
    ],
    acoustic: true,
    ..X
};
const E_NYATITI: Extra =
    Extra { maj: &["I I I IV I I V I"], min: &["i i i iv i i v i", "i iv i i i bVII i i"], arp: &[AR_PLUCK], ..X };

const YO_PADS: &[(usize, Pal)] = &[(CH_PAD, &[("warm pad", pad_warm), ("shimmer", pad_shimmer)])];
const E_MEDITATION: Extra = Extra { min: &["i i bVII bVII iv iv i i"], arp: &[AR_SPARSE, AR_HARP], alts: YO_PADS, ..X };
const E_FLOW: Extra = Extra { min: &["i i bVII bVII iv iv i i"], arp: &[AR_SPARSE, AR_PLUCK], alts: YO_PADS, ..X };
const E_KIRTAN: Extra = Extra {
    maj: &["I I IV I I V IV I"],
    min: &["i i bVII i i iv bVII i", "i iv i bVII i iv bVII i"],
    comp: &[CP_BLOCK4],
    second: &[SV_ANSWER, SV_OCTAVE],
    ..X
};

const EL_MIN_MORE: &[&str] = &["i7 bVImaj7 iim7b5 V7 i7 bVImaj7 iim7b5 V7"];
const E_BOSSA: Extra = Extra {
    min: EL_MIN_MORE,
    grooves: &[[
        lanes![
            (CH_KICK, 0, "x.....x.x.....x."),
            (CH_SNARE, 1, "x..x...x..x..x.."),
            (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x.")
        ],
        lanes![(CH_KICK, 0, "x.....x.x.....x."), (CH_SNARE, 1, "x..x...x..x..x.."), (CH_CYM, 1, "x.x.x.x.x.x.x.x.")],
    ]],
    second: &[SV_ANSWER, SV_COUNTER],
    alts: &[(CH_KEYS, &[("piano", piano), ("Rhodes", ep_rhodes)])],
    ..X
};
const E_LOUNGE: Extra = Extra {
    min: EL_MIN_MORE,
    grooves: &[[
        lanes![
            (CH_CYM, 1, "x...x.x.x...x.x."),
            (CH_HAT, HAT_C, BACKBEAT),
            (CH_KICK, 0, "o.......o......."),
            (CH_SNARE, 0, "......o.......o."),
        ],
        lanes![
            (CH_CYM, 1, "x...x.x.x...x.x."),
            (CH_HAT, HAT_C, BACKBEAT),
            (CH_KICK, 0, "o...o...o...o..."),
            (CH_SNARE, 0, "..o.......o..o.."),
        ],
    ]],
    bass: &[BS_TWO],
    comp: &[CP_BLOCK4],
    second: &[SV_ANSWER],
    alts: &[(CH_KEYS, &[("Rhodes", ep_rhodes), ("guitar", nylon)])],
    ..X
};
const E_SMOOTH: Extra = Extra {
    min: EL_MIN_MORE,
    bass: &[BS_ROOT8, BS_BOSSA],
    comp: &[CP_BLOCK4, CP_STABS],
    second: &[SV_ANSWER, SV_GUIDE],
    alts: &[
        (CH_LEAD, &[("flute", flute), ("vibraphone", vibes), ("muted trumpet", muted_trumpet)]),
        (CH_KEYS, &[("piano", piano), ("guitar", nylon)]),
    ],
    lift: true,
    ..X
};

const CO_MIN_MORE: &[&str] =
    &["i bVI bIII bVII i bVI bIII bVII", "i bVII bVI bVII i bVII bVI bVII", "i bIII bVII iv i bIII bVII bVII"];
const E_UPLIFT: Extra = Extra {
    min: CO_MIN_MORE,
    bass: &[BS_ROOT4, BS_HOLD],
    comp: &[CP_BLOCK4, CP_ARP8],
    arp: &[AR_8, AR_16],
    second: &[SV_THIRDS, SV_ANSWER],
    alts: &[(CH_BASS, &[("upright bass", bass_upright)]), (CH_PAD, &[("warm pad", pad_warm)])],
    build: true,
    lift: true,
    ..X
};
const E_TECH: Extra = Extra {
    maj: &["I V vi IV I V IV IV"],
    min: &["i bVII bVI bVII i bVII bVI V", "bVI bVII i i bVI bVII bIII bIII", "i bIII bVII bVI i bIII bVII bVI"],
    grooves: &[[
        lanes![(CH_KICK, 0, "x.......x......."), (CH_SNARE, 1, BACKBEAT), (CH_HAT, HAT_C, EIGHTHS)],
        lanes![(CH_KICK, 0, FOUR), (CH_SNARE, 1, BACKBEAT), (CH_HAT, HAT_C, SIXTEENTHS), (CH_HAT, HAT_O, OFF)],
    ]],
    bass: &[BS_ROOT8, BS_DISCO],
    comp: &[CP_OFFBEAT, CP_BLOCK8],
    arp: &[AR_16, AR_PLUCK],
    alts: &[
        (CH_LEAD, &[("glockenspiel", glockenspiel), ("marimba", marimba), ("celesta", celesta)]),
        (CH_KEYS, &[("piano", piano)]),
    ],
    build: true,
    lift: true,
    ..X
};
const E_ACOUSTIC: Extra = Extra {
    min: CO_MIN_MORE,
    bass: &[BS_TWO, BS_ROOT8],
    comp: &[CP_ARP8],
    arp: &[AR_PLUCK, AR_SPARSE],
    second: &[SV_THIRDS],
    alts: &[
        (CH_LEAD, &[("glockenspiel", glockenspiel), ("celesta", celesta), ("piano", piano)]),
        (CH_ARP, &[("marimba", marimba), ("pizzicato strings", pizzicato)]),
    ],
    lift: true,
    ..X
};

const WO_MIN_MORE: &[&str] =
    &["i bVII bVI bVII i bVII bVI V", "bVI bVII i i bVI bVII i i", "i bIII bVII bVI i bIII bVII bVII"];
const WO_ALTS: &[(usize, Pal)] = &[(CH_ARP, &[("piano", piano)])];
const E_WBALLAD: Extra = Extra {
    min: WO_MIN_MORE,
    bass: &[BS_HOLD, BS_ROOT8],
    comp: &[CP_BLOCK4, CP_HOLD],
    arp: &[AR_8],
    alts: WO_ALTS,
    acoustic: true,
    lift: true,
    ..X
};
const E_HYMN: Extra = Extra {
    maj: &["I IV V I vi IV V I"],
    min: &["i bVI bIII V i iv V i", "i V i iv i iv V i", "i iv bVII bIII bVI iv V i"],
    bass: &[BS_TWO],
    comp: &[CP_HOLD],
    alts: &[
        (CH_LEAD, &[("cello", sw_cellos), ("piano", piano)]),
        (CH_PAD, &[("strings", pad_strings), ("shimmer", pad_shimmer)]),
    ],
    ..X
};
const E_WANTHEM: Extra = Extra {
    min: WO_MIN_MORE,
    bass: &[BS_ROOT4],
    comp: &[CP_BLOCK4, CP_ARP8],
    arp: &[AR_8],
    alts: WO_ALTS,
    acoustic: true,
    lift: true,
    ..X
};

const PR_ALTS: &[(usize, Pal)] =
    &[(CH_KEYS, &[("Hammond", ska_hammond)]), (CH_BASS, &[("picked bass", bass_pick_dark)])];
const E_GOSPEL: Extra = Extra {
    min: &["i iv7 i V7 i iv7 V7 i", "i7 iv7 bVII7 bIIImaj7 iv7 V7 i7 i7", "i bVI iv V7 i bVI iv V7"],
    bass: &[BS_WALK, BS_ROOT4],
    comp: &[CP_BLOCK4, CP_BLOCK8],
    second: &[SV_GUIDE],
    alts: PR_ALTS,
    acoustic: true,
    lift: true,
    ..X
};
const E_PSHUFFLE: Extra = Extra {
    maj: &["I IV I V7 I IV I+V7 I"],
    min: &["i iv i V7 i iv V7 i", "i i iv iv i V7 i i", "i bVI iv V7 i iv V7 i"],
    grooves: &[[
        lanes![
            (CH_KICK, 0, "x...........x.....x....."),
            (CH_SNARE, 0, "......x...........x....."),
            (CH_HAT, HAT_C, "x...x.x...x.x...x.x...x."),
        ],
        lanes![
            (CH_KICK, 0, "x.....x.....x.....x....."),
            (CH_SNARE, 0, "......X...........X....."),
            (CH_SNARE, 1, "......x...........x....."),
            (CH_HAT, HAT_C, "x...x.x...x.x...x.x...x."),
            (CH_PERC, SHAKER, "......x...........x....."),
        ],
    ]],
    bass: &[BS_BOOGIE, BS_WALK],
    comp: &[CP_BLOCK4, CP_STABS],
    alts: PR_ALTS,
    acoustic: true,
    lift: true,
    ..X
};
const E_PRAISE: Extra = Extra {
    maj: &["vi IV I V vi IV I V", "I V^1 vi IV I V^1 IV IV"],
    min: &["i bVI bIII bVII i bVI bIII bVII", "i bVII bVI bVII i bVII bVI V", "i bIII bVII bVI i bIII bVII bVII"],
    bass: &[BS_ROOT4],
    comp: &[CP_ARP8, CP_BLOCK8],
    arp: &[AR_8, AR_PLUCK],
    alts: &[(CH_KEYS, &[("piano", piano)]), (CH_BASS, &[("fingered bass", ska_bass_flat)])],
    acoustic: true,
    build: true,
    lift: true,
    ..X
};

/// What the style `st` has more of (by its name).
pub(super) fn extra(st: &Style) -> &'static Extra {
    match st.meter {
        "4/4 sonata" => &E_SONATA,
        "3/4 minuet" => &E_MINUET,
        "4/4 baroque" => &E_BAROQUE,
        "4/4 symphony" => &E_SYMPHONY,
        "3/4 waltz" => &E_WALTZ,
        "4/4 adagio" => &E_ADAGIO,
        "4/4 hard rock" => &E_HARD,
        "4/4 anthem" => &E_ANTHEM,
        "12/8 blues" => &E_BLUES,
        "4/4 soul" => &E_SOUL,
        "4/4 surf" => &E_SURF,
        "4/4 beat" => &E_BEAT,
        "4/4 disco" => &E_DISCO,
        "4/4 funk" => &E_FUNK,
        "4/4 soft rock" => &E_SOFT,
        "4/4 synth-pop" => &E_SYNTHPOP,
        "4/4 power ballad" => &E_BALLAD,
        "4/4 new wave" => &E_NEWWAVE,
        "4/4 eurodance" => &E_EURO,
        "4/4 grunge" => &E_GRUNGE,
        "4/4 slow jam" => &E_SLOWJAM,
        "4/4 pop-punk" => &E_PUNK,
        "4/4 r&b" => &E_RNB,
        "4/4 electro-pop" => &E_ELECTRO,
        "4/4 big room" => &E_EDM,
        "4/4 tropical house" => &E_TROPICAL,
        "4/4 trap-pop" => &E_TRAP,
        "4/4 folk stomp" => &E_STOMP,
        "4/4 nu-disco" => &E_NUDISCO,
        "4/4 amapiano" => &E_AMAPIANO,
        "4/4 afrobeats" => &E_AFRO,
        "4/4 benga" => &E_BENGA,
        "4/4 rumba" => &E_RUMBA,
        "4/4 nyatiti" => &E_NYATITI,
        "4/4 meditation" => &E_MEDITATION,
        "4/4 flow" => &E_FLOW,
        "4/4 kirtan" => &E_KIRTAN,
        "4/4 bossa nova" => &E_BOSSA,
        "4/4 lounge" => &E_LOUNGE,
        "4/4 smooth" => &E_SMOOTH,
        "4/4 uplifting" => &E_UPLIFT,
        "4/4 tech" => &E_TECH,
        "4/4 acoustic" => &E_ACOUSTIC,
        "4/4 worship ballad" => &E_WBALLAD,
        "6/8 hymn" => &E_HYMN,
        "4/4 worship anthem" => &E_WANTHEM,
        "4/4 gospel" => &E_GOSPEL,
        "12/8 praise" => &E_PSHUFFLE,
        "4/4 praise" => &E_PRAISE,
        _ => &X,
    }
}

/// The styles' names that have a row above (for the tests: each is a
/// style's).
#[cfg(test)]
pub(super) const NAMED: usize = 49;
