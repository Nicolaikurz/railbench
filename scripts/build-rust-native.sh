#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

if command -v node &> /dev/null; then
    PKG_PATCH="$(node -p "String(require('$PROJECT_ROOT/package.json').version || '26.11.1').split('.')[2] || '1'" 2>/dev/null || echo "1")"
else
    PKG_PATCH="$(grep -m1 '"version"' "$PROJECT_ROOT/package.json" | sed -E 's/.*"version"[[:space:]]*:[[:space:]]*"[^."]*\.[^."]*\.([^."]*).*/\1/' || echo "1")"
    [ -n "$PKG_PATCH" ] || PKG_PATCH="1"
fi
DEFAULT_CALVER="$(date -u +%y.%m).${PKG_PATCH}"
VERSION="${RAILBENCH_VERSION:-$DEFAULT_CALVER}"
VERSION="${VERSION#v}"
VERSION="${VERSION#V}"
export RAILBENCH_VERSION="$VERSION"
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
