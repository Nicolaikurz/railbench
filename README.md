# RAILbench – Native Rust Model Railway Track Planner (v26.11.1)

An intuitive, fast, and precise native desktop model railway CAD track planning, 3D visualization, terrain sculpting, train cab simulation, and electrical simulation application built in **Rust (`egui` / `eframe` + `wgpu`)**.

---

## Why RAILbench?

**RAILbench** is built with a strict **100% offline-first** philosophy—every track geometry, manufacturer catalog, 3D asset, PDF blueprint generator, electrical simulation, and multi-user WebSocket server is compiled directly into a fast native executable with zero browser or Electron runtime overhead.

---

## Quickstart (Run & Test from Source)

### Prerequisites
* **Rust toolchain** (stable Cargo + `rustc`, e.g. via [rustup](https://rustup.rs/))
* On Linux desktop: standard Wayland/X11, Vulkan/OpenGL ICD, and GTK3 runtime/dev libraries (`gtk3`, `libxkbcommon`, `vulkan-icd-loader`, `wayland`)

```bash
# Run the native application
cargo run --manifest-path rust/Cargo.toml -p railbench-app

# Run the full Rust unit & UI test suite
cargo test --manifest-path rust/Cargo.toml
```

### Multi-User Server Collaboration & Auto-Start

RAILbench includes an **in-app embedded WebSocket server** (`File` -> `Live Collaboration...` or CLI flags):

```bash
# Launch RAILbench with an auto-started collaboration server:
railbench-app --server --port 3000 --password mysecret --room layout1 --connect
```

---

## Building Release Binaries & Packages

| Target / Package | Command | Output |
|---|---|---|
| **Native binary (current OS/arch)** | `./scripts/build-rust-native.sh` | `release/railbench-<ver>-native-<os>-<arch>` |
| **Linux AppImage (`x64` / `arm64`)** | `./scripts/build-linux-appimage.sh x64` | `release/railbench-<ver>-x64.AppImage` |
| **Linux Flatpak (`x64` / `arm64`)** | `./scripts/build-flatpak.sh x64` | `release/railbench-<ver>-x86_64.flatpak` |
| **Linux Snap (`x64` / `arm64`)** | `./scripts/build-snap.sh x64` | `release/railbench_<ver>_amd64.snap` |
| **Windows Portable ZIP (`x64` / `arm64`)** | `scripts/build-windows-portable.bat` or `./scripts/build-windows-portable.sh x64` | `release/railbench-<ver>-win-x64.zip` |
| **macOS `.dmg` & `.zip` (`arm64` / `x64` / `universal`)** | `./scripts/build-macos.sh` | `release/railbench-<ver>-mac-*.dmg` / `.zip` |

See [Build & Binary Execution Guide](docs/BUILD_GUIDE.md) for prerequisites and cross-compilation details.

---

## Key Capabilities

* **Multi-Scale Track & Street Catalogs**: Built-in track systems across **H0, TT, N, Z, 0, 1, and G** (Märklin C/K/M/1/Z, Trix, Piko, Roco, Tillig, Kuehn, Peco, Kato, Bachmann, Fleischmann, Rokuhan, Lenz, LGB) plus H0–G road catalogs and custom XML/CSV catalog import.
* **2D CAD Canvas & Table/Layer Sketcher**: Magnetic snap connections, flex-track bending & grade calculation, marquee & tab-scoped selection, polygon/rectangle table sketcher, and millimeter rulers.
* **Hardware-Accelerated 3D Viewer (`wgpu`)**: 360° × 180° orbit camera with interactive CAD ViewCube, procedural buildings & depot sheds (with transparent shed roof inspection), catenary wiring, bridges, tunnels, 24h day/night lighting, and `.glb`/`.gltf` custom model import.
* **Terrain Sculpting & Auto-Tunnels**: Interactive heightfield raise/lower/smooth/flatten/grade brushes, biome painting, mountain ridges, and automatic track tunnel cutouts.
* **Drive, 3D Cab View & Signal Box (Stellwerk)**: Real-time train physics, couplers, turnouts, uncouplers, turntables, transfer tables, and a detachable secondary Cab + Signal Box window.
* **Electrical Traction & Short-Circuit Simulation**: Multi-source Dijkstra voltage drop heatmap, feeder/isolator placement, catenary wiring simulation, and short-circuit fault beacons.
* **1:1 Blueprint PDF Printing & Part List (BOM)**: Layer-separated printable PDF blueprints, used-items Part List with article numbers, and CSV export.

---

## Keyboard Shortcuts Reference

| Shortcut | Action | Category |
|---|---|---|
| <kbd>Ctrl</kbd>+<kbd>N</kbd> / <kbd>O</kbd> / <kbd>S</kbd> / <kbd>Shift</kbd>+<kbd>S</kbd> | New / Open / Save / Save As | File |
| <kbd>Ctrl</kbd>+<kbd>P</kbd> | Blueprint Export & Print (PDF / SVG) | File |
| <kbd>Ctrl</kbd>+<kbd>E</kbd> | Export Part List (CSV) | File |
| <kbd>Ctrl</kbd>+<kbd>Z</kbd> / <kbd>Y</kbd> | Undo / Redo | Edit |
| <kbd>Ctrl</kbd>+<kbd>C</kbd> / <kbd>V</kbd> / <kbd>D</kbd> / <kbd>A</kbd> | Copy / Paste / Duplicate / Select All | Edit |
| <kbd>R</kbd> / <kbd>Shift</kbd>+<kbd>R</kbd> / <kbd>Alt</kbd>+<kbd>F</kbd> | Rotate CW / CCW / Flip Selected | Edit |
| <kbd>G</kbd> / <kbd>Delete</kbd> | Group-Ungroup / Delete Selected | Edit |
| <kbd>Alt</kbd>+<kbd>L</kbd> | Toggle Layer Selection & Movement Lock | Edit |
| <kbd>M</kbd> / <kbd>Shift</kbd>+<kbd>M</kbd> | Distance Measure / Height & Ramp Dialog / Auto-Ramp | Tools |
| <kbd>F</kbd> / <kbd>Shift</kbd>+<kbd>F</kbd> | Rail Power Feeder / Catenary Feeder | Electrical |
| <kbd>I</kbd> / <kbd>Shift</kbd>+<kbd>I</kbd> | Rail Isolator / Catenary Isolator | Electrical |
| <kbd>O</kbd> / <kbd>P</kbd> | Toggle Catenary / Voltage Heatmap | Electrical |
| <kbd>W</kbd> / <kbd>Shift</kbd>+<kbd>W</kbd> / <kbd>Shift</kbd>+<kbd>S</kbd> | Turnout Lantern / Decoder / Signal | Turnouts |
| <kbd>3</kbd> / <kbd>V</kbd> / <kbd>Esc</kbd> | Toggle 3D View / Deselect & Exit | View |
| <kbd>Shift</kbd>+<kbd>B</kbd> / <kbd>Shift</kbd>+<kbd>L</kbd> | Toggle Layers Sidebar / Rulers | View |
| <kbd>C</kbd> / <kbd>N</kbd> / <kbd>S</kbd> | Cycle Color Mode / Track Labels / Snapping | View / Tools |
| <kbd>Ctrl</kbd>+<kbd>0</kbd> / <kbd>F3</kbd> / <kbd>F1</kbd> | Fit View / Hardware Monitor / Help Guide | View / Help |

## Documentation

* [Build & Binary Execution Guide (Windows, Linux & macOS)](docs/BUILD_GUIDE.md)
* [Menus & User Interface Guide](docs/UI_GUIDE.md)
* [System Architecture & Technical Documentation](docs/architektur.md)
