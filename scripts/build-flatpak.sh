#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build-flatpak"
REPO_DIR="$PROJECT_ROOT/release/flatpak-repo"
OUTPUT_BUNDLE="$PROJECT_ROOT/release/Railbench.flatpak"

echo "=== 1. Building Linux unpacked directory ==="
cd "$PROJECT_ROOT"
npm run build:all
npx electron-builder --linux dir

echo ""
echo "=== 2. Checking Flatpak tools ==="
if ! command -v flatpak-builder &> /dev/null; then
    echo "⚠️  flatpak-builder is not installed."
    echo ""
    echo "To install flatpak-builder:"
    echo "  • Arch Linux / Manjaro: sudo pacman -S flatpak-builder"
    echo "  • Ubuntu / Debian:      sudo apt install flatpak-builder"
    echo "  • Fedora:               sudo dnf install flatpak-builder"
    exit 1
fi

echo "=== 3. Ensuring Freedesktop Flatpak runtime is present ==="
flatpak --user remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.repo || true
flatpak --user install -y flathub org.freedesktop.Platform//24.08 org.freedesktop.Sdk//24.08 org.electronjs.Electron2.BaseApp//24.08 || true

echo "=== 4. Building Flatpak package ==="
mkdir -p "$BUILD_DIR"
mkdir -p "$REPO_DIR"

cd "$PROJECT_ROOT/packaging/flatpak"

flatpak-builder --force-clean \
    --repo="$REPO_DIR" \
    "$BUILD_DIR" \
    de.railbench.Railbench.yml

echo ""
echo "=== 5. Creating Standalone Flatpak Bundle (.flatpak) ==="
flatpak build-bundle "$REPO_DIR" "$OUTPUT_BUNDLE" de.railbench.Railbench

echo ""
echo "==========================================="
echo "✅ Flatpak build complete!"
echo "Bundle file: $OUTPUT_BUNDLE"
echo "To install: flatpak install --user $OUTPUT_BUNDLE"
echo "To run:     flatpak run de.railbench.Railbench"
echo "==========================================="

