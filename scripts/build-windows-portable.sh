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
    local FOLDER_NAME="${PRODUCT_NAME}-${APP_VERSION}-win-${TARGET_ARCH}"
    local STAGED_DIR="release/${FOLDER_NAME}"
    local ZIP_OUTPUT="release/${FOLDER_NAME}.zip"

    echo ""
    echo "• Packaging Windows Portable Directory for ${TARGET_ARCH}..."
    npx electron-builder --win dir --${TARGET_ARCH}

    if [ -d "$UNPACKED_DIR" ]; then
        echo "• Staging folder with architecture badge: $STAGED_DIR"
        rm -rf "$STAGED_DIR" "$ZIP_OUTPUT"
        mv "$UNPACKED_DIR" "$STAGED_DIR"

        echo "• Creating standalone portable ZIP: $ZIP_OUTPUT"
        if command -v zip &> /dev/null; then
            (cd release && zip -qry "${FOLDER_NAME}.zip" "${FOLDER_NAME}")
        elif command -v python3 &> /dev/null; then
            python3 -c "import shutil; shutil.make_archive('release/${FOLDER_NAME}', 'zip', 'release', '${FOLDER_NAME}')"
        elif command -v powershell.exe &> /dev/null; then
            powershell.exe -NoProfile -Command "Compress-Archive -Path 'release/${FOLDER_NAME}' -DestinationPath 'release/${FOLDER_NAME}.zip' -Force"
        else
            echo "⚠️  No zip tool found (zip, python3, or powershell.exe). Staged directory is ready in $STAGED_DIR"
            return
        fi

        local SIZE
        SIZE=$(du -h "$ZIP_OUTPUT" | awk '{print $1}')
        echo "✓ Created Portable ZIP: $ZIP_OUTPUT ($SIZE)"
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
echo "Folders:"
echo "  • x64:   release/${PRODUCT_NAME}-${APP_VERSION}-win-x64/${PRODUCT_NAME}.exe"
echo "  • arm64: release/${PRODUCT_NAME}-${APP_VERSION}-win-arm64/${PRODUCT_NAME}.exe"
echo "ZIP Archives:"
echo "  • x64:   release/${PRODUCT_NAME}-${APP_VERSION}-win-x64.zip"
echo "  • arm64: release/${PRODUCT_NAME}-${APP_VERSION}-win-arm64.zip"
echo "==========================================="
