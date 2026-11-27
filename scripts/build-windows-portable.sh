#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

ARCH="${1:-x64}"
VERSION="$(resolve_railbench_version "$PROJECT_ROOT")"
HOST_OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
IS_WINDOWS=false
case "$HOST_OS" in
    mingw*|msys*|cygwin*|windows*) IS_WINDOWS=true ;;
esac

echo "=== Building Native Rust Windows Portable (ZIP) ==="
echo "Application        : railbench v${VERSION}"
echo "Target Architecture: $ARCH"

archive_old_binaries "$PROJECT_ROOT/release" "*-win-*.zip" "*-portable-*.exe"

package_target() {
    local TARGET_ARCH="$1" # x64 | arm64
    local RUST_TARGET=""
    local FOLDER_NAME="railbench-${VERSION}-win-${TARGET_ARCH}"
    local STAGE_DIR="$PROJECT_ROOT/release/win-stage-${TARGET_ARCH}/${FOLDER_NAME}"
    local ZIP_OUTPUT="$PROJECT_ROOT/release/${FOLDER_NAME}.zip"

    case "$TARGET_ARCH" in
        x64) RUST_TARGET="x86_64-pc-windows-msvc" ;;
        arm64) RUST_TARGET="aarch64-pc-windows-msvc" ;;
        *)
            echo "Unknown architecture: $TARGET_ARCH"
            return 1
            ;;
    esac

    echo ""
    echo "• Compiling Windows release binary for ${TARGET_ARCH}..."
    if [ "$IS_WINDOWS" = true ]; then
        cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"
    elif command -v cargo-xwin &> /dev/null; then
        cargo xwin build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"
    elif [ "$TARGET_ARCH" = "x64" ] && command -v x86_64-w64-mingw32-gcc &> /dev/null; then
        RUST_TARGET="x86_64-pc-windows-gnu"
        cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"
    else
        echo "⚠️  Cross-compiling Windows executables from $HOST_OS requires 'cargo-xwin' (or MinGW-w64 for x64 GNU)."
        echo ""
        echo "Install cargo-xwin:"
        echo "  cargo install cargo-xwin"
        echo "  rustup target add $RUST_TARGET"
        return 1
    fi

    local EXE_SRC="$PROJECT_ROOT/rust/target/$RUST_TARGET/release/railbench-app.exe"
    if [ ! -f "$EXE_SRC" ]; then
        echo "⚠️  Expected binary not found: $EXE_SRC"
        return 1
    fi

    rm -rf "$PROJECT_ROOT/release/win-stage-${TARGET_ARCH}" "$ZIP_OUTPUT"
    mkdir -p "$STAGE_DIR"
    cp "$EXE_SRC" "$STAGE_DIR/railbench.exe"
    [ -f "$PROJECT_ROOT/LICENSE" ] && cp "$PROJECT_ROOT/LICENSE" "$STAGE_DIR/LICENSE"

    echo "• Creating standalone portable ZIP: $ZIP_OUTPUT"
    if command -v zip &> /dev/null; then
        (cd "$PROJECT_ROOT/release/win-stage-${TARGET_ARCH}" && zip -qry "$ZIP_OUTPUT" "${FOLDER_NAME}")
    elif command -v python3 &> /dev/null; then
        python3 -c "import shutil; shutil.make_archive('${PROJECT_ROOT}/release/${FOLDER_NAME}', 'zip', '${PROJECT_ROOT}/release/win-stage-${TARGET_ARCH}', '${FOLDER_NAME}')"
    elif command -v powershell.exe &> /dev/null; then
        powershell.exe -NoProfile -Command "Compress-Archive -Path '${STAGE_DIR}' -DestinationPath '${ZIP_OUTPUT}' -Force"
    else
        echo "⚠️  No zip tool found (zip, python3, or powershell.exe). Staged binary is in $STAGE_DIR"
        return 1
    fi

    local SIZE
    SIZE="$(du -h "$ZIP_OUTPUT" | awk '{print $1}')"
    echo "✓ Created Portable ZIP: $ZIP_OUTPUT ($SIZE)"
    rm -rf "$PROJECT_ROOT/release/win-stage-${TARGET_ARCH}"
}

case "$ARCH" in
    x64)
        package_target "x64"
        ;;
    arm64)
        package_target "arm64"
        ;;
    all)
        package_target "x64"
        package_target "arm64"
        ;;
    *)
        echo "Unknown architecture: $ARCH (choose x64, arm64, or all)"
        exit 1
        ;;
esac

cleanup_build_artifacts "$PROJECT_ROOT"

echo ""
echo "==========================================="
echo " Windows Portable build complete!"
echo " Output files in: release/"
echo "==========================================="
