#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

HOST_ARCH="$(uname -m)"
DEFAULT_TARGET="x64"
if [ "$HOST_ARCH" = "aarch64" ] || [ "$HOST_ARCH" = "arm64" ]; then
    DEFAULT_TARGET="arm64"
fi

TARGET="${1:-$DEFAULT_TARGET}"
VERSION="$(resolve_railbench_version "$PROJECT_ROOT")"

if ! command -v snapcraft &> /dev/null; then
    echo "⚠️  snapcraft CLI is required to build Snap packages."
    echo "Install snapcraft: sudo snap install snapcraft --classic"
    exit 1
fi

echo "=== Building Native Rust Snap Package (RAILbench v${VERSION}) ==="
echo "Target Architecture: $TARGET"

archive_old_binaries "$PROJECT_ROOT/release" "*.snap"

build_snap_arch() {
    local ARCH_LABEL="$1"     # amd64 | arm64
    local RUST_TARGET="$2"    # x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu
    local STAGE_DIR="$PROJECT_ROOT/release/snap-stage-${ARCH_LABEL}"

    echo ""
    echo "• Compiling Rust release binary ($RUST_TARGET)..."
    cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"

    rm -rf "$STAGE_DIR" "$PROJECT_ROOT/snap"
    mkdir -p "$STAGE_DIR/bin" "$STAGE_DIR/share/applications" "$STAGE_DIR/share/icons/hicolor/512x512/apps" "$PROJECT_ROOT/snap"
    cp "$PROJECT_ROOT/rust/target/$RUST_TARGET/release/railbench-app" "$STAGE_DIR/bin/railbench"
    chmod +x "$STAGE_DIR/bin/railbench"
    [ -f "$PROJECT_ROOT/build/icon.png" ] && cp "$PROJECT_ROOT/build/icon.png" "$STAGE_DIR/share/icons/hicolor/512x512/apps/railbench.png"

    cat > "$PROJECT_ROOT/snap/snapcraft.yaml" <<EOF
name: railbench
base: core24
version: '${VERSION}'
summary: Native Rust Model Railway Track Planner
description: |
  Intuitive, hardware-accelerated 2D & 3D model railway CAD track planner,
  terrain editor, train cab simulator, and electrical traction voltage simulator.
grade: stable
confinement: strict

apps:
  railbench:
    command: bin/railbench
    plugs:
      - desktop
      - desktop-legacy
      - wayland
      - x11
      - opengl
      - network
      - network-bind
      - home
      - removable-media

parts:
  railbench:
    plugin: dump
    source: release/snap-stage-${ARCH_LABEL}
EOF

    snapcraft pack --output "$PROJECT_ROOT/release/railbench_${VERSION}_${ARCH_LABEL}.snap"
    rm -rf "$STAGE_DIR" "$PROJECT_ROOT/snap"
}

case "$TARGET" in
    x64|amd64|x86_64) build_snap_arch "amd64" "x86_64-unknown-linux-gnu" ;;
    arm64|aarch64) build_snap_arch "arm64" "aarch64-unknown-linux-gnu" ;;
    all)
        build_snap_arch "amd64" "x86_64-unknown-linux-gnu"
        build_snap_arch "arm64" "aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Unknown architecture: $TARGET (choose x64, arm64, or all)"
        exit 1
        ;;
esac

cleanup_build_artifacts "$PROJECT_ROOT"

echo ""
echo "==========================================="
echo " Snap build complete!"
echo " Output files in: release/"
echo "==========================================="
