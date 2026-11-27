# RAILbench – Menus & User Interface Guide

An overview of the native Rust RAILbench window and what each menu, toolbar button, and panel does. The interface is available in **English and German** (toggle with the **EN / DE** button in the toolbar; *Settings -> Settings & Shortcuts -> Language* also supports system auto-detection). Shortcuts can be customized under *Settings -> Customize Shortcuts...*. Press <kbd>F1</kbd> in the app for the built-in 11-tab guide.

## 1. Window layout

```
┌──────────────────────────────────────────────────────────────┐
│ Top menu bar: Layout · Edit · View · Settings · Help + badge │
│ Quick bar: Tabs · Actions · Snap · Tools · 3D · Drive · UI   │
├──────────┬───────────────────────────────────┬───────────────┤
│ Left     │                                   │ Right sidebar │
│ sidebar  │      2D canvas  /  3D view        │ (Layers)      │
│ (Catalog │                                   │               │
│  & Tabs) │        [ Selection bar ]          │               │
└──────────┴───────────────────────────────────┴───────────────┘
```

| Area | Purpose |
|---|---|
| **Menu & Quick Toolbar** (top) | Dropdown menus, sidebar tab register, file/edit actions, CAD tools, color/voltage/3D/Drive toggles, language, collaboration, theme |
| **Left sidebar** | Catalogs and editors for **Tracks**, **Streets**, **3D** models, **Terrain**, **Table**, and **Part List**. Toggle with <kbd>B</kbd>. Active tab also scopes canvas selection and dragging (`Tracks`, `Streets`, `3D`, `Terrain`, `Table`) |
| **Canvas / 3D Viewport** | 2D millimetric CAD area (or native `wgpu` 3D view, <kbd>3</kbd>). Zoom: mouse wheel / `+`/`-` · Pan: right/middle mouse drag · Fit view: <kbd>Ctrl</kbd>+<kbd>0</kbd> |
| **Right sidebar** | Layers manager (decks, elevation, visibility, lock) and **Heights / Ramp...** button. Toggle with <kbd>Shift</kbd>+<kbd>B</kbd> |
| **Selection bar** (bottom center) | Appears when items are selected: count, group indicator `[G]`, layer dropdown, height, angle, flex-track properties, and quick accessory toggles |

## 2. Top menus

### Layout
New Layout · Open Layout... · Open Recent · Demo Layouts... · Save (<kbd>Ctrl</kbd>+<kbd>S</kbd>) · Save As... (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd>) · **Blueprint Export & Print...** (Vector PDF, Print/Open PDF, 1:1 mm SVG preview & export) · Export XML (`.xml`)... · Export Part List (`.csv`)... · Import Track Catalog (`.json`)... · Collaborate (WebSocket)... · Welcome Screen...

### Edit
Undo (<kbd>Ctrl</kbd>+<kbd>Z</kbd>) · Redo (<kbd>Ctrl</kbd>+<kbd>Y</kbd>) · Copy (<kbd>Ctrl</kbd>+<kbd>C</kbd>) · Paste (<kbd>Ctrl</kbd>+<kbd>V</kbd>) · Duplicate (<kbd>Ctrl</kbd>+<kbd>D</kbd>) · Select All (<kbd>Ctrl</kbd>+<kbd>A</kbd>) · Select Connected Tracks · Rotate CCW/CW · Flip Selected (<kbd>Alt</kbd>+<kbd>F</kbd>) · Group / Ungroup (<kbd>G</kbd>) · Split / Replace Track... (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd>) · Track Properties... (<kbd>Alt</kbd>+<kbd>Enter</kbd>) · **Height & Ramp... (<kbd>M</kbd>)** · Apply Auto-Ramp (<kbd>Shift</kbd>+<kbd>M</kbd>) · Delete (<kbd>Del</kbd>).

### View
Reset / Fit View · Toggle 3D View (<kbd>3</kbd>) · Labels (<kbd>N</kbd>) · Connectors · Rulers & Grid (<kbd>Shift</kbd>+<kbd>L</kbd>) · Snap (<kbd>S</kbd>) · Layer Protection (<kbd>Alt</kbd>+<kbd>L</kbd>) · **Show Wood Plates** (toggle 3D contoured wood sub-plates) · Layers Sidebar (<kbd>Shift</kbd>+<kbd>B</kbd>) · Voltage Simulation (<kbd>P</kbd>) · Hardware Monitor (<kbd>F3</kbd>) · Lock Sidebars & Panels · Fullscreen (<kbd>F11</kbd>) · Theme (System / Dark / Light) · Color Mode (Track Types / Layers / Off).

### Settings
Voltage & Power Settings... · Height & Ramp... · Settings & Shortcuts... · Customize Shortcuts... · Blueprint Print & Export...

### Help
Help & Feature Guide... (<kbd>F1</kbd>) · Visual Guide: Menus & UI · Keyboard Shortcuts (<kbd>?</kbd>) · Welcome Screen... · Demo Layouts... · Layers & Heights Guide... · Voltage Simulation Guide... · Layer Printing Guide... · Track Catalog (JSON)... · Multi-User & Server... · About RAILbench...

## 3. Left sidebar tabs

| Tab | Content & Canvas Scope |
|---|---|
| **Tracks** | 23 track systems (H0, TT, N, Z, 0, 1, G) with search, favorites, and categories. Click to append or drag onto the canvas; scopes canvas selection/editing to track pieces |
| **Streets** | Generic street pieces for H0–G; scopes canvas selection/editing to road pieces |
| **3D** | 16 built-in procedural 3D models (Station, Signal Box, Engine Shed & Roundhouse with transparent roofs in 3D, Coaling & Diesel Fueling Stations, Water Tower, Turntable, Trees, etc.) plus `.glb`/`.gltf` import; scopes selection/editing to 3D models |
| **Terrain** | Unified **Terrain Brush** (Heightfield sculpt Raise/Lower/Smooth/Flatten/Grade + Biome Paint + Erase) and **Mountain Generator** with automatic 3D tunnels. Clips strictly to table/plate polygons |
| **Table** | Parametric multi-board benchwork (`Rect`, `L`, `U`, `T`, `Triangle`) + **2D CAD Sketcher** (`Poly`, `Rect`, `Edit`) for freeform boards and layer plates |
| **Part List** | Compact Bill of Materials listing only the currently **used items** and their **article numbers** (with category tabs + one-click **PDF** and **CSV** export) |

## 4. Selection bar & Context menu

* **Bottom Selection Bar**: Shows piece ID/name (or multi-selection count), `[G]` group badge, layer selector, **Height (mm)**, **Angle (°)**, flex-track `L`/`R`/`W`/gradient fields, and tri-state toggles for Power Feeder (`F`), Isolator (`I`), Catenary (`O`), Catenary Feeder (`Shift+F`), Catenary Isolator (`Shift+I`), Turnout Lantern (`W`), Decoder (`Shift+W`), Bridge (`B`), and Tunnel (`T`).
* **Right-Click Context Menu**: Open **Height & Ramp... (<kbd>M</kbd>)**, **Auto-Ramp (<kbd>Shift</kbd>+<kbd>M</kbd>)**, **Select Connected**, **Select Layer**, **Split / Replace**, **Track Properties**, signal/lantern/decoder/bridge/tunnel toggles, and **Delete**.

## 5. Right sidebar (Layers) & Height / Ramp panel

* **Layers Sidebar (<kbd>Shift</kbd>+<kbd>B</kbd>)**: Decks sorted by elevation (`mm`) with visibility eye, lock toggle, rename/color editor, `+ Add Layer`, orphan-item reassignment, and **Heights / Ramp...**.
* **Height & Ramp Panel (<kbd>M</kbd>)**:
  * **Level / Height**: Assign selected tracks to a target deck height or custom millimeter value.
  * **Slope**: Compute linear or S-curve transitions between **A (Start)** and **B (End)** with `Swap`, `Auto-Fit`, `Continuous`/`Split` path indicator, and slope severity colors.
  * **Automatic**: Generate straight, curved, or helix ramp pieces directly from an open connector into a target deck.

## 6. 3D View & Drive (Cab View & Signal Box)

* **3D View (<kbd>3</kbd>)**: Toggle between **Hall & Landscape** and **CAD / Minimal**, enable **Edit 3D** to pick/drag/elevate items directly in 3D, use the interactive **CAD ViewCube**, scrub the 24h **Day/Night** sun slider, and inspect tracks inside **Engine Sheds / Roundhouses** through their transparent 3D roofs.
* **Drive (`Fahren` / `Drive`)**: Detachable window (ideal for a 2nd monitor) with **Split**, **Cab View** (BR 243, BR 96 Mallet, V 60), and interactive **Signal Box (Stellwerk)** modes for controlling trains, couplers, turnouts, uncouplers, turntables, and transfer tables.

See the [README Keyboard Shortcuts Reference](../README.md#keyboard-shortcuts-reference) for the full keybinding list.
