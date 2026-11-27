# Release (Native Rust)

## Downloads

| Platform | Architecture | Package |
|---|---|---|
| Linux | x86_64 | Native binary / `.AppImage` / `.flatpak` |
| Linux | ARM64 (`aarch64`) | Native binary / `.AppImage` / `.flatpak` |
| Windows | x86_64 / ARM64 | Portable ZIP (`railbench.exe`) |
| macOS | Apple Silicon (`arm64`) / Intel (`x64`) | `.dmg` / `.zip` (`RAILbench.app`) |

## Installation

### Linux (Native binary, AppImage, or Flatpak)

* **Native standalone binary**:
  ```bash
  chmod +x ./railbench-<version>-native-linux-x86_64
  ./railbench-<version>-native-linux-x86_64
  ```
* **AppImage**:
  ```bash
  chmod +x ./railbench-<version>-x64.AppImage
  ./railbench-<version>-x64.AppImage
  ```
  *(If `libfuse.so.2` is missing on Ubuntu 24.04+, install `libfuse2` or run with `--appimage-extract-and-run`.)*
* **Flatpak**:
  ```bash
  flatpak install --user release/railbench-<version>-x86_64.flatpak
  flatpak run io.github.Nicolaikurz.railbench
  ```

### Windows (Portable ZIP)

1. Download the ZIP matching your CPU (`x64` or `arm64`).
2. Extract it to any folder (no installation or admin rights needed).
3. Double-click `railbench.exe`.

If Windows SmartScreen shows a warning, click **More info -> Run anyway** (unsigned open-source binary).

### macOS (Apple Silicon & Intel)

1. Open the `.dmg` or unzip the macOS `.zip`.
2. Move `RAILbench.app` to `/Applications`.
3. On first launch, right-click the app and choose **Open** (or clear quarantine for unsigned open-source builds):
   ```bash
   xattr -cr /Applications/RAILbench.app
   open /Applications/RAILbench.app
   ```

## Reporting Bugs

Please open an issue and include your OS, architecture, GPU backend, and app version (shown in the top title bar).
