#!/usr/bin/env bash
# Pack dist/Vista.app into dist/Vista.dmg. Run scripts/bundle.sh first.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
[ -d dist/Vista.app ] || { echo "error: run scripts/bundle.sh first" >&2; exit 1; }
stage="$(mktemp -d)"
cp -R dist/Vista.app "$stage/"
ln -s /Applications "$stage/Applications"
rm -f dist/Vista.dmg
hdiutil create -volname Vista -srcfolder "$stage" -ov -format UDZO dist/Vista.dmg >/dev/null
rm -rf "$stage"
if [ -z "${CODESIGN_IDENTITY:-}" ]; then
  CODESIGN_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)"
  CODESIGN_IDENTITY="${CODESIGN_IDENTITY:--}"
fi
if [ "$CODESIGN_IDENTITY" != "-" ]; then
  codesign --force --timestamp -s "$CODESIGN_IDENTITY" dist/Vista.dmg
fi
echo "[dmg] -> dist/Vista.dmg"
