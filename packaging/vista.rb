cask "vista" do
  version "VERSION_PLACEHOLDER"
  sha256 "SHA_PLACEHOLDER"

  url "https://github.com/wess/vista/releases/download/v#{version}/Vista.dmg"
  name "Vista"
  desc "Native virtual machines for Apple silicon"
  homepage "https://wess.github.io/vista/"

  depends_on arch: :arm64
  depends_on macos: :sequoia

  app "Vista.app"
  binary "#{appdir}/Vista.app/Contents/MacOS/vistamcp"

  zap trash: [
    "~/Library/Application Support/Vista",
    "~/Library/Preferences/io.wess.vista.plist",
    "~/Library/Saved Application State/io.wess.vista.savedState",
  ]
end
