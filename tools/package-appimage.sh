#!/usr/bin/env bash
# Build a Linux x86_64 AppImage: the launcher (the program the AppImage starts) with the game
# beside it. No original game data, caches or music.
#
#   tools/package-appimage.sh <name> [--no-build]
#
# writes dist/<name>.AppImage, e.g. `tools/package-appimage.sh ffers-v0.9.1-x86_64`. Needs the
# Rust toolchain, Python 3 (the licence texts) and curl; it downloads appimagetool once into
# dist/.tools/. The player puts the `game` folder beside the .AppImage file: the launcher keeps its
# saves, recordings and music pack there too (`$APPIMAGE`).
set -euo pipefail

name="${1:?usage: package-appimage.sh <name> [--no-build]}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [ "${2:-}" != "--no-build" ]; then
    cargo build --release --locked -p ffe-rs -p ffe-launcher
fi

dist="$root/dist"
appdir="$dist/$name.AppDir"
rm -rf "$appdir" "$dist/$name.AppImage"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/doc/ffers/docs"

cp target/release/ffe-launcher target/release/ffe-rs "$appdir/usr/bin/"
doc="$appdir/usr/share/doc/ffers"
cp LICENSE README.md recomplauncher.json "$doc/"
cp docs/CONTROLS.md docs/LEGAL.md docs/THIRD_PARTY.md docs/LGPL.md "$doc/docs/"
cp crates/ffe-audio/assets/soundfont/LICENSE "$doc/GeneralUser-GS-LICENSE.txt"
python3 tools/third-party.py --texts "$doc/THIRD_PARTY_LICENSES.txt"

cp crates/ffe-launcher/assets/icon.png "$appdir/ffers.png"
cp crates/ffe-launcher/assets/icon.png "$appdir/.DirIcon"

cat > "$appdir/ffers.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Frontier: First Encounters (ffe-rs)
Comment=A native Rust reimplementation of Frontier: First Encounters
Exec=ffe-launcher
Icon=ffers
Categories=Game;
Terminal=false
EOF

# No arguments: the launcher. Arguments that start with -- (`--game DIR --root DIR`, or
# `--prepare-music`, as a host launcher such as RecompLauncher passes them): the game itself.
cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
case "${1:-}" in
    --*) exec "$HERE/usr/bin/ffe-rs" "$@" ;;
    *) exec "$HERE/usr/bin/ffe-launcher" "$@" ;;
esac
EOF
chmod +x "$appdir/AppRun" "$appdir/usr/bin/ffe-launcher" "$appdir/usr/bin/ffe-rs"

# Nothing the player supplies or the game generates may ship.
if find "$appdir" -type f \( -iname '*.hmp' -o -iname '*.bmp' -o -iname '*.avi' -o -iname '*.raw' \
    -o -iname '*.sf2' -o -name 'FIRSTENC.EXE' -o -name 'MISSION.DAT' -o -name 'data.bin' \
    -o -name 'music.bin' \) | grep -q .; then
    echo "Refusing to package player data or generated files." >&2
    exit 1
fi

tool="$dist/.tools/appimagetool-x86_64.AppImage"
if [ ! -x "$tool" ]; then
    mkdir -p "$dist/.tools"
    curl -fsSL -o "$tool" \
        https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$tool"
fi
# No FUSE on a build machine: let appimagetool unpack and run itself.
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$tool" "$appdir" "$dist/$name.AppImage"
chmod +x "$dist/$name.AppImage"
echo "Packaged $dist/$name.AppImage"
