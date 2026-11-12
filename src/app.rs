//! What the window, the terminal and the browser share: what a key or a
//! click asks for (`Action`), what they show besides the music (`State`),
//! and what each action does (`act`), to the music through a `Deck`: the
//! desktop's player, or the browser's worker.

use crate::favorites;
use crate::music;
use crate::tap::Now;
use crate::time::Instant;

/// What a key, a click or a button asks for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    /// the next or previous genre, and the next or previous drummer (the
    /// genre writing the music, for its own drums, among them)
    Genre(i32),
    Drums(i32),
    SetGenre(usize),
    SetDrums(Option<usize>),
    /// a new piece of the same music; stop or play on; the piece playing
    /// again from its start
    Skip,
    Play,
    Replay,
    /// the song playing from its start, or (in its first seconds) the one
    /// before it; seconds on (or back) in the song playing, or a point in it;
    /// a song by its id
    Back,
    Seek(f64),
    SeekTo(f64),
    Go(music::SongId),
    /// the length asked of each piece: set (seconds; 0: as long as it was
    /// written), or 15 s longer or shorter
    SetLength(f64),
    Longer(i32),
    /// how loud it plays: set (per cent), or a step louder or quieter
    SetVolume(u32),
    Louder(i32),
    /// the listener's effects (`music::FX_NAMES`): one set (per cent), or a
    /// step more or less of it; all as the genre has them
    SetEffect(usize, u32),
    MoreEffect(usize, i32),
    ResetEffects,
    /// the song playing's id onto the clipboard; a song's id on the
    /// clipboard played; the song playing kept among the favorites, or no
    /// longer
    Copy,
    Paste,
    Keep,
    /// a song no longer among the favorites
    Forget(music::SongId),
    /// the piece playing saved as an MP3
    Export,
    /// how the music is made: the optimiser's settings on or off, the
    /// network's tunes on or off, the next to play the instruments, the
    /// songs written with more variety or as they first were; all four as
    /// given
    Tuned,
    NetTunes,
    Voices,
    Varied,
    SetOpts(music::Opts),
    /// the terminal's equalizer on or off, the window open or closed,
    /// fullscreen, the window's list of the keys, its effects and its
    /// favorites shown or hidden
    Eq,
    Window,
    Fullscreen,
    Keys,
    EffectsPanel,
    FavoritesPanel,
    /// the window's own, done by it: the part of its selector shown (what a
    /// genre chosen there sets: the music's, or the one on drums), and its
    /// list of favorites moved by so many
    Tab(u8),
    Scroll(i32),
    Quit,
}

/// What the displays show besides the music's own state.
pub struct State {
    pub primary: usize,
    pub drums: Option<usize>,
    pub seed: u64,
    /// the recorded instruments loaded (else the synthesiser plays everything)
    pub sampled: bool,
    /// how the music is to be made
    pub opts: music::Opts,
    /// the terminal's equalizer rows
    pub eq: bool,
    /// the length asked of each piece (seconds; 0: as long as it was written)
    pub length: f64,
    /// how loud it plays (per cent)
    pub volume: u32,
    /// the listener's effects
    pub effects: music::Effects,
    /// the songs kept to play again
    pub favorites: favorites::Favorites,
    /// a message for a few seconds (a window that would not open), and a
    /// word on an export
    pub msg: Option<(String, Instant)>,
    pub note: Option<(String, Instant)>,
}

impl State {
    pub fn id(&self) -> usize {
        music::mix_id(self.primary, self.drums)
    }

    /// Applies a genre or drummer change (true: the music changes).
    pub fn apply(&mut self, a: Action) -> bool {
        let ng = music::GENRES.len();
        let before = self.id();
        match a {
            Action::Genre(d) => self.primary = (self.primary as i32 + d).rem_euclid(ng as i32) as usize,
            Action::SetGenre(g) => self.primary = g % ng,
            Action::Drums(d) => {
                // the genres in order, the primary for its own drums
                let at = self.drums.unwrap_or(self.primary) as i32;
                self.drums = Some((at + d).rem_euclid(ng as i32) as usize);
            }
            Action::SetDrums(g) => self.drums = g.filter(|&g| g < ng),
            _ => return false,
        }
        // the drummer the new primary is: the primary alone
        if self.drums == Some(self.primary) {
            self.drums = None;
        }
        self.id() != before
    }
}

/// The lengths a piece can be asked to be (seconds), and the slider's longest.
pub const MIN_LENGTH: f64 = 30.0;
pub const MAX_LENGTH: f64 = 3600.0;
pub const SLIDER_LENGTH: f64 = 600.0;

/// A length as typed: auto (or 0) for the piece's own; M:SS; 3.5m or 210s;
/// a bare number in minutes under 30, else seconds. None if it is none of these.
pub fn parse_length(s: &str) -> Option<f64> {
    let s = s.trim().to_ascii_lowercase();
    if s == "auto" || s == "0" {
        return Some(0.0);
    }
    let num = |x: &str| x.trim().parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0);
    let secs = if let Some((m, sec)) = s.split_once(':') {
        let sec = num(sec).filter(|&v| v < 60.0)?;
        num(m)?.trunc() * 60.0 + sec
    } else if let Some(x) = s.strip_suffix('s') {
        num(x)?
    } else if let Some(x) = s.strip_suffix('m') {
        num(x)? * 60.0
    } else {
        let x = num(&s)?;
        if x < 30.0 { x * 60.0 } else { x }
    };
    (secs > 0.0).then(|| secs.clamp(MIN_LENGTH, MAX_LENGTH).round())
}

/// How far - and + turn the volume down and up (per cent).
pub const VOLUME_STEP: u32 = 5;

/// A step louder or quieter (`by` steps), from 0 to 100.
fn louder(volume: u32, by: i32) -> u32 {
    (volume as i32 + by * VOLUME_STEP as i32).clamp(0, 100) as u32
}

/// `by` steps more or less of an effect, from 0 to `EFFECT_MAX`.
fn more(effect: u32, by: i32) -> u32 {
    (effect as i32 + by * music::EFFECT_STEP as i32).clamp(0, music::EFFECT_MAX as i32) as u32
}

/// Each effect's keys (less, more), and what it does, as `music::FX_NAMES`
/// orders them.
pub const FX_KEYS: [(char, char); music::NFX] =
    [('7', '8'), ('9', '0'), (';', '\''), ('5', '6'), ('1', '2'), ('3', '4')];
pub const FX_HELP: [&str; music::NFX] = [
    "The reverb: 100% is the genre's own, 0 none; past 100 it is wetter and its tail longer, at 500 a cathedral's.",
    "The echo: 100% is the genre's own (many have none), 0 none; past 100 an echo in time repeats the tunes, longer the more of it.",
    "The chorus: 100% is the genre's own (most have none), 0 none; past 100 copies of the keys, pads, guitars and tunes, swung a little out of tune, thicken them.",
    "The stereo width: 100% as it was mixed, 0 mono; past 100 the sides louder than the middle, wider than the speakers at 500.",
    "The bass, below about 150 Hz: 100% as it was mixed, 0 none, 200 twice as loud, 500 five times.",
    "The treble, above about 3.5 kHz: 100% as it was mixed, 0 none (dull), 200 twice as loud, 500 five times.",
];

/// A key's step to an effect, if it is one of `FX_KEYS`.
pub fn fx_key(c: char) -> Option<Action> {
    let k = FX_KEYS.iter().position(|&(lo, hi)| c == lo || c == hi)?;
    Some(Action::MoreEffect(k, if c == FX_KEYS[k].0 { -1 } else { 1 }))
}

/// A length as shown: M:SS, or "auto".
pub fn show_length(secs: f64) -> String {
    if secs <= 0.0 { "auto".into() } else { clock(secs) }
}

/// Seconds as M:SS.
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// 15 s longer or shorter: from auto, about the piece's own length (`planned`);
/// under the shortest, auto again.
fn longer(length: f64, planned: f64, by: i32) -> f64 {
    let base = if length > 0.0 { length } else { (planned / 15.0).round() * 15.0 };
    let v = base + 15.0 * by as f64;
    if v < MIN_LENGTH { 0.0 } else { v.min(SLIDER_LENGTH.max(base)).min(MAX_LENGTH) }
}

/// How far , and . go back and on in a song (seconds).
pub const SEEK_SECS: f64 = 10.0;

/// How far into a song Back still goes to the one before (seconds).
pub const BACK_SECS: f64 = 3.0;

/// What the actions ask of the music: the desktop's player, or the page's
/// worker (by a message).
pub trait Deck {
    fn skip(&self);
    fn replay(&self);
    fn playing(&self) -> bool;
    fn set_playing(&self, on: bool);
    fn seek(&self, secs: f64);
    fn go(&self, id: music::SongId);
    fn set_genre(&self, id: usize);
    fn set_length(&self, secs: f64);
    fn set_volume(&self, volume: u32);
    fn set_effects(&self, e: music::Effects);
    fn set_opts(&self, opts: music::Opts);
    /// the song playing saved as an MP3
    fn export(&self);
}

/// Plays the song `id`, the displays' genre, drummer, length and settings
/// those it asks.
pub fn go_to(deck: &impl Deck, st: &mut State, id: music::SongId) {
    (st.primary, st.drums) = music::parts(id.music);
    st.opts = id.opts;
    st.length = id.length;
    st.seed = id.seed;
    deck.go(id);
    deck.set_playing(true);
}

/// Does what `a` asks of the music and of `st` (`now`: what is playing);
/// what the window or the terminal does itself (the clipboard, the window,
/// its panels, the terminal's equalizer, quitting) is handed back.
pub fn act(a: Action, now: &Now, st: &mut State, deck: &impl Deck) -> Option<Action> {
    match a {
        // on through the songs gone back from, else a new one
        Action::Skip => match now.fwd {
            Some(id) => go_to(deck, st, id),
            None => deck.skip(),
        },
        Action::Back => {
            match now.back {
                Some(id) if now.elapsed < BACK_SECS => go_to(deck, st, id),
                _ => deck.seek(0.0),
            }
            deck.set_playing(true);
        }
        Action::Seek(by) => deck.seek((now.elapsed + by).max(0.0)),
        Action::SeekTo(secs) => deck.seek(secs),
        Action::Go(id) => go_to(deck, st, id),
        Action::Play => deck.set_playing(!deck.playing()),
        Action::Replay => {
            // (the count first: a stopped player starts on the piece's start)
            deck.replay();
            deck.set_playing(true);
        }
        Action::SetLength(v) => {
            st.length = v;
            deck.set_length(v);
        }
        Action::Longer(by) => {
            st.length = longer(st.length, now.planned, by);
            deck.set_length(st.length);
        }
        Action::SetVolume(v) => {
            st.volume = v.min(100);
            deck.set_volume(st.volume);
        }
        Action::Louder(by) => {
            st.volume = louder(st.volume, by);
            deck.set_volume(st.volume);
        }
        Action::SetEffect(..) | Action::MoreEffect(..) | Action::ResetEffects => {
            match a {
                Action::SetEffect(k, v) if k < music::NFX => st.effects.0[k] = v.min(music::EFFECT_MAX),
                Action::MoreEffect(k, by) if k < music::NFX => st.effects.0[k] = more(st.effects.0[k], by),
                Action::ResetEffects => st.effects = music::Effects::default(),
                _ => {}
            }
            deck.set_effects(st.effects);
        }
        Action::Keep => match now.sid {
            Some(id) => match st.favorites.toggle(&id) {
                Ok(true) => st.note = Some((format!("{id} kept among the favorites"), Instant::now())),
                Ok(false) => st.note = Some((format!("{id} no longer a favorite"), Instant::now())),
                Err(e) => st.msg = Some((format!("not kept: {e}"), Instant::now())),
            },
            None => st.msg = Some(("the song has not begun".into(), Instant::now())),
        },
        Action::Forget(id) => {
            if st.favorites.has(&id)
                && let Err(e) = st.favorites.toggle(&id)
            {
                st.msg = Some((format!("not let go: {e}"), Instant::now()));
            }
        }
        Action::Export => deck.export(),
        Action::Tuned | Action::NetTunes | Action::Voices | Action::Varied | Action::SetOpts(_) => {
            match a {
                Action::Tuned => st.opts.tuned = !st.opts.tuned,
                Action::NetTunes => st.opts.net_tunes = !st.opts.net_tunes,
                Action::Voices => st.opts.voices = st.opts.voices.next(),
                Action::Varied => st.opts.varied = !st.opts.varied,
                Action::SetOpts(o) => st.opts = o,
                _ => {}
            }
            // (the song made again from its start, the new way: a stopped
            // player starts on it)
            deck.set_opts(st.opts);
            deck.set_playing(true);
        }
        Action::Genre(_) | Action::Drums(_) | Action::SetGenre(_) | Action::SetDrums(_) => {
            if st.apply(a) {
                deck.set_genre(st.id());
            }
        }
        a => return Some(a),
    }
    None
}
