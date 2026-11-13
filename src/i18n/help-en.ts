import type { HelpDialogTranslations } from "./types";

export const helpEn: HelpDialogTranslations = {
  modalTitle: "Railbench – User & Feature Guide",
  closeTitle: "Close Help (Esc)",
  closeBtnText: "Close Guide",
  footerTip:
    "Tip: Press <strong>F1</strong> anywhere to open this guide, or <strong>M</strong> when tracks are selected to change layer elevation.",
  copiedText: "✓ Copied to clipboard!",
  tabs: {
    overview: "Getting Started",
    layers: "Layers & Heights",
    print: "Layer Printing",
    power: "Voltage Simulation",
    baseboard: "Table & Baseboard",
    view3d: "3D View",
    server: "Multi-User & Server",
    shortcuts: "Keyboard Shortcuts",
    catalog: "Track Catalog (JSON)",
    about: "About & Systems",
  },
  overview: {
    title: "Getting Started with Railbench",
    intro:
      "<strong>Railbench</strong> is a high-precision, web- and desktop-based track planning application engineered for model railway hobbyists. It combines millimetric CAD geometry with electrical simulation, multi-level layout design, and print-ready schematics.",
    cards: [
      {
        title: "1. Adding Tracks",
        body: "Browse track pieces in the <strong>left catalog sidebar</strong> (press <code>B</code> to toggle). Click any piece to append it, or <strong>drag and drop</strong> it directly onto the canvas.",
      },
      {
        title: "2. Smart Snapping",
        body: "Automatic magnetic snapping (<code>S</code> key) seamlessly aligns track ends, calculates exact angles, and connects track geometry within millimetric tolerances.",
      },
      {
        title: "3. Rotating & Moving",
        body: "Drag tracks with the mouse to move them. Rotate in 7.5° steps using <code>R</code> (or <code>Shift+R</code> for counter-clockwise). Hold <code>Shift + Mouse Wheel</code> to rotate smoothly, or drag the blue rotation handle. You can also enter any custom angle directly in the Angle input field in the bottom selection bar!",
      },
      {
        title: "4. Multi-Selection (Marquee Box)",
        body: "Click directly on any track to select it. Click and drag with the <strong>Left Mouse Button in an empty area</strong> to draw a selection rectangle (marquee box). All tracks within the rectangle will be selected. Hold <code>Ctrl</code> or <code>Shift</code> to add to your existing selection or toggle individual tracks.",
      },
    ],
    callout:
      "<strong>Pro-Tip for Navigation:</strong> Hold <strong>Right Mouse Button</strong> or the <strong>Mouse Wheel (Middle Click)</strong> to pan the canvas. Use the <strong>Mouse Wheel</strong> to zoom in and out. Press <code>Ctrl+0</code> anytime to re-center the layout.",
  },
  layers: {
    title: "Multi-Deck Layers & Height Management",
    intro:
      "Railbench allows designing multi-level model train layouts (e.g. Ground level, Upper mountain branches, ramps, and hidden Staging Yards / Shadow Stations) with real vertical millimeter elevations.",
    cards: [
      {
        title: "Floating Layers Menu",
        body: "Press <code>Shift+B</code> or click the <strong>Ebenen</strong> icon in the toolbar to toggle the floating layers manager. Layers are sorted by height with the highest deck at the top.",
      },
      {
        title: "Visibility & Lock Protection",
        body: "Click the <strong>Eye icon</strong> to hide a layer when working on tracks underneath. Click the <strong>Lock icon</strong> to freeze a layer against accidental movement or editing.",
      },
      {
        title: "Rename Layers Inline",
        body: "Double-click any layer name to rename it immediately inline. Press <code>Enter</code> to confirm or <code>Esc</code> to cancel. Click the elevation badge (e.g. <code>+80 mm</code>) to adjust its base height.",
      },
      {
        title: "Move Tracks to Layer (M)",
        body: 'Select tracks and press <code>M</code> or click <strong>"↳ Move & Set Height"</strong> on any layer card. The tracks will be assigned to that layer and their height automatically synchronized.',
      },
      {
        title: "Ramp & Incline Calculator",
        body: "Configure gradual elevation transitions (e.g. 0 mm → 80 mm). The built-in calculator divides elevation difference by total track length and shows a live gradient percentage:<br>• <strong>&le; 3.0%</strong>: Recommended (Gentle incline)<br>• <strong>3.0% – 4.5%</strong>: Moderate (Watch heavy trains)<br>• <strong>&gt; 4.5%</strong>: Warning (Risk of wheel slippage)",
      },
    ],
    callout:
      "<strong>Canvas Selection Bar:</strong> When one or more tracks are selected, a sleek floating bar appears at the bottom center of the screen, allowing you to change layer and height in a single click!",
  },
  print: {
    title: "Layer-Separated Layout Export / Printing (Ctrl+P)",
    intro:
      "Traditional track planning software prints or exports the whole layout as one overlapping tangle. Railbench introduces <strong>true layer-separated export</strong> designed for building real layouts benchwork by benchwork.",
    cards: [
      {
        title: "1. One Page Per Layer",
        body: "Every layer is exported on its own dedicated sheet. Automatic pagination guarantees clean page breaks without overlapping content.",
      },
      {
        title: "2. 100% Identical Scale & Bounding Box",
        body: "All exported layer sheets share the exact same bounding box, origin, and scale. You can view or print transparent or paper sheets, stack them on top of each other, and verify that bridge pillars, ramp portals, and track curves align perfectly!",
      },
      {
        title: "3. Ghosted Reference Outlines",
        body: "Optionally show faint, dimmed outlines of the layers above and below on the current sheet so you have full spatial context.",
      },
      {
        title: "4. Layer-Specific Bill of Materials",
        body: "Each sheet includes a bottom inventory banner showing exactly which track pieces and quantities are required to construct that specific layer (e.g., <code>24188: 8×, 24172: 4×</code>).",
      },
    ],
    callout:
      '<strong>How to Export / Print:</strong> Press <code>Ctrl+P</code> (or <code>Cmd+P</code> on macOS), choose your layers in the preview dialog, and click <strong>"System Print..."</strong> or <strong>"Export as PDF"</strong>.',
  },
  power: {
    title: "Traction Voltage Drop & Analog Simulation",
    intro:
      "Ensure reliable operation of digital and analog locomotives with Railbench's integrated electrical simulation engine.",
    cards: [
      {
        title: "Color Heatmap (P Key)",
        body: 'Press <code>P</code> to toggle the voltage heatmap:<br>• <strong style="color: #10b981;">Green</strong>: Optimal voltage (&gt; 92% of supply)<br>• <strong style="color: #f59e0b;">Amber</strong>: Moderate drop (&gt; 80% of supply)<br>• <strong style="color: #ef4444;">Red</strong>: Critical drop (&lt; 80% — add feed point!)<br>• <strong style="color: #dc2626;">Dashed Red</strong>: Isolated / disconnected block',
      },
      {
        title: "Power Feeder Points (F Key)",
        body: "Select a track and press <code>F</code> to attach a power feed point (e.g. from your booster or transformer). Railbench immediately recomputes voltage distribution across the entire rail network.",
      },
      {
        title: "Track Isolators (I Key)",
        body: "Select a track and press <code>I</code> to place an electrical gap (Trennstelle). Use isolators to simulate separate booster districts, analog block control, signal stopping sections, or staging yard siding cutoffs.",
      },
      {
        title: "Supply Voltage & Traction Current",
        body: "Click the arrow next to the Voltage button in the toolbar and select <strong>Custom Voltage & Current...</strong> to configure voltage and traction currents:<br>• <strong>16V AC / Digital</strong>: Märklin H0 Standard<br>• <strong>14V DC / DCC</strong>: 2-Rail H0 (Piko, Roco, Trix)<br>• <strong>12V DC</strong>: N Scale & TT Scale<br>• <strong>18V DC</strong>: Gauge 1 & LGB Garden Railways<br>• <strong>Locomotive Current Presets</strong>:<br>&nbsp;&nbsp;• <strong>N / Z</strong>: 50–150 mA (typ. 100 mA)<br>&nbsp;&nbsp;• <strong>TT</strong>: 200–400 mA (typ. 300 mA)<br>&nbsp;&nbsp;• <strong>H0</strong>: 300–600 mA (typ. 450 mA)<br>&nbsp;&nbsp;• <strong>Max. per Loco</strong>: 1 A (1000 mA)<br>• <strong>Multi-Train Load</strong>: Multiply current by the number of simultaneously operating locomotives.",
      },
    ],
  },
  baseboard: {
    title: "Table & Baseboard Benchwork Planning",
    intro:
      "Design your layout to fit your actual room and carpentry constraints using realistic baseboard boundaries.",
    cards: [
      {
        title: "Modular Table Shapes",
        body: "Choose between four common model railway benchwork forms:<br>• <strong>Rectangular</strong>: Classic flat table or island layout<br>• <strong>L-Shape</strong>: Corner layouts with left or right return wing<br>• <strong>U-Shape</strong>: Walk-in operator pit layout<br>• <strong>T-Shape</strong>: Peninsula or branch line extension",
      },
      {
        title: "Wood Finishes & Materials",
        body: "Select realistic table textures including <strong>Natural Birch Wood</strong>, <strong>Dark Walnut</strong>, <strong>Plywood</strong>, <strong>Dark Slate</strong>, or clean <strong>Drafting White</strong>.",
      },
      {
        title: "Live Dimensions & Rulers",
        body: "Displays millimetric dimension arrows directly on the board. Toggle millimeter rulers along canvas borders with <code>Shift+L</code>.",
      },
      {
        title: "Fit Table to Tracks",
        body: 'In the Table tab of the left sidebar, click <strong>"Fit Table to Tracks"</strong> to automatically calculate the minimum table dimensions needed to enclose all track pieces with a safe buffer.',
      },
    ],
  },
  view3d: {
    title: "3D Perspective & Elevation View",
    intro:
      "The 3D Viewer renders your entire model railroad layout in real-time three dimensions, including the baseboard table, multi-deck elevations, bridge pillars, ballasting, and tracks.",
    cards: [
      {
        title: "Camera Navigation",
        body: '• <strong>Orbit / Rotate:</strong> Left-click and drag across the 3D scene.<br>• <strong>Pan:</strong> Right-click and drag (or Shift + Left-click).<br>• <strong>Zoom:</strong> Scroll wheel or trackpad pinch.<br>• <strong>Reset:</strong> Click "⌖ Reset View" to re-center on the benchwork.',
      },
      {
        title: "Bridges & Elevated Decks",
        body: "Tracks on elevated layers (or custom ramps) are rendered at their true vertical heights. Tracks elevated &gt; 15 mm automatically generate realistic bridge support pillars connecting to the baseboard!",
      },
      {
        title: "Modular 3D Benchwork",
        body: "The wooden baseboard table (Rectangular, L-shape, U-shape, or T-profile) is extruded in full 3D with authentic wood grain shading and sturdy steel table legs.",
      },
      {
        title: "3D Models & Scenery (.glb / .gltf)",
        body: "Place built-in open-source models (Passenger Station, Signal Box, Engine Shed, Water Tower, Half-Timbered House, Light Signal, Buffer Stop, Trees, Platform Lamps) or import custom <code>.glb</code> / <code>.gltf</code> files via the <strong>3D Models</strong> sidebar tab or by dragging files onto the canvas!",
      },
      {
        title: "3D Models Licensing & Origin",
        body: "• <strong>Built-in 3D Scenery:</strong> All 10 included models are <strong>100% procedurally generated pure code</strong> (Three.js procedural geometry in <code>src/render/builtin-models-3d.ts</code>). They contain no third-party binary assets or proprietary textures and are fully covered by the project's open-source license (GPLv3 / royalty-free for personal, hobby, and commercial layout design).<br>• <strong>Imported Models (.glb / .gltf):</strong> Custom models imported from external sources remain governed by their respective authors' licenses (e.g. Creative Commons CC0, CC-BY).",
      },
      {
        title: "Return to 2D Plan",
        body: "Press <code>3</code>, <code>V</code>, or <code>Esc</code> anytime to smoothly return to 2D track planning mode.",
      },
    ],
    callout:
      "<strong>Quick Switch:</strong> Press <code>3</code> or <code>V</code> at any time to toggle between the 2D CAD drafting mode and the 3D perspective visualization.",
  },
  server: {
    title: "Multi-User Collaboration & Server Auto-Start",
    intro:
      "Railbench allows multiple modelers to build and design layouts simultaneously in real time across a local area network (LAN) or over the Internet.",
    cards: [
      {
        title: "1. Real-Time Synchronization",
        body: "Track placements, drags, rotations, gradient ramps, and baseboard adjustments propagate instantly without lag to all connected peers.",
      },
      {
        title: "2. Live Cursors & Awareness",
        body: "Every participant has a unique color and name tag. See remote mouse movements and track selections live on the canvas.",
      },
      {
        title: "3. Password Protection & Rooms",
        body: "Rooms can be secured with passwords to prevent unauthorized access and protect layouts during collaborative sessions.",
      },
      {
        title: "4. Automatic Persistence",
        body: "The server automatically persists all changes to disk under <code>./saved_plans/&lt;room&gt;.gleisplan</code>.",
      },
    ],
    cliTitle: "Start Variables for Server Auto-Start (CLI & Environment)",
    cliIntro:
      "Launch Railbench with the collaboration server auto-started in the background without needing to toggle menus manually.",
    cliHeaders: ["Start Variable / Flag", "Default", "Description"],
    cliRows: [
      [
        "<code>--server</code> or <code>--collab-server</code>",
        "off",
        "Automatically starts the embedded WebSocket server on application launch.",
      ],
      [
        "<code>--port &lt;Port&gt;</code> or <code>--server-port</code>",
        "3000",
        "Custom network port (e.g. <code>--port 8080</code>).",
      ],
      [
        "<code>--password &lt;PW&gt;</code>",
        "none",
        "Configures a password requirement for the default room.",
      ],
      [
        "<code>--room &lt;Name&gt;</code>",
        "default",
        "Sets the active collaborative room name (e.g. <code>--room central-station</code>).",
      ],
      [
        "<code>--connect</code> or <code>--client</code>",
        "off",
        "Automatically connects the local window to the server upon launch.",
      ],
    ],
    envTitle: "Environment Variables:",
    envHeaders: ["Variable", "Example", "Effect"],
    envRows: [
      [
        "<code>RAILBENCH_SERVER</code>",
        "<code>1</code> or <code>true</code>",
        "Auto-starts the server on launch.",
      ],
      [
        "<code>RAILBENCH_SERVER_PORT</code>",
        "<code>3000</code>",
        "Configures the server port (or <code>COLLAB_PORT</code> / <code>PORT</code>).",
      ],
      [
        "<code>RAILBENCH_SERVER_PASSWORD</code>",
        "<code>secret123</code>",
        "Sets the room password.",
      ],
      [
        "<code>RAILBENCH_SERVER_ROOM</code>",
        "<code>staging-yard</code>",
        "Configures default room name.",
      ],
      [
        "<code>RAILBENCH_CONNECT</code>",
        "<code>1</code>",
        "Auto-connects local client on start.",
      ],
    ],
    callout:
      "<strong>Quickstart Examples:</strong><br>• Desktop App with Server & Password: <code>railbench --server --port 3000 --password secret --connect</code><br>• Standalone Node.js Server: <code>node server.mjs --port 8080 --password secret --room yard1</code><br>• 1-Click Browser Link for Collaborators: <code>http://&lt;IP-Address&gt;:3000/?collab=1&room=default&pass=secret</code>",
  },
  shortcuts: {
    title: "⌨️ Keyboard Shortcuts Reference",
    intro: "Master these shortcuts for rapid layout design and editing:",
    headers: ["Shortcut", "Action", "Category"],
    rows: [
      ["<kbd>Ctrl</kbd> + <kbd>N</kbd>", "New Layout Plan", "File"],
      ["<kbd>Ctrl</kbd> + <kbd>O</kbd>", "Open Plan File (.railbench / .gleisplan / XML)", "File"],
      ["<kbd>Ctrl</kbd> + <kbd>S</kbd>", "Save Plan (Quick Save)", "File"],
      ["<kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd>", "Save Plan As...", "File"],
      ["<kbd>Ctrl</kbd> + <kbd>W</kbd>", "<strong>Close Plan / Dialog / Window</strong>", "File / Navigation"],
      ["<kbd>Ctrl</kbd> + <kbd>P</kbd>", "<strong>Print Plan (Layer-Separated Pages)</strong>", "Print / Export"],
      ["<kbd>Ctrl</kbd> + <kbd>E</kbd>", "Export Bill of Materials (CSV)", "Print / Export"],
      ["<kbd>Ctrl</kbd> + <kbd>Z</kbd>", "Undo last action", "Edit"],
      ["<kbd>Ctrl</kbd> + <kbd>Y</kbd>", "Redo undone action", "Edit"],
      ["<kbd>Ctrl</kbd> + <kbd>C</kbd>", "Copy selected tracks", "Edit"],
      ["<kbd>Ctrl</kbd> + <kbd>V</kbd>", "Paste tracks from clipboard", "Edit"],
      ["<kbd>Ctrl</kbd> + <kbd>D</kbd>", "<strong>Duplicate selected tracks</strong>", "Edit"],
      ["<kbd>Ctrl</kbd> + <kbd>A</kbd>", "Select all tracks", "Selection"],
      ["<kbd>R</kbd>", "Rotate selected item +7.5° clockwise", "Editing"],
      ["<kbd>Shift</kbd> + <kbd>R</kbd>", "Rotate selected item -7.5° counter-clockwise", "Editing"],
      ["<kbd>Shift</kbd> + <kbd>Wheel</kbd>", "Rotate track under cursor by mouse wheel", "Editing"],
      ["<kbd>Left Click</kbd> + Drag (empty area)", "<strong>Marquee Box Selection (Multiple Tracks)</strong>", "Selection"],
      ["<kbd>Right Click</kbd> / <kbd>Middle Click</kbd> + Drag", "Pan Canvas Viewport", "Navigation"],
      ["<kbd>Ctrl</kbd> / <kbd>Shift</kbd> + <kbd>Left Click</kbd>", "Toggle track in selection (Multi-selection)", "Selection"],
      ["<kbd>Alt</kbd> + <kbd>L</kbd>", "<strong>Toggle Selection Lock (Select locked & deleted layer items)</strong>", "Workspace / Layers"],
      ["<kbd>M</kbd>", "<strong>Change Track Height / Move to Layer (Dialog)</strong>", "Layers"],
      ["<kbd>Shift</kbd> + <kbd>M</kbd>", "<strong>Auto-Calculate Ramp</strong> (Constant gradient between levels/heights)", "Layers / Ramps"],
      ["<kbd>Del</kbd> / <kbd>Backspace</kbd>", "Delete selected tracks", "Editing"],
      ["<kbd>F</kbd>", "Toggle Power Feeder point", "Electrical"],
      ["<kbd>Shift</kbd> + <kbd>F</kbd>", "Toggle Catenary Power Feeder", "Electrical / Catenary"],
      ["<kbd>I</kbd>", "Toggle Track Isolator / Gap", "Electrical"],
      ["<kbd>Shift</kbd> + <kbd>I</kbd>", "Toggle Catenary Section Insulator", "Electrical / Catenary"],
      ["<kbd>O</kbd>", "Toggle Catenary (Overhead wire & masts)", "Electrical / 3D"],
      ["<kbd>Shift</kbd> + <kbd>D</kbd>", "Toggle Digital Turnout Decoder", "Electrical / Turnouts"],
      ["<kbd>Shift</kbd> + <kbd>L</kbd>", "Toggle Lighted Turnout Lantern", "Electrical / Turnouts"],
      ["<kbd>P</kbd>", "Toggle Traction Voltage Heatmap Simulation", "Electrical"],
      ["<kbd>3</kbd> or <kbd>V</kbd>", "<strong>Toggle 3D Elevation & Perspective View</strong>", "View / 3D"],
      ["<kbd>Esc</kbd>", "Exit 3D View / Close Dialogs", "View / Navigation"],
      ["<kbd>S</kbd>", "Toggle Automatic Track Snapping", "View / Snapping"],
      ["<kbd>C</kbd>", "Toggle Track Geometry Color-Coding", "View"],
      ["<kbd>N</kbd>", "Toggle Track Item Part Numbers", "View"],
      ["<kbd>Shift</kbd> + <kbd>B</kbd>", "Toggle Floating Layers Manager Menu", "Workspace"],
      ["<kbd>Shift</kbd> + <kbd>L</kbd>", "Toggle Millimeter Edge Rulers", "View"],
      ["<kbd>Ctrl</kbd> + <kbd>0</kbd>", "Reset View / Center on Layout", "View"],
      ["<kbd>F1</kbd> or <kbd>?</kbd>", "Open this Help & Feature Guide", "Help"],
    ],
  },
  catalog: {
    title: "Track Catalog Format & JSON Example (H0 Scale)",
    intro:
      "Railbench allows importing custom track libraries via <strong>File ➔ Import Track Catalog (JSON)...</strong>. Catalogs are stored in a streamlined, universal JSON format.",
    cards: [
      {
        title: "Catalog Header Properties",
        body: '• <code>scale</code>: Model scale (e.g. <code>"H0"</code>, <code>"TT"</code>, <code>"N"</code>, <code>"Z"</code>, <code>"1"</code>, <code>"G"</code>, <code>"0"</code>)<br>• <code>gauge</code>: Track gauge in mm (H0: <code>16.5</code>, TT: <code>12.0</code>, N: <code>9.0</code>, Z: <code>6.5</code>, 1/G: <code>45.0</code>, 0: <code>32.0</code>)<br>• <code>hasRoadbed</code>: <code>true</code> for roadbed / ballast, <code>false</code> for ties only<br>• <code>conductors</code>: <code>2</code> (2-rail DC) or <code>3</code> (3-rail AC / studs)',
      },
      {
        title: "Supported Track Types (17 Kinds)",
        body: "The template below contains <strong>exactly one representative entry of each track type</strong>:<br><code>straight</code>, <code>curve</code>, <code>turnout-left</code>, <code>turnout-right</code>, <code>curved-turnout-left</code>, <code>curved-turnout-right</code>, <code>three-way</code>, <code>double-slip</code>, <code>crossing</code>, <code>uncoupler</code>, <code>buffer-stop</code>, <code>feeder</code>, <code>isolating</code>, <code>flex</code>, <code>turntable</code>, <code>transfer-table</code>, <code>accessory</code>.",
      },
    ],
    copyBtn: "📋 Copy JSON",
    downloadBtn: "💾 Download JSON (railbench-catalog-h0-example.json)",
    metaInfo: "Scale H0 · 16.5 mm Gauge · 17 Track Kinds",
  },
  about: {
    title: "About Railbench & Track Libraries",
    intro:
      "<strong>Railbench {version}</strong> is an open-source model railway track design and layout planning CAD tool. It is built using modern TypeScript, PixiJS WebGL rendering, Vite, and Electron.",
    systemsHeading: "Supported Scale Systems & Catalogs:",
    systems: [
      "<strong>H0 (1:87)</strong>: Märklin C-Gleis (Full 3-rail with roadbed geometry, curves R1–R5, turnouts, crossings)",
      "<strong>H0 (1:87)</strong>: Märklin K-Gleis (Prototypical 3-rail tie-only track without roadbed, R1–R5, slim turnouts 14° 26' and flex-track)",
      "<strong>H0 (1:87)</strong>: Märklin M-Gleis (Classic sheet-metal roadbed 3-rail track, R1, R2, industrial circle and crossings)",
      "<strong>H0 (1:87)</strong>: Trix C-Gleis (2-rail DC with C-track roadbed geometry, identical footprint)",
      "<strong>H0 (1:87)</strong>: Piko A-Gleis (2-rail standard gauge, flexible geometries, turnouts)",
      "<strong>H0 (1:87)</strong>: Roco Line (2-rail realistic high-fidelity track with roadbed)",
      "<strong>H0 (1:87)</strong>: Peco Streamline Code 100 (Universal 2-rail standard gauge system)",
      "<strong>H0 (1:87)</strong>: Tillig Elite (Fine scale 2-rail prototypical track system)",
      "<strong>TT (1:120)</strong>: Tillig TT Modellgleis",
      "<strong>TT (1:120)</strong>: Tillig TT Bettungsgleis (Track system with integrated roadbed)",
      "<strong>TT (1:120)</strong>: Kuehn TT Gleis (2-rail TT track geometry)",
      "<strong>N (1:160)</strong>: Fleischmann Piccolo (N-scale with roadbed ballasting)",
      "<strong>N (1:160)</strong>: Kato Unitrack (Integrated roadbed track system with Unijoiners)",
      "<strong>N (1:160)</strong>: Minitrix (Classic 2-rail tie-only track system)",
      "<strong>Z (1:220)</strong>: Märklin mini-club (Classic Z-scale 2-rail track system)",
      "<strong>Z (1:220)</strong>: Rokuhan (Z-scale integrated roadbed track system)",
    ],
    callout:
      "<strong>Testing & Platform Support:</strong><br>• Tested & Verified on: <strong>Windows x86_64</strong> and <strong>Linux (x86_64 AppImage / Flatpak)</strong>.<br>• Built & Available for: <strong>macOS</strong> (Apple Silicon M1-M4 arm64 & Intel x64), <strong>Windows ARM64</strong>, and <strong>Linux ARM64</strong>.",
    licenseNote:
      "Licensed under the GNU General Public License v3 (GPLv3). All built-in procedural 3D models are open source and royalty-free. All track product names, numbers, and brands (Märklin, Trix, Piko, Roco, Fleischmann, Tillig) are trademarks of their respective manufacturers.",
  },
};
