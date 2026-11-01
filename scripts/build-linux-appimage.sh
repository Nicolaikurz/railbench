#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

ARCH="${1:-all}"

echo "=== Building Linux AppImage (Gleisplan 0.4.0) ==="
echo "Target Architecture: $ARCH"

npm run build:all

if [ "$ARCH" = "x64" ]; then
    echo "• Packaging Linux AppImage for x64..."
    npx electron-builder --linux AppImage --x64
elif [ "$ARCH" = "arm64" ]; then
    echo "• Packaging Linux AppImage for arm64..."
    npx electron-builder --linux AppImage --arm64
elif [ "$ARCH" = "all" ]; then
    echo "• Packaging Linux AppImage for x64 and arm64..."
    npx electron-builder --linux AppImage --x64 --arm64
else
    echo "Unknown architecture: $ARCH (choose x64, arm64, or all)"
    exit 1
fi

echo ""
echo "==========================================="
echo "✅ Linux AppImage build complete!"
echo "AppImages located in: release/"
echo "==========================================="

