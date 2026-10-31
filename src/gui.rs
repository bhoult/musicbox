//! The window: what is playing and its id, where in the song it is (a bar
//! to click or drag to a point in it), a live equalizer of what the speakers
//! play (LED bars with peak markers over a frequency scale), the transport
//! (stop, back, replay, next song) with the songs' length on a slider and in
//! a field to type it into, and the genre selector, a grid of the genres to write the
//! music and one of the drummers, with the window's keys as buttons below.
//! Drawn on the CPU into a small picture that SDL scales up by a whole number.

use crate::font::{self, ADVANCE};
use crate::sdl::{self, Window};
use crate::spectrum::{self, Spectrum};
use crate::tap::{Now, Tap};
use crate::{Action, MIN_LENGTH, SLIDER_LENGTH, State, clock, music, parse_length, show_length};

const BG: u32 = 0x0E1116;
const PANEL: u32 = 0x141A21;
const TEXT: u32 = 0xEBEEF5;
const DIM: u32 = 0x8C96A5;
const ACCENT: u32 = 0x5FD3C6;
const AMBER: u32 = 0xF2B544;
const LED_OFF: u32 = 0x1B232C;
const PEAK: u32 = 0xF4F6FA;

/// A button's box and what a left and a right click on it do.
struct Button {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    left: Action,
    right: Option<Action>,
}

pub struct Gui {
    win: Window,
    px: Vec<u32>,
    w: i32,
    h: i32,
    spec: Spectrum,
    /// the mouse, in the picture's pixels
    mouse: Option<(i32, i32)>,
    buttons: Vec<Button>,
    title: usize,
    /// the length slider's track and the length field, as last drawn (x0, y0, x1, y1)
    slider: Option<(i32, i32, i32, i32)>,
    field: Option<(i32, i32, i32, i32)>,
    /// the slider held, and the length it last set
    drag: Option<f64>,
    /// the song's bar, as last drawn, the seconds it spans, and the point
    /// in the song it is held at (gone to when let go)
    bar: Option<(i32, i32, i32, i32)>,
    span: f64,
    scrub: Option<f64>,
    /// a length being typed into the field (T, or a click on it), or a
    /// song's id (D); the key's own text still to come; and a word about the
    /// last one typed
    edit: Option<String>,
    edit_song: bool,
    eat: Option<char>,
    note: Option<(String, std::time::Instant)>,
}

/// An option of a choice of how the music is made: its label, and the
/// settings it asks for.
type Choice = (&'static str, music::Opts);

fn inside(r: Option<(i32, i32, i32, i32)>, (x, y): (i32, i32)) -> bool {
    r.is_some_and(|(x0, y0, x1, y1)| x >= x0 && x < x1 && y >= y0 && y < y1)
}

impl Gui {
    pub fn open() -> Result<Gui, String> {
        let win = Window::open("musicbox", 1280, 720)?;
        Ok(Gui {
            win,
            px: Vec::new(),
            w: 0,
            h: 0,
            spec: Spectrum::new(48000),
            mouse: None,
            buttons: Vec::new(),
            title: usize::MAX,
            slider: None,
            field: None,
            drag: None,
            bar: None,
            span: 0.0,
            scrub: None,
            edit: None,
            edit_song: false,
            eat: None,
            note: None,
        })
    }

    pub fn fullscreen(&mut self) {
        self.win.toggle_fullscreen();
    }

    /// The length at `x` on the slider: 0:30 to 10:00, in steps of 15 s.
    fn slider_length(&self, x: i32) -> f64 {
        let Some((x0, _, x1, _)) = self.slider else { return 0.0 };
        let f = ((x - x0) as f64 / (x1 - x0 - 1).max(1) as f64).clamp(0.0, 1.0);
        ((MIN_LENGTH + f * (SLIDER_LENGTH - MIN_LENGTH)) / 15.0).round() * 15.0
    }

    /// Where a length sits on the slider.
    fn slider_x(&self, secs: f64) -> i32 {
        let Some((x0, _, x1, _)) = self.slider else { return 0 };
        let f = ((secs - MIN_LENGTH) / (SLIDER_LENGTH - MIN_LENGTH)).clamp(0.0, 1.0);
        x0 + (f * (x1 - x0 - 1) as f64).round() as i32
    }

    /// The point in the song at `x` on its bar (seconds).
    fn bar_secs(&self, x: i32) -> f64 {
        let Some((x0, _, x1, _)) = self.bar else { return 0.0 };
        ((x - x0) as f64 / (x1 - x0 - 1).max(1) as f64).clamp(0.0, 1.0) * self.span
    }

    /// A key while a length or a song's id is typed: Enter sets it (plays
    /// the song), Esc forgets it.
    fn edit_key(&mut self, k: i32, v: &mut Vec<Action>) {
        let Some(text) = &mut self.edit else { return };
        match k {
            sdl::K_BACKSPACE => {
                text.pop();
            }
            sdl::K_RETURN | sdl::K_KP_ENTER => {
                let now = std::time::Instant::now();
                if self.edit_song {
                    match music::SongId::parse(text) {
                        Some(id) => v.push(Action::Go(id)),
                        None => self.note = Some((format!("NOT A SONG ID: {text}"), now)),
                    }
                } else {
                    match parse_length(text) {
                        Some(x) => v.push(Action::SetLength(x)),
                        None => self.note = Some((format!("NOT A LENGTH: {text}"), now)),
                    }
                }
                self.edit = None;
            }
            sdl::K_ESCAPE => self.edit = None,
            _ => {}
        }
    }

    /// The window's keys and clicks, as actions (its close: `Window`, which closes it).
    pub fn events(&mut self) -> Vec<Action> {
        let mut v = Vec::new();
        while let Some(e) = self.win.poll() {
            match e.kind {
                sdl::SDL_WINDOWEVENT if e.window_event() == sdl::SDL_WINDOWEVENT_CLOSE => v.push(Action::Window),
                sdl::SDL_WINDOWEVENT if e.window_event() == sdl::SDL_WINDOWEVENT_LEAVE => self.mouse = None,
                sdl::SDL_MOUSEMOTION => {
                    let (x, y) = e.mouse_xy();
                    let (x, y) = self.win.to_picture(x, y);
                    self.mouse = Some((x, y));
                    if let Some(was) = self.drag {
                        let l = self.slider_length(x);
                        if l != was {
                            self.drag = Some(l);
                            v.push(Action::SetLength(l));
                        }
                    }
                    if self.scrub.is_some() {
                        self.scrub = Some(self.bar_secs(x));
                    }
                }
                sdl::SDL_MOUSEBUTTONUP => {
                    self.drag = None;
                    v.extend(self.scrub.take().map(Action::SeekTo));
                }
                sdl::SDL_MOUSEWHEEL => {
                    let over = self.mouse.is_some_and(|m| inside(self.slider, m) || inside(self.field, m));
                    if over && e.wheel() != 0 {
                        v.push(Action::Longer(e.wheel().signum()));
                    }
                }
                sdl::SDL_TEXTINPUT => {
                    let t = e.text();
                    let eat = self.eat.take().is_some_and(|k| t.eq_ignore_ascii_case(&k.to_string()));
                    let most = if self.edit_song { 48 } else { 12 };
                    if let Some(text) = self.edit.as_mut().filter(|_| !eat) {
                        for c in t.chars().filter(|c| c.is_ascii_graphic()) {
                            if text.len() < most {
                                text.push(c);
                            }
                        }
                    }
                }
                sdl::SDL_MOUSEBUTTONDOWN => {
                    let (x, y) = e.mouse_xy();
                    let (x, y) = self.win.to_picture(x, y);
                    self.mouse = Some((x, y));
                    let left = e.button() == sdl::SDL_BUTTON_LEFT;
                    // a click anywhere else forgets a length half typed
                    if !inside(self.field, (x, y)) {
                        self.edit = None;
                    }
                    if left && inside(self.bar, (x, y)) {
                        self.scrub = Some(self.bar_secs(x));
                    } else if left && inside(self.slider, (x, y)) {
                        let l = self.slider_length(x);
                        self.drag = Some(l);
                        v.push(Action::SetLength(l));
                    } else if left && inside(self.field, (x, y)) {
                        if self.edit_song {
                            self.edit = None;
                        }
                        self.edit_song = false;
                        self.edit.get_or_insert_with(String::new);
                    } else if let Some(b) = self.buttons.iter().find(|b| x >= b.x0 && x < b.x1 && y >= b.y0 && y < b.y1)
                    {
                        match e.button() {
                            sdl::SDL_BUTTON_LEFT => v.push(b.left),
                            sdl::SDL_BUTTON_RIGHT => v.extend(b.right),
                            _ => {}
                        }
                    }
                }
                sdl::SDL_KEYDOWN if self.edit.is_some() => self.edit_key(e.key(), &mut v),
                sdl::SDL_KEYDOWN => {
                    let a = match e.key() {
                        sdl::K_RIGHT => Some(Action::Genre(1)),
                        sdl::K_LEFT => Some(Action::Genre(-1)),
                        sdl::K_UP => Some(Action::Drums(1)),
                        sdl::K_DOWN => Some(Action::Drums(-1)),
                        sdl::K_ESCAPE => Some(Action::Window),
                        sdl::K_F11 => Some(Action::Fullscreen),
                        k => match u8::try_from(k).map(|c| c as char) {
                            Ok(' ' | '\r' | 'y') => Some(Action::Skip),
                            Ok('s') => Some(Action::Play),
                            Ok('r') => Some(Action::Replay),
                            Ok('b') => Some(Action::Back),
                            Ok(',') => Some(Action::Seek(-crate::term::SEEK_SECS)),
                            Ok('.') => Some(Action::Seek(crate::term::SEEK_SECS)),
                            Ok('x') => Some(Action::Export),
                            Ok('[') => Some(Action::Longer(-1)),
                            Ok(']') => Some(Action::Longer(1)),
                            Ok(c @ ('t' | 'd')) => {
                                self.edit = Some(String::new());
                                self.edit_song = c == 'd';
                                self.eat = Some(c);
                                None
                            }
                            Ok('n' | 'l') => Some(Action::Genre(1)),
                            Ok('p' | 'h') => Some(Action::Genre(-1)),
                            Ok('j') => Some(Action::Drums(1)),
                            Ok('e') => Some(Action::Eq),
                            Ok('o') => Some(Action::Tuned),
                            Ok('m') => Some(Action::NetTunes),
                            Ok('i') => Some(Action::Voices),
                            Ok('g') => Some(Action::Window),
                            Ok('f') => Some(Action::Fullscreen),
                            Ok('q') => Some(Action::Quit),
                            _ => None,
                        },
                    };
                    v.extend(a);
                }
                _ => {}
            }
        }
        v
    }

    pub fn draw(&mut self, tap: &Tap, now: &Now, st: &State) {
        let (w, h) = self.win.size();
        let (w, h) = (w as i32, h as i32);
        if (w, h) != (self.w, self.h) {
            (self.w, self.h) = (w, h);
            self.px = vec![0; (w * h) as usize];
        }
        if self.title != st.id() {
            self.title = st.id();
            self.win.set_title(&format!("musicbox: {}", music::name(st.id()).to_lowercase()));
        }
        self.px.fill(0xFF00_0000 | BG);
        self.buttons.clear();
        let m = 8;

        // what is playing
        self.text(m, m, "MUSICBOX", ACCENT, 2);
        let mut right = now.sid.map_or_else(|| format!("SEED {}", st.seed), |id| format!("SONG {id}"));
        if !st.sampled {
            right += "  NO RECORDINGS";
        }
        let rw = font::width(&right, 1);
        self.text(w - m - rw, m + 4, &right, DIM, 1);
        let playing = if now.bpm > 0.0 { now.id } else { st.id() };
        let (p, d) = music::parts(playing);
        let mut y = m + 22;
        let big = if font::width(&music::name(playing), 2) + 160 < w - 2 * m { 2 } else { 1 };
        let mut x = m;
        x += self.text(x, y, music::GENRES[p], TEXT, big);
        if let Some(d) = d {
            x += self.text(x, y, " + ", DIM, big);
            x += self.text(x, y, music::GENRES[d], AMBER, big);
            self.text(x, y, " DRUMS", DIM, big);
        }
        y += 7 * big + 6;
        if self.note.as_ref().is_some_and(|n| n.1.elapsed().as_secs() >= 4) {
            self.note = None;
        }
        let msg = st.msg.as_ref().or(self.note.as_ref());
        let typing = self.edit.as_ref().filter(|_| self.edit_song);
        let line = match msg {
            _ if typing.is_some() => {
                format!("SONG ID: {}_   AS SHOWN AT THE TOP, SKA.2817.0.3: ENTER PLAYS IT, ESC CANCELS", typing.expect("typed").to_uppercase())
            }
            Some((msg, _)) => msg.clone(),
            None if st.note.is_some() => st.note.as_ref().expect("a note").0.to_uppercase(),
            None if now.bpm > 0.0 => {
                let mut s = format!("SONG {}  /  {}  /  {:.0} BPM", now.piece, now.meter, now.bpm);
                if !now.key.is_empty() {
                    // the font is upper case: Ab would read AB
                    let key = now.key.strip_suffix('b').map_or(now.key.to_string(), |k| format!("{k} FLAT"));
                    s += &format!("  /  {key} {}", now.mode);
                }
                if !now.section.is_empty() {
                    s += &format!("  /  {} {} OF {}", now.section, now.bar.max(1), now.bars);
                }
                s += &format!("  /  {}", clock(now.elapsed));
                if now.length > 0.0 {
                    s += &format!(" OF {}", clock(now.length));
                }
                if now.stopped {
                    s += "  /  STOPPED";
                }
                s
            }
            None => "STARTING".into(),
        };
        let color = if typing.is_some() {
            TEXT
        } else if msg.is_some() {
            0xFF6B5F
        } else if st.note.is_some() {
            ACCENT
        } else {
            DIM
        };
        self.text(m, y, &line, color, 1);
        if playing != st.id() {
            let next = format!("NEXT: {}", music::name(st.id()));
            self.text(w - m - font::width(&next, 1), y, &next, ACCENT, 1);
        }
        let y = y + 14;

        // where in the song it is: a bar to click or drag to a point in it,
        // between buttons 10 s back and on
        let bh = 13;
        let fill = (0x30353F, 0x4A5160);
        let seek = crate::term::SEEK_SECS;
        let lw = font::width(",", 1) + font::width("<< 10 S", 1) + ADVANCE + 8;
        self.button(m, y, lw, bh, "<< 10 S", ",", fill, None, Action::Seek(-seek), None);
        let rw = font::width(".", 1) + font::width("10 S >>", 1) + ADVANCE + 8;
        self.button(w - m - rw, y, rw, bh, "10 S >>", ".", fill, None, Action::Seek(seek), None);
        self.span = if now.length > 0.0 { now.length } else { now.planned }.max(1.0);
        let at = self.scrub.unwrap_or(now.elapsed).min(self.span);
        let time = format!("{} / {}", clock(at), clock(self.span));
        let tw = font::width("00:00 / 00:00", 1);
        let (bx0, bx1) = (m + lw + 8, (w - m - rw - tw - 16).max(m + lw + 48));
        self.bar = Some((bx0, y, bx1, y + bh));
        self.rect(bx0, y + 4, bx1 - bx0, 5, 0x30353F);
        let hx = bx0 + (at / self.span * (bx1 - bx0 - 1) as f64).round() as i32;
        self.rect(bx0, y + 4, hx - bx0, 5, if self.scrub.is_some() { AMBER } else { ACCENT });
        self.rect(hx - 1, y + 1, 3, bh - 2, TEXT);
        self.text(bx1 + 8, y + 3, &time, DIM, 1);
        let top = y + bh + 6;

        // the selector, from the bottom up
        let gap = 3;
        let bh = 13;
        let cols = ((w - 2 * m + gap) / (84 + gap)).clamp(3, 13);
        let bw = (w - 2 * m - (cols - 1) * gap) / cols;
        let rows = |n: i32| (n + cols - 1) / cols;
        let ng = music::GENRES.len() as i32;
        let height = 2 * (bh + 8) + 2 * 11 + (rows(ng) + rows(ng + 1)) * (bh + gap) + 6 + bh;
        let mut y = h - m - height;
        let bottom = y - 6;

        // the transport, and the songs' length
        let mut x = m;
        let keys: [(&str, &str, Action); 5] = [
            ("S", if now.stopped { "START" } else { "STOP" }, Action::Play),
            ("B", "BACK", Action::Back),
            ("R", "REPLAY", Action::Replay),
            ("SPACE", "NEXT SONG", Action::Skip),
            ("X", "EXPORT MP3", Action::Export),
        ];
        for (hint, label, a) in keys {
            let bw = font::width(hint, 1) + font::width(label, 1) + ADVANCE + 8;
            self.button(x, y, bw, bh, label, hint, (0x30353F, 0x4A5160), None, a, None);
            x += bw + 6;
        }
        x += 6;
        x += self.text(x, y + 3, "LENGTH", DIM, 1) + 6;
        let auto = st.length <= 0.0;
        let aw = font::width("AUTO", 1) + 8;
        let fill = if auto { (0x2A5FC1, 0x3F78E0) } else { (0x30353F, 0x4A5160) };
        self.button(x, y, aw, bh, "AUTO", "", fill, None, Action::SetLength(0.0), None);
        x += aw + 8;
        let fw = font::width("00:00", 1) + 10;
        let (sx0, sx1) = (x, (w - m - fw - 8).max(x + 40));
        self.slider = Some((sx0, y, sx1, y + bh));
        self.slider_track(st, now, sx0, sx1, y, bh);
        let fx = sx1 + 8;
        self.field = Some((fx, y, fx + fw, y + bh));
        let editing = self.edit.is_some() && !self.edit_song;
        self.rect(fx, y, fw, bh, if editing { ACCENT } else { 0x30353F });
        self.rect(fx + 1, y + 1, fw - 2, bh - 2, PANEL);
        let shown = match self.edit.as_ref().filter(|_| !self.edit_song) {
            Some(t) => format!("{}_", t.to_uppercase()),
            None => show_length(st.length).to_uppercase(),
        };
        let tw = font::width(&shown, 1);
        self.text(fx + (fw - tw).max(4) / 2, y + 3, &shown, if editing { TEXT } else { ACCENT }, 1);
        y += bh + 8;
        self.made(st.opts, st.id(), m, y, bh);
        y += bh + 8;

        self.text(m, y, "MUSIC BY", DIM, 1);
        self.text(m + 9 * ADVANCE, y, "(RIGHT CLICK: DRUMS BY)", 0x5C6573, 1);
        y += 11;
        for g in 0..ng {
            let (bx, by) = (m + (g % cols) * (bw + gap), y + (g / cols) * (bh + gap));
            let g = g as usize;
            let fill = if g == st.primary { (0x2A5FC1, 0x3F78E0) } else { (0x30353F, 0x4A5160) };
            let mark = (Some(g) == st.drums).then_some(AMBER);
            self.button(
                bx,
                by,
                bw,
                bh,
                music::GENRES[g],
                "",
                fill,
                mark,
                Action::SetGenre(g),
                Some(Action::SetDrums(Some(g))),
            );
        }
        y += rows(ng) * (bh + gap) + 2;
        self.text(m, y, "DRUMS BY", DIM, 1);
        y += 11;
        for k in 0..=ng {
            let (bx, by) = (m + (k % cols) * (bw + gap), y + (k / cols) * (bh + gap));
            let g = (k > 0).then(|| k as usize - 1);
            let label = g.map_or("NONE", |g| music::GENRES[g]);
            let fill = if g == st.drums {
                (0x8A5A12, 0xA8701C)
            } else if g == Some(st.primary) {
                (0x1C2027, 0x1C2027)
            } else {
                (0x30353F, 0x4A5160)
            };
            self.button(bx, by, bw, bh, label, "", fill, None, Action::SetDrums(g), None);
        }
        y += rows(ng + 1) * (bh + gap) + 4;
        let mut x = m;
        let keys: [(&str, &str, Action); 3] =
            [("F", "FULLSCREEN", Action::Fullscreen), ("ESC", "CLOSE", Action::Window), ("Q", "QUIT", Action::Quit)];
        for (hint, label, a) in keys {
            let bw = font::width(hint, 1) + font::width(label, 1) + ADVANCE + 8;
            self.button(x, y, bw, bh, label, hint, (0x30353F, 0x4A5160), None, a, None);
            x += bw + 6;
        }

        // the equalizer
        if bottom - top > 24 {
            self.equalizer(tap, m, top, w - m, bottom);
        }
        self.win.present(&self.px);
    }

    /// How the music is made, a row of three choices, each a key and its
    /// options side by side, the one chosen lit: the settings as written
    /// or tuned, the tunes by the rules or the network, the instruments. An
    /// option underlined does something for the music `id` (the optimiser
    /// tuned it, a network writes its tunes, network instruments are in).
    fn made(&mut self, o: music::Opts, id: usize, x0: i32, y: i32, bh: i32) {
        use music::Voices;
        let groups: [(&str, &str, Vec<Choice>); 3] = [
            (
                "O",
                "TUNING",
                vec![
                    ("AS WRITTEN", music::Opts { tuned: false, ..o }),
                    ("OPTIMISED", music::Opts { tuned: true, ..o }),
                ],
            ),
            (
                "M",
                "TUNES",
                vec![
                    ("RULES", music::Opts { net_tunes: false, ..o }),
                    ("NETWORK", music::Opts { net_tunes: true, ..o }),
                ],
            ),
            (
                "I",
                "INSTRUMENTS",
                Voices::ALL
                    .iter()
                    .map(|&v| (["SYNTH", "RECORDED", "NETWORK"][v as usize], music::Opts { voices: v, ..o }))
                    .collect(),
            ),
        ];
        let mut x = x0;
        for (key, label, opts) in groups {
            x += self.text(x, y + 3, key, 0x9AA6B8, 1) + ADVANCE;
            x += self.text(x, y + 3, label, DIM, 1) + 6;
            for (name, to) in opts {
                let bw = font::width(name, 1) + 8;
                let fill = if to == o { (0x2A5FC1, 0x3F78E0) } else { (0x30353F, 0x4A5160) };
                let real = match (label, name) {
                    ("TUNING", "OPTIMISED") => music::has_tuning(id),
                    ("TUNES", "NETWORK") => music::has_net_tunes(id),
                    ("INSTRUMENTS", "NETWORK") => crate::net::count() > 0,
                    _ => false,
                };
                self.button(x, y, bw, bh, name, "", fill, real.then_some(ACCENT), Action::SetOpts(to), None);
                x += bw + 2;
            }
            x += 14;
        }
    }

    /// The length slider between x0 and x1: a tick a minute, the length
    /// asked lit up to its handle; at auto, a hollow mark at the piece's own.
    fn slider_track(&mut self, st: &State, now: &Now, x0: i32, x1: i32, y: i32, h: i32) {
        let mid = y + h / 2;
        self.rect(x0, mid - 1, x1 - x0, 3, LED_OFF);
        for min in 1..=(SLIDER_LENGTH / 60.0) as i32 {
            let tx = self.slider_x(min as f64 * 60.0);
            let tall = if min % 5 == 0 { 4 } else { 2 };
            self.rect(tx, mid - 1 - tall, 1, 2 * tall + 3, 0x3A4350);
        }
        if st.length > 0.0 {
            let hx = self.slider_x(st.length);
            self.rect(x0, mid - 1, hx - x0, 3, ACCENT);
            let over = self.drag.is_some() || self.mouse.is_some_and(|m| inside(self.slider, m));
            self.rect(hx - 2, y + 1, 5, h - 2, if over { PEAK } else { TEXT });
        } else if now.planned > 0.0 {
            let hx = self.slider_x(now.planned);
            self.rect(hx - 2, y + 1, 5, h - 2, DIM);
            self.rect(hx - 1, y + 2, 3, h - 4, PANEL);
        }
    }

    /// The bars between x0 and x1, from y0 down to the frequency scale above y1.
    fn equalizer(&mut self, tap: &Tap, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.rect(x0, y0, x1 - x0, y1 - y0, PANEL);
        let (x0, x1, y0) = (x0 + 4, x1 - 4, y0 + 4);
        let y1 = y1 - 12;
        let n = ((x1 - x0) / 9).clamp(16, 96);
        self.spec.update(tap, n as usize);
        let slot = (x1 - x0) as f32 / n as f32;
        let bw = (slot.round() as i32 - 2).max(1);
        let segs = ((y1 - y0) / 4).max(1);
        for b in 0..n {
            let bx = x0 + (b as f32 * slot).round() as i32;
            let lit = (self.spec.bars[b as usize] * segs as f32).round() as i32;
            let peak = (self.spec.peaks[b as usize] * segs as f32).round() as i32;
            for s in 0..segs {
                let c = if s < lit { led(s as f32 / segs as f32) } else { LED_OFF };
                self.rect(bx, y1 - (s + 1) * 4 + 1, bw, 3, c);
            }
            if peak > 0 {
                self.rect(bx, y1 - peak * 4 + 1, bw, 2, PEAK);
            }
        }
        for (f, label) in [
            (50.0, "50"),
            (100.0, "100"),
            (200.0, "200"),
            (500.0, "500"),
            (1000.0, "1K"),
            (2000.0, "2K"),
            (5000.0, "5K"),
            (10000.0, "10K"),
        ] {
            let x = x0 + (spectrum::position(f) * (x1 - x0) as f32) as i32;
            self.text(x - font::width(label, 1) / 2, y1 + 3, label, 0x5C6573, 1);
        }
    }

    /// A button: `fill` (and its colour under the mouse), a key `hint`
    /// before its label, a `mark` underlining it.
    #[allow(clippy::too_many_arguments)]
    fn button(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        hint: &str,
        fill: (u32, u32),
        mark: Option<u32>,
        left: Action,
        right: Option<Action>,
    ) {
        let over = self.mouse.is_some_and(|(mx, my)| mx >= x && mx < x + w && my >= y && my < y + h);
        self.rect(x, y, w, h, if over { fill.1 } else { fill.0 });
        let tw = font::width(label, 1) + if hint.is_empty() { 0 } else { font::width(hint, 1) + ADVANCE };
        let mut tx = x + (w - tw) / 2;
        if !hint.is_empty() {
            tx += self.text(tx, y + 3, hint, 0x9AA6B8, 1);
            tx += ADVANCE;
        }
        self.text(tx, y + 3, label, TEXT, 1);
        if let Some(c) = mark {
            self.rect(x + 2, y + h - 2, w - 4, 2, c);
        }
        self.buttons.push(Button { x0: x, y0: y, x1: x + w, y1: y + h, left, right });
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: u32) {
        let (xa, xb) = (x.max(0), (x + w).min(self.w));
        for yy in y.max(0)..(y + h).min(self.h) {
            let row = (yy * self.w) as usize;
            if xb > xa {
                self.px[row + xa as usize..row + xb as usize].fill(0xFF00_0000 | rgb);
            }
        }
    }

    /// Draws `s`; returns its advance.
    fn text(&mut self, x: i32, y: i32, s: &str, rgb: u32, scale: i32) -> i32 {
        font::draw(&mut self.px, self.w, self.h, x, y, s, rgb, scale);
        s.len() as i32 * ADVANCE * scale
    }
}

/// An LED's colour at height `t` (0 bottom, 1 top): teal, then amber, then red.
fn led(t: f32) -> u32 {
    let mix = |a: u32, b: u32, k: f32| {
        let ch = |s: u32| ((a >> s & 0xFF) as f32 * (1.0 - k) + (b >> s & 0xFF) as f32 * k) as u32;
        ch(16) << 16 | ch(8) << 8 | ch(0)
    };
    match t {
        t if t < 0.55 => mix(0x1E9E92, 0x5FD3C6, t / 0.55),
        t if t < 0.8 => mix(0x5FD3C6, 0xF2B544, (t - 0.55) / 0.25),
        t => mix(0xF2B544, 0xEF4F5F, ((t - 0.8) / 0.2).min(1.0)),
    }
}
