//! The few SDL2 calls used: the sound device the music plays through, and
//! for the window a 2D renderer drawing one streaming texture (the picture
//! is made on the CPU, pixel by pixel), with its keys and mouse. In the
//! browser the window is the page's canvas (`page`): the page hands in its
//! keys and mouse as SDL's events and its size, and takes the picture.

#[cfg(not(target_arch = "wasm32"))]
use std::ffi::CString;
use std::ffi::{CStr, c_char, c_int, c_void};

const SDL_INIT_AUDIO: u32 = 0x10;
const SDL_INIT_VIDEO: u32 = 0x20;
const SDL_INIT_EVENTS: u32 = 0x4000;
const AUDIO_S16LSB: u16 = 0x8010;
const SDL_WINDOW_RESIZABLE: u32 = 0x20;
const SDL_WINDOW_ALLOW_HIGHDPI: u32 = 0x2000;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2FFF_0000;
const SDL_RENDERER_ACCELERATED: u32 = 0x2;
const SDL_RENDERER_PRESENTVSYNC: u32 = 0x4;
const SDL_PIXELFORMAT_ARGB8888: u32 = 0x1636_2004;
const SDL_TEXTUREACCESS_STREAMING: c_int = 1;

pub const SDL_WINDOWEVENT: u32 = 0x200;
pub const SDL_KEYDOWN: u32 = 0x300;
pub const SDL_TEXTINPUT: u32 = 0x303;
pub const SDL_MOUSEMOTION: u32 = 0x400;
pub const SDL_MOUSEBUTTONDOWN: u32 = 0x401;
pub const SDL_MOUSEBUTTONUP: u32 = 0x402;
pub const SDL_MOUSEWHEEL: u32 = 0x403;
pub const SDL_WINDOWEVENT_LEAVE: u8 = 11;
pub const SDL_WINDOWEVENT_CLOSE: u8 = 14;
pub const SDL_BUTTON_LEFT: u8 = 1;
pub const SDL_BUTTON_RIGHT: u8 = 3;
/// A finger on the page's canvas, down, lifted and moved (the page's own
/// events, in SDL's own numbers for a program's: where, as a mouse's): a
/// finger acts when it lifts, and one that stays says what it is on.
pub const TOUCH_DOWN: u32 = 0x8001;
pub const TOUCH_UP: u32 = 0x8002;
pub const TOUCH_MOVE: u32 = 0x8003;

pub const K_BACKSPACE: i32 = 8;
pub const K_RETURN: i32 = 13;
pub const K_ESCAPE: i32 = 27;
pub const K_KP_ENTER: i32 = 0x4000_0058;
pub const K_KP_MINUS: i32 = 0x4000_0056;
pub const K_KP_PLUS: i32 = 0x4000_0057;
pub const K_RIGHT: i32 = 0x4000_004F;
pub const K_LEFT: i32 = 0x4000_0050;
pub const K_DOWN: i32 = 0x4000_0051;
pub const K_UP: i32 = 0x4000_0052;
pub const K_F1: i32 = 0x4000_003A;
pub const K_F11: i32 = 0x4000_0044;

const SDL_WINDOW_FULLSCREEN_DESKTOP: u32 = 0x1001;

/// The window's least (its picture's, at a scale of 1): what all the
/// genres' buttons, the settings and the transport fit.
const MIN_W: i32 = 480;
const MIN_H: i32 = 300;

/// A hand's window's least (a phone's, held upright: the picture's parts
/// one above the other), and its least on its side (the parts side by side).
pub const HAND_W: i32 = 208;
pub const HAND_H: i32 = 360;
const SIDE_W: i32 = 416;
const SIDE_H: i32 = 196;

/// The picture for an output `ow` x `oh` pixels, `dpr` of them to one of
/// the window's own (the page's): its size, how many output pixels each of
/// its pixels takes (a whole number), and whether it is a hand's (`touch`:
/// a finger works it; or it is too narrow for a desk's). A desk's: the
/// largest scale that leaves it at least `MIN_W` x `MIN_H`. A hand's: the
/// largest that leaves `HAND_W` x `HAND_H` upright or `SIDE_W` x `SIDE_H` on
/// its side, a pixel at most two of the window's own (rows a finger's
/// height, letters a comfortable size, on a phone as on a tablet).
pub fn picture(ow: i32, oh: i32, dpr: f32, touch: bool) -> (u32, u32, u32, bool) {
    let (ow, oh) = (ow.max(1), oh.max(1));
    let dpr = if dpr.is_finite() && dpr > 0.1 { dpr } else { 1.0 };
    if !touch && ow as f32 / dpr >= MIN_W as f32 - 0.5 {
        let scale = (ow / MIN_W).min(oh / MIN_H).max(1);
        return ((ow / scale) as u32, (oh / scale) as u32, scale as u32, false);
    }
    let most = ((2.0 * dpr + 0.01) as i32).max(1);
    let upright = (ow / HAND_W).min(oh / HAND_H);
    let side = (ow / SIDE_W).min(oh / SIDE_H);
    let scale = upright.max(side).min(most).max(1);
    ((ow / scale) as u32, (oh / scale) as u32, scale as u32, true)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Event {
    pub kind: u32,
    raw: [u8; 52],
}

impl Event {
    pub fn new() -> Event {
        Event { kind: 0, raw: [0; 52] }
    }
    fn i32_at(&self, off: usize) -> i32 {
        let o = off - 4;
        i32::from_le_bytes([self.raw[o], self.raw[o + 1], self.raw[o + 2], self.raw[o + 3]])
    }
    /// SDL_KEYDOWN: the key code (keysym.sym)
    pub fn key(&self) -> i32 {
        self.i32_at(20)
    }
    /// SDL_KEYDOWN: a Ctrl key held with it (keysym.mod's KMOD_CTRL)
    pub fn ctrl(&self) -> bool {
        u16::from_le_bytes([self.raw[24 - 4], self.raw[25 - 4]]) & 0x00C0 != 0
    }
    /// SDL_WINDOWEVENT: the event id
    pub fn window_event(&self) -> u8 {
        self.raw[12 - 4]
    }
    /// SDL_MOUSEMOTION, SDL_MOUSEBUTTONDOWN: where, in window coordinates
    pub fn mouse_xy(&self) -> (i32, i32) {
        (self.i32_at(20), self.i32_at(24))
    }
    /// SDL_MOUSEBUTTONDOWN: which button (1 left, 3 right)
    pub fn button(&self) -> u8 {
        self.raw[16 - 4]
    }
    /// SDL_MOUSEWHEEL: the scroll, positive away from the user
    pub fn wheel(&self) -> i32 {
        self.i32_at(20)
    }
    /// SDL_TEXTINPUT: the text typed (UTF-8)
    pub fn text(&self) -> String {
        let t = &self.raw[12 - 4..12 - 4 + 32];
        let n = t.iter().position(|&c| c == 0).unwrap_or(t.len());
        String::from_utf8_lossy(&t[..n]).into_owned()
    }
}

unsafe extern "C" {
    fn SDL_InitSubSystem(flags: u32) -> c_int;
    fn SDL_QuitSubSystem(flags: u32);
    fn SDL_OpenAudioDevice(
        device: *const c_char,
        iscapture: c_int,
        desired: *const AudioSpec,
        obtained: *mut AudioSpec,
        allowed: c_int,
    ) -> u32;
    fn SDL_PauseAudioDevice(dev: u32, pause: c_int);
    fn SDL_CloseAudioDevice(dev: u32);
    fn SDL_GetCurrentAudioDriver() -> *const c_char;
    fn SDL_GetError() -> *const c_char;
    fn SDL_SetHint(name: *const c_char, value: *const c_char) -> c_int;
    fn SDL_CreateWindow(title: *const c_char, x: c_int, y: c_int, w: c_int, h: c_int, flags: u32) -> *mut c_void;
    fn SDL_DestroyWindow(win: *mut c_void);
    fn SDL_SetWindowFullscreen(win: *mut c_void, flags: u32) -> c_int;
    fn SDL_SetWindowTitle(win: *mut c_void, title: *const c_char);
    fn SDL_SetWindowMinimumSize(win: *mut c_void, w: c_int, h: c_int);
    fn SDL_GetWindowSize(win: *mut c_void, w: *mut c_int, h: *mut c_int);
    fn SDL_CreateRenderer(win: *mut c_void, index: c_int, flags: u32) -> *mut c_void;
    fn SDL_DestroyRenderer(r: *mut c_void);
    fn SDL_GetRendererOutputSize(r: *mut c_void, w: *mut c_int, h: *mut c_int) -> c_int;
    fn SDL_CreateTexture(r: *mut c_void, format: u32, access: c_int, w: c_int, h: c_int) -> *mut c_void;
    fn SDL_DestroyTexture(t: *mut c_void);
    fn SDL_UpdateTexture(t: *mut c_void, rect: *const c_void, pixels: *const c_void, pitch: c_int) -> c_int;
    fn SDL_RenderClear(r: *mut c_void) -> c_int;
    fn SDL_RenderCopy(r: *mut c_void, t: *mut c_void, src: *const c_void, dst: *const c_void) -> c_int;
    fn SDL_RenderPresent(r: *mut c_void);
    fn SDL_PollEvent(ev: *mut Event) -> c_int;
    fn SDL_SetClipboardText(text: *const c_char) -> c_int;
    fn SDL_GetClipboardText() -> *mut c_char;
    fn SDL_free(p: *mut c_void);
}

/// SDL's audio callback: fill `len` bytes at `stream`.
pub type AudioCallback = unsafe extern "C" fn(user: *mut c_void, stream: *mut u8, len: c_int);

#[repr(C)]
struct AudioSpec {
    freq: c_int,
    format: u16,
    channels: u8,
    silence: u8,
    samples: u16,
    padding: u16,
    size: u32,
    callback: Option<AudioCallback>,
    userdata: *mut c_void,
}

/// A sound output: SDL calls the callback for 16-bit stereo on its own audio
/// thread, which it runs at real-time priority where the system allows
/// (RealtimeKit), ahead of everything else on a busy machine.
pub struct AudioOut {
    id: u32,
}

impl AudioOut {
    /// Opens the default output at `rate`, `frames` per callback, playing at once.
    ///
    /// # Safety
    /// `user` must stay valid, and `callback` safe to call with it from
    /// another thread, until the AudioOut is dropped.
    pub unsafe fn open(rate: u32, frames: u16, callback: AudioCallback, user: *mut c_void) -> Result<AudioOut, String> {
        unsafe {
            SDL_SetHint(c"SDL_THREAD_FORCE_REALTIME_TIME_CRITICAL".as_ptr(), c"1".as_ptr());
            SDL_SetHint(c"SDL_AUDIO_DEVICE_APP_NAME".as_ptr(), c"musicbox".as_ptr());
            SDL_SetHint(c"SDL_AUDIO_DEVICE_STREAM_NAME".as_ptr(), c"music".as_ptr());
            if SDL_InitSubSystem(SDL_INIT_AUDIO) != 0 {
                return Err(error("SDL audio"));
            }
            let want = AudioSpec {
                freq: rate as c_int,
                format: AUDIO_S16LSB,
                channels: 2,
                silence: 0,
                samples: frames,
                padding: 0,
                size: 0,
                callback: Some(callback),
                userdata: user,
            };
            let mut have = AudioSpec { callback: None, userdata: std::ptr::null_mut(), ..want };
            // no allowed changes: SDL converts to the device's format itself
            let id = SDL_OpenAudioDevice(std::ptr::null(), 0, &want, &mut have, 0);
            if id == 0 {
                let e = error("SDL_OpenAudioDevice");
                SDL_QuitSubSystem(SDL_INIT_AUDIO);
                return Err(e);
            }
            SDL_PauseAudioDevice(id, 0);
            Ok(AudioOut { id })
        }
    }

    /// The audio driver in use (pipewire, pulseaudio, alsa...).
    pub fn driver(&self) -> String {
        let d = unsafe { SDL_GetCurrentAudioDriver() };
        if d.is_null() { "?".into() } else { unsafe { CStr::from_ptr(d) }.to_string_lossy().into_owned() }
    }
}

impl Drop for AudioOut {
    /// Closes the device (SDL waits for a callback under way), then the subsystem.
    fn drop(&mut self) {
        unsafe {
            SDL_CloseAudioDevice(self.id);
            SDL_QuitSubSystem(SDL_INIT_AUDIO);
        }
    }
}

pub fn error(what: &str) -> String {
    let e = unsafe { SDL_GetError() };
    let msg = if e.is_null() { "?".into() } else { unsafe { CStr::from_ptr(e) }.to_string_lossy().into_owned() };
    format!("{what}: {msg}")
}

/// SDL leaves SIGINT and SIGTERM alone: the program's own handlers end it,
/// whether or not the window is open. Before any subsystem starts.
pub fn no_signal_handlers() {
    unsafe {
        SDL_SetHint(c"SDL_NO_SIGNAL_HANDLERS".as_ptr(), c"1".as_ptr());
    }
}

/// A window showing one texture of `size()` pixels, scaled up by a whole
/// number to fill the window (sharp pixels at any size).
#[cfg(not(target_arch = "wasm32"))]
pub struct Window {
    win: *mut c_void,
    ren: *mut c_void,
    tex: *mut c_void,
    /// the texture's size, and how many of the renderer's pixels each of its pixels takes
    w: u32,
    h: u32,
    scale: u32,
    fullscreen: bool,
    /// worked by a finger (`--touch`): a hand's window, whatever its size;
    /// and whether the picture is a hand's (`picture`)
    touch: bool,
    compact: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Window {
    pub fn open(title: &str, w: u32, h: u32, touch: bool) -> Result<Window, String> {
        unsafe {
            // nearest-neighbour scaling, and no compositor bypass for a small window
            SDL_SetHint(c"SDL_RENDER_SCALE_QUALITY".as_ptr(), c"0".as_ptr());
            SDL_SetHint(c"SDL_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR".as_ptr(), c"0".as_ptr());
            if SDL_InitSubSystem(SDL_INIT_VIDEO | SDL_INIT_EVENTS) != 0 {
                return Err(error("SDL video"));
            }
            let t = CString::new(title).unwrap_or_default();
            let flags = SDL_WINDOW_RESIZABLE | SDL_WINDOW_ALLOW_HIGHDPI;
            let win = SDL_CreateWindow(
                t.as_ptr(),
                SDL_WINDOWPOS_CENTERED,
                SDL_WINDOWPOS_CENTERED,
                w as c_int,
                h as c_int,
                flags,
            );
            if win.is_null() {
                let e = error("SDL_CreateWindow");
                SDL_QuitSubSystem(SDL_INIT_VIDEO | SDL_INIT_EVENTS);
                return Err(e);
            }
            let (min_w, min_h) = if touch { (HAND_W, HAND_H) } else { (MIN_W, MIN_H) };
            SDL_SetWindowMinimumSize(win, min_w as c_int, min_h as c_int);
            let mut ren = SDL_CreateRenderer(win, -1, SDL_RENDERER_ACCELERATED | SDL_RENDERER_PRESENTVSYNC);
            if ren.is_null() {
                // no GPU: SDL's software renderer
                ren = SDL_CreateRenderer(win, -1, 0);
            }
            if ren.is_null() {
                let e = error("SDL_CreateRenderer");
                SDL_DestroyWindow(win);
                SDL_QuitSubSystem(SDL_INIT_VIDEO | SDL_INIT_EVENTS);
                return Err(e);
            }
            let mut w = Window {
                win,
                ren,
                tex: std::ptr::null_mut(),
                w: 0,
                h: 0,
                scale: 1,
                fullscreen: false,
                touch,
                compact: touch,
            };
            w.fit()?;
            Ok(w)
        }
    }

    /// Sizes the texture to the renderer's output: the largest whole scale
    /// that leaves it at least the window's least (`picture`).
    fn fit(&mut self) -> Result<(), String> {
        let (mut ow, mut oh, mut ww, mut wh) = (0, 0, 0, 0);
        unsafe {
            SDL_GetRendererOutputSize(self.ren, &mut ow, &mut oh);
            SDL_GetWindowSize(self.win, &mut ww, &mut wh);
        }
        let (w, h, scale, compact) = picture(ow, oh, ow as f32 / ww.max(1) as f32, self.touch);
        self.compact = compact;
        if (w, h, scale) == (self.w, self.h, self.scale) && !self.tex.is_null() {
            return Ok(());
        }
        unsafe {
            if !self.tex.is_null() {
                SDL_DestroyTexture(self.tex);
            }
            self.tex = SDL_CreateTexture(
                self.ren,
                SDL_PIXELFORMAT_ARGB8888,
                SDL_TEXTUREACCESS_STREAMING,
                w as c_int,
                h as c_int,
            );
        }
        if self.tex.is_null() {
            return Err(error("SDL_CreateTexture"));
        }
        (self.w, self.h, self.scale) = (w, h, scale);
        Ok(())
    }

    /// The picture's size in pixels (it changes with the window's).
    pub fn size(&mut self) -> (u32, u32) {
        let _ = self.fit();
        (self.w, self.h)
    }

    /// Whether the picture is a hand's (as `size` last found), whether a
    /// finger works the window, and whether it can fill the screen.
    pub fn compact(&self) -> bool {
        self.compact
    }

    pub fn touch(&self) -> bool {
        self.touch
    }

    pub fn can_fullscreen(&self) -> bool {
        true
    }

    /// Shows `px` (0xAARRGGBB, `size()` of them), stretched over the whole
    /// output (any remainder of the whole scale is a sliver at most).
    pub fn present(&self, px: &[u32]) {
        if self.tex.is_null() || px.len() < (self.w * self.h) as usize {
            return;
        }
        unsafe {
            SDL_UpdateTexture(self.tex, std::ptr::null(), px.as_ptr() as *const c_void, (self.w * 4) as c_int);
            SDL_RenderClear(self.ren);
            SDL_RenderCopy(self.ren, self.tex, std::ptr::null(), std::ptr::null());
            SDL_RenderPresent(self.ren);
        }
    }

    /// A point in window coordinates (the mouse's) in the picture's pixels.
    pub fn to_picture(&self, x: i32, y: i32) -> (i32, i32) {
        let (mut ww, mut wh) = (0, 0);
        unsafe { SDL_GetWindowSize(self.win, &mut ww, &mut wh) };
        let fx = self.w as f32 / ww.max(1) as f32;
        let fy = self.h as f32 / wh.max(1) as f32;
        ((x as f32 * fx) as i32, (y as f32 * fy) as i32)
    }

    /// The clipboard's text (empty when it holds none), and the clipboard set
    /// to `text` (whether it took it): the desktop's, while the window is open.
    pub fn clipboard(&self) -> String {
        unsafe {
            let p = SDL_GetClipboardText();
            if p.is_null() {
                return String::new();
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            SDL_free(p.cast());
            s
        }
    }

    pub fn set_clipboard(&self, text: &str) -> bool {
        CString::new(text).is_ok_and(|c| unsafe { SDL_SetClipboardText(c.as_ptr()) } == 0)
    }

    pub fn set_title(&self, title: &str) {
        let t = CString::new(title).unwrap_or_default();
        unsafe { SDL_SetWindowTitle(self.win, t.as_ptr()) }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        unsafe {
            SDL_SetWindowFullscreen(self.win, if self.fullscreen { SDL_WINDOW_FULLSCREEN_DESKTOP } else { 0 });
        }
    }

    pub fn poll(&self) -> Option<Event> {
        let mut ev = Event::new();
        if unsafe { SDL_PollEvent(&mut ev) } != 0 { Some(ev) } else { None }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if !self.tex.is_null() {
                SDL_DestroyTexture(self.tex);
            }
            SDL_DestroyRenderer(self.ren);
            SDL_DestroyWindow(self.win);
            SDL_QuitSubSystem(SDL_INIT_VIDEO | SDL_INIT_EVENTS);
        }
    }
}

/// The browser's window: the page's canvas.
#[cfg(target_arch = "wasm32")]
pub use page::Window;

#[cfg(target_arch = "wasm32")]
pub mod page {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    use super::*;

    /// What the window asks of the page: a title, the clipboard set, to be
    /// fullscreen or not.
    pub enum Ask {
        Title(String),
        Copy(String),
        Fullscreen,
    }

    thread_local! {
        static EVENTS: RefCell<VecDeque<Event>> = const { RefCell::new(VecDeque::new()) };
        /// the canvas's size: in the screen's pixels, and in the page's
        static SIZE: Cell<(i32, i32, f32, f32)> = const { Cell::new((1280, 720, 1280.0, 720.0)) };
        /// the page's device: a finger works it (not a mouse), and it can
        /// fill the screen
        static DEVICE: Cell<(bool, bool)> = const { Cell::new((false, true)) };
        /// what was last pasted, and whether it has not been read since
        static PASTED: RefCell<(String, bool)> = const { RefCell::new((String::new(), false)) };
        static ASKS: RefCell<Vec<Ask>> = const { RefCell::new(Vec::new()) };
        /// the picture as last shown (RGBA, as the canvas takes it), and its size
        static SHOWN: RefCell<(Vec<u8>, u32, u32)> = const { RefCell::new((Vec::new(), 0, 0)) };
    }

    fn event(kind: u32, f: impl FnOnce(&mut [u8; 52])) {
        let mut e = Event::new();
        e.kind = kind;
        f(&mut e.raw);
        EVENTS.with(|v| v.borrow_mut().push_back(e));
    }

    /// `x` at `at` in SDL's event (where `Event`'s accessors read it)
    fn put(raw: &mut [u8; 52], at: usize, x: i32) {
        raw[at - 4..at].copy_from_slice(&x.to_le_bytes());
    }

    /// A key pressed (SDL's key code, a Ctrl key with it or not).
    pub fn key(code: i32, ctrl: bool) {
        event(SDL_KEYDOWN, |r| {
            put(r, 20, code);
            if ctrl {
                r[24 - 4] = 0x40;
            }
        });
    }

    /// Text typed (what SDL's text input gives: the key's character).
    pub fn text(t: &str) {
        event(SDL_TEXTINPUT, |r| {
            let b = t.as_bytes();
            let n = b.len().min(31);
            r[8..8 + n].copy_from_slice(&b[..n]);
        });
    }

    /// The mouse at (x, y), the page's pixels on the canvas.
    pub fn motion(x: i32, y: i32) {
        event(SDL_MOUSEMOTION, |r| {
            put(r, 20, x);
            put(r, 24, y);
        });
    }

    /// A mouse button down or up at (x, y) (1 left, 3 right).
    pub fn button(down: bool, x: i32, y: i32, button: u8) {
        event(if down { SDL_MOUSEBUTTONDOWN } else { SDL_MOUSEBUTTONUP }, |r| {
            r[16 - 4] = button;
            put(r, 20, x);
            put(r, 24, y);
        });
    }

    /// A finger down, lifted or moved (`TOUCH_DOWN`, `TOUCH_UP`,
    /// `TOUCH_MOVE`) at (x, y), the page's pixels on the canvas.
    pub fn finger(kind: u32, x: i32, y: i32) {
        event(kind, |r| {
            put(r, 20, x);
            put(r, 24, y);
        });
    }

    /// The page's device: whether a finger works it, and whether the page
    /// can fill the screen.
    pub fn device(touch: bool, fullscreen: bool) {
        DEVICE.with(|d| d.set((touch, fullscreen)));
    }

    /// The wheel turned (positive away from the user).
    pub fn wheel(by: i32) {
        event(SDL_MOUSEWHEEL, |r| put(r, 20, by));
    }

    /// The mouse gone from the canvas.
    pub fn leave() {
        event(SDL_WINDOWEVENT, |r| r[12 - 4] = SDL_WINDOWEVENT_LEAVE);
    }

    /// The canvas's size, in the screen's pixels and the page's.
    pub fn resize(ow: i32, oh: i32, css_w: f32, css_h: f32) {
        SIZE.with(|s| s.set((ow, oh, css_w, css_h)));
    }

    /// What was pasted (Ctrl-V, or the clipboard read for PASTE): its text,
    /// unread until the window or a paste takes it.
    pub fn pasted(t: &str) {
        PASTED.with(|p| *p.borrow_mut() = (t.to_string(), true));
    }

    /// Whether something pasted is waiting to be read.
    pub fn fresh() -> bool {
        PASTED.with(|p| p.borrow().1)
    }

    /// What the window asked of the page since last time.
    pub fn asks() -> Vec<Ask> {
        ASKS.with(|a| std::mem::take(&mut *a.borrow_mut()))
    }

    /// The picture last shown (RGBA) and its size, for `f` to put on the canvas.
    pub fn shown<R>(f: impl FnOnce(&[u8], u32, u32) -> R) -> R {
        SHOWN.with(|s| {
            let s = s.borrow();
            f(&s.0, s.1, s.2)
        })
    }

    fn ask(a: Ask) {
        ASKS.with(|v| v.borrow_mut().push(a));
    }

    pub struct Window {
        w: u32,
        h: u32,
        compact: bool,
    }

    impl Window {
        pub fn open(title: &str, _w: u32, _h: u32, _touch: bool) -> Result<Window, String> {
            ask(Ask::Title(title.to_string()));
            Ok(Window { w: 0, h: 0, compact: false })
        }

        pub fn size(&mut self) -> (u32, u32) {
            let (ow, oh, css_w, _) = SIZE.with(|s| s.get());
            let (w, h, _, compact) = picture(ow, oh, ow as f32 / css_w.max(1.0), self.touch());
            (self.w, self.h, self.compact) = (w, h, compact);
            (w, h)
        }

        pub fn compact(&self) -> bool {
            self.compact
        }

        pub fn touch(&self) -> bool {
            DEVICE.with(|d| d.get().0)
        }

        pub fn can_fullscreen(&self) -> bool {
            DEVICE.with(|d| d.get().1)
        }

        /// `px` (0xAARRGGBB) as the canvas takes it (R, G, B, A).
        pub fn present(&self, px: &[u32]) {
            SHOWN.with(|s| {
                let mut s = s.borrow_mut();
                let n = (self.w * self.h) as usize;
                s.0.clear();
                s.0.extend(px.iter().take(n).flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8, 0xFF]));
                (s.1, s.2) = (self.w, self.h);
            });
        }

        pub fn to_picture(&self, x: i32, y: i32) -> (i32, i32) {
            let (.., ww, wh) = SIZE.with(|s| s.get());
            let fx = self.w as f32 / ww.max(1.0);
            let fy = self.h as f32 / wh.max(1.0);
            ((x as f32 * fx) as i32, (y as f32 * fy) as i32)
        }

        pub fn clipboard(&self) -> String {
            PASTED.with(|p| {
                let mut p = p.borrow_mut();
                p.1 = false;
                p.0.clone()
            })
        }

        pub fn set_clipboard(&self, text: &str) -> bool {
            ask(Ask::Copy(text.to_string()));
            true
        }

        pub fn set_title(&self, title: &str) {
            ask(Ask::Title(title.to_string()));
        }

        pub fn toggle_fullscreen(&mut self) {
            ask(Ask::Fullscreen);
        }

        pub fn poll(&self) -> Option<Event> {
            EVENTS.with(|v| v.borrow_mut().pop_front())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_desks_picture_is_as_it_was() {
        // (a window of 1280 x 720, and one on a screen of twice the pixels)
        assert_eq!(picture(1280, 720, 1.0, false), (640, 360, 2, false));
        assert_eq!(picture(2560, 1440, 2.0, false), (640, 360, 4, false));
        assert_eq!(picture(480, 300, 1.0, false), (480, 300, 1, false));
        assert_eq!(picture(1920, 1080, 1.0, false), (640, 360, 3, false));
    }

    #[test]
    fn a_hands_picture_has_rows_a_finger_can_hit() {
        // phones upright (the page's size, the screen's pixels to one of its own)
        for (w, h, dpr) in
            [(390.0, 664.0, 3.0), (360.0, 640.0, 3.0), (375.0, 553.0, 2.0), (412.0, 780.0, 2.625), (430.0, 740.0, 3.0)]
        {
            let (ow, oh) = ((w * dpr) as i32, (h * dpr) as i32);
            let (pw, ph, scale, hand) = picture(ow, oh, dpr, true);
            assert!(hand);
            assert!(pw as i32 >= HAND_W && ph as i32 >= HAND_H, "{w} x {h}: {pw} x {ph}");
            // a picture pixel 1.3 to 2 of the page's: a row of 24 is 32 to 48
            let css = scale as f32 / dpr;
            assert!((1.3..=2.01).contains(&css), "{w} x {h}: {css}");
        }
        // on its side: the parts side by side
        let (pw, ph, _, hand) = picture(2532, 1170, 3.0, true);
        assert!(hand && pw as i32 >= SIDE_W && ph as i32 >= SIDE_H && (ph as i32) < HAND_H, "{pw} x {ph}");
        // a window too narrow for a desk's, with a mouse: a hand's too
        assert!(picture(420, 700, 1.0, false).3);
        assert!(!picture(480, 700, 1.0, false).3);
    }
}
