#!/bin/sh
# Stamps the page's script address in docs/index.html with a hash of the
# page's files (v) and one of the recordings (r): the page asks for each
# file with its stamp, so a browser that kept an older copy of any of them
# (GitHub Pages lets one be kept 10 minutes) fetches the new set whole. Run
# it after changing anything in docs/app or docs/samples.
set -e
cd "$(dirname "$0")/../docs"
v=$(cat app/*.js app/musicbox.wasm app/models/* app/mp3/* samples/pack.json samples/pack.bin | sha256sum | cut -c1-10)
r=$(cat samples/*.opus | sha256sum | cut -c1-10)
sed -i "s|<script src=\"app/app.js[^\"]*\">|<script src=\"app/app.js?v=$v\&r=$r\">|" index.html
grep -o 'app/app.js?[^"]*' index.html
