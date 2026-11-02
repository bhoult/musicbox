# /// script
# requires-python = ">=3.10"
# dependencies = ["numpy", "scipy"]
# ///
"""Each band style's balance (src/music/styles.rs's `balance`): every channel
heard alone at its level in the mix (MUSICBOX_SOLO, the leveller held:
MUSICBOX_NOLEVEL), over whole pieces of the style (MUSICBOX_STYLE), and the
trim (dB) that brings its share of the energy to a band's.

    uv run tools/validate/balance.py [--bin PATH] [--seeds 301,302,303] [GENRE...]

A band's balance (each channel's share of the energy, as a mix of a pop or
rock record carries it: the bass and the kick most of the low end, the
rhythm part and the tune most of the middle, the tune where a singer would
be) is ROLE, shared among the channels a style plays (each above 0.3 % of
its energy; fx is left as it is). The pieces are heard with the style's
balance as it is, so each trim found is added to the row there (a style's
instruments changed, it is run again). Trims are clamped to +-12 dB. It
prints each style's row for styles.rs, and its low, middle and high bands'
shares before and after (the records': about 0.5, 0.37, 0.10).
"""

import os
import re
import subprocess
import sys
import tempfile
import wave
from concurrent.futures import ThreadPoolExecutor

import numpy as np
from scipy.signal import welch

CH = ["kick", "snare", "hat", "perc", "cym", "bass", "pad", "arp", "lead", "bell", "keys", "drone", "fx"]
ROLE = {"kick": 0.18, "snare": 0.07, "hat": 0.006, "perc": 0.015, "cym": 0.006, "bass": 0.26, "pad": 0.07,
        "arp": 0.04, "lead": 0.15, "bell": 0.015, "keys": 0.16, "drone": 0.02}
BAND = ["rock", "60s", "70s", "80s", "90s", "2000s", "2010s", "2020s", "kenyan", "elevator", "corporate", "worship", "praise"]
LISTS = {"rock": "ROCK_", "60s": "SIXTIES_", "70s": "SEVENTIES_", "80s": "EIGHTIES_", "90s": "NINETIES_",
         "2000s": "NOUGHTIES_", "2010s": "TENS_", "2020s": "TWENTIES_", "kenyan": "KENYAN_", "elevator": "ELEVATOR_",
         "corporate": "CORPORATE_", "worship": "WORSHIP_", "praise": "PRAISE_"}
EDGES = [(0, 250), (250, 2000), (2000, 24000)]


def styles(src):
    """Each genre's styles' names, in order, from styles_of."""
    out = {}
    for g, lst in LISTS.items():
        m = re.search(rf"const {lst}: &\[\(f32, Style\)\] = &\[(.*?)\];", src, re.S)
        out[g] = re.findall(r"\(\s*[0-9.]+,\s*([A-Z0-9_]+)\)", m.group(1)) if m else []
    return out


def rows(src):
    """Each style's balance as styles.rs has it (dB by channel)."""
    out = {}
    for m in re.finditer(r"const ([A-Z0-9_]+): Style = Style \{(.*?)\n\};", src, re.S):
        b = re.search(r"balance: &\[(.*?)\]", m.group(2), re.S)
        out[m.group(1)] = {c.lower(): float(v) for c, v in re.findall(r"\(CH_([A-Z]+),\s*(-?[0-9.]+)\)", b.group(1))} if b else {}
    return out


def render(job):
    bin_, g, k, seed, ch, tmp = job
    p = os.path.join(tmp, f"{g}-{k}-{seed}-{ch}.wav")
    env = dict(os.environ, SDL_AUDIODRIVER="disk", MUSICBOX_SOLO=ch, MUSICBOX_NOLEVEL="1", MUSICBOX_STYLE=str(k))
    subprocess.run([bin_, "--export", g, p, "--seed", seed], env=env, capture_output=True, check=True)
    with wave.open(p) as f:
        x = np.frombuffer(f.readframes(f.getnframes()), np.int16).reshape(-1, 2).astype(np.float64) / 32768
    os.remove(p)
    fr, pw = welch(x.mean(1), 48000, nperseg=4096)
    tot = max(pw.sum(), 1e-30)
    return g, k, ch, float((x ** 2).sum()), [float(pw[(fr >= a) & (fr < b)].sum() / tot) for a, b in EDGES]


def main():
    args = sys.argv[1:]
    bin_ = "target/release/musicbox"
    seeds = ["301", "302", "303"]
    if "--bin" in args:
        i = args.index("--bin")
        bin_ = args[i + 1]
        del args[i : i + 2]
    if "--seeds" in args:
        i = args.index("--seeds")
        seeds = args[i + 1].split(",")
        del args[i : i + 2]
    genres = args or BAND
    src = open(os.path.join(os.path.dirname(__file__), "../../src/music/styles.rs")).read()
    names, had = styles(src), rows(src)
    tmp = tempfile.mkdtemp()
    jobs = [(bin_, g, k, s, ch, tmp) for g in genres for k in range(len(names[g])) for s in seeds for ch in CH]
    energy, bands = {}, {}
    with ThreadPoolExecutor(int(os.environ.get("JOBS", "3"))) as ex:
        for g, k, ch, e, b in ex.map(render, jobs):
            energy[g, k, ch] = energy.get((g, k, ch), 0.0) + e
            bb = bands.setdefault((g, k, ch), np.zeros(3))
            bb += np.array(b) * e
    os.rmdir(tmp)
    for g in genres:
        for k, name in enumerate(names[g]):
            e = np.array([energy[g, k, c] for c in CH])
            share = e / e.sum()
            band = np.array([bands[g, k, c] / max(energy[g, k, c], 1e-30) for c in CH])
            live = [i for i, c in enumerate(CH) if c in ROLE and share[i] > 0.003]
            want = np.array([ROLE[CH[i]] for i in live])
            want = want / want.sum() * share[live].sum()
            old = had.get(name, {})
            trim = {i: float(np.clip(old.get(CH[i], 0.0) + 10 * np.log10(w / share[i]), -12, 12)) for i, w in zip(live, want)}
            after = share.copy()
            for i, db in trim.items():
                after[i] = share[i] * 10 ** ((db - old.get(CH[i], 0.0)) / 10)
            after /= after.sum()
            b0, b1 = share @ band, after @ band
            row = ", ".join(f"(CH_{CH[i].upper()}, {db:.1f})" for i, db in sorted(trim.items()) if abs(db) >= 0.5)
            print(f"// {g} {name}: bands {b0[0]:.2f}/{b0[1]:.2f}/{b0[2]:.3f} -> {b1[0]:.2f}/{b1[1]:.2f}/{b1[2]:.3f}; "
                  + ", ".join(f"{CH[i]} {100 * share[i]:.0f}%" for i in np.argsort(-share) if share[i] > 0.01))
            print(f"{name}: balance: &[{row}],")


if __name__ == "__main__":
    main()
