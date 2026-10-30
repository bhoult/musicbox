//! The command line's display: a few lines redrawn in place (what is playing,
//! the genres with the one playing lit, the equalizer when it is on, the
//! keys), and the keys read raw from stdin. Only when stdin and stdout are
//! both terminals; the terminal is put back as it was on the way out.

use std::ffi::c_int;
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

use crate::spectrum::Spectrum;
use crate::tap::{Now, Tap};
use crate::{Action, State, music};

/// glibc's struct termios on Linux.
#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    iflag: u32,
    oflag: u32,
    cflag: u32,
    lflag: u32,
    line: u8,
    cc: [u8; 32],
    ispeed: u32,
    ospeed: u32,
}

#[repr(C)]
#[derive(Default)]
struct Winsize {
    rows: u16,
    cols: u16,
    xpixel: u16,
    ypixel: u16,
}

const ICANON: u32 = 0o2;
const ECHO: u32 = 0o10;
const VTIME: usize = 5;
const VMIN: usize = 6;
const TCSANOW: c_int = 0;
const TIOCGWINSZ: u64 = 0x5413;

unsafe extern "C" {
    fn isatty(fd: c_int) -> c_int;
    fn tcgetattr(fd: c_int, t: *mut Termios) -> c_int;
    fn tcsetattr(fd: c_int, when: c_int, t: *const Termios) -> c_int;
    fn ioctl(fd: c_int, req: u64, ...) -> c_int;
}

/// What the terminal was set to, for the panic hook to put back.
static SAVED: std::sync::Mutex<Option<Termios>> = std::sync::Mutex::new(None);

fn restore() {
    if let Some(t) = SAVED.lock().unwrap_or_else(|e| e.into_inner()).take() {
        unsafe { tcsetattr(0, TCSANOW, &t) };
        // the cursor back, below what was drawn
        let _ = std::io::stdout().write_all(b"\x1b[?25h\r\n");
        let _ = std::io::stdout().flush();
    }
}

pub struct Term {
    keys: Receiver<Action>,
    /// lines drawn last time (the cursor is on the last of them)
    drawn: usize,
    spec: Spectrum,
    last: Option<(String, Instant)>,
}

impl Term {
    /// Raw keys and the display, when stdin and stdout are terminals.
    pub fn start() -> Option<Term> {
        if unsafe { isatty(0) == 0 || isatty(1) == 0 } {
            return None;
        }
        let mut t = Termios { iflag: 0, oflag: 0, cflag: 0, lflag: 0, line: 0, cc: [0; 32], ispeed: 0, ospeed: 0 };
        if unsafe { tcgetattr(0, &mut t) } != 0 {
            return None;
        }
        *SAVED.lock().unwrap_or_else(|e| e.into_inner()) = Some(t);
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            prev(info);
        }));
        // keys one at a time, unechoed; Ctrl-C still a signal
        let mut raw = t;
        raw.lflag &= !(ICANON | ECHO);
        raw.cc[VMIN] = 1;
        raw.cc[VTIME] = 0;
        unsafe { tcsetattr(0, TCSANOW, &raw) };
        let (tx, keys) = channel();
        let _ = std::thread::Builder::new().name("keys".into()).spawn(move || {
            let mut buf = [0u8; 64];
            let mut stdin = std::io::stdin();
            while let Ok(n) = stdin.read(&mut buf) {
                if n == 0 {
                    break;
                }
                for a in parse_keys(&buf[..n]) {
                    if tx.send(a).is_err() {
                        return;
                    }
                }
            }
        });
        print!("\x1b[?25l");
        Some(Term { keys, drawn: 0, spec: Spectrum::new(48000), last: None })
    }

    pub fn keys(&mut self) -> Vec<Action> {
        self.keys.try_iter().collect()
    }

    /// Redraws the lines: at once when they change, at most 30 times a second
    /// while the equalizer moves, else twice a second.
    pub fn draw(&mut self, tap: &Tap, now: &Now, st: &State, in_piece: f64, window: bool) {
        let cols = cols().saturating_sub(1).max(20);
        let mut lines: Vec<Line> = Vec::new();

        let mut l = Line::new(cols);
        l.push("♪ ", "36");
        l.push("musicbox  ", "1");
        let playing = if now.bpm > 0.0 { now.id } else { st.id() };
        let (p, d) = music::parts(playing);
        l.push(&music::GENRES[p].to_lowercase(), "1;36");
        if let Some(d) = d {
            l.push(" with ", "2");
            l.push(&music::GENRES[d].to_lowercase(), "1;33");
            l.push(" drums", "2");
        }
        if playing != st.id() {
            l.push(" -> ", "2");
            l.push(&music::name(st.id()).to_lowercase(), "36");
            l.push(" at the next bar", "2");
        }
        let right = format!("seed {}{}", st.seed, if st.sampled { "" } else { " · synthesiser" });
        l.right(&right, "2");
        lines.push(l);

        let mut l = Line::new(cols);
        l.push("  ", "");
        match &st.msg {
            Some((m, _)) => l.push(m, "31"),
            None if now.bpm > 0.0 => {
                l.push(&describe_piece(now), "");
                l.push(&format!(" · {}:{:02}", in_piece as u64 / 60, in_piece as u64 % 60), "2");
            }
            None => l.push("starting", "2"),
        }
        lines.push(l);

        // the genres, the primary lit and the drummer marked; a window of
        // them around the primary when the terminal is narrow
        let mut l = Line::new(cols);
        l.push("  ", "");
        let names: Vec<String> = music::GENRES.iter().map(|g| g.to_lowercase()).collect();
        let span = |a: usize, b: usize| names[a..b].iter().map(|n| n.chars().count() + 1).sum::<usize>();
        let mut first = 0;
        if span(0, names.len()) + 2 > cols {
            while first < st.primary && span(first, st.primary + 1) + 4 > cols * 2 / 3 {
                first += 1;
            }
        }
        if first > 0 {
            l.push("‹ ", "2");
        }
        for (g, n) in names.iter().enumerate().skip(first) {
            let style = if g == st.primary {
                "1;7;36"
            } else if Some(g) == st.drums {
                "1;4;33"
            } else {
                "2"
            };
            l.push(n, style);
            l.push(" ", "");
        }
        lines.push(l);

        if st.eq {
            let n = cols.saturating_sub(4).min(96).max(8);
            self.spec.update(tap, n);
            const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
            for row in (0..2).rev() {
                let mut l = Line::new(cols);
                l.push("  ", "");
                for b in 0..n {
                    let h = self.spec.bars[b];
                    let eighths = ((h * 16.0).round() as i32 - row * 8).clamp(0, 8) as usize;
                    let color = if row == 0 {
                        "36"
                    } else if h > 0.85 {
                        "31"
                    } else {
                        "33"
                    };
                    l.push(&BLOCKS[eighths].to_string(), color);
                }
                lines.push(l);
            }
        }

        let mut l = Line::new(cols);
        l.push("  ←/→ genre  ↑/↓ drums  space next song  e equalizer  ", "2");
        l.push(if window { "g close window  " } else { "g window  " }, "2");
        l.push("q quit", "2");
        lines.push(l);

        let text: String = lines.iter().map(|l| l.out.as_str()).collect::<Vec<_>>().join("\n");
        let due = match &self.last {
            Some((t, at)) => *t != text && at.elapsed().as_millis() >= 30 || at.elapsed().as_millis() >= 500,
            None => true,
        };
        if !due {
            return;
        }
        let mut out = String::new();
        if self.drawn > 1 {
            out.push_str(&format!("\x1b[{}A", self.drawn - 1));
        }
        out.push('\r');
        for (i, l) in lines.iter().enumerate() {
            out.push_str("\x1b[2K");
            out.push_str(&l.out);
            out.push_str("\x1b[0m");
            if i + 1 < lines.len() {
                out.push_str("\r\n");
            }
        }
        // fewer lines than last time: clear the rest
        if self.drawn > lines.len() {
            out.push_str("\x1b[J");
        }
        let mut so = std::io::stdout();
        let _ = so.write_all(out.as_bytes());
        let _ = so.flush();
        self.drawn = lines.len();
        self.last = Some((text, Instant::now()));
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        restore();
    }
}

/// A line of styled text, cut at `max` characters.
struct Line {
    out: String,
    width: usize,
    max: usize,
}

impl Line {
    fn new(max: usize) -> Line {
        Line { out: String::new(), width: 0, max }
    }

    fn push(&mut self, text: &str, sgr: &str) {
        let room = self.max.saturating_sub(self.width);
        let t: String = text.chars().take(room).collect();
        if t.is_empty() {
            return;
        }
        self.width += t.chars().count();
        if sgr.is_empty() {
            self.out.push_str(&t);
        } else {
            self.out.push_str(&format!("\x1b[{sgr}m{t}\x1b[0m"));
        }
    }

    /// `text` flush right, if it fits with a gap.
    fn right(&mut self, text: &str, sgr: &str) {
        let n = text.chars().count();
        if self.width + 2 + n <= self.max {
            let pad = self.max - self.width - n;
            self.push(&" ".repeat(pad), "");
            self.push(text, sgr);
        }
    }
}

fn cols() -> usize {
    let mut w = Winsize::default();
    let ok = unsafe { ioctl(1, TIOCGWINSZ, &mut w as *mut Winsize) } == 0;
    if ok && w.cols > 0 { w.cols as usize } else { 80 }
}

/// Keys as they come from a terminal: arrows as escape sequences, a lone
/// escape as itself.
fn parse_keys(b: &[u8]) -> Vec<Action> {
    let mut v = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let a = match b[i] {
            0x1b if b.get(i + 1).is_some_and(|&c| c == b'[' || c == b'O') => {
                let k = b.get(i + 2).copied();
                i += 2;
                match k {
                    Some(b'C') => Some(Action::Genre(1)),
                    Some(b'D') => Some(Action::Genre(-1)),
                    Some(b'A') => Some(Action::Drums(1)),
                    Some(b'B') => Some(Action::Drums(-1)),
                    _ => None,
                }
            }
            0x1b => Some(Action::Quit),
            b'q' | b'Q' => Some(Action::Quit),
            b' ' | b'\n' | b'\r' | b'y' | b'Y' => Some(Action::Skip),
            b'n' | b'l' => Some(Action::Genre(1)),
            b'N' | b'p' | b'h' => Some(Action::Genre(-1)),
            b'j' => Some(Action::Drums(1)),
            b'J' => Some(Action::Drums(-1)),
            b'e' | b'E' => Some(Action::Eq),
            b'g' | b'G' => Some(Action::Window),
            _ => None,
        };
        v.extend(a);
        i += 1;
    }
    v
}

/// The piece: its number, meter (and form), tempo, key, section and bar.
fn describe_piece(now: &Now) -> String {
    let mut s = format!("song {} · {} · {:.0} bpm", now.piece, now.meter, now.bpm);
    if !now.key.is_empty() {
        s += &format!(" · {} {}", now.key, now.mode);
    }
    if !now.section.is_empty() {
        s += &format!(" · {}, bar {} of {}", now.section, now.bar.max(1), now.bars);
    }
    s
}

/// A piece in a line of its own, for a log: the music and the piece.
pub fn describe(now: &Now) -> String {
    let mut s = format!("{}, {}, {:.0} bpm", music::name(now.id).to_lowercase(), now.meter, now.bpm);
    if !now.key.is_empty() {
        s += &format!(", {} {}", now.key, now.mode);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_parse() {
        assert_eq!(
            parse_keys(b"\x1b[C\x1b[D q"),
            vec![Action::Genre(1), Action::Genre(-1), Action::Skip, Action::Quit]
        );
        assert_eq!(parse_keys(b"\x1b"), vec![Action::Quit]);
        assert_eq!(parse_keys(b"\x1bOA"), vec![Action::Drums(1)]);
        assert_eq!(parse_keys(b"eg"), vec![Action::Eq, Action::Window]);
    }

    #[test]
    fn lines_cut_at_the_width() {
        let mut l = Line::new(10);
        l.push("abcdef", "1");
        l.push("ghijkl", "");
        assert_eq!(l.width, 10);
        assert!(l.out.ends_with("ghij"));
        l.right("x", "");
        assert_eq!(l.width, 10);
    }
}
