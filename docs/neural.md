# How it is made: tuning, tunes and instruments

As it comes, musicbox's music is written by rules (each genre's scales, chords, rhythms and forms, chosen among at random) and played by recordings of real instruments, or by a small synthesiser without them. Three settings each swap one part of that for something learned from real music or tuned against it:

| setting | as it comes | the other way | key | flag |
| --- | --- | --- | --- | --- |
| tuning | as written | optimised: each genre's mix as an optimiser set it, judged by taggers trained on real recordings | O | `--tuned` |
| tunes | rules | network: celtic's tunes written by a network that learned 1,061 of O'Neill's | M | `--net-tunes` |
| instruments | recorded (or synth) | network: 18 instruments played by networks that learned their recordings | I | `--voices synth\|recorded\|network` |

They are in every interface: the keys in the terminal and the window, the window's and the page's row of buttons, and the flags for playing, `--wav` and `--export` (an exported MP3's tag gives the flags that make it again). Changing one plays the song playing again from its start, made the new way, so the two can be heard one after the other.

With all three as they come, musicbox makes the same bytes as before they existed: 14 genres and mixes, with recordings and without, compared byte for byte against the build before them.

The networks are small and run on the CPU in musicbox's own code (no libraries, no GPU), on the desktop and in the browser. They were trained on a CPU with PyTorch (`tools/neural/`); the files are in `models/`, listed in `models/index.tsv`. `MUSICBOX_MODELS=DIR` reads them from another directory.

## Tuning: optimised

Six of the twelve genres have a setting: ambient, techno, lo-fi, ska, indonesian and starconflict. The other six play as written either way, because none of the settings found for them held up (below). A mix takes the setting of the genre that writes it, its first: ska+celtic is tuned, celtic+ska is not.

The mix as written has, for each genre, the levels, stereo width, tone and feel its rules give it. The optimiser (`tools/tune/tune.py`) turns 20 knobs on top of those (`src/music/tuned.rs`): each of the 13 channels' level (±6 dB), the master's low and high shelves (±5 dB), the reverb's send (0.5 to 1.8 times), the tempo (0.88 to 1.12 times), the swing (±0.12 of a 16th), how late the hits come (0.4 to 2.5 times) and the stereo spread (0.4 to 1.8 times). The notes are never changed.

It searches with CMA-ES. Each setting is scored on six training songs of 1:30 (seeds 201 to 206, the same songs for every setting) by the judges of [the validation](validation.md): Essentia's Discogs-EffNet embedding (how close the songs lie to the genre's reference recordings) and its style tagger (how much it names the genre, capped at what it names the real recordings). A small penalty keeps each knob near where it rests.

    mean cosine to the references' centroid
    + 0.25 × min(the genre's style share, the references' share)
    − 0.05 × Σ (knob − rest)²         (knobs scaled to 0..1)

The best setting is then checked on eight held-out whole songs (seeds 101 to 108) with every measure the validation uses. It is kept only if, on those songs, the mean cosine rises by at least 0.005 with at least 6 of the 8 songs closer, and nothing else gets much worse: the Jamendo tagger's verdict on the genre (by 15%), how many songs lie nearest their own genre (by two), and the loudness (by 1.5 LU), spectral centroid and energy above 2 kHz (by 25%) against the references' medians.

The first pass searched every knob's full range. A second searched the levels within ±4 dB and the shelves within ±2.5 dB, for the genres where the first had pushed a knob to its limit or failed a judge. Where both were kept, the setting with no knob at a limit wins, then the closer one.

**On the held-out songs**, as written → optimised, with the references' figure in brackets (* the references are stand-ins or few: [the validation](validation.md) says which):

| genre | cosine to the references | songs closer | loudness, LUFS | energy above 2 kHz | kept |
|---|---|---|---|---|---|
| ambient | 0.799 → 0.820 | 8 of 8 | −13.4 → −12.9 (−15.4) | 0.8% → 0.8% (0.8%) | yes |
| techno | 0.792 → 0.865 | 8 of 8 | −13.7 → −13.7 (−10.5) | 2.5% → 2.1% (2.6%) | yes, second pass |
| synthwave | 0.778 → 0.794 | 8 of 8 | −13.4 → −12.2 (−11.4) | 3.7% → 2.9% (4.8%) | no: darker |
| lo-fi | 0.731 → 0.770 | 8 of 8 | −13.7 → −14.1 (−12.1) | 0.6% → 0.9% (2.4%) | yes |
| polka | 0.491 → 0.559 | 8 of 8 | −12.0 → −9.6 (−17.7) | 18.5% → 37.5% (12.8%) | no: brighter, louder |
| tavern | 0.656 → 0.701 | 7 of 8 | −11.7 → −10.5 (−14.3) | 9.9% → 21.7% (3.4%) | no: brighter |
| ska | 0.620 → 0.658 | 7 of 8 | −13.0 → −11.7 (−11.0) | 9.4% → 7.7% (8.9%) | yes |
| mariachi * | 0.469 → 0.507 | 8 of 8 | −11.2 → −9.1 (−13.9) | 10.9% → 25.4% (3.3%) | no: brighter, louder |
| indonesian | 0.756 → 0.793 | 8 of 8 | −11.3 → −11.0 (−15.9) | 9.6% → 9.1% (5.4%) | yes, second pass |
| starconflict * | 0.738 → 0.790 | 8 of 8 | −13.2 → −13.5 (−15.8) | 8.1% → 6.5% (5.2%) | yes |
| celtic | 0.634 → 0.710 | 8 of 8 | −11.2 → −9.7 (−15.8) | 9.0% → 22.5% (10.6%) | no: brighter, louder |
| western * | 0.778 → 0.778 | 0 of 8 | −14.2 → −14.2 (−14.1) | 5.4% → 5.4% (3.9%) | no: nothing better |

Where both passes failed, the table gives the first. [`tools/tune/RESULTS.md`](../tools/tune/RESULTS.md) has both passes of every genre with all the judges (the style share, the nearest genre, the Jamendo and YAMNet taggers, the centroid) and what each search moved.

**What the six settings do**, mostly (levels in dB):
- **ambient:** arpeggio −4.1, hats −3.6, effects +2.7, bells −2.6, drone −2.4, pad and kick +2.2.
- **techno:** cymbals +4.0, hats +3.9, snare +2.9, kick −2.5, bass +2.4, low shelf +2.3, high shelf −2.2.
- **lo-fi:** keys (the electric piano) −5.9, hats +3.2, effects +2.2, bells −2.0, 4% slower. This one most needs a listen: the piano nearly goes, and the band from 250 Hz to 2 kHz drops from 14% of the energy to 7% (the references have 17%), yet it passed every rule.
- **ska:** drone +6 (in ska, the piano or the distorted guitar), bass +3.9, keys −3.0, bells +2.8, 9% faster (136 → 150 bpm, inside the references' upper cluster of 129 to 172), hits half as late, stereo narrower.
- **indonesian:** pad +4.0, keys −3.2, drone +2.8, cymbals +2.6, high shelf +2.5, and a little swing (0.11 of a 16th, which gamelan does not have).
- **starconflict:** pad +6, lead −3.5, bass −2.7, cymbals +2.6, kick −2.4, stereo 1.4 times as wide.

**What it found:** the taggers favour a brighter, more melody-forward mix than the real recordings have. The searches kept making the same move: lead up, the keys (each genre's comping instrument: lo-fi's electric piano, polka's accordion, ska's guitar chop, tavern's lute, mariachi's vihuela) down by about 3 to 6 dB in 8 of the 10 genres that have them (in one pass or both), and the highs up. Lo-fi and synthwave went the other way, darker than their references. In the first pass, 10 of the 11 genres that moved pushed a knob to its limit. Narrowing the ranges only moved the pins to the new limits (9 of 10). So the pins are what the taggers want, not something another pass would fix: five of the six settings kept still have a knob at a limit, all but ambient's. What stopped the five genres that went too bright or too dark was the signal judges (loudness, centroid and energy above 2 kHz, measured directly), which share nothing with the taggers. The Jamendo tagger reads the same embedding the search optimised, so its agreeing proves little.

The cosine gains did hold out of the search: every genre that moved got 7 or 8 of its 8 held-out songs closer, keeping from about 40% (tavern) to all of its training gain.

**Knobs that do nothing:** the tempo in celtic and indonesian, whose pieces set their own timing (the bpm shown changes, the audio does not); the swing below zero, which the engine stops at straight, in every genre but lo-fi; how late the hits come in techno and polka, which have no lateness. The optimiser finds these for each genre and leaves them at rest.

**What "optimised" means:** the mix the taggers rate as more like the genre. The taggers are neural networks trained on millions of recordings, but they are not listeners, and they were also the judges of the validation. The held-out songs keep the optimiser from simply fitting six songs, but not from fitting the taggers' taste. The signal judges are a check on that taste, not a listener either.

The search took about 10 hours on the CPU (at most one and a half cores, no GPU): 5 h 52 min for the first pass, 3 h 56 min for the second.

## Tunes: network

Celtic's reels, jigs, hornpipes, marches and airs are written eight bars at a time: an A part and a B part over chords the rules choose. With network tunes, a small network writes those parts instead. The chords, the form, the instruments and everything else in the piece stay as the rules make them, and the network rolls dice of its own, so the rest of the piece comes out the same either way and only the tunes differ.

**The network** (`src/tunes.rs`, `models/tunes_celtic.mbt`, 0.8 MB): a GRU of two layers of 128. At each note it is given the tune type, the mode, the chord under it, where it is in the bar and the part, and the notes so far. It gives the next note's length, then its pitch, sampled under masks: the length must fit the bar, the pitch must be in the part's range, and rests are allowed only in airs.

**What it learned from:** 1,061 tunes from *O'Neill's Music of Ireland* (Chicago, 1903), as the volunteers of the O'Neill's Project transcribed it into ABC (GPL-2.0-or-later, with credit to the transcribers). Validated on a held-out tenth: 0.30 nats a note for the length and 1.12 for the pitch, against 0.57 and 1.91 for a bigram on the note before. [`tools/neural/TUNES.md`](../tools/neural/TUNES.md) has the credits, the licence, which tunes, and how the ABC became training parts. No data from The Session was used: its licence forbids processing it with language models or their tools.

**Against the rules,** on the same statistics, compared with the held-out tenth of O'Neill's tunes (the network's parts written over those tunes' chords, the rules' over chords of their own, 170 tunes of each type):

| type | source | steps | skips | leaps | repeats | range | ends on tonic | downbeat chord tone | interval distance | rhythm distance |
|---|---|---|---|---|---|---|---|---|---|---|
| reel A | O'Neill's | 0.50 | 0.31 | 0.13 | 0.06 | 15.6 | 0.88 | 0.98 | – | – |
| reel A | network | 0.47 | 0.32 | 0.12 | 0.10 | 13.3 | 0.66 | 0.93 | 0.06 | 0.03 |
| reel A | rules | 0.45 | 0.13 | 0.08 | 0.34 | 13.7 | 0.64 | 1.00 | 0.29 | 0.23 |
| reel B | O'Neill's | 0.50 | 0.29 | 0.15 | 0.05 | 15.6 | 0.56 | 0.95 | – | – |
| reel B | network | 0.50 | 0.32 | 0.10 | 0.09 | 13.4 | 0.43 | 0.90 | 0.08 | 0.12 |
| reel B | rules | 0.43 | 0.13 | 0.07 | 0.36 | 13.9 | 0.59 | 1.00 | 0.31 | 0.27 |
| jig A | O'Neill's | 0.43 | 0.30 | 0.13 | 0.14 | 14.1 | 0.92 | 0.97 | – | – |
| jig A | network | 0.42 | 0.30 | 0.12 | 0.17 | 12.8 | 0.95 | 0.92 | 0.06 | 0.03 |
| jig A | rules | 0.43 | 0.16 | 0.10 | 0.31 | 13.3 | 0.61 | 1.00 | 0.20 | 0.21 |
| jig B | O'Neill's | 0.49 | 0.31 | 0.10 | 0.10 | 14.6 | 0.90 | 0.98 | – | – |
| jig B | network | 0.45 | 0.31 | 0.10 | 0.14 | 13.1 | 0.72 | 0.91 | 0.09 | 0.04 |
| jig B | rules | 0.40 | 0.15 | 0.10 | 0.35 | 13.6 | 0.71 | 1.00 | 0.27 | 0.24 |
| hornpipe A | O'Neill's | 0.53 | 0.29 | 0.13 | 0.04 | 17.7 | 0.95 | 0.98 | – | – |
| hornpipe A | network | 0.48 | 0.32 | 0.11 | 0.09 | 13.6 | 0.87 | 0.92 | 0.09 | 0.06 |
| hornpipe A | rules | 0.47 | 0.13 | 0.07 | 0.33 | 13.8 | 0.51 | 1.00 | 0.29 | 0.25 |
| hornpipe B | O'Neill's | 0.61 | 0.21 | 0.12 | 0.06 | 18.1 | 0.86 | 0.97 | – | – |
| hornpipe B | network | 0.53 | 0.31 | 0.08 | 0.08 | 13.4 | 0.66 | 0.89 | 0.13 | 0.11 |
| hornpipe B | rules | 0.45 | 0.13 | 0.06 | 0.36 | 14.0 | 0.54 | 1.00 | 0.31 | 0.26 |
| air A | O'Neill's | 0.64 | 0.23 | 0.05 | 0.09 | 13.6 | 0.50 | 0.96 | – | – |
| air A | network | 0.47 | 0.26 | 0.14 | 0.13 | 12.4 | 0.82 | 0.89 | 0.18 | 0.10 |
| air A | rules | 0.44 | 0.10 | 0.05 | 0.41 | 10.4 | 0.59 | 1.00 | 0.34 | 0.13 |
| air B | O'Neill's | 0.64 | 0.21 | 0.04 | 0.11 | 15.8 | 1.00 | 0.99 | – | – |
| air B | network | 0.53 | 0.25 | 0.10 | 0.11 | 12.4 | 0.64 | 0.85 | 0.17 | 0.05 |
| air B | rules | 0.43 | 0.10 | 0.05 | 0.42 | 10.7 | 0.59 | 1.00 | 0.35 | 0.13 |

Steps, skips, leaps and repeats are the shares of the intervals between notes (a second; a third or fourth; wider; the same note again). Range is in semitones. The two distances are summed absolute differences from O'Neill's distributions of intervals and of note lengths by place in the bar: lower is closer. Summed over the part types, the network's interval distance is 0.85 and the rules' 2.38; for rhythm, 0.55 and 1.72.

The rules repeat the same note about a third of the time, where real tunes do so 5 to 14% of the time, and they leap by a third or fourth half as often. The network gets both about right. Where it falls short: it puts a chord tone on the downbeat a little less often than O'Neill (89 to 93% against 95 to 98%; the rules always do), it spans a smaller range, and it ends a part on the tonic less reliably.

**The same tune everywhere:** the network's arithmetic is written out in tunes.rs, with an `exp` of its own, so a seed writes the same tune on the desktop and in the browser. A test checks its logits against PyTorch's to 1e-4 (`models/tunes_celtic.check`).

Only celtic has a tunes network. Other genres write their tunes by their rules either way. `tools/neural/tunes.py` makes one, given a corpus whose licence allows it.

## Instruments: network

A sampler plays an instrument by replaying its recordings, a few seconds of each note at a few velocities. A network instrument plays it from a description instead, in the manner of DDSP (Engel et al., 2020). Each 192 samples (250 times a second), a small network is given the note's pitch, its velocity and the time since it began. It answers with how loud the note is, how that loudness is shared among 48 harmonics, how loud the noise (breath, bow, buzz) is in each of 16 bands from 80 Hz to 16 kHz, and, for 11 of the 18 instruments, the vibrato's depth and rate. Oscillators and filtered noise turn that into sound (`src/net.rs`).

Each network is a dense one of four layers of 128 (58,000 parameters, 234 KB, against the megabytes of its recordings), and plays every pitch and velocity between the recorded ones. They render about 100 times faster than real time on one core: a whole ska song of 2:09 took 1.3 s with network instruments against 0.8 s with recordings alone.

**The 18:** violin, violins, violas, cellos and contrabass; trumpet (held, and with vibrato), trombone, horn and tuba; tenor saxophone, flute, clarinet, oboe and bassoon; recorder, ocarina and harmonica. They are held-note articulations of instruments the genres play, from VS Chamber Orchestra 2: Community Edition and the Versilian Community Sample Library (both CC0). Everything else, including short notes, plucked and struck instruments and drums, plays from its recordings, so network instruments need the recordings installed.

**How they learned** (`tools/neural/voices.py`): musicbox plays each instrument through its sampler (`--voice-data`) at every note of its range (from a fifth below its lowest recording to a fifth above its highest) and at four velocities, each note held for 3 s. Each note is then analysed on net.rs's frame grid (its pitch, each harmonic's energy, the noise between the harmonics in each band, and the vibrato's depth and rate) and the network is fitted to that. Then it is fine-tuned end to end, through a differentiable copy of net.rs's synthesis, on a multi-resolution spectral distance plus the loudness envelope, and kept only where that does better. An instrument gets vibrato outputs when its recordings' held notes waver by at least 3 cents, mostly at 3.5 to 8.5 Hz. Each took about 10 minutes of CPU.

**Against their recordings**, played by a copy of net.rs's synthesis:

| instrument | vibrato | level error, dB (baseline) | envelope error, dB (baseline) | MR-LSD: network / vibrato off / analysis / baseline | vibrato depth, cents: network / recordings | rate, Hz: network / recordings |
|---|---|---|---|---|---|---|
| trumpet vib | yes | 0.13 (0.86) | 0.98 (2.22) | 6.04 / 5.98 / 6.67 / 8.13 | 10.6 / 10.2 | 5.40 / 5.55 |
| trombone vib | yes | 0.14 (0.82) | 0.81 (3.33) | 5.77 / 5.76 / 5.88 / 7.75 | 7.0 / 6.8 | 5.05 / 5.06 |
| tenor sax vib | yes | 0.15 (0.54) | 0.77 (2.11) | 6.30 / 6.20 / 6.77 / 7.36 | 12.1 / 12.5 | 5.00 / 4.94 |
| flute vib | yes | 0.26 (0.53) | 1.10 (2.33) | 6.09 / 6.06 / 5.98 / 6.75 | 6.7 / 6.8 | 5.09 / 5.06 |
| violins sus | yes | 0.36 (1.31) | 1.23 (3.42) | 6.35 / 6.33 / 6.31 / 7.53 | 4.3 / 4.4 | 5.52 / 5.42 |
| contrabass sus | no | 0.26 (0.31) | 0.77 (1.61) | 5.29 / – / 5.60 / 5.90 | – / 2.0 | |
| clarinet sus | no | 0.06 (0.28) | 0.22 (1.75) | 5.16 / – / 5.35 / 5.92 | – / 0.2 | |
| cellos sus | yes | 0.41 (0.97) | 1.20 (2.20) | 6.36 / 6.35 / 6.64 / 7.27 | 4.5 / 4.9 | 5.61 / 5.24 |
| horn sus | no | 0.14 (0.29) | 0.45 (1.24) | 3.85 / – / 5.53 / 5.02 | – / 1.2 | |
| tuba sus | no | 0.23 (0.30) | 0.51 (1.24) | 4.09 / – / 4.40 / 4.70 | – / 1.3 | |
| harmonica vib | no | 0.16 (0.45) | 1.08 (1.70) | 6.31 / – / 6.35 / 7.03 | – / 1.4 | |
| recorder vib | yes | 0.28 (0.53) | 1.04 (1.99) | 5.51 / 5.48 / 5.52 / 6.08 | 4.3 / 4.5 | 5.41 / 5.37 |
| ocarina vib | yes | 0.31 (0.35) | 1.22 (1.63) | 4.68 / 4.65 / 5.38 / 5.52 | 9.7 / 9.7 | 4.96 / 5.06 |
| oboe vib | yes | 0.19 (0.70) | 1.05 (1.95) | 5.56 / 5.60 / 5.53 / 6.14 | 5.4 / 5.5 | 4.83 / 4.86 |
| bassoon sus | no | 0.10 (0.12) | 0.21 (0.66) | 4.72 / – / 5.59 / 5.73 | – / 0.2 | |
| violas sus | yes | 0.40 (1.04) | 1.17 (2.87) | 6.21 / 6.15 / 6.34 / 7.24 | 4.7 / 5.1 | 5.30 / 5.23 |
| violin sus | yes | 0.39 (0.82) | 1.12 (2.09) | 7.06 / 6.84 / 7.27 / 7.57 | 9.6 / 9.8 | 5.53 / 5.12 |
| trumpet sus | no | 0.09 (0.23) | 0.40 (0.96) | 5.51 / – / 6.55 / 7.08 | – / 1.5 | |

- **Level error:** each held note's level against its recording's (the mean of |dB|). **Envelope error:** the 50 ms loudness envelope's, over the frames within 40 dB of the note's peak.
- **MR-LSD:** the multi-resolution log-spectral distance (FFTs of 2048, 1024, 512 and 256 samples; lower is closer). It is given for the network, the network with its vibrato off, the analysis it learned from played back as is (about the best net.rs's synthesis could do from those targets), and the baseline.
- **Baseline:** each note's own average harmonics and noise, held steady for the 3 s with no vibrato. It is what a network that learned nothing about how a note changes would give.

All 18 are closer to their recordings than the baseline on every measure, and 15 are closer in spectrum than their own analysis played back. Flute, violins and oboe are within 0.11 dB of it.

**Where it falls short:**
- **Vibrato phase.** The vibrato starts at phase 0 on every note, while each recording's starts wherever it does. A frame-by-frame measure like MR-LSD counts that against the network, so the networks with vibrato score 0.0 to 0.2 dB worse with it on than off. Violin is the clearest case: 7.06 with vibrato, 6.84 without.
- **Loudness.** A whole song played with network instruments came out 1.2 dB louder (RMS, at the same peak) than with the recordings.
- **Short notes.** The networks learned held notes only. Short notes, and the instruments' other articulations, still play from the recordings.

## Running the training

    # the tunes network, from the O'Neill's ABC in ~/.cache/musicbox/tunes/oneills (see TUNES.md)
    uv run tools/neural/tunes.py all

    # an instrument: its data, then its network
    target/release/musicbox --voice-data ~/.cache/musicbox/neural/data "multi violin sus"
    systemd-run --user --scope -q -p CPUQuota=250% -p MemoryMax=8G nice -n 10 \
      uv run --script tools/neural/voices.py multi_violin_sus --e2e-minutes 6

    # the optimiser: a first pass, a narrower second, then the table and TUNED
    systemd-run --user --scope -q -p CPUQuota=150% -p MemoryMax=8G nice -n 10 \
      uv run tools/tune/tune.py run celtic
    systemd-run --user --scope -q -p CPUQuota=150% -p MemoryMax=8G nice -n 10 \
      uv run tools/tune/tune.py run celtic --tag narrow --range levels=-4:4 --range tilt=-2.5:2.5
    uv run tools/tune/tune.py table
    uv run tools/tune/tune.py write && rustfmt src/music/tuned.rs

All of it runs on the CPU (PyTorch's CPU build, Essentia's TensorFlow on the CPU). Each script's opening comment has its options.
