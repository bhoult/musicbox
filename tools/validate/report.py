# /// script
# requires-python = ">=3.10"
# dependencies = ["numpy"]
# ///
"""musicbox's songs against real recordings, genre by genre, from critic.py's
measurements.

    uv run tools/validate/report.py REFS.jsonl OURS.jsonl [OURS2.jsonl ...] [--md OUT.md]

Each file's genre is its directory's name (refs/<genre>/..., ours/<run>/<genre>/...).
With more than one OURS, each is a column (a baseline, then what changed).

What it compares:
  - Discogs style: the share the tagger gives the genre's own styles, ours and
    the real recordings' (their mean); the tagger's top styles.
  - Nearest genre: each song's EffNet embedding against each genre's real
    recordings' mean (cosine); the share of a genre's songs nearest their own
    genre (the real recordings: each left out of its genre's mean).
  - Similarity: the mean cosine of a genre's songs to its real recordings' mean.
  - Signal: tempo, loudness and its range, the spectrum's centroid and the
    share above 2 kHz, stereo width, onsets a second, the share of minor keys.
"""

import collections
import json
import os
import sys

import numpy as np

GENRES = ["ambient", "techno", "synthwave", "lo-fi", "polka", "tavern", "ska", "mariachi", "indonesian", "starconflict", "celtic", "western"]

# the Discogs styles each genre is (the tagger has no gamelan)
TARGET = {
    "ambient": ["Electronic---Ambient"],
    "techno": ["Electronic---Techno", "Electronic---Minimal Techno"],
    "synthwave": ["Electronic---Synthwave"],
    "lo-fi": ["Hip Hop---Boom Bap", "Hip Hop---Instrumental", "Hip Hop---Jazzy Hip-Hop"],
    "polka": ["Folk, World, & Country---Polka"],
    "tavern": ["Classical---Medieval", "Classical---Renaissance"],
    "ska": ["Reggae---Ska", "Rock---Ska"],
    "mariachi": ["Latin---Mariachi", "Latin---Ranchera"],
    "indonesian": [],
    "starconflict": ["Stage & Screen---Score", "Stage & Screen---Soundtrack"],
    "celtic": ["Folk, World, & Country---Celtic"],
    "western": ["Stage & Screen---Score", "Stage & Screen---Soundtrack"],
}


def genre_of(path):
    parts = path.split(os.sep)
    for p in reversed(parts[:-1]):
        if p in GENRES:
            return p
    return None


def load(path):
    rows = collections.defaultdict(list)
    for line in open(path):
        r = json.loads(line)
        g = genre_of(r["file"])
        if g and "error" not in r and "/stems/" not in r["file"]:
            rows[g].append(r)
    return rows


def unit(v):
    v = np.asarray(v, dtype=float)
    return v / (np.linalg.norm(v) + 1e-12)


def target_share(r, g):
    return sum(r["genre_discogs400"].get(s, 0.0) for s in TARGET[g])


def mean_styles(rs):
    d = collections.defaultdict(float)
    for r in rs:
        for k, v in r["genre_discogs400"].items():
            d[k] += v / len(rs)
    return d


def med(xs):
    xs = [x for x in xs if x is not None]
    return float(np.median(xs)) if xs else float("nan")


def signal(rs):
    def high(r):
        b = r.get("bands")
        return (b[3] + b[4]) if b else None

    minor = [r["key"].endswith("minor") for r in rs if "key" in r]
    return {
        "bpm": med([r.get("bpm") for r in rs]),
        "lufs": med([r.get("lufs") for r in rs]),
        "lra": med([r.get("lra") for r in rs]),
        "centroid": med([r.get("centroid") for r in rs]),
        "high": med([high(r) for r in rs]),
        "width": med([r.get("width") for r in rs]),
        "onsets": med([r.get("onset_rate") for r in rs]),
        "minor": (sum(minor) / len(minor)) if minor else float("nan"),
    }


def main():
    args = sys.argv[1:]
    md = None
    if "--md" in args:
        i = args.index("--md")
        md = args[i + 1]
        del args[i : i + 2]
    refs = load(args[0])
    runs = [(os.path.splitext(os.path.basename(a))[0], load(a)) for a in args[1:]]
    out = []
    p = out.append

    # each genre's real recordings' mean embedding
    cent = {g: unit(np.mean([unit(r["embedding"]) for r in refs[g]], axis=0)) for g in GENRES if refs.get(g)}

    def nearest(r, own=None):
        e = unit(r["embedding"])
        best, bs = None, -2
        for g, c in cent.items():
            if own == g and len(refs[g]) > 1:
                # (a real recording: its genre's mean without it)
                c = unit(np.mean([unit(x["embedding"]) for x in refs[g] if x is not r], axis=0))
            s = float(e @ c)
            if s > bs:
                best, bs = g, s
        return best

    p("| genre | real recordings | style share: real | " + " | ".join(f"style share: {n}" for n, _ in runs) + " | nearest own: real | " + " | ".join(f"nearest own: {n}" for n, _ in runs) + " | " + " | ".join(f"similarity: {n}" for n, _ in runs) + " |")
    p("|" + "---|" * (3 + 2 * len(runs) + 1 + len(runs)))
    for g in GENRES:
        rr = refs.get(g, [])
        cells = [g, str(len(rr))]
        cells.append(f"{np.mean([target_share(r, g) for r in rr]):.2f}" if rr and TARGET[g] else "–")
        for _, run in runs:
            rs = run.get(g, [])
            cells.append(f"{np.mean([target_share(r, g) for r in rs]):.2f}" if rs and TARGET[g] else "–")
        cells.append(f"{np.mean([nearest(r, g) == g for r in rr]):.0%}" if rr and g in cent else "–")
        for _, run in runs:
            rs = run.get(g, [])
            cells.append(f"{np.mean([nearest(r) == g for r in rs]):.0%}" if rs and g in cent else "–")
        for _, run in runs:
            rs = run.get(g, [])
            cells.append(f"{np.mean([float(unit(r['embedding']) @ cent[g]) for r in rs]):.3f}" if rs and g in cent else "–")
        p("| " + " | ".join(cells) + " |")
    p("")

    for g in GENRES:
        rr = refs.get(g, [])
        p(f"## {g}")
        p("")
        if rr:
            d = mean_styles(rr)
            top = sorted(d, key=d.get, reverse=True)[:5]
            p(f"- real recordings ({len(rr)}): " + ", ".join(f"{k.split('---')[1]} {d[k]:.2f}" for k in top))
        for n, run in runs:
            rs = run.get(g, [])
            if not rs:
                continue
            d = mean_styles(rs)
            top = sorted(d, key=d.get, reverse=True)[:5]
            near = collections.Counter(nearest(r) for r in rs)
            p(f"- {n} ({len(rs)}): " + ", ".join(f"{k.split('---')[1]} {d[k]:.2f}" for k in top) + "; nearest: " + ", ".join(f"{k} {v}" for k, v in near.most_common(3)))
        p("")
        cols = [("real", rr)] + [(n, run.get(g, [])) for n, run in runs]
        sig = [(n, signal(rs)) for n, rs in cols if rs]
        if sig:
            p("| | " + " | ".join(n for n, _ in sig) + " |")
            p("|---|" + "---|" * len(sig))
            for key, fmt, label in [
                ("bpm", "{:.0f}", "tempo (bpm)"),
                ("lufs", "{:.1f}", "loudness (LUFS)"),
                ("lra", "{:.1f}", "loudness range (LU)"),
                ("centroid", "{:.0f}", "spectral centroid (Hz)"),
                ("high", "{:.1%}", "energy above 2 kHz"),
                ("width", "{:.3f}", "stereo width (side share)"),
                ("onsets", "{:.1f}", "onsets a second"),
                ("minor", "{:.0%}", "in a minor key"),
            ]:
                p(f"| {label} | " + " | ".join(fmt.format(s[key]) for _, s in sig) + " |")
            p("")
    text = "\n".join(out)
    print(text)
    if md:
        open(md, "w").write(text + "\n")


if __name__ == "__main__":
    main()
