# /// script
# requires-python = ">=3.10"
# dependencies = ["numpy", "scipy", "torch"]
#
# [tool.uv.sources]
# torch = { index = "pytorch-cpu" }
#
# [[tool.uv.index]]
# name = "pytorch-cpu"
# url = "https://download.pytorch.org/whl/cpu"
# explicit = true
# ///
"""musicbox's network instruments (src/net.rs): one small network per
recorded instrument, trained from the sampler's own renderings of it.

    uv run tools/neural/voices.py NAME... [--data DIR] [--models DIR] [--work DIR]
                                          [--hidden 128,128,128,128] [--sup-steps N]
                                          [--e2e-minutes M] [--queue] [--eval-only]
    uv run tools/neural/voices.py --table NAME...
    uv run tools/neural/voices.py --selftest

NAME is a --voice-data file's stem: multi_violin_sus, gm_Nylon_String_Guitar
(made by `musicbox --voice-data DIR "multi violin sus" "gm Nylon String Guitar"`).
Writes models/NAME.mbn, its line in models/index.tsv (the file's other lines
kept as they are), listening files (the recording, then the network) and a
report (NAME.json, NAME.log) in the work directory. With --queue, several
copies share the list, each taking the next name no copy has taken
(VOICES_THREADS: each copy's threads, 2 by default).

How:
 1. Analysis, on net.rs's frame grid (a frame every 192 samples, its value
    the one at its first sample): the note's pitch tracked within 80 cents
    each frame, each harmonic's energy within half a harmonic of k f0 (Hann,
    about six periods), the noise between the harmonics (Blackman-Harris,
    about ten periods, the harmonics masked), and the frames sharpened in
    time to a three-period window's energy. The noise is fitted per frame
    to what the 16 bands make: one white noise through all of them, so its
    spectrum is |sum_b a_b H_b|^2. The vibrato from the pitch track: its
    3.5-8.5 Hz part's analytic signal, the depth its magnitude (cents), the
    rate its phase's speed.
 2. A dense network (3, 4 x 128, 65 or 67: 58k parameters, 234 KB) fitted
    to those in the log domain (ln total, log shares of the harmonics under
    20 kHz, ln band RMS; ln vibrato depth, its rate), each recording's frames
    at velocities across its own velocity layer (the manifest's vel_lo to
    vel_hi, velocity = MIDI / 127, so the network steps where the sampler
    does), and up to 12 semitones past each end of the range the end note
    held (an untaught network blows up there). An instrument whose held
    notes waver at least 3 cents, most of it at 3.5-8.5 Hz, gets the
    vibrato outputs (MBN2); the rest MBN1.
 3. Fine-tuning end to end: a differentiable copy of net.rs's synthesis
    (the same ramp, harmonics, cut-off, bands, normalisation, and for MBN2
    the vibrato: the network's depth, at the recording's own vibrato phase
    and slow intonation, so the comparison is between the same wobbles)
    scored by the multi-resolution log-spectral distance below plus the
    loudness envelope's; kept only if it is better. The vibrato outputs
    learn only from the analysis.

What it reports, from a numpy port of net.rs's synthesis playing the file
written (the network's own float32 arithmetic, the frame ramp, the bands'
norms measured as net.rs measures them, the vibrato from phase 0):
  - loudness: each held note's level (RMS over its 3 s, dB) against the
    recording's, by velocity and by register; and the 50 ms envelope's
    mean error over the frames within 40 dB of the note's peak;
  - MR-LSD: the mean over FFT sizes 2048, 1024, 512, 256 (hop a quarter,
    Hann) of the mean |difference| of 20 log10(|X| / sum(w) + 1e-6) over
    the bins under 20 kHz (1e-6: about 16-bit quantisation noise per bin);
  - the same for the analysis played back as is (the most net.rs's
    synthesis gets from these targets) and for a baseline: each note's own
    average harmonics and noise held steady for the 3 s, no vibrato;
  - for MBN2, the network with its vibrato off, and its depth and rate
    against the recordings' (medians over the held notes' last 2.5 s).
"""

import argparse
import fcntl
import hashlib
import json
import math
import os
import struct
import sys
import time
import traceback
from pathlib import Path

THREADS = int(os.environ.get("VOICES_THREADS", "2"))
os.environ.setdefault("CUDA_VISIBLE_DEVICES", "-1")
for v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ.setdefault(v, str(THREADS))

import numpy as np  # noqa: E402
import scipy.io.wavfile as wavfile  # noqa: E402
from scipy.ndimage import median_filter  # noqa: E402
from scipy.signal import butter, hilbert, lfilter, sosfiltfilt  # noqa: E402
import torch  # noqa: E402

torch.set_num_threads(THREADS)

# ------------------------------------------------------------------ net.rs's constants

SR = 48000
FRAME = 192
FPS = SR / FRAME
HARMS = 48
BANDS = 16
T_MAX = 3.0
T_KNEE = 0.01
NYQ = 20000.0
BAND_Q = 1.5
MAGIC = b"MBN1"
MAGIC_VIB = b"MBN2"
VIB_MAX = 150.0
NOUT = 1 + HARMS + BANDS
HOLD = 144000  # 3 s held, as --voice-data plays a note
NFR = HOLD // FRAME  # 750 frames
RES = (2048, 1024, 512, 256)
EPS = 1e-6
SUS = slice(125, NFR)  # a held note's last 2.5 s
VIB_BAND = (3.5, 8.5)

HOME = Path.home()
REPO = Path(__file__).resolve().parents[2]


def band_hz(b):
    return np.float32(80.0) * np.float32(200.0) ** np.float32(b / (BANDS - 1))


def f0_of(pitch):
    return float(np.float32(440.0) * np.exp2(np.float32((pitch - 69.0) / 12.0)))


def top_of(pitch):
    """How many harmonics sound (net.rs: those under NYQ)."""
    return int(np.clip(math.ceil(NYQ / max(f0_of(pitch), 1.0)), 1, HARMS + 1)) - 1


def t_in(t):
    t = np.clip(np.asarray(t, np.float32), 0.0, T_MAX)
    return (np.log(np.float32(1.0) + t / np.float32(T_KNEE)) / np.float32(math.log(1.0 + T_MAX / T_KNEE))).astype(
        np.float32
    )


# ------------------------------------------------------------------ noise, as net.rs makes it


def xorshift(seed, n):
    out = np.empty(n, np.uint32)
    s = seed & 0xFFFFFFFF
    for i in range(n):
        s ^= (s << 13) & 0xFFFFFFFF
        s ^= s >> 17
        s ^= (s << 5) & 0xFFFFFFFF
        out[i] = s
    return out


def white(seed, n):
    return xorshift(seed, n).view(np.int32).astype(np.float32) * np.float32(1.0 / 2147483648.0)


def svf_params():
    hz = np.array([band_hz(b) for b in range(BANDS)], np.float32)
    g = np.tan(np.float32(math.pi) * hz / np.float32(SR)).astype(np.float32)
    k = np.float32(1.0 / BAND_Q)
    a1 = (np.float32(1.0) / (np.float32(1.0) + g * (g + k))).astype(np.float32)
    return hz, g, a1, (g * a1).astype(np.float32), (g * g * a1).astype(np.float32)


def svf_literal(x, b):
    """Bp::tick, literally (float32), for checking svf_ba."""
    _, g, a1, a2, a3 = svf_params()
    a1, a2, a3 = a1[b], a2[b], a3[b]
    ic1 = ic2 = np.float32(0.0)
    y = np.empty(len(x), np.float32)
    for i, v in enumerate(x.astype(np.float32)):
        v3 = v - ic2
        v1 = a1 * ic1 + a2 * v3
        v2 = ic2 + a2 * ic1 + a3 * v3
        ic1 = np.float32(2.0) * v1 - ic1
        ic2 = np.float32(2.0) * v2 - ic2
        y[i] = v1
    return y


def svf_ba(b):
    """The band-pass's transfer function (the SVF is the bilinear transform's)."""
    _, g, _, _, _ = svf_params()
    g = float(g[b])
    k = 1.0 / BAND_Q
    d = 1.0 + k * g + g * g
    return np.array([g / d, 0.0, -g / d]), np.array([1.0, (2.0 * g * g - 2.0) / d, (1.0 - k * g + g * g) / d])


_NORMS = None


def band_norms():
    """net.rs's bands(): each filter's RMS for 2^16 samples of noise after 2^12, made 1."""
    global _NORMS
    if _NORMS is None:
        _, g, a1, a2, a3 = svf_params()
        w = white(0x2545F491, (1 << 12) + (1 << 16))
        ic1 = np.zeros(BANDS, np.float32)
        ic2 = np.zeros(BANDS, np.float32)
        acc = np.zeros(BANDS)
        two = np.float32(2.0)
        for i, x in enumerate(w):
            v3 = x - ic2
            v1 = a1 * ic1 + a2 * v3
            v2 = ic2 + a2 * ic1 + a3 * v3
            ic1 = two * v1 - ic1
            ic2 = two * v2 - ic2
            if i >= 1 << 12:
                acc += v1.astype(np.float64) ** 2
        _NORMS = (np.float32(1.0) / np.maximum(np.sqrt(acc / 65536.0).astype(np.float32), np.float32(1e-9))).astype(
            np.float32
        )
    return _NORMS


_BANKS = {}


def noise_bank(seed, n):
    """Each band's normalised output for the white noise of `seed` (seed | 1, as a voice's)."""
    key = (seed, n)
    if key not in _BANKS:
        w = white(seed | 1, n).astype(np.float64)
        norms = band_norms()
        y = np.empty((BANDS, n), np.float32)
        for b in range(BANDS):
            bb, aa = svf_ba(b)
            y[b] = norms[b] * lfilter(bb, aa, w)
        _BANKS[key] = y
    return _BANKS[key]


def band_response(freqs):
    """norm_b H_b(f) (complex), bands by frequencies."""
    z1 = np.exp(-2j * np.pi * np.asarray(freqs) / SR)
    norms = band_norms()
    g = np.empty((BANDS, len(z1)), np.complex128)
    for b in range(BANDS):
        bb, aa = svf_ba(b)
        g[b] = norms[b] * (bb[0] + bb[2] * z1 * z1) / (1 + aa[1] * z1 + aa[2] * z1 * z1)
    return g


SIGMA_W2 = 1.0 / 3.0  # the white noise's variance (uniform in [-1, 1))

# ------------------------------------------------------------------ MBN1 and MBN2 files


def write_mbn(path, layers):
    """layers: [(W (out, in), b (out,))]; MBN2 when the last has the vibrato's two outputs."""
    out = layers[-1][0].shape[0]
    assert out in (NOUT, NOUT + 2)
    v = bytearray(MAGIC_VIB if out == NOUT + 2 else MAGIC)
    v += struct.pack("<4I", 3, HARMS, BANDS, len(layers))
    for w, _ in layers:
        v += struct.pack("<2I", w.shape[1], w.shape[0])
    for w, b in layers:
        v += np.ascontiguousarray(w, "<f4").tobytes()
        v += np.ascontiguousarray(b, "<f4").tobytes()
    Path(path).write_bytes(bytes(v))
    return len(v)


def read_mbn(path_or_bytes):
    b = path_or_bytes if isinstance(path_or_bytes, (bytes, bytearray)) else Path(path_or_bytes).read_bytes()
    if b[:4] not in (MAGIC, MAGIC_VIB):
        raise ValueError("not a musicbox network")
    vib = b[:4] == MAGIC_VIB
    inp, harms, bands, n = struct.unpack_from("<4I", b, 4)
    if (inp, harms, bands) != (3, HARMS, BANDS) or not 1 <= n <= 8:
        raise ValueError(f"{inp} in, {harms} harmonics, {bands} bands, {n} layers")
    at = 20
    shapes = []
    for _ in range(n):
        shapes.append(struct.unpack_from("<2I", b, at))
        at += 8
    layers = []
    want = 3
    for i, o in shapes:
        if i != want:
            raise ValueError(f"a layer takes {i} from {want}")
        w = np.frombuffer(b, "<f4", i * o, at).reshape(o, i).astype(np.float32)
        at += 4 * i * o
        bias = np.frombuffer(b, "<f4", o, at).astype(np.float32)
        at += 4 * o
        layers.append((w, bias))
        want = o
    if want != NOUT + 2 * vib:
        raise ValueError(f"{want} out")
    if at != len(b):
        raise ValueError("trailing bytes")
    return layers


def kind_of(layers):
    return "MBN2" if layers[-1][0].shape[0] == NOUT + 2 else "MBN1"


# ------------------------------------------------------------------ net.rs's synthesis, in numpy


def forward_np(layers, x):
    x = x.astype(np.float32)
    for i, (w, b) in enumerate(layers):
        x = x @ w.T + b
        if i < len(layers) - 1:
            x = np.maximum(x, 0)
    return x.astype(np.float32)


def decode_np(out, pitch):
    """Net::frame after the layers: harmonic amplitudes (n, HARMS), band
    amplitudes (n, BANDS), the vibrato (n, 2: depth in cents, rate in Hz)."""
    top = top_of(pitch)
    amp = np.exp(np.minimum(out[:, 0], np.float32(4.0))).astype(np.float32)
    lg = out[:, 1 : 1 + HARMS]
    most = lg[:, : max(top, 1)].max(1, keepdims=True)
    h = np.where(np.arange(HARMS) < top, np.exp(lg - most), np.float32(0.0)).astype(np.float32)
    s = h.sum(1, keepdims=True, dtype=np.float32)
    h = (h * (amp[:, None] / np.maximum(s, np.float32(1e-12)))).astype(np.float32)
    nb = np.exp(np.minimum(out[:, 1 + HARMS : 1 + HARMS + BANDS], np.float32(4.0))).astype(np.float32)
    if out.shape[1] == NOUT + 2:
        y = out[:, NOUT:]
        vb = np.stack(
            [
                np.exp(np.minimum(y[:, 0], np.float32(math.log(VIB_MAX)))),
                np.float32(4.0) + np.float32(4.0) / (np.float32(1.0) + np.exp(-y[:, 1])),
            ],
            1,
        ).astype(np.float32)
    else:
        vb = np.tile(np.array([[0.0, 5.0]], np.float32), (len(out), 1))
    return h, nb, vb


def controls_np(layers, pitch, vel, nframes=NFR + 1):
    """The controls at frames 0..nframes-1 (t = j * 4 ms)."""
    t = np.arange(nframes, dtype=np.float32) * np.float32(FRAME / SR)
    x = np.stack([np.full(nframes, (pitch - 60.0) / 24.0), np.full(nframes, vel), t_in(t)], 1).astype(np.float32)
    return decode_np(forward_np(layers, x), pitch)


def ramp(c, n):
    """Per-sample values of frame controls c (frames, ...), ramped as NetVoice::fill does."""
    i = np.arange(n)
    j = i // FRAME
    need = j[-1] + 2
    if len(c) < need:
        c = np.concatenate([c, np.repeat(c[-1:], need - len(c), 0)])
    r = ((i % FRAME) / FRAME).astype(np.float32).reshape((-1,) + (1,) * (c.ndim - 1))
    c0, c1 = c[j], c[j + 1]
    return c0 + (c1 - c0) * r, c0


def phase_np(pitch, n, vb=None):
    """The note's phase (cycles) at each sample, its vibrato from phase 0."""
    dphi = np.float32(f0_of(pitch)) / np.float32(SR)
    if vb is None:
        return np.mod(np.arange(n) * float(dphi), 1.0)
    v, _ = ramp(vb, n)
    depth, rate = v[:, 0].astype(np.float64), v[:, 1].astype(np.float64)
    on = depth > 0
    vph = np.cumsum(np.where(on, rate / SR, 0.0))  # advanced before it is used
    y = depth * np.sin(2 * np.pi * vph) * (math.log(2) / 1200)
    step = float(dphi) * np.where(on, 1 + y + 0.5 * y * y, 1.0)
    return np.mod(np.concatenate([[0.0], np.cumsum(step[:-1])]), 1.0)


def render_np(h, nb, pitch, n, seed=12345, vb=None):
    """NetVoice::fill for n samples from frame controls h, nb, vb (vibrato; None: none)."""
    a, h0 = ramp(h, n)
    # the loop stops at the first harmonic silent at both ends of its ramp
    keep = np.cumsum((a == 0) & (h0 == 0), 1) == 0
    a = a * keep
    phase = phase_np(pitch, n, vb)
    x = np.zeros(n)
    for k in range(HARMS):
        if not a[:, k].any():
            continue
        x += a[:, k] * np.sin(2 * np.pi * (k + 1) * phase)
    an, _ = ramp(nb, n)
    x += np.einsum("nb,bn->n", an, noise_bank(seed, n)[:, :n])
    return x.astype(np.float32)


# ------------------------------------------------------------------ measures


def stft_mag(x, n):
    hop = n // 4
    w = np.hanning(n + 1)[:-1]
    xp = np.pad(x.astype(np.float64), (n // 2, n // 2))
    m = 1 + (len(xp) - n) // hop
    fr = np.lib.stride_tricks.as_strided(xp, (m, n), (xp.strides[0] * hop, xp.strides[0]))
    s = np.abs(np.fft.rfft(fr * w, axis=1)) / w.sum()
    return s[:, : int(NYQ / (SR / n))]


def mrlsd(x, ref):
    d = []
    for n in RES:
        a = 20 * np.log10(stft_mag(x, n) + EPS)
        b = 20 * np.log10(stft_mag(ref, n) + EPS)
        d.append(np.abs(a - b).mean())
    return float(np.mean(d))


def level_db(x):
    return 10 * math.log10(float(np.mean(np.asarray(x, np.float64) ** 2)) + 1e-12)


def env_db(x, w=2400):
    m = len(x) // w
    return 10 * np.log10((np.asarray(x[: m * w], np.float64).reshape(m, w) ** 2).mean(1) + 1e-12)


def env_err(x, ref):
    a, b = env_db(x), env_db(ref)
    on = np.maximum(a, b) > b.max() - 40
    return float(np.abs(a - b)[on].mean())


# ------------------------------------------------------------------ the data


def manifest():
    """The bank's manifest rows (instrument, articulation, vel_lo, vel_hi), if it is here."""
    for d in (HOME / ".local/share/musicbox/samples", HOME / ".local/share/fleece/samples"):
        m = d / "manifest.tsv"
        if m.exists():
            rows = []
            for line in m.read_text().splitlines():
                f = line.split("\t")
                if line.startswith("#") or len(f) < 5 or f[0] == "instrument":
                    continue
                try:
                    rows.append((f[0], f[1], int(f[3]), int(f[4])))
                except ValueError:
                    continue
            return rows
    return []


def index_entry(name):
    """A --voice-data stem's index entry: multi_folk_harp_pluck is (multi, folk_harp, pluck)."""
    kind, _, rest = name.partition("_")
    if kind == "multi":
        w = rest.split("_")
        splits = [("_".join(w[:i]), "_".join(w[i:])) for i in range(1, len(w))]
        known = {(r[0], r[1]) for r in manifest()}
        inst, art = next((s for s in splits if s in known), splits[-1])
        return ("multi", inst, art)
    return ("gm", rest.replace("_", " "), "")


def layers_of(name):
    """The instrument's velocity layers (MIDI, lo-hi), from the manifest; None if it is not there."""
    kind, inst, art = index_entry(name)
    if kind != "multi":
        return None
    ls = sorted({(lo, hi) for i, a, lo, hi in manifest() if (i, a) == (inst, art)})
    return ls or None


def load_data(data_dir, name):
    sr, pcm = wavfile.read(data_dir / f"{name}.wav")
    assert sr == SR and pcm.ndim == 1, f"{name}.wav: want mono {SR} Hz"
    x = pcm.astype(np.float32) / 32768.0
    rows = []
    for line in (data_dir / f"{name}.tsv").read_text().splitlines()[1:]:
        f = line.split("\t")
        if len(f) >= 4:
            rows.append(dict(note=int(f[0]), vel=float(f[1]), first=int(f[2]), held=int(f[3])))
    # the same recording at several velocities (one velocity layer): analysed once
    uniq = []
    by_hash = {}
    for r in rows:
        assert r["held"] >= HOLD
        seg = x[r["first"] : r["first"] + HOLD]
        hsh = hashlib.sha1(seg.tobytes()).hexdigest()
        if hsh not in by_hash:
            by_hash[hsh] = len(uniq)
            uniq.append(dict(note=r["note"], vels=[], first=r["first"]))
        r["u"] = by_hash[hsh]
        uniq[r["u"]]["vels"].append(r["vel"])
    # each recording's velocities: its layer's (the sampler's vel * 127, rounded), else halfway to the next's
    vl = layers_of(name)
    midi = lambda v: int(round(v * 127))  # noqa: E731
    for note in sorted({u["note"] for u in uniq}):
        us = sorted((u for u in uniq if u["note"] == note), key=lambda u: min(u["vels"]))
        layer = lambda u: [L for L in (vl or []) if any(L[0] <= midi(v) <= L[1] for v in u["vels"])]  # noqa: E731
        for i, u in enumerate(us):
            lo = 0.0 if i == 0 else 0.5 * (max(us[i - 1]["vels"]) + min(u["vels"]))
            hi = 1.0 if i == len(us) - 1 else 0.5 * (max(u["vels"]) + min(us[i + 1]["vels"]))
            mine = layer(u)
            if mine:
                llo = max(0.0, (min(L[0] for L in mine) - 0.5) / 127)
                lhi = min(1.0, (max(L[1] for L in mine) + 0.5) / 127)
                if not any(set(map(tuple, layer(o))) & set(map(tuple, mine)) for o in us if o is not u):
                    # the only recording in its layer: the whole layer's velocities
                    lo, hi = llo, lhi
                else:
                    lo, hi = max(lo, llo), min(hi, lhi)
            u["vlo"], u["vhi"] = lo + 0.004, hi - 0.004
    return x, rows, uniq


def segment(x, first, pre, post):
    a, b = first - pre, first + HOLD + post
    out = np.zeros(b - a, np.float32)
    s, e = max(a, 0), min(b, len(x))
    out[s - a : e - a] = x[s:e]
    return out


# ------------------------------------------------------------------ analysis

PAD = 8192
NG = 56
GRID = 50.0 * (20000.0 / 50.0) ** (np.arange(NG) / (NG - 1))


def frames_of(seg, n, centres):
    idx = centres[:, None] - n // 2 + np.arange(n)[None, :]
    return seg[idx]


def bh4(n):
    t = 2 * np.pi * np.arange(n) / n
    return 0.35875 - 0.48829 * np.cos(t) + 0.14128 * np.cos(2 * t) - 0.01168 * np.cos(3 * t)


def analyse(seg, pitch):
    """seg: the note from PAD before its start to PAD after its held part.
    Returns per frame (NFR): harmonic amplitudes (NFR, HARMS), the noise's
    PSD on GRID (NFR, NG), the pitch track (cents)."""
    f0n = f0_of(pitch)
    kmod = top_of(pitch)
    centres = PAD + FRAME * np.arange(NFR)
    # harmonics: Hann, about six periods
    n1 = int(np.clip(round(6 * SR / f0n), 512, 2048)) // 2 * 2
    w1 = np.hanning(n1 + 1)[:-1]
    nfft = 8192
    bin_hz = SR / nfft
    X = np.fft.rfft(frames_of(seg, n1, centres) * w1, nfft, axis=1)
    P = X.real**2 + X.imag**2
    M = np.sqrt(P)
    # the pitch, within 80 cents: harmonic sum over the first dozen
    cents = np.arange(-80.0, 81.0, 2.0)
    K = min(12, kmod)
    fk = f0n * 2 ** (cents[:, None] / 1200) * np.arange(1, K + 1)[None, :]
    pos = fk / bin_hz
    i0 = np.floor(pos).astype(int)
    fr = pos - i0
    sal = (M[:, i0] * (1 - fr) + M[:, i0 + 1] * fr).sum(-1)
    best = sal.argmax(1)
    c = cents[best].copy()
    inner = (best > 0) & (best < len(cents) - 1)
    ym, y0, yp = (sal[np.arange(NFR), np.clip(best + d, 0, len(cents) - 1)] for d in (-1, 0, 1))
    den = ym - 2 * y0 + yp
    c[inner] += 2.0 * np.clip(0.5 * (ym - yp)[inner] / np.where(den[inner] == 0, -1, den[inner]), -1, 1)
    e = P.sum(1)
    loud = e > 1e-3 * e.max()
    c = median_filter(c, 7, mode="nearest")
    c[~loud] = np.median(c[loud]) if loud.any() else 0.0
    f0 = f0n * 2 ** (c / 1200)
    # each harmonic's energy within half a harmonic
    cs = np.concatenate([np.zeros((NFR, 1)), np.cumsum(P, 1)], 1)

    def cum(fpos):
        p = np.clip(fpos / bin_hz + 0.5, 0, P.shape[1] - 1e-6)
        i = np.floor(p).astype(int)
        f = p - i
        rr = np.arange(NFR)[:, None]
        return cs[rr, i] * (1 - f) + cs[rr, i + 1] * f

    ks = np.arange(1, kmod + 1)[None, :]
    E = cum((ks + 0.5) * f0[:, None]) - cum((ks - 0.5) * f0[:, None])
    # the noise: Blackman-Harris (sidelobes under -92 dB), about ten periods, harmonics masked
    n2 = int(np.clip(round(10 * SR / f0n), 1024, 4096)) // 2 * 2
    w2 = bh4(n2)
    nfft2 = 16384
    bin2 = SR / nfft2
    X2 = np.fft.rfft(frames_of(seg, n2, centres) * w2, nfft2, axis=1)
    S2 = (X2.real**2 + X2.imag**2) / (w2**2).sum()  # the PSD (white noise of variance s2: s2)
    f2 = np.arange(S2.shape[1]) * bin2
    lobe = 4.0 * SR / n2
    S = np.empty((NFR, NG))
    gi = np.clip(np.round(np.log(np.maximum(f2, 1.0) / GRID[0]) / np.log(GRID[1] / GRID[0])).astype(int), -1, NG)
    valid_bin = (gi >= 0) & (gi < NG)
    for j in range(NFR):
        # distance to the nearest modelled harmonic
        h = f2 / f0[j]
        kn = np.clip(np.round(h), 1, kmod)
        dist = np.abs(f2 - kn * f0[j])
        free = (dist > lobe + 0.003 * kn * f0[j]) & valid_bin
        num = np.bincount(gi[free], S2[j, free], NG)
        cnt = np.bincount(gi[free], None, NG)
        ok = cnt > 0
        if ok.sum() >= 2:
            S[j] = np.exp(np.interp(np.log(GRID), np.log(GRID[ok]), np.log(num[ok] / cnt[ok] + 1e-30)))
        else:
            S[j] = 1e-30
    # the harmonics' energy less the noise under them; amplitudes
    s1 = (w1**2).sum()
    under = np.stack([np.exp(np.interp(np.log(ks[0] * f0[j]), np.log(GRID), np.log(S[j] + 1e-30))) for j in range(NFR)])
    E = np.maximum(E - under * s1 * (f0[:, None] / bin_hz), 0.0)
    A = np.sqrt(4.0 * E / (nfft * s1))
    # sharpened in time: a three-period window's energy over each window's own
    n3 = int(max(384, round(3 * SR / f0n))) // 2 * 2
    w3 = np.hanning(n3 + 1)[:-1]

    def energy(n, w):
        f = frames_of(seg, n, centres).astype(np.float64)
        return (f**2 * w**2).sum(1) / (w**2).sum()

    es = energy(n3, w3)
    c1 = np.sqrt((es + 1e-14) / (energy(n1, w1) + 1e-14)).clip(0.03, 3.0)
    c2 = np.sqrt((es + 1e-14) / (energy(n2, w2) + 1e-14)).clip(0.03, 3.0)
    A = A * c1[:, None]
    S = S * (c2**2)[:, None]
    # frames whose windows run past the held part: the last whole one's
    for n in (n1, n2, n3):
        last = (HOLD - n // 2) // FRAME
        A[last + 1 :] = A[last]
        S[last + 1 :] = S[last]
    Afull = np.zeros((NFR, HARMS), np.float32)
    Afull[:, :kmod] = A
    return Afull, S.astype(np.float32), c.astype(np.float32)


def fit_noise(S, iters=400):
    """Band amplitudes (n, BANDS) whose noise has PSD S (n, NG) on GRID, in the log domain."""
    G = torch.tensor(band_response(GRID), dtype=torch.complex64)  # (BANDS, NG)
    St = torch.tensor(S, dtype=torch.float32)
    lt = torch.log(St + 1e-13)
    # a start: each band alone at its centre
    centre = np.array([band_hz(b) for b in range(BANDS)], np.float64)
    at_c = np.stack([np.exp(np.interp(np.log(centre), np.log(GRID), np.log(S[j] + 1e-30))) for j in range(len(S))])
    peak = np.abs(np.diag(band_response(centre)))
    th = torch.tensor(np.log(np.sqrt(at_c / SIGMA_W2) / peak * 0.6 + 1e-9), dtype=torch.float32, requires_grad=True)
    opt = torch.optim.Adam([th], lr=0.05)
    for it in range(iters):
        a = torch.exp(th).to(torch.complex64)
        m = SIGMA_W2 * (a @ G).abs() ** 2
        fit = ((torch.log(m + 1e-13) - lt) ** 2).mean()
        # neighbouring bands overlap: of the ways to make a spectrum, the smoothest
        loss = fit + 0.02 * ((th[:, 1:] - th[:, :-1]) ** 2).mean()
        opt.zero_grad()
        loss.backward()
        opt.step()
    return torch.exp(th).detach().numpy().astype(np.float32), float(fit.detach()) ** 0.5 * 10 / math.log(10)


def vibrato(cents, A):
    """From a note's pitch track (cents, per frame): the vibrato's depth
    (cents, its peak), rate (Hz) and phase (cycles), the slower intonation
    (cents), and the share of the held part's wavering in the vibrato band."""
    tot = A.sum(1)
    loud = tot > 0.1 * tot.max()
    c = cents.astype(np.float64).copy()
    if loud.any():
        c[~loud] = np.median(c[loud])
    slow = sosfiltfilt(butter(2, 2.5, "low", fs=FPS, output="sos"), c)
    v = sosfiltfilt(butter(2, VIB_BAND, "band", fs=FPS, output="sos"), c)
    z = hilbert(v)
    depth = np.abs(z)
    ph = np.unwrap(np.angle(z)) / (2 * np.pi)
    rate = median_filter(np.gradient(ph) * FPS, 25, mode="nearest")
    share = float(np.var(v[SUS]) / max(np.var((c - slow)[SUS]), 1e-9))
    return depth.astype(np.float32), rate.astype(np.float32), ph, slow, share


def analyse_all(name, x, uniq, work):
    cache = work / f"{name}.analysis.npz"
    if cache.exists():
        z = np.load(cache)
        if z["A"].shape[0] == len(uniq):
            return z["A"], z["N"], z["cents"]
    t0 = time.time()
    As, Ss, Cs = [], [], []
    for u in uniq:
        seg = segment(x, u["first"], PAD, PAD)
        A, S, c = analyse(seg, u["note"])
        As.append(A)
        Ss.append(S)
        Cs.append(c)
    S = np.stack(Ss)
    nb, loss = fit_noise(S.reshape(-1, NG))
    nb = nb.reshape(len(uniq), NFR, BANDS)
    # the noise's per-frame randomness smoothed out (20 ms)
    nb = np.exp(median_filter(np.log(nb + 1e-12), (1, 5, 1), mode="nearest")).astype(np.float32)
    print(f"  noise fit: RMS error {loss:.2f} dB of PSD; analysis {time.time() - t0:.0f} s")
    A = np.stack(As)
    np.savez_compressed(cache, A=A, N=nb, cents=np.stack(Cs))
    return A, nb, np.stack(Cs)


class Vib:
    """Each recording's vibrato, frame by frame, and whether it is worth playing."""

    def __init__(self, A, cents):
        out = [vibrato(c, a) for c, a in zip(cents, A)]
        self.depth = np.stack([o[0] for o in out])
        self.rate = np.stack([o[1] for o in out])
        self.phase = np.stack([o[2] for o in out])
        self.slow = np.stack([o[3] for o in out])
        self.share = np.array([o[4] for o in out])
        self.sus_depth = float(np.median(np.median(self.depth[:, SUS], 1)))
        r = self.rate[:, SUS][self.depth[:, SUS] > 3]
        self.sus_rate = float(np.median(r)) if r.size else float("nan")
        self.worth = self.sus_depth >= 3.0 and float(np.median(self.share)) >= 0.7


# ------------------------------------------------------------------ the network (torch)

FL_TOT = math.log(1e-4)
FL_SH = math.log(1e-4)
FL_N = math.log(1e-5)


def make_net(hidden, nout):
    dims = [3] + hidden + [nout]
    mods = []
    for i in range(len(dims) - 1):
        mods.append(torch.nn.Linear(dims[i], dims[i + 1]))
        if i < len(dims) - 2:
            mods.append(torch.nn.ReLU())
    return torch.nn.Sequential(*mods)


def net_layers(net):
    return [
        (m.weight.detach().numpy().astype(np.float32), m.bias.detach().numpy().astype(np.float32))
        for m in net
        if isinstance(m, torch.nn.Linear)
    ]


class Targets:
    """The analysis as training rows: recording u, frame j."""

    def __init__(self, uniq, A, N, vib=None):
        self.uniq = uniq
        self.pitch = np.array([u["note"] for u in uniq], np.float32)
        self.vlo = np.array([u["vlo"] for u in uniq], np.float32)
        self.vhi = np.array([u["vhi"] for u in uniq], np.float32)
        tot = A.sum(-1)
        self.T0 = np.log(tot + 1e-12).astype(np.float32)
        self.Ts = np.log(A / (tot[..., None] + 1e-12) + 1e-12).astype(np.float32)
        self.Tn = np.log(N + 1e-12).astype(np.float32)
        tops = np.array([top_of(p) for p in self.pitch])
        self.mask = np.arange(HARMS)[None, :] < tops[:, None]
        # the shares matter in proportion to how loud the frame is (to 30 dB under the note's peak)
        self.wsh = np.clip(tot / (0.03 * tot.max(1, keepdims=True)), 0, 1).astype(np.float32)
        self.T0t, self.Tst, self.Tnt = (torch.tensor(a) for a in (self.T0, self.Ts, self.Tn))
        self.maskt = torch.tensor(self.mask)
        self.wsht = torch.tensor(self.wsh)
        self.vib = vib
        if vib is not None:
            self.Td = torch.tensor(np.log(np.maximum(vib.depth, 0.05)).astype(np.float32))
            self.Tr = torch.tensor(np.clip(vib.rate, 4.05, 7.95).astype(np.float32))
            # the rate matters where there is a vibrato to have one
            self.wr = torch.tensor(np.clip((vib.depth - 2.0) / 6.0, 0, 1).astype(np.float32))
        # past the range: the end notes held (the network has no other way to know)
        self.ends = [np.flatnonzero(self.pitch == self.pitch.min()), np.flatnonzero(self.pitch == self.pitch.max())]

    @property
    def nout(self):
        return NOUT + 2 * (self.vib is not None)

    def inputs(self, u, j, vel, shift=0.0):
        return torch.stack(
            [
                torch.tensor((self.pitch[u] + shift - 60.0) / 24.0),
                torch.as_tensor(vel, dtype=torch.float32),
                torch.tensor(t_in(j * FRAME / SR)),
            ],
            -1,
        ).float()

    def batch(self, rng, n, attack=0.5):
        u = rng.integers(0, len(self.uniq), n)
        j = rng.integers(0, NFR, n)
        na = int(n * attack)
        j[:na] = rng.integers(0, 100, na)  # the first 0.4 s, twice as often
        shift = np.zeros(n, np.float32)
        out = rng.random(n) < 0.08
        side = rng.random(n) < 0.5
        for k, sel in enumerate((out & side, out & ~side)):
            u[sel] = rng.choice(self.ends[k], sel.sum())
            shift[sel] = (1 if k else -1) * rng.uniform(0.5, 12.0, sel.sum())
        vel = self.vlo[u] + (self.vhi[u] - self.vlo[u]) * rng.random(n, dtype=np.float32)
        return u, j, self.inputs(u, j, vel, shift)


def sup_loss(out, T, u, j):
    o0 = out[:, 0]
    lg = out[:, 1 : 1 + HARMS]
    on = out[:, 1 + HARMS : NOUT]
    mask = T.maskt[u]
    lgm = lg.masked_fill(~mask, -1e4)
    ls = lgm - torch.logsumexp(lgm, 1, keepdim=True)
    fl = torch.logaddexp
    t_tot = torch.tensor(FL_TOT)
    t_sh = torch.tensor(FL_SH)
    t_n = torch.tensor(FL_N)
    l_tot = ((fl(o0, t_tot) - fl(T.T0t[u, j], t_tot)) ** 2).mean()
    w = T.wsht[u, j][:, None] * mask
    l_sh = (((fl(ls, t_sh) - fl(T.Tst[u, j], t_sh)) ** 2) * w).sum() / w.sum().clamp(min=1)
    l_n = ((fl(on, t_n) - fl(T.Tnt[u, j], t_n)) ** 2).mean()
    loss = 4 * l_tot + l_sh + 0.5 * l_n
    parts = [float(l_tot.detach()), float(l_sh.detach()), float(l_n.detach())]
    if T.vib is not None:
        # depth in the log domain above a cent; the rate (Hz) where there is a vibrato
        zero = torch.tensor(0.0)
        od = out[:, NOUT].clamp(max=math.log(VIB_MAX))
        l_d = ((fl(od, zero) - fl(T.Td[u, j], zero)) ** 2).mean()
        wr = T.wr[u, j]
        rate = 4.0 + 4.0 * torch.sigmoid(out[:, NOUT + 1])
        l_r = (((rate - T.Tr[u, j]) ** 2) * wr).sum() / wr.sum().clamp(min=1)
        loss = loss + l_d + 0.5 * l_r
        parts += [float(l_d.detach()), float(l_r.detach())]
    return loss, parts


def train_sup(T, hidden, steps, seed=1, log=print):
    torch.manual_seed(seed)
    rng = np.random.default_rng(seed)
    net = make_net(hidden, T.nout)
    last = net[-1]
    with torch.no_grad():
        last.weight.mul_(0.1)
        last.bias[0] = float(np.mean(np.maximum(T.T0, FL_TOT)))
        sh = np.where(T.mask[:, None, :], np.maximum(T.Ts, FL_SH), np.nan)
        last.bias[1 : 1 + HARMS] = torch.tensor(np.nan_to_num(np.nanmean(sh, (0, 1)), nan=FL_SH))
        last.bias[1 + HARMS : NOUT] = torch.tensor(np.maximum(T.Tn, FL_N).mean((0, 1)))
        if T.vib is not None:
            last.bias[NOUT] = math.log(max(T.vib.sus_depth, 1.0))
            r = T.vib.sus_rate if T.vib.sus_rate == T.vib.sus_rate else 5.5
            q = min(max((r - 4.0) / 4.0, 0.05), 0.95)
            last.bias[NOUT + 1] = math.log(q / (1 - q))
    opt = torch.optim.Adam(net.parameters(), lr=3e-3)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, 3e-3, total_steps=steps, pct_start=0.05, final_div_factor=100)
    t0 = time.time()
    for s in range(steps):
        u, j, x = T.batch(rng, 2048)
        loss, parts = sup_loss(net(x), T, u, j)
        opt.zero_grad()
        loss.backward()
        opt.step()
        sched.step()
        if s % 3000 == 0 or s == steps - 1:
            log(f"  supervised {s:6d}: {float(loss):.4f} ({', '.join(f'{p:.4f}' for p in parts)}) {time.time() - t0:.0f} s")
    return net


# ------------------------------------------------------------------ end to end: net.rs's synthesis in torch


def stft_t(x, n):
    w = torch.hann_window(n, periodic=True, dtype=x.dtype)
    s = torch.stft(x, n, n // 4, window=w, center=True, pad_mode="constant", return_complex=True).abs() / w.sum()
    return s[..., : int(NYQ / (SR / n)), :]


def mrlsd_t(x, ref):
    d = 0
    for n in RES:
        a = 20 * torch.log10(stft_t(x, n) + EPS)
        b = 20 * torch.log10(stft_t(ref, n) + EPS)
        d = d + (a - b).abs().mean()
    return d / len(RES)


def loud_t(x, ref, n=512):
    """The loudness envelope's mean |error| (dB, blocks of n)."""
    m = x.shape[-1] // n
    a = 10 * torch.log10((x[..., : m * n].reshape(*x.shape[:-1], m, n) ** 2).mean(-1) + 1e-10)
    b = 10 * torch.log10((ref[..., : m * n].reshape(*ref.shape[:-1], m, n) ** 2).mean(-1) + 1e-10)
    return (a - b).abs().mean()


def train_e2e(net, T, x, minutes, steps_max=100000, seed=2, log=print, L=100, B=4, lr=2e-4):
    rng = np.random.default_rng(seed)
    torch.manual_seed(seed)
    Y = torch.tensor(noise_bank(7, HOLD + FRAME * 2))
    opt = torch.optim.Adam(net.parameters(), lr=lr)
    ns = L * FRAME
    r = torch.arange(ns) % FRAME / FRAME
    jj = torch.arange(ns) // FRAME
    segs = [torch.tensor(x[u["first"] : u["first"] + HOLD]) for u in T.uniq]
    ks = torch.arange(1, HARMS + 1, dtype=torch.float64)[None, :]
    t0 = time.time()
    hist = []
    s = 0
    while time.time() - t0 < minutes * 60 and s < steps_max:
        u = rng.integers(0, len(T.uniq), B)
        j0 = rng.integers(0, NFR - L, B)
        j0[: B // 4] = 0  # an attack a batch
        vel = T.vlo[u] + (T.vhi[u] - T.vlo[u]) * rng.random(B, dtype=np.float32)
        jg = j0[:, None] + np.arange(L + 1)[None, :]
        xin = T.inputs(np.repeat(u, L + 1), jg.ravel(), np.repeat(vel, L + 1)).reshape(B, L + 1, 3)
        out = net(xin)
        amp = torch.exp(out[..., 0].clamp(max=4.0))
        mask = T.maskt[u][:, None, :]
        lg = out[..., 1 : 1 + HARMS].masked_fill(~mask, -1e4)
        h = torch.softmax(lg, -1) * mask * amp[..., None]
        nb = torch.exp(out[..., 1 + HARMS : NOUT].clamp(max=4.0))
        hr = h[:, jj] + (h[:, jj + 1] - h[:, jj]) * r[None, :, None]
        nr = nb[:, jj] + (nb[:, jj + 1] - nb[:, jj]) * r[None, :, None]
        ys = []
        refs = []
        for b in range(B):
            n0 = int(j0[b]) * FRAME
            dphi = float(np.float32(f0_of(T.pitch[u[b]])) / np.float32(SR))
            if T.vib is None:
                ph = torch.remainder(torch.arange(n0, n0 + ns, dtype=torch.float64) * dphi, 1.0)
            else:
                # the network's depth (no gradient: it learns from the analysis), at the recording's
                # own vibrato phase and slow intonation: the same wobble as the recording's
                with torch.no_grad():
                    nf = int(j0[b]) + L + 1
                    pre = net(T.inputs(np.full(nf, u[b]), np.arange(nf), np.full(nf, vel[b], np.float32)))
                    depth = torch.exp(pre[:, NOUT].clamp(max=math.log(VIB_MAX))).double().numpy()
                n1 = n0 + ns
                depth_s, _ = ramp(depth, n1)
                tf = np.arange(n1) / FRAME
                psi = np.interp(tf, np.arange(NFR), T.vib.phase[u[b]])
                slow = np.interp(tf, np.arange(NFR), T.vib.slow[u[b]])
                cents = slow + depth_s * np.sin(2 * np.pi * psi)
                step = dphi * np.exp2(cents / 1200)
                phs = np.concatenate([[0.0], np.cumsum(step[:-1])])[n0:n1]
                ph = torch.tensor(np.mod(phs, 1.0))
            sn = torch.sin(2 * math.pi * ph[:, None] * ks).float()
            y = (hr[b] * sn).sum(-1) + (nr[b] * Y[:, n0 : n0 + ns].T).sum(-1)
            ys.append(y)
            refs.append(segs[u[b]][n0 : n0 + ns])
        y = torch.stack(ys)
        ref = torch.stack(refs)
        spec = mrlsd_t(y, ref)
        loud = loud_t(y, ref)
        loss = spec + loud
        # held near the analysis where the spectrum says little (and the vibrato to it)
        uu, j, xs = T.batch(rng, 512)
        reg, _ = sup_loss(net(xs), T, uu, j)
        opt.zero_grad()
        (loss + 0.05 * reg).backward()
        torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
        opt.step()
        for g in opt.param_groups:
            g["lr"] = lr * 0.5 * (1 + math.cos(math.pi * min(1.0, (time.time() - t0) / (minutes * 60))))
        hist.append((float(spec), float(loud)))
        if s % 500 == 0:
            hm = np.mean(hist[-500:], 0)
            log(f"  end to end {s:5d}: MR-LSD {hm[0]:.2f} dB, loudness {hm[1]:.2f} dB, reg {float(reg):.3f}, {time.time() - t0:.0f} s")
        s += 1
    return net, s


# ------------------------------------------------------------------ evaluation


def evaluate(name, x, rows, uniq, layers, A, N, vib=None, seed=12345, novib=False):
    """Per TSV row: the network, the analysis played as is, and the steady baseline."""
    res = []
    cache = {}
    has_vib = kind_of(layers) == "MBN2"
    for r in rows:
        u = r["u"]
        p, v = r["note"], r["vel"]
        ref = x[r["first"] : r["first"] + HOLD]
        h, nb, vb = controls_np(layers, p, v)
        y = render_np(h, nb, p, HOLD, seed, vb if has_vib else None)
        row = dict(note=p, vel=v, level_ref=level_db(ref), dlevel=level_db(y) - level_db(ref), env=env_err(y, ref), mrlsd=mrlsd(y, ref))
        if has_vib:
            row["net_depth"] = float(np.median(vb[SUS, 0]))
            row["net_rate"] = float(np.median(vb[SUS, 1]))
            if novib:
                row["novib_mrlsd"] = mrlsd(render_np(h, nb, p, HOLD, seed), ref)
        if vib is not None:
            row["rec_depth"] = float(np.median(vib.depth[u, SUS]))
            rr = vib.rate[u, SUS][vib.depth[u, SUS] > 3]
            row["rec_rate"] = float(np.median(rr)) if rr.size else float("nan")
        if u not in cache:
            vba = np.stack([vib.depth[u], vib.rate[u].clip(4.0, 8.0)], 1) if (vib is not None and has_vib) else None
            ya = render_np(A[u], N[u], p, HOLD, seed, vba)
            hb = np.repeat(A[u].mean(0, keepdims=True), 2, 0)
            yb = render_np(hb, np.repeat(N[u].mean(0, keepdims=True), 2, 0), p, HOLD, seed)
            cache[u] = dict(
                oracle_mrlsd=mrlsd(ya, ref),
                oracle_dlevel=level_db(ya) - level_db(ref),
                oracle_env=env_err(ya, ref),
                base_mrlsd=mrlsd(yb, ref),
                base_dlevel=level_db(yb) - level_db(ref),
                base_env=env_err(yb, ref),
            )
        row.update(cache[u])
        res.append(row)
    return res


def summarise(res):
    notes = sorted({r["note"] for r in res})
    lo, hi = notes[len(notes) // 3], notes[2 * len(notes) // 3]
    reg = lambda n: "low" if n < lo else ("mid" if n < hi else "high")  # noqa: E731
    out = {}
    groups = [("all", lambda r: True)]
    groups += [(f"vel {v:g}", (lambda v: lambda r: r["vel"] == v)(v)) for v in sorted({r["vel"] for r in res})]
    for g in ("low", "mid", "high"):
        nn = [n for n in notes if reg(n) == g]
        if nn:
            groups.append((f"{g} ({min(nn)}-{max(nn)})", (lambda g: lambda r: reg(r["note"]) == g)(g)))
    for key, sel in groups:
        rs = [r for r in res if sel(r)]
        m = lambda k: float(np.nanmean([r[k] for r in rs])) if rs and k in rs[0] else float("nan")  # noqa: E731
        out[key] = {
            "notes": len(rs),
            "net |dB|": float(np.mean([abs(r["dlevel"]) for r in rs])),
            "net env": m("env"),
            "net MR-LSD": m("mrlsd"),
            "no-vib MR-LSD": m("novib_mrlsd"),
            "analysis |dB|": float(np.mean([abs(r["oracle_dlevel"]) for r in rs])),
            "analysis MR-LSD": m("oracle_mrlsd"),
            "base |dB|": float(np.mean([abs(r["base_dlevel"]) for r in rs])),
            "base env": m("base_env"),
            "base MR-LSD": m("base_mrlsd"),
            "net depth": m("net_depth"),
            "rec depth": m("rec_depth"),
            "net rate": m("net_rate"),
            "rec rate": m("rec_rate"),
        }
    return out


def print_summary(name, s, log=print):
    cols = ["notes", "net |dB|", "net env", "net MR-LSD", "no-vib MR-LSD", "analysis MR-LSD", "base |dB|", "base env", "base MR-LSD"]
    log(f"{name}:")
    log("  " + f"{'':16s}" + "".join(f"{c:>16s}" for c in cols))
    for k, v in s.items():
        log("  " + f"{k:16s}" + "".join(f"{v[c]:16.2f}" if c != "notes" else f"{v[c]:16d}" for c in cols))


def listening(name, x, rows, layers, out_dir, seed=12345):
    notes = sorted({r["note"] for r in rows})
    picks = [notes[len(notes) // 6], notes[len(notes) // 2], notes[5 * len(notes) // 6]]
    vels = sorted({r["vel"] for r in rows})
    vels = [vels[0], vels[-1]]
    fade = np.minimum(1.0, (HOLD - np.arange(HOLD)) / 1440.0).astype(np.float32)
    has_vib = kind_of(layers) == "MBN2"
    files = []
    for p in picks:
        for v in vels:
            r = next(r for r in rows if r["note"] == p and r["vel"] == v)
            ref = x[r["first"] : r["first"] + HOLD] * fade
            h, nb, vb = controls_np(layers, p, v)
            y = render_np(h, nb, p, HOLD, seed, vb if has_vib else None) * fade
            gap = np.zeros(int(0.5 * SR), np.float32)
            both = np.concatenate([ref, gap, y, gap])
            g = min(1.0, 0.9 / max(1e-6, float(np.abs(both).max())))
            f = out_dir / f"{name}_n{p}_v{v:g}.wav"
            wavfile.write(f, SR, (both * g * 32767).astype(np.int16))
            files.append(str(f))
    return files


# ------------------------------------------------------------------ the index


def write_index(models, work, entries):
    """Puts each (kind, name, articulation) -> file of `entries` in models/index.tsv:
    its line replaced where it has one, else added; every other line kept as it is."""
    path = models / "index.tsv"
    with open(work / "index.lock", "w") as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        lines = path.read_text().splitlines() if path.exists() else []
        if not lines or not lines[0].startswith("kind\t"):
            lines.insert(0, "kind\tname\tarticulation\tfile")
        todo = dict(entries)
        for i, line in enumerate(lines[1:], 1):
            f = line.split("\t")
            if len(f) == 4 and tuple(f[:3]) in todo:
                lines[i] = "\t".join(f[:3]) + "\t" + todo.pop(tuple(f[:3]))
        lines += ["\t".join(k) + "\t" + v for k, v in todo.items()]
        tmp = path.with_suffix(".tsv.tmp")
        tmp.write_text("\n".join(lines) + "\n")
        os.replace(tmp, path)


# ------------------------------------------------------------------ self-tests


def flat_net(a, band, n, vib=None):
    """net.rs's tests' network: one layer, weights nothing; vib: (ln depth, rate logit)."""
    bias = [math.log(a)] + [10.0 if k == 0 else -30.0 for k in range(HARMS)]
    bias += [math.log(n) if k == band else -30.0 for k in range(BANDS)]
    if vib is not None:
        bias += list(vib)
    out = len(bias)
    v = (MAGIC_VIB if vib is not None else MAGIC) + struct.pack("<6I", 3, HARMS, BANDS, 1, 3, out)
    v += np.zeros(3 * out, "<f4").tobytes() + np.array(bias, "<f4").tobytes()
    return read_mbn(v)


def selftest():
    ok = True
    # the band-pass as lfilter against Bp::tick
    rng = np.random.default_rng(0)
    xin = rng.uniform(-1, 1, 3000).astype(np.float32)
    xin[0] = 1.0
    for b in (0, 7, 15):
        bb, aa = svf_ba(b)
        d = np.abs(lfilter(bb, aa, xin.astype(np.float64)) - svf_literal(xin, b)).max()
        print(f"band {b}: lfilter against tick, max difference {d:.2e}")
        ok &= d < 1e-4
    # net.rs's own tests
    L = flat_net(0.1, 0, 1e-9)
    h, nb, vb = controls_np(L, 69.0, 0.8, 24000 // FRAME + 2)
    y = render_np(h, nb, 69.0, 24000, 5)
    ups = int(((y[:-1] < 0) & (y[1:] >= 0)).sum())
    rms = float(np.sqrt(np.mean(y.astype(np.float64) ** 2)))
    print(f"flat net: {ups} cycles in half a second (220), RMS {rms:.4f} ({0.1 / math.sqrt(2):.4f})")
    ok &= abs(ups - 220) <= 1 and abs(rms - 0.1 / math.sqrt(2)) < 0.002
    L = flat_net(1e-9, 6, 0.05)
    h, nb, vb = controls_np(L, 69.0, 0.8, 24000 // FRAME + 2)
    y = render_np(h, nb, 69.0, 24000, 5)
    rms = float(np.sqrt(np.mean(y[4800:].astype(np.float64) ** 2)))
    print(f"flat net, band 6 at 0.05: RMS {rms:.4f}")
    ok &= abs(rms - 0.05) < 0.005
    L = flat_net(0.1, 0, 1e-9, (math.log(50.0), 0.0))
    h, nb, vb = controls_np(L, 69.0, 0.8, 48000 // FRAME + 2)
    y = render_np(h, nb, 69.0, 48000, 9, vb)
    ups = np.flatnonzero((y[:-1] < 0) & (y[1:] >= 0)) + 1
    f = 48000.0 / np.diff(ups)
    cents = 1200 * math.log2(f.max() / f.min())
    # the vibrato's rate: the period count's own wobble
    wob = np.abs(np.fft.rfft(f - f.mean()))
    fr = np.fft.rfftfreq(len(f), np.diff(ups).mean() / 48000)
    print(f"vibrato net: depth {vb[0, 0]:.2f}, rate {vb[0, 1]:.2f} Hz; {cents:.0f} cents lowest to highest (100), wobbling at {fr[wob.argmax()]:.2f} Hz")
    ok &= abs(vb[0, 0] - 50) < 0.01 and abs(vb[0, 1] - 6) < 0.01 and abs(cents - 100) < 15 and abs(fr[wob.argmax()] - 6) < 0.3
    # MBN2 files written and read back
    import tempfile

    with tempfile.TemporaryDirectory() as d:
        write_mbn(Path(d) / "x.mbn", L)
        ok &= (Path(d) / "x.mbn").read_bytes()[:4] == MAGIC_VIB and kind_of(read_mbn(Path(d) / "x.mbn")) == "MBN2"
    # the analysis recovers what the synthesis was given, the vibrato too
    for pitch, dep in ((48, 0.0), (69, 20.0), (96, 0.0)):
        top = top_of(pitch)
        a = np.zeros(HARMS, np.float32)
        a[:top] = 0.1 / np.arange(1, top + 1) ** 1.2
        nbt = (0.002 * np.exp(-0.5 * ((np.arange(BANDS) - 9) / 3.0) ** 2) + 1e-4).astype(np.float32)
        hh = np.repeat(a[None], NFR + 2, 0)
        nn = np.repeat(nbt[None], NFR + 2, 0)
        vv = np.repeat(np.array([[dep, 5.5]], np.float32), NFR + 2, 0) if dep else None
        y = render_np(hh, nn, pitch, HOLD + PAD, 99, vv)
        seg = np.concatenate([np.zeros(PAD, np.float32), y[:HOLD], y[HOLD : HOLD + PAD]])
        A, S, c = analyse(seg, pitch)
        mid = slice(200, 600)
        dh = 20 * np.log10(A[mid, :top].mean(0) / a[:top])
        d_, r_, _, _, share = vibrato(c, A)
        print(f"analysis at {pitch}: harmonics 1-4 {np.round(dh[:4], 2)} dB, mean |err| first 10 {np.abs(dh[:10]).mean():.2f} dB; "
              f"vibrato {dep:.0f} cents at 5.5 Hz found as {np.median(d_[mid]):.1f} cents at {np.median(r_[mid]):.2f} Hz (share {share:.2f})")
        ok &= np.abs(dh[:10]).mean() < 0.5 if not dep else np.abs(dh[:6]).mean() < 1.0
        if dep:
            ok &= abs(np.median(d_[mid]) - dep) < 0.15 * dep and abs(np.median(r_[mid]) - 5.5) < 0.2
    print("selftest", "passed" if ok else "FAILED")
    return ok


# ------------------------------------------------------------------ main


def table(names, work):
    """A compact table of the reports."""
    print(f"{'instrument':22s} {'kind':4s} {'|dB|':>5s} {'base':>5s} {'env':>5s} {'base':>5s} {'MR-LSD':>6s} {'novib':>6s} {'replay':>6s} {'base':>6s}"
          f" {'depth net/rec':>13s} {'rate net/rec':>13s} {'e2e?':>4s} {'min':>4s}")
    for n in names:
        p = work / f"{n}.json"
        if not p.exists():
            print(f"{n:22s} (no report)")
            continue
        r = json.loads(p.read_text())
        if "kind" not in r:
            print(f"{n:22s} (an older report)")
            continue
        a = r["summary"]["all"]
        f = lambda k, w=5: f"{a[k]:{w}.2f}" if a[k] == a[k] else " " * (w - 1) + "-"  # noqa: E731
        print(f"{n:22s} {r['kind']:4s} {f('net |dB|')} {f('base |dB|')} {f('net env')} {f('base env')} {f('net MR-LSD', 6)} {f('no-vib MR-LSD', 6)}"
              f" {f('analysis MR-LSD', 6)} {f('base MR-LSD', 6)} {a['net depth']:6.1f}/{a['rec depth']:<6.1f} {a['net rate']:6.2f}/{a['rec rate']:<6.2f}"
              f" {'yes' if r.get('picked') == 'end to end' else 'no':>4s} {r.get('total_s', 0) / 60:4.1f}")


def run(name, a, hidden, log):
    log(f"== {name} ({time.strftime('%H:%M:%S')}, pid {os.getpid()})")
    t_all = time.time()
    x, rows, uniq = load_data(a.data, name)
    log(f"  {len(rows)} notes, {len(uniq)} distinct recordings; manifest layers {layers_of(name)}; velocity ranges "
        + ", ".join(sorted({f"{u['vlo']:.3f}-{u['vhi']:.3f}" for u in uniq})))
    t0 = time.time()
    A, N, cents = analyse_all(name, x, uniq, a.work)
    t_an = time.time() - t0
    vib = Vib(A, cents)
    want_vib = vib.worth if a.vib == "auto" else a.vib == "on"
    log(f"  vibrato: held notes' depth median {vib.sus_depth:.1f} cents, rate {vib.sus_rate:.2f} Hz, "
        f"in-band share {np.median(vib.share):.2f}: {'MBN2' if want_vib else 'MBN1'}")
    T = Targets(uniq, A, N, vib if want_vib else None)
    report = dict(name=name, analysis_s=t_an, vib_rec_depth=vib.sus_depth, vib_rec_rate=vib.sus_rate, vib_share=float(np.median(vib.share)))
    if a.eval_only:
        layers = read_mbn(a.models / f"{name}.mbn")
        res = evaluate(name, x, rows, uniq, layers, A, N, vib, novib=True)
        pick = "as written"
    else:
        t0 = time.time()
        net = train_sup(T, hidden, a.sup_steps, log=log)
        report["supervised_s"] = time.time() - t0
        sup = net_layers(net)
        write_mbn(a.work / f"{name}.sup.mbn", sup)
        r_sup = evaluate(name, x, rows, uniq, read_mbn(a.work / f"{name}.sup.mbn"), A, N, vib, novib=True)
        s_sup = summarise(r_sup)
        print_summary(f"{name}, supervised", s_sup, log)
        layers, res, pick = sup, r_sup, "supervised"
        report["supervised"] = s_sup["all"]
        if a.e2e_minutes > 0:
            t0 = time.time()
            try:
                net, steps = train_e2e(net, T, x, a.e2e_minutes, log=log)
                report["e2e_s"] = time.time() - t0
                report["e2e_steps"] = steps
                e2e = net_layers(net)
                ok = all(np.isfinite(w).all() and np.isfinite(b).all() for w, b in e2e)
                if ok:
                    write_mbn(a.work / f"{name}.e2e.mbn", e2e)
                    r_e2e = evaluate(name, x, rows, uniq, read_mbn(a.work / f"{name}.e2e.mbn"), A, N, vib, novib=True)
                    s_e2e = summarise(r_e2e)
                    print_summary(f"{name}, end to end", s_e2e, log)
                    report["e2e"] = s_e2e["all"]
                    if s_e2e["all"]["net MR-LSD"] < s_sup["all"]["net MR-LSD"] and s_e2e["all"]["net |dB|"] < s_sup["all"]["net |dB|"] + 0.5:
                        layers, res, pick = e2e, r_e2e, "end to end"
                else:
                    log("  end to end: weights not finite; the supervised network kept")
            except Exception:
                log("  end to end failed; the supervised network kept:\n" + traceback.format_exc())
        size = write_mbn(a.models / f"{name}.mbn", layers)
        report["params"] = sum(w.size + b.size for w, b in layers)
        report["bytes"] = size
        write_index(a.models, a.work, {index_entry(name): f"{name}.mbn"})
        log(f"  {name}.mbn: {kind_of(layers)}, {pick}, {report['params']} parameters, {size} bytes")
    report["picked"] = pick
    report["kind"] = kind_of(layers)
    s = summarise(res)
    report["summary"] = s
    report["rows"] = res
    report["listening"] = listening(name, x, rows, layers, a.eval_dir)
    report["total_s"] = time.time() - t_all
    (a.work / f"{name}.json").write_text(json.dumps(report, indent=1, default=float))
    log(f"  {name}: {pick}; listening files written; total {report['total_s']:.0f} s")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("names", nargs="*")
    ap.add_argument("--data", type=Path, default=HOME / ".cache/musicbox/neural/data")
    ap.add_argument("--models", type=Path, default=REPO / "models")
    ap.add_argument("--work", type=Path, default=HOME / ".cache/musicbox/neural/work")
    ap.add_argument("--eval-dir", type=Path, default=HOME / ".cache/musicbox/neural/eval")
    ap.add_argument("--hidden", default="128,128,128,128")
    ap.add_argument("--sup-steps", type=int, default=12000)
    ap.add_argument("--e2e-minutes", type=float, default=6.0)
    ap.add_argument("--vib", choices=("auto", "on", "off"), default="auto")
    ap.add_argument("--queue", action="store_true", help="share the names with other copies (each takes one no copy has)")
    ap.add_argument("--eval-only", action="store_true")
    ap.add_argument("--table", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        sys.exit(0 if selftest() else 1)
    if a.table:
        table(a.names, a.work)
        return
    for d in (a.work, a.eval_dir, a.models):
        d.mkdir(parents=True, exist_ok=True)
    hidden = [int(h) for h in a.hidden.split(",")]
    claims = a.work / "claims"
    claims.mkdir(exist_ok=True)
    for name in a.names:
        if a.queue:
            try:
                os.close(os.open(claims / name, os.O_CREAT | os.O_EXCL | os.O_WRONLY))
            except FileExistsError:
                continue
        with open(a.work / f"{name}.log", "a") as lf:

            def log(s, lf=lf):
                print(s, flush=True)
                lf.write(s + "\n")
                lf.flush()

            try:
                run(name, a, hidden, log)
            except Exception:
                log(f"  {name} FAILED:\n" + traceback.format_exc())


if __name__ == "__main__":
    main()
