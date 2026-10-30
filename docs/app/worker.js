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
      w = x.mb_new(m.genre, lo, hi, m.opts ?? 0);
      postMessage({ type: 'genres', genres: JSON.parse(text(x.mb_genres(w))) });
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
    case 'export': {
      const [lo, hi] = seedParts(m.seed);
      const p = x.mb_song_wav(w, m.id, lo, hi, m.song, m.length, m.opts ?? 0);
      const bytes = new Uint8Array(x.memory.buffer, p, x.mb_bytes_len(w)).slice();
      postMessage({ type: 'wav', bytes, name: m.name }, [bytes.buffer]);
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
