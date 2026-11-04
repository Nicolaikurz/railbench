#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

ARCH="${1:-all}"

# Dynamically fetch App Name and Version from package.json
APP_NAME=$(node -p "require('./package.json').name" 2>/dev/null || echo "RAILbench")
APP_VERSION=$(node -p "require('./package.json').version" 2>/dev/null || echo "0.5.1")

echo "=== Building macOS Packages ($APP_NAME $APP_VERSION) ==="
echo "Target Architecture: $ARCH"

npm run build:all

IS_MACOS=false
if [[ "$OSTYPE" == "darwin"* ]]; then
    IS_MACOS=true
fi

TARGETS="zip"
if [ "$IS_MACOS" = true ]; then
    TARGETS="dmg zip"
fi

case "$ARCH" in
    x64)
        echo "• Packaging for macOS Intel (x64)..."
        npx electron-builder --mac $TARGETS --x64
        ;;
    arm64)
        echo "• Packaging for macOS Apple Silicon (arm64)..."
        npx electron-builder --mac $TARGETS --arm64
        ;;
    all)
        echo "• Packaging for macOS Intel (x64) and Apple Silicon (arm64)..."
        npx electron-builder --mac $TARGETS --x64
        npx electron-builder --mac $TARGETS --arm64
        ;;
    universal)
        echo "• Packaging Universal macOS Binary..."
        npx electron-builder --mac $TARGETS --universal
        ;;
    *)
        echo "Unknown architecture: $ARCH (choose x64, arm64, universal, or all)"
        exit 1
        ;;
esac

echo ""
echo "==========================================="
echo "macOS package build finished!"
echo "Files located in: release/"
echo "==========================================="