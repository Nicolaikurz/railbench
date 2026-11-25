#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

VERSION="$(node -p "require('$PROJECT_ROOT/package.json').version")"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
EXE_EXT=""
case "$OS" in
    mingw*|msys*|cygwin*|windows*)
        OS="windows"
        EXE_EXT=".exe"
        ;;
esac
OUT_BIN="$PROJECT_ROOT/release/railbench-${VERSION}-native-${OS}-${ARCH}${EXE_EXT}"

echo "=== Building Native Rust RAILbench v${VERSION} (${OS}/${ARCH}) ==="

archive_old_binaries "$PROJECT_ROOT/release" "*-native-*"

cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app

mkdir -p "$PROJECT_ROOT/release"
cp "$PROJECT_ROOT/rust/target/release/railbench-app${EXE_EXT}" "$OUT_BIN"
if [ -z "$EXE_EXT" ]; then
    chmod +x "$OUT_BIN"
fi
if command -v strip &> /dev/null; then
    case "$OS" in
        darwin*) strip -x "$OUT_BIN" || true ;;
        *) strip "$OUT_BIN" || true ;;
    esac
fi

SIZE="$(du -h "$OUT_BIN" | awk '{print $1}')"
echo ""
echo "==========================================="
echo " Native Rust release build complete!"
echo " Binary: $OUT_BIN ($SIZE)"
echo "==========================================="
