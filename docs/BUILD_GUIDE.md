# Build- & Ausführungs-Anleitung: Windows, macOS & Linux (Release 0.6.0)

Diese Anleitung beschreibt, wie die vorkompilierten Binaries unter **Windows**, **macOS** und **Linux** ausgeführt werden und wie eigenständige Pakete für **x86_64 (`x64`)** und **ARM64 (`arm64`)** aus dem Quellcode gebaut werden.

---

> [!IMPORTANT]
> **Release 0.6.0 Multi-Plattform-Binaries**:
> In Release **0.6.0** werden Pakete für folgende Plattformen bereitgestellt:
> * ✅ **Windows `x64` / `arm64`**: Portables Verzeichnis (`win-unpacked` / `win-arm64-unpacked`) & Portable ZIP-Archive (`railbench-0.6.0-portable-*.zip`) mit `railbench.exe`.
> * ✅ **macOS `arm64` (Apple Silicon M1/M2/M3/M4) & `x64` (Intel)**: `.zip`-App-Bundle (`railbench-0.6.0-mac-*.zip`) & `.dmg`-Installer sowie Homebrew Cask.
> * ✅ **Linux `x86_64` / `arm64`**: AppImage (`railbench-0.6.0-*.AppImage`) & Flatpak (`Railbench.flatpak`).

---

## Schnellstart: Vorkompilierte Binaries ausführen

### 1. Windows (`x64` & `arm64`) – Portables Verzeichnis (dir)
1. Lade `railbench-0.6.0-portable-x64.zip` (Intel/AMD) bzw. `railbench-0.6.0-portable-arm64.zip` (ARM) herunter und entpacke das Verzeichnis an einen beliebigen Ort (z. B. auf einen USB-Stick oder `C:\Tools\`).
2. Alternativ kann direkt der beim Build generierte Ordner `release/win-unpacked/` (x64) bzw. `release/win-arm64-unpacked/` (arm64) verwendet werden.
3. Starte **`railbench.exe`** per Doppelklick:
   * **Keine Installation** und **keine Administratorrechte** erforderlich.
   * **Schneller Sofortstart**, da nichts bei jedem Start temporär entpackt werden muss.
4. **Hinweis zu Windows SmartScreen**: Falls Windows Defender SmartScreen beim ersten Start *„Der PC wurde durch Windows geschützt“* anzeigt, klicke auf **Weitere Informationen** (*More info*) $\rightarrow$ **Trotzdem ausführen** (*Run anyway*).

### 2. macOS (Apple Silicon `arm64` & Intel `x64`)
* **Per Homebrew Cask (empfohlen)**:
  ```bash
  brew tap Nicolaikurz/railbench
  brew install --cask railbench
  ```
* **Manuelle Installation (DMG / ZIP)**:
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
| **Windows Portable (dir & ZIP)** | x86_64 | `npm run dist:win:portable:x64` | `scripts/build-windows-portable.sh x64` | `release/win-unpacked/` & `*.zip` | Portables Verzeichnis & ZIP für Intel/AMD PCs |
| **Windows Portable (dir & ZIP)** | ARM64 | `npm run dist:win:portable:arm64` | `scripts/build-windows-portable.sh arm64` | `release/win-arm64-unpacked/` & `*.zip` | Portables Verzeichnis & ZIP für ARM-Windows |
| **Windows Portable (Alle)** | x64 + arm64 | `npm run dist:win:portable:all` | `scripts/build-windows-portable.sh all` | `release/win-*-unpacked/` & `*.zip` | Baut beide portablen Windows-Architekturen |
| **Linux AppImage** | x86_64 | `npm run dist:linux:appimage:x64` | `scripts/build-linux-appimage.sh x64` | `release/railbench-0.6.0-x64.AppImage` | Universal-Binärdatei für Linux x64 |
| **Linux AppImage** | ARM64 | `npm run dist:linux:appimage:arm64` | `scripts/build-linux-appimage.sh arm64` | `release/railbench-0.6.0-arm64.AppImage` | Für Raspberry Pi 4/5, Asahi Linux |
| **Linux AppImage (Alle)** | x64 + arm64 | `npm run dist:linux:appimage:all` | `scripts/build-linux-appimage.sh all` | `release/railbench-0.6.0-*.AppImage` | Baut beide Linux-Architekturen |
| **Linux Flatpak** | x86_64 | `npm run dist:linux:flatpak` | `scripts/build-flatpak.sh` | `release/Railbench.flatpak` | Sandbox-Paket (Wayland/X11) |
| **macOS DMG (Apple Silicon)** | ARM64 | `npm run dist:mac:dmg:arm64` | `scripts/build-macos.sh arm64` | `release/railbench-0.6.0-mac-arm64.dmg` | DMG-Installer für Apple Silicon (M1/M2/M3/M4) |
| **macOS DMG (Intel)** | x86_64 | `npm run dist:mac:dmg:x64` | `scripts/build-macos.sh x64` | `release/railbench-0.6.0-mac-x64.dmg` | DMG-Installer für Intel Macs |
| **macOS Pakete (DMG + ZIP, Standard)** | x64 + arm64 | `npm run dist:mac` | `scripts/build-macos.sh` | `release/railbench-0.6.0-mac-*` | Baut signierte DMG-Installer und ZIP-Pakete für Intel & Apple Silicon |
| **macOS Package (ZIP)** | ARM64 | `npm run dist:mac:zip:arm64` | `scripts/build-macos.sh arm64 zip` | `release/railbench-0.6.0-mac-arm64.zip` | Optionales ZIP für Apple Silicon |
| **macOS Package (ZIP)** | x86_64 | `npm run dist:mac:zip:x64` | `scripts/build-macos.sh x64 zip` | `release/railbench-0.6.0-mac-x64.zip` | Optionales ZIP für Intel Macs |
| **macOS Packages (ZIP Alle)** | x64 + arm64 | `npm run dist:mac:zip:all` | `scripts/build-macos.sh all zip` | `release/railbench-0.6.0-mac-*.zip` | Baut beide optionalen ZIP-Pakete |

---

## 1. Voraussetzungen zum Selbstbauen

* **Node.js**: Version $\ge 20.x$
* **npm**: Version $\ge 10.x$

Vor dem ersten Build einmalig alle Abhängigkeiten installieren:
```bash
npm install
```

---

## 2. Windows Portable Build (dir & ZIP)

Die Windows-Builds werden als **unpacked directory (`dir`)** erzeugt und als sofort startbare, portable ZIP-Archive verpackt. Dies vermeidet die Startverzögerungen von Single-File-Packern (die bei jedem Start das gesamte App-Paket in `AppData\Local\Temp` entpacken) und verhindert Fehlalarme gängiger Antivirenprogramme.

```bash
# Beide Architekturen (x64 und ARM64) als portable Ordner & ZIPs bauen:
npm run dist:win:portable:all
# oder kurz:
npm run dist:win

# Nur x86_64 (Standard Intel/AMD PCs):
npm run dist:win:portable:x64

# Nur ARM64 (Snapdragon X, Surface Pro ARM, etc.):
npm run dist:win:portable:arm64

# Nur das Verzeichnis ohne ZIP erzeugen:
npm run dist:win:dir:x64
```

### Ausgabe-Dateien:
* **x64**:
  * Portables Verzeichnis: `release/win-unpacked/railbench.exe`
  * Standalone ZIP: `release/railbench-0.6.0-portable-x64.zip`
* **ARM64**:
  * Portables Verzeichnis: `release/win-arm64-unpacked/railbench.exe`
  * Standalone ZIP: `release/railbench-0.6.0-portable-arm64.zip`

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

## 4. macOS Build (`.dmg` & `.zip`) & Code Signing (`rcodesign`)

Der Build-Prozess unterstützt Cross-Compiling von Linux aus und signiert die macOS `.app`-Bundles automatisch via **`rcodesign`** (apple-codesign):

```bash
# Beide Architekturen (Apple Silicon arm64 & Intel x64) als signierte DMG und ZIP bauen (Standard):
npm run dist:mac

# Nur Apple Silicon (M1/M2/M3/M4) als DMG + ZIP:
npm run dist:mac:dmg:arm64 # bzw. scripts/build-macos.sh arm64

# Nur Intel (x86_64) als DMG + ZIP:
npm run dist:mac:dmg:x64   # bzw. scripts/build-macos.sh x64

# Nur ZIP-Pakete bauen:
npm run dist:mac:zip:all
```

### Signierung via `rcodesign`:
* **Ad-Hoc Signatur (Standard ohne Zertifikat)**:
  `rcodesign` signiert rekursiv alle 12 verschachtelten Bundles (`Electron Framework`, `libffmpeg`, Helper-Apps und die Haupt-Executable). Dies erfüllt die strikten Anforderungen von macOS AMFI auf Apple Silicon (M1/M2/M3/M4) und verhindert Kernel-Abstürze (`SIGKILL`).
* **Offizielles Apple Developer ID Zertifikat (.p12)**:
  Falls ein Apple-Entwicklerzertifikat vorliegt, kann dieses vor dem Build per Umgebungsvariablen übergeben werden:
  ```bash
  export APPLE_CERT_P12="/pfad/zu/developer_id.p12"
  export APPLE_CERT_PASSWORD="dein-zertifikats-passwort"
  npm run dist:mac
  ```

---

## 5. Fehlerbehebung (Troubleshooting)

* **Windows SmartScreen**: Klicke beim ersten Start auf *„Weitere Informationen“* $\rightarrow$ *„Trotzdem ausführen“*.
* **macOS Apple Silicon (M1+)**: Durch die integrierte `rcodesign`-Signierung starten die DMGs direkt. Falls Gatekeeper die App blockiert, genügt bei unnotarisierten Builds das einmalige Entfernen des Quarantäne-Flags:
  ```bash
  xattr -cr /Applications/railbench.app
  ```
  Oder per Finder: Rechtsklick auf `railbench.app` $\rightarrow$ *Öffnen* $\rightarrow$ *Öffnen bestätigen*.
* **Linux AppImage ohne FUSE 2**: Installiere `libfuse2` (`sudo apt install libfuse2`) oder starte das AppImage mit `--appimage-extract-and-run`.
