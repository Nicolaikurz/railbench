#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

HOST_ARCH="$(uname -m)"
DEFAULT_ARCH="x64"
if [ "$HOST_ARCH" = "aarch64" ] || [ "$HOST_ARCH" = "arm64" ]; then
    DEFAULT_ARCH="arm64"
fi

ARCH="${1:-$DEFAULT_ARCH}"
VERSION="$(resolve_railbench_version "$PROJECT_ROOT")"

find_appimagetool() {
    if [ -n "${APPIMAGETOOL:-}" ] && [ -x "$APPIMAGETOOL" ]; then
        echo "$APPIMAGETOOL"
    elif command -v appimagetool &> /dev/null; then
        command -v appimagetool
    elif [ -x "$HOME/.local/bin/appimagetool" ]; then
        echo "$HOME/.local/bin/appimagetool"
    elif [ -x "$HOME/.local/bin/appimagetool-x86_64.AppImage" ]; then
        echo "$HOME/.local/bin/appimagetool-x86_64.AppImage"
    else
        echo ""
    fi
}

APPIMAGETOOL_BIN="$(find_appimagetool)"
if [ -z "$APPIMAGETOOL_BIN" ]; then
    echo "⚠️  appimagetool not found."
    echo ""
    echo "Download appimagetool and place it on your PATH (or ~/.local/bin/appimagetool):"
    echo "  curl -fsSL -o ~/.local/bin/appimagetool https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$(uname -m).AppImage"
    echo "  chmod +x ~/.local/bin/appimagetool"
    exit 1
fi

echo "=== Building Native Rust Linux AppImage (RAILbench v${VERSION}) ==="
echo "Target Architecture : $ARCH"
echo "appimagetool        : $APPIMAGETOOL_BIN"

archive_old_binaries "$PROJECT_ROOT/release" "*.AppImage"

build_appimage_arch() {
    local ARCH_LABEL="$1"     # x64 | arm64
    local RUST_TARGET="$2"    # x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu
    local APPIMAGE_ARCH="$3"  # x86_64 | aarch64
    local APPDIR="$PROJECT_ROOT/release/AppDir-${ARCH_LABEL}"
    local OUT_APPIMAGE="$PROJECT_ROOT/release/railbench-${VERSION}-${ARCH_LABEL}.AppImage"

    echo ""
    echo "• Compiling Rust release binary ($RUST_TARGET)..."
    cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"
    local BIN_PATH="$PROJECT_ROOT/rust/target/$RUST_TARGET/release/railbench-app"

    rm -rf "$APPDIR"
    mkdir -p \
        "$APPDIR/usr/bin" \
        "$APPDIR/usr/share/applications" \
        "$APPDIR/usr/share/icons/hicolor/512x512/apps"

    cp "$BIN_PATH" "$APPDIR/usr/bin/railbench"
    chmod +x "$APPDIR/usr/bin/railbench"
    if [ "$HOST_ARCH" = "$APPIMAGE_ARCH" ] && command -v strip &> /dev/null; then
        strip "$APPDIR/usr/bin/railbench" || true
    fi

    cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/railbench" "$@"
EOF
    chmod +x "$APPDIR/AppRun"

    cat > "$APPDIR/railbench.desktop" <<'EOF'
[Desktop Entry]
Name=Railbench
GenericName=Model Railway Track Planner
Comment=Design and simulate model railway layouts with authentic track geometries
Exec=railbench %U
Icon=railbench
Type=Application
Terminal=false
Categories=Graphics;2DGraphics;Engineering;
StartupWMClass=railbench-app
EOF
    cp "$APPDIR/railbench.desktop" "$APPDIR/usr/share/applications/railbench.desktop"

    if [ -f "$PROJECT_ROOT/build/icon.png" ]; then
        cp "$PROJECT_ROOT/build/icon.png" "$APPDIR/railbench.png"
        cp "$PROJECT_ROOT/build/icon.png" "$APPDIR/usr/share/icons/hicolor/512x512/apps/railbench.png"
    fi

    echo "• Packaging AppImage ($APPIMAGE_ARCH) -> $OUT_APPIMAGE"
    mkdir -p "$PROJECT_ROOT/release"
    ARCH="$APPIMAGE_ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL_BIN" "$APPDIR" "$OUT_APPIMAGE"
    chmod +x "$OUT_APPIMAGE"
    rm -rf "$APPDIR"

    local SIZE
    SIZE="$(du -h "$OUT_APPIMAGE" | awk '{print $1}')"
    echo "✓ Created AppImage: $OUT_APPIMAGE ($SIZE)"
}

case "$ARCH" in
    x64|x86_64)
        build_appimage_arch "x64" "x86_64-unknown-linux-gnu" "x86_64"
        ;;
    arm64|aarch64)
        build_appimage_arch "arm64" "aarch64-unknown-linux-gnu" "aarch64"
        ;;
    all)
        build_appimage_arch "x64" "x86_64-unknown-linux-gnu" "x86_64"
        build_appimage_arch "arm64" "aarch64-unknown-linux-gnu" "aarch64"
        ;;
    *)
        echo "Unknown architecture: $ARCH (choose x64, arm64, or all)"
        exit 1
        ;;
esac

cleanup_build_artifacts "$PROJECT_ROOT"

echo ""
echo "==========================================="
echo " Linux AppImage build complete!"
echo " AppImages located in: release/"
echo "==========================================="
