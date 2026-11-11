#!/bin/sh
# Install Vista Tools in a macOS guest. Double-click this file in
# "/Volumes/My Shared Files/Vista Tools"; it asks for your password.
set -e
here="$(cd "$(dirname "$0")" && pwd)"
echo "Installing Vista Tools (your password is needed once)…"
sudo install -d /usr/local/bin
sudo cp "$here/macos/vistatools" /usr/local/bin/vistatools
sudo chmod 755 /usr/local/bin/vistatools
sudo tee /Library/LaunchDaemons/io.wess.vista.tools.plist >/dev/null <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>io.wess.vista.tools</string>
  <key>ProgramArguments</key><array><string>/usr/local/bin/vistatools</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardErrorPath</key><string>/var/log/vistatools.log</string>
</dict>
</plist>
PLIST
sudo launchctl bootout system/io.wess.vista.tools 2>/dev/null || true
sudo launchctl bootstrap system /Library/LaunchDaemons/io.wess.vista.tools.plist
echo "Vista Tools is installed and running. You can close this window."
