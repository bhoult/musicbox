# /// script
# requires-python = ">=3.10,<3.12"
# dependencies = ["essentia-tensorflow", "numpy"]
# ///
"""What a recording sounds like, measured: pretrained taggers' verdicts and
signal features, one JSON line a file, embeddings beside them.

    uv run tools/validate/critic.py OUT.jsonl FILE_OR_DIR... [--label NAME] [--jobs N] [--fast]

--fast: the taggers alone, no signal features.

The taggers (Essentia's models, fetched by tools/validate/get-models.sh into
~/.cache/musicbox/validate/models; CC BY-NC-SA 4.0, used here only to measure):
  - Discogs-EffNet: 400 Discogs styles, and a 1280-d embedding
  - MTG-Jamendo: 87 genres and 40 instruments, on the same embedding
  - YAMNet: AudioSet's 521 classes
Signal features (Essentia): tempo and its confidence, onset rate, loudness
(EBU R128) and its range, spectral centroid and flatness, the share of energy
in five bands, key and scale, danceability, and how the beat's sixteenths
carry the onsets (a beat-synchronous onset profile).

Each file's first 6 minutes are measured. Runs on the CPU.
"""

import json
import os
import sys

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "-1")
os.environ.setdefault("TF_CPP_MIN_LOG_LEVEL", "3")

MODELS = os.path.expanduser("~/.cache/musicbox/validate/models")
MAX_SECS = 360
EXTS = (".wav", ".mp3", ".ogg", ".flac", ".oga", ".opus", ".m4a")


def head(name):
    meta = json.load(open(os.path.join(MODELS, name + ".json")))
    return meta


class Critic:
    def __init__(self):
        import essentia.standard as es

        self.es = es
        m = lambda f: os.path.join(MODELS, f)
        self.effnet = es.TensorflowPredictEffnetDiscogs(graphFilename=m("discogs-effnet-bs64-1.pb"), output="PartitionedCall:1")
        self.heads = {}
        for name in ["genre_discogs400-discogs-effnet-1", "mtg_jamendo_genre-discogs-effnet-1", "mtg_jamendo_instrument-discogs-effnet-1"]:
            meta = head(name)
            inp = meta["schema"]["inputs"][0]["name"]
            out = meta["schema"]["outputs"][0]["name"]
            self.heads[name] = (es.TensorflowPredict2D(graphFilename=m(name + ".pb"), input=inp, output=out), meta["classes"])
        ymeta = head("audioset-yamnet-1")
        self.yamnet = (es.TensorflowPredictVGGish(graphFilename=m("audioset-yamnet-1.pb"), input="melspectrogram", output="activations"), ymeta["classes"])

    def measure(self, path, fast=False):
        import numpy as np

        es = self.es
        out = {"file": path}
        # the taggers, at 16 kHz mono
        a16 = es.MonoLoader(filename=path, sampleRate=16000, resampleQuality=4)()
        a16 = a16[: 16000 * MAX_SECS]
        out["secs"] = len(a16) / 16000
        emb = self.effnet(a16)
        out["embedding"] = np.mean(emb, axis=0).tolist()
        for name, (model, classes) in self.heads.items():
            act = np.mean(model(emb), axis=0)
            out[name.split("-")[0]] = {c: round(float(v), 5) for c, v in zip(classes, act)}
        model, classes = self.yamnet
        act = np.mean(model(a16), axis=0)
        out["yamnet"] = {c: round(float(v), 5) for c, v in zip(classes, act)}
        if fast:
            return out

        # the signal, at 44.1 kHz
        st, sr, nch, _, _, _ = es.AudioLoader(filename=path)()
        st = st[: int(sr) * MAX_SECS]
        if nch == 1:
            st = np.stack([st[:, 0], st[:, 0]], axis=1) if st.ndim == 2 else np.stack([st, st], axis=1)
        if sr != 44100:
            l = es.Resample(inputSampleRate=sr, outputSampleRate=44100)(np.ascontiguousarray(st[:, 0]))
            r = es.Resample(inputSampleRate=sr, outputSampleRate=44100)(np.ascontiguousarray(st[:, 1]))
            n = min(len(l), len(r))
            st = np.stack([l[:n], r[:n]], axis=1).astype(np.float32)
        mono = np.ascontiguousarray(st.mean(axis=1), dtype=np.float32)
        _, _, integrated, lra = es.LoudnessEBUR128(sampleRate=44100)(np.ascontiguousarray(st, dtype=np.float32))
        out["lufs"] = float(integrated)
        out["lra"] = float(lra)
        bpm, beats, conf, _, _ = es.RhythmExtractor2013(method="multifeature")(mono)
        out["bpm"] = float(bpm)
        out["bpm_conf"] = float(conf)
        out["onset_rate"] = float(es.OnsetRate()(mono)[1])
        key, scale, strength = es.KeyExtractor(profileType="edma")(mono)
        out["key"] = f"{key} {scale}"
        out["key_strength"] = float(strength)
        out["danceability"] = float(es.Danceability()(mono)[0])

        # the spectrum: centroid, flatness, the share in five bands
        w = es.Windowing(type="hann")
        spec = es.Spectrum()
        fft = 2048
        hop = 4096
        frames = [spec(w(f)) for f in es.FrameGenerator(mono, frameSize=fft, hopSize=hop, startFromZero=True)]
        if frames:
            p = np.mean(np.array(frames) ** 2, axis=0)
            f = np.arange(len(p)) * 44100 / fft
            tot = p.sum() + 1e-20
            out["centroid"] = float((f * p).sum() / tot)
            out["flatness"] = float(np.mean([es.Flatness()(fr) if fr.sum() > 0 else 0 for fr in frames[:: 4]]))
            bands = [(0, 60), (60, 250), (250, 2000), (2000, 8000), (8000, 22050)]
            out["bands"] = [float(p[(f >= a) & (f < b)].sum() / tot) for a, b in bands]
        # stereo width: the side's share of the energy
        mid, side = (st[:, 0] + st[:, 1]) / 2, (st[:, 0] - st[:, 1]) / 2
        out["width"] = float((side ** 2).sum() / ((mid ** 2).sum() + (side ** 2).sum() + 1e-20))

        # where in the beat the onsets fall: onset strength averaged over each
        # beat's four sixteenths (by the beat tracker's beats), normalised
        if len(beats) > 8:
            od = es.OnsetDetection(method="complex")
            ffta = es.FFT()
            c2p = es.CartesianToPolar()
            hop2 = 256
            env = []
            for fr in es.FrameGenerator(mono, frameSize=1024, hopSize=hop2, startFromZero=True):
                mag, ph = c2p(ffta(w(fr)))
                env.append(od(mag, ph))
            env = np.array(env)
            t = np.arange(len(env)) * hop2 / 44100
            prof = np.zeros(4)
            for b0, b1 in zip(beats[:-1], beats[1:]):
                for k in range(4):
                    s0 = b0 + (b1 - b0) * (k - 0.5) / 4
                    s1 = b0 + (b1 - b0) * (k + 0.5) / 4
                    m = (t >= s0) & (t < s1)
                    if m.any():
                        prof[k] += env[m].max()
            out["beat_profile"] = (prof / (prof.sum() + 1e-20)).round(4).tolist()
        return out


def files_of(args):
    for a in args:
        if os.path.isdir(a):
            for root, _, names in os.walk(a):
                for n in sorted(names):
                    if n.lower().endswith(EXTS):
                        yield os.path.join(root, n)
        else:
            yield a


CRITIC = None
FAST = False


def work(path):
    global CRITIC
    if CRITIC is None:
        CRITIC = Critic()
    try:
        return CRITIC.measure(path, FAST)
    except Exception as e:  # a file that will not decode: noted, skipped
        return {"file": path, "error": str(e)}


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        print(__doc__)
        sys.exit(2)
    label = None
    jobs = 4
    if "--label" in args:
        i = args.index("--label")
        label = args[i + 1]
        del args[i : i + 2]
    if "--jobs" in args:
        i = args.index("--jobs")
        jobs = int(args[i + 1])
        del args[i : i + 2]
    global FAST
    if "--fast" in args:
        args.remove("--fast")
        FAST = True
    out_path, inputs = args[0], args[1:]
    done = set()
    if os.path.exists(out_path):
        for line in open(out_path):
            try:
                done.add(json.loads(line)["file"])
            except Exception:
                pass
    todo = [f for f in files_of(inputs) if f not in done]
    print(f"{len(todo)} files to measure ({len(done)} done before)", file=sys.stderr)
    from multiprocessing import Pool

    with Pool(jobs) as pool, open(out_path, "a") as out:
        for k, r in enumerate(pool.imap_unordered(work, todo), 1):
            if label:
                r["label"] = label
            out.write(json.dumps(r) + "\n")
            out.flush()
            if "error" in r:
                print(f"[{k}/{len(todo)}] {r['file']}: {r['error']}", file=sys.stderr)
            else:
                d = r["genre_discogs400"]
                top = sorted(d, key=d.get, reverse=True)[:3]
                bpm = f"{r['bpm']:.0f} bpm; " if "bpm" in r else ""
                print(f"[{k}/{len(todo)}] {os.path.basename(r['file'])}: {bpm}{', '.join(top)}", file=sys.stderr)


if __name__ == "__main__":
    main()
