#!/usr/bin/env bash
# Build a macOS package: ffers.app (the launcher, with the game inside it) plus the documents,
# zipped. No original game data, caches or music.
#
#   tools/package-macos.sh <name> [--no-build] [--flat]
#
# writes dist/<name>.zip, e.g. `tools/package-macos.sh ffers-v0.9.1-macOS`. With --flat the zip
# holds ffers.app and the documents at its top level instead of inside a folder called <name>
# (the layout recomplauncher.json's start.program path expects). Run it on a Mac: it
# uses sips, iconutil, codesign and ditto. The app is signed ad hoc only (there is no Apple
# developer identity), so the first start needs right-click > Open, or
# `xattr -dr com.apple.quarantine ffers.app`; see README.md.
set -euo pipefail

name="${1:?usage: package-macos.sh <name> [--no-build] [--flat]}"
shift
build=1
flat=0
for flag in "$@"; do
    case "$flag" in
        --no-build) build=0 ;;
        --flat) flat=1 ;;
        *) echo "Unknown option $flag" >&2; exit 2 ;;
    esac
done
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [ "$build" = 1 ]; then
    cargo build --release --locked -p ffe-rs -p ffe-launcher
fi

version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' crates/ffe-rs/Cargo.toml | head -1)"
dist="$root/dist"
stage="$dist/$name"
app="$stage/ffers.app"
rm -rf "$stage" "$dist/$name.zip"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$stage/docs"

cp target/release/ffe-launcher target/release/ffe-rs "$app/Contents/MacOS/"
chmod +x "$app/Contents/MacOS/ffe-launcher" "$app/Contents/MacOS/ffe-rs"

# The icon: an .icns made from the launcher's PNG.
iconset="$dist/ffers.iconset"
rm -rf "$iconset"
mkdir -p "$iconset"
for size in 16 32 64 128 256 512; do
    sips -z "$size" "$size" crates/ffe-launcher/assets/icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
done
cp "$iconset/icon_32x32.png" "$iconset/icon_16x16@2x.png"
cp "$iconset/icon_64x64.png" "$iconset/icon_32x32@2x.png"
cp "$iconset/icon_256x256.png" "$iconset/icon_128x128@2x.png"
cp "$iconset/icon_512x512.png" "$iconset/icon_256x256@2x.png"
rm "$iconset/icon_64x64.png"
iconutil -c icns "$iconset" -o "$app/Contents/Resources/ffers.icns"
rm -rf "$iconset"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>ffers</string>
    <key>CFBundleDisplayName</key><string>Frontier: First Encounters (ffe-rs)</string>
    <key>CFBundleIdentifier</key><string>io.github.louisvarley.ffers</string>
    <key>CFBundleExecutable</key><string>ffe-launcher</string>
    <key>CFBundleIconFile</key><string>ffers</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
</dict>
</plist>
EOF

cp LICENSE README.md recomplauncher.json "$stage/"
cp docs/CONTROLS.md docs/LEGAL.md docs/THIRD_PARTY.md docs/LGPL.md "$stage/docs/"
cp crates/ffe-audio/assets/soundfont/LICENSE "$stage/GeneralUser-GS-LICENSE.txt"
python3 tools/third-party.py --texts "$stage/THIRD_PARTY_LICENSES.txt"

cat > "$stage/PACKAGE.txt" <<EOF
ffe-rs $version (macOS)

1. Move ffers.app wherever you like (for example Applications) and open it. It is not signed by
   an Apple developer, so macOS blocks the first start: right-click ffers.app, choose Open, then
   Open again. If macOS says the app is damaged, run
       xattr -dr com.apple.quarantine /path/to/ffers.app
2. In the launcher, choose the folder with your installed First Encounters (the floppy English 1.02
   or CD English 1.01 release). The launcher keeps its saves, recordings and music pack in
   ~/Library/Application Support/ffe-rs, or beside ffers.app if that folder already holds a
   "game" or "saves" folder.
3. Controls: docs/CONTROLS.md. The README explains the launcher and the enhancements.

This package contains no original game data.
EOF

# Nothing the player supplies or the game generates may ship.
if find "$stage" -type f \( -iname '*.hmp' -o -iname '*.bmp' -o -iname '*.avi' -o -iname '*.raw' \
    -o -iname '*.sf2' -o -name 'FIRSTENC.EXE' -o -name 'MISSION.DAT' -o -name 'data.bin' \
    -o -name 'music.bin' \) | grep -q .; then
    echo "Refusing to package player data or generated files." >&2
    exit 1
fi

codesign --force --deep --sign - "$app"
if [ "$flat" = 1 ]; then
    (cd "$stage" && ditto -c -k . "$dist/$name.zip")
else
    (cd "$dist" && ditto -c -k --keepParent "$name" "$name.zip")
fi
echo "Packaged $dist/$name.zip"
