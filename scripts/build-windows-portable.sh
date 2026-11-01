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
    npx electron-builder --win portable --x64
elif [ "$ARCH" = "arm64" ]; then
    echo "• Packaging Windows Portable for arm64..."
    npx electron-builder --win portable --arm64
elif [ "$ARCH" = "all" ]; then
    echo "• Packaging Windows Portable for x64 and arm64..."
    npx electron-builder --win portable --x64 --arm64
else
    echo "Unknown architecture: $ARCH (choose x64, arm64, or all)"
    exit 1
fi

echo ""
echo "==========================================="
echo "✅ Windows Portable build complete!"
echo "Executables located in: release/"
echo "==========================================="

