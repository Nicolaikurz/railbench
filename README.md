# Railbench – Model Railway Track Planner (v0.5.2)

An intuitive, fast, and precise model railway track planning and layout simulation application (TypeScript + PixiJS + Vite + Electron).

---

> [!IMPORTANT]
> **Release 0.5.2 Testing Notice**:
> For **Release 0.5.2, the application includes multi-platform executables for **Linux (x86_64, arm64 AppImage)**, **Windows (x64, arm64 portable Dir)**, and **macOS (Apple Silicon M1/M2/M3/M4 arm64(not tested))**.

---

## Key Features & Capabilities

### 1. Precision Track CAD & Multi-Scale Catalogs
* **Popular Track Systems**: Includes ready-to-use geometries for H0 (1:87) (Märklin C-Gleis, Märklin K-Gleis, Märklin M-Gleis, Trix C-Gleis, Piko A-Gleis, Roco Line, Tillig Elite, Peco Streamline Code 100), TT (1:120) (Tillig Modellgleis, Tillig Bettungsgleis, Kuehn Gleis), N (1:160) (Fleischmann Piccolo, Kato Unitrack, Minitrix), and Z (1:220) (Märklin mini-club, Rokuhan).
* **Märklin C-Gleis Drehscheibe 74862 & Trix 66862**: Full turntable support (DB 23 m standard type) with 30 exits at 12° increments, circular pit rail, bridge with wooden catwalks, operator cab, and true 3D pit rendering.
* **Integrated Street & Road Systems**: Dedicated street catalogs for H0 and N scale in their own sidebar tab (Straßen) with straights, curves, junctions, turning loops, and asphalt rendering.
* **Point-to-Point Distance Measuring Tool**: Measure straight-line distance, horizontal/vertical deltas, and angle between any two points with snap guidance.
* **Smart Magnetic Snapping (<kbd>S</kbd>)**: Connectors automatically align with exact millimetric coordinates and tangent angles.
* **Color-Coding (<kbd>C</kbd>) & Part Numbers (<kbd>N</kbd>)**: Distinguish identical track radii and straights by color or display official manufacturer part numbers directly on the track bed.
* **Intuitive Controls**: Move tracks with drag & drop, rotate in 7.5° steps with <kbd>R</kbd> / <kbd>Shift+R</kbd>, enter any exact angle directly in the bottom selection bar, rotate under the cursor with <kbd>Shift</kbd> + **Mouse Wheel**, and select multiple tracks using **Left-Click Box Drag** (marquee selection). Pan the canvas with **Right-Click** or **Middle-Click Drag**.

### 2. Multi-Deck Layers & Height Management
* **Floating Layers Menu (<kbd>Shift+B</kbd>)**: Manage multiple elevation decks (Ground level, Mountain branches, Staging yards / *Schattenbahnhöfe*) sorted by vertical height.
* **Custom Layer Naming & Renaming**: Name layers when creating them, or rename any existing layer at any time by clicking the **✏️** icon or **double-clicking** the layer title. Click the elevation badge (e.g. `+80 mm`) to directly adjust the layer's base height.
* **Deck Protection**: Independent visibility toggles (<kbd>👁️</kbd>) to hide upper decks and editing locks (<kbd>🔒</kbd>) to prevent accidental movement.
* **Change Height / Move to Layer (<kbd>M</kbd>)**: Assign selected tracks to another layer and automatically synchronize their elevation.
* **Continuous Ramp & Incline Calculator**: Calculate continuous multi-track ramps using proportional length (*Dreisatz*), optional vertical easing (*Ausrundung* 1%→2%→3%→2%→1%), 2D slope direction chevrons & badges, and 3D helical elevation rendering.
* **Canvas Selection Bar**: Quick floating pill on canvas showing current selection count, layer dropdown, elevation badge, and one-click height adjustment.

### 3. 3D Perspective & Elevation View *(Alpha)*
> [!NOTE]
> **Alpha Status**: The 3D layout viewer is currently in active **Alpha preview**. It provides realistic 3D perspective visualization of your plan, multi-deck elevations, bridge structures, benchwork, and scenery. Full 3D track connection editing is actively being expanded.
* **3D Perspective (<kbd>3</kbd> or <kbd>V</kbd>)**: One-key toggle between 2D precision CAD drafting and realistic 3D perspective.
* **3D Models & Scenery Support (`.glb` / `.gltf`)**:
  * **Import custom 3D models**: Import binary `.glb` or `.gltf` 3D files directly from disk or via drag-and-drop onto the canvas. Auto-scales models from real-world meters to prototypical H0/N/TT scale.
  * **Built-in Open-Source Models Library (100% Procedural & Royalty-Free)**: Place standard prototypical railway scenery items generated natively via Three.js parametric code (no external copyrighted binaries or textures; licensed under the project's open-source GPLv3 license):
    * 🏢 **Bahnhof Neustadt**: Passenger station with gabled roof, clock tower, and platform canopy.
    * 🏭 **Stellwerk W1**: Two-storey brick signal tower with panoramic glass windows and steel staircase.
    * 🚂 **Lokschuppen**: Single-stall red brick locomotive engine shed with arched doorway.
    * 🏠 **Fachwerkhaus**: Half-timbered country town house with steep tile roof.
    * 💧 **Wasserturm**: Round stone water tower with elevated reservoir and pivoting crane.
    * 🚦 **Lichtsignal**: Railway block signal with slender mast and colored visor optics.
    * 🛑 **Prellbock**: End of track buffer stop with steel rail A-frame and bumper beam.
    * 🌲 **Tanne / Fichte**: Tiered evergreen coniferous pine trees.
    * 🌳 **Laubbaum**: Deciduous leafy oak tree.
    * 💡 **Bahnsteigleuchte**: Curved swan-neck platform lamppost.
  * **3D Models Licensing Notice**: All built-in 3D scenery models are 100% procedurally coded in TypeScript (`src/render/builtin-models-3d.ts`), completely free of third-party copyright encumbrances, and free for personal, hobby, and commercial layout design. External models imported by users (`.glb`/`.gltf`) retain their original creator licenses.
  * **2D CAD Footprint & 3D Lighting**: In 2D drafting mode, models show architectural footprints with labels, dimension badges, and rotation handles. In 3D, models cast and receive realistic sunlight shadows.
* **Elevation Decks & Bridge Pillars**: Elevated track segments and ramps are placed at their true vertical heights in millimeters. Track sections elevated above ground level ($>15\text{ mm}$) automatically generate sturdy bridge support pillars connecting to the baseboard.
* **Modular Benchwork in 3D**: Extruded wooden baseboard geometry matching your chosen shape (Rectangular, L-shape, U-shape, T-shape) with realistic table legs and authentic surface materials.
* **Interactive Orbit Controls**: Smooth orbit rotation (Left-Click + drag), panning (Right-Click / Shift + Left-Click + drag), zooming (Mouse wheel), and instant center reset ("⌖ Reset View").
* **Return to 2D Plan**: Press <kbd>Esc</kbd> or <kbd>3</kbd> / <kbd>V</kbd> to return to standard 2D editing mode.

### 4. Layer-Separated Layout Printing (<kbd>Ctrl+P</kbd>)
* **One Sheet per Layer**: Generates clean, separate printable pages for each individual deck.
* **100% Identical Scale & Alignment**: Every layer page shares the exact same coordinate bounding box and scale. Print on paper or transparency sheets and stack them to verify bridge clearances and alignment pillars!
* **Layer Inventory Banners**: Each printed page features a bottom banner listing the exact track pieces and quantities needed to build that specific layer.
* **Full Overview & Bill of Materials**: Optional combined layout overview sheet and full inventory table with piece counts and total trackage lengths in meters.
* **Ink-Saver Mode**: Toggle between vibrant full color and crisp monochrome outline mode.

### 5. Traction Voltage Drop & Analog Simulation (<kbd>P</kbd>)
* **Electrical Heatmap (<kbd>P</kbd>)**: Simulates real-world resistance across rail sections and connectors. Color-codes trackage: Green (optimal $>92\%$), Amber (moderate drop), and Red (critical drop $<80\%$).
* **Power Feed Points (<kbd>F</kbd>)**: Mark power feeder tracks from boosters or transformers to eliminate dead zones.
* **Track Isolators (<kbd>I</kbd>)**: Insert electrical gap cutoffs to simulate separate booster districts, analog block control, signal stopping sections, or staging yard sidings.
* **Voltage Presets**: One-click switching between 16V AC (Märklin), 14V DC/DCC (2-Rail H0), 12V DC (N / TT), 18V DC (Gauge 1 / LGB), or custom user-defined voltages.

### 6. Modular Benchwork & Baseboard Table
* **Table Shapes**: Rectangular, L-Shape (left/right wing), U-Shape (operator pit), and T-Shape (peninsula) benchwork.
* **Custom Dimensions**: Set width, depth, corner radius, wing sizes, and surface textures (Natural Birch, Plywood, Dark Slate, Walnut, or Drafting White).
* **Fit Table to Tracks**: Automatically sizes the baseboard to fit the outer boundary of your track plan with a safe buffer margin.
* **Millimeter Rulers (<kbd>Shift+L</kbd>)**: Real-time measurement edge rulers along canvas borders.

### 7. Bill of Materials (BOM) & File Export
* **Live Piece Counter**: Real-time inventory list with total parts and track lengths in meters.
* **CSV Export (<kbd>Ctrl+E</kbd>)**: Export structured BOM tables for Microsoft Excel, LibreOffice Calc, or Google Sheets.
* **File Formats**: Save and open layouts in native `.gleisplan` (JSON) or standard `.xml` formats with full layer and elevation persistence.

---

## Keyboard Shortcuts Reference

| Shortcut | Action | Category |
|---|---|---|
| <kbd>Ctrl</kbd> + <kbd>N</kbd> | New empty layout | File |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Open layout plan (`.gleisplan` / `.xml`) | File |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Save layout plan | File |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd> | Save layout plan as... | File |
| <kbd>Ctrl</kbd> + <kbd>W</kbd> | **Close Plan / Dialog / Window** | File / Navigation |
| <kbd>Ctrl</kbd> + <kbd>P</kbd> | **Print Plan (Layer-Separated Pages)** | Print / Export |
| <kbd>Ctrl</kbd> + <kbd>E</kbd> | Export Bill of Materials (CSV) | Print / Export |
| <kbd>Ctrl</kbd> + <kbd>Z</kbd> | Undo last action | Edit |
| <kbd>Ctrl</kbd> + <kbd>Y</kbd> | Redo undone action | Edit |
| <kbd>Ctrl</kbd> + <kbd>C</kbd> | Copy selected tracks to clipboard | Edit |
| <kbd>Ctrl</kbd> + <kbd>V</kbd> | Paste tracks from clipboard | Edit |
| <kbd>Ctrl</kbd> + <kbd>D</kbd> | Duplicate selected tracks | Edit |
| <kbd>Ctrl</kbd> + <kbd>A</kbd> | Select all tracks and models | Selection |
| <kbd>R</kbd> | Rotate selected track +7.5° clockwise | Editing |
| <kbd>Shift</kbd> + <kbd>R</kbd> | Rotate selected track -7.5° counter-clockwise | Editing |
| <kbd>Shift</kbd> + **Mouse Wheel** | Rotate track under cursor continuously | Editing |
| <kbd>W</kbd> / <kbd>Shift</kbd> + <kbd>W</kbd> | Toggle Turnout Lantern / Decoder | Electrical / Turnouts |
| <kbd>M</kbd> | **Change Track Height / Move to Layer** | Layers |
| <kbd>Del</kbd> / <kbd>Backspace</kbd> | Delete selected tracks | Editing |
| <kbd>F</kbd> | Toggle Power Feeder point (⚡) | Electrical |
| <kbd>Shift</kbd> + <kbd>F</kbd> | Toggle Catenary Power Feeder (⚡🔌) | Electrical / Catenary |
| <kbd>I</kbd> | Toggle Track Isolator gap (⛔) | Electrical |
| <kbd>Shift</kbd> + <kbd>I</kbd> | Toggle Catenary Section Insulator (⚡⛔) | Electrical / Catenary |
| <kbd>O</kbd> | Toggle Catenary (Overhead wire & masts) (⚡) | Electrical / 3D |
| <kbd>P</kbd> | Toggle Traction Voltage Simulation heatmap | Electrical |
| <kbd>3</kbd> or <kbd>V</kbd> | **Toggle 3D Elevation & Perspective View (Alpha)** | View / 3D |
| <kbd>Esc</kbd> | Exit 3D View / Close dialogs | View / Navigation |
| <kbd>S</kbd> | Toggle Magnetic Track Snapping | View / Snapping |
| <kbd>C</kbd> | Toggle Geometry Color-Coding | View |
| <kbd>N</kbd> | Toggle Track Part Numbers | View |
| <kbd>B</kbd> | Toggle Left Track Catalog Sidebar | Workspace |
| <kbd>Shift</kbd> + <kbd>B</kbd> | Toggle Floating Layers Manager Menu | Workspace |
| <kbd>Shift</kbd> + <kbd>L</kbd> | Toggle Millimeter Edge Rulers | View |
| <kbd>Ctrl</kbd> + <kbd>0</kbd> | Reset View / Center Layout | View |
| <kbd>F1</kbd> or <kbd>?</kbd> | **Open Help & Feature Guide** | Help |

---

## Quickstart

```bash
# Install dependencies
npm install

# Start development web server
npm run dev

# Run unit tests
npm test

# Build web application
npm run build

# Start desktop app in development
npm run electron:dev

# Start multi-user collaboration server
npm run server
```

### Multi-User Server Collaboration & Auto-Start

Railbench includes both an **in-app embedded server** (toggleable with one click in the Electron app via `File` → `Server Collaboration...`) and a **standalone Node.js server** (`server.mjs`) for headless or dedicated server hosting. Multiple people can plan, lay tracks, adjust elevations, and edit baseboards simultaneously in real time over LAN or the Internet.

#### 1. Auto-Start Variables & CLI Arguments

You can auto-start the collaboration server automatically when launching Railbench without needing to open the in-app menus:

##### Command-Line Arguments (CLI Flags)
| CLI Flag | Default | Description |
|---|---|---|
| `--server` or `--collab-server` | `off` | Automatically starts the embedded server in the background upon launch. |
| `--port <number>` or `--server-port <number>` | `3000` | Network port to listen on (e.g. `--port 8080` or `--port=8080`). |
| `--password <string>` or `--server-password <string>` | *none* | Requires password authentication to enter the default room. |
| `--room <name>` or `--server-room <name>` | `default` | Configures the default collaborative room name (e.g. `--room yard`). |
| `--connect` or `--auto-connect` | `off` | Automatically connects the local desktop client to the server on start. |

##### Environment Variables
| Environment Variable | Example | Description |
|---|---|---|
| `RAILBENCH_SERVER` | `1` or `true` | Auto-starts the server on launch (equivalent to `--server`). |
| `RAILBENCH_SERVER_PORT` / `COLLAB_PORT` / `PORT` | `3000` | Custom port to listen on. |
| `RAILBENCH_SERVER_PASSWORD` / `COLLAB_PASSWORD` | `secret123` | Password protection for the room. |
| `RAILBENCH_SERVER_ROOM` / `COLLAB_ROOM` | `main-station` | Default room name. |
| `RAILBENCH_CONNECT` | `1` | Automatically connects local client to the local server. |

##### Quick Start Examples:
```bash
# Start Desktop App with auto-started server, custom port, password, and auto-connected local client:
railbench --server --port 3000 --password mysecret --room layout1 --connect

# Using environment variables:
RAILBENCH_SERVER=1 RAILBENCH_SERVER_PORT=3000 RAILBENCH_SERVER_PASSWORD=mysecret railbench

# Run standalone headless server (Node.js):
node server.mjs --port 8080 --password mysecret --room layout1

# Invite other users to join in their browser (LAN / Internet):
# http://<server-ip>:3000/?collab=1&room=layout1&pass=mysecret
```

#### 2. Collaboration Features
* **Real-time Live Sync**: Instantly synchronizes track additions, movements, rotations, deletions, elevations, and baseboards across all connected participants.
* **Live Cursors & Awareness**: See everyone's mouse cursor in real time with individual name badges and colors.
* **Password Protection**: Prevents unauthorized participants from joining or altering plans.
* **Auto-Save & Rooms**: Automatically saves shared layouts to `./saved_plans/<room>.gleisplan`.
* **1-Click Invite Links**: Share plans via `http://<host>:3000/?collab=1&room=hauptbahnhof&pass=secret`.


## Building Executables

### 1. Windows Portable (`.exe`)
Builds standalone, portable `.exe` files without requiring installation.

* **x86_64 (Intel/AMD)**:
  ```bash
  npm run dist:win:portable:x64
  # or ./scripts/build-windows-portable.sh x64
  ```
* **ARM64** (Snapdragon X, Surface Pro ARM):
  ```bash
  npm run dist:win:portable:arm64
  # or ./scripts/build-windows-portable.sh arm64
  ```
* **Both Architectures**:
  ```bash
  npm run dist:win:portable:all
  # or ./scripts/build-windows-portable.sh
  ```

### 2. Linux AppImage (`.AppImage`)
Single executable binary running across modern Linux distributions.

* **x86_64**:
  ```bash
  npm run dist:linux:appimage:x64
  # or ./scripts/build-linux-appimage.sh x64
  ```
* **ARM64** (Raspberry Pi 4/5, Asahi Linux):
  ```bash
  npm run dist:linux:appimage:arm64
  # or ./scripts/build-linux-appimage.sh arm64
  ```
* **Both Architectures**:
  ```bash
  npm run dist:linux:appimage:all
  # or ./scripts/build-linux-appimage.sh
  ```

### 3. macOS (`.zip` / `.dmg`)
Packaging for Apple Silicon (M1-M4) and Intel Macs.

* **Apple Silicon (`arm64`)**:
  ```bash
  npm run dist:mac:zip:arm64
  # or ./scripts/build-macos.sh arm64
  ```
* **Intel (`x64`)**:
  ```bash
  npm run dist:mac:zip:x64
  # or ./scripts/build-macos.sh x64
  ```
* **Both Architectures**:
  ```bash
  npm run dist:mac:zip:all
  # or ./scripts/build-macos.sh all
  ```
* **DMG Installer (native macOS only)**:
  ```bash
  npm run dist:mac:dmg
  ```

### 4. Linux Flatpak (`.flatpak`)
Sandboxed application bundle (Flathub compatible).

```bash
npm run dist:linux:flatpak
# or ./scripts/build-flatpak.sh
```

For detailed requirements, architecture differences, cross-compilation notes, and troubleshooting, see the [Build Guide](docs/BUILD_GUIDE.md).

## Documentation

* [Build Guide (Windows, Linux & macOS)](docs/BUILD_GUIDE.md)
* [Architecture Documentation](docs/architektur.md)
