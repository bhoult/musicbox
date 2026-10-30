#!/usr/bin/env bash
# The recorded instruments for the browser: musicbox --web-pack writes the
# bank's tables and each group's recordings (mono, 48 kHz), and this
# compresses each group to Opus.
#
#   tools/web-samples.sh OUT [KBPS]
#
# OUT gets pack.bin (the tables), pack.json (the groups, and those each
# genre plays) and N.opus for each group. KBPS: Opus's bit rate (48 by
# default). Needs ffmpeg with libopus, and the samples (tools/get-samples.sh).
set -euo pipefail

out=${1:?usage: tools/web-samples.sh OUT [KBPS]}
kbps=${2:-48}
here=$(cd "$(dirname "$0")/.." && pwd)
bin="$here/target/release/musicbox"
[ -x "$bin" ] || (cd "$here" && cargo build --release)

rm -rf "$out"
mkdir -p "$out"
"$bin" --web-pack "$out"

n=0
for w in "$out"/raw/*.wav; do
  g=$(basename "$w" .wav)
  ffmpeg -nostdin -loglevel error -y -i "$w" -c:a libopus -b:a "${kbps}k" -vbr on \
    -application audio -frame_duration 20 "$out/$g.opus"
  n=$((n + 1))
done
# each group's bytes in the index (the page's progress)
python3 - "$out" <<'PY'
import json, os, sys
out = sys.argv[1]
j = json.load(open(f"{out}/pack.json"))
for g in j["groups"]:
    g["bytes"] = os.path.getsize(f"{out}/{g['file']}")
json.dump(j, open(f"{out}/pack.json", "w"), separators=(",", ":"))
PY
raw=$(du -sb "$out/raw" | cut -f1)
rm -rf "$out/raw"
packed=$(du -cb "$out"/*.opus | tail -1 | cut -f1)
echo "$n groups: $((raw / 1000000)) MB of 16-bit mono made $((packed / 1000000)) MB of Opus at $kbps kb/s;" \
  "tables $(($(stat -c %s "$out/pack.bin") / 1000)) kB"
