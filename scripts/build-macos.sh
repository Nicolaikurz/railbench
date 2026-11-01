#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

ARCH="${1:-all}"

echo "=== Building macOS Packages (Gleisplan 0.4.0) ==="
echo "Target Architecture: $ARCH"

npm run build:all

IS_MACOS=false
if [[ "$OSTYPE" == "darwin"* ]]; then
    IS_MACOS=true
fi

if [ "$ARCH" = "x64" ]; then
    echo "• Packaging for macOS Intel (x64)..."
    if [ "$IS_MACOS" = true ]; then
        npx electron-builder --mac dmg zip --x64
    else
        npx electron-builder --mac zip --x64
    fi
elif [ "$ARCH" = "arm64" ]; then
    echo "• Packaging for macOS Apple Silicon (arm64)..."
    if [ "$IS_MACOS" = true ]; then
        npx electron-builder --mac dmg zip --arm64
    else
        npx electron-builder --mac zip --arm64
    fi
elif [ "$ARCH" = "all" ]; then
    echo "• Packaging for macOS Intel (x64) and Apple Silicon (arm64)..."
    if [ "$IS_MACOS" = true ]; then
        npx electron-builder --mac dmg zip --x64 --arm64
    else
        npx electron-builder --mac zip --x64 --arm64
    fi
else
    echo "Unknown architecture: $ARCH (choose x64, arm64, or all)"
    exit 1
fi

echo ""
echo "==========================================="
echo "✅ macOS package build finished!"
echo "Files located in: release/"
echo "==========================================="

