# Release

## Downloads

| Platform | Architecture | Package | Status |
|----------|--------------|---------|--------|
| Linux | x86_64 | AppImage | ✅ Tested |
| Linux | ARM64 (aarch64) | AppImage | ✅ Tested |
| Windows | x86_64 | ZIP (portable) | ✅ Tested |
| Windows | ARM64 | ZIP (portable) | ⚠️ Not tested |
| macOS | Apple Silicon (ARM64) | ZIP | ⚠️ Not tested |

> **Note:** The Windows ARM64 and macOS builds have not been tested yet. They may not work as expected. Feedback and bug reports are welcome.

## Installation

### Linux (AppImage)

```bash
chmod +x ./<file>.AppImage
./<file>.AppImage
```

Use the ARM64 (`aarch64`) file on ARM devices.
If the AppImage does not start, make sure FUSE 2 is installed
(e.g. `sudo apt install libfuse2`), or run it with
`--appimage-extract-and-run`.

### Windows (portable ZIP)

1. Download the ZIP matching your CPU (x86_64 or ARM64).
2. Extract it to any folder (no installation or admin rights needed).
3. Run the `.exe` inside the extracted folder.

If Windows SmartScreen shows a warning, click **More info → Run anyway**
(the app is not code-signed).

### macOS (Apple Silicon)

1. Download and unzip the macOS ZIP.
2. Move the `.app` to your Applications folder.
3. On first launch, right-click the app and choose **Open**.

If macOS says the app is damaged or from an unidentified developer
(the app is not notarized), run:

```bash
xattr -cr /Applications/<app>.app
```

## Known issues

- Windows ARM64 and macOS builds are untested.
- Binaries are not code-signed, so Windows and macOS may show security warnings.

## Reporting bugs

Please open an issue and include your OS, architecture, and app version.