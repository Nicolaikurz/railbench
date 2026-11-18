# RAILbench – Menus & User Interface Guide

An overview of the RAILbench window and what each menu and panel does. The interface is available in **English and German** (toggle with the language button in the toolbar; *Settings → Auto-select language* follows the system). Shortcuts can be changed under *Settings → Customize Shortcuts…*. Press <kbd>F1</kbd> in the app for the built-in guide.

## 1. Window layout

```
┌──────────────────────────────────────────────────────────────┐
│ Toolbar: menus · quick actions · toggles · language/theme    │
├──────────┬───────────────────────────────────┬───────────────┤
│ Catalog  │                                   │ Layers panel  │
│ sidebar  │      2D canvas  /  3D view        │ (right)       │
│ (left)   │                                   │               │
│          │        [ Selection bar ]          │               │
└──────────┴───────────────────────────────────┴───────────────┘
```

| Area | Purpose |
|---|---|
| **Toolbar** (top) | Menus, quick actions (undo/redo/rotate/delete), mode toggles, collaboration, theme and language |
| **Left sidebar** | Catalog of tracks, streets, 3D models, terrain, table and parts list. Toggle with <kbd>B</kbd> |
| **Canvas** | 2D CAD drawing area (or 3D view, <kbd>3</kbd>). Zoom: mouse wheel · Pan: right/middle mouse button · Re-center: <kbd>Ctrl</kbd>+<kbd>0</kbd> |
| **Right sidebar** | Layers manager and the **Height & Ramp…** button. Toggle with <kbd>Shift</kbd>+<kbd>B</kbd> |
| **Selection bar** (bottom center) | Appears when objects are selected: layer, height, angle and quick property buttons |

## 2. Toolbar menus

### Layout (file)
New Layout · Open Layout… · Demo Layouts… · Save / Save As… · Import Layout XML · Import Track Catalog (JSON) · Import 3D Model (`.glb`/`.gltf`) · Export Layout as XML · Export Parts List (`.csv`) · Export Layout Plan as PDF · Live Collaboration (multi-user).

### Edit
Undo / Redo · Copy / Paste / Duplicate · Select All / Deselect All · Rotate clockwise / counter-clockwise · Delete · **Elevation & Layer…** · toggles for power feeder, isolator, catenary (+ feeder/isolator), switch lantern, turnout decoder, signal and bridge.

### View
Zoom to Fit / Center · Toggle 3D View · Show Rulers · Roadbed boards · Theme (System / Dark / Light) · Performance Monitor (<kbd>F3</kbd>) · Lock bar resizing · Reset UI sizes.

### Settings
Voltage & current configuration · Snap · Selection lock · Track number labels · Voltage simulation · Color mode (Types / Layers / Off) · Language · Customize Shortcuts · Reset personal configuration.

### Help
User guide (<kbd>F1</kbd>) · Keyboard shortcuts (<kbd>?</kbd>) · Welcome screen · Guides for layers, voltage and PDF export · About.

## 3. Left sidebar tabs

| Tab | Content |
|---|---|
| **Tracks** | Track systems by manufacturer and scale, with search and categories (Favorites, Straights, Curves, Turnouts, Crossings, Accessories). Click to add or drag onto the canvas |
| **Streets** | Generic street pieces for H0, TT, N, Z, 0, 1 and G |
| **3D** | Imported and built-in 3D models (buildings, scenery) |
| **Terrain** | Mountain and terrain editor: sculpting brushes, peak and ridge height, automatic tunnels |
| **Table** | Baseboard / table benchwork and the 2D sketcher for freeform plates |
| **Parts List** | Bill of materials for the layout; exportable as CSV |

## 4. Selection bar

Shown at the bottom when tracks are selected:

- **Count / type** of the selection and **layer** dropdown.
- **Height** field (mm): <kbd>Enter</kbd> applies, <kbd>↑</kbd>/<kbd>↓</kbd> change by ±1 mm (<kbd>Shift</kbd> ±0.1). Clicking the ⤉ icon opens the Height & Ramp panel.
- **Angle** field and rotate buttons.
- **Property buttons** (highlighted when active, "mixed" if the selection differs): power feeder ⚡, isolator, catenary and its feeder/isolator, signal 🚦, bridge 🌉, tunnel 🚇.

## 5. Right-click context menu

Right-click a track for: **Height & Ramp…** (<kbd>M</kbd>), **Auto-Generate Ramp…** (<kbd>Shift</kbd>+<kbd>M</kbd>), *Selection* (select connected tracks, select whole layer), and *Track-specific* actions (properties/info, signal and its side/direction, bridge, tunnel, catenary, reset flex track, turnout decoder and switch lantern).

## 6. Layers panel

Layers model different deck levels with a height in mm. Click the eye to hide, the lock to protect, double-click the name to rename and click the height badge to change the elevation. The **Height & Ramp…** button opens the panel below.

## 7. Height & Ramp panel

Open it via the right sidebar button, the context menu, the selection bar or <kbd>M</kbd> (<kbd>Shift</kbd>+<kbd>M</kbd> for auto-ramp). Drag the title bar to move it; <kbd>Esc</kbd> closes it. It has three tabs:

### Height / Level
Sets the selected tracks to a **target layer**. *Height mode*: **Level height** uses the layer's height, **Custom value** sets an absolute height in mm.

### Slope
Creates a ramp over **existing, connected** tracks between **A (Start)** and **B (End)**.

- Pick A and B with the 📍 buttons (click a track in the layout) or drag the pins; enter start/end heights in mm.
- ⇅ swaps start and end.
- **Maximum slope** (2.0 / 2.5 / 3.0 % or custom) with **Auto-Fit** to compute the end height from it.
- **Slope profile**: *Linear (uniform)* or *S-curve (eased)*.
- Live status: ✓ safe (≤ 3 %), ⚠ moderate (3–4.5 %), ⚠ steep (> 4.5 %, risk of wheel slip).
- **Continuous / Split** (next to the swap button): *Continuous* means the selected tracks form one unbroken chain from A to B. *Split* means they don't, for example because of a gap, a missing piece, a branch/turnout or a loop. Select the complete path to fix it.

### Automatic
**Generates new track pieces** for a ramp from an open connector.

- **Start track & connector**: pick in the layout or from the list.
- **Target height**, direction (↑ slope up / ↓ slope down), **Target slope**.
- **Ramp shape**: Straight, Curve (radius, left/right) or Helix (angle).
- **Target level**: an existing layer or a new one above/below (with the minimum clearance).
- A live preview is drawn on the plan; **Create Ramp** inserts the pieces.

## 8. Other dialogs

| Dialog | Opened from | Purpose |
|---|---|---|
| Voltage & current settings | Settings | Supply voltage, locomotive current, feeder defaults |
| Keyboard shortcuts | Settings / <kbd>?</kbd> | View and remap shortcuts |
| Print / PDF export | <kbd>Ctrl</kbd>+<kbd>P</kbd> | One page per layer with identical scale and per-layer parts list |
| Collaboration | Toolbar users icon | Start/join a multi-user session |
| Demo layouts / Welcome | Layout / Help menu | Example plans and quick start |
| Track properties | Context menu | Details of a piece and replacement by another type |

See the [README](../README.md#keyboard-shortcuts-reference) for the full shortcut table.
