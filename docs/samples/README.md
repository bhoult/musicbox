# musicbox's recordings for the browser

The recorded instruments of musicbox's web page (github.com/bhoult/musicbox), made by
`tools/web-samples.sh` from the recordings `tools/get-samples.sh` fetches: each group of
recordings at 48 kHz (those recorded in mono first, then those recorded in stereo, in
stereo; a group with none in stereo, in mono), compressed to Opus at 128 kb/s
(`N.opus`), the sampler's
tables (`pack.bin`), and which groups each genre and mix plays (`pack.json`).
`lite/` is a lighter copy of the same for a phone, which cannot hold them all: every
group in one channel at 24 kHz, at 40 kb/s (`N.K.opus`: a group's stream in parts
of a minute and a half, cut between recordings), with tables of its own.

They come from:

- **MuseScore General 0.2**, a General MIDI SoundFont: MIT licence. FluidR3 by Frank Wen,
  FluidR3Mono by Michael Cowgill, adapted by S. Christian Collins; Temple Blocks by Ethan
  Winer; Drumline Cymbals by Michael Schorsch. Its acknowledgements and copyright notices,
  in `licenses/MuseScore_General/`, go with any copy.
- **VS Chamber Orchestra 2: Community Edition** by Versilian Studios (Sam Gossner) and Ivy
  Audio (Simon Dalzell), http://vis.versilstudios.net/vsco-community.html: CC0 1.0
  (`licenses/VSCO-2-CE/`).
- **Versilian Community Sample Library** by Versilian Studios,
  https://github.com/sgossner/VCSL: CC0 1.0 (`licenses/VCSL/`).
- **Big Rusty Drums**, **Black And Blue Basses**, **Black And Green Guitars**, **Meatbass**
  and **Shinyguitar** by Karoryfer Samples, https://github.com/sfzinstruments: CC0 1.0
  (`licenses/Big-Rusty-Drums/`, `licenses/Black-And-Blue-Basses/`,
  `licenses/Black-And-Green-Guitars/`, `licenses/Meatbass/`, `licenses/Shinyguitar/`).
- **Spanish Classical Guitar** and **Electric Guitar FSBS (dist1)** by FreePats,
  https://github.com/freepats: CC0 1.0 (`licenses/Spanish-Classical-Guitar/`,
  `licenses/FSBS-dist1/`).

`SOURCES.md` lists every file taken from each into the sample bank these were cut from.
