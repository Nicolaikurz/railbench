# Build- & Ausführungs-Anleitung: Windows, macOS & Linux (Release 0.6.0)

Diese Anleitung beschreibt, wie die vorkompilierten Binaries unter **Windows**, **macOS** und **Linux** ausgeführt werden und wie eigenständige Pakete für **x86_64 (`x64`)** und **ARM64 (`arm64`)** aus dem Quellcode gebaut werden.

---

> [!IMPORTANT]
> **Release 0.6.0 Multi-Plattform-Binaries**:
> In Release **0.6.0** werden Pakete für folgende Plattformen bereitgestellt:
> * ✅ **Windows `x64` / `arm64`**: Portable EXE (`railbench-0.6.0-portable-*.exe`) & entpacktes Portable-Verzeichnis (`win-unpacked`).
> * ✅ **macOS `arm64` (Apple Silicon M1/M2/M3/M4) & `x64` (Intel)**: `.zip`-App-Bundle (`railbench-0.6.0-mac-*.zip`) & `.dmg`-Installer.
> * ✅ **Linux `x86_64` / `arm64`**: AppImage (`railbench-0.6.0-*.AppImage`) & Flatpak (`Railbench.flatpak`).

---

## Schnellstart: Vorkompilierte Binaries ausführen

### 1. Windows (`x64` & `arm64`)
1. Lade `railbench-0.6.0-portable-x64.exe` (Intel/AMD) bzw. `railbench-0.6.0-portable-arm64.exe` (ARM) herunter oder entpacke das portable Verzeichnis (`win-unpacked`).
2. Starte `railbench-0.6.0-portable-x64.exe` bzw. `railbench.exe` per Doppelklick (keine Installation oder Administratorrechte nötig).
3. **Hinweis zu Windows SmartScreen**: Falls Windows Defender SmartScreen beim ersten Start *„Der PC wurde durch Windows geschützt“* anzeigt, klicke auf **Weitere Informationen** (*More info*) $\rightarrow$ **Trotzdem ausführen** (*Run anyway*).

### 2. macOS (Apple Silicon `arm64` & Intel `x64`)
1. Entpacke `railbench-0.6.0-mac-arm64.zip` (Apple Silicon M1–M4) bzw. `railbench-0.6.0-mac-x64.zip` (Intel) per Doppelklick oder öffne die `.dmg`-Datei und verschiebe **`railbench.app`** in den Ordner `/Applications`.
2. **Hinweis zu macOS Gatekeeper (Unsigniertes Open-Source-Binary)**:
   Da das kostenlose Open-Source-Release ohne kostenpflichtiges Apple-Developer-Zertifikat gebaut wird, blockiert macOS Gatekeeper ggf. den ersten Start (*„Entwickler kann nicht verifiziert werden“* oder *„ist beschädigt“*).
   * **Option A – Per Terminal (empfohlen)**:
     ```bash
     xattr -cr /Applications/railbench.app
     open /Applications/railbench.app
     ```
   * **Option B – Im Finder**:
     **Rechtsklick** (bzw. <kbd>Ctrl</kbd>-Klick) auf `railbench.app` $\rightarrow$ **Öffnen** wählen $\rightarrow$ im Dialog mit **Öffnen** bestätigen.

### 3. Linux (`x86_64` & `arm64`)
* **AppImage**:
  ```bash
  chmod +x release/railbench-0.6.0-x64.AppImage
  ./release/railbench-0.6.0-x64.AppImage
  ```
* **Flatpak**:
  ```bash
  flatpak install --user release/Railbench.flatpak
  flatpak run de.railbench.Railbench
  ```

---

## Übersicht der Build-Ziele & Skripte

| Plattform / Format | Architektur | npm Script | Shell / Batch Script | Ausgabe-Datei | Beschreibung |
|---|---|---|---|---|---|
| **Windows Portable** | x86_64 | `npm run dist:win:portable:x64` | `scripts/build-windows-portable.sh x64` | `release/railbench-0.6.0-portable-x64.exe` | Standalone-EXE für Intel/AMD PCs |
| **Windows Portable** | ARM64 | `npm run dist:win:portable:arm64` | `scripts/build-windows-portable.sh arm64` | `release/railbench-0.6.0-portable-arm64.exe` | Für Snapdragon X & ARM-Windows |
| **Windows Portable (Alle)** | x64 + arm64 | `npm run dist:win:portable:all` | `scripts/build-windows-portable.sh all` | `release/railbench-0.6.0-portable-*.exe` | Baut beide Windows-Architekturen |
| **Linux AppImage** | x86_64 | `npm run dist:linux:appimage:x64` | `scripts/build-linux-appimage.sh x64` | `release/railbench-0.6.0-x64.AppImage` | Universal-Binärdatei für Linux x64 |
| **Linux AppImage** | ARM64 | `npm run dist:linux:appimage:arm64` | `scripts/build-linux-appimage.sh arm64` | `release/railbench-0.6.0-arm64.AppImage` | Für Raspberry Pi 4/5, Asahi Linux |
| **Linux AppImage (Alle)** | x64 + arm64 | `npm run dist:linux:appimage:all` | `scripts/build-linux-appimage.sh all` | `release/railbench-0.6.0-*.AppImage` | Baut beide Linux-Architekturen |
| **Linux Flatpak** | x86_64 | `npm run dist:linux:flatpak` | `scripts/build-flatpak.sh` | `release/Railbench.flatpak` | Sandbox-Paket (Wayland/X11) |
| **macOS Package (ZIP)** | ARM64 | `npm run dist:mac:zip:arm64` | `scripts/build-macos.sh arm64 zip` | `release/railbench-0.6.0-mac-arm64.zip` | Für Apple Silicon (M1/M2/M3/M4) |
| **macOS Package (ZIP)** | x86_64 | `npm run dist:mac:zip:x64` | `scripts/build-macos.sh x64 zip` | `release/railbench-0.6.0-mac-x64.zip` | Für Intel-basierte Macs |
| **macOS Packages (ZIP Alle)** | x64 + arm64 | `npm run dist:mac:zip:all` | `scripts/build-macos.sh all zip` | `release/railbench-0.6.0-mac-*.zip` | Baut beide macOS-Architekturen |
| **macOS DMG (Apple Silicon)** | ARM64 | `npm run dist:mac:dmg:arm64` | `scripts/build-macos.sh arm64 dmg` | `release/railbench-0.6.0-mac-arm64.dmg` | DMG-Installer für Apple Silicon |
| **macOS DMG (Intel)** | x86_64 | `npm run dist:mac:dmg:x64` | `scripts/build-macos.sh x64 dmg` | `release/railbench-0.6.0-mac-x64.dmg` | DMG-Installer für Intel Macs |
| **macOS DMG (Alle)** | x64 + arm64 | `npm run dist:mac:dmg:all` | `scripts/build-macos.sh all dmg` | `release/railbench-0.6.0-mac-*.dmg` | Baut beide DMG-Installer |

---

## 1. Voraussetzungen zum Selbstbauen

* **Node.js**: Version $\ge 20.x$
* **npm**: Version $\ge 10.x$

Vor dem ersten Build einmalig alle Abhängigkeiten installieren:
```bash
npm install
```

---

## 2. Windows Portable Build (`.exe`)

```bash
# Beide Architekturen (x64 und ARM64) bauen:
npm run dist:win:portable:all

# Nur x86_64 (Standard Intel/AMD):
npm run dist:win:portable:x64

# Nur ARM64:
npm run dist:win:portable:arm64
```

---

## 3. Linux AppImage & Flatpak Build

### AppImage bauen:
```bash
npm run dist:linux:appimage:all
# oder einzeln:
npm run dist:linux:appimage:x64
npm run dist:linux:appimage:arm64
```

### Flatpak bauen:
```bash
npm run dist:linux:flatpak
# oder direkt:
./scripts/build-flatpak.sh
```

---

## 4. macOS Build (`.zip` & `.dmg`)

```bash
# Beide Architekturen (Apple Silicon arm64 & Intel x64) als ZIP verpacken:
npm run dist:mac:zip:all

# Auf nativem macOS zusätzlich als DMG-Installer bauen:
npm run dist:mac:dmg:all
```

---

## 5. Fehlerbehebung (Troubleshooting)

* **Windows SmartScreen**: Klicke beim ersten Start auf *„Weitere Informationen“* $\rightarrow$ *„Trotzdem ausführen“*.
* **macOS Gatekeeper**: Führe `xattr -cr /Applications/railbench.app` im Terminal aus oder öffne die App per Rechtsklick $\rightarrow$ *Öffnen*.
* **Linux AppImage ohne FUSE 2**: Installiere `libfuse2` (`sudo apt install libfuse2`) oder starte das AppImage mit `--appimage-extract-and-run`.
