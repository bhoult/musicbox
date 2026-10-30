#!/bin/sh
# Essentia's pretrained taggers for critic.py, into ~/.cache/musicbox/validate/models
# (about 41 MB). They are CC BY-NC-SA 4.0 (https://essentia.upf.edu/models.html):
# used here only to measure musicbox's music, never shipped with it.
set -e
dir="${MUSICBOX_MODELS:-$HOME/.cache/musicbox/validate/models}"
mkdir -p "$dir"
cd "$dir"
base=https://essentia.upf.edu/models
for f in \
    feature-extractors/discogs-effnet/discogs-effnet-bs64-1.pb \
    classification-heads/genre_discogs400/genre_discogs400-discogs-effnet-1.pb \
    classification-heads/genre_discogs400/genre_discogs400-discogs-effnet-1.json \
    classification-heads/mtg_jamendo_genre/mtg_jamendo_genre-discogs-effnet-1.pb \
    classification-heads/mtg_jamendo_genre/mtg_jamendo_genre-discogs-effnet-1.json \
    classification-heads/mtg_jamendo_instrument/mtg_jamendo_instrument-discogs-effnet-1.pb \
    classification-heads/mtg_jamendo_instrument/mtg_jamendo_instrument-discogs-effnet-1.json \
    audio-event-recognition/yamnet/audioset-yamnet-1.pb \
    audio-event-recognition/yamnet/audioset-yamnet-1.json; do
    [ -s "$(basename "$f")" ] || curl -sfLO "$base/$f"
done
echo "models in $dir"
