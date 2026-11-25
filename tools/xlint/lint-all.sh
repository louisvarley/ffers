#!/bin/sh
# Lint every CI target (Windows, Linux, macOS) from a Windows checkout, as CI does.
# The Linux/macOS builds are check-only: stand-in cc/ar/pkg-config let the C build
# scripts (alsa-sys, minifb) succeed without producing anything linkable.
# Needs: rustup target add x86_64-unknown-linux-gnu aarch64-apple-darwin; python.
set -e
here="$(cd "$(dirname "$0")" && pwd -W 2>/dev/null || pwd)"
export PKG_CONFIG="$here/fakepc.bat" PKG_CONFIG_ALLOW_CROSS=1
export CC_x86_64_unknown_linux_gnu="$here/fakecc.bat" AR_x86_64_unknown_linux_gnu="$here/fakear.bat"
export CC_aarch64_apple_darwin="$here/fakecc.bat" AR_aarch64_apple_darwin="$here/fakear.bat"
cargo fmt --all -- --check
for target in "" "--target x86_64-unknown-linux-gnu" "--target aarch64-apple-darwin"; do
  cargo clippy --workspace --all-targets --locked $target -- -D warnings
done
echo "lint ok on all targets"
