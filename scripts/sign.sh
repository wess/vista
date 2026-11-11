#!/usr/bin/env bash
# Sign the debug binaries with the virtualization entitlement so they can run
# machines straight out of target/. Usage: scripts/sign.sh [debug|release]
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
profile="${1:-debug}"
for bin in vistamachine vista vistamcp; do
  path="$root/target/$profile/$bin"
  [ -f "$path" ] || continue
  codesign --force --entitlements "$root/assets/vista.entitlements" -s - "$path" 2>/dev/null
  echo "signed $path"
done
