# Flathub Submission & Flatpak Packaging Guide for RAILbench

This guide explains how the native Rust **RAILbench** desktop application (`io.github.Nicolaikurz.railbench`) is packaged as a Flatpak and submitted to [Flathub](https://flathub.org).

---

## 1. Package Overview & Standards Compliance

| Component | Value | Notes |
| :--- | :--- | :--- |
| **App ID** | `io.github.Nicolaikurz.railbench` | Reverse-DNS notation matching Flathub naming standards |
| **Runtime / SDK** | `org.freedesktop.Platform//24.08` / `Sdk//24.08` | Long-Term Support Freedesktop runtime |
| **SDK Extension (Flathub)** | `org.freedesktop.Sdk.Extension.rust-stable` | Compiles the native Rust binary offline in Flathub CI |
| **Command** | `/app/bin/railbench` | Native compiled `railbench-app` executable (no wrapper required) |
| **Icon** | `512x512` PNG | `packaging/flatpak/icon.png` installed to `/app/share/icons/hicolor/512x512/apps/` |
| **Desktop Entry** | `io.github.Nicolaikurz.railbench.desktop` | Passes `desktop-file-validate` |
| **Metainfo** | `io.github.Nicolaikurz.railbench.metainfo.xml` | Conforms to AppStream 1.0; passes `appstreamcli validate` |

### Permissions (`finish-args`)
- `--socket=wayland` & `--socket=x11`: Native Wayland display support with X11 support.
- `--device=dri`: Hardware-accelerated 3D rendering (Vulkan / OpenGL via `wgpu`).
- `--share=ipc`: Shared memory for X11/rendering.
- `--share=network`: Real-time WebSocket multi-user collaboration (`ws://`).
- `--filesystem=xdg-documents` & `--filesystem=xdg-download`: Opening, importing, and saving `.railbench`, `.xml`, PDF blueprints, and `.csv` part lists.
- `--talk-name=org.freedesktop.Notifications`: Desktop notifications.

---

## 2. Directory Structure

```
packaging/flatpak/
├── io.github.Nicolaikurz.railbench.desktop      # XDG desktop launcher entry
├── io.github.Nicolaikurz.railbench.metainfo.xml # AppStream metadata with screenshots & releases
├── io.github.Nicolaikurz.railbench.yml          # Manifest for local offline builds
├── io.github.Nicolaikurz.railbench.flathub.yml  # Flathub manifest building from source with rust-stable
├── flathub.json                                 # Target architectures (x86_64, aarch64)
└── icon.png                                     # 512x512 application icon
```

---

## 3. Local Flatpak Build & Test

```bash
# 1. Ensure flatpak and flatpak-builder are installed
# Arch Linux:      sudo pacman -S flatpak flatpak-builder
# Ubuntu / Debian: sudo apt install flatpak flatpak-builder
# Fedora:          sudo dnf install flatpak flatpak-builder

# 2. Build the native Rust Flatpak bundle (x64, arm64, or all)
./scripts/build-flatpak.sh x64
```

This script:
1. Compiles the Rust release binary (`cargo build --release --manifest-path rust/Cargo.toml -p railbench-app --target ...`).
2. Installs `org.freedesktop.Platform//24.08` and `org.freedesktop.Sdk//24.08` if missing.
3. Builds the Flatpak repository into `release/flatpak-repo`.
4. Exports a standalone bundle at `release/railbench-<YY.MM.P>-<arch>.flatpak`.

### Installing and Running Locally:
```bash
flatpak install --user release/railbench-26.11.1-x86_64.flatpak
flatpak run io.github.Nicolaikurz.railbench
```

---

## 4. Validation & Flathub Submission

Before submitting or updating the Flathub PR:
```bash
appstreamcli validate packaging/flatpak/io.github.Nicolaikurz.railbench.metainfo.xml
desktop-file-validate packaging/flatpak/io.github.Nicolaikurz.railbench.desktop
```

In the Flathub repository PR, copy `io.github.Nicolaikurz.railbench.flathub.yml` as `io.github.Nicolaikurz.railbench.yml` (referencing the tagged git commit/archive plus offline Cargo vendor sources if required by Flathub's `flatpak-cargo-generator`), alongside `flathub.json`, `.desktop`, `.metainfo.xml`, and `icon.png`.
