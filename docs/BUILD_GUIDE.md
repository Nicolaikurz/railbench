# Build & Binary Execution Guide: Windows, macOS & Linux (Native Rust)

This guide explains how to run and package the **native Rust** RAILbench desktop application (`rust/railbench-core` + `rust/railbench-app`) on **Linux**, **Windows**, and **macOS**.

---

## 1. Prerequisites

* **Rust toolchain** (via `rustup` — stable `cargo` and `rustc`)
* Optional `RAILBENCH_VERSION` environment variable to override the CalVer tag (defaults to `<YY>.<MM>.<patch>` from `rust/Cargo.toml`)

Platform-specific packaging tools:
* **Linux Native Binary**: `cargo` (plus Wayland/X11/GTK3/Vulkan libs)
* **Linux AppImage**: `appimagetool` on `PATH` or at `~/.local/bin/appimagetool`
* **Linux Flatpak**: `flatpak-builder` and `org.freedesktop.Platform` / `org.freedesktop.Sdk` (`24.08`)
* **Linux Snap**: `snapcraft`
* **Windows Portable ZIP**: Windows MSVC toolchain natively (`scripts/build-windows-portable.bat`), or `cargo-xwin` / `x86_64-w64-mingw32-gcc` when cross-compiling from Linux/macOS (`./scripts/build-windows-portable.sh`)
* **macOS `.dmg` & `.zip`**: Xcode Command Line Tools on macOS (or `cargo-zigbuild` + `zig` + optional `rcodesign` / `genisoimage` on Linux)

---

## 2. Native Binary Build (Current Host)

```bash
./scripts/build-rust-native.sh
# Outputs: release/railbench-<YY.MM.P>-native-<os>-<arch>
```

You can also pass a specific Rust target triple:

```bash
./scripts/build-rust-native.sh x86_64-unknown-linux-gnu
```

---

## 3. Linux AppImage Build

```bash
# Download appimagetool once if not installed:
curl -fsSL -o ~/.local/bin/appimagetool \
  https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$(uname -m).AppImage
chmod +x ~/.local/bin/appimagetool

# Build AppImage (x64, arm64, or all):
./scripts/build-linux-appimage.sh x64
```

Run the resulting AppImage:

```bash
chmod +x release/railbench-*-x64.AppImage
./release/railbench-*-x64.AppImage
# If libfuse2 is missing on Ubuntu 24.04+:
./release/railbench-*-x64.AppImage --appimage-extract-and-run
```

---

## 4. Linux Flatpak Build

```bash
./scripts/build-flatpak.sh x64   # or arm64 | all

# Install and run locally:
flatpak install --user release/railbench-*-x86_64.flatpak
flatpak run io.github.Nicolaikurz.railbench
```

---

## 5. Windows Portable ZIP Build

On Windows (MSVC):

```bat
scripts\build-windows-portable.bat x64
```

From Linux/macOS (requires `cargo-xwin` + `rustup target add x86_64-pc-windows-msvc`, or `x86_64-w64-mingw32-gcc`):

```bash
./scripts/build-windows-portable.sh x64
```

Extract `release/railbench-<ver>-win-<arch>.zip` and double-click `railbench.exe` (no installation needed).

---

## 6. macOS `.dmg` & `.zip` Build

On macOS (native) or Linux (`cargo-zigbuild`):

```bash
./scripts/build-macos.sh           # all (x64 + arm64), dmg + zip
./scripts/build-macos.sh arm64 dmg # Apple Silicon DMG only
./scripts/build-macos.sh universal # Universal macOS binary (native macOS)
```

For unsigned open-source macOS builds on Apple Silicon, remove quarantine on first launch:

```bash
xattr -cr /Applications/RAILbench.app
open /Applications/RAILbench.app
```

---

## 7. Running Tests & Checking Targets

```bash
cd rust
cargo test
cargo build --release -p railbench-app
```
