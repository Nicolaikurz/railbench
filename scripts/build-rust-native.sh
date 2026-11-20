#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

VERSION="$(node -p "require('$PROJECT_ROOT/package.json').version")"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
OUT_BIN="$PROJECT_ROOT/release/railbench-${VERSION}-native-${OS}-${ARCH}"

echo "=== Building Native Rust RAILbench v${VERSION} (${OS}/${ARCH}) ==="

archive_old_binaries "$PROJECT_ROOT/release" "*-native-*"

cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app

mkdir -p "$PROJECT_ROOT/release"
cp "$PROJECT_ROOT/rust/target/release/railbench-app" "$OUT_BIN"
chmod +x "$OUT_BIN"
if command -v strip &> /dev/null; then
    strip "$OUT_BIN" || true
fi

SIZE="$(du -h "$OUT_BIN" | awk '{print $1}')"
echo ""
echo "==========================================="
echo " Native Rust release build complete!"
echo " Binary: $OUT_BIN ($SIZE)"
echo "==========================================="
