//! The genres from CLASSICAL on, each a few styles: a style is a table (in
//! styles.rs) of its meter, tempo, swing and modes, its chords, its form,
//! its drums' lines, how its bass, its chords, its arpeggio, its tune and
//! its second voice go, and its instruments and room. One composer here
//! plays any of them, and drums any genre's music a style's way in a mix.

use super::*;

// -- a section's part: which progression, drums and tune it takes
pub(super) const PA_INTRO: u8 = 0;
pub(super) const PA_VERSE: u8 = 1;
pub(super) const PA_CHORUS: u8 = 2;
pub(super) const PA_BRIDGE: u8 = 3;
pub(super) const PA_SOLO: u8 = 4;
pub(super) const PA_BUILD: u8 = 5;
pub(super) const PA_DROP: u8 = 6;
pub(super) const PA_OUTRO: u8 = 7;

// -- the bass's ways
pub(super) const BS_NONE: u8 = 0;
/// the root in eighths, an octave now and then, into the next chord
pub(super) const BS_ROOT8: u8 = 1;
/// the root in quarters, the fifth now and then
pub(super) const BS_ROOT4: u8 = 2;
/// root and fifth, half a bar each (a waltz's: the root on its one)
pub(super) const BS_TWO: u8 = 3;
/// a walking line in quarters, its last a half step from the next chord
pub(super) const BS_WALK: u8 = 4;
/// a soul bass's figures: dotted roots, octaves, a third, a pickup
pub(super) const BS_MOTOWN: u8 = 5;
/// octaves in eighths
pub(super) const BS_DISCO: u8 = 6;
/// syncopated 16ths with ghosts, the seventh and the octave
pub(super) const BS_FUNK: u8 = 7;
/// the off-beats (eurodance's)
pub(super) const BS_OFFBEAT: u8 = 8;
/// long low notes with the kick, sliding
pub(super) const BS_808: u8 = 9;
/// a bossa's: the root a dotted quarter, the fifth before the next
pub(super) const BS_BOSSA: u8 = 10;
/// a running line through the chord, high (benga's)
pub(super) const BS_BENGA: u8 = 11;
/// each chord's root held
pub(super) const BS_HOLD: u8 = 12;
/// amapiano's log drum
pub(super) const BS_LOG: u8 = 13;
/// a baroque bass's walking eighths
pub(super) const BS_CONTINUO: u8 = 14;
/// a boogie in eighths: root, third, fifth, sixth, seventh and back
pub(super) const BS_BOOGIE: u8 = 15;

// -- the chords' ways (the keys channel)
pub(super) const CP_NONE: u8 = 0;
/// block chords on the beats
pub(super) const CP_BLOCK4: u8 = 1;
/// in eighths
pub(super) const CP_BLOCK8: u8 = 2;
/// on the off-beats
pub(super) const CP_OFFBEAT: u8 = 3;
/// a guitar's strumming, down on the beats, up between
pub(super) const CP_STRUM: u8 = 4;
/// power chords in eighths, muted in verses, ringing in choruses
pub(super) const CP_POWER: u8 = 5;
/// broken chords in eighths
pub(super) const CP_ARP8: u8 = 6;
/// an Alberti bass: low, high, middle, high
pub(super) const CP_ALBERTI: u8 = 7;
/// each chord held
pub(super) const CP_HOLD: u8 = 8;
/// a bossa guitar's syncopated chords
pub(super) const CP_BOSSA: u8 = 9;
/// a funk guitar's muted 16ths, chords between
pub(super) const CP_SCRATCH: u8 = 10;
/// short syncopated stabs
pub(super) const CP_STABS: u8 = 11;
/// the off-beats of a march or a waltz (its pah)
pub(super) const CP_OOMPAH: u8 = 12;
/// a gospel piano's: the beats, the next chord pushed an eighth early
pub(super) const CP_GOSPEL: u8 = 13;

// -- the arpeggio's ways
pub(super) const AR_NONE: u8 = 0;
pub(super) const AR_8: u8 = 1;
pub(super) const AR_16: u8 = 2;
/// syncopated plucks (3 3 2)
pub(super) const AR_PLUCK: u8 = 3;
/// a harp's sweep up at each chord
pub(super) const AR_HARP: u8 = 4;
/// triplets, three to the beat
pub(super) const AR_TRIP: u8 = 5;
/// a note a beat, left to ring (into the delay's echoes)
pub(super) const AR_SPARSE: u8 = 6;

// -- the tune's ways
pub(super) const TU_NONE: u8 = 0;
/// a singer's: syncopated, a breath at each phrase's end
pub(super) const TU_SONG: u8 = 1;
/// a period: a phrase to a half close, the phrase again to the full close
pub(super) const TU_CLASSIC: u8 = 2;
/// long notes, sliding (a flute's alap)
pub(super) const TU_LONG: u8 = 3;
/// a short hook over and over
pub(super) const TU_HOOK: u8 = 4;
/// a riff on the chord's root
pub(super) const TU_RIFF: u8 = 5;

// -- the second voice's ways (the bell channel)
pub(super) const SV_NONE: u8 = 0;
/// the tune a third below
pub(super) const SV_THIRDS: u8 = 1;
/// the tune's long notes an octave up
pub(super) const SV_OCTAVE: u8 = 2;
/// answers in the tune's rests
pub(super) const SV_ANSWER: u8 = 3;
/// the chords' thirds and sevenths, held
pub(super) const SV_GUIDE: u8 = 4;
/// short chords off the beat (horns)
pub(super) const SV_STABS: u8 = 5;
/// a bell struck every other bar, ringing
pub(super) const SV_BOWL: u8 = 6;
/// a slow line of its own below the tune
pub(super) const SV_COUNTER: u8 = 7;

// -- the drone's ways
pub(super) const DR_NONE: u8 = 0;
/// a tanpura's four strings, one a beat (the fifth, the tonic twice, the low tonic)
pub(super) const DR_TANPURA: u8 = 1;
/// the tonic and its fifth held
pub(super) const DR_PEDAL: u8 = 2;
/// timpani: the tonic opening a loud part, a roll closing a group
pub(super) const DR_TIMPANI: u8 = 3;

// -- the fills
pub(super) const FI_NONE: u8 = 0;
/// down the toms (a small fill on the snare)
pub(super) const FI_TOMS: u8 = 1;
/// a snare roll, louder as it goes
pub(super) const FI_SNARE: u8 = 2;
/// a side stick's pickup, the beat going on
pub(super) const FI_SOFT: u8 = 3;

/// A form's row: a part, its bars, its energy, and the chance it is there.
pub(super) type FormRow = (u8, u32, f32, f32);
/// A drum's line: its channel, its kind, and a character a step (`.`
/// nothing, `o` a ghost, `-` soft, `x` a hit, `X` an accent).
pub(super) type Lane = (usize, u8, &'static str);
/// A channel's instrument: how to make it, its level, pan, reverb and delay
/// sends (the instrument first brought to a reference loudness).
pub(super) type Tone = (usize, Make, f32, f32, f32, f32);
/// A drum channel's kit: how to make it, its level, pan and reverb send.
pub(super) type KitRow = (usize, MakeKit, f32, f32, f32);
pub(super) type Pal = &'static [(&'static str, Make)];
pub(super) type KitPal = &'static [(&'static str, MakeKit)];

/// A style's instruments and room.
pub(super) struct Sound {
    pub(super) tones: &'static [Tone],
    pub(super) kits: &'static [KitRow],
    /// the reverb's feedback and level; the delay (16ths) and its feedback
    pub(super) room: (f32, f32),
    pub(super) echo: (f32, f32),
    /// anything more (the gate, the tape, a chorus, the master's tilt)
    pub(super) extra: fn(&mut Rng, &mut Synth),
    /// the instruments a section may wear instead (`Band`)
    pub(super) alts: &'static [(usize, Pal)],
    pub(super) kit_alts: &'static [(usize, KitPal)],
}

/// A style (see the module's note).
pub(super) struct Style {
    /// its meter and name, for the log ("4/4 disco")
    pub(super) meter: &'static str,
    /// 16ths a bar and a beat
    pub(super) bs: u32,
    pub(super) beat: u32,
    pub(super) bpm: (f32, f32),
    pub(super) swing: (f32, f32),
    pub(super) human: f32,
    /// the modes' weights (ionian, dorian, phrygian, lydian, mixolydian, aeolian, harmonic minor)
    pub(super) modes: [f32; 7],
    /// the progressions (eight bars, or twelve), major and minor
    pub(super) maj: &'static [&'static str],
    pub(super) min: &'static [&'static str],
    /// the outro's (a close), major and minor
    pub(super) cadence: (&'static str, &'static str),
    pub(super) form: &'static [FormRow],
    /// the layers each part plays (by part)
    pub(super) layers: [u32; 8],
    /// the drums: A (verses), B (choruses); the bar's length each
    pub(super) grooves: [&'static [Lane]; 2],
    pub(super) fill: u8,
    pub(super) bass: u8,
    pub(super) comp: u8,
    pub(super) arp: u8,
    pub(super) tune: u8,
    pub(super) second: u8,
    pub(super) drone: u8,
    /// the tune's centre (MIDI) and range (scale degrees about it), its
    /// density in verses and in choruses
    pub(super) voice: (i32, i32, i32),
    pub(super) density: (f32, f32),
    /// an intro opening the filters, an outro closing them, a build its own
    pub(super) sweep: bool,
    /// how deep the kick pumps the rest
    pub(super) pump: f32,
    /// the last bar a hit (else a chord left to ring)
    pub(super) end_hit: bool,
    pub(super) sound: Sound,
    /// each channel's trim (dB) on its level: the style's channels heard
    /// alone, measured over whole pieces, and brought to a band's balance
    /// (tools/validate/balance.py)
    pub(super) balance: &'static [(usize, f32)],
    /// the parts double-tracked (a rhythm guitar's two takes, one to each
    /// side) and the second take's share
    pub(super) double: &'static [(usize, f32)],
}

fn no_extra(_: &mut Rng, _: &mut Synth) {}

/// Where a style starts from.
pub(super) const BASE: Style = Style {
    meter: "4/4",
    bs: 16,
    beat: 4,
    bpm: (100.0, 120.0),
    swing: (0.0, 0.0),
    human: 0.006,
    modes: [4.0, 0.5, 0.0, 0.3, 0.8, 1.5, 0.0],
    maj: &[],
    min: &[],
    cadence: ("IV V7 I I", "iv V7 i i"),
    form: F_SONG,
    layers: [0; 8],
    grooves: [&[(CH_KICK, 0, "x.......x.......")], &[(CH_KICK, 0, "x.......x.......")]],
    fill: FI_TOMS,
    bass: BS_ROOT8,
    comp: CP_BLOCK4,
    arp: AR_NONE,
    tune: TU_SONG,
    second: SV_NONE,
    drone: DR_NONE,
    voice: (72, -2, 9),
    density: (0.4, 0.5),
    sweep: false,
    pump: 0.0,
    end_hit: true,
    sound: Sound {
        tones: &[],
        kits: &[],
        room: (0.8, 0.22),
        echo: (3.0, 0.0),
        extra: no_extra,
        alts: &[],
        kit_alts: &[],
    },
    balance: &[],
    double: &[],
};

// -- forms
/// intro, verse, chorus, verse, chorus, a bridge or a solo (or both), the
/// chorus once or twice, the outro
pub(super) const F_SONG: &[FormRow] = &[
    (PA_INTRO, 4, 0.4, 1.0),
    (PA_VERSE, 8, 0.55, 1.0),
    (PA_CHORUS, 8, 0.75, 1.0),
    (PA_VERSE, 8, 0.6, 1.0),
    (PA_CHORUS, 8, 0.8, 1.0),
    (PA_BRIDGE, 8, 0.55, 0.6),
    (PA_SOLO, 8, 0.8, 0.5),
    (PA_CHORUS, 8, 0.9, 1.0),
    (PA_CHORUS, 8, 0.95, 0.4),
    (PA_OUTRO, 4, 0.45, 1.0),
];
/// a dance record's: the intro (drums and bass), a build to the drop, the
/// break, another build, the drop twice, the outro
pub(super) const F_DANCE: &[FormRow] = &[
    (PA_INTRO, 8, 0.5, 1.0),
    (PA_VERSE, 8, 0.6, 1.0),
    (PA_BUILD, 8, 0.7, 1.0),
    (PA_DROP, 8, 0.95, 1.0),
    (PA_DROP, 8, 0.95, 0.5),
    (PA_BRIDGE, 8, 0.45, 1.0),
    (PA_BUILD, 8, 0.75, 1.0),
    (PA_DROP, 8, 1.0, 1.0),
    (PA_DROP, 8, 1.0, 0.6),
    (PA_OUTRO, 8, 0.5, 1.0),
];
/// a concert piece's: the theme (A), A again fuller, the contrast (B), A
/// once more, perhaps B and A again, the close
pub(super) const F_CLASSICAL: &[FormRow] = &[
    (PA_INTRO, 4, 0.35, 1.0),
    (PA_VERSE, 8, 0.5, 1.0),
    (PA_CHORUS, 8, 0.7, 1.0),
    (PA_BRIDGE, 8, 0.6, 1.0),
    (PA_VERSE, 8, 0.65, 1.0),
    (PA_BRIDGE, 8, 0.7, 0.4),
    (PA_CHORUS, 8, 0.9, 1.0),
    (PA_OUTRO, 4, 0.5, 1.0),
];
/// twelve-bar choruses: the turnaround, the head twice, a solo or two, the
/// head, the outro
pub(super) const F_BLUES: &[FormRow] = &[
    (PA_INTRO, 4, 0.5, 1.0),
    (PA_VERSE, 12, 0.6, 1.0),
    (PA_CHORUS, 12, 0.75, 1.0),
    (PA_SOLO, 12, 0.85, 1.0),
    (PA_SOLO, 12, 0.9, 0.5),
    (PA_CHORUS, 12, 0.85, 1.0),
    (PA_OUTRO, 4, 0.55, 1.0),
];
/// a vamp's: the groove in two halves, round again, a break between (the
/// sebene, the solo), the outro
pub(super) const F_VAMP: &[FormRow] = &[
    (PA_INTRO, 4, 0.45, 1.0),
    (PA_VERSE, 8, 0.6, 1.0),
    (PA_CHORUS, 8, 0.75, 1.0),
    (PA_VERSE, 8, 0.65, 1.0),
    (PA_SOLO, 8, 0.85, 1.0),
    (PA_CHORUS, 8, 0.85, 1.0),
    (PA_SOLO, 8, 0.9, 0.5),
    (PA_CHORUS, 8, 0.9, 1.0),
    (PA_OUTRO, 4, 0.5, 1.0),
];
/// a meditation's: the drone and the flute alone, the music gathering, a
/// still part, gathering again, the drone alone to close
pub(super) const F_RAGA: &[FormRow] = &[
    (PA_INTRO, 4, 0.3, 1.0),
    (PA_VERSE, 8, 0.4, 1.0),
    (PA_CHORUS, 8, 0.55, 1.0),
    (PA_VERSE, 8, 0.5, 1.0),
    (PA_BRIDGE, 4, 0.35, 1.0),
    (PA_CHORUS, 8, 0.6, 1.0),
    (PA_CHORUS, 8, 0.65, 0.5),
    (PA_OUTRO, 4, 0.3, 1.0),
];

/// Whether genre `g` is one of these.
pub(super) fn is_styled(g: usize) -> bool {
    (CLASSICAL..NG).contains(&g)
}

/// A drum line's character as a velocity.
fn vel_of(c: u8) -> f32 {
    match c {
        b'o' => 0.3,
        b'-' => 0.5,
        b'x' => 0.78,
        b'X' => 1.0,
        _ => 0.0,
    }
}

/// The step of a style's line (`sbs` 16ths, beats of `sbeat`) that step `w`
/// of this bar plays (`bs` 16ths, beats of `beat`): beat for beat (a
/// shorter bar's last beat the line's last, a longer one's middle beats its
/// middle ones round again), a simple beat's 16ths on a compound's sixths
/// (its off-beat on the beat's last third, a shuffle's), and back.
pub(super) fn map_step(w: u32, bs: u32, beat: u32, sbs: u32, sbeat: u32) -> Option<u32> {
    if bs == sbs && beat == sbeat {
        return Some(w);
    }
    let (beats, sbeats) = ((bs / beat).max(1), (sbs / sbeat).max(1));
    let (k, u) = ((w / beat).min(beats - 1), w - (w / beat).min(beats - 1) * beat);
    if u >= beat {
        return None;
    }
    let j = if k == 0 {
        0
    } else if k + 1 == beats {
        sbeats - 1
    } else if sbeats >= 3 {
        1 + (k - 1) % (sbeats - 2)
    } else {
        k % sbeats
    };
    let q = match (beat, sbeat) {
        (a, b) if a == b => Some(u),
        (6, 4) => [Some(0), None, Some(1), None, Some(2), Some(3)][u as usize],
        (4, 6) => [Some(0), Some(2), Some(4), Some(5)][u as usize],
        _ => ((u * sbeat).is_multiple_of(beat)).then_some(u * sbeat / beat),
    }?;
    Some(j * sbeat + q)
}

/// Where step `st` of a bar sits, as a weight (the downbeat 1): the beats,
/// their halves (a compound beat's eighths), the rest; a singer leans on
/// the off-beat eighths, a classical tune less on the 16ths.
fn sy_weight(st: u32, bs: u32, beat: u32, kind: u8) -> f32 {
    let st = st % bs;
    let w = if st == 0 {
        1.0
    } else if st.is_multiple_of(beat) {
        if st * 2 == bs { 0.85 } else { 0.7 }
    } else if beat == 6 {
        if st.is_multiple_of(2) { 0.4 } else { 0.1 }
    } else if st.is_multiple_of(2) {
        0.42
    } else {
        0.13
    };
    match kind {
        TU_SONG if !st.is_multiple_of(beat) && st.is_multiple_of(2) => w * 1.3,
        TU_CLASSIC if st % 2 == 1 => w * 0.5,
        TU_LONG if !st.is_multiple_of(beat) => w * 0.4,
        _ => w,
    }
}

/// A cell of `bars` bars of `bs` 16ths (`make_cell`'s, in any meter): its
/// rhythm drawn by metric weight, its contour mostly steps, turning back
/// after a leap. A cadence ends long on the tonic; a singer's cell leaves
/// its last beat for a breath.
#[allow(clippy::too_many_arguments)]
fn sy_cell(
    r: &mut Rng,
    bars: u32,
    bs: u32,
    beat: u32,
    density: f32,
    lo: i32,
    hi: i32,
    cadence: bool,
    kind: u8,
) -> Vec<MNote> {
    let steps = bars * bs;
    let mut on: Vec<u32> =
        (0..steps).filter(|&i| r.chance((sy_weight(i, bs, beat, kind) * density * 1.5).min(0.95))).collect();
    if on.first().is_none_or(|&s| s > beat) {
        on.insert(0, r.pick(&[0, 0, 2]));
    }
    if kind == TU_SONG && !cadence && on.len() > 2 {
        on.retain(|&s| s + beat < steps);
    }
    if cadence {
        let end = steps.saturating_sub(bs / 2);
        on.retain(|&s| s < end);
        on.push(end.max(on.last().map_or(0, |&s| s + 1)));
    }
    let mut deg = r.pick(&[0, 2, 4, 7]).clamp(lo, hi);
    let mut last = 0;
    let mut notes = Vec::with_capacity(on.len());
    for (k, &st) in on.iter().enumerate() {
        if k > 0 {
            let mut iv = match r.weighted(&[0.12, 0.5, 0.26, 0.12]) {
                0 => 0,
                1 => 1,
                2 => 2,
                _ => r.pick(&[3, 4, 5, 7]),
            };
            let centre = (lo + hi) as f32 / 2.0;
            let up = if last >= 3 {
                false
            } else if last <= -3 {
                true
            } else {
                r.chance((0.5 + 0.07 * (centre - deg as f32)).clamp(0.1, 0.9))
            };
            if i32::abs(last) >= 3 {
                iv = iv.min(2);
            }
            if !up {
                iv = -iv;
            }
            deg = (deg + iv).clamp(lo, hi);
            last = iv;
        }
        let next = on.get(k + 1).copied().unwrap_or(steps);
        let mut len = (next - st).max(1);
        if len > 2 && kind != TU_LONG && r.chance(0.3) {
            len /= 2;
        }
        let vel = (0.72 + 0.25 * sy_weight(st, bs, beat, kind) + r.range(-0.06, 0.06)).min(1.0);
        notes.push(MNote { step: st as u16, len: len as u16, deg, vel });
    }
    if cadence && let Some(n) = notes.last_mut() {
        let mut t = (n.deg as f32 / 7.0).round() as i32 * 7;
        if t > hi {
            t -= 7;
        }
        if t < lo {
            t += 7;
        }
        n.deg = t;
        n.len = (steps - n.step as u32) as u16;
    }
    notes
}

/// An eight-bar tune of two-bar cells: the motif, the motif moved with a new
/// ending, a contrast, the close (a classical period instead: the phrase to
/// a half close on the fifth, then again to the full close; a hook: one bar
/// over and over, its last turn changed).
pub(super) fn sy_melody(r: &mut Rng, bs: u32, beat: u32, density: f32, lo: i32, hi: i32, kind: u8) -> Vec<MNote> {
    let cell = |r: &mut Rng, bars: u32, d: f32, cad: bool| sy_cell(r, bars, bs, beat, d, lo, hi, cad, kind);
    let cells: Vec<Vec<MNote>> = match kind {
        TU_CLASSIC => {
            let a = cell(r, 2, density, false);
            let mut half = cell(r, 2, density * 0.8, true);
            if let Some(n) = half.last_mut() {
                n.deg = (n.deg + 4).clamp(lo, hi);
            }
            let close = cell(r, 2, density * 0.8, true);
            vec![a.clone(), half, a, close]
        }
        TU_HOOK => {
            let h = cell(r, 1, density, false);
            let h2: Vec<MNote> = h.iter().map(|n| MNote { step: n.step + bs as u16, ..*n }).collect();
            let two: Vec<MNote> = h.iter().chain(h2.iter()).copied().collect();
            let mut end = two.clone();
            for n in end.iter_mut().rev().take(2) {
                n.deg = (n.deg + r.pick(&[-2, -1, 1, 2])).clamp(lo, hi);
            }
            vec![two.clone(), two.clone(), two, end]
        }
        _ => {
            let a = cell(r, 2, density, false);
            let mut a2 = moved(&a, r.pick(&[1, -1, 2, -2, 3]), lo, hi);
            let k = a2.len();
            for n in a2[k.saturating_sub(2)..].iter_mut() {
                n.deg = (n.deg + r.pick(&[-2, -1, 1, 2])).clamp(lo, hi);
            }
            let db = (density * r.range(0.7, 1.3)).min(0.95);
            let b = cell(r, 2, db, false);
            let c = cell(r, 2, density * 0.8, true);
            vec![a, a2, b, c]
        }
    };
    let mut v = Vec::new();
    for (k, c) in cells.iter().enumerate() {
        v.extend(c.iter().map(|n| MNote { step: n.step + (2 * bs * k as u32) as u16, ..*n }));
    }
    v
}

/// A riff of a bar: its notes (16th, length, semitones over the chord's
/// root), from the minor pentatonic, beginning on the root.
pub(super) fn sy_riff(r: &mut Rng, bs: u32, beat: u32) -> Vec<(u16, u16, i32)> {
    let on: Vec<u32> = (0..bs)
        .filter(|&w| {
            let p = if w == 0 {
                1.0
            } else if w % beat == 0 {
                0.7
            } else if w % 2 == 0 {
                0.5
            } else if beat == 4 {
                0.18
            } else {
                0.0
            };
            r.chance(p)
        })
        .collect();
    const PENT: [i32; 6] = [0, 3, 5, 7, 10, 12];
    let mut k = 0usize;
    on.iter()
        .enumerate()
        .map(|(j, &w)| {
            if j > 0 {
                k = (k as i32 + r.pick(&[-1, 1, 1, 2, -2, 0])).clamp(0, 5) as usize;
            }
            let next = on.get(j + 1).copied().unwrap_or(bs);
            (w as u16, (next - w).max(1) as u16, PENT[k])
        })
        .collect()
}

/// The style (by its index in the genre's) a mix's drummer plays, drawn.
pub(super) fn pick_style(g: usize, r: &mut Rng) -> usize {
    let w: Vec<f32> = styles_of(g).iter().map(|x| x.0).collect();
    r.weighted(&w)
}

/// The note a channel's instrument is levelled at.
fn note_of(ch: usize) -> f32 {
    match ch {
        CH_BASS => 40.0,
        CH_DRONE => 45.0,
        CH_LEAD => 72.0,
        CH_BELL => 74.0,
        CH_ARP => 70.0,
        _ => 60.0,
    }
}

/// `p` as loud at `note` as a plain saw's at gain 0.3 (the channels' levels
/// then set the balance).
fn at_level(p: Patch, note: f32) -> Patch {
    let want = loudness(&Patch { gain: 0.3, ..PATCH }, note);
    Patch { gain: p.gain * (want / loudness(&p, note)).clamp(0.05, 20.0), ..p }
}

/// The style's instruments, kits and room on `s`, everything else shut.
pub(super) fn sounds_style(st: &Style, r: &mut Rng, s: &mut Synth) {
    for c in s.ch.iter_mut() {
        (c.gain, c.mono, c.swell, c.width, c.chorus, c.rev2, c.duck) = (0.0, false, 0.0, 1.0, 0.0, 0.0, 0.0);
        (c.lp_base, c.lp) = (20000.0, 20000.0);
    }
    let sd = &st.sound;
    for &(ch, make, gain, pan, rev, dly) in sd.tones {
        s.ch[ch].patch = at_level(make(r), note_of(ch));
        mix(&mut s.ch[ch], gain, pan, rev, dly, 0.0);
    }
    s.ch[CH_BASS].mono = true;
    for &(ch, make, gain, pan, rev) in sd.kits {
        s.ch[ch].drums = make(r);
        mix(&mut s.ch[ch], gain, pan, rev, 0.0, 0.0);
    }
    for &(ch, db) in st.balance {
        s.ch[ch].gain *= 10f32.powf(db / 20.0);
    }
    for &(ch, d) in st.double {
        (s.ch[ch].double, s.ch[ch].pan) = (d, 0.0);
    }
    mix(&mut s.ch[CH_FX], 0.6, 0.0, 0.3, 0.0, 0.0);
    s.sc_rel = (-1.0 / (0.12 * SR)).exp();
    if st.pump > 0.0 {
        for (c, k) in [
            (CH_BASS, 1.0),
            (CH_PAD, 1.2),
            (CH_KEYS, 0.8),
            (CH_ARP, 0.6),
            (CH_LEAD, 0.3),
            (CH_BELL, 0.3),
            (CH_DRONE, 0.8),
        ] {
            s.ch[c].duck = (st.pump * k).min(0.9);
        }
        s.sc_rel = (-1.0 / (0.16 * SR)).exp();
    }
    (s.rev.fb, s.rev.damp, s.rev_wet) = (sd.room.0, 0.5, sd.room.1);
    (s.dly.fb, s.dly.lpk, s.dly_rev) = (sd.echo.1, 0.25, 0.2);
    s.r2mode = R2_OFF;
    (s.fx.crackle, s.fx.hiss, s.fx.wind_to, s.fx.rain_to) = (0.0, 0.0, 0.0, 0.0);
    s.m.wow_to = 0.0;
    s.m.set_lp(20000.0);
    s.m.set_tilt(0.0, 0.0);
    (s.m.gain, s.m.target, s.m.lk, s.m.lmax) = (1.5, 0.2, 0.5, 2.0);
    (sd.extra)(r, s);
}

/// The style's instruments to swap at sections, and its kits.
pub(super) fn band_style(st: &Style, r: &mut Rng, s: &Synth) -> Band {
    let mut b = Band::default();
    for &(ch, pal) in st.sound.alts {
        b.tone(r, s, ch, pal, note_of(ch));
    }
    for &(ch, kits) in st.sound.kit_alts {
        b.kit(r, s, ch, kits);
    }
    b
}

/// A styled genre's piece's state.
#[derive(Clone, Default)]
pub(super) struct Styled {
    /// the style (its index in the genre's), its meter
    pub(super) style: usize,
    pub(super) bar_steps: u32,
    pub(super) beat_steps: u32,
    pub(super) meter: &'static str,
    /// each section's part
    parts: Vec<u8>,
    /// the tunes: the verse's, the chorus's, the bridge's, the counter-line's
    tunes: Vec<Vec<MNote>>,
    riff: Vec<(u16, u16, i32)>,
    arp: Vec<u8>,
    /// the tune's bar this bar, the second voice's (and the solo's) last note
    tune_bar: u32,
    guide: i32,
    /// a mix's drummer's style (the drummer one of these)
    pub(super) dstyle: usize,
}

/// A held chord's halves: as many as follow it unchanged, within the section.
const HOLD_MAX: u32 = 16;

impl Song {
    /// The style playing.
    pub(super) fn style(&self) -> &'static Style {
        let v = styles_of(self.genre);
        &v[self.sy.style.min(v.len() - 1)].1
    }

    /// The part section `k` plays.
    fn part_of(&self, k: usize) -> u8 {
        self.sy.parts.get(k).copied().unwrap_or(PA_VERSE)
    }

    /// The style's progressions for the mode.
    pub(super) fn sy_pool(&self) -> &'static [&'static str] {
        let st = self.style();
        let major = matches!(self.mode, IONIAN | LYDIAN | MIXOLYDIAN);
        match (major, st.maj.is_empty(), st.min.is_empty()) {
            (true, false, _) | (false, _, true) => st.maj,
            _ => st.min,
        }
    }

    /// The progression the section plays: the verses' (A), the choruses'
    /// (B), the bridges' (C), the close.
    pub(super) fn sy_which(&self) -> usize {
        match self.part_of(self.sec) {
            PA_CHORUS | PA_DROP | PA_SOLO => 1,
            PA_BRIDGE | PA_BUILD => 2,
            PA_OUTRO => 3,
            _ => 0,
        }
    }

    pub(super) fn piece_styled(&mut self, s: &mut Synth) {
        let list = styles_of(self.genre);
        let w: Vec<f32> = list.iter().map(|x| x.0).collect();
        let r = &mut self.rng;
        self.sy.style = r.weighted(&w);
        // (`MUSICBOX_STYLE=K`: the Kth style, for tools/validate to measure
        // each; the dice as they were)
        if let Some(k) = std::env::var("MUSICBOX_STYLE").ok().and_then(|v| v.parse::<usize>().ok()) {
            self.sy.style = k.min(list.len() - 1);
        }
        let st = &list[self.sy.style].1;
        self.mode = r.weighted(&st.modes);
        self.bpm = r.range(st.bpm.0, st.bpm.1) as f64;
        self.swing = r.range(st.swing.0, st.swing.1);
        self.human = st.human;
        (self.sy.bar_steps, self.sy.beat_steps, self.sy.meter) = (st.bs, st.beat, st.meter);
        // the form: each row there by its chance
        self.plan.clear();
        self.sy.parts.clear();
        for &(part, bars, energy, p) in st.form {
            if p < 1.0 && !r.chance(p) {
                continue;
            }
            let kind = match part {
                PA_INTRO => INTRO,
                PA_OUTRO => OUTRO,
                PA_BRIDGE => BREAK,
                PA_BUILD => BUILD,
                _ => MAIN,
            };
            self.plan.push(Sec { kind, bars, mask: st.layers[part as usize], energy });
            self.sy.parts.push(part);
        }
        let (centre, lo, hi) = st.voice;
        self.lead_center = centre;
        self.prange = (lo, hi);
        let (bs, beat, (dv, dc)) = (st.bs, st.beat, st.density);
        self.sy.tunes = vec![
            sy_melody(r, bs, beat, dv, lo, hi, st.tune),
            sy_melody(r, bs, beat, dc, lo + 2, hi + 2, st.tune),
            sy_melody(r, bs, beat, 0.5 * (dv + dc), lo, hi, st.tune),
            sy_melody(r, bs, beat, (dv * 0.6).min(0.3), lo - 2, hi - 4, TU_LONG),
        ];
        self.sy.riff = sy_riff(r, bs, beat);
        self.sy.arp = make_arp(r);
        self.sy.tune_bar = 0;
        self.ptrans = 0;
        sounds_style(st, r, s);
        self.band = band_style(st, r, s);
        s.dly.target = (self.step_len() * st.sound.echo.0 as f64) as f32;
        self.draw_progs();
        let major = matches!(self.mode, IONIAN | LYDIAN | MIXOLYDIAN);
        self.progs.push(halves(if major { st.cadence.0 } else { st.cadence.1 }));
    }

    pub(super) fn styled_section(&mut self, _s: &mut Synth) {
        let part = self.part_of(self.sec);
        self.sy.tune_bar = 0;
        let r = &mut self.rng;
        // a verse's tune come round again: as it was, or moved
        self.ptrans = if part == PA_VERSE && r.chance(0.3) { r.pick(&[1, -1, 2]) } else { 0 };
        if r.chance(0.3) {
            self.sy.arp = make_arp(r);
        }
    }

    /// A bar of a styled genre's music.
    pub(super) fn styled(&mut self, _s: &mut Synth, b: &mut Bar, sec: Sec, m: u32) {
        let st = self.style();
        let part = self.part_of(self.sec);
        let (bs, i) = (self.bar_steps(), self.sec_bar);
        if sec.kind == OUTRO && i + 1 == sec.bars {
            self.sy_last(b, st, m);
            return;
        }
        if part == PA_BUILD && i == 0 {
            b.ev.push((b.at(0.0, 0.0), Ev::Riser { secs: b.secs((bs * sec.bars) as f32), vel: 0.45 }));
        }
        let vb = matches!(part, PA_CHORUS | PA_DROP | PA_SOLO) || (part == PA_BRIDGE && self.var_b());
        let fill = self.fill(sec);
        if part == PA_BUILD {
            self.sy_build(b, st, sec, m);
        } else {
            let opens = matches!(part, PA_CHORUS | PA_DROP | PA_SOLO) && i == 0;
            self.sy_drums(b, st, sec, m, vb as usize, fill, opens);
        }
        if m & L_BASS != 0 {
            self.sy_bass(b, st, sec, vb);
        }
        if m & L_KEYS != 0 {
            self.sy_comp(b, st, sec, vb, part);
        }
        if m & L_PAD != 0 {
            self.sy_pads(b, sec);
        }
        if m & L_ARP != 0 {
            self.sy_arp(b, st, sec, vb);
        }
        if m & L_DRONE != 0 {
            self.sy_drone(b, st, sec, fill);
        }
        let tune = m & L_LEAD != 0 && part != PA_SOLO && st.tune != TU_RIFF && st.tune != TU_NONE;
        if m & L_LEAD != 0 {
            match part {
                PA_SOLO => self.sy_solo(b, st, sec),
                _ if st.tune == TU_RIFF => self.sy_riff_bar(b, sec),
                _ if st.tune == TU_NONE => {}
                _ => self.sy_tune(b, st, part, m, sec),
            }
        }
        if m & L_CTR != 0 {
            self.sy_second(b, st, sec, part, tune);
        }
    }

    /// A mix's bar of drums, the drummer one of these: its style's lines in
    /// the writer's meter, all of them in a main section, the hats in a
    /// break, and wherever the writer's own drums play.
    pub(super) fn styled_beat(&mut self, _s: &mut Synth, b: &mut Bar, sec: Sec, m: u32) {
        let Some(g) = self.guest.filter(|&g| is_styled(g)) else { return };
        let list = styles_of(g);
        let st = &list[self.sy.dstyle.min(list.len() - 1)].1;
        let drums = L_KICK | L_SNARE | L_HAT | L_OHAT | L_PERC;
        if sec.kind == INTRO && m & drums == 0 {
            return;
        }
        let own = match sec.kind {
            MAIN | BUILD | OUTRO => drums,
            _ => L_HAT | L_PERC,
        };
        let var = (sec.energy >= 0.7 || self.var_b()) as usize;
        let fill = self.fill(sec);
        let opens = sec.kind == MAIN && self.sec_bar == 0;
        self.sy_drums(b, st, sec, own | (m & drums), var, fill, opens);
    }

    /// A bar of the style `st`'s drums in this bar's meter (its lines
    /// written in its own): the A lines or the B (`var`), the layers `m`
    /// lets play, a fill closing a group, a crash opening a part.
    #[allow(clippy::too_many_arguments)]
    fn sy_drums(&mut self, b: &mut Bar, st: &Style, sec: Sec, m: u32, var: usize, fill: u8, crash: bool) {
        let (bs, beat) = (self.bar_steps(), self.beat_steps());
        let beats = (bs / beat).max(1);
        let (h, e) = (self.human, sec.energy);
        // where the fill takes over (the last beat; a big roll the last two)
        let from = match (st.fill, fill) {
            (FI_NONE | FI_SOFT, _) | (_, 0) => bs,
            (FI_SNARE, 2) if beats >= 4 => bs - 2 * beat,
            _ => bs.saturating_sub(beat),
        };
        let layer = |ch: usize| match ch {
            CH_KICK => L_KICK,
            CH_SNARE => L_SNARE,
            CH_HAT => L_HAT,
            CH_PERC => L_PERC,
            _ => L_OHAT,
        };
        let r = &mut self.rng;
        for &(ch, kind, pat) in st.grooves[var] {
            if m & layer(ch) == 0 || pat.is_empty() {
                continue;
            }
            let p = pat.as_bytes();
            for w in 0..bs {
                if w >= from && ch != CH_CYM && !(ch == CH_KICK && w == from) {
                    continue;
                }
                let Some(q) = map_step(w, bs, beat, st.bs, st.beat) else { continue };
                let v = vel_of(p[q as usize % p.len()]);
                if v == 0.0 {
                    continue;
                }
                let late = r.range(0.0, if ch == CH_KICK { h * 0.3 } else { h });
                b.hit(w as f32, late, ch, kind, v * (0.8 + 0.25 * e).min(1.0) * r.range(0.88, 1.0));
            }
        }
        if fill > 0 && m & (L_SNARE | L_PERC) != 0 {
            match st.fill {
                FI_SOFT => {
                    if fill == 2 {
                        let w = bs - 1;
                        b.hit(w as f32, r.range(0.0, h), CH_SNARE, 1, 0.35);
                    }
                }
                FI_NONE => {}
                _ => {
                    let n = (bs - from).max(1);
                    for (j, w) in (from..bs).enumerate() {
                        let x = (j as f32 + 1.0) / n as f32;
                        if st.fill == FI_TOMS && fill == 2 && m & L_PERC != 0 {
                            let kind = [TOM_HI, TOM_HI, TOM_MID, TOM_MID, TOM_LO, TOM_LO][(j * 6 / n as usize).min(5)];
                            b.hit(w as f32, r.range(0.0, h), CH_PERC, kind, 0.55 + 0.35 * x);
                        } else {
                            b.hit(w as f32, r.range(0.0, h), CH_SNARE, 0, 0.3 + 0.5 * x * x);
                        }
                    }
                }
            }
        }
        if crash && m & L_KICK != 0 {
            b.hit(0.0, 0.0, CH_CYM, 0, 0.75);
        }
    }

    /// A build's bar: the snare (or the clap) in quarters, then eighths,
    /// then 16ths for its last two bars, louder and louder; the hats on.
    fn sy_build(&mut self, b: &mut Bar, st: &Style, sec: Sec, m: u32) {
        let (bs, beat, i) = (self.bar_steps(), self.beat_steps(), self.sec_bar);
        let left = sec.bars - i;
        let gap = if left <= 2 {
            1
        } else if left <= sec.bars / 2 {
            beat / 2
        } else {
            beat
        };
        let x = (i as f32 + 1.0) / sec.bars as f32;
        let h = self.human;
        if m & L_SNARE != 0 {
            for w in (0..bs).step_by(gap.max(1) as usize) {
                let v = 0.35 + 0.55 * x * (0.85 + 0.15 * w as f32 / bs as f32);
                b.hit(w as f32, self.rng.range(0.0, h), CH_SNARE, 0, v);
            }
        }
        // the beat's hats go on under it
        self.sy_drums(b, st, sec, m & (L_HAT | L_OHAT), 0, 0, false);
    }

    /// The bass's bar.
    fn sy_bass(&mut self, b: &mut Bar, st: &Style, sec: Sec, vb: bool) {
        let (bs, beat) = (self.bar_steps(), self.beat_steps());
        let (half, i, h) = (bs / 2, self.sec_bar, self.human);
        let beats = (bs / beat).max(1);
        let lo = self.tonic_near(40) - 5;
        let c = [self.chord(i * 2), self.chord(i * 2 + 1)];
        let rt = [self.bass_note(c[0], lo), self.bass_note(c[1], lo)];
        let next = self.bass_note(self.chord(i * 2 + 2), lo);
        let hi_tones = [self.arp_tones(c[0], lo + 2), self.arp_tones(c[1], lo + 2)];
        let tones = [self.arp_tones(c[0], lo), self.arp_tones(c[1], lo)];
        let third = |x: Harm, n: i32| n + if q_iv(x.q)[1] == 3 { 3 } else { 4 };
        let fifth = |n: i32| if n + 7 > lo + 16 { n - 5 } else { n + 7 };
        let sc: Vec<i32> = MODES[self.mode].iter().map(|&x| (self.root + x).rem_euclid(12)).collect();
        let kick: Vec<u32> = st.grooves[vb as usize]
            .iter()
            .find(|l| l.0 == CH_KICK)
            .map(|l| {
                (0..bs)
                    .filter(|&w| {
                        map_step(w, bs, beat, st.bs, st.beat)
                            .is_some_and(|q| vel_of(l.2.as_bytes()[q as usize % l.2.len()]) > 0.0)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let e = sec.energy;
        let r = &mut self.rng;
        let at = |w: u32| (w >= half) as usize;
        let note = |b: &mut Bar, r: &mut Rng, w: f32, n: i32, v: f32, len: f32, slide: bool| {
            b.note(w, r.range(0.0, h * 0.5), CH_BASS, n, v, len, slide);
        };
        match st.bass {
            BS_NONE => {}
            BS_ROOT8 => {
                for w in (0..bs).step_by(2) {
                    let mut p = rt[at(w)];
                    if vb && w % beat != 0 && r.chance(0.12) {
                        p += 12;
                    }
                    if w + 2 >= bs && next != rt[1] && r.chance(0.5) {
                        p = next + r.pick(&[-1, 1, -2]);
                    }
                    let v = if w % beat == 0 { 0.85 } else { 0.68 };
                    note(b, r, w as f32, p, v, 1.7, false);
                }
            }
            BS_ROOT4 | BS_WALK => {
                let mut prev = rt[0];
                for k in 0..beats {
                    let w = k * beat;
                    let mut p = rt[at(w)];
                    if k > 0 && st.bass == BS_WALK {
                        // toward the next chord: its tones, a step at a time
                        let t = &tones[at(w)];
                        let aim = prev + (next - prev).signum() * 3;
                        p = t[nearest(t, aim)];
                        if p == prev {
                            p = t[(nearest(t, aim) + 1).min(t.len() - 1)];
                        }
                    } else if k == 2 && r.chance(0.35) {
                        p = fifth(rt[at(w)]);
                    }
                    if k + 1 == beats && next != rt[1] && (st.bass == BS_WALK || r.chance(0.4)) {
                        p = next + if prev > next { 1 } else { -1 };
                    }
                    note(b, r, w as f32, p, if k == 0 { 0.85 } else { 0.72 }, beat as f32 * 0.92, false);
                    prev = p;
                }
            }
            BS_TWO => {
                note(b, r, 0.0, rt[0], 0.85, half.min(beat * 2) as f32 * 0.9, false);
                if beats != 3 {
                    let p = if c[1] != c[0] { rt[1] } else { fifth(rt[0]) };
                    note(b, r, half as f32, p, 0.75, half.min(beat * 2) as f32 * 0.9, false);
                } else if i % 2 == 1 && r.chance(0.5) {
                    // a waltz's: the fifth on another bar's one now and then
                    note(b, r, 0.0, fifth(rt[0]), 0.6, beat as f32 * 0.9, false);
                }
            }
            BS_MOTOWN | BS_FUNK | BS_LOG if bs == 16 => {
                // (step, length, which: 0 root, 1 octave, 2 fifth, 3 into the next, 4 third, 5 sixth, 6 seventh; velocity)
                const MOTOWN: [&[(u32, u32, u8, f32)]; 3] = [
                    &[(0, 3, 0, 0.9), (3, 1, 0, 0.6), (6, 2, 1, 0.7), (8, 3, 2, 0.8), (11, 1, 4, 0.6), (14, 2, 3, 0.7)],
                    &[
                        (0, 2, 0, 0.9),
                        (2, 2, 4, 0.65),
                        (4, 2, 2, 0.75),
                        (6, 2, 1, 0.7),
                        (8, 2, 0, 0.85),
                        (10, 1, 2, 0.6),
                        (11, 1, 4, 0.6),
                        (12, 2, 5, 0.7),
                        (14, 2, 3, 0.7),
                    ],
                    &[(0, 4, 0, 0.9), (6, 1, 0, 0.6), (7, 1, 4, 0.6), (8, 4, 2, 0.8), (12, 2, 1, 0.7), (14, 2, 3, 0.7)],
                ];
                const FUNK: [&[(u32, u32, u8, f32)]; 2] = [
                    &[
                        (0, 2, 0, 0.95),
                        (3, 1, 0, 0.55),
                        (6, 1, 1, 0.75),
                        (7, 1, 0, 0.3),
                        (10, 2, 2, 0.8),
                        (12, 1, 0, 0.35),
                        (14, 1, 6, 0.7),
                        (15, 1, 0, 0.5),
                    ],
                    &[
                        (0, 1, 0, 1.0),
                        (2, 1, 0, 0.35),
                        (3, 2, 1, 0.75),
                        (6, 2, 2, 0.7),
                        (9, 1, 0, 0.35),
                        (10, 1, 6, 0.7),
                        (11, 1, 1, 0.75),
                        (14, 2, 3, 0.7),
                    ],
                ];
                const LOG: [&[(u32, u32, u8, f32)]; 2] = [
                    &[
                        (0, 2, 0, 0.9),
                        (3, 2, 0, 0.75),
                        (6, 2, 1, 0.7),
                        (10, 2, 0, 0.8),
                        (12, 1, 2, 0.6),
                        (14, 2, 0, 0.75),
                    ],
                    &[
                        (0, 3, 0, 0.95),
                        (3, 1, 1, 0.7),
                        (7, 2, 0, 0.8),
                        (10, 3, 2, 0.75),
                        (13, 1, 0, 0.6),
                        (14, 2, 1, 0.7),
                    ],
                ];
                let cell = match st.bass {
                    BS_MOTOWN => MOTOWN[(i as usize + vb as usize) % 3],
                    BS_FUNK => FUNK[vb as usize],
                    _ => LOG[(i % 2) as usize],
                };
                for &(w, len, which, v) in cell {
                    let (x, n) = (c[at(w)], rt[at(w)]);
                    let p = match which {
                        1 => n + 12,
                        2 => fifth(n),
                        3 if next != n => next + if r.bit_chance() { 1 } else { -1 },
                        3 => fifth(n),
                        4 => third(x, n),
                        5 => n + 9,
                        6 => n + 10,
                        _ => n,
                    };
                    let slide = st.bass == BS_LOG && which != 0 && r.chance(0.3);
                    note(b, r, w as f32, p, v, len as f32 * 0.9, slide);
                }
            }
            BS_DISCO | BS_OFFBEAT => {
                for k in 0..beats {
                    let w = k * beat;
                    let n = rt[at(w)];
                    let off = w + beat / 2;
                    if st.bass == BS_DISCO {
                        note(b, r, w as f32, n, 0.85, beat as f32 * 0.4, false);
                    }
                    let up = st.bass == BS_DISCO || (vb && k % 2 == 1);
                    note(b, r, off as f32, if up { n + 12 } else { n }, 0.8, beat as f32 * 0.42, false);
                }
            }
            BS_808 => {
                let hits = if kick.is_empty() { vec![0, half] } else { kick.clone() };
                let mut prev = rt[0];
                for (j, &w) in hits.iter().enumerate() {
                    let end = hits.get(j + 1).copied().unwrap_or(bs);
                    let p = rt[at(w)] - 12 + if lo - 12 < 28 { 12 } else { 0 };
                    let slide = j > 0 && p != prev && r.chance(0.35);
                    note(b, r, w as f32, p, if w == 0 { 0.95 } else { 0.8 }, (end - w) as f32 - 0.2, slide);
                    prev = p;
                }
            }
            BS_BOSSA if bs == 16 => {
                note(b, r, 0.0, rt[0], 0.85, 5.0, false);
                note(b, r, 6.0, fifth(rt[0]) - 12 + if fifth(rt[0]) - 12 < lo { 12 } else { 0 }, 0.7, 2.0, false);
                note(b, r, 8.0, rt[1], 0.8, 5.0, false);
                let n = if next != rt[1] { next + 7 - if next + 7 > lo + 16 { 12 } else { 0 } } else { fifth(rt[1]) };
                note(b, r, 14.0, n, 0.7, 2.0, false);
            }
            BS_BENGA => {
                // eighths through the chord, a 16th's pickup into the half
                // bar now and then (in the choruses)
                let mut k = 0usize;
                for w in (0..bs).step_by(2) {
                    let t = &hi_tones[at(w)];
                    let order = [0usize, 1, 2, 1, 0, 2, 3, 2];
                    let p = t[order[k % order.len()].min(t.len() - 1)];
                    k += 1;
                    let v = if w % beat == 0 { 0.85 } else { 0.62 + 0.1 * e };
                    let pickup = vb && beat == 4 && (w + 2) % half == 0 && r.chance(0.5);
                    note(b, r, w as f32, p, v, if pickup { 0.9 } else { 1.7 }, false);
                    if pickup {
                        let q = t[(order[k % order.len()] + 1).min(t.len() - 1)];
                        note(b, r, w as f32 + 1.0, q, v * 0.85, 0.9, false);
                    }
                }
            }
            BS_CONTINUO => {
                for w in (0..bs).step_by(2) {
                    let n = rt[at(w)];
                    let t = &tones[at(w)];
                    let p = if w % beat == 0 {
                        if w % half == 0 { n } else { t[(w / beat) as usize % t.len()] }
                    } else {
                        // a passing tone of the scale toward the next beat's
                        let to = if w + 2 >= bs { next } else { t[((w / beat) as usize + 1) % t.len()] };
                        let from = t[(w / beat) as usize % t.len()];
                        let between: Vec<i32> =
                            (from.min(to) + 1..from.max(to)).filter(|x| sc.contains(&x.rem_euclid(12))).collect();
                        between.get(between.len() / 2).copied().unwrap_or(from)
                    };
                    note(b, r, w as f32, p, if w % beat == 0 { 0.8 } else { 0.65 }, 1.8, false);
                }
            }
            BS_BOOGIE => {
                const UP: [i32; 8] = [0, 4, 7, 9, 10, 9, 7, 4];
                let off = if beat == 6 { 4 } else { 2 };
                for k in 0..beats {
                    for (j, o) in [0, off].iter().enumerate() {
                        let w = k * beat + o;
                        let n = rt[at(w)];
                        let mut iv = UP[(2 * k as usize + j) % 8];
                        if iv == 4 && q_iv(c[at(w)].q)[1] == 3 {
                            iv = 3;
                        }
                        note(b, r, w as f32, n + iv, if j == 0 { 0.85 } else { 0.7 }, (off as f32) * 0.9, false);
                    }
                }
            }
            _ => {
                // each chord's root held (and BS_HOLD itself)
                for hh in 0..2u32 {
                    let k = i * 2 + hh;
                    if !(hh == 0 || c[1] != c[0]) {
                        continue;
                    }
                    let mut n = 1;
                    while n < HOLD_MAX && n + k < sec.bars * 2 && self.chord(k + n) == c[hh as usize] {
                        n += 1;
                    }
                    note(b, &mut self.rng, (hh * half) as f32, rt[hh as usize], 0.8, (n * half) as f32 - 0.5, false);
                }
            }
        }
    }

    /// The keys' (the chords') bar.
    fn sy_comp(&mut self, b: &mut Bar, st: &Style, sec: Sec, vb: bool, part: u8) {
        let (bs, beat) = (self.bar_steps(), self.beat_steps());
        let (half, i, h) = (bs / 2, self.sec_bar, self.human);
        let beats = (bs / beat).max(1);
        let e = sec.energy;
        let c = [self.chord(i * 2), self.chord(i * 2 + 1)];
        let next = self.chord(i * 2 + 2);
        let at = |w: u32| (w >= half) as usize;
        let (lo, hi) = if matches!(st.comp, CP_STRUM | CP_BOSSA | CP_SCRATCH) { (48, 72) } else { (52, 76) };
        let v = [self.voicing(c[0], lo, hi, &self.keys_v, false), self.voicing(c[1], lo, hi, &self.keys_v, false)];
        self.keys_v = v[1].clone();
        let power = [self.bass_note(c[0], 40), self.bass_note(c[1], 40)];
        let vel = 0.5 + 0.25 * e;
        let r = &mut self.rng;
        let chord = |b: &mut Bar, r: &mut Rng, w: f32, notes: &[i32], v: f32, len: f32, gap: f32| {
            let late = r.range(0.0, h);
            for (k, &n) in notes.iter().enumerate() {
                b.note(w, late + gap * k as f32, CH_KEYS, n, v * r.range(0.9, 1.05), len, false);
            }
        };
        match st.comp {
            CP_NONE => {}
            CP_BLOCK4 | CP_GOSPEL => {
                for k in 0..beats {
                    let w = k * beat;
                    let acc = if k % 2 == 0 { 1.0 } else { 0.85 };
                    let push = st.comp == CP_GOSPEL && k + 1 == beats && next != c[1];
                    chord(b, r, w as f32, &v[at(w)], vel * acc, beat as f32 * if push { 0.45 } else { 0.85 }, 0.004);
                }
                if st.comp == CP_GOSPEL && next != c[1] {
                    // the next chord an eighth early, tied over
                    let w = bs - beat / 2;
                    let nx = self.voicing(next, lo, hi, &v[1], false);
                    chord(b, &mut self.rng, w as f32, &nx, vel * 0.9, beat as f32, 0.004);
                }
            }
            CP_BLOCK8 | CP_OFFBEAT => {
                let gap = (beat / 2).max(1);
                for w in (0..bs).step_by(gap as usize) {
                    let off = w % beat != 0;
                    if st.comp == CP_OFFBEAT && !off {
                        continue;
                    }
                    let acc = if w % beat == 0 {
                        1.0
                    } else if st.comp == CP_OFFBEAT {
                        0.95
                    } else {
                        0.75
                    };
                    chord(b, r, w as f32, &v[at(w)], vel * acc, gap as f32 * 0.8, 0.002);
                }
            }
            CP_STRUM => {
                let up = if beat == 6 { [0, 2, 4] } else { [0, 2, 3] };
                for k in 0..beats {
                    for (j, &o) in up.iter().enumerate() {
                        let w = k * beat + o;
                        if w >= bs || (j == 2 && !vb && r.chance(0.5)) || (j == 1 && k == 0 && r.chance(0.5)) {
                            continue;
                        }
                        let down = j == 0;
                        let mut notes = v[at(w)].clone();
                        if !down {
                            notes.reverse();
                            notes.truncate(3);
                        }
                        let a = if down { if k % 2 == 0 { 0.95 } else { 0.85 } } else { 0.6 };
                        let gap = r.range(0.007, 0.014);
                        chord(b, r, w as f32, &notes, vel * a, beat as f32 * 0.9, gap);
                    }
                }
            }
            CP_POWER => {
                let open = matches!(part, PA_CHORUS | PA_SOLO | PA_DROP);
                for w in (0..bs).step_by(2) {
                    let root = power[at(w)];
                    let notes = [root, root + 7, root + 12];
                    let change = w == 0 || (w == half && c[1] != c[0]);
                    if open && !change && w % beat != 0 && r.chance(0.6) {
                        continue;
                    }
                    let len = if open && change { (if c[1] != c[0] { half } else { bs }) as f32 * 0.95 } else { 1.2 };
                    let v = if change {
                        0.85
                    } else if open {
                        0.6
                    } else {
                        0.5
                    };
                    chord(b, r, w as f32, &notes, v, len, 0.003);
                }
            }
            CP_ARP8 | CP_ALBERTI => {
                let gap = if beat == 6 {
                    2
                } else if st.comp == CP_ALBERTI && st.bpm.1 < 100.0 {
                    1
                } else {
                    2
                };
                for (k, w) in (0..bs).step_by(gap).enumerate() {
                    let t = &v[at(w)];
                    let n = if st.comp == CP_ALBERTI {
                        // the chord's low, high, middle, high (its root low under it)
                        let low = self.bass_note(c[at(w)], lo - 2);
                        let top = *t.last().unwrap_or(&low);
                        let mid = t[t.len() / 2];
                        [low, top, mid, top][k % 4]
                    } else {
                        let order = [0usize, 1, 2, 3, 2, 1];
                        t[order[k % order.len()].min(t.len() - 1)]
                    };
                    let accent = if w % beat == 0 { 1.0 } else { 0.8 };
                    b.note(w as f32, self.rng.range(0.0, h), CH_KEYS, n, vel * accent, gap as f32 * 1.6, false);
                }
            }
            CP_BOSSA if bs == 16 => {
                const HITS: [&[u32]; 2] = [&[0, 3, 6, 10, 12], &[2, 6, 8, 11, 14]];
                for &w in HITS[(i % 2) as usize] {
                    let notes: Vec<i32> = v[at(w)].iter().rev().take(3).copied().collect();
                    chord(b, r, w as f32, &notes, vel * if w % 4 == 0 { 0.9 } else { 0.8 }, 1.8, 0.006);
                }
            }
            CP_SCRATCH | CP_STABS => {
                let hits: &[u32] = match (st.comp, vb, beat) {
                    (_, _, 6) => &[0, 4, 10, 16, 22],
                    (CP_SCRATCH, false, _) => &[0, 6, 10, 14],
                    (CP_SCRATCH, true, _) => &[3, 6, 11, 14],
                    (_, false, _) => &[0, 6, 10],
                    _ => &[3, 6, 11, 14],
                };
                if st.comp == CP_SCRATCH {
                    for w in 0..bs {
                        if hits.contains(&w) {
                            continue;
                        }
                        // the muted strings, scratched
                        let n = v[at(w)][v[at(w)].len() / 2];
                        b.note(
                            w as f32,
                            r.range(0.0, h),
                            CH_KEYS,
                            n,
                            0.18 + 0.06 * (w % 2 == 0) as i32 as f32,
                            0.35,
                            false,
                        );
                    }
                }
                for &w in hits.iter().filter(|&&w| w < bs) {
                    chord(b, r, w as f32, &v[at(w)], vel, 1.2, 0.003);
                }
            }
            CP_OOMPAH => {
                for k in 1..beats {
                    if beats != 3 && k % 2 == 0 {
                        continue;
                    }
                    let w = k * beat;
                    chord(b, r, w as f32, &v[at(w)], vel * 0.85, beat as f32 * 0.55, 0.002);
                }
            }
            _ => {
                // each chord held (and CP_HOLD itself)
                for hh in 0..2u32 {
                    if !(hh == 0 || c[1] != c[0]) {
                        continue;
                    }
                    let len = if c[1] != c[0] { half } else { bs };
                    chord(b, r, (hh * half) as f32, &v[hh as usize], vel, len as f32 + 0.5, 0.0);
                }
            }
        }
    }

    /// The pad: a chord at each change, voice-led, held until the next.
    fn sy_pads(&mut self, b: &mut Bar, sec: Sec) {
        let (bs, i) = (self.bar_steps(), self.sec_bar);
        let half = bs / 2;
        let vel = 0.5 + 0.2 * sec.energy;
        for hh in 0..2u32 {
            let k = i * 2 + hh;
            let c = self.chord(k);
            if !(k == 0 || hh == 0 && i == 0 || self.chord(k - 1) != c) {
                continue;
            }
            let mut n = 1;
            while n < HOLD_MAX && k + n < sec.bars * 2 && self.chord(k + n) == c {
                n += 1;
            }
            let v = self.voicing(c, 53, 77, &self.pad_v, false);
            for (j, &note) in v.iter().enumerate() {
                b.note(
                    (hh * half) as f32,
                    0.0,
                    CH_PAD,
                    note,
                    vel * (1.0 - 0.05 * j as f32),
                    (n * half) as f32 + 1.0,
                    false,
                );
            }
            self.pad_v = v;
        }
    }

    /// The arpeggio's bar.
    fn sy_arp(&mut self, b: &mut Bar, st: &Style, sec: Sec, vb: bool) {
        let (bs, beat) = (self.bar_steps(), self.beat_steps());
        let (half, i, h) = (bs / 2, self.sec_bar, self.human);
        let c = [self.chord(i * 2), self.chord(i * 2 + 1)];
        let lo = if st.arp == AR_HARP { self.tonic_near(55) - 7 } else { self.tonic_near(64) - 5 };
        let tones = [self.arp_tones(c[0], lo), self.arp_tones(c[1], lo)];
        let order = self.sy.arp.clone();
        let vel = 0.45 + 0.25 * sec.energy;
        let r = &mut self.rng;
        if st.arp == AR_HARP {
            for hh in 0..2u32 {
                if !(hh == 0 || c[1] != c[0]) {
                    continue;
                }
                let t = &tones[hh as usize];
                let span = if c[1] != c[0] { half } else { bs };
                for (j, &n) in t.iter().take(8).enumerate() {
                    let w = hh * half + j as u32 * (span / 8).max(1);
                    b.note(
                        w as f32,
                        r.range(0.0, h),
                        CH_ARP,
                        n,
                        vel * (0.7 + 0.04 * j as f32),
                        (span - j as u32) as f32,
                        false,
                    );
                }
            }
            return;
        }
        let steps: Vec<f32> = match st.arp {
            AR_16 => (0..bs).map(|w| w as f32).collect(),
            AR_TRIP if beat == 4 => (0..bs / beat)
                .flat_map(|k| (0..3).map(move |j| (k * beat) as f32 + j as f32 * beat as f32 / 3.0))
                .collect(),
            AR_SPARSE => (0..bs).step_by(beat as usize).map(|w| w as f32).collect(),
            AR_PLUCK if bs == 16 => {
                if vb {
                    vec![0.0, 3.0, 6.0, 8.0, 10.0, 12.0, 14.0]
                } else {
                    vec![0.0, 3.0, 6.0, 10.0, 12.0]
                }
            }
            _ => (0..bs).step_by(2).map(|w| w as f32).collect(),
        };
        let gap = bs as f32 / steps.len().max(1) as f32;
        for (k, &w) in steps.iter().enumerate() {
            let t = &tones[(w >= half as f32) as usize];
            let idx = order[(k + i as usize * steps.len()) % order.len()] as usize % t.len();
            let len = match st.arp {
                AR_PLUCK => gap * 2.5,
                AR_SPARSE => gap * 1.8,
                _ => gap * 0.9,
            };
            b.note(
                w,
                r.range(0.0, h * 0.5),
                CH_ARP,
                t[idx],
                vel * if w % beat as f32 == 0.0 { 1.0 } else { 0.85 },
                len,
                false,
            );
        }
    }

    /// The drone's bar.
    fn sy_drone(&mut self, b: &mut Bar, st: &Style, sec: Sec, fill: u8) {
        let (bs, beat, i) = (self.bar_steps(), self.beat_steps(), self.sec_bar);
        let beats = (bs / beat).max(1);
        let e = sec.energy;
        match st.drone {
            DR_TANPURA => {
                let t = self.tonic_near(50);
                let strings = [t - 5, t, t, t - 12];
                for k in 0..beats {
                    let n = strings[((k + i * beats) % 4) as usize];
                    b.note((k * beat) as f32, 0.0, CH_DRONE, n, 0.5, beat as f32 * 2.5, false);
                }
            }
            DR_TIMPANI => {
                let t = self.tonic_near(45);
                let five = if t + 7 > 52 { t - 5 } else { t + 7 };
                let root = self.chord(i * 2).root.rem_euclid(12);
                if i == 0 && e >= 0.6 {
                    b.note(0.0, 0.0, CH_DRONE, t, 0.85, beat as f32 * 2.0, false);
                } else if e >= 0.8 && (root == 0 || root == 7) {
                    b.note(0.0, 0.0, CH_DRONE, if root == 0 { t } else { five }, 0.7, beat as f32, false);
                }
                if fill == 2 && e >= 0.6 {
                    // a roll on the dominant, swelling into the next part
                    let from = bs - beat;
                    for k in 0..beat * 2 {
                        let x = (k + 1) as f32 / (beat * 2) as f32;
                        b.note(from as f32 + k as f32 * 0.5, 0.0, CH_DRONE, five, 0.35 + 0.5 * x, 0.6, false);
                    }
                }
            }
            DR_PEDAL => {
                // the tonic and its fifth held
                if i % 2 == 0 {
                    let t = self.tonic_near(43);
                    for n in [t, t + 7] {
                        b.note(0.0, 0.0, CH_DRONE, n, 0.5, (2 * bs) as f32 + 1.0, false);
                    }
                }
            }
            _ => {}
        }
    }

    /// The tune's bar (the second voice too when it doubles it).
    fn sy_tune(&mut self, b: &mut Bar, st: &Style, part: u8, m: u32, sec: Sec) {
        let (bs, beat) = (self.bar_steps(), self.beat_steps());
        let half = bs / 2;
        let k = match part {
            PA_CHORUS | PA_DROP => 1,
            PA_BRIDGE | PA_BUILD => 2,
            _ => 0,
        };
        let pb = self.sy.tune_bar % 8;
        if pb == 0 && self.sy.tune_bar > 0 && self.rng.chance(0.4) {
            self.ptrans = self.rng.pick(&[0, 1, -1, 2]);
        }
        self.sy.tune_bar += 1;
        let tonic = self.tonic_near(self.lead_center);
        let i = self.sec_bar;
        let vel = 0.7 + 0.25 * sec.energy;
        let legato = if st.tune == TU_LONG { 1.0 } else { 0.9 };
        let double = if m & L_CTR != 0 { st.second } else { SV_NONE };
        for j in 0..self.sy.tunes[k].len() {
            let n = self.sy.tunes[k][j];
            if n.step as u32 / bs != pb {
                continue;
            }
            let w = n.step as u32 % bs;
            let c = self.chord(i * 2 + (w >= half) as u32);
            let strong = (w.is_multiple_of(beat) && (w == 0 || w == half)) || n.len as u32 >= beat;
            let p = tonic + self.mel_pitch(n.deg + self.ptrans, c, strong);
            let late = self.rng.range(0.0, self.human);
            let slide = st.tune == TU_LONG && j > 0 && self.rng.chance(0.3);
            let len = n.len as f32 * legato;
            b.note(w as f32, late, CH_LEAD, p, n.vel * vel, len, slide);
            match double {
                SV_THIRDS => {
                    let q = tonic + self.mel_pitch(n.deg + self.ptrans - 2, c, strong);
                    b.note(w as f32, late + 0.004, CH_BELL, q, n.vel * vel * 0.75, len, false);
                }
                SV_OCTAVE if w.is_multiple_of(beat) || n.len as u32 >= beat => {
                    b.note(w as f32, late, CH_BELL, p + 12, n.vel * vel * 0.7, len.min(beat as f32), false);
                }
                _ => {}
            }
        }
    }

    /// The riff's bar, on each chord's root (its second bar of two now and
    /// then ending otherwise).
    fn sy_riff_bar(&mut self, b: &mut Bar, sec: Sec) {
        let (bs, beat) = (self.bar_steps(), self.beat_steps());
        let (half, i, h) = (bs / 2, self.sec_bar, self.human);
        let lo = self.tonic_near(52) - 5;
        let roots = [self.bass_note(self.chord(i * 2), lo), self.bass_note(self.chord(i * 2 + 1), lo)];
        let vel = 0.7 + 0.2 * sec.energy;
        let riff = self.sy.riff.clone();
        let r = &mut self.rng;
        let vary = i % 2 == 1 && r.chance(0.5);
        for (k, &(w, len, semi)) in riff.iter().enumerate() {
            let w = w as u32;
            if w >= bs {
                continue;
            }
            let root = roots[(w >= half) as usize];
            let p = if vary && k + 2 >= riff.len() { root + r.pick(&[0, 3, 5, 7, 10, 12]) } else { root + semi };
            let a = if w.is_multiple_of(beat) { 1.0 } else { 0.85 };
            b.note(w as f32, r.range(0.0, h), CH_LEAD, p, vel * a, len as f32 * 0.9, false);
        }
    }

    /// A solo's bar: runs up and down the pentatonic (the minor's, but a
    /// major song's sweet one), every second bar ending on a long note.
    fn sy_solo(&mut self, b: &mut Bar, st: &Style, sec: Sec) {
        let (bs, beat, i) = (self.bar_steps(), self.beat_steps(), self.sec_bar);
        let major = MODES[self.mode][2] == 4 && st.tune != TU_RIFF;
        let pent: [i32; 5] = if major { [0, 2, 4, 7, 9] } else { [0, 3, 5, 7, 10] };
        let base = self.tonic_near(self.lead_center) - 12;
        let scale: Vec<i32> = (0..3).flat_map(|o| pent.iter().map(move |&x| base + 12 * o + x)).collect();
        let from = if self.sy.guide == 0 { self.tonic_near(self.lead_center) } else { self.sy.guide };
        let mut k = nearest(&scale, from);
        let (e, h) = (sec.energy, self.human);
        let r = &mut self.rng;
        let mut dir: i32 = if r.bit_chance() { 1 } else { -1 };
        let mut w = 0u32;
        while w < bs {
            let left = bs - w;
            if i % 2 == 1 && left <= beat {
                b.note(w as f32, r.range(0.0, h), CH_LEAD, scale[k], 0.85, (left + beat) as f32, r.chance(0.4));
                break;
            }
            let run = beat == 4 && r.chance(0.2 + 0.3 * e);
            let gap = if run { 1 } else { 2 };
            if !r.chance(0.12) {
                let v = if w.is_multiple_of(beat) { 0.9 } else { 0.7 };
                b.note(w as f32, r.range(0.0, h), CH_LEAD, scale[k], v, gap as f32 * 0.95, false);
            }
            if r.chance(0.2) {
                dir = -dir;
            }
            let jump = if r.chance(0.15) { 2 } else { 1 };
            let nk = k as i32 + dir * jump;
            if nk < 2 || nk as usize + 2 >= scale.len() {
                dir = -dir;
            }
            k = (k as i32 + dir * jump).clamp(0, scale.len() as i32 - 1) as usize;
            w += gap;
        }
        self.sy.guide = scale[k];
    }

    /// The second voice's bar (but the tune's doubles, played with it).
    fn sy_second(&mut self, b: &mut Bar, st: &Style, sec: Sec, part: u8, tune: bool) {
        let (bs, beat, i) = (self.bar_steps(), self.beat_steps(), self.sec_bar);
        let half = bs / 2;
        let beats = (bs / beat).max(1);
        let h = self.human;
        let vel = 0.5 + 0.2 * sec.energy;
        match st.second {
            SV_THIRDS | SV_OCTAVE if tune => {}
            SV_ANSWER if tune => {
                // the tune's rests of a beat or more, answered (two at most)
                let k = match part {
                    PA_CHORUS | PA_DROP => 1,
                    PA_BRIDGE | PA_BUILD => 2,
                    _ => 0,
                };
                let pb = (self.sy.tune_bar + 7) % 8;
                let mut busy = vec![false; bs as usize];
                for n in self.sy.tunes[k].iter().filter(|n| n.step as u32 / bs == pb) {
                    let w = n.step as u32 % bs;
                    for x in busy[w as usize..(w + n.len as u32).min(bs) as usize].iter_mut() {
                        *x = true;
                    }
                }
                let base = self.tonic_near(self.lead_center - 5);
                let mut w = 0;
                let mut figures = 0;
                while w < bs && figures < 2 {
                    let run = (w..bs).take_while(|&x| !busy[x as usize]).count() as u32;
                    if run >= beat && w % 2 == 0 {
                        let c = self.chord(i * 2 + (w >= half) as u32);
                        let t: Vec<i32> = self.arp_tones(c, base - 7).into_iter().filter(|&p| p <= base + 7).collect();
                        let mut j = nearest(&t, if self.sy.guide == 0 { base } else { self.sy.guide });
                        let up = self.rng.bit_chance();
                        for n in 0..(run / 2).min(3) {
                            let at = w + 2 * n;
                            b.note(
                                at as f32,
                                self.rng.range(0.0, h),
                                CH_BELL,
                                t[j],
                                vel * if n == 0 { 1.0 } else { 0.85 },
                                1.8,
                                false,
                            );
                            self.sy.guide = t[j];
                            j = if up { (j + 1).min(t.len() - 1) } else { j.saturating_sub(1) };
                        }
                        figures += 1;
                        w += run;
                    } else {
                        w += run.max(1);
                    }
                }
            }
            SV_STABS => {
                let hits: Vec<u32> = if bs == 16 {
                    if part == PA_CHORUS || self.var_b() { vec![3, 6, 11, 14] } else { vec![6, 14] }
                } else {
                    (0..beats).filter(|k| k % 2 == 1).map(|k| k * beat + beat / 2).collect()
                };
                for w in hits {
                    let c = self.chord(i * 2 + (w >= half) as u32);
                    let v = self.voicing(c, 57, 76, &[], false);
                    let late = self.rng.range(0.0, h);
                    for n in v {
                        b.note(w as f32, late, CH_BELL, n, vel, 1.4, false);
                    }
                }
            }
            SV_BOWL => {
                if i % 2 == 0 {
                    let t = self.tonic_near(self.lead_center);
                    let n = if i % 4 == 2 { t + 7 } else { t };
                    b.note(0.0, 0.0, CH_BELL, n, 0.45, (2 * bs) as f32, false);
                }
            }
            SV_COUNTER => {
                let pb = i % 8;
                let tonic = self.tonic_near(self.lead_center - 12);
                for j in 0..self.sy.tunes[3].len() {
                    let n = self.sy.tunes[3][j];
                    if n.step as u32 / bs != pb {
                        continue;
                    }
                    let w = n.step as u32 % bs;
                    let c = self.chord(i * 2 + (w >= half) as u32);
                    let p = tonic + self.mel_pitch(n.deg, c, true);
                    b.note(w as f32, self.rng.range(0.0, h), CH_BELL, p, n.vel * vel, n.len as f32, false);
                }
            }
            _ => {
                // the chords' thirds and sevenths, held, each near the last
                let base = self.tonic_near(self.lead_center - 7);
                for hh in 0..2u32 {
                    let k = i * 2 + hh;
                    let c = self.chord(k);
                    if !(k == 0 || hh == 0 || self.chord(k - 1) != c) {
                        continue;
                    }
                    let iv = q_iv(c.q);
                    let guides: Vec<i32> = [iv[1], *iv.get(3).unwrap_or(&iv[2])]
                        .iter()
                        .map(|x| (self.root + c.root + x).rem_euclid(12))
                        .collect();
                    let cands = in_range(&guides, base - 5, base + 9);
                    if cands.is_empty() {
                        continue;
                    }
                    let p = cands[nearest(&cands, if self.sy.guide == 0 { base } else { self.sy.guide })];
                    let len = if hh == 0 && self.chord(k + 1) == c { bs } else { half };
                    b.note((hh * half) as f32, 0.0, CH_BELL, p, vel * 0.8, len as f32, false);
                    self.sy.guide = p;
                }
            }
        }
    }

    /// The piece's last bar: the home chord, held, on what still plays; a
    /// hit under it where the style ends on one.
    fn sy_last(&mut self, b: &mut Bar, st: &Style, m: u32) {
        let bs = self.bar_steps();
        let major = MODES[self.mode][2] == 4;
        let home = Harm { root: 0, q: if major { Q_MAJ } else { Q_MIN }, inv: 0 };
        let len = bs as f32 + 4.0;
        for (layer, ch, lo, hi) in [(L_KEYS, CH_KEYS, 52, 76), (L_PAD, CH_PAD, 53, 77)] {
            if m & layer != 0 {
                let prev = if ch == CH_KEYS { self.keys_v.clone() } else { self.pad_v.clone() };
                for n in self.voicing(home, lo, hi, &prev, false) {
                    b.note(0.0, 0.0, ch, n, 0.6, len, false);
                }
            }
        }
        if m & L_BASS != 0 {
            b.note(0.0, 0.0, CH_BASS, self.bass_note(home, self.tonic_near(40) - 5), 0.8, len, false);
        }
        if m & L_LEAD != 0 {
            b.note(0.0, 0.0, CH_LEAD, self.tonic_near(self.lead_center), 0.7, len, false);
        }
        if m & L_DRONE != 0 {
            b.note(0.0, 0.0, CH_DRONE, self.tonic_near(43), 0.6, len, false);
        }
        if st.end_hit && m & L_KICK != 0 {
            b.hit(0.0, 0.0, CH_KICK, 0, 0.9);
            b.hit(0.0, 0.0, CH_CYM, 0, 0.8);
        }
    }
}
