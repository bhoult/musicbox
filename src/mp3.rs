//! MP3s, made by LAME's library loaded when one is first wanted (so that
//! musicbox runs without it and only exporting needs it): VBR at quality 2
//! (about 190 kbps), with an ID3v2 tag and LAME's info frame (for players to
//! tell the length).

use std::ffi::{CString, c_char, c_float, c_int, c_uchar, c_void};

const RTLD_NOW: c_int = 2;
/// LAME's vbr_mtrh, its default VBR mode.
const VBR_MTRH: c_int = 4;

unsafe extern "C" {
    fn dlopen(file: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
}

/// What an MP3's tag says.
pub struct Tags {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub comment: String,
}

type Gfp = *mut c_void;

/// LAME's calls, looked up by name.
struct Lame {
    init: unsafe extern "C" fn() -> Gfp,
    set_in_samplerate: unsafe extern "C" fn(Gfp, c_int) -> c_int,
    set_num_channels: unsafe extern "C" fn(Gfp, c_int) -> c_int,
    set_vbr: unsafe extern "C" fn(Gfp, c_int) -> c_int,
    set_vbr_quality: unsafe extern "C" fn(Gfp, c_float) -> c_int,
    set_quality: unsafe extern "C" fn(Gfp, c_int) -> c_int,
    init_params: unsafe extern "C" fn(Gfp) -> c_int,
    encode: unsafe extern "C" fn(Gfp, *const i16, c_int, *mut c_uchar, c_int) -> c_int,
    flush: unsafe extern "C" fn(Gfp, *mut c_uchar, c_int) -> c_int,
    lametag: unsafe extern "C" fn(Gfp, *mut c_uchar, usize) -> usize,
    close: unsafe extern "C" fn(Gfp) -> c_int,
    id3_init: unsafe extern "C" fn(Gfp),
    id3_v2_only: unsafe extern "C" fn(Gfp),
    id3_title: unsafe extern "C" fn(Gfp, *const c_char),
    id3_artist: unsafe extern "C" fn(Gfp, *const c_char),
    id3_album: unsafe extern "C" fn(Gfp, *const c_char),
    id3_year: unsafe extern "C" fn(Gfp, *const c_char),
    id3_comment: unsafe extern "C" fn(Gfp, *const c_char),
}

impl Lame {
    fn load() -> Result<Lame, String> {
        let names = ["libmp3lame.so.0", "libmp3lame.so"];
        let lib = names
            .iter()
            .map(|n| {
                let c = CString::new(*n).expect("no NUL");
                // SAFETY: a library name, NUL-terminated; the handle is never closed
                unsafe { dlopen(c.as_ptr(), RTLD_NOW) }
            })
            .find(|h| !h.is_null())
            .ok_or("MP3s need LAME's library (libmp3lame0 on Debian and Ubuntu, lame elsewhere)")?;
        let sym = |name: &str| -> Result<*mut c_void, String> {
            let c = CString::new(name).expect("no NUL");
            // SAFETY: a handle dlopen gave, and a NUL-terminated name
            let p = unsafe { dlsym(lib, c.as_ptr()) };
            if p.is_null() { Err(format!("LAME's library has no {name}")) } else { Ok(p) }
        };
        // SAFETY: each symbol is LAME's function of that name, with the
        // signature lame.h gives it
        unsafe {
            Ok(Lame {
                init: std::mem::transmute::<*mut c_void, _>(sym("lame_init")?),
                set_in_samplerate: std::mem::transmute::<*mut c_void, _>(sym("lame_set_in_samplerate")?),
                set_num_channels: std::mem::transmute::<*mut c_void, _>(sym("lame_set_num_channels")?),
                set_vbr: std::mem::transmute::<*mut c_void, _>(sym("lame_set_VBR")?),
                set_vbr_quality: std::mem::transmute::<*mut c_void, _>(sym("lame_set_VBR_quality")?),
                set_quality: std::mem::transmute::<*mut c_void, _>(sym("lame_set_quality")?),
                init_params: std::mem::transmute::<*mut c_void, _>(sym("lame_init_params")?),
                encode: std::mem::transmute::<*mut c_void, _>(sym("lame_encode_buffer_interleaved")?),
                flush: std::mem::transmute::<*mut c_void, _>(sym("lame_encode_flush")?),
                lametag: std::mem::transmute::<*mut c_void, _>(sym("lame_get_lametag_frame")?),
                close: std::mem::transmute::<*mut c_void, _>(sym("lame_close")?),
                id3_init: std::mem::transmute::<*mut c_void, _>(sym("id3tag_init")?),
                id3_v2_only: std::mem::transmute::<*mut c_void, _>(sym("id3tag_v2_only")?),
                id3_title: std::mem::transmute::<*mut c_void, _>(sym("id3tag_set_title")?),
                id3_artist: std::mem::transmute::<*mut c_void, _>(sym("id3tag_set_artist")?),
                id3_album: std::mem::transmute::<*mut c_void, _>(sym("id3tag_set_album")?),
                id3_year: std::mem::transmute::<*mut c_void, _>(sym("id3tag_set_year")?),
                id3_comment: std::mem::transmute::<*mut c_void, _>(sym("id3tag_set_comment")?),
            })
        }
    }
}

/// `pcm` (interleaved stereo at `rate`) as an MP3 file's bytes.
pub fn encode(pcm: &[i16], rate: u32, tags: &Tags) -> Result<Vec<u8>, String> {
    let lame = Lame::load()?;
    let text = |s: &str| CString::new(s.replace('\0', "")).expect("no NUL");
    let (title, artist, album, year, comment) =
        (text(&tags.title), text(&tags.artist), text(&tags.album), text(&tags.year), text(&tags.comment));
    // SAFETY: LAME's calls as lame.h gives them, on a handle from lame_init
    // closed once at the end; the buffers outlive each call and are as large
    // as the sizes passed
    unsafe {
        let gfp = (lame.init)();
        if gfp.is_null() {
            return Err("LAME would not start".into());
        }
        (lame.set_in_samplerate)(gfp, rate as c_int);
        (lame.set_num_channels)(gfp, 2);
        (lame.set_vbr)(gfp, VBR_MTRH);
        (lame.set_vbr_quality)(gfp, 2.0);
        (lame.set_quality)(gfp, 2);
        (lame.id3_init)(gfp);
        (lame.id3_v2_only)(gfp);
        (lame.id3_title)(gfp, title.as_ptr());
        (lame.id3_artist)(gfp, artist.as_ptr());
        (lame.id3_album)(gfp, album.as_ptr());
        (lame.id3_year)(gfp, year.as_ptr());
        (lame.id3_comment)(gfp, comment.as_ptr());
        if (lame.init_params)(gfp) < 0 {
            (lame.close)(gfp);
            return Err("LAME would not take the settings".into());
        }
        let mut out = Vec::with_capacity(pcm.len() / 4);
        // LAME's rule for the buffer: 1.25 a sample and 7200 bytes
        const FRAMES: usize = 8192;
        let mut buf = vec![0u8; FRAMES * 5 / 4 + 7200];
        for chunk in pcm.chunks(2 * FRAMES) {
            let n =
                (lame.encode)(gfp, chunk.as_ptr(), (chunk.len() / 2) as c_int, buf.as_mut_ptr(), buf.len() as c_int);
            if n < 0 {
                (lame.close)(gfp);
                return Err(format!("LAME failed ({n})"));
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        let n = (lame.flush)(gfp, buf.as_mut_ptr(), buf.len() as c_int);
        if n > 0 {
            out.extend_from_slice(&buf[..n as usize]);
        }
        // the info frame, over the empty one LAME left after the tag
        let n = (lame.lametag)(gfp, buf.as_mut_ptr(), buf.len());
        let at = id3v2_len(&out);
        if n > 0 && n <= buf.len() && at + n <= out.len() {
            out[at..at + n].copy_from_slice(&buf[..n]);
        }
        (lame.close)(gfp);
        Ok(out)
    }
}

/// The bytes an ID3v2 tag at the start of `b` takes (0 when there is none).
fn id3v2_len(b: &[u8]) -> usize {
    if b.len() < 10 || &b[..3] != b"ID3" {
        return 0;
    }
    let size = b[6..10].iter().fold(0usize, |a, &x| a << 7 | (x & 0x7F) as usize);
    10 + size + if b[5] & 0x10 != 0 { 10 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_encodes_with_its_tag_and_info_frame() {
        let rate = 48000;
        let pcm: Vec<i16> = (0..rate as usize * 2)
            .flat_map(|i| {
                let x = ((i as f32 * 440.0 / rate as f32) * std::f32::consts::TAU).sin() * 8000.0;
                [x as i16, x as i16]
            })
            .collect();
        let tags = Tags {
            title: "a sine".into(),
            artist: "musicbox".into(),
            album: "tests".into(),
            year: "2026".into(),
            comment: "440 Hz".into(),
        };
        let Ok(mp3) = encode(&pcm, rate, &tags) else { return eprintln!("no LAME here: skipped") };
        let at = id3v2_len(&mp3);
        assert!(at > 10, "an ID3v2 tag");
        // an MPEG frame's sync after the tag, holding LAME's info ("Info" or "Xing")
        assert_eq!(mp3[at], 0xFF);
        assert_eq!(mp3[at + 1] & 0xE0, 0xE0);
        let frame = &mp3[at..at + 200];
        assert!(frame.windows(4).any(|w| w == b"Xing" || w == b"Info"));
        // two seconds of a sine: VBR's least to its most
        assert!(mp3.len() > 8_000 && mp3.len() < 120_000, "{} bytes", mp3.len());
    }
}
