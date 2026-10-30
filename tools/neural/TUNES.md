# The tune network: credits and method

`models/tunes_celtic.mbt` is a small recurrent network (src/tunes.rs) that
writes the A and B parts of celtic dance tunes, eight bars at a time, over the
chords the celtic genre's rules choose. It learned them from *O'Neill's Music
of Ireland* (Captain Francis O'Neill, Chicago, 1903; 1,850 melodies) as the
volunteers of the O'Neill's Project transcribed it into ABC, 1997-2000.
`tools/neural/tunes.py` turns their ABC into training sequences, trains the
network and writes the file.

## Source

The O'Neill's Project's files, in John Chambers' copy of the collection:
<https://trillian.mit.edu/~jc/music/book/ONeills/_1850/> (the `F/` blocks of
tunes, `History.html`, `Copyright.html`, `GPL.txt`), read from
`~/.cache/musicbox/tunes/oneills/`: 39 ABC blocks holding 2,009
transcriptions of 1,838 X: numbers (some number ranges were transcribed twice,
by different people; one version of each is used).

No data from The Session (thesession.org), or derived from it, was used.

## Licence

The project's notice (`Copyright.html`), in full apart from its links:

> The original O'Neill's Music of Ireland has been out of copyright for many
> years now. This collection of ABC transcriptions is copyrighted 1997-2000 by
> the contributors to the O'Neill's Project. You are welcome to download
> anything here and use it, as long as you give credit to the transcribers,
> and take credit for any corrections or enhancements that you might make.
>
> This information is free; you can redistribute it and/or modify it under
> the terms of the GNU General Public License as published by the Free
> Software Foundation; either version 2 of the License, or (at your option)
> any later version.
>
> This work is distributed in the hope that it will be useful, but WITHOUT ANY
> WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
> FOR A PARTICULAR PURPOSE. In particular, if you find yourself engaging in
> wild dancing or sitting in a quiet corner with a glass of stout and a
> contemplative mood, don't blame us. This is traditional Irish music, you
> know.
>
> You should have received a copy of the GNU General Public License along
> with this work; if not, write to the Free Software Foundation, Inc., 675
> Mass Ave, Cambridge, MA 02139, USA. See the GNU General Public License for
> more details.
>
> If you find any discrepancies between the notes in the ABC here and the
> original O'Neill's, please send in the error reports. We've found a few
> typos in the transcriptions (and in the book), and we expect there are more
> to be found.

The trained weights are a work derived from the transcriptions. musicbox takes
the licence's "any later version" option: `models/tunes_celtic.mbt` is
distributed under the GNU General Public License, version 3 or later
(GPL-3.0-or-later), like the rest of musicbox, with the credit below. Any
corrections or changes are musicbox's (the method below), not the
transcribers'.

## The transcribers

Block by block, as `History.html` records them (with the names as the tunes'
`Z:` lines spell them where they differ). "Used" is how many of the block's
tunes the network learned from; where a range was transcribed twice, the
first of the two in this list that came through cleanly was used.

| Tunes | Type | Transcriber | Used |
|---|---|---|---|
| 0001-0050 | Airs | Norbert Paap | 6 |
| 0051-0100 | Airs, songs | John Chambers | 5 |
| 0101-0200 | Airs | Henrik Norbeck | 19 |
| 0201-0300 | Airs | John Chambers | 18 |
| 0301-0350 | Airs | John Chambers | 11 |
| 0351-0400 | Airs | Chris Falt | 7 |
| 0401-0486 | Airs, songs | Henrik Norbeck | 11 |
| 0487-0500 | Airs, songs | John Chambers | 0 |
| 0501-0550 | Airs, songs | David (Dave) Wooldridge | 8 |
| 0550-0625 | Airs, songs | John (J. B.) Walsh | 9 |
| 0626-0635 | O'Carolan | Ted Hastings | 0 |
| 0626-0700 | O'Carolan | John Chambers | 3 |
| 0701-0720 | Jigs | (A.) Lee Worman | 19 |
| 0721-0730 | Jigs | Marguerite Plank (proofread by Steven Foy) | 9 |
| 0731-0731 | Jig | Henrik Norbeck | 1 |
| 0732-0758 | Jigs | Bob Safranek | 24 |
| 0732-0758 | Jigs | Michael Hogan | 0 |
| 0759-0810 | Jigs | Steven (Stephen) Foy | 47 |
| 0811-0899 | Jigs | Dan Peterson (Dan G. Petersen) | 83 |
| 0900-0950 | Jigs | Tom Keays | 48 |
| 0951-0981 | Jigs | Dan Peterson (Dan G. Petersen) | 24 |
| 0981-1000 | Jigs | RJ (Robert) Thorpe | 16 |
| 1001-1031 | Jigs | Dan Peterson (Dan G. Petersen) | 30 |
| 1031-1115 | Jigs | Henrik Norbeck | 79 |
| 1116-1135 | Slip jigs | Michael Hogan | 0 |
| 1116-1135 | Slip jigs | Michael D. Long | 0 |
| 1136-1175 | Slip jigs | Steven (Stephen) Foy | 0 |
| 1176-1275 | Reels | Trish O'Neil (proofread by Phil Taylor) | 91 |
| 1276-1375 | Reels | Trish O'Neil | 87 |
| 1376-1475 | Reels | Bob Safranek | 92 |
| 1476-1555 | Reels | John (John B.) Walsh | 73 |
| 1556-1576 | Hornpipes | Bill Haneman | 15 |
| 1556-1624 | Hornpipes | Michael Hogan | 43 |
| 1577-1624 | Hornpipes | Michael D. Long | 6 |
| 1625-1700 | Hornpipes | Nick Terhorst | 70 |
| 1701-1780 | Hornpipes | Bob Safranek | 79 |
| 1710-1750 | Hornpipes ("Jigs" in History.html) | (A.) Lee Worman | 2 |
| 1781-1800 | Hornpipes | Robert Thorpe | 0 |
| 1801-1850 | Marches | Bob Safranek | 26 |

Steven Foy proofread 0721-0730 and Phil Taylor 1176-1275 (`History.html`).
John Chambers keeps the collection and corrected a number of tunes; Frank
Nordberg proofread some of Michael Hogan's hornpipes; Henrik Norbeck and Frank
Nordberg ran the whole collection through their ABC tools and reported the
errors; Colin McEwen amended X:517 (2007).

## Which tunes, and how many

By X: number and meter, as O'Neill's arranges them; everything else (slip
jigs, airs and marches in other meters, 2/4 reels...) was left out.

| Type (tunes.rs) | X: | Meter | Kept |
|---|---|---|---|
| T_REEL (reel) | 1176-1555 | C, C\| or 4/4 | 343 (266 ionian, 43 dorian, 7 mixolydian, 27 aeolian) |
| T_JIG (jig) | 701-1115 | 6/8 | 380 (267 ionian, 35 dorian, 29 mixolydian, 49 aeolian) |
| T_HORNPIPE (hornpipe) | 1556-1800 | C, C\|, 4/4; 2/4 doubled | 215 (208 ionian, 1 dorian, 2 mixolydian, 4 aeolian) |
| T_MARCH (march) | 1801-1850 | 2/4 | 6 (5 ionian, 1 aeolian) |
| T_MARCH68 (march68) | 1801-1850 | 6/8 | 20 (16 ionian, 3 mixolydian, 1 aeolian) |
| T_AIR (air) | 1-700 | 3/4 | 97 (66 ionian, 8 dorian, 10 mixolydian, 13 aeolian) |
| all | | | 1061 |

Of the 1274 chosen tunes, 1061 came through and 213 were skipped:

- 109: parts of other lengths (12- or 16-bar strains, airs in 16-bar sections)
- 76: a bar of the wrong length inside a part (most are slips in the transcription: a missing or extra bar line, a dropped note)
- 14: one section only, not 16 bars
- 4: a key change inside the tune
- 3: in phrygian mode
- 2: a meter change inside the tune
- 2: a multi-bar rest
- 2: in lydian mode
- 1: a K: field it could not read

64 tunes took their tonic from their endings and 79 their mode from their notes (above). After placing each part's octave, 8.1% of the notes' length lies outside the runtime's range, and 86% of the parts end on the tonic.

## How the ABC became training parts

tunes.py has its own ABC reader (no music21), for the dialect these files use:

- Grace notes, decorations, slurs, chord symbols and text dropped; a chord
  (`[E3A3]`) is its top note with the first note's length; `Hx` (a fermata's
  placeholder) is nothing.
- Lengths in 16ths. Broken rhythm `a>b` divides the pair 3:1 (3+1 16ths for
  two eighths), `a>>b` 7:1. A tuplet's notes share its time on the 16th grid,
  the longer ones first: a triplet of eighths `(3abc` is 2,1,1, of quarters
  3,3,2; a triplet of 16ths, two 16ths for three notes, keeps its first and
  last note. Anything else off the grid (a few 32nds) is rounded by its onset,
  and a note rounded to nothing dropped.
- Hornpipes in 2/4 (O'Neill's prints many in half values, in 16ths) are
  doubled to 4/4.
- A tie joins two notes of one pitch within a bar; across a bar line they stay
  two notes (the runtime's notes never cross one). In the dance tunes a rest
  lengthens the note before it; the airs keep theirs.
- Sections end at `||`, `|]`, `:|`, `|:` and `::`. With `|1 |2` endings the
  first ending is kept and the second dropped. A section's pickup (anacrusis)
  is dropped, so its bar 0 is the first full bar; its last bar, short by a
  pickup (or by half a bar at most), has its last note lengthened to fill it.
  A four-bar strain with a repeat is played through (first ending, then the
  second) to make its eight bars. The first two sections must then be eight
  whole bars each: the A and B parts (a tune written as one 16-bar section is
  cut in two). Later parts (C, D...) are not used.
- Notes over 12 16ths are 12, the rest of their length a second note of the
  same pitch.
- Pitches are semitones above the tonic. The tonic and mode come from `K:`
  (major as ionian, minor as aeolian, dorian, mixolydian; phrygian and lydian
  tunes skipped), with two corrections, because `K:` holds the key signature
  as O'Neill printed it (often a major key's for a modal tune): when both
  parts end on another note that is a tonic of one of the four modes in that
  signature, it is the tonic (X:703, `K:G` ending on A, is A dorian); and the
  mode's third, sixth and seventh are the ones the notes use most (a `K:Em`
  tune full of C sharps is E dorian). Each part is then moved by octaves to
  put most of its length in the runtime's range (A -3..12, B -1..15).
- Chords, two a bar, inferred from each half bar's notes, weighted by length
  and metric strength (a half bar's first 16th most, then the beats, then the
  eighths), against the mode's triads (not the diminished; in mixolydian also
  the major V the rules use): a chord tone scores 1, its root a quarter more,
  the tonic chord and the chords the rules use are a little favoured. The
  result is (root above the tonic, minor or not), as the rules feed it.

A tenth of the tunes of each type (by tune, a fixed seed) were held out to
validate.

## The network

A GRU of 2 layers, 128 hidden, dropout 0.25 in training (778,336 bytes), trained with teacher forcing on 954 tunes and validated on 107; Adam, the best epoch by validation kept.

Validation negative log-likelihood per note (nats), with the runtime's masks (a length within the bar, a rest only in an air):

| | length | pitch |
|---|---|---|
| network | 0.304 | 1.116 |
| bigram on the note before | 0.567 | 1.907 |

By type (the march figures rest on one or two held-out tunes):

| type | notes | network length | network pitch | bigram length | bigram pitch |
|---|---|---|---|---|---|
| reel | 4127 | 0.240 | 1.075 | 0.435 | 1.891 |
| jig | 3267 | 0.290 | 1.111 | 0.492 | 1.999 |
| hornpipe | 2572 | 0.272 | 1.153 | 0.610 | 1.824 |
| march | 79 | 0.733 | 1.383 | 1.176 | 2.148 |
| march68 | 133 | 0.593 | 0.984 | 1.038 | 1.815 |
| air | 645 | 0.813 | 1.251 | 1.450 | 1.867 |
| all | 10823 | 0.304 | 1.116 | 0.567 | 1.907 |

The sizes tried (validation length + pitch, dropout 0.2-0.3): 1 layer of 128, 1.454; 1 of 192, 1.438; 2 of 128, 1.421; 2 of 144, 1.424 (958 KB); more dropout (0.4, 0.5) did worse. `tunes.py check` compares the network's parts, sampled as tunes.rs samples them over the rules' chords, with the held-out tunes (`stats.json`), writes sample parts as ABC (`samples.abc`) and reference logits for tunes.rs's tests (`check.json`), all in `~/.cache/musicbox/tunes/`.

