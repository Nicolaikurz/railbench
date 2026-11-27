#!/usr/bin/env bash
set -e

export VIPS_WARNING=0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
REPO_DIR="$PROJECT_ROOT/release/flatpak-repo"
source "$SCRIPT_DIR/release-cleanup.sh"

HOST_ARCH="$(uname -m)"
DEFAULT_TARGET="x64"
if [ "$HOST_ARCH" = "aarch64" ] || [ "$HOST_ARCH" = "arm64" ]; then
    DEFAULT_TARGET="arm64"
fi

TARGET="${1:-$DEFAULT_TARGET}"
VERSION="$(resolve_railbench_version "$PROJECT_ROOT")"

archive_old_binaries "$PROJECT_ROOT/release" "*.flatpak"

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

flatpak --user remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.repo || true
mkdir -p "$REPO_DIR"

build_arch() {
    local RUST_TARGET="$1"  # x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu
    local FLATPAK_ARCH="$2" # x86_64 | aarch64
    local STAGE_DIR="$PROJECT_ROOT/release/flatpak-stage-$FLATPAK_ARCH"
    local BUILD_DIR="$PROJECT_ROOT/build-flatpak-$FLATPAK_ARCH"
    local OUTPUT_BUNDLE="$PROJECT_ROOT/release/railbench-${VERSION}-${FLATPAK_ARCH}.flatpak"

    echo ""
    echo "==========================================="
    echo " Building Native Rust Flatpak for $FLATPAK_ARCH ($RUST_TARGET)"
    echo "==========================================="

    if [ "$HOST_ARCH" != "$FLATPAK_ARCH" ]; then
        if ! ls /proc/sys/fs/binfmt_misc/*qemu*"$FLATPAK_ARCH"* /proc/sys/fs/binfmt_misc/qemu-"$FLATPAK_ARCH"* &> /dev/null; then
            echo "⚠️  Cross-building $FLATPAK_ARCH on a $HOST_ARCH host requires QEMU user-mode binfmt."
            exit 1
        fi
    fi

    echo "=== 2. Building Rust release binary ($RUST_TARGET) ==="
    cd "$PROJECT_ROOT"
    cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"

    rm -rf "$STAGE_DIR"
    mkdir -p "$STAGE_DIR"
    cp "$PROJECT_ROOT/rust/target/$RUST_TARGET/release/railbench-app" "$STAGE_DIR/railbench"
    chmod +x "$STAGE_DIR/railbench"
    if [ "$HOST_ARCH" = "$FLATPAK_ARCH" ] && command -v strip &> /dev/null; then
        strip "$STAGE_DIR/railbench" || true
    fi

    echo ""
    echo "=== 3. Ensuring Freedesktop Flatpak runtime ($FLATPAK_ARCH) is present ==="
    flatpak --user install -y --arch="$FLATPAK_ARCH" flathub \
        "org.freedesktop.Platform/$FLATPAK_ARCH/24.08" \
        "org.freedesktop.Sdk/$FLATPAK_ARCH/24.08" || true

    echo ""
    echo "=== 4. Building Flatpak package ($FLATPAK_ARCH) ==="
    mkdir -p "$BUILD_DIR"
    cd "$PROJECT_ROOT/packaging/flatpak"

    flatpak-builder --force-clean \
        --arch="$FLATPAK_ARCH" \
        --repo="$REPO_DIR" \
        "$BUILD_DIR" \
        io.github.Nicolaikurz.railbench.yml

    echo ""
    echo "=== 5. Creating Standalone Flatpak Bundle ($FLATPAK_ARCH) ==="
    flatpak build-bundle --arch="$FLATPAK_ARCH" "$REPO_DIR" "$OUTPUT_BUNDLE" io.github.Nicolaikurz.railbench

    echo ""
    echo "✅ Flatpak ($FLATPAK_ARCH) ready: $OUTPUT_BUNDLE"
}

case "$TARGET" in
    x64|x86_64)
        build_arch "x86_64-unknown-linux-gnu" "x86_64"
        ;;
    arm64|aarch64)
        build_arch "aarch64-unknown-linux-gnu" "aarch64"
        ;;
    all)
        build_arch "x86_64-unknown-linux-gnu" "x86_64"
        build_arch "aarch64-unknown-linux-gnu" "aarch64"
        ;;
    *)
        echo "Unknown architecture: $TARGET (choose x64, arm64, or all)"
        exit 1
        ;;
esac

cleanup_build_artifacts "$PROJECT_ROOT"

echo ""
echo "==========================================="
echo " Flatpak build complete!"
echo " Bundles located in: $PROJECT_ROOT/release/"
echo " To install: flatpak install --user release/railbench-${VERSION}-<arch>.flatpak"
echo " To run:     flatpak run io.github.Nicolaikurz.railbench"
echo "==========================================="
