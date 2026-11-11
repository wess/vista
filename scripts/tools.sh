#!/usr/bin/env bash
# Build Vista Tools for Linux and macOS guests into target/tools, the folder
# every machine sees as its tools share. Usage: scripts/tools.sh
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
cargo build --release -p vista-tools --target aarch64-unknown-linux-musl
cargo build --release -p vista-tools --target aarch64-apple-darwin
out="target/tools"
rm -rf "$out"
mkdir -p "$out/linux" "$out/macos"
cp target/aarch64-unknown-linux-musl/release/vistatools "$out/linux/vistatools"
cp target/aarch64-apple-darwin/release/vistatools "$out/macos/vistatools"
cp crates/tools/install/install.sh crates/tools/install/install.command "$out/"
chmod +x "$out"/install.sh "$out"/install.command "$out"/*/vistatools
echo "[tools] -> $out"
