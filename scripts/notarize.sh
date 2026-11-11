#!/usr/bin/env bash
# Submit a signed app or disk image, staple its ticket, and verify the result.
# Store credentials once with `xcrun notarytool store-credentials vista`.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

profile="${NOTARY_PROFILE:-vista}"
target="${1:-dist/Vista.app}"
[ -e "$target" ] || { echo "error: $target not found" >&2; exit 1; }
if codesign -dvv "$target" 2>&1 | grep -q "Signature=adhoc"; then
  echo "error: $target is ad-hoc signed" >&2
  exit 1
fi

upload="$target"
temporary=""
if [ -d "$target" ]; then
  temporary="$(mktemp -d)"
  upload="$temporary/$(basename "$target").zip"
  ditto -c -k --keepParent "$target" "$upload"
fi

xcrun notarytool submit "$upload" --keychain-profile "$profile" --wait
[ -n "$temporary" ] && rm -rf "$temporary"
xcrun stapler staple "$target"
xcrun stapler validate "$target"

if [ -d "$target" ]; then
  spctl -a -vvv --type execute "$target"
else
  mount="$(mktemp -d)"
  hdiutil attach -nobrowse -quiet "$target" -mountpoint "$mount"
  trap 'hdiutil detach "$mount" -quiet >/dev/null 2>&1 || true; rm -rf "$mount"' EXIT
  spctl -a -vvv --type execute "$mount/Vista.app"
  xcrun stapler validate "$mount/Vista.app"
fi
