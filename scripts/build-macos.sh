#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Argument handling:
# ./scripts/build-macos.sh [arch] [format]
# e.g.:
#   ./scripts/build-macos.sh              -> all architectures, dmg format (default)
#   ./scripts/build-macos.sh arm64        -> arm64 (Apple Silicon), dmg format
#   ./scripts/build-macos.sh x64          -> x64 (Intel), dmg format
#   ./scripts/build-macos.sh dmg          -> all architectures, dmg format
#   ./scripts/build-macos.sh arm64 dmg    -> arm64, dmg format
#   ./scripts/build-macos.sh all dmg      -> all architectures, dmg format
#   ./scripts/build-macos.sh all zip      -> all architectures, zip format
#   ./scripts/build-macos.sh all both     -> all architectures, dmg + zip
ARG1="${1:-all}"
ARG2="${2:-dmg}"

if [ "$ARG1" = "dmg" ] || [ "$ARG1" = "zip" ] || [ "$ARG1" = "both" ]; then
    FORMAT="$ARG1"
    ARCH="${2:-all}"
else
    ARCH="$ARG1"
    FORMAT="$ARG2"
fi

# Dynamically fetch Product Name and Version from package.json
PRODUCT_NAME=$(node -p "require('./package.json').build?.productName || require('./package.json').name || 'railbench'" 2>/dev/null || echo "railbench")
APP_VERSION=$(node -p "require('./package.json').version || '0.6.0'" 2>/dev/null || echo "0.6.0")

IS_MACOS=false
if [[ "$OSTYPE" == "darwin"* ]]; then
    IS_MACOS=true
fi

WANT_DMG=true
WANT_ZIP=false

case "$FORMAT" in
    dmg|"")
        WANT_DMG=true
        WANT_ZIP=false
        ;;
    zip)
        WANT_DMG=false
        WANT_ZIP=true
        ;;
    both|all)
        WANT_DMG=true
        WANT_ZIP=true
        ;;
    *)
        WANT_DMG=true
        WANT_ZIP=false
        ;;
esac

# Locate rcodesign binary
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

RCODESIGN_BIN=$(find_rcodesign)

echo "=========================================================="
echo " Building macOS Packages ($PRODUCT_NAME v$APP_VERSION)"
echo "=========================================================="
echo "Target Architecture : $ARCH"
echo "Build DMG           : $WANT_DMG (Primary / Default)"
echo "Build ZIP           : $WANT_ZIP"
echo "Host OS             : $([ "$IS_MACOS" = true ] && echo 'macOS (native)' || echo 'Linux/Unix (cross-compilation)')"
if [ -n "$RCODESIGN_BIN" ]; then
    echo "Code Signing        : rcodesign enabled ($("$RCODESIGN_BIN" --version 2>&1))"
elif [ "$IS_MACOS" = true ]; then
    echo "Code Signing        : Apple codesign tool (native)"
else
    echo "Code Signing        : None (rcodesign not found)"
fi
echo "----------------------------------------------------------"

case "$ARCH" in
    x64|arm64|all|universal)
        ;;
    *)
        echo "Unknown architecture: $ARCH (choose x64, arm64, universal, or all)"
        exit 1
        ;;
esac

# 1. Build TypeScript and Vite client & electron backend
npm run build:all

# Helper to recursively sign an .app bundle
sign_app_bundle() {
    local APP_BUNDLE="$1"

    if [ ! -d "$APP_BUNDLE" ]; then
        echo "⚠️  App bundle '$APP_BUNDLE' not found to sign."
        return 1
    fi

    if [ -n "$RCODESIGN_BIN" ]; then
        echo "• Code signing with rcodesign: $APP_BUNDLE"
        if [ -n "$APPLE_CERT_P12" ] && [ -f "$APPLE_CERT_P12" ]; then
            echo "  Using Developer ID Certificate: $APPLE_CERT_P12"
            if [ -n "$APPLE_CERT_PASSWORD" ]; then
                "$RCODESIGN_BIN" sign --p12-file "$APPLE_CERT_P12" --p12-password "$APPLE_CERT_PASSWORD" "$APP_BUNDLE"
            else
                "$RCODESIGN_BIN" sign --p12-file "$APPLE_CERT_P12" "$APP_BUNDLE"
            fi
        else
            echo "  Applying Ad-Hoc signature (AMFI / Apple Silicon M1+ compliant)..."
            "$RCODESIGN_BIN" sign "$APP_BUNDLE"
        fi
        echo "✓ Signature applied to $APP_BUNDLE"
    elif [ "$IS_MACOS" = true ]; then
        echo "• Code signing with native macOS codesign: $APP_BUNDLE"
        codesign --force --deep -s - "$APP_BUNDLE"
        echo "✓ Native ad-hoc signature applied"
    else
        echo "⚠️  rcodesign not found! The bundle will remain unsigned."
        echo "   Apple Silicon (M1/M2/M3/M4) may terminate unsigned Mach-O binaries."
        echo "   To fix: install rcodesign (e.g. into ~/.local/bin/rcodesign or via 'cargo install apple-codesign')."
    fi
}

# Helper to create DMG on Linux/Unix using genisoimage or mkisofs
create_dmg_unix() {
    local TARGET_ARCH="$1"
    local APP_DIR="$2"
    local OUTPUT_DMG="release/${PRODUCT_NAME}-${APP_VERSION}-mac-${TARGET_ARCH}.dmg"

    if [ ! -d "$APP_DIR" ]; then
        echo "⚠️  App directory '$APP_DIR' not found, skipping DMG for $TARGET_ARCH."
        return
    fi

    # Find the .app bundle inside APP_DIR
    local APP_BUNDLE=""
    for item in "$APP_DIR"/*.app; do
        if [ -d "$item" ]; then
            APP_BUNDLE="$item"
            break
        fi
    done

    if [ -z "$APP_BUNDLE" ]; then
        echo "⚠️  No .app bundle found in '$APP_DIR', skipping DMG."
        return
    fi

    local ISO_MAKER=""
    if command -v genisoimage &> /dev/null; then
        ISO_MAKER="genisoimage"
    elif command -v mkisofs &> /dev/null; then
        ISO_MAKER="mkisofs"
    fi

    if [ -z "$ISO_MAKER" ]; then
        echo "⚠️  'genisoimage' or 'mkisofs' not found. Cannot create .dmg on Linux."
        echo "   Please install genisoimage (e.g. 'sudo apt install genisoimage' or 'pacman -S cdrtools')."
        return
    fi

    echo "• Creating macOS DMG for $TARGET_ARCH with $ISO_MAKER..."
    local STAGE_DIR
    STAGE_DIR=$(mktemp -d "/tmp/dmg-stage-${TARGET_ARCH}-XXXXXX")

    cp -a "$APP_BUNDLE" "$STAGE_DIR/"
    ln -s /Applications "$STAGE_DIR/Applications"

    mkdir -p release
    rm -f "$OUTPUT_DMG"

    "$ISO_MAKER" -V "$PRODUCT_NAME" -D -R -apple -no-pad -quiet -o "$OUTPUT_DMG" "$STAGE_DIR"
    rm -rf "$STAGE_DIR"

    if [ -f "$OUTPUT_DMG" ]; then
        local SIZE
        SIZE=$(du -h "$OUTPUT_DMG" | awk '{print $1}')
        echo "✓ Created DMG: $OUTPUT_DMG ($SIZE)"
    else
        echo "⚠️  Failed to generate $OUTPUT_DMG"
    fi
}

# Helper to create ZIP on Linux/Unix preserving symlinks
create_zip_unix() {
    local TARGET_ARCH="$1"
    local APP_DIR="$2"
    local OUTPUT_ZIP="release/${PRODUCT_NAME}-${APP_VERSION}-mac-${TARGET_ARCH}.zip"

    if [ ! -d "$APP_DIR" ]; then
        return
    fi

    local APP_BUNDLE=""
    for item in "$APP_DIR"/*.app; do
        if [ -d "$item" ]; then
            APP_BUNDLE="$item"
            break
        fi
    done

    if [ -z "$APP_BUNDLE" ]; then
        return
    fi

    echo "• Creating macOS ZIP for $TARGET_ARCH (preserving symlinks)..."
    rm -f "$OUTPUT_ZIP"
    local BASE_NAME
    BASE_NAME=$(basename "$APP_BUNDLE")
    (cd "$APP_DIR" && zip -qry "../../$OUTPUT_ZIP" "$BASE_NAME")

    if [ -f "$OUTPUT_ZIP" ]; then
        local SIZE
        SIZE=$(du -h "$OUTPUT_ZIP" | awk '{print $1}')
        echo "✓ Created ZIP: $OUTPUT_ZIP ($SIZE)"
    fi
}

# Cross-compilation target processor
process_unix_target() {
    local TARGET_ARCH="$1"
    local APP_DIR="$2"

    echo "• Packaging macOS bundle for $TARGET_ARCH (dir)..."
    npx electron-builder --mac dir --$TARGET_ARCH

    # Locate and sign the .app bundle
    local APP_BUNDLE=""
    for item in "$APP_DIR"/*.app; do
        if [ -d "$item" ]; then
            APP_BUNDLE="$item"
            break
        fi
    done

    if [ -n "$APP_BUNDLE" ]; then
        sign_app_bundle "$APP_BUNDLE"
    fi

    if [ "$WANT_DMG" = true ]; then
        create_dmg_unix "$TARGET_ARCH" "$APP_DIR"
    fi

    if [ "$WANT_ZIP" = true ]; then
        create_zip_unix "$TARGET_ARCH" "$APP_DIR"
    fi
}

# 2. Package with electron-builder
if [ "$IS_MACOS" = true ]; then
    # Native macOS build with electron-builder
    TARGETS=""
    [ "$WANT_DMG" = true ] && TARGETS="$TARGETS dmg"
    [ "$WANT_ZIP" = true ] && TARGETS="$TARGETS zip"
    TARGETS=$(echo "$TARGETS" | xargs)

    case "$ARCH" in
        x64)
            echo "• Packaging for macOS Intel (x64)..."
            npx electron-builder --mac $TARGETS --x64
            ;;
        arm64)
            echo "• Packaging for macOS Apple Silicon (arm64)..."
            npx electron-builder --mac $TARGETS --arm64
            ;;
        all)
            echo "• Packaging for macOS Intel (x64) and Apple Silicon (arm64)..."
            npx electron-builder --mac $TARGETS --x64
            npx electron-builder --mac $TARGETS --arm64
            ;;
        universal)
            echo "• Packaging Universal macOS Binary..."
            npx electron-builder --mac $TARGETS --universal
            ;;
    esac
else
    # Linux cross-compilation with rcodesign & genisoimage
    case "$ARCH" in
        x64)
            process_unix_target "x64" "release/mac"
            ;;
        arm64)
            process_unix_target "arm64" "release/mac-arm64"
            ;;
        all)
            process_unix_target "x64" "release/mac"
            process_unix_target "arm64" "release/mac-arm64"
            ;;
        universal)
            process_unix_target "universal" "release/mac-universal"
            ;;
    esac
fi

echo ""
echo "=========================================================="
echo " macOS package build finished!"
echo " Output files in: release/"
ls -lh release/*.dmg 2>/dev/null || true
[ "$WANT_ZIP" = true ] && ls -lh release/*-mac-*.zip 2>/dev/null || true
echo "=========================================================="
