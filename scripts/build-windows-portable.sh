#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

ARCH="${1:-all}"

echo "=== Building Windows Portable Executable (Gleisplan 0.4.0) ==="
echo "Target Architecture: $ARCH"

npm run build:all

if [ "$ARCH" = "x64" ]; then
    echo "• Packaging Windows Portable for x64..."
    npx electron-builder --win dir --x64
elif [ "$ARCH" = "arm64" ]; then
    echo "• Packaging Windows Portable for arm64..."
    npx electron-builder --win dir --arm64
elif [ "$ARCH" = "all" ]; then
    echo "• Packaging Windows Portable for x64 and arm64..."
    npx electron-builder --win dir --x64 --arm64
else
    echo "Unknown architecture: $ARCH (choose x64, arm64, or all)"
    exit 1
fi

echo ""
echo "=== Zipping Windows Executables ==="
if command -v zip &> /dev/null; then
    cd release
    # Find all newly generated portable .exe files and zip each one individually
    for EXE_FILE in *-portable-*.exe; do
        if [ -f "$EXE_FILE" ]; then
            ZIP_NAME="${EXE_FILE%.exe}.zip"
            zip -j "$ZIP_NAME" "$EXE_FILE"
            echo "Created archive: release/$ZIP_NAME"
        fi
    done
    cd ..
else
    echo "[WARNING] 'zip' command not found. Skipping compression."
fi

echo ""
echo "==========================================="
echo " Windows Portable build and zip complete!"
echo "Archives located in: release/"
echo "==========================================="