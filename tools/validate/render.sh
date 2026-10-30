#!/bin/sh
# Renders a run of musicbox's songs for critic.py: each genre's first song
# for seeds 101-108, whole (its ending and ring-out), as WAV, into
# ~/.cache/musicbox/validate/ours/RUN/<genre>/.
#
#     tools/validate/render.sh RUN [musicbox binary]
set -e
run="${1:?a run name}"
bin="${2:-target/release/musicbox}"
out="$HOME/.cache/musicbox/validate/ours/$run"
for g in ambient techno synthwave lo-fi polka tavern ska mariachi indonesian starconflict celtic western; do
    mkdir -p "$out/$g"
    for s in 101 102 103 104 105 106 107 108; do
        echo "$g $s"
    done
done | xargs -P "${JOBS:-8}" -n 2 sh -c "\"$bin\" --export \$0 \"$out/\$0/\$0-\$1.wav\" --seed \$1 >/dev/null 2>&1"
echo "$out"
