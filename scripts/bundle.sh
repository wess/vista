#!/usr/bin/env bash
# Build Vista (release) and assemble dist/Vista.app:
#
#   Vista.app/Contents/
#     MacOS/vista                  the library window and menu bar item
#     MacOS/vistamcp               the MCP server agents talk to
#     Helpers/VistaMachine.app/    one process per running machine
#       Contents/MacOS/vistamachine
#       Contents/Resources/tools/  Vista Tools, shared into every guest
#
# Signs with CODESIGN_IDENTITY if set (a Developer ID for a notarizable
# build), otherwise ad-hoc so it runs on this Mac.
# Usage: scripts/bundle.sh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

identity="${CODESIGN_IDENTITY:--}"
version="$(sed -n 's/^version = "\([0-9][^"]*\)".*/\1/p' Cargo.toml | head -1)"
plist_version="${version%%-*}"
echo "[bundle] Vista $version"

[ -f assets/icon.icns ] || scripts/icon.sh

cargo build --release -p vista-app -p vista-mcp -p vista-machine
scripts/tools.sh
scripts/windows.sh

app="dist/Vista.app"
helper="$app/Contents/Helpers/VistaMachine.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$helper/Contents/MacOS" "$helper/Contents/Resources"

cp target/release/vista "$app/Contents/MacOS/vista"
cp target/release/vistamcp "$app/Contents/MacOS/vistamcp"
cp assets/icon.icns "$app/Contents/Resources/icon.icns"
cp -R target/windows "$app/Contents/Resources/windows"
cp target/release/vistamachine "$helper/Contents/MacOS/vistamachine"
cp assets/icon.icns "$helper/Contents/Resources/icon.icns"
cp -R target/tools "$helper/Contents/Resources/tools"

plist() {
  # plist <path> <name> <executable> <identifier> <extra keys>
  cat > "$1" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>$2</string>
	<key>CFBundleDisplayName</key><string>$2</string>
	<key>CFBundleExecutable</key><string>$3</string>
	<key>CFBundleIdentifier</key><string>$4</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleVersion</key><string>$plist_version</string>
	<key>CFBundleShortVersionString</key><string>$plist_version</string>
	<key>CFBundleIconFile</key><string>icon</string>
	<key>LSMinimumSystemVersion</key><string>15.0</string>
	<key>LSArchitecturePriority</key><array><string>arm64</string></array>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSSupportsAutomaticGraphicsSwitching</key><true/>
$5
</dict>
</plist>
PLIST
}

plist "$app/Contents/Info.plist" "Vista" "vista" "io.wess.vista" \
  "	<key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>"
plist "$helper/Contents/Info.plist" "Vista Machine" "vistamachine" "io.wess.vista.machine" \
  "	<key>NSMicrophoneUsageDescription</key><string>A virtual machine you gave the microphone to is listening.</string>"

echo "[bundle] codesign ($identity)"
runtime=()
[ "$identity" != "-" ] && runtime=(--options runtime --timestamp)
sign() {
  codesign --force ${runtime[@]+"${runtime[@]}"} --entitlements assets/vista.entitlements -s "$identity" "$1"
}
codesign --force ${runtime[@]+"${runtime[@]}"} -s "$identity" "$app/Contents/Resources/windows/libwim.dylib"
codesign --force ${runtime[@]+"${runtime[@]}"} -s "$identity" "$app/Contents/Resources/windows/imagex"
sign "$helper/Contents/Resources/tools/macos/vistatools"
sign "$helper/Contents/MacOS/vistamachine"
sign "$helper"
sign "$app/Contents/MacOS/vistamcp"
sign "$app/Contents/MacOS/vista"
sign "$app"
codesign --verify --strict --verbose=2 "$app" || true
echo "[bundle] -> $app"
