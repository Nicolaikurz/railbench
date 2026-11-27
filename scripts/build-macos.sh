#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
source "$SCRIPT_DIR/release-cleanup.sh"

# Usage:
#   ./scripts/build-macos.sh [x64|arm64|universal|all] [dmg|zip|both]
ARG1="${1:-all}"
ARG2="${2:-both}"

if [ "$ARG1" = "dmg" ] || [ "$ARG1" = "zip" ] || [ "$ARG1" = "both" ]; then
    FORMAT="$ARG1"
    ARCH="${2:-all}"
else
    ARCH="$ARG1"
    FORMAT="$ARG2"
fi

PRODUCT_NAME="RAILbench"
APP_VERSION="$(resolve_railbench_version "$PROJECT_ROOT")"

IS_MACOS=false
if [[ "$OSTYPE" == "darwin"* ]]; then
    IS_MACOS=true
fi

WANT_DMG=true
WANT_ZIP=true
case "$FORMAT" in
    dmg) WANT_DMG=true; WANT_ZIP=false ;;
    zip) WANT_DMG=false; WANT_ZIP=true ;;
    *) WANT_DMG=true; WANT_ZIP=true ;;
esac

find_rcodesign() {
    if command -v rcodesign &> /dev/null; then
        echo "rcodesign"
    elif [ -x "$HOME/.local/bin/rcodesign" ]; then
        echo "$HOME/.local/bin/rcodesign"
    elif [ -x "$HOME/.cargo/bin/rcodesign" ]; then
        echo "$HOME/.cargo/bin/rcodesign"
    elif [ -x "/usr/local/bin/rcodesign" ]; then
        echo "/usr/local/bin/rcodesign"
    else
        echo ""
    fi
}

RCODESIGN_BIN="$(find_rcodesign)"

echo "=========================================================="
echo " Building Native Rust macOS Packages ($PRODUCT_NAME v$APP_VERSION)"
echo "=========================================================="
echo "Target Architecture : $ARCH"
echo "Build DMG           : $WANT_DMG"
echo "Build ZIP           : $WANT_ZIP"
echo "Host OS             : $([ "$IS_MACOS" = true ] && echo 'macOS (native)' || echo 'Linux/Unix (cross-compilation)')"
echo "----------------------------------------------------------"

case "$ARCH" in
    x64|arm64|all|universal) ;;
    *)
        echo "Unknown architecture: $ARCH (choose x64, arm64, universal, or all)"
        exit 1
        ;;
esac

compile_darwin_binary() {
    local RUST_TARGET="$1"
    if [ "$IS_MACOS" = true ]; then
        cargo build --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"
    elif command -v cargo-zigbuild &> /dev/null; then
        cargo zigbuild --release --manifest-path "$PROJECT_ROOT/rust/Cargo.toml" -p railbench-app --target "$RUST_TARGET"
    else
        echo "⚠️  Building macOS binaries from Linux requires 'cargo-zigbuild' + 'zig' (or run natively on macOS)."
        echo ""
        echo "Install:"
        echo "  cargo install cargo-zigbuild"
        echo "  rustup target add $RUST_TARGET"
        return 1
    fi
}

sign_app_bundle() {
    local APP_BUNDLE="$1"
    if [ -n "$RCODESIGN_BIN" ]; then
        echo "• Code signing with rcodesign: $APP_BUNDLE"
        if [ -n "${APPLE_CERT_P12:-}" ] && [ -f "$APPLE_CERT_P12" ]; then
            if [ -n "${APPLE_CERT_PASSWORD:-}" ]; then
                "$RCODESIGN_BIN" sign --p12-file "$APPLE_CERT_P12" --p12-password "$APPLE_CERT_PASSWORD" "$APP_BUNDLE"
            else
                "$RCODESIGN_BIN" sign --p12-file "$APPLE_CERT_P12" "$APP_BUNDLE"
            fi
        else
            "$RCODESIGN_BIN" sign "$APP_BUNDLE"
        fi
    elif [ "$IS_MACOS" = true ]; then
        echo "• Code signing with native macOS codesign: $APP_BUNDLE"
        codesign --force --deep -s - "$APP_BUNDLE"
    else
        echo "⚠️  rcodesign not found; bundle remains unsigned (ad-hoc sign on macOS with 'codesign --force --deep -s -')."
    fi
}

create_app_bundle() {
    local TARGET_ARCH="$1" # x64 | arm64 | universal
    local STAGE_DIR="$PROJECT_ROOT/release/mac-stage-${TARGET_ARCH}"
    local APP_BUNDLE="$STAGE_DIR/${PRODUCT_NAME}.app"
    local BIN_DEST="$APP_BUNDLE/Contents/MacOS/railbench"

    rm -rf "$STAGE_DIR"
    mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources"

    if [ "$TARGET_ARCH" = "x64" ]; then
        compile_darwin_binary "x86_64-apple-darwin"
        cp "$PROJECT_ROOT/rust/target/x86_64-apple-darwin/release/railbench-app" "$BIN_DEST"
    elif [ "$TARGET_ARCH" = "arm64" ]; then
        compile_darwin_binary "aarch64-apple-darwin"
        cp "$PROJECT_ROOT/rust/target/aarch64-apple-darwin/release/railbench-app" "$BIN_DEST"
    else
        compile_darwin_binary "x86_64-apple-darwin"
        compile_darwin_binary "aarch64-apple-darwin"
        lipo -create \
            "$PROJECT_ROOT/rust/target/x86_64-apple-darwin/release/railbench-app" \
            "$PROJECT_ROOT/rust/target/aarch64-apple-darwin/release/railbench-app" \
            -output "$BIN_DEST"
    fi
    chmod +x "$BIN_DEST"

    cat > "$APP_BUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>railbench</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.Nicolaikurz.railbench</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>RAILbench</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${APP_VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${APP_VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

    if [ -f "$PROJECT_ROOT/build/icon.png" ]; then
        cp "$PROJECT_ROOT/build/icon.png" "$APP_BUNDLE/Contents/Resources/icon.png"
    fi

    sign_app_bundle "$APP_BUNDLE"

    if [ "$WANT_DMG" = true ]; then
        local OUTPUT_DMG="$PROJECT_ROOT/release/railbench-${APP_VERSION}-mac-${TARGET_ARCH}.dmg"
        rm -f "$OUTPUT_DMG"
        mkdir -p "$PROJECT_ROOT/release"
        if [ "$IS_MACOS" = true ]; then
            local DMG_SRC="$STAGE_DIR/dmg-src"
            mkdir -p "$DMG_SRC"
            cp -R "$APP_BUNDLE" "$DMG_SRC/"
            ln -s /Applications "$DMG_SRC/Applications"
            hdiutil create -volname "$PRODUCT_NAME" -srcfolder "$DMG_SRC" -ov -format UDZO "$OUTPUT_DMG"
            echo "✓ Created DMG: $OUTPUT_DMG"
        else
            local ISO_MAKER=""
            command -v genisoimage &> /dev/null && ISO_MAKER="genisoimage"
            [ -z "$ISO_MAKER" ] && command -v mkisofs &> /dev/null && ISO_MAKER="mkisofs"
            if [ -n "$ISO_MAKER" ]; then
                local DMG_SRC="$STAGE_DIR/dmg-src"
                mkdir -p "$DMG_SRC"
                cp -a "$APP_BUNDLE" "$DMG_SRC/"
                ln -s /Applications "$DMG_SRC/Applications"
                "$ISO_MAKER" -V "$PRODUCT_NAME" -D -R -apple -no-pad -quiet -o "$OUTPUT_DMG" "$DMG_SRC"
                echo "✓ Created DMG: $OUTPUT_DMG"
            else
                echo "⚠️  genisoimage/mkisofs not found; skipping DMG on Linux."
            fi
        fi
    fi

    if [ "$WANT_ZIP" = true ]; then
        local OUTPUT_ZIP="$PROJECT_ROOT/release/railbench-${APP_VERSION}-mac-${TARGET_ARCH}.zip"
        rm -f "$OUTPUT_ZIP"
        mkdir -p "$PROJECT_ROOT/release"
        if command -v zip &> /dev/null; then
            (cd "$STAGE_DIR" && zip -qry "$OUTPUT_ZIP" "${PRODUCT_NAME}.app")
        elif command -v python3 &> /dev/null; then
            python3 -c "import shutil; shutil.make_archive('${PROJECT_ROOT}/release/railbench-${APP_VERSION}-mac-${TARGET_ARCH}', 'zip', '${STAGE_DIR}', '${PRODUCT_NAME}.app')"
        else
            echo "⚠️  No zip tool found; skipping ZIP."
        fi
        [ -f "$OUTPUT_ZIP" ] && echo "✓ Created ZIP: $OUTPUT_ZIP"
    fi

    rm -rf "$STAGE_DIR"
}

[ "$WANT_DMG" = true ] && archive_old_binaries "$PROJECT_ROOT/release" "*-mac-*.dmg"
[ "$WANT_ZIP" = true ] && archive_old_binaries "$PROJECT_ROOT/release" "*-mac-*.zip"

case "$ARCH" in
    x64) create_app_bundle "x64" ;;
    arm64) create_app_bundle "arm64" ;;
    universal) create_app_bundle "universal" ;;
    all)
        create_app_bundle "x64"
        create_app_bundle "arm64"
        ;;
esac

cleanup_build_artifacts "$PROJECT_ROOT"

echo ""
echo "=========================================================="
echo " macOS package build finished!"
echo " Output files in: release/"
echo "=========================================================="
