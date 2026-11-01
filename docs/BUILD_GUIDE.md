# Build-Anleitung: Windows, Linux & macOS (Release 0.4.0)

Diese Anleitung beschreibt Schritt für Schritt, wie eigenständige ausführbare Pakete für **Windows (Portable EXE)**, **Linux (AppImage & Flatpak)** und **macOS (ZIP & DMG)** für **x86_64** und **ARM64** gebaut und verteilt werden.

---

> [!IMPORTANT]
> **Release 0.4 Testing Notice (Wichtiger Hinweis zur Testabdeckung)**:
> In Release **0.4.0** wurden die Binärpakete für folgende Plattformen bereitgestellt:
> * ✅ **Windows x86_64 / ARM64**: Portable EXE.
> * ✅ **Linux x86_64 / ARM64**: AppImage & Flatpak.
> * ✅ **macOS ARM64 (Apple Silicon M1/M2/M3/M4) / x64**: ZIP-Paket & DMG.

---

## Übersicht der Build-Ziele & Skripte

| Plattform / Format | Architektur | npm Script | Shell / Batch Script | Ausgabe-Datei | Beschreibung |
|---|---|---|---|---|---|
| **Windows Portable** | x86_64 | `npm run dist:win:portable:x64` | `scripts/build-windows-portable.sh x64` | `release/Gleisplan-0.4.0-portable-x64.exe` | Standalone-EXE für Intel/AMD PCs |
| **Windows Portable** | ARM64 | `npm run dist:win:portable:arm64` | `scripts/build-windows-portable.sh arm64` | `release/Gleisplan-0.4.0-portable-arm64.exe` | Für Snapdragon X & ARM-Windows |
| **Windows Portable (Alle)** | x64 + arm64 | `npm run dist:win:portable:all` | `scripts/build-windows-portable.sh all` | `release/Gleisplan-0.4.0-portable-*.exe` | Baut beide Windows-Architekturen |
| **Linux AppImage** | x86_64 | `npm run dist:linux:appimage:x64` | `scripts/build-linux-appimage.sh x64` | `release/Gleisplan-0.4.0-x64.AppImage` | Universal-Binärdatei für Linux x64 |
| **Linux AppImage** | ARM64 | `npm run dist:linux:appimage:arm64` | `scripts/build-linux-appimage.sh arm64` | `release/Gleisplan-0.4.0-arm64.AppImage` | Für Raspberry Pi 4/5, Asahi Linux |
| **Linux AppImage (Alle)** | x64 + arm64 | `npm run dist:linux:appimage:all` | `scripts/build-linux-appimage.sh all` | `release/Gleisplan-0.4.0-*.AppImage` | Baut beide Linux-Architekturen |
| **Linux Flatpak** | x86_64 | `npm run dist:linux:flatpak` | `scripts/build-flatpak.sh` | `release/Railbench.flatpak` | Sandbox-Paket (Wayland/X11) |
| **macOS Package** | ARM64 | `npm run dist:mac:zip:arm64` | `scripts/build-macos.sh arm64` | `release/Gleisplan-0.4.0-mac-arm64.zip` | Für Apple Silicon (M1/M2/M3/M4) |
| **macOS Package** | x86_64 | `npm run dist:mac:zip:x64` | `scripts/build-macos.sh x64` | `release/Gleisplan-0.4.0-mac-x64.zip` | Für Intel-basierte Macs |
| **macOS Packages (Alle)** | x64 + arm64 | `npm run dist:mac:zip:all` | `scripts/build-macos.sh all` | `release/Gleisplan-0.4.0-mac-*.zip` | Baut beide macOS-Architekturen |
| **macOS DMG (Installer)** | x64 / arm64 | `npm run dist:mac:dmg` | `scripts/build-macos.sh` (auf macOS) | `release/Gleisplan-0.4.0-mac-*.dmg` | Native DMG-Erstellung auf macOS |

---

## 1. Voraussetzungen

* **Node.js**: Version $\ge 20.x$
* **npm**: Version $\ge 10.x$

Vor dem ersten Build einmalig alle Abhängigkeiten installieren:
```bash
npm install
```

---

## 2. Windows Portable Build (.exe)

Der Windows Portable Build erzeugt eine einzelne `.exe`-Datei, die ohne Installationsassistenten, ohne Administratorrechte und ohne Registry-Einträge direkt ausgeführt werden kann.

### A. Bauen unter Linux (Cross-Compilation)
`electron-builder` kann Windows Portable EXEs direkt unter Linux ohne zusätzliche Software (kein Wine zwingend nötig) kompilieren:

```bash
# Beide Architekturen (x64 und ARM64) bauen:
npm run dist:win:portable:all
# oder: ./scripts/build-windows-portable.sh

# Nur x86_64 (Standard Intel/AMD):
npm run dist:win:portable:x64

# Nur ARM64:
npm run dist:win:portable:arm64
```

### B. Bauen unter Windows
Unter Windows in der Eingabeaufforderung (CMD) oder PowerShell:

```cmd
npm run dist:win:portable:all
# oder per Batch-Datei:
scripts\build-windows-portable.bat
```

### Ergebnis:
Die fertigen Dateien liegen in `release/`:
* `release/Gleisplan-0.4.0-portable-x64.exe`
* `release/Gleisplan-0.4.0-portable-arm64.exe`

---

## 3. Linux AppImage Build (.AppImage)

Ein AppImage ist eine eigenständige ausführbare Linux-Anwendung, die auf Ubuntu, Debian, Fedora, Arch Linux, openSUSE und anderen Distributionen ohne Installation läuft.

### Bauen:
```bash
# Beide Architekturen (x64 und ARM64) bauen:
npm run dist:linux:appimage:all
# oder: ./scripts/build-linux-appimage.sh

# Nur x86_64:
npm run dist:linux:appimage:x64

# Nur ARM64:
npm run dist:linux:appimage:arm64
```

### Ausführen des AppImages:
```bash
chmod +x release/Gleisplan-0.4.0-x64.AppImage
./release/Gleisplan-0.4.0-x64.AppImage
```

> [!NOTE]
> Auf einigen neueren Distributionen (z. B. Ubuntu 24.04) wird `libfuse2` benötigt:
> `sudo apt install libfuse2`

---

## 4. Linux Flatpak Build

Flatpak isoliert die Anwendung in einer sicheren Sandbox und ist der bevorzugte Standard für modernes Desktop-Linux (Flathub, Steam Deck, Fedora Silverblue, etc.).

### A. Voraussetzungen auf dem Host-System
Installiere `flatpak` und `flatpak-builder`:
* **Arch Linux / Manjaro**:
  ```bash
  sudo pacman -S flatpak flatpak-builder
  ```
* **Ubuntu / Debian**:
  ```bash
  sudo apt install flatpak flatpak-builder
  ```
* **Fedora**:
  ```bash
  sudo dnf install flatpak flatpak-builder
  ```

Einmalig die Freedesktop- und Electron-Laufzeitumgebungen installieren:
```bash
flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.repo
flatpak install flathub org.freedesktop.Platform//24.08 org.freedesktop.Sdk//24.08 org.electronjs.Electron2.BaseApp//24.08
```

### B. Flatpak automatisch bauen
Führe das mitgelieferte Build-Skript aus:
```bash
# Per npm Script:
npm run dist:linux:flatpak

# Oder direkt:
./scripts/build-flatpak.sh
```

Das Skript führt automatisch folgende Schritte durch:
1. Erstellt das entpackte Linux-Binary (`npm run dist:linux:dir`).
2. Überprüft die Flatpak-Tools und Runtimes.
3. Baut das Paket über das Manifest [`packaging/flatpak/de.railbench.Railbench.yml`](file:///home/nicolai/Documents/Projects/railbench/packaging/flatpak/de.railbench.Railbench.yml).
4. Erzeugt die eigenständige Bundle-Datei `release/Railbench.flatpak`.

### C. Flatpak installieren und ausführen

**Aus dem generierten Bundle installieren:**
```bash
flatpak install --user release/Railbench.flatpak
```

**Anwendung starten:**
```bash
flatpak run de.railbench.Railbench
```

**Wieder deinstallieren:**
```bash
flatpak uninstall --user de.railbench.Railbench
```

---

## 5. macOS Build (.zip & .dmg)

Unterstützt werden moderne **Apple Silicon Macs (M1/M2/M3/M4, `arm64`)** sowie ältere **Intel Macs (`x64`)**.

### A. Bauen auf Linux (Cross-Compilation)
Auf Linux kann `electron-builder` native macOS `.app`-Bündel erstellen und in `.zip`-Archive verpacken:

```bash
# Beide Architekturen (Apple Silicon arm64 & Intel x64) als ZIP verpacken:
npm run dist:mac:zip:all
# oder: ./scripts/build-macos.sh all

# Nur Apple Silicon:
npm run dist:mac:zip:arm64
# oder: ./scripts/build-macos.sh arm64

# Nur Intel:
npm run dist:mac:zip:x64
# oder: ./scripts/build-macos.sh x64
```

> [!NOTE]
> Das Erstellen von `.dmg`-Dateien unter Linux erfordert Apples proprietäres Tool `sips`, welches nur auf macOS verfügbar ist. Daher erzeugt der Linux-Cross-Compile standardmäßig signierbare `.zip`-Archive mit der vollständigen macOS `.app`.

### B. Bauen auf macOS (Nativ)
Direkt auf macOS können sowohl `.zip` als auch `.dmg` (Disk Image Installer mit Drag-and-Drop in den Applications-Ordner) erstellt werden:

```bash
# Automatisches Skript (erkennt macOS und baut DMG + ZIP für beide Architekturen):
./scripts/build-macos.sh all

# Oder per npm:
npm run dist:mac:dmg
```

### C. macOS Sicherheit & Gatekeeper
Da die Builds für Open-Source-Zwecke unentgeltlich ohne kostenpflichtiges Apple Developer Zertifikat erstellt werden, blockiert macOS Gatekeeper die App beim ersten Start eventuell ("Entwickler kann nicht verifiziert werden").

Um die App auf macOS freizugeben:
1. Im Finder mit **Rechtsklick** (oder Control-Klick) auf `Gleisplan.app` klicken $\rightarrow$ **Öffnen** wählen $\rightarrow$ Im Dialog auf **Öffnen** bestätigen.
2. Alternativ über das Terminal:
   ```bash
   xattr -cr /Applications/Gleisplan.app
   ```

---

## 6. Architektur-Unterschiede (x86_64 vs. ARM64)

| Plattform | `x64` (x86_64) | `arm64` (AArch64) |
|---|---|---|
| **Windows** | Standard für klassische PCs und Laptops mit Intel Core oder AMD Ryzen Prozessoren. | Für neuere Windows-Geräte mit ARM-Prozessoren (z. B. Qualcomm Snapdragon X Elite, Microsoft Surface Pro ARM). |
| **Linux** | Standard-Desktop- und Notebook-Distributionen (Ubuntu, Arch, Fedora, Debian). | Für Single-Board-Computer (Raspberry Pi 4 / 5), Asahi Linux auf Apple Hardware oder Linux-ARM-Server/Laptops. |
| **macOS** | Ältere Macs mit Intel-Prozessoren (bis ca. 2020). Läuft auch via Rosetta 2 auf Apple Silicon. | Native Ausführung auf modernen Macs mit Apple Silicon (M1, M2, M3, M4). Bietet maximale Performance und Energieeffizienz. |

---

## 7. Struktur der Packaging-Dateien

```
railbench/
├── packaging/
│   └── flatpak/
│       ├── de.railbench.Railbench.yml          # Flatpak-Manifest
│       ├── de.railbench.Railbench.desktop      # Desktop-Starter
│       ├── de.railbench.Railbench.metainfo.xml  # AppStream Metadaten
│       └── railbench.sh                        # Zypak/Electron-Launcher-Wrapper
├── scripts/
│   ├── build-windows-portable.sh               # Windows Portable Build (Linux/macOS)
│   ├── build-windows-portable.bat              # Windows Portable Build (Windows CMD)
│   ├── build-linux-appimage.sh                 # Linux AppImage Build
│   ├── build-macos.sh                          # macOS ZIP / DMG Build
│   └── build-flatpak.sh                        # Linux Flatpak Build
├── build/
│   └── icon.png                                # 512x512 Anwendungs-Icon
├── release/                                    # Ausgabe-Verzeichnis für Binaries
└── package.json                                # Build- & Target-Konfigurationen
```

---

## 8. Fehlerbehebung (Troubleshooting)

### Windows Portable: Antiviren-Warnung ("SmartScreen")
Da das Binary nicht mit einem kommerziellen Zertifikat digital signiert ist, meldet Windows Defender SmartScreen beim ersten Start eventuell "Unbekannter Herausgeber". Klicke auf *"Weitere Informationen"* $\rightarrow$ *"Trotzdem ausführen"*.

### Linux AppImage startet nicht: FUSE Fehler
Wenn das System meldet: `dlopen(): error loading libfuse.so.2`:
* Entweder FUSE 2 installieren (`sudo apt install libfuse2`).
* Oder das AppImage mit `--appimage-extract-and-run` aufrufen:
  ```bash
  ./release/Gleisplan-0.4.0-x64.AppImage --appimage-extract-and-run
  ```

### Wayland & Hardware-Beschleunigung
Die Linux-Pakete (sowohl AppImage als auch Flatpak) unterstützen native Wayland-Darstellung über die Chromium/Electron-Flags:
```bash
./release/Gleisplan-0.4.0-x64.AppImage --ozone-platform-hint=auto --enable-features=WaylandWindowDecorations
```
