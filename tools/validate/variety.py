#!/usr/bin/env python3
"""Checks on the songs written with more variety (--variety more, a song id's v), by rendering to
files (nothing is played).

    tools/validate/variety.py same OLD NEW [SECONDS]   every genre (3 seeds) and 14 mixes, on the
        synthesiser and on the recordings, rendered by OLD (a musicbox built before the variety) and
        by NEW with --variety less: the same bytes, or the option leaks
    tools/validate/variety.py loud BIN [SECONDS]       every genre (3 seeds) both ways: each one's
        level (RMS, dBFS) and peak, and how far the variety moved them (a genre that moved more than
        1.5 dB, or reached full scale, is marked)
    tools/validate/variety.py pairs BIN DIR            three songs of each of eight genres as MP3s in
        DIR, each as its songs were and with more variety, with a playlist, to listen to

How much one genre's songs differ, and which recordings they play, are tests:

    cargo test --release variety_table -- --ignored --nocapture
    cargo test --release variety_recordings -- --ignored --nocapture
    MUSICBOX_VARIETY_GENRE=70s cargo test --release variety_songs -- --ignored --nocapture

Renders go under ~/.cache/musicbox/validate/variety and are deleted as they are read. Standard
library only.
"""
import array
import hashlib
import math
import os
import subprocess
import sys
import wave
from concurrent.futures import ThreadPoolExecutor

OUT = os.path.expanduser('~/.cache/musicbox/validate/variety/renders')
GENRES = ('ambient techno synthwave lo-fi polka tavern ska mariachi gamelan starconflict celtic western classical '
          'orchestra rock 60s 70s 80s 90s 2000s 2010s 2020s kenyan yoga elevator corporate worship praise').split()
MIXES = ['rock+ska', 'celtic+70s', 'techno+2010s', '80s+polka', 'praise+kenyan', 'western+rock', 'lo-fi+elevator',
         '2020s+techno', 'classical+60s', 'ska+worship', 'mariachi+90s', 'yoga+gamelan', 'ambient+2000s',
         'corporate+tavern']
SEEDS = (11, 2817, 90210)


def render(binary, tag, genre, seed, secs, extra):
    path = os.path.join(OUT, f'{tag}.wav')
    r = subprocess.run([binary, '--wav', genre, secs, path, '--seed', str(seed)] + extra, capture_output=True, text=True)
    if r.returncode != 0:
        return None
    with open(path, 'rb') as f:
        data = f.read()
    os.remove(path)
    return data


def same(old, new, secs):
    jobs = [(g, s, v) for g in GENRES for s in SEEDS for v in ('synth', 'recorded')]
    jobs += [(m, s, v) for m in MIXES for s in (5, 777) for v in ('synth', 'recorded')]

    def one(job):
        g, s, v = job
        tag = f'{g}-{s}-{v}'.replace('+', '_')
        a = render(old, tag + '-old', g, s, secs, ['--voices', v])
        b = render(new, tag + '-new', g, s, secs, ['--voices', v, '--variety', 'less'])
        return job, a is not None and b is not None and hashlib.sha256(a).digest() == hashlib.sha256(b).digest()

    bad = 0
    with ThreadPoolExecutor(max_workers=6) as ex:
        for job, ok in ex.map(one, jobs):
            if not ok:
                bad += 1
                print('DIFFERS', *job)
    print(f'{len(jobs)} renders of {secs} s each way: {len(jobs) - bad} the same bytes, {bad} not')
    return bad == 0


def loud(binary, secs):
    def level(job):
        g, s, way = job
        data = render(binary, f'{g}-{s}-{way}', g, s, secs, ['--variety', way])
        if data is None:
            return job, None
        tmp = os.path.join(OUT, f'{g}-{s}-{way}.lvl.wav')
        with open(tmp, 'wb') as f:
            f.write(data)
        with wave.open(tmp) as w:
            a = array.array('h', w.readframes(w.getnframes()))
        os.remove(tmp)
        ms = sum(x * x for x in a) / max(1, len(a))
        return job, (10 * math.log10(ms / 32768 ** 2 + 1e-12), max(abs(x) for x in a) / 32768)

    res = {}
    jobs = [(g, s, way) for g in GENRES for s in SEEDS for way in ('less', 'more')]
    with ThreadPoolExecutor(max_workers=4) as ex:
        for (g, s, way), v in ex.map(level, jobs):
            if v:
                res.setdefault((g, way), []).append(v)
    print('genre\tless dB\tmore dB\tmoved\tpeak less\tpeak more')
    ok = True
    for g in GENRES:
        a, b = res[(g, 'less')], res[(g, 'more')]
        la, lb = sum(x[0] for x in a) / len(a), sum(x[0] for x in b) / len(b)
        pa, pb = max(x[1] for x in a), max(x[1] for x in b)
        flag = ('  <-- moved' if abs(lb - la) > 1.5 else '') + ('  <-- full scale' if pb >= 0.999 else '')
        ok = ok and not flag
        print(f'{g}\t{la:.1f}\t{lb:.1f}\t{lb - la:+.1f}\t{pa:.3f}\t{pb:.3f}{flag}')
    return ok


def pairs(binary, out):
    os.makedirs(out, exist_ok=True)
    jobs, n = [], 0
    for g in ['rock', '70s', '90s', '2010s', 'kenyan', 'worship', 'western', 'techno']:
        for s in (101, 202, 303):
            for way, name in (('less', 'as-before'), ('more', 'more-variety')):
                n += 1
                jobs.append((g, s, way, f'{n:02d}-{g}-seed{s}-{name}.mp3'))

    def one(job):
        g, s, way, name = job
        r = subprocess.run([binary, '--export', g, os.path.join(out, name), '--seed', str(s), '--variety', way],
                           capture_output=True, text=True)
        return name, r.returncode

    ok = True
    with ThreadPoolExecutor(max_workers=3) as ex:
        for name, code in ex.map(one, jobs):
            ok = ok and code == 0
            print('made' if code == 0 else 'FAILED', name)
    with open(os.path.join(out, 'variety.m3u'), 'w') as f:
        f.write('#EXTM3U\n' + ''.join(j[3] + '\n' for j in jobs))
    print('playlist:', os.path.join(out, 'variety.m3u'))
    return ok


def main():
    a = sys.argv[1:]
    os.makedirs(OUT, exist_ok=True)
    if len(a) >= 3 and a[0] == 'same':
        ok = same(a[1], a[2], a[3] if len(a) > 3 else '150')
    elif len(a) >= 2 and a[0] == 'loud':
        ok = loud(a[1], a[2] if len(a) > 2 else '150')
    elif len(a) == 3 and a[0] == 'pairs':
        ok = pairs(a[1], a[2])
    else:
        sys.exit(__doc__)
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
