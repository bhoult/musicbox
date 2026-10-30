//! musicbox: music made up as it plays, without end, in twelve genres and any
//! pair of them, shown on the command line, with a window (a live equalizer
//! and a genre selector) to open alongside.

mod font;
mod gui;
mod music;
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
    /// a new piece of the same music
    Skip,
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
    /// the terminal's equalizer rows
    pub eq: bool,
    /// a message for a few seconds (a window that would not open)
    pub msg: Option<(String, Instant)>,
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

fn usage() -> ! {
    help(2)
}

/// The usage, then exit with `code` (0 when asked for).
fn help(code: i32) -> ! {
    eprintln!("usage: musicbox [GENRE[+DRUMS]] [--seed N] [--gui] [--eq] [--samples DIR] [--no-samples]");
    eprintln!("       musicbox --wav GENRE[+DRUMS] SECONDS OUT.wav [--seed N] [--ending natural|fade|cut] [--fade SECS]");
    eprintln!("       musicbox --list | --samples-list");
    eprintln!();
    eprintln!("  GENRE: {} (or its number, 0-11)", music::cli_names());
    eprintln!("         two joined with + (polka+techno): the first writes the music, the second drums it");
    eprintln!("  --gui: open the window (a live equalizer and the genre selector); G opens and closes it too");
    eprintln!("  --eq: the equalizer in the terminal as well; E turns it on and off");
    eprintln!("  --seed N: the music the same every time for a genre and N (random by default)");
    eprintln!("  --samples DIR: the recorded instruments (tools/get-samples.sh fetches them; default");
    eprintln!("         ~/.local/share/musicbox/samples); --no-samples: the synthesiser alone");
    eprintln!("  --wav: SECONDS of the music to a 48 kHz WAV; --ending: how it ends (cut by default);");
    eprintln!("         --fade: faded in over half of SECS");
    eprintln!();
    eprintln!("  keys: left/right genre, up/down drums, space next song, E equalizer, G window, Q quit");
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed: Option<u64> = None;
    let (mut gui, mut eq, mut no_samples) = (false, false, false);
    let mut samples_dir: Option<String> = None;
    let mut wav: Option<Vec<String>> = None;
    let mut ending = music::Ending::Cut;
    let mut fade = 0.0f64;
    let mut genre: Option<usize> = None;
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
            "--ending" => {
                i += 1;
                ending = music::Ending::parse(&value(i)).unwrap_or_else(|| usage());
            }
            "--fade" => {
                i += 1;
                fade = value(i).parse().unwrap_or_else(|_| usage());
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
    let seed = seed.unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u32 as u64)
    });
    if let Some(w) = wav {
        music::cli(&w, seed, ending, fade * 0.5);
        return;
    }
    let id = genre.unwrap_or((seed % music::GENRES.len() as u64) as usize);
    play(id, seed, gui, eq);
}

/// Plays until Q, Ctrl-C or the window's close (when there is no terminal).
fn play(id: usize, seed: u64, gui: bool, eq: bool) {
    unsafe {
        for sig in [1, 2, 15] {
            signal(sig, on_signal as usize);
        }
    }
    sdl::no_signal_handlers();
    // the recorded instruments before the first note: every piece played on them
    let sampled = samples::load_now();
    let player = match music::Player::start(id, seed) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("musicbox: no sound: {e}");
            std::process::exit(1);
        }
    };
    let tap = player.tap();
    let (primary, drums) = music::parts(id);
    let mut st = State { primary, drums, seed, sampled, eq, msg: None };
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
    let mut piece_start = (0u64, 0.0f64);
    while !QUIT.load(Ordering::Relaxed) {
        let t0 = Instant::now();
        let mut acts = term.as_mut().map(|t| t.keys()).unwrap_or_default();
        if let Some(g) = win.as_mut() {
            acts.extend(g.events());
        }
        for a in acts {
            match a {
                Action::Quit => QUIT.store(true, Ordering::Relaxed),
                Action::Skip => player.skip(),
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
        if now.piece != piece_start.0 {
            piece_start = (now.piece, tap.seconds());
        }
        let in_piece = (tap.seconds() - piece_start.1).max(0.0);
        if st.msg.as_ref().is_some_and(|m| m.1.elapsed() > Duration::from_secs(5)) {
            st.msg = None;
        }
        match term.as_mut() {
            Some(t) => t.draw(&tap, &now, &st, in_piece, win.is_some()),
            None if now.piece > logged => {
                logged = now.piece;
                println!("song {}: {} (seed {seed})", now.piece, term::describe(&now));
            }
            None => {}
        }
        if let Some(g) = win.as_mut() {
            g.draw(&tap, &now, &st, in_piece);
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
