//! The few SDL2 calls used: the sound device the music plays through, and
//! for the window a 2D renderer drawing one streaming texture (the picture
//! is made on the CPU, pixel by pixel), with its keys and mouse.

use std::ffi::{CStr, CString, c_char, c_int, c_void};

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
pub struct Window {
    win: *mut c_void,
    ren: *mut c_void,
    tex: *mut c_void,
    /// the texture's size, and how many of the renderer's pixels each of its pixels takes
    w: u32,
    h: u32,
    scale: u32,
    fullscreen: bool,
}

impl Window {
    pub fn open(title: &str, w: u32, h: u32) -> Result<Window, String> {
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
            SDL_SetWindowMinimumSize(win, 480, 300);
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
            let mut w = Window { win, ren, tex: std::ptr::null_mut(), w: 0, h: 0, scale: 1, fullscreen: false };
            w.fit()?;
            Ok(w)
        }
    }

    /// Sizes the texture to the renderer's output: the largest whole scale
    /// that leaves it at least `MIN_W` x `MIN_H`.
    fn fit(&mut self) -> Result<(), String> {
        const MIN_W: i32 = 400;
        const MIN_H: i32 = 250;
        let (mut ow, mut oh) = (0, 0);
        unsafe { SDL_GetRendererOutputSize(self.ren, &mut ow, &mut oh) };
        let (ow, oh) = (ow.max(1), oh.max(1));
        let scale = (ow / MIN_W).min(oh / MIN_H).max(1);
        let (w, h) = ((ow / scale) as u32, (oh / scale) as u32);
        if (w, h, scale as u32) == (self.w, self.h, self.scale) && !self.tex.is_null() {
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
        (self.w, self.h, self.scale) = (w, h, scale as u32);
        Ok(())
    }

    /// The picture's size in pixels (it changes with the window's).
    pub fn size(&mut self) -> (u32, u32) {
        let _ = self.fit();
        (self.w, self.h)
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
