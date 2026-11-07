#!/bin/sh
# Installs musicbox for this user, built from this tree:
#   ~/.local/bin/musicbox                          the program
#   ~/.local/share/musicbox/models/                its networks (models/), so it needs no source tree
#   ~/.local/share/applications/musicbox.desktop   a launcher that opens it with its window
#   ~/.local/share/icons/hicolor/scalable/apps/musicbox.svg
# Run it again after changing the program or its networks: the installed copy is a copy. The
# recordings are tools/get-samples.sh's, in ~/.local/share/musicbox/samples; without them it plays
# its synthesiser. Opened from the launcher it has no terminal, so closing the window ends it.
set -e
cd "$(dirname "$0")/.."
cargo build --release
bin="$HOME/.local/bin"
share="$HOME/.local/share"
mkdir -p "$bin" "$share/musicbox/models" "$share/applications" "$share/icons/hicolor/scalable/apps"
install -m 755 target/release/musicbox "$bin/musicbox.new"
mv -f "$bin/musicbox.new" "$bin/musicbox"
cp models/* "$share/musicbox/models/"
cp tools/musicbox.svg "$share/icons/hicolor/scalable/apps/musicbox.svg"
cat > "$share/applications/musicbox.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=musicbox
GenericName=Music generator
Comment=Music made up as it plays, without end, in 28 genres
Exec=$bin/musicbox --gui
Icon=musicbox
Terminal=false
Categories=AudioVideo;Audio;Player;
StartupWMClass=musicbox
EOF
command -v update-desktop-database >/dev/null && update-desktop-database -q "$share/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$share/icons/hicolor" || true
[ -d "$share/musicbox/samples" ] || echo "musicbox: no recordings yet; tools/get-samples.sh fetches them (about 1 GB)"
echo "musicbox: installed $bin/musicbox and its launcher"
