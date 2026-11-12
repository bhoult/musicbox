//! The styled genres' tables (styled.rs plays them): for each genre its
//! styles, each style's meter, tempo, chords, form, drums, ways and
//! instruments; the instruments these genres add; each genre's palettes for
//! a mix, its room and its tempo.
//!
//! CLASSICAL: a piano sonata's allegro (an Alberti bass under a periodic
//! tune), a string quartet's minuet, a baroque trio sonata (harpsichord,
//! continuo, two violins in thirds). ORCHESTRA: a romantic symphony's
//! movement (strings, horns' counter-line, brass chorale, harp, timpani), a
//! Viennese waltz, a strings' adagio. ROCK: riff-driven hard rock, an arena
//! anthem, a 12/8 blues shuffle. The decades: 60S soul (the snare on every
//! beat, a tambourine, a running bass, horns), surf (a tremolo guitar,
//! toms), beat (twelve-string jangle, harmonica); 70S disco (four on the
//! floor, octave bass, strings), funk (a 16th groove, clavinet, horns),
//! soft rock; 80S synth-pop (a LinnDrum, gated snare, FM keys, chorus),
//! the power ballad, new wave; 90S eurodance (off-beat bass, piano stabs, a
//! supersaw hook), grunge (quiet verse, loud chorus), the R&B slow jam;
//! 2000S pop-punk, the R&B of pizzicato stabs and 808s, electro-pop; 2010S
//! big-room EDM (builds and drops), tropical house, trap-pop, the folk
//! stomp; 2020S nu-disco, amapiano (the log drum), afrobeats. KENYAN: benga
//! (a running bass, interlocking guitars, the kayamba's shaker), Kenyan
//! rumba, the Luo nyatiti lyre with its ongeng'o ring. YOGA: a meditation
//! (tanpura, bansuri, bowls), a flow (hand drums, kalimba), a kirtan
//! (harmonium, tabla, manjira). ELEVATOR: bossa nova, lounge swing,
//! smooth. CORPORATE: uplifting piano and glockenspiel, tech, acoustic.
//! WORSHIP: the ballad that builds, the 6/8 hymn, the anthem. PRAISE:
//! gospel, the 12/8 shuffle, contemporary praise.

use super::*;

/// A kit's drums and its cymbals and percussion.
const KIT: u32 = L_KICK | L_SNARE | L_HAT;
const FULL: u32 = KIT | L_OHAT | L_PERC;

// ---------------------------------------------------------------- instruments

/// A clavinet: a narrow pulse, bright and quick to die.
/// A picked electric bass under a band, its tone rolled off (the
/// SoundFont's, through the patch's low-pass about 1 kHz): bright and on
/// its own, its recording reads as a chip's bass to the taggers
/// (tools/validate); a fingered one has `ska_bass_flat`.
pub(super) fn bass_pick_dark(r: &mut Rng) -> Patch {
    let p = ska_bass_pick(r);
    if p.samp == 0 {
        return p;
    }
    Patch { filter: true, cutoff: r.range(900.0, 1100.0), keytrack: 0.2, res: 0.0, env_amt: 0.0, ..p }
}

pub(super) fn clav(r: &mut Rng) -> Patch {
    let p = Patch {
        wave: Wave::Square,
        pw: r.range(0.18, 0.28),
        cutoff: r.range(2600.0, 3600.0),
        res: 0.2,
        env_amt: 1.2,
        fdec: 0.12,
        keytrack: 0.6,
        a: 0.002,
        d: 0.4,
        s: 0.12,
        r: 0.08,
        gain: 0.3,
        ..PATCH
    };
    rec(p, samples::gm("Clavinet"), 60.0)
}

/// A harpsichord: a bright plucked string.
pub(super) fn harpsichord(r: &mut Rng) -> Patch {
    let p = Patch {
        ks: r.range(1.0, 1.4),
        ks_tone: 0.85,
        cutoff: 5000.0,
        a: 0.001,
        d: 2.0,
        s: 1.0,
        r: 0.12,
        gain: 0.45,
        ..PATCH
    };
    rec(p, samples::gm("Harpsichord"), 60.0)
}

/// Guitars: a hollow-body's mellow tone, a steel-string's, a twelve-string's.
pub(super) fn jazz_guitar(r: &mut Rng) -> Patch {
    let p = Patch { ks_tone: 0.3, cutoff: 1800.0, ..nylon_synth(r) };
    rec(p, samples::gm("Jazz Guitar"), 60.0)
}

pub(super) fn steel_guitar(r: &mut Rng) -> Patch {
    let p = Patch { ks_tone: 0.7, cutoff: 4500.0, ..nylon_synth(r) };
    rec(p, samples::gm("Steel String Guitar"), 60.0)
}

pub(super) fn twelve_string(r: &mut Rng) -> Patch {
    let p = Patch { ks_tone: 0.75, cutoff: 5000.0, unison: 2, detune: 6.0, ..nylon_synth(r) };
    rec(p, samples::gm("12-String Guitar"), 60.0)
}

/// An electric guitar, clean.
pub(super) fn elec_clean(r: &mut Rng) -> Patch {
    let p = Patch { ks_tone: 0.65, cutoff: 3800.0, drive: 0.15, ..nylon_synth(r) };
    rec(p, samples::gm("Clean Guitar"), 60.0)
}

/// A surf guitar: clean, its tremolo on, deep in the spring.
pub(super) fn surf_guitar(r: &mut Rng) -> Patch {
    let q = Patch {
        ks_tone: 0.7,
        cutoff: 4000.0,
        drive: 0.2,
        trem: r.range(0.25, 0.4),
        vib_rate: r.range(5.5, 7.0),
        ..nylon_synth(r)
    };
    rec_keep(q, samples::gm("Clean Guitar"), 60.0, |x| Patch { trem: q.trem, vib_rate: q.vib_rate, ..x })
}

/// A brass section, playing together.
pub(super) fn brass_section(r: &mut Rng) -> Patch {
    rec(trumpet_synth(r), samples::gm("Brass Section"), 64.0)
}

/// Strings for stabs and runs (quick to speak), and a synthesiser's.
pub(super) fn strings_fast(r: &mut Rng) -> Patch {
    let p = Patch { a: 0.02, r: 0.25, ..pad_strings_synth(r) };
    rec(p, samples::gm("Strings Fast"), 60.0)
}

pub(super) fn synth_strings(r: &mut Rng) -> Patch {
    rec_through(pad_strings_synth(r), samples::gm("Synth Strings 1"), 60.0)
}

/// A church's pipe organ.
pub(super) fn church_organ(r: &mut Rng) -> Patch {
    let p = Patch { a: 0.06, r: 0.4, trem: 0.0, ..pad_organ_synth(r) };
    rec(p, samples::gm("Church Organ"), 60.0)
}

/// A pan flute; a bansuri (the end-blown bamboo flute's recording its
/// stand-in), sliding between its notes.
pub(super) fn pan_flute(r: &mut Rng) -> Patch {
    rec(flute_synth(r), samples::gm("Pan Flute"), 74.0)
}

pub(super) fn bansuri(r: &mut Rng) -> Patch {
    let p = Patch { noise: r.range(0.05, 0.08), vib: 0.18, glide: r.range(0.08, 0.14), ..flute_synth(r) };
    rec(p, samples::gm("Shakuhachi"), 72.0)
}

/// A tanpura's string: a buzzing saw through the bridge's vowel, ringing
/// long (the synthesiser's always: there is no recording of one).
pub(super) fn tanpura(r: &mut Rng) -> Patch {
    Patch {
        unison: 2,
        detune: r.range(3.0, 6.0),
        width: 0.6,
        cutoff: r.range(2200.0, 3000.0),
        res: 0.25,
        keytrack: 0.3,
        vowel: 1,
        a: 0.004,
        d: r.range(2.5, 3.5),
        s: 0.25,
        r: 1.6,
        gain: 0.22,
        ..PATCH
    }
}

/// A singing bowl: inharmonic partials beating slowly, ringing for seconds.
pub(super) fn singing_bowl(r: &mut Rng) -> Patch {
    Patch {
        filter: false,
        fm: r.range(0.6, 1.0),
        ratio: r.pick(&[2.76, 2.71, 2.83]),
        fm_dec: 3.0,
        a: 0.01,
        d: 7.0,
        s: 0.0,
        r: 4.0,
        trem: 0.12,
        vib_rate: r.range(3.0, 5.0),
        gain: 0.3,
        ..PATCH
    }
}

/// A harmonium: reeds and bellows.
pub(super) fn harmonium(r: &mut Rng) -> Patch {
    rec(accordion_synth(r), samples::gm("Reed Organ"), 60.0)
}

/// Amapiano's log drum: a sine bending down onto its note, quick to die,
/// a little driven (the synthesiser's).
pub(super) fn log_drum(r: &mut Rng) -> Patch {
    Patch {
        wave: Wave::Sine,
        wave2: Wave::Tri,
        mix2: 0.2,
        cutoff: 900.0,
        keytrack: 0.5,
        a: 0.001,
        d: r.range(0.3, 0.45),
        s: 0.0,
        r: 0.1,
        bend: 7.0,
        glide: 0.035,
        drive: 0.35,
        gain: 0.7,
        ..PATCH
    }
}

/// An 808's long sine bass, sliding (the synthesiser's).
pub(super) fn sub808(r: &mut Rng) -> Patch {
    Patch {
        wave: Wave::Sine,
        filter: false,
        a: 0.002,
        d: r.range(1.2, 2.0),
        s: 0.55,
        r: 0.2,
        glide: 0.06,
        drive: r.range(0.3, 0.6),
        gain: 0.8,
        ..PATCH
    }
}

/// A bright grand (a house record's piano).
pub(super) fn bright_piano(r: &mut Rng) -> Patch {
    rec(piano_synth(r), samples::gm("Bright Grand Piano"), 62.0)
}

/// A sine lead with a slow slide (G-funk's whistle).
pub(super) fn sine_lead(_: &mut Rng) -> Patch {
    let p = Patch {
        wave: Wave::Sine,
        filter: false,
        a: 0.02,
        d: 0.3,
        s: 0.9,
        r: 0.2,
        vib: 0.2,
        vib_rate: 5.0,
        glide: 0.08,
        gain: 0.35,
        ..PATCH
    };
    rec_through(p, samples::gm("Sine Wave"), 74.0)
}

/// The nyatiti: a lyre, plucked, dying quicker than a harp.
pub(super) fn nyatiti(r: &mut Rng) -> Patch {
    let p = Patch { d: 1.2, ..harp_synth(r) };
    rec(p, multi_or("harp", "pluck", "Harp"), 66.0)
}

/// The kalimba, the marimba and the vibraphone as they are; strings
/// pizzicato; a piano felted; FM keys.
pub(super) fn steel_drums(r: &mut Rng) -> Patch {
    rec(marimba_synth(r), samples::gm("Steel Drums"), 72.0)
}

// -- kits

/// A crash (kind 0) and a ride (kind 1).
fn kit_cymbals(r: &mut Rng) -> Vec<DrumPatch> {
    let mut v = ska_kit_crash(r);
    v.extend(ska_kit_ride(r).into_iter().take(1));
    v
}

/// A side stick, recorded.
fn side_stick(r: &mut Rng) -> DrumPatch {
    rec_kit_db(vec![percs(r)[0]], &[(samples::drum("Standard", 37), -1.0)])[0]
}

/// A snare and hand claps (kind 1); a snare and its side stick; brushes and a side stick.
fn kit_snare_clap(r: &mut Rng) -> Vec<DrumPatch> {
    vec![ska_kit_snare(r)[0], kit_hand_clap(r)[0]]
}

fn kit_snare_rim(r: &mut Rng) -> Vec<DrumPatch> {
    vec![ska_kit_snare(r)[0], side_stick(r)]
}

fn kit_bossa_snare(r: &mut Rng) -> Vec<DrumPatch> {
    vec![kit_brush(r)[0], side_stick(r)]
}

/// The 808's kick, clap and hats; the 909's.
fn kit_clap_rim(r: &mut Rng) -> Vec<DrumPatch> {
    vec![kit_clap_808(r)[0], side_stick(r)]
}

/// A tabla's strokes (the hand drums' recordings near them): na (the small
/// drum's ring), ge (the bass drum's, bending), tin, te, and the finger
/// cymbals (by kind: rim, the toms low to high, the shaker's place).
fn kit_tabla(r: &mut Rng) -> Vec<DrumPatch> {
    let tone = |f: f32, d: f32, g: f32| DrumPatch {
        tone: 1.0,
        f0: f * 1.15,
        f1: f,
        pdec: 0.03,
        tdec: d,
        noise: 0.15,
        nmode: 1,
        ncut: 3000.0,
        ndec: 0.02,
        gain: g,
        ..DRUM
    };
    let na = tone(r.range(520.0, 600.0), 0.35, 0.5);
    let ge = DrumPatch { f0: 140.0, pdec: 0.12, ..tone(85.0, 0.6, 0.8) };
    let tin = tone(r.range(380.0, 440.0), 0.2, 0.45);
    let te = DrumPatch { tone: 0.3, noise: 0.6, ndec: 0.04, ..tone(700.0, 0.05, 0.4) };
    let cym = DrumPatch { tone: 0.6, f0: 2600.0, f1: 2600.0, tdec: 1.2, metal: 0.3, mf: 3.1, gain: 0.3, ..DRUM };
    rec_kit_db(
        vec![na, ge, tin, te, cym],
        &[
            cc0_or("bongo", "high", -2.0, samples::drum("Standard", 60)),
            frame_drum(0, 4.0),
            cc0_or("conga", "muted", 0.0, samples::drum("Standard", 62)),
            cc0_or("bongo", "high_muted", -2.0, samples::drum("Standard", 60)),
            finger_cymbals(-4.0),
        ],
    )
}

/// Finger cymbals (the manjira), struck short and let ring.
fn kit_manjira(r: &mut Rng) -> Vec<DrumPatch> {
    let c = DrumPatch {
        tone: 0.6,
        f0: 2600.0,
        f1: 2600.0,
        tdec: 0.4,
        metal: 0.3,
        mf: r.range(2.9, 3.3),
        gain: 0.3,
        ..DRUM
    };
    rec_kit_db(vec![c, DrumPatch { tdec: 1.4, ..c }], &[finger_cymbals(-6.0), finger_cymbals(-3.0)])
}

/// The nyatiti player's ongeng'o (an iron ring on the toe, struck), a
/// drum's strokes, a bongo, a shaker.
fn kit_ongengo(r: &mut Rng) -> Vec<DrumPatch> {
    let mut v = percs(r);
    v[0] = DrumPatch { tone: 0.5, f0: 1800.0, f1: 1800.0, tdec: 0.12, metal: 0.4, mf: 2.3, gain: 0.45, ..DRUM };
    rec_kit_db(
        v,
        &[
            cc0_or("agogo", "high", -6.0, samples::drum("Standard", 67)),
            frame_drum(0, 2.0),
            frame_drum(1, 2.0),
            cc0_or("bongo", "high_muted", -2.0, samples::drum("Standard", 60)),
            shaker_rec(0, -10.0),
        ],
    )
}

/// The orchestra's percussion: the bass drum, the side drum, the
/// triangle, cymbals (the percussion's place a tambourine's).
fn kit_orch_snare(r: &mut Rng) -> Vec<DrumPatch> {
    sw_kit_snare(r)
}

// ---------------------------------------------------------------- the grooves

/// Rock's: the kick on one and the and of three, eighths on the hats; the
/// chorus's on the ride, the kick busier.
const G_ROCK: [&[Lane]; 2] = [
    &[(CH_KICK, 0, "x.....x.x......."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
    &[
        (CH_KICK, 0, "x.....x.x.x...x."),
        (CH_SNARE, 0, "....X.......X..."),
        (CH_CYM, 1, "x.x.x.x.x.x.x.x."),
        (CH_HAT, HAT_C, "x..............."),
    ],
];
/// Four on the floor, the open hat off the beat; the chorus's clap and shaker.
const G_FLOOR: [&[Lane]; 2] = [
    &[(CH_KICK, 0, "x...x...x...x..."), (CH_HAT, HAT_O, "..x...x...x...x."), (CH_HAT, HAT_C, "x...x...x...x...")],
    &[
        (CH_KICK, 0, "x...x...x...x..."),
        (CH_SNARE, 0, "....x.......x..."),
        (CH_HAT, HAT_C, "x...x...x...x..."),
        (CH_HAT, HAT_O, "..x...x...x...x."),
        (CH_PERC, SHAKER, "-x-x-x-x-x-x-x-x"),
    ],
];
/// A 12/8 shuffle: the kick on the beats, the backbeat, the hats' long-short.
const G_SHUFFLE: [&[Lane]; 2] = [
    &[
        (CH_KICK, 0, "x.....x.....x.....x....."),
        (CH_SNARE, 0, "......x...........x....."),
        (CH_HAT, HAT_C, "x...x.x...x.x...x.x...x."),
    ],
    &[
        (CH_KICK, 0, "x.....x.....x.....x....."),
        (CH_SNARE, 0, "......X.....o.....X....o"),
        (CH_CYM, 1, "x...x.x...x.x...x.x...x."),
    ],
];
/// The orchestra's percussion (for a mix; its own music has the timpani):
/// the bass drum on the one, the triangle; the side drum's light backbeat.
const G_ORCH: [&[Lane]; 2] = [
    &[(CH_KICK, 0, "x.......x......."), (CH_HAT, 0, "........x.......")],
    &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 0, "....o.......o..."), (CH_HAT, 0, "x...............")],
];
const G_ORCH3: [&[Lane]; 2] = [
    &[(CH_KICK, 0, "x..........."), (CH_HAT, 0, "....x...x...")],
    &[(CH_KICK, 0, "x..........."), (CH_SNARE, 0, "....o...o..."), (CH_HAT, 0, "x...........")],
];
/// A tabla's theka, near enough (dha dhin dhin dha): ge low, na and tin, te.
const G_TABLA: [&[Lane]; 2] = [
    &[(CH_PERC, TOM_LO, "x.......x......."), (CH_PERC, RIM, "..x.x.....x.x..."), (CH_PERC, TOM_HI, ".o...o...o...o..")],
    &[
        (CH_PERC, TOM_LO, "x.....x.x......."),
        (CH_PERC, RIM, "..x.x.x...x.x.x."),
        (CH_PERC, TOM_MID, "....x.......x..."),
        (CH_PERC, TOM_HI, ".o.o.o.o.o.o.o.o"),
        (CH_HAT, 0, "x...x...x...x..."),
    ],
];

// ---------------------------------------------------------------- CLASSICAL

/// A quiet style's master: more gain in, the leveller free to add more.
fn quiet(_: &mut Rng, s: &mut Synth) {
    (s.m.gain, s.m.lmax) = (2.6, 3.0);
}

const CL_MAJ: &[&str] = &[
    "I IV I V I ii^1 I^2+V7 I",
    "I vi IV V I IV I^2+V7 I",
    "I V7^2 I^1 IV V V7/V V V",
    "I ii7^1 V7 I vi ii V7 I",
    "I I^1 IV V I vi ii+V7 I",
];
const CL_MIN: &[&str] =
    &["i iv V7 i bVI iv^1 V7 i", "i V7^1 i iv^1 i^2 V7 i i", "i iio^1 V7 i iv i^2+V7 i", "i bVI iv V i iv V7 i"];
const CL_ORCH: [&[Lane]; 2] = G_ORCH;
const CL_KITS: &[KitRow] = &[
    (CH_KICK, sw_kit_bass_drum, 0.6, 0.0, 0.3),
    (CH_SNARE, kit_orch_snare, 0.35, 0.1, 0.3),
    (CH_HAT, sw_kit_triangle, 0.3, -0.3, 0.3),
    (CH_CYM, sw_kit_cymbals, 0.3, 0.2, 0.35),
];

/// A piano sonata's allegro: the Alberti bass, the tune a period.
const CL_SONATA: Style = Style {
    meter: "4/4 sonata",
    human: 0.012,
    bpm: (100.0, 132.0),
    modes: [5.0, 0.0, 0.0, 0.0, 0.0, 1.2, 1.2],
    maj: CL_MAJ,
    min: CL_MIN,
    cadence: ("ii^1+V7 I IV+V7 I", "iio^1+V7 i iv+V7 i"),
    form: F_CLASSICAL,
    layers: [L_KEYS, L_KEYS | L_LEAD, L_KEYS | L_LEAD, L_KEYS | L_LEAD, 0, 0, 0, L_KEYS | L_LEAD],
    grooves: CL_ORCH,
    fill: FI_NONE,
    bass: BS_NONE,
    comp: CP_ALBERTI,
    tune: TU_CLASSIC,
    voice: (74, -3, 10),
    density: (0.45, 0.55),
    sound: Sound {
        tones: &[(CH_KEYS, piano, 0.36, -0.12, 0.2, 0.0), (CH_LEAD, piano, 0.4, 0.12, 0.2, 0.0)],
        kits: CL_KITS,
        room: (0.82, 0.24),
        extra: quiet,
        ..BASE.sound
    },
    ..BASE
};

/// A string quartet's minuet: the cello on the one, the inner strings on
/// two and three, the first violin's tune, the second's in thirds.
const CL_MINUET: Style = Style {
    meter: "3/4 minuet",
    human: 0.012,
    bs: 12,
    bpm: (104.0, 126.0),
    modes: [5.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.2],
    maj: &["I V7 I IV I V V I", "I IV V7 I vi ii V7 I", "I I^1 IV V7 I IV^1 V7 I", "I V vi iii IV I ii V"],
    min: &["i V7 i iv i iv V7 i", "i bVI iv V7 i iv^1 V7 i", "i iv bVII bIII bVI iio V7 i"],
    cadence: ("IV V7 I I", "iv V7 i i"),
    form: F_CLASSICAL,
    layers: [
        L_KEYS | L_BASS,
        L_KEYS | L_BASS | L_LEAD,
        L_KEYS | L_BASS | L_LEAD | L_CTR,
        L_KEYS | L_BASS | L_LEAD,
        0,
        0,
        0,
        L_KEYS | L_BASS | L_LEAD,
    ],
    grooves: G_ORCH3,
    fill: FI_NONE,
    bass: BS_TWO,
    comp: CP_OOMPAH,
    tune: TU_CLASSIC,
    second: SV_THIRDS,
    voice: (76, -3, 9),
    density: (0.42, 0.5),
    sound: Sound {
        tones: &[
            (CH_LEAD, fiddle, 0.34, 0.25, 0.22, 0.0),
            (CH_BELL, fiddle, 0.24, -0.25, 0.22, 0.0),
            (CH_KEYS, sw_violas_spic, 0.28, -0.1, 0.22, 0.0),
            (CH_BASS, sw_cellos, 0.42, 0.1, 0.22, 0.0),
        ],
        kits: CL_KITS,
        room: (0.8, 0.22),
        extra: quiet,
        ..BASE.sound
    },
    ..BASE
};

/// A baroque trio sonata: the harpsichord's chords, the continuo walking,
/// two violins (one an oboe now and then) in thirds.
const CL_BAROQUE: Style = Style {
    meter: "4/4 baroque",
    human: 0.01,
    bpm: (84.0, 108.0),
    modes: [4.0, 0.0, 0.0, 0.0, 0.0, 1.5, 1.5],
    maj: &["I IV viio iii vi ii V I", "I V vi iii IV I IV V", "I vi ii V I IV V I", "I V^1 vi iii^1 IV I^1 ii V"],
    min: &["i iv bVII bIII bVI iio V i", "i V bVII bIII iv i iv V", "i iv V i bVI iv V7 i"],
    cadence: ("IV V I I", "iv V i i"),
    form: F_CLASSICAL,
    layers: [
        L_KEYS | L_BASS,
        L_KEYS | L_BASS | L_LEAD,
        L_KEYS | L_BASS | L_LEAD | L_CTR,
        L_KEYS | L_BASS | L_LEAD,
        0,
        0,
        0,
        L_KEYS | L_BASS | L_LEAD | L_CTR,
    ],
    grooves: G_ORCH,
    fill: FI_NONE,
    bass: BS_CONTINUO,
    comp: CP_ARP8,
    tune: TU_CLASSIC,
    second: SV_THIRDS,
    voice: (76, -3, 10),
    density: (0.55, 0.65),
    sound: Sound {
        tones: &[
            (CH_KEYS, harpsichord, 0.3, -0.2, 0.18, 0.0),
            (CH_BASS, sw_cellos, 0.4, 0.05, 0.18, 0.0),
            (CH_LEAD, fiddle, 0.32, 0.25, 0.2, 0.0),
            (CH_BELL, fiddle, 0.24, -0.3, 0.2, 0.0),
        ],
        kits: CL_KITS,
        room: (0.78, 0.2),
        extra: quiet,
        alts: &[(CH_BELL, &[("oboe", sw_oboe), ("violin", fiddle), ("flute", flute)])],
        ..BASE.sound
    },
    ..BASE
};

// ---------------------------------------------------------------- ORCHESTRA

const OR_MAJ: &[&str] = &[
    "I vi IV V I IV ii^1+V7 I",
    "I iii IV I^1 ii V7 vi IV+V7",
    "I V^1 vi I^1 IV I^2 V7 I",
    "I bVI IV V I bIII IV V",
    "I IV^1 V7/V V I vi ii7 V7",
];
const OR_MIN: &[&str] = &[
    "i bVI bIII bVII i iv V7 i",
    "i iv^1 V7 i bVI bII^1 V7 i",
    "i bIII iv V i bVI iio^1+V7 i",
    "i bVII bVI V i iv bVI V",
];
const OR_SOUND: Sound = Sound {
    tones: &[
        (CH_LEAD, sw_violins, 0.36, 0.15, 0.3, 0.0),
        (CH_PAD, sw_ensemble, 0.24, 0.0, 0.32, 0.0),
        (CH_BASS, sw_cellos, 0.38, 0.12, 0.28, 0.0),
        (CH_ARP, harp, 0.2, -0.35, 0.32, 0.0),
        (CH_BELL, sw_horns, 0.26, -0.25, 0.32, 0.0),
        (CH_KEYS, sw_brass, 0.24, 0.1, 0.32, 0.0),
        (CH_DRONE, sw_timpani, 0.4, 0.0, 0.25, 0.0),
    ],
    kits: CL_KITS,
    room: (0.86, 0.3),
    alts: &[
        (CH_LEAD, &[("violins", sw_violins), ("flute", flute), ("oboe", sw_oboe), ("horns", sw_horns)]),
        (CH_BELL, &[("horns", sw_horns), ("cellos", sw_cellos), ("clarinet", clarinet)]),
    ],
    ..BASE.sound
};

/// A romantic symphony's movement: the violins' theme over the strings,
/// the harp, the horns' counter-line, the brass's chorale and the timpani
/// in the loud parts.
const OR_SYMPHONY: Style = Style {
    meter: "4/4 symphony",
    human: 0.015,
    bpm: (72.0, 100.0),
    modes: [4.0, 0.0, 0.0, 0.0, 0.0, 2.5, 1.5],
    maj: OR_MAJ,
    min: OR_MIN,
    cadence: ("IV+V7 I IV+V7 I", "iv+V7 i iv+V7 i"),
    form: F_CLASSICAL,
    layers: [
        L_PAD | L_ARP,
        L_PAD | L_LEAD | L_BASS | L_ARP,
        L_PAD | L_LEAD | L_BASS | L_CTR | L_KEYS | L_DRONE | L_KICK | L_OHAT,
        L_PAD | L_LEAD | L_BASS | L_KEYS | L_DRONE,
        0,
        0,
        0,
        L_PAD | L_BASS | L_ARP | L_LEAD,
    ],
    grooves: [&[(CH_KICK, 0, "x...............")], &[(CH_KICK, 0, "x.......x.......")]],
    fill: FI_NONE,
    bass: BS_HOLD,
    comp: CP_HOLD,
    arp: AR_HARP,
    tune: TU_CLASSIC,
    second: SV_COUNTER,
    drone: DR_TIMPANI,
    voice: (74, -3, 9),
    density: (0.32, 0.4),
    sound: OR_SOUND,
    ..BASE
};

/// A Viennese waltz: the basses on the one, the strings and horns on two
/// and three, the violins' tune, the flutes in thirds.
const OR_WALTZ: Style = Style {
    meter: "3/4 waltz",
    human: 0.012,
    bs: 12,
    bpm: (150.0, 176.0),
    modes: [6.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
    maj: &["I I V7 V7 V7 V7 I I", "I I IV IV V7 V7 I I", "I I^1 IV ii V7 V7 I I", "I I V7 V7 I I^1 IV V7"],
    min: &["i i V7 V7 V7 V7 i i", "i i iv iv V7 V7 i i"],
    cadence: ("IV V7 I I", "iv V7 i i"),
    form: F_CLASSICAL,
    layers: [
        L_KEYS | L_BASS,
        L_KEYS | L_BASS | L_LEAD,
        L_KEYS | L_BASS | L_LEAD | L_CTR | L_PAD | L_KICK | L_HAT | L_DRONE,
        L_KEYS | L_BASS | L_LEAD | L_CTR,
        0,
        0,
        0,
        L_KEYS | L_BASS | L_LEAD,
    ],
    grooves: G_ORCH3,
    fill: FI_NONE,
    bass: BS_TWO,
    comp: CP_OOMPAH,
    tune: TU_CLASSIC,
    second: SV_THIRDS,
    drone: DR_TIMPANI,
    voice: (76, -3, 10),
    density: (0.36, 0.42),
    sound: Sound {
        tones: &[
            (CH_LEAD, sw_violins, 0.36, 0.15, 0.3, 0.0),
            (CH_BELL, flute, 0.24, -0.25, 0.3, 0.0),
            (CH_KEYS, sw_violas_spic, 0.28, -0.1, 0.3, 0.0),
            (CH_BASS, sw_contrabass, 0.4, 0.1, 0.28, 0.0),
            (CH_PAD, sw_horns, 0.18, -0.2, 0.3, 0.0),
            (CH_DRONE, sw_timpani, 0.35, 0.0, 0.25, 0.0),
        ],
        kits: CL_KITS,
        room: (0.84, 0.28),
        alts: &[(CH_BELL, &[("flute", flute), ("clarinet", clarinet), ("oboe", sw_oboe)])],
        ..BASE.sound
    },
    ..BASE
};

/// A strings' adagio: slow, the tune on the violins or the cellos, the
/// violas' line below it, the harp in the fuller parts.
const OR_ADAGIO: Style = Style {
    meter: "4/4 adagio",
    human: 0.015,
    bpm: (54.0, 68.0),
    modes: [3.0, 0.0, 0.0, 0.0, 0.0, 3.0, 1.0],
    maj: OR_MAJ,
    min: OR_MIN,
    cadence: ("IV V7 I I", "iv V7 i i"),
    form: F_CLASSICAL,
    layers: [
        L_PAD,
        L_PAD | L_LEAD | L_BASS,
        L_PAD | L_LEAD | L_BASS | L_CTR | L_ARP,
        L_PAD | L_LEAD | L_BASS,
        0,
        0,
        0,
        L_PAD | L_BASS | L_LEAD,
    ],
    grooves: G_ORCH,
    fill: FI_NONE,
    bass: BS_HOLD,
    comp: CP_NONE,
    arp: AR_HARP,
    tune: TU_LONG,
    second: SV_COUNTER,
    voice: (72, -3, 8),
    density: (0.3, 0.36),
    end_hit: false,
    sound: Sound {
        alts: &[(CH_LEAD, &[("violins", sw_violins), ("cellos", sw_cellos), ("violas", sw_violas)])],
        ..OR_SOUND
    },
    ..BASE
};

// ---------------------------------------------------------------- ROCK

const RK_KITS: &[KitRow] = &[
    (CH_KICK, ska_kit_punk_kick, 0.55, 0.0, 0.05),
    (CH_SNARE, ska_kit_punk_snare, 1.0, 0.05, 0.16),
    (CH_HAT, ska_kit_hats, 0.42, 0.25, 0.05),
    (CH_PERC, ska_kit_toms, 0.45, -0.15, 0.12),
    (CH_CYM, kit_cymbals, 0.32, -0.3, 0.15),
];
const RK_SOUND: Sound = Sound {
    tones: &[
        (CH_KEYS, ska_guitar_dist, 0.3, -0.4, 0.1, 0.0),
        (CH_LEAD, ska_guitar_fuzz, 0.32, 0.3, 0.16, 0.05),
        (CH_BASS, bass_pick_dark, 0.45, 0.0, 0.03, 0.0),
        (CH_PAD, ska_organ_rec, 0.16, 0.2, 0.15, 0.0),
    ],
    kits: RK_KITS,
    room: (0.76, 0.16),
    alts: &[(CH_LEAD, &[("overdriven guitar", ska_guitar_fuzz), ("distorted guitar", ska_guitar_dist)])],
    ..BASE.sound
};
const RK_LAYERS: [u32; 8] = [
    L_KEYS | L_KICK | L_HAT,
    FULL | L_BASS | L_KEYS | L_LEAD,
    FULL | L_BASS | L_KEYS | L_LEAD | L_PAD,
    L_KICK | L_HAT | L_BASS | L_PAD | L_KEYS,
    FULL | L_BASS | L_KEYS | L_LEAD,
    0,
    0,
    FULL | L_BASS | L_KEYS,
];

/// Hard rock: the riff on the lead guitar, power chords under it, a solo.
const RK_HARD: Style = Style {
    meter: "4/4 hard rock",
    bpm: (108.0, 140.0),
    modes: [1.0, 1.5, 0.0, 0.0, 1.5, 3.0, 0.0],
    maj: &["I I bVII IV I I bVII IV", "I bVII IV I I bVII IV IV", "I IV I V I IV bVII V"],
    min: &["i i bVI bVII i i bVI bVII", "i bIII bVII IV i bIII bVII IV", "i iv bVI bVII i iv bVII i"],
    cadence: ("bVII IV I I", "bVI bVII i i"),
    layers: RK_LAYERS,
    grooves: G_ROCK,
    bass: BS_ROOT8,
    comp: CP_POWER,
    tune: TU_RIFF,
    human: 0.008,
    sound: RK_SOUND,
    double: &[(CH_KEYS, 0.9)],
    balance: &[
        (CH_KICK, -3.4),
        (CH_SNARE, -5.4),
        (CH_HAT, 3.4),
        (CH_PERC, 3.8),
        (CH_BASS, 2.1),
        (CH_PAD, 6.1),
        (CH_LEAD, 8.0),
        (CH_KEYS, 1.3),
    ],
    tilt: (0.0, -5.0),
    ..BASE
};

/// An arena anthem: the tune on the lead guitar, eighths driving.
const RK_ANTHEM: Style = Style {
    meter: "4/4 anthem",
    human: 0.009,
    bpm: (118.0, 136.0),
    modes: [5.0, 0.0, 0.0, 0.0, 1.5, 2.0, 0.0],
    maj: &["I V vi IV I V vi IV", "I IV vi V I IV V V", "vi IV I V vi IV I V", "I bVII IV I I bVII IV V"],
    min: &["i bVI bIII bVII i bVI bIII bVII", "i bVII bVI bVII i bVII bVI V"],
    cadence: ("IV V I I", "bVI bVII i i"),
    layers: RK_LAYERS,
    grooves: G_ROCK,
    bass: BS_ROOT8,
    comp: CP_POWER,
    tune: TU_SONG,
    density: (0.38, 0.48),
    sound: RK_SOUND,
    double: &[(CH_KEYS, 0.9)],
    balance: &[
        (CH_KICK, -3.4),
        (CH_SNARE, -5.4),
        (CH_HAT, 3.4),
        (CH_PERC, 3.8),
        (CH_BASS, 2.1),
        (CH_PAD, 6.2),
        (CH_LEAD, 7.1),
        (CH_KEYS, 1.2),
    ],
    tilt: (0.0, -5.0),
    ..BASE
};

/// A blues shuffle in 12/8: twelve-bar choruses, the boogie bass, the
/// organ's stabs, the riff and the solos.
const RK_BLUES: Style = Style {
    meter: "12/8 blues",
    human: 0.01,
    bs: 24,
    beat: 6,
    bpm: (78.0, 96.0),
    modes: [2.0, 0.5, 0.0, 0.0, 4.0, 1.5, 0.0],
    maj: &["I7 IV7 I7 I7 IV7 IV7 I7 I7 V7 IV7 I7 V7", "I7 I7 I7 I7 IV7 IV7 I7 I7 V7 IV7 I7 I7"],
    min: &["i7 iv7 i7 i7 iv7 iv7 i7 i7 bVI7 V7 i7 V7"],
    cadence: ("IV7 V7 I7 I7", "iv7 V7 i7 i7"),
    form: F_BLUES,
    layers: [
        L_KEYS | L_BASS | L_KICK | L_HAT,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD,
        L_KICK | L_HAT | L_BASS | L_PAD,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS,
    ],
    grooves: G_SHUFFLE,
    bass: BS_BOOGIE,
    comp: CP_STABS,
    tune: TU_RIFF,
    sound: Sound {
        tones: &[
            (CH_KEYS, ska_hammond, 0.24, -0.3, 0.15, 0.0),
            (CH_LEAD, ska_guitar_crunch, 0.32, 0.3, 0.16, 0.0),
            (CH_BASS, ska_bass_flat, 0.45, 0.0, 0.03, 0.0),
            (CH_PAD, ska_organ_rec, 0.14, 0.2, 0.15, 0.0),
        ],
        alts: &[(CH_LEAD, &[("crunchy guitar", ska_guitar_crunch), ("harmonica", harmonica), ("tenor sax", sax)])],
        ..RK_SOUND
    },
    balance: &[
        (CH_KICK, -1.9),
        (CH_SNARE, -5.3),
        (CH_HAT, 4.1),
        (CH_PERC, 2.5),
        (CH_PAD, 8.0),
        (CH_LEAD, 5.2),
        (CH_KEYS, 6.8),
    ],
    ..BASE
};

// ---------------------------------------------------------------- 60S

/// The sixties' records: a narrower stereo, the top dull, a little hiss.
fn sixties(_: &mut Rng, s: &mut Synth) {
    for c in s.ch.iter_mut() {
        c.width = 0.6;
    }
    s.m.set_lp(10000.0);
    s.m.set_tilt(1.0, -2.0);
    s.fx.hiss = 0.0015;
}

const SX_KITS: &[KitRow] = &[
    (CH_KICK, kit_kick_soft, 0.75, 0.0, 0.08),
    (CH_SNARE, ska_kit_snare, 0.9, 0.05, 0.2),
    (CH_HAT, ska_kit_hats, 0.42, 0.25, 0.08),
    (CH_PERC, ska_kit_toms, 0.5, -0.3, 0.15),
    (CH_CYM, kit_cymbals, 0.28, -0.2, 0.15),
];

/// Soul in the Motown way: the snare on every beat in the chorus, the
/// tambourine, the bass's running figures, piano, horns, strings.
const SX_SOUL: Style = Style {
    meter: "4/4 soul",
    human: 0.009,
    bpm: (116.0, 132.0),
    modes: [6.0, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0],
    maj: &["I vi IV V I vi IV V", "I IV I V I IV V V", "I iii IV V I iii IV V", "I I7 IV iv I V7 I I"],
    min: &["i iv bVII bIII i iv V7 i"],
    layers: [
        L_KEYS | L_BASS | L_KICK | L_HAT,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
        KIT | L_BASS | L_KEYS | L_CTR,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD,
        0,
        0,
        FULL | L_BASS | L_KEYS | L_LEAD,
    ],
    grooves: [
        &[
            (CH_KICK, 0, "x.......x......."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-."),
            (CH_PERC, SHAKER, "....x.......x..."),
        ],
        &[
            (CH_KICK, 0, "x.......x.x....."),
            (CH_SNARE, 0, "x...x...x...x..."),
            (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-."),
            (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x."),
        ],
    ],
    bass: BS_MOTOWN,
    comp: CP_BLOCK4,
    second: SV_STABS,
    density: (0.4, 0.5),
    sound: Sound {
        tones: &[
            (CH_KEYS, piano, 0.3, -0.3, 0.15, 0.0),
            (CH_BASS, ska_bass_flat, 0.48, 0.0, 0.03, 0.0),
            (CH_LEAD, sax, 0.32, 0.2, 0.2, 0.0),
            (CH_BELL, brass_section, 0.22, -0.2, 0.18, 0.0),
            (CH_PAD, pad_strings, 0.16, 0.3, 0.25, 0.0),
        ],
        kits: SX_KITS,
        room: (0.8, 0.22),
        extra: sixties,
        alts: &[(CH_LEAD, &[("tenor sax", sax), ("vibraphone", vibes), ("organ", ska_hammond), ("flute", flute)])],
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -4.7),
        (CH_SNARE, -2.5),
        (CH_PERC, 4.2),
        (CH_BASS, 1.9),
        (CH_PAD, 11.4),
        (CH_LEAD, 7.1),
        (CH_KEYS, 6.5),
    ],
    ..BASE
};

/// Surf: the tremolo guitar's tune deep in the spring, the floor toms
/// rumbling, a Farfisa, the bass in eighths.
const SX_SURF: Style = Style {
    meter: "4/4 surf",
    human: 0.008,
    bpm: (150.0, 172.0),
    modes: [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.5],
    maj: &["I I IV IV V V IV IV", "I bVII bVI V I bVII bVI V", "I I bII bII I I bII I"],
    min: &["i i bVII bVII bVI bVI V V", "i iv V i i iv V V", "i bVI bVII i i bVI V V"],
    cadence: ("bVI V I I", "bVI V i i"),
    layers: [
        L_LEAD | L_KICK | L_PERC,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD,
        L_KICK | L_PERC | L_BASS | L_PAD,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_LEAD,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x.......x......."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_PERC, TOM_LO, "x.x.x.x.x.x.x.x."),
            (CH_PERC, TOM_HI, "...x.......x...."),
        ],
    ],
    bass: BS_ROOT8,
    comp: CP_STRUM,
    density: (0.5, 0.6),
    sound: Sound {
        tones: &[
            (CH_LEAD, surf_guitar, 0.34, 0.2, 0.45, 0.0),
            (CH_KEYS, elec_clean, 0.22, -0.35, 0.3, 0.0),
            (CH_BASS, bass_pick_dark, 0.45, 0.0, 0.05, 0.0),
            (CH_PAD, ska_farfisa, 0.16, -0.2, 0.25, 0.0),
        ],
        kits: SX_KITS,
        room: (0.86, 0.3),
        extra: sixties,
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.7)],
    balance: &[
        (CH_KICK, -5.3),
        (CH_SNARE, -2.0),
        (CH_PERC, -6.6),
        (CH_BASS, 3.7),
        (CH_PAD, 11.8),
        (CH_LEAD, 8.2),
        (CH_KEYS, 4.7),
    ],
    ..BASE
};

/// Beat music: the twelve-string's jangle, the bass in eighths, the
/// harmonica's tune with a second voice in thirds.
const SX_BEAT: Style = Style {
    meter: "4/4 beat",
    human: 0.009,
    bpm: (126.0, 146.0),
    modes: [6.0, 0.5, 0.0, 0.0, 1.5, 0.5, 0.0],
    maj: &["I IV V IV I IV V V", "I vi IV V I vi ii V", "I bVII IV I I V IV I", "I iii IV V I iii ii V"],
    min: &["i bVII bVI bVII i bVII bVI V"],
    layers: [
        L_KEYS | L_KICK | L_HAT,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_CTR,
        KIT | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x.......x.x....."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_CYM, 1, "x.x.x.x.x.x.x.x."),
            (CH_PERC, SHAKER, "....x.......x..."),
        ],
    ],
    bass: BS_ROOT8,
    comp: CP_STRUM,
    second: SV_THIRDS,
    sound: Sound {
        tones: &[
            (CH_KEYS, twelve_string, 0.28, -0.3, 0.15, 0.0),
            (CH_BASS, bass_upright, 0.45, 0.0, 0.03, 0.0),
            (CH_LEAD, harmonica, 0.3, 0.2, 0.2, 0.0),
            (CH_BELL, elec_clean, 0.22, -0.2, 0.2, 0.0),
        ],
        kits: SX_KITS,
        extra: sixties,
        alts: &[(CH_LEAD, &[("harmonica", harmonica), ("clean guitar", elec_clean), ("organ", ska_farfisa)])],
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.8)],
    balance: &[(CH_KICK, -5.2), (CH_PERC, 5.6), (CH_BASS, 3.4), (CH_LEAD, 10.0), (CH_KEYS, 4.1)],
    ..BASE
};

// ---------------------------------------------------------------- 70S

const SV_KITS: &[KitRow] = &[
    (CH_KICK, kit_kick_soft, 0.85, 0.0, 0.03),
    (CH_SNARE, kit_snare_clap, 0.75, 0.05, 0.14),
    (CH_HAT, ska_kit_hats, 0.45, 0.3, 0.05),
    (CH_PERC, kit_perc_hand, 0.4, -0.3, 0.1),
    (CH_CYM, kit_cymbals, 0.25, -0.25, 0.15),
];

/// Disco: four on the floor, the open hat off the beat, the octave bass,
/// the funk guitar's 16ths, strings and horns.
const SV_DISCO: Style = Style {
    meter: "4/4 disco",
    human: 0.006,
    bpm: (116.0, 126.0),
    modes: [2.0, 3.0, 0.0, 0.0, 1.0, 3.0, 0.0],
    maj: &[
        "IVmaj7 iii7 ii7 Imaj7 IVmaj7 iii7 ii7 V7sus4",
        "Imaj7 vi7 ii7 V7 Imaj7 vi7 ii7 V7",
        "ii7 V7 Imaj7 vi7 ii7 V7 Imaj7 Imaj7",
    ],
    min: &[
        "i7 i7 iv7 iv7 i7 i7 iv7 bVII7",
        "i7 IV7 i7 IV7 bVImaj7 bVII7 i7 i7",
        "i9 bVII i9 bVII bVImaj7 bVII i9 V7sus4",
    ],
    cadence: ("ii7 V7 Imaj7 Imaj7", "iv7 bVII7 i7 i7"),
    layers: [
        L_KICK | L_HAT | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
        L_KICK | L_HAT | L_BASS | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS,
    ],
    grooves: [
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_HAT, HAT_C, "x...x...x...x..."),
            (CH_HAT, HAT_O, "..x...x...x...x."),
        ],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_C, "x...x...x...x..."),
            (CH_HAT, HAT_O, "..x...x...x...x."),
            (CH_PERC, SHAKER, "-x-x-x-x-x-x-x-x"),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_DISCO,
    comp: CP_SCRATCH,
    second: SV_STABS,
    density: (0.4, 0.5),
    sound: Sound {
        tones: &[
            (CH_BASS, ska_bass_flat, 0.48, 0.0, 0.03, 0.0),
            (CH_KEYS, ska_guitar_clean, 0.22, -0.35, 0.1, 0.0),
            (CH_PAD, pad_strings, 0.2, 0.2, 0.25, 0.0),
            (CH_BELL, brass_section, 0.22, 0.3, 0.15, 0.0),
            (CH_LEAD, strings_fast, 0.3, 0.1, 0.22, 0.0),
        ],
        kits: SV_KITS,
        room: (0.8, 0.22),
        alts: &[(CH_LEAD, &[("strings", strings_fast), ("flute", flute), ("tenor sax", sax), ("Rhodes", ep_rhodes)])],
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.7)],
    balance: &[
        (CH_KICK, -5.6),
        (CH_SNARE, 1.5),
        (CH_HAT, -0.6),
        (CH_BASS, 2.8),
        (CH_PAD, 6.5),
        (CH_LEAD, 6.7),
        (CH_KEYS, 9.1),
    ],
    ..BASE
};

/// Funk: the one, ghosted snares, 16th hats, the bass's syncopations, the
/// guitar's scratch, horns and a solo.
const SV_FUNK: Style = Style {
    meter: "4/4 funk",
    human: 0.007,
    bpm: (96.0, 110.0),
    swing: (0.0, 0.08),
    modes: [0.5, 4.0, 0.0, 0.0, 3.0, 1.0, 0.0],
    maj: &["I7 I7 I7 I7 IV7 IV7 I7 I7", "I9 I9 IV9 I9 I9 IV9 bVII9 I9", "I7 bVII7 I7 bVII7 IV7 IV7 I7 V7"],
    min: &["i7 i7 IV7 IV7 i7 i7 IV7 IV7", "i7 i7 i7 i7 iv7 iv7 i7 bVII7", "i9 i9 i9 bVII9 i9 i9 IV9 IV9"],
    cadence: ("IV7 IV7 I7 I7", "IV7 IV7 i7 i7"),
    form: F_VAMP,
    layers: [
        L_KICK | L_HAT | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_CTR,
        FULL | L_BASS | L_KEYS | L_LEAD | L_CTR | L_PAD,
        L_KICK | L_HAT | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.....x...x..x.."), (CH_SNARE, 0, "....x..o.o..x..o"), (CH_HAT, HAT_C, "xoxoxoxoxoxoxoxo")],
        &[
            (CH_KICK, 0, "x.x...x...x..x.x"),
            (CH_SNARE, 0, "....x..o.o..x..o"),
            (CH_HAT, HAT_C, "xoxoxoxoxoxoxo.o"),
            (CH_HAT, HAT_O, "..............x."),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_FUNK,
    comp: CP_SCRATCH,
    second: SV_STABS,
    density: (0.45, 0.5),
    sound: Sound {
        tones: &[
            (CH_BASS, ska_bass_flat, 0.5, 0.0, 0.03, 0.0),
            (CH_KEYS, clav, 0.24, -0.3, 0.08, 0.0),
            (CH_BELL, brass_section, 0.24, 0.3, 0.12, 0.0),
            (CH_LEAD, sax, 0.32, 0.15, 0.15, 0.0),
            (CH_PAD, ep_rhodes, 0.16, -0.1, 0.15, 0.0),
        ],
        kits: SV_KITS,
        room: (0.74, 0.16),
        alts: &[
            (CH_KEYS, &[("clavinet", clav), ("funk guitar", ska_guitar_clean)]),
            (CH_LEAD, &[("tenor sax", sax), ("trumpet", trumpet), ("alto sax", ska_alto)]),
        ],
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -6.3),
        (CH_SNARE, 0.8),
        (CH_BASS, 4.3),
        (CH_PAD, 12.0),
        (CH_LEAD, 7.2),
        (CH_BELL, 6.2),
        (CH_KEYS, 8.2),
    ],
    ..BASE
};

/// Soft rock: an electric piano's broken chords, the bass in quarters, a
/// light kit, strings, the tune on the sax or the flute.
const SV_SOFT: Style = Style {
    meter: "4/4 soft rock",
    human: 0.009,
    bpm: (82.0, 98.0),
    modes: [6.0, 0.5, 0.0, 0.5, 1.0, 1.0, 0.0],
    maj: &[
        "Imaj7 IVmaj7 Imaj7 IVmaj7 vi7 ii7 IVmaj7 V7sus4",
        "I V^1 vi I^1 IV I^1 ii7 V7",
        "IVmaj7 V^1 iii7 vi7 ii7 V7 Imaj7 Imaj7",
    ],
    min: &["i7 bVImaj7 bIIImaj7 bVII i7 iv7 bVII bIIImaj7"],
    layers: [
        L_KEYS | L_PAD,
        KIT | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
        KIT | L_BASS | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        KIT | L_BASS | L_KEYS | L_PAD,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-")],
    ],
    bass: BS_ROOT4,
    comp: CP_ARP8,
    second: SV_GUIDE,
    sound: Sound {
        tones: &[
            (CH_KEYS, ep_rhodes, 0.3, -0.25, 0.18, 0.0),
            (CH_BASS, bass_upright, 0.44, 0.0, 0.03, 0.0),
            (CH_LEAD, sax, 0.3, 0.2, 0.22, 0.0),
            (CH_PAD, pad_strings, 0.16, 0.2, 0.28, 0.0),
            (CH_BELL, pad_strings, 0.14, -0.3, 0.28, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.75, 0.0, 0.05),
            (CH_SNARE, kit_snare_rim, 0.7, 0.05, 0.15),
            (CH_HAT, ska_kit_hats, 0.5, 0.25, 0.05),
            (CH_PERC, ska_kit_toms, 0.45, -0.2, 0.12),
            (CH_CYM, kit_cymbals, 0.22, -0.25, 0.15),
        ],
        alts: &[
            (CH_LEAD, &[("tenor sax", sax), ("flute", flute), ("clean guitar", elec_clean)]),
            (CH_KEYS, &[("Rhodes", ep_rhodes), ("Wurlitzer", ep_wurli), ("steel-string guitar", steel_guitar)]),
        ],
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -5.6),
        (CH_SNARE, 1.7),
        (CH_HAT, -4.0),
        (CH_PERC, 6.8),
        (CH_BASS, 3.5),
        (CH_PAD, 4.7),
        (CH_LEAD, 3.4),
        (CH_KEYS, 4.3),
    ],
    ..BASE
};

// ---------------------------------------------------------------- 80S

/// The eighties' snare in a gated room; a chorus on the synthesisers.
fn eighties(r: &mut Rng, s: &mut Synth) {
    s.ch[CH_SNARE].rev2 = 0.6;
    s.r2mode = R2_GATE;
    s.rev2.fb = 0.9;
    s.rev2.damp = 0.15;
    s.r2gain = 0.6;
    s.gate_hold = (r.range(0.2, 0.3) * SR) as u32;
    for (c, k) in [(CH_PAD, 0.8), (CH_KEYS, 0.5), (CH_ARP, 0.5), (CH_LEAD, 0.35)] {
        s.ch[c].chorus = k;
    }
}

const EI_KITS: &[KitRow] = &[
    (CH_KICK, kit_kick_linn, 0.62, 0.0, 0.0),
    (CH_SNARE, ska_kit_snare, 0.42, 0.0, 0.1),
    (CH_HAT, ska_kit_hats, 0.42, 0.2, 0.05),
    (CH_PERC, kit_perc_metal, 0.35, -0.2, 0.1),
    (CH_CYM, kit_cymbals, 0.22, -0.25, 0.12),
];
const EI_LAYERS: [u32; 8] = [
    L_PAD | L_ARP | L_KICK | L_HAT,
    KIT | L_BASS | L_KEYS | L_LEAD,
    FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_ARP | L_CTR,
    KIT | L_BASS | L_PAD | L_ARP,
    FULL | L_BASS | L_KEYS | L_PAD | L_LEAD,
    0,
    0,
    KIT | L_BASS | L_PAD | L_ARP,
];
const EI_SOUND: Sound = Sound {
    tones: &[
        (CH_BASS, bass_moog, 0.42, 0.0, 0.03, 0.0),
        (CH_KEYS, glass_fm, 0.24, -0.3, 0.2, 0.1),
        (CH_PAD, hoover, 0.18, 0.0, 0.3, 0.0),
        (CH_ARP, arp_pluck, 0.2, 0.3, 0.2, 0.2),
        (CH_LEAD, lead_saw, 0.3, 0.1, 0.25, 0.15),
        (CH_BELL, bell_fm, 0.14, -0.3, 0.3, 0.15),
    ],
    kits: EI_KITS,
    room: (0.84, 0.26),
    echo: (3.0, 0.35),
    extra: eighties,
    alts: &[
        (CH_LEAD, &[("saw lead", lead_saw), ("alto sax", ska_alto), ("sync lead", lead_sync)]),
        (CH_PAD, &[("polysynth", hoover), ("warm pad", pad_warm), ("synth strings", synth_strings)]),
    ],
    ..BASE.sound
};

/// Synth-pop: a LinnDrum, the snare gated, the sequenced bass in eighths,
/// FM keys, a polysynth with its chorus, the arpeggio, a bright lead.
const EI_SYNTHPOP: Style = Style {
    meter: "4/4 synth-pop",
    bpm: (112.0, 126.0),
    modes: [3.0, 1.0, 0.0, 0.0, 0.5, 3.0, 0.0],
    maj: &["vi IV I V vi IV I V", "I V vi IV I V IV IV", "IV V vi vi IV V I I"],
    min: &["i bVI bIII bVII i bVI bIII bVII", "i bVII bVI bVII i bVII bVI V"],
    cadence: ("IV V I I", "bVI bVII i i"),
    layers: EI_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 0, "....X.......X..."),
            (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-"),
            (CH_PERC, 0, "..........x....."),
        ],
    ],
    bass: BS_ROOT8,
    comp: CP_STABS,
    arp: AR_8,
    second: SV_OCTAVE,
    sweep: true,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_pick_dark, 0.36, 0.0, 0.03, 0.0),
            (CH_KEYS, glass_fm, 0.24, -0.3, 0.2, 0.1),
            (CH_PAD, hoover, 0.18, 0.0, 0.3, 0.0),
            (CH_ARP, arp_pluck, 0.18, 0.3, 0.2, 0.2),
            (CH_LEAD, lead_saw, 0.3, 0.1, 0.25, 0.15),
            (CH_BELL, bell_fm, 0.14, -0.3, 0.3, 0.15),
        ],
        ..EI_SOUND
    },
    balance: &[
        (CH_KICK, -5.9),
        (CH_SNARE, 1.7),
        (CH_HAT, -3.3),
        (CH_PERC, 1.0),
        (CH_BASS, 4.4),
        (CH_PAD, 5.3),
        (CH_ARP, 4.8),
        (CH_LEAD, 5.9),
        (CH_KEYS, 8.6),
    ],
    ..BASE
};

/// The power ballad: slow, a piano's broken chords, the gated snare, the
/// big chorus, a sax's solo.
const EI_BALLAD: Style = Style {
    meter: "4/4 power ballad",
    human: 0.008,
    bpm: (64.0, 78.0),
    modes: [5.0, 0.0, 0.0, 0.0, 0.5, 2.0, 0.0],
    maj: &["I V^1 vi I^1 IV I^1 ii V", "I iii vi IV I iii IV V", "vi IV I V vi IV V V", "I V vi iii IV I IV V"],
    min: &["i bVI bIII bVII i bVI iv V"],
    layers: [
        L_KEYS | L_PAD,
        KIT | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
        KIT | L_BASS | L_PAD | L_KEYS,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD,
        0,
        0,
        L_BASS | L_KEYS | L_PAD,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[(CH_KICK, 0, "x.....x.x......."), (CH_SNARE, 0, "....X.......X..."), (CH_CYM, 1, "x.x.x.x.x.x.x.x.")],
    ],
    bass: BS_ROOT4,
    comp: CP_ARP8,
    second: SV_GUIDE,
    density: (0.34, 0.42),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_KEYS, piano, 0.3, -0.2, 0.25, 0.0),
            (CH_BASS, bass_fm, 0.42, 0.0, 0.05, 0.0),
            (CH_LEAD, ska_guitar_fuzz, 0.3, 0.2, 0.28, 0.12),
            (CH_PAD, pad_warm, 0.18, 0.0, 0.35, 0.0),
            (CH_BELL, synth_strings, 0.14, -0.3, 0.3, 0.0),
        ],
        alts: &[(CH_LEAD, &[("lead guitar", ska_guitar_fuzz), ("alto sax", ska_alto), ("FM piano", glass_fm)])],
        ..EI_SOUND
    },
    balance: &[
        (CH_KICK, -4.2),
        (CH_SNARE, 1.6),
        (CH_HAT, 3.0),
        (CH_PERC, 1.7),
        (CH_BASS, 0.8),
        (CH_PAD, -1.3),
        (CH_LEAD, 6.2),
        (CH_BELL, 6.9),
        (CH_KEYS, 6.4),
    ],
    ..BASE
};

/// New wave: fast, the bass in eighths, a clean guitar's broken chords,
/// synthesiser stabs, a hook.
const EI_NEWWAVE: Style = Style {
    meter: "4/4 new wave",
    bpm: (138.0, 156.0),
    modes: [3.0, 1.0, 0.0, 0.0, 1.0, 3.0, 0.0],
    maj: &["I bVII IV I I bVII IV IV", "I V vi IV I V IV V", "I iii IV IV I iii IV V"],
    min: &["i bVII bVI bVII i bVII bVI bVII", "i iv bVI bVII i iv bVII bVII"],
    layers: EI_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 0, "....X.......X..."), (CH_HAT, HAT_C, "xxxxxxxxxxxxxxxx")],
    ],
    bass: BS_ROOT8,
    comp: CP_ARP8,
    arp: AR_8,
    tune: TU_HOOK,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_pick_dark, 0.45, 0.0, 0.03, 0.0),
            (CH_KEYS, elec_clean, 0.24, -0.3, 0.2, 0.15),
            (CH_PAD, hoover, 0.16, 0.0, 0.25, 0.0),
            (CH_ARP, stab_saw, 0.18, 0.3, 0.2, 0.1),
            (CH_LEAD, lead_sync, 0.28, 0.1, 0.2, 0.1),
        ],
        ..EI_SOUND
    },
    double: &[(CH_KEYS, 0.8)],
    balance: &[
        (CH_KICK, -5.9),
        (CH_SNARE, 2.1),
        (CH_HAT, -3.2),
        (CH_PERC, 2.4),
        (CH_BASS, 3.7),
        (CH_PAD, 5.9),
        (CH_ARP, 6.9),
        (CH_LEAD, 8.0),
        (CH_KEYS, 9.9),
    ],
    ..BASE
};

// ---------------------------------------------------------------- 90S

/// Eurodance: four on the floor, the off-beat bass, the piano's stabs in
/// the drop, a supersaw hook, builds.
const NI_EURO: Style = Style {
    meter: "4/4 eurodance",
    bpm: (128.0, 138.0),
    modes: [0.5, 0.5, 0.0, 0.0, 0.0, 4.0, 0.5],
    maj: &["vi IV I V vi IV I V", "IV V vi vi IV V iii vi"],
    min: &["i bVI bIII bVII i bVI bIII bVII", "i bVI bVII i i bVI bVII V", "bVI bVII i i bVI bVII bIII bIII"],
    cadence: ("bVI bVII i i", "bVI bVII i i"),
    form: F_DANCE,
    layers: [
        L_KICK | L_HAT | L_BASS,
        FULL | L_BASS | L_PAD | L_LEAD,
        0,
        L_PAD | L_ARP | L_LEAD,
        0,
        L_SNARE | L_HAT | L_PAD | L_ARP,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD,
        L_KICK | L_HAT | L_BASS | L_KEYS,
    ],
    grooves: G_FLOOR,
    fill: FI_SNARE,
    bass: BS_OFFBEAT,
    comp: CP_OFFBEAT,
    arp: AR_16,
    tune: TU_HOOK,
    density: (0.5, 0.55),
    sweep: true,
    pump: 0.35,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_seq, 0.45, 0.0, 0.0, 0.0),
            (CH_KEYS, bright_piano, 0.26, -0.2, 0.2, 0.0),
            (CH_PAD, pad_warm, 0.18, 0.0, 0.3, 0.0),
            (CH_ARP, arp_pluck, 0.18, 0.3, 0.2, 0.2),
            (CH_LEAD, lead_saw, 0.3, 0.1, 0.25, 0.15),
        ],
        kits: &[
            (CH_KICK, kit_kick_909, 0.85, 0.0, 0.0),
            (CH_SNARE, kit_clap_rim, 0.55, 0.0, 0.12),
            (CH_HAT, kit_hats_909, 0.6, 0.2, 0.05),
            (CH_PERC, kit_shaker, 0.4, -0.25, 0.05),
            (CH_CYM, kit_cymbals, 0.22, -0.2, 0.12),
        ],
        room: (0.82, 0.24),
        echo: (3.0, 0.3),
        alts: &[(CH_LEAD, &[("saw lead", lead_saw), ("polysynth", hoover)])],
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -8.5),
        (CH_SNARE, 9.5),
        (CH_HAT, -2.7),
        (CH_BASS, 10.3),
        (CH_PAD, 6.5),
        (CH_ARP, 9.4),
        (CH_LEAD, 12.0),
        (CH_KEYS, 11.4),
    ],
    chorus: &[(CH_LEAD, 0.5), (CH_PAD, 0.6)],
    ..BASE
};

/// Grunge: the verse quiet (muted power chords), the chorus loud, a
/// distorted tune.
const NI_GRUNGE: Style = Style {
    meter: "4/4 grunge",
    human: 0.009,
    bpm: (96.0, 120.0),
    modes: [1.0, 1.0, 0.5, 0.0, 1.5, 3.0, 0.0],
    maj: &["I bIII IV I I bIII IV IV", "I bVI bVII I I bVI bVII bVII", "I IV bIII bVI I IV bIII bVI"],
    min: &["i bIII iv bVI i bIII iv bVI", "i bVII bVI V i bVII bVI V", "i bVI bIII bVII i bVI bIII bVII"],
    cadence: ("bVI bVII I I", "bVI bVII i i"),
    layers: [
        L_KEYS | L_BASS,
        L_KICK | L_HAT | L_SNARE | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        L_KICK | L_HAT | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[(CH_KICK, 0, "x.....x.x......."), (CH_SNARE, 0, "....X.......X..."), (CH_CYM, 1, "x.x.x.x.x.x.x.x.")],
    ],
    bass: BS_ROOT8,
    comp: CP_POWER,
    tune: TU_SONG,
    density: (0.35, 0.45),
    sound: Sound {
        tones: &[
            (CH_KEYS, ska_guitar_dist, 0.3, -0.4, 0.12, 0.0),
            (CH_LEAD, ska_guitar_fuzz, 0.3, 0.35, 0.16, 0.0),
            (CH_BASS, bass_pick_dark, 0.46, 0.0, 0.03, 0.0),
        ],
        ..RK_SOUND
    },
    double: &[(CH_KEYS, 0.9)],
    balance: &[(CH_KICK, -1.7), (CH_SNARE, -6.0), (CH_HAT, 4.2), (CH_PERC, 5.4), (CH_BASS, 0.5), (CH_LEAD, 9.2)],
    tilt: (0.0, -5.0),
    ..BASE
};

/// The R&B slow jam: a swung boom-bap, the long synth bass, Rhodes, strings,
/// a whistle's slide.
const NI_RNB: Style = Style {
    meter: "4/4 slow jam",
    human: 0.008,
    bpm: (84.0, 96.0),
    swing: (0.1, 0.2),
    modes: [3.0, 3.0, 0.0, 0.0, 0.5, 1.0, 0.0],
    maj: &[
        "Imaj7 vi7 ii7 V7 Imaj7 vi7 ii7 V7",
        "IVmaj7 iii7 vi7 V7/vi ii7 V7 Imaj7 Imaj7",
        "Imaj9 IVmaj7 iii7 vi7 ii7 V7 Imaj9 Imaj9",
    ],
    min: &["i9 iv9 bVIImaj7 bIIImaj7 bVImaj7 iim7b5 V7 i9", "i7 iv7 i7 iv7 bVImaj7 bVII7 i7 i7"],
    cadence: ("ii7 V7 Imaj7 Imaj7", "iv9 V7 i9 i9"),
    layers: [
        L_KEYS | L_PAD,
        KIT | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD,
        KIT | L_BASS | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        L_BASS | L_KEYS | L_PAD,
    ],
    grooves: [
        &[(CH_KICK, 0, "x......x..x....."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x......x..x..x.."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_HAT, HAT_C, "x-x-x-x-x-x-x.x-"),
            (CH_HAT, HAT_O, "..............x."),
        ],
    ],
    fill: FI_SOFT,
    bass: BS_808,
    comp: CP_HOLD,
    density: (0.35, 0.42),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_KEYS, ep_rhodes, 0.3, -0.2, 0.2, 0.0),
            (CH_BASS, bass_synth, 0.46, 0.0, 0.0, 0.0),
            (CH_LEAD, sine_lead, 0.28, 0.15, 0.25, 0.2),
            (CH_PAD, synth_strings, 0.16, 0.1, 0.3, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_808, 0.85, 0.0, 0.0),
            (CH_SNARE, kit_snare_clap, 0.6, 0.0, 0.15),
            (CH_HAT, kit_hats_808, 0.65, 0.2, 0.05),
            (CH_PERC, kit_shaker, 0.35, -0.25, 0.05),
            (CH_CYM, kit_cymbals, 0.2, -0.2, 0.12),
        ],
        room: (0.8, 0.22),
        echo: (6.0, 0.3),
        alts: &[(CH_LEAD, &[("whistle", sine_lead), ("alto sax", ska_alto), ("synth voice", vox_chop)])],
        ..BASE.sound
    },
    balance: &[(CH_KICK, -6.7), (CH_SNARE, 6.6), (CH_BASS, 5.1), (CH_PAD, 9.5), (CH_LEAD, 9.5), (CH_KEYS, 6.5)],
    chorus: &[(CH_PAD, 0.6)],
    ..BASE
};

// ---------------------------------------------------------------- 2000S

/// Pop-punk: fast, power chords in eighths, the snare on every off-beat in
/// the chorus, the tune on the lead guitar.
const NO_PUNK: Style = Style {
    meter: "4/4 pop-punk",
    human: 0.007,
    bpm: (160.0, 188.0),
    modes: [5.0, 0.0, 0.0, 0.0, 1.0, 1.5, 0.0],
    maj: &["I V vi IV I V vi IV", "I IV vi V I IV V V", "vi IV I V vi IV I V", "I V IV IV I V IV V"],
    min: &["i bVI bIII bVII i bVI bIII bVII"],
    cadence: ("IV V I I", "bVI bVII i i"),
    layers: RK_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x.....x.x......."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 0, "..x...x...x...x."), (CH_CYM, 1, "x.x.x.x.x.x.x.x.")],
    ],
    fill: FI_SNARE,
    bass: BS_ROOT8,
    comp: CP_POWER,
    tune: TU_SONG,
    density: (0.42, 0.5),
    sound: Sound {
        tones: &[
            (CH_KEYS, ska_guitar_dist, 0.3, -0.4, 0.08, 0.0),
            (CH_LEAD, ska_guitar_fuzz, 0.3, 0.35, 0.12, 0.0),
            (CH_BASS, bass_pick_dark, 0.46, 0.0, 0.03, 0.0),
        ],
        ..RK_SOUND
    },
    double: &[(CH_KEYS, 0.9)],
    balance: &[(CH_KICK, -2.8), (CH_SNARE, -6.3), (CH_HAT, 4.5), (CH_BASS, 3.1), (CH_LEAD, 9.5), (CH_KEYS, 2.6)],
    tilt: (0.0, -5.0),
    ..BASE
};

/// The R&B of the two-thousands: a syncopated kick, an 808, pizzicato
/// strings' stabs, a vocal chop's hook, FM keys answering.
const NO_RNB: Style = Style {
    meter: "4/4 r&b",
    bpm: (94.0, 106.0),
    swing: (0.0, 0.06),
    modes: [1.0, 2.0, 1.0, 0.0, 0.0, 3.0, 1.0],
    maj: &["vi IV I V vi IV I V", "ii7 V7 Imaj7 vi7 ii7 V7 Imaj7 vi7"],
    min: &["i i bVI bVI i i bVII bVII", "i iv bVI V i iv bVI V", "i7 i7 iv7 iv7 bVImaj7 bVImaj7 V7 V7"],
    cadence: ("IV V vi vi", "bVI V i i"),
    layers: [
        L_KEYS | L_KICK | L_HAT,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
        KIT | L_BASS | L_PAD,
        FULL | L_BASS | L_KEYS | L_CTR,
        0,
        0,
        KIT | L_BASS | L_KEYS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x..x..x...x..x.."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "xoxoxoxoxoxoxoxo")],
        &[
            (CH_KICK, 0, "x..x..x...x..x.x"),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_HAT, HAT_C, "xoxoxoxoxoxoxxxx"),
            (CH_PERC, RIM, "..x.......x....."),
        ],
    ],
    fill: FI_SOFT,
    bass: BS_808,
    comp: CP_STABS,
    tune: TU_HOOK,
    second: SV_ANSWER,
    density: (0.45, 0.5),
    sound: Sound {
        tones: &[
            (CH_KEYS, pizzicato, 0.26, -0.25, 0.12, 0.0),
            (CH_BASS, sub808, 0.5, 0.0, 0.0, 0.0),
            (CH_LEAD, vox_chop, 0.26, 0.15, 0.2, 0.15),
            (CH_BELL, glass_fm, 0.18, -0.3, 0.2, 0.1),
            (CH_PAD, synth_strings, 0.14, 0.2, 0.25, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_808, 0.85, 0.0, 0.0),
            (CH_SNARE, kit_clap_rim, 0.55, 0.0, 0.1),
            (CH_HAT, kit_hats_808, 0.6, 0.2, 0.03),
            (CH_PERC, kit_perc_hand, 0.4, -0.25, 0.05),
            (CH_CYM, kit_cymbals, 0.18, -0.2, 0.1),
        ],
        room: (0.76, 0.16),
        echo: (3.0, 0.25),
        alts: &[(CH_KEYS, &[("pizzicato strings", pizzicato), ("FM keys", glass_fm), ("marimba", marimba)])],
        ..BASE.sound
    },
    balance: &[(CH_KICK, -8.1), (CH_SNARE, 12.0), (CH_BASS, 5.3), (CH_PAD, 12.0), (CH_LEAD, 12.0), (CH_KEYS, 11.9)],
    chorus: &[(CH_PAD, 0.6)],
    ..BASE
};

/// Electro-pop: four on the floor, the saw bass off the beat, saw stabs,
/// the square's hook, the arpeggio, the pump.
const NO_ELECTRO: Style = Style {
    meter: "4/4 electro-pop",
    bpm: (122.0, 130.0),
    modes: [2.0, 1.0, 0.0, 0.0, 0.5, 3.0, 0.0],
    maj: &["I V vi IV I V vi IV", "vi IV I V vi IV I V"],
    min: &["i bVI bIII bVII i bVI bIII bVII", "i i bVI bVII i i bVI V"],
    cadence: ("IV V I I", "bVI bVII i i"),
    form: F_DANCE,
    layers: [
        L_KICK | L_HAT | L_BASS | L_ARP,
        FULL | L_BASS | L_ARP | L_LEAD,
        0,
        L_PAD | L_ARP | L_LEAD,
        0,
        L_SNARE | L_HAT | L_PAD | L_ARP,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD,
        L_KICK | L_HAT | L_BASS | L_ARP,
    ],
    grooves: G_FLOOR,
    fill: FI_SNARE,
    bass: BS_OFFBEAT,
    comp: CP_STABS,
    arp: AR_16,
    tune: TU_HOOK,
    density: (0.5, 0.55),
    sweep: true,
    pump: 0.4,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_moog, 0.45, 0.0, 0.0, 0.0),
            (CH_KEYS, stab_saw, 0.22, -0.2, 0.2, 0.1),
            (CH_PAD, pad_warm, 0.16, 0.0, 0.3, 0.0),
            (CH_ARP, arp_pluck, 0.18, 0.3, 0.2, 0.2),
            (CH_LEAD, lead_square, 0.28, 0.1, 0.22, 0.15),
        ],
        ..NI_EURO.sound
    },
    balance: &[
        (CH_KICK, -8.9),
        (CH_SNARE, 8.7),
        (CH_HAT, -3.4),
        (CH_BASS, 9.8),
        (CH_PAD, 8.0),
        (CH_ARP, 6.4),
        (CH_LEAD, 12.0),
        (CH_KEYS, 12.0),
    ],
    chorus: &[(CH_LEAD, 0.45), (CH_PAD, 0.6)],
    ..BASE
};

// ---------------------------------------------------------------- 2010S

const TE_KITS: &[KitRow] = &[
    (CH_KICK, kit_kick_909, 0.9, 0.0, 0.0),
    (CH_SNARE, kit_clap_rim, 0.6, 0.0, 0.15),
    (CH_HAT, kit_hats_909, 0.55, 0.2, 0.05),
    (CH_PERC, kit_shaker, 0.4, -0.25, 0.05),
    (CH_CYM, kit_cymbals, 0.22, -0.2, 0.15),
];

/// Big-room EDM: the build (the snare rolling, the riser, the filters
/// opening), the drop (the supersaw's chords and its hook, the pump).
const TE_EDM: Style = Style {
    meter: "4/4 big room",
    bpm: (126.0, 128.0),
    modes: [1.0, 0.5, 0.0, 0.0, 0.0, 4.0, 0.5],
    maj: &["IV V vi vi IV V I I", "vi IV I V vi IV I V"],
    min: &["bVI bVII i i bVI bVII bIII bIII", "i bVI bIII bVII i bVI bIII bVII", "i bVII bVI bVII i bVII bVI V"],
    cadence: ("bVI bVII i i", "bVI bVII i i"),
    form: F_DANCE,
    layers: [
        L_KICK | L_HAT | L_BASS,
        L_KICK | L_HAT | L_BASS | L_ARP | L_LEAD,
        0,
        L_PAD | L_ARP,
        0,
        L_SNARE | L_HAT | L_PAD | L_ARP | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD,
        L_KICK | L_HAT | L_BASS | L_ARP,
    ],
    grooves: [
        G_FLOOR[0],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_O, "..x...x...x...x."),
            (CH_PERC, 0, "x.x.x.x.x.x.x.x."),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_OFFBEAT,
    comp: CP_OFFBEAT,
    arp: AR_16,
    tune: TU_HOOK,
    density: (0.5, 0.6),
    sweep: true,
    pump: 0.5,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_reese, 0.42, 0.0, 0.0, 0.0),
            (CH_KEYS, hoover, 0.24, 0.0, 0.25, 0.0),
            (CH_PAD, pad_warm, 0.18, 0.0, 0.35, 0.0),
            (CH_ARP, arp_pluck, 0.2, 0.25, 0.2, 0.25),
            (CH_LEAD, lead_saw, 0.3, 0.0, 0.25, 0.2),
        ],
        kits: TE_KITS,
        room: (0.86, 0.28),
        echo: (3.0, 0.35),
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -10.2),
        (CH_SNARE, 8.6),
        (CH_HAT, -2.5),
        (CH_BASS, 12.0),
        (CH_PAD, 9.4),
        (CH_ARP, 4.4),
        (CH_LEAD, 10.9),
        (CH_KEYS, 12.0),
    ],
    chorus: &[(CH_LEAD, 0.5), (CH_KEYS, 0.5), (CH_PAD, 0.6)],
    ..BASE
};

/// Tropical house: soft four on the floor, the shaker, a marimba's (or a
/// steel pan's) syncopated plucks, a pan flute's hook, the pump.
const TE_TROPICAL: Style = Style {
    meter: "4/4 tropical house",
    bpm: (100.0, 112.0),
    modes: [5.0, 0.5, 0.0, 0.0, 0.5, 2.0, 0.0],
    maj: &["I V vi IV I V vi IV", "vi IV I V vi IV I V", "IV I V vi IV I V vi"],
    min: &["i bVI bIII bVII i bVI bIII bVII"],
    cadence: ("IV V I I", "bVI bVII i i"),
    layers: [
        L_ARP | L_PAD,
        KIT | L_BASS | L_ARP | L_LEAD,
        FULL | L_BASS | L_ARP | L_LEAD | L_KEYS | L_PAD,
        L_PAD | L_ARP | L_HAT,
        FULL | L_BASS | L_ARP | L_KEYS | L_LEAD,
        0,
        0,
        L_ARP | L_PAD | L_KICK,
    ],
    grooves: [
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "-x-x-x-x-x-x-x-x")],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_C, "-x-x-x-x-x-x-x-x"),
            (CH_PERC, TOM_MID, "..x..x....x..x.."),
        ],
    ],
    fill: FI_SOFT,
    bass: BS_ROOT4,
    comp: CP_BLOCK4,
    arp: AR_PLUCK,
    tune: TU_HOOK,
    density: (0.45, 0.5),
    pump: 0.35,
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_sub, 0.45, 0.0, 0.0, 0.0),
            (CH_KEYS, piano, 0.18, -0.2, 0.25, 0.0),
            (CH_PAD, pad_warm, 0.16, 0.0, 0.35, 0.0),
            (CH_ARP, marimba, 0.26, 0.25, 0.2, 0.2),
            (CH_LEAD, pan_flute, 0.3, 0.0, 0.25, 0.2),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.85, 0.0, 0.0),
            (CH_SNARE, kit_clap_rim, 0.5, 0.0, 0.2),
            (CH_HAT, kit_shaker, 0.5, 0.2, 0.05),
            (CH_PERC, kit_perc_hand, 0.4, -0.25, 0.08),
        ],
        room: (0.84, 0.28),
        echo: (3.0, 0.35),
        alts: &[
            (CH_ARP, &[("marimba", marimba), ("steel drums", steel_drums), ("kalimba", kalimba)]),
            (CH_LEAD, &[("pan flute", pan_flute), ("alto sax", ska_alto), ("synth voice", vox_chop)]),
        ],
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -7.1),
        (CH_PERC, -2.3),
        (CH_BASS, 3.7),
        (CH_PAD, 5.0),
        (CH_ARP, 2.2),
        (CH_LEAD, 5.9),
        (CH_KEYS, 12.0),
    ],
    chorus: &[(CH_PAD, 0.6)],
    ..BASE
};

/// Trap-pop: the half-time clap, the hats rolling, the 808 sliding, a
/// bell's hook, keys answering.
const TE_TRAP: Style = Style {
    meter: "4/4 trap-pop",
    bpm: (136.0, 150.0),
    modes: [1.0, 1.0, 1.0, 0.0, 0.0, 4.0, 1.0],
    maj: &["vi IV I V vi IV I V"],
    min: &["i i bVI bVI i i bVII bVII", "i bVI bIII bVII i bVI bIII bVII", "i iv bVI bVII i iv bVI V"],
    cadence: ("bVI bVII i i", "bVI bVII i i"),
    layers: [
        L_KEYS | L_HAT,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
        L_HAT | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_CTR,
        0,
        0,
        L_KEYS | L_HAT | L_BASS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, "........x......."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x......xx.x...x."),
            (CH_SNARE, 0, "........x......."),
            (CH_HAT, HAT_C, "x.xxx.x.x.x.xxxx"),
            (CH_HAT, HAT_O, "......x........."),
        ],
    ],
    fill: FI_NONE,
    bass: BS_808,
    comp: CP_HOLD,
    tune: TU_HOOK,
    second: SV_ANSWER,
    density: (0.42, 0.5),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_BASS, sub808, 0.52, 0.0, 0.0, 0.0),
            (CH_KEYS, piano_felt, 0.22, -0.2, 0.25, 0.0),
            (CH_PAD, pad_dark, 0.14, 0.0, 0.3, 0.0),
            (CH_LEAD, bell_fm, 0.26, 0.15, 0.25, 0.2),
            (CH_BELL, glass_fm, 0.18, -0.3, 0.25, 0.15),
        ],
        kits: &[
            (CH_KICK, kit_kick_808, 0.85, 0.0, 0.0),
            (CH_SNARE, kit_clap_808, 0.6, 0.0, 0.15),
            (CH_HAT, kit_hats_808, 0.6, 0.2, 0.03),
            (CH_PERC, kit_perc_metal, 0.3, -0.25, 0.05),
        ],
        room: (0.8, 0.22),
        echo: (3.0, 0.3),
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -8.8),
        (CH_SNARE, 12.0),
        (CH_HAT, 2.3),
        (CH_BASS, 3.5),
        (CH_PAD, 12.0),
        (CH_LEAD, 11.6),
        (CH_KEYS, 12.0),
    ],
    ..BASE
};

/// The folk stomp: kick and claps, a strummed guitar, a banjo or a
/// mandolin picking, a glockenspiel's tune, the piano answering.
const TE_STOMP: Style = Style {
    meter: "4/4 folk stomp",
    human: 0.01,
    bpm: (108.0, 124.0),
    modes: [6.0, 0.5, 0.0, 0.0, 1.0, 1.0, 0.0],
    maj: &["I V vi IV I V vi IV", "vi IV I V vi IV I V", "I IV I V I IV V V", "I iii IV I I iii IV V"],
    min: &["i bVI bIII bVII i bVI bIII bVII"],
    layers: [
        L_KEYS | L_KICK,
        L_KICK | L_SNARE | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_ARP | L_CTR,
        L_KICK | L_KEYS | L_ARP,
        FULL | L_BASS | L_KEYS | L_ARP,
        0,
        0,
        L_KICK | L_SNARE | L_KEYS | L_BASS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 1, "....x.......x...")],
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 1, "....x.......x..."), (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x.")],
    ],
    fill: FI_NONE,
    bass: BS_ROOT4,
    comp: CP_STRUM,
    arp: AR_8,
    second: SV_ANSWER,
    sound: Sound {
        tones: &[
            (CH_KEYS, steel_guitar, 0.3, -0.25, 0.15, 0.0),
            (CH_BASS, bass_upright, 0.42, 0.0, 0.05, 0.0),
            (CH_ARP, banjo, 0.2, 0.35, 0.15, 0.0),
            (CH_LEAD, glockenspiel, 0.26, 0.1, 0.2, 0.0),
            (CH_BELL, piano, 0.2, -0.1, 0.2, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.9, 0.0, 0.1),
            (CH_SNARE, kit_snare_clap, 0.7, 0.0, 0.2),
            (CH_PERC, ska_kit_toms, 0.45, -0.25, 0.1),
        ],
        room: (0.8, 0.25),
        alts: &[
            (CH_ARP, &[("banjo", banjo), ("mandolin", cittern)]),
            (CH_LEAD, &[("glockenspiel", glockenspiel), ("whistle", whistle), ("piano", piano)]),
        ],
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.8)],
    balance: &[
        (CH_KICK, -7.5),
        (CH_SNARE, 12.0),
        (CH_PERC, 2.9),
        (CH_BASS, 4.8),
        (CH_ARP, 5.9),
        (CH_LEAD, 12.0),
        (CH_KEYS, 2.0),
    ],
    ..BASE
};

// ---------------------------------------------------------------- 2020S

/// Nu-disco: a LinnDrum's floor, the synth bass in octaves, polysynth
/// stabs, the arpeggio, a saw's hook, the gated snare.
const TW_NUDISCO: Style = Style {
    meter: "4/4 nu-disco",
    bpm: (110.0, 124.0),
    modes: [2.0, 2.0, 0.0, 0.0, 0.5, 3.0, 0.0],
    maj: &["vi IV I V vi IV I V", "Imaj7 vi7 ii7 V7 Imaj7 vi7 ii7 V7", "ii7 V7 iii7 vi7 ii7 V7 Imaj7 Imaj7"],
    min: &["i bVII bVI bVII i bVII bVI V", "i7 iv7 bVII7 bIIImaj7 bVImaj7 iv7 V7 i7"],
    cadence: ("ii7 V7 I I", "bVI bVII i i"),
    layers: [
        L_KICK | L_HAT | L_ARP | L_BASS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_ARP,
        L_KICK | L_HAT | L_PAD | L_ARP,
        FULL | L_BASS | L_KEYS | L_ARP | L_LEAD,
        0,
        0,
        L_KICK | L_HAT | L_BASS | L_ARP,
    ],
    grooves: [
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-")],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 0, "....X.......X..."),
            (CH_HAT, HAT_C, "x...x...x...x..."),
            (CH_HAT, HAT_O, "..x...x...x...x."),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_DISCO,
    comp: CP_STABS,
    arp: AR_8,
    tune: TU_HOOK,
    sweep: true,
    pump: 0.25,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_moog, 0.45, 0.0, 0.0, 0.0),
            (CH_KEYS, hoover, 0.22, -0.2, 0.2, 0.0),
            (CH_PAD, pad_warm, 0.15, 0.0, 0.3, 0.0),
            (CH_ARP, arp_pluck, 0.2, 0.3, 0.2, 0.2),
            (CH_LEAD, lead_saw, 0.28, 0.1, 0.22, 0.15),
        ],
        ..EI_SOUND
    },
    balance: &[
        (CH_KICK, -6.2),
        (CH_SNARE, 1.4),
        (CH_HAT, -3.5),
        (CH_BASS, 1.8),
        (CH_PAD, 7.8),
        (CH_ARP, 4.1),
        (CH_LEAD, 6.3),
        (CH_KEYS, 12.0),
    ],
    chorus: &[(CH_LEAD, 0.45), (CH_KEYS, 0.5), (CH_PAD, 0.6)],
    ..BASE
};

/// Amapiano: the soft floor, the shaker's 16ths, the log drum, jazz
/// piano chords, a deep pad, a melody answered.
const TW_AMAPIANO: Style = Style {
    meter: "4/4 amapiano",
    human: 0.007,
    bpm: (110.0, 116.0),
    swing: (0.04, 0.1),
    modes: [1.0, 3.0, 0.0, 0.0, 0.5, 3.0, 0.0],
    maj: &["ii7 V7 Imaj7 vi7 ii7 V7 Imaj7 vi7", "Imaj7 iii7 vi7 IVmaj7 Imaj7 iii7 ii7 V7"],
    min: &[
        "i7 iv7 i7 iv7 bVImaj7 bVII7 i7 i7",
        "i9 bVImaj7 iv9 V7sus4 i9 bVImaj7 iv9 V7",
        "i7 i7 bVImaj7 bVImaj7 iv7 iv7 bVII7 bVII7",
    ],
    cadence: ("ii7 V7 Imaj7 Imaj7", "iv7 bVII7 i7 i7"),
    form: F_VAMP,
    layers: [
        L_KEYS | L_HAT | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD | L_CTR,
        L_HAT | L_PERC | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        L_HAT | L_KEYS | L_BASS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-"), (CH_PERC, RIM, "...x..x....x..x.")],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-"),
            (CH_PERC, RIM, "...x..x....x..x."),
            (CH_SNARE, 1, "........x......."),
        ],
    ],
    fill: FI_NONE,
    bass: BS_LOG,
    comp: CP_STABS,
    second: SV_ANSWER,
    density: (0.35, 0.45),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_BASS, log_drum, 0.5, 0.0, 0.05, 0.0),
            (CH_KEYS, piano, 0.26, -0.2, 0.2, 0.0),
            (CH_PAD, pad_dark, 0.16, 0.0, 0.35, 0.0),
            (CH_LEAD, flute, 0.26, 0.2, 0.25, 0.15),
            (CH_BELL, ep_rhodes, 0.18, -0.3, 0.2, 0.1),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.75, 0.0, 0.0),
            (CH_SNARE, kit_clap_rim, 0.5, 0.0, 0.15),
            (CH_HAT, kit_shaker, 0.55, 0.2, 0.05),
            (CH_PERC, kit_perc_hand, 0.45, -0.25, 0.08),
        ],
        room: (0.82, 0.24),
        echo: (3.0, 0.3),
        alts: &[(CH_LEAD, &[("flute", flute), ("alto sax", ska_alto), ("Rhodes", ep_rhodes)])],
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -5.7),
        (CH_HAT, -0.8),
        (CH_PERC, 3.3),
        (CH_BASS, 1.2),
        (CH_PAD, 7.1),
        (CH_LEAD, 8.1),
        (CH_KEYS, 7.1),
    ],
    chorus: &[(CH_PAD, 0.6)],
    ..BASE
};

/// Afrobeats: the kick and rim in their 3-3-2, the shaker, congas, a
/// highlife guitar's broken chords, the tune.
const TW_AFRO: Style = Style {
    meter: "4/4 afrobeats",
    human: 0.008,
    bpm: (100.0, 110.0),
    swing: (0.02, 0.08),
    modes: [4.0, 2.0, 0.0, 0.0, 1.0, 2.0, 0.0],
    maj: &["I V vi IV I V vi IV", "ii7 V7 Imaj7 Imaj7 ii7 V7 Imaj7 vi7", "IV V iii vi IV V I I"],
    min: &["i7 iv7 bVII7 bIIImaj7 i7 iv7 bVImaj7 V7", "i bVI bIII bVII i bVI bIII bVII"],
    cadence: ("IV V I I", "iv bVII i i"),
    layers: [
        L_KEYS | L_HAT | L_PERC,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
        L_HAT | L_PERC | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD,
        0,
        0,
        L_HAT | L_PERC | L_KEYS | L_BASS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.....x...x....."), (CH_SNARE, 1, "...x..x....x..x."), (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-")],
        &[
            (CH_KICK, 0, "x.....x...x....."),
            (CH_SNARE, 1, "...x..x....x..x."),
            (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-"),
            (CH_PERC, TOM_MID, "..x...x...x..x.."),
        ],
    ],
    fill: FI_SOFT,
    bass: BS_ROOT4,
    comp: CP_ARP8,
    second: SV_ANSWER,
    pump: 0.15,
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_BASS, bass_synth, 0.45, 0.0, 0.0, 0.0),
            (CH_KEYS, elec_clean, 0.24, -0.25, 0.18, 0.1),
            (CH_PAD, pad_warm, 0.15, 0.0, 0.3, 0.0),
            (CH_LEAD, ska_guitar_clean, 0.26, 0.15, 0.22, 0.15),
            (CH_BELL, marimba, 0.2, -0.3, 0.2, 0.1),
        ],
        kits: &[
            (CH_KICK, kit_kick_808, 0.8, 0.0, 0.0),
            (CH_SNARE, kit_clap_rim, 0.55, 0.0, 0.12),
            (CH_HAT, kit_shaker, 0.55, 0.2, 0.05),
            (CH_PERC, kit_perc_hand, 0.45, -0.25, 0.08),
        ],
        room: (0.8, 0.22),
        alts: &[(CH_LEAD, &[("highlife guitar", ska_guitar_clean), ("alto sax", ska_alto), ("flute", flute)])],
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.7)],
    balance: &[
        (CH_KICK, -7.1),
        (CH_SNARE, 12.0),
        (CH_HAT, 0.8),
        (CH_PERC, -1.3),
        (CH_BASS, 6.1),
        (CH_PAD, 7.3),
        (CH_LEAD, 10.6),
        (CH_KEYS, 9.1),
    ],
    chorus: &[(CH_PAD, 0.6)],
    ..BASE
};

// ---------------------------------------------------------------- KENYAN

const KE_KITS: &[KitRow] = &[
    (CH_KICK, ska_kit_kick, 0.8, 0.0, 0.05),
    (CH_SNARE, kit_snare_rim, 0.7, 0.05, 0.15),
    (CH_HAT, ska_kit_hats, 0.4, 0.25, 0.05),
    (CH_PERC, kit_shaker, 0.45, -0.3, 0.05),
    (CH_CYM, kit_cymbals, 0.22, -0.2, 0.12),
];

/// Benga: fast, the bass running high through the chord, the rhythm
/// guitar's broken chords, the lead guitar's hook with a second in
/// thirds, the kayamba's shaker, a guitar solo.
const KE_BENGA: Style = Style {
    meter: "4/4 benga",
    human: 0.008,
    bpm: (128.0, 148.0),
    modes: [6.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.0],
    maj: &["I IV I V I IV V I", "I I IV IV V V I I", "I IV V IV I IV V I", "I V IV V I V IV I"],
    min: &["i iv i V i iv V i"],
    cadence: ("IV V I I", "iv V i i"),
    form: F_VAMP,
    layers: [
        L_KEYS | L_BASS | L_HAT,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_CTR,
        L_HAT | L_PERC | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS,
    ],
    grooves: [
        &[
            (CH_KICK, 0, "x.......x......."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-."),
            (CH_PERC, 0, "-o-o-o-o-o-o-o-o"),
        ],
        &[
            (CH_KICK, 0, "x...-...x...-..."),
            (CH_SNARE, 0, "....x..o....x.o."),
            (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-"),
            (CH_PERC, 0, "-o-o-o-o-o-o-o-o"),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_BENGA,
    comp: CP_STRUM,
    tune: TU_HOOK,
    second: SV_THIRDS,
    voice: (72, -2, 9),
    density: (0.42, 0.5),
    sound: Sound {
        tones: &[
            (CH_LEAD, jazz_guitar, 0.4, 0.25, 0.15, 0.0),
            (CH_BELL, elec_clean, 0.22, -0.3, 0.15, 0.0),
            (CH_KEYS, elec_clean, 0.22, -0.15, 0.12, 0.0),
            (CH_BASS, ska_bass_flat, 0.38, 0.0, 0.03, 0.0),
        ],
        kits: KE_KITS,
        room: (0.76, 0.18),
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.7)],
    balance: &[(CH_KICK, -3.9), (CH_HAT, -1.2), (CH_BASS, 3.9), (CH_LEAD, 3.4), (CH_KEYS, 4.4)],
    ..BASE
};

/// Kenyan rumba: the congas, the side stick, the rhythm guitar, the bass's
/// figures, the lead guitar's lines answered by the second's; the sebene
/// (the solo) faster on the hats.
const KE_RUMBA: Style = Style {
    meter: "4/4 rumba",
    human: 0.008,
    bpm: (118.0, 134.0),
    modes: [6.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.0],
    maj: &["I IV V IV I IV V IV", "I V IV V I V IV V", "I IV I V I IV I V"],
    min: &["i iv V iv i iv V iv"],
    cadence: ("IV V I I", "iv V i i"),
    form: F_VAMP,
    layers: [
        L_KEYS | L_PERC,
        FULL | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_CTR,
        L_PERC | L_HAT | L_BASS | L_KEYS,
        FULL | L_BASS | L_KEYS | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS,
    ],
    grooves: [
        &[
            (CH_KICK, 0, "x..x....x..x...."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-."),
            (CH_PERC, TOM_MID, "..x...x...x...x."),
            (CH_PERC, TOM_LO, "......x.x......."),
        ],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 0, "....x.......x..x"),
            (CH_HAT, HAT_C, "xoxoxoxoxoxoxoxo"),
            (CH_PERC, TOM_MID, "..x...x...x...x."),
            (CH_PERC, TOM_LO, "......x.x......."),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_MOTOWN,
    comp: CP_STABS,
    tune: TU_HOOK,
    second: SV_ANSWER,
    voice: (72, -2, 9),
    density: (0.5, 0.55),
    sound: Sound {
        tones: &[
            (CH_LEAD, jazz_guitar, 0.4, 0.25, 0.18, 0.0),
            (CH_BELL, elec_clean, 0.22, -0.3, 0.18, 0.0),
            (CH_KEYS, jazz_guitar, 0.22, -0.15, 0.15, 0.0),
            (CH_BASS, bass_upright, 0.46, 0.0, 0.03, 0.0),
        ],
        kits: &[
            (CH_KICK, ska_kit_kick, 0.75, 0.0, 0.05),
            (CH_SNARE, kit_snare_rim, 0.7, 0.05, 0.15),
            (CH_HAT, ska_kit_hats, 0.5, 0.25, 0.05),
            (CH_PERC, kit_perc_hand, 0.5, -0.3, 0.1),
            (CH_CYM, kit_cymbals, 0.2, -0.2, 0.12),
        ],
        room: (0.78, 0.2),
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.6)],
    balance: &[
        (CH_KICK, -4.6),
        (CH_SNARE, 1.0),
        (CH_HAT, -1.7),
        (CH_PERC, -6.5),
        (CH_BASS, 4.5),
        (CH_LEAD, 3.2),
        (CH_KEYS, 9.9),
    ],
    ..BASE
};

/// The nyatiti: the lyre's running figures (its low strings the bass), the
/// ongeng'o ringing in eighths, a drum, the orutu fiddle's tune.
const KE_NYATITI: Style = Style {
    meter: "4/4 nyatiti",
    human: 0.01,
    bpm: (110.0, 128.0),
    modes: [3.0, 1.0, 0.0, 0.0, 2.0, 1.0, 0.0],
    maj: &["I I IV I I I V I", "I IV I V I IV I I", "I I V V IV IV I I"],
    min: &["i i iv i i i v i", "i bVII i bVII i iv i i"],
    cadence: ("IV V I I", "iv v i i"),
    form: F_VAMP,
    layers: [
        L_ARP | L_PERC,
        L_ARP | L_BASS | L_PERC | L_KICK | L_LEAD,
        L_ARP | L_BASS | L_PERC | L_KICK | L_HAT | L_LEAD,
        L_ARP | L_PERC,
        L_ARP | L_BASS | L_PERC | L_KICK | L_HAT | L_LEAD,
        0,
        0,
        L_ARP | L_BASS | L_PERC,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.....x...x....."), (CH_PERC, RIM, "x.x.x.x.x.x.x.x."), (CH_HAT, 0, "-x-x-x-x-x-x-x-x")],
        &[
            (CH_KICK, 0, "x.....x...x....."),
            (CH_PERC, RIM, "x.x.x.x.x.x.x.x."),
            (CH_PERC, TOM_MID, "..x...x...x...x."),
            (CH_HAT, 0, "-x-x-x-x-x-x-x-x"),
        ],
    ],
    fill: FI_NONE,
    bass: BS_BENGA,
    comp: CP_NONE,
    arp: AR_8,
    tune: TU_SONG,
    voice: (74, -2, 8),
    density: (0.42, 0.5),
    sound: Sound {
        tones: &[
            (CH_ARP, nyatiti, 0.3, -0.2, 0.15, 0.0),
            (CH_BASS, nyatiti, 0.3, -0.1, 0.12, 0.0),
            (CH_LEAD, fiddle, 0.3, 0.2, 0.2, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_frame, 0.8, 0.0, 0.08),
            (CH_HAT, kit_shaker, 0.45, 0.25, 0.05),
            (CH_PERC, kit_ongengo, 0.5, -0.25, 0.1),
        ],
        room: (0.74, 0.16),
        ..BASE.sound
    },
    balance: &[(CH_KICK, -5.0), (CH_HAT, 4.6), (CH_PERC, -2.7), (CH_BASS, 8.6), (CH_ARP, 1.6), (CH_LEAD, 9.7)],
    ..BASE
};

// ---------------------------------------------------------------- YOGA

const YO_MAJ: &[&str] = &[
    "I I I I IV IV I I",
    "I I bVII bVII I I IV I",
    "Isus2 Isus2 Isus2 Isus2 IVadd9 IVadd9 Isus2 Isus2",
    "I I IV IV I I V I",
];
const YO_MIN: &[&str] = &["i i i i iv iv i i", "i9 i9 bVII bVII i9 i9 iv9 i9", "i i bVI bVII i i iv i"];
const YO_KITS: &[KitRow] = &[
    (CH_KICK, kit_kick_frame, 0.7, 0.0, 0.2),
    (CH_HAT, kit_manjira, 0.4, 0.25, 0.3),
    (CH_PERC, kit_tabla, 0.6, -0.15, 0.2),
];
const YO_SOUND: Sound = Sound {
    tones: &[
        (CH_LEAD, bansuri, 0.32, 0.1, 0.35, 0.18),
        (CH_DRONE, tanpura, 0.24, -0.1, 0.35, 0.0),
        (CH_PAD, pad_shimmer, 0.14, 0.0, 0.45, 0.0),
        (CH_ARP, harp, 0.2, 0.3, 0.35, 0.1),
        (CH_BELL, singing_bowl, 0.2, -0.3, 0.45, 0.0),
    ],
    kits: YO_KITS,
    room: (0.88, 0.38),
    echo: (6.0, 0.35),
    extra: quiet,
    alts: &[
        (CH_LEAD, &[("bansuri", bansuri), ("pan flute", pan_flute), ("flute", flute)]),
        (CH_ARP, &[("harp", harp), ("kalimba", kalimba), ("dulcimer", dulcimer)]),
    ],
    ..BASE.sound
};

/// A meditation: the tanpura alone, the bansuri's alap over it, the pad
/// and the harp gathering, bowls ringing.
const YO_MEDITATION: Style = Style {
    meter: "4/4 meditation",
    human: 0.012,
    bpm: (56.0, 66.0),
    modes: [2.0, 2.0, 1.0, 1.0, 2.0, 1.0, 0.0],
    maj: YO_MAJ,
    min: YO_MIN,
    cadence: ("IV IV I I", "iv iv i i"),
    form: F_RAGA,
    layers: [
        L_DRONE | L_LEAD,
        L_DRONE | L_LEAD | L_PAD,
        L_DRONE | L_LEAD | L_PAD | L_ARP | L_CTR,
        L_DRONE | L_PAD | L_CTR,
        0,
        0,
        0,
        L_DRONE | L_PAD | L_LEAD,
    ],
    grooves: G_TABLA,
    fill: FI_NONE,
    bass: BS_NONE,
    comp: CP_NONE,
    arp: AR_8,
    tune: TU_LONG,
    second: SV_BOWL,
    drone: DR_TANPURA,
    voice: (72, -3, 9),
    density: (0.25, 0.3),
    end_hit: false,
    sound: YO_SOUND,
    ..BASE
};

/// A flow: the hand drums' theka under the kalimba or the harp, the bass
/// held, the flute, the tanpura.
const YO_FLOW: Style = Style {
    meter: "4/4 flow",
    human: 0.01,
    bpm: (80.0, 96.0),
    modes: [3.0, 2.0, 0.0, 1.0, 2.0, 1.5, 0.0],
    maj: YO_MAJ,
    min: YO_MIN,
    cadence: ("IV IV I I", "iv iv i i"),
    form: F_RAGA,
    layers: [
        L_DRONE | L_ARP,
        L_DRONE | L_ARP | L_PERC | L_LEAD,
        L_DRONE | L_ARP | L_PERC | L_LEAD | L_PAD | L_BASS | L_HAT,
        L_DRONE | L_PAD | L_ARP,
        0,
        0,
        0,
        L_DRONE | L_ARP | L_PAD,
    ],
    grooves: G_TABLA,
    fill: FI_NONE,
    bass: BS_HOLD,
    comp: CP_NONE,
    arp: AR_8,
    tune: TU_LONG,
    drone: DR_TANPURA,
    voice: (72, -3, 9),
    density: (0.3, 0.38),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_LEAD, flute, 0.3, 0.1, 0.32, 0.15),
            (CH_DRONE, tanpura, 0.22, -0.1, 0.32, 0.0),
            (CH_PAD, pad_warm, 0.14, 0.0, 0.4, 0.0),
            (CH_ARP, kalimba, 0.24, 0.3, 0.3, 0.15),
            (CH_BASS, bass_sub, 0.3, 0.0, 0.05, 0.0),
        ],
        ..YO_SOUND
    },
    ..BASE
};

/// A kirtan: the harmonium's chords, a chant's tune and its answer, the
/// tabla, the manjira on the beats, the tempo up as it goes.
const YO_KIRTAN: Style = Style {
    meter: "4/4 kirtan",
    human: 0.01,
    bpm: (84.0, 104.0),
    modes: [4.0, 0.5, 0.0, 0.0, 3.0, 1.0, 0.0],
    maj: &["I I IV IV V V I I", "I IV I V I IV V I", "I bVII IV I I bVII IV I"],
    min: &["i i iv iv bVII bVII i i", "i bVII i bVII i iv bVII i"],
    cadence: ("IV V I I", "iv bVII i i"),
    form: F_VAMP,
    layers: [
        L_DRONE | L_KEYS,
        L_DRONE | L_KEYS | L_PERC | L_LEAD,
        L_DRONE | L_KEYS | L_PERC | L_LEAD | L_HAT | L_CTR,
        L_DRONE | L_KEYS | L_PERC,
        L_DRONE | L_KEYS | L_PERC | L_LEAD | L_HAT,
        0,
        0,
        L_DRONE | L_KEYS | L_LEAD,
    ],
    grooves: [
        &[(CH_PERC, TOM_LO, "x.......x......."), (CH_PERC, RIM, "..x.x.....x.x..."), (CH_HAT, 0, "x...x...x...x...")],
        G_TABLA[1],
    ],
    fill: FI_NONE,
    bass: BS_NONE,
    comp: CP_HOLD,
    tune: TU_SONG,
    second: SV_THIRDS,
    drone: DR_TANPURA,
    voice: (70, -3, 8),
    density: (0.4, 0.45),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_KEYS, harmonium, 0.26, -0.15, 0.25, 0.0),
            (CH_LEAD, bansuri, 0.3, 0.15, 0.3, 0.1),
            (CH_BELL, pan_flute, 0.2, -0.25, 0.3, 0.1),
            (CH_DRONE, tanpura, 0.2, 0.0, 0.3, 0.0),
        ],
        ..YO_SOUND
    },
    ..BASE
};

// ---------------------------------------------------------------- ELEVATOR

const EL_MAJ: &[&str] = &[
    "Imaj7 Imaj7 II7 II7 ii7 V7 Imaj7 Imaj7",
    "Imaj7 vi7 ii7 V7 iii7 VI7 ii7 V7",
    "Imaj9 IVmaj7 iii7 VI7 ii9 V9 Imaj7 V7sus4",
    "IVmaj7 iii7 vi7 II7 ii7 V7 Imaj7 Imaj7",
];
const EL_MIN: &[&str] = &[
    "i7 i7 iv7 iv7 iim7b5 V7 i7 i7",
    "i9 iv9 bVIImaj7 bIIImaj7 bVImaj7 iim7b5 V7 i9",
    "i6 i6 iv6 iv6 iim7b5 V7 i6 V7",
];
const EL_LAYERS: [u32; 8] = [
    L_KEYS | L_BASS,
    KIT | L_BASS | L_KEYS | L_LEAD,
    FULL | L_BASS | L_KEYS | L_LEAD | L_PAD | L_CTR,
    KIT | L_BASS | L_KEYS | L_PAD,
    FULL | L_BASS | L_KEYS | L_LEAD,
    0,
    0,
    KIT | L_BASS | L_KEYS | L_PAD,
];

/// Bossa nova: the guitar's syncopated chords, the bass's root and fifth,
/// the side stick's clave, a flute's or the vibraphone's tune, strings.
const EL_BOSSA: Style = Style {
    meter: "4/4 bossa nova",
    human: 0.009,
    bpm: (116.0, 134.0),
    modes: [5.0, 1.0, 0.0, 1.0, 0.5, 1.0, 0.0],
    maj: EL_MAJ,
    min: EL_MIN,
    cadence: ("ii7 V7 Imaj7 Imaj7", "iim7b5 V7 i7 i7"),
    layers: EL_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x.....x.x.....x."), (CH_SNARE, 1, "x..x..x...x..x.."), (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-")],
        &[(CH_KICK, 0, "x.....x.x.....x."), (CH_SNARE, 1, "x..x..x...x..x.."), (CH_CYM, 1, "x.x.x.x.x.x.x.x.")],
    ],
    fill: FI_SOFT,
    bass: BS_BOSSA,
    comp: CP_BOSSA,
    second: SV_GUIDE,
    voice: (74, -2, 8),
    density: (0.35, 0.42),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_KEYS, nylon, 0.3, -0.25, 0.15, 0.0),
            (CH_BASS, bass_upright, 0.42, 0.0, 0.05, 0.0),
            (CH_LEAD, flute, 0.28, 0.2, 0.22, 0.0),
            (CH_PAD, pad_strings, 0.14, 0.1, 0.3, 0.0),
            (CH_BELL, pad_strings, 0.12, -0.3, 0.3, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.6, 0.0, 0.05),
            (CH_SNARE, kit_bossa_snare, 0.6, 0.1, 0.15),
            (CH_HAT, kit_shaker, 0.45, 0.25, 0.05),
            (CH_CYM, kit_cymbals, 0.18, -0.25, 0.15),
        ],
        room: (0.8, 0.22),
        alts: &[(
            CH_LEAD,
            &[("flute", flute), ("vibraphone", vibes), ("Rhodes", ep_rhodes), ("muted trumpet", muted_trumpet)],
        )],
        ..BASE.sound
    },
    balance: &[(CH_KICK, -5.3), (CH_SNARE, 8.2), (CH_BASS, 3.6), (CH_PAD, 6.3), (CH_LEAD, 4.4), (CH_KEYS, 2.4)],
    ..BASE
};

/// Lounge: a light swing, the ride, brushes, the walking bass, the
/// piano's comping, the vibraphone's tune, strings.
const EL_LOUNGE: Style = Style {
    meter: "4/4 lounge",
    human: 0.01,
    bpm: (92.0, 116.0),
    swing: (0.18, 0.28),
    modes: [5.0, 1.0, 0.0, 0.5, 0.5, 1.0, 0.0],
    maj: EL_MAJ,
    min: EL_MIN,
    cadence: ("ii7 V7 Imaj7 Imaj7", "iim7b5 V7 i6 i6"),
    layers: EL_LAYERS,
    grooves: [
        &[
            (CH_CYM, 1, "x...x.x.x...x.x."),
            (CH_HAT, HAT_C, "....x.......x..."),
            (CH_KICK, 0, "o.......o......."),
            (CH_SNARE, 0, "..........o....."),
        ],
        &[
            (CH_CYM, 1, "x...x.x.x...x.x."),
            (CH_HAT, HAT_C, "....x.......x..."),
            (CH_KICK, 0, "o...o...o...o..."),
            (CH_SNARE, 0, "......o...o....o"),
        ],
    ],
    fill: FI_SOFT,
    bass: BS_WALK,
    comp: CP_STABS,
    second: SV_GUIDE,
    voice: (72, -2, 8),
    density: (0.35, 0.42),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_KEYS, piano, 0.26, -0.25, 0.18, 0.0),
            (CH_BASS, bass_upright, 0.44, 0.0, 0.05, 0.0),
            (CH_LEAD, vibes, 0.3, 0.2, 0.22, 0.0),
            (CH_PAD, pad_strings, 0.14, 0.1, 0.3, 0.0),
            (CH_BELL, pad_strings, 0.12, -0.3, 0.3, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.6, 0.0, 0.05),
            (CH_SNARE, kit_brush, 0.7, 0.1, 0.15),
            (CH_HAT, ska_kit_hats, 0.4, 0.25, 0.05),
            (CH_CYM, kit_cymbals, 0.3, -0.25, 0.15),
        ],
        room: (0.8, 0.24),
        alts: &[(CH_LEAD, &[("vibraphone", vibes), ("alto sax", ska_alto), ("flute", flute), ("clarinet", clarinet)])],
        ..BASE.sound
    },
    balance: &[(CH_SNARE, 11.6), (CH_CYM, 3.3), (CH_BASS, -2.1), (CH_PAD, 1.0), (CH_KEYS, 1.7)],
    ..BASE
};

/// Smooth: the Rhodes's broken chords, the fretless bass, a soft kit,
/// the alto's tune, the vibraphone an octave up.
const EL_SMOOTH: Style = Style {
    meter: "4/4 smooth",
    human: 0.008,
    bpm: (86.0, 100.0),
    modes: [5.0, 1.0, 0.0, 1.0, 0.5, 1.0, 0.0],
    maj: EL_MAJ,
    min: EL_MIN,
    cadence: ("ii7 V7 Imaj7 Imaj7", "iim7b5 V7 i7 i7"),
    layers: EL_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "-.-.-.-.-.-.-.-.")],
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-")],
    ],
    fill: FI_SOFT,
    bass: BS_ROOT4,
    comp: CP_ARP8,
    second: SV_OCTAVE,
    voice: (70, -2, 8),
    density: (0.35, 0.42),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_KEYS, ep_rhodes, 0.28, -0.25, 0.2, 0.0),
            (CH_BASS, bass_upright, 0.42, 0.0, 0.05, 0.0),
            (CH_LEAD, ska_alto, 0.3, 0.2, 0.25, 0.0),
            (CH_PAD, pad_strings, 0.14, 0.1, 0.3, 0.0),
            (CH_BELL, vibes, 0.18, -0.3, 0.25, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.65, 0.0, 0.05),
            (CH_SNARE, kit_snare_rim, 0.55, 0.1, 0.15),
            (CH_HAT, ska_kit_hats, 0.4, 0.25, 0.05),
            (CH_PERC, ska_kit_toms, 0.35, -0.25, 0.1),
            (CH_CYM, kit_cymbals, 0.18, -0.25, 0.15),
        ],
        room: (0.8, 0.24),
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -5.3),
        (CH_SNARE, 12.0),
        (CH_HAT, -2.7),
        (CH_BASS, 2.0),
        (CH_PAD, 4.4),
        (CH_LEAD, 3.9),
        (CH_KEYS, 3.0),
    ],
    ..BASE
};

// ---------------------------------------------------------------- CORPORATE

const CO_MAJ: &[&str] = &[
    "I V vi IV I V vi IV",
    "IV I V vi IV I V V",
    "I iii IV V I iii IV V",
    "vi IV I V vi IV I V",
    "I Isus4 vi IV I V^1 IV IV",
];
const CO_LAYERS: [u32; 8] = [
    L_KEYS | L_ARP,
    KIT | L_BASS | L_KEYS | L_ARP | L_LEAD,
    FULL | L_BASS | L_KEYS | L_ARP | L_LEAD | L_PAD | L_CTR,
    L_KEYS | L_ARP | L_PAD | L_KICK,
    FULL | L_BASS | L_KEYS | L_ARP | L_PAD,
    0,
    0,
    L_KEYS | L_ARP | L_BASS,
];

/// Uplifting: the piano's eighths, a pluck's syncopations, the claps, the
/// glockenspiel's hook doubled, the strings in the chorus.
const CO_UPLIFT: Style = Style {
    meter: "4/4 uplifting",
    bpm: (110.0, 124.0),
    modes: [8.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.0],
    maj: CO_MAJ,
    min: &["vi IV I V vi IV I V"],
    cadence: ("IV V I I", "IV V vi vi"),
    layers: CO_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-"),
            (CH_PERC, SHAKER, "..x...x...x...x."),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_ROOT8,
    comp: CP_BLOCK8,
    arp: AR_PLUCK,
    tune: TU_HOOK,
    second: SV_OCTAVE,
    density: (0.45, 0.5),
    sound: Sound {
        tones: &[
            (CH_KEYS, piano, 0.28, -0.15, 0.2, 0.0),
            (CH_ARP, pizzicato, 0.22, 0.3, 0.15, 0.1),
            (CH_BASS, ska_bass_flat, 0.4, 0.0, 0.03, 0.0),
            (CH_LEAD, glockenspiel, 0.26, 0.1, 0.2, 0.1),
            (CH_BELL, glockenspiel, 0.16, -0.2, 0.2, 0.1),
            (CH_PAD, pad_strings, 0.16, 0.0, 0.28, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.85, 0.0, 0.03),
            (CH_SNARE, kit_snare_clap, 0.65, 0.0, 0.15),
            (CH_HAT, kit_shaker, 0.5, 0.25, 0.05),
            (CH_PERC, ska_kit_toms, 0.45, -0.25, 0.1),
            (CH_CYM, kit_cymbals, 0.22, -0.2, 0.15),
        ],
        room: (0.8, 0.24),
        echo: (3.0, 0.25),
        alts: &[
            (CH_ARP, &[("pizzicato strings", pizzicato), ("marimba", marimba), ("steel-string guitar", steel_guitar)]),
            (CH_LEAD, &[("glockenspiel", glockenspiel), ("whistle", whistle), ("celesta", celesta)]),
        ],
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -6.0),
        (CH_SNARE, 1.9),
        (CH_HAT, 2.2),
        (CH_PERC, 7.1),
        (CH_BASS, 2.9),
        (CH_PAD, 8.4),
        (CH_ARP, 7.3),
        (CH_LEAD, 9.6),
        (CH_KEYS, 3.4),
    ],
    chorus: &[(CH_PAD, 0.5)],
    ..BASE
};

/// Tech: four on the floor, claps, a synth's plucked 16ths, the pump.
const CO_TECH: Style = Style {
    meter: "4/4 tech",
    bpm: (118.0, 126.0),
    modes: [6.0, 0.0, 0.0, 0.5, 0.5, 1.0, 0.0],
    maj: &["vi IV I V vi IV I V", "I V vi IV I V vi IV", "IV V vi I IV V vi vi"],
    min: &["i bVI bIII bVII i bVI bIII bVII"],
    cadence: ("IV V I I", "bVI bVII i i"),
    layers: CO_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_C, "x-x-x-x-x-x-x-x-"),
            (CH_HAT, HAT_O, "..x...x...x...x."),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_OFFBEAT,
    comp: CP_STABS,
    arp: AR_8,
    tune: TU_HOOK,
    density: (0.45, 0.5),
    sweep: true,
    pump: 0.3,
    sound: Sound {
        tones: &[
            (CH_KEYS, stab_saw, 0.2, -0.2, 0.2, 0.1),
            (CH_ARP, arp_pluck, 0.22, 0.3, 0.2, 0.2),
            (CH_BASS, bass_moog, 0.42, 0.0, 0.0, 0.0),
            (CH_LEAD, pluck_hook, 0.28, 0.1, 0.22, 0.15),
            (CH_PAD, pad_warm, 0.16, 0.0, 0.3, 0.0),
        ],
        kits: TE_KITS,
        room: (0.82, 0.24),
        echo: (3.0, 0.3),
        ..BASE.sound
    },
    balance: &[
        (CH_KICK, -8.0),
        (CH_HAT, 0.7),
        (CH_BASS, 11.7),
        (CH_PAD, 10.8),
        (CH_ARP, 6.7),
        (CH_LEAD, 10.0),
        (CH_KEYS, 12.0),
    ],
    chorus: &[(CH_LEAD, 0.4), (CH_PAD, 0.6)],
    ..BASE
};

/// Acoustic: the steel-string strummed, a mandolin picking, the claps,
/// a whistled hook, the glockenspiel an octave up.
const CO_ACOUSTIC: Style = Style {
    meter: "4/4 acoustic",
    human: 0.009,
    bpm: (96.0, 112.0),
    modes: [8.0, 0.0, 0.0, 0.5, 1.0, 0.5, 0.0],
    maj: CO_MAJ,
    min: &["vi IV I V vi IV I V"],
    cadence: ("IV V I I", "IV V vi vi"),
    layers: CO_LAYERS,
    grooves: [
        &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 1, "....x.......x...")],
        &[
            (CH_KICK, 0, "x.......x.x....."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-."),
            (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x."),
        ],
    ],
    fill: FI_NONE,
    bass: BS_ROOT4,
    comp: CP_STRUM,
    arp: AR_8,
    tune: TU_HOOK,
    second: SV_OCTAVE,
    sound: Sound {
        tones: &[
            (CH_KEYS, steel_guitar, 0.3, -0.2, 0.15, 0.0),
            (CH_ARP, cittern, 0.2, 0.3, 0.15, 0.0),
            (CH_BASS, bass_upright, 0.4, 0.0, 0.05, 0.0),
            (CH_LEAD, whistle, 0.26, 0.1, 0.2, 0.0),
            (CH_BELL, glockenspiel, 0.16, -0.2, 0.2, 0.0),
        ],
        kits: &[
            (CH_KICK, kit_kick_soft, 0.8, 0.0, 0.05),
            (CH_SNARE, kit_snare_clap, 0.65, 0.0, 0.18),
            (CH_HAT, kit_shaker, 0.45, 0.25, 0.05),
            (CH_PERC, ska_kit_toms, 0.45, -0.25, 0.1),
        ],
        room: (0.8, 0.24),
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.8)],
    balance: &[(CH_KICK, -6.2), (CH_SNARE, 12.0), (CH_PERC, 2.2), (CH_BASS, 4.2), (CH_ARP, 6.2), (CH_LEAD, 5.4)],
    ..BASE
};

// ---------------------------------------------------------------- WORSHIP

const WO_MAJ: &[&str] = &[
    "I V vi IV I V vi IV",
    "IV I V vi IV I V V",
    "I V^1 vi IV I V^1 IV IV",
    "vi IV I V vi IV I V",
    "I IV vi V I IV V V",
];
const WO_KITS: &[KitRow] = &[
    (CH_KICK, kit_kick_soft, 0.8, 0.0, 0.1),
    (CH_SNARE, kit_snare_rim, 0.7, 0.05, 0.25),
    (CH_HAT, ska_kit_hats, 0.36, 0.25, 0.1),
    (CH_PERC, ska_kit_toms, 0.5, -0.2, 0.2),
    (CH_CYM, kit_cymbals, 0.28, -0.2, 0.25),
];
const WO_SOUND: Sound = Sound {
    tones: &[
        (CH_KEYS, piano, 0.26, -0.15, 0.3, 0.0),
        (CH_LEAD, piano, 0.3, 0.1, 0.3, 0.0),
        (CH_PAD, pad_shimmer, 0.2, 0.0, 0.45, 0.0),
        (CH_ARP, elec_clean, 0.18, 0.35, 0.3, 0.45),
        (CH_BASS, ska_bass_flat, 0.4, 0.0, 0.05, 0.0),
    ],
    kits: WO_KITS,
    room: (0.88, 0.34),
    echo: (3.0, 0.45),
    alts: &[
        (CH_LEAD, &[("piano", piano), ("cello", sw_cellos), ("strings", pad_strings)]),
        (CH_PAD, &[("shimmer", pad_shimmer), ("warm pad", pad_warm), ("strings", pad_strings)]),
    ],
    ..BASE.sound
};

/// The ballad that builds: the piano and the pad alone, the tune, the
/// band coming in at the chorus, the guitar's dotted-eighth echoes.
const WO_BALLAD: Style = Style {
    meter: "4/4 worship ballad",
    human: 0.01,
    bpm: (66.0, 76.0),
    modes: [8.0, 0.0, 0.0, 0.3, 0.5, 1.0, 0.0],
    maj: WO_MAJ,
    min: &["i bVI bIII bVII i bVI bIII bVII"],
    cadence: ("IV V I I", "bVI bVII i i"),
    layers: [
        L_PAD | L_KEYS,
        L_PAD | L_KEYS | L_LEAD | L_BASS,
        FULL | L_PAD | L_KEYS | L_LEAD | L_BASS | L_ARP,
        L_PAD | L_KEYS | L_ARP | L_KICK | L_BASS,
        FULL | L_PAD | L_KEYS | L_ARP | L_BASS,
        0,
        0,
        L_PAD | L_KEYS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x......."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[(CH_KICK, 0, "x.....x.x......."), (CH_SNARE, 0, "....X.......X..."), (CH_CYM, 1, "x...x...x...x...")],
    ],
    bass: BS_ROOT4,
    comp: CP_ARP8,
    arp: AR_SPARSE,
    density: (0.34, 0.42),
    end_hit: false,
    sound: WO_SOUND,
    double: &[(CH_ARP, 0.6)],
    balance: &[(CH_KICK, -4.7), (CH_PERC, 5.5), (CH_PAD, 0.5), (CH_ARP, 7.5), (CH_LEAD, 7.2), (CH_KEYS, 6.8)],
    chorus: &[(CH_PAD, 0.5)],
    ..BASE
};

/// A hymn in 6/8: the piano's eighths, the organ, the tune, the band on
/// the chorus.
const WO_HYMN: Style = Style {
    meter: "6/8 hymn",
    human: 0.012,
    bs: 12,
    beat: 6,
    bpm: (50.0, 62.0),
    modes: [8.0, 0.0, 0.0, 0.0, 0.5, 1.0, 0.0],
    maj: &["I IV I V I IV V I", "I vi IV V I IV I^2+V I", "I V vi IV I IV V I"],
    min: &["i iv i V i iv V i"],
    cadence: ("IV V I I", "iv V i i"),
    layers: [
        L_KEYS,
        L_KEYS | L_LEAD | L_PAD,
        FULL | L_KEYS | L_LEAD | L_PAD | L_BASS,
        L_KEYS | L_PAD | L_BASS,
        FULL | L_KEYS | L_PAD | L_BASS,
        0,
        0,
        L_KEYS | L_PAD,
    ],
    grooves: [
        &[(CH_KICK, 0, "x..........."), (CH_SNARE, 1, "......x.....")],
        &[(CH_KICK, 0, "x.....x....."), (CH_SNARE, 0, "......x....."), (CH_HAT, HAT_C, "x.x.x.x.x.x.")],
    ],
    bass: BS_HOLD,
    comp: CP_ARP8,
    tune: TU_CLASSIC,
    voice: (72, -2, 8),
    density: (0.4, 0.45),
    end_hit: false,
    sound: Sound {
        tones: &[
            (CH_KEYS, piano, 0.28, -0.15, 0.3, 0.0),
            (CH_LEAD, pad_strings, 0.28, 0.1, 0.3, 0.0),
            (CH_PAD, church_organ, 0.16, 0.0, 0.35, 0.0),
            (CH_BASS, sw_cellos, 0.36, 0.0, 0.2, 0.0),
        ],
        ..WO_SOUND
    },
    balance: &[
        (CH_KICK, -5.5),
        (CH_SNARE, 1.9),
        (CH_HAT, 0.6),
        (CH_PERC, 1.0),
        (CH_BASS, 2.2),
        (CH_LEAD, 7.1),
        (CH_KEYS, 4.8),
    ],
    ..BASE
};

/// The anthem: the kick in eighths through the chorus, the bass driving,
/// the piano's eighths, the delayed guitar.
const WO_ANTHEM: Style = Style {
    meter: "4/4 worship anthem",
    human: 0.009,
    bpm: (70.0, 80.0),
    modes: [8.0, 0.0, 0.0, 0.3, 0.5, 1.0, 0.0],
    maj: WO_MAJ,
    min: &["i bVI bIII bVII i bVI bIII bVII"],
    cadence: ("IV V I I", "bVI bVII i i"),
    layers: [
        L_PAD | L_ARP,
        L_PAD | L_KEYS | L_LEAD | L_BASS | L_KICK,
        FULL | L_PAD | L_KEYS | L_LEAD | L_BASS | L_ARP,
        L_PAD | L_ARP | L_KICK | L_BASS,
        FULL | L_PAD | L_KEYS | L_ARP | L_BASS,
        0,
        0,
        L_PAD | L_ARP | L_KEYS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x...x...x...x..."), (CH_SNARE, 1, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[(CH_KICK, 0, "x.-.x.-.x.-.x.-."), (CH_SNARE, 0, "....X.......X..."), (CH_CYM, 1, "x.x.x.x.x.x.x.x.")],
    ],
    bass: BS_ROOT8,
    comp: CP_BLOCK8,
    arp: AR_SPARSE,
    density: (0.36, 0.44),
    end_hit: false,
    sound: WO_SOUND,
    double: &[(CH_ARP, 0.6)],
    balance: &[
        (CH_KICK, -5.5),
        (CH_SNARE, 2.7),
        (CH_PERC, 8.4),
        (CH_BASS, 2.4),
        (CH_PAD, 3.3),
        (CH_ARP, 9.7),
        (CH_LEAD, 8.3),
        (CH_KEYS, 5.7),
    ],
    chorus: &[(CH_PAD, 0.5)],
    ..BASE
};

// ---------------------------------------------------------------- PRAISE

const PR_KITS: &[KitRow] = &[
    (CH_KICK, kit_kick_soft, 0.8, 0.0, 0.05),
    (CH_SNARE, kit_snare_clap, 0.75, 0.05, 0.16),
    (CH_HAT, ska_kit_hats, 0.38, 0.25, 0.05),
    (CH_PERC, ska_kit_toms, 0.5, -0.3, 0.12),
    (CH_CYM, kit_cymbals, 0.25, -0.2, 0.15),
];
const PR_SOUND: Sound = Sound {
    tones: &[
        (CH_KEYS, piano, 0.28, -0.2, 0.18, 0.0),
        (CH_PAD, pad_organ, 0.18, 0.15, 0.2, 0.0),
        (CH_BASS, ska_bass_flat, 0.46, 0.0, 0.03, 0.0),
        (CH_LEAD, ska_hammond, 0.3, 0.2, 0.2, 0.0),
        (CH_BELL, pad_choir, 0.22, -0.2, 0.3, 0.0),
    ],
    kits: PR_KITS,
    room: (0.82, 0.26),
    alts: &[(CH_LEAD, &[("organ", ska_hammond), ("trumpet", trumpet), ("tenor sax", sax), ("piano", piano)])],
    ..BASE.sound
};

/// Gospel: the piano's chords (the next pushed early), the organ, the
/// bass's runs, the choir's stabs, the tambourine and the claps.
const PR_GOSPEL: Style = Style {
    meter: "4/4 gospel",
    human: 0.009,
    bpm: (100.0, 120.0),
    swing: (0.08, 0.16),
    modes: [6.0, 0.0, 0.0, 0.0, 2.0, 1.0, 0.0],
    maj: &[
        "I IV^2 I I IV iv I V7",
        "I I7 IV iv I vi ii7 V7",
        "I iii7 vi7 V7/V ii7 V7 I I",
        "IV V^1 iii7 vi7 ii7 V7 I I7",
    ],
    min: &["i iv7 bVII7 bIIImaj7 bVImaj7 iim7b5 V7 i"],
    cadence: ("IV iv I I", "iv V7 i i"),
    layers: [
        L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD | L_CTR,
        L_KICK | L_HAT | L_BASS | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS | L_PAD,
    ],
    grooves: [
        &[
            (CH_KICK, 0, "x.......x.x....."),
            (CH_SNARE, 0, "....x.......x..."),
            (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-."),
            (CH_PERC, SHAKER, "..x...x...x...x."),
        ],
        &[
            (CH_KICK, 0, "x.....x.x.x....."),
            (CH_SNARE, 0, "....X.......X..."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-."),
            (CH_PERC, SHAKER, "x.x.x.x.x.x.x.x."),
        ],
    ],
    bass: BS_MOTOWN,
    comp: CP_GOSPEL,
    second: SV_STABS,
    density: (0.42, 0.5),
    sound: PR_SOUND,
    balance: &[
        (CH_KICK, -5.0),
        (CH_HAT, 2.4),
        (CH_PERC, 1.4),
        (CH_BASS, 3.0),
        (CH_PAD, 6.0),
        (CH_LEAD, 7.5),
        (CH_KEYS, 6.2),
    ],
    ..BASE
};

/// A 12/8 praise shuffle: slow, the piano's triplets, the organ, the
/// backbeat with claps, the choir.
const PR_SHUFFLE: Style = Style {
    meter: "12/8 praise",
    human: 0.01,
    bs: 24,
    beat: 6,
    bpm: (60.0, 76.0),
    modes: [6.0, 0.0, 0.0, 0.0, 2.0, 1.0, 0.0],
    maj: &["I IV I I IV IV I V7", "I I7 IV iv I V7 I I", "I vi IV V7 I IV I V7"],
    min: &["i iv i i iv iv i V7"],
    cadence: ("IV iv I I", "iv V7 i i"),
    layers: [
        L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD | L_CTR,
        L_KICK | L_HAT | L_BASS | L_KEYS | L_PAD,
        FULL | L_BASS | L_KEYS | L_PAD | L_LEAD,
        0,
        0,
        FULL | L_BASS | L_KEYS | L_PAD,
    ],
    grooves: [
        &[
            (CH_KICK, 0, "x.....x.....x.....x....."),
            (CH_SNARE, 0, "......x...........x....."),
            (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x.x.x.x.x."),
        ],
        &[
            (CH_KICK, 0, "x.....x.....x.....x....."),
            (CH_SNARE, 0, "......X...........X....."),
            (CH_SNARE, 1, "......x...........x....."),
            (CH_HAT, HAT_C, "x.x.x.x.x.x.x.x.x.x.x.x."),
            (CH_PERC, SHAKER, "......x...........x....."),
        ],
    ],
    bass: BS_ROOT4,
    comp: CP_ARP8,
    second: SV_GUIDE,
    density: (0.36, 0.42),
    sound: PR_SOUND,
    balance: &[
        (CH_KICK, -5.0),
        (CH_SNARE, 0.7),
        (CH_PERC, 4.1),
        (CH_BASS, 2.1),
        (CH_PAD, 5.8),
        (CH_LEAD, 7.1),
        (CH_BELL, 7.9),
        (CH_KEYS, 8.2),
    ],
    ..BASE
};

/// Contemporary praise: four on the floor in the chorus, the guitar
/// strummed and an electric's echoes, the bass in eighths, claps.
const PR_MODERN: Style = Style {
    meter: "4/4 praise",
    human: 0.008,
    bpm: (124.0, 138.0),
    modes: [8.0, 0.0, 0.0, 0.3, 1.0, 0.5, 0.0],
    maj: &["I V vi IV I V vi IV", "IV I V vi IV I V V", "I IV vi V I IV V V"],
    min: &["vi IV I V vi IV I V"],
    cadence: ("IV V I I", "IV V vi vi"),
    layers: [
        L_KEYS | L_ARP,
        KIT | L_BASS | L_KEYS | L_LEAD,
        FULL | L_BASS | L_KEYS | L_LEAD | L_ARP | L_PAD,
        L_KICK | L_KEYS | L_ARP | L_PAD,
        FULL | L_BASS | L_KEYS | L_ARP | L_PAD,
        0,
        0,
        KIT | L_BASS | L_KEYS,
    ],
    grooves: [
        &[(CH_KICK, 0, "x.......x.x....."), (CH_SNARE, 0, "....x.......x..."), (CH_HAT, HAT_C, "x.-.x.-.x.-.x.-.")],
        &[
            (CH_KICK, 0, "x...x...x...x..."),
            (CH_SNARE, 0, "....X.......X..."),
            (CH_SNARE, 1, "....x.......x..."),
            (CH_HAT, HAT_O, "..x...x...x...x."),
        ],
    ],
    fill: FI_SNARE,
    bass: BS_ROOT8,
    comp: CP_STRUM,
    arp: AR_SPARSE,
    density: (0.4, 0.48),
    sound: Sound {
        tones: &[
            (CH_KEYS, steel_guitar, 0.28, -0.25, 0.18, 0.0),
            (CH_ARP, elec_clean, 0.18, 0.35, 0.25, 0.4),
            (CH_BASS, bass_pick_dark, 0.44, 0.0, 0.03, 0.0),
            (CH_LEAD, lead_saw, 0.26, 0.1, 0.22, 0.1),
            (CH_PAD, pad_warm, 0.16, 0.0, 0.35, 0.0),
        ],
        kits: PR_KITS,
        room: (0.84, 0.28),
        echo: (3.0, 0.4),
        alts: &[(CH_LEAD, &[("saw lead", lead_saw), ("piano", piano), ("clean guitar", elec_clean)])],
        ..BASE.sound
    },
    double: &[(CH_KEYS, 0.8), (CH_ARP, 0.6)],
    balance: &[
        (CH_KICK, -6.4),
        (CH_SNARE, -2.0),
        (CH_HAT, 1.0),
        (CH_BASS, 4.5),
        (CH_PAD, 5.9),
        (CH_ARP, 8.8),
        (CH_LEAD, 10.3),
        (CH_KEYS, 3.0),
    ],
    chorus: &[(CH_PAD, 0.6)],
    ..BASE
};

// ---------------------------------------------------------------- the genres

/// Genre `g`'s styles, weighted.
pub(super) fn styles_of(g: usize) -> &'static [(f32, Style)] {
    const CLASSICAL_: &[(f32, Style)] = &[(1.2, CL_SONATA), (1.0, CL_MINUET), (1.0, CL_BAROQUE)];
    const ORCHESTRA_: &[(f32, Style)] = &[(1.4, OR_SYMPHONY), (1.0, OR_WALTZ), (0.8, OR_ADAGIO)];
    const ROCK_: &[(f32, Style)] = &[(1.2, RK_HARD), (1.2, RK_ANTHEM), (0.8, RK_BLUES)];
    const SIXTIES_: &[(f32, Style)] = &[(1.4, SX_SOUL), (0.9, SX_SURF), (1.0, SX_BEAT)];
    const SEVENTIES_: &[(f32, Style)] = &[(1.3, SV_DISCO), (1.0, SV_FUNK), (0.9, SV_SOFT)];
    const EIGHTIES_: &[(f32, Style)] = &[(1.4, EI_SYNTHPOP), (0.9, EI_BALLAD), (1.0, EI_NEWWAVE)];
    const NINETIES_: &[(f32, Style)] = &[(1.2, NI_EURO), (1.0, NI_GRUNGE), (1.0, NI_RNB)];
    const NOUGHTIES_: &[(f32, Style)] = &[(1.0, NO_PUNK), (1.0, NO_RNB), (1.0, NO_ELECTRO)];
    const TENS_: &[(f32, Style)] = &[(1.0, TE_EDM), (1.0, TE_TROPICAL), (1.0, TE_TRAP), (0.8, TE_STOMP)];
    const TWENTIES_: &[(f32, Style)] = &[(1.0, TW_NUDISCO), (1.0, TW_AMAPIANO), (1.0, TW_AFRO)];
    const KENYAN_: &[(f32, Style)] = &[(1.5, KE_BENGA), (1.0, KE_RUMBA), (0.8, KE_NYATITI)];
    const YOGA_: &[(f32, Style)] = &[(1.3, YO_MEDITATION), (1.0, YO_FLOW), (0.8, YO_KIRTAN)];
    const ELEVATOR_: &[(f32, Style)] = &[(1.2, EL_BOSSA), (1.0, EL_LOUNGE), (1.0, EL_SMOOTH)];
    const CORPORATE_: &[(f32, Style)] = &[(1.4, CO_UPLIFT), (1.0, CO_TECH), (1.0, CO_ACOUSTIC)];
    const WORSHIP_: &[(f32, Style)] = &[(1.4, WO_BALLAD), (0.8, WO_HYMN), (1.0, WO_ANTHEM)];
    const PRAISE_: &[(f32, Style)] = &[(1.3, PR_GOSPEL), (0.8, PR_SHUFFLE), (1.0, PR_MODERN)];
    match g {
        CLASSICAL => CLASSICAL_,
        ORCHESTRA => ORCHESTRA_,
        ROCK => ROCK_,
        SIXTIES => SIXTIES_,
        SEVENTIES => SEVENTIES_,
        EIGHTIES => EIGHTIES_,
        NINETIES => NINETIES_,
        NOUGHTIES => NOUGHTIES_,
        TENS => TENS_,
        TWENTIES => TWENTIES_,
        KENYAN => KENYAN_,
        YOGA => YOGA_,
        ELEVATOR => ELEVATOR_,
        CORPORATE => CORPORATE_,
        WORSHIP => WORSHIP_,
        _ => PRAISE_,
    }
}

/// Each styled genre's room (the reverb's size).
pub(super) fn room_styled(g: usize) -> f32 {
    match g {
        ORCHESTRA | WORSHIP | YOGA => 1.3,
        CLASSICAL | PRAISE => 1.05,
        ROCK | SIXTIES | NOUGHTIES | KENYAN => 0.85,
        _ => 1.0,
    }
}

/// Each styled genre's tempo, about (for a mix's: 60 % of the way to its drummer's).
pub(super) const MID_BPM_STYLED: [f64; NG - CLASSICAL] =
    [110.0, 100.0, 120.0, 135.0, 112.0, 115.0, 115.0, 130.0, 118.0, 112.0, 132.0, 78.0, 110.0, 116.0, 72.0, 110.0];

/// The genre's instruments for the part channel `ch` plays in a mix: those
/// of its styles' (the first style's first).
pub(super) fn palette_styled(g: usize, ch: usize) -> Option<&'static [(&'static str, Make)]> {
    let pal: &[(usize, Pal)] = match g {
        CLASSICAL => &[
            (CH_LEAD, &[("piano", piano), ("violin", fiddle), ("oboe", sw_oboe)]),
            (CH_KEYS, &[("piano", piano), ("harpsichord", harpsichord)]),
            (CH_BASS, &[("cellos", sw_cellos)]),
            (CH_BELL, &[("violin", fiddle), ("flute", flute)]),
        ],
        ORCHESTRA => &[
            (CH_LEAD, &[("violins", sw_violins), ("horns", sw_horns), ("flute", flute)]),
            (CH_PAD, &[("strings", sw_ensemble), ("brass", sw_brass)]),
            (CH_BASS, &[("cellos", sw_cellos), ("contrabass", sw_contrabass)]),
            (CH_ARP, &[("harp", harp)]),
            (CH_BELL, &[("horns", sw_horns), ("clarinet", clarinet)]),
            (CH_KEYS, &[("brass", sw_brass), ("violas", sw_violas_spic)]),
        ],
        ROCK | NOUGHTIES => &[
            (CH_LEAD, &[("overdriven guitar", ska_guitar_fuzz), ("distorted guitar", ska_guitar_dist)]),
            (CH_KEYS, &[("distorted guitar", ska_guitar_dist), ("organ", ska_organ_rec)]),
            (CH_BASS, &[("picked bass", bass_pick_dark)]),
            (CH_PAD, &[("organ", ska_organ_rec)]),
        ],
        SIXTIES => &[
            (CH_LEAD, &[("tenor sax", sax), ("harmonica", harmonica), ("surf guitar", surf_guitar)]),
            (CH_KEYS, &[("piano", piano), ("twelve-string", twelve_string), ("organ", ska_farfisa)]),
            (CH_BASS, &[("bass", ska_bass_flat)]),
            (CH_BELL, &[("horns", brass_section), ("vibraphone", vibes)]),
            (CH_PAD, &[("strings", pad_strings)]),
        ],
        SEVENTIES => &[
            (CH_LEAD, &[("strings", strings_fast), ("tenor sax", sax), ("flute", flute)]),
            (CH_KEYS, &[("clavinet", clav), ("funk guitar", ska_guitar_clean), ("Rhodes", ep_rhodes)]),
            (CH_BASS, &[("bass", ska_bass_flat)]),
            (CH_BELL, &[("horns", brass_section)]),
            (CH_PAD, &[("strings", pad_strings)]),
        ],
        EIGHTIES => &[
            (CH_LEAD, &[("square lead", lead_square), ("alto sax", ska_alto)]),
            (CH_KEYS, &[("FM keys", glass_fm), ("piano", piano)]),
            (CH_BASS, &[("sequenced bass", bass_seq), ("FM bass", bass_fm)]),
            (CH_PAD, &[("polysynth", hoover)]),
            (CH_ARP, &[("pluck", arp_pluck)]),
        ],
        NINETIES => &[
            (CH_LEAD, &[("saw lead", lead_saw), ("whistle", sine_lead)]),
            (CH_KEYS, &[("house piano", bright_piano), ("Rhodes", ep_rhodes)]),
            (CH_BASS, &[("sequenced bass", bass_seq), ("synth bass", bass_synth)]),
            (CH_PAD, &[("warm pad", pad_warm), ("synth strings", synth_strings)]),
        ],
        TENS => &[
            (CH_LEAD, &[("saw lead", lead_saw), ("pan flute", pan_flute), ("bell", bell_fm)]),
            (CH_KEYS, &[("supersaw", hoover), ("piano", piano)]),
            (CH_BASS, &[("808", sub808), ("sub", bass_sub)]),
            (CH_ARP, &[("marimba", marimba), ("pluck", arp_pluck)]),
            (CH_PAD, &[("warm pad", pad_warm)]),
        ],
        TWENTIES => &[
            (CH_LEAD, &[("saw lead", lead_saw), ("flute", flute)]),
            (CH_KEYS, &[("piano", piano), ("polysynth", hoover)]),
            (CH_BASS, &[("log drum", log_drum), ("synth bass", bass_moog)]),
            (CH_PAD, &[("dark pad", pad_dark)]),
        ],
        KENYAN => &[
            (CH_LEAD, &[("lead guitar", ska_guitar_clean), ("orutu", fiddle)]),
            (CH_BELL, &[("second guitar", elec_clean)]),
            (CH_KEYS, &[("rhythm guitar", jazz_guitar)]),
            (CH_ARP, &[("nyatiti", nyatiti)]),
            (CH_BASS, &[("bass", ska_bass_flat)]),
        ],
        YOGA => &[
            (CH_LEAD, &[("bansuri", bansuri), ("pan flute", pan_flute)]),
            (CH_DRONE, &[("tanpura", tanpura)]),
            (CH_PAD, &[("shimmer", pad_shimmer)]),
            (CH_ARP, &[("harp", harp), ("kalimba", kalimba)]),
            (CH_BELL, &[("singing bowl", singing_bowl)]),
            (CH_KEYS, &[("harmonium", harmonium)]),
        ],
        ELEVATOR => &[
            (CH_LEAD, &[("flute", flute), ("vibraphone", vibes), ("alto sax", ska_alto)]),
            (CH_KEYS, &[("nylon guitar", nylon), ("Rhodes", ep_rhodes), ("piano", piano)]),
            (CH_BASS, &[("upright bass", bass_upright)]),
            (CH_PAD, &[("strings", pad_strings)]),
        ],
        CORPORATE => &[
            (CH_LEAD, &[("glockenspiel", glockenspiel), ("whistle", whistle)]),
            (CH_KEYS, &[("piano", piano), ("steel-string guitar", steel_guitar)]),
            (CH_ARP, &[("pizzicato strings", pizzicato), ("marimba", marimba)]),
            (CH_BASS, &[("bass", ska_bass_flat)]),
            (CH_PAD, &[("strings", pad_strings)]),
        ],
        WORSHIP => &[
            (CH_LEAD, &[("piano", piano), ("strings", pad_strings)]),
            (CH_KEYS, &[("piano", piano)]),
            (CH_PAD, &[("shimmer", pad_shimmer), ("organ", church_organ)]),
            (CH_ARP, &[("delayed guitar", elec_clean)]),
        ],
        _ => &[
            (CH_LEAD, &[("organ", ska_hammond), ("trumpet", trumpet)]),
            (CH_KEYS, &[("piano", piano)]),
            (CH_PAD, &[("organ", pad_organ)]),
            (CH_BELL, &[("choir", pad_choir)]),
            (CH_BASS, &[("bass", ska_bass_flat)]),
        ],
    };
    pal.iter().find(|x| x.0 == ch).map(|x| x.1)
}

/// The genre's kits for drum channel `ch`, for a mix.
pub(super) fn kit_palette_styled(g: usize, ch: usize) -> Option<&'static [(&'static str, MakeKit)]> {
    let kits: &[(usize, KitPal)] = match g {
        CLASSICAL | ORCHESTRA => {
            &[(CH_KICK, &[("bass drum", sw_kit_bass_drum)]), (CH_HAT, &[("triangle", sw_kit_triangle)])]
        }
        ROCK | NOUGHTIES => {
            &[(CH_KICK, &[("rock kick", ska_kit_punk_kick)]), (CH_SNARE, &[("rock snare", ska_kit_punk_snare)])]
        }
        EIGHTIES => &[(CH_KICK, &[("Linn kick", kit_kick_linn)]), (CH_SNARE, &[("Linn snare", kit_snare_linn)])],
        NINETIES | TENS => &[(CH_KICK, &[("909 kick", kit_kick_909)]), (CH_HAT, &[("909 hats", kit_hats_909)])],
        YOGA => &[(CH_PERC, &[("tabla", kit_tabla)]), (CH_HAT, &[("manjira", kit_manjira)])],
        KENYAN => &[(CH_PERC, &[("kayamba", kit_shaker), ("ongeng'o", kit_ongengo)])],
        ELEVATOR => &[(CH_SNARE, &[("brushes", kit_brush), ("side stick", kit_bossa_snare)])],
        _ => &[(CH_SNARE, &[("snare and claps", kit_snare_clap)]), (CH_PERC, &[("tambourine and toms", ska_kit_toms)])],
    };
    kits.iter().find(|x| x.0 == ch).map(|x| x.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every style's progressions parse, its drums' lines are its bar's
    /// length, its form opens on an intro and ends on an outro, and both its
    /// grooves hit in every bar (a mix's drummer plays every main bar).
    #[test]
    fn styles_are_well_formed() {
        for g in CLASSICAL..NG {
            for (k, (_, st)) in styles_of(g).iter().enumerate() {
                let what = format!("{} style {k} ({})", GENRES[g], st.meter);
                for plan in st.maj.iter().chain(st.min.iter()).chain([st.cadence.0, st.cadence.1].iter()) {
                    for tok in plan.split_whitespace().flat_map(|t| t.split('+')) {
                        assert!(parse_harm(tok).is_some(), "{what}: {tok} in {plan}");
                    }
                }
                assert!(!st.maj.is_empty() || !st.min.is_empty(), "{what}: no progressions");
                assert_eq!(st.form.first().map(|r| r.0), Some(PA_INTRO), "{what}");
                assert_eq!(st.form.last().map(|r| (r.0, r.3)), Some((PA_OUTRO, 1.0)), "{what}");
                assert!(st.bs % st.beat == 0, "{what}: {} 16ths of {}", st.bs, st.beat);
                for gr in st.grooves {
                    assert!(
                        gr.iter().any(|l| l.2.bytes().any(|c| vel_of_test(c) > 0.0)),
                        "{what}: a groove with no hits"
                    );
                    for l in gr {
                        assert_eq!(l.2.len() as u32, st.bs, "{what}: {}", l.2);
                    }
                }
            }
        }
    }

    fn vel_of_test(c: u8) -> f32 {
        if c == b'.' { 0.0 } else { 1.0 }
    }

    /// What each style has more of (varied.rs): its name finds it and no
    /// two styles share a name; its rows parse and are as long as the
    /// style's own; its drum lines are a bar long, hit, and play only the
    /// drums the style's own lines play (its kit has them); its other ways
    /// are others, for parts the style has, and fit its meter; its other
    /// players are for parts the style has, and make a sound.
    #[test]
    fn the_variety_is_well_formed() {
        let mut names: Vec<&str> = Vec::new();
        for g in CLASSICAL..NG {
            for (k, (_, st)) in styles_of(g).iter().enumerate() {
                let what = format!("{} style {k} ({})", GENRES[g], st.meter);
                assert!(!names.contains(&st.meter), "{what}: a name twice");
                names.push(st.meter);
                let x = extra(st);
                let some = !x.maj.is_empty() || !x.min.is_empty() || !x.grooves.is_empty() || !x.alts.is_empty();
                assert!(some || !x.bass.is_empty() || !x.comp.is_empty() || !x.arp.is_empty(), "{what}: nothing more");
                let bars = |p: &str| p.split_whitespace().count();
                for (own, more) in [(st.maj, x.maj), (st.min, x.min)] {
                    for plan in more {
                        for tok in plan.split_whitespace().flat_map(|t| t.split('+')) {
                            assert!(parse_harm(tok).is_some(), "{what}: {tok} in {plan}");
                        }
                        assert!(!own.is_empty(), "{what}: more rows for a mode it has none of: {plan}");
                        assert_eq!(bars(plan), bars(own[0]), "{what}: {plan}");
                        assert!(!own.contains(plan), "{what}: {plan} twice");
                    }
                }
                // four rows at least for each of the modes it draws
                let w = st.modes;
                for (weight, own, more) in
                    [(w[0] + w[3] + w[4], st.maj, x.maj), (w[1] + w[2] + w[5] + w[6], st.min, x.min)]
                {
                    assert!(weight == 0.0 || own.is_empty() || own.len() + more.len() >= 4, "{what}: a short list");
                }
                let has = |ch: usize, kind: u8| st.grooves.iter().any(|gr| gr.iter().any(|l| (l.0, l.1) == (ch, kind)));
                for pair in x.grooves {
                    for gr in pair {
                        assert!(gr.iter().any(|l| l.2.bytes().any(|c| vel_of_test(c) > 0.0)), "{what}: no hits");
                        for l in gr.iter() {
                            assert_eq!(l.2.len() as u32, st.bs, "{what}: {}", l.2);
                            assert!(has(l.0, l.1), "{what}: a drum its own lines do not play: {l:?}");
                            assert!(l.2.bytes().all(|c| b".o-xX".contains(&c)), "{what}: {}", l.2);
                        }
                    }
                }
                for (own, more, none) in [
                    (st.bass, x.bass, BS_NONE),
                    (st.comp, x.comp, CP_NONE),
                    (st.arp, x.arp, AR_NONE),
                    (st.second, x.second, SV_NONE),
                ] {
                    assert!(!more.contains(&own) && !more.contains(&none), "{what}: {more:?}");
                    assert!(more.is_empty() || own != none, "{what}: ways for a part it has not: {more:?}");
                }
                if st.bs != 16 {
                    assert!(!x.bass.iter().any(|b| matches!(*b, BS_MOTOWN | BS_FUNK | BS_LOG | BS_BOSSA)), "{what}");
                    assert!(!x.comp.contains(&CP_BOSSA) && !x.arp.contains(&AR_PLUCK), "{what}");
                }
                let mut r = Rng::new(7);
                for &(ch, pal) in x.alts {
                    assert!(st.sound.tones.iter().any(|t| t.0 == ch), "{what}: no part on channel {ch}");
                    assert!(!pal.is_empty(), "{what}");
                    for (name, make) in pal {
                        let l = loudness(&make(&mut r), 60.0);
                        assert!(l.is_finite() && l > 0.0, "{what}: {name} is silent");
                    }
                }
            }
        }
        assert_eq!(names.len(), NAMED);
        // the lines of a kind: a bar long, and their first the plain one
        let r = &mut Rng::new(3);
        for (st, bpm) in
            [(&styles_of(ROCK)[0].1, 120.0), (&styles_of(ROCK)[2].1, 90.0), (&styles_of(SEVENTIES)[1].1, 100.0)]
        {
            for _ in 0..40 {
                for gr in draw_lanes(r, st, bpm) {
                    assert!(!gr.is_empty());
                    for l in gr {
                        assert_eq!(l.2.len() as u32, st.bs, "{}", l.2);
                    }
                }
            }
        }
    }

    /// Each style of each genre plays: twenty seconds of each, nothing
    /// silent for long, nothing past the limiter's ceiling.
    #[test]
    fn every_style_plays() {
        for g in CLASSICAL..NG {
            for k in 0..styles_of(g).len() {
                let mut seed = 0u64;
                let mut e = loop {
                    let e = Engine::new(g, seed);
                    if e.song.sy.style == k {
                        break e;
                    }
                    seed += 1;
                    assert!(seed < 400, "{} never drew its style {k}", GENRES[g]);
                };
                let mut out = vec![0.0f32; 2 * 20 * RATE as usize];
                for c in out.chunks_mut(2 * CHUNK) {
                    e.render(c);
                }
                assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 0.945), "{} style {k}", GENRES[g]);
                let rms = (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt();
                assert!(rms > 0.01, "{} style {k} seed {seed}: rms {rms}", GENRES[g]);
            }
        }
    }

    /// What a few songs of a genre written with more variety drew (the
    /// genre `MUSICBOX_VARIETY_GENRE` names, rock by default): each one's
    /// style, tempo, form, keys, ways and drum lines, printed.
    ///
    ///     cargo test --release variety_songs -- --ignored --nocapture
    #[test]
    #[ignore]
    fn variety_songs() {
        let name = std::env::var("MUSICBOX_VARIETY_GENRE").unwrap_or("rock".into());
        let g = find(&name).expect("a genre");
        let varied = Opts { varied: true, voices: Voices::Synth, ..Opts::default() };
        for seed in 1..=8u64 {
            let e = Engine::with(g, seed * 7919, varied);
            let (song, st) = (&e.song, e.song.style());
            let form: Vec<String> = song
                .plan
                .iter()
                .zip(&song.sy.parts)
                .zip(&song.lifts)
                .map(|((sec, part), lift)| {
                    let p = ["intro", "verse", "chorus", "bridge", "solo", "build", "drop", "outro"][*part as usize];
                    if *lift != 0 { format!("{p} {} (+{lift})", sec.bars) } else { format!("{p} {}", sec.bars) }
                })
                .collect();
            println!("seed {}: {} at {:.0} bpm: {}", seed * 7919, st.meter, song.bpm, form.join(", "));
            println!(
                "  ways (bass, chords, arpeggio, second): {:?}, the style's {:?}",
                song.sy.vary.ways,
                (st.bass, st.comp, st.arp, st.second)
            );
            for (k, side) in song.sy.vary.lanes.iter().enumerate() {
                for l in side {
                    let own = st.grooves[k].contains(l);
                    println!(
                        "  {} ch {} kind {} {}{}",
                        ["A", "B"][k],
                        l.0,
                        l.1,
                        l.2,
                        if own { "" } else { "   (not the style's own)" }
                    );
                }
            }
        }
    }

    /// Each style plays written with more variety: several pieces of each
    /// (whatever they draw), then each of its other ways and each of its
    /// other pairs of drum lines in turn, a whole piece each time: nothing
    /// silent, nothing past the limiter's ceiling, and the piece ends.
    #[test]
    fn every_style_plays_with_more_variety() {
        let varied = Opts { varied: true, voices: Voices::Synth, ..Opts::default() };
        let play = |e: &mut Engine, secs: usize, what: &str| {
            let mut out = vec![0.0f32; 2 * secs * RATE as usize];
            for c in out.chunks_mut(2 * CHUNK) {
                e.render(c);
            }
            assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 0.945), "{what}");
            let rms = (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt();
            assert!(rms > 0.01, "{what}: rms {rms}");
        };
        for g in CLASSICAL..NG {
            let mut drawn = vec![0usize; styles_of(g).len()];
            let mut seed = 0u64;
            while drawn.iter().any(|&n| n < 3) {
                let mut e = Engine::with(g, seed, varied);
                let k = e.song.sy.style;
                assert!(e.song.sy.vary.ways.is_some() && !e.song.sy.vary.lanes[0].is_empty(), "{}", GENRES[g]);
                if drawn[k] < 3 {
                    drawn[k] += 1;
                    play(&mut e, 30, &format!("{} style {k} seed {seed}", GENRES[g]));
                }
                seed += 1;
                assert!(seed < 600, "{} never drew each style three times", GENRES[g]);
            }
            for (k, (_, st)) in styles_of(g).iter().enumerate() {
                let x = extra(st);
                let own = (st.bass, st.comp, st.arp, st.second);
                let mut cases: Vec<((u8, u8, u8, u8), Option<usize>)> = vec![(own, None)];
                cases.extend(x.bass.iter().map(|&b| ((b, own.1, own.2, own.3), None)));
                cases.extend(x.comp.iter().map(|&c| ((own.0, c, own.2, own.3), None)));
                cases.extend(x.arp.iter().map(|&a| ((own.0, own.1, a, own.3), None)));
                cases.extend(x.second.iter().map(|&s| ((own.0, own.1, own.2, s), None)));
                cases.extend((0..x.grooves.len()).map(|i| (own, Some(i))));
                let mut seed = 1000u64;
                let mut e = loop {
                    let e = Engine::with(g, seed, varied);
                    if e.song.sy.style == k {
                        break e;
                    }
                    seed += 1;
                };
                for (ways, groove) in cases {
                    e.song.sy.vary.ways = Some(ways);
                    if let Some(i) = groove {
                        e.song.sy.vary.lanes = [x.grooves[i][0].to_vec(), x.grooves[i][1].to_vec()];
                    }
                    play(&mut e, 12, &format!("{} style {k}: ways {ways:?}, lines {groove:?}", GENRES[g]));
                }
            }
        }
    }
}
