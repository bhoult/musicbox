# Tuning each genre's mix against real recordings: results

Run 2026-10-30/31 with `tools/tune/tune.py` (see its header for how to run it). One
binary throughout (`target/release/musicbox` of 2026-10-30 16:19, sha256 a25dada3…),
under `CPUQuota=150%`, TensorFlow on one thread, no GPU.

## How

- **Search**: CMA-ES (`cma` 4.5, population 10, σ₀ 0.2 in knob space normalised to
  [0, 1], starting at the rest values) over each genre's *live* knobs, at most 150
  evaluations, stopping when the best score gained less than 0.002 in four
  generations. A knob, or one side of it, is dead when its limit leaves all six
  training songs byte-identical; dead ones stay at rest.
- **Score** on training seeds 201–206 (one 1:30 song each, the same six every time):
  mean cosine of the songs' Discogs-EffNet embeddings to the genre's reference
  centroid + 0.25 × min(mean target-style share, the references' own mean share)
  − 0.05 × Σ(x − rest)². The share is capped at the references' share: past it, the
  tagger naming the genre more is not more like the recordings (polka's 78s, ambient,
  western and mariachi already out-score their references). λ = 0.05: a single knob at
  a limit costs 0.0125 (the pilot used 0.02 and over-reached).
- **Held out**, nothing optimised on it: seeds 101–108, whole songs, as written against
  the chosen setting *rounded to 2 decimals* (what TUNED holds). Judges: cosine to the
  centroid, style share and nearest-genre share (as `tools/validate/report.py`), the
  MTG-Jamendo genre head on the nearest Jamendo genres, YAMNet on the nearest AudioSet
  classes, and LUFS, spectral centroid and energy above 2 kHz against the references'
  medians.
- **Kept** only if the mean held-out cosine rose by at least 0.005 *and* at least 6 of
  the 8 songs moved closer, the Jamendo verdict did not fall by more than 15 % (and
  0.01), no more than one song fewer was nearest its own genre, and none of the three
  signal features moved clearly further from the references (LUFS by more than 1.5 LU;
  centroid or energy above 2 kHz by more than 25 % in ratio).
- **Caveats on the judges**: the Jamendo head reads the same EffNet embedding the search
  optimised, so it agreeing is weak corroboration. YAMNet is a different network
  (reported, not used to decide). The signal features are the only judge wholly apart
  from the search. Starconflict and western have stand-in references (no open
  originals), and mariachi has six, four of them from 1908: their results are weakly
  grounded (marked *).
- **Two passes.** The first searched the full KNOBS ranges. Where it pinned a knob at a
  limit, or failed a signal judge, a second pass ("narrow") searched levels within
  ±4 dB and both tilts within ±2.5 dB, all else equal. When both passes of a genre were
  kept, TUNED takes the one with no knob at a limit, then the closer on held-out cosine.

## Results

Held out (seeds 101–108, whole songs): as written → tuned; references' value in
brackets. "narrow" is the second pass. * weakly grounded references.

| genre | cosine: written → tuned | closer | style share | nearest own | Jamendo (genres) | YAMNet (classes) | LUFS (refs) | centroid Hz (refs) | > 2 kHz (refs) | kept |
|---|---|---|---|---|---|---|---|---|---|---|
| ambient | 0.799 → 0.820 | 8/8 | 0.371 → 0.371 | 4/8 → 4/8 | 0.472 → 0.489 (ambient+atmospheric) | 0.062 → 0.064 (Ambient music + New-age music) | -13.4 → -12.9 (-15.4) | 212 → 220 (321) | 0.8% → 0.8% (0.8%) | yes |
| techno | 0.792 → 0.868 | 8/8 | 0.525 → 0.622 | 8/8 → 8/8 | 0.306 → 0.344 (techno+minimal) | 0.102 → 0.105 (Techno + House music + Electronic dance music) | -13.7 → -13.0 (-10.5) | 247 → 352 (284) | 2.5% → 3.5% (2.6%) | no |
| techno (narrow) | 0.792 → 0.865 | 8/8 | 0.525 → 0.567 | 8/8 → 8/8 | 0.306 → 0.320 (techno+minimal) | 0.102 → 0.101 (Techno + House music + Electronic dance music) | -13.7 → -13.7 (-10.5) | 247 → 227 (284) | 2.5% → 2.1% (2.6%) | yes |
| synthwave | 0.778 → 0.794 | 8/8 | 0.126 → 0.154 | 8/8 → 8/8 | 0.132 → 0.133 (synthpop+80s) | 0.019 → 0.021 (Electronic music + Synthesizer) | -13.4 → -12.2 (-11.4) | 382 → 363 (502) | 3.7% → 2.9% (4.8%) | no |
| synthwave (narrow) | 0.778 → 0.790 | 8/8 | 0.126 → 0.149 | 8/8 → 8/8 | 0.132 → 0.138 (synthpop+80s) | 0.019 → 0.017 (Electronic music + Synthesizer) | -13.4 → -12.8 (-11.4) | 382 → 302 (502) | 3.7% → 2.6% (4.8%) | no |
| lo-fi | 0.731 → 0.770 | 8/8 | 0.122 → 0.161 | 7/8 → 8/8 | 0.148 → 0.214 (hiphop+triphop) | 0.000 → 0.001 (Hip hop music) | -13.7 → -14.1 (-12.1) | 174 → 158 (353) | 0.6% → 0.9% (2.4%) | yes |
| lo-fi (narrow) | 0.731 → 0.769 | 8/8 | 0.122 → 0.207 | 7/8 → 8/8 | 0.148 → 0.283 (hiphop+triphop) | 0.000 → 0.001 (Hip hop music) | -13.7 → -13.3 (-12.1) | 174 → 138 (353) | 0.6% → 0.4% (2.4%) | no |
| polka | 0.491 → 0.559 | 8/8 | 0.180 → 0.189 | 1/8 → 3/8 | 0.244 → 0.222 (folk+world) | 0.056 → 0.137 (Accordion + Brass instrument) | -12.0 → -9.6 (-17.7) | 1229 → 2075 (1177) | 18.5% → 37.5% (12.8%) | no |
| polka (narrow) | 0.491 → 0.540 | 8/8 | 0.180 → 0.169 | 1/8 → 3/8 | 0.244 → 0.205 (folk+world) | 0.056 → 0.095 (Accordion + Brass instrument) | -12.0 → -10.8 (-17.7) | 1229 → 1872 (1177) | 18.5% → 32.6% (12.8%) | no |
| tavern | 0.656 → 0.701 | 7/8 | 0.099 → 0.181 | 6/8 → 8/8 | 0.112 → 0.146 (medieval+folk) | 0.019 → 0.028 (Traditional music + Folk music) | -11.7 → -10.5 (-14.3) | 743 → 1223 (601) | 9.9% → 21.7% (3.4%) | no |
| tavern (narrow) | 0.656 → 0.688 | 7/8 | 0.099 → 0.172 | 6/8 → 8/8 | 0.112 → 0.141 (medieval+folk) | 0.019 → 0.022 (Traditional music + Folk music) | -11.7 → -10.4 (-14.3) | 743 → 1014 (601) | 9.9% → 15.2% (3.4%) | no |
| ska | 0.620 → 0.658 | 7/8 | 0.266 → 0.311 | 7/8 → 8/8 | 0.168 → 0.204 (reggae+punkrock) | 0.011 → 0.010 (Ska + Reggae) | -13.0 → -11.7 (-11.0) | 797 → 734 (707) | 9.4% → 7.7% (8.9%) | yes |
| ska (narrow) | 0.620 → 0.653 | 8/8 | 0.266 → 0.271 | 7/8 → 7/8 | 0.168 → 0.167 (reggae+punkrock) | 0.011 → 0.008 (Ska + Reggae) | -13.0 → -12.2 (-11.0) | 797 → 813 (707) | 9.4% → 9.4% (8.9%) | yes |
| mariachi * | 0.469 → 0.507 | 8/8 | 0.053 → 0.035 | 0/8 → 0/8 | 0.286 → 0.262 (latin+world) | 0.007 → 0.013 (Music of Latin America + Trumpet) | -11.2 → -9.1 (-13.9) | 820 → 1353 (844) | 10.9% → 25.4% (3.3%) | no |
| mariachi (narrow) * | 0.469 → 0.508 | 8/8 | 0.053 → 0.043 | 0/8 → 0/8 | 0.286 → 0.246 (latin+world) | 0.007 → 0.019 (Music of Latin America + Trumpet) | -11.2 → -10.3 (-13.9) | 820 → 1219 (844) | 10.9% → 19.4% (3.3%) | no |
| indonesian | 0.756 → 0.790 | 8/8 | – | 5/8 → 6/8 | 0.075 → 0.077 (world+ethno+worldfusion) | 0.135 → 0.122 (Mallet percussion + Gong) | -11.3 → -10.1 (-15.9) | 718 → 815 (541) | 9.6% → 13.3% (5.4%) | no |
| indonesian (narrow) | 0.756 → 0.793 | 8/8 | – | 5/8 → 6/8 | 0.075 → 0.077 (world+ethno+worldfusion) | 0.135 → 0.149 (Mallet percussion + Gong) | -11.3 → -11.0 (-15.9) | 718 → 707 (541) | 9.6% → 9.1% (5.4%) | yes |
| starconflict * | 0.738 → 0.790 | 8/8 | 0.070 → 0.073 | 2/8 → 6/8 | 0.626 → 0.759 (soundtrack+orchestral+symphonic) | 0.031 → 0.034 (Orchestra + Soundtrack music) | -13.2 → -13.5 (-15.8) | 833 → 784 (671) | 8.1% → 6.5% (5.2%) | yes |
| starconflict (narrow) * | 0.738 → 0.783 | 8/8 | 0.070 → 0.088 | 2/8 → 5/8 | 0.626 → 0.710 (soundtrack+orchestral+symphonic) | 0.031 → 0.026 (Orchestra + Soundtrack music) | -13.2 → -14.4 (-15.8) | 833 → 763 (671) | 8.1% → 7.2% (5.2%) | yes |
| celtic | 0.634 → 0.710 | 8/8 | 0.177 → 0.363 | 2/8 → 4/8 | 0.215 → 0.268 (celtic+folk) | 0.006 → 0.017 (Traditional music + Folk music) | -11.2 → -9.7 (-15.8) | 863 → 1466 (1060) | 9.0% → 22.5% (10.6%) | no |
| celtic (narrow) | 0.634 → 0.705 | 8/8 | 0.177 → 0.332 | 2/8 → 3/8 | 0.215 → 0.247 (celtic+folk) | 0.006 → 0.012 (Traditional music + Folk music) | -11.2 → -10.6 (-15.8) | 863 → 1273 (1060) | 9.0% → 16.9% (10.6%) | no |
| western * | 0.778 → 0.778 | 0/8 | 0.059 → 0.059 | 7/8 → 7/8 | 0.283 → 0.283 (soundtrack+country) | 0.009 → 0.009 (Country + Soundtrack music) | -14.2 → -14.2 (-14.1) | 679 → 679 (557) | 5.4% → 5.4% (3.9%) | no |

Into TUNED: **ambient, techno (narrow), lo-fi, ska, indonesian (narrow), starconflict**.
Synthwave, polka, tavern, mariachi, celtic and western stay None. Checked after
writing it: with the rebuilt binary, `--tuned` (no `MUSICBOX_TUNING`) renders each kept
genre's seed 101 byte-identical to its held-out tuned render, the as-written songs are
unchanged (celtic, ambient, polka checked), and `cargo test --release` passes.

## Each genre

Knobs as the engine reads them: levels in dB, tilts in dB, reverb, tempo and human as
times, swing in 16ths added, spread as times. "Narrow" = levels within ±4 dB, tilts
within ±2.5 dB.

- **ambient**: kept (first pass). Arp −4.1, hat −3.6, fx +2.7, bell −2.6, drone −2.4,
  pad +2.2, kick +2.2. No knob at a limit; the spectrum stayed where it was (centroid
  212 → 220 Hz against the references' 321). The style share is capped (ambient
  already out-scores its references), so this is the cosine alone.
- **techno**: kept (narrow). Cymbals +4.0, hats +3.9, snare +2.9, bass +2.4, low shelf
  +2.3, high shelf −2.2. Hats, cymbals and the low shelf are at the narrow limits.
  The first pass (hats +6, kick −3.5) came as close but doubled the energy above
  8 kHz (2.5 % → 3.5 % above 2 kHz against 2.6 %); the narrow pass got the same
  cosine with the highs shelved down and the treble back at the references'.
- **lo-fi**: kept (first pass), **the one to look at hardest**. Keys −5.9 (at the
  limit), hats +3.2, fx +2.2, tempo ×0.96. It passes every rule, but the keys (the
  electric piano) are nearly gone, the 250 Hz–2 kHz band halved (13.6 % → 7.4 % of
  the energy, the references 16.6 %) and the centroid moved further from the
  references (174 → 158 Hz, against 353). The narrow pass went darker still and
  failed. YAMNet says nothing for lo-fi (its "Hip hop music" is 0.000–0.001 on
  everything). Drop it if it sounds hollow.
- **ska**: kept (first pass). Drone +6 (at the limit; in ska that channel is the
  piano, or the distorted guitar in third-wave songs), bass +3.9,
  tempo ×1.09, keys −3.0, spread ×0.65. Loudness, centroid and treble all moved
  toward the references. Its tempo went 136 → 150 bpm: the references' median is
  136, but their tempos are two clusters (85–99, likely half-time detections, and
  129–172), and 150 sits inside the upper one. The narrow pass was kept too (tempo
  at its ×1.12 limit, arp +3.9), slightly less close (0.653): the first pass is in
  TUNED.
- **indonesian**: kept (narrow). Pad +4.0 and high shelf +2.5 (both at the narrow
  limits), keys −3.2, drone +2.8, cymbals +2.6, swing +0.11 (close to its limit:
  swung sixteenths are not gamelan's, and are worth a listen). The treble came
  slightly closer to the references (9.6 % → 9.1 %, against 5.4 %). The first pass
  (bell +6) put the brightness the last round took out back in (13.3 %).
- **starconflict** *: kept (first pass). Pad +6 (at the limit), lead
  −3.5, bass −2.7, cymbals +2.6, spread ×1.38. Treble, centroid and stereo width all
  moved toward the references (width 0.19 → 0.30 of the energy, against 0.31). The
  narrow pass (lead and keys −4) was kept too, less close; the references are
  stand-ins (orchestral marches and Romantic symphonies), not film scores.
- **synthwave**: not kept. Both passes went darker (energy above 2 kHz 3.7 % → 2.9 %
  and 2.6 %, the references 4.8 %; centroid 382 → 302 Hz in the narrow pass, against
  502) for small gains (+0.015, +0.012). The first pass's pads +5.8 echo the last
  round's finding that synthwave's pads sat too far back.
- **celtic**: not kept. Both passes: highs up, kick and keys down, arp up (first
  pass high shelf +5, its limit). Twice the references' treble (9.0 % → 22.5 %, and
  16.9 % narrow, against 10.6 %) and most of the 60–250 Hz band gone (18 % → 5 % of
  the energy, the references 12 %). The cosine
  gain was the largest of any genre (+0.076), and all of it came with that.
- **polka**: not kept. Lead +6 and keys −6 (both limits), bass −4.6: treble 18.5 % →
  37.5 % (32.6 % narrow) against the 78s' 12.8 %, and the Jamendo verdict fell.
- **tavern**: not kept. Lead +4.6, keys −6 (limit), bell −4.4: even with the highs
  shelved down, the treble went 9.9 % → 21.7 % (15.2 % narrow) against 3.4 %.
- **mariachi** *: not kept. Lead +6 and highs up: treble 10.9 % → 25.4 % (19.4 %
  narrow) against the 1908 records' 3.3 %, louder by 2 LU, the Jamendo verdict down,
  no song nearest mariachi before or after.
- **western** *: not kept. Nothing scored better than as written in 62 evaluations;
  the tuned held-out songs were byte-identical to the written ones.

## What it found

- **The tagger rewards brightness and a melody-forward mix more than the references
  have them.** In the first pass, 10 of the 11 genres that moved pinned a knob at a
  limit, and the searches converged on the same move: lead up, keys (and often bass
  and kick) down, highs up. The keys channel, each genre's comping instrument
  (lo-fi's Rhodes, polka's accordion, ska's guitar chop, tavern's lute, mariachi's
  vihuela), was cut 3 to 6 dB in 8 of the 10 genres that have one.
  Celtic, polka, tavern, mariachi and the first passes of indonesian and techno all
  ended up well brighter than their references. Lo-fi and synthwave went the other
  way, darker than theirs. Narrowing the ranges did not stop it: 9 of the 10 narrow
  passes pinned at the narrow limits, and the bright genres stayed too bright. So
  the pins are the finding, not something a third pass would fix. Five of the six
  kept settings have a knob at a limit. What guarded against the gaming was the
  signal judges, not the pins.
- **The cosine gains held out.** Every genre that moved got 7 or 8 of its 8 held-out
  songs closer, keeping from about 40 % to all of its training gain (tavern
  0.106 → 0.045 and ska 0.071 → 0.038 kept least): not seed-overfitting, but
  taggers' gains. The Jamendo head mostly agreed (it reads the same embedding).
  YAMNet moved little and not consistently (polka's "Accordion + Brass" rose
  0.056 → 0.137 as its lead came forward).
- **Noise floor**: rounding a setting to 2 decimals moved the training cosine by up
  to 0.005 (ambient: tempo 1.026 → 1.03 shifts every onset), so differences under
  about 0.005 between two settings mean nothing.
- **Engine findings** (not fixed; no Rust was changed but TUNED). The `tempo` knob
  changes the bpm reported but not the audio, byte for byte, in celtic and
  indonesian, whose pieces fix their own timing. Negative `swing` is clamped to 0
  in every genre but lo-fi, the only one written with swing, so half its range does
  nothing elsewhere. `human` does nothing in techno and polka, which have no
  lateness. `spread` hardly registers with a tagger that listens in mono (celtic's
  ablation: no change). The tool finds dead knobs per genre and leaves them at rest.

## Doing it again

```
systemd-run --user --scope -q -p CPUQuota=150% -p MemoryMax=8G nice -n 10 \
    uv run tools/tune/tune.py run GENRE... --bin COPY_OF_THE_BINARY
systemd-run ... uv run tools/tune/tune.py run GENRE... --tag narrow --range levels=-4:4 --range tilt=-2.5:2.5
uv run tools/tune/tune.py table          # this file's table
uv run tools/tune/tune.py write          # TUNED; then rustfmt src/music/tuned.rs
```

Logs, search state and decisions are in `~/.cache/musicbox/tune/` (`GENRE[.narrow].jsonl`
every evaluation, `.search.json`, `decisions.json`); the held-out renders, for
listening, are in `~/.cache/musicbox/tune/held/GENRE/{written,tuned,tuned.narrow}/`.
Run time: pilot 15 min, first pass 5 h 52 min, second 3 h 56 min.
