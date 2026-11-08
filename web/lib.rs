//! musicbox in the browser: the composer and synthesiser (src/music.rs and
//! what it uses, unchanged) built as WebAssembly, for docs/app to drive. No
//! crates and no bindings generator: plain exported functions over the
//! module's memory, a handle (`Web`) passed back and forth as a pointer.
//!
//!     cargo build --release --target wasm32-unknown-unknown --manifest-path web/Cargo.toml
//!
//! The recorded instruments come apart (tools/web-samples.sh): the tables
//! (`mb_pack`), then each group of recordings a genre plays (`mb_group`),
//! decoded by the page; until a group is in, the synthesiser plays its parts.
//!
//! The page's window is the desktop's (src/gui.rs, drawn the same, pixel for
//! pixel), in a second copy of the module on the page's own thread
//! (`mb_ui_*`): the page hands it its keys and mouse, what is playing and
//! the sound as the speakers play it, and takes the picture and what the
//! window's actions ask of the music, for the worker.

#![allow(dead_code)]

#[path = "../src/app.rs"]
mod app;
#[path = "../src/favorites.rs"]
mod favorites;
#[path = "../src/font.rs"]
mod font;
#[path = "../src/gui.rs"]
mod gui;
#[path = "../src/mp3.rs"]
mod mp3;
#[path = "../src/music.rs"]
mod music;
#[path = "../src/net.rs"]
mod net;
#[path = "../src/samples.rs"]
mod samples;
#[path = "../src/sdl.rs"]
mod sdl;
#[path = "../src/spectrum.rs"]
mod spectrum;
#[path = "../src/tap.rs"]
mod tap;
#[path = "../src/time.rs"]
mod time;
#[path = "../src/tunes.rs"]
mod tunes;

use std::cell::{Cell, RefCell};

use app::*;
use music::Stream;
use time::Instant;

/// What JavaScript holds: the music, the buffer it is made into, and the
/// last text, bytes or tag handed out.
pub struct Web {
    stream: Stream,
    buf: Vec<f32>,
    text: String,
    bytes: Vec<u8>,
    tag: Vec<u8>,
}

fn seed(lo: u32, hi: u32) -> u64 {
    (hi as u64) << 32 | lo as u64
}

/// # Safety
/// `w` is a handle from `mb_new`, not freed.
unsafe fn web<'a>(w: *mut Web) -> &'a mut Web {
    // SAFETY: the caller's
    unsafe { &mut *w }
}

/// The music `genre` (a mix id) from a seed (its low and high 32 bits),
/// made as `opts` (`Opts::code`) asks.
#[unsafe(no_mangle)]
pub extern "C" fn mb_new(genre: u32, seed_lo: u32, seed_hi: u32, opts: u32) -> *mut Web {
    let stream = Stream::new(genre as usize, seed(seed_lo, seed_hi), music::Opts::from_code(opts));
    Box::into_raw(Box::new(Web { stream, buf: Vec::new(), text: String::new(), bytes: Vec::new(), tag: Vec::new() }))
}

/// The next `frames` (interleaved stereo, 48 kHz): where they are.
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_render(w: *mut Web, frames: u32) -> *const f32 {
    let w = unsafe { web(w) };
    w.buf.resize(2 * frames as usize, 0.0);
    w.stream.render(&mut w.buf);
    w.buf.as_ptr()
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_set_genre(w: *mut Web, genre: u32) {
    unsafe { web(w) }.stream.set_genre(genre as usize);
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_skip(w: *mut Web) {
    unsafe { web(w) }.stream.skip();
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_replay(w: *mut Web) {
    unsafe { web(w) }.stream.replay();
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_set_playing(w: *mut Web, on: u32) {
    unsafe { web(w) }.stream.set_playing(on != 0);
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_set_length(w: *mut Web, secs: f64) {
    unsafe { web(w) }.stream.set_length(secs);
}

/// How the music is to be made (`Opts::code`): the piece playing made
/// again from its start, the new way.
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_set_opts(w: *mut Web, opts: u32) {
    unsafe { web(w) }.stream.set_opts(music::Opts::from_code(opts));
}

/// The listener's effects (`Effects::halves`: reverb, echo and chorus in
/// `lo`, width, bass and treble in `hi`), from the next frames.
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_set_effects(w: *mut Web, lo: u32, hi: u32) {
    unsafe { web(w) }.stream.set_effects(music::Effects::from_halves(lo, hi));
}

/// The `n` bytes of text at `p`, as a song's id.
///
/// # Safety
/// `p` and `n` as `mb_alloc` gave and took them, filled.
unsafe fn song_id(p: *const u8, n: u32) -> Option<music::SongId> {
    // SAFETY: the caller's
    let b = unsafe { std::slice::from_raw_parts(p, n as usize) };
    music::SongId::parse(std::str::from_utf8(b).ok()?)
}

/// The song id at `p` (`n` bytes of text), read: as JSON (its music, the
/// genre writing it and the drummer, or -1, its seed, length and settings,
/// and the id as written), or `null` when it is none (its length from
/// `mb_text_len`).
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_song_id(w: *mut Web, p: *const u8, n: u32) -> *const u8 {
    let w = unsafe { web(w) };
    w.text = match unsafe { song_id(p, n) } {
        Some(id) => {
            let (p, d) = music::parts(id.music);
            format!(
                "{{\"id\":{},\"primary\":{p},\"drums\":{},\"seed\":\"{}\",\"length\":{},\"opts\":{},\"text\":\"{id}\"}}",
                id.music,
                d.map_or(-1, |d| d as i64),
                id.seed,
                id.length,
                id.opts.code()
            )
        }
        None => "null".into(),
    };
    w.text.as_ptr()
}

/// Plays the song whose id is at `p` (`n` bytes of text) from its start, at
/// once (as the history kept it if it played lately): 1, or 0 when it is
/// not an id.
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_go(w: *mut Web, p: *const u8, n: u32) -> u32 {
    let w = unsafe { web(w) };
    match unsafe { song_id(p, n) } {
        Some(id) => {
            w.stream.go(id);
            1
        }
        None => 0,
    }
}

/// Plays on from `secs` into the song playing (the start of the bar then).
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_seek(w: *mut Web, secs: f64) {
    unsafe { web(w) }.stream.seek(secs);
}

/// Room for `n` bytes in the module's memory, for JavaScript to fill
/// (aligned for any number); `mb_free` gives it back.
#[unsafe(no_mangle)]
pub extern "C" fn mb_alloc(n: u32) -> *mut u8 {
    let mut v = vec![0u64; (n as usize).div_ceil(8)];
    let p = v.as_mut_ptr() as *mut u8;
    std::mem::forget(v);
    p
}

/// # Safety
/// `p` and `n` as `mb_alloc` gave and took them, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_free(p: *mut u8, n: u32) {
    let words = (n as usize).div_ceil(8);
    // SAFETY: the caller's
    drop(unsafe { Vec::from_raw_parts(p as *mut u64, words, words) });
}

/// The recordings' tables (pack.bin: `n` bytes at `p`): 1, else 0 and why
/// in `mb_text`.
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` holds `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_pack(w: *mut Web, p: *const u8, n: u32) -> u32 {
    let w = unsafe { web(w) };
    // SAFETY: the caller's
    let meta = unsafe { std::slice::from_raw_parts(p, n as usize) };
    match samples::Partial::new(meta) {
        Ok(pt) => {
            // SAFETY: between calls: nothing holds the bank
            unsafe { *samples::web() = Some(pt) };
            1
        }
        Err(e) => {
            w.text = e;
            0
        }
    }
}

/// Group `g`'s recordings (`n` 16-bit samples at `p`: its frames in stereo,
/// the channels interleaved): its instruments playable from the next song
/// made. 1, else 0 and why in `mb_text`.
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` holds `n` samples, aligned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_group(w: *mut Web, g: u32, p: *const i16, n: u32) -> u32 {
    let w = unsafe { web(w) };
    // SAFETY: the caller's
    let pcm = unsafe { std::slice::from_raw_parts(p, n as usize) };
    // SAFETY: between calls: nothing holds the bank
    let r = match unsafe { samples::web() } {
        Some(pt) => pt.add(g as usize, pcm),
        None => Err("no tables yet".into()),
    };
    match r {
        Ok(()) => 1,
        Err(e) => {
            w.text = e;
            0
        }
    }
}

/// A network instrument: its line of the networks' index (`ln` bytes at
/// `lp`) and its file (`n` bytes at `p`), playing the recording it names
/// once the tables are in (`mb_pack`). 1, else 0 and why in `mb_text`.
///
/// # Safety
/// `w` is a handle from `mb_new`; `lp` and `p` hold `ln` and `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_net(w: *mut Web, lp: *const u8, ln: u32, p: *const u8, n: u32) -> u32 {
    let w = unsafe { web(w) };
    // SAFETY: the caller's
    let (line, file) =
        unsafe { (std::slice::from_raw_parts(lp, ln as usize), std::slice::from_raw_parts(p, n as usize)) };
    let r = (|| {
        let line = std::str::from_utf8(line).map_err(|_| "an index line not UTF-8".to_string())?;
        let (kind, name, art, _) = net::index_line(line).ok_or_else(|| format!("not an index line: {line}"))?;
        // SAFETY: between calls: nothing holds the bank or the networks
        let pt = unsafe { samples::web() }.as_ref().ok_or("no tables yet")?;
        let id = pt.find(kind, name, art).ok_or_else(|| format!("{kind} {name} {art}: not in the recordings"))?;
        let net = net::Net::parse(file, pt.bank.release(id))?;
        unsafe { net::web() }.add(id, net);
        Ok::<(), String>(())
    })();
    match r {
        Ok(()) => 1,
        Err(e) => {
            w.text = e;
            0
        }
    }
}

/// A tunes network (`n` bytes at `p`) for the genre `genre` (celtic's
/// alone so far): 1, else 0 and why in `mb_text`.
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` holds `n` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_tunes(w: *mut Web, genre: u32, p: *const u8, n: u32) -> u32 {
    let w = unsafe { web(w) };
    // SAFETY: the caller's
    let file = unsafe { std::slice::from_raw_parts(p, n as usize) };
    match tunes::TuneNet::parse(file) {
        Ok(t) if music::canon(genre as usize) == music::CELTIC => {
            // SAFETY: between calls: nothing holds the networks
            unsafe { tunes::web() }.celtic = Some(t);
            1
        }
        Ok(_) => {
            w.text = format!("no tunes network for genre {genre}");
            0
        }
        Err(e) => {
            w.text = e;
            0
        }
    }
}

/// The last text handed out (`mb_now`'s, an error's), its length from `mb_text_len`.
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_text(w: *mut Web) -> *const u8 {
    unsafe { web(w) }.text.as_ptr()
}

/// What is playing, as JSON (its length from `mb_text_len`).
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_now(w: *mut Web) -> *const u8 {
    let w = unsafe { web(w) };
    let n = w.stream.now();
    w.text = format!(
        concat!(
            "{{\"id\":{},\"name\":\"{}\",\"piece\":{},\"bpm\":{:.2},\"key\":\"{}\",\"mode\":\"{}\",\"meter\":\"{}\",",
            "\"section\":\"{}\",\"bar\":{},\"bars\":{},\"stopped\":{},\"elapsed\":{:.3},\"length\":{:.1},",
            "\"planned\":{:.1},\"opts\":{},\"sid\":{},\"back\":{},\"fwd\":{}}}"
        ),
        n.id,
        music::name(n.id).to_lowercase(),
        n.piece,
        n.bpm,
        n.key,
        n.mode,
        n.meter,
        n.section,
        n.bar,
        n.bars,
        n.stopped,
        n.elapsed,
        n.length,
        n.planned,
        n.opts.code(),
        quoted(n.sid),
        quoted(n.back),
        quoted(n.fwd)
    );
    w.text.as_ptr()
}

/// A song's id as a JSON string, or `null`.
fn quoted(id: Option<music::SongId>) -> String {
    id.map_or_else(|| "null".into(), |id| format!("\"{id}\""))
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_text_len(w: *mut Web) -> u32 {
    unsafe { web(w) }.text.len() as u32
}

/// The genres' names, as JSON.
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_genres(w: *mut Web) -> *const u8 {
    let w = unsafe { web(w) };
    let names: Vec<String> = music::GENRES.iter().map(|g| format!("\"{}\"", g.to_lowercase())).collect();
    w.text = format!("[{}]", names.join(","));
    w.text.as_ptr()
}

/// How many genres there are (`mb_genres` names them).
#[unsafe(no_mangle)]
pub extern "C" fn mb_genre_count() -> u32 {
    music::GENRES.len() as u32
}

/// A mix's id: `primary` writing the music, `drums` (-1: none) drumming it.
#[unsafe(no_mangle)]
pub extern "C" fn mb_mix_id(primary: u32, drums: i32) -> u32 {
    music::mix_id(primary as usize, (drums >= 0).then_some(drums as usize)) as u32
}

/// The song whose id is at `p` (`n` bytes of text), alone, as `--export
/// ID` makes it, in the listener's effects (`lo`, `hi`: `Effects::halves`): a WAV file's bytes (their length from `mb_bytes_len`), and
/// the ID3v2 tag for its MP3 (made in `year`; `mb_tag`). Null when the id
/// is none.
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_song_wav(w: *mut Web, p: *const u8, n: u32, year: u32, lo: u32, hi: u32) -> *const u8 {
    let w = unsafe { web(w) };
    let Some(id) = (unsafe { song_id(p, n) }) else { return std::ptr::null() };
    let (wav, tags) = music::song_wav(&id, year as u64, music::Effects::from_halves(lo, hi));
    w.bytes = wav;
    w.tag = mp3::id3v2(&tags);
    w.bytes.as_ptr()
}

/// The tag `mb_song_wav` made (its length from `mb_tag_len`).
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_tag(w: *mut Web) -> *const u8 {
    unsafe { web(w) }.tag.as_ptr()
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_tag_len(w: *mut Web) -> u32 {
    unsafe { web(w) }.tag.len() as u32
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_bytes_len(w: *mut Web) -> u32 {
    unsafe { web(w) }.bytes.len() as u32
}

// ---------------------------------------------------------------- the window

/// What the page holds of its window: the window, what it shows besides
/// the music, what is playing (as the worker last said), the sound as the
/// speakers play it (for the equalizer), and what the actions ask of the
/// music, for the worker.
pub struct Ui {
    gui: gui::Gui,
    st: State,
    now: tap::Now,
    tap: tap::Tap,
    page: Page,
    started: bool,
    text: String,
}

/// The page's side of the music: each action's message for the worker
/// (JSON), and whether it plays.
struct Page {
    asks: RefCell<Vec<String>>,
    playing: Cell<bool>,
}

impl Page {
    fn ask(&self, s: String) {
        self.asks.borrow_mut().push(s);
    }
}

fn drums_json(d: Option<usize>) -> i64 {
    d.map_or(-1, |d| d as i64)
}

impl Deck for Page {
    fn skip(&self) {
        self.ask(r#"{"do":"skip"}"#.into());
    }
    fn replay(&self) {
        self.ask(r#"{"do":"replay"}"#.into());
    }
    fn playing(&self) -> bool {
        self.playing.get()
    }
    fn set_playing(&self, on: bool) {
        self.playing.set(on);
        self.ask(format!(r#"{{"do":"playing","on":{on}}}"#));
    }
    fn seek(&self, secs: f64) {
        self.ask(format!(r#"{{"do":"seek","secs":{secs}}}"#));
    }
    fn go(&self, id: music::SongId) {
        let (p, d) = music::parts(id.music);
        self.ask(format!(
            r#"{{"do":"go","id":"{id}","primary":{p},"drums":{},"opts":{},"length":{}}}"#,
            drums_json(d),
            id.opts.code(),
            id.length
        ));
    }
    fn set_genre(&self, id: usize) {
        let (p, d) = music::parts(id);
        self.ask(format!(r#"{{"do":"genre","primary":{p},"drums":{}}}"#, drums_json(d)));
    }
    fn set_length(&self, secs: f64) {
        self.ask(format!(r#"{{"do":"length","secs":{secs}}}"#));
    }
    fn set_volume(&self, volume: u32) {
        self.ask(format!(r#"{{"do":"volume","volume":{volume}}}"#));
    }
    fn set_effects(&self, e: music::Effects) {
        let (lo, hi) = e.halves();
        self.ask(format!(r#"{{"do":"effects","fx":[{lo},{hi}]}}"#));
    }
    fn set_opts(&self, opts: music::Opts) {
        self.ask(format!(r#"{{"do":"opts","code":{}}}"#, opts.code()));
    }
    fn export(&self) {
        self.ask(r#"{"do":"export"}"#.into());
    }
}

/// `s` as a JSON string.
fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            c if (c as u32) < 0x20 => o += &format!("\\u{:04x}", c as u32),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// The text at `p` (`n` bytes).
///
/// # Safety
/// `p` and `n` as `mb_alloc` gave and took them.
unsafe fn text_at(p: *const u8, n: u32) -> String {
    // SAFETY: the caller's
    String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(p, n as usize) }).into_owned()
}

/// # Safety
/// `u` is a handle from `mb_ui_new`, not freed.
unsafe fn ui<'a>(u: *mut Ui) -> &'a mut Ui {
    // SAFETY: the caller's
    unsafe { &mut *u }
}

/// Each name `Now` takes as `&'static str` (a meter, a section, a key, a
/// mode): kept once, the first time it comes.
fn kept(s: &str) -> &'static str {
    thread_local! {
        static KEPT: RefCell<std::collections::HashMap<String, &'static str>> = RefCell::default();
    }
    KEPT.with(|k| *k.borrow_mut().entry(s.to_string()).or_insert_with(|| Box::leak(s.to_string().into_boxed_str())))
}

/// What is playing, from the worker's `mb_now` (a flat JSON object: its
/// numbers, strings, true, false and null).
fn now_from(json: &str) -> tap::Now {
    let mut n = tap::Now::default();
    let b = json.as_bytes();
    let mut i = 0;
    while let Some(k0) = json[i..].find('"').map(|x| i + x + 1) {
        let Some(k1) = json[k0..].find('"').map(|x| k0 + x) else { break };
        let key = &json[k0..k1];
        let mut v0 = k1 + 1;
        while v0 < b.len() && (b[v0] == b':' || b[v0] == b' ') {
            v0 += 1;
        }
        let (val, end) = if b.get(v0) == Some(&b'"') {
            let e = json[v0 + 1..].find('"').map_or(b.len(), |x| v0 + 1 + x);
            (&json[v0 + 1..e], e + 1)
        } else {
            let e = json[v0..].find([',', '}']).map_or(b.len(), |x| v0 + x);
            (json[v0..e].trim(), e)
        };
        let num = || val.parse::<f64>().unwrap_or(0.0);
        let id = || if val == "null" { None } else { music::SongId::parse(val) };
        match key {
            "id" => n.id = num() as usize,
            "piece" => n.piece = num() as u64,
            "bpm" => n.bpm = num(),
            "key" => n.key = kept(val),
            "mode" => n.mode = kept(val),
            "meter" => n.meter = kept(val),
            "section" => n.section = kept(val),
            "bar" => n.bar = num() as u32,
            "bars" => n.bars = num() as u32,
            "stopped" => n.stopped = val == "true",
            "elapsed" => n.elapsed = num(),
            "length" => n.length = num(),
            "planned" => n.planned = num(),
            "opts" => n.opts = music::Opts::from_code(num() as u32),
            "sid" => n.sid = id(),
            "back" => n.back = id(),
            "fwd" => n.fwd = id(),
            _ => {}
        }
        i = end.min(b.len());
    }
    n
}

/// The window, opened on the page: playing `primary` (with `drums`, -1
/// none) from `seed` (its low and high 32 bits), made as `opts` asks
/// (`Opts::code`), each song `length` seconds (0: its own), at `volume`
/// per cent, in the effects `lo`, `hi` (`Effects::halves`); the favorites
/// as the page's storage keeps them (`fav_n` bytes at `fav`).
///
/// # Safety
/// `fav` and `fav_n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn mb_ui_new(
    primary: u32,
    drums: i32,
    seed_lo: u32,
    seed_hi: u32,
    opts: u32,
    length: f64,
    volume: u32,
    lo: u32,
    hi: u32,
    fav: *const u8,
    fav_n: u32,
) -> *mut Ui {
    favorites::hand_in(unsafe { text_at(fav, fav_n) });
    let gui = match gui::Gui::open() {
        Ok(g) => g,
        Err(_) => return std::ptr::null_mut(),
    };
    let ng = music::GENRES.len();
    let st = State {
        primary: primary as usize % ng,
        drums: (drums >= 0 && drums as u32 != primary).then_some(drums as usize % ng),
        seed: seed(seed_lo, seed_hi),
        sampled: true,
        opts: music::Opts::from_code(opts),
        eq: false,
        length,
        volume: volume.min(100),
        effects: music::Effects::from_halves(lo, hi),
        favorites: favorites::Favorites::load(),
        msg: None,
        note: None,
    };
    let page = Page { asks: RefCell::new(Vec::new()), playing: Cell::new(true) };
    let u = Ui {
        gui,
        st,
        now: tap::Now::default(),
        tap: tap::Tap::new(48000, 0),
        page,
        started: false,
        text: String::new(),
    };
    Box::into_raw(Box::new(u))
}

/// A key pressed: SDL's key code (a letter lower case, a symbol as itself),
/// Ctrl held with it or not.
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_key(code: i32, ctrl: u32) {
    sdl::page::key(code, ctrl != 0);
}

/// The text a key typed (`n` bytes at `p`).
///
/// # Safety
/// `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_text(p: *const u8, n: u32) {
    sdl::page::text(&unsafe { text_at(p, n) });
}

/// The mouse at (x, y), in the page's pixels on the canvas.
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_motion(x: i32, y: i32) {
    sdl::page::motion(x, y);
}

/// A mouse button (1 left, 3 right) down (1) or up (0) at (x, y).
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_button(down: u32, x: i32, y: i32, button: u32) {
    sdl::page::button(down != 0, x, y, button as u8);
}

/// The wheel turned (positive away from the user).
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_wheel(by: i32) {
    sdl::page::wheel(by);
}

/// The mouse gone from the canvas.
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_leave() {
    sdl::page::leave();
}

/// The network instruments the page has to fetch when they are chosen.
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_nets(n: u32) {
    net::listed(n as usize);
}

/// The canvas's size: in the screen's pixels, and in the page's.
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_resize(ow: i32, oh: i32, css_w: f32, css_h: f32) {
    sdl::page::resize(ow, oh, css_w, css_h);
}

/// Text pasted (`n` bytes at `p`): into a field being typed, else played
/// when it holds a song's id; as Ctrl-V does.
///
/// # Safety
/// `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_paste(p: *const u8, n: u32) {
    sdl::page::pasted(&unsafe { text_at(p, n) });
    sdl::page::key('v' as i32, true);
}

/// What is playing: the worker's `mb_now` (`n` bytes at `p`).
///
/// # Safety
/// `u` is a handle from `mb_ui_new`; `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_now(u: *mut Ui, p: *const u8, n: u32) {
    let u = unsafe { ui(u) };
    u.now = now_from(&unsafe { text_at(p, n) });
}

/// The sound the speakers play, its last `n` frames (mono) at `p`.
///
/// # Safety
/// `u` is a handle from `mb_ui_new`; `p` and `n` as `mb_alloc` gave and took
/// them (`n` numbers).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_tap(u: *mut Ui, p: *const f32, n: u32) {
    let u = unsafe { ui(u) };
    // SAFETY: the caller's
    let x = unsafe { std::slice::from_raw_parts(p, n as usize) };
    let both: Vec<f32> = x.iter().flat_map(|&s| [s, s]).collect();
    u.tap.push(&both, tap::Now::default);
}

/// A word for the window's line (`n` bytes at `p`): a note (`kind` 0, gone
/// in a few seconds), or what went wrong (1).
///
/// # Safety
/// `u` is a handle from `mb_ui_new`; `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_say(u: *mut Ui, kind: u32, p: *const u8, n: u32) {
    let u = unsafe { ui(u) };
    let t = (unsafe { text_at(p, n) }, Instant::now());
    if kind == 0 { u.st.note = Some(t) } else { u.st.msg = Some(t) }
}

/// The music begun (the page's first key or click): the line stops asking for one.
///
/// # Safety
/// `u` is a handle from `mb_ui_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_started(u: *mut Ui) {
    let u = unsafe { ui(u) };
    u.started = true;
    u.st.msg = None;
}

/// How the music is made after all (`Opts::code`; the recordings would not
/// load: the synthesiser's), and whether there are recordings.
///
/// # Safety
/// `u` is a handle from `mb_ui_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_made(u: *mut Ui, opts: u32, sampled: u32) {
    let u = unsafe { ui(u) };
    u.st.opts = music::Opts::from_code(opts);
    u.st.sampled = sampled != 0;
}

/// The song in `n` bytes of text at `p` (an id, a link to the page playing
/// one), played: 1, else 0 (and the window says it is none).
///
/// # Safety
/// `u` is a handle from `mb_ui_new`; `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_go(u: *mut Ui, p: *const u8, n: u32) -> u32 {
    let u = unsafe { ui(u) };
    let text = unsafe { text_at(p, n) };
    match favorites::song_in(&text) {
        Some(id) => {
            go_to(&u.page, &mut u.st, id);
            1
        }
        None => {
            u.st.msg = Some((format!("not a song's id: {text}"), Instant::now()));
            0
        }
    }
}

/// What the music is to be when it begins (JSON): the genre and drummer,
/// the seed, how it is made, the length, the volume and the effects.
///
/// # Safety
/// `u` is a handle from `mb_ui_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_state(u: *mut Ui) -> *const u8 {
    let u = unsafe { ui(u) };
    let st = &u.st;
    let (lo, hi) = st.effects.halves();
    u.text = format!(
        r#"{{"primary":{},"drums":{},"seed":"{}","opts":{},"length":{},"volume":{},"fx":[{lo},{hi}],"playing":{}}}"#,
        st.primary,
        drums_json(st.drums),
        st.seed,
        st.opts.code(),
        st.length,
        st.volume,
        u.page.playing.get()
    );
    u.text.as_ptr()
}

/// A frame: the keys and clicks since the last one acted on, the window
/// drawn (`mb_ui_picture`), and what it asks of the page since (a JSON list:
/// the worker's messages, the clipboard, the title, fullscreen, the
/// favorites to store).
///
/// # Safety
/// `u` is a handle from `mb_ui_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_frame(u: *mut Ui) -> *const u8 {
    let u = unsafe { ui(u) };
    let Ui { gui, st, now, tap, page, started, .. } = u;
    if !*started {
        st.msg = Some(("CLICK OR PRESS A KEY TO START THE MUSIC".into(), Instant::now()));
    }
    for a in gui.events() {
        match act(a, now, st, page) {
            Some(Action::Copy) => match now.sid {
                Some(id) => {
                    let text = id.to_string();
                    gui.copy(&text);
                    st.note = Some((format!("copied {text}"), Instant::now()));
                }
                None => st.msg = Some(("the song has not begun".into(), Instant::now())),
            },
            // (pasted: Ctrl-V, or the clipboard just read; else the page reads it)
            Some(Action::Paste) if sdl::page::fresh() => match favorites::song_in(&gui.pasted()) {
                Some(id) => go_to(page, st, id),
                None => st.msg = Some(("no song's id on the clipboard".into(), Instant::now())),
            },
            Some(Action::Paste) => page.ask(r#"{"do":"paste"}"#.into()),
            Some(Action::Fullscreen) => gui.fullscreen(),
            Some(a @ (Action::Keys | Action::EffectsPanel | Action::FavoritesPanel)) => gui.toggle(a),
            _ => {}
        }
    }
    if st.note.as_ref().is_some_and(|m| m.1.elapsed() > std::time::Duration::from_secs(8)) {
        st.note = None;
    }
    if st.msg.as_ref().is_some_and(|m| m.1.elapsed() > std::time::Duration::from_secs(5)) {
        st.msg = None;
    }
    gui.draw(tap, now, st);
    let mut asks = std::mem::take(&mut *page.asks.borrow_mut());
    if let Some(f) = favorites::to_store() {
        asks.push(format!(r#"{{"do":"favorites","text":{}}}"#, json_str(&f)));
    }
    for a in sdl::page::asks() {
        asks.push(match a {
            sdl::page::Ask::Title(t) => format!(r#"{{"do":"title","text":{}}}"#, json_str(&t)),
            sdl::page::Ask::Copy(t) => format!(r#"{{"do":"copy","text":{}}}"#, json_str(&t)),
            sdl::page::Ask::Fullscreen => r#"{"do":"fullscreen"}"#.into(),
        });
    }
    u.text = format!("[{}]", asks.join(","));
    u.text.as_ptr()
}

/// The length of the text `mb_ui_frame` or `mb_ui_state` gave.
///
/// # Safety
/// `u` is a handle from `mb_ui_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_ui_text_len(u: *mut Ui) -> u32 {
    unsafe { ui(u) }.text.len() as u32
}

/// The picture the last frame drew (RGBA, `mb_ui_width` x `mb_ui_height`).
#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_picture() -> *const u8 {
    sdl::page::shown(|px, _, _| px.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_width() -> u32 {
    sdl::page::shown(|_, w, _| w)
}

#[unsafe(no_mangle)]
pub extern "C" fn mb_ui_height() -> u32 {
    sdl::page::shown(|_, _, h| h)
}
