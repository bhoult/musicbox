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

#![allow(dead_code)]

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
#[path = "../src/tap.rs"]
mod tap;
#[path = "../src/tunes.rs"]
mod tunes;

use music::Stream;

/// Seconds as M:SS (the app's, for music.rs).
pub fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

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

/// Group `g`'s recordings (`frames` 16-bit samples at `p`, mono): its
/// instruments playable from the next song made. 1, else 0 and why in `mb_text`.
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` holds `frames` samples, aligned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_group(w: *mut Web, g: u32, p: *const i16, frames: u32) -> u32 {
    let w = unsafe { web(w) };
    // SAFETY: the caller's
    let pcm = unsafe { std::slice::from_raw_parts(p, frames as usize) };
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

/// A mix's id: `primary` writing the music, `drums` (-1: none) drumming it.
#[unsafe(no_mangle)]
pub extern "C" fn mb_mix_id(primary: u32, drums: i32) -> u32 {
    music::mix_id(primary as usize, (drums >= 0).then_some(drums as usize)) as u32
}

/// The song whose id is at `p` (`n` bytes of text), alone, as `--export
/// ID` makes it: a WAV file's bytes (their length from `mb_bytes_len`), and
/// the ID3v2 tag for its MP3 (made in `year`; `mb_tag`). Null when the id
/// is none.
///
/// # Safety
/// `w` is a handle from `mb_new`; `p` and `n` as `mb_alloc` gave and took them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mb_song_wav(w: *mut Web, p: *const u8, n: u32, year: u32) -> *const u8 {
    let w = unsafe { web(w) };
    let Some(id) = (unsafe { song_id(p, n) }) else { return std::ptr::null() };
    let (wav, tags) = music::song_wav(&id, year as u64);
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
