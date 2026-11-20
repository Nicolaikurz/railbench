#!/usr/bin/env bash
# Shared helper for RAILbench release build scripts:
#  1) archive_old_binaries <release_dir> <glob>...  -> moves existing matching binaries into release/old/
#  2) cleanup_build_artifacts <project_root>        -> removes all intermediate build dirs & metadata from release/

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

    echo "• Cleaning intermediate build artifacts..."
    rm -rf \
        "$RELEASE_DIR/linux-unpacked" \
        "$RELEASE_DIR/linux-arm64-unpacked" \
        "$RELEASE_DIR/win-unpacked" \
        "$RELEASE_DIR/win-arm64-unpacked" \
        "$RELEASE_DIR/mac" \
        "$RELEASE_DIR/mac-arm64" \
        "$RELEASE_DIR/mac-universal" \
        "$RELEASE_DIR/flatpak-repo" \
        "$RELEASE_DIR"/__snap-* \
        "$RELEASE_DIR/.icon-icns" \
        "$RELEASE_DIR/builder-debug.yml" \
        "$RELEASE_DIR/builder-effective-config.yaml" \
        "$RELEASE_DIR"/latest*.yml \
        "$RELEASE_DIR"/*.blockmap \
        "$PROJECT_ROOT/build-flatpak" \
        "$PROJECT_ROOT"/build-flatpak-*
}
