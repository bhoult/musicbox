# musicbox

Music made up as it plays, without end, in twelve genres and any pair of them. It plays from the command line, and can open a window with a live equalizer and a genre selector.

![The window: mariachi drummed by techno, the equalizer, and the genre selector](docs/window.png)

No AI model makes any of the sound, and no song is sampled. The music comes from a composer that is ordinary code: for each genre, rules for its scales, chords, rhythms, song forms and who plays what, and a random number generator choosing among them. It is generated at random and without end by software that an AI wrote. A genre and a seed always give the same music.

It was extracted from fleece, a fractal flame animator, whose videos in [flock](https://experiments.galedra.org/flock/) it scored. The composer, synthesiser and sampler are fleece's code unchanged: for a given genre and seed, `musicbox --wav` writes the same bytes as `fleece --musicwav`.

```
cargo build --release
target/release/musicbox                    # a random genre, from a random seed
target/release/musicbox celtic             # a genre by name (or number, 0-11)
target/release/musicbox polka+techno       # polka's music with techno's drums
target/release/musicbox western --gui      # with the window open
target/release/musicbox lo-fi --eq         # with an equalizer in the terminal
```

## The command line

The terminal shows what is playing and the keys, redrawn in place:

```
♪ musicbox  celtic with ska drums                                          seed 2817
  song 2 · 4/4 reel · 131 bpm · C ionian · main, bar 3 of 8 · 0:07
  ambient techno synthwave lo-fi polka tavern ska mariachi indonesian starconflict celtic western
  ←/→ genre  ↑/↓ drums  space next song  e equalizer  g window  q quit
```

The third line is the genre selector: the genre writing the music is lit, and the drummer is underlined. The song line gives the piece's number, meter and form, tempo, key and mode, its section and bar, and how long it has played.

| key | |
| --- | --- |
| Left / Right (also P / N) | previous / next genre |
| Up / Down (also J) | the drummer: none, then each other genre |
| Space, Enter (also Y) | next song: a new piece of the same music |
| E | the equalizer in the terminal, on or off (`--eq` starts with it on) |
| G | open or close the window (`--gui` opens it at the start) |
| Q, Esc, Ctrl-C | quit |

A change waits for the next bar line, or the next beat when bars are long, and then crossfades. Run outside a terminal, musicbox prints one line for each new piece instead.

## The window

`--gui` or G opens it. It has three parts:

- **What is playing:** the genres, then the same line as the terminal.
- **The equalizer:** what the speakers are playing, in LED bars from 35 Hz to 16 kHz with peak markers. Each bar is the loudest part of its band in an 85 ms window, tilted up 3 dB an octave so the highs show.
- **The genre selector:** click a genre under MUSIC BY to write the music, or right-click it to make it the drummer. The DRUMS BY row picks the drummer too, NONE for none. Below are buttons for NEXT SONG, FULLSCREEN (F or F11), CLOSE (Esc) and QUIT (Q).

The keys work in the window too. Closing the window leaves the music playing in the terminal. Without a terminal (a launcher, say), closing it quits.

The picture is drawn on the CPU and scaled up by a whole number, so its pixel font stays sharp at any size.

## The genres

| genre | |
| --- | --- |
| ambient | slow pads and choirs, bells, celesta, kalimba and harp over a drone, with wind or rain drifting through |
| techno | a kick on every beat, claps, an acid bassline, a rumble under the kick and the sidechain pumping |
| synthwave | arpeggios and pads, a synth lead, and a drum machine with the kick on one and three |
| lo-fi | a boom-bap beat under electric piano and a lazy lead, with vinyl crackle and tape wow |
| polka | oom on the beat and pah off it, with tuba and accordion |
| tavern | medieval jigs, reels and dances on lute, fiddle, whistle, hurdy-gurdy and frame drum |
| ska | Jamaican first wave, faster and straighter 2-tone, or third-wave ska-punk with choruses of distorted power chords under four horns |
| mariachi | a son jalisciense, huapango, ranchera valseada or polkeada, or bolero ranchero: violins and trumpets trading the tune in thirds and sixths over vihuela, guitar, guitarrón and harp |
| indonesian | gamelan in tunings of its own, not the piano's: Javanese court gamelan or Balinese gong kebyar |
| starconflict | a space-opera film score in John Williams's manner, its themes the composer's own: a heroic march, a dark march or a battle |
| celtic | the Irish and Scottish session: sets of reels, jigs and hornpipes, a pipe set, or a slow air on the harp |
| western | a spaghetti-western score in Ennio Morricone's manner, its tunes the composer's own: the duel, the ride or the lament |

The names `keltic`, `gamelan`, `spaghetti-western` and `space-opera` work too.

The composer writes pieces of about two minutes (60 to 158 bpm). Each piece has its own key, mode, tempo, chord progressions, drum patterns, bassline, melody and a second voice answering it, laid out in sections with something changing every four bars. Within a piece the bass mutates at phrase ends, the melody comes back varied, and filters drift. The next piece usually moves to a neighbouring key. It never loops, though each genre keeps its style.

**Mixes.** Two genres joined with `+` (or chosen with the drummer keys) make a mix. The first writes the music: its form, meter, chords and tunes. The second drums it: its beat fitted to the first's meter, its kits and effects, and a tempo 60% of the way to its own. Each part gets an instrument from each genre. Any of the 132 pairs plays.

## Recorded instruments

Every genre plays recordings of real instruments once `tools/get-samples.sh` has fetched them. Without them, a small synthesiser plays everything: band-limited oscillators, resonant filters, FM bells and electric piano, and synthesised drums.

```
tools/get-samples.sh                          # about 1 GB into ~/.local/share/musicbox/samples
target/release/musicbox --samples-list        # what loads
target/release/musicbox mariachi --no-samples # the synthesiser alone
```

The script needs git, curl, ffmpeg and python3. It also keeps about 3 GB of git clones in `~/.cache/musicbox/samples-src`, which can be deleted afterwards. If fleece's samples are already in `~/.local/share/fleece/samples` and musicbox's own directory doesn't exist, musicbox reads fleece's. `--samples DIR` reads any other directory.

The recordings come from two sources:
- MuseScore General 0.2: MIT; FluidR3 by Frank Wen, FluidR3Mono by Michael Cowgill, adapted by S. Christian Collins.
- Versilian Studios' VS Chamber Orchestra 2: Community Edition and Versilian Community Sample Library: CC0.

`SOURCES.md` and `licenses/` in the samples directory list everything taken. The bank takes about 1 GB of memory and loads before the first note (0.6 s here), so every piece plays on it.

## To a file

```
target/release/musicbox --wav celtic 120 celtic.wav --seed 7
target/release/musicbox --wav western+celtic 90 ride.wav --ending natural --fade 3
```

`--wav GENRE SECONDS OUT.wav` renders 16-bit stereo at 48 kHz, faster than real time. It then prints the pieces it holds and some statistics about the composition. `--ending natural` winds the music down to an ending of its own; `fade` fades it over the last 4 s; `cut` (the default) stops where the time runs out. `--fade SECS` fades it in over half of SECS.

## Files

| file | contents |
| --- | --- |
| `src/music.rs`, `src/music/` | the composer, the synthesiser and the player (fleece's); the later genres each in a file of their own |
| `src/samples.rs` | the sampler: SoundFont and multisample loading (fleece's) |
| `src/tap.rs` | what the player hands the displays: the last second of sound and what is playing |
| `src/spectrum.rs` | the equalizer's FFT and bars |
| `src/term.rs` | the command line's display and keys |
| `src/gui.rs`, `src/font.rs` | the window, and its 5x7 font (from FRAKTAL256) |
| `src/sdl.rs` | the SDL2 calls: the sound device and the window |
| `tools/get-samples.sh`, `tools/samples_manifest.py` | fetch and prepare the recorded instruments |

## Requirements and licence

- Linux on x86-64 (glibc), and Rust 1.88 or newer (edition 2024). No crates.
- libSDL2, which plays the sound and opens the window. The build links `/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0`; set `SDLLIB=/path/to/libSDL2-2.0.so.0` at build time if yours is elsewhere.
- Sound goes through SDL's default output, or `pacat` (PulseAudio or PipeWire) if SDL cannot open a device.

GPL v3 or later (see `LICENSE`).
