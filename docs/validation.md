# musicbox against real music

Each genre's songs were measured against recordings of the real genre, and the clearest differences fixed. This is what was measured, what it found, what changed, and what it can't tell.

## How

**The references:** 228 recordings, about 20 a genre, all in the public domain or under a Creative Commons licence: from the Internet Archive (netlabels, the Library of Congress's National Jukebox, 78rpm transfers), Wikimedia Commons, the Free Music Archive's 2017 dataset, Musopen and ccMixter. `tools/validate/refs-manifest.csv` lists each with its source, licence and notes; the audio itself is not in the repository. They were only analysed, never redistributed. Their limits:

- **starconflict** and **western** have no open originals (Williams, Morricone): they were measured against the nearest open music, orchestral marches and Romantic symphonies, and twangy surf and western-guitar tracks. Treat those two genres' numbers as rough.
- **polka** is mostly 78rpm records and cylinders (17 of 20), and **mariachi** has only 6, four of them the Cuarteto Coculense in 1908 (before trumpets joined). Their recording quality (mono, little low end) is in every measure of them.
- **ska**'s first wave is copyrighted: its references are 2-tone and third-wave bands and netlabel ska.

**The judges:** pretrained neural networks, used only as measuring instruments (Essentia's models, CC BY-NC-SA 4.0, fetched by `tools/validate/get-models.sh`):

- *Discogs-EffNet*, trained on millions of tracks labelled with Discogs's 400 styles. Its verdict on a song is the share it gives the genre's own styles (Polka; Ska; Celtic; Medieval and Renaissance for tavern; Score and Soundtrack for the film genres; Boom Bap, Instrumental and Jazzy Hip-Hop for lo-fi; it has no gamelan). Its embedding, a 1280-number summary of how a recording sounds, gives the other two measures: **similarity**, the cosine between a song's embedding and the mean of its genre's real recordings, and **nearest own**, the share of a genre's songs whose nearest genre mean is their own (for the real recordings, each left out of its genre's mean: the ceiling to compare against).
- *MTG-Jamendo* (instruments) and Google's *YAMNet* (AudioSet's 521 sounds), to hear each instrument alone: `MUSICBOX_SOLO=lead,bass` renders only those channels. `MUSICBOX_NOLEVEL=1` holds the master's leveller at unity (a channel alone is then as loud as it is in the mix), and `MUSICBOX_STYLE=K` plays a styled genre's Kth style (the dice otherwise as they fall).

Alongside them, signal measures from Essentia: tempo, loudness and its range, the spectrum's centroid and the share of energy in five bands, stereo width (the side's share of the energy), onsets a second, key.

**The songs:** each genre's first song for seeds 101-108, whole, with the recorded instruments (`tools/validate/render.sh`).

## Results

Before is the code as extracted from fleece; after is this version.

| genre | real recordings | style share: real | style share: before | style share: after | nearest own: real | nearest own: before | nearest own: after | similarity: before | similarity: after |
|---|---|---|---|---|---|---|---|---|---|
| ambient | 21 | 0.29 | 0.37 | 0.37 | 86% | 50% | 50% | 0.799 | 0.799 |
| techno | 22 | 0.59 | 0.52 | 0.52 | 100% | 100% | 100% | 0.792 | 0.792 |
| synthwave | 22 | 0.25 | 0.12 | 0.13 | 91% | 100% | 100% | 0.784 | 0.778 |
| lo-fi | 21 | 0.46 | 0.14 | 0.12 | 100% | 88% | 88% | 0.732 | 0.731 |
| polka | 20 | 0.04 | 0.02 | 0.18 | 85% | 0% | 12% | 0.411 | 0.491 |
| tavern | 17 | 0.17 | 0.12 | 0.10 | 65% | 75% | 75% | 0.639 | 0.656 |
| ska | 22 | 0.35 | 0.27 | 0.27 | 95% | 88% | 88% | 0.620 | 0.620 |
| mariachi | 6 | 0.00 | 0.05 | 0.05 | 67% | 0% | 0% | 0.465 | 0.469 |
| indonesian | 22 | – | – | – | 95% | 62% | 62% | 0.745 | 0.756 |
| starconflict | 18 | 0.23 | 0.07 | 0.07 | 78% | 25% | 25% | 0.735 | 0.738 |
| celtic | 22 | 0.33 | 0.18 | 0.18 | 68% | 25% | 25% | 0.634 | 0.634 |
| western | 15 | 0.03 | 0.06 | 0.06 | 47% | 88% | 88% | 0.778 | 0.778 |

## What it found, and what changed

**Polka** read as chiptune and synth-pop (Chiptune 0.19, Polka 0.02), and its songs lay nearest real synthwave. Rendering it with one part silent at a time found the cause: the bass drum. Silencing the kick alone took Polka to 0.15 and Chiptune to 0.04. A loud kick on every beat of a 2/4 at 120 bpm is a dance track's four on the floor; in a polka band the drummer plays the bass drum lightly, under the tuba's oom, and the tuba carries the beat. Now it does (a quarter of its old velocity, the first beat a little heavier), and the drums are loosened by a few milliseconds: Polka is the tagger's top style for musicbox's polka (0.18, more than the 78s' own 0.04), its brightness matches the records' (centroid 1229 Hz against 1177), and its similarity rose from 0.41 to 0.49.

**Synthwave** was nearly mono (side 4% of the energy, against 20% on synthwave records), with its middle at 9% of the energy against the records' 32%: the pads and leads under the bass and kick. Its recorded polysynth had also lost the synthesiser patch's unison width (a recording plays as one voice). The channels gained a chorus (two delayed copies, their delays swung half a cycle apart, one to each side: a Juno's), which synthwave's pad, lead, arpeggio and glass now use; the pads and leads came forward and the bass and kick back; and its keys, 88% minor against the records' 55%, are now 62% minor. Width and balance moved toward the records' (side 7%, middle 12%); the tagger's verdict barely moved (Synthwave 0.12 to 0.13, the records 0.25): it hears synth-pop and Italo-disco, synthwave's parents.

**Gamelan** was over three times as bright as real gamelan (18% of its energy between 2 and 8 kHz, against 5%) and light below (30% against 44%). A tilt (lows up 3 dB, highs down 6) brought it to 9% and 42%; similarity rose from 0.745 to 0.756.

**Tavern** had a heavy low end (55% against early-music records' 29%) from its frame drum and bass: both were turned down (to 43%). **Lo-fi** was muffled (0.4% between 2 and 8 kHz against the records' 2.3%): its highs were lifted 5 dB. **Starconflict** was twice as bright as an orchestra (10.6% against 5.2%): its highs went down 4 dB (to 8.1%). Tavern's and starconflict's similarity rose a little, lo-fi's not at all; none moved the tagger.

**Mariachi's** recorded violins and trumpets played nearly straight: measured (the pitch's standard deviation over a held note), the violin section's recording has 4 cents of vibrato and a held trumpet's 2, where the style's, as the synthesiser's patches for them have it, swings 20 to 30 cents either way (14 to 21 as a deviation). They now get the difference. It made little difference to the taggers: musicbox's mariachi still reads as folk, chanson or Greek éntekhno, and the 1908 references are no help (the tagger gives them no mariachi either). Real mariachi on record is nearly all sung, which is the likeliest reason.

**Unchanged:** ambient, techno, ska, celtic and western, whose numbers were already near their records' (ska's tempo, brightness, width and onsets match its references almost exactly), or whose references can't say (western's).

**Tried and taken back:** a second round of tilts for synthwave and tavern, and a louder snare for lo-fi, which moved the spectrum closer but the embedding further. A rebalancing of celtic's tune types and keys was taken from The Session's tune database and then withdrawn: that database's licence forbids processing it with LLM tools, which this work is. Celtic is as fleece had it.

## Against records: why it sounded like a computer from 1985

Measured a second time (2026-11-01), against a private collection of 115 pop, rock, hip-hop and R&B records from 1956 to 2011 (only analysed: no audio, features or titles from them are in the repository; only the aggregate targets below). Every genre's songs, on recorded instruments too, read to the Discogs tagger as Chiptune 20 to 150 times more than the records do (median 0.01-0.10 against 0.0006), and to YAMNet as "video game music" (0.002-0.035 against 0.0004). Real instrumental recordings (celtic, polka) read 0.0002, so it is not the missing singer. On the synthesiser it is 0.2-0.33 in tavern, polka and mariachi: a Commodore's numbers.

What it found:

- **The styled genres' kick drowned the band.** Each of their instruments is brought to a reference loudness before its level is set; their drum kits were not, and played at the level of the kits they came from. Heard a channel at a time at its level in the mix, the kick was 46-87 % of the energy and the tune 1-5 %; the low band 0.75-0.94 of the energy against the records' 0.5, the middle 0.1-0.2 against 0.37, and the stereo near mono (the side 1-5 % of the energy against 14 %). Each band style now has a balance, a trim a channel, measured by `tools/validate/balance.py` and brought to a band's shares of the energy (the bass and kick most of the low end, the rhythm part and the tune most of the middle). The low band is now 0.50-0.65, the middle 0.28-0.44, the width 0.05-0.14 (the records': 0.42-0.58, 0.32-0.49, 0.09-0.19 in the decades they stand for). Sixteen styles double-track their rhythm guitar; the synthesiser styles' pads and leads have a chorus; hard rock, grunge and punk are 5 dB less bright.
- **The SoundFont's bass is the chip.** Left out, a channel at a time, the bass is what moved the "video game" reading most (3.5-5 times, in rock, 70s, 80s, classical and polka). Alone, the SoundFont's picked and fingered basses read Chiptune 0.15-0.36; the same lines on a recorded upright bass, 0.005-0.010. A low-pass helps (600 Hz: 0.08-0.13); a longer release does not. The band styles now play the bass darker (about 1 kHz), and an upright where the period has one.
- **Short loops.** The SoundFont holds many of its notes on a loop of one to ten cycles of the wave (the basses, guitars, electric pianos, clavinet, tuba, clarinet, harp): a held note is then one wave repeated, as a sound chip's is. Those 34 presets now wander a few cents in pitch, and the plucked ones lose their highs before their body. It moved the taggers little (a first version took a solo bass's Chiptune down by a third); it is for ears.
- **What did not help:** humanised timing (8 ms) and velocity moved nothing; an EQ toward the records' bands alone made tavern read more chip (its instruments' upper middle, uncovered).

Balancing made the band genres read more chip, not less: the kick had covered the bass and the tune, and now they are heard (rock 0.027 to 0.056, 2000s 0.031 to 0.125). The old twelve genres moved only with the short loops (their style shares and similarity held; celtic's rose, 0.18 to 0.22 and 0.634 to 0.660). Left out again, the bass and the lead are what still read as chip: the SoundFont's electric bass, and the synthesiser's own basses and leads in the electronic styles. The next step is better recordings of an electric bass (and a drum kit) than the SoundFont's, or a better bass on the synthesiser.

## What it can't tell

The taggers are trained on records, so they hear production as much as music: the 78s' surface noise, a vocal (most mariachi, ska and lo-fi on record has one; musicbox has none), a mastering engineer's loudness. A song can move toward a genre's records in these numbers and sound no more like the genre, or the reverse. The numbers found where to listen (polka's kick, gamelan's brightness); whether each change is better is for ears. Eight songs a genre is few: a change of a song or two in "nearest own" is noise.

## Would a neural network help?

As a judge, yes: that is what was used here, and it found the polka kick in an afternoon. As the composer, not for musicbox: a generative model (MusicGen, Stable Audio and the like) would replace the seeded composer that makes the same music for the same seed and never ends, would need a GPU to play live, would not fit in a browser, and its weights are mostly licensed for non-commercial use. A small model trained on a genre's tunes could suggest melodies within the rules, but the rules are the point of musicbox, and a model trained on a corpus brings that corpus's licence with it.

## Doing it again

```
tools/validate/get-models.sh
tools/validate/render.sh after                  # into ~/.cache/musicbox/validate/ours/after
cd ~/.cache/musicbox/validate
uv run ~/programming/musicbox/tools/validate/critic.py refs.jsonl refs        # the references (once)
uv run ~/programming/musicbox/tools/validate/critic.py after.jsonl ours/after
uv run ~/programming/musicbox/tools/validate/report.py refs.jsonl after.jsonl
uv run ~/programming/musicbox/tools/validate/balance.py    # each band style's balance, its rows for styles.rs
```

The references are fetched by hand from the manifest's URLs (Essentia's loader stalls on some Ogg and Opus files: convert those to WAV first).

## Each genre

### ambient

- real recordings (21): Ambient 0.29, Experimental 0.23, Drone 0.12, Dark Ambient 0.09, Industrial 0.08
- before (8): Ambient 0.37, Downtempo 0.19, Chillwave 0.12, Experimental 0.10, New Age 0.06; nearest: ambient 4, synthwave 3, western 1
- after (8): Ambient 0.37, Downtempo 0.19, Chillwave 0.12, Experimental 0.10, New Age 0.06; nearest: ambient 4, synthwave 3, western 1

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 123 | 96 | 96 |
| loudness (LUFS) | -15.4 | -13.4 | -13.4 |
| loudness range (LU) | 7.8 | 4.3 | 4.3 |
| spectral centroid (Hz) | 321 | 212 | 212 |
| energy above 2 kHz | 0.8% | 0.8% | 0.8% |
| stereo width (side share) | 0.270 | 0.057 | 0.057 |
| onsets a second | 1.7 | 2.0 | 2.0 |
| in a minor key | 38% | 38% | 38% |

### techno

- real recordings (22): Techno 0.39, Minimal Techno 0.20, Minimal 0.16, House 0.12, Tech House 0.11
- before (8): Techno 0.46, House 0.19, Electro 0.14, Trance 0.09, EBM 0.09; nearest: techno 8
- after (8): Techno 0.46, House 0.19, Electro 0.14, Trance 0.09, EBM 0.09; nearest: techno 8

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 128 | 133 | 133 |
| loudness (LUFS) | -10.5 | -13.7 | -13.7 |
| loudness range (LU) | 3.8 | 5.1 | 5.1 |
| spectral centroid (Hz) | 284 | 247 | 247 |
| energy above 2 kHz | 2.6% | 2.5% | 2.5% |
| stereo width (side share) | 0.046 | 0.009 | 0.009 |
| onsets a second | 5.9 | 5.4 | 5.4 |
| in a minor key | 50% | 75% | 75% |

### synthwave

- real recordings (22): Synthwave 0.25, Synth-pop 0.18, Ambient 0.13, Electro 0.11, Chillwave 0.08
- before (8): Synth-pop 0.29, Chiptune 0.14, Electro 0.13, Italo-Disco 0.13, Synthwave 0.12; nearest: synthwave 8
- after (8): Synth-pop 0.32, Italo-Disco 0.16, Chiptune 0.16, Euro-Disco 0.13, Synthwave 0.13; nearest: synthwave 8

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 120 | 115 | 115 |
| loudness (LUFS) | -11.4 | -12.7 | -13.4 |
| loudness range (LU) | 3.9 | 4.8 | 4.4 |
| spectral centroid (Hz) | 502 | 317 | 382 |
| energy above 2 kHz | 4.8% | 3.0% | 3.7% |
| stereo width (side share) | 0.200 | 0.040 | 0.073 |
| onsets a second | 3.9 | 4.3 | 4.3 |
| in a minor key | 55% | 88% | 62% |

### lo-fi

- real recordings (21): Instrumental 0.24, Trip Hop 0.20, Downtempo 0.19, Trip Hop 0.17, Jazzy Hip-Hop 0.14
- before (8): Downtempo 0.15, Ambient 0.11, Experimental 0.10, Future Jazz 0.07, Instrumental 0.07; nearest: lo-fi 7, western 1
- after (8): Downtempo 0.15, Ambient 0.11, Experimental 0.10, Future Jazz 0.07, Broken Beat 0.06; nearest: lo-fi 7, western 1

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 94 | 89 | 89 |
| loudness (LUFS) | -12.1 | -13.8 | -13.7 |
| loudness range (LU) | 5.9 | 3.3 | 3.2 |
| spectral centroid (Hz) | 353 | 159 | 174 |
| energy above 2 kHz | 2.4% | 0.4% | 0.6% |
| stereo width (side share) | 0.092 | 0.017 | 0.021 |
| onsets a second | 3.6 | 3.3 | 3.6 |
| in a minor key | 81% | 50% | 50% |

### polka

- real recordings (20): Marches 0.19, Ragtime 0.15, Music Hall 0.06, Dixieland 0.06, Vocal 0.06
- before (8): Chiptune 0.19, Synth-pop 0.09, Electro 0.08, Techno 0.07, Experimental 0.06; nearest: synthwave 7, western 1
- after (8): Polka 0.18, Folk 0.11, Ranchera 0.07, Norteño 0.05, Chiptune 0.05; nearest: western 6, synthwave 1, polka 1

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 121 | 119 | 120 |
| loudness (LUFS) | -17.7 | -11.8 | -12.0 |
| loudness range (LU) | 3.6 | 2.1 | 3.1 |
| spectral centroid (Hz) | 1177 | 698 | 1229 |
| energy above 2 kHz | 12.8% | 10.2% | 18.5% |
| stereo width (side share) | 0.000 | 0.069 | 0.108 |
| onsets a second | 3.7 | 4.2 | 4.2 |
| in a minor key | 25% | 0% | 0% |

### tavern

- real recordings (17): Folk 0.19, Baroque 0.15, Nordic 0.13, Renaissance 0.11, Medieval 0.06
- before (8): Folk 0.24, Experimental 0.14, Ambient 0.13, Medieval 0.12, Tribal 0.07; nearest: tavern 6, western 1, ambient 1
- after (8): Folk 0.22, Ambient 0.13, Experimental 0.11, Medieval 0.10, Electro 0.05; nearest: tavern 6, western 2

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 129 | 112 | 112 |
| loudness (LUFS) | -14.3 | -11.5 | -11.7 |
| loudness range (LU) | 4.8 | 2.2 | 2.5 |
| spectral centroid (Hz) | 601 | 605 | 743 |
| energy above 2 kHz | 3.4% | 7.5% | 9.9% |
| stereo width (side share) | 0.121 | 0.078 | 0.076 |
| onsets a second | 3.3 | 5.0 | 5.0 |
| in a minor key | 41% | 50% | 50% |

### ska

- real recordings (22): Ska 0.20, Punk 0.16, Ska 0.15, Reggae 0.06, Alternative Rock 0.06
- before (8): Ska 0.14, Parody 0.13, Ska 0.13, Parody 0.11, Folk 0.06; nearest: ska 7, western 1
- after (8): Ska 0.14, Parody 0.13, Ska 0.13, Parody 0.11, Folk 0.06; nearest: ska 7, western 1

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 136 | 136 | 136 |
| loudness (LUFS) | -11.0 | -13.0 | -13.0 |
| loudness range (LU) | 3.3 | 3.0 | 3.0 |
| spectral centroid (Hz) | 707 | 797 | 797 |
| energy above 2 kHz | 8.9% | 9.4% | 9.4% |
| stereo width (side share) | 0.061 | 0.066 | 0.066 |
| onsets a second | 4.1 | 4.2 | 4.2 |
| in a minor key | 59% | 62% | 62% |

### mariachi

- real recordings (6): Vocal 0.26, Religious 0.15, Folk 0.11, Music Hall 0.05, Novelty 0.05
- before (8): Folk 0.08, Chanson 0.07, Éntekhno 0.06, Celtic 0.04, Ranchera 0.04; nearest: celtic 7, tavern 1
- after (8): Folk 0.08, Éntekhno 0.07, Chanson 0.06, Celtic 0.05, Fado 0.04; nearest: celtic 7, western 1

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 133 | 119 | 119 |
| loudness (LUFS) | -13.9 | -11.2 | -11.2 |
| loudness range (LU) | 10.6 | 2.9 | 3.0 |
| spectral centroid (Hz) | 844 | 821 | 820 |
| energy above 2 kHz | 3.3% | 11.0% | 10.9% |
| stereo width (side share) | 0.000 | 0.087 | 0.087 |
| onsets a second | 2.9 | 3.9 | 3.9 |
| in a minor key | 33% | 12% | 12% |

### indonesian

- real recordings (22): Experimental 0.16, Ambient 0.06, Experimental 0.06, Contemporary 0.05, Folk 0.05
- before (8): Experimental 0.20, Ambient 0.08, Abstract 0.06, Industrial 0.05, Techno 0.05; nearest: indonesian 5, ambient 3
- after (8): Experimental 0.23, Ambient 0.10, Abstract 0.07, Electro 0.05, Industrial 0.05; nearest: indonesian 5, ambient 3

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 137 | 103 | 103 |
| loudness (LUFS) | -15.9 | -11.1 | -11.3 |
| loudness range (LU) | 7.0 | 4.6 | 4.2 |
| spectral centroid (Hz) | 541 | 1104 | 718 |
| energy above 2 kHz | 5.4% | 19.0% | 9.6% |
| stereo width (side share) | 0.048 | 0.046 | 0.045 |
| onsets a second | 2.6 | 3.9 | 3.9 |
| in a minor key | 41% | 50% | 50% |

### starconflict

- real recordings (18): Romantic 0.16, Modern 0.15, Classical 0.13, Soundtrack 0.13, Neo-Romantic 0.11
- before (8): Ambient 0.06, Dungeon Synth 0.06, Easy Listening 0.05, Brass Band 0.05, Soundtrack 0.05; nearest: western 6, starconflict 2
- after (8): Ambient 0.07, Dungeon Synth 0.07, Easy Listening 0.06, Experimental 0.05, Soundtrack 0.05; nearest: western 6, starconflict 2

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 128 | 104 | 104 |
| loudness (LUFS) | -15.8 | -12.9 | -13.2 |
| loudness range (LU) | 15.4 | 12.3 | 12.1 |
| spectral centroid (Hz) | 671 | 930 | 833 |
| energy above 2 kHz | 5.2% | 10.7% | 8.1% |
| stereo width (side share) | 0.310 | 0.194 | 0.193 |
| onsets a second | 2.2 | 2.0 | 1.8 |
| in a minor key | 50% | 38% | 38% |

### celtic

- real recordings (22): Celtic 0.33, Folk 0.29, Bluegrass 0.04, Nordic 0.04, Folk Rock 0.04
- before (8): Celtic 0.18, Folk 0.13, Medieval 0.11, Chiptune 0.05, Baroque 0.04; nearest: tavern 6, celtic 2
- after (8): Celtic 0.18, Folk 0.13, Medieval 0.11, Chiptune 0.05, Baroque 0.04; nearest: tavern 6, celtic 2

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 123 | 123 | 123 |
| loudness (LUFS) | -15.8 | -11.2 | -11.2 |
| loudness range (LU) | 3.8 | 2.2 | 2.2 |
| spectral centroid (Hz) | 1060 | 863 | 863 |
| energy above 2 kHz | 10.6% | 9.0% | 9.0% |
| stereo width (side share) | 0.014 | 0.140 | 0.140 |
| onsets a second | 4.2 | 4.1 | 4.1 |
| in a minor key | 23% | 88% | 88% |

### western

- real recordings (15): Ambient 0.06, Experimental 0.06, Alternative Rock 0.05, Indie Rock 0.04, Downtempo 0.04
- before (8): Ambient 0.12, Folk 0.05, New Age 0.05, Experimental 0.05, Soundtrack 0.04; nearest: western 7, ambient 1
- after (8): Ambient 0.12, Folk 0.05, New Age 0.05, Experimental 0.05, Soundtrack 0.04; nearest: western 7, ambient 1

| | real | before | after |
|---|---|---|---|
| tempo (bpm) | 120 | 138 | 138 |
| loudness (LUFS) | -14.1 | -14.2 | -14.2 |
| loudness range (LU) | 6.0 | 8.9 | 8.9 |
| spectral centroid (Hz) | 557 | 679 | 679 |
| energy above 2 kHz | 3.9% | 5.4% | 5.4% |
| stereo width (side share) | 0.060 | 0.199 | 0.199 |
| onsets a second | 3.9 | 1.9 | 1.9 |
| in a minor key | 93% | 100% | 100% |
