#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

VERSION="$(resolve_railbench_version "$PROJECT_ROOT")"
TARGET_TRIPLE="${1:-}"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
EXE_EXT=""
CARGO_ARGS=(build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app)

if [ -n "$TARGET_TRIPLE" ]; then
    CARGO_ARGS+=(--target "$TARGET_TRIPLE")
    BIN_SRC="$PROJECT_ROOT/rust/target/$TARGET_TRIPLE/release/railbench-app"
    case "$TARGET_TRIPLE" in
        *windows*)
            OS="windows"
            EXE_EXT=".exe"
            BIN_SRC="${BIN_SRC}.exe"
            ;;
        *apple-darwin*) OS="darwin" ;;
        *linux*) OS="linux" ;;
    esac
    case "$TARGET_TRIPLE" in
        x86_64*) ARCH="x86_64" ;;
        aarch64*|arm64*) ARCH="aarch64" ;;
    esac
else
    case "$OS" in
        mingw*|msys*|cygwin*|windows*)
            OS="windows"
            EXE_EXT=".exe"
            ;;
    esac
    BIN_SRC="$PROJECT_ROOT/rust/target/release/railbench-app${EXE_EXT}"
fi

OUT_BIN="$PROJECT_ROOT/release/railbench-${VERSION}-native-${OS}-${ARCH}${EXE_EXT}"

echo "=== Building Native Rust RAILbench v${VERSION} (${OS}/${ARCH}) ==="

archive_old_binaries "$PROJECT_ROOT/release" "*-native-${OS}-${ARCH}*"

cargo "${CARGO_ARGS[@]}"

mkdir -p "$PROJECT_ROOT/release"
cp "$BIN_SRC" "$OUT_BIN"
if [ -z "$EXE_EXT" ]; then
    chmod +x "$OUT_BIN"
fi
if [ -z "$TARGET_TRIPLE" ] && command -v strip &> /dev/null; then
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
