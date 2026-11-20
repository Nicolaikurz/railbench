#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

TARGET="${1:-x64}"
VERSION="$(node -p "require('$PROJECT_ROOT/package.json').version")"

echo "=== Building Linux Snap package for RAILbench v${VERSION} ==="
echo "Target Architecture: $TARGET"

npm run build:all

build_x64() {
    echo ""
    echo "• Packaging Snap for x64 (amd64)..."
    npx electron-builder --linux snap --x64
}

build_arm64() {
    echo ""
    echo "• Packaging Snap for arm64..."
    if ! command -v snapcraft &> /dev/null; then
        echo "⚠️  snapcraft CLI is required for ARM64 snap builds (x64 uses electron-builder's template without snapcraft)."
        echo "Install snapcraft: sudo snap install snapcraft --classic"
        return 1
    fi
    npx electron-builder --linux snap --arm64
}

case "$TARGET" in
    x64|amd64|x86_64)
        build_x64
        ;;
    arm64|aarch64)
        build_arm64
        ;;
    all)
        build_x64
        if command -v snapcraft &> /dev/null; then
            build_arm64
        else
            echo "⚠️  Skipping arm64: snapcraft CLI not installed."
        fi
        ;;
    *)
        echo "Unknown architecture: $TARGET (choose x64, arm64, or all)"
        exit 1
        ;;
esac

rm -rf "$PROJECT_ROOT/release/__snap-"*

echo ""
echo "==========================================="
echo " Snap build complete!"
echo "Package(s) located in: $PROJECT_ROOT/release/"
echo "To install locally: sudo snap install --dangerous release/railbench_${VERSION}_amd64.snap"
echo "To upload to store: snapcraft upload release/railbench_${VERSION}_amd64.snap --release=stable"
echo "==========================================="
