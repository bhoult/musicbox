//! Recorded instruments for the music: multisampled WAVs listed in a
//! manifest, and a General MIDI SoundFont's presets and drum kits, loaded
//! once (never on the audio thread), held read-only and played by the
//! synthesiser's voices in place of their oscillators.
//!
//! The samples directory (`~/.local/share/musicbox/samples`, else fleece's
//! `~/.local/share/fleece/samples` when only that is there, or `--samples
//! DIR`; `tools/get-samples.sh` fills it) holds `MuseScore_General.sf2`,
//! and `manifest.tsv` with the WAVs it lists under `multi/`. Genre code asks
//! for an instrument by name when it builds a patch (`multi`, `gm`, `drum`)
//! and gets an id, or None (no samples, or no such instrument) and keeps its
//! synthesiser patch. A note on a sampled patch is then an index lookup: the
//! id's table of note x velocity layer gives the zones to play.
//!
//! Levels: every instrument is brought to one reference, its loudest layer
//! at -20 dBFS RMS over the first second of a middle note (the manifest's
//! `gain_db` does it for the WAVs, each root and velocity layer evened to
//! the instrument's level as well; `normalise` for the SoundFont's presets;
//! a kit keeps its maker's balance, brought to the reference at its snare),
//! and a sampled voice is then scaled by `CAL` (a drum by `CAL_DRUM`),
//! measured so that a sampled note sounds about as loud as the
//! synthesiser's at the same patch gain and velocity.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::{Once, OnceLock};
use std::time::Instant;

/// The engine's sample rate.
const RATE: f32 = 48000.0;

/// The SoundFont, the manifest, in the samples directory.
pub const SF2: &str = "MuseScore_General.sf2";
const MANIFEST: &str = "manifest.tsv";

/// The SoundFont's melodic presets loaded, by name (as its phdr has them,
/// bank 0, else the lowest melodic bank: Funk Guitar is bank 8's; never the
/// "Expr." variants, whose dynamics follow a controller). The pianos are
/// one set of samples (100 MB).
pub const GM_PRESETS: &[&str] = &[
    // guitars
    "Nylon String Guitar",
    "Steel String Guitar",
    "Jazz Guitar",
    "Clean Guitar",
    "Palm Muted Guitar",
    "Overdrive Guitar",
    "Distortion Guitar",
    "Funk Guitar",
    "Mandolin",
    "Banjo",
    // organs
    "Drawbar Organ",
    "Percussive Organ",
    "Rock Organ",
    "Church Organ",
    "Reed Organ",
    // pianos and keys
    "Grand Piano",
    "Bright Grand Piano",
    "Mellow Grand Piano",
    "Honky-Tonk Piano",
    "Tine Electric Piano",
    "Detuned Tine EP",
    "FM Electric Piano",
    "Harpsichord",
    "Clavinet",
    // mallets and bells
    "Celesta",
    "Glockenspiel",
    "Music Box",
    "Vibraphone",
    "Marimba",
    "Xylophone",
    "Tubular Bells",
    "Dulcimer",
    "Kalimba",
    "Tinker Bell",
    "Steel Drums",
    // free reeds
    "Accordion",
    "Italian Accordion",
    "Bandoneon",
    "Harmonica",
    // basses
    "Acoustic Bass",
    "Fingered Bass",
    "Picked Bass",
    "Fretless Bass",
    "Slap Bass",
    "Synth Bass 1",
    "Synth Bass 2",
    "Synth Bass 3",
    "Synth Bass 4",
    // saxes and reeds
    "Soprano Sax",
    "Alto Sax",
    "Tenor Sax",
    "Baritone Sax",
    "Oboe",
    "English Horn",
    "Bassoon",
    "Clarinet",
    "Bagpipe",
    "Shenai",
    // pipes
    "Piccolo",
    "Flute",
    "Recorder",
    "Pan Flute",
    "Whistle",
    "Ocarina",
    // brass
    "Trumpet",
    "Harmon Mute Trumpet",
    "Trombone",
    "Tuba",
    "French Horns",
    "Brass Section",
    "Synth Brass 1",
    "Synth Brass 2",
    // strings (General MIDI's String Ensembles 1 and 2 are "Strings Fast"
    // and "Strings Slow" here)
    "Violin",
    "Viola",
    "Cello",
    "Fiddle",
    "Strings Fast",
    "Strings Slow",
    "Strings Pizzicato",
    "Synth Strings 1",
    "Synth Strings 2",
    "Harp",
    "Contrabass",
    // voices
    "Choir Aahs",
    "Voice Oohs",
    "Synth Voice",
    "Orchestra Hit",
    // synthesisers: leads
    "Square Lead",
    "Saw Lead",
    "Calliope Lead",
    "Chiffer Lead",
    "Charang",
    "Solo Vox",
    "5th Saw Wave",
    "Bass & Lead",
    "Detuned Saw",
    "Sine Wave",
    // pads
    "Fantasia",
    "Warm Pad",
    "Polysynth",
    "Space Voice",
    "Bowed Glass",
    "Metal Pad",
    "Halo Pad",
    "Sweep Pad",
    // effects
    "Ice Rain",
    "Soundtrack",
    "Crystal",
    "Atmosphere",
    "Brightness",
    "Echo Drops",
    "Star Theme",
    // percussion played by note
    "Agogo",
    "Woodblock",
    "Taiko Drum",
    "Melodic Tom",
    "Synth Drum",
    "Castanets",
    "Concert Bass Drum",
    "808 Tom",
    "Timpani",
    // -- starconflict / western: the strings' tremolo, a cymbal's swell (a
    // reversed crash), a church bell
    "Strings Tremolo",
    "Reverse Cymbal",
    "Church Bell",
    // -- end starconflict / western
    // -- indonesian / celtic: the suling's stand-in (an end-blown bamboo
    // flute), the bouzouki's (a twelve-string's octave courses)
    "Shakuhachi",
    "12-String Guitar",
    // -- end indonesian / celtic
];

/// Its drum kits loaded (bank 128), by name.
pub const GM_KITS: &[&str] = &["Standard", "Room", "Power", "Jazz", "Brush", "Electronic", "TR-808", "Orchestra Kit"];

/// The reference every instrument is brought to: -20 dBFS RMS (full scale 1).
const REF_RMS: f32 = 0.1;

/// A sampled voice's level over the reference. Measured (the test
/// `calibration` in music.rs): the plain saw `PATCH` at gain 1, velocity
/// 0.8, note 60 has an RMS of 0.325 by `loudness` (30-150 ms), a sine at
/// the reference played the same way 0.08; so 0.325 / 0.08 (+12.2 dB).
pub const CAL: f32 = 4.06;

/// A sampled drum's level over the reference. Measured (the test
/// `calibration`): a dozen of the synthesiser's drums (two kicks, two
/// snares, a clap, closed and open hats, a shaker, a ride, a crash, a tom,
/// a conga) at gain 1, velocity 0.8, against MuseScore General's Standard
/// kit's keys of the same kinds (the kit brought to the reference at its
/// snare), RMS over the first second: 4.4 dB under the reference on
/// average (each kind within 7 dB of that).
pub const CAL_DRUM: f32 = 0.60;

/// Gates shorter than this (seconds) take the short articulation of an
/// "auto" instrument.
pub const AUTO_SHORT: f32 = 0.25;

/// The window (seconds from the onset) an "auto" instrument's short
/// articulation's level is taken over, to match the long one's (over its
/// first second, the reference's window, mostly steady): a short note's
/// momentary loudness (EBU R128's 400 ms).
const AUTO_MATCH: f32 = 0.4;

/// Zones sounding at once in one voice: a stereo pair (the SoundFont keeps
/// a stereo sample as two zones panned apart), or layers (the pianos have
/// two stereo pairs over some velocities).
const MAXZ: usize = 4;

/// 32.32 fixed point's one.
const FX: f64 = 4294967296.0;

/// The fade a sample that runs out ends with (10 ms, samples).
const END_FADE: f64 = 480.0;

/// The SoundFont's recordings that hold a note on a loop of a few cycles of
/// the wave (24 or fewer, after an attack of 30 ms or more: a synthesiser's
/// wave loops from its first milliseconds, and is meant to hold still), and
/// whether each is plucked or struck (it decays). Played over and over, so
/// short a loop holds the sound still, as no instrument does: the note is
/// kept alive (`Life`). Its pitch wanders by up to `DRIFT` cents, a new aim
/// every `DRIFT_EVERY` seconds reached in about `DRIFT_GLIDE`; a plucked or
/// struck one's highs die before its body, a low-pass falling from
/// `LIFE_OPEN` (Hz: the attack as recorded) to `LIFE_FLOOR` harmonics over a
/// time that shortens up the keyboard. By name, so that the browser's copy
/// (its loops played out longer) keeps the same notes alive.
const LIVELY: &[(&str, bool)] = &[
    ("Nylon String Guitar", true),
    ("Steel String Guitar", true),
    ("12-String Guitar", true),
    ("Jazz Guitar", true),
    ("Clean Guitar", true),
    ("Palm Muted Guitar", true),
    ("Funk Guitar", true),
    ("Mandolin", true),
    ("Banjo", true),
    ("Harp", true),
    ("Acoustic Bass", true),
    ("Fingered Bass", true),
    ("Picked Bass", true),
    ("Fretless Bass", true),
    ("Slap Bass", true),
    ("Tine Electric Piano", true),
    ("Detuned Tine EP", true),
    ("Harpsichord", true),
    ("Clavinet", true),
    ("Celesta", true),
    ("Music Box", true),
    ("Vibraphone", true),
    ("Kalimba", true),
    ("Overdrive Guitar", false),
    ("Distortion Guitar", false),
    ("Reed Organ", false),
    ("Bandoneon", false),
    ("Harmonica", false),
    ("Clarinet", false),
    ("Shenai", false),
    ("Whistle", false),
    ("Harmon Mute Trumpet", false),
    ("Trombone", false),
    ("Tuba", false),
];
const DRIFT: f32 = 3.0;
const DRIFT_EVERY: f32 = 0.35;
const DRIFT_GLIDE: f32 = 0.15;
const LIFE_OPEN: f32 = 8000.0;
const LIFE_FLOOR: f32 = 4.0;

/// A sustained recording ("sus", "vib") at least `LOOP_MIN` seconds long
/// loops `LOOP` (seconds from its onset) while the gate is open, the last
/// `LOOP_XF` of each time round crossfaded into the start's: the manifest's
/// stop at 5.5 s, faded, with no loop points of their own.
const LOOP_MIN: f32 = 5.0;
const LOOP: (f32, f32) = (1.5, 4.5);
const LOOP_XF: f32 = 0.25;

/// One sample as played over some keys and velocities.
#[derive(Clone, Copy, Debug)]
struct Zone {
    /// where its frames start in the bank's data, how many, and its
    /// channels (1, or 2 interleaved)
    off: usize,
    len: u32,
    chans: u8,
    /// the key that plays it at its own rate, tuning on top (semitones), and
    /// semitones per key (0: every key the same pitch)
    root: f32,
    tune: f32,
    scale: f32,
    /// its sample rate over the engine's
    rate: f32,
    /// the loop (frames from its start, the end exclusive) and its mode: 0
    /// none, 1 always, 3 until release; frames before its end crossfaded
    /// into those before its start (0: none, a SoundFont's loop is made to
    /// join)
    ls: u32,
    le: u32,
    mode: u8,
    xf: u32,
    /// the left and right gains (level, normalisation, pan, 1/32768)
    gl: f32,
    gr: f32,
    /// the SoundFont's exclusive class (0 none): a hit chokes its class
    excl: u8,
    /// its own volume envelope (the SoundFont's): hold and decay
    /// (timecents) and how they shorten up the keyboard (per key from 60),
    /// the sustain level, the release (seconds to -60 dB)
    hold_tc: f32,
    dec_tc: f32,
    key_hold: f32,
    key_dec: f32,
    sus: f32,
    rel: f32,
}

/// Where a note and layer's zones are in `Bank::picks`: `rr` round robins
/// of `per` zones each (all of them to sound together).
#[derive(Clone, Copy, Default, Debug)]
struct Cell {
    first: u32,
    rr: u8,
    per: u8,
}

/// An instrument as the voices see it: velocity to layer, and a cell for
/// every note (or a kit's one key) and layer.
struct Inst {
    layer: [u8; 128],
    layers: usize,
    cells: Vec<Cell>,
    /// a kit's key (its one note), None for an instrument played by note
    key: Option<u8>,
    /// "auto": the short articulation's id and the long one's, and the
    /// short one's gain (matching the long one's level)
    auto: Option<(u16, u16, f32)>,
    /// its zones (each instrument has its own, contiguous)
    zones: std::ops::Range<u32>,
}

/// Every sample loaded, and what plays them. Read-only once loaded.
pub struct Bank {
    data: Vec<i16>,
    zones: Vec<Zone>,
    picks: Vec<u32>,
    /// ids are 1 + an index here (0 is the synthesiser)
    insts: Vec<Inst>,
    multis: Vec<(String, String, u16)>,
    gms: Vec<(String, u16)>,
    kits: Vec<(String, [u16; 128])>,
    /// the instruments whose notes are kept alive (`LIVELY`), and whether each decays
    lively: Vec<(u16, bool)>,
    /// what loading skipped, and why
    pub warnings: Vec<String>,
}

impl Bank {
    fn new() -> Bank {
        Bank {
            data: Vec::new(),
            zones: Vec::new(),
            picks: Vec::new(),
            insts: Vec::new(),
            multis: Vec::new(),
            gms: Vec::new(),
            kits: Vec::new(),
            lively: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// A preset named: `p` is `id` (and, one of `LIVELY`, its notes kept
    /// alive: the SoundFont's, not a recording standing in for it, whose
    /// zones have no SoundFont envelope, `GM_RECORDED`).
    fn name_gm(&mut self, p: &str, id: u16) {
        self.gms.push((p.to_string(), id));
        let sf2 = self.insts.get(id as usize - 1).and_then(|i| self.zones.get(i.zones.start as usize));
        if let Some(&(_, decays)) = LIVELY.iter().find(|x| x.0 == p)
            && sf2.is_some_and(|z| z.hold_tc > -32768.0)
        {
            self.lively.push((id, decays));
        }
    }

    /// A multisampled instrument's articulation ("sus", "stac", "pizz",
    /// "vib", ... or "auto": by each note's gate, see `AUTO_SHORT`).
    pub fn multi(&self, instrument: &str, articulation: &str) -> Option<u16> {
        self.multis.iter().find(|m| m.0 == instrument && m.1 == articulation).map(|m| m.2)
    }

    /// A SoundFont preset, by name (one of `GM_PRESETS`).
    #[allow(dead_code)] // the genres' hook
    pub fn gm(&self, preset: &str) -> Option<u16> {
        self.gms.iter().find(|g| g.0 == preset).map(|g| g.1)
    }

    /// Whether a recording stands in for the SoundFont's `preset`
    /// (`GM_RECORDED`): what plays under its name has no SoundFont envelope.
    pub fn stands_in(&self, preset: &str) -> bool {
        self.gm(preset)
            .and_then(|id| self.insts.get(id as usize - 1))
            .and_then(|i| self.zones.get(i.zones.start as usize))
            .is_some_and(|z| z.hold_tc <= -32768.0)
    }

    /// One key of a SoundFont kit (one of `GM_KITS`), General MIDI's
    /// numbering (36 kick, 38 snare, 42 closed hat, 46 open hat, ...).
    #[allow(dead_code)] // the genres' hook
    pub fn drum(&self, kit: &str, key: u8) -> Option<u16> {
        let id = self.kits.iter().find(|k| k.0 == kit)?.1.get(key as usize).copied()?;
        (id != 0).then_some(id)
    }

    /// Bytes held: the samples, and the tables.
    pub fn bytes(&self) -> usize {
        self.data.len() * 2
            + self.zones.len() * std::mem::size_of::<Zone>()
            + self.picks.len() * 4
            + self.insts.iter().map(|i| i.cells.len() * std::mem::size_of::<Cell>() + 160).sum::<usize>()
    }

    /// Instruments, presets and kits held.
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.multis.iter().filter(|m| m.1 != "auto").count(), self.gms.len(), self.kits.len())
    }

    /// The instrument that plays `id` for a note `len` samples long: an
    /// "auto" instrument's short or long articulation, else itself.
    pub fn resolve(&self, id: u16, len: i64) -> u16 {
        match self.insts.get((id as usize).wrapping_sub(1)).and_then(|i| i.auto) {
            Some((short, long, _)) => {
                if len < (AUTO_SHORT * RATE) as i64 {
                    short
                } else {
                    long
                }
            }
            None => id,
        }
    }

    /// An instrument's own release (its first zone's: seconds to -60 dB).
    pub fn release(&self, id: u16) -> f32 {
        let id = self.resolve(id, i64::MAX);
        self.insts
            .get((id as usize).wrapping_sub(1))
            .and_then(|i| self.zones.get(i.zones.start as usize))
            .map_or(0.2, |z| z.rel)
    }

    /// The zones for `id` at `note`, velocity `vel` (0-1), `len` samples of
    /// gate, round robin `rr`; a kit's key; a gain on them (an "auto"
    /// instrument's short articulation's).
    fn pick(&self, id: u16, note: f32, vel: f32, len: i64, rr: u32) -> Option<(&[u32], Option<u8>, f32)> {
        seen(id);
        let mut inst = self.insts.get((id as usize).checked_sub(1)?)?;
        let mut g = 1.0;
        if let Some((short, long, gs)) = inst.auto {
            let sub = if len < (AUTO_SHORT * RATE) as i64 {
                g = gs;
                short
            } else {
                long
            };
            inst = self.insts.get((sub as usize).checked_sub(1)?)?;
        }
        let n = if inst.key.is_some() { 0 } else { note.round().clamp(0.0, 127.0) as usize };
        let layer = *inst.layer.get((vel * 127.0).round().clamp(0.0, 127.0) as usize)? as usize;
        let c = inst.cells.get(n * inst.layers + layer)?;
        if c.per == 0 || c.rr == 0 {
            return None;
        }
        let s = c.first as usize + (rr % c.rr as u32) as usize * c.per as usize;
        Some((self.picks.get(s..s + c.per as usize)?, inst.key, g))
    }

    /// A zone's frames.
    #[inline]
    fn frames(&self, z: &Zone) -> &[i16] {
        self.data.get(z.off..z.off + z.len as usize * z.chans as usize).unwrap_or(&[])
    }

    /// RMS over the first second of `id` at `note`, its loudest layer (both
    /// channels, as the synthesiser's `loudness` measures).
    fn rms_1s(&self, id: u16, note: f32) -> f32 {
        self.rms_over(id, note, 1.0)
    }

    /// RMS over the first `secs` of `id` at `note`, its loudest layer.
    fn rms_over(&self, id: u16, note: f32, secs: f32) -> f32 {
        self.rms_at(id, note, 1.0, secs)
    }

    /// RMS over the first `secs` of `id` at `note` and velocity `vel` (the
    /// layer it picks, at that layer's own level).
    fn rms_at(&self, id: u16, note: f32, vel: f32, secs: f32) -> f32 {
        let mut s = SAMPLER;
        if !s.start(self, id, note, vel, i64::MAX, 0) {
            return 0.0;
        }
        let pitch = s.key;
        let mut buf = [[0.0f32; 2]; 64];
        let mut sum = 0.0f64;
        let blocks = (secs * RATE) as usize / 64;
        for _ in 0..blocks {
            let live = s.fill(self, pitch, true, &mut buf);
            sum += buf.iter().map(|x| (x[0] * x[0] + x[1] * x[1]) as f64).sum::<f64>();
            if !live {
                break;
            }
        }
        (sum / (128 * blocks.max(1)) as f64).sqrt() as f32
    }

    /// Brings `id` to the reference (its loudest layer at `note`).
    fn normalise(&mut self, id: u16, note: f32, what: &str) -> bool {
        let rms = self.rms_1s(id, note);
        if !(rms > 1e-6) {
            self.warnings.push(format!("{what}: silent"));
            return false;
        }
        let k = REF_RMS / rms;
        let r = self.insts[id as usize - 1].zones.clone();
        for z in &mut self.zones[r.start as usize..r.end as usize] {
            z.gl *= k;
            z.gr *= k;
        }
        true
    }

    /// The recordings in place of SoundFont presets and kit pieces
    /// (`GM_RECORDED`; the recorded kit's pieces in `KITS_RECORDED` for
    /// `KIT_KEYS`): each a copy of the recording, each velocity layer at
    /// the level of what it stands in for at that velocity (the
    /// recordings' layers are evened, the SoundFont's soft ones quieter),
    /// a drum in its piece's choke class, so that a genre's parts keep the
    /// balance and dynamics they were set at.
    fn recorded(&mut self) {
        self.stand_in(GM_RECORDED);
        for &kit in KITS_RECORDED {
            let Some(k) = self.kits.iter().position(|x| x.0 == kit) else { continue };
            // the SoundFont's own kept, for a genre whose sound is built on it
            if self.multis.iter().any(|m| m.0 == "kit") {
                let own = (format!("{kit} (SoundFont)"), self.kits[k].1);
                self.kits.push(own);
            }
            for &(key, art) in KIT_KEYS {
                let (Some(rec), was) = (self.multi("kit", art), self.kits[k].1[key as usize]) else { continue };
                if was == 0 {
                    continue;
                }
                let g = self.layer_gains(was, rec, &[60.0], 0.5);
                let excl = self.insts[was as usize - 1].zones.clone().next().map_or(0, |z| self.zones[z as usize].excl);
                let id = self.copy_inst(rec, &g, excl);
                self.kits[k].1[key as usize] = id;
            }
        }
    }

    /// Each preset in `table` (rows as `GM_RECORDED`'s) played by its
    /// recording from here on, where both are loaded: a copy of the
    /// recording, its layers at the preset's levels (note by note where
    /// the row asks: `zone_gains`).
    fn stand_in(&mut self, table: &[Standing]) {
        for &(preset, inst, art, note, secs, by_note) in table {
            let Some(rec) = self.multi(inst, art) else { continue };
            let Some(gi) = self.gms.iter().position(|g| g.0 == preset) else { continue };
            let was = self.gms[gi].1;
            let notes = [note - 12.0, note - 6.0, note, note + 6.0, note + 12.0];
            let g = self.layer_gains(was, rec, &notes, secs);
            self.gms[gi].1 = if by_note {
                let zg = self.zone_gains(was, rec, &g, secs);
                self.copy_zones(rec, &zg, 0)
            } else {
                self.copy_inst(rec, &g, 0)
            };
        }
    }

    /// `layer_gains`, note by note: a gain for each of `rec`'s zones, that
    /// makes its layer as loud as `was` at the zone's own root (at the
    /// layer's middle velocity, over the first `secs`). The recordings are
    /// evened from note to note and a SoundFont's preset is not (its muted
    /// guitar 5 dB weaker above middle C), and a genre levels a part by one
    /// note of it: followed note by note, the recording is as loud as the
    /// preset wherever that note is. Each root's gain is the middle one of
    /// its own and its neighbours' (a single hot or weak note of the
    /// preset's is not copied); where the preset is silent, the layer's
    /// (`layer`).
    fn zone_gains(&self, was: u16, rec: u16, layer: &[f32], secs: f32) -> Vec<f32> {
        let inst = &self.insts[rec as usize - 1];
        let zr = inst.zones.clone();
        let mut zg: Vec<Option<f32>> = vec![None; (zr.end - zr.start) as usize];
        for li in 0..inst.layers {
            let vs: Vec<usize> = (0..128).filter(|&v| inst.layer[v] as usize == li).collect();
            let v = vs.get(vs.len() / 2).map_or(1.0, |&v| v as f32 / 127.0);
            let of_layer = |c: &(usize, &Cell)| c.0 % inst.layers == li;
            let zones = |c: &Cell| c.first as usize..c.first as usize + c.rr as usize * c.per as usize;
            // the layer's roots, low to high, and each one's gain (dB)
            let mut roots: Vec<i32> = inst
                .cells
                .iter()
                .enumerate()
                .filter(of_layer)
                .flat_map(|(_, c)| zones(c))
                .map(|p| self.zones[self.picks[p] as usize].root.round() as i32)
                .collect();
            roots.sort_unstable();
            roots.dedup();
            let own = 20.0 * layer.get(li).copied().unwrap_or(1.0).max(1e-6).log10();
            let raw: Vec<f32> = roots
                .iter()
                .map(|&r| {
                    let (a, b) = (self.rms_at(was, r as f32, v, secs), self.rms_at(rec, r as f32, v, secs));
                    if a > 1e-6 && b > 1e-6 { 20.0 * (a / b).log10() } else { own }
                })
                .collect();
            // (the lowest and the highest root, with one neighbour, keep their own)
            let middle = |i: usize| {
                let mut w: Vec<f32> = raw[i.saturating_sub(1)..(i + 2).min(raw.len())].to_vec();
                w.sort_by(f32::total_cmp);
                if w.len() == 3 { w[1] } else { raw[i] }
            };
            let gains: Vec<f32> = (0..raw.len()).map(|i| 10f32.powf(middle(i) / 20.0)).collect();
            for (_, c) in inst.cells.iter().enumerate().filter(of_layer) {
                for p in zones(c) {
                    let z = self.picks[p] as usize;
                    let at = roots.binary_search(&(self.zones[z].root.round() as i32)).unwrap_or(0);
                    zg[z - zr.start as usize].get_or_insert(gains.get(at).copied().unwrap_or(1.0));
                }
            }
        }
        zg.into_iter().map(|g| g.unwrap_or(1.0)).collect()
    }

    /// How much louder each velocity layer of `rec` is to be to sound as
    /// `was` does at that layer's middle velocity: their power over the
    /// first `secs` of `notes`.
    fn layer_gains(&self, was: u16, rec: u16, notes: &[f32], secs: f32) -> Vec<f32> {
        let inst = &self.insts[rec as usize - 1];
        (0..inst.layers)
            .map(|li| {
                let vs: Vec<usize> = (0..128).filter(|&v| inst.layer[v] as usize == li).collect();
                let v = vs.get(vs.len() / 2).map_or(1.0, |&v| v as f32 / 127.0);
                let pow = |id: u16| notes.iter().map(|&n| self.rms_at(id, n, v, secs).powi(2)).sum::<f32>();
                let (a, b) = (pow(was), pow(rec));
                if a > 1e-12 && b > 1e-12 { (a / b).sqrt() } else { 1.0 }
            })
            .collect()
    }

    /// A copy of the instrument `id` (its zones' recordings shared), each
    /// velocity layer `g[layer]` times as loud (a zone in two layers, as
    /// the first), in choke class `excl`. Its id.
    fn copy_inst(&mut self, id: u16, g: &[f32], excl: u8) -> u16 {
        let src = &self.insts[id as usize - 1];
        let (zr, layers) = (src.zones.clone(), src.layers);
        let mut zg: Vec<Option<f32>> = vec![None; (zr.end - zr.start) as usize];
        for (i, c) in src.cells.iter().enumerate() {
            for k in 0..c.rr as usize * c.per as usize {
                let z = self.picks[c.first as usize + k] - zr.start;
                zg[z as usize].get_or_insert(g.get(i % layers).copied().unwrap_or(1.0));
            }
        }
        let zg: Vec<f32> = zg.into_iter().map(|g| g.unwrap_or(1.0)).collect();
        self.copy_zones(id, &zg, excl)
    }

    /// A copy of the instrument `id` (its zones' recordings shared), each
    /// of its zones `zg[zone]` times as loud, in choke class `excl`. Its id.
    fn copy_zones(&mut self, id: u16, zg: &[f32], excl: u8) -> u16 {
        let src = &self.insts[id as usize - 1];
        let (zr, layer, layers, key, old) = (src.zones.clone(), src.layer, src.layers, src.key, src.cells.clone());
        let base = self.zones.len() as u32;
        let zones: Vec<Zone> = self.zones[zr.start as usize..zr.end as usize]
            .iter()
            .zip(zg)
            .map(|(z, &g)| Zone { gl: z.gl * g, gr: z.gr * g, excl, ..*z })
            .collect();
        self.zones.extend(zones);
        let mut cells = Vec::with_capacity(old.len());
        for c in old {
            let first = self.picks.len() as u32;
            let n = c.rr as usize * c.per as usize;
            for i in 0..n {
                let p = self.picks[c.first as usize + i];
                self.picks.push(p - zr.start + base);
            }
            cells.push(Cell { first, ..c });
        }
        self.insts.push(Inst { layer, layers, cells, key, auto: None, zones: base..self.zones.len() as u32 });
        self.insts.len() as u16
    }

    /// Adds an instrument made of `zones`, each covering keys and
    /// velocities (`covers`, in step). `nearest`: each note plays the zones
    /// whose root is nearest (ties: the lower), the ones of that root all
    /// round robins in their order; else (the SoundFont's) every zone whose
    /// range holds the note sounds (`MAXZ` at most), and a note none holds
    /// takes the nearest note's. Its id.
    fn add(&mut self, zones: Vec<Zone>, covers: &[Cover], nearest: bool, key: Option<u8>) -> u16 {
        let base = self.zones.len() as u32;
        self.zones.extend(zones);
        // layers: wherever a zone's velocities start or end
        let mut cuts: Vec<u8> = vec![0];
        for c in covers {
            cuts.push(c.vlo);
            if c.vhi < 127 {
                cuts.push(c.vhi + 1);
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        let layers = cuts.len();
        let mut layer = [0u8; 128];
        for (v, l) in layer.iter_mut().enumerate() {
            *l = (cuts.partition_point(|&c| c as usize <= v) - 1) as u8;
        }
        let notes: Vec<u8> = match key {
            Some(k) => vec![k],
            None => (0..128).collect(),
        };
        let mut cells = vec![Cell::default(); notes.len() * layers];
        let mut here: Vec<usize> = Vec::new();
        for (ni, &n) in notes.iter().enumerate() {
            for (li, &v) in cuts.iter().enumerate() {
                here.clear();
                here.extend(
                    (0..covers.len())
                        .filter(|&i| covers[i].vlo <= v && v <= covers[i].vhi)
                        .filter(|&i| nearest || (covers[i].klo <= n && n <= covers[i].khi)),
                );
                if here.is_empty() {
                    continue;
                }
                let first = self.picks.len() as u32;
                if nearest {
                    let d = |i: usize| ((covers[i].root as i32 - n as i32).abs(), covers[i].root);
                    let best = covers[here.iter().copied().min_by_key(|&i| d(i)).unwrap_or(here[0])].root;
                    here.retain(|&i| covers[i].root == best);
                    here.sort_by_key(|&i| covers[i].rr);
                    here.truncate(255);
                    self.picks.extend(here.iter().map(|&i| base + i as u32));
                    cells[ni * layers + li] = Cell { first, rr: here.len() as u8, per: 1 };
                } else {
                    here.truncate(MAXZ);
                    self.picks.extend(here.iter().map(|&i| base + i as u32));
                    cells[ni * layers + li] = Cell { first, rr: 1, per: here.len() as u8 };
                }
            }
        }
        // gaps: the nearest note's cell in the layer, then the nearest layer's
        let nn = notes.len();
        for li in 0..layers {
            let full: Vec<usize> = (0..nn).filter(|&ni| cells[ni * layers + li].per > 0).collect();
            if full.is_empty() {
                continue;
            }
            for ni in 0..nn {
                if cells[ni * layers + li].per == 0 {
                    let near = full.iter().copied().min_by_key(|&f| (f.abs_diff(ni), f)).unwrap_or(full[0]);
                    cells[ni * layers + li] = cells[near * layers + li];
                }
            }
        }
        for ni in 0..nn {
            let full: Vec<usize> = (0..layers).filter(|&li| cells[ni * layers + li].per > 0).collect();
            for li in 0..layers {
                if cells[ni * layers + li].per == 0 {
                    if let Some(&near) = full.iter().min_by_key(|&&f| (f.abs_diff(li), f)) {
                        cells[ni * layers + li] = cells[ni * layers + near];
                    }
                }
            }
        }
        self.insts.push(Inst { layer, layers, cells, key, auto: None, zones: base..self.zones.len() as u32 });
        self.insts.len() as u16
    }
}

/// A zone's keys, velocities, root and round robin, as the table is made.
#[derive(Clone, Copy)]
struct Cover {
    klo: u8,
    khi: u8,
    vlo: u8,
    vhi: u8,
    root: u8,
    rr: u32,
}

// ---------------------------------------------------------------- playing

/// What a voice playing samples keeps: its zones (a stereo pair, or
/// layers), their read positions, and their own envelope (the SoundFont's
/// hold, decay and sustain; 1 throughout for a WAV).
#[derive(Clone, Copy)]
pub struct Sampler {
    /// zones sounding (0: none, the synthesiser plays)
    pub n: u8,
    z: [u32; MAXZ],
    pos: [f64; MAXZ],
    live: [bool; MAXZ],
    env: f32,
    hold: u32,
    dec: f32,
    sus: f32,
    /// the note a kit's key sounds (its pitch), else the note started
    pub key: f32,
    /// the instrument's own release (seconds to -60 dB)
    pub rel: f32,
    /// the zones' exclusive class (0 none)
    pub excl: u8,
    /// whether looped zones loop (a drum's hit plays once through)
    pub loops: bool,
    /// a gain on the zones (an "auto" instrument's short articulation's)
    g: f32,
    /// a short loop's note kept alive (`LIFE_PERIODS`)
    life: Life,
}

/// What keeps a short loop's note alive: whether it is (and whether it
/// decays, a pluck's or a strike's), its age (samples), the pitch's wander
/// (semitones) and its aim, the dice, the low-pass's state (two poles, left
/// and right).
#[derive(Clone, Copy)]
struct Life {
    on: bool,
    decays: bool,
    age: u32,
    drift: f32,
    aim: f32,
    dice: u32,
    lp: [[f32; 2]; 2],
}

const LIFE: Life = Life { on: false, decays: false, age: 0, drift: 0.0, aim: 0.0, dice: 1, lp: [[0.0; 2]; 2] };

pub const SAMPLER: Sampler = Sampler {
    n: 0,
    z: [0; MAXZ],
    pos: [0.0; MAXZ],
    live: [false; MAXZ],
    env: 1.0,
    hold: 0,
    dec: 1.0,
    sus: 1.0,
    key: 60.0,
    rel: 0.1,
    excl: 0,
    loops: true,
    g: 1.0,
    life: LIFE,
};

/// Frames `i` and `j` (mono or interleaved stereo) `fr` of the way between.
#[inline(always)]
fn lerp(d: &[i16], chans: u8, i: usize, j: usize, fr: f32) -> (f32, f32) {
    if chans == 2 {
        let (a, b) = (d[2 * i] as f32, d[2 * j] as f32);
        let (c, e) = (d[2 * i + 1] as f32, d[2 * j + 1] as f32);
        (a + (b - a) * fr, c + (e - c) * fr)
    } else {
        let (a, b) = (d[i] as f32, d[j] as f32);
        let m = a + (b - a) * fr;
        (m, m)
    }
}

/// Timecents to seconds.
fn tc(t: f32) -> f32 {
    if t <= -32000.0 { 0.0 } else { (t / 1200.0).exp2() }
}

impl Sampler {
    /// Starts `id` at `note` (a kit's key sounds whatever the note),
    /// velocity `vel` (0-1), `len` samples of gate (an "auto" instrument's
    /// articulation), round robin `rr`. False (nothing sounding) when the
    /// bank has nothing for it.
    pub fn start(&mut self, b: &Bank, id: u16, note: f32, vel: f32, len: i64, rr: u32) -> bool {
        self.n = 0;
        let Some((zs, key, g)) = b.pick(id, note, vel, len, rr) else {
            return false;
        };
        let n = zs.len().min(MAXZ);
        for (k, &z) in zs.iter().take(n).enumerate() {
            self.z[k] = z;
            self.pos[k] = 0.0;
            self.live[k] = true;
        }
        let Some(z0) = b.zones.get(zs[0] as usize) else {
            return false;
        };
        self.n = n as u8;
        self.key = key.map_or(note, |k| k as f32);
        // the hold and decay shorten up the keyboard as the zone says
        let k = self.key - 60.0;
        self.hold = (tc(z0.hold_tc - z0.key_hold * k).min(20.0) * RATE) as u32;
        let dec = tc(z0.dec_tc - z0.key_dec * k).clamp(0.001, 100.0);
        // -96 dB over the decay (FluidSynth's), down to the sustain
        self.dec = 10f32.powf(-4.8 / (dec * RATE));
        self.sus = z0.sus;
        self.env = 1.0;
        self.rel = z0.rel;
        self.excl = z0.excl;
        self.loops = true;
        self.g = g;
        let lively = b.lively.iter().find(|x| x.0 == id).filter(|_| key.is_none());
        self.life = Life {
            on: lively.is_some(),
            decays: lively.is_some_and(|x| x.1),
            dice: (note.to_bits() ^ vel.to_bits().rotate_left(13) ^ rr.wrapping_mul(0x9e37_79b9)) | 1,
            ..LIFE
        };
        true
    }

    /// The zones at `pitch` (a note, fractional: glides and vibrato) into
    /// `out` (overwritten); `gate`: still held (a loop until release keeps
    /// looping). False once they have all ended or died away.
    #[inline]
    pub fn fill(&mut self, b: &Bank, pitch: f32, gate: bool, out: &mut [[f32; 2]]) -> bool {
        out.fill([0.0; 2]);
        let mut any = false;
        let life = &mut self.life;
        let pitch = if life.on {
            let every = (DRIFT_EVERY * RATE) as u32;
            if life.age % every < out.len() as u32 {
                life.dice ^= life.dice << 13;
                life.dice ^= life.dice >> 17;
                life.dice ^= life.dice << 5;
                life.aim = ((life.dice >> 8) as f32 / (1u32 << 24) as f32 - 0.5) * 2.0 * DRIFT / 100.0;
            }
            life.drift += (life.aim - life.drift) * (1.0 - (-(out.len() as f32) / (DRIFT_GLIDE * RATE)).exp());
            pitch + life.drift
        } else {
            pitch
        };
        for k in 0..self.n as usize {
            if !self.live[k] {
                continue;
            }
            let Some(z) = b.zones.get(self.z[k] as usize) else {
                self.live[k] = false;
                continue;
            };
            let d = b.frames(z);
            if d.len() < 2 * z.chans as usize {
                self.live[k] = false;
                continue;
            }
            let step = ((((pitch - z.root) * z.scale + z.tune) / 12.0).exp2() * z.rate).clamp(1e-3, 64.0) as f64;
            let looping = self.loops && (z.mode == 1 || (z.mode == 3 && gate));
            let (ls, le) = (z.ls as f64, z.le as f64);
            let end = (z.len - 1) as f64;
            let fade = 1.0 / (END_FADE * step);
            let (gl, gr) = (z.gl * self.g, z.gr * self.g);
            let xf = if looping { z.xf as f64 } else { 0.0 };
            let xs = le - xf;
            let mut pos = self.pos[k];
            let n = out.len();
            let mut at = 0;
            while at < n {
                // plain reading as far as it goes: short of the loop's end
                // (or its crossfade) or of the end's fade, a frame to spare
                let limit = if looping { if xf > 0.0 { xs } else { le } } else { end - END_FADE * step } - 1.0;
                if pos < limit {
                    let m = (((limit - pos) / step).ceil() as usize).clamp(1, n - at);
                    // in 32.32 fixed point from the frame under it
                    let base = pos.floor();
                    let b0 = base as usize;
                    let (mut fx, st) = (((pos - base) * FX) as u64, (step * FX) as u64);
                    if z.chans == 2 {
                        for o in out[at..at + m].iter_mut() {
                            let i = 2 * (b0 + (fx >> 32) as usize);
                            let fr = (fx as u32) as f32 * (1.0 / FX as f32);
                            let (a, b) = (d[i] as f32, d[i + 2] as f32);
                            let (c, e) = (d[i + 1] as f32, d[i + 3] as f32);
                            o[0] += (a + (b - a) * fr) * gl;
                            o[1] += (c + (e - c) * fr) * gr;
                            fx += st;
                        }
                    } else {
                        for o in out[at..at + m].iter_mut() {
                            let i = b0 + (fx >> 32) as usize;
                            let fr = (fx as u32) as f32 * (1.0 / FX as f32);
                            let (a, b) = (d[i] as f32, d[i + 1] as f32);
                            let x = a + (b - a) * fr;
                            o[0] += x * gl;
                            o[1] += x * gr;
                            fx += st;
                        }
                    }
                    pos = base + fx as f64 / FX;
                    at += m;
                    continue;
                }
                // a sample at a time: the loop's wrap and crossfade, the end's fade
                if looping {
                    if pos >= le {
                        pos = ls + (pos - le) % (le - ls);
                    }
                } else if pos >= end {
                    self.live[k] = false;
                    break;
                }
                let i = pos as usize;
                let fr = (pos - i as f64) as f32;
                let j = if looping && i + 1 >= z.le as usize { z.ls as usize } else { i + 1 };
                let (mut l, mut r) = lerp(d, z.chans, i, j, fr);
                if xf > 0.0 && pos >= xs {
                    // equal power, into the same stretch before the loop's start
                    let q = pos - (le - ls);
                    let qi = q as usize;
                    let (l2, r2) = lerp(d, z.chans, qi, qi + 1, (q - qi as f64) as f32);
                    let t = ((pos - xs) / xf) as f32;
                    let (a, b) = ((1.0 - t).sqrt(), t.sqrt());
                    l = l * a + l2 * b;
                    r = r * a + r2 * b;
                }
                if !looping {
                    let left = ((end - pos) * fade) as f32;
                    if left < 1.0 {
                        l *= left;
                        r *= left;
                    }
                }
                out[at][0] += l * gl;
                out[at][1] += r * gr;
                pos += step;
                at += 1;
            }
            self.pos[k] = pos;
            any |= self.live[k];
        }
        let life = &mut self.life;
        if life.on && life.decays {
            // the highs dying first: two one-poles, their corner falling from
            // open to a few harmonics (once a block)
            let f0 = 440.0 * ((pitch - 69.0) / 12.0).exp2();
            let tau = (0.5 * (220.0 / f0).sqrt()).clamp(0.08, 1.5) * RATE;
            let floor = f0 * LIFE_FLOOR;
            let fc = floor + (LIFE_OPEN - floor).max(0.0) * (-(life.age as f32) / tau).exp();
            let a = 1.0 - (-std::f32::consts::TAU * fc.min(18000.0) / RATE).exp();
            for o in out.iter_mut() {
                let [p0, p1] = &mut life.lp;
                for ((x, y0), y1) in o.iter().zip(p0.iter_mut()).zip(p1.iter_mut()) {
                    *y0 += a * (x - *y0);
                    *y1 += a * (*y0 - *y1);
                }
                *o = *p1;
            }
        }
        life.age = life.age.saturating_add(out.len() as u32);
        if self.hold == 0 && self.env == self.sus {
            if self.env != 1.0 {
                for o in out.iter_mut() {
                    o[0] *= self.env;
                    o[1] *= self.env;
                }
            }
        } else {
            // held, then decaying to the sustain: a ramp to where the block ends
            let n = out.len();
            let h = (self.hold as usize).min(n);
            self.hold -= h as u32;
            let e0 = self.env;
            let e1 = if n > h { (e0 * self.dec.powi((n - h) as i32)).max(self.sus) } else { e0 };
            let de = (e1 - e0) / n.max(1) as f32;
            let mut e = e0;
            for o in out.iter_mut() {
                e += de;
                o[0] *= e;
                o[1] *= e;
            }
            self.env = e1;
        }
        any && self.env >= 1e-3
    }
}

// ---------------------------------------------------------------- WAV

/// A WAV's frames (16-bit PCM, mono or stereo, interleaved), its rate and
/// channels. Every chunk is bounds-checked; anything else is an error.
fn read_wav(b: &[u8]) -> Result<(Vec<i16>, u32, u8), String> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE file".into());
    }
    let mut fmt: Option<(u16, u16, u32, u16, u16)> = None;
    let mut data: Option<&[u8]> = None;
    let mut p = 12usize;
    while p + 8 <= b.len() {
        let id = &b[p..p + 4];
        let size = u32::from_le_bytes([b[p + 4], b[p + 5], b[p + 6], b[p + 7]]) as usize;
        let body = p + 8;
        let end = body.checked_add(size).filter(|&e| e <= b.len()).ok_or("a chunk runs past the end")?;
        let c = &b[body..end];
        match id {
            b"fmt " => {
                if c.len() < 16 {
                    return Err("fmt chunk too short".into());
                }
                let u16_at = |i: usize| u16::from_le_bytes([c[i], c[i + 1]]);
                let mut tag = u16_at(0);
                if tag == 0xFFFE {
                    // WAVE_FORMAT_EXTENSIBLE: the subformat's first two bytes
                    if c.len() < 26 {
                        return Err("extensible fmt chunk too short".into());
                    }
                    tag = u16_at(24);
                }
                let rate = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
                fmt = Some((tag, u16_at(2), rate, u16_at(12), u16_at(14)));
            }
            b"data" => data = Some(c),
            _ => {}
        }
        p = end + (size & 1);
    }
    let (tag, chans, rate, align, bits) = fmt.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    if tag != 1 || bits != 16 {
        return Err(format!("format {tag}, {bits} bits: only 16-bit PCM"));
    }
    if !(1..=2).contains(&chans) || align != 2 * chans {
        return Err(format!("{chans} channels, block {align}: only mono or stereo"));
    }
    if !(4000..=192_000).contains(&rate) {
        return Err(format!("rate {rate}"));
    }
    let frames = data.len() / align as usize;
    if frames < 2 {
        return Err("no audio".into());
    }
    let v = data[..frames * align as usize].chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]])).collect();
    Ok((v, rate, chans as u8))
}

// ---------------------------------------------------------------- manifest

/// One manifest row.
struct Row {
    instrument: String,
    articulation: String,
    root: u8,
    vlo: u8,
    vhi: u8,
    rr: u32,
    gain_db: f32,
    file: String,
}

fn parse_row(f: &[&str]) -> Result<Row, String> {
    if f.len() != 8 {
        return Err(format!("{} fields, not 8", f.len()));
    }
    let num = |s: &str, what: &str| s.trim().parse::<u32>().map_err(|_| format!("{what} {s:?}"));
    let (root, vlo, vhi) = (num(f[2], "root")?, num(f[3], "vel_lo")?, num(f[4], "vel_hi")?);
    if root > 127 || vhi > 127 || vlo > vhi {
        return Err(format!("root {root}, velocities {vlo}..{vhi}"));
    }
    let gain_db = f[6].trim().parse::<f32>().map_err(|_| format!("gain_db {:?}", f[6]))?;
    if !gain_db.is_finite() || gain_db.abs() > 96.0 {
        return Err(format!("gain_db {gain_db}"));
    }
    let file = f[7].trim();
    // inside the samples directory, nowhere else
    if file.is_empty() || !Path::new(file).components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(format!("file {file:?}"));
    }
    let (instrument, articulation) = (f[0].trim(), f[1].trim());
    if instrument.is_empty() || articulation.is_empty() || articulation == "auto" {
        return Err("no instrument or articulation".into());
    }
    Ok(Row {
        instrument: instrument.into(),
        articulation: articulation.into(),
        root: root as u8,
        vlo: vlo as u8,
        vhi: vhi as u8,
        rr: num(f[5], "rr")?,
        gain_db,
        file: file.into(),
    })
}

/// The manifest's instruments, and an "auto" for each: short notes (gates
/// under `AUTO_SHORT`) the staccato (or pizzicato, when that is all there
/// is), long ones the vibrato (else the sustain); either, failing that,
/// whatever there is. The short articulation is brought to the long one's
/// level (its RMS over the first `AUTO_MATCH` to the long one's over the
/// first second, at the long one's middle root, loudest layer): the
/// manifest levels each by its first second, which makes a short note hot.
/// One root does for all, as the manifest evens each articulation's roots.
fn load_multi(b: &mut Bank, dir: &Path) -> Result<(), String> {
    let path = dir.join(MANIFEST);
    let text = read_file(&path, 16 << 20)?;
    let text = String::from_utf8(text).map_err(|_| format!("{}: not UTF-8", path.display()))?;
    let mut rows: Vec<Row> = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f[0].trim() == "instrument" {
            continue;
        }
        match parse_row(&f) {
            Ok(r) => rows.push(r),
            Err(e) => b.warnings.push(format!("{MANIFEST} line {}: {e}", ln + 1)),
        }
    }
    let mut groups: Vec<(String, String)> = Vec::new();
    for r in &rows {
        if !groups.iter().any(|g| g.0 == r.instrument && g.1 == r.articulation) {
            groups.push((r.instrument.clone(), r.articulation.clone()));
        }
    }
    // each file read once, however many rows name it
    let mut wavs: HashMap<String, Option<(usize, u32, u8, u32)>> = HashMap::new();
    for (inst, art) in groups {
        let (mut zones, mut covers) = (Vec::new(), Vec::new());
        for r in rows.iter().filter(|r| r.instrument == inst && r.articulation == art) {
            let w = match wavs.get(&r.file) {
                Some(w) => *w,
                None => {
                    let w = match read_file(&dir.join(&r.file), 256 << 20).and_then(|v| read_wav(&v)) {
                        Ok((v, rate, chans)) => {
                            let off = b.data.len();
                            b.data.extend_from_slice(&v);
                            Some((off, (v.len() / chans as usize) as u32, chans, rate))
                        }
                        Err(e) => {
                            b.warnings.push(format!("{}: {e}", r.file));
                            None
                        }
                    };
                    wavs.insert(r.file.clone(), w);
                    w
                }
            };
            let Some((off, len, chans, rate)) = w else { continue };
            let g = 10f32.powf(r.gain_db / 20.0) / 32768.0;
            // a held note outlasting a sustained recording (5.5 s, its last
            // 0.5 s faded): its steady middle looped, crossfaded
            let sec = |t: f32| (t * rate as f32) as u32;
            let held = matches!(r.articulation.as_str(), "sus" | "vib") && len >= sec(LOOP_MIN);
            let (ls, le) = if held { (sec(LOOP.0), sec(LOOP.1)) } else { (0, 0) };
            zones.push(Zone {
                off,
                len,
                chans,
                root: r.root as f32,
                tune: 0.0,
                scale: 1.0,
                rate: rate as f32 / RATE,
                ls,
                le,
                mode: if held { 3 } else { 0 },
                xf: if held { sec(LOOP_XF) } else { 0 },
                gl: g,
                gr: g,
                excl: 0,
                hold_tc: -32768.0,
                dec_tc: -32768.0,
                key_hold: 0.0,
                key_dec: 0.0,
                sus: 1.0,
                rel: 0.1,
            });
            covers.push(Cover { klo: 0, khi: 127, vlo: r.vlo, vhi: r.vhi, root: r.root, rr: r.rr });
        }
        if zones.is_empty() {
            continue;
        }
        let id = b.add(zones, &covers, true, None);
        b.multis.push((inst, art, id));
    }
    // "auto" for each instrument
    let mut names: Vec<String> = Vec::new();
    for m in &b.multis {
        if !names.contains(&m.0) {
            names.push(m.0.clone());
        }
    }
    for name in names {
        let find = |arts: &[&str]| arts.iter().find_map(|a| b.multi(&name, a));
        let any = b.multis.iter().find(|m| m.0 == name).map(|m| m.2);
        let long = find(&["vib", "sus"]).or(find(&["stac", "pizz"])).or(any);
        let short = find(&["stac", "pizz"]).or(long);
        if let (Some(s), Some(l)) = (short, long) {
            let mut g = 1.0;
            if s != l {
                let r = b.insts[l as usize - 1].zones.clone();
                let mut roots: Vec<f32> = b.zones[r.start as usize..r.end as usize].iter().map(|z| z.root).collect();
                roots.sort_by(f32::total_cmp);
                let mid = roots[roots.len() / 2];
                let (rs, rl) = (b.rms_over(s, mid, AUTO_MATCH), b.rms_1s(l, mid));
                if rs > 1e-6 && rl > 1e-6 {
                    g = (rl / rs).clamp(0.25, 4.0);
                }
            }
            b.insts.push(Inst {
                layer: [0; 128],
                layers: 0,
                cells: Vec::new(),
                key: None,
                auto: Some((s, l, g)),
                zones: 0..0,
            });
            b.multis.push((name, "auto".into(), b.insts.len() as u16));
        }
    }
    Ok(())
}

/// A file's bytes, refused past `max` bytes.
fn read_file(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let mut f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = f.metadata().map_err(|e| format!("{}: {e}", path.display()))?.len();
    if len > max {
        return Err(format!("{}: {len} bytes", path.display()));
    }
    let mut v = Vec::with_capacity(len as usize);
    f.read_to_end(&mut v).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(v)
}

// ---------------------------------------------------------------- SoundFont

/// A generator as stored: its operator and amount.
type Gen = (u16, [u8; 2]);

/// Generators: a zone's values (as stored) and which it sets.
#[derive(Clone, Copy)]
struct Gens {
    v: [i16; 61],
    set: u64,
}

const G_START: usize = 0;
const G_END: usize = 1;
const G_LOOP_START: usize = 2;
const G_LOOP_END: usize = 3;
const G_START_COARSE: usize = 4;
const G_END_COARSE: usize = 12;
const G_PAN: usize = 17;
const G_HOLD: usize = 35;
const G_DECAY: usize = 36;
const G_SUSTAIN: usize = 37;
const G_RELEASE: usize = 38;
const G_KEY_HOLD: usize = 39;
const G_KEY_DECAY: usize = 40;
const G_INSTRUMENT: u16 = 41;
const G_KEYS: usize = 43;
const G_VELS: usize = 44;
const G_LOOP_START_COARSE: usize = 45;
const G_ATTEN: usize = 48;
const G_LOOP_END_COARSE: usize = 50;
const G_COARSE: usize = 51;
const G_FINE: usize = 52;
const G_SAMPLE: u16 = 53;
const G_MODES: usize = 54;
const G_SCALE: usize = 56;
const G_EXCL: usize = 57;
const G_ROOT: usize = 58;

impl Gens {
    /// None set: a preset zone's (its values add to the instrument's).
    fn none() -> Gens {
        Gens { v: [0; 61], set: 0 }
    }

    /// The SoundFont's defaults: an instrument zone's before its own.
    fn defaults() -> Gens {
        let mut g = Gens::none();
        g.v[G_KEYS] = 0x7F00;
        g.v[G_VELS] = 0x7F00;
        g.v[G_ROOT] = -1;
        g.v[G_SCALE] = 100;
        for t in [33, 34, G_HOLD, G_DECAY, G_RELEASE] {
            g.v[t] = -12000;
        }
        g
    }

    fn put(&mut self, (op, amt): Gen) {
        if (op as usize) < 61 {
            self.v[op as usize] = i16::from_le_bytes(amt);
            self.set |= 1 << op;
        }
    }

    fn has(&self, op: usize) -> bool {
        self.set & (1 << op) != 0
    }

    fn range(&self, op: usize) -> (u8, u8) {
        let a = self.v[op] as u16;
        ((a & 0xFF) as u8, (a >> 8) as u8)
    }

    fn i(&self, op: usize) -> i32 {
        self.v[op] as i32
    }
}

struct Shdr {
    start: u32,
    end: u32,
    ls: u32,
    le: u32,
    rate: u32,
    pitch: u8,
    corr: i8,
    kind: u16,
}

/// The SoundFont's articulation data (pdta), and where its samples are.
struct Sf2 {
    file: File,
    /// the smpl chunk: offset in the file, frames
    smpl: (u64, u64),
    phdr: Vec<(String, u16, u16, u16)>,
    pbag: Vec<u16>,
    pgen: Vec<Gen>,
    inst: Vec<u16>,
    ibag: Vec<u16>,
    igen: Vec<Gen>,
    shdr: Vec<Shdr>,
}

/// A zone as the SoundFont resolves it (instrument and preset generators
/// combined), before its sample is loaded.
struct Raw {
    sample: usize,
    klo: u8,
    khi: u8,
    vlo: u8,
    vhi: u8,
    root: u8,
    tune: f32,
    scale: f32,
    mode: u8,
    /// address offsets: start, end, loop start, loop end (frames)
    offs: [i64; 4],
    atten: i32,
    pan: i32,
    excl: u8,
    hold: i32,
    decay: i32,
    key_hold: i32,
    key_decay: i32,
    sus: i32,
    rel: i32,
}

fn read_at(f: &mut File, len_file: u64, off: u64, len: usize) -> Result<Vec<u8>, String> {
    if off.checked_add(len as u64).is_none_or(|e| e > len_file) {
        return Err("a chunk runs past the end".into());
    }
    f.seek(SeekFrom::Start(off)).map_err(|e| e.to_string())?;
    let mut v = vec![0u8; len];
    f.read_exact(&mut v).map_err(|e| e.to_string())?;
    Ok(v)
}

/// A chunk: its id and body.
type Chunk<'a> = ([u8; 4], &'a [u8]);

/// The sub-chunks of a chunk's body, bounds-checked.
fn sub_chunks(b: &[u8]) -> Result<Vec<Chunk<'_>>, String> {
    let mut v = Vec::new();
    let mut p = 0usize;
    while p + 8 <= b.len() {
        let id = [b[p], b[p + 1], b[p + 2], b[p + 3]];
        let size = u32::from_le_bytes([b[p + 4], b[p + 5], b[p + 6], b[p + 7]]) as usize;
        let end = (p + 8).checked_add(size).filter(|&e| e <= b.len()).ok_or("a sub-chunk runs past the end")?;
        v.push((id, &b[p + 8..end]));
        p = end + (size & 1);
    }
    Ok(v)
}

/// A NUL-padded name.
fn name(b: &[u8]) -> String {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    b[..n].iter().map(|&c| c as char).collect::<String>().trim().to_string()
}

impl Sf2 {
    fn open(path: &Path) -> Result<Sf2, String> {
        let mut f = File::open(path).map_err(|e| e.to_string())?;
        let flen = f.metadata().map_err(|e| e.to_string())?.len();
        let h = read_at(&mut f, flen, 0, 12)?;
        if &h[0..4] != b"RIFF" || &h[8..12] != b"sfbk" {
            return Err("not a SoundFont".into());
        }
        let riff_end = (8 + u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as u64).min(flen);
        let (mut smpl, mut pdta) = (None, None);
        let mut p = 12u64;
        while p + 8 <= riff_end {
            let h = read_at(&mut f, flen, p, 8)?;
            let size = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as u64;
            let (body, end) = (p + 8, p + 8 + size);
            if end > riff_end {
                return Err("a chunk runs past the end".into());
            }
            if &h[0..4] == b"LIST" && size >= 4 {
                match &read_at(&mut f, flen, body, 4)?[..] {
                    b"sdta" => {
                        let mut q = body + 4;
                        while q + 8 <= end {
                            let s = read_at(&mut f, flen, q, 8)?;
                            let n = u32::from_le_bytes([s[4], s[5], s[6], s[7]]) as u64;
                            if q + 8 + n > end {
                                return Err("smpl runs past the end".into());
                            }
                            if &s[0..4] == b"smpl" {
                                smpl = Some((q + 8, n / 2));
                            }
                            q += 8 + n + (n & 1);
                        }
                    }
                    b"pdta" => {
                        if size > 64 << 20 {
                            return Err(format!("pdta of {size} bytes"));
                        }
                        pdta = Some(read_at(&mut f, flen, body + 4, size as usize - 4)?);
                    }
                    _ => {}
                }
            }
            p = end + (size & 1);
        }
        let smpl = smpl.ok_or("no smpl chunk")?;
        let pdta = pdta.ok_or("no pdta chunk")?;
        let subs = sub_chunks(&pdta)?;
        let get = |id: &[u8; 4], rec: usize, min: usize| -> Result<Vec<&[u8]>, String> {
            let c =
                subs.iter().find(|s| &s.0 == id).ok_or_else(|| format!("no {} chunk", String::from_utf8_lossy(id)))?.1;
            if c.len() % rec != 0 || c.len() / rec < min {
                return Err(format!("{} chunk of {} bytes", String::from_utf8_lossy(id), c.len()));
            }
            Ok(c.chunks_exact(rec).collect())
        };
        let u16_at = |r: &[u8], i: usize| u16::from_le_bytes([r[i], r[i + 1]]);
        let u32_at = |r: &[u8], i: usize| u32::from_le_bytes([r[i], r[i + 1], r[i + 2], r[i + 3]]);
        let gens = |r: Vec<&[u8]>| r.iter().map(|g| (u16_at(g, 0), [g[2], g[3]])).collect();
        Ok(Sf2 {
            phdr: get(b"phdr", 38, 2)?
                .iter()
                .map(|r| (name(&r[..20]), u16_at(r, 20), u16_at(r, 22), u16_at(r, 24)))
                .collect(),
            pbag: get(b"pbag", 4, 1)?.iter().map(|r| u16_at(r, 0)).collect(),
            pgen: gens(get(b"pgen", 4, 1)?),
            inst: get(b"inst", 22, 2)?.iter().map(|r| u16_at(r, 20)).collect(),
            ibag: get(b"ibag", 4, 1)?.iter().map(|r| u16_at(r, 0)).collect(),
            igen: gens(get(b"igen", 4, 1)?),
            shdr: get(b"shdr", 46, 1)?
                .iter()
                .map(|r| Shdr {
                    start: u32_at(r, 20),
                    end: u32_at(r, 24),
                    ls: u32_at(r, 28),
                    le: u32_at(r, 32),
                    rate: u32_at(r, 36),
                    pitch: r[40],
                    corr: r[41] as i8,
                    kind: u16_at(r, 44),
                })
                .collect(),
            file: f,
            smpl,
        })
    }

    /// Zones `a..b` of a bag list: each one's generators.
    fn zones<'a>(bags: &[u16], gens: &'a [Gen], a: usize, b: usize) -> Result<Vec<&'a [Gen]>, String> {
        (a..b)
            .map(|z| {
                let (g0, g1) =
                    (*bags.get(z).ok_or("bag index")? as usize, *bags.get(z + 1).ok_or("bag index")? as usize);
                gens.get(g0..g1).ok_or_else(|| "generator index".to_string())
            })
            .collect()
    }

    /// Preset `pi`'s zones, resolved: instrument generators absolute (a
    /// global zone's under each zone's own), the preset's added to them
    /// (its global zone's under each zone's own), ranges intersected.
    fn resolve(&self, pi: usize) -> Result<Vec<Raw>, String> {
        let (a, b) = (self.phdr[pi].3 as usize, self.phdr.get(pi + 1).ok_or("preset index")?.3 as usize);
        let mut out = Vec::new();
        let mut pglobal = Gens::none();
        for (k, z) in Sf2::zones(&self.pbag, &self.pgen, a, b)?.iter().enumerate() {
            if z.last().map(|g| g.0) != Some(G_INSTRUMENT) {
                if k == 0 {
                    z.iter().for_each(|&g| pglobal.put(g));
                }
                continue;
            }
            let mut pg = pglobal;
            z.iter().for_each(|&g| pg.put(g));
            let ii = pg.v[G_INSTRUMENT as usize] as u16 as usize;
            let (ia, ib) = (
                *self.inst.get(ii).ok_or("instrument index")? as usize,
                *self.inst.get(ii + 1).ok_or("instrument index")? as usize,
            );
            let mut iglobal = Gens::defaults();
            for (m, iz) in Sf2::zones(&self.ibag, &self.igen, ia, ib)?.iter().enumerate() {
                if iz.last().map(|g| g.0) != Some(G_SAMPLE) {
                    if m == 0 {
                        iz.iter().for_each(|&g| iglobal.put(g));
                    }
                    continue;
                }
                let mut ig = iglobal;
                iz.iter().for_each(|&g| ig.put(g));
                let full = (0u8, 127u8);
                let both = |op: usize| {
                    let (i, p) = (ig.range(op), if pg.has(op) { pg.range(op) } else { full });
                    (i.0.max(p.0), i.1.min(p.1).min(127))
                };
                let ((klo, khi), (vlo, vhi)) = (both(G_KEYS), both(G_VELS));
                if klo > khi || vlo > vhi {
                    continue;
                }
                let sample = ig.v[G_SAMPLE as usize] as u16 as usize;
                let Some(sh) = self.shdr.get(sample) else { continue };
                let add = |op: usize| ig.i(op) + pg.i(op);
                let root = match ig.i(G_ROOT) {
                    r @ 0..=127 => r as u8,
                    _ if sh.pitch <= 127 => sh.pitch,
                    _ => 60,
                };
                let addr = |fine: usize, coarse: usize| ig.i(fine) as i64 + 32768 * ig.i(coarse) as i64;
                out.push(Raw {
                    sample,
                    klo,
                    khi,
                    vlo,
                    vhi,
                    root,
                    tune: add(G_COARSE) as f32 + (add(G_FINE) as f32 + sh.corr as f32) / 100.0,
                    scale: add(G_SCALE) as f32 / 100.0,
                    mode: (ig.i(G_MODES) & 3) as u8,
                    offs: [
                        addr(G_START, G_START_COARSE),
                        addr(G_END, G_END_COARSE),
                        addr(G_LOOP_START, G_LOOP_START_COARSE),
                        addr(G_LOOP_END, G_LOOP_END_COARSE),
                    ],
                    atten: add(G_ATTEN),
                    pan: add(G_PAN),
                    excl: ig.i(G_EXCL).clamp(0, 255) as u8,
                    hold: add(G_HOLD),
                    decay: add(G_DECAY),
                    key_hold: add(G_KEY_HOLD),
                    key_decay: add(G_KEY_DECAY),
                    sus: add(G_SUSTAIN),
                    rel: add(G_RELEASE),
                });
            }
        }
        Ok(out)
    }

    /// A preset by name: in bank 128 for a kit, else bank 0 (or the lowest
    /// melodic bank that has it).
    fn find(&self, preset: &str, kit: bool) -> Option<usize> {
        let n = self.phdr.len() - 1;
        (0..n).filter(|&i| self.phdr[i].0 == preset && (self.phdr[i].2 == 128) == kit).min_by_key(|&i| self.phdr[i].2)
    }
}

/// The presets of a SoundFont: bank, program, name.
pub fn sf2_presets(path: &Path) -> Result<Vec<(u16, u16, String)>, String> {
    let s = Sf2::open(path)?;
    let mut v: Vec<(u16, u16, String)> = s.phdr[..s.phdr.len() - 1].iter().map(|p| (p.2, p.1, p.0.clone())).collect();
    v.sort();
    Ok(v)
}

/// Loads `melodic` presets and `kits` from a SoundFont into `b`: only the
/// samples they use, read from the file one by one.
fn load_sf2(b: &mut Bank, path: &Path, melodic: &[&str], kits: &[&str]) -> Result<(), String> {
    let mut s = Sf2::open(path)?;
    let mut want: Vec<(String, bool, Vec<Raw>)> = Vec::new();
    for (names, kit) in [(melodic, false), (kits, true)] {
        for &p in names {
            match s.find(p, kit).map(|i| s.resolve(i)) {
                Some(Ok(z)) if !z.is_empty() => want.push((p.to_string(), kit, z)),
                Some(Ok(_)) => b.warnings.push(format!("{p}: no zones")),
                Some(Err(e)) => b.warnings.push(format!("{p}: {e}")),
                None => b.warnings.push(format!("{p}: not in the SoundFont")),
            }
        }
    }
    // each sample read once: where it went in the data, its frames
    let mut loaded: HashMap<usize, Option<(usize, u32)>> = HashMap::new();
    let flen = s.file.metadata().map_err(|e| e.to_string())?.len();
    for (_, _, raws) in &want {
        for r in raws {
            if loaded.contains_key(&r.sample) {
                continue;
            }
            let sh = &s.shdr[r.sample];
            let ok =
                sh.kind & 0x8000 == 0 && sh.start < sh.end && sh.end as u64 <= s.smpl.1 && sh.end - sh.start <= 1 << 25;
            let got = if ok {
                let n = (sh.end - sh.start) as usize;
                read_at(&mut s.file, flen, s.smpl.0 + 2 * sh.start as u64, 2 * n).ok().map(|v| {
                    let off = b.data.len();
                    b.data.extend(v.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])));
                    (off, n as u32)
                })
            } else {
                None
            };
            loaded.insert(r.sample, got);
        }
    }
    let zone = |r: &Raw, s: &Sf2| -> Option<Zone> {
        let (off, slen) = loaded.get(&r.sample).copied().flatten()?;
        let sh = &s.shdr[r.sample];
        let slen = slen as i64;
        if slen < 4 {
            return None;
        }
        let start = r.offs[0].clamp(0, slen - 2);
        let end = (slen + r.offs[1]).clamp(start + 2, slen);
        let ls = sh.ls as i64 - sh.start as i64 + r.offs[2] - start;
        let le = sh.le as i64 - sh.start as i64 + r.offs[3] - start;
        let len = end - start;
        let looped = (r.mode == 1 || r.mode == 3) && ls >= 0 && ls + 2 <= le && le <= len;
        // FluidSynth's (and the EMU hardware's) attenuation: 0.4 of a centibel a unit
        let g = 10f32.powf(-(r.atten.clamp(0, 1440) as f32 * 0.4) / 200.0) / 32768.0;
        let pan = (r.pan as f32 / 500.0).clamp(-1.0, 1.0);
        Some(Zone {
            off: off + start as usize,
            len: len as u32,
            chans: 1,
            root: r.root as f32,
            tune: r.tune,
            scale: r.scale,
            rate: sh.rate.clamp(4000, 192_000) as f32 / RATE,
            ls: if looped { ls as u32 } else { 0 },
            le: if looped { le as u32 } else { 0 },
            mode: if looped { r.mode } else { 0 },
            xf: 0,
            gl: g * (1.0 - pan).min(1.0),
            gr: g * (1.0 + pan).min(1.0),
            excl: r.excl,
            hold_tc: r.hold as f32,
            dec_tc: r.decay as f32,
            key_hold: r.key_hold as f32,
            key_dec: r.key_decay as f32,
            sus: if r.sus >= 1000 { 0.0 } else { 10f32.powf(-(r.sus.max(0) as f32) / 200.0) },
            // -96 dB over the release (FluidSynth's): to -60 dB in 0.625 of it
            rel: (tc(r.rel as f32) * 0.625).clamp(0.003, 10.0),
        })
    };
    let cover = |r: &Raw| Cover { klo: r.klo, khi: r.khi, vlo: r.vlo, vhi: r.vhi, root: r.root, rr: 0 };
    for (p, kit, raws) in &want {
        if !kit {
            let (zones, covers): (Vec<Zone>, Vec<Cover>) =
                raws.iter().filter_map(|r| Some((zone(r, &s)?, cover(r)))).unzip();
            if zones.is_empty() {
                b.warnings.push(format!("{p}: no samples"));
                continue;
            }
            // the middle note: 60 if the preset has it, else its range's middle
            let mid = if covers.iter().any(|c| c.klo <= 60 && 60 <= c.khi) {
                60.0
            } else {
                let lo = covers.iter().map(|c| c.klo).min().unwrap_or(60) as f32;
                let hi = covers.iter().map(|c| c.khi).max().unwrap_or(60) as f32;
                ((lo + hi) / 2.0).round()
            };
            let id = b.add(zones, &covers, false, None);
            if b.normalise(id, mid, p) {
                b.name_gm(p, id);
            }
        } else {
            // each key its own instrument
            let mut keys = [0u16; 128];
            let first = b.zones.len();
            for (key, slot) in keys.iter_mut().enumerate() {
                let k = key as u8;
                let (zones, covers): (Vec<Zone>, Vec<Cover>) = raws
                    .iter()
                    .filter(|r| r.klo <= k && k <= r.khi)
                    .filter_map(|r| Some((zone(r, &s)?, cover(r))))
                    .unzip();
                if !zones.is_empty() {
                    *slot = b.add(zones, &covers, false, Some(k));
                }
            }
            // the kit as its maker balanced it, brought to the reference at
            // its snare (38; else the kick, else its first key)
            let Some(at) = [38, 36].into_iter().chain(0..128).find(|&k| keys[k] != 0) else {
                b.warnings.push(format!("{p}: no samples"));
                continue;
            };
            let rms = b.rms_1s(keys[at], at as f32);
            if !(rms > 1e-6) {
                b.warnings.push(format!("{p}: silent"));
                continue;
            }
            for z in &mut b.zones[first..] {
                z.gl *= REF_RMS / rms;
                z.gr *= REF_RMS / rms;
            }
            b.kits.push((p.clone(), keys));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- loading

/// Loads the samples directory: the manifest's WAVs and the SoundFont's
/// `GM_PRESETS` and `GM_KITS`. Either may be missing (a warning); an error
/// when neither loads.
pub fn load(dir: &Path) -> Result<Bank, String> {
    if !dir.is_dir() {
        return Err(format!("no directory {}", dir.display()));
    }
    let mut b = Bank::new();
    let (m, s) = (dir.join(MANIFEST), dir.join(SF2));
    let mut any = false;
    if m.is_file() {
        match load_multi(&mut b, dir) {
            Ok(()) => any = true,
            Err(e) => b.warnings.push(e),
        }
    } else {
        b.warnings.push(format!("no {MANIFEST}"));
    }
    if s.is_file() {
        match load_sf2(&mut b, &s, GM_PRESETS, GM_KITS) {
            Ok(()) => any = true,
            Err(e) => b.warnings.push(format!("{SF2}: {e}")),
        }
    } else {
        b.warnings.push(format!("no {SF2}"));
    }
    if !any || b.insts.is_empty() {
        return Err(format!("nothing loaded from {} ({})", dir.display(), b.warnings.join("; ")));
    }
    b.recorded();
    Ok(b)
}

/// SoundFont presets played by recordings when they are loaded: the
/// preset, the recording (a multisampled instrument and articulation), the
/// notes their levels are matched over (an octave either side of this),
/// how long a note to match them over, and whether the levels are then
/// matched note by note (`Bank::zone_gains`).
///
/// The electric basses (Karoryfer's); a clean electric guitar (for the
/// SoundFont's clean guitar and its brighter funk guitar, each at its own
/// levels) and the same one damped as it is played, a double bass plucked
/// and an acoustic guitar (Karoryfer's too); a nylon-strung guitar and an
/// electric guitar through an amplifier (FreePats'), the one for the
/// distorted guitar and the overdriven, each at its own levels.
///
/// The windows: a strummed or picked note's first 0.4 s (most are shorter
/// than a second, and the recordings speak harder than the SoundFont's:
/// over a whole second they would come out quieter where it matters); the
/// damped guitar's 0.15 s (ska's chop is 0.14 s long); an amplifier's
/// held notes a second.
type Standing = (&'static str, &'static str, &'static str, f32, f32, bool);
const GM_RECORDED: &[Standing] = &[
    ("Fingered Bass", "bass_finger", "pluck", 36.0, 1.0, false),
    ("Picked Bass", "bass_pick", "pluck", 36.0, 1.0, false),
    ("Clean Guitar", "guitar_clean", "pluck", 57.0, 0.4, true),
    ("Funk Guitar", "guitar_clean", "pluck", 57.0, 0.4, true),
    ("Palm Muted Guitar", "guitar_mute", "pluck", 52.0, 0.15, true),
    ("Acoustic Bass", "bass_upright", "pluck", 36.0, 0.4, true),
    ("Steel String Guitar", "guitar_steel", "pluck", 57.0, 0.4, true),
    ("Nylon String Guitar", "guitar_nylon", "pluck", 60.0, 0.4, true),
    ("Distortion Guitar", "guitar_dist", "pluck", 52.0, 1.0, true),
    ("Overdrive Guitar", "guitar_dist", "pluck", 52.0, 1.0, true),
];

/// The SoundFont kits a recorded kit (the manifest's instrument "kit", a
/// piece an articulation) plays in, and the piece for each General MIDI key.
/// Each SoundFont kit is kept as "KIT (SoundFont)" as well (synthwave's
/// 80s kick and gated snare).
const KITS_RECORDED: &[&str] = &["Standard", "Room", "Power", "Jazz"];
const KIT_KEYS: &[(u8, &str)] = &[
    (35, "kick"),
    (36, "kick"),
    (37, "stick"),
    (38, "snare"),
    (40, "snare"),
    (42, "hat"),
    (44, "hat_pedal"),
    (46, "hat_open"),
    (41, "tom_floor"),
    (43, "tom_lo"),
    (45, "tom_mid"),
    (47, "tom_mid"),
    (48, "tom_hi"),
    (50, "tom_hi"),
    (49, "crash"),
    (57, "crash"),
    (51, "ride"),
    (59, "ride"),
    (53, "ride_bell"),
];

static BANK: OnceLock<Bank> = OnceLock::new();
static CONFIG: OnceLock<(PathBuf, bool)> = OnceLock::new();
static LOAD: Once = Once::new();

/// Makes `bank` the one the music plays (the first installed stays).
pub fn install(bank: Bank) {
    let _ = BANK.set(bank);
}

/// Whether samples are installed.
pub fn ready() -> bool {
    installed().is_some()
}

thread_local! {
    /// The installed bank hidden from this thread for now (an engine set to
    /// the synthesiser alone composing or playing), and the network's
    /// instruments playing in the recordings' place (see `hidden`).
    static HIDDEN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static NETS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether the network's instruments play (where they have learned one) in
/// place of the recordings, for now.
#[inline]
pub fn nets_on() -> bool {
    NETS.with(|n| n.get())
}

/// The installed bank (an atomic load: the audio thread may ask); None
/// while `hidden` hides it.
#[inline]
pub fn bank() -> Option<&'static Bank> {
    if HIDDEN.with(|h| h.get()) { None } else { installed() }
}

#[cfg(not(target_arch = "wasm32"))]
#[inline]
fn installed() -> Option<&'static Bank> {
    BANK.get()
}

/// The browser's bank: built as its groups come (`Partial`), on the
/// worker's one thread.
#[cfg(target_arch = "wasm32")]
struct WebBank(std::cell::UnsafeCell<Option<Partial>>);

// SAFETY: wasm32-unknown-unknown runs one thread
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WebBank {}

#[cfg(target_arch = "wasm32")]
static WEB: WebBank = WebBank(std::cell::UnsafeCell::new(None));

#[cfg(target_arch = "wasm32")]
#[inline]
fn installed() -> Option<&'static Bank> {
    // SAFETY: one thread, and the bank changes only between calls into the
    // module (`web`), never while anything holds it
    unsafe { (*WEB.0.get()).as_ref().map(|p| &p.bank) }
}

/// The browser's bank, to build.
///
/// # Safety
/// Only between calls into the module that play or compose (nothing may
/// hold the bank meanwhile).
#[cfg(target_arch = "wasm32")]
pub unsafe fn web() -> &'static mut Option<Partial> {
    // SAFETY: the caller's
    unsafe { &mut *WEB.0.get() }
}

/// `f`, with the bank hidden from it (`hide`: what an engine composes and
/// plays with when the synthesiser alone is asked of it) or not, and the
/// network's instruments in the recordings' place (`nets`) or not.
pub fn hidden<R>(hide: bool, nets: bool, f: impl FnOnce() -> R) -> R {
    let was = (HIDDEN.with(|h| h.replace(hide)), NETS.with(|n| n.replace(nets)));
    let r = f();
    HIDDEN.with(|h| h.set(was.0));
    NETS.with(|n| n.set(was.1));
    r
}

/// `Bank::multi` of the installed bank (None: no samples).
#[allow(dead_code)] // the genres' hook
pub fn multi(instrument: &str, articulation: &str) -> Option<u16> {
    bank()?.multi(instrument, articulation)
}

/// `Bank::gm` of the installed bank.
#[allow(dead_code)] // the genres' hook
pub fn gm(preset: &str) -> Option<u16> {
    bank()?.gm(preset)
}

/// `Bank::stands_in` of the installed bank.
pub fn stands_in(preset: &str) -> bool {
    bank().is_some_and(|b| b.stands_in(preset))
}

/// `Bank::drum` of the installed bank.
#[allow(dead_code)] // the genres' hook
pub fn drum(kit: &str, key: u8) -> Option<u16> {
    bank()?.drum(kit, key)
}

/// `--samples DIR` (None: `default_dir`) and
/// `--no-samples`, before anything loads.
pub fn configure(dir: Option<String>, off: bool) {
    let dir = dir.map(PathBuf::from).unwrap_or_else(default_dir);
    let _ = CONFIG.set((dir, off));
}

/// `~/.local/share/musicbox/samples`, or fleece's, which holds the same
/// recordings, when only that one is there.
pub fn default_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let own = Path::new(&home).join(".local/share/musicbox/samples");
    let fleece = Path::new(&home).join(".local/share/fleece/samples");
    if !own.exists() && fleece.is_dir() { fleece } else { own }
}

fn config() -> &'static (PathBuf, bool) {
    CONFIG.get_or_init(|| (default_dir(), false))
}

/// Copies the samples directory's SOURCES.md and licenses/ (each source's
/// licence and readme, whose notices go wherever its recordings go) into
/// `out`, for the browser's copy; returns how many files.
pub fn copy_licences(out: &Path) -> Result<usize, String> {
    let dir = &config().0;
    let err = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    std::fs::copy(dir.join("SOURCES.md"), out.join("SOURCES.md")).map_err(|e| err(&dir.join("SOURCES.md"), e))?;
    let mut n = 1;
    let lic = dir.join("licenses");
    for src in std::fs::read_dir(&lic).map_err(|e| err(&lic, e))? {
        let src = src.map_err(|e| err(&lic, e))?.path();
        let Some(name) = src.file_name().filter(|_| src.is_dir()) else { continue };
        let to = out.join("licenses").join(name);
        std::fs::create_dir_all(&to).map_err(|e| err(&to, e))?;
        for f in std::fs::read_dir(&src).map_err(|e| err(&src, e))? {
            let f = f.map_err(|e| err(&src, e))?.path();
            if let Some(file) = f.file_name().filter(|_| f.is_file()) {
                std::fs::copy(&f, to.join(file)).map_err(|e| err(&f, e))?;
                n += 1;
            }
        }
    }
    Ok(n)
}

/// Loads and installs the samples now (once; later calls wait for it):
/// for `--musicwav` and a video's soundtrack, so a seed gives the same
/// music. A missing directory is one line on stderr, and the synthesiser.
pub fn load_now() -> bool {
    LOAD.call_once(|| {
        // the tunes networks (the recordings or not)
        #[cfg(not(target_arch = "wasm32"))]
        for w in crate::tunes::load(&crate::net::default_dir()) {
            eprintln!("musicbox: tunes: {w}");
        }
        let (dir, off) = config();
        if *off {
            return;
        }
        if !dir.is_dir() {
            eprintln!(
                "musicbox: no samples in {} (tools/get-samples.sh fetches them): the music is all synthesiser",
                dir.display()
            );
            return;
        }
        let t0 = Instant::now();
        match load(dir) {
            Ok(b) => {
                let (m, g, k) = b.counts();
                eprintln!(
                    "musicbox: samples: {m} multisampled instruments, {g} presets, {k} kits ({:.0} MB) in {:.1} s{}",
                    b.bytes() as f64 / 1e6,
                    t0.elapsed().as_secs_f64(),
                    if b.warnings.is_empty() {
                        String::new()
                    } else {
                        format!("; {} skipped (--samples-list)", b.warnings.len())
                    }
                );
                install(b);
                // the network's instruments, for the recordings they learned
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(b) = bank() {
                    let dir = crate::net::default_dir();
                    let (n, skipped) = crate::net::load(&dir, b);
                    if n > 0 || !skipped.is_empty() {
                        eprintln!("musicbox: {n} network instruments from {}", dir.display());
                    }
                    for w in skipped {
                        eprintln!("musicbox: networks: {w}");
                    }
                }
            }
            Err(e) => eprintln!("musicbox: samples: {e}: the music is all synthesiser"),
        }
    });
    ready()
}

/// `--samples-list`: the SoundFont's presets, then what loads, and what not.
pub fn list_cli() {
    let (dir, _) = config();
    println!("samples directory: {}", dir.display());
    match sf2_presets(&dir.join(SF2)) {
        Ok(v) => {
            println!("{SF2}: {} presets (bank program name):", v.len());
            for (bank, prog, name) in v {
                println!("  {bank:3} {prog:3} {name}");
            }
        }
        Err(e) => println!("{SF2}: {e}"),
    }
    let t0 = Instant::now();
    match load(dir) {
        Ok(b) => {
            println!("loaded in {:.2} s, {:.1} MB", t0.elapsed().as_secs_f64(), b.bytes() as f64 / 1e6);
            for (i, a, id) in &b.multis {
                let inst = &b.insts[*id as usize - 1];
                let (z, l) = (inst.zones.end - inst.zones.start, inst.layers);
                if let Some((s, l, g)) = inst.auto {
                    let art = |id: u16| b.multis.iter().find(|m| m.2 == id).map_or("?", |m| m.1.as_str());
                    println!(
                        "  multi {i:12} {a:18} gates under {AUTO_SHORT} s {} ({:+.1} dB), else {}",
                        art(s),
                        20.0 * g.log10(),
                        art(l)
                    );
                } else {
                    println!("  multi {i:12} {a:18} {z} samples, {l} velocity layers");
                }
            }
            for (p, id) in &b.gms {
                let inst = &b.insts[*id as usize - 1];
                println!("  gm    {p:20} {} zones, {} velocity layers", inst.zones.end - inst.zones.start, inst.layers);
            }
            for (k, keys) in &b.kits {
                let ks: Vec<String> = (0..128).filter(|&i| keys[i] != 0).map(|i| i.to_string()).collect();
                println!("  kit   {k:8} keys {}", ks.join(" "));
            }
            for w in &b.warnings {
                println!("  skipped: {w}");
            }
        }
        Err(e) => println!("nothing loaded: {e} (tools/get-samples.sh fetches the samples)"),
    }
}

// ---------------------------------------------------------------- what the network learns from

/// The velocities `voice_data` plays each note at.
const DATA_VELS: [f32; 4] = [0.3, 0.55, 0.8, 1.0];
/// A note as `voice_data` plays it: held, let go, then silence (seconds).
const DATA_HOLD: f32 = 3.0;
const DATA_GO: f32 = 0.25;
const DATA_REST: f32 = 0.25;

/// `--voice-data DIR NAME...`: each instrument named ("multi violin sus",
/// "gm Nylon String Guitar") played by the sampler as a voice plays it, at
/// every note of its range (its lowest root a fifth down to its highest a
/// fifth up, every `step` semitones) and at `DATA_VELS`, for tools/neural to
/// learn: DIR/NAME.wav (mono, 48 kHz, the notes one after another) and
/// DIR/NAME.tsv (note, velocity, first frame, frames held, frames).
pub fn voice_data(dir: &Path, names: &[String], step: usize) -> Result<(), String> {
    let b = bank().ok_or("no samples loaded")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for name in names {
        let w: Vec<&str> = name.splitn(3, ' ').collect();
        let id = match w.as_slice() {
            ["multi", inst, art] => b.multi(inst, art),
            ["gm", preset] => b.gm(preset),
            ["gm", a, rest] => b.gm(&format!("{a} {rest}")),
            _ => None,
        }
        .ok_or_else(|| format!("{name}: no such instrument (multi INSTRUMENT ARTICULATION, or gm PRESET)"))?;
        let inst = &b.insts[id as usize - 1];
        let r = match inst.auto {
            Some((_, long, _)) => b.insts[long as usize - 1].zones.clone(),
            None => inst.zones.clone(),
        };
        let roots: Vec<f32> = b.zones[r.start as usize..r.end as usize].iter().map(|z| z.root).collect();
        let lo = roots.iter().copied().fold(f32::MAX, f32::min).max(12.0) as i32 - 7;
        let hi = roots.iter().copied().fold(f32::MIN, f32::max).min(115.0) as i32 + 7;
        let (hold, go, rest) = ((DATA_HOLD * RATE) as usize, (DATA_GO * RATE) as usize, (DATA_REST * RATE) as usize);
        let mut pcm: Vec<i16> = Vec::new();
        let mut tsv = String::from("note\tvelocity\tfirst\theld\tframes\n");
        for note in (lo..=hi).step_by(step.max(1)) {
            for vel in DATA_VELS {
                let mut s = SAMPLER;
                if !s.start(b, id, note as f32, vel, hold as i64, 0) {
                    continue;
                }
                let first = pcm.len();
                let mut buf = [[0.0f32; 2]; 64];
                let mut env = 1.0f32;
                // the release as a voice has it: the instrument's own, to -60 dB
                let fall = (-6.9 / (s.rel.max(0.003) * RATE)).exp();
                for k in 0..(hold + go) / 64 {
                    let gate = k * 64 < hold;
                    s.fill(b, s.key, gate, &mut buf);
                    for x in &buf {
                        if !gate {
                            env *= fall;
                        }
                        let m = 0.5 * (x[0] + x[1]) * env;
                        pcm.push((m * 32768.0).round().clamp(-32768.0, 32767.0) as i16);
                    }
                }
                tsv += &format!("{note}\t{vel}\t{first}\t{hold}\t{}\n", pcm.len() - first);
                pcm.resize(pcm.len() + rest, 0);
            }
        }
        let file = name.replace(' ', "_");
        let mut wv = Vec::with_capacity(44 + 2 * pcm.len());
        wv.extend_from_slice(b"RIFF");
        wv.extend_from_slice(&(36 + 2 * pcm.len() as u32).to_le_bytes());
        wv.extend_from_slice(b"WAVEfmt ");
        for x in [16u32, 1 | 1 << 16, RATE as u32, 2 * RATE as u32, 2 | 16 << 16] {
            wv.extend_from_slice(&x.to_le_bytes());
        }
        wv.extend_from_slice(b"data");
        wv.extend_from_slice(&(2 * pcm.len() as u32).to_le_bytes());
        for s in &pcm {
            wv.extend_from_slice(&s.to_le_bytes());
        }
        for (ext, bytes) in [("wav", wv), ("tsv", tsv.into_bytes())] {
            let path = dir.join(format!("{file}.{ext}"));
            std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        eprintln!("{name}: notes {lo}-{hi}, {:.0} s", pcm.len() as f32 / RATE);
    }
    Ok(())
}

// ---------------------------------------------------------------- the browser's copy

// The browser cannot fetch a gigabyte before it plays: `--web-pack` writes
// the bank's tables (pack.bin) and its recordings in groups (a multisampled
// instrument's articulations, a preset, a kit), each group one stream at the
// engine's rate (raw/N.wav: its mono recordings first, then its stereo ones,
// in stereo, the mono ones' two channels the same; mono when it has no
// stereo ones), for tools/web-samples.sh to compress.
// The browser loads the tables, then each group before anything plays it:
// until its stream is in, a group's instruments have no names (the genres'
// hooks find none, and the synthesiser plays their parts).
// A phone cannot hold a genre's half a gigabyte either: a lighter copy goes
// with it (lite/: its own tables and streams), every recording at half the
// engine's rate and in one channel, a quarter of the memory or less. Its
// zones say their rate, and are played as the SoundFont's recordings at
// rates of their own are.

/// The tables' first bytes: as they were (every recording at the engine's
/// rate), or followed by the recordings' rate over the engine's.
const PACK_MAGIC: &[u8; 4] = b"MBP2";
const PACK_MAGIC_AT: &[u8; 4] = b"MBP3";
/// The lighter copy's rate over the engine's.
pub const PACK_LITE: f32 = 0.5;
/// Silence between a group's recordings (frames): a lossy codec's smear
/// kept off the next one's attack.
const PACK_GAP: usize = 2048;
/// The most times a loop is repeated to make it a whole number of frames
/// at the engine's rate, and the most it comes to (seconds): a longer loop
/// is taken as it is, its seam rarer.
const PACK_REPEATS: usize = 16;
const PACK_UNROLL: f32 = 0.09;

/// A group of instruments loaded together, with the names it gives them.
#[derive(Clone, Debug, Default)]
pub struct Group {
    pub name: String,
    insts: Vec<u16>,
    multis: Vec<(String, String, u16)>,
    gms: Vec<(String, u16)>,
    kits: Vec<(String, [u16; 128])>,
    /// its stream's frames, and how many of them, first, are its mono
    /// recordings'
    pub frames: u32,
    pub mono: u32,
}

thread_local! {
    /// Picked ids counted on this thread (`--web-pack`'s census of what each
    /// genre plays, a genre or mix a thread), when counting.
    static SEEN: std::cell::RefCell<Option<Vec<bool>>> = const { std::cell::RefCell::new(None) };
}

/// Counting the ids picked on this thread on (or off), and those picked
/// since it began.
pub fn census(on: bool) -> Vec<u16> {
    let was = SEEN.with(|s| s.replace(on.then(|| vec![false; 1 << 16])));
    was.map_or_else(Vec::new, |v| (0..=u16::MAX).filter(|&id| v[id as usize]).collect())
}

fn seen(id: u16) {
    SEEN.with(|s| {
        if let Some(v) = s.borrow_mut().as_mut() {
            v[id as usize] = true;
        }
    });
}

struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, x: u8) {
        self.0.push(x);
    }
    fn u16(&mut self, x: u16) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn u32(&mut self, x: u32) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn f32(&mut self, x: f32) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u16(s.len() as u16);
        self.0.extend_from_slice(s.as_bytes());
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
struct In<'a> {
    b: &'a [u8],
    at: usize,
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
impl In<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let s = self.b.get(self.at..self.at + n).ok_or("pack: cut short")?;
        self.at += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("2")))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    fn f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }
    fn str(&mut self) -> Result<String, String> {
        let n = self.u16()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "pack: a name not UTF-8".to_string())
    }
    /// A count, each item at least `min` bytes (a count the bytes cannot
    /// hold is refused before anything is allocated).
    fn count(&mut self, min: usize) -> Result<usize, String> {
        let n = self.u32()? as usize;
        if n.saturating_mul(min) > self.b.len() - self.at {
            return Err("pack: a count past its end".into());
        }
        Ok(n)
    }
}

/// `x` (one channel) at `rate` times the engine's, at the engine's: a
/// Kaiser-windowed sinc, 16 zero crossings a side, cut below the lower
/// Nyquist.
fn resample(x: &[f32], rate: f32) -> Vec<f32> {
    const W: usize = 16;
    const RES: usize = 256;
    let i0 = |x: f64| {
        let (mut s, mut t) = (1.0, 1.0);
        for k in 1..30 {
            t *= (x / 2.0) / k as f64;
            s += t * t;
        }
        s
    };
    let beta = 8.0;
    let table: Vec<f64> = (0..=W * RES)
        .map(|k| {
            let u = k as f64 / RES as f64;
            let sinc = if k == 0 { 1.0 } else { (std::f64::consts::PI * u).sin() / (std::f64::consts::PI * u) };
            let w = (u / W as f64).min(1.0);
            sinc * i0(beta * (1.0 - w * w).sqrt()) / i0(beta)
        })
        .collect();
    let step = rate as f64;
    let fc = (1.0 / step).min(1.0);
    let n = (x.len() as f64 / step).floor() as usize;
    let reach = (W as f64 / fc).ceil() as isize;
    (0..n)
        .map(|i| {
            let t = i as f64 * step;
            let c = t.floor() as isize;
            let mut acc = 0.0;
            for k in c - reach + 1..=c + reach {
                if k < 0 || k as usize >= x.len() {
                    continue;
                }
                let u = (t - k as f64).abs() * fc * RES as f64;
                let j = u as usize;
                if j >= W * RES {
                    continue;
                }
                let h = table[j] + (table[j + 1] - table[j]) * (u - j as f64);
                acc += x[k as usize] as f64 * h;
            }
            (acc * fc) as f32
        })
        .collect()
}

impl Bank {
    /// The groups: each multisampled instrument (its articulations), each
    /// preset, each kit; and each instrument's group (the first to hold it).
    fn groups(&self) -> (Vec<Group>, Vec<Option<usize>>) {
        let mut gs: Vec<Group> = Vec::new();
        for m in &self.multis {
            let name = format!("multi {}", m.0);
            let g = match gs.iter().position(|g| g.name == name) {
                Some(g) => g,
                None => {
                    gs.push(Group { name, ..Group::default() });
                    gs.len() - 1
                }
            };
            gs[g].multis.push(m.clone());
            gs[g].insts.push(m.2);
            if let Some((s, l, _)) = self.insts[m.2 as usize - 1].auto {
                gs[g].insts.extend([s, l]);
            }
        }
        for (name, id) in &self.gms {
            gs.push(Group {
                name: format!("gm {name}"),
                insts: vec![*id],
                gms: vec![(name.clone(), *id)],
                ..Group::default()
            });
        }
        for (name, keys) in &self.kits {
            let insts = keys.iter().copied().filter(|&k| k != 0).collect();
            gs.push(Group {
                name: format!("kit {name}"),
                insts,
                kits: vec![(name.clone(), *keys)],
                ..Group::default()
            });
        }
        let mut owner = vec![None; self.insts.len()];
        for (g, gr) in gs.iter_mut().enumerate() {
            gr.insts.sort_unstable();
            gr.insts.dedup();
            for &id in &gr.insts {
                owner[id as usize - 1].get_or_insert(g);
            }
        }
        (gs, owner)
    }

    /// A zone's recording as the browser's copy has it: its channels as
    /// recorded (one, or two interleaved), at `tr` times the engine's rate
    /// (1: the engine's; less: the lighter copy's, its two channels as one),
    /// cut where it falls silent; its loop (start, end) there. A loop
    /// without a crossfade (the SoundFont's, a few periods long) is played
    /// out a few times more first, so that its seam lands inside the
    /// compressed stream (its error the codec's noise, not a click), and
    /// looped over as many repeats as come to a whole number of frames.
    fn pack_zone(&self, z: &Zone, tr: f32) -> (Vec<f32>, u32, u32) {
        let c = z.chans as usize;
        let mut chans: Vec<Vec<f32>> =
            (0..c).map(|ch| self.frames(z).chunks_exact(c).map(|f| f[ch] as f32).collect()).collect();
        if tr < 1.0 && c == 2 {
            let both: Vec<f32> = chans[0].iter().zip(&chans[1]).map(|(a, b)| (a + b) * 0.5).collect();
            chans = vec![both];
        }
        let k = tr as f64 / z.rate as f64;
        let all = chans[0].len();
        let (ls, le) = (z.ls as usize, (z.le as usize).min(all));
        let (mut lsx, mut lex) = (ls as f64 * k, le as f64 * k);
        let mut ring = (le - ls.min(le)) as f64 * k;
        if z.mode != 0 && z.xf == 0 && le > ls + 1 {
            let n = le - ls;
            let off = |m: usize| (m as f64 * n as f64 * k - (m as f64 * n as f64 * k).round()).abs();
            let most = ((PACK_UNROLL * z.rate * RATE) as usize / n).clamp(1, PACK_REPEATS);
            let m = (1..=most).min_by(|&a, &b| off(a).total_cmp(&off(b))).unwrap_or(1);
            for x in chans.iter_mut() {
                let lp: Vec<f32> = x[ls..le].to_vec();
                let tail: Vec<f32> = x[le..].to_vec();
                x.truncate(le);
                for _ in 0..m + 1 {
                    x.extend_from_slice(&lp);
                }
                x.extend_from_slice(&tail);
            }
            lsx = le as f64 * k;
            lex = lsx + (m * n) as f64 * k;
            ring = n as f64 * k;
        }
        if (z.rate / tr - 1.0).abs() > 1e-6 {
            for x in chans.iter_mut() {
                *x = resample(x, z.rate / tr);
            }
        }
        let (ls2, le2) = (lsx.round() as u32, lsx.round() as u32 + (lex - lsx).round() as u32);
        // to where it falls under -60 dB of its peak (all of a release, a
        // piano's or a gong's whole ring); a loop held for ever, to its end
        let peak = chans.iter().flatten().fold(0.0f32, |m, &s| m.max(s.abs()));
        let quiet = chans
            .iter()
            .filter_map(|x| x.iter().rposition(|&s| s.abs() > peak * 1e-3))
            .max()
            .map_or(0, |i| i + (480.0 * tr) as usize);
        let looped = (le2 as f64 + ring).ceil() as usize + 4;
        let keep = match z.mode {
            1 => looped,
            3 => quiet.max(looped),
            _ => quiet,
        };
        let len = keep.max(2).min(chans[0].len());
        let x: Vec<f32> = (0..len).flat_map(|i| chans.iter().map(move |x| x[i])).collect();
        if z.mode == 0 || le2 as usize > len || ls2 + 2 > le2 {
            return (x, 0, 0);
        }
        (x, ls2, le2)
    }

    /// Writes the browser's copy into `dir` (pack.bin, raw/N.wav) and its
    /// lighter copy into `dir`/lite, and returns their index (pack.json): the
    /// groups (each one's frames in both), those each genre in `used` (its
    /// name, the ids it picked) loads, and those each mix ("a+b") loads
    /// beyond its two genres'.
    pub fn web_pack(&self, dir: &Path, used: &[(String, Vec<u16>)]) -> Result<String, String> {
        let (groups, owner, deps) = self.pack_at(dir, 1.0)?;
        let (lite, ..) = self.pack_at(&dir.join("lite"), PACK_LITE)?;
        // the index: each group's file and the groups it needs; each genre's groups
        let list = |v: &[usize]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let mut j = String::from("{\"groups\":[\n");
        for (g, gr) in groups.iter().enumerate() {
            j += &format!(
                "{{\"name\":\"{}\",\"file\":\"{g}.opus\",\"frames\":{},\"mono\":{},\"lite\":{},\"deps\":[{}]}}{}\n",
                esc(&gr.name),
                gr.frames,
                gr.mono,
                lite[g].frames,
                list(&deps[g]),
                if g + 1 < groups.len() { "," } else { "" }
            );
        }
        // each genre's groups and those they need; each mix's that neither
        // of its genres has (most have none)
        let need = |ids: &[u16]| {
            let mut gs: Vec<usize> =
                ids.iter().filter_map(|&id| owner.get((id as usize).checked_sub(1)?).copied().flatten()).collect();
            let mut i = 0;
            while i < gs.len() {
                let more = deps[gs[i]].clone();
                gs.extend(more);
                i += 1;
            }
            gs.sort_unstable();
            gs.dedup();
            gs
        };
        let genres: Vec<(&String, Vec<usize>)> =
            used.iter().filter(|(name, _)| !name.contains('+')).map(|(name, ids)| (name, need(ids))).collect();
        let mut mixes = Vec::new();
        for (name, ids) in used.iter().filter(|(name, _)| name.contains('+')) {
            let has = |part: &str| genres.iter().find(|(g, _)| g.as_str() == part).map_or(&[][..], |(_, gs)| &gs[..]);
            let (a, b) = name.split_once('+').expect("a mix");
            let gs: Vec<usize> = need(ids).into_iter().filter(|g| !has(a).contains(g) && !has(b).contains(g)).collect();
            if !gs.is_empty() {
                mixes.push((name, gs));
            }
        }
        let entries = |v: &[(&String, Vec<usize>)]| {
            let lines: Vec<String> = v.iter().map(|(name, gs)| format!("\"{}\":[{}]", esc(name), list(gs))).collect();
            lines.join(",\n")
        };
        j += &format!("],\"genres\":{{\n{}\n}},\"mixes\":{{\n{}\n}}}}\n", entries(&genres), entries(&mixes));
        Ok(j)
    }

    /// The browser's copy at `tr` times the engine's rate, written into
    /// `dir` (pack.bin, raw/N.wav): its groups, each instrument's group, and
    /// the groups each needs first.
    #[allow(clippy::type_complexity)]
    fn pack_at(&self, dir: &Path, tr: f32) -> Result<(Vec<Group>, Vec<Option<usize>>, Vec<Vec<usize>>), String> {
        let lite = tr < 1.0;
        let gap = (PACK_GAP as f32 * tr) as usize;
        let raw = dir.join("raw");
        std::fs::create_dir_all(&raw).map_err(|e| format!("{}: {e}", raw.display()))?;
        let (mut groups, owner) = self.groups();
        let mut zones = self.zones.clone();
        let mut src = vec![0u16; zones.len()];
        // a recording (where, how long, channels, looping, loop) to where it
        // went: its group, its part of that group's stream (mono or stereo)
        // and where in that part, how long, the loop
        type Recording = (usize, u32, u8, u8, u32, u32);
        let mut done: HashMap<Recording, (usize, u8, u32, u32, u32, u32)> = HashMap::new();
        let mut deps: Vec<Vec<usize>> = vec![Vec::new(); groups.len()];
        // each group's mono part (frames, the gap after it included): where
        // its stereo part begins
        let mut monos = vec![0u32; groups.len()];
        for (g, gr) in groups.iter_mut().enumerate() {
            // each zone's recording, at the engine's rate, cut to what plays,
            // among the group's mono or stereo ones; a recording zones share,
            // once (in the first group to have it, which the others then need)
            let (mut mono, mut stereo): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
            let mut placed: Vec<(usize, usize, u8, u32)> = Vec::new();
            for &id in &gr.insts {
                let o = owner[id as usize - 1].expect("an owner");
                if o != g {
                    if !deps[g].contains(&o) {
                        deps[g].push(o);
                    }
                    continue;
                }
                let r = self.insts[id as usize - 1].zones.clone();
                for zi in r.start as usize..r.end as usize {
                    let z = self.zones[zi];
                    let key = (z.off, z.len, z.chans, z.mode, z.ls, z.le);
                    let (h, chans, off, len, ls, le) = match done.get(&key) {
                        Some(&d) => d,
                        None => {
                            let (x, ls, le) = self.pack_zone(&z, tr);
                            let pc = if lite { 1 } else { z.chans };
                            let part = if pc == 2 { &mut stereo } else { &mut mono };
                            let c = pc as usize;
                            if !part.is_empty() {
                                part.resize(part.len() + c * gap, 0.0);
                            }
                            let d = (g, pc, (part.len() / c) as u32, (x.len() / c) as u32, ls, le);
                            part.extend_from_slice(&x);
                            done.insert(key, d);
                            d
                        }
                    };
                    src[zi] = h as u16;
                    if h != g && !deps[g].contains(&h) {
                        deps[g].push(h);
                    }
                    placed.push((zi, h, chans, off));
                    let mut nz = Zone { len, chans, rate: tr, ls, le, ..z };
                    if z.mode != 0 && le <= ls {
                        nz.mode = 0;
                    }
                    nz.xf = (z.xf as f32 * tr / z.rate).round() as u32;
                    zones[zi] = nz;
                }
            }
            // the stream: the mono part (each frame's two channels the same),
            // then the stereo part; each zone where its recording is in it
            if !mono.is_empty() && !stereo.is_empty() {
                mono.resize(mono.len() + gap, 0.0);
            }
            monos[g] = mono.len() as u32;
            for (zi, h, chans, off) in placed {
                zones[zi].off = (if chans == 2 { monos[h] + off } else { off }) as usize;
            }
            gr.mono = monos[g];
            gr.frames = monos[g] + (stereo.len() / 2) as u32;
            // (all mono: a mono stream, half the browser's to decode)
            let c = if stereo.is_empty() { 1 } else { 2 };
            let pcm: Vec<f32> = mono.iter().flat_map(|&s| [s; 2].into_iter().take(c)).chain(stereo).collect();
            let mut w = Vec::with_capacity(44 + 2 * pcm.len());
            w.extend_from_slice(b"RIFF");
            w.extend_from_slice(&(36 + 2 * pcm.len() as u32).to_le_bytes());
            w.extend_from_slice(b"WAVEfmt ");
            let (c, block, rate) = (c as u32, 2 * c as u32, (RATE * tr) as u32);
            for x in [16u32, 1 | c << 16, rate, block * rate, block | 16 << 16] {
                w.extend_from_slice(&x.to_le_bytes());
            }
            w.extend_from_slice(b"data");
            w.extend_from_slice(&(2 * pcm.len() as u32).to_le_bytes());
            for s in &pcm {
                w.extend_from_slice(&(s.round().clamp(-32768.0, 32767.0) as i16).to_le_bytes());
            }
            let path = raw.join(format!("{g}.wav"));
            std::fs::write(&path, w).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        let mut o = Out(if lite { PACK_MAGIC_AT } else { PACK_MAGIC }.to_vec());
        if lite {
            o.f32(tr);
        }
        o.u32(zones.len() as u32);
        for (z, &h) in zones.iter().zip(&src) {
            o.u16(h);
            o.u32(z.off as u32);
            o.u32(z.len);
            o.u8(z.chans);
            for x in [z.root, z.tune, z.scale] {
                o.f32(x);
            }
            o.u32(z.ls);
            o.u32(z.le);
            o.u8(z.mode);
            o.u32(z.xf);
            for x in [z.gl, z.gr] {
                o.f32(x);
            }
            o.u8(z.excl);
            for x in [z.hold_tc, z.dec_tc, z.key_hold, z.key_dec, z.sus, z.rel] {
                o.f32(x);
            }
        }
        o.u32(self.picks.len() as u32);
        for &p in &self.picks {
            o.u32(p);
        }
        o.u32(self.insts.len() as u32);
        for i in &self.insts {
            o.0.extend_from_slice(&i.layer);
            o.u16(i.layers as u16);
            o.u32(i.cells.len() as u32);
            for c in &i.cells {
                o.u32(c.first);
                o.u8(c.rr);
                o.u8(c.per);
            }
            o.u8(i.key.unwrap_or(255));
            match i.auto {
                Some((s, l, g)) => {
                    o.u8(1);
                    o.u16(s);
                    o.u16(l);
                    o.f32(g);
                }
                None => o.u8(0),
            }
            o.u32(i.zones.start);
            o.u32(i.zones.end);
        }
        o.u32(groups.len() as u32);
        for gr in &groups {
            o.str(&gr.name);
            o.u32(gr.frames);
            o.u32(gr.mono);
            o.u32(gr.insts.len() as u32);
            for &id in &gr.insts {
                o.u16(id);
            }
            o.u32(gr.multis.len() as u32);
            for (i, a, id) in &gr.multis {
                o.str(i);
                o.str(a);
                o.u16(*id);
            }
            o.u32(gr.gms.len() as u32);
            for (n, id) in &gr.gms {
                o.str(n);
                o.u16(*id);
            }
            o.u32(gr.kits.len() as u32);
            for (n, keys) in &gr.kits {
                o.str(n);
                for &k in keys {
                    o.u16(k);
                }
            }
        }
        let path = dir.join("pack.bin");
        std::fs::write(&path, &o.0).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok((groups, owner, deps))
    }
}

/// The bank as the browser builds it: the tables at once (`pack.bin`), each
/// group's recordings as they come.
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
pub struct Partial {
    pub bank: Bank,
    pub groups: Vec<Group>,
    /// each zone's frames, until its group is in (0 meanwhile: silent), and
    /// the group whose stream holds its recording (`Zone::off` the frame it
    /// begins on there, until then)
    lens: Vec<u32>,
    srcs: Vec<u16>,
    /// where each group's stream begins in the bank's data, once it is in;
    /// the group being filled in, and where
    bases: Vec<Option<usize>>,
    filling: Option<(usize, usize)>,
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
impl Partial {
    pub fn new(meta: &[u8]) -> Result<Partial, String> {
        let mut r = In { b: meta, at: 0 };
        let rate = match r.take(4)? {
            m if m == PACK_MAGIC => 1.0,
            m if m == PACK_MAGIC_AT => r.f32()?,
            _ => return Err("not a musicbox pack".into()),
        };
        if !(rate > 0.01 && rate <= 1.0) {
            return Err(format!("pack: recordings at {rate} of the engine's rate"));
        }
        let mut b = Bank::new();
        let n = r.count(73)?;
        let (mut lens, mut srcs) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for _ in 0..n {
            srcs.push(r.u16()?);
            let off = r.u32()? as usize;
            lens.push(r.u32()?);
            let chans = r.u8()?;
            if !(1..=2).contains(&chans) {
                return Err(format!("pack: a recording of {chans} channels"));
            }
            let (root, tune, scale) = (r.f32()?, r.f32()?, r.f32()?);
            let (ls, le, mode, xf) = (r.u32()?, r.u32()?, r.u8()?, r.u32()?);
            let (gl, gr, excl) = (r.f32()?, r.f32()?, r.u8()?);
            let mut e = [0.0f32; 6];
            for x in e.iter_mut() {
                *x = r.f32()?;
            }
            b.zones.push(Zone {
                off,
                len: 0,
                chans,
                root,
                tune,
                scale,
                rate,
                ls,
                le,
                mode,
                xf,
                gl,
                gr,
                excl,
                hold_tc: e[0],
                dec_tc: e[1],
                key_hold: e[2],
                key_dec: e[3],
                sus: e[4],
                rel: e[5],
            });
        }
        let n = r.count(4)?;
        for _ in 0..n {
            b.picks.push(r.u32()?);
        }
        let n = r.count(140)?;
        for _ in 0..n {
            let layer: [u8; 128] = r.take(128)?.try_into().expect("128");
            let layers = r.u16()? as usize;
            let nc = r.count(6)?;
            let mut cells = Vec::with_capacity(nc);
            for _ in 0..nc {
                cells.push(Cell { first: r.u32()?, rr: r.u8()?, per: r.u8()? });
            }
            let key = Some(r.u8()?).filter(|&k| k < 128);
            let auto = if r.u8()? == 1 { Some((r.u16()?, r.u16()?, r.f32()?)) } else { None };
            let zones = r.u32()?..r.u32()?;
            if zones.end as usize > b.zones.len() || zones.start > zones.end || layers > 128 {
                return Err("pack: an instrument past the zones".into());
            }
            b.insts.push(Inst { layer, layers, cells, key, auto, zones });
        }
        let n = r.count(18)?;
        let mut groups = Vec::with_capacity(n);
        for _ in 0..n {
            let mut g = Group { name: r.str()?, frames: r.u32()?, mono: r.u32()?, ..Group::default() };
            if g.mono > g.frames {
                return Err(format!("pack: {}: more mono than all", g.name));
            }
            for _ in 0..r.count(2)? {
                g.insts.push(r.u16()?);
            }
            for _ in 0..r.count(6)? {
                g.multis.push((r.str()?, r.str()?, r.u16()?));
            }
            for _ in 0..r.count(4)? {
                g.gms.push((r.str()?, r.u16()?));
            }
            for _ in 0..r.count(258)? {
                let name = r.str()?;
                let mut keys = [0u16; 128];
                for k in keys.iter_mut() {
                    *k = r.u16()?;
                }
                g.kits.push((name, keys));
            }
            if g.insts.iter().any(|&id| id == 0 || id as usize > b.insts.len()) {
                return Err(format!("pack: {}: an instrument it does not have", g.name));
            }
            groups.push(g);
        }
        if srcs.iter().any(|&h| h as usize >= groups.len()) {
            return Err("pack: a recording in no group".into());
        }
        let bases = vec![None; groups.len()];
        Ok(Partial { bank: b, groups, lens, srcs, bases, filling: None })
    }

    /// An instrument by its kind and name, as an index line gives it ("multi"
    /// and an instrument and articulation, or "gm" and a preset), whether
    /// or not its group is in.
    pub fn find(&self, kind: &str, name: &str, art: &str) -> Option<u16> {
        self.groups.iter().find_map(|g| match kind {
            "multi" => g.multis.iter().find(|m| m.0 == name && m.1 == art).map(|m| m.2),
            "gm" => g.gms.iter().find(|m| m.0 == name).map(|m| m.1),
            _ => None,
        })
    }

    /// The samples group `g`'s stream comes to as the bank keeps it: its
    /// mono part one a frame, its stereo part two (interleaved).
    pub fn kept(&self, g: usize) -> Option<usize> {
        self.groups.get(g).map(|gr| gr.mono as usize + 2 * (gr.frames - gr.mono) as usize)
    }

    /// Room in the bank's data for `n` samples more (`kept` of the groups
    /// about to come), made at once rather than a group at a time.
    pub fn reserve(&mut self, n: usize) {
        self.bank.data.reserve_exact(n);
    }

    /// Room for group `g`'s stream at the end of the bank's data, `n`
    /// samples (`kept`), for the page to fill before `settle`: made in place,
    /// so that a genre's hundreds of megabytes are never copied whole.
    pub fn room(&mut self, g: usize, n: usize) -> Result<&mut [i16], String> {
        let want = self.kept(g).ok_or_else(|| format!("no group {g}"))?;
        let name = &self.groups[g].name;
        if self.bases[g].is_some() {
            return Err(format!("{name} is in already"));
        }
        if n != want {
            return Err(format!("{name}: {n} samples, not {want}"));
        }
        let base = self.bank.data.len();
        self.bank.data.reserve_exact(n);
        self.bank.data.resize(base + n, 0);
        self.filling = Some((g, base));
        Ok(&mut self.bank.data[base..])
    }

    /// Group `g`'s stream, in the room `room` made for it: its instruments
    /// playable, and named. Once (later calls do nothing).
    pub fn settle(&mut self, g: usize) -> Result<(), String> {
        if self.bases.get(g).ok_or_else(|| format!("no group {g}"))?.is_some() {
            return Ok(());
        }
        let base = match self.filling.take() {
            Some((f, b)) if f == g => b,
            _ => return Err(format!("{}: no room made for it", self.groups[g].name)),
        };
        let r = self.place(g, base);
        if r.is_err() {
            self.bank.data.truncate(base);
        }
        r
    }

    /// Group `g`'s stream (stereo, 16-bit, interleaved, `frames` long):
    /// its instruments playable, and named. Once (later calls do nothing).
    pub fn add(&mut self, g: usize, pcm: &[i16]) -> Result<(), String> {
        let gr = self.groups.get(g).ok_or_else(|| format!("no group {g}"))?;
        if self.bases[g].is_some() {
            return Ok(());
        }
        if pcm.len() != 2 * gr.frames as usize {
            return Err(format!("{}: {} samples, not {} frames' two", gr.name, pcm.len(), gr.frames));
        }
        // as the bank keeps it: the mono part a channel of it, the stereo part as it is
        let mono = gr.mono as usize;
        let kept: Vec<i16> = pcm[..2 * mono].iter().step_by(2).chain(&pcm[2 * mono..]).copied().collect();
        self.room(g, kept.len())?.copy_from_slice(&kept);
        self.settle(g)
    }

    /// Group `g`'s stream in, at `base` in the bank's data: each zone's
    /// recording in it, or in one already in; its instruments named.
    fn place(&mut self, g: usize, base: usize) -> Result<(), String> {
        let gr = &self.groups[g];
        let owned: Vec<u16> = gr
            .insts
            .iter()
            .copied()
            .filter(|&id| {
                // (an instrument another group holds comes with that one)
                !self.groups[..g].iter().any(|o| o.insts.contains(&id))
            })
            .collect();
        let mut at = Vec::new();
        for &id in &owned {
            let r = self.bank.insts[id as usize - 1].zones.clone();
            for zi in r.start as usize..r.end as usize {
                let h = self.srcs[zi] as usize;
                let start = match self.bases[h] {
                    _ if h == g => base,
                    Some(b) => b,
                    None => return Err(format!("{} needs {} first", gr.name, self.groups[h].name)),
                };
                let (z, len) = (&self.bank.zones[zi], self.lens[zi] as usize);
                let (mono, frames) = (self.groups[h].mono as usize, self.groups[h].frames as usize);
                let (from, to) = if z.chans == 2 { (mono, frames) } else { (0, mono) };
                if z.off < from || z.off + len > to {
                    return Err(format!("{}: a zone at {}..{}, outside {from}..{to}", gr.name, z.off, z.off + len));
                }
                // (the stereo part's frames two samples each)
                let off = if z.chans == 2 { mono + 2 * (z.off - mono) } else { z.off };
                at.push((zi, start + off));
            }
        }
        self.bases[g] = Some(base);
        for (zi, off) in at {
            self.bank.zones[zi].off = off;
            self.bank.zones[zi].len = self.lens[zi];
        }
        let gr = &self.groups[g];
        self.bank.multis.extend(gr.multis.iter().cloned());
        for (p, id) in gr.gms.clone() {
            self.bank.name_gm(&p, id);
        }
        self.bank.kits.extend(gr.kits.iter().cloned());
        Ok(())
    }
}

// ---------------------------------------------------------------- tests

/// Test material: a directory written under the target directory.
#[cfg(test)]
pub mod testdata {
    use std::path::{Path, PathBuf};

    /// A 16-bit WAV of `frames` frames from `f(frame, channel)`.
    pub fn wav(rate: u32, chans: u16, frames: usize, f: impl Fn(usize, usize) -> f32) -> Vec<u8> {
        let bytes = frames * chans as usize * 2;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + bytes as u32).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&chans.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * 2 * chans as u32).to_le_bytes());
        v.extend_from_slice(&(2 * chans).to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(bytes as u32).to_le_bytes());
        for i in 0..frames {
            for c in 0..chans as usize {
                v.extend_from_slice(&((f(i, c) * 32767.0).round() as i16).to_le_bytes());
            }
        }
        v
    }

    /// A sine at `hz`, its RMS the reference (-20 dBFS).
    pub fn sine(rate: u32, chans: u16, hz: f64, secs: f64) -> Vec<u8> {
        let a = 0.1 * std::f64::consts::SQRT_2;
        wav(rate, chans, (secs * rate as f64) as usize, |i, _| {
            (a * (std::f64::consts::TAU * hz * i as f64 / rate as f64).sin()) as f32
        })
    }

    /// A bank of a SoundFont's presets and kits alone.
    pub fn sf2_bank(path: &Path, melodic: &[&str], kits: &[&str]) -> Result<super::Bank, String> {
        let mut b = super::Bank::new();
        super::load_sf2(&mut b, path, melodic, kits)?;
        Ok(b)
    }

    /// A fresh directory for test `name`.
    pub fn dir(name: &str) -> PathBuf {
        let d = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("samples-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("multi")).expect("test dir");
        d
    }

    /// Frequency by upward zero crossings (interpolated), over `x`.
    pub fn freq(x: &[f32]) -> f64 {
        let ups: Vec<f64> = (1..x.len())
            .filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0)
            .map(|i| (i - 1) as f64 + (-x[i - 1] / (x[i] - x[i - 1])) as f64)
            .collect();
        (ups.len() - 1) as f64 * 48000.0 / (ups[ups.len() - 1] - ups[0])
    }

    /// A small SoundFont: one sample (a sine at 440 Hz, 32 kHz, looped),
    /// an instrument with a global zone (loop until release, attenuation) and
    /// two zones (the lower keys tuned +100 cents with their root at 72),
    /// a preset ("Trumpet") adding a fine tune of +50 cents; and a kit
    /// ("Standard", bank 128) with keys 42 and 46 in exclusive class 1.
    pub fn tiny_sf2() -> Vec<u8> {
        let rate = 32000u32;
        let period = 32000.0 / 440.0;
        let n = 4000usize;
        let smp: Vec<i16> =
            (0..n).map(|i| (8000.0 * (std::f64::consts::TAU * i as f64 / period).sin()) as i16).collect();
        let chunk = |id: &[u8], body: &[u8]| {
            let mut v = id.to_vec();
            v.extend_from_slice(&(body.len() as u32).to_le_bytes());
            v.extend_from_slice(body);
            if body.len() & 1 == 1 {
                v.push(0);
            }
            v
        };
        let list = |typ: &[u8], parts: &[Vec<u8>]| {
            let mut body = typ.to_vec();
            for p in parts {
                body.extend_from_slice(p);
            }
            chunk(b"LIST", &body)
        };
        let name20 = |s: &str| {
            let mut v = s.as_bytes().to_vec();
            v.resize(20, 0);
            v
        };
        let gn = |op: u16, amt: i16| [op.to_le_bytes(), amt.to_le_bytes()].concat();
        let range = |op: u16, lo: u8, hi: u8| [op.to_le_bytes(), [lo, hi]].concat();
        let bag = |g: u16| [g.to_le_bytes(), 0u16.to_le_bytes()].concat();
        // presets: "Trumpet" (bank 0) zone 0 -> instrument 0; "Standard" (bank 128) zone 1 -> instrument 1
        let mut phdr = Vec::new();
        for (nm, prog, bank, bagi) in [("Trumpet", 56u16, 0u16, 0u16), ("Standard", 0, 128, 1), ("EOP", 0, 0, 2)] {
            phdr.extend(name20(nm));
            phdr.extend(prog.to_le_bytes());
            phdr.extend(bank.to_le_bytes());
            phdr.extend(bagi.to_le_bytes());
            phdr.extend([0u8; 12]);
        }
        let pgen = [gn(52, 50), gn(41, 0), gn(41, 1), gn(0, 0)].concat();
        let pbag = [bag(0), bag(2), bag(3)].concat();
        // instrument 0: global (mode 3, attenuation 60), zone keys 0-59 (root 72, coarse +1), zone keys 60-127
        // instrument 1: key 42 and key 46, class 1, mode 0
        let igen = [
            gn(54, 3),
            gn(48, 60),
            range(43, 0, 59),
            gn(58, 72),
            gn(51, 1),
            gn(53, 0),
            range(43, 60, 127),
            gn(53, 0),
            range(43, 42, 42),
            gn(57, 1),
            gn(53, 0),
            range(43, 46, 46),
            gn(57, 1),
            gn(53, 0),
            gn(0, 0),
        ]
        .concat();
        let ibag = [bag(0), bag(2), bag(6), bag(8), bag(11), bag(14)].concat();
        let mut inst = Vec::new();
        for (nm, b) in [("Sine", 0u16), ("Hats", 3), ("EOI", 5)] {
            inst.extend(name20(nm));
            inst.extend(b.to_le_bytes());
        }
        let mut shdr = Vec::new();
        for (nm, s, e) in [("sine", 0u32, n as u32), ("EOS", 0, 0)] {
            shdr.extend(name20(nm));
            for x in [s, e, 800, 800 + (period * 10.0).round() as u32, rate] {
                shdr.extend(x.to_le_bytes());
            }
            shdr.extend([69u8, 0]);
            shdr.extend(0u16.to_le_bytes());
            shdr.extend(1u16.to_le_bytes());
        }
        let mut sbytes: Vec<u8> = smp.iter().flat_map(|s| s.to_le_bytes()).collect();
        sbytes.extend([0u8; 92]);
        let mut body = b"sfbk".to_vec();
        body.extend(list(b"INFO", &[chunk(b"ifil", &[2, 0, 1, 0])]));
        body.extend(list(b"sdta", &[chunk(b"smpl", &sbytes)]));
        body.extend(list(
            b"pdta",
            &[
                chunk(b"phdr", &phdr),
                chunk(b"pbag", &pbag),
                chunk(b"pmod", &[0; 10]),
                chunk(b"pgen", &pgen),
                chunk(b"inst", &inst),
                chunk(b"ibag", &ibag),
                chunk(b"imod", &[0; 10]),
                chunk(b"igen", &igen),
                chunk(b"shdr", &shdr),
            ],
        ));
        chunk(b"RIFF", &body)
    }

    /// A samples directory: "held" sus (one 5.5 s sine at 69, 22.05 kHz,
    /// its last 0.5 s faded, as the manifest's are) and "shake" hit (three
    /// round robins, 50 ms each, 1, 2 and 3 cycles of a square).
    pub fn held(name: &str) -> PathBuf {
        let d = dir(name);
        let a = 0.1 * std::f64::consts::SQRT_2;
        let long = wav(22050, 1, 22050 * 11 / 2, |i, _| {
            let t = i as f64 / 22050.0;
            (a * (std::f64::consts::TAU * 440.0 * t).sin() * ((5.5 - t) / 0.5).min(1.0)) as f32
        });
        std::fs::write(d.join("multi/held.wav"), long).expect("write");
        let mut m = String::from("instrument\tarticulation\troot\tvel_lo\tvel_hi\trr\tgain_db\tfile\n");
        m += "held\tsus\t69\t0\t127\t1\t0\tmulti/held.wav\n";
        for k in 1..=3 {
            let sq = wav(48000, 1, 2400, |i, _| if (i * k / 1200) % 2 == 0 { 0.5 } else { -0.5 });
            std::fs::write(d.join(format!("multi/shake{k}.wav")), sq).expect("write");
            m += &format!("shake\thit\t60\t0\t127\t{k}\t0\tmulti/shake{k}.wav\n");
        }
        std::fs::write(d.join("manifest.tsv"), m).expect("manifest");
        d
    }

    /// A samples directory: "sine" sus (roots 57 at 44.1 kHz, 69 stereo,
    /// two round robins of 81 loud and one soft) and stac (69 only, 0.2 s),
    /// "hit" one (a second of middle C, for a drum), and a manifest naming
    /// them; all at the reference level.
    pub fn synthetic(name: &str) -> PathBuf {
        let d = dir(name);
        let hz = |n: f64| 440.0 * ((n - 69.0) / 12.0).exp2();
        let put = |f: &str, b: Vec<u8>| std::fs::write(d.join(f), b).expect("write");
        put("multi/s57.wav", sine(44100, 1, hz(57.0), 1.0));
        put("multi/s69.wav", sine(48000, 2, hz(69.0), 1.0));
        put("multi/s81a.wav", sine(48000, 1, hz(81.0), 1.0));
        put("multi/s81b.wav", sine(48000, 1, hz(81.0), 1.0));
        put("multi/s81soft.wav", sine(48000, 1, hz(81.0), 1.0));
        put("multi/stac.wav", sine(48000, 1, hz(69.0), 0.2));
        put("multi/hit.wav", sine(48000, 1, hz(60.0), 1.0));
        // stereo, its right channel a fifth up and half as loud
        put(
            "multi/wide.wav",
            wav(44100, 2, 44100, |i, c| {
                let f = if c == 0 { hz(69.0) } else { hz(76.0) };
                let a = if c == 0 { 0.14 } else { 0.07 };
                (a * (std::f64::consts::TAU * f * i as f64 / 44100.0).sin()) as f32
            }),
        );
        let m = "# a test\ninstrument\tarticulation\troot\tvel_lo\tvel_hi\trr\tgain_db\tfile\n\
                 sine\tsus\t57\t0\t127\t0\t0\tmulti/s57.wav\n\
                 sine\tsus\t69\t0\t127\t0\t0\tmulti/s69.wav\n\
                 sine\tsus\t81\t64\t127\t0\t0\tmulti/s81a.wav\n\
                 sine\tsus\t81\t64\t127\t1\t0\tmulti/s81b.wav\n\
                 sine\tsus\t81\t0\t63\t0\t-6\tmulti/s81soft.wav\n\
                 sine\tstac\t69\t0\t127\t0\t0\tmulti/stac.wav\n\
                 hit\tone\t60\t0\t127\t0\t0\tmulti/hit.wav\n\
                 wide\tsus\t69\t0\t127\t0\t0\tmulti/wide.wav\n";
        std::fs::write(d.join("manifest.tsv"), m).expect("manifest");
        d
    }
}

#[cfg(test)]
mod tests {
    use super::testdata::*;
    use super::*;

    #[test]
    fn the_browsers_copy_plays_as_the_bank_does() {
        let d = synthetic("pack");
        std::fs::write(d.join(SF2), tiny_sf2()).expect("sf2");
        let b = load(&d).expect("load");
        let out = d.join("web");
        let all: Vec<u16> = (1..=b.insts.len() as u16).collect();
        let used = [
            ("all".into(), all.clone()),
            ("none".into(), Vec::new()),
            ("none+all".into(), all.clone()),
            ("none+none".into(), all),
        ];
        let json = b.web_pack(&out, &used).expect("pack");
        let mut p = Partial::new(&std::fs::read(out.join("pack.bin")).expect("pack.bin")).expect("tables");
        p.reserve((0..p.groups.len()).filter_map(|g| p.kept(g)).sum());
        assert!(json.contains("\"all\":[") && json.contains("\"none\":[]"), "{json}");
        // a mix lists only what neither of its genres has
        assert!(json.contains("\"mixes\":{\n\"none+none\":[0") && !json.contains("none+all"), "{json}");
        assert_eq!(p.bank.multi("sine", "sus"), None, "nothing named before its group is in");
        assert_eq!(p.find("multi", "sine", "sus"), b.multi("sine", "sus"), "but found by name in the tables");
        assert_eq!(p.find("gm", "Trumpet", ""), b.gm("Trumpet"));
        for g in 0..p.groups.len() {
            let (pcm, rate, chans) =
                read_wav(&std::fs::read(out.join(format!("raw/{g}.wav"))).expect("raw")).expect("wav");
            // (all mono: a mono stream)
            let mono = p.groups[g].mono == p.groups[g].frames;
            assert_eq!((rate, chans), (48000, if mono { 1 } else { 2 }));
            assert_eq!(pcm.len() as u32, chans as u32 * p.groups[g].frames);
            let both: Vec<i16> = if mono { pcm.iter().flat_map(|&s| [s, s]).collect() } else { pcm };
            p.add(g, &both).expect("add");
        }
        // the lighter copy: every group one channel at half the rate, a
        // quarter of what the stereo ones hold (a half of the mono ones)
        let mut lite = Partial::new(&std::fs::read(out.join("lite/pack.bin")).expect("lite pack.bin")).expect("tables");
        for g in 0..lite.groups.len() {
            let (pcm, rate, chans) =
                read_wav(&std::fs::read(out.join(format!("lite/raw/{g}.wav"))).expect("raw")).expect("wav");
            assert_eq!((rate, chans), (24000, 1));
            assert_eq!(lite.groups[g].mono, lite.groups[g].frames, "all in one channel");
            assert_eq!(pcm.len() as u32, lite.groups[g].frames);
            assert!(json.contains(&format!("\"lite\":{},", lite.groups[g].frames)), "{json}");
            let (full, kept) = (p.kept(g).expect("a group"), lite.kept(g).expect("a group"));
            assert!(kept * 2 <= full + 4096, "{}: {kept} samples of {full}", lite.groups[g].name);
            let both: Vec<i16> = pcm.iter().flat_map(|&s| [s, s]).collect();
            lite.add(g, &both).expect("add");
        }
        let _ = std::fs::remove_dir_all(&d);
        let w = &p.bank;
        let rms =
            |v: &[[f32; 2]]| (v.iter().map(|x| x[0] * x[0] + x[1] * x[1]).sum::<f32>() / (2 * v.len()) as f32).sqrt();
        let same = |what: &str, a: &[[f32; 2]], c: &[[f32; 2]]| {
            let l = |v: &[[f32; 2]]| v[4800..].iter().map(|x| x[0] + x[1]).collect::<Vec<f32>>();
            let (fa, fc) = (freq(&l(a)), freq(&l(c)));
            assert!((fc - fa).abs() < fa * 0.002, "{what}: {fc:.2} Hz, the bank's {fa:.2}");
            let db = 20.0 * (rms(c) / rms(a)).log10();
            assert!(db.abs() < 1.0, "{what}: {db:+.2} dB from the bank's");
            // no click anywhere: no step past what the waveform takes
            let most = |v: &[[f32; 2]]| v.windows(2).map(|x| (x[1][0] - x[0][0]).abs()).fold(0.0f32, f32::max);
            assert!(most(c) < most(a) * 1.3 + 1e-4, "{what}: a step of {} (the bank's {})", most(c), most(a));
        };
        for (inst, art, note) in [("sine", "sus", 69.0), ("sine", "sus", 57.0), ("sine", "stac", 69.0)] {
            let (ia, ic) = (b.multi(inst, art).expect("bank"), w.multi(inst, art).expect("copy"));
            same(&format!("{inst} {art} {note}"), &play(&b, ia, note, 0.9, 0.18), &play(w, ic, note, 0.9, 0.18));
            let il = lite.bank.multi(inst, art).expect("lighter copy");
            same(
                &format!("lite {inst} {art} {note}"),
                &play(&b, ia, note, 0.9, 0.18),
                &play(&lite.bank, il, note, 0.9, 0.18),
            );
        }
        // a stereo recording stays stereo: each side its own tune and level
        let (ia, ic) = (b.multi("wide", "sus").expect("bank"), w.multi("wide", "sus").expect("copy"));
        let (a, c) = (play(&b, ia, 69.0, 0.9, 0.5), play(w, ic, 69.0, 0.9, 0.5));
        for side in 0..2 {
            let one = |v: &[[f32; 2]]| v[4800..].iter().map(|x| x[side]).collect::<Vec<f32>>();
            let (fa, fc) = (freq(&one(&a)), freq(&one(&c)));
            assert!((fc - fa).abs() < fa * 0.002, "side {side}: {fc:.2} Hz, the bank's {fa:.2}");
            let level = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
            let db = 20.0 * (level(&one(&c)) / level(&one(&a))).log10();
            assert!(db.abs() < 0.5, "side {side}: {db:+.2} dB from the bank's");
        }
        // the SoundFont's looped sine, at 32 kHz: two seconds, round its loop
        let (ia, ic) = (b.gm("Trumpet").expect("bank"), w.gm("Trumpet").expect("copy"));
        same("Trumpet", &play(&b, ia, 69.0, 0.9, 2.0), &play(w, ic, 69.0, 0.9, 2.0));
        let il = lite.bank.gm("Trumpet").expect("lighter copy");
        same("lite Trumpet", &play(&b, ia, 69.0, 0.9, 2.0), &play(&lite.bank, il, 69.0, 0.9, 2.0));
        // (its stereo recording: both sides in one, still sounding)
        let il = lite.bank.multi("wide", "sus").expect("lighter copy");
        assert!(rms(&play(&lite.bank, il, 69.0, 0.9, 0.5)) > 0.3 * rms(&a), "the stereo recording, in one channel");
        let (ka, kc) = (b.drum("Standard", 42).expect("bank"), w.drum("Standard", 42).expect("copy"));
        assert!(rms(&play(w, kc, 60.0, 0.9, 0.3)) > 0.5 * rms(&play(&b, ka, 60.0, 0.9, 0.3)), "the kit's key");
    }

    fn play(b: &Bank, id: u16, note: f32, vel: f32, secs: f32) -> Vec<[f32; 2]> {
        let mut s = SAMPLER;
        assert!(s.start(b, id, note, vel, (secs * RATE) as i64, 0));
        let mut v = vec![[0.0f32; 2]; (secs * RATE) as usize];
        for c in v.chunks_mut(32) {
            s.fill(b, note, true, c);
        }
        v
    }

    #[test]
    fn synthetic_dir_plays_in_tune() {
        let d = synthetic("tune");
        let b = load(&d).expect("load");
        let _ = std::fs::remove_dir_all(&d);
        let id = b.multi("sine", "sus").expect("sine sus");
        // at a root, a fifth over one (from 57, recorded at 44.1 kHz), seven
        // semitones under one, between two roots, far past the last
        for note in [69.0f32, 64.0, 62.0, 75.0, 76.0, 90.0, 40.0] {
            let v = play(&b, id, note, 0.9, 0.5);
            let l: Vec<f32> = v[2400..].iter().map(|x| x[0]).collect();
            let want = 440.0 * ((note as f64 - 69.0) / 12.0).exp2();
            let got = freq(&l);
            assert!((got / want - 1.0).abs() < 0.002, "note {note}: {got:.2} Hz, want {want:.2}");
            // the level: the reference, both channels
            let rms = (v[2400..].iter().map(|x| x[0] * x[0] + x[1] * x[1]).sum::<f32>()
                / (2 * (v.len() - 2400)) as f32)
                .sqrt();
            assert!((rms / REF_RMS - 1.0).abs() < 0.05, "note {note}: RMS {rms}");
        }
        // nearest roots: 62 from 57 (5 away, not 69's 7), 63 a tie (57)
        for (note, root) in [(62.0, 57.0), (63.0, 57.0), (64.0, 69.0), (76.0, 81.0), (20.0, 57.0)] {
            let (z, _, _) = b.pick(id, note, 0.9, 48000, 0).expect("pick");
            assert_eq!(b.zones[z[0] as usize].root, root, "note {note}");
        }
        // round robins rotate at 81 (loud), the soft layer has one
        let rr: Vec<u32> = (0..4).map(|k| b.pick(id, 81.0, 0.9, 48000, k).unwrap().0[0]).collect();
        assert!(rr[0] != rr[1] && rr[0] == rr[2] && rr[1] == rr[3], "{rr:?}");
        let soft: Vec<u32> = (0..2).map(|k| b.pick(id, 81.0, 0.3, 48000, k).unwrap().0[0]).collect();
        assert!(soft[0] == soft[1] && soft[0] != rr[0] && soft[0] != rr[1]);
        // auto: short gates the staccato, long the sustain
        let auto = b.multi("sine", "auto").expect("auto");
        let stac = b.multi("sine", "stac").expect("stac");
        assert_eq!(b.pick(auto, 69.0, 0.9, 4800, 0).unwrap().0, b.pick(stac, 69.0, 0.9, 4800, 0).unwrap().0);
        assert_eq!(b.pick(auto, 69.0, 0.9, 48000, 0).unwrap().0, b.pick(id, 69.0, 0.9, 48000, 0).unwrap().0);
        // the short one's first 0.4 s matched to the long one's first second
        // at its middle root (81): the staccato's 0.2 s, an octave up, lasts
        // 0.1 s, the sustain all of it; so twice as loud
        let g = b.pick(auto, 69.0, 0.9, 4800, 0).unwrap().2;
        assert!((g / 2.0 - 1.0).abs() < 0.05 && b.pick(auto, 69.0, 0.9, 48000, 0).unwrap().2 == 1.0, "{g}");
        assert!(b.multi("sine", "pizz").is_none() && b.gm("Trumpet").is_none() && b.drum("Standard", 36).is_none());
    }

    #[test]
    fn a_sample_that_runs_out_fades_and_ends() {
        let d = synthetic("end");
        let b = load(&d).expect("load");
        let _ = std::fs::remove_dir_all(&d);
        let id = b.multi("sine", "stac").unwrap();
        let mut s = SAMPLER;
        assert!(s.start(&b, id, 69.0, 0.9, 48000, 0));
        let mut v = vec![[0.0f32; 2]; 48000];
        let mut live = 0;
        for (k, c) in v.chunks_mut(32).enumerate() {
            if s.fill(&b, 69.0, true, c) {
                live = k;
            }
        }
        // 0.2 s long: over by then, the last 10 ms fading
        assert!((live * 32) as f32 / RATE < 0.205 && (live * 32) as f32 / RATE > 0.19, "ended at block {live}");
        let peak = |a: usize, b: usize| v[a..b].iter().map(|x| x[0].abs()).fold(0.0, f32::max);
        assert!(peak(9000, 9200) > 0.1 && peak(9580, 9600) < 0.02 && peak(9700, 48000) == 0.0);
    }

    /// A held note outlasting a sustained recording: its middle loops,
    /// crossfaded, steady to within a few dB, until the gate closes.
    #[test]
    fn held_notes_loop_past_the_recording() {
        let d = held("held");
        let b = load(&d).expect("load");
        let _ = std::fs::remove_dir_all(&d);
        let id = b.multi("held", "sus").unwrap();
        let mut s = SAMPLER;
        assert!(s.start(&b, id, 69.0, 0.9, 12 * 48000, 0));
        let mut v = vec![[0.0f32; 2]; 12 * 48000];
        for c in v.chunks_mut(32) {
            assert!(s.fill(&b, 69.0, true, c));
        }
        // 50 ms windows from 0.1 s to 12 s
        for w in v[4800..].chunks(2400) {
            let rms = (w.iter().map(|x| x[0] * x[0]).sum::<f32>() / w.len() as f32).sqrt();
            assert!(rms > 0.1 / 1.5 && rms < 0.1 * 1.5, "a window's RMS {rms}");
        }
        let l: Vec<f32> = v[5 * 48000..].iter().map(|x| x[0]).collect();
        assert!((freq(&l) / 440.0 - 1.0).abs() < 0.002);
        // released, it plays out to its end (a drum's hit never loops)
        let mut out = [[0.0f32; 2]; 32];
        let mut blocks = 0;
        while s.fill(&b, 69.0, false, &mut out) {
            blocks += 1;
        }
        assert!(blocks * 32 < 6 * 48000);
    }

    #[test]
    fn malformed_wavs_are_errors() {
        let good = sine(48000, 1, 440.0, 0.01);
        assert!(read_wav(&good).is_ok());
        // every truncation, and every byte of the header broken
        for n in 0..good.len() {
            let _ = read_wav(&good[..n]);
        }
        for i in 0..44 {
            for x in [0u8, 0xFF, 0x7F, 0x80] {
                let mut b = good.clone();
                b[i] = x;
                let _ = read_wav(&b);
            }
        }
        let mut big = good.clone();
        big[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read_wav(&big).is_err(), "a data chunk past the end");
        let mut eight = good.clone();
        eight[34] = 8;
        assert!(read_wav(&eight).is_err(), "8-bit");
        let mut three = good.clone();
        three[22] = 3;
        assert!(read_wav(&three).is_err(), "three channels");
        assert!(read_wav(b"RIFF").is_err() && read_wav(&[]).is_err());
        assert!(read_wav(&wav(48000, 1, 1, |_, _| 0.0)).is_err(), "one frame");
    }

    #[test]
    fn malformed_manifests_skip_rows() {
        let d = dir("bad");
        std::fs::write(d.join("multi/ok.wav"), sine(48000, 1, 440.0, 0.1)).unwrap();
        std::fs::write(d.join("multi/short.wav"), &sine(48000, 1, 440.0, 0.1)[..30]).unwrap();
        let m = "instrument\tarticulation\troot\tvel_lo\tvel_hi\trr\tgain_db\tfile\n\
                 a\tsus\t69\t0\t127\t0\t0\tmulti/ok.wav\n\
                 a\tsus\t70\t0\t127\t0\t0\n\
                 a\tsus\tx\t0\t127\t0\t0\tmulti/ok.wav\n\
                 a\tsus\t69\t100\t20\t0\t0\tmulti/ok.wav\n\
                 a\tsus\t200\t0\t127\t0\t0\tmulti/ok.wav\n\
                 a\tsus\t69\t0\t127\t0\tNaN\tmulti/ok.wav\n\
                 a\tsus\t69\t0\t127\t0\t0\t../outside.wav\n\
                 a\tsus\t69\t0\t127\t0\t0\t/etc/passwd\n\
                 b\tsus\t69\t0\t127\t0\t0\tmulti/missing.wav\n\
                 c\tsus\t69\t0\t127\t0\t0\tmulti/short.wav\n\
                 \n\
                 garbage line without tabs\n";
        std::fs::write(d.join("manifest.tsv"), m).unwrap();
        let b = load(&d).expect("the good row loads");
        assert!(b.multi("a", "sus").is_some() && b.multi("b", "sus").is_none() && b.multi("c", "sus").is_none());
        assert_eq!(b.warnings.iter().filter(|w| w.contains("line")).count(), 8, "{:?}", b.warnings);
        assert!(
            b.warnings.iter().any(|w| w.contains("missing.wav")) && b.warnings.iter().any(|w| w.contains("short.wav"))
        );
        // nothing loadable at all, and no directory: errors
        std::fs::write(d.join("manifest.tsv"), "garbage\n\u{0}\u{1}").unwrap();
        assert!(load(&d).is_err());
        std::fs::write(d.join("manifest.tsv"), [0xFFu8, 0xFE, 0x00]).unwrap();
        assert!(load(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
        assert!(load(&d).is_err());
    }

    #[test]
    fn a_tiny_soundfont_resolves() {
        let d = dir("sf2");
        let path = d.join("tiny.sf2");
        let sf = tiny_sf2();
        std::fs::write(&path, &sf).unwrap();
        let mut b = Bank::new();
        load_sf2(&mut b, &path, &["Trumpet", "Nothing"], &["Standard"]).expect("tiny sf2");
        assert!(b.warnings.iter().any(|w| w.contains("Nothing")), "{:?}", b.warnings);
        let id = b.gm("Trumpet").expect("Trumpet");
        let z = b.zones[b.pick(id, 64.0, 0.8, 48000, 0).unwrap().0[0] as usize];
        assert_eq!((z.mode, z.root, z.tune), (3, 69.0, 0.5), "the global zone's mode, the preset's fine tune");
        let z = b.zones[b.pick(id, 50.0, 0.8, 48000, 0).unwrap().0[0] as usize];
        assert_eq!((z.root, z.tune), (72.0, 1.5), "the zone's root and coarse tune over the preset's fine");
        // in tune through the loop: 440 Hz sounds where the root less the
        // tuning is (68.5 above key 60, 70.5 below)
        for note in [69.5f32, 76.5, 60.0, 50.0] {
            let v = play(&b, id, note, 0.8, 0.5);
            let l: Vec<f32> = v.iter().map(|x| x[0]).collect();
            let at = if note >= 60.0 { 68.5 } else { 70.5 };
            let want = 440.0 * ((note as f64 - at) / 12.0).exp2();
            let got = freq(&l[1000..]);
            assert!((got / want - 1.0).abs() < 0.003, "note {note}: {got:.2} Hz, want {want:.2}");
            let rms = (l[1000..].iter().map(|x| x * x).sum::<f32>() / (l.len() - 1000) as f32).sqrt();
            assert!((rms / REF_RMS - 1.0).abs() < 0.1, "note {note}: RMS {rms} (normalised)");
        }
        // the loop holds past the sample's 0.125 s while the gate is on
        let v = play(&b, id, 68.5, 0.8, 0.5);
        assert!(v[20000..].iter().map(|x| x[0].abs()).fold(0.0, f32::max) > 0.1);
        // the kit: two keys, one class, nothing at 36
        let (h, o) = (b.drum("Standard", 42).expect("42"), b.drum("Standard", 46).expect("46"));
        assert!(b.drum("Standard", 36).is_none() && h != o);
        let mut s = SAMPLER;
        assert!(s.start(&b, o, 0.0, 0.8, 0, 0));
        assert_eq!((s.excl, s.key), (1, 46.0));
        // broken: every truncation, and bytes of the structure overwritten
        for n in (0..sf.len()).step_by(7) {
            std::fs::write(&path, &sf[..n]).unwrap();
            let mut b = Bank::new();
            let _ = load_sf2(&mut b, &path, &["Trumpet"], &["Standard"]);
        }
        let pdta = sf.windows(4).position(|w| w == b"pdta").unwrap();
        for i in (pdta..sf.len()).step_by(3) {
            for x in [0u8, 0xFF, 0x80] {
                let mut s = sf.clone();
                s[i] = x;
                std::fs::write(&path, &s).unwrap();
                let mut b = Bank::new();
                if load_sf2(&mut b, &path, &["Trumpet"], &["Standard"]).is_ok() {
                    // whatever loaded plays without a panic
                    for &(_, id) in &b.gms {
                        let mut v = [[0.0f32; 2]; 64];
                        let mut sm = SAMPLER;
                        if sm.start(&b, id, 64.0, 0.8, 4800, 0) {
                            for _ in 0..50 {
                                sm.fill(&b, 64.0, true, &mut v);
                            }
                        }
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn timgm6mb_parses_when_present() {
        let path = Path::new("/usr/share/sounds/sf2/TimGM6mb.sf2");
        if !path.is_file() {
            eprintln!("no {}: skipped", path.display());
            return;
        }
        let presets = sf2_presets(path).expect("presets");
        assert!(presets.len() > 128, "{} presets", presets.len());
        let melodic: Vec<&str> = presets.iter().filter(|p| p.0 == 0 && p.1 < 6).map(|p| p.2.as_str()).collect();
        assert!(presets.iter().any(|p| p.0 == 128 && p.2 == "Standard"));
        let mut b = Bank::new();
        load_sf2(&mut b, path, &melodic, &["Standard"]).expect("load");
        assert_eq!(b.gms.len(), melodic.len(), "{:?}", b.warnings);
        for (p, id) in &b.gms {
            let (z, _, _) = b.pick(*id, 60.0, 0.8, 48000, 0).expect(p);
            assert!(!z.is_empty() && z.iter().all(|&z| b.frames(&b.zones[z as usize]).iter().any(|&x| x != 0)), "{p}");
            let rms = b.rms_1s(*id, 60.0);
            assert!((rms / REF_RMS - 1.0).abs() < 0.01, "{p}: {rms}");
        }
        for key in [36u8, 38, 42, 46] {
            assert!(b.drum("Standard", key).is_some(), "key {key}");
        }
        assert!(b.bytes() > 100_000);
    }

    #[test]
    fn the_recordings_stand_in_for_presets_the_bank_loads() {
        for (i, &(preset, inst, art, note, secs, _)) in GM_RECORDED.iter().enumerate() {
            assert!(GM_PRESETS.contains(&preset), "{preset}: not a preset the bank loads");
            assert!(!GM_RECORDED[..i].iter().any(|r| r.0 == preset), "{preset}: twice");
            // (as the manifest names its instruments)
            let named =
                |x: &str| !x.is_empty() && x.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_');
            assert!(named(inst) && named(art), "{preset}: {inst} {art}");
            assert!((21.0..=96.0).contains(&note) && secs > 0.0 && secs <= 2.0, "{preset}: {note}, {secs} s");
        }
    }

    #[test]
    fn a_recording_stands_in_for_a_preset_at_its_level() {
        let d = synthetic("standin");
        std::fs::write(d.join(SF2), tiny_sf2()).expect("sf2");
        let mut b = load(&d).expect("load");
        let _ = std::fs::remove_dir_all(&d);
        let (was, rec) = (b.gm("Trumpet").expect("the preset"), b.multi("sine", "sus").expect("the recording"));
        let (zones, insts) = (b.zones.len(), b.insts.len());
        assert!(!b.stands_in("Trumpet") && !b.stands_in("No Such Preset"));
        // a preset or a recording that is not there: nothing changes
        b.stand_in(&[
            ("No Such Preset", "sine", "sus", 69.0, 0.5, false),
            ("Trumpet", "sine", "no such", 69.0, 0.5, true),
        ]);
        assert_eq!((b.gm("Trumpet"), b.zones.len(), b.insts.len()), (Some(was), zones, insts));
        b.stand_in(&[("Trumpet", "sine", "sus", 69.0, 0.5, false)]);
        let now = b.gm("Trumpet").expect("still a preset by its name");
        assert!(b.stands_in("Trumpet"));
        assert!(now != was && now != rec, "a copy of the recording, the recording itself as it was");
        assert_eq!(b.multi("sine", "sus"), Some(rec));
        // each of the recording's layers as loud as the preset at that
        // layer's middle velocity, over the notes they were matched on (the
        // softer layer to the hundredth; the louder, whose lower notes'
        // recordings are the softer one's too and keep its gain, within 1 dB)
        let notes = [57.0, 63.0, 69.0, 75.0, 81.0];
        let power = |b: &Bank, id: u16, v: f32| notes.iter().map(|&n| b.rms_at(id, n, v, 0.5).powi(2)).sum::<f32>();
        for (v, within) in [(32.0 / 127.0, 0.1), (96.0 / 127.0, 1.0)] {
            let db = 10.0 * (power(&b, now, v) / power(&b, was, v)).log10();
            assert!(db.abs() < within, "velocity {v}: {db:+.2} dB from the preset's");
        }
        // it is the recording that plays: the manifest's own two takes of its top note
        assert_eq!(
            b.pick(now, 81.0, 1.0, 48000, 0).map(|p| p.0.len()),
            b.pick(rec, 81.0, 1.0, 48000, 0).map(|p| p.0.len())
        );
        assert_ne!(
            b.pick(now, 81.0, 1.0, 48000, 0).map(|p| p.0.to_vec()),
            b.pick(now, 81.0, 1.0, 48000, 1).map(|p| p.0.to_vec()),
            "its round robins kept"
        );
        // and no SoundFont envelope, so its notes are not `LIVELY`'s to keep alive
        assert!(!b.lively.iter().any(|l| l.0 == now));
    }

    #[test]
    fn a_recording_follows_a_presets_levels_note_by_note() {
        // the recording (roots 57, 69 and 81) made uneven, as no preset is: one root's zones 12 dB up
        let uneven = |name: &str, root: f32| {
            let d = synthetic(name);
            std::fs::write(d.join(SF2), tiny_sf2()).expect("sf2");
            let mut b = load(&d).expect("load");
            let _ = std::fs::remove_dir_all(&d);
            let rec = b.multi("sine", "sus").expect("the recording");
            let r = b.insts[rec as usize - 1].zones.clone();
            for z in b.zones[r.start as usize..r.end as usize].iter_mut().filter(|z| z.root == root) {
                z.gl *= 4.0;
                z.gr *= 4.0;
            }
            b
        };
        let level = |b: &Bank, id: u16, n: f32| 20.0 * b.rms_at(id, n, 0.25, 0.5).max(1e-9).log10();
        // its top root: one gain a layer leaves it as uneven as it was; a gain
        // a root brings every root to the preset's level
        let mut b = uneven("standin-top", 81.0);
        let was = b.gm("Trumpet").expect("the preset");
        b.stand_in(&[("Trumpet", "sine", "sus", 69.0, 0.5, false)]);
        let flat = b.gm("Trumpet").expect("a preset");
        let by =
            |b: &Bank, id: u16| (level(b, id, 81.0) - level(b, was, 81.0)) - (level(b, id, 57.0) - level(b, was, 57.0));
        assert!(by(&b, flat) > 5.0, "{:+.2} dB", by(&b, flat));
        let mut b = uneven("standin-top2", 81.0);
        let was = b.gm("Trumpet").expect("the preset");
        b.stand_in(&[("Trumpet", "sine", "sus", 69.0, 0.5, true)]);
        let now = b.gm("Trumpet").expect("a preset");
        for n in [57.0, 69.0, 81.0] {
            let db = level(&b, now, n) - level(&b, was, n);
            assert!(db.abs() < 0.5, "note {n}: {db:+.2} dB from the preset's");
        }
        // a lone root between two others that agree: taken for a slip (the
        // preset's or the recording's), and given its neighbours' gain
        let mut b = uneven("standin-mid", 69.0);
        let was = b.gm("Trumpet").expect("the preset");
        b.stand_in(&[("Trumpet", "sine", "sus", 69.0, 0.5, true)]);
        let now = b.gm("Trumpet").expect("a preset");
        let db = level(&b, now, 69.0) - level(&b, was, 69.0);
        assert!(db > 5.0, "the middle root: {db:+.2} dB from the preset's");
        assert!((level(&b, now, 57.0) - level(&b, was, 57.0)).abs() < 0.5);
    }
}
