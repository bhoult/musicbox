//! musicbox: music made up as it plays, without end, in 28 genres and any
//! pair of them, shown on the command line, with a window (a live equalizer
//! and a genre selector) to open alongside.

mod app;
mod favorites;
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
mod time;
mod tunes;

use app::*;

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

/// An effect's per cent as given (0 to `EFFECT_MAX`, a % sign or not).
fn parse_effect(s: &str) -> Option<u32> {
    s.trim_end_matches('%').parse().ok().filter(|&v| v <= music::EFFECT_MAX)
}

fn usage() -> ! {
    help(2)
}

/// The usage, then exit with `code` (0 when asked for).
fn help(code: i32) -> ! {
    eprintln!(
        "usage: musicbox [GENRE[+DRUMS] | ID] [--seed N] [--length L] [--volume V] [--gui] [--touch] [--eq] [--samples DIR] [--no-samples] [MADE] [ROOM]"
    );
    eprintln!(
        "       musicbox --wav GENRE[+DRUMS] SECONDS OUT.wav [--seed N] [--length L] [--ending natural|fade|cut] [--fade SECS]"
    );
    eprintln!("       musicbox --export GENRE[+DRUMS] OUT.mp3 [--seed N] [--song K] [--length L] [ROOM]");
    eprintln!("       musicbox --export ID OUT.mp3 [ROOM]");
    eprintln!("       musicbox --list | --samples-list | --web-pack DIR");
    eprintln!(
        "  MADE: [--tuned] [--net-tunes] [--voices synth|recorded|network] [--variety more|less], for any of the above"
    );
    eprintln!(
        "  ROOM: [--reverb R] [--echo E] [--chorus C] [--width W] [--bass B] [--treble T], 0 to 500 per cent (100, the default: as the genre has them)"
    );
    eprintln!();
    eprintln!("  GENRE: {} (or its number, 0-{})", music::cli_names(), music::GENRES.len() - 1);
    eprintln!("         two joined with + (polka+techno): the first writes the music, the second drums it");
    eprintln!("  ID: a song's id, as the displays show it (ska.2817.0.3): that song, and on from it");
    eprintln!("  --gui: open the window (a live equalizer and the genre selector); G opens and closes it too");
    eprintln!("  --touch: the window laid out for a finger, as the page is on a phone (opens it)");
    eprintln!("  --eq: the equalizer in the terminal as well; E turns it on and off");
    eprintln!("  --seed N: the music the same every time for a genre and N (random by default)");
    eprintln!("  --variety more|less: each genre's songs written with more to draw on (more, the default: other");
    eprintln!("         beats, chords and players, tunes that change as they come round), or as they first were");
    eprintln!("         (less); an ID says which itself (a v among its letters: more)");
    eprintln!("  --length L: how long each song is (3:00, 2.5m, 150s; auto, the default: as long as it was written)");
    eprintln!("  --volume V: how loud it plays, 0 to 100 (100 by default); - and + turn it down and up");
    eprintln!("  --reverb, --echo, --chorus: more or less of the genre's own, 0 (none) to 500 per cent; past 100");
    eprintln!("         the reverb is longer, an echo repeats the tunes, copies thicken the keys and tunes");
    eprintln!("  --width, --bass, --treble: the stereo width (0 mono), the bass and the treble, 0 to 500 per cent");
    eprintln!("         of the mix's; keys 7 8 reverb, 9 0 echo, ; ' chorus, 5 6 width, 1 2 bass, 3 4 treble");
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
    eprintln!("  keys: left/right genre, up/down drums, space next song, B back, , and . 10 s back and on,");
    eprintln!("        S stop/start, R replay, [ ] length, T type a length, D or V type (or paste) a song's id,");
    eprintln!("        C copy its id, K keep it among the favorites, - + volume, the effects' keys,");
    eprintln!("        X export the song, O tuned, M network tunes, I instruments, W variety, E equalizer, G window, Q quit");
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed: Option<u64> = None;
    let (mut gui, mut eq, mut no_samples) = (false, false, false);
    let mut touch = false;
    let mut samples_dir: Option<String> = None;
    let mut wav: Option<Vec<String>> = None;
    let mut export: Option<Vec<String>> = None;
    let mut song = 1usize;
    let mut ending = music::Ending::Cut;
    let mut fade = 0.0f64;
    let mut length = 0.0f64;
    let mut volume = 100u32;
    let mut effects = music::Effects::default();
    let mut genre: Option<usize> = None;
    let mut from: Option<music::SongId> = None;
    let mut opts = music::Opts::default();
    let mut variety: Option<bool> = None;
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
            "--touch" => (gui, touch) = (true, true),
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
            "--volume" => {
                i += 1;
                volume = value(i).trim_end_matches('%').parse().ok().filter(|&v| v <= 100).unwrap_or_else(|| usage());
            }
            a if a.strip_prefix("--").is_some_and(|n| music::FX_NAMES.contains(&n)) => {
                let k = music::FX_NAMES.iter().position(|&n| a[2..] == *n).unwrap_or(0);
                i += 1;
                effects.0[k] = parse_effect(&value(i)).unwrap_or_else(|| usage());
            }
            "--tuned" => opts.tuned = true,
            "--net-tunes" => opts.net_tunes = true,
            "--variety" => {
                i += 1;
                variety = Some(match value(i).as_str() {
                    "more" => true,
                    "less" => false,
                    _ => usage(),
                });
            }
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
            a if !a.starts_with('-') && genre.is_none() => match (music::SongId::parse(a), music::find(a)) {
                (Some(id), _) => {
                    // a song's id: the music, seed, length and settings it gives
                    from = Some(id);
                    genre = Some(id.music);
                    seed = Some(id.seed);
                    length = id.length;
                    opts = id.opts;
                    voices = Some(id.opts.voices);
                }
                (None, Some(g)) => genre = Some(g),
                (None, None) => {
                    eprintln!(
                        "musicbox: no genre or song {a}: one of {}, or two (polka+techno), or a song's id (ska.2817.0.3)",
                        music::cli_names()
                    );
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
    // more variety unless less is asked for; a song's id says which itself
    opts.varied = variety.unwrap_or(from.is_none_or(|id| id.opts.varied));
    let seed = seed.unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u32 as u64)
    });
    if let Some(w) = wav {
        music::cli(&w, seed, ending, fade * 0.5, length, opts);
        return;
    }
    if let Some(x) = export {
        music::export_cli(&x, seed, song, length, opts, effects);
        return;
    }
    let id = genre.unwrap_or((seed % music::GENRES.len() as u64) as usize);
    // (the listener's own way, for the runs of songs after an id's: more unless less is asked for)
    let vary = variety.unwrap_or(true);
    play(id, seed, gui, eq, Start { length, volume, effects, opts, from, touch, vary });
}

/// How playing starts: the length asked of each piece, the volume, the
/// effects, how the music is made, a song to begin on, whether the
/// window is laid out for a finger, and whether the listener has songs
/// written with more variety (a song begun on by its id is as its id says).
struct Start {
    length: f64,
    volume: u32,
    effects: music::Effects,
    opts: music::Opts,
    from: Option<music::SongId>,
    touch: bool,
    vary: bool,
}

/// Where X saves songs: ~/Music/musicbox.
fn export_dir() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map_or_else(|| ".".into(), std::path::PathBuf::from);
    home.join("Music").join("musicbox")
}

impl Deck for music::Player {
    fn skip(&self) {
        music::Player::skip(self)
    }
    fn replay(&self) {
        music::Player::replay(self)
    }
    fn playing(&self) -> bool {
        music::Player::playing(self)
    }
    fn set_playing(&self, on: bool) {
        music::Player::set_playing(self, on)
    }
    fn seek(&self, secs: f64) {
        music::Player::seek(self, secs)
    }
    fn go(&self, id: music::SongId) {
        music::Player::go(self, id)
    }
    fn set_genre(&self, id: usize) {
        music::Player::set_genre(self, id)
    }
    fn set_length(&self, secs: f64) {
        music::Player::set_length(self, secs)
    }
    fn set_volume(&self, volume: u32) {
        music::Player::set_volume(self, volume)
    }
    fn set_effects(&self, e: music::Effects) {
        music::Player::set_effects(self, e)
    }
    fn set_opts(&self, opts: music::Opts) {
        music::Player::set_opts(self, opts)
    }
    fn export(&self) {
        music::Player::export(self, export_dir())
    }
}

/// Plays until Q, Ctrl-C or the window's close (when there is no terminal).
fn play(id: usize, seed: u64, gui: bool, eq: bool, start: Start) {
    let Start { length, volume, effects, opts, from, touch, vary } = start;
    unsafe {
        for sig in [1, 2, 15] {
            signal(sig, on_signal as usize);
        }
    }
    sdl::no_signal_handlers();
    // the recorded instruments before the first note: every piece played on them
    let sampled = samples::load_now();
    let player = match music::Player::start(id, seed, opts, from, volume) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("musicbox: no sound: {e}");
            std::process::exit(1);
        }
    };
    player.set_length(length);
    player.set_effects(effects);
    let tap = player.tap();
    let (primary, drums) = music::parts(id);
    let favorites = favorites::Favorites::load();
    let mut st =
        State { primary, drums, seed, sampled, opts, vary, eq, length, volume, effects, favorites, msg: None, note: None };
    let mut term = term::Term::start();
    let mut win: Option<gui::Gui> = None;
    let toggle_window = |win: &mut Option<gui::Gui>, st: &mut State| {
        if win.take().is_none() {
            match gui::Gui::open(touch) {
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
            match app::act(a, &now, &mut st, &player) {
                Some(Action::Quit) => QUIT.store(true, Ordering::Relaxed),
                Some(Action::Copy) => match now.sid {
                    Some(id) => {
                        let text = id.to_string();
                        let copied = match (win.as_ref(), term.as_mut()) {
                            (Some(g), _) => g.copy(&text),
                            (None, Some(t)) => t.copy(&text),
                            (None, None) => false,
                        };
                        if copied {
                            st.note = Some((format!("copied {text}"), Instant::now()));
                        } else {
                            st.msg = Some(("the clipboard would not take it".into(), Instant::now()));
                        }
                    }
                    None => st.msg = Some(("the song has not begun".into(), Instant::now())),
                },
                Some(Action::Paste) => {
                    if let Some(g) = win.as_ref() {
                        match favorites::song_in(&g.pasted()) {
                            Some(id) => go_to(&player, &mut st, id),
                            None => st.msg = Some(("no song's id on the clipboard".into(), Instant::now())),
                        }
                    }
                }
                Some(Action::Eq) => st.eq = !st.eq,
                Some(Action::Window) => {
                    toggle_window(&mut win, &mut st);
                    // the window closed with no terminal to go on in
                    if win.is_none() && term.is_none() {
                        QUIT.store(true, Ordering::Relaxed);
                    }
                }
                Some(Action::Fullscreen) => {
                    if let Some(g) = win.as_mut() {
                        g.fullscreen();
                    }
                }
                Some(a @ (Action::Keys | Action::EffectsPanel | Action::FavoritesPanel)) => {
                    if let Some(g) = win.as_mut() {
                        g.toggle(a);
                    }
                }
                _ => {}
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
                let id = now.sid.map_or(String::new(), |i| format!(" ({i})"));
                println!("song {}: {}{id}", now.piece, term::describe(&now));
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
