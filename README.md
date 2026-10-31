# musicbox

Music made up as it plays, without end, in twelve genres and any pair of them. It plays from the command line, can open a window with a live equalizer and a genre selector, and runs in a browser.

**[Play it in your browser](https://bhoult.github.io/musicbox/)**: no install; it starts on its synthesiser, and fetches recorded instruments when they are asked for.

[![The browser version: celtic music with ska drums, its tunes by the network and its instruments recorded; the equalizer, the transport, the three settings and the genre selector](docs/web.png)](https://bhoult.github.io/musicbox/)

As it comes, no AI model makes any of the sound or the music, and no song is sampled. The music comes from a composer that is ordinary code: for each genre, rules for its scales, chords, rhythms, song forms and who plays what, and a random number generator choosing among them. It is generated at random and without end by software that an AI wrote. A genre and a seed always give the same music.

Three settings, each off until chosen, bring in what was learned from real music ([below](#how-it-is-made)): six genres' mixes as an optimiser tuned them against real recordings, celtic tunes written by a small network that learned O'Neill's *Music of Ireland*, and 18 instruments played by small networks that learned their recordings.

It was extracted from fleece, a fractal flame animator, whose videos in [flock](https://experiments.galedra.org/flock/) it scored. The composer, synthesiser and sampler are fleece's code, since tuned against recordings of the real genres ([below](#checked-against-real-music)): polka, synthwave, tavern, lo-fi, starconflict, gamelan and mariachi now sound different from fleece's, as do mixes with any of them in either part; the other genres, and mixes of them alone, still write fleece's bytes for a given genre and seed, with the three settings as they come.

```
cargo build --release
target/release/musicbox                    # a random genre, from a random seed
target/release/musicbox celtic             # a genre by name (or number, 0-11)
target/release/musicbox polka+techno       # polka's music with techno's drums
target/release/musicbox western --gui      # with the window open
target/release/musicbox lo-fi --eq         # with an equalizer in the terminal
target/release/musicbox ska --length 3:00  # each song three minutes long
target/release/musicbox --export celtic reel.mp3 --seed 7 --song 2   # one song, to an MP3
target/release/musicbox celtic --net-tunes --voices network    # tunes and instruments by networks
```

## The command line

The terminal shows what is playing and the keys, redrawn in place:

```
♪ musicbox  celtic with ska drums                                          seed 2817
  song 2 · 4/4 reel · 131 bpm · C ionian · main, bar 3 of 8 · 0:07 / 3:00
  ambient techno synthwave lo-fi polka tavern ska mariachi indonesian starconflict celtic western
  ←/→ genre  ↑/↓ drums  space next song  s stop  r replay  x export mp3  q quit
  [ ] length 3:00  t type a length  e equalizer  g window
  o tuning as written  m tunes rules  i instruments recorded
```

The third line is the genre selector: the genre writing the music is lit, and the drummer is underlined. The song line gives the piece's number, meter and form, tempo, key and mode, its section and bar, how long it has played, and the length asked of it. The last line is how the music is made ([below](#how-it-is-made)), with a note where a choice does nothing for the genre playing (celtic alone has network tunes, for one).

| key | |
| --- | --- |
| Left / Right (also P / N) | previous / next genre |
| Up / Down (also J) | the drummer: none, then each other genre |
| Space, Enter (also Y) | next song: a new piece of the same music |
| S | stop, and start again where it stopped |
| R | replay: the song playing, from its start again |
| [ / ] | each song 15 s shorter or longer (from auto: about the song's own length; below 0:30, auto again) |
| T | type a length: 3:30, 3.5m, 210s, or auto; Enter sets it, Esc cancels |
| X | export the song playing, from its start to its end, to an MP3 in ~/Music/musicbox |
| O | tuning: as written, or optimised |
| M | tunes: by the rules, or by the network |
| I | instruments: the synthesiser, the recordings, or the networks |
| E | the equalizer in the terminal, on or off (`--eq` starts with it on) |
| G | open or close the window (`--gui` opens it at the start) |
| Q, Esc, Ctrl-C | quit |

A change waits for the next bar line, or the next beat when bars are long, and then crossfades. Run outside a terminal, musicbox prints one line for each new piece instead.

## The window

![The window: mariachi drummed by techno, on network instruments; the equalizer, the transport, the three settings and the genre selector](docs/window.png)

`--gui` or G opens it. It has five parts:

- **What is playing:** the genres, then the same line as the terminal.
- **The equalizer:** what the speakers are playing, in LED bars from 35 Hz to 16 kHz with peak markers. Each bar is the loudest part of its band in an 85 ms window, tilted up 3 dB an octave so the highs show.
- **The transport and the length:** STOP / START, REPLAY, NEXT SONG and EXPORT MP3, then the songs' length: AUTO (each song as long as it was written), a slider from 0:30 to 10:00 (drag it, click it, or turn the mouse wheel over it), and a field to type a length into (click it, or press T).
- **How it is made:** TUNING (AS WRITTEN, OPTIMISED), TUNES (RULES, NETWORK) and INSTRUMENTS (SYNTH, RECORDED, NETWORK), as O, M and I have them. A choice that does something for the genre playing is marked in the accent colour.
- **The genre selector:** click a genre under MUSIC BY to write the music, or right-click it to make it the drummer. The DRUMS BY row picks the drummer too, NONE for none. Below are buttons for FULLSCREEN (F or F11), CLOSE (Esc) and QUIT (Q).

The keys work in the window too. Closing the window leaves the music playing in the terminal. Without a terminal (a launcher, say), closing it quits.

The picture is drawn on the CPU and scaled up by a whole number, so its pixel font stays sharp at any size.

## Song length

A song is as long as the composer writes it (about two minutes, from one to five) until a length is asked for: `--length 3:00`, the slider, `[` and `]`, or a length typed. Then each song steers for it as it plays: at a section's start, or every second bar of a main section, it turns to its outro early when going on would run long, or back to a main section (the earliest whose sections up to the outro fit) when it would end short. So songs keep their intro, form and ending, and land within a few seconds of the mark (asked 1:30, 3:00 or 6:40, every genre's songs came within 6 s). The shortest a song can be is its intro, four bars of its first main section and its outro (30 s to a minute). A new length applies to the song playing at its next turn.

## Start, stop and replay

Stop fades out in 30 ms and holds the music where it is; start goes on from there. A genre, drummer or next song chosen while stopped begins at once, from its start, when it plays again. Replay plays the song playing again from its start, crossfading at the next bar line as a genre change does: the engine is kept as it was when each song began, so the replay is the same music, note for note.

## Saving a song

X (or the window's EXPORT MP3) saves the song playing, on its own, from its start to its end and a few seconds' ring-out, to `~/Music/musicbox/GENRE-SEED-SONG.mp3`. It is rendered again from the song's start, faster than it plays (a three-minute song in two or three seconds), so the music goes on meanwhile. The MP3 is VBR at about 190 kbps, made by LAME's library (libmp3lame0 on Debian and Ubuntu), which musicbox loads only when it exports; its ID3 tag has the genre, the song's meter, tempo and key, and the command that makes it again:

```
target/release/musicbox --export GENRE OUT.mp3 --seed SEED --song K [--length L] [--tuned] [--net-tunes] [--voices V]
```

`--export` renders the Kth song (1 by default) of a genre and seed alone; a name ending in .wav writes a WAV instead (no LAME needed).

## In the browser

[bhoult.github.io/musicbox](https://bhoult.github.io/musicbox/) is the same composer and synthesiser compiled to WebAssembly (1.3 MB): the genres and mixes, the equalizer, the genre selector, the transport, the length, the three settings and the keys as the window has them. EXPORT saves a WAV. The engine runs in a Web Worker a few tenths of a second ahead of an AudioWorklet that plays it, so writing a bar or starting a song never stalls the sound.

It starts on the synthesiser. RECORDED (or I) fetches the recordings the genre plays, and those of the genre drumming. They are the desktop's recordings made mono at 48 kHz and compressed to Opus at 48 kb/s: 4 to 26 MB a genre, and 59 MB for all twelve, against the desktop's gigabyte. NETWORK instruments fetch their networks as well (4.2 MB), and network tunes fetch celtic's (0.8 MB). Each is fetched once, when it is first needed.

```
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --manifest-path web/Cargo.toml
cp web/target/wasm32-unknown-unknown/release/musicbox_web.wasm docs/app/musicbox.wasm
cp models/index.tsv models/*.mbn models/*.mbt docs/app/models/
tools/web-samples.sh docs/samples  # the recordings for the page (needs ffmpeg with libopus)
python3 -m http.server -d docs     # then http://localhost:8000
```

`tools/web-samples.sh` runs `musicbox --web-pack`, which plays 60 songs of each genre and 8 of each mix, at their own length, to see which recordings each uses. It writes them out as one stream for each instrument (a recording that two instruments share is stored once). A mix fetches its two genres' recordings and whatever more it was heard to play. Anything a song plays that was never heard plays on the synthesiser in the browser, as a part with no recording does on the desktop. It then compresses each stream to Opus. `web/lib.rs` builds src/music.rs and what it uses unchanged, and exports plain functions over the module's memory (no crates, no bindings generator); `docs/app/` is the page's JavaScript.

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

## How it is made

Three settings change how the music is made. Each is in every interface: a key in the terminal and the window, a row of buttons in the window and the page, and a flag for playing, `--wav` and `--export`. Changing one plays the song playing again from its start, made the new way, so the two can be heard one after the other. As they come, the music is as written, its tunes are the rules' and its instruments are recorded.

| setting | choices | key | flag |
| --- | --- | --- | --- |
| tuning | as written, optimised | O | `--tuned` |
| tunes | rules, network | M | `--net-tunes` |
| instruments | synth, recorded, network | I | `--voices synth\|recorded\|network` |

- **Optimised tuning:** each genre's mix as an optimiser set it, so that its songs sound more like the genre's real recordings to the taggers that checked them ([below](#checked-against-real-music)): each channel's level and stereo width, the low and high shelves, the reverb, the tempo (within 12%), the swing, and how late the hits come. The notes stay as written. Six genres have a setting: ambient, techno, lo-fi, ska, indonesian and starconflict. The other six play as written either way, and a mix takes the setting of its first genre. The taggers turned out to like a brighter, more melody-forward mix than the recordings have, so a setting was kept only where the songs' loudness and brightness, measured directly, stayed near the recordings'.
- **Network tunes:** celtic's reels, jigs, hornpipes, marches and airs written by a small network (a GRU, 0.8 MB) that learned 1,061 tunes from O'Neill's *Music of Ireland* (1903), as the O'Neill's Project transcribed them. The rest of the piece (its chords, form and instruments) is the rules' either way, so only the tunes change. On the statistics of real tunes, the network's are much closer than the rules': the rules repeat the same note a third of the time, where O'Neill's tunes do so 5 to 14% of the time. Other genres keep their rules' tunes.
- **Network instruments:** 18 held-note instruments (violin, violins, violas, cellos, contrabass, trumpet, trombone, horn, tuba, tenor sax, flute, clarinet, oboe, bassoon, recorder, ocarina and harmonica, with trumpet both held and with vibrato) played by small networks in the manner of DDSP. 250 times a second, a network of 234 KB gives a note's loudness, how it is shared among 48 harmonics, its noise in 16 bands and its vibrato, and oscillators and filtered noise play that. Each learned from its own recordings and plays within half a dB of their level. Everything else plays from the recordings, so the networks need them installed. SYNTH is the synthesiser alone, as `--no-samples` has it.

The networks run on the CPU in musicbox's own code, in the window and the browser alike. They are in `models/`; `MUSICBOX_MODELS=DIR` reads them from another directory. How each was made, and what it measured: [docs/neural.md](docs/neural.md).

## To a file

```
target/release/musicbox --wav celtic 120 celtic.wav --seed 7
target/release/musicbox --wav western+celtic 90 ride.wav --ending natural --fade 3
```

`--wav GENRE SECONDS OUT.wav` renders 16-bit stereo at 48 kHz, faster than real time. It then prints the pieces it holds and some statistics about the composition. `--ending natural` winds the music down to an ending of its own; `fade` fades it over the last 4 s; `cut` (the default) stops where the time runs out. `--fade SECS` fades it in over half of SECS. The settings' flags (`--tuned`, `--net-tunes`, `--voices`) apply here too.

## Checked against real music

Each genre was measured against recordings of the real thing, and the clearest differences fixed. The method, the numbers before and after, and what changed are in [docs/validation.md](docs/validation.md); in short:

- **The references:** 228 recordings, 20 or so a genre (6 for mariachi), all public domain or Creative Commons, from the Internet Archive, Wikimedia Commons, the Free Music Archive and Musopen. For a Williams-style space opera and a Morricone-style western none are open, so those genres were measured against the nearest open music. `tools/validate/refs-manifest.csv` lists each with its source and licence.
- **The judges:** neural networks trained on millions of real recordings, used only to measure: Essentia's Discogs-EffNet (400 Discogs styles, and an embedding in which similar music lies close), its MTG-Jamendo instrument tagger, and Google's YAMNet (AudioSet's 521 sounds). None of them makes any of musicbox's sound; the optimised tuning ([above](#how-it-is-made)) is the one place their verdict steers it. Alongside them, signal measures: tempo, loudness, the spectrum's balance, stereo width, onsets.
- **What they found, and what changed:** polka's loud bass drum on every beat made the tagger hear chiptune and synth-pop (polka 0.02): played light under the tuba, as a polka band's drummer does, it reads as polka (0.18, its top style). Synthwave had its pads buried under the bass and kick, and was nearly mono: they came forward, with a polysynth's chorus. Gamelan was over three times as bright as real gamelan; tavern and synthwave too heavy below, lo-fi too muffled, the space opera too bright: each was rebalanced. Mariachi's recorded violins and trumpets had lost the wide vibrato the style has.

```
tools/validate/get-models.sh                 # the taggers (41 MB)
tools/validate/render.sh after               # 8 songs a genre
uv run tools/validate/critic.py ours.jsonl ~/.cache/musicbox/validate/ours/after
uv run tools/validate/report.py refs.jsonl ours.jsonl
```

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
| `src/mp3.rs` | MP3s, by LAME's library, loaded when wanted |
| `src/net.rs` | the network instruments: the networks, and the oscillators and noise that play them |
| `src/tunes.rs` | the tunes network: the GRU and how it writes a part |
| `src/music/tuned.rs` | the optimiser's knobs, and each genre's settings |
| `models/` | the networks (`index.tsv` lists them); `docs/app/models/` is the page's copy |
| `web/lib.rs`, `docs/index.html`, `docs/app/` | the browser version: the WebAssembly build, and the page, its worker and its audio worklet |
| `tools/get-samples.sh`, `tools/samples_manifest.py` | fetch and prepare the recorded instruments |
| `tools/web-samples.sh` | the recorded instruments for the page, as Opus |
| `tools/neural/` | the networks' training: `tunes.py` and `TUNES.md` (its sources and credits), `voices.py` |
| `tools/tune/` | the optimiser |
| `tools/validate/` | the measuring against real music: the taggers, the renders, the comparison, the references' list |
| `docs/validation.md` | what was measured, and what changed |
| `docs/neural.md` | the three settings: how each was made, and what it measured |

## Requirements and licence

- Linux on x86-64 (glibc), and Rust 1.88 or newer (edition 2024). No crates.
- libSDL2, which plays the sound and opens the window. The build links `/usr/lib/x86_64-linux-gnu/libSDL2-2.0.so.0`; set `SDLLIB=/path/to/libSDL2-2.0.so.0` at build time if yours is elsewhere.
- Sound goes through SDL's default output, or `pacat` (PulseAudio or PipeWire) if SDL cannot open a device.
- MP3 export: LAME's library (`libmp3lame.so.0`), if it is there. Without it, export to .wav.
- The browser version: any current browser (WebAssembly and AudioWorklet). Making its recordings: ffmpeg with libopus.
- Only to train the networks or run the optimiser: Python and uv (PyTorch's CPU build; Essentia for the optimiser). Playing needs neither.

GPL v3 or later (see `LICENSE`).
