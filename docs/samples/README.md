# musicbox's recordings for the browser

The recorded instruments of musicbox's web page (github.com/bhoult/musicbox), made by
`tools/web-samples.sh` from the recordings `tools/get-samples.sh` fetches: each group of
recordings at 48 kHz, mono, compressed to Opus at 48 kb/s (`N.opus`), the sampler's
tables (`pack.bin`), and which groups each genre and mix plays (`pack.json`).

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
- **Big Rusty Drums** and **Black And Blue Basses** by Karoryfer Samples,
  https://github.com/sfzinstruments: CC0 1.0 (`licenses/Big-Rusty-Drums/`,
  `licenses/Black-And-Blue-Basses/`).

`SOURCES.md` lists every file taken from each into the sample bank these were cut from.
