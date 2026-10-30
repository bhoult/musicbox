//! musicbox in the browser: the composer and synthesiser (src/music.rs and
//! what it uses, unchanged) built as WebAssembly, for docs/app to drive. No
//! crates and no bindings generator: plain exported functions over the
//! module's memory, a handle (`Web`) passed back and forth as a pointer.
//!
//!     cargo build --release --target wasm32-unknown-unknown --manifest-path web/Cargo.toml
//!
//! The recorded instruments are not here (a gigabyte): the synthesiser plays
//! everything, as `--no-samples` has it.

#![allow(dead_code)]

#[path = "../src/mp3.rs"]
mod mp3;
#[path = "../src/music.rs"]
mod music;
#[path = "../src/samples.rs"]
mod samples;
#[path = "../src/sdl.rs"]
mod sdl;
#[path = "../src/tap.rs"]
mod tap;

use music::Stream;

/// Seconds as M:SS (the app's, for music.rs).
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// What JavaScript holds: the music, the buffer it is made into, and the
/// last text or bytes handed out.
pub struct Web {
    stream: Stream,
    buf: Vec<f32>,
    text: String,
    bytes: Vec<u8>,
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

/// The music `genre` (a mix id) from a seed (its low and high 32 bits).
#[unsafe(no_mangle)]
pub extern "C" fn mb_new(genre: u32, seed_lo: u32, seed_hi: u32) -> *mut Web {
    let stream = Stream::new(genre as usize, seed(seed_lo, seed_hi));
    Box::into_raw(Box::new(Web { stream, buf: Vec::new(), text: String::new(), bytes: Vec::new() }))
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
            "\"planned\":{:.1},\"seed\":\"{}\",\"song\":{}}}"
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
        n.seed,
        n.song
    );
    w.text.as_ptr()
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

/// A mix's id: `primary` writing the music, `drums` (-1: none) drumming it.
#[unsafe(no_mangle)]
pub extern "C" fn mb_mix_id(primary: u32, drums: i32) -> u32 {
    music::mix_id(primary as usize, (drums >= 0).then_some(drums as usize)) as u32
}

/// The `song`th piece of `genre` from an engine seeded so (as `mb_now`
/// gives them), alone, as a WAV file's bytes (their length from
/// `mb_bytes_len`).
///
/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_song_wav(
    w: *mut Web,
    genre: u32,
    seed_lo: u32,
    seed_hi: u32,
    song: u32,
    length: f64,
) -> *const u8 {
    let w = unsafe { web(w) };
    w.bytes = music::song_wav(genre as usize, seed(seed_lo, seed_hi), song as usize, length);
    w.bytes.as_ptr()
}

/// # Safety
/// `w` is a handle from `mb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_bytes_len(w: *mut Web) -> u32 {
    unsafe { web(w) }.bytes.len() as u32
}
