#!/usr/bin/env bash
set -e

# Suppress harmless libvips optional module warnings (e.g. missing libopenslide.so.1 on Arch Linux)
export VIPS_WARNING=0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
REPO_DIR="$PROJECT_ROOT/release/flatpak-repo"

TARGET="${1:-x64}"
HOST_ARCH="$(uname -m)"
VERSION="$(node -p "require('$PROJECT_ROOT/package.json').version")"

echo "=== 1. Checking Flatpak tools ==="
if ! command -v flatpak-builder &> /dev/null; then
    echo "⚠️  flatpak-builder is not installed."
    echo ""
    echo "To install flatpak-builder:"
    echo "  • Arch Linux / Manjaro: sudo pacman -S flatpak-builder"
    echo "  • Ubuntu / Debian:      sudo apt install flatpak-builder"
    echo "  • Fedora:               sudo dnf install flatpak-builder"
    exit 1
fi

echo "=== 2. Building app bundle (Vite + Electron) ==="
cd "$PROJECT_ROOT"
npm run build:all

flatpak --user remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.repo || true
mkdir -p "$REPO_DIR"

build_arch() {
    local EB_ARCH="$1"      # x64 | arm64
    local FLATPAK_ARCH="$2" # x86_64 | aarch64
    local BUILD_DIR="$PROJECT_ROOT/build-flatpak-$FLATPAK_ARCH"
    local OUTPUT_BUNDLE="$PROJECT_ROOT/release/railbench-${VERSION}-${FLATPAK_ARCH}.flatpak"

    echo ""
    echo "==========================================="
    echo " Building Flatpak for $FLATPAK_ARCH ($EB_ARCH)"
    echo "==========================================="

    if [ "$HOST_ARCH" != "$FLATPAK_ARCH" ]; then
        if ! ls /proc/sys/fs/binfmt_misc/*qemu*"$FLATPAK_ARCH"* /proc/sys/fs/binfmt_misc/qemu-"$FLATPAK_ARCH"* &> /dev/null; then
            echo "⚠️  Cross-building $FLATPAK_ARCH on a $HOST_ARCH host requires QEMU user-mode binfmt."
            echo ""
            echo "Install QEMU binfmt support:"
            echo "  • Arch Linux / Manjaro: sudo pacman -S qemu-user-static qemu-user-static-binfmt && sudo systemctl restart systemd-binfmt"
            echo "  • Ubuntu / Debian:      sudo apt install qemu-user-static binfmt-support"
            echo "  • Fedora:               sudo dnf install qemu-user-static"
            exit 1
        fi
    fi

    echo "=== Building Linux unpacked directory ($EB_ARCH) ==="
    cd "$PROJECT_ROOT"
    npx electron-builder --linux dir --"$EB_ARCH"

    echo ""
    echo "=== Ensuring Freedesktop Flatpak runtime ($FLATPAK_ARCH) is present ==="
    flatpak --user install -y --arch="$FLATPAK_ARCH" flathub \
        "org.freedesktop.Platform/$FLATPAK_ARCH/24.08" \
        "org.freedesktop.Sdk/$FLATPAK_ARCH/24.08" \
        "org.electronjs.Electron2.BaseApp/$FLATPAK_ARCH/24.08" || true

    echo ""
    echo "=== Building Flatpak package ($FLATPAK_ARCH) ==="
    mkdir -p "$BUILD_DIR"
    cd "$PROJECT_ROOT/packaging/flatpak"

    flatpak-builder --force-clean \
        --arch="$FLATPAK_ARCH" \
        --repo="$REPO_DIR" \
        "$BUILD_DIR" \
        io.github.Nicolaikurz.railbench.yml

    echo ""
    echo "=== Creating Standalone Flatpak Bundle ($FLATPAK_ARCH) ==="
    flatpak build-bundle --arch="$FLATPAK_ARCH" "$REPO_DIR" "$OUTPUT_BUNDLE" io.github.Nicolaikurz.railbench

    echo ""
    echo "✅ Flatpak ($FLATPAK_ARCH) ready: $OUTPUT_BUNDLE"
}

case "$TARGET" in
    x64|x86_64)
        build_arch x64 x86_64
        ;;
    arm64|aarch64)
        build_arch arm64 aarch64
        ;;
    all)
        build_arch x64 x86_64
        build_arch arm64 aarch64
        ;;
    *)
        echo "Unknown architecture: $TARGET (choose x64, arm64, or all)"
        exit 1
        ;;
esac

echo ""
echo "==========================================="
echo " Flatpak build complete!"
echo "Bundles located in: $PROJECT_ROOT/release/"
echo "To install: flatpak install --user release/railbench-${VERSION}-<arch>.flatpak"
echo "To run:     flatpak run io.github.Nicolaikurz.railbench"
echo "==========================================="
