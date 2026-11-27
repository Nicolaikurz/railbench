#!/usr/bin/env bash
# Shared helpers for RAILbench native Rust release & packaging scripts:
#  1) resolve_railbench_version <project_root>      -> prints & exports RAILBENCH_VERSION (CalVer YY.MM.P)
#  2) archive_old_binaries <release_dir> <glob>...  -> moves existing matching binaries into release/old/
#  3) cleanup_build_artifacts <project_root>        -> removes intermediate staging directories

resolve_railbench_version() {
    local PROJECT_ROOT="$1"
    local CARGO_TOML="$PROJECT_ROOT/rust/Cargo.toml"
    local PKG_PATCH="1"
    if [ -f "$CARGO_TOML" ]; then
        local RAW_VER
        RAW_VER="$(grep -E '^[[:space:]]*version[[:space:]]*=' "$CARGO_TOML" | head -n1 | sed -E 's/.*"([^"]+)".*/\1/')"
        if [[ "$RAW_VER" =~ ^[0-9]+\.[0-9]+\.([0-9]+)$ ]]; then
            PKG_PATCH="${BASH_REMATCH[1]}"
        fi
    fi
    local DEFAULT_CALVER
    DEFAULT_CALVER="$(date -u +%y.%m).${PKG_PATCH}"
    local VER="${RAILBENCH_VERSION:-$DEFAULT_CALVER}"
    VER="${VER#v}"
    VER="${VER#V}"
    export RAILBENCH_VERSION="$VER"
    export RAILBENCH_PATCH="$PKG_PATCH"
    echo "$VER"
}

archive_old_binaries() {
    local RELEASE_DIR="$1"
    shift
    local OLD_DIR="$RELEASE_DIR/old"
    local MOVED=0

    shopt -s nullglob
    for pattern in "$@"; do
        for f in "$RELEASE_DIR"/$pattern; do
            [ -f "$f" ] || continue
            mkdir -p "$OLD_DIR"
            local base
            base="$(basename "$f")"
            local dest="$OLD_DIR/$base"
            if [ -e "$dest" ]; then
                local stamp
                stamp="$(date +%Y%m%d-%H%M%S)"
                local name="${base%.*}"
                local ext="${base##*.}"
                if [ "$name" != "$base" ]; then
                    dest="$OLD_DIR/${name}-${stamp}.${ext}"
                else
                    dest="$OLD_DIR/${base}-${stamp}"
                fi
            fi
            mv "$f" "$dest"
            echo "• Archived old binary -> release/old/$(basename "$dest")"
            MOVED=1
        done
    done
    shopt -u nullglob

    if [ "$MOVED" -eq 0 ]; then
        echo "• No previous binaries to archive."
    fi
}

cleanup_build_artifacts() {
    local PROJECT_ROOT="$1"
    local RELEASE_DIR="$PROJECT_ROOT/release"

    echo "• Cleaning intermediate staging artifacts..."
    rm -rf \
        "$RELEASE_DIR"/AppDir* \
        "$RELEASE_DIR"/flatpak-stage* \
        "$RELEASE_DIR"/snap-stage* \
        "$RELEASE_DIR"/mac-stage* \
        "$RELEASE_DIR"/win-stage* \
        "$RELEASE_DIR/flatpak-repo" \
        "$PROJECT_ROOT/build-flatpak" \
        "$PROJECT_ROOT"/build-flatpak-* \
        "$PROJECT_ROOT/snap"
}
