#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

if command -v node &> /dev/null; then
    VERSION="$(node -p "require('$PROJECT_ROOT/package.json').version")"
else
    VERSION="$(grep -m1 '"version"' "$PROJECT_ROOT/package.json" | sed -E 's/.*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')"
fi
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
OUT_BIN="$PROJECT_ROOT/release/railbench-${VERSION}-native-${OS}-${ARCH}"

echo "=== Building Native Rust RAILbench v${VERSION} (${OS}/${ARCH}) ==="

archive_old_binaries "$PROJECT_ROOT/release" "*-native-*"

cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app

mkdir -p "$PROJECT_ROOT/release"
cp "$PROJECT_ROOT/rust/target/release/railbench-app" "$OUT_BIN"
chmod +x "$OUT_BIN"

SIZE="$(du -h "$OUT_BIN" | awk '{print $1}')"
echo ""
echo "==========================================="
echo " Native Rust release build complete!"
echo " Binary: $OUT_BIN ($SIZE)"
echo "==========================================="
