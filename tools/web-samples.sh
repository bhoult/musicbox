#!/usr/bin/env bash
# The recorded instruments for the browser: musicbox --web-pack writes the
# bank's tables and each group's recordings (48 kHz: its mono recordings,
# then those recorded in stereo, in stereo; mono when it has none in
# stereo), and a lighter copy of them for a phone (lite/: every group in one
# channel at 24 kHz, a quarter of the memory or less), and this compresses
# each group of both to Opus.
#
#   tools/web-samples.sh OUT [KBPS [LITE_KBPS]]
#
# OUT gets pack.bin (the tables), pack.json (the groups, and those each
# genre plays), N.opus for each group, the same again in lite/ (pack.bin and
# N.opus), and README.md, SOURCES.md and licenses/ (the samples' sources and
# licences, which go with them).
# KBPS: Opus's bit rate (128 by default: the desktop's recordings, near enough
# that no one hears the difference); LITE_KBPS: the lighter copy's (40).
# Needs ffmpeg with libopus, and the samples (tools/get-samples.sh).
set -euo pipefail

out=${1:?usage: tools/web-samples.sh OUT [KBPS [LITE_KBPS]]}
kbps=${2:-128}
lite=${3:-40}
here=$(cd "$(dirname "$0")/.." && pwd)
bin="$here/target/release/musicbox"
[ -x "$bin" ] || (cd "$here" && cargo build --release)

rm -rf "$out"
mkdir -p "$out"
"$bin" --web-pack "$out"

n=0
for w in "$out"/raw/*.wav; do
  g=$(basename "$w" .wav)
  # (bitexact: the stream's serial number fixed, not random, so the same
  # recordings make the same bytes)
  ffmpeg -nostdin -loglevel error -y -fflags +bitexact -i "$w" -c:a libopus -b:a "${kbps}k" -vbr on \
    -application audio -frame_duration 20 -flags:a +bitexact -fflags +bitexact "$out/$g.opus"
  n=$((n + 1))
done
for w in "$out"/lite/raw/*.wav; do
  g=$(basename "$w" .wav)
  ffmpeg -nostdin -loglevel error -y -fflags +bitexact -i "$w" -c:a libopus -b:a "${lite}k" -vbr on \
    -application audio -frame_duration 20 -flags:a +bitexact -fflags +bitexact "$out/lite/$g.opus"
done
# each group's bytes in the index, in both copies (the page's progress)
python3 - "$out" <<'PY'
import json, os, sys
out = sys.argv[1]
j = json.load(open(f"{out}/pack.json"))
for g in j["groups"]:
    g["bytes"] = os.path.getsize(f"{out}/{g['file']}")
    g["lite_bytes"] = os.path.getsize(f"{out}/lite/{g['file']}")
json.dump(j, open(f"{out}/pack.json", "w"), separators=(",", ":"))
PY
cat > "$out/README.md" <<EOF
# musicbox's recordings for the browser

The recorded instruments of musicbox's web page (github.com/bhoult/musicbox), made by
\`tools/web-samples.sh\` from the recordings \`tools/get-samples.sh\` fetches: each group of
recordings at 48 kHz (those recorded in mono first, then those recorded in stereo, in
stereo; a group with none in stereo, in mono), compressed to Opus at $kbps kb/s
(\`N.opus\`), the sampler's
tables (\`pack.bin\`), and which groups each genre and mix plays (\`pack.json\`).
\`lite/\` is a lighter copy of the same for a phone, which cannot hold them all: every
group in one channel at 24 kHz, at $lite kb/s, with tables of its own.

They come from:

- **MuseScore General 0.2**, a General MIDI SoundFont: MIT licence. FluidR3 by Frank Wen,
  FluidR3Mono by Michael Cowgill, adapted by S. Christian Collins; Temple Blocks by Ethan
  Winer; Drumline Cymbals by Michael Schorsch. Its acknowledgements and copyright notices,
  in \`licenses/MuseScore_General/\`, go with any copy.
- **VS Chamber Orchestra 2: Community Edition** by Versilian Studios (Sam Gossner) and Ivy
  Audio (Simon Dalzell), http://vis.versilstudios.net/vsco-community.html: CC0 1.0
  (\`licenses/VSCO-2-CE/\`).
- **Versilian Community Sample Library** by Versilian Studios,
  https://github.com/sgossner/VCSL: CC0 1.0 (\`licenses/VCSL/\`).
- **Big Rusty Drums** and **Black And Blue Basses** by Karoryfer Samples,
  https://github.com/sfzinstruments: CC0 1.0 (\`licenses/Big-Rusty-Drums/\`,
  \`licenses/Black-And-Blue-Basses/\`).

\`SOURCES.md\` lists every file taken from each into the sample bank these were cut from.
EOF
raw=$(du -sb "$out/raw" | cut -f1)
rawl=$(du -sb "$out/lite/raw" | cut -f1)
rm -rf "$out/raw" "$out/lite/raw"
packed=$(du -cb "$out"/*.opus | tail -1 | cut -f1)
packedl=$(du -cb "$out"/lite/*.opus | tail -1 | cut -f1)
echo "$n groups: $((raw / 1000000)) MB of 16-bit made $((packed / 1000000)) MB of Opus at $kbps kb/s;" \
  "tables $(($(stat -c %s "$out/pack.bin") / 1000)) kB"
echo "the lighter copy: $((rawl / 1000000)) MB of 16-bit made $((packedl / 1000000)) MB of Opus at $lite kb/s"
