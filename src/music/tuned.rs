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
/// Found 2026-10-31 by tools/tune/tune.py: CMA-ES against the tools/validate
/// taggers, each kept only where whole songs held out of the search came
/// closer and the other judges held (tools/tune/RESULTS.md).
pub const TUNED: [Option<[f32; NKNOBS]>; 12] = [
    // ambient
    Some([
        2.16, 0.00, -3.56, -0.84, 0.00, 0.56, 2.18, -4.06, 0.49, -2.64, 0.00, -2.44, 2.70, 1.64, -0.07, 1.16, 1.03,
        0.00, 0.70, 1.29,
    ]),
    // techno
    Some([
        -2.46, 2.93, 3.88, 0.39, 3.95, 2.36, -0.78, 0.00, 0.44, 0.00, 2.26, 0.00, -0.09, 2.26, -2.18, 0.91, 0.97, 0.04,
        1.00, 1.24,
    ]),
    None, // synthwave
    // lo-fi
    Some([
        0.62, -0.44, 3.17, -0.05, 0.00, -1.15, 0.00, 0.00, -0.36, -2.00, -5.90, 0.00, 2.19, 0.99, 1.56, 1.19, 0.96,
        -0.01, 1.23, 0.98,
    ]),
    None, // polka
    None, // tavern
    // ska
    Some([
        -0.76, -1.81, 0.18, 1.45, 0.25, 3.88, 0.30, 0.61, 1.88, 2.81, -2.97, 6.00, 1.93, 2.24, 1.64, 1.00, 1.09, 0.02,
        0.49, 0.65,
    ]),
    None, // mariachi
    // indonesian
    Some([
        -1.76, 1.13, -0.44, 0.52, 2.55, -1.93, 4.00, -0.57, 0.24, -0.88, -3.19, 2.81, 0.00, 0.15, 2.49, 1.09, 1.00,
        0.11, 1.17, 0.92,
    ]),
    // starconflict
    Some([
        -2.39, 0.47, 1.73, 0.00, 2.62, -2.74, 6.00, -0.14, -3.51, -0.66, -0.18, -0.04, 0.00, -0.61, -0.62, 1.17, 0.97,
        0.00, 1.02, 1.38,
    ]),
    None, // celtic
    None, // western
];
