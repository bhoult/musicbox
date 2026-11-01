// The music, made ahead of the speakers: musicbox's engine (WebAssembly)
// on a thread of its own, handing the audio worklet chunks as it asks for
// them, each with what is playing as it starts. A second one renders a song
// on its own for a download. The recordings come from the page, decoded
// (the tables, then each group a genre plays).

const CHUNK = 2048;
let x = null;
let w = 0;
let port = null;

function text(p) {
  return new TextDecoder().decode(new Uint8Array(x.memory.buffer, p, x.mb_text_len(w)));
}

function seedParts(seed) {
  const s = BigInt(seed);
  return [Number(s & 0xffffffffn), Number((s >> 32n) & 0xffffffffn)];
}

// `frames` more, in chunks
function produce(frames) {
  for (let made = 0; made < frames; made += CHUNK) {
    const p = x.mb_render(w, CHUNK);
    const pcm = new Float32Array(x.memory.buffer, p, 2 * CHUNK).slice();
    const now = text(x.mb_now(w));
    port.postMessage({ pcm, now }, [pcm.buffer]);
  }
}

// into the module's memory, for `f(ptr, length)`, and out again
function put(array, f) {
  const n = array.byteLength;
  const p = x.mb_alloc(n);
  new array.constructor(x.memory.buffer, p, array.length).set(array);
  const r = f(p, array.length);
  x.mb_free(p, n);
  return r;
}

// LAME (mp3/: see its README), loaded from this site the first time a song
// is exported
let encoder = null;
async function lame() {
  if (!encoder) {
    importScripts(`mp3/WasmMediaEncoder.min.js${location.search}`);
    const r = await fetch(`mp3/mp3.wasm${location.search}`);
    if (!r.ok) throw new Error(`mp3.wasm: ${r.status}`);
    encoder = await WasmMediaEncoder.createEncoder('audio/mpeg', await r.arrayBuffer());
  }
  return encoder;
}

// a WAV file's 16-bit stereo at 48 kHz as MP3 frames (192 kb/s: this LAME
// writes no info frame, which a VBR file needs for players to tell its
// length), a second at a time
function mp3(enc, wav) {
  const pcm = new Int16Array(wav.buffer, 44, (wav.byteLength - 44) >> 1);
  const frames = pcm.length >> 1;
  const step = 48000;
  const [l, r] = [new Float32Array(step), new Float32Array(step)];
  const out = [];
  enc.configure({ sampleRate: 48000, channels: 2, bitrate: 192 });
  for (let at = 0; at < frames; at += step) {
    const n = Math.min(step, frames - at);
    // (LAME takes ±1 as ±32767: the samples the desktop's LAME is given)
    for (let i = 0; i < n; i++) {
      l[i] = pcm[2 * (at + i)] / 32767;
      r[i] = pcm[2 * (at + i) + 1] / 32767;
    }
    // (each a view of the encoder's memory, copied before it is used again)
    out.push(enc.encode([l.subarray(0, n), r.subarray(0, n)]).slice());
  }
  out.push(enc.finalize().slice());
  return out;
}

// one message at a time, in order (the first instantiates the module)
let queue = Promise.resolve();
onmessage = (e) => {
  queue = queue.then(() => handle(e.data)).catch((err) => postMessage({ type: 'error', error: String(err) }));
};

async function handle(m) {
  switch (m.type) {
    case 'init': {
      x = (await WebAssembly.instantiate(m.module, {})).exports;
      const [lo, hi] = seedParts(m.seed);
      // (no genre: the seed's, among them all, as the desktop picks one)
      const genre = m.genre ?? m.seed % x.mb_genre_count();
      w = x.mb_new(genre, lo, hi, m.opts ?? 0);
      if (m.paused) x.mb_set_playing(w, 0);
      postMessage({ type: 'genres', genres: JSON.parse(text(x.mb_genres(w))), genre });
      if (m.port) {
        port = m.port;
        port.onmessage = (ev) => produce(ev.data.need);
      }
      break;
    }
    case 'genre':
      x.mb_set_genre(w, x.mb_mix_id(m.primary, m.drums ?? -1));
      break;
    case 'skip':
      x.mb_skip(w);
      break;
    case 'replay':
      x.mb_replay(w);
      break;
    case 'playing':
      x.mb_set_playing(w, m.on ? 1 : 0);
      break;
    case 'length':
      x.mb_set_length(w, m.secs);
      break;
    case 'opts':
      x.mb_set_opts(w, m.code);
      break;
    // a song's id read (for the page to fetch what it needs), then played
    case 'parse': {
      const id = put(new TextEncoder().encode(m.text), (p, n) => JSON.parse(text(x.mb_song_id(w, p, n))));
      postMessage({ type: 'id', id, text: m.text });
      break;
    }
    case 'go':
      put(new TextEncoder().encode(m.text), (p, n) => x.mb_go(w, p, n));
      break;
    case 'seek':
      x.mb_seek(w, m.secs);
      break;
    // a song by its id, alone: an MP3, or a WAV when asked (or when the
    // encoder will not load)
    case 'export': {
      const p = put(new TextEncoder().encode(m.sid), (q, n) => x.mb_song_wav(w, q, n, m.year));
      if (!p) {
        postMessage({ type: 'error', error: `no song ${m.sid}` });
        break;
      }
      const wav = new Uint8Array(x.memory.buffer, p, x.mb_bytes_len(w)).slice();
      const tag = new Uint8Array(x.memory.buffer, x.mb_tag(w), x.mb_tag_len(w)).slice();
      let [enc, why] = [null, ''];
      if (!m.wav) {
        postMessage({ type: 'say', text: `encoding ${m.sid}.mp3…` });
        enc = await lame().catch((err) => {
          why = `no MP3 encoder: ${err.message}`;
          return null;
        });
      }
      const [blob, name] = enc
        ? [new Blob([tag, ...mp3(enc, wav)], { type: 'audio/mpeg' }), `${m.sid}.mp3`]
        : [new Blob([wav], { type: 'audio/wav' }), `${m.sid}.wav`];
      postMessage({ type: 'saved', blob, name, why });
      break;
    }
    case 'pack': {
      const ok = put(new Uint8Array(m.meta), (p, n) => x.mb_pack(w, p, n));
      if (!ok) postMessage({ type: 'error', error: `recordings: ${text(x.mb_text(w))}` });
      break;
    }
    case 'net': {
      const line = new TextEncoder().encode(m.line);
      const ok = put(line, (lp, ln) => put(new Uint8Array(m.bytes), (p, n) => x.mb_net(w, lp, ln, p, n)));
      if (!ok) postMessage({ type: 'error', error: `network: ${text(x.mb_text(w))}` });
      break;
    }
    case 'tunes': {
      const ok = put(new Uint8Array(m.bytes), (p, n) => x.mb_tunes(w, m.genre, p, n));
      if (!ok) postMessage({ type: 'error', error: `tunes: ${text(x.mb_text(w))}` });
      break;
    }
    case 'group': {
      const ok = put(m.pcm, (p, n) => x.mb_group(w, m.g, p, n));
      if (!ok) postMessage({ type: 'error', error: `recordings: ${text(x.mb_text(w))}` });
      break;
    }
  }
}
