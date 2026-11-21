#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

ARCH="${1:-all}"
VERSION="$(node -p "require('$PROJECT_ROOT/package.json').version")"

echo "=== Building Linux AppImage (RAILbench v${VERSION}) ==="
echo "Target Architecture: $ARCH"

archive_old_binaries "$PROJECT_ROOT/release" "*.AppImage"

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

cleanup_build_artifacts "$PROJECT_ROOT"

echo ""
echo "==========================================="
echo " Linux AppImage build complete!"
echo "AppImages located in: release/"
echo "==========================================="

