# /// script
# requires-python = ">=3.10"
# dependencies = ["numpy", "torch"]
#
# [tool.uv.sources]
# torch = { index = "pytorch-cpu" }
#
# [[tool.uv.index]]
# name = "pytorch-cpu"
# url = "https://download.pytorch.org/whl/cpu"
# explicit = true
# ///
"""musicbox's tune network (src/tunes.rs): a small GRU that writes the parts
of celtic dance tunes a note at a time, learned from O'Neill's Music of
Ireland (1903) as the O'Neill's Project transcribed it (GPL-2.0-or-later;
credits in tools/neural/TUNES.md).

    uv run tools/neural/tunes.py prep   [--abc DIR] [--work DIR]
    uv run tools/neural/tunes.py train  [--work DIR] [--layers 2] [--hidden 128] [--drop 0.25] [--out models/tunes_celtic.mbt]
    uv run tools/neural/tunes.py sweep  [--work DIR]          (a few sizes; the report in sweep.json)
    uv run tools/neural/tunes.py check  [--work DIR] [--model models/tunes_celtic.mbt]
    uv run tools/neural/tunes.py all    (prep, train, check)

(CPU only: one thread; ~/.cache/musicbox/tunes is the work directory.)

prep reads the ABC blocks (~/.cache/musicbox/tunes/oneills/F/*.abc), keeps
one transcription per X: number and turns each tune it can into two parts
of eight bars (parts.json, prep.json: what came through and why the rest
did not). train fits the network with teacher forcing, a tenth of the
tunes held out, and writes the MBT1 file. check reads the MBT1 file back
with a numpy port of tunes.rs (its masks; heat 1.0), writes sample parts as
ABC (samples.abc), compares their statistics with the held-out tunes'
(stats.json) and writes check.json, three steps of a fixed sequence with
the logits PyTorch gives, for tunes.rs's tests.

How the ABC becomes parts:
 - Grace notes, decorations, slurs, chord symbols and annotations dropped;
   a chord ([E3A3]) is its top note, the first note's length; "Hx" (a
   fermata's placeholder) nothing.
 - Lengths in 16ths. Broken rhythm a>b 3:1 of the pair (3+1 16ths for two
   eighths), a>>b 7:1. A tuplet's notes share its time on the grid, the
   longer ones first: (3abc in eighths 2,1,1; in quarters 3,3,2; in 16ths
   (2 16ths for 3 notes) its first and last note a 16th each. Anything
   still off the grid (32nds) is rounded by onset, a note rounded to
   nothing dropped.
 - 2/4 hornpipes (O'Neill's prints many in half values) are doubled to 4/4.
 - A tie joins notes of one pitch within a bar; across a bar line they stay
   two notes (the runtime's notes never cross one). In the dance tunes a
   rest lengthens the note before it; airs keep their rests.
 - Sections end at ||, |], :|, |: and ::. With |1 |2 endings the first is
   kept, the second (as many bars as the first) dropped. A section's pickup
   is dropped; its last bar, short by a pickup (its own, the next section's
   or the tune's) or by half a bar at most, has its last note lengthened to
   fill it. A four-bar strain with a repeat is played through (first
   ending, then second). The first two sections must then be eight whole
   bars each (a tune of one 16-bar section is cut in two): the A and the B
   part. Further parts unused.
 - Notes over 12 16ths: 12, the rest of the length a second note of the
   same pitch.
 - Pitches in semitones above the tonic of K: (major and ionian as ionian,
   minor and aeolian as aeolian, dorian, mixolydian; other modes skipped),
   but K: is the signature as O'Neill's printed it: the tonic is the note
   both parts end on when that is one of the four modes' tonics in the
   signature, and the third, sixth and seventh the ones the notes use most
   (`tonic_mode`). Each part is moved by octaves to put the most of its
   length within the runtime's range (A -3..12, B -1..15), a tie to the
   octave nearest the tonic's.
 - Chords, two a bar: each half bar's notes (by length, more on its first
   16th, then the beats, then the eighths) scored against the mode's
   triads (not the diminished; in mixolydian also the major V the rules
   use), a chord tone 1, its root 0.25 more, the tonic chord a little
   favoured and the rules' chords less; the best (root above the tonic,
   minor or not), ties to the tonic's, the fifth's, the fourth's.
"""

import os

os.environ.setdefault("CUDA_VISIBLE_DEVICES", "-1")

import argparse
import collections
import hashlib
import json
import math
import random
import re
import struct
import sys
import time
from fractions import Fraction as Fr
from pathlib import Path

import numpy as np

# tunes.rs's
TYPES, DURS, PITCHES, LOW = 6, 12, 40, -12
REST = PITCHES - 1
FEATS = TYPES + 4 + 1 + 8 + 16 + 13 + PITCHES + DURS
T_REEL, T_JIG, T_HORNPIPE, T_MARCH, T_MARCH68, T_AIR = range(6)
TYPE_NAMES = ["reel", "jig", "hornpipe", "march", "march68", "air"]
MODE_NAMES = ["ionian", "dorian", "mixolydian", "aeolian"]
# a bar in 16ths, a beat in 16ths (for the rhythm cells)
BAR = {T_REEL: 16, T_JIG: 12, T_HORNPIPE: 16, T_MARCH: 8, T_MARCH68: 12, T_AIR: 12}
BEAT = {T_REEL: 4, T_JIG: 6, T_HORNPIPE: 4, T_MARCH: 4, T_MARCH68: 6, T_AIR: 4}
# the runtime's ranges (celtic.rs, a session's): A part, B part
RANGE = [(-3, 12), (-1, 15)]

HOME = Path.home()
ABC_DIR = HOME / ".cache/musicbox/tunes/oneills/F"
WORK = HOME / ".cache/musicbox/tunes"
REPO = Path(__file__).resolve().parents[2]


class Skip(Exception):
    pass


# ---------------------------------------------------------------------------
# ABC

PC = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
LETTERS = "CDEFGAB"
SCALES = {
    "ionian": [0, 2, 4, 5, 7, 9, 11],
    "dorian": [0, 2, 3, 5, 7, 9, 10],
    "phrygian": [0, 1, 3, 5, 7, 8, 10],
    "lydian": [0, 2, 4, 6, 7, 9, 11],
    "mixolydian": [0, 2, 4, 5, 7, 9, 10],
    "aeolian": [0, 2, 3, 5, 7, 8, 10],
    "locrian": [0, 1, 3, 5, 6, 8, 10],
}


def parse_key(s):
    """K: as (tonic pitch class, mode name, the signature: letter -> accidental)."""
    m = re.match(r"\s*([A-G])([#b]?)\s*([A-Za-z]*)\s*$", s)
    if not m:
        raise Skip("key")
    letter, acc, mode = m.group(1), m.group(2), m.group(3).lower()
    if mode in ("", "maj", "major") or mode.startswith("ion"):
        name = "ionian"
    elif mode in ("m", "min", "minor") or mode.startswith("aeo"):
        name = "aeolian"
    else:
        name = next((k for k in SCALES if mode.startswith(k[:3])), None)
        if name is None:
            raise Skip("key")
    tonic = (PC[letter] + {"#": 1, "b": -1, "": 0}[acc]) % 12
    sig = {}
    li = LETTERS.index(letter)
    for d, iv in enumerate(SCALES[name]):
        lt = LETTERS[(li + d) % 7]
        a = ((tonic + iv) - PC[lt] + 6) % 12 - 6
        sig[lt] = a
    return tonic, name, sig


NOTE = re.compile(r"(\^\^|\^|__|_|=)?([A-Ga-g])([,']*)")
DUR = re.compile(r"(\d*)(/*)(\d*)")
ACC = {"^": 1, "^^": 2, "_": -1, "__": -2, "=": 0}


def mult(m):
    a, s, b = m.groups()
    num = int(a) if a else 1
    if not s:
        return Fr(num)
    if b:
        return Fr(num, int(b) * 2 ** (len(s) - 1))
    return Fr(num, 2 ** len(s))


def bar_flags(text):
    """A bar line's meaning: (a repeat ends, one starts, a double bar)."""
    return (
        text.startswith(":") or "::" in text,
        text.endswith(":") or "::" in text,
        "||" in text or "|]" in text or "[|" in text,
    )


class Lexer:
    """An ABC tune's body as notes (['n', midi or None, length in wholes,
    tied, tuplet]) and bar lines (['bar', flags, ending])."""

    def __init__(self, unit, sig, compound, key, meter):
        self.unit, self.sig, self.compound = unit, sig, compound
        self.key, self.meter = key, meter
        self.ev = []
        self.baracc = {}
        self.tup = None
        self.tups = 0
        self.broken = None
        self.counts = collections.Counter()

    def last_note(self):
        for e in reversed(self.ev):
            if e[0] == "n":
                return e
            if e[0] == "bar":
                return None
        return None

    def note(self, pitch, m):
        d = self.unit * m
        if self.broken is not None:
            d *= self.broken
            self.broken = None
        tid = None
        if self.tup:
            d *= self.tup[1]
            tid = self.tup[2]
            self.tup[0] -= 1
            if self.tup[0] <= 0:
                self.tup = None
        self.ev.append(["n", pitch, d, False, tid])

    def pitch(self, acc, letter, octs):
        up = letter.upper()
        base = 60 + PC[up] + (12 if letter.islower() else 0) + 12 * octs.count("'") - 12 * octs.count(",")
        if acc:
            a = ACC[acc]
            self.baracc[(up, base)] = a
        else:
            a = self.baracc.get((up, base), self.sig[up])
        return base + a

    def bar(self, text, ending):
        f = bar_flags(text)
        self.baracc = {}
        if self.ev and self.ev[-1][0] == "bar":
            p = self.ev[-1]
            p[1] = tuple(a or b for a, b in zip(p[1], f))
            p[2] = p[2] or ending
        else:
            self.ev.append(["bar", f, ending])

    def field(self, k, v):
        v = v.strip()
        if k == "K":
            if v != self.key:
                raise Skip("key change")
        elif k == "M":
            if v.replace(" ", "") != self.meter:
                raise Skip("meter change")
        elif k == "L":
            m = re.match(r"(\d+)/(\d+)", v)
            if not m:
                raise Skip("unparsed")
            self.unit = Fr(int(m.group(1)), int(m.group(2)))

    def line(self, s):
        i, n = 0, len(s)
        while i < n:
            c = s[i]
            if c == "%":
                break
            if c in " \t\\\r":
                i += 1
                continue
            if c == '"':
                j = s.find('"', i + 1)
                i = n if j < 0 else j + 1
                continue
            if c == "!":
                j = s.find("!", i + 1)
                i = i + 1 if j < 0 else j + 1
                continue
            if c == "+":
                m = re.match(r"\+[^+\s]*\+", s[i:])
                i += len(m.group()) if m else 1
                continue
            if c == "{":
                j = s.find("}", i + 1)
                i = n if j < 0 else j + 1
                self.counts["grace"] += 1
                continue
            if c == "(":
                m = re.match(r"\((\d+)(?::(\d*))?(?::(\d*))?", s[i:])
                if m:
                    p = int(m.group(1))
                    q = int(m.group(2)) if m.group(2) else None
                    if q is None:
                        q = {2: 3, 3: 2, 4: 3, 6: 2, 8: 3}.get(p, 3 if self.compound else 2)
                    r = int(m.group(3)) if m.group(3) else p
                    self.tups += 1
                    self.tup = [r, Fr(q, p), self.tups]
                    self.counts[f"tuplet {p}"] += 1
                    i += len(m.group())
                else:
                    i += 1
                continue
            if c == ")":
                i += 1
                continue
            if c in "|:" or (c == "[" and i + 1 < n and (s[i + 1] == "|" or s[i + 1].isdigit())):
                j, text = i, ""
                while j < n:
                    if s[j] in "|:":
                        text += s[j]
                    elif s[j] == "]" and text.endswith("|"):
                        text += "]"
                    elif s[j] == "[" and j + 1 < n and s[j + 1] == "|":
                        text += "["
                    else:
                        break
                    j += 1
                m = re.match(r"\[?(\d+)(?:[,-]\d+)*", s[j:])
                ending = None
                if m:
                    ending = int(m.group(1))
                    j += len(m.group())
                self.bar(text, ending)
                i = j
                continue
            if c == "[":
                m = re.match(r"\[([A-Za-z]):([^\]]*)\]", s[i:])
                if m:
                    self.field(m.group(1), m.group(2))
                    i += len(m.group())
                    continue
                j = s.find("]", i)
                if j < 0:
                    raise Skip("unparsed")
                inner, top, first = s[i + 1 : j], None, None
                k = 0
                while k < len(inner):
                    m = NOTE.match(inner, k)
                    if not m:
                        k += 1
                        continue
                    p = self.pitch(*m.groups())
                    d = DUR.match(inner, m.end())
                    if first is None:
                        first = mult(d)
                    top = p if top is None else max(top, p)
                    k = d.end()
                d = DUR.match(s, j + 1)
                if top is not None:
                    self.note(top, first * mult(d))
                    self.counts["chord"] += 1
                    if inner.rstrip().endswith("-"):
                        self.ev[-1][3] = True
                i = d.end()
                continue
            m = NOTE.match(s, i)
            if m:
                p = self.pitch(*m.groups())
                d = DUR.match(s, m.end())
                self.note(p, mult(d))
                i = d.end()
                continue
            if c in "zx":
                d = DUR.match(s, i + 1)
                if c == "x" and s[:i].rstrip().endswith("H"):
                    # "Hx": a fermata's placeholder, no time
                    i = d.end()
                    continue
                self.note(None, mult(d))
                self.counts["rest"] += 1
                i = d.end()
                continue
            if c == "Z":
                raise Skip("multi-bar rest")
            if c in "<>":
                j = i
                while j < n and s[j] == c:
                    j += 1
                k = j - i
                last = self.last_note()
                short = Fr(1, 2**k)
                if last is not None:
                    if c == ">":
                        last[2] *= 2 - short
                        self.broken = short
                    else:
                        last[2] *= short
                        self.broken = 2 - short
                    self.counts["broken"] += 1
                i = j
                continue
            if c == "-":
                last = self.last_note()
                if last is not None:
                    last[3] = True
                i += 1
                continue
            if c == "&":
                raise Skip("voice overlay")
            i += 1


def split_tunes(text):
    """An ABC file's tunes: (X, the tune's text)."""
    out = []
    for b in re.split(r"\n(?=X:)", text):
        m = re.match(r"X:\s*(\d+)", b.strip())
        if m:
            out.append((int(m.group(1)), b.strip()))
    return out


def header(text):
    """The header's fields (the first of each) and the body's lines."""
    lines = text.split("\n")
    h, body = {}, []
    for k, ln in enumerate(lines):
        m = re.match(r"([A-Za-z]):(.*)$", ln)
        if m and "K" not in h:
            h.setdefault(m.group(1), m.group(2).strip())
            if m.group(1) == "K":
                body = lines[k + 1 :]
                break
    return h, body


def tune_type(x, meter):
    """The network's type for O'Neill's X:, its meter: (type, time scale) or None."""
    if 1 <= x <= 700:
        return (T_AIR, 1) if meter == "3/4" else None
    if 701 <= x <= 1115:
        return (T_JIG, 1) if meter == "6/8" else None
    if 1176 <= x <= 1555:
        return (T_REEL, 1) if meter in ("C", "C|", "4/4") else None
    if 1556 <= x <= 1800:
        if meter in ("C", "C|", "4/4"):
            return (T_HORNPIPE, 1)
        return (T_HORNPIPE, 2) if meter == "2/4" else None
    if 1801 <= x <= 1850:
        return {"2/4": (T_MARCH, 1), "6/8": (T_MARCH68, 1)}.get(meter)
    return None


def rnd(x):
    """To the nearest 16th, halves up."""
    return math.floor(x + Fr(1, 2))


def quantize(notes, scale):
    """A bar's notes on the 16th grid: [pitch, start, length, tied], its length."""
    t, items, k = Fr(0), [], 0
    d16 = [n[2] * 16 * scale for n in notes]
    while k < len(notes):
        tid = notes[k][4]
        if tid is None:
            items.append((t, notes[k]))
            t += d16[k]
            k += 1
            continue
        g = [k]
        k += 1
        while k < len(notes) and notes[k][4] == tid:
            g.append(k)
            k += 1
        T = sum(d16[j] for j in g)
        r = len(g)
        if T.denominator == 1 and len({d16[j] for j in g}) == 1:
            Ti = int(T)
            if Ti >= r:
                base, extra = divmod(Ti, r)
                tt = t
                for q, j in enumerate(g):
                    items.append((tt, notes[j]))
                    tt += base + (1 if q < extra else 0)
            elif Ti >= 1:
                keep = [g[0]] + (g[r - (Ti - 1) :] if Ti > 1 else [])
                for q, j in enumerate(keep):
                    items.append((t + q, notes[j]))
        else:
            tt = t
            for j in g:
                items.append((tt, notes[j]))
                tt += d16[j]
        t += T
    total = rnd(t)
    out = []
    for j, (on, n) in enumerate(items):
        s = rnd(on)
        e = rnd(items[j + 1][0]) if j + 1 < len(items) else total
        if e > s:
            out.append([n[1], s, e - s, n[3]])
    # ties within the bar
    merged = []
    for n in out:
        if merged and merged[-1][3] and merged[-1][0] == n[0] and n[0] is not None:
            merged[-1][2] += n[2]
            merged[-1][3] = n[3]
        else:
            merged.append(n)
    return merged, total


def whole(bars, bar, shorts, pickup):
    """`bars` as whole bars: all but the last full; the last, short by one of
    `shorts` (a pickup's length) or by half a bar at most, completed by
    `pickup`'s notes if it is that long, else its last note lengthened. None
    if they are not."""
    if not bars or any(b["len"] != bar for b in bars[:-1]):
        return None
    last = bars[-1]
    if last["len"] == bar:
        return list(bars)
    short = bar - last["len"]
    if short <= 0 or (short not in shorts and short > bar // 2) or not last["notes"]:
        return None
    notes = [list(n) for n in last["notes"]]
    if pickup is not None and pickup["len"] == short and pickup["notes"]:
        notes += [[n[0], n[1] + last["len"], n[2], n[3]] for n in pickup["notes"]]
    else:
        notes[-1][2] += short
    return list(bars[:-1]) + [dict(last, notes=notes, len=bar)]


def sections(bars, bar):
    """The tune's sections as played once (a four-bar strain with a repeat
    played twice), lists of whole bars; and their lengths in bars."""

    def boundary(b):
        return any(b["end"])

    def new():
        return {"body": [], "e1": None, "e2": None, "rep": False}

    secs, cur, i = [], new(), 0
    while i < len(bars):
        b = bars[i]
        if b["ending"] == 1:
            e1 = []
            while i < len(bars):
                e1.append(bars[i])
                i += 1
                if boundary(bars[i - 1]) or (i < len(bars) and bars[i]["ending"]):
                    break
            cur["e1"], cur["rep"] = e1, True
            while i < len(bars) and (bars[i]["ending"] or 0) >= 2:
                grp = []
                while i < len(bars) and len(grp) < len(e1):
                    grp.append(bars[i])
                    i += 1
                    if boundary(bars[i - 1]):
                        break
                if cur["e2"] is None:
                    cur["e2"] = grp
            secs.append(cur)
            cur = new()
            continue
        cur["body"].append(b)
        i += 1
        if boundary(b):
            cur["rep"] = b["end"][0]
            secs.append(cur)
            cur = new()
    if cur["body"]:
        secs.append(cur)
    # a section that is only a pickup goes before the next
    k = 0
    while k < len(secs) - 1:
        s = secs[k]
        if s["e1"] is None and sum(b["len"] for b in s["body"]) < bar:
            secs[k + 1]["body"] = s["body"] + secs[k + 1]["body"]
            del secs[k]
        else:
            k += 1
    picks = [s["body"][0]["len"] if s["body"] and s["body"][0]["len"] < bar else 0 for s in secs]
    out = []
    for k, s in enumerate(secs):
        body, pick = (s["body"][1:], s["body"][0]) if picks[k] else (s["body"], None)
        shorts = {picks[k], picks[0], picks[k + 1] if k + 1 < len(secs) else 0}
        once = whole(body + (s["e1"] or []), bar, shorts, None)
        if once is None:
            raise Skip("bar length", k, [b["len"] for b in s["body"]], [b["len"] for b in s["e1"] or []], sorted(shorts))
        if len(once) == 4 and s["rep"]:
            first = whole(body + (s["e1"] or []), bar, shorts, pick or (secs[0]["body"][0] if picks[0] else None))
            second = whole(body + (s["e2"] or s["e1"] or []), bar, shorts, None)
            if first and second and len(first) + len(second) == 8:
                once = first + second
        out.append(once)
    return out, [len(s) for s in out]


def parse_tune(x, text):
    """One transcription as {type, mode, tonic, parts: [A, B]}, each part
    [(start, length, pitch above the tonic or None)] over eight bars."""
    h, body = header(text)
    meter = h.get("M", "").replace(" ", "")
    tt = tune_type(x, meter)
    if tt is None:
        raise Skip("not chosen")
    ty, scale = tt
    if "K" not in h:
        raise Skip("key")
    tonic, mode, sig = parse_key(h["K"])
    if mode not in MODE_NAMES:
        raise Skip("mode " + mode)
    num, den = (4, 4) if meter in ("C", "C|") else map(int, meter.split("/"))
    if "L" in h:
        m = re.match(r"(\d+)/(\d+)", h["L"])
        if not m:
            raise Skip("unparsed")
        unit = Fr(int(m.group(1)), int(m.group(2)))
    else:
        unit = Fr(1, 16) if Fr(num, den) < Fr(3, 4) else Fr(1, 8)
    lx = Lexer(unit, sig, num in (6, 9, 12), h["K"], meter)
    for ln in body:
        if not ln.strip():
            continue
        m = re.match(r"([A-Za-z]):(.*)$", ln)
        if m:
            lx.field(m.group(1), m.group(2))
            continue
        lx.line(ln)
    bar = BAR[ty]
    # the bars: notes on the grid, the ending they start, their end's bar line
    bars, cur, ending = [], [], None
    for e in lx.ev:
        if e[0] == "bar":
            if cur:
                notes, total = quantize(cur, scale)
                bars.append({"notes": notes, "len": total, "ending": ending, "end": e[1]})
                cur = []
            ending = e[2]
        else:
            cur.append(e)
    if cur:
        notes, total = quantize(cur, scale)
        bars.append({"notes": notes, "len": total, "ending": ending, "end": (False, False, True)})
    if not bars:
        raise Skip("unparsed")

    full, pattern = sections(bars, bar)
    if len(full) == 1 and len(full[0]) == 16:
        full = [full[0][:8], full[0][8:]]
    elif len(full) < 2 or len(full[0]) != 8 or len(full[1]) != 8:
        raise Skip("parts not 8+8" if len(full) >= 2 else "one part", pattern)
    parts = []
    for p, s in enumerate(full[:2]):
        notes = []
        for bi, b in enumerate(s):
            for n in b["notes"]:
                notes.append([n[0], bi * bar + n[1], n[2]])
        parts.append(clean(notes, ty, bar))
    tonic, mode, how = tonic_mode(parts, tonic, mode)
    rel = []
    for p, notes in enumerate(parts):
        rel.append(place(notes, tonic, *RANGE[p]))
    return {
        "x": x,
        "type": ty,
        "mode": MODE_NAMES.index(mode),
        "tonic": tonic,
        "key": h["K"],
        "read": how,
        "parts": rel,
        "extra_parts": len(full) - 2,
        "counts": dict(lx.counts),
    }


def tonic_mode(parts, tonic, mode):
    """The tune's tonic and mode. K: gives the key signature as O'Neill's
    printed it, often a major key's for a modal tune: the tonic is K:'s, or
    the note both parts end on if that is a tonic of one of the four modes in
    the signature; then the third, sixth and seventh are the ones the notes
    use most (the signature's where they use both alike)."""
    sig = {(tonic + iv) % 12 for iv in SCALES[mode]}
    ends = [[p for p, s, d in n if p is not None][-1] % 12 for n in parts]
    how = []
    if ends[0] == ends[1] != tonic:
        rot = sorted((v - ends[0]) % 12 for v in sig)
        m = next((k for k in MODE_NAMES if SCALES[k] == rot), None)
        if m:
            tonic, mode = ends[0], m
            how.append("tonic")
    w = collections.Counter()
    for n in parts:
        for p, s, d in n:
            if p is not None:
                w[(p - tonic) % 12] += d
    sc = set(SCALES[mode])
    minor, flat6, flat7 = 3 in sc, 8 in sc, 10 in sc
    if w[3] != w[4]:
        minor = w[3] > w[4]
    if w[8] != w[9]:
        flat6 = w[8] > w[9]
    if w[10] != w[11]:
        flat7 = w[10] > w[11]
    new = ("aeolian" if flat6 else "dorian") if minor else ("mixolydian" if flat7 else "ionian")
    if new != mode:
        how.append("mode")
    return tonic, new, how


def clean(notes, ty, bar):
    """Rests into notes (but in an air), notes over 12 16ths split."""
    out = []
    for k, (p, s, d) in enumerate(notes):
        if p is None and ty != T_AIR:
            if out and out[-1][1] // bar == s // bar:
                out[-1][2] += d
                continue
            prev = out[-1][0] if out else next((q[0] for q in notes[k:] if q[0] is not None), None)
            if prev is None:
                raise Skip("rests")
            p = prev
        if out and p is None and out[-1][0] is None:
            out[-1][2] += d
            continue
        out.append([p, s, d])
    res = []
    for p, s, d in out:
        while d > DURS:
            res.append([p, s, DURS])
            s, d = s + DURS, d - DURS
        res.append([p, s, d])
    return res


def place(notes, tonic, lo, hi):
    """Pitches above the tonic, moved by octaves for the most within lo..=hi."""
    ref = 60 + tonic
    best = None
    for sh in (0, -12, 12, -24, 24):
        out = sum(d for p, s, d in notes if p is not None and not lo <= p - ref + sh <= hi)
        if best is None or out < best[0]:
            best = (out, sh)
    sh = best[1]
    res = []
    for p, s, d in notes:
        if p is None:
            res.append((s, d, None))
            continue
        v = p - ref + sh
        while v < LOW:
            v += 12
        while v > LOW + PITCHES - 2:
            v -= 12
        res.append((s, d, v))
    return res


TONIC_NAMES = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]
MODE_ABC = ["", "dor", "mix", "m"]


def to_abc(x, title, notes, ty, mode, chords, tonic=2, comment=None):
    """A part as ABC (L:1/16), its chords as chord symbols; `tonic` the pitch
    class the part's 0 sounds as (D4 by default)."""
    name = TONIC_NAMES[tonic]
    _, _, sig = parse_key(name + MODE_ABC[mode])
    li = LETTERS.index(name[0])
    letters = [LETTERS[(li + d) % 7] for d in range(7)]
    sc = {(PC[lt] + sig[lt]) % 12: lt for lt in letters}
    bar = BAR[ty]
    meter = {T_REEL: "C|", T_JIG: "6/8", T_HORNPIPE: "C", T_MARCH: "2/4", T_MARCH68: "6/8", T_AIR: "3/4"}[ty]
    out = [f"X:{x}", f"T:{title}"]
    if comment:
        out.append(f"N:{comment}")
    out += [f"M:{meter}", "L:1/16", f"R:{TYPE_NAMES[ty]}", f"K:{name}{MODE_ABC[mode]}"]

    def spell(m, state):
        pc = m % 12
        if pc in sc:
            lt = sc[pc]
            acc = sig[lt]
        else:
            # flats for the minor and flat degrees, else sharps
            flat = (pc - tonic) % 12 in (1, 3, 8, 10)
            opts = []
            for lt in letters:
                a = ((pc - PC[lt]) + 6) % 12 - 6
                if abs(a) <= 1 and abs(a - sig[lt]) == 1:
                    opts.append((abs(a) if a != 0 else 0, (a - sig[lt] == 1) == flat, lt, a))
            _, _, lt, acc = min(opts)
        base = m - acc
        octv = base // 12 - 1
        cur = state.get((lt, octv), sig[lt])
        sym = ""
        if acc != cur:
            sym = {-1: "_", 0: "=", 1: "^", -2: "__", 2: "^^"}[acc]
            state[(lt, octv)] = acc
        s = lt if octv <= 4 else lt.lower()
        s += "," * max(0, 4 - octv) + "'" * max(0, octv - 5)
        return sym + s

    def chord_name(root, minor):
        pc = (tonic + root) % 12
        if pc in sc:
            lt = sc[pc]
            a = sig[lt]
            nm = lt + {-1: "b", 0: "", 1: "#"}[a]
        else:
            nm = TONIC_NAMES[pc]
        return nm + ("m" if minor else "")

    body, line, state, k = [], "", {}, 0
    last_chord = None
    for b in range(8):
        state = {}
        for s, d, p in [n for n in notes if n[0] // bar == b]:
            hb = b * 2 + (2 * (s % bar)) // bar
            if s % (bar // 2) == 0 and chords and chords[hb] != last_chord:
                line += f'"{chord_name(*chords[hb])}"'
                last_chord = chords[hb]
            tok = "z" if p is None else spell(60 + tonic + p, state)
            line += tok + (str(d) if d != 1 else "") + " "
        line = line.rstrip() + (" |" if b < 7 else " |]")
        if b % 4 == 3:
            body.append(line)
            line = ""
        else:
            line += " "
    return "\n".join(out + body) + "\n"


# ---------------------------------------------------------------------------
# chords

PREFER = [0, 7, 5, 10, 9, 8, 3, 2, 4]


def triads(mode):
    """The mode's triads (not the diminished) as (root, minor), in the order
    ties go: the tonic's, the fifth's, the fourth's, the seventh's...;
    mixolydian's also the major V (the rules' bVII+V)."""
    sc = SCALES[MODE_NAMES[mode]]
    out = []
    for d in range(7):
        r, t, f = sc[d], sc[(d + 2) % 7], sc[(d + 4) % 7]
        if (f - r) % 12 == 7:
            out.append((r, (t - r) % 12 == 3))
    if mode == 2:
        out.append((7, False))
    return sorted(out, key=lambda c: (PREFER.index(c[0]), c[1] != (mode in (1, 3) or c[0] != 0)))


def rules_chords(mode):
    """The chords the rules' progressions use in `mode`."""
    return {c for pool in RULES[mode] for plan in pool for c in halves(plan)}


def chord_pcs(root, minor):
    return {root % 12, (root + (3 if minor else 4)) % 12, (root + 7) % 12}


def metric(s, bar, beat):
    """A 16th's weight in the bar: a half bar's first, a beat's, an eighth's, other."""
    if s % (bar // 2) == 0:
        return 2.0
    if s % beat == 0:
        return 1.5
    if s % 2 == 0:
        return 1.0
    return 0.6


def infer_chords(notes, ty, mode):
    """Two chords a bar over a part's notes: [(root, minor)] * 16."""
    bar, beat = BAR[ty], BEAT[ty]
    half = bar // 2
    cands = triads(mode)
    vocab = rules_chords(mode)
    out = []
    for hb in range(16):
        a, b = hb * half, (hb + 1) * half
        w = collections.Counter()
        for s, d, p in notes:
            if p is None:
                continue
            ov = min(b, s + d) - max(a, s)
            if ov <= 0:
                continue
            w[p % 12] += ov * (metric(s, bar, beat) if s >= a else 1.0)
        tot = sum(w.values()) or 1.0
        best = None
        for k, (r, mi) in enumerate(cands):
            pcs = chord_pcs(r, mi)
            sc = sum(v for pc, v in w.items() if pc in pcs) + 0.25 * w[r]
            if k == 0:
                sc += 0.15 * tot
            if (r, mi) in vocab:
                sc += 0.05 * tot
            if best is None or sc > best[0] + 1e-9:
                best = (sc, (r, mi))
        if not w:
            best = (0, out[-1] if out else cands[0])
        out.append(best[1])
    return out


# the rules' chords (celtic.rs's CE_ION, CE_MIX, CE_DOR, CE_AEO): A parts, B parts
RULES = {
    0: (
        ["I I V V I I IV+V I", "I IV I V I IV V I", "I I IV IV I I V+IV I", "I vi IV V I vi V I"],
        ["IV IV I I IV IV V V", "vi vi IV IV I I V I", "IV I V vi IV I V I"],
    ),
    2: (
        ["I I bVII bVII I I bVII+V I", "I bVII I bVII I IV bVII I", "I I IV bVII I I bVII+IV I"],
        ["bVII bVII I I bVII bVII IV I", "IV IV I I bVII bVII I I", "I IV bVII IV I IV bVII I"],
    ),
    1: (
        ["i i bVII bVII i i bVII+v i", "i bVII i bVII i bVII IV+bVII i", "i i IV IV i i bVII i", "i bIII bVII IV i bIII bVII i"],
        ["bIII bIII bVII bVII bIII bVII i i", "bVII bVII i i bVII bVII IV i", "IV IV i i bVII bVII i i"],
    ),
    3: (
        ["i i bVII bVII i i bVI+bVII i", "i bVI bVII i i bVI bVII+v i", "i i bIII bVII i iv bVII i"],
        ["bIII bIII bVII bVII bVI bVI bVII i", "bIII bVII i i bIII bVII bVI+bVII i", "bVI bVII i i bVI bVII v i"],
    ),
}


def numeral(t):
    acc = 0
    if t[0] in "b#":
        acc = -1 if t[0] == "b" else 1
        t = t[1:]
    semi = {"I": 0, "II": 2, "III": 4, "IV": 5, "V": 7, "VI": 9, "VII": 11}[t.upper()]
    return ((semi + acc) % 12, t[0] in "iv")


def halves(plan):
    """A progression (celtic.rs's `halves`): a chord per half bar."""
    out = []
    for tok in plan.split():
        v = [numeral(x) for x in tok.split("+")]
        out += [v[0], v[0]] if len(v) == 1 else v[:2]
    return out


# ---------------------------------------------------------------------------
# prep

def prep(args):
    files = [ln.strip() for ln in (Path(args.abc).parent / "blocks.txt").read_text().split() if ln.strip()]
    versions = collections.defaultdict(list)
    for f in files:
        text = (Path(args.abc) / f).read_text(encoding="latin-1")
        for x, t in split_tunes(text):
            versions[x].append((f, t))
    kept, reasons, dups = [], collections.Counter(), 0
    by_type = collections.defaultdict(collections.Counter)
    counts = collections.Counter()
    for x in sorted(versions):
        got, why = None, None
        for f, t in versions[x]:
            try:
                tune = parse_tune(x, t)
                tune["file"] = f
                got = tune
                break
            except Skip as e:
                why = why or e.args[0]
        if len(versions[x]) > 1:
            dups += len(versions[x]) - 1
        h, _ = header(versions[x][0][1])
        tt = tune_type(x, h.get("M", "").replace(" ", ""))
        if got is None and why == "not chosen":
            continue
        name = TYPE_NAMES[got["type"] if got else tt[0]] if (got or tt) else "?"
        if got is None:
            reasons[why] += 1
            by_type[name]["skipped: " + why] += 1
            continue
        for p, notes in enumerate(got["parts"]):
            got["chords"] = got.get("chords", []) + [infer_chords(notes, got["type"], got["mode"])]
        by_type[name]["kept"] += 1
        counts.update(got.pop("counts"))
        kept.append(got)
    # held out: a tenth of the tunes of each type
    rng = random.Random(1903)
    for ty in range(TYPES):
        xs = [t["x"] for t in kept if t["type"] == ty]
        rng.shuffle(xs)
        val = set(xs[: max(1, round(len(xs) / 10))]) if xs else set()
        for t in kept:
            if t["type"] == ty:
                t["val"] = t["x"] in val
    work = Path(args.work)
    work.mkdir(parents=True, exist_ok=True)
    (work / "parts.json").write_text(json.dumps(kept))
    out = {t: dict(c) for t, c in by_type.items()}
    report = {
        "transcriptions": sum(len(v) for v in versions.values()),
        "x_numbers": len(versions),
        "second_transcriptions": dups,
        "by_type": out,
        "kept": len(kept),
        "held_out": sum(t["val"] for t in kept),
        "modes": dict(collections.Counter(f"{TYPE_NAMES[t['type']]} {MODE_NAMES[t['mode']]}" for t in kept)),
        "tonic_from_ending": sum("tonic" in t["read"] for t in kept),
        "mode_from_notes": sum("mode" in t["read"] for t in kept),
        "with_more_parts": sum(t["extra_parts"] > 0 for t in kept),
        "constructs": dict(counts),
    }
    stats = data_stats(kept)
    report.update(stats)
    (work / "prep.json").write_text(json.dumps(report, indent=1))
    print(json.dumps(report, indent=1))


def data_stats(kept):
    """How the parts sit: outside the runtime's range, ending on the tonic."""
    out_t, tot_t, end_ton, n = 0, 0, 0, 0
    for t in kept:
        for p, notes in enumerate(t["parts"]):
            lo, hi = RANGE[p]
            for s, d, v in notes:
                if v is not None:
                    tot_t += d
                    out_t += d if not lo <= v <= hi else 0
            last = [v for s, d, v in notes if v is not None][-1]
            end_ton += last % 12 == 0
            n += 1
    return {"out_of_range_share": round(out_t / max(1, tot_t), 4), "parts_ending_on_tonic": round(end_ton / max(1, n), 4)}


# ---------------------------------------------------------------------------
# sequences, as tunes.rs's `part` sees them


def features(ty, mode, part, bar, step, root, minor, prev):
    """tunes.rs's `features`."""
    x = np.zeros(FEATS, np.float32)
    o = 0
    for k, n in ((ty, TYPES), (mode, 4)):
        x[o + min(k, n - 1)] = 1.0
        o += n
    x[o] = float(min(part, 1))
    o += 1
    for k, n in ((bar, 8), (step, 16), (root % 12, 12)):
        x[o + min(k, n - 1)] = 1.0
        o += n
    x[o] = float(bool(minor))
    o += 1
    if prev is not None:
        p, d = prev
        x[o + min(p, PITCHES - 1)] = 1.0
        x[o + PITCHES + min(d, DURS - 1)] = 1.0
    return x


def pitch_index(v):
    return REST if v is None else v - LOW


def steps(ty, mode, part, notes, chords):
    """A part's steps: (the features' arguments, the length index, the pitch
    index, the room left in the bar)."""
    bar = BAR[ty]
    out, prev = [], None
    for s, d, v in notes:
        b, st = divmod(s, bar)
        root, minor = chords[min(b * 2 + (2 * st) // bar, len(chords) - 1)]
        di, pi = min(d, DURS) - 1, pitch_index(v)
        out.append(((ty, mode, part, b, st, root, bool(minor), prev), di, pi, bar - st))
        prev = (pi, di)
    return out


def sequences(tunes):
    """Each part as arrays: features, length, pitch, length mask, rest allowed."""
    seqs = []
    for t in tunes:
        for p in range(2):
            st = steps(t["type"], t["mode"], p, t["parts"][p], t["chords"][p])
            x = np.stack([features(*a) for a, _, _, _ in st])
            d = np.array([di for _, di, _, _ in st], np.int64)
            pi = np.array([q for _, _, q, _ in st], np.int64)
            room = np.array([r for _, _, _, r in st], np.int64)
            seqs.append({"x": x, "d": d, "p": pi, "room": room, "air": t["type"] == T_AIR, "type": t["type"], "x_no": t["x"]})
    return seqs


def load_parts(work):
    tunes = json.loads((Path(work) / "parts.json").read_text())
    for t in tunes:
        t["parts"] = [[tuple(n) for n in p] for p in t["parts"]]
        t["chords"] = [[(r, bool(m)) for r, m in c] for c in t["chords"]]
    return tunes


# ---------------------------------------------------------------------------
# the network


def torch_setup():
    import torch

    torch.set_num_threads(1)
    if torch.get_num_interop_threads() != 1:
        try:
            torch.set_num_interop_threads(1)
        except RuntimeError:
            pass
    return torch


def make_net(torch, hid, layers, drop):
    nn = torch.nn

    class Net(nn.Module):
        def __init__(self):
            super().__init__()
            self.gru = nn.GRU(FEATS, hid, layers, batch_first=True, dropout=drop if layers > 1 else 0.0)
            self.drop = nn.Dropout(drop)
            self.dur = nn.Linear(hid, DURS)
            self.pitch = nn.Linear(hid + DURS, PITCHES)
            self.hid, self.layers = hid, layers

        def forward(self, x, d):
            h, _ = self.gru(x)
            h = self.drop(h)
            dl = self.dur(h)
            oh = torch.nn.functional.one_hot(d, DURS).float()
            pl = self.pitch(torch.cat([h, oh], -1))
            return dl, pl

    return Net()


def batch(torch, seqs):
    T = max(len(s["d"]) for s in seqs)
    B = len(seqs)
    x = np.zeros((B, T, FEATS), np.float32)
    d = np.zeros((B, T), np.int64)
    p = np.zeros((B, T), np.int64)
    dm = np.zeros((B, T, DURS), bool)
    pm = np.ones((B, T, PITCHES), bool)
    valid = np.zeros((B, T), bool)
    for i, s in enumerate(seqs):
        n = len(s["d"])
        x[i, :n], d[i, :n], p[i, :n], valid[i, :n] = s["x"], s["d"], s["p"], True
        dm[i, :n] = np.arange(DURS)[None, :] < s["room"][:, None]
        if not s["air"]:
            pm[i, :, REST] = False
    dm[~valid] = True
    t = torch.from_numpy
    return t(x), t(d), t(p), t(dm), t(pm), t(valid)


def nll(torch, net, b):
    """Summed negative log-likelihoods (length, pitch) over a batch's notes,
    with the runtime's masks (a length within the bar; a rest only in an air),
    and the count of notes."""
    x, d, p, dm, pm, valid = b
    dl, pl = net(x, d)
    dl = dl.masked_fill(~dm, -1e9)
    pl = pl.masked_fill(~pm, -1e9)
    F = torch.nn.functional
    ld = F.cross_entropy(dl.reshape(-1, DURS), d.reshape(-1), reduction="none").reshape(d.shape)
    lp = F.cross_entropy(pl.reshape(-1, PITCHES), p.reshape(-1), reduction="none").reshape(p.shape)
    v = valid.float()
    return (ld * v).sum(), (lp * v).sum(), v.sum()


def evaluate(torch, net, seqs, by_type=False):
    net.eval()
    out = collections.defaultdict(lambda: [0.0, 0.0, 0.0])
    with torch.no_grad():
        for i in range(0, len(seqs), 64):
            chunk = seqs[i : i + 64]
            groups = [chunk] if not by_type else [[s for s in chunk if s["type"] == ty] for ty in range(TYPES)]
            for g in groups:
                if not g:
                    continue
                a, b, n = nll(torch, net, batch(torch, g))
                key = "all" if not by_type else TYPE_NAMES[g[0]["type"]]
                o = out[key]
                o[0] += a.item()
                o[1] += b.item()
                o[2] += n.item()
    return {k: {"length": v[0] / v[2], "pitch": v[1] / v[2], "notes": int(v[2])} for k, v in out.items()}


def fit(torch, train_seqs, val_seqs, hid, layers, drop, epochs, seed, lr=3e-3, log=print):
    torch.manual_seed(seed)
    rng = random.Random(seed)
    net = make_net(torch, hid, layers, drop)
    opt = torch.optim.Adam(net.parameters(), lr=lr, weight_decay=1e-6)
    sched = torch.optim.lr_scheduler.ReduceLROnPlateau(opt, factor=0.5, patience=5)
    best, best_state, since = None, None, 0
    order = list(range(len(train_seqs)))
    t0 = time.time()
    for ep in range(epochs):
        net.train()
        rng.shuffle(order)
        for i in range(0, len(order), 32):
            b = batch(torch, [train_seqs[j] for j in order[i : i + 32]])
            a, c, n = nll(torch, net, b)
            loss = (a + c) / n
            opt.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
            opt.step()
        v = evaluate(torch, net, val_seqs)["all"]
        tot = v["length"] + v["pitch"]
        sched.step(tot)
        if best is None or tot < best - 1e-4:
            best, since = tot, 0
            best_state = {k: t.detach().clone() for k, t in net.state_dict().items()}
            best_ep = ep
        else:
            since += 1
        if ep % 5 == 0 or since == 0:
            log(f"  {hid}x{layers} drop {drop} epoch {ep}: val length {v['length']:.4f} pitch {v['pitch']:.4f} (best {best:.4f} at {best_ep}) {time.time() - t0:.0f}s", flush=True)
        if since >= 16:
            break
    net.load_state_dict(best_state)
    net.eval()
    return net, best_ep


def mbt_size(hid, layers):
    n = 0
    for l in range(layers):
        inp = FEATS if l == 0 else hid
        n += 3 * hid * inp + 3 * hid * hid + 6 * hid
    n += DURS * hid + DURS + PITCHES * (hid + DURS) + PITCHES
    return 16 + 4 * n


def write_mbt(path, net):
    """tunes.rs's MBT1: the magic, features, hidden, layers; each layer's input
    weights, hidden weights, input biases, hidden biases; the length head; the
    pitch head (hidden + DURS inputs); f32s, little-endian, rows of outputs."""
    out = bytearray(b"MBT1") + struct.pack("<3I", FEATS, net.hid, net.layers)
    f = lambda t: t.detach().cpu().numpy().astype("<f4").tobytes()
    for l in range(net.layers):
        for name in ("weight_ih", "weight_hh", "bias_ih", "bias_hh"):
            out += f(getattr(net.gru, f"{name}_l{l}"))
    out += f(net.dur.weight) + f(net.dur.bias) + f(net.pitch.weight) + f(net.pitch.bias)
    assert len(out) == mbt_size(net.hid, net.layers)
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_bytes(bytes(out))


def bigram(train_seqs, val_seqs, alpha=0.5):
    """The baseline: the length from the length before, the pitch from the
    pitch before (add-alpha counts from the training parts), the same masks."""
    cd = np.full((DURS + 1, DURS), alpha)
    cp = np.full((PITCHES + 1, PITCHES), alpha)
    for s in train_seqs:
        pd, pp = DURS, PITCHES
        for d, p in zip(s["d"], s["p"]):
            cd[pd, d] += 1
            cp[pp, p] += 1
            pd, pp = d, p
    out = collections.defaultdict(lambda: [0.0, 0.0, 0])
    for s in val_seqs:
        pd, pp = DURS, PITCHES
        o, a = out["all"], out[TYPE_NAMES[s["type"]]]
        for d, p, room in zip(s["d"], s["p"], s["room"]):
            wd = cd[pd].copy()
            wd[room:] = 0
            wp = cp[pp].copy()
            if not s["air"]:
                wp[REST] = 0
            ld, lp = -math.log(wd[d] / wd.sum()), -math.log(wp[p] / wp.sum())
            for q in (o, a):
                q[0] += ld
                q[1] += lp
                q[2] += 1
            pd, pp = d, p
    return {k: {"length": v[0] / v[2], "pitch": v[1] / v[2], "notes": v[2]} for k, v in out.items()}


def split(tunes):
    return [t for t in tunes if not t["val"]], [t for t in tunes if t["val"]]


def train(args):
    torch = torch_setup()
    tr, va = split(load_parts(args.work))
    trs, vas = sequences(tr), sequences(va)
    print(f"{len(tr)} tunes ({len(trs)} parts, {sum(len(s['d']) for s in trs)} notes) to train, {len(va)} ({sum(len(s['d']) for s in vas)} notes) held out")
    net, ep = fit(torch, trs, vas, args.hidden, args.layers, args.drop, args.epochs, args.seed, args.lr)
    write_mbt(args.out, net)
    torch.save({"state": net.state_dict(), "hid": net.hid, "layers": net.layers, "drop": args.drop}, Path(args.work) / "tunes_celtic.pt")
    rep = {
        "hidden": net.hid,
        "layers": net.layers,
        "dropout": args.drop,
        "lr": args.lr,
        "epoch": ep,
        "bytes": Path(args.out).stat().st_size,
        "network": evaluate(torch, net, vas, by_type=True) | evaluate(torch, net, vas),
        "bigram": bigram(trs, vas),
        "train_tunes": len(tr),
        "held_out_tunes": len(va),
    }
    (Path(args.work) / "train.json").write_text(json.dumps(rep, indent=1))
    print(json.dumps(rep, indent=1))


def sweep(args):
    torch = torch_setup()
    tr, va = split(load_parts(args.work))
    trs, vas = sequences(tr), sequences(va)
    res = []
    for hid, layers, drop in [(128, 1, 0.2), (192, 1, 0.3), (128, 2, 0.25), (144, 2, 0.3)]:
        net, ep = fit(torch, trs, vas, hid, layers, drop, args.epochs, args.seed)
        v = evaluate(torch, net, vas)["all"]
        res.append({"hidden": hid, "layers": layers, "dropout": drop, "epoch": ep, "bytes": mbt_size(hid, layers), **v})
        print(res[-1], flush=True)
        (Path(args.work) / "sweep.json").write_text(json.dumps(res, indent=1))


# ---------------------------------------------------------------------------
# tunes.rs's arithmetic in numpy: reading MBT1, a part as `part` writes it


def rs_exp(x):
    x = np.clip(np.asarray(x, np.float32), np.float32(-87.0), np.float32(88.0))
    k = np.floor(x * np.float32(1.4426950408889634) + np.float32(0.5)).astype(np.float32)
    r = x - k * np.float32(0.69314575) - k * np.float32(1.4286068e-6)
    p = np.float32(1.0) + r * (1 + r * (0.5 + r * (np.float32(1 / 6) + r * (np.float32(1 / 24) + r * (np.float32(1 / 120) + r * np.float32(1 / 720))))))
    return np.ldexp(p.astype(np.float32), k.astype(np.int32)).astype(np.float32)


def rs_sigmoid(x):
    return np.float32(1.0) / (np.float32(1.0) + rs_exp(-x))


class NpNet:
    """An MBT1 file read as tunes.rs's `TuneNet::parse` reads it."""

    def __init__(self, path):
        b = Path(path).read_bytes()
        assert b[:4] == b"MBT1"
        feats, hid, n = struct.unpack("<3I", b[4:16])
        assert feats == FEATS
        at = 16

        def f(shape):
            nonlocal at
            cnt = int(np.prod(shape))
            a = np.frombuffer(b, "<f4", cnt, at).reshape(shape).astype(np.float32)
            at += 4 * cnt
            return a

        self.hid, self.layers = hid, []
        for l in range(n):
            inp = feats if l == 0 else hid
            self.layers.append((f((3 * hid, inp)), f((3 * hid, hid)), f((3 * hid,)), f((3 * hid,))))
        self.wd, self.bd = f((DURS, hid)), f((DURS,))
        self.wp, self.bp = f((PITCHES, hid + DURS)), f((PITCHES,))
        assert at == len(b), "trailing bytes"

    def step(self, x, h):
        n = self.hid
        for l, (wi, wh, bi, bh) in enumerate(self.layers):
            a = wi @ x + bi
            c = wh @ h[l] + bh
            r = rs_sigmoid(a[:n] + c[:n])
            z = rs_sigmoid(a[n : 2 * n] + c[n : 2 * n])
            cand = np.float32(2.0) * rs_sigmoid(np.float32(2.0) * (a[2 * n :] + r * c[2 * n :])) - np.float32(1.0)
            h[l] = ((np.float32(1.0) - z) * cand + z * h[l]).astype(np.float32)
            x = h[l]
        return h[-1]

    def dur_logits(self, top):
        return self.wd @ top + self.bd

    def pitch_logits(self, top, d):
        oh = np.zeros(DURS, np.float32)
        oh[d] = 1.0
        return self.wp @ np.concatenate([top, oh]) + self.bp

    def part(self, ty, bar, mode, part, chords, lo, hi, heat, u):
        h = [np.zeros(self.hid, np.float32) for _ in self.layers]
        at, end, prev, out = 0, 8 * bar, None, []
        while at < end:
            b, s = divmod(at, bar)
            root, minor = chords[min(b * 2 + (2 * s) // bar, max(len(chords), 1) - 1)] if chords else (0, False)
            top = self.step(features(ty, mode, part, b, s, root, minor, prev), h)
            room = bar - s
            d = pick(self.dur_logits(top), heat, u, lambda k: k < room)
            pl = self.pitch_logits(top, d)
            p = pick(pl, heat, u, lambda k: (ty == T_AIR) if k == REST else lo <= LOW + k <= hi)
            out.append((at, d + 1, None if p == REST else LOW + p))
            prev = (p, d)
            at += d + 1
        return out


def pick(logits, heat, u, ok):
    """tunes.rs's `pick`."""
    oks = [ok(k) for k in range(len(logits))]
    most = max((float(logits[k]) for k in range(len(logits)) if oks[k]), default=-3.4e38)
    w = [float(rs_exp((logits[k] - most) / max(heat, 0.05))) if oks[k] else 0.0 for k in range(len(logits))]
    tot = sum(w)
    if not tot > 0:
        return next((k for k in range(len(logits)) if oks[k]), 0)
    x = u() * tot
    for k, wk in enumerate(w):
        if wk > 0 and x < wk:
            return k
        x -= wk
    return next(k for k in reversed(range(len(logits))) if w[k] > 0)


# ---------------------------------------------------------------------------
# check: samples, statistics, the reference logits


def part_stats(parts, ty):
    """Statistics of parts [(notes, chords)] of one type."""
    bar, beat = BAR[ty], BEAT[ty]
    iv, cells = collections.Counter(), collections.Counter()
    ranges, ends, down, downn, notes_n = [], 0, 0, 0, 0
    for notes, chords in parts:
        ps = [v for s, d, v in notes if v is not None]
        notes_n += len(notes)
        for a, b in zip(ps, ps[1:]):
            iv[max(-12, min(12, b - a))] += 1
        ranges.append(max(ps) - min(ps))
        ends += ps[-1] % 12 == 0
        for s, d, v in notes:
            if s % bar == 0 and v is not None:
                downn += 1
                down += v % 12 in chord_pcs(*chords[(s // bar) * 2])
        for b0 in range(0, 8 * bar, beat):
            cell = tuple((s - b0, min(d, b0 + beat - s)) for s, d, v in notes if b0 <= s < b0 + beat)
            if not cell or cell[0][0] != 0:
                cell = (("held",),) + cell
            cells[cell] += 1
    n_iv = sum(iv.values()) or 1
    n_cells = sum(cells.values()) or 1
    a = sorted(abs(k) for k in iv.elements())
    return {
        "parts": len(parts),
        "notes_per_bar": round(notes_n / (8 * len(parts)), 2),
        "intervals": {str(k): round(iv[k] / n_iv, 3) for k in range(-12, 13) if iv[k]},
        "repeat": round(iv[0] / n_iv, 3),
        "step_1_2": round(sum(iv[k] for k in (-2, -1, 1, 2)) / n_iv, 3),
        "skip_3_4": round(sum(iv[k] for k in (-4, -3, 3, 4)) / n_iv, 3),
        "leap_5_up": round(sum(v for k, v in iv.items() if abs(k) >= 5) / n_iv, 3),
        "mean_abs_interval": round(sum(a) / max(1, len(a)), 2),
        "range_mean": round(float(np.mean(ranges)), 2),
        "range_median": float(np.median(ranges)),
        "ends_on_tonic": round(ends / len(parts), 3),
        "downbeat_chord_tone": round(down / max(1, downn), 3),
        "rhythm_cells": {" ".join("held" if c == ("held",) else f"{c[0]}+{c[1]}" for c in k): round(v / n_cells, 3) for k, v in cells.most_common(8)},
    }


def interval_distance(a, b):
    ks = set(a) | set(b)
    return round(0.5 * sum(abs(a.get(k, 0) - b.get(k, 0)) for k in ks), 3)


def check(args):
    torch = torch_setup()
    work = Path(args.work)
    tr, va = split(load_parts(work))
    net = NpNet(args.model)
    ck = torch.load(work / "tunes_celtic.pt")
    tnet = make_net(torch, ck["hid"], ck["layers"], ck["drop"])
    tnet.load_state_dict(ck["state"])
    tnet.eval()
    # the port against torch on held-out parts
    worst = 0.0
    for s in sequences(va)[:40]:
        with torch.no_grad():
            x, d = torch.from_numpy(s["x"][None]), torch.from_numpy(s["d"][None])
            dl, pl = tnet(x, d)
        h = [np.zeros(net.hid, np.float32) for _ in net.layers]
        for t in range(len(s["d"])):
            top = net.step(s["x"][t], h)
            worst = max(worst, float(np.abs(net.dur_logits(top) - dl[0, t].numpy()).max()))
            worst = max(worst, float(np.abs(net.pitch_logits(top, s["d"][t]) - pl[0, t].numpy()).max()))
    print(f"numpy port against torch: largest logit difference {worst:.2e}")
    rng = random.Random(args.seed + 7)
    u = rng.random
    # statistics: the held-out tunes against the network's parts on the rules' chords
    stats = {"port_max_logit_diff": worst, "heat": 1.0}
    for ty in range(TYPES):
        held = [t for t in va if t["type"] == ty]
        if not held:
            continue
        real = {p: [(t["parts"][p], t["chords"][p]) for t in held] for p in range(2)}
        gen = {0: [], 1: []}
        reps = max(4, 200 // len(held))
        for t in held:
            for _ in range(reps):
                for p in range(2):
                    ch = halves(rng.choice(RULES[t["mode"]][p]))
                    notes = net.part(ty, BAR[ty], t["mode"], p, ch, *RANGE[p], 1.0, u)
                    gen[p].append((notes, ch))
        st = {}
        for p in range(2):
            a, b = part_stats(real[p], ty), part_stats(gen[p], ty)
            st["AB"[p]] = {"held_out": a, "network": b, "interval_distance": interval_distance(a["intervals"], b["intervals"])}
        stats[TYPE_NAMES[ty]] = st
    (work / "stats.json").write_text(json.dumps(stats, indent=1))
    # samples, for the eye: each type, A and B, ionian and a modal mode
    out, x = [], 1
    for ty in range(TYPES):
        modes = [0] + [m for m in (1, 2, 3) if any(t["type"] == ty and t["mode"] == m for t in tr)][:1]
        for mode in modes:
            for p in range(2):
                for k in range(2 if mode == 0 else 1):
                    plan = rng.choice(RULES[mode][p])
                    ch = halves(plan)
                    notes = net.part(ty, BAR[ty], mode, p, ch, *RANGE[p], 1.0, u)
                    title = f"{TYPE_NAMES[ty]}, {'AB'[p]} part, {MODE_NAMES[mode]} ({k + 1})"
                    out.append(to_abc(x, title, notes, ty, mode, ch, comment=f"chords {plan}; heat 1.0"))
                    x += 1
    (work / "samples.abc").write_text("% musicbox's tune network (tools/neural/tunes.py), sampled with tunes.rs's masks\n\n" + "\n".join(out))
    # the reference: three consecutive steps of a held-out reel's A part, from a zero state
    t = sorted((t for t in va if t["type"] == T_REEL), key=lambda t: t["x"])[0]
    st = steps(t["type"], t["mode"], 0, t["parts"][0], t["chords"][0])[:3]
    x = torch.from_numpy(np.stack([features(*a) for a, _, _, _ in st])[None])
    d = torch.tensor([[di for _, di, _, _ in st]])
    with torch.no_grad():
        dl, pl = tnet(x, d)
    blob = Path(args.model).read_bytes()
    ref = {
        "about": "Three consecutive steps of tunes.rs's `part` from a zero hidden state (the steps of O'Neill's X: below, its A part, teacher-forced): for each, the arguments to `features`, the length logits, the length index fed to the pitch head (0-based, the note's own) and the pitch logits, as PyTorch gives them in float32.",
        "model": Path(args.model).name,
        "model_bytes": len(blob),
        "model_sha256": hashlib.sha256(blob).hexdigest(),
        "hidden": ck["hid"],
        "layers": ck["layers"],
        "x": t["x"],
        "steps": [],
    }
    for k, (a, di, pi, room) in enumerate(st):
        ty, mode, part, b, s, root, minor, prev = a
        ref["steps"].append(
            {
                "features": {"ty": ty, "mode": mode, "part": part, "bar": b, "step": s, "root": root, "minor": minor, "prev": list(prev) if prev else None},
                "nonzero_features": [int(i) for i in np.nonzero(features(*a))[0]],
                "length_logits": [float(v) for v in dl[0, k]],
                "length_index": di,
                "pitch_logits": [float(v) for v in pl[0, k]],
                "pitch_index": pi,
            }
        )
    (work / "check.json").write_text(json.dumps(ref, indent=1))
    print(json.dumps({k: v for k, v in stats.items()}, indent=1)[:20000])


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=["prep", "train", "sweep", "check", "all"])
    ap.add_argument("--abc", default=str(ABC_DIR))
    ap.add_argument("--work", default=str(WORK))
    ap.add_argument("--layers", type=int, default=2)
    ap.add_argument("--hidden", type=int, default=128)
    ap.add_argument("--drop", type=float, default=0.25)
    ap.add_argument("--lr", type=float, default=3e-3)
    ap.add_argument("--epochs", type=int, default=200)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", default=str(REPO / "models/tunes_celtic.mbt"))
    ap.add_argument("--model", default=str(REPO / "models/tunes_celtic.mbt"))
    args = ap.parse_args()
    if args.cmd in ("prep", "all"):
        prep(args)
    if args.cmd in ("train", "all"):
        train(args)
    if args.cmd == "sweep":
        sweep(args)
    if args.cmd in ("check", "all"):
        check(args)


if __name__ == "__main__":
    main()
