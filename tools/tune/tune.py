# /// script
# requires-python = ">=3.10,<3.12"
# dependencies = ["essentia-tensorflow", "numpy", "cma"]
# ///
"""Tunes each genre's mix (src/music/tuned.rs's KNOBS) so musicbox's songs
sound more like the genre's real recordings to the taggers of tools/validate.

    uv run tools/tune/tune.py search GENRE [--budget N] [--popsize N] [--sigma S]
    uv run tools/tune/tune.py heldout GENRE
    uv run tools/tune/tune.py run GENRE... [--budget N]     # search, then held-out, each in turn
    uv run tools/tune/tune.py table [GENRE...]              # the held-out numbers, as Markdown
    uv run tools/tune/tune.py write                         # the kept settings into TUNED

Common options: --bin PATH (the musicbox binary; by default target/release/musicbox),
--lam L (the L2 penalty's weight), --w W (the style share's weight),
--range KNOB=LO:HI (repeatable; search a narrower range than KNOBS's; KNOB may
be "levels" for the 13 channels or "tilt" for both shelves), --tag NAME (a
second pass: its own log, state, settings and decision, GENRE.NAME.*).

The search: CMA-ES over the genre's live knobs, each normalised to [0, 1] and
starting at its rest value. A knob, or one side of it, that leaves every
training song byte-identical at its limit is dead and fixed where it rests (a
channel no training song plays; the swing below a straight genre's 0, which the
engine clamps; the tempo where a piece fixes its own timing; human where a
genre has no lateness). A setting's score, on training seeds 201-206 (one 1:30 song
each, the same songs for every setting):

    mean cosine to the genre's reference centroid
    + W * min(mean target style share, the references' mean share)
    - LAM * sum((x - rest)^2)            (x normalised; W 0.25, LAM 0.05)

The share is capped at the references' own: past it, the tagger naming the
genre more is not more like the recordings (polka's 78s, mariachi's 1908
records, western's stand-ins are named their genre less than musicbox is).

Every evaluation is logged to ~/.cache/musicbox/tune/GENRE.jsonl, and the
search's state to GENRE.cma.pkl after each generation, so a run resumes.

Held-out: seeds 101-108, whole songs, as written against the best setting
rounded to 2 decimals (what TUNED will hold), measured in full: cosine to the
centroid, style share, nearest genre, the MTG-Jamendo genre head on the
genre's nearest Jamendo genres, and loudness, spectral centroid and energy
above 2 kHz against the references' medians. Decisions go to
~/.cache/musicbox/tune/decisions.json.

Runs on the CPU (CUDA_VISIBLE_DEVICES=-1), TensorFlow on one thread a pool,
renders one at a time beside the measuring. Run it under a CPU quota, e.g.

    systemd-run --user --scope -q -p CPUQuota=150% -p MemoryMax=8G nice -n 10 uv run tools/tune/tune.py run celtic
"""

import os

os.environ["CUDA_VISIBLE_DEVICES"] = "-1"
os.environ.setdefault("TF_CPP_MIN_LOG_LEVEL", "3")
os.environ.setdefault("TF_NUM_INTRAOP_THREADS", "1")
os.environ.setdefault("TF_NUM_INTEROP_THREADS", "1")
os.environ.setdefault("OMP_NUM_THREADS", "1")

import hashlib
import json
import pickle
import re
import subprocess
import sys
import time
import wave
from concurrent.futures import ThreadPoolExecutor

import numpy as np

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.dont_write_bytecode = True  # (no __pycache__ left in tools/validate)
sys.path.insert(0, os.path.join(REPO, "tools", "validate"))
import report  # noqa: E402  (GENRES, TARGET, load, unit, target_share)

GENRES = report.GENRES
TARGET = report.TARGET
unit = report.unit

TUNE = os.path.expanduser("~/.cache/musicbox/tune")
REFS = os.path.expanduser("~/.cache/musicbox/validate/refs.jsonl")
TUNED_RS = os.path.join(REPO, "src", "music", "tuned.rs")

TRAIN_SEEDS = [201, 202, 203, 204, 205, 206]
TRAIN_LENGTH = "1:30"
HELD_SEEDS = [101, 102, 103, 104, 105, 106, 107, 108]
CHANNELS = 13  # the first 13 knobs are channel levels

# the MTG-Jamendo genres nearest each (Jamendo has no polka, ska, mariachi or
# gamelan: those are proxies), each scored by the references above the
# all-genre mean
JAMENDO = {
    "ambient": ["ambient", "atmospheric"],
    "techno": ["techno", "minimal"],
    "synthwave": ["synthpop", "80s"],
    "lo-fi": ["hiphop", "triphop"],
    "polka": ["folk", "world"],
    "tavern": ["medieval", "folk"],
    "ska": ["reggae", "punkrock"],
    "mariachi": ["latin", "world"],
    "indonesian": ["world", "ethno", "worldfusion"],
    "starconflict": ["soundtrack", "orchestral", "symphonic"],
    "celtic": ["celtic", "folk"],
    "western": ["soundtrack", "country"],
}

# YAMNet's AudioSet classes nearest each (a different network from the
# EffNet that the search and the Jamendo head both listen through; AudioSet has
# no polka, synthwave or gamelan: instruments stand in), each scored by the
# references above the all-genre mean. Reported, not used to decide.
YAMNET = {
    "ambient": ["Ambient music", "New-age music"],
    "techno": ["Techno", "House music", "Electronic dance music"],
    "synthwave": ["Electronic music", "Synthesizer"],
    "lo-fi": ["Hip hop music"],
    "polka": ["Accordion", "Brass instrument"],
    "tavern": ["Traditional music", "Folk music"],
    "ska": ["Ska", "Reggae"],
    "mariachi": ["Music of Latin America", "Trumpet"],
    "indonesian": ["Mallet percussion", "Gong"],
    "starconflict": ["Orchestra", "Soundtrack music"],
    "celtic": ["Traditional music", "Folk music"],
    "western": ["Country", "Soundtrack music"],
}

# the references that can't say much (docs/validation.md)
WEAK = {
    "starconflict": "stand-in references (orchestral marches, Romantic symphonies)",
    "western": "stand-in references (surf and western-guitar tracks)",
    "mariachi": "six references, four of them 1908 recordings",
}

# when a held-out result is kept
KEEP_COS = 0.005  # mean cosine gain at least
KEEP_SEEDS = 6  # of 8 songs closer
JAM_DROP = 0.15  # the Jamendo verdict falling more than this share (and 0.01) is worse
NEAR_DROP = 2  # songs fewer nearest their own genre
LUFS_WORSE = 1.5  # LU further from the references' median
LOG_WORSE = float(np.log(1.25))  # 25% further (centroid, energy above 2 kHz)


def knobs():
    """KNOBS from src/music/tuned.rs: (name, low, high, rest)."""
    text = open(TUNED_RS).read()
    body = text[text.index("pub const KNOBS") :]
    body = body[: body.index("];")]
    ks = [(n, float(a), float(b), float(c)) for n, a, b, c in re.findall(r'\("(\w+)",\s*(-?[\d.]+),\s*(-?[\d.]+),\s*(-?[\d.]+)\)', body)]
    assert len(ks) == 20, ks
    return ks


KNOBS = knobs()
LOW = np.array([k[1] for k in KNOBS])
HIGH = np.array([k[2] for k in KNOBS])
REST = np.array([k[3] for k in KNOBS])
REST_N = (REST - LOW) / (HIGH - LOW)


def denorm(xn):
    return LOW + np.clip(xn, 0, 1) * (HIGH - LOW)


def norm(v):
    return (np.asarray(v) - LOW) / (HIGH - LOW)


def key_of(vals):
    return ",".join(f"{v:.5f}" for v in vals)


def write_tsv(path, genre, vals):
    with open(path, "w") as f:
        for (name, *_), v in zip(KNOBS, vals):
            f.write(f"{genre}\t{name}\t{v:.6f}\n")


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()[:16]


class Tuner:
    def __init__(self, bin_path, lam=0.05, w=0.25, ranges=None, tag=None):
        self.bin = bin_path
        # narrower search ranges than KNOBS's, by knob: (low, high)
        self.ranges = ranges or {}
        # a second pass's name: its own log, state, settings and decision
        self.tag = tag
        self.bin_sha = sha(bin_path)
        self.lam = lam
        self.w = w
        refs = report.load(REFS)
        self.refs = refs
        self.cent = {g: unit(np.mean([unit(r["embedding"]) for r in refs[g]], axis=0)) for g in GENRES if refs.get(g)}
        self.ref_share = {g: float(np.mean([report.target_share(r, g) for r in refs[g]])) for g in GENRES if TARGET[g]}
        self.critic = None

    def measure(self, path, fast=True):
        if self.critic is None:
            import essentia
            from critic import Critic

            self.critic = Critic()
            # (each measurement warns "No network created" thousands of times)
            essentia.log.warningActive = False
            essentia.log.infoActive = False
        return self.critic.measure(path, fast=fast)

    def render(self, genre, seed, out, tsv=None, length=None):
        cmd = [self.bin, "--export", genre, out, "--seed", str(seed)]
        if length:
            cmd += ["--length", length]
        env = dict(os.environ)
        env.pop("MUSICBOX_SOLO", None)
        env.pop("MUSICBOX_TUNING", None)
        if tsv:
            cmd.append("--tuned")
            env["MUSICBOX_TUNING"] = tsv
        r = subprocess.run(cmd, env=env, capture_output=True, text=True)
        if r.returncode != 0 or not os.path.exists(out):
            raise RuntimeError(f"render failed: {' '.join(cmd)}: {r.stderr[-500:]}")
        return out

    # ------------------------------------------------------------ live knobs

    def live(self, genre):
        """The knobs a training song can hear, and which way: {"knobs": [index],
        "lo": [normalised], "hi": [normalised]}. A knob (or one side of it) is
        dead when setting it to its limit leaves every training song's audio
        byte for byte as written: a channel no training song plays, the swing
        below a straight genre's 0 (the engine clamps it there), a genre whose
        piece fixes its own timing (the tempo), one with no lateness (human)."""
        path = os.path.join(TUNE, f"{genre}.live.json")
        if os.path.exists(path):
            return json.load(open(path))
        work = os.path.join(TUNE, "work", genre)
        os.makedirs(work, exist_ok=True)
        rest = {}

        def rest_sha(seed):
            if seed not in rest:
                out = os.path.join(work, f"rest-{seed}.wav")
                self.render(genre, seed, out, None, TRAIN_LENGTH)
                rest[seed] = sha(out)
                os.remove(out)
            return rest[seed]

        def heard(k, v):
            vals = REST.copy()
            vals[k] = v
            tsv = os.path.join(work, "live.tsv")
            write_tsv(tsv, genre, vals)
            for seed in TRAIN_SEEDS:
                out = os.path.join(work, f"live-{seed}.wav")
                self.render(genre, seed, out, tsv, TRAIN_LENGTH)
                h = sha(out)
                os.remove(out)
                if h != rest_sha(seed):
                    return True
            return False

        out = {"knobs": [], "lo": [], "hi": [], "dead": []}
        for k, (name, low, high, r) in enumerate(KNOBS):
            if k < CHANNELS:
                up = down = heard(k, high)
            else:
                up, down = heard(k, high), heard(k, low)
            if not (up or down):
                out["dead"].append(name)
                continue
            out["knobs"].append(k)
            out["lo"].append(0.0 if down else float(REST_N[k]))
            out["hi"].append(1.0 if up else float(REST_N[k]))
            if not (up and down):
                out["dead"].append(name + (" below rest" if up else " above rest"))
        json.dump(out, open(path, "w"))
        return out

    # ------------------------------------------------------------ the score

    def score_rows(self, genre, rows):
        """Cosine, share and the capped share term of a setting's training songs."""
        cos = [float(unit(r["embedding"]) @ self.cent[genre]) for r in rows]
        share = [report.target_share(r, genre) for r in rows] if TARGET[genre] else [0.0] * len(rows)
        mcos, mshare = float(np.mean(cos)), float(np.mean(share))
        term = min(mshare, self.ref_share[genre]) if TARGET[genre] else 0.0
        return cos, share, mcos, mshare, term

    def evaluate(self, genre, vals):
        """Renders the training songs with `vals` and measures them, rendering
        the next song while the last is measured."""
        work = os.path.join(TUNE, "work", genre)
        os.makedirs(work, exist_ok=True)
        tsv = os.path.join(work, "eval.tsv")
        write_tsv(tsv, genre, vals)
        rows = []
        with ThreadPoolExecutor(max_workers=1) as pool:
            futs = [pool.submit(self.render, genre, s, os.path.join(work, f"train-{s}.wav"), tsv, TRAIN_LENGTH) for s in TRAIN_SEEDS]
            for f in futs:
                path = f.result()
                r = self.measure(path, fast=True)
                os.remove(path)
                rows.append(r)
        return rows

    # ------------------------------------------------------------ the search

    def stem(self, genre):
        return genre + (f".{self.tag}" if self.tag else "")

    def search(self, genre, budget=150, popsize=10, sigma=0.2, seed=1):
        import cma

        stem = self.stem(genre)
        log_path = os.path.join(TUNE, f"{stem}.jsonl")
        pkl_path = os.path.join(TUNE, f"{stem}.cma.pkl")
        lv = self.live(genre)
        live, lo, hi = lv["knobs"], np.array(lv["lo"]), np.array(lv["hi"])
        for i, k in enumerate(live):
            if KNOBS[k][0] in self.ranges:
                a, b = ((v - LOW[k]) / (HIGH[k] - LOW[k]) for v in self.ranges[KNOBS[k][0]])
                assert a <= REST_N[k] <= b, f"{KNOBS[k][0]}'s range must hold its rest value"
                lo[i], hi[i] = max(lo[i], a), min(hi[i], b)
        bounds = {KNOBS[k][0]: [float(LOW[k] + lo[i] * (HIGH[k] - LOW[k])), float(LOW[k] + hi[i] * (HIGH[k] - LOW[k]))] for i, k in enumerate(live)}
        print(f"[{stem}] live knobs: {', '.join(KNOBS[k][0] for k in live)}; dead: {', '.join(lv['dead']) or 'none'}"
              + (f"; narrowed: {', '.join(f'{n} {a:g}..{b:g}' for n, (a, b) in self.ranges.items())}" if self.ranges else ""), flush=True)
        cache = {}
        if os.path.exists(log_path):
            for line in open(log_path):
                e = json.loads(line)
                if e.get("lam") == self.lam and e.get("w") == self.w and e.get("bin") == self.bin_sha:
                    cache[e["key"]] = e
        log = open(log_path, "a")
        n_done = [len(cache)]

        def f(xn_live, gen, tag="cma"):
            xn = REST_N.copy()
            xn[live] = np.clip(xn_live, lo, hi)
            vals = denorm(xn)
            k = key_of(vals)
            if k in cache:
                return cache[k]
            t0 = time.time()
            rows = self.evaluate(genre, vals)
            cos, share, mcos, mshare, term = self.score_rows(genre, rows)
            pen = float(np.sum((xn - REST_N) ** 2))
            score = mcos + self.w * term - self.lam * pen
            e = {
                "genre": genre, "n": n_done[0], "gen": gen, "tag": tag, "key": k,
                "knobs": {name: round(float(v), 5) for (name, *_), v in zip(KNOBS, vals)},
                "x": [round(float(x), 5) for x in xn], "score": score, "cos": mcos, "share": mshare,
                "share_term": term, "pen": pen, "cos_seeds": cos, "share_seeds": share,
                "lam": self.lam, "w": self.w, "bin": self.bin_sha, "secs": round(time.time() - t0, 1),
            }
            log.write(json.dumps(e) + "\n")
            log.flush()
            cache[k] = e
            n_done[0] += 1
            print(f"[{genre}] #{e['n']} gen {gen} score {score:.4f} (cos {mcos:.4f}, share {mshare:.3f}, pen {pen:.3f}) {e['secs']}s", flush=True)
            return e

        base = f(REST_N[live], -1, "rest")
        if os.path.exists(pkl_path):
            st = pickle.load(open(pkl_path, "rb"))
            es, best_hist = st["es"], st["best_hist"]
            np.random.set_state(st["rng"])
            print(f"[{genre}] resumed at generation {es.countiter}", flush=True)
        else:
            np.random.seed(seed)
            es = cma.CMAEvolutionStrategy(
                list(REST_N[live]), sigma,
                {"bounds": [list(lo), list(hi)], "popsize": popsize, "seed": seed, "maxfevals": budget, "verbose": -9},
            )
            best_hist = []
        while True:
            done = sum(1 for e in cache.values() if e["tag"] == "cma")
            if done >= budget - 1:
                break
            if es.stop():
                print(f"[{genre}] CMA-ES stopped: {es.stop()}", flush=True)
                break
            if len(best_hist) >= 6 and best_hist[-1] - best_hist[-5] < 0.002:
                print(f"[{genre}] plateau: best {best_hist[-1]:.4f}, {best_hist[-5]:.4f} four generations ago", flush=True)
                break
            pickle.dump({"es": es, "best_hist": best_hist, "rng": np.random.get_state()}, open(pkl_path, "wb"))
            gen = es.countiter
            xs = es.ask()
            res = [f(np.asarray(x), gen) for x in xs]
            es.tell(xs, [-r["score"] for r in res])
            best = max(e["score"] for e in cache.values())
            best_hist.append(best)
            print(f"[{genre}] generation {gen}: best {best:.4f} (rest {base['score']:.4f}), sigma {es.sigma:.3f}", flush=True)
        pickle.dump({"es": es, "best_hist": best_hist, "rng": np.random.get_state()}, open(pkl_path, "wb"))

        # the best, and the search's mean, rounded to what TUNED holds
        best = max(cache.values(), key=lambda e: e["score"])
        xn = REST_N.copy()
        xn[live] = np.clip(es.mean, lo, hi)
        mean = f(xn[live], es.countiter, "mean")
        pick = max([best, mean], key=lambda e: e["score"])
        vals = np.round(np.array([pick["knobs"][k[0]] for k in KNOBS]), 2)
        vals = np.clip(vals, LOW, HIGH)
        rounded = f(norm(vals)[live], es.countiter, "rounded")
        out = {
            "genre": genre, "live": [KNOBS[k][0] for k in live], "dead": lv["dead"], "rest": base, "best": best, "mean": mean,
            "picked": pick["tag"] if pick is mean else "best", "rounded": rounded,
            "vals": [float(v) for v in vals], "evals": len(cache), "lam": self.lam, "w": self.w,
            "bounds": bounds, "ranges": self.ranges, "tag": self.tag,
        }
        json.dump(out, open(os.path.join(TUNE, f"{stem}.search.json"), "w"), indent=1)
        print(f"[{genre}] rest {base['score']:.4f} (cos {base['cos']:.4f}) -> rounded {rounded['score']:.4f} (cos {rounded['cos']:.4f}), {len(cache)} evaluations", flush=True)
        return out

    # ------------------------------------------------------------ held out

    def heldout_rows(self, genre, label, tsv=None):
        """The held-out songs, whole, measured in full (cached by setting)."""
        path = os.path.join(TUNE, "held", f"{genre}.jsonl")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        tag = label + "@" + (sha(tsv) if tsv else "written") + "@" + self.bin_sha
        have = {}
        if os.path.exists(path):
            for line in open(path):
                r = json.loads(line)
                if r.get("tag") == tag:
                    have[r["seed"]] = r
        work = os.path.join(TUNE, "held", genre, label)
        os.makedirs(work, exist_ok=True)
        todo = [s for s in HELD_SEEDS if s not in have]
        with ThreadPoolExecutor(max_workers=1) as pool, open(path, "a") as out:
            futs = [(s, pool.submit(self.render, genre, s, os.path.join(work, f"{genre}-{s}.wav"), tsv)) for s in todo]
            for s, fut in futs:
                wav = fut.result()
                r = self.measure(wav, fast=False)
                r.update(seed=s, tag=tag, label=label)
                out.write(json.dumps(r) + "\n")
                out.flush()
                have[s] = r
                print(f"[{genre}] held-out {label} {s}: cos {float(unit(r['embedding']) @ self.cent[genre]):.4f}", flush=True)
        return [have[s] for s in HELD_SEEDS]

    def nearest(self, r):
        e = unit(r["embedding"])
        return max(self.cent, key=lambda g: float(e @ self.cent[g]))

    def summary(self, genre, rows):
        def high(r):
            b = r.get("bands")
            return b[3] + b[4] if b else None

        return {
            "cos": [float(unit(r["embedding"]) @ self.cent[genre]) for r in rows],
            "share": [report.target_share(r, genre) for r in rows] if TARGET[genre] else None,
            "nearest": [self.nearest(r) for r in rows],
            "jam": [sum(r["mtg_jamendo_genre"][c] for c in JAMENDO[genre]) for r in rows],
            "yam": [sum(r["yamnet"][c] for c in YAMNET[genre]) for r in rows],
            "lufs": float(np.median([r["lufs"] for r in rows])),
            "centroid": float(np.median([r["centroid"] for r in rows])),
            "high": float(np.median([high(r) for r in rows])),
        }

    def ref_signal(self, genre):
        rr = self.refs[genre]
        return {
            "lufs": float(np.median([r["lufs"] for r in rr])),
            "centroid": float(np.median([r["centroid"] for r in rr])),
            "high": float(np.median([r["bands"][3] + r["bands"][4] for r in rr])),
            "jam": float(np.mean([sum(r["mtg_jamendo_genre"][c] for c in JAMENDO[genre]) for r in rr])),
            "yam": float(np.mean([sum(r["yamnet"][c] for c in YAMNET[genre]) for r in rr])),
            "share": self.ref_share.get(genre),
        }

    def heldout(self, genre):
        stem = self.stem(genre)
        s = json.load(open(os.path.join(TUNE, f"{stem}.search.json")))
        vals = np.array(s["vals"])
        tsv = os.path.join(TUNE, f"{stem}.tuned.tsv")
        write_tsv(tsv, genre, vals)
        before = self.summary(genre, self.heldout_rows(genre, "written"))
        after = self.summary(genre, self.heldout_rows(genre, "tuned" + (f".{self.tag}" if self.tag else ""), tsv))
        ref = self.ref_signal(genre)

        d_cos = np.array(after["cos"]) - np.array(before["cos"])
        closer = int(np.sum(d_cos > 0))
        mb, ma = float(np.mean(before["cos"])), float(np.mean(after["cos"]))
        jb, ja = float(np.mean(before["jam"])), float(np.mean(after["jam"]))
        nb = sum(n == genre for n in before["nearest"])
        na = sum(n == genre for n in after["nearest"])
        why = []
        if ma - mb < KEEP_COS:
            why.append(f"cosine {ma - mb:+.4f} (< +{KEEP_COS})")
        if closer < KEEP_SEEDS:
            why.append(f"only {closer} of 8 songs closer")
        if ja < jb - max(0.01, JAM_DROP * jb):
            why.append(f"Jamendo {'+'.join(JAMENDO[genre])} {jb:.3f} -> {ja:.3f}")
        if na <= nb - NEAR_DROP:
            why.append(f"nearest own {nb}/8 -> {na}/8")
        dist = {
            "lufs": lambda v: abs(v - ref["lufs"]),
            "centroid": lambda v: abs(np.log(v / ref["centroid"])),
            "high": lambda v: abs(np.log(v / ref["high"])),
        }
        for k, lim in [("lufs", LUFS_WORSE), ("centroid", LOG_WORSE), ("high", LOG_WORSE)]:
            if dist[k](after[k]) - dist[k](before[k]) > lim:
                why.append(f"{k} {before[k]:.3g} -> {after[k]:.3g} (references {ref[k]:.3g})")
        live = s["live"]
        xn = norm(vals)
        # at a limit of its search range (not where a dead side stops it at rest)
        bounds = s.get("bounds") or {n: [a, b] for n, a, b, _ in KNOBS}
        pinned = []
        for k, (name, low, high, r) in enumerate(KNOBS):
            if name in live:
                a, b = bounds[name]
                near = 0.03 * (high - low)
                if (a < r and vals[k] - a <= near) or (b > r and b - vals[k] <= near):
                    pinned.append(name)
        moved = sorted(
            [(KNOBS[k][0], float(vals[k]), float(xn[k] - REST_N[k])) for k in range(len(KNOBS))],
            key=lambda t: -abs(t[2]),
        )
        yb, ya = float(np.mean(before["yam"])), float(np.mean(after["yam"]))
        res = {
            "genre": genre, "stem": stem, "tag": self.tag, "ranges": s.get("ranges") or {}, "yam_before": yb, "yam_after": ya, "keep": not why, "why": why, "vals": [float(v) for v in vals],
            "before": before, "after": after, "ref": ref, "closer": closer,
            "cos_before": mb, "cos_after": ma, "jam_before": jb, "jam_after": ja,
            "near_before": nb, "near_after": na, "pinned": pinned,
            "moved": [m for m in moved if abs(m[2]) > 1e-9][:6], "weak": WEAK.get(genre), "search": {
                "rest": s["rest"]["score"], "rounded": s["rounded"]["score"], "cos_rest": s["rest"]["cos"],
                "cos_rounded": s["rounded"]["cos"], "evals": s["evals"], "picked": s["picked"], "lam": s["lam"], "w": s["w"],
            },
        }
        dpath = os.path.join(TUNE, "decisions.json")
        dec = json.load(open(dpath)) if os.path.exists(dpath) else {}
        dec[stem] = res
        json.dump(dec, open(dpath, "w"), indent=1)
        print(f"[{stem}] held-out cos {mb:.4f} -> {ma:.4f} ({closer}/8 closer), Jamendo {jb:.3f} -> {ja:.3f}, nearest own {nb} -> {na}; "
              + ("KEEP" if not why else "drop: " + "; ".join(why)) + (f"; pinned: {', '.join(pinned)}" if pinned else ""), flush=True)
        return res


def table(genres=None):
    dec = json.load(open(os.path.join(TUNE, "decisions.json")))
    stems = [k for g in GENRES if not genres or g in genres for k in dec if dec[k]["genre"] == g]
    out = []
    p = out.append
    p("| genre | cosine: written → tuned | closer | style share | nearest own | Jamendo (genres) | YAMNet (classes) | LUFS (refs) | centroid Hz (refs) | > 2 kHz (refs) | kept |")
    p("|---|---|---|---|---|---|---|---|---|---|---|")
    for st in stems:
        d = dec[st]
        g = d["genre"]
        name = g + (f" ({d['tag']})" if d.get("tag") else "")
        yam = f"{d['yam_before']:.3f} → {d['yam_after']:.3f} ({' + '.join(YAMNET[g])})" if "yam_before" in d else "–"
        b, a, r = d["before"], d["after"], d["ref"]
        sh = f"{np.mean(b['share']):.3f} → {np.mean(a['share']):.3f}" if b["share"] else "–"
        p(f"| {name}{' *' if d['weak'] else ''} | {d['cos_before']:.3f} → {d['cos_after']:.3f} | {d['closer']}/8 | {sh} | {d['near_before']}/8 → {d['near_after']}/8 "
          f"| {d['jam_before']:.3f} → {d['jam_after']:.3f} ({'+'.join(JAMENDO[g])}) | {yam} | {b['lufs']:.1f} → {a['lufs']:.1f} ({r['lufs']:.1f}) "
          f"| {b['centroid']:.0f} → {a['centroid']:.0f} ({r['centroid']:.0f}) | {b['high']:.1%} → {a['high']:.1%} ({r['high']:.1%}) | {'yes' if d['keep'] else 'no'} |")
    p("")
    for st in stems:
        d = dec[st]
        g = d["genre"]
        name = g + (f" ({d['tag']})" if d.get("tag") else "")
        mv = ", ".join(f"{n} {v:g}" for n, v, _ in d["moved"][:4])
        s = d["search"]
        rg = ", ".join(f"{n} {a:g}..{b:g}" for n, (a, b) in (d.get("ranges") or {}).items())
        p(f"- **{name}**: " + (f"searched within {rg}; " if rg else "") + f"moved most: {mv}. Search {s['evals']} evaluations, training score {s['rest']:.4f} → {s['rounded']:.4f} "
          f"(cosine {s['cos_rest']:.4f} → {s['cos_rounded']:.4f}). " + ("Kept." if d["keep"] else "Not kept: " + "; ".join(d["why"]) + ".")
          + (f" Knobs at a limit: {', '.join(d['pinned'])}." if d["pinned"] else "") + (f" Weakly grounded: {d['weak']}." if d["weak"] else ""))
    return "\n".join(out)


def write_tuned():
    """The kept settings into TUNED in src/music/tuned.rs (2 decimals)."""
    dec = json.load(open(os.path.join(TUNE, "decisions.json")))
    text = open(TUNED_RS).read()
    i = text.index("pub const TUNED")
    j = text.index("];", text.index("=", i)) + 2
    rows = []
    for g in GENRES:
        # a setting with no knob at a limit before one with (a pinned knob is
        # the likelier gaming of the tagger), then the closer held out
        kept = [d for d in dec.values() if d["genre"] == g and d["keep"]]
        d = max(kept, key=lambda d: (not d["pinned"], d["cos_after"])) if kept else None
        if len(kept) > 1:
            print(f"{g}: {len(kept)} kept settings; writing {d.get('stem', g)}'s", file=sys.stderr)
        if d:
            vals = ", ".join(f"{0.0 if abs(v) < 0.005 else v:.2f}" for v in d["vals"])
            rows.append(f"    // {g}\n    Some([{vals}]),")
        else:
            rows.append(f"    None, // {g}")
    new = "pub const TUNED: [Option<[f32; NKNOBS]>; 12] = [\n" + "\n".join(rows) + "\n];"
    # the comment above: its own line, then when and how these were found
    head = text[:i].rstrip("\n").split("\n")
    own = "/// Each genre's settings, by its number (None: as written)."
    if own in head:
        head = head[: len(head) - head[::-1].index(own)]
    head.append(f"/// Found {time.strftime('%Y-%m-%d')} by tools/tune/tune.py: CMA-ES against the tools/validate")
    head.append("/// taggers, each kept only where whole songs held out of the search came")
    head.append("/// closer and the other judges held (tools/tune/RESULTS.md).")
    open(TUNED_RS, "w").write("\n".join(head) + "\n" + new + text[j:])
    print(new)


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        sys.exit(2)

    def opt(name, default, conv=str):
        if name in args:
            i = args.index(name)
            v = conv(args[i + 1])
            del args[i : i + 2]
            return v
        return default

    bin_path = opt("--bin", os.path.join(REPO, "target", "release", "musicbox"))
    budget = opt("--budget", 150, int)
    popsize = opt("--popsize", 10, int)
    sigma = opt("--sigma", 0.2, float)
    lam = opt("--lam", 0.05, float)
    w = opt("--w", 0.25, float)
    tag = opt("--tag", None)
    ranges = {}
    while "--range" in args:
        name, span = opt("--range", None).split("=")
        a, b = (float(x) for x in span.split(":"))
        group = {"levels": [k[0] for k in KNOBS[:CHANNELS]], "tilt": ["tilt_lo", "tilt_hi"]}.get(name, [name])
        for n in group:
            assert n in [k[0] for k in KNOBS], f"unknown knob {n}"
            ranges[n] = (a, b)
    cmd, genres = args[0], args[1:]
    for g in genres:
        assert g in GENRES, f"unknown genre {g}"
    os.makedirs(TUNE, exist_ok=True)
    if cmd == "table":
        print(table(genres))
        return
    if cmd == "write":
        write_tuned()
        return
    t = Tuner(bin_path, lam=lam, w=w, ranges=ranges, tag=tag)
    for g in genres:
        t0 = time.time()
        if cmd in ("search", "run"):
            t.search(g, budget=budget, popsize=popsize, sigma=sigma)
        if cmd in ("heldout", "run"):
            t.heldout(g)
        print(f"[{g}] {cmd} took {(time.time() - t0) / 60:.1f} min", flush=True)


if __name__ == "__main__":
    main()
