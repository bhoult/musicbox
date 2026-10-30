//! The optimiser's settings for each genre (tools/tune writes this file):
//! how far it turned each of `KNOBS` from the music as written, to sound
//! more like the genre's real recordings to the taggers (tools/validate).
//! Played when "optimised" is chosen (`Opts::tuned`); a genre with none
//! plays as written either way.

/// The knobs, their range, and where they rest (the music as written):
/// each channel's level (dB); the master's low and high shelves (dB, added
/// to the genre's own); the reverb's send (times); the tempo (times); the
/// swing (16ths, added); how late hits come (times); how wide each
/// channel's stereo is spread (times).
pub const KNOBS: [(&str, f32, f32, f32); NKNOBS] = [
    ("kick", -6.0, 6.0, 0.0),
    ("snare", -6.0, 6.0, 0.0),
    ("hat", -6.0, 6.0, 0.0),
    ("perc", -6.0, 6.0, 0.0),
    ("cym", -6.0, 6.0, 0.0),
    ("bass", -6.0, 6.0, 0.0),
    ("pad", -6.0, 6.0, 0.0),
    ("arp", -6.0, 6.0, 0.0),
    ("lead", -6.0, 6.0, 0.0),
    ("bell", -6.0, 6.0, 0.0),
    ("keys", -6.0, 6.0, 0.0),
    ("drone", -6.0, 6.0, 0.0),
    ("fx", -6.0, 6.0, 0.0),
    ("tilt_lo", -5.0, 5.0, 0.0),
    ("tilt_hi", -5.0, 5.0, 0.0),
    ("reverb", 0.5, 1.8, 1.0),
    ("tempo", 0.88, 1.12, 1.0),
    ("swing", -0.12, 0.12, 0.0),
    ("human", 0.4, 2.5, 1.0),
    ("spread", 0.4, 1.8, 1.0),
];
pub const NKNOBS: usize = 20;
pub const K_TILT_LO: usize = 13;
pub const K_TILT_HI: usize = 14;
pub const K_REVERB: usize = 15;
pub const K_TEMPO: usize = 16;
pub const K_SWING: usize = 17;
pub const K_HUMAN: usize = 18;
pub const K_SPREAD: usize = 19;

/// Each genre's settings, by its number (None: as written).
pub const TUNED: [Option<[f32; NKNOBS]>; 12] = [None; 12];
