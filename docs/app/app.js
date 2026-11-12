// musicbox in the browser: the page's side. The window is the desktop's
// (src/gui.rs), drawn by the module on this thread into the canvas, pixel
// for pixel as the desktop draws it; this hands it the keys, the mouse, what
// is playing and the sound, and does what its actions ask: the engine runs
// in a worker (worker.js), the audio worklet (worklet.js) plays what it
// makes, and the recordings, the downloads, the clipboard and what this
// browser keeps are this page's.

const $ = (id) => document.getElementById(id);
const canvas = $('window');
const g2d = canvas.getContext('2d');

// where the recordings are (tools/web-samples.sh makes them), and their
// lighter copy (one channel, half the rate: a quarter of the memory or less)
const SAMPLES = 'samples/';
const LITE_RATE = 24000;
// the stamps tools/stamp-page.sh puts on this script's address, one for the
// page's files and one for the recordings: each file is asked for with its
// stamp, so an older copy a browser kept of one is never used with the rest
const STAMPS = new URL(document.currentScript.src).searchParams;
const V = `?v=${STAMPS.get('v') ?? ''}`;
const R = `?r=${STAMPS.get('r') ?? ''}`;
// the module's one import: the page's clock, for the window's
const IMPORTS = { env: { now_ms: () => performance.now() } };
// the equalizer's window (spectrum.rs's N)
const TAP = 4096;
const VOICES = ['synth', 'recorded', 'network'];
// how the music is made, as one number (the engine's Opts::code): what
// plays the instruments is its bits 2 and 3, the rest its other settings
const voicesOf = (code) => VOICES[(code >> 2) & 3] ?? 'recorded';
const onSynth = (code) => code & ~12;

let module = null;
let ui = null; // the window's module
let u = 0; // its handle
let worker = null;
let exporter = null;
let audio = null;
let analyser = null;
let gain = null;
// what is playing (the worker's, as its chunk starts), and the song's id
// the address has
const st = { now: null, sid: null, genres: [] };
// the music as the window last asked for it: its genre and drummer and how
// it is made (what to fetch for it)
const cur = { primary: 0, drums: null, opts: 0 };
// the music begun (the first key or click), and the worker ready for messages
let started = false;
let ready = null;
let isReady = null;
// a song asked for (the address's, or played) before the music began
let goAsked = false;

// ---- what this browser keeps: the volume, the effects, the favorites

function kept(k) {
  try {
    return localStorage.getItem(k);
  } catch {
    return null;
  }
}

function keep(k, v) {
  try {
    localStorage.setItem(k, v);
  } catch {}
}

function savedVolume() {
  const v = parseInt(kept('musicbox.volume') ?? '', 10);
  return Number.isFinite(v) ? Math.min(100, Math.max(0, v)) : 100;
}

// the effects as the engine's Effects::halves has them (three to a number,
// ten bits each); kept as they were, one a key
const FX = ['reverb', 'echo', 'chorus', 'width', 'bass', 'treble'];

function savedEffects() {
  const f = FX.map((k) => {
    const v = parseInt(kept(`musicbox.${k}`) ?? '', 10);
    return Number.isFinite(v) ? Math.min(500, Math.max(0, v)) : 100;
  });
  return [0, 3].map((at) => f.slice(at, at + 3).reduce((c, v, i) => c | (v << (10 * i)), 0));
}

function keepEffects([lo, hi]) {
  FX.forEach((k, i) => keep(`musicbox.${k}`, String(((i < 3 ? lo : hi) >> (10 * (i % 3))) & 1023)));
}

// the favorites, one id a line (as the desktop's file has them); the list
// an earlier page kept (JSON) taken over
function savedFavorites() {
  const t = kept('musicbox.favorites.txt');
  if (t !== null) return t;
  try {
    const v = JSON.parse(kept('musicbox.favorites') ?? '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string').map((x) => `${x}\n`).join('') : '';
  } catch {
    return '';
  }
}

// ---- the window's module: text in and out of its memory

function readText(p, n) {
  return new TextDecoder().decode(new Uint8Array(ui.memory.buffer, p, n));
}

function withText(text, f) {
  const b = new TextEncoder().encode(text);
  const p = ui.mb_alloc(b.length);
  new Uint8Array(ui.memory.buffer, p, b.length).set(b);
  const r = f(p, b.length);
  ui.mb_free(p, b.length);
  return r;
}

function say(text, kind = 'note') {
  if (u) withText(text, (p, n) => ui.mb_ui_say(u, kind === 'error' ? 1 : 0, p, n));
}

function uiState() {
  const p = ui.mb_ui_state(u);
  return JSON.parse(readText(p, ui.mb_ui_text_len(u)));
}

// ---- the canvas: its size, the keys, the mouse and the fingers

function resize() {
  const dpr = window.devicePixelRatio || 1;
  const [w, h] = [canvas.clientWidth, canvas.clientHeight];
  ui?.mb_ui_resize(Math.round(w * dpr), Math.round(h * dpr), w, h);
}

// the device: whether a finger works it rather than a mouse (the window
// then lays itself out for one: ?touch asks for that anywhere, ?desk for
// the desk's on a phone), and whether the page can fill the screen
const ASKED = new URLSearchParams(location.search);
const coarse = matchMedia('(pointer: coarse)');

function device() {
  const touch = ASKED.has('touch') || (coarse.matches && !ASKED.has('desk'));
  ui?.mb_ui_device(touch ? 1 : 0, document.fullscreenEnabled ? 1 : 0);
}
coarse.addEventListener?.('change', device);

// a phone, a tablet, or a computer short of memory holds the recordings'
// lighter copy (a genre's full ones come to half a gigabyte and more);
// ?lite asks for it anywhere, ?full for the full ones on a phone
const LITE = ASKED.has('lite') || (!ASKED.has('full') && (coarse.matches || (navigator.deviceMemory ?? 8) <= 4));
const PACK = LITE ? `${SAMPLES}lite/` : SAMPLES;
// the most of them such a device holds at once (16-bit samples): past it,
// those of genres no longer playing are let go before the next genre's
// are fetched
const LITE_MOST = 150e6;

const KEYS = {
  Enter: 13,
  Backspace: 8,
  Escape: 27,
  Tab: 9,
  ArrowRight: 0x4000004f,
  ArrowLeft: 0x40000050,
  ArrowDown: 0x40000051,
  ArrowUp: 0x40000052,
  F1: 0x4000003a,
};
const PAD = { NumpadEnter: 0x40000058, NumpadSubtract: 0x40000056, NumpadAdd: 0x40000057 };

// SDL's key code for a key (a letter lower case, a symbol as itself)
function sdlKey(e) {
  if (e.code in PAD) return PAD[e.code];
  if (e.key in KEYS) return KEYS[e.key];
  if ([...e.key].length === 1) return e.key.toLowerCase().codePointAt(0);
  return null;
}

document.addEventListener('keydown', (e) => {
  if (!start()) return;
  if (e.altKey) return;
  if (e.ctrlKey || e.metaKey) {
    // Ctrl-C copies the song's id; Ctrl-V is the paste that follows; the
    // rest are the browser's
    if (e.key.toLowerCase() === 'c' && !e.shiftKey) {
      e.preventDefault();
      ui.mb_ui_key(99, 1);
    }
    return;
  }
  const code = sdlKey(e);
  if (code === null) return;
  e.preventDefault();
  ui.mb_ui_key(code, 0);
  if ([...e.key].length === 1) withText(e.key, (p, n) => ui.mb_ui_text(p, n));
  step();
});

document.addEventListener('paste', (e) => {
  e.preventDefault();
  if (!start()) return;
  withText(e.clipboardData?.getData('text/plain') ?? '', (p, n) => ui.mb_ui_paste(p, n));
  step();
});

// A mouse acts as its button goes down, a finger (or a pen) as it lifts:
// each is the moment a browser lets a page begin its sound, copy, read what
// was copied or fill the screen, so the window is stepped there and then
// (step), while that moment lasts, not at the next frame.
const BUTTONS = { 0: 1, 2: 3 };
const finger = (e) => e.pointerType !== 'mouse';
const where = (e) => [Math.round(e.offsetX), Math.round(e.offsetY)];
canvas.addEventListener('pointerdown', (e) => {
  if (!u || !e.isPrimary) return;
  if (finger(e)) {
    ui.mb_ui_finger(0, ...where(e));
    step();
    return;
  }
  if (!(e.button in BUTTONS) || !start()) return;
  canvas.setPointerCapture(e.pointerId);
  ui.mb_ui_motion(...where(e));
  ui.mb_ui_button(1, ...where(e), BUTTONS[e.button]);
  step();
});
canvas.addEventListener('pointerup', (e) => {
  if (!u || !e.isPrimary) return;
  if (finger(e)) {
    start();
    ui.mb_ui_finger(1, ...where(e));
    step();
  } else if (e.button in BUTTONS) {
    ui.mb_ui_button(0, ...where(e), BUTTONS[e.button]);
  }
});
// (a touch the browser took for itself: lifted nowhere)
canvas.addEventListener('pointercancel', (e) => {
  if (u && e.isPrimary && finger(e)) ui.mb_ui_finger(1, -1, -1);
});
canvas.addEventListener('pointermove', (e) => {
  if (!u || !e.isPrimary) return;
  if (finger(e)) ui.mb_ui_finger(2, ...where(e));
  else ui.mb_ui_motion(...where(e));
});
canvas.addEventListener('pointerleave', (e) => {
  if (!finger(e)) ui?.mb_ui_leave();
});
canvas.addEventListener('contextmenu', (e) => e.preventDefault());
canvas.addEventListener(
  'wheel',
  (e) => {
    e.preventDefault();
    if (ui && e.deltaY) ui.mb_ui_wheel(e.deltaY < 0 ? 1 : -1);
  },
  { passive: false },
);
addEventListener('resize', resize);
window.visualViewport?.addEventListener('resize', resize);

// ---- each frame: the sound to the equalizer, the window acted on and
// drawn, what it asks done

const tapped = new Float32Array(TAP);

function frame() {
  requestAnimationFrame(frame);
  step();
}

function step() {
  if (!u) return;
  if (analyser) {
    analyser.getFloatTimeDomainData(tapped);
    const p = ui.mb_alloc(4 * TAP);
    new Float32Array(ui.memory.buffer, p, TAP).set(tapped);
    ui.mb_ui_tap(u, p, TAP);
    ui.mb_free(p, 4 * TAP);
  }
  const p = ui.mb_ui_frame(u);
  const asks = JSON.parse(readText(p, ui.mb_ui_text_len(u)));
  const [w, h] = [ui.mb_ui_width(), ui.mb_ui_height()];
  if (w && h) {
    if (canvas.width !== w || canvas.height !== h) Object.assign(canvas, { width: w, height: h });
    const px = new Uint8ClampedArray(ui.memory.buffer, ui.mb_ui_picture(), 4 * w * h);
    g2d.putImageData(new ImageData(px, w, h), 0, 0);
  }
  for (const a of asks) perform(a);
}

// what the window asks: the page's own at once, the worker's in order once
// the music has begun
let chain = Promise.resolve();

function perform(a) {
  switch (a.do) {
    case 'title':
      document.title = a.text;
      return;
    case 'copy':
      (navigator.clipboard?.writeText(a.text) ?? Promise.reject(new Error('no clipboard'))).catch(() =>
        say('this browser would not copy it: copy the address instead', 'error'),
      );
      return;
    case 'paste':
      (navigator.clipboard?.readText?.() ?? Promise.reject(new Error('no clipboard'))).then(
        (t) => withText(t, (p, n) => ui.mb_ui_paste(p, n)),
        () =>
          say(
            coarse.matches
              ? 'this browser keeps what was copied to itself: open the link to the song instead'
              : 'this browser keeps its clipboard to itself: press ctrl-V',
            'error',
          ),
      );
      return;
    case 'fullscreen':
      if (document.fullscreenElement) document.exitFullscreen?.();
      else document.documentElement.requestFullscreen?.().catch(() => {});
      return;
    case 'favorites':
      keep('musicbox.favorites.txt', a.text);
      return;
    case 'volume':
      keep('musicbox.volume', String(a.volume));
      if (gain) gain.gain.setTargetAtTime((a.volume / 100) ** 3, audio.currentTime, 0.02);
      return;
    case 'effects':
      keepEffects(a.fx);
      break;
    case 'go':
      if (!started) goAsked = true;
      break;
  }
  chain = chain.then(() => ready).then(() => toWorker(a)).catch((e) => say(String(e.message ?? e), 'error'));
}

async function toWorker(a) {
  switch (a.do) {
    case 'skip':
    case 'replay':
      worker.postMessage({ type: a.do });
      return;
    case 'playing':
      worker.postMessage({ type: 'playing', on: a.on });
      return;
    case 'seek':
      worker.postMessage({ type: 'seek', secs: a.secs });
      return;
    case 'length':
      worker.postMessage({ type: 'length', secs: a.secs });
      return;
    case 'effects':
      worker.postMessage({ type: 'effects', fx: a.fx });
      return;
    case 'export':
      return exportSong();
    case 'genre': {
      Object.assign(cur, { primary: a.primary, drums: a.drums < 0 ? null : a.drums });
      if (voicesOf(cur.opts) !== 'synth') await loadRecordings(cur.primary, cur.drums).catch(() => {});
      worker.postMessage({ type: 'genre', primary: a.primary, drums: a.drums < 0 ? null : a.drums });
      return;
    }
    case 'opts': {
      const code = await fetched(cur.primary, cur.drums, a.code);
      cur.opts = code;
      worker.postMessage({ type: 'opts', code });
      return;
    }
    case 'go': {
      Object.assign(cur, { primary: a.primary, drums: a.drums < 0 ? null : a.drums });
      const code = await fetched(cur.primary, cur.drums, a.opts);
      cur.opts = code;
      worker.postMessage({ type: 'go', text: a.id });
      // (no recordings to be had: the synthesiser's instruments after all)
      if (code !== a.opts) worker.postMessage({ type: 'opts', code });
      return;
    }
  }
}

// what music made as `code` (Opts::code) asks fetched for the genre and
// drummer: the recordings, the network's instruments, the tunes networks;
// how it can be made (on the synthesiser when the recordings will not load)
async function fetched(primary, drums, code) {
  const voices = voicesOf(code);
  if (voices !== 'synth') {
    try {
      await loadRecordings(primary, drums);
      if (voices === 'network') await loadNets();
    } catch (e) {
      say(`no recordings (${e.message}): the synthesiser plays`, 'error');
      code = onSynth(code);
      ui.mb_ui_made(u, code, 0);
    }
  }
  if (code & 2) await loadTunes();
  return code;
}

// ---- the recordings: the tables, then each group a genre plays, fetched,
// decoded here (a worker has no decoder) and handed to the engine (moved,
// not copied: a genre's come to hundreds of megabytes decoded); one load at a time

const rec = { index: null, meta: null, have: new Set(), queue: Promise.resolve(), nets: null, tunes: null };

// a genre's groups, and a mix's: both genres', and what the mix plays that
// neither does
function groupsFor(primary, drums) {
  const names = [st.genres[primary], drums === null ? null : st.genres[drums]].filter((n) => n);
  const mix = drums === null ? [] : (rec.index.mixes?.[names.join('+')] ?? []);
  return [...new Set([...names.flatMap((n) => rec.index.genres[n] ?? []), ...mix])].sort((a, b) => a - b);
}

function loadRecordings(primary, drums) {
  rec.queue = rec.queue.catch(() => {}).then(() => fetchRecordings(primary, drums));
  return rec.queue;
}

async function tables() {
  if (rec.index) return;
  const [j, b] = await Promise.all([fetch(`${SAMPLES}pack.json${V}`), fetch(`${PACK}pack.bin${V}`)]);
  if (!j.ok || !b.ok) throw new Error(`the tables did not load (${j.status} ${b.status})`);
  rec.index = await j.json();
  rec.meta = await b.arrayBuffer();
}

// group `g`'s stream, decoded to 16-bit as the engine keeps it: its mono
// recordings one channel (both are the same), then its stereo ones both,
// interleaved (empty for a group whose recordings are all in groups before it)
async function decodeGroup(g) {
  const info = rec.index.groups[g];
  const s16 = (x) => Math.max(-32768, Math.min(32767, Math.round(x * 32768)));
  const mono = info.mono;
  const pcm = new Int16Array(mono + 2 * (info.frames - mono));
  if (!info.frames) return pcm;
  const r = await fetch(`${SAMPLES}${info.file}${R}`);
  if (!r.ok) throw new Error(`${info.name}: ${r.status}`);
  const d = await audio.decodeAudioData(await r.arrayBuffer());
  const [l, rt] = [d.getChannelData(0), d.getChannelData(d.numberOfChannels > 1 ? 1 : 0)];
  const n = Math.min(l.length, info.frames);
  for (let k = 0; k < Math.min(n, mono); k++) pcm[k] = s16(l[k]);
  for (let k = mono; k < n; k++) {
    pcm[2 * k - mono] = s16(l[k]);
    pcm[2 * k - mono + 1] = s16(rt[k]);
  }
  return pcm;
}

// a part of a group's stream in the lighter copy (`n` frames of one channel
// at its own rate), decoded at that rate where the browser will (half the
// memory while it is decoded), else at the engine's and every other frame
// taken
async function decodePart(t) {
  const pcm = new Int16Array(t.n);
  if (!t.n) return pcm;
  const r = await fetch(`${PACK}${t.g}.${t.k}.opus${R}`);
  if (!r.ok) throw new Error(`${rec.index.groups[t.g].name}: ${r.status}`);
  const bytes = await r.arrayBuffer();
  let d = null;
  try {
    d = await new OfflineAudioContext(1, 1, LITE_RATE).decodeAudioData(bytes.slice(0));
  } catch {}
  if (!d || d.sampleRate % LITE_RATE) d = await audio.decodeAudioData(bytes);
  const [l, step] = [d.getChannelData(0), Math.round(d.sampleRate / LITE_RATE)];
  const n = Math.min(Math.floor(l.length / step), t.n);
  for (let k = 0; k < n; k++) pcm[k] = Math.max(-32768, Math.min(32767, Math.round(l[k * step] * 32768)));
  return pcm;
}

// the groups `todo` (in order) handed to `to`, room for them all made
// first. The full recordings: a group's stream whole, four decoded at a
// time, each handed on once those before it are (a group's recordings may
// be in one before it), so that no more than a few are held here at once.
// The lighter copy: each stream a part at a time (a minute and a half of
// it), two decoded at a time, written into the group's room as they come;
// and the room, the first time, for all such a device is to hold, so that
// it is never made again larger.
async function sendGroups(todo, to, progress) {
  const idx = rec.index.groups;
  let done = 0;
  if (LITE) {
    const all = todo.reduce((a, g) => a + idx[g].lite, 0);
    to.postMessage({ type: 'reserve', n: to.reserved ? all : Math.max(all, LITE_MOST) });
    to.reserved = true;
    const parts = todo.flatMap((g) => {
      let at = 0;
      return idx[g].parts.map((n, k, list) => {
        const t = { g, k, n, at, total: idx[g].lite, last: k === list.length - 1 };
        at += n;
        return t;
      });
    });
    const got = new Map();
    let [next, taken] = [0, 0];
    const hand = () => {
      while (next < parts.length && got.has(next)) {
        const [t, pcm] = [parts[next], got.get(next)];
        got.delete(next++);
        to.postMessage({ type: 'part', g: t.g, at: t.at, total: t.total, last: t.last, pcm }, [pcm.buffer]);
        if (t.last) progress?.(++done);
      }
    };
    await Promise.all(
      [0, 1].map(async () => {
        while (taken < parts.length) {
          const i = taken++;
          got.set(i, await decodePart(parts[i]));
          hand();
        }
      }),
    );
    return;
  }
  const kept = (g) => idx[g].mono + 2 * (idx[g].frames - idx[g].mono);
  to.postMessage({ type: 'reserve', n: todo.map(kept).reduce((a, b) => a + b, 0) });
  const got = new Map();
  const left = [...todo];
  let next = 0;
  const hand = () => {
    while (next < todo.length && got.has(todo[next])) {
      const g = todo[next++];
      const pcm = got.get(g);
      got.delete(g);
      to.postMessage({ type: 'group', g, pcm }, [pcm.buffer]);
    }
  };
  await Promise.all(
    [0, 1, 2, 3].map(async () => {
      while (left.length) {
        const g = left.shift();
        got.set(g, await decodeGroup(g));
        progress?.(++done);
        hand();
      }
    }),
  );
}

async function fetchRecordings(primary, drums) {
  if (!rec.index) {
    await tables();
    worker.postMessage({ type: 'pack', meta: rec.meta.slice(0) });
  }
  const need = groupsFor(primary, drums);
  let todo = need.filter((g) => !rec.have.has(g));
  if (!todo.length) return;
  // a phone's memory: with these it would hold too much, so everything is
  // let go and what the music now needs fetched again (the browser kept
  // the files; what was playing falls quiet till it is in)
  const held = (gs) => gs.reduce((a, g) => a + rec.index.groups[g].lite, 0);
  if (LITE && held([...rec.have]) + held(todo) > LITE_MOST && [...rec.have].some((g) => !need.includes(g))) {
    worker.postMessage({ type: 'forget' });
    rec.have.clear();
    todo = need;
  }
  const mb = todo.map((g) => rec.index.groups[g][LITE ? 'lite_bytes' : 'bytes'] ?? 0).reduce((a, b) => a + b, 0) / 1e6;
  const of = `${todo.length} groups${mb ? `, ${mb.toFixed(1)} MB` : ''}`;
  say(`loading recordings: 0 of ${of}`);
  await sendGroups(todo, worker, (n) => say(`loading recordings: ${n} of ${of}`));
  todo.forEach((g) => rec.have.add(g));
  say('recordings loaded');
}

// the tunes networks (models/index.tsv's "tunes" lines: the genre, then the
// file), once
async function loadTunes() {
  if (rec.tunes) return;
  rec.tunes = [];
  const r = await fetch(`app/models/index.tsv${V}`);
  const lines = r.ok ? (await r.text()).split('\n').filter((l) => /^tunes\t/.test(l)) : [];
  for (const line of lines) {
    const [, genre, , file] = line.split('\t');
    const g = st.genres.indexOf(genre.trim());
    const f = await fetch(`app/models/${file.trim()}${V}`);
    if (g < 0 || !f.ok) continue;
    const bytes = await f.arrayBuffer();
    rec.tunes.push({ genre: g, bytes });
    worker.postMessage({ type: 'tunes', genre: g, bytes: bytes.slice(0) });
  }
}

// the network's instruments (models/index.tsv: kind, name, articulation,
// file), once, after the tables
async function loadNets() {
  if (rec.nets) return;
  const r = await fetch(`app/models/index.tsv${V}`);
  const lines = r.ok ? (await r.text()).split('\n').filter((l) => /^(multi|gm)\t/.test(l)) : [];
  rec.nets = [];
  for (const line of lines) {
    const f = await fetch(`app/models/${line.split('\t')[3].trim()}${V}`);
    if (!f.ok) continue;
    const bytes = await f.arrayBuffer();
    rec.nets.push({ line, bytes });
    worker.postMessage({ type: 'net', line, bytes: bytes.slice(0) });
  }
}

// ---- the song playing saved: an MP3, by its id, made again on a worker of
// its own with what it plays

async function exportSong() {
  const n = st.now;
  if (!n?.sid) return say('the song has not begun');
  exporter ??= makeExporter();
  say(`exporting ${n.sid}.mp3…`);
  // the recordings the song plays, decoded again for the exporter's engine
  if (voicesOf(n.opts) !== 'synth' && rec.index) {
    if (!exporter.meta) {
      exporter.postMessage({ type: 'pack', meta: rec.meta.slice(0) });
      exporter.meta = true;
    }
    const todo = [...rec.have].filter((g) => !exporter.groups.has(g)).sort((a, b) => a - b);
    await sendGroups(todo, exporter);
    todo.forEach((g) => exporter.groups.add(g));
    if (rec.nets && !exporter.nets) {
      for (const { line, bytes } of rec.nets) exporter.postMessage({ type: 'net', line, bytes: bytes.slice(0) });
      exporter.nets = true;
    }
  }
  // the tunes networks, whatever plays them
  if (n.opts & 2 && rec.tunes && !exporter.tunes) {
    for (const { genre, bytes } of rec.tunes) exporter.postMessage({ type: 'tunes', genre, bytes: bytes.slice(0) });
    exporter.tunes = true;
  }
  exporter.postMessage({ type: 'export', sid: n.sid, year: new Date().getFullYear(), effects: uiState().fx });
}

function makeExporter() {
  const ex = new Worker(`app/worker.js${V}`);
  ex.groups = new Set();
  ex.onmessage = (e) => {
    if (e.data.type === 'error') say(e.data.error, 'error');
    if (e.data.type === 'say') say(e.data.text, e.data.kind);
    if (e.data.type !== 'saved') return;
    const url = URL.createObjectURL(e.data.blob);
    const a = Object.assign(document.createElement('a'), { href: url, download: e.data.name });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
    say(e.data.why ? `saved ${e.data.name} (${e.data.why})` : `saved ${e.data.name}`, e.data.why ? 'error' : 'note');
  };
  ex.postMessage({ type: 'init', module, genre: 0, seed: 1 });
  return ex;
}

// ---- the window opened, and the music begun (the browser plays nothing
// until a key or a click asks it to)

async function open() {
  // (a server that sends the module as anything but application/wasm:
  // compiled from its bytes instead)
  module = await WebAssembly.compileStreaming(fetch(`app/musicbox.wasm${V}`)).catch(async () =>
    WebAssembly.compile(await (await fetch(`app/musicbox.wasm${V}`)).arrayBuffer()),
  );
  const x = (await WebAssembly.instantiate(module, IMPORTS)).exports;
  ui = x;
  const seed = Math.floor(Math.random() * 0xffffffff);
  const [lo, hi] = savedEffects();
  // (the genre the seed's, among them all, as the desktop picks one)
  const primary = seed % x.mb_genre_count();
  Object.assign(cur, { primary, drums: null, opts: VOICES.indexOf('recorded') << 2 });
  u = withText(savedFavorites(), (p, n) => x.mb_ui_new(primary, -1, seed >>> 0, 0, cur.opts, 0, savedVolume(), lo, hi, p, n));
  device();
  resize();
  // the networks there are to fetch (the window underlines NETWORK where
  // one plays): the instruments, and celtic's tunes
  fetch(`app/models/index.tsv${V}`)
    .then((r) => (r.ok ? r.text() : ''))
    .then((t) => {
      const lines = t.split('\n');
      x.mb_ui_nets(lines.filter((l) => /^(multi|gm)\t/.test(l)).length, lines.some((l) => /^tunes\tceltic\t/.test(l)) ? 1 : 0);
    }, () => {});
  // the song the address names
  const asked = decodeURIComponent(location.hash.slice(1));
  if (asked) withText(asked, (p, n) => x.mb_ui_go(u, p, n));
  ready = new Promise((r) => (isReady = r));
  requestAnimationFrame(frame);
}

// the music begun, once: true when the window is open to take the key or
// click that begun it
function start() {
  if (!u) return false;
  if (started) {
    // (the sound a phone stopped, for a call or its lock screen: on again)
    if (audio && audio.state !== 'running') audio.resume().catch(() => {});
    return true;
  }
  started = true;
  ui.mb_ui_started(u);
  begin().catch((e) => say(`could not start: ${e.message}`, 'error'));
  return true;
}

async function begin() {
  const ctx = (audio = new AudioContext({ sampleRate: 48000, latencyHint: 'playback' }));
  // asked at once, while the key or the tap that began it is still being
  // handled: a phone lets sound begin only then (and plays it as music,
  // whatever its ring switch says, where it can be told to)
  ctx.resume().catch(() => {});
  try {
    if (navigator.audioSession) navigator.audioSession.type = 'playback';
  } catch {}
  await ctx.audioWorklet.addModule(`app/worklet.js${V}`);
  const node = new AudioWorkletNode(ctx, 'musicbox', { numberOfInputs: 0, outputChannelCount: [2] });
  // the equalizer's analyser before the volume (it shows the music as made,
  // whatever the volume)
  analyser = Object.assign(ctx.createAnalyser(), { fftSize: TAP, smoothingTimeConstant: 0 });
  gain = ctx.createGain();
  const s = uiState();
  gain.gain.value = (s.volume / 100) ** 3;
  node.connect(analyser);
  analyser.connect(gain);
  gain.connect(ctx.destination);
  node.port.onmessage = (e) => {
    st.now = JSON.parse(e.data.now);
    withText(e.data.now, (p, n) => ui.mb_ui_now(u, p, n));
    // the song's id in the address, to copy as a link
    if (st.now.sid && st.now.sid !== st.sid) {
      st.sid = st.now.sid;
      history.replaceState(null, '', `#${st.sid}`);
    }
  };
  worker = new Worker(`app/worker.js${V}`);
  const ch = new MessageChannel();
  const genres = new Promise((r) => {
    worker.onmessage = (e) => {
      if (e.data.type === 'genres') r(e.data.genres);
      else if (e.data.type === 'error') say(e.data.error, 'error');
    };
  });
  // silent till the first song is ready: its recordings fetched, then it
  // made again on them (on the synthesiser when they will not load)
  Object.assign(cur, { primary: s.primary, drums: s.drums < 0 ? null : s.drums });
  const init = {
    type: 'init',
    module,
    genre: ui.mb_mix_id(s.primary, s.drums),
    seed: s.seed,
    opts: onSynth(s.opts),
    paused: true,
  };
  worker.postMessage({ ...init, port: ch.port1 }, [ch.port1]);
  worker.postMessage({ type: 'effects', fx: s.fx });
  worker.postMessage({ type: 'length', secs: s.length });
  node.port.postMessage({ port: ch.port2 }, [ch.port2]);
  st.genres = await genres;
  if (!goAsked) {
    const code = await fetched(cur.primary, cur.drums, s.opts);
    cur.opts = code;
    worker.postMessage({ type: 'opts', code });
    worker.postMessage({ type: 'playing', on: s.playing });
  }
  isReady();
}

window.addEventListener('hashchange', () => {
  const asked = decodeURIComponent(location.hash.slice(1));
  if (u && asked && asked !== st.sid) withText(asked, (p, n) => ui.mb_ui_go(u, p, n));
});

open().catch((e) => {
  document.body.append(Object.assign(document.createElement('p'), { className: 'failed', textContent: `musicbox could not open: ${e.message}` }));
});
