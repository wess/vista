#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
"$root/scripts/firmware.sh"
tool="$(command -v wimlib-imagex || true)"
[ -n "$tool" ] || { echo "Windows packaging requires wimlib (brew install wimlib)." >&2; exit 1; }
library="$(otool -L "$tool" | awk '/libwim.*dylib/ {print $1; exit}')"
[ -f "$library" ] || { echo "Cannot locate libwim." >&2; exit 1; }
out="$root/target/windows"
mkdir -p "$out"
cp -L "$tool" "$out/imagex"
cp -L "$library" "$out/libwim.dylib"
chmod u+w "$out/imagex" "$out/libwim.dylib"
install_name_tool -change "$library" '@loader_path/libwim.dylib' "$out/imagex"
install_name_tool -id '@loader_path/libwim.dylib' "$out/libwim.dylib"
prefix="$(dirname "$(dirname "$library")")"
cp "$prefix/COPYING" "$out/license"
cp "$prefix/COPYING.GPLv3" "$out/gpl"
cp "$prefix/COPYING.LGPL" "$out/lgpl"
cat > "$out/source.txt" <<'SOURCE'
wimlib 1.14.5: https://wimlib.net/downloads/wimlib-1.14.5.tar.gz
wimlib-imagex is a separate GPLv3 program. libwim is LGPLv3 or GPLv2.
The source archive above includes corresponding source and build instructions.
Homebrew build recipe: https://github.com/Homebrew/homebrew-core/blob/master/Formula/w/wimlib.rb
SOURCE
for artifact in "$out/imagex" "$out/libwim.dylib"; do
  if otool -L "$artifact" | awk 'NR > 1 {print $1}' | grep -E '^(/opt/|/usr/local/)' >/dev/null; then
    echo "Unbundled dependency in $artifact" >&2; exit 1
  fi
  codesign --force -s - "$artifact"
done
"$out/imagex" --version
