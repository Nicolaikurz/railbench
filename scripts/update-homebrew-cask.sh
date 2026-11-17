#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CASK_FILE="$PROJECT_ROOT/packaging/homebrew/Casks/railbench.rb"

cd "$PROJECT_ROOT"

APP_VERSION="${1:-$(node -p "require('./package.json').version" 2>/dev/null || echo "0.6.0")}"
echo "=== Updating Homebrew Cask for RAILbench v${APP_VERSION} ==="

ARM64_DMG="$PROJECT_ROOT/release/railbench-${APP_VERSION}-mac-arm64.dmg"
X64_DMG="$PROJECT_ROOT/release/railbench-${APP_VERSION}-mac-x64.dmg"

# Compute SHA256 if local release DMGs exist, or fetch from GitHub Release if available
if [ -f "$ARM64_DMG" ]; then
    echo "• Calculating SHA-256 for local arm64 DMG: $ARM64_DMG"
    SHA256_ARM64=$(sha256sum "$ARM64_DMG" | awk '{print $1}')
else
    echo "⚠️  Local $ARM64_DMG not found. Attempting to download checksum from GitHub..."
    URL_ARM64="https://github.com/Nicolaikurz/railbench/releases/download/v${APP_VERSION}/railbench-${APP_VERSION}-mac-arm64.dmg"
    SHA256_ARM64=$(curl -sL "$URL_ARM64" | sha256sum | awk '{print $1}' || echo "__REPLACE_WITH_SHA256_ARM64__")
fi

if [ -f "$X64_DMG" ]; then
    echo "• Calculating SHA-256 for local x64 DMG: $X64_DMG"
    SHA256_X64=$(sha256sum "$X64_DMG" | awk '{print $1}')
else
    echo "⚠️  Local $X64_DMG not found. Attempting to download checksum from GitHub..."
    URL_X64="https://github.com/Nicolaikurz/railbench/releases/download/v${APP_VERSION}/railbench-${APP_VERSION}-mac-x64.dmg"
    SHA256_X64=$(curl -sL "$URL_X64" | sha256sum | awk '{print $1}' || echo "__REPLACE_WITH_SHA256_X64__")
fi

echo "  arm64 sha256: $SHA256_ARM64"
echo "  x64   sha256: $SHA256_X64"

mkdir -p "$PROJECT_ROOT/packaging/homebrew/Casks"

cat > "$CASK_FILE" <<EOF
cask "railbench" do
  arch arm: "arm64", intel: "x64"

  version "${APP_VERSION}"
  sha256 arm:   "${SHA256_ARM64}",
         intel: "${SHA256_X64}"

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
EOF

echo "✓ Updated Cask file: $CASK_FILE"
