# RAILbench – Model Railway Track Planner (v0.6.0)

An intuitive, fast, and precise model railway CAD track planning, 3D visualization, terrain sculpting, and electrical simulation application built with **TypeScript + PixiJS v8 + Three.js + Vite + Electron**.
<img width="2346" height="1428" alt="Screenshot From 2026-11-14 21-49-15" src="https://github.com/user-attachments/assets/33434aab-9b8e-4923-9fc9-fb291fcc4329" />

---

> [!IMPORTANT]
> **Release 0.6.0 Multi-Platform Binaries**:
> Release **0.6.0** provides standalone executables for **Windows (`x64`, `arm64`)**, **macOS (`arm64` Apple Silicon M1–M4, `x64` Intel)**, and **Linux (`x86_64`, `arm64` AppImage & Flatpak)**.

---

## Running Pre-Built Binaries (Windows, macOS & Linux)

### 🪟 Windows (`x64` & `arm64`)
1. **Download**:
   * **Unpacked Portable Folder**: `win-unpacked` (`x64`) or `win-arm64-unpacked` (`arm64`).
2. **Run**:
   * Double-click `railbench.exe` inside the unpacked folder. No installation or administrator privileges are required.
3. **Windows Defender SmartScreen Note**:
   * Because the open-source binary is not signed with a commercial EV certificate, Windows SmartScreen may show an *"Windows protected your PC"* prompt on first launch. Click **More info** (*Weitere Informationen*) $\rightarrow$ **Run anyway** (*Trotzdem ausführen*).

### 🍎 macOS (Apple Silicon `arm64` & Intel `x64`)
1. **Download & Extract**:
   * **Apple Silicon (M1 / M2 / M3 / M4)**: `railbench-0.6.0-mac-arm64.zip`.
   * **Intel Mac**: `railbench-0.6.0-mac-x64.zip` (or `.dmg`)
   * Double-click the `.zip` file to extract **`railbench.app`** and move **`railbench.app`** into `/Applications`.
2. **Bypass macOS Gatekeeper on First Launch (Unsigned Open-Source Build)**:
   * Because the binary is built without a paid Apple Developer ID certificate, macOS Gatekeeper may block the first launch with *"railbench.app cannot be opened because the developer cannot be verified"* or *"is damaged"*.
   * **Option A — Terminal (Recommended, fastest)**:
     ```bash
     xattr -cr /Applications/railbench.app
     open /Applications/railbench.app
     ```
     *(If running directly from `Downloads`, use `xattr -cr ~/Downloads/railbench.app`)*
   * **Option B — Finder UI**:
     **Right-click** (or <kbd>Control</kbd>-click) `railbench.app` in Finder $\rightarrow$ click **Open** $\rightarrow$ confirm **Open** in the security dialog (or allow it under *System Settings → Privacy & Security → Open Anyway*).

### 🐧 Linux (`x86_64` & `arm64` AppImage / Flatpak)
* **AppImage**:
  ```bash
  chmod +x railbench-0.6.0-x64.AppImage
  ./railbench-0.6.0-x64.AppImage
  ```
  *(If `libfuse.so.2` is not installed on Ubuntu 24.04+, either run `sudo apt install libfuse2` or run `./railbench-0.6.0-x64.AppImage --appimage-extract-and-run`.)*

---

## Key Features & Capabilities (v0.6.0)

### 1. Precision Track CAD & Multi-Scale Catalogs
* **16 Ready-to-Use Track Systems across H0, TT, N & Z**:
  * **H0 (1:87)**: Märklin C-Gleis, Märklin K-Gleis, Märklin M-Gleis, Trix C-Gleis, Piko A-Gleis, Roco Line, Tillig Elite, Peco Streamline Code 100.
  * **TT (1:120)**: Tillig TT Modellgleis, Tillig TT Bettungsgleis, Kuehn TT Gleis.
  * **N (1:160)**: Fleischmann Piccolo, Kato Unitrack, Minitrix.
  * **Z (1:220)**: Märklin mini-club, Rokuhan.
* **Turntables & Transfer Tables**: Full support for **Märklin C-Gleis Drehscheibe 74862 / Trix 66862** (DB 23 m standard turntable with 30 exits at 12° increments) and **Transfer Tables (*Schiebebühnen*)** with 2D/3D pit rendering and catenary spider wiring.
* **Customizable Flex-Track**: Dynamic length, radius, and arc angle adjustment directly from the selection bar.
* **Integrated Street & Road Systems**: Dedicated street catalogs for H0, TT, N, and Z scale in their own sidebar tab (**Straßen / Roads**) with straights, curves, intersections, turning loops, road markings, and asphalt rendering.
* **1:1 Direct 2D Drag-and-Drop Canvas Preview**: Dragging tracks, roads, or 3D buildings from the catalog renders an exact 1:1 millimetric preview directly under the cursor.
* **Point-to-Point Distance Measuring Tool**: Measure straight-line distance, horizontal/vertical deltas ($\Delta X, \Delta Y$), real-world scaled distance, and angle between any two points with magnetic snap guidance.

### 2. Interactive 2D CAD Sketcher for Tables & Layer Plates
* **Freeform CAD Sketcher (`Tisch & Platten`)**: Draw and edit both **Baseboard Tables** and **Layer Plates (*Ebenenplatten*)** directly on the 2D canvas like in a CAD sketcher:
  * **Polygon Pen (`Polygon-Stift`)**: Click point-by-point on the canvas to sketch custom room-fitting tables or multi-deck plates (`Enter`, double-click, or click the start point to close).
  * **Rectangle Tool (`Rechteck`)**: Click two corners on the canvas to define exact rectangular boards or plates.
  * **Vertex & Edge Editor (`Ecken & Kanten`)**: Drag existing vertices (`●`) to reshape, drag edge midpoints (`+`) to insert new vertices, or right-click / <kbd>Alt</kbd>-click a vertex to remove it.
* **Standard Parametric Table Shapes**: Rectangular, L-Shape, U-Shape, and T-Shape presets with configurable dimensions, corner radii, surface styles (Birch, Dark Walnut, Plywood, Slate, Grass, White), and automatic **"Fit Table to Tracks"**.

### 3. Dedicated Terrain & Mountain Editor with Tunnels
* **Dedicated Terrain Menu (`Gelände / Terrain`)**: Streamlined monochrome-icon sidebar tab for sculpting 3D mountains, hills, valleys, and rock formations.
* **Interactive 2D/3D Sculpting Brushes**:
  * **Raise (`Hebepinsel`)**, **Smooth (`Glätten`)**, **Flatten (`Plateau`)**, and **Erase (`Radierer`)** brushes with adjustable brush radius and intensity.
  * **Freehand Contour Pen (`Freihand-Kontur`)**: Draw custom mountain footprints directly on the canvas.
  * **5 Mountain Presets**: Alpine Peak (*Alpengipfel*), Mountain Ridge (*Gebirgskamm*), Twin Peaks (*Doppelgipfel*), Valley Gorge (*Talschlucht*), and Gentle Hill (*Sanfter Hügel*).
* **Automatic & Manual Mountain Tunnels**:
  * One-click automatic detection (`Tunnel für Gleise & Straßen im Berg automatisch erzeugen`) cuts realistic 3D tunnel tubes with masonry portals and dark inner walls through mountains along track and road paths.
  * Individual tracks and roads can also toggle tunnel rendering manually via the `🚇` button in the floating selection bar.

### 4. Full-Perspective 3D Viewer & Interactive CAD ViewCube
* **Full 360° × 180° Camera Freedom (<kbd>3</kbd> or <kbd>V</kbd>)**: Orbit freely around the layout from **every perspective**—including straight down from above (`Top`) and straight up from **underneath the table (`Bottom`)** to inspect table framing, legs, risers, and staging yards (the hall/landscape floor automatically hides when viewing from below).
* **Interactive CAD ViewCube (*Ansichtswürfel*)**:
  * Positioned directly beneath the 3D environment toggle button in the top-left overlay.
  * Live-synchronized 3D cube with **6 clickable faces** (`Front`/`Vorne`, `Back`/`Hinten`, `Left`/`Links`, `Right`/`Rechts`, `Top`/`Oben`, `Bottom`/`Unten`), **4 isometric corner hotspots** (`↖`, `↗`, `↙`, `↘`), direct pointer drag-to-orbit on the cube, and quick-action buttons (`⟲ 45°`, `⌂ ISO`, `⟳ 45°`, `⊤ Top`, `▭ Front`, `⊥ Bottom`).
  * Automatically switches labels and tooltips between **English** and **German** based on the active application language.
* **3D Environments & 24h Day/Night Sun Cycle**:
  * Toggle between the detailed **Locomotive Hall (`Halle`)** with surrounding forest and **CAD / Minimal** mode.
  * Dynamic 24-hour Day/Night cycle with moving sun and moon, atmospheric sky gradients, and automatic night illumination for signals, buildings, and platform lamps.
* **25 Built-in Procedural 3D Scenery & Depot Models (`.glb` / `.gltf` Import Supported)**:
  * **Steam & Diesel Locomotive Depot**: **Coaling Station (*Bekohlungsanlage für Dampfloks*)** with elevated bunker, slewing crane, grab bucket, and coal piles; **Diesel Fueling Station (*Dieseltankstelle für Dieselloks*)** with storage tank, bund wall, canopy, and fuel dispensers; **3-Stall Roundhouse (*Ringlokschuppen*)**; **Single-Stall Engine Shed (*Lokschuppen*)**; **Water Tower (*Wasserturm*)**.
  * **Stations, Buildings & Infrastructure**: Passenger Station (*Bahnhof Neustadt*), Station Platform (*Bahnsteig*), Goods Shed (*Güterschuppen*), Signal Tower (*Stellwerk W1*), Town Church (*Stadtkirche*), Half-Timbered Houses (*Fachwerkhaus* & *Eckhaus*), Level Crossing (*Bahnübergang*), Steel Truss & Stone Arch Bridges, Tunnel Portals, Signals, Buffer Stops, Trees, and Platform Lamps.
  * All 25 built-in models are 100% procedural TypeScript/Three.js geometry (`src/render/models-3d/`), royalty-free under GPLv3. Users can also import custom `.glb` / `.gltf` models.

### 5. Multi-Deck Layers & Height Management
* **Floating Layers Menu (<kbd>Shift+B</kbd>)**: Manage multiple elevation decks sorted by height with visibility toggles (`👁️`), lock protection (`🔒`), inline renaming (`✏️`), and elevation badges.
* **Continuous Ramp & Incline Calculator (<kbd>M</kbd> / <kbd>Shift+M</kbd>)**: Calculate smooth multi-track ramps with proportional length (*Dreisatz*), optional vertical easing (*Ausrundung*), live slope percentage warnings, 2D slope chevrons, and 3D bridge pillars.
* **Floating Selection Action Bar**: Quick access to elevation (`⤉`), angle (`↻`), flex-track geometry, power feeder (`⚡`), isolator (`⊘`), catenary (`⌁`), turnout decoder (`📟`), lantern (`💡`), track signal (`🚦`), bridge (`🌉`), and tunnel (`🚇`).

### 6. Traction Voltage Simulation & Short-Circuit Detection (<kbd>P</kbd>)
* **Electrical Heatmap (<kbd>P</kbd>)**: Multi-source Dijkstra resistance simulation across rails and connectors (`>92%` Green, `80–92%` Amber, `<80%` Red, isolated dashed Red).
* **Short-Circuit Detection (`⚠️⚡`)**: Automatically detects electrical short circuits (e.g. unisolated reversing loops / *Kehrschleifen*) and highlights the exact fault points in both 2D and 3D with clickable jump-to-fault beacons.
* **Configurable Voltage & Multi-Train Current Load**: Presets for 16V AC, 14V DC/DCC, 12V DC, 18V DC, scale-specific locomotive current draw (50–1000 mA), and simultaneous multi-train load multipliers.

### 7. Layer-Separated Layout Printing (<kbd>Ctrl+P</kbd>) & BOM Export (<kbd>Ctrl+E</kbd>)
* **1:1 Layer-Separated Printing**: Prints each layer on its own sheet with an identical coordinate bounding box and scale, optional ghosted reference outlines of adjacent decks, and per-layer part inventories.
* **Live Bill of Materials & CSV Export (<kbd>Ctrl+E</kbd>)**: Real-time part counts, trackage length in meters, and one-click CSV export.

---

## Keyboard Shortcuts Reference

| Shortcut | Action | Category |
|---|---|---|
| <kbd>Ctrl</kbd> + <kbd>N</kbd> | New empty layout | File |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Open layout plan (`.railbench` / `.gleisplan` / `.xml`) | File |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Save layout plan | File |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd> | Save layout plan as... | File |
| <kbd>Ctrl</kbd> + <kbd>W</kbd> | Close Plan / Dialog / Window | File / Navigation |
| <kbd>Ctrl</kbd> + <kbd>P</kbd> | Print Plan (Layer-Separated Pages) | Print / Export |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> | Export Bill of Materials (CSV) | Print / Export |
| <kbd>Ctrl</kbd> + <kbd>Z</kbd> / <kbd>Y</kbd> | Undo / Redo | Edit |
| <kbd>Ctrl</kbd> + <kbd>C</kbd> / <kbd>V</kbd> / <kbd>D</kbd> | Copy / Paste / Duplicate selected items | Edit |
| <kbd>Ctrl</kbd> + <kbd>A</kbd> | Select all tracks and models | Selection |
| <kbd>R</kbd> / <kbd>Shift</kbd> + <kbd>R</kbd> | Rotate selected item $+7.5^\circ$ / $-7.5^\circ$ | Editing |
| <kbd>Shift</kbd> + **Mouse Wheel** | Rotate item under cursor continuously | Editing |
| <kbd>Alt</kbd> + <kbd>L</kbd> | Toggle Selection Lock / Layer Protection Bypass | Workspace / Layers |
| <kbd>M</kbd> / <kbd>Shift</kbd> + <kbd>M</kbd> | Open Height & Ramp Dialog / Auto-Calculate Ramp | Layers / Ramps |
| <kbd>Del</kbd> / <kbd>Backspace</kbd> | Delete selected tracks or models | Editing |
| <kbd>F</kbd> / <kbd>Shift</kbd> + <kbd>F</kbd> | Toggle Rail Power Feeder (`⚡`) / Catenary Feeder (`⌁+`) | Electrical |
| <kbd>I</kbd> / <kbd>Shift</kbd> + <kbd>I</kbd> | Toggle Rail Isolator (`⊘`) / Catenary Insulator (`⌁⊘`) | Electrical |
| <kbd>O</kbd> | Toggle Catenary (Overhead wire & masts `⌁`) | Electrical / 3D |
| <kbd>Shift</kbd> + <kbd>D</kbd> / <kbd>L</kbd> | Toggle Turnout Decoder (`📟`) / Turnout Lantern (`💡`) | Turnouts |
| <kbd>P</kbd> | Toggle Traction Voltage Simulation Heatmap | Electrical |
| <kbd>3</kbd> or <kbd>V</kbd> | Toggle 3D Perspective View (with CAD ViewCube) | View / 3D |
| <kbd>Esc</kbd> | Exit 3D View / Cancel Sketcher / Close Dialogs | Navigation |
| <kbd>S</kbd> | Toggle Magnetic Track Snapping | View / Snapping |
| <kbd>C</kbd> | Cycle Color-Coding (Track Types / Layers / Off) | View |
| <kbd>N</kbd> | Toggle Track Part Numbers | View |
| <kbd>B</kbd> / <kbd>Shift</kbd> + <kbd>B</kbd> | Toggle Left Catalog Sidebar / Right Layers Menu | Workspace |
| <kbd>Shift</kbd> + <kbd>L</kbd> | Toggle Millimeter Edge Rulers | View |
| <kbd>Ctrl</kbd> + <kbd>0</kbd> | Reset View / Center Layout | View |
| <kbd>F1</kbd> or <kbd>?</kbd> | Open In-App Help & Feature Guide | Help |

---

## Development Quickstart

```bash
# Install dependencies
npm install

# Start development web server
npm run dev

# Run unit tests (Vitest)
npm test

# Type-check and build web application
npm run build

# Start desktop app in development
npm run electron:dev

# Start standalone multi-user collaboration server
npm run server
```

### Multi-User Server Collaboration & Auto-Start

RAILbench includes both an **in-app embedded WebSocket server** (`File` → `Server Collaboration...`) and a **standalone Node.js server** (`server.mjs`).

```bash
# Launch Desktop App with auto-started server, custom port, password, and auto-connected client:
railbench --server --port 3000 --password mysecret --room layout1 --connect

# Or run standalone headless server (Node.js):
node server.mjs --port 8080 --password mysecret --room layout1
```

---

## Building Executables from Source

* **Windows Portable (`.exe` / unpacked dir)**:
  ```bash
  npm run dist:win:portable:x64
  npm run dist:win:portable:arm64
  npm run dist:win:portable:all
  ```
* **macOS (`.zip` & `.dmg` for Apple Silicon `arm64` & Intel `x64`)**:
  ```bash
  npm run dist:mac:zip:arm64
  npm run dist:mac:zip:x64
  npm run dist:mac:zip:all
  ```
* **Linux AppImage & Flatpak**:
  ```bash
  npm run dist:linux:appimage:x64
  npm run dist:linux:appimage:arm64
  npm run dist:linux:appimage:all
  ```

## Documentation

* [Build & Binary Execution Guide (Windows, Linux & macOS)](docs/BUILD_GUIDE.md)
* [System Architecture & Technical Documentation](docs/architektur.md)
