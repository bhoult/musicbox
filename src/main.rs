//! musicbox: music made up as it plays, without end, in twelve genres and any
//! pair of them, shown on the command line, with a window (a live equalizer
//! and a genre selector) to open alongside.

mod font;
mod gui;
mod mp3;
mod music;
mod net;
mod samples;
mod sdl;
mod spectrum;
mod tap;
mod term;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Set by SIGINT, SIGTERM and SIGHUP: the main loop ends, the terminal is put back.
static QUIT: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: std::ffi::c_int) {
    QUIT.store(true, Ordering::Relaxed);
}

unsafe extern "C" {
    fn signal(sig: std::ffi::c_int, handler: usize) -> usize;
}

/// What a key, a click or a button asks for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    /// the next or previous genre, and the next or previous drummer (none among them)
    Genre(i32),
    Drums(i32),
    SetGenre(usize),
    SetDrums(Option<usize>),
    /// a new piece of the same music; stop or play on; the piece playing
    /// again from its start
    Skip,
    Play,
    Replay,
    /// the length asked of each piece: set (seconds; 0: as long as it was
    /// written), or 15 s longer or shorter
    SetLength(f64),
    Longer(i32),
    /// the piece playing saved as an MP3
    Export,
    /// how the music is made: the optimiser's settings on or off, the
    /// network's tunes on or off, the next to play the instruments; all three as given
    Tuned,
    NetTunes,
    Voices,
    SetOpts(music::Opts),
    /// the terminal's equalizer on or off, the window open or closed, fullscreen
    Eq,
    Window,
    Fullscreen,
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
    fn apply(&mut self, a: Action) -> bool {
        let ng = music::GENRES.len();
        let before = self.id();
        match a {
            Action::Genre(d) => self.primary = (self.primary as i32 + d).rem_euclid(ng as i32) as usize,
            Action::SetGenre(g) => self.primary = g % ng,
            Action::Drums(d) => {
                // none, then each genre but the primary
                let order: Vec<Option<usize>> =
                    std::iter::once(None).chain((0..ng).filter(|&g| g != self.primary).map(Some)).collect();
                let i = order.iter().position(|&x| x == self.drums).unwrap_or(0) as i32;
                self.drums = order[(i + d).rem_euclid(order.len() as i32) as usize];
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

fn usage() -> ! {
    help(2)
}

/// The usage, then exit with `code` (0 when asked for).
fn help(code: i32) -> ! {
    eprintln!(
        "usage: musicbox [GENRE[+DRUMS]] [--seed N] [--length L] [--gui] [--eq] [--samples DIR] [--no-samples] [MADE]"
    );
    eprintln!(
        "       musicbox --wav GENRE[+DRUMS] SECONDS OUT.wav [--seed N] [--length L] [--ending natural|fade|cut] [--fade SECS]"
    );
    eprintln!("       musicbox --export GENRE[+DRUMS] OUT.mp3 [--seed N] [--song K] [--length L]");
    eprintln!("       musicbox --list | --samples-list | --web-pack DIR");
    eprintln!("  MADE: [--tuned] [--net-tunes] [--voices synth|recorded|network], for any of the above");
    eprintln!();
    eprintln!("  GENRE: {} (or its number, 0-11)", music::cli_names());
    eprintln!("         two joined with + (polka+techno): the first writes the music, the second drums it");
    eprintln!("  --gui: open the window (a live equalizer and the genre selector); G opens and closes it too");
    eprintln!("  --eq: the equalizer in the terminal as well; E turns it on and off");
    eprintln!("  --seed N: the music the same every time for a genre and N (random by default)");
    eprintln!("  --length L: how long each song is (3:00, 2.5m, 150s; auto, the default: as long as it was written)");
    eprintln!("  --samples DIR: the recorded instruments (tools/get-samples.sh fetches them; default");
    eprintln!("         ~/.local/share/musicbox/samples); --no-samples: the synthesiser alone");
    eprintln!("  --export: the Kth song (1 by default) of the music and seed on its own, start to end, to an MP3");
    eprintln!("         (or a WAV, named .wav); X exports the song playing to ~/Music/musicbox");
    eprintln!("  --wav: SECONDS of the music to a 48 kHz WAV; --ending: how it ends (cut by default);");
    eprintln!("         --fade: faded in over half of SECS");
    eprintln!("  --tuned: each genre's settings as the optimiser tuned them against real music (O)");
    eprintln!("  --net-tunes: tunes written by the network where the genre has one, else by its rules (M)");
    eprintln!("  --voices: what plays the instruments: the synthesiser, the recordings (the default), or the");
    eprintln!("         network's instruments where it has one, else the recording (I)");
    eprintln!();
    eprintln!("  keys: left/right genre, up/down drums, space next song, S stop/start, R replay, [ ] length,");
    eprintln!("        T type a length, X export the song, O tuned, M network tunes, I instruments,");
    eprintln!("        E equalizer, G window, Q quit");
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed: Option<u64> = None;
    let (mut gui, mut eq, mut no_samples) = (false, false, false);
    let mut samples_dir: Option<String> = None;
    let mut wav: Option<Vec<String>> = None;
    let mut export: Option<Vec<String>> = None;
    let mut song = 1usize;
    let mut ending = music::Ending::Cut;
    let mut fade = 0.0f64;
    let mut length = 0.0f64;
    let mut genre: Option<usize> = None;
    let mut opts = music::Opts::default();
    let mut voices: Option<music::Voices> = None;
    let mut i = 0;
    let value = |i: usize| args.get(i).cloned().unwrap_or_else(|| usage());
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => {
                i += 1;
                seed = Some(value(i).parse().unwrap_or_else(|_| usage()));
            }
            "--gui" => gui = true,
            "--eq" => eq = true,
            "--no-samples" => no_samples = true,
            "--samples" => {
                i += 1;
                samples_dir = Some(value(i));
            }
            "--wav" => {
                wav = Some(args.get(i + 1..i + 4).unwrap_or_else(|| usage()).to_vec());
                i += 3;
            }
            "--export" => {
                export = Some(args.get(i + 1..i + 3).unwrap_or_else(|| usage()).to_vec());
                i += 2;
            }
            "--song" => {
                i += 1;
                song = value(i).parse().ok().filter(|&k| k >= 1).unwrap_or_else(|| usage());
            }
            "--ending" => {
                i += 1;
                ending = music::Ending::parse(&value(i)).unwrap_or_else(|| usage());
            }
            "--fade" => {
                i += 1;
                fade = value(i).parse().unwrap_or_else(|_| usage());
            }
            "--length" => {
                i += 1;
                length = parse_length(&value(i)).unwrap_or_else(|| usage());
            }
            "--tuned" => opts.tuned = true,
            "--net-tunes" => opts.net_tunes = true,
            "--voices" => {
                i += 1;
                voices = Some(music::Voices::parse(&value(i)).unwrap_or_else(|| usage()));
            }
            "--list" => {
                for (n, g) in music::GENRES.iter().enumerate() {
                    println!("{n:2} {}", g.to_lowercase());
                }
                return;
            }
            "--samples-list" => {
                samples::configure(samples_dir, false);
                samples::list_cli();
                return;
            }
            "--voice-data" => {
                // DIR, then the instruments' names
                let dir = value(i + 1);
                let names: Vec<String> = args[i + 2..].to_vec();
                samples::configure(samples_dir, false);
                if !samples::load_now() {
                    std::process::exit(1);
                }
                if let Err(e) = samples::voice_data(std::path::Path::new(&dir), &names, 2) {
                    eprintln!("musicbox: --voice-data: {e}");
                    std::process::exit(1);
                }
                return;
            }
            "--web-pack" => {
                i += 1;
                let dir = value(i);
                samples::configure(samples_dir, false);
                music::web_pack_cli(&dir);
                return;
            }
            "-h" | "--help" => help(0),
            a if !a.starts_with('-') && genre.is_none() => match music::find(a) {
                Some(g) => genre = Some(g),
                None => {
                    eprintln!("musicbox: no genre {a}: one of {}, or two (polka+techno)", music::cli_names());
                    std::process::exit(2);
                }
            },
            _ => usage(),
        }
        i += 1;
    }
    samples::configure(samples_dir, no_samples);
    // without the recordings, the synthesiser plays (as it would anyway)
    opts.voices = voices.unwrap_or(if no_samples { music::Voices::Synth } else { music::Voices::Recorded });
    let seed = seed.unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u32 as u64)
    });
    if let Some(w) = wav {
        music::cli(&w, seed, ending, fade * 0.5, length, opts);
        return;
    }
    if let Some(x) = export {
        music::export_cli(&x, seed, song, length, opts);
        return;
    }
    let id = genre.unwrap_or((seed % music::GENRES.len() as u64) as usize);
    play(id, seed, gui, eq, length, opts);
}

/// Where X saves songs: ~/Music/musicbox.
fn export_dir() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map_or_else(|| ".".into(), std::path::PathBuf::from);
    home.join("Music").join("musicbox")
}

/// Plays until Q, Ctrl-C or the window's close (when there is no terminal).
fn play(id: usize, seed: u64, gui: bool, eq: bool, length: f64, opts: music::Opts) {
    unsafe {
        for sig in [1, 2, 15] {
            signal(sig, on_signal as usize);
        }
    }
    sdl::no_signal_handlers();
    // the recorded instruments before the first note: every piece played on them
    let sampled = samples::load_now();
    let player = match music::Player::start(id, seed, opts) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("musicbox: no sound: {e}");
            std::process::exit(1);
        }
    };
    player.set_length(length);
    let tap = player.tap();
    let (primary, drums) = music::parts(id);
    let mut st = State { primary, drums, seed, sampled, opts, eq, length, msg: None, note: None };
    let mut term = term::Term::start();
    let mut win: Option<gui::Gui> = None;
    let toggle_window = |win: &mut Option<gui::Gui>, st: &mut State| {
        if win.take().is_none() {
            match gui::Gui::open() {
                Ok(g) => *win = Some(g),
                Err(e) => st.msg = Some((format!("no window: {e}"), Instant::now())),
            }
        }
    };
    if gui {
        toggle_window(&mut win, &mut st);
        if win.is_none() && term.is_none() {
            eprintln!("musicbox: {}", st.msg.as_ref().map_or("no window", |m| m.0.as_str()));
            std::process::exit(1);
        }
    }
    // without a terminal to draw in, a line for each new piece
    let mut logged = 0u64;
    while !QUIT.load(Ordering::Relaxed) {
        let t0 = Instant::now();
        let now = tap.now();
        let mut acts = term.as_mut().map(|t| t.keys()).unwrap_or_default();
        if let Some(g) = win.as_mut() {
            acts.extend(g.events());
        }
        for a in acts {
            match a {
                Action::Quit => QUIT.store(true, Ordering::Relaxed),
                Action::Skip => player.skip(),
                Action::Play => player.set_playing(!player.playing()),
                Action::Replay => {
                    // (the count first: a stopped player starts on the piece's start)
                    player.replay();
                    player.set_playing(true);
                }
                Action::SetLength(v) => {
                    st.length = v;
                    player.set_length(v);
                }
                Action::Longer(by) => {
                    st.length = longer(st.length, now.planned, by);
                    player.set_length(st.length);
                }
                Action::Export => player.export(export_dir()),
                Action::Tuned | Action::NetTunes | Action::Voices | Action::SetOpts(_) => {
                    match a {
                        Action::Tuned => st.opts.tuned = !st.opts.tuned,
                        Action::NetTunes => st.opts.net_tunes = !st.opts.net_tunes,
                        Action::Voices => st.opts.voices = st.opts.voices.next(),
                        Action::SetOpts(o) => st.opts = o,
                        _ => {}
                    }
                    // (the song made again from its start, the new way: a
                    // stopped player starts on it)
                    player.set_opts(st.opts);
                    player.set_playing(true);
                }
                Action::Eq => st.eq = !st.eq,
                Action::Window => {
                    toggle_window(&mut win, &mut st);
                    // the window closed with no terminal to go on in
                    if win.is_none() && term.is_none() {
                        QUIT.store(true, Ordering::Relaxed);
                    }
                }
                Action::Fullscreen => {
                    if let Some(g) = win.as_mut() {
                        g.fullscreen();
                    }
                }
                a => {
                    if st.apply(a) {
                        player.set_genre(st.id());
                    }
                }
            }
        }
        let now = tap.now();
        if let Some(n) = player.news() {
            // (the home directory as ~)
            let n = match std::env::var("HOME") {
                Ok(h) if !h.is_empty() => n.replace(&h, "~"),
                _ => n,
            };
            if term.is_none() {
                println!("{n}");
            }
            st.note = Some((n, Instant::now()));
        }
        if st.note.as_ref().is_some_and(|m| m.1.elapsed() > Duration::from_secs(8)) {
            st.note = None;
        }
        if st.msg.as_ref().is_some_and(|m| m.1.elapsed() > Duration::from_secs(5)) {
            st.msg = None;
        }
        match term.as_mut() {
            Some(t) => t.draw(&tap, &now, &st, win.is_some()),
            None if now.piece > logged => {
                logged = now.piece;
                println!("song {}: {} (seed {seed})", now.piece, term::describe(&now));
            }
            None => {}
        }
        if let Some(g) = win.as_mut() {
            g.draw(&tap, &now, &st);
        }
        // the window's vsync paces the loop when it is open; else about 30 a second
        let frame = if win.is_some() { Duration::from_millis(15) } else { Duration::from_millis(33) };
        if let Some(rest) = frame.checked_sub(t0.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    drop(win);
    drop(term);
    drop(player);
}
