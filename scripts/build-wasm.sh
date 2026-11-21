#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CRATE_DIR="$PROJECT_ROOT/rust/railbench-wasm"
OUT_DIR="$PROJECT_ROOT/src/wasm"

echo "=== Building RAILbench Rust WebAssembly kernel ==="

if ! command -v cargo &> /dev/null; then
    echo "⚠️  Rust / Cargo is not installed."
    echo ""
    echo "Install Rust:"
    echo "  • Arch Linux:     sudo pacman -S rustup && rustup default stable"
    echo "  • Ubuntu / macOS: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

if ! command -v wasm-pack &> /dev/null; then
    echo "⚠️  wasm-pack is not installed."
    echo ""
    echo "Install wasm-pack:"
    echo "  • Arch Linux: sudo pacman -S wasm-pack   (or: cargo install wasm-pack)"
    echo "  • Other OS:   cargo install wasm-pack"
    exit 1
fi

if command -v rustup &> /dev/null; then
    rustup target add wasm32-unknown-unknown || true
fi

cd "$CRATE_DIR"
wasm-pack build --target web --release --out-dir "$OUT_DIR"
rm -f "$OUT_DIR/.gitignore"

echo ""
echo "✅ WASM kernel built at: $OUT_DIR"
