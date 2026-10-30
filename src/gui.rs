//! The window: what is playing, a live equalizer of what the speakers play
//! (LED bars with peak markers over a frequency scale), and the genre
//! selector, a grid of the genres to write the music and one of the
//! drummers, with the music's keys as buttons below. Drawn on the CPU into a
//! small picture that SDL scales up by a whole number.

use crate::font::{self, ADVANCE};
use crate::sdl::{self, Window};
use crate::spectrum::{self, Spectrum};
use crate::tap::{Now, Tap};
use crate::{Action, State, music};

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
        })
    }

    pub fn fullscreen(&mut self) {
        self.win.toggle_fullscreen();
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
                    self.mouse = Some(self.win.to_picture(x, y));
                }
                sdl::SDL_MOUSEBUTTONDOWN => {
                    let (x, y) = e.mouse_xy();
                    let (x, y) = self.win.to_picture(x, y);
                    self.mouse = Some((x, y));
                    if let Some(b) = self.buttons.iter().find(|b| x >= b.x0 && x < b.x1 && y >= b.y0 && y < b.y1) {
                        match e.button() {
                            sdl::SDL_BUTTON_LEFT => v.push(b.left),
                            sdl::SDL_BUTTON_RIGHT => v.extend(b.right),
                            _ => {}
                        }
                    }
                }
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
                            Ok('n' | 'l') => Some(Action::Genre(1)),
                            Ok('p' | 'h') => Some(Action::Genre(-1)),
                            Ok('j') => Some(Action::Drums(1)),
                            Ok('e') => Some(Action::Eq),
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

    pub fn draw(&mut self, tap: &Tap, now: &Now, st: &State, in_piece: f64) {
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
        let mut right = format!("SEED {}", st.seed);
        if !st.sampled {
            right += "  SYNTHESISER";
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
        let line = match &st.msg {
            Some((msg, _)) => msg.clone(),
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
                s + &format!("  /  {}:{:02}", in_piece as u64 / 60, in_piece as u64 % 60)
            }
            None => "STARTING".into(),
        };
        let color = if st.msg.is_some() { 0xFF6B5F } else { DIM };
        self.text(m, y, &line, color, 1);
        if playing != st.id() {
            let next = format!("NEXT: {}", music::name(st.id()));
            self.text(w - m - font::width(&next, 1), y, &next, ACCENT, 1);
        }
        let top = y + 14;

        // the selector, from the bottom up
        let gap = 3;
        let bh = 13;
        let cols = ((w - 2 * m + gap) / (84 + gap)).clamp(3, 13);
        let bw = (w - 2 * m - (cols - 1) * gap) / cols;
        let rows = |n: i32| (n + cols - 1) / cols;
        let ng = music::GENRES.len() as i32;
        let height = 2 * 11 + (rows(ng) + rows(ng + 1)) * (bh + gap) + 6 + bh;
        let mut y = h - m - height;
        let bottom = y - 8;

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
        let keys: [(&str, &str, Action); 4] = [
            ("SPACE", "NEXT SONG", Action::Skip),
            ("F", "FULLSCREEN", Action::Fullscreen),
            ("ESC", "CLOSE", Action::Window),
            ("Q", "QUIT", Action::Quit),
        ];
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
