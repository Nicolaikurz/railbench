cask "railbench" do
  arch arm: "arm64", intel: "x64"

  version "0.6.0"
  sha256 arm:   "deb239fc0f14892108aa136501bfd05f6d9b16601cf8c59f54e660de3dcfc69e",
         intel: "4226a712ecc2e5d419e5f40a70658ad24c8c611d7afcf3ba30f83a7f3045669b"

  url "https://github.com/Nicolaikurz/railbench/releases/download/v#{version}/railbench-#{version}-mac-#{arch}.dmg"
  name "RAILbench"
  desc "Model railway track planning and layout simulation tool"
  homepage "https://github.com/Nicolaikurz/railbench"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates false
  depends_on macos: ">= :catalina"

  app "railbench.app"

  zap trash: [
    "~/Library/Application Support/railbench",
    "~/Library/Preferences/de.railbench.app.plist",
    "~/Library/Saved Application State/de.railbench.app.savedState",
  ]
end
