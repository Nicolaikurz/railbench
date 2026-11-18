# Flathub Submission & Flatpak Packaging Guide for RAILbench

This guide explains how **RAILbench** (`io.github.Nicolaikurz.railbench`) is packaged as a Flatpak and submitted to [Flathub](https://flathub.org).

---

## 1. Package Overview & Standards Compliance

| Component | Value | Notes |
| :--- | :--- | :--- |
| **App ID** | `io.github.Nicolaikurz.railbench` | Reverse-DNS notation matching Flathub naming standards |
| **Base App** | `org.electronjs.Electron2.BaseApp//24.08` | Includes `zypak` sandbox wrapper for Electron |
| **Runtime / SDK** | `org.freedesktop.Platform//24.08` / `Sdk//24.08` | Long-Term Support Freedesktop runtime |
| **Launcher** | `railbench.sh` | Launches via `zypak-wrapper` with `--ozone-platform-hint=auto` (Wayland + X11) |
| **Icon** | `512x512` PNG | `packaging/flatpak/icon.png` installed to `/app/share/icons/hicolor/512x512/apps/` |
| **Desktop Entry** | `io.github.Nicolaikurz.railbench.desktop` | Passes `desktop-file-validate` |
| **Metainfo** | `io.github.Nicolaikurz.railbench.metainfo.xml` | Conforms to AppStream 1.0; passes `appstreamcli validate` |

### Permissions (`finish-args`)
- `--socket=wayland` & `--socket=x11`: Native Wayland display support with transparent X11 fallback.
- `--device=dri`: Hardware-accelerated 3D rendering (WebGL / Three.js).
- `--share=ipc`: Required for X11 shared memory.
- `--share=network`: Required for real-time WebSocket multi-user collaboration (`ws://`).
- `--filesystem=xdg-documents` & `--filesystem=xdg-download`: For opening, importing, and saving `.gleisplan`, `.json`, `.xml`, and Bill of Materials `.csv` files.
- `--talk-name=org.freedesktop.Notifications`: System toast notifications.

---

## 2. Directory Structure

```
packaging/flatpak/
├── io.github.Nicolaikurz.railbench.desktop      # Validated XDG desktop launcher entry
├── io.github.Nicolaikurz.railbench.metainfo.xml # Validated AppStream metadata with screenshots & releases
├── io.github.Nicolaikurz.railbench.yml          # Manifest for local offline development builds
├── io.github.Nicolaikurz.railbench.flathub.yml  # Production manifest for the Flathub build bots
├── flathub.json                        # Configures target architectures (x86_64, aarch64)
├── icon.png                            # 512x512 application icon
└── railbench.sh                        # Electron startup wrapper script with Wayland ozone flags
```

---

## 3. Local Flatpak Build & Test

You can build and test the Flatpak bundle on your local Linux machine at any time using the automated script:

```bash
# 1. Ensure flatpak and flatpak-builder are installed
# Ubuntu / Debian: sudo apt install flatpak flatpak-builder
# Fedora:          sudo dnf install flatpak flatpak-builder
# Arch:            sudo pacman -S flatpak flatpak-builder

# 2. Run the automated build script
npm run dist:linux:flatpak
# or
bash scripts/build-flatpak.sh
```

This script:
1. Compiles TypeScript and packages the Linux binary via `electron-builder --linux dir`.
2. Installs the Freedesktop 24.08 and Electron BaseApp runtimes.
3. Builds the Flatpak package into `release/flatpak-repo`.
4. Exports a single-file bundle to `release/Railbench.flatpak`.

### Installing and Running the Local Build:
```bash
flatpak install --user release/Railbench.flatpak
flatpak run io.github.Nicolaikurz.railbench
```

---

## 4. Preparing a GitHub Release for Flathub

Flathub build bots build in an isolated container without internet access. Because RAILbench is an Electron application, Flathub downloads the prebuilt Linux release tarballs directly from your GitHub Releases.

### Step 1: Create Linux release archives
Run:
```bash
npm run build:all
npx electron-builder --linux dir --x64
tar -czf release/railbench-0.6.0-linux-x64.tar.gz -C release/linux-unpacked .

# If building for arm64 as well:
npx electron-builder --linux dir --arm64
tar -czf release/railbench-0.6.0-linux-arm64.tar.gz -C release/linux-arm64-unpacked .
```

### Step 2: Compute SHA-256 Checksums
```bash
sha256sum release/railbench-0.6.0-linux-x64.tar.gz
sha256sum release/railbench-0.6.0-linux-arm64.tar.gz
```

### Step 3: Publish GitHub Release
1. Tag the release:
   ```bash
   git tag v0.6.0
   git push origin v0.6.0
   ```
2. On GitHub (`https://github.com/Nicolaikurz/railbench/releases/new`), create release `v0.6.0` and upload:
   - `railbench-0.6.0-linux-x64.tar.gz`
   - `railbench-0.6.0-linux-arm64.tar.gz`
   - AppImages, Windows executables, and macOS DMGs/ZIPs

---

## 5. Submitting to Flathub

### Step 1: Fork the Flathub submission repository
Visit [https://github.com/flathub/flathub](https://github.com/flathub/flathub) and click **Fork**.

### Step 2: Clone your fork and create a new branch
```bash
git clone https://github.com/<YOUR-GITHUB-USERNAME>/flathub.git
cd flathub
git checkout -b new-pr-io.github.Nicolaikurz.railbench
```

### Step 3: Add RAILbench files
Copy the prepared files from `packaging/flatpak/` to the branch root:
- Copy `packaging/flatpak/io.github.Nicolaikurz.railbench.flathub.yml` and rename it to `io.github.Nicolaikurz.railbench.yml`.
- Edit `io.github.Nicolaikurz.railbench.yml` and insert the exact `sha256` checksums from Step 4.2:
  ```yaml
  sources:
    - type: archive
      url: https://github.com/Nicolaikurz/railbench/releases/download/v0.6.0/railbench-0.6.0-linux-x64.tar.gz
      sha256: <ACTUAL_SHA256_HEX_FOR_X64>
      only-arches:
        - x86_64
    - type: archive
      url: https://github.com/Nicolaikurz/railbench/releases/download/v0.6.0/railbench-0.6.0-linux-arm64.tar.gz
      sha256: <ACTUAL_SHA256_HEX_FOR_ARM64>
      only-arches:
        - aarch64
  ```
- Copy `flathub.json`
- Copy `railbench.sh`
- Copy `io.github.Nicolaikurz.railbench.desktop`
- Copy `io.github.Nicolaikurz.railbench.metainfo.xml`
- Copy `icon.png`

### Step 4: Verify AppStream and Desktop Validation
```bash
appstreamcli validate packaging/flatpak/io.github.Nicolaikurz.railbench.metainfo.xml
desktop-file-validate packaging/flatpak/io.github.Nicolaikurz.railbench.desktop
```

### Step 5: Commit and Open Pull Request
```bash
git add .
git commit -m "Add io.github.Nicolaikurz.railbench"
git push origin new-pr-io.github.Nicolaikurz.railbench
```
Open a Pull Request on [https://github.com/flathub/flathub/pulls](https://github.com/flathub/flathub/pulls).

### Step 6: Review & Flathub Bot Testing
1. Flathub's automated bot (`@flathubbot`) will comment on the PR and initiate test builds for `x86_64` and `aarch64`.
2. Reviewers from Flathub will inspect the manifest, permissions, and AppStream metadata.
3. Once approved, the PR is merged, and Flathub creates a new dedicated repository `https://github.com/flathub/io.github.Nicolaikurz.railbench`.
4. Your application will then appear on [Flathub](https://flathub.org/apps/io.github.Nicolaikurz.railbench)!
