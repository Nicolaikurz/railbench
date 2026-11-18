#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

ARCH="${1:-all}"
PRODUCT_NAME=$(node -p "require('./package.json').build?.productName || require('./package.json').name || 'railbench'" 2>/dev/null || echo "railbench")
APP_VERSION=$(node -p "require('./package.json').version || '0.6.1'" 2>/dev/null || echo "0.6.1")

echo "=== Building Windows Portable (Directory & ZIP) ==="
echo "Application: $PRODUCT_NAME v$APP_VERSION"
echo "Target Architecture: $ARCH"

npm run build:all

package_target() {
    local TARGET_ARCH="$1"
    local UNPACKED_DIR="release/win-unpacked"
    if [ "$TARGET_ARCH" = "arm64" ]; then
        UNPACKED_DIR="release/win-arm64-unpacked"
    fi
    local ZIP_OUTPUT="release/${PRODUCT_NAME}-${APP_VERSION}-portable-${TARGET_ARCH}.zip"

    echo ""
    echo "• Packaging Windows Portable Directory for ${TARGET_ARCH}..."
    npx electron-builder --win dir --${TARGET_ARCH}

    if [ -d "$UNPACKED_DIR" ]; then
        echo "• Creating standalone portable ZIP from $UNPACKED_DIR..."
        rm -f "$ZIP_OUTPUT"
        if command -v zip &> /dev/null; then
            (cd "$UNPACKED_DIR" && zip -qry "../../$ZIP_OUTPUT" .)
            local SIZE
            SIZE=$(du -h "$ZIP_OUTPUT" | awk '{print $1}')
            echo "✓ Created Portable ZIP: $ZIP_OUTPUT ($SIZE)"
        else
            echo "⚠️  'zip' command not found. Unpacked directory is ready in $UNPACKED_DIR"
        fi
    else
        echo "⚠️  Directory $UNPACKED_DIR was not generated."
    fi
}

if [ "$ARCH" = "x64" ]; then
    package_target "x64"
elif [ "$ARCH" = "arm64" ]; then
    package_target "arm64"
elif [ "$ARCH" = "all" ]; then
    package_target "x64"
    package_target "arm64"
else
    echo "Unknown architecture: $ARCH (choose x64, arm64, or all)"
    exit 1
fi

echo ""
echo "==========================================="
echo " Windows Portable build complete!"
echo "Directories:"
echo "  • x64:   release/win-unpacked/railbench.exe"
echo "  • arm64: release/win-arm64-unpacked/railbench.exe"
echo "ZIP Archives: release/${PRODUCT_NAME}-${APP_VERSION}-portable-*.zip"
echo "==========================================="
