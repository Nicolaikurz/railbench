#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Argument handling:
# ./scripts/build-macos.sh [arch] [format]
# e.g.:
#   ./scripts/build-macos.sh              -> all architectures, auto format
#   ./scripts/build-macos.sh arm64        -> arm64, auto format
#   ./scripts/build-macos.sh dmg          -> all architectures, dmg format
#   ./scripts/build-macos.sh arm64 dmg    -> arm64, dmg format
#   ./scripts/build-macos.sh all dmg      -> all architectures, dmg format
#   ./scripts/build-macos.sh all zip      -> all architectures, zip format
#   ./scripts/build-macos.sh all both     -> all architectures, dmg + zip
ARG1="${1:-all}"
ARG2="${2:-auto}"

if [ "$ARG1" = "dmg" ] || [ "$ARG1" = "zip" ] || [ "$ARG1" = "both" ]; then
    FORMAT="$ARG1"
    ARCH="${2:-all}"
else
    ARCH="$ARG1"
    FORMAT="$ARG2"
fi

# Dynamically fetch App Name and Version from package.json
APP_NAME=$(node -p "require('./package.json').name" 2>/dev/null || echo "RAILbench")
APP_VERSION=$(node -p "require('./package.json').version" 2>/dev/null || echo "0.6.0")

IS_MACOS=false
if [[ "$OSTYPE" == "darwin"* ]]; then
    IS_MACOS=true
fi

case "$FORMAT" in
    dmg)
        TARGETS="dmg"
        ;;
    zip)
        TARGETS="zip"
        ;;
    both|all)
        TARGETS="dmg zip"
        ;;
    auto|*)
        if [ "$IS_MACOS" = true ]; then
            TARGETS="dmg zip"
        else
            TARGETS="zip"
        fi
        ;;
esac

echo "=== Building macOS Packages ($APP_NAME $APP_VERSION) ==="
echo "Target Architecture : $ARCH"
echo "Target Format(s)    : $TARGETS"

if [[ "$TARGETS" == *"dmg"* ]] && [ "$IS_MACOS" = false ]; then
    echo "⚠️  Hinweis: Die native Erstellung von .dmg-Dateien erfordert macOS."
    echo "   Für Cross-Compiling unter Linux/Windows wird das ZIP-Format empfohlen."
fi

case "$ARCH" in
    x64|arm64|all|universal)
        ;;
    *)
        echo "Unknown architecture: $ARCH (choose x64, arm64, universal, or all)"
        exit 1
        ;;
esac

npm run build:all

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
