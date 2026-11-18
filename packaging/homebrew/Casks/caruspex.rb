# Template for a Homebrew tap, filled in by tools/dist_manifests.py (see packaging/README.md).
cask "caruspex" do
  version "@VERSION@"
  sha256 "@SHA256_MACOS_DMG@"

  url "https://github.com/SenseiIssei/Caruspex/releases/download/v#{version}/Caruspex-#{version}-macos.dmg"
  name "Caruspex"
  desc "Free car diagnostics over OBD-II, K-Line, CAN and Ethernet"
  homepage "https://github.com/SenseiIssei/Caruspex"

  livecheck do
    url :url
    strategy :github_latest
  end

  # The app updates itself after checking the signature of the new version.
  auto_updates true
  depends_on arch: :arm64
  depends_on macos: ">= :ventura"

  app "Caruspex.app"

  zap trash: [
    "~/Library/Application Support/Caruspex",
    "~/Library/Application Support/dev.senseiissei.caruspex",
    "~/Library/Caches/dev.senseiissei.caruspex",
    "~/Library/Saved Application State/dev.senseiissei.caruspex.savedState",
    "~/Library/WebKit/dev.senseiissei.caruspex",
  ]
end
