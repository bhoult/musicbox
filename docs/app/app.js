// musicbox in the browser: the page's side. The engine runs in a worker
// (worker.js, WebAssembly), the audio worklet (worklet.js) plays what it
// makes, and this draws what is playing, the equalizer (an analyser on the
// output) and the controls, as the desktop app's window has them.

const $ = (id) => document.getElementById(id);
const MIN_LENGTH = 30;
const MAX_LENGTH = 3600;

const st = {
  genres: [],
  primary: 0,
  drums: null,
  length: 0,
  stopped: false,
  seed: Math.floor(Math.random() * 0xffffffff),
  now: null,
  // how the music is made (as the engine's Opts): the optimiser's settings,
  // the network's tunes, and what plays the instruments (recordings, as on
  // the desktop)
  opts: { tuned: false, netTunes: false, voices: 'recorded' },
  volume: 100,
};
// the hover help: the control under the mouse, the one whose help shows,
// one clicked (quiet until the mouse moves on), the help's timer, and when
// help last showed
const tip = { over: null, shown: null, quiet: null, timer: 0, warm: -1e9 };
const VOICES = ['synth', 'recorded', 'network'];
const optsCode = (o) => (o.tuned ? 1 : 0) | (o.netTunes ? 2 : 0) | (VOICES.indexOf(o.voices) << 2);
let worker = null;
let exporter = null;
let module = null;
let analyser = null;
let audio = null;
// the volume's gain, after the equalizer's analyser (it shows the music as
// made, whatever the volume)
let gain = null;

// where the recordings are (tools/web-samples.sh makes them)
const SAMPLES = 'samples/';
// the stamps tools/stamp-page.sh puts on this script's address, one for the
// page's files and one for the recordings: each file is asked for with its
// stamp, so an older copy a browser kept of one is never used with the rest
// (a new app.js under an old index.html drew no controls that worked)
const STAMPS = new URL(document.currentScript.src).searchParams;
const V = `?v=${STAMPS.get('v') ?? ''}`;
const R = `?r=${STAMPS.get('r') ?? ''}`;

// a length as typed: auto (or 0); M:SS; 3.5m or 210s; a bare number in
// minutes under 30, else seconds (as the desktop app reads it)
function parseLength(s) {
  s = s.trim().toLowerCase();
  if (s === 'auto' || s === '0') return 0;
  const num = (x) => (/^\d+(\.\d+)?$/.test(x.trim()) ? parseFloat(x) : NaN);
  let secs;
  if (s.includes(':')) {
    const [m, sec] = s.split(':');
    if (!(num(sec) < 60)) return null;
    secs = Math.trunc(num(m)) * 60 + num(sec);
  } else if (s.endsWith('s')) secs = num(s.slice(0, -1));
  else if (s.endsWith('m')) secs = num(s.slice(0, -1)) * 60;
  else {
    const x = num(s);
    secs = x < 30 ? x * 60 : x;
  }
  if (!(secs > 0)) return null;
  return Math.round(Math.min(MAX_LENGTH, Math.max(MIN_LENGTH, secs)));
}

const clock = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
const showLength = (s) => (s > 0 ? clock(s) : 'auto');
const cap = (s) => (s ? s[0].toUpperCase() + s.slice(1) : '');

// the genres the optimiser tuned (src/music/tuned.rs's TUNED), and those a
// network writes tunes for
const TUNED = ['ambient', 'techno', 'lo-fi', 'ska', 'gamelan', 'starconflict'];
const NET_TUNES = ['celtic'];

function say(text, kind = 'note') {
  st.note = { text, kind, until: performance.now() + 6000 };
}

// ---- the controls

function setGenre(primary, drums) {
  st.primary = primary;
  st.drums = drums === primary ? null : drums;
  drawButtons();
  // its recordings first, when they play
  const [p, d] = [st.primary, st.drums];
  const go = () => worker?.postMessage({ type: 'genre', primary: p, drums: d });
  if (st.opts.voices === 'synth') go();
  else loadRecordings(p, d).then(go, go);
}

function stepGenre(by) {
  setGenre((st.primary + by + st.genres.length) % st.genres.length, st.drums);
}

// through the genres in order, the one writing the music for its own drums
function stepDrums(by) {
  const n = st.genres.length;
  setGenre(st.primary, ((st.drums ?? st.primary) + by + n) % n);
}

function setLength(secs) {
  st.length = secs;
  worker?.postMessage({ type: 'length', secs });
  $('auto').classList.toggle('lit', secs <= 0);
  if (secs > 0) $('slider').value = Math.min(600, secs);
  if (document.activeElement !== $('typed')) $('typed').value = showLength(secs);
}

// 15 s longer or shorter: from auto, about the song's own length; under the
// shortest, auto again
function longer(by) {
  const base = st.length > 0 ? st.length : Math.round((st.now?.planned ?? 180) / 15) * 15;
  const v = base + 15 * by;
  setLength(v < MIN_LENGTH ? 0 : Math.min(v, Math.max(600, base), MAX_LENGTH));
}

function setPlaying(on) {
  st.stopped = !on;
  worker?.postMessage({ type: 'playing', on });
  drawPlay();
}

function drawPlay() {
  const b = $('play');
  b.querySelector('span').textContent = st.stopped ? 'start' : 'stop';
  b.querySelector('path').setAttribute('d', st.stopped ? 'M2.5 1.5v9L10.5 6z' : 'M2 2h8v8H2z');
  setHelp(b, st.stopped ? 'Play on from where it stopped.' : 'Stop the music, holding it where it is.');
  refreshTip();
}

function replay() {
  worker?.postMessage({ type: 'replay' });
  setPlaying(true);
}

// ---- songs by their ids: back and forward through those played, a point
// in the one playing, any song by its id

// how far into a song Back still goes to the one before, and how far , and
// . go (seconds)
const BACK_SECS = 3;
const SEEK_SECS = 10;

// ---- the volume: per cent, cubic as the desktop's is, eased in (and
// remembered in this browser)

const VOLUME_STEP = 5;

function setVolume(v) {
  st.volume = Math.min(100, Math.max(0, Math.round(v / VOLUME_STEP) * VOLUME_STEP));
  if (gain) gain.gain.setTargetAtTime((st.volume / 100) ** 3, audio.currentTime, 0.02);
  $('volume').value = st.volume;
  $('vol').textContent = `${st.volume}%`;
  $('vol').classList.toggle('off', st.volume === 0);
  try {
    localStorage.setItem('musicbox.volume', String(st.volume));
  } catch {}
}

function savedVolume() {
  try {
    const v = parseInt(localStorage.getItem('musicbox.volume') ?? '', 10);
    return Number.isFinite(v) ? v : 100;
  } catch {
    return 100;
  }
}

function back() {
  const n = st.now;
  if (n?.back && n.elapsed < BACK_SECS) goTo(n.back);
  else seek(0);
  setPlaying(true);
}

// on through the songs gone back from, else a new one
function next() {
  if (st.now?.fwd) goTo(st.now.fwd);
  else worker?.postMessage({ type: 'skip' });
}

function seek(secs) {
  worker?.postMessage({ type: 'seek', secs: Math.max(0, secs) });
}

// the worker reads the id; `adopt` then plays it
function goTo(text) {
  worker?.postMessage({ type: 'parse', text: text.trim() });
}

// a song read from its id: what it plays fetched, the controls set as it
// asks, then played
async function adopt(id) {
  const opts = { tuned: (id.opts & 1) !== 0, netTunes: (id.opts & 2) !== 0, voices: VOICES[id.opts >> 2] ?? 'recorded' };
  const drums = id.drums < 0 ? null : id.drums;
  try {
    if (opts.voices !== 'synth') {
      await loadRecordings(id.primary, drums);
      if (opts.voices === 'network') await loadNets();
    }
    if (opts.netTunes) await loadTunes();
  } catch (e) {
    say(`no recordings: ${e.message}`, 'error');
    opts.voices = 'synth';
  }
  Object.assign(st.opts, opts);
  st.primary = id.primary;
  st.drums = drums;
  st.length = id.length;
  worker.postMessage({ type: 'go', text: id.text });
  // (no recordings to be had: the synthesiser's instruments after all)
  if (optsCode(opts) !== id.opts) worker.postMessage({ type: 'opts', code: optsCode(opts) });
  drawButtons();
  drawMade();
  setLength(id.length);
  setPlaying(true);
}

// another way of making it: the song playing made again from its start
// (the recordings it plays loaded first)
async function setOpts(change) {
  const next = { ...st.opts, ...change };
  if (next.voices !== 'synth') {
    try {
      await loadRecordings(st.primary, st.drums);
      if (next.voices === 'network') await loadNets();
    } catch (e) {
      say(`no recordings: ${e.message}`, 'error');
      return;
    }
  }
  if (next.netTunes) await loadTunes();
  Object.assign(st.opts, next);
  worker?.postMessage({ type: 'opts', code: optsCode(st.opts) });
  drawMade();
  setPlaying(true);
}

// ---- the recordings: the tables, then each group a genre plays, fetched,
// decoded here (a worker has no decoder) and handed to the engine (moved,
// not copied: a genre's come to tens of megabytes decoded); one load at a time

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
  const [j, b] = await Promise.all([fetch(`${SAMPLES}pack.json${V}`), fetch(`${SAMPLES}pack.bin${V}`)]);
  if (!j.ok || !b.ok) throw new Error(`the tables did not load (${j.status} ${b.status})`);
  rec.index = await j.json();
  rec.meta = await b.arrayBuffer();
}

// group `g`'s stream, decoded to 16-bit (empty for a group whose
// recordings are all in groups before it)
async function decodeGroup(g) {
  const info = rec.index.groups[g];
  const pcm = new Int16Array(info.frames);
  if (!info.frames) return pcm;
  const r = await fetch(`${SAMPLES}${info.file}${R}`);
  if (!r.ok) throw new Error(`${info.name}: ${r.status}`);
  const f = (await audio.decodeAudioData(await r.arrayBuffer())).getChannelData(0);
  const n = Math.min(f.length, pcm.length);
  for (let k = 0; k < n; k++) pcm[k] = Math.max(-32768, Math.min(32767, Math.round(f[k] * 32768)));
  return pcm;
}

// the groups `todo` (in order) decoded four at a time, handed to `to` in
// order (a group's recordings may be in one before it)
async function sendGroups(todo, to, progress) {
  const got = new Map();
  const left = [...todo];
  let done = 0;
  await Promise.all(
    [0, 1, 2, 3].map(async () => {
      while (left.length) {
        const g = left.shift();
        got.set(g, await decodeGroup(g));
        progress?.(++done);
      }
    }),
  );
  for (const g of todo) {
    const pcm = got.get(g);
    to.postMessage({ type: 'group', g, pcm }, [pcm.buffer]);
  }
}

async function fetchRecordings(primary, drums) {
  if (!rec.index) {
    await tables();
    worker.postMessage({ type: 'pack', meta: rec.meta.slice(0) });
  }
  const todo = groupsFor(primary, drums).filter((g) => !rec.have.has(g));
  if (!todo.length) return;
  const mb = todo.map((g) => rec.index.groups[g].bytes ?? 0).reduce((a, b) => a + b, 0) / 1e6;
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
  say(rec.tunes.length ? `tunes networks: ${rec.tunes.map((t) => st.genres[t.genre]).join(', ')}` : 'no tunes networks here yet: the rules write');
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
  say(rec.nets.length ? `${rec.nets.length} network instruments` : 'no network instruments here yet: recordings play');
}

// the settings, the one chosen lit, an option underlined where it changes
// the genre writing the music, and each one's help
function drawMade() {
  const g = st.genres[st.primary] ?? '';
  const G = cap(g) || 'This genre';
  const help = {
    'tuned:false': ["Each genre's mix as it was written by hand."],
    'tuned:true': [
      'The mix as an optimiser tuned it against real recordings, for six genres: ambient, techno, lo-fi, ska, gamelan and starconflict.',
      TUNED.includes(g),
      `${G} is one of them.`,
      `${G} is not one yet: it plays as written.`,
    ],
    'netTunes:false': ["Tunes written by the composer's rules, as every genre has them."],
    'netTunes:true': [
      "Celtic tunes by a small network that learned O'Neill's Music of Ireland (1903).",
      NET_TUNES.includes(g),
      'It writes the tunes playing.',
      `Celtic has one alone: ${g || 'this genre'} keeps to the rules.`,
    ],
    'voices:synth': ['A small synthesiser plays the notes: nothing to download.'],
    'voices:recorded': ['Recordings of real instruments play the notes, fetched as each genre needs them (4 to 26 MB a genre).'],
    'voices:network': ['18 instruments played by small networks that learned their recordings, the rest recorded; fetched as they are first chosen.', true],
  };
  for (const b of document.querySelectorAll('#made button')) {
    const [k, v] = [b.dataset.k, b.dataset.v];
    const on = k === 'voices' ? st.opts.voices === v : String(st.opts[k]) === v;
    const [text, real, yes, no] = help[`${k}:${v}`];
    b.classList.toggle('lit', on);
    b.classList.toggle('real', !!real);
    setHelp(b, [text, real ? yes : no].filter((t) => t).join(' '));
  }
  refreshTip();
}

// the song playing, by its id, as an MP3 (a WAV when `wav`)
async function exportSong(wav = false) {
  const n = st.now;
  if (!n || !module) return;
  if (!n.sid) return say('the song has not begun');
  exporter ??= makeExporter();
  say(`exporting ${n.sid}.${wav ? 'wav' : 'mp3'}…`);
  // the recordings the song plays, decoded again for the exporter's engine
  if ((n.opts >> 2) !== 0 && rec.index) {
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
  exporter.postMessage({ type: 'export', sid: n.sid, wav, year: new Date().getFullYear() });
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

// ---- the buttons

// the genres to write the music, the one that does lit and the drummer
// underlined (a right-click puts a genre on drums); the drummer chosen from
// a list too (the music's own drums first)
function drawButtons() {
  const g = $('genres');
  const d = $('drums');
  if (!g.children.length) {
    st.genres.forEach((name, i) => {
      const b = Object.assign(document.createElement('button'), { textContent: name });
      b.dataset.key = '← →';
      b.onclick = () => setGenre(i, st.drums);
      b.oncontextmenu = (e) => {
        e.preventDefault();
        setGenre(st.primary, i);
      };
      g.append(b);
    });
    // the drummer: the music's own drums, or any genre's
    d.append(Object.assign(document.createElement('option'), { value: '', textContent: 'its own' }));
    st.genres.forEach((name, i) => d.append(Object.assign(document.createElement('option'), { value: i, textContent: name })));
    d.onchange = () => setGenre(st.primary, d.value === '' ? null : Number(d.value));
  }
  const p = st.genres[st.primary];
  [...g.children].forEach((b, i) => {
    const [n, N] = [st.genres[i], cap(st.genres[i])];
    b.className = i === st.primary ? 'lit' : i === st.drums ? 'mark' : '';
    setHelp(
      b,
      i === st.primary
        ? `${N} writes the music. Right-click a genre to put it on drums.`
        : `${i === st.drums ? `${N} is on drums (underlined). ` : ''}Click: ${n} writes the music. Right-click: ${n} on drums.`,
    );
  });
  d.value = st.drums ?? '';
  d.className = st.drums == null ? '' : 'drum';
  setHelp(
    d,
    st.drums == null
      ? `${cap(p)} on its own drums. Choose a genre to put on drums, or right-click one.`
      : `${cap(st.genres[st.drums])} drums ${p}'s music. "Its own": ${p} on its own drums.`,
  );
  drawMade();
}

// ---- what is playing

function drawNow() {
  const n = st.now;
  if (!n) return;
  drawWhere(n);
  const [p, d] = n.name.split('+');
  $('playing').innerHTML = d ? `${p} <span class="and">+</span> <span class="drums">${d}</span> <span class="and">drums</span>` : p;
  const want = st.drums === null ? st.genres[st.primary] : `${st.genres[st.primary]}+${st.genres[st.drums]}`;
  const line = $('line');
  if (st.note && performance.now() < st.note.until) {
    line.textContent = st.note.text;
    line.className = `line ${st.note.kind}`;
    return;
  }
  let s = `song ${n.piece} / ${n.meter} / ${Math.round(n.bpm)} bpm`;
  if (n.key) s += ` / ${n.key.replace(/b$/, '♭')} ${n.mode}`;
  if (n.section) s += ` / ${n.section} ${Math.max(1, n.bar)} of ${n.bars}`;
  if (n.stopped) s += ' / stopped';
  if (want !== n.name) s += `  →  next: ${want}`;
  line.textContent = s;
  line.className = 'line';
}

// the song's id (in its field, and the address the page is at) and where
// in it the music is
function drawWhere(n) {
  if (n.sid && n.sid !== st.sid) {
    st.sid = n.sid;
    if (document.activeElement !== $('sid')) $('sid').value = n.sid;
    history.replaceState(null, '', `#${n.sid}`);
  }
  const span = Math.max(1, n.length > 0 ? n.length : n.planned);
  const pos = $('pos');
  pos.max = String(Math.round(span));
  if (!st.scrubbing) pos.value = String(Math.min(span, n.elapsed));
  $('time').textContent = `${clock(st.scrubbing ? Number(pos.value) : n.elapsed)} / ${clock(span)}`;
}

// ---- the equalizer: LED bars 35 Hz to 16 kHz, each band's loudest bin
// tilted up 3 dB an octave (about 1 kHz), falling 1.4 a second, a peak held
// 0.6 s

const eq = { bars: [], peaks: [], held: [], last: 0 };

function led(t) {
  const mix = (a, b, k) => a.map((x, i) => Math.round(x + (b[i] - x) * k));
  const teal0 = [30, 158, 146], teal = [95, 211, 198], amber = [242, 181, 68], red = [239, 79, 95];
  const c = t < 0.55 ? mix(teal0, teal, t / 0.55) : t < 0.8 ? mix(teal, amber, (t - 0.55) / 0.25) : mix(amber, red, Math.min(1, (t - 0.8) / 0.2));
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

function drawEq(t) {
  requestAnimationFrame(drawEq);
  const cv = $('eq');
  const dpr = window.devicePixelRatio || 1;
  const W = Math.round(cv.clientWidth * dpr);
  const H = Math.round(cv.clientHeight * dpr);
  if (cv.width !== W || cv.height !== H) Object.assign(cv, { width: W, height: H });
  const g = cv.getContext('2d');
  g.fillStyle = '#141a21';
  g.fillRect(0, 0, W, H);
  const n = Math.max(16, Math.min(96, Math.floor(cv.clientWidth / 11)));
  const label = 16 * dpr;
  const top = 8 * dpr;
  const seg = 4 * dpr;
  const segs = Math.max(1, Math.floor((H - label - top) / seg));
  const slot = (W - 8 * dpr) / n;
  const dt = Math.min(0.1, (t - eq.last) / 1000 || 0);
  eq.last = t;
  let db = null;
  if (analyser) {
    db = new Float32Array(analyser.frequencyBinCount);
    analyser.getFloatFrequencyData(db);
  }
  const hz = (k) => 35 * Math.pow(16000 / 35, k / n);
  const bin = (f) => (f * analyser.fftSize) / analyser.context.sampleRate;
  for (let b = 0; b < n; b++) {
    let level = 0;
    if (db) {
      const lo = Math.floor(bin(hz(b)));
      const hi = Math.max(lo + 1, Math.ceil(bin(hz(b + 1))));
      let m = -200;
      for (let k = lo; k < hi && k < db.length; k++) m = Math.max(m, db[k]);
      const fc = Math.sqrt(hz(b) * hz(b + 1));
      m += 3 * Math.log2(fc / 1000);
      level = Math.min(1, Math.max(0, (m + 85) / 60));
    }
    eq.bars[b] = Math.max(level, (eq.bars[b] ?? 0) - 1.4 * dt);
    if (level >= (eq.peaks[b] ?? 0)) {
      eq.peaks[b] = level;
      eq.held[b] = t;
    } else if (t - (eq.held[b] ?? 0) > 600) {
      eq.peaks[b] = Math.max(0, eq.peaks[b] - 1.4 * dt);
    }
    const x = 4 * dpr + b * slot;
    const bw = Math.max(1, slot - 2 * dpr);
    const lit = Math.round(eq.bars[b] * segs);
    for (let s = 0; s < segs; s++) {
      g.fillStyle = s < lit ? led(s / segs) : '#1b232c';
      g.fillRect(x, H - label - (s + 1) * seg + dpr, bw, seg - dpr);
    }
    const pk = Math.round(eq.peaks[b] * segs);
    if (pk > 0) {
      g.fillStyle = '#f4f6fa';
      g.fillRect(x, H - label - pk * seg + dpr, bw, 2 * dpr);
    }
  }
  g.fillStyle = '#5c6573';
  g.font = `${11 * dpr}px ui-monospace, monospace`;
  g.textAlign = 'center';
  for (const [f, s] of [[50, '50'], [100, '100'], [200, '200'], [500, '500'], [1000, '1K'], [2000, '2K'], [5000, '5K'], [10000, '10K']]) {
    const x = 4 * dpr + (Math.log(f / 35) / Math.log(16000 / 35)) * (W - 8 * dpr);
    g.fillText(s, x, H - 4 * dpr);
  }
  drawNow();
}

// ---- the start (sound needs a press first)

async function start() {
  $('start').remove();
  const ctx = (audio = new AudioContext({ sampleRate: 48000, latencyHint: 'playback' }));
  // (a server that sends the module as anything but application/wasm:
  // compiled from its bytes instead)
  module = await WebAssembly.compileStreaming(fetch(`app/musicbox.wasm${V}`)).catch(async () =>
    WebAssembly.compile(await (await fetch(`app/musicbox.wasm${V}`)).arrayBuffer()),
  );
  await ctx.audioWorklet.addModule(`app/worklet.js${V}`);
  const node = new AudioWorkletNode(ctx, 'musicbox', { numberOfInputs: 0, outputChannelCount: [2] });
  analyser = Object.assign(ctx.createAnalyser(), { fftSize: 8192, smoothingTimeConstant: 0 });
  gain = ctx.createGain();
  gain.gain.value = (st.volume / 100) ** 3;
  node.connect(analyser);
  analyser.connect(gain);
  gain.connect(ctx.destination);
  node.port.onmessage = (e) => {
    st.now = JSON.parse(e.data.now);
  };
  worker = new Worker(`app/worker.js${V}`);
  const ch = new MessageChannel();
  worker.onmessage = (e) => {
    if (e.data.type === 'genres') {
      st.genres = e.data.genres;
      st.primary = e.data.genre;
      drawButtons();
      // (the genres known: what the first song plays can be fetched)
      if (!st.waiting) begin();
    } else if (e.data.type === 'id') {
      if (e.data.id) adopt(e.data.id);
      else say(`not a song's id: ${e.data.text}`, 'error');
      if (st.waiting && !e.data.id) begin();
      st.waiting = false;
    } else if (e.data.type === 'error') {
      say(e.data.error, 'error');
    }
  };
  // silence till the first song is ready: the one the address names, read
  // and what it plays fetched; else a new one, made on the synthesiser
  // until its recordings are in
  const asked = decodeURIComponent(location.hash.slice(1));
  st.waiting = !!asked;
  const init = { type: 'init', module, genre: null, seed: st.seed, opts: optsCode({ ...st.opts, voices: 'synth' }), paused: true };
  worker.postMessage({ ...init, port: ch.port1 }, [ch.port1]);
  node.port.postMessage({ port: ch.port2 }, [ch.port2]);
  await ctx.resume();
  if (asked) goTo(asked);
}

// the first song: the recordings it plays fetched, then it made again on
// them and played (on the synthesiser, when they will not load)
async function begin() {
  if (st.opts.voices !== 'synth') {
    try {
      await loadRecordings(st.primary, st.drums);
    } catch (e) {
      say(`no recordings (${e.message}): the synthesiser plays`, 'error');
      st.opts.voices = 'synth';
      drawMade();
    }
  }
  worker.postMessage({ type: 'opts', code: optsCode(st.opts) });
  setPlaying(true);
}

window.addEventListener('hashchange', () => {
  const asked = decodeURIComponent(location.hash.slice(1));
  if (worker && asked && asked !== st.sid) goTo(asked);
});

$('go').onclick = () => start().catch((e) => say(`could not start: ${e.message}`, 'error'));
$('play').onclick = () => setPlaying(st.stopped);
$('skip').onclick = next;
$('back').onclick = back;
$('rew').onclick = () => seek((st.now?.elapsed ?? 0) - SEEK_SECS);
$('ff').onclick = () => seek((st.now?.elapsed ?? 0) + SEEK_SECS);
// the bar held: where it would go shown, gone to when let go
$('pos').oninput = () => {
  st.scrubbing = true;
};
$('pos').onchange = (e) => {
  st.scrubbing = false;
  seek(Number(e.target.value));
};
// the song's id: all of it chosen when the field is, so what is pasted or
// typed takes its place; Enter plays it
let chosen = false;
$('sid').onfocus = (e) => {
  e.target.select();
  chosen = true;
};
$('sid').onmouseup = (e) => {
  if (chosen) e.preventDefault();
  chosen = false;
};
$('sid').onkeydown = (e) => {
  if (e.key === 'Enter') {
    const text = e.target.value.trim();
    if (text && text !== st.sid) goTo(text);
    e.target.blur();
  } else if (e.key === 'Escape') {
    e.target.blur();
  }
};
$('sid').onblur = (e) => {
  e.target.value = st.sid ?? '';
};
$('copy').onclick = async () => {
  if (!st.sid) return say('the song has not begun');
  try {
    await navigator.clipboard.writeText(`${location.origin}${location.pathname}#${st.sid}`);
    say('a link to this song copied');
  } catch {
    $('sid').focus();
    say('copy its id instead (ctrl-c)');
  }
};
$('export').onclick = (e) => exportSong(e.shiftKey);
$('wav').onclick = () => exportSong(true);
$('keys').onclick = $('keys2').onclick = showKeys;
for (const b of document.querySelectorAll('#made button')) {
  const [k, v] = [b.dataset.k, b.dataset.v];
  b.onclick = () => setOpts({ [k]: k === 'voices' ? v : v === 'true' });
}
drawMade();
$('auto').onclick = () => setLength(0);
$('slider').oninput = (e) => setLength(Number(e.target.value));
$('typed').onkeydown = (e) => {
  if (e.key === 'Enter') {
    const v = parseLength(e.target.value);
    if (v === null) say(`not a length: ${e.target.value}`, 'error');
    else setLength(v);
    e.target.blur();
  } else if (e.key === 'Escape') {
    e.target.blur();
  }
};
$('typed').onblur = () => {
  $('typed').value = showLength(st.length);
};
$('slider').onwheel = (e) => {
  e.preventDefault();
  longer(e.deltaY < 0 ? 1 : -1);
};
$('volume').oninput = (e) => setVolume(+e.target.value);
$('volume').onwheel = (e) => {
  e.preventDefault();
  setVolume(st.volume + (e.deltaY < 0 ? VOLUME_STEP : -VOLUME_STEP));
};
setVolume(savedVolume());

document.addEventListener('keydown', (e) => {
  if (e.target.tagName === 'INPUT' && e.target.type === 'text') return;
  // (the drummer's list takes its own arrows)
  if (e.target.tagName === 'SELECT' && e.key.startsWith('Arrow')) return;
  if (e.ctrlKey || e.metaKey || e.altKey || !worker) return;
  const k = e.key;
  const act = {
    ArrowRight: () => stepGenre(1),
    ArrowLeft: () => stepGenre(-1),
    ArrowUp: () => stepDrums(1),
    ArrowDown: () => stepDrums(-1),
    ' ': next,
    s: () => setPlaying(st.stopped),
    b: back,
    ',': () => seek((st.now?.elapsed ?? 0) - SEEK_SECS),
    '.': () => seek((st.now?.elapsed ?? 0) + SEEK_SECS),
    d: () => $('sid').focus(),
    '?': showKeys,
    r: replay,
    x: () => exportSong(e.shiftKey),
    o: () => setOpts({ tuned: !st.opts.tuned }),
    m: () => setOpts({ netTunes: !st.opts.netTunes }),
    i: () => setOpts({ voices: VOICES[(VOICES.indexOf(st.opts.voices) + 1) % VOICES.length] }),
    '[': () => longer(-1),
    ']': () => longer(1),
    '-': () => setVolume(st.volume - VOLUME_STEP),
    '=': () => setVolume(st.volume + VOLUME_STEP),
    '+': () => setVolume(st.volume + VOLUME_STEP),
    t: () => {
      $('typed').value = '';
      $('typed').focus();
    },
  }[k.length === 1 ? k.toLowerCase() : k];
  if (act) {
    e.preventDefault();
    act();
  }
});

// ---- hover help: a control's data-help, and its keys (data-key), shown by
// it a moment after the mouse comes to rest there (at once just after
// another), or when the keyboard reaches it; screen readers have it as the
// control's description

function setHelp(el, text) {
  el.dataset.help = text;
  el.setAttribute('aria-description', text);
}

function showTip(el) {
  const t = $('tip');
  t.replaceChildren(el.dataset.help);
  const keys = el.dataset.key?.split(' ') ?? [];
  if (keys.length) {
    const line = Object.assign(document.createElement('span'), { className: 'keys', textContent: keys.length > 1 ? 'keys ' : 'key ' });
    for (const k of keys) line.append(Object.assign(document.createElement('kbd'), { textContent: k }));
    t.append(line);
  }
  t.hidden = false;
  const r = el.getBoundingClientRect();
  const x = Math.max(8, Math.min(r.left, innerWidth - t.offsetWidth - 8));
  const below = r.bottom + 8;
  const y = below + t.offsetHeight > innerHeight - 8 ? Math.max(8, r.top - t.offsetHeight - 8) : below;
  Object.assign(t.style, { left: `${x}px`, top: `${y}px` });
  tip.shown = el;
}

function hideTip() {
  clearTimeout(tip.timer);
  if (tip.shown) tip.warm = performance.now();
  $('tip').hidden = true;
  tip.shown = null;
}

// what it says changed (a button that now does something else)
function refreshTip() {
  if (tip.shown) showTip(tip.shown);
}

document.addEventListener('pointerover', (e) => {
  if (e.pointerType === 'touch') return;
  const el = e.target.closest?.('[data-help]') ?? null;
  if (el === tip.over) return;
  tip.over = el;
  tip.quiet = null;
  hideTip();
  if (el) tip.timer = setTimeout(() => showTip(el), performance.now() - tip.warm < 300 ? 0 : 450);
});
// (a click quiets it until the mouse moves on)
document.addEventListener(
  'pointerdown',
  () => {
    tip.quiet = tip.over;
    hideTip();
  },
  true,
);
document.addEventListener('focusin', (e) => {
  const el = e.target.closest?.('[data-help]');
  if (el && e.target.matches(':focus-visible') && el !== tip.quiet) showTip(el);
});
document.addEventListener('focusout', () => {
  if (tip.shown && tip.shown !== tip.over) hideTip();
});
addEventListener('scroll', hideTip, { passive: true });
addEventListener('blur', hideTip);
for (const el of document.querySelectorAll('[data-help]')) setHelp(el, el.dataset.help);

// ---- the keys, all of them

function showKeys() {
  hideTip();
  const d = $('keyhelp');
  if (d.open) d.close();
  else d.showModal();
}

drawPlay();
requestAnimationFrame(drawEq);
