//! The window: what is playing and its id (a click on it copies it), where
//! in the song it is (a bar to click or drag to a point in it), a live
//! equalizer of what the speakers play (LED bars with peak markers over a
//! frequency scale), the transport (back, stop, next song, export) with the
//! songs' length on a slider and in a field to type it into, how the music is
//! made, and the genre selector, a grid of the genres: a click on one sets
//! what the switch in its heading says, the genre that writes the music or
//! the one on drums (a right-click: the one on drums, whichever), the
//! window's own buttons below and the volume on a slider beside them.
//! Whatever the mouse rests on says what it does and its keys; ? lists them
//! all.
//!
//! In a hand (a phone's screen, `--touch`, or a window too narrow for the
//! above) the same, laid out for a finger: what is playing, the song's bar,
//! the equalizer where there is room, the transport, and under them the
//! selector a part at a time, by its tabs: the music's genre, the drummer,
//! the sound (the volume and the effects), the song (its id, its length,
//! the favorites, the export) and how the music is made. Rows are a
//! finger's height; a finger acts when it lifts, and one that stays on
//! something is told what it does.
//! Drawn on the CPU into a small picture that SDL scales up by a whole number.

use std::time::Duration;

use crate::font::{self, ADVANCE};
use crate::sdl::{self, Window};
use crate::spectrum::{self, Spectrum};
use crate::tap::{Now, Tap};
use crate::time::Instant;
use crate::{Action, MIN_LENGTH, SLIDER_LENGTH, State, VOLUME_STEP, clock, music, parse_length, show_length};

/// The sliders: the volume (0), then the effects (1 + their index in
/// `music::FX_NAMES`), with their names and the keys that turn them
/// (`crate::FX_KEYS`, as the help names them).
const NKNOBS: usize = 1 + music::NFX;
const FX_LABELS: [&str; music::NFX] = ["REVERB", "ECHO", "CHORUS", "WIDTH", "BASS", "TREBLE"];
const FX_KEY_NAMES: [&str; music::NFX] = ["7 8", "9 0", "; '", "5 6", "1 2", "3 4"];

/// A slider's name, its most, the step a click lands on, the value its
/// taller tick marks (the volume's middle, the effects' 100 %), and its keys.
fn knob(k: usize) -> (&'static str, u32, u32, u32, &'static str) {
    match k {
        0 => ("VOLUME", 100, VOLUME_STEP, 50, "- +"),
        k => (FX_LABELS[k - 1], music::EFFECT_MAX, music::EFFECT_STEP, 100, FX_KEY_NAMES[k - 1]),
    }
}

/// What a slider does (`touch`: for a finger, which has no wheel).
fn knob_help(k: usize, touch: bool) -> String {
    match (k, touch) {
        (0, false) => {
            "The volume. It changes what you hear, not the equalizer or exports. The mouse wheel turns it too.".into()
        }
        (0, true) => "The volume. It changes what you hear, not the equalizer or exports.".into(),
        (k, false) => format!("{} Exports have it too. The mouse wheel turns it.", crate::FX_HELP[k - 1]),
        (k, true) => format!("{} Exports have it too.", crate::FX_HELP[k - 1]),
    }
}

/// A slider set, and turned a step.
fn knob_set(k: usize, v: u32) -> Action {
    if k == 0 { Action::SetVolume(v) } else { Action::SetEffect(k - 1, v) }
}

fn knob_by(k: usize, by: i32) -> Action {
    if k == 0 { Action::Louder(by) } else { Action::MoreEffect(k - 1, by) }
}

/// What the window shows in its equalizer's place: the effects' sliders, or
/// the favorites.
#[derive(Clone, Copy, PartialEq)]
enum Panel {
    Effects,
    Favorites,
}

/// How tall each panel needs to be (in the equalizer's place, else over the middle).
const EFFECTS_H: i32 = 84;
const FAVORITES_H: i32 = 104;

/// The selector's parts in a hand, as its tabs name them: the grid of the
/// genres for the music's, the same for the drummer, the volume and the
/// effects, the song (its id, its length, the favorites, the export), and
/// how the music is made. At a desk the grid alone, for the music's genre
/// or (`Drums`) the drummer.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Tab {
    Music,
    Drums,
    Sound,
    Song,
    More,
}

impl Tab {
    const ALL: [(Tab, &'static str, &'static str); 5] = [
        (Tab::Music, "MUSIC", "The genre that writes the music: its form, meter, chords and tunes."),
        (
            Tab::Drums,
            "DRUMS",
            "The genre on drums: its beat, fitted to the music's meter, and its kits and effects. Any genre can drum for any other.",
        ),
        (
            Tab::Sound,
            "SOUND",
            "The volume, and the effects: reverb, echo, chorus, stereo width, bass and treble. Underlined when an effect is turned.",
        ),
        (
            Tab::Song,
            "SONG",
            "This song: its id to copy or share, how long songs are, your favorites, and saving it as an MP3.",
        ),
        (Tab::More, "MORE", "How the music is made: its tuning, what writes the tunes, and what plays the notes."),
    ];
}

/// A row in a hand: a finger's height, with the gap under it.
const ROW: i32 = 24;

const BG: u32 = 0x0E1116;
const PANEL: u32 = 0x141A21;
const TEXT: u32 = 0xEBEEF5;
const DIM: u32 = 0x8C96A5;
const FAINT: u32 = 0x5C6573;
const ACCENT: u32 = 0x5FD3C6;
const AMBER: u32 = 0xF2B544;
const LED_OFF: u32 = 0x1B232C;
const PEAK: u32 = 0xF4F6FA;
const TIP: u32 = 0x1F2630;
const EDGE: u32 = 0x3A4350;
/// A button's colour, and under the mouse: plain, lit (chosen), the drummer.
const PLAIN: (u32, u32) = (0x30353F, 0x4A5160);
const LIT: (u32, u32) = (0x2A5FC1, 0x3F78E0);
const DRUM: (u32, u32) = (0x8A5A12, 0xA8701C);
/// How long the mouse rests on something before it says what it does, and
/// how long a finger stays on it (longer: a slow tap is still a tap).
const TIP_DELAY: Duration = Duration::from_millis(450);
const TIP_DELAY_FINGER: Duration = Duration::from_millis(600);

type Rect = (i32, i32, i32, i32);

/// A button's box and what a left and a right click on it do.
struct Button {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    left: Action,
    right: Option<Action>,
}

/// What a part of the window does, and its keys, for when the mouse rests on it.
struct Tip {
    r: Rect,
    help: String,
    keys: &'static str,
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
    slider: Option<Rect>,
    field: Option<Rect>,
    /// the slider held, and the length it last set
    drag: Option<f64>,
    /// the sliders (the volume, the effects), as last drawn, and (held)
    /// which and what it last set
    knob: [Option<Rect>; NKNOBS],
    knob_drag: Option<(usize, u32)>,
    /// the song's bar, as last drawn, the seconds it spans, and the point
    /// in the song it is held at (gone to when let go)
    bar: Option<Rect>,
    span: f64,
    scrub: Option<f64>,
    /// the song's id, as last drawn (a click on it copies it)
    sid: Option<Rect>,
    /// a length being typed into the field (T, or a click on it), or a
    /// song's id (D); the key's own text still to come; and a word about the
    /// last one typed
    edit: Option<String>,
    edit_song: bool,
    eat: Option<char>,
    note: Option<(String, Instant)>,
    /// what each part does, as last drawn; the part the mouse is on and
    /// since when; one clicked, its help held back until the mouse moves on
    tips: Vec<Tip>,
    hover: Option<(Rect, Instant)>,
    quiet: Option<Rect>,
    /// all the keys shown
    keys: bool,
    /// the panel shown in the equalizer's place, where it was last drawn,
    /// and the first favorite its list shows
    panel: Option<Panel>,
    panel_rect: Option<Rect>,
    fav_top: usize,
    /// laid out for a hand (as last drawn); the selector's part shown there
    /// (at a desk: what a click on a genre sets, the music's or the drummer)
    compact: bool,
    tab: Tab,
    /// a finger is down; the button it came down on (its box, and what it
    /// does if it lifts there); and whether what it rests on has said what
    /// it does (lifting then does nothing more)
    finger: bool,
    press: Option<(Rect, Action)>,
    tipped: bool,
}

/// An option of a choice of how the music is made: its label, the settings
/// it asks for, whether it changes the music playing, and what it does.
type Choice = (&'static str, music::Opts, bool, String);

fn inside(r: Option<Rect>, (x, y): (i32, i32)) -> bool {
    r.is_some_and(|(x0, y0, x1, y1)| x >= x0 && x < x1 && y >= y0 && y < y1)
}

/// `s` in lines of at most `cols` characters, broken at spaces.
fn wrap(s: &str, cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in s.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.len() + 1 + word.len() <= cols => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// The keys, as the list of them shows them.
const KEYS: [(&str, &str); 29] = [
    ("LEFT RIGHT", "GENRE"),
    ("UP DOWN", "DRUMS"),
    ("SPACE", "NEXT SONG"),
    ("B", "BACK"),
    ("R", "THIS SONG FROM ITS START"),
    ("S", "STOP / START"),
    (", .", "10 S BACK / ON"),
    ("- +", "VOLUME"),
    ("A", "THE EFFECTS"),
    ("7 8", "REVERB"),
    ("9 0", "ECHO"),
    ("; '", "CHORUS"),
    ("5 6", "WIDTH"),
    ("1 2", "BASS"),
    ("3 4", "TREBLE"),
    ("[ ]", "LENGTH"),
    ("T", "TYPE A LENGTH"),
    ("C", "COPY THE SONG'S ID"),
    ("V", "PLAY THE ID COPIED"),
    ("D", "TYPE A SONG'S ID"),
    ("K", "KEEP AS A FAVORITE"),
    ("X", "EXPORT AN MP3"),
    ("O", "TUNING"),
    ("M", "TUNES"),
    ("I", "INSTRUMENTS"),
    ("F  F11", "FULLSCREEN"),
    ("ESC  G", "CLOSE THE WINDOW"),
    ("Q", "QUIT"),
    ("?  F1", "THESE KEYS"),
];

/// In the browser the window is the page: no window to close, nor a program
/// to quit.
const WEB: bool = cfg!(target_arch = "wasm32");

/// The keys as this window lists them.
fn keys_listed() -> Vec<(&'static str, &'static str)> {
    KEYS.iter().copied().filter(|k| !WEB || !matches!(k.0, "ESC  G" | "Q")).collect()
}

impl Gui {
    /// The window, opened: `touch`, laid out for a finger whatever its size
    /// (the page finds that out for itself).
    pub fn open(touch: bool) -> Result<Gui, String> {
        let (w, h) = if touch { (390, 760) } else { (1280, 720) };
        let win = Window::open("musicbox", w, h, touch)?;
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
            knob: [None; NKNOBS],
            knob_drag: None,
            bar: None,
            span: 0.0,
            scrub: None,
            sid: None,
            edit: None,
            edit_song: false,
            eat: None,
            note: None,
            tips: Vec::new(),
            hover: None,
            quiet: None,
            keys: false,
            panel: None,
            panel_rect: None,
            fav_top: 0,
            compact: touch,
            tab: Tab::Music,
            finger: false,
            press: None,
            tipped: false,
        })
    }

    /// Whether a finger works the window.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn touch(&self) -> bool {
        self.win.touch()
    }

    pub fn fullscreen(&mut self) {
        self.win.toggle_fullscreen();
    }

    /// The list of the keys (`Keys`), the effects (`EffectsPanel`: in a
    /// hand, the selector's part for the sound) or the favorites
    /// (`FavoritesPanel`) shown, or put away.
    pub fn toggle(&mut self, a: Action) {
        let want = match a {
            Action::EffectsPanel if self.compact => {
                self.tab = if self.tab == Tab::Sound { Tab::Music } else { Tab::Sound };
                self.panel = None;
                return;
            }
            Action::EffectsPanel => Panel::Effects,
            Action::FavoritesPanel => Panel::Favorites,
            _ => {
                self.keys = !self.keys;
                return;
            }
        };
        self.panel = if self.panel == Some(want) { None } else { Some(want) };
        self.fav_top = 0;
    }

    /// What a button asks: done here when it is the window's own (the
    /// selector's part shown, the favorites scrolled), else handed on.
    fn fire(&mut self, a: Action, v: &mut Vec<Action>) {
        match a {
            Action::Tab(t) => {
                self.tab = Tab::ALL.get(t as usize).map_or(Tab::Music, |x| x.0);
                if self.compact {
                    self.panel = None;
                }
            }
            Action::Scroll(by) => self.fav_top = (self.fav_top as i32 + by).max(0) as usize,
            a => v.push(a),
        }
    }

    /// The mouse's button, or a finger, down at (x, y) in the picture: a
    /// slider or the song's bar taken hold of, a button pressed (a mouse's
    /// acts at once, a finger's when it lifts on it).
    fn pressed(&mut self, (x, y): (i32, i32), button: u8, finger: bool, v: &mut Vec<Action>) {
        self.mouse = Some((x, y));
        (self.finger, self.press, self.tipped) = (finger, None, false);
        if finger {
            // (a finger that stays is told what it is on, from now)
            (self.hover, self.quiet) = (None, None);
        } else {
            self.quiet = self.hover.map(|h| h.0);
        }
        // the keys listed: a click puts them away
        if self.keys {
            self.keys = false;
            return;
        }
        let left = button == sdl::SDL_BUTTON_LEFT;
        // a click anywhere else forgets a length half typed; one on a panel
        // is the panel's
        let on_panel = inside(self.panel_rect, (x, y));
        if !inside(self.field, (x, y)) || on_panel {
            self.edit = None;
        }
        let hit = self
            .buttons
            .iter()
            .rev()
            .find(|b| x >= b.x0 && x < b.x1 && y >= b.y0 && y < b.y1)
            .map(|b| ((b.x0, b.y0, b.x1, b.y1), b.left, b.right));
        let knob = self.knob_under((x, y)).filter(|_| left);
        if let Some(k) = knob.filter(|_| on_panel || !(inside(self.bar, (x, y)) || inside(self.slider, (x, y)))) {
            let to = self.knob_at(k, x);
            self.knob_drag = Some((k, to));
            v.push(knob_set(k, to));
        } else if on_panel {
            if let Some((r, a, _)) = hit.filter(|_| left) {
                self.press_or_fire(r, a, finger, v);
            }
        } else if left && inside(self.bar, (x, y)) {
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
        } else if left && inside(self.sid, (x, y)) {
            let r = self.sid.expect("the id's box");
            self.press_or_fire(r, Action::Copy, finger, v);
        } else if let Some((r, l, right)) = hit {
            match button {
                sdl::SDL_BUTTON_LEFT => self.press_or_fire(r, l, finger, v),
                sdl::SDL_BUTTON_RIGHT => v.extend(right),
                _ => {}
            }
        }
    }

    fn press_or_fire(&mut self, r: Rect, a: Action, finger: bool, v: &mut Vec<Action>) {
        if finger {
            self.press = Some((r, a));
        } else {
            self.fire(a, v);
        }
    }

    /// The mouse, or a finger, moved to (x, y): what is held follows it.
    fn moved(&mut self, (x, y): (i32, i32), v: &mut Vec<Action>) {
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
        if let Some((k, was)) = self.knob_drag {
            let to = self.knob_at(k, x);
            if to != was {
                self.knob_drag = Some((k, to));
                v.push(knob_set(k, to));
            }
        }
    }

    /// The button, or the finger, up: what was held let go (the song gone
    /// to where its bar was left); a finger's button done, if it lifts on
    /// it and was not just told what it does.
    fn released(&mut self, at: Option<(i32, i32)>, finger: bool, v: &mut Vec<Action>) {
        self.drag = None;
        self.knob_drag = None;
        v.extend(self.scrub.take().map(Action::SeekTo));
        if !finger {
            return;
        }
        if let Some((r, a)) = self.press.take()
            && at.is_some_and(|at| inside(Some(r), at))
            && !self.tipped
        {
            self.fire(a, v);
        }
        (self.mouse, self.hover, self.finger) = (None, None, false);
    }

    /// `text` onto the clipboard (whether it took it); what the clipboard holds.
    pub fn copy(&self, text: &str) -> bool {
        self.win.set_clipboard(text)
    }

    pub fn pasted(&self) -> String {
        self.win.clipboard()
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

    /// The value at `x` on the slider `k`: 0 to its most, in its steps.
    fn knob_at(&self, k: usize, x: i32) -> u32 {
        let Some((x0, _, x1, _)) = self.knob[k] else { return 0 };
        let (_, most, step, _, _) = knob(k);
        let f = ((x - x0) as f64 / (x1 - x0 - 1).max(1) as f64).clamp(0.0, 1.0);
        ((f * most as f64 / step as f64).round() as u32 * step).min(most)
    }

    /// Where a value sits on the slider `k`.
    fn knob_x(&self, k: usize, v: u32) -> i32 {
        let Some((x0, _, x1, _)) = self.knob[k] else { return 0 };
        let most = knob(k).1;
        x0 + (v.min(most) as f64 / most as f64 * (x1 - x0 - 1) as f64).round() as i32
    }

    /// The slider under the mouse at `at` (an effect's, over the volume's).
    fn knob_under(&self, at: (i32, i32)) -> Option<usize> {
        (0..NKNOBS).rev().find(|&k| inside(self.knob[k], at))
    }

    /// The point in the song at `x` on its bar (seconds).
    fn bar_secs(&self, x: i32) -> f64 {
        let Some((x0, _, x1, _)) = self.bar else { return 0.0 };
        ((x - x0) as f64 / (x1 - x0 - 1).max(1) as f64).clamp(0.0, 1.0) * self.span
    }

    /// A key while a length or a song's id is typed: Enter sets it (plays
    /// the song), Esc forgets it, Ctrl-V pastes.
    fn edit_key(&mut self, k: i32, ctrl: bool, v: &mut Vec<Action>) {
        let most = if self.edit_song { 48 } else { 12 };
        let pasted = if ctrl && k == 'v' as i32 { self.win.clipboard() } else { String::new() };
        let Some(text) = &mut self.edit else { return };
        match k {
            _ if !pasted.is_empty() => {
                text.extend(pasted.trim().chars().filter(|c| c.is_ascii_graphic()));
                text.truncate(if self.edit_song { 256 } else { most });
            }
            sdl::K_BACKSPACE => {
                text.pop();
            }
            sdl::K_RETURN | sdl::K_KP_ENTER => {
                let now = Instant::now();
                if self.edit_song {
                    match crate::favorites::song_in(text) {
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
                    let at = self.win.to_picture(x, y);
                    self.moved(at, &mut v);
                }
                sdl::SDL_MOUSEBUTTONUP => self.released(None, false, &mut v),
                sdl::SDL_MOUSEWHEEL => {
                    let over = self.mouse.is_some_and(|m| inside(self.slider, m) || inside(self.field, m));
                    if over && e.wheel() != 0 {
                        v.push(Action::Longer(e.wheel().signum()));
                    }
                    if let Some(k) = self.mouse.and_then(|m| self.knob_under(m))
                        && e.wheel() != 0
                    {
                        v.push(knob_by(k, e.wheel().signum()));
                    } else if self.panel == Some(Panel::Favorites)
                        && self.mouse.is_some_and(|m| inside(self.panel_rect, m))
                    {
                        self.fav_top = (self.fav_top as i32 - e.wheel()).max(0) as usize;
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
                    let at = self.win.to_picture(x, y);
                    self.pressed(at, e.button(), false, &mut v);
                }
                // a finger (the page's): as the mouse's left button, but a
                // button waits for it to lift
                sdl::TOUCH_DOWN => {
                    let (x, y) = e.mouse_xy();
                    let at = self.win.to_picture(x, y);
                    self.pressed(at, sdl::SDL_BUTTON_LEFT, true, &mut v);
                }
                sdl::TOUCH_MOVE => {
                    let (x, y) = e.mouse_xy();
                    let at = self.win.to_picture(x, y);
                    self.moved(at, &mut v);
                }
                sdl::TOUCH_UP => {
                    let (x, y) = e.mouse_xy();
                    let at = self.win.to_picture(x, y);
                    self.released(Some(at), true, &mut v);
                }
                sdl::SDL_KEYDOWN if self.edit.is_some() => self.edit_key(e.key(), e.ctrl(), &mut v),
                // the keys listed: a key puts them away (? and F1 too)
                sdl::SDL_KEYDOWN if self.keys => self.keys = false,
                sdl::SDL_KEYDOWN => {
                    let a = match e.key() {
                        sdl::K_RIGHT => Some(Action::Genre(1)),
                        sdl::K_LEFT => Some(Action::Genre(-1)),
                        sdl::K_UP => Some(Action::Drums(1)),
                        sdl::K_DOWN => Some(Action::Drums(-1)),
                        // (a panel first)
                        sdl::K_ESCAPE if self.panel.is_some() => {
                            self.panel = None;
                            None
                        }
                        sdl::K_ESCAPE => Some(Action::Window),
                        sdl::K_F1 => Some(Action::Keys),
                        sdl::K_F11 => Some(Action::Fullscreen),
                        sdl::K_KP_MINUS => Some(Action::Louder(-1)),
                        sdl::K_KP_PLUS => Some(Action::Louder(1)),
                        k => match u8::try_from(k).map(|c| c as char) {
                            Ok(' ' | '\r' | 'y') => Some(Action::Skip),
                            Ok('s') => Some(Action::Play),
                            Ok('r') => Some(Action::Replay),
                            Ok('b') => Some(Action::Back),
                            Ok(',') => Some(Action::Seek(-crate::SEEK_SECS)),
                            Ok('.') => Some(Action::Seek(crate::SEEK_SECS)),
                            Ok('x') => Some(Action::Export),
                            Ok('[') => Some(Action::Longer(-1)),
                            Ok(']') => Some(Action::Longer(1)),
                            Ok('-') => Some(Action::Louder(-1)),
                            Ok('=' | '+') => Some(Action::Louder(1)),
                            // (Ctrl-C and Ctrl-V as well)
                            Ok('c') => Some(Action::Copy),
                            Ok('v') => Some(Action::Paste),
                            Ok('k') => Some(Action::Keep),
                            Ok('a') => Some(Action::EffectsPanel),
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
                            // (? is shifted /)
                            Ok('/' | '?') => Some(Action::Keys),
                            Ok(c) => crate::fx_key(c),
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
        self.tips.clear();
        self.knob = [None; NKNOBS];
        (self.slider, self.field, self.bar, self.sid, self.panel_rect) = (None, None, None, None, None);
        self.compact = self.win.compact();
        if self.note.as_ref().is_some_and(|n| n.1.elapsed().as_secs() >= 4) {
            self.note = None;
        }
        if self.compact {
            self.draw_hand(tap, now, st);
        } else {
            self.draw_desk(tap, now, st);
        }
        // over it all the keys, or what the mouse (or a finger) rests on
        if self.keys {
            self.key_list();
        } else {
            self.show_tip();
        }
        self.win.present(&self.px);
    }

    /// "Click", or for a finger "Tap".
    fn click(&self) -> &'static str {
        if self.win.touch() { "Tap" } else { "Click" }
    }

    /// The line under the song's name, and its colour: a song's id being
    /// typed, a word on what went wrong, a note (an export, a copy), else
    /// the song: which, its meter, tempo, key and where in it.
    fn status(&self, now: &Now, st: &State) -> (String, u32) {
        let msg = st.msg.as_ref().or(self.note.as_ref());
        let typing = self.edit.as_ref().filter(|_| self.edit_song);
        let line = match msg {
            _ if typing.is_some() => {
                format!(
                    "SONG ID: {}_   AS SHOWN AT THE TOP, SKA.2817.0.3, OR CTRL-V: ENTER PLAYS IT, ESC CANCELS",
                    typing.expect("typed").to_uppercase()
                )
            }
            Some((msg, _)) => msg.to_uppercase(),
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
        (line, color)
    }

    /// The window at a desk (see the top of the file).
    fn draw_desk(&mut self, tap: &Tap, now: &Now, st: &State) {
        let (w, h) = (self.w, self.h);
        let m = 8;

        // what is playing, and its id (a click on it copies it)
        self.text(m, m, "MUSICBOX", ACCENT, 2);
        let mut right = now.sid.map_or_else(|| format!("SEED {}", st.seed), |id| format!("SONG {id}"));
        if !st.sampled {
            right += "  NO RECORDINGS";
        }
        let rw = font::width(&right, 1);
        self.text(w - m - rw, m + 4, &right, DIM, 1);
        let tag = "FOR WHEN QUANTITY TAKES PRIORITY OVER QUALITY";
        let tx = m + font::width("MUSICBOX", 2) + 14;
        if tx + font::width(tag, 1) + 24 <= w - m - rw {
            self.text(tx, m + 4, tag, DIM, 1);
        }
        self.sid = Some((w - m - rw - 2, m + 1, w - m + 2, m + 14));
        self.tip(
            self.sid,
            if WEB {
                "This song's id: it plays the same song again, here (PASTE) or in the desktop app (musicbox ID). Click it to copy it."
            } else {
                "This song's id: it plays the same song again, here (PASTE, or musicbox ID) or in the browser. Click it to copy it."
            },
            "C",
        );
        let playing = if now.bpm > 0.0 { now.id } else { st.id() };
        let (p, d) = music::parts(playing);
        let mut y = m + 22;
        // at the name's right, the id's buttons: copy it, play one pasted,
        // keep it, the favorites
        let kept = now.sid.is_some_and(|id| st.favorites.has(&id));
        let n = st.favorites.ids().len();
        let ids: [(&str, Action, String, &'static str, bool); 4] = [
            (
                "COPY",
                Action::Copy,
                if WEB {
                    "Copy this song's id, to play it again later: here (PASTE) or in the desktop app (musicbox ID)."
                } else {
                    "Copy this song's id, to play it again later: here (PASTE, or musicbox ID) or in the browser."
                }
                .into(),
                "C",
                false,
            ),
            (
                "PASTE",
                Action::Paste,
                "Play the song whose id is copied (or a link to the page playing it).".into(),
                "V",
                false,
            ),
            (
                "*",
                Action::Keep,
                if kept {
                    "One of your favorites. Click to let it go.".into()
                } else {
                    "Keep this song among your favorites.".into()
                },
                "K",
                kept,
            ),
            (
                "FAVORITES",
                Action::FavoritesPanel,
                format!("Your favorites ({n}), to play again: click one."),
                "",
                self.panel == Some(Panel::Favorites),
            ),
        ];
        let iw = ids.iter().map(|b| font::width(b.0, 1) + 10).sum::<i32>() - 2;
        let big = if font::width(&music::name(playing), 2) + iw + 24 <= w - 2 * m { 2 } else { 1 };
        let mut x = m;
        x += self.text(x, y, music::GENRES[p], TEXT, big);
        if let Some(d) = d {
            x += self.text(x, y, " + ", DIM, big);
            x += self.text(x, y, music::GENRES[d], AMBER, big);
            x += self.text(x, y, " DRUMS", DIM, big);
        }
        let mut bx = (w - m - iw).max(x + 12);
        let by = y + (7 * big - 13) / 2;
        for (label, a, help, key, lit) in ids {
            let bw = font::width(label, 1) + 8;
            self.button(bx, by, bw, 13, label, if lit { LIT } else { PLAIN }, None, a, None);
            self.tip(Some((bx, by, bx + bw, by + 13)), help, key);
            bx += bw + 2;
        }
        // the music asked for, from the next bar: at the right of the song's line
        let next = (playing != st.id()).then(|| format!("NEXT: {}", music::name(st.id())));
        let nw = next.as_ref().map_or(0, |n| font::width(n, 1));
        y += 7 * big + 6;
        let (line, color) = self.status(now, st);
        self.text(m, y, &line, color, 1);
        if let Some(next) = next {
            self.rect(w - m - nw - 12, y - 2, nw + 12, 11, BG);
            self.text(w - m - nw, y, &next, ACCENT, 1);
        }
        let y = y + 14;

        // where in the song it is: a bar to click or drag to a point in it,
        // between buttons 10 s back and on
        let bh = 13;
        let seek = crate::SEEK_SECS;
        let lw = font::width("<< 10 S", 1) + 8;
        self.button(m, y, lw, bh, "<< 10 S", PLAIN, None, Action::Seek(-seek), None);
        self.tip(Some((m, y, m + lw, y + bh)), "10 seconds back.", ",");
        let rw = font::width("10 S >>", 1) + 8;
        self.button(w - m - rw, y, rw, bh, "10 S >>", PLAIN, None, Action::Seek(seek), None);
        self.tip(Some((w - m - rw, y, w - m, y + bh)), "10 seconds on.", ".");
        self.span = if now.length > 0.0 { now.length } else { now.planned }.max(1.0);
        let at = self.scrub.unwrap_or(now.elapsed).min(self.span);
        let time = format!("{} / {}", clock(at), clock(self.span));
        let tw = font::width("00:00 / 00:00", 1);
        let (bx0, bx1) = (m + lw + 8, (w - m - rw - tw - 16).max(m + lw + 48));
        self.bar = Some((bx0, y, bx1, y + bh));
        self.tip(self.bar, "Where the song is: click or drag to go anywhere in it.", "");
        self.rect(bx0, y + 4, bx1 - bx0, 5, 0x30353F);
        let hx = bx0 + (at / self.span * (bx1 - bx0 - 1) as f64).round() as i32;
        self.rect(bx0, y + 4, hx - bx0, 5, if self.scrub.is_some() { AMBER } else { ACCENT });
        self.rect(hx - 1, y + 1, 3, bh - 2, TEXT);
        self.text(bx1 + 8, y + 3, &time, DIM, 1);
        self.tip(Some((bx1 + 8, y, bx1 + 8 + tw, y + bh)), "How far into the song, and how long it is.", "");
        let top = y + bh + 6;

        // the selector, from the bottom up
        let gap = 3;
        let bh = 13;
        let cols = ((w - 2 * m + gap) / (84 + gap)).clamp(3, 13);
        let rows = |n: i32| (n + cols - 1) / cols;
        let ng = music::GENRES.len() as i32;
        // (the settings' row in two when the picture is narrow)
        let made_lines = if made_width() > w - 2 * m { 2 } else { 1 };
        let height = 2 * (bh + 8) + (made_lines - 1) * (bh + 4) + 11 + rows(ng) * (bh + gap) + 4 + bh;
        let mut y = h - m - height;
        let bottom = y - 6;

        // the transport, and the songs' length
        let mut x = m;
        let keys: [(&str, Action, &str, &str); 4] = [
            ("BACK", Action::Back, "Back to this song's start; in its first 3 seconds, to the song before.", "B"),
            if now.stopped {
                ("START", Action::Play, "Play on from where it stopped.", "S")
            } else {
                ("STOP", Action::Play, "Stop the music, holding it where it is.", "S")
            },
            (
                "NEXT SONG",
                Action::Skip,
                "On to the next song: a new one, or after going back, the one you came from.",
                "SPACE",
            ),
            (
                "EXPORT MP3",
                Action::Export,
                "Save the whole song playing as an MP3 in ~/Music/musicbox, named by its id. It is made again from its start, so the music plays on.",
                "X",
            ),
        ];
        for (i, (label, a, help, key)) in keys.into_iter().enumerate() {
            // (the transport together, the export apart)
            if i == 3 {
                x += 8;
            }
            let bw = font::width(label, 1) + 8;
            self.button(x, y, bw, bh, label, PLAIN, None, a, None);
            self.tip(Some((x, y, x + bw, y + bh)), help, key);
            x += bw + if i < 2 { 2 } else { 6 };
        }
        x += 10;
        let lw = self.text(x, y + 3, "LENGTH", DIM, 1);
        self.tip(
            Some((x, y, x + lw, y + bh)),
            "How long each song is. The music is shaped to end near it, at its outro.",
            "",
        );
        x += lw + 6;
        let auto = st.length <= 0.0;
        let aw = font::width("AUTO", 1) + 8;
        self.button(x, y, aw, bh, "AUTO", if auto { LIT } else { PLAIN }, None, Action::SetLength(0.0), None);
        self.tip(Some((x, y, x + aw, y + bh)), "Each song as long as it was written: about two minutes.", "[ ]");
        x += aw + 8;
        let fw = font::width("00:00", 1) + 10;
        let (sx0, sx1) = (x, (w - m - fw - 8).max(x + 40));
        self.slider = Some((sx0, y, sx1, y + bh));
        self.tip(
            self.slider,
            if auto {
                "How long each song is to be, 0:30 to 10:00 (at auto, the hollow mark is this song's own length). The mouse wheel turns it too."
            } else {
                "How long each song is to be, 0:30 to 10:00. The mouse wheel turns it too."
            },
            "[ ]",
        );
        self.slider_track(st, now, sx0, sx1, y, bh);
        let fx = sx1 + 8;
        self.field = Some((fx, y, fx + fw, y + bh));
        self.tip(self.field, "Click it and type a length, then Enter: 3:30, 3.5m, 210s (up to an hour) or auto.", "T");
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
        y = self.made(st, m, w - m, y, bh);
        y += bh + 8;

        // the genres, under a switch for what a click on one sets: the
        // genre that writes the music, or the one on drums
        let on_drums = self.tab == Tab::Drums;
        let nw = music::GENRES.iter().map(|g| font::width(g, 1)).max().unwrap_or(0) + 10;
        let mut x = m;
        for (drums, label) in [(false, "MUSIC BY"), (true, "DRUMS BY")] {
            let lw = self.text(x, y, label, if drums == on_drums { TEXT } else { DIM }, 1);
            let name = music::GENRES[if drums { st.drums.unwrap_or(st.primary) } else { st.primary }];
            let (fill, mark) = match (drums, drums == on_drums) {
                (false, true) => (LIT, None),
                (true, true) => (DRUM, None),
                (false, false) => (PLAIN, Some(LIT.1)),
                (true, false) => (PLAIN, st.drums.map(|_| AMBER)),
            };
            let bx = x + lw + 6;
            self.button(bx, y - 2, nw, 11, name, fill, mark, Action::Tab(drums as u8), None);
            let help = match (drums, st.drums) {
                (false, _) => format!(
                    "{name} writes the music: its form, meter, chords and tunes. Click, then a genre below, to have another write it."
                ),
                (true, Some(_)) => format!(
                    "{name} is on drums: its beat, fitted to the music's meter, and its kits and effects. Click, then a genre below, for another drummer ({} itself: its own).",
                    music::GENRES[st.primary]
                ),
                (true, None) => format!(
                    "{name} plays its own drums. Click, then a genre below, to put another genre on drums: its beat, fitted to the music's meter, and its kits and effects."
                ),
            };
            self.tip(Some((x, y - 2, bx + nw, y + 9)), help, if drums { "UP DOWN" } else { "LEFT RIGHT" });
            x = bx + nw + 16;
        }
        let say =
            if on_drums { "A CLICK ON A GENRE PUTS IT ON DRUMS" } else { "A CLICK ON A GENRE HAS IT WRITE THE MUSIC" };
        if x + font::width(say, 1) <= w - m {
            self.text(w - m - font::width(say, 1), y, say, FAINT, 1);
        }
        y += 11;
        self.grid(st, (m, y, w - m, y + rows(ng) * (bh + gap)), cols, bh + gap, gap, on_drums);
        y += rows(ng) * (bh + gap) + 4;

        // the window's own buttons, the effects', and the volume
        let mut x = m;
        let keys = [
            ("FULLSCREEN", Action::Fullscreen, "Fill the screen, or go back to a window.", "F F11"),
            (
                "CLOSE",
                Action::Window,
                "Close the window: the music plays on in the terminal (without one, it ends).",
                "ESC G",
            ),
            ("QUIT", Action::Quit, "Stop the music and quit.", "Q"),
            ("KEYS", Action::Keys, "Every key, in one list.", "? F1"),
            (
                "EFFECTS",
                Action::EffectsPanel,
                "Reverb, echo, chorus, stereo width, bass and treble, 0 to 500% (100: as the genre has them). They change what you hear and export, not the music. Underlined when one is turned.",
                "A",
            ),
        ];
        let shown = keys.into_iter().filter(|k| !WEB || !matches!(k.1, Action::Window | Action::Quit));
        for (label, a, help, key) in shown {
            let bw = font::width(label, 1) + 8;
            let fx = a == Action::EffectsPanel;
            let (fill, mark) = if fx {
                let turned = st.effects != music::Effects::default();
                (if self.panel == Some(Panel::Effects) { LIT } else { PLAIN }, turned.then_some(ACCENT))
            } else {
                (PLAIN, None)
            };
            if fx {
                x += 8;
            }
            self.button(x, y, bw, bh, label, fill, mark, a, None);
            self.tip(Some((x, y, x + bw, y + bh)), help, key);
            x += bw + 6;
        }
        self.knob_one(0, st.volume, x + 8, w - m, y, bh, 0);

        // the equalizer, or a panel in its place (over the middle when the
        // place is too small for it)
        match self.panel {
            Some(p) => {
                let need = if p == Panel::Effects { EFFECTS_H } else { FAVORITES_H };
                // (the effects over the top of the equalizer's place, the
                // equalizer still playing under them where there is room)
                let r = if p == Panel::Effects && bottom - top >= need + 40 {
                    self.equalizer(tap, m, top + need + 4, w - m, bottom);
                    (m, top, w - m, top + need)
                } else if bottom - top >= need {
                    (m, top, w - m, bottom)
                } else {
                    let y0 = ((h - need) / 2).max(2);
                    (m, y0, w - m, (y0 + need).min(h - 2))
                };
                self.panel_rect = Some(r);
                if p == Panel::Effects {
                    self.effects_panel(st, r);
                } else {
                    self.favorites_panel(st, now, r, 13);
                }
            }
            None if bottom - top > 24 => self.equalizer(tap, m, top, w - m, bottom),
            None => {}
        }
    }

    /// The window in a hand (see the top of the file): what plays above the
    /// selector, or on a screen on its side at its left.
    fn draw_hand(&mut self, tap: &Tap, now: &Now, st: &State) {
        let (w, h) = (self.w, self.h);
        let m = 6;
        let ng = music::GENRES.len() as i32;
        // (the effects are the selector's SOUND here)
        if self.panel == Some(Panel::Effects) {
            (self.panel, self.tab) = (None, Tab::Sound);
        }
        let side = h < sdl::HAND_H && w >= 2 * sdl::HAND_W;
        let sel_w = if side { (w - 3 * m) * 11 / 20 } else { w - 2 * m };
        let cols = ((sel_w + 2) / 60).clamp(3, 7);
        let rows = (ng + cols - 1) / cols;
        // what plays needs this much without its equalizer (and the gap
        // under it); the grid's rows a finger's height where there is room
        // for them, less on a short screen; the selector's other parts need
        // 150 at least
        const PLAYS: i32 = 125;
        let room = if side { h - 2 * m - ROW } else { h - 2 * m - PLAYS - ROW };
        let pitch = (room / rows).clamp(16, ROW);
        let body = if side { room } else { (rows * pitch).max(150.min(room)) };
        let (plays, sel) = if side {
            ((m, m, w - 2 * m - sel_w, h - m), (w - m - sel_w, m, w - m, h - m))
        } else {
            let top = h - m - ROW - body;
            ((m, m, w - m, top - 4), (m, top, w - m, h - m))
        };
        self.playing(tap, now, st, plays);
        self.selector(now, st, sel, cols, pitch);
    }

    /// What plays, in `r`: the name, a star to keep the song, the genre and
    /// its drummer, the song's line (two lines of it), its bar to drag, and
    /// at the foot the transport, with the equalizer between where there is
    /// room for one.
    fn playing(&mut self, tap: &Tap, now: &Now, st: &State, r: Rect) {
        let (x0, y0, x1, y1) = r;
        let click = self.click();
        self.text(x0, y0, "MUSICBOX", ACCENT, 2);
        let kept = now.sid.is_some_and(|id| st.favorites.has(&id));
        let (sw, sh) = (30, 18);
        self.button(x1 - sw, y0 - 2, sw, sh, "*", if kept { LIT } else { PLAIN }, None, Action::Keep, None);
        let help = if kept {
            format!("One of your favorites. {click} to let it go.")
        } else {
            "Keep this song among your favorites: SONG lists them, to play again.".into()
        };
        self.tip(Some((x1 - sw, y0 - 2, x1, y0 - 2 + sh)), help, "K");
        let mut y = y0 + 19;
        let playing = if now.bpm > 0.0 { now.id } else { st.id() };
        let (p, d) = music::parts(playing);
        let name = music::GENRES[p];
        let big = if font::width(name, 2) <= x1 - x0 { 2 } else { 1 };
        self.text(x0, y + (2 - big) * 3, name, TEXT, big);
        y += 17;
        // its drummer; or the music asked for, from the next bar
        if playing != st.id() {
            self.text(x0, y, &format!("NEXT: {}", music::name(st.id())), ACCENT, 1);
        } else if let Some(d) = d {
            let lw = self.text(x0, y, "WITH ", DIM, 1);
            let nw = self.text(x0 + lw, y, music::GENRES[d], AMBER, 1);
            self.text(x0 + lw + nw, y, " ON DRUMS", DIM, 1);
        } else {
            self.text(x0, y, "ON ITS OWN DRUMS", FAINT, 1);
        }
        self.tip(
            Some((x0, y0 + 19, x1, y + 9)),
            "The genre that writes the music, and the one on drums: MUSIC and DRUMS below choose them.",
            "",
        );
        y += 12;
        let (line, color) = self.status(now, st);
        let cols = ((x1 - x0) / ADVANCE).max(10) as usize;
        for (i, l) in lines_of(&line, cols).iter().take(2).enumerate() {
            self.text(x0, y + i as i32 * 10, l, color, 1);
        }
        y += 23;
        // where in the song it is: a bar to drag to a point in it
        let bh = 20;
        self.span = if now.length > 0.0 { now.length } else { now.planned }.max(1.0);
        let at = self.scrub.unwrap_or(now.elapsed).min(self.span);
        let time = format!("{} / {}", clock(at), clock(self.span));
        let tw = font::width("00:00 / 00:00", 1);
        let (bx0, bx1) = (x0, x1 - tw - 8);
        self.bar = Some((bx0, y, bx1, y + bh));
        self.tip(self.bar, "Where the song is: drag to go anywhere in it.", ", .");
        self.rect(bx0, y + 7, bx1 - bx0, 6, 0x30353F);
        let hx = bx0 + (at / self.span * (bx1 - bx0 - 1) as f64).round() as i32;
        self.rect(bx0, y + 7, hx - bx0, 6, if self.scrub.is_some() { AMBER } else { ACCENT });
        self.rect(hx - 2, y + 2, 5, bh - 4, TEXT);
        self.text(x1 - font::width(&time, 1), y + 7, &time, DIM, 1);
        self.tip(Some((bx1 + 8, y, x1, y + bh)), "How far into the song, and how long it is.", "");
        y += bh + 4;
        // the transport at the foot, the equalizer between where there is room
        let th = 26;
        let ty = y1 - th;
        if ty - 4 - y >= 22 {
            self.equalizer(tap, x0, y, x1, ty - 4);
        }
        let keys: [(&str, Action, &str, &str); 3] = [
            ("BACK", Action::Back, "Back to this song's start; in its first 3 seconds, to the song before.", "B"),
            if now.stopped {
                ("START", Action::Play, "Play on from where it stopped.", "S")
            } else {
                ("STOP", Action::Play, "Stop the music, holding it where it is.", "S")
            },
            (
                "NEXT SONG",
                Action::Skip,
                "On to the next song: a new one, or after going back, the one you came from.",
                "SPACE",
            ),
        ];
        for ((label, a, help, key), (bx, bw)) in keys.into_iter().zip(across(x0, x1, 3, 2)) {
            self.button(bx, ty, bw, th, label, PLAIN, None, a, None);
            self.tip(Some((bx, ty, bx + bw, ty + th)), help, key);
        }
    }

    /// The selector in a hand, in `r`: its tabs, and under them the part
    /// chosen (the favorites over it when they are shown); the genres' grid
    /// `cols` wide, a row every `pitch`.
    fn selector(&mut self, now: &Now, st: &State, r: Rect, cols: i32, pitch: i32) {
        let (x0, y0, x1, y1) = r;
        let turned = st.effects != music::Effects::default();
        // (under the tabs a line in the chosen one's colour: its part is below)
        let line = match (self.panel, self.tab) {
            (Some(_), _) => EDGE,
            (None, Tab::Drums) => DRUM.0,
            (None, _) => LIT.0,
        };
        self.rect(x0, y0 + ROW - 3, x1 - x0, 2, line);
        for (i, ((tab, label, help), (bx, tw))) in Tab::ALL.into_iter().zip(across(x0, x1, 5, 2)).enumerate() {
            let on = self.tab == tab && self.panel.is_none();
            let fill = match (on, tab) {
                (true, Tab::Drums) => DRUM,
                (true, _) => LIT,
                _ => PLAIN,
            };
            let mark = match tab {
                Tab::Drums if !on => st.drums.map(|_| AMBER),
                Tab::Sound => turned.then_some(ACCENT),
                _ => None,
            };
            self.button(bx, y0, tw, ROW - 4, label, fill, mark, Action::Tab(i as u8), None);
            self.tip(Some((bx, y0, bx + tw, y0 + ROW - 2)), help, "");
        }
        let body = (x0, y0 + ROW, x1, y1);
        if self.panel == Some(Panel::Favorites) {
            self.panel_rect = Some(body);
            self.favorites_panel(st, now, body, ROW - 4);
            return;
        }
        match self.tab {
            Tab::Music | Tab::Drums => self.grid(st, body, cols, pitch, 2, self.tab == Tab::Drums),
            Tab::Sound => self.sound_tab(st, body),
            Tab::Song => self.song_tab(now, st, body),
            Tab::More => self.more_tab(st, body),
        }
    }

    /// The sound, in `r`: the volume and the effects, a slider each, and a
    /// button that sets the effects as the genre has them.
    fn sound_tab(&mut self, st: &State, r: Rect) {
        let (x0, y0, x1, y1) = r;
        let pitch = ((y1 - y0) / (NKNOBS as i32 + 1)).clamp(16, ROW);
        let lw = (0..NKNOBS).map(|k| font::width(knob(k).0, 1)).max().unwrap_or(0) + 8;
        for k in 0..NKNOBS {
            let v = if k == 0 { st.volume } else { st.effects.0[k - 1] };
            self.knob_one(k, v, x0, x1, y0 + k as i32 * pitch, pitch - 2, lw);
        }
        let y = y0 + NKNOBS as i32 * pitch;
        let label = "ALL EFFECTS 100%";
        let aw = font::width(label, 1) + 12;
        let turned = st.effects != music::Effects::default();
        self.button(
            x1 - aw,
            y,
            aw,
            pitch - 2,
            label,
            if turned { LIT } else { PLAIN },
            None,
            Action::ResetEffects,
            None,
        );
        self.tip(
            Some((x1 - aw, y, x1, y + pitch - 2)),
            "Every effect back to 100%: as the genre has it. The effects change what you hear and export, not the music.",
            "",
        );
    }

    /// The song, in `r`: its id (a tap copies it), buttons to copy it, play
    /// one copied and keep it, the favorites and the export, and how long
    /// songs are: as written, a step shorter or longer, or on a slider.
    fn song_tab(&mut self, now: &Now, st: &State, r: Rect) {
        let (x0, y0, x1, y1) = r;
        let click = self.click();
        let pitch = ((y1 - y0 - 24) / 4).clamp(18, ROW);
        let bh = pitch - 2;
        let id = now.sid.map_or_else(|| format!("SEED {}", st.seed), |id| id.to_string().to_uppercase());
        let cols = ((x1 - x0) / ADVANCE).max(8) as usize;
        for (i, l) in id.as_bytes().chunks(cols).take(2).enumerate() {
            self.text(x0, y0 + 2 + i as i32 * 10, &String::from_utf8_lossy(l), TEXT, 1);
        }
        self.sid = Some((x0, y0, x1, y0 + 22));
        let help = if WEB {
            format!(
                "This song's id: it plays the same song again, here (PASTE) or in the desktop app. {click} it to copy it."
            )
        } else {
            format!(
                "This song's id: it plays the same song again, here (PASTE, or musicbox ID) or in the browser. {click} it to copy it."
            )
        };
        self.tip(self.sid, help, "C");
        let mut y = y0 + 24;
        let kept = now.sid.is_some_and(|id| st.favorites.has(&id));
        let n = st.favorites.ids().len();
        let favorites = format!("FAVORITES {n}");
        let rows: [Vec<(&str, Action, String, &'static str, bool)>; 2] = [
            vec![
                ("COPY", Action::Copy, "Copy this song's id, to play it again later.".into(), "C", false),
                (
                    "PASTE",
                    Action::Paste,
                    "Play the song whose id is copied (or a link to the page playing it).".into(),
                    "V",
                    false,
                ),
                (
                    if kept { "KEPT *" } else { "KEEP *" },
                    Action::Keep,
                    if kept {
                        format!("One of your favorites. {click} to let it go.")
                    } else {
                        "Keep this song among your favorites.".into()
                    },
                    "K",
                    kept,
                ),
            ],
            vec![
                (&favorites, Action::FavoritesPanel, format!("Your favorites ({n}), to play again."), "", false),
                (
                    "EXPORT MP3",
                    Action::Export,
                    if WEB {
                        "Save the whole song playing as an MP3, named by its id. It is made again from its start, so the music plays on.".into()
                    } else {
                        "Save the whole song playing as an MP3 in ~/Music/musicbox, named by its id. It is made again from its start, so the music plays on.".into()
                    },
                    "X",
                    false,
                ),
            ],
        ];
        for row in rows {
            let n = row.len() as i32;
            for ((label, a, help, key, lit), (bx, bw)) in row.into_iter().zip(across(x0, x1, n, 2)) {
                self.button(bx, y, bw, bh, label, if lit { LIT } else { PLAIN }, None, a, None);
                self.tip(Some((bx, y, bx + bw, y + bh)), help, key);
            }
            y += pitch;
        }
        // how long songs are
        let auto = st.length <= 0.0;
        let lw = self.text(x0, y + (bh - 7) / 2, "LENGTH", DIM, 1);
        self.tip(
            Some((x0, y, x0 + lw, y + bh)),
            "How long each song is. The music is shaped to end near it, at its outro.",
            "",
        );
        let aw = font::width("AUTO", 1) + 12;
        let mut x = x0 + lw + 6;
        self.button(x, y, aw, bh, "AUTO", if auto { LIT } else { PLAIN }, None, Action::SetLength(0.0), None);
        self.tip(Some((x, y, x + aw, y + bh)), "Each song as long as it was written: about two minutes.", "[ ]");
        x += aw + 6;
        let sw = ((x1 - x - 44) / 2).clamp(20, 34);
        self.button(x, y, sw, bh, "-", PLAIN, None, Action::Longer(-1), None);
        self.tip(Some((x, y, x + sw, y + bh)), "Songs 15 seconds shorter.", "[");
        let shown = if auto { clock(now.planned) } else { show_length(st.length).to_uppercase() };
        let vx = x + sw;
        let vw = x1 - sw - vx;
        self.text(vx + (vw - font::width(&shown, 1)) / 2, y + (bh - 7) / 2, &shown, if auto { DIM } else { ACCENT }, 1);
        self.button(x1 - sw, y, sw, bh, "+", PLAIN, None, Action::Longer(1), None);
        self.tip(Some((x1 - sw, y, x1, y + bh)), "Songs 15 seconds longer.", "]");
        y += pitch;
        let (sx0, sx1) = (x0 + 3, x1 - 3);
        self.slider = Some((sx0, y, sx1, y + bh));
        self.tip(
            self.slider,
            if auto {
                "How long each song is to be, 0:30 to 10:00 (at auto, the hollow mark is this song's own length)."
            } else {
                "How long each song is to be, 0:30 to 10:00."
            },
            "[ ]",
        );
        self.slider_track(st, now, sx0, sx1, y, bh);
    }

    /// How the music is made, in `r`, and the window's own buttons; for a
    /// finger, a word on how to be told what things do.
    fn more_tab(&mut self, st: &State, r: Rect) {
        let (x0, y0, x1, y1) = r;
        let touch = self.win.touch();
        let bh = ((y1 - y0 - 69) / 4).clamp(14, ROW - 2);
        let mut y = self.made_stacked(st, (x0, y0 + 2, x1, y1), bh);
        let mut own: Vec<(&str, Action, &str, &'static str)> = Vec::new();
        if self.win.can_fullscreen() {
            own.push(("FULLSCREEN", Action::Fullscreen, "Fill the screen, or go back.", "F F11"));
        }
        if !touch {
            own.push(("KEYS", Action::Keys, "Every key, in one list.", "? F1"));
        }
        if !WEB {
            own.push((
                "CLOSE",
                Action::Window,
                "Close the window: the music plays on in the terminal (without one, it ends).",
                "ESC G",
            ));
            own.push(("QUIT", Action::Quit, "Stop the music and quit.", "Q"));
        }
        let n = own.len() as i32;
        if n > 0 {
            for ((label, a, help, key), (bx, bw)) in own.into_iter().zip(across(x0, x1, n.max(2), 2)) {
                self.button(bx, y, bw, bh, label, PLAIN, None, a, None);
                self.tip(Some((bx, y, bx + bw, y + bh)), help, key);
            }
            y += bh + 5;
        }
        let say = if touch {
            "HOLD A FINGER ON ANYTHING: IT SAYS WHAT IT DOES"
        } else {
            "REST THE MOUSE ON ANYTHING: IT SAYS WHAT IT DOES"
        };
        for (i, l) in wrap(say, ((x1 - x0) / ADVANCE).max(10) as usize).iter().enumerate() {
            if y + i as i32 * 10 + 7 <= y1 {
                self.text(x0, y + i as i32 * 10, l, FAINT, 1);
            }
        }
    }

    /// The genres, a grid of `cols` in `r`, a row every `pitch` with `gap`
    /// between its buttons: a click on one has it write the music, or
    /// (`drums`) puts it on drums; a right-click puts it on drums either
    /// way. Lit: the genre writing the music, or (`drums`) the drummer, the
    /// other underlined.
    fn grid(&mut self, st: &State, r: Rect, cols: i32, pitch: i32, gap: i32, drums: bool) {
        let (x0, y0, x1, _) = r;
        let xs: Vec<(i32, i32)> = across(x0, x1, cols, gap).collect();
        let bh = pitch - gap;
        let drummer = st.drums.unwrap_or(st.primary);
        let click = self.click();
        for g in 0..music::GENRES.len() {
            let ((bx, bw), by) = (xs[g % cols as usize], y0 + (g as i32 / cols) * pitch);
            let name = music::GENRES[g];
            let (fill, mark, left) = if drums {
                (
                    if g == drummer { DRUM } else { PLAIN },
                    (g == st.primary && g != drummer).then_some(LIT.1),
                    Action::SetDrums(Some(g)),
                )
            } else {
                (if g == st.primary { LIT } else { PLAIN }, (Some(g) == st.drums).then_some(AMBER), Action::SetGenre(g))
            };
            self.button(bx, by, bw, bh, name, fill, mark, left, Some(Action::SetDrums(Some(g))));
            let pname = music::GENRES[st.primary];
            let help = match (drums, g == st.primary, g == drummer) {
                (false, true, _) => format!("{name} writes the music."),
                (false, false, true) => format!("{name} is on drums (underlined). {click}: {name} writes the music."),
                (false, false, false) => format!("{click}: {name} writes the music."),
                (true, true, true) => format!("{name} plays its own drums."),
                (true, true, false) => {
                    format!("{name} writes the music (underlined). {click}: it plays its own drums.")
                }
                (true, false, true) => format!("{name} is on drums, for {pname}'s music."),
                (true, false, false) => format!("{click}: {name} on drums, for {pname}'s music."),
            };
            self.tip(Some((bx, by, bx + bw, by + bh)), help, if drums { "UP DOWN" } else { "LEFT RIGHT" });
        }
    }

    /// What `r` does (`help`) and its keys, for when the mouse rests on it.
    fn tip(&mut self, r: Option<Rect>, help: impl Into<String>, keys: &'static str) {
        if let Some(r) = r {
            self.tips.push(Tip { r, help: help.into(), keys });
        }
    }

    /// What the mouse rests on: what it does and its keys, in a box by it,
    /// once it has rested there a moment (not while something is held, nor
    /// after a click on it until the mouse moves on).
    fn show_tip(&mut self) {
        let Some(at) = self.mouse else {
            self.hover = None;
            return;
        };
        let Some(t) = self.tips.iter().rev().find(|t| inside(Some(t.r), at)) else {
            (self.hover, self.quiet) = (None, None);
            return;
        };
        let (r, help, keys) = (t.r, t.help.to_uppercase(), t.keys);
        let since = match self.hover {
            Some((was, since)) if was == r => since,
            _ => {
                self.quiet = None;
                self.hover = Some((r, Instant::now()));
                Instant::now()
            }
        };
        let held = self.drag.is_some() || self.knob_drag.is_some() || self.scrub.is_some();
        let delay = if self.finger { TIP_DELAY_FINGER } else { TIP_DELAY };
        if held || self.quiet == Some(r) || since.elapsed() < delay {
            return;
        }
        let (w, h) = (self.w, self.h);
        let cols = ((w - 20) / ADVANCE).clamp(10, 46) as usize;
        let lines = wrap(&help, cols);
        // (no keys for a finger)
        let keys = if self.win.touch() { "" } else { keys };
        let key_line =
            (!keys.is_empty()).then(|| format!("{} {keys}", if keys.contains(' ') { "KEYS" } else { "KEY" }));
        let longest = lines.iter().chain(key_line.iter()).map(|l| font::width(l, 1)).max().unwrap_or(0);
        let (bw, bh) = (longest + 12, (lines.len() + key_line.iter().len()) as i32 * 10 + 7);
        let x = (at.0 - 12).clamp(2, (w - bw - 2).max(2));
        let (below, above) = (r.3 + 4, r.1 - 4 - bh);
        // (a finger hides what is under it: above, where there is room)
        let y = if self.finger && above >= 2 {
            above
        } else if below + bh <= h - 2 {
            below
        } else {
            above.max(2)
        };
        self.tipped = true;
        self.rect(x, y, bw, bh, EDGE);
        self.rect(x + 1, y + 1, bw - 2, bh - 2, TIP);
        let mut ty = y + 5;
        for l in &lines {
            self.text(x + 6, ty, l, TEXT, 1);
            ty += 10;
        }
        if let Some(k) = key_line {
            let lw = self.text(x + 6, ty, if keys.contains(' ') { "KEYS " } else { "KEY " }, DIM, 1);
            self.text(x + 6 + lw, ty, &k[k.find(' ').map_or(0, |i| i + 1)..], ACCENT, 1);
        }
    }

    /// All the keys, in a box in the middle: two columns where they fit.
    fn key_list(&mut self) {
        let (w, h) = (self.w, self.h);
        let listed = keys_listed();
        let kw = listed.iter().map(|k| font::width(k.0, 1)).max().unwrap_or(0) + 10;
        let dw = listed.iter().map(|k| font::width(k.1, 1)).max().unwrap_or(0);
        let two = 2 * (kw + dw) + 24 + 24 <= w;
        let per = if two { listed.len().div_ceil(2) } else { listed.len() };
        let bw = if two { 2 * (kw + dw) + 24 } else { kw + dw } + 24;
        let bh = per as i32 * 10 + 44;
        let (x, y) = ((w - bw) / 2, ((h - bh) / 2).max(2));
        self.rect(x, y, bw, bh, EDGE);
        self.rect(x + 1, y + 1, bw - 2, bh - 2, TIP);
        self.text(x + 12, y + 10, "KEYS", ACCENT, 1);
        for (i, (key, what)) in listed.iter().enumerate() {
            let (col, row) = (i / per, i % per);
            let cx = x + 12 + col as i32 * (kw + dw + 24);
            let cy = y + 26 + row as i32 * 10;
            self.text(cx + kw - 10 - font::width(key, 1), cy, key, TEXT, 1);
            self.text(cx + kw, cy, what, DIM, 1);
        }
        self.text(x + 12, y + bh - 13, "ANY KEY OR CLICK PUTS THIS AWAY", FAINT, 1);
    }

    /// How the music is made, a row of three choices, each a name and its
    /// options side by side, the one chosen lit: the settings as written
    /// or tuned, the tunes by the rules or the network, the instruments. An
    /// option underlined does something for the music playing (the
    /// optimiser tuned it, a network writes its tunes, network instruments
    /// are in).
    fn made(&mut self, st: &State, x0: i32, x1: i32, mut y: i32, bh: i32) -> i32 {
        let o = st.opts;
        let mut x = x0;
        for (label, help, key, opts) in made_groups(st) {
            // a group that would run past the right edge starts a line of its own
            let gw = font::width(label, 1) + 6 + opts.iter().map(|o| font::width(o.0, 1) + 10).sum::<i32>();
            if x > x0 && x + gw > x1 {
                (x, y) = (x0, y + bh + 4);
            }
            let lw = self.text(x, y + 3, label, DIM, 1);
            self.tip(Some((x, y, x + lw, y + bh)), help, "");
            x += lw + 6;
            for (name, to, real, help) in opts {
                let bw = font::width(name, 1) + 8;
                self.button(
                    x,
                    y,
                    bw,
                    bh,
                    name,
                    if to == o { LIT } else { PLAIN },
                    real.then_some(ACCENT),
                    Action::SetOpts(to),
                    None,
                );
                self.tip(Some((x, y, x + bw, y + bh)), help, key);
                x += bw + 2;
            }
            x += 14;
        }
        y
    }

    /// The same in a hand: each choice's name on a line, its options
    /// across the width under it, in `r`.
    fn made_stacked(&mut self, st: &State, r: Rect, bh: i32) -> i32 {
        let (x0, mut y, x1, _) = r;
        let o = st.opts;
        for (label, help, key, opts) in made_groups(st) {
            let lw = self.text(x0, y, label, DIM, 1);
            self.tip(Some((x0, y - 2, x0 + lw, y + 9)), help, "");
            y += 10;
            let n = opts.len() as i32;
            for ((name, to, real, help), (bx, bw)) in opts.into_iter().zip(across(x0, x1, n, 2)) {
                let fill = if to == o { LIT } else { PLAIN };
                self.button(bx, y, bw, bh, name, fill, real.then_some(ACCENT), Action::SetOpts(to), None);
                self.tip(Some((bx, y, bx + bw, y + bh)), help, key);
            }
            y += bh + 5;
        }
        y
    }
}

/// How the music is made, as three choices: each its name, what it is, its
/// key, and its options.
fn made_groups(st: &State) -> [(&'static str, &'static str, &'static str, Vec<Choice>); 3] {
    use music::Voices;
    let (o, id) = (st.opts, st.id());
    let g = music::GENRES[music::parts(id).0];
    let tuned = music::has_tuning(id);
    let net_tunes = music::has_net_tunes(id);
    let nets = crate::net::count();
    let tuning = format!(
        "The mix as an optimiser tuned it against real recordings, for six genres: ambient, techno, lo-fi, ska, gamelan and starconflict. {}",
        if tuned { format!("{g} is one of them.") } else { format!("{g} is not one yet: it plays as written.") }
    );
    let tunes = format!(
        "Celtic tunes by a small network that learned O'Neill's Music of Ireland (1903). {}",
        match (net_tunes, music::parts(id).0 == music::CELTIC) {
            (true, _) => "It writes the tunes playing.".to_string(),
            (false, true) => "Its network is not installed: the rules write.".to_string(),
            (false, false) => format!("Celtic has one alone: {g} keeps to the rules."),
        }
    );
    let groups: [(&str, &str, &str, Vec<Choice>); 3] = [
            (
                "TUNING",
                "Each genre's mix: the level of each part, the tone, reverb, tempo, swing and stereo width. A change plays this song again from its start.",
                "O",
                vec![
                    ("AS WRITTEN", music::Opts { tuned: false, ..o }, false, "Each genre's mix as it was written by hand.".into()),
                    ("OPTIMISED", music::Opts { tuned: true, ..o }, tuned, tuning),
                ],
            ),
            (
                "TUNES",
                "What writes the melodies. A change plays this song again from its start.",
                "M",
                vec![
                    (
                        "RULES",
                        music::Opts { net_tunes: false, ..o },
                        false,
                        "Tunes written by the composer's rules, as every genre has them.".into(),
                    ),
                    ("NETWORK", music::Opts { net_tunes: true, ..o }, net_tunes, tunes),
                ],
            ),
            (
                "INSTRUMENTS",
                "What plays the notes. A change plays this song again from its start.",
                "I",
                Voices::ALL
                    .iter()
                    .map(|&v| {
                        let help = match v {
                            Voices::Synth => "A small synthesiser plays the notes.".to_string(),
                            Voices::Recorded if st.sampled => "Recordings of real instruments play the notes.".into(),
                            Voices::Recorded => {
                                "Recordings of real instruments play the notes. None are installed: the synthesiser plays.".into()
                            }
                            Voices::Net if nets > 0 => {
                                format!("{nets} instruments played by small networks that learned their recordings, the rest recorded.")
                            }
                            Voices::Net => "Instruments played by small networks that learned their recordings. None are installed: recordings play.".into(),
                        };
                        let name = ["SYNTH", "RECORDED", "NETWORK"][v as usize];
                        (name, music::Opts { voices: v, ..o }, v == Voices::Net && nets > 0, help)
                    })
                    .collect(),
            ),
        ];
    groups
}

impl Gui {
    /// The effects, in a box in `r`: their sliders in two columns, and
    /// buttons to set them all as the genre has them and to put them away.
    fn effects_panel(&mut self, st: &State, r: Rect) {
        let (x0, y0, x1, y1) = r;
        self.rect(x0, y0, x1 - x0, y1 - y0, EDGE);
        self.rect(x0 + 1, y0 + 1, x1 - x0 - 2, y1 - y0 - 2, PANEL);
        self.tip(
            Some(r),
            "The effects change what you hear and export, not the music: 100% is each as the genre has it.",
            "A",
        );
        let (ix0, ix1) = (x0 + 8, x1 - 8);
        let y = y0 + 7;
        let tw = self.text(ix0, y + 3, "EFFECTS", ACCENT, 1);
        let cw = font::width("CLOSE", 1) + 8;
        let aw = font::width("ALL 100%", 1) + 8;
        let say = "100% IS AS THE GENRE HAS IT";
        if ix0 + tw + 12 + font::width(say, 1) + 12 <= ix1 - cw - aw - 6 {
            self.text(ix0 + tw + 12, y + 3, say, FAINT, 1);
        }
        self.button(ix1 - cw, y, cw, 13, "CLOSE", PLAIN, None, Action::EffectsPanel, None);
        self.tip(Some((ix1 - cw, y, ix1, y + 13)), "Put the effects away (they stay as they are).", "A ESC");
        let rx = ix1 - cw - 4 - aw;
        let fill = if st.effects == music::Effects::default() { PLAIN } else { LIT };
        self.button(rx, y, aw, 13, "ALL 100%", fill, None, Action::ResetEffects, None);
        self.tip(Some((rx, y, rx + aw, y + 13)), "Every effect back to 100%: as the genre has it.", "");
        let y = y + 13 + 8;
        let gap = 24;
        let cw = (ix1 - ix0 - gap) / 2;
        let lw = FX_LABELS.iter().map(|l| font::width(l, 1)).max().unwrap_or(0) + 8;
        for k in 0..music::NFX {
            let (col, row) = (k as i32 / 3, k as i32 % 3);
            let cx = ix0 + col * (cw + gap);
            self.knob_one(1 + k, st.effects.0[k], cx, cx + cw, y + row * 17, 13, lw);
        }
    }

    /// The favorites, in a box in `r`: each a row to click to play it (the
    /// one playing lit) with an X to let it go, as many as fit from
    /// `fav_top` (the wheel scrolls), and where they are kept.
    fn favorites_panel(&mut self, st: &State, now: &Now, r: Rect, bh: i32) {
        let (x0, y0, x1, y1) = r;
        let touch = self.win.touch();
        self.rect(x0, y0, x1 - x0, y1 - y0, EDGE);
        self.rect(x0 + 1, y0 + 1, x1 - x0 - 2, y1 - y0 - 2, PANEL);
        self.tip(
            Some(r),
            if touch {
                "Your favorites: tap one to play it."
            } else {
                "Your favorites: click one to play it. The mouse wheel scrolls them."
            },
            "",
        );
        let ids = st.favorites.ids();
        let (ix0, ix1) = (x0 + 8, x1 - 8);
        let mut y = y0 + 7;
        let tw = self.text(ix0, y + 3, "FAVORITES", ACCENT, 1);
        self.text(ix0 + tw + 8, y + 3, &ids.len().to_string(), DIM, 1);
        let cw = font::width("CLOSE", 1) + 8;
        self.button(ix1 - cw, y, cw, bh, "CLOSE", PLAIN, None, Action::FavoritesPanel, None);
        self.tip(Some((ix1 - cw, y, ix1, y + bh)), "Put the favorites away.", "ESC");
        let head = y;
        y += bh + 6;
        let home = std::env::var("HOME").unwrap_or_default();
        let path = st.favorites.path().display().to_string();
        let path = if home.is_empty() { path } else { path.replacen(&home, "~", 1) };
        let foot = y1 - 13;
        let fit = ((foot - 4 - y) / (bh + 2)).max(1) as usize;
        self.fav_top = self.fav_top.min(ids.len().saturating_sub(fit));
        // more than fit: buttons to move the list a page (beside CLOSE)
        let more = ids.len().saturating_sub(self.fav_top + fit);
        if self.fav_top > 0 || more > 0 {
            let (uw, dw) = (font::width("UP", 1) + 8, font::width("DOWN", 1) + 8);
            let (ux, dx) = (ix1 - cw - 6 - dw - 2 - uw, ix1 - cw - 6 - dw);
            let page = fit.max(2) as i32 - 1;
            self.button(
                ux,
                head,
                uw,
                bh,
                "UP",
                if self.fav_top > 0 { PLAIN } else { (LED_OFF, LED_OFF) },
                None,
                Action::Scroll(-page),
                None,
            );
            self.tip(Some((ux, head, ux + uw, head + bh)), format!("The favorites above ({}).", self.fav_top), "");
            self.button(
                dx,
                head,
                dw,
                bh,
                "DOWN",
                if more > 0 { PLAIN } else { (LED_OFF, LED_OFF) },
                None,
                Action::Scroll(page),
                None,
            );
            self.tip(Some((dx, head, dx + dw, head + bh)), format!("The favorites below ({more})."), "");
        }
        if ids.is_empty() {
            let none = if self.compact {
                "NONE YET: KEEP, OR THE * AT THE TOP, KEEPS THE SONG PLAYING"
            } else {
                "NONE YET: THE * BESIDE THE SONG'S NAME (OR K) KEEPS THE SONG PLAYING"
            };
            for (i, l) in wrap(none, ((ix1 - ix0) / ADVANCE).max(10) as usize).iter().enumerate() {
                self.text(ix0, y + 3 + i as i32 * 10, l, DIM, 1);
            }
        }
        let playing = now.sid.map(|s| s.to_string());
        let xw = (font::width("X", 1) + 8).max(bh);
        let cols = ((ix1 - ix0 - xw - 10) / ADVANCE).max(4) as usize;
        for (i, id) in ids.iter().enumerate().skip(self.fav_top).take(fit) {
            let ry = y + (i - self.fav_top) as i32 * (bh + 2);
            let text = id.to_string();
            let lit = playing.as_ref() == Some(&text);
            let rw = ix1 - ix0 - xw - 2;
            self.button(ix0, ry, rw, bh, "", if lit { LIT } else { PLAIN }, None, Action::Go(*id), None);
            // (an id too long for the row: on two lines where the row is
            // tall enough for them, else its start, and that there is more)
            let shown = text.to_uppercase();
            let cut = |t: &str| if t.len() > cols { format!("{}..", &t[..cols - 2]) } else { t.to_string() };
            if shown.len() > cols && bh >= 18 {
                let (a, b) = shown.split_at(cols);
                let ty = ry + (bh - 16) / 2;
                self.text(ix0 + 4, ty, a, TEXT, 1);
                self.text(ix0 + 4, ty + 9, &cut(b), TEXT, 1);
            } else {
                self.text(ix0 + 4, ry + (bh - 7) / 2, &cut(&shown), TEXT, 1);
            }
            let help = if lit { "Playing now.".to_string() } else { format!("Play {text}.") };
            self.tip(Some((ix0, ry, ix0 + rw, ry + bh)), help, "");
            self.button(ix1 - xw, ry, xw, bh, "X", PLAIN, None, Action::Forget(*id), None);
            self.tip(Some((ix1 - xw, ry, ix1, ry + bh)), "Let it go from your favorites.", "");
        }
        let mut foot_text = format!("KEPT IN {}", path.to_uppercase());
        if (self.fav_top > 0 || more > 0) && !self.compact {
            foot_text = format!("{} ABOVE, {more} BELOW: THE WHEEL SCROLLS   {foot_text}", self.fav_top);
        }
        if !(self.compact && WEB) {
            self.text(ix0, foot + 2, &foot_text, FAINT, 1);
        }
    }

    /// The slider `k` between x0 and x1: its name, its slider (a tick each
    /// tenth, the one at its home value taller, lit up to its handle), and
    /// how much. With `aligned` (a width for the name), the name at x0 and
    /// the slider from there to the value, else the slider at the right (at
    /// most 240 wide) with its name before it.
    #[allow(clippy::too_many_arguments)]
    fn knob_one(&mut self, k: usize, v: u32, x0: i32, x1: i32, y: i32, h: i32, aligned: i32) {
        let (name, most, _, home, keys) = knob(k);
        let shown = format!("{v}%");
        let pw = font::width(&format!("{most}%"), 1);
        let label = aligned.max(font::width(name, 1) + 6);
        let sw = (x1 - x0 - label - pw - 6).min(if aligned > 0 { i32::MAX } else { 240 });
        if sw < 40 {
            return;
        }
        let sx1 = x1 - pw - 6;
        let sx0 = sx1 - sw;
        self.text(sx0 - label, y + (h - 7) / 2, name, DIM, 1);
        self.knob[k] = Some((sx0, y, sx1, y + h));
        self.tip(Some((sx0 - label, y, x1, y + h)), knob_help(k, self.win.touch()), keys);
        let mid = y + h / 2;
        self.rect(sx0, mid - 1, sw, 3, LED_OFF);
        for tick in 1..10 {
            let tx = self.knob_x(k, tick * most / 10);
            let tall = if tick * most / 10 == home { 4 } else { 2 };
            self.rect(tx, mid - 1 - tall, 1, 2 * tall + 3, EDGE);
        }
        let hx = self.knob_x(k, v);
        self.rect(sx0, mid - 1, hx - sx0, 3, ACCENT);
        let over = self.knob_drag.is_some_and(|d| d.0 == k) || self.mouse.is_some_and(|m| inside(self.knob[k], m));
        self.rect(hx - 2, y + 1, 5, h - 2, if over { PEAK } else { TEXT });
        let color = if v == 0 {
            AMBER
        } else if k > 0 && v != 100 {
            TEXT
        } else {
            ACCENT
        };
        self.text(x1 - font::width(&shown, 1), y + (h - 7) / 2, &shown, color, 1);
    }

    /// The length slider between x0 and x1: a tick a minute, the length
    /// asked lit up to its handle; at auto, a hollow mark at the piece's own.
    fn slider_track(&mut self, st: &State, now: &Now, x0: i32, x1: i32, y: i32, h: i32) {
        let mid = y + h / 2;
        self.rect(x0, mid - 1, x1 - x0, 3, LED_OFF);
        for min in 1..=(SLIDER_LENGTH / 60.0) as i32 {
            let tx = self.slider_x(min as f64 * 60.0);
            let tall = if min % 5 == 0 { 4 } else { 2 };
            self.rect(tx, mid - 1 - tall, 1, 2 * tall + 3, EDGE);
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
        // (a strip: the bars alone, without the scale under them)
        let scale = y1 - y0 >= 44;
        let y1 = if scale { y1 - 12 } else { y1 - 3 };
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
        let marks: &[(f32, &str)] = if scale { &FREQS } else { &[] };
        for &(f, label) in marks {
            let x = x0 + (spectrum::position(f) * (x1 - x0) as f32) as i32;
            self.text(x - font::width(label, 1) / 2, y1 + 3, label, FAINT, 1);
        }
    }
}

/// The equalizer's scale.
const FREQS: [(f32, &str); 8] = [
    (50.0, "50"),
    (100.0, "100"),
    (200.0, "200"),
    (500.0, "500"),
    (1000.0, "1K"),
    (2000.0, "2K"),
    (5000.0, "5K"),
    (10000.0, "10K"),
];

impl Gui {
    /// A button: `fill` (and its colour under the mouse), its label, a
    /// `mark` underlining it.
    #[allow(clippy::too_many_arguments)]
    fn button(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        fill: (u32, u32),
        mark: Option<u32>,
        left: Action,
        right: Option<Action>,
    ) {
        let over = self.mouse.is_some_and(|(mx, my)| mx >= x && mx < x + w && my >= y && my < y + h);
        self.rect(x, y, w, h, if over { fill.1 } else { fill.0 });
        match halves(label).filter(|_| font::width(label, 1) > w - 2 && h >= 19) {
            Some((a, b)) => {
                let ty = y + (h - 16) / 2;
                self.text(x + (w - font::width(a, 1)) / 2, ty, a, TEXT, 1);
                self.text(x + (w - font::width(b, 1)) / 2, ty + 9, b, TEXT, 1);
            }
            None => {
                let tw = font::width(label, 1);
                self.text(x + (w - tw) / 2, y + (h - 7) / 2, label, TEXT, 1);
            }
        }
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

/// `n` boxes side by side from `x0` to `x1` with `gap` between them, as even
/// as whole pixels allow, the last one's right edge at `x1`: each one's
/// left edge and width.
fn across(x0: i32, x1: i32, n: i32, gap: i32) -> impl Iterator<Item = (i32, i32)> {
    let w = x1 - x0 - (n - 1) * gap;
    (0..n).map(move |i| (x0 + i * gap + w * i / n, w * (i + 1) / n - w * i / n))
}

/// A label too wide for its button, in two lines: at its space, or where
/// its two words meet.
fn halves(label: &str) -> Option<(&str, &str)> {
    if let Some((a, b)) = label.split_once(' ') {
        return Some((a, b));
    }
    let at = match label {
        "STARCONFLICT" | "FULLSCREEN" => 4,
        "SYNTHWAVE" => 5,
        _ => return None,
    };
    Some(label.split_at(at))
}

/// The parts of a line (a song's: "A  /  B  /  C") on lines of at most
/// `cols` characters, as many parts to a line as fit; a line with no such
/// parts, broken at its spaces.
fn lines_of(s: &str, cols: usize) -> Vec<String> {
    if !s.contains("  /  ") {
        // (a word longer than a line, a mix's id: cut at the line's end)
        return wrap(s, cols)
            .iter()
            .flat_map(|l| {
                l.as_bytes().chunks(cols.max(1)).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>()
            })
            .collect();
    }
    let mut lines: Vec<String> = Vec::new();
    for part in s.split("  /  ") {
        match lines.last_mut() {
            Some(l) if l.len() + 3 + part.len() <= cols => {
                l.push_str(" / ");
                l.push_str(part);
            }
            _ => lines.push(part.to_string()),
        }
    }
    lines
}

/// The settings' row's width on one line: its groups' labels and buttons
/// (as `made` draws them).
fn made_width() -> i32 {
    const MADE: [(&str, &[&str]); 3] = [
        ("TUNING", &["AS WRITTEN", "OPTIMISED"]),
        ("TUNES", &["RULES", "NETWORK"]),
        ("INSTRUMENTS", &["SYNTH", "RECORDED", "NETWORK"]),
    ];
    MADE.iter()
        .map(|(l, bs)| font::width(l, 1) + 6 + bs.iter().map(|b| font::width(b, 1) + 10).sum::<i32>() + 14)
        .sum::<i32>()
        - 14
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_wraps_at_spaces() {
        assert_eq!(wrap("ten seconds back", 11), ["ten seconds", "back"]);
        assert_eq!(wrap("one  two", 40), ["one two"]);
        // a word longer than the line has a line of its own
        assert_eq!(wrap("a ~/music/musicbox b", 6), ["a", "~/music/musicbox", "b"]);
    }

    #[test]
    fn rows_of_buttons_reach_both_edges() {
        for (x0, x1, n, gap) in [(6, 228, 5, 2), (6, 210, 3, 2), (8, 632, 7, 3), (0, 100, 1, 2)] {
            let v: Vec<(i32, i32)> = across(x0, x1, n, gap).collect();
            assert_eq!(v.len(), n as usize);
            assert_eq!(v[0].0, x0);
            assert_eq!(v[v.len() - 1].0 + v[v.len() - 1].1, x1, "{v:?}");
            for w in v.windows(2) {
                assert_eq!(w[0].0 + w[0].1 + gap, w[1].0, "{v:?}");
                assert!((w[0].1 - w[1].1).abs() <= 1, "{v:?}");
            }
        }
    }

    #[test]
    fn a_songs_line_breaks_between_its_parts() {
        let line = "SONG 3  /  4/4 2-TONE  /  137 BPM  /  A DORIAN  /  MAIN 1 OF 8";
        assert_eq!(lines_of(line, 34), ["SONG 3 / 4/4 2-TONE / 137 BPM", "A DORIAN / MAIN 1 OF 8"]);
        assert_eq!(lines_of(line, 80), ["SONG 3 / 4/4 2-TONE / 137 BPM / A DORIAN / MAIN 1 OF 8"]);
        // a word on something else: at its spaces
        assert_eq!(lines_of("TAP TO START THE MUSIC", 12), ["TAP TO START", "THE MUSIC"]);
        // a word longer than the line is cut at its end
        assert_eq!(lines_of("STARCONFLICT+SYNTHWAVE.7.0.1 KEPT", 12), ["STARCONFLICT", "+SYNTHWAVE.7", ".0.1", "KEPT"]);
    }

    #[test]
    fn long_names_take_two_lines() {
        assert_eq!(halves("STARCONFLICT"), Some(("STAR", "CONFLICT")));
        assert_eq!(halves("NEXT SONG"), Some(("NEXT", "SONG")));
        assert_eq!(halves("SKA"), None);
        // every genre's name fits a hand's narrowest button, in one line or two
        let narrow = across(6, sdl::HAND_W - 6, 3, 2).map(|b| b.1).min().expect("three");
        for g in music::GENRES {
            let fits = |s: &str| font::width(s, 1) <= narrow - 2;
            assert!(fits(g) || halves(g).is_some_and(|(a, b)| fits(a) && fits(b)), "{g}");
        }
    }

    #[test]
    fn every_tab_has_its_place() {
        for (i, (tab, label, help)) in Tab::ALL.into_iter().enumerate() {
            assert_eq!(Tab::ALL[i].0, tab);
            assert!(!label.is_empty() && !help.is_empty());
        }
        // (the desk's switch asks for them by these numbers)
        assert_eq!((Tab::ALL[0].0, Tab::ALL[1].0), (Tab::Music, Tab::Drums));
    }

    #[test]
    fn the_sliders_name_the_effects_keys() {
        for (k, &(lo, hi)) in crate::FX_KEYS.iter().enumerate() {
            assert_eq!(FX_KEY_NAMES[k], format!("{lo} {hi}"));
            assert_eq!(FX_LABELS[k], music::FX_NAMES[k].to_uppercase());
            assert!(KEYS.contains(&(FX_KEY_NAMES[k], FX_LABELS[k])), "{}", FX_LABELS[k]);
        }
    }

    #[test]
    fn every_key_listed_draws() {
        for c in KEYS.iter().flat_map(|(k, what)| format!("{k}{what}").into_bytes()).filter(|&c| c != b' ') {
            let mut px = vec![0u32; 6 * 7];
            font::draw(&mut px, 6, 7, 0, 0, &(c as char).to_string(), 0xFFFFFF, 1);
            assert!(px.iter().any(|&p| p != 0), "{}", c as char);
        }
    }
}
