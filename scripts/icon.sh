#!/usr/bin/env bash
# Render assets/icon.icns from scripts/icon.swift. macOS only.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
set="$work/icon.iconset"
mkdir -p "$set"
for size in 16 32 128 256 512; do
  swift "$root/scripts/icon.swift" "$set/icon_${size}x${size}.png" "$size"
  swift "$root/scripts/icon.swift" "$set/icon_${size}x${size}@2x.png" "$((size * 2))"
done
iconutil -c icns "$set" -o "$root/assets/icon.icns"
swift "$root/scripts/icon.swift" "$root/assets/icon.png" 1024
rm -rf "$work"
echo "[icon] -> assets/icon.icns"
