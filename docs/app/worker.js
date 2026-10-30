// The music, made ahead of the speakers: musicbox's engine (WebAssembly)
// on a thread of its own, handing the audio worklet chunks as it asks for
// them, each with what is playing as it starts. A second one renders a song
// on its own for a download.

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

onmessage = async (e) => {
  const m = e.data;
  switch (m.type) {
    case 'init': {
      x = (await WebAssembly.instantiate(m.module, {})).exports;
      const [lo, hi] = seedParts(m.seed);
      w = x.mb_new(m.genre, lo, hi);
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
    case 'export': {
      const [lo, hi] = seedParts(m.seed);
      const p = x.mb_song_wav(w, m.id, lo, hi, m.song, m.length);
      const bytes = new Uint8Array(x.memory.buffer, p, x.mb_bytes_len(w)).slice();
      postMessage({ type: 'wav', bytes, name: m.name }, [bytes.buffer]);
      break;
    }
  }
};
