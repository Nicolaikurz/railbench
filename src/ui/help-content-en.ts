import type { HelpTab } from "./help-dialog";
import { getAppVersionTag } from "@/version";

export function renderEnglishTab(tab: HelpTab, contentArea: HTMLElement) {
  switch (tab) {
    case "overview":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Getting Started with Railbench</h2>
          <p>
            <strong>Railbench</strong> is a high-precision, web- and desktop-based track planning application
            engineered for model railway hobbyists. It combines millimetric CAD geometry with electrical simulation,
            multi-level layout design, and print-ready schematics.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Adding Tracks</h3>
              <p>
                Browse track pieces in the <strong>left catalog sidebar</strong> (press <code>B</code> to toggle).
                Click any piece to append it, or <strong>drag and drop</strong> it directly onto the canvas.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Smart Snapping</h3>
              <p>
                Automatic magnetic snapping (<code>S</code> key) seamlessly aligns track ends, calculates exact
                angles, and connects track geometry within millimetric tolerances.
              </p>
            </div>

            <div class="help-card">
              <h3>3. Rotating & Moving</h3>
              <p>
                Drag tracks with the mouse to move them. Rotate in 7.5° steps using <code>R</code> (or <code>Shift+R</code> for counter-clockwise).
                Hold <code>Shift + Mouse Wheel</code> to rotate smoothly, or drag the blue rotation handle.
                You can also enter any custom angle directly in the Angle input field in the bottom selection bar!
              </p>
            </div>

            <div class="help-card">
              <h3>4. Multi-Selection (Marquee Box)</h3>
              <p>
                Click directly on any track to select it. Click and drag with the <strong>Left Mouse Button in an empty area</strong> to draw a selection rectangle (marquee box).
                All tracks within the rectangle will be selected. Hold <code>Ctrl</code> or <code>Shift</code> to add to your existing selection or toggle individual tracks.
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Pro-Tip for Navigation:</strong>
            Hold <strong>Right Mouse Button</strong> or the <strong>Mouse Wheel (Middle Click)</strong> to pan the canvas. Use the <strong>Mouse Wheel</strong> to zoom in and out. Press <code>Ctrl+0</code> anytime to re-center the layout.
          </div>
        </div>
      `;
      break;

    case "layers":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>️ Multi-Deck Layers & Height Management</h2>
          <p>
            Railbench allows designing multi-level model train layouts (e.g. Ground level, Upper mountain branches,
            ramps, and hidden Staging Yards / Shadow Stations) with real vertical millimeter elevations.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Floating Layers Menu</h3>
              <p>
                Press <code>Shift+B</code> or click the <strong>Ebenen</strong> icon in the toolbar to toggle the floating
                layers manager. Layers are sorted by height with the highest deck at the top.
              </p>
            </div>

            <div class="help-card">
              <h3>Visibility & Lock Protection</h3>
              <p>
                Click the <strong>Eye icon</strong> icon to hide a layer when working on tracks underneath.
                Click the <strong>Lock icon</strong> icon to freeze a layer against accidental movement or editing.
              </p>
            </div>

            <div class="help-card">
              <h3>Rename Layers Inline</h3>
              <p>
                Double-click any layer name to rename it immediately inline. Press <code>Enter</code> to confirm or <code>Esc</code> to cancel. Click the elevation badge (e.g. <code>+80 mm</code>) to adjust its base height.
              </p>
            </div>

            <div class="help-card">
              <h3>Move Tracks to Layer (M)</h3>
              <p>
                Select tracks and press <code>M</code> or click <strong>"↳ Move & Set Height"</strong> on any layer card.
                The tracks will be assigned to that layer and their height automatically synchronized.
              </p>
            </div>

            <div class="help-card">
              <h3>Ramp & Incline Calculator</h3>
              <p>
                Configure gradual elevation transitions (e.g. 0 mm → 80 mm). The built-in calculator divides elevation
                difference by total track length and shows a live gradient percentage:
                <br>• <strong>&le; 3.0%</strong>: Recommended (Gentle incline)
                <br>• <strong>3.0% – 4.5%</strong>: Moderate (Watch heavy trains)
                <br>• <strong>&gt; 4.5%</strong>: Warning (Risk of wheel slippage)
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Canvas Selection Bar:</strong> When one or more tracks are selected, a sleek floating bar appears at the bottom center of the screen, allowing you to change layer and height in a single click!
          </div>
        </div>
      `;
      break;

    case "print":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>️ Layer-Separated Layout Export / Printing (Ctrl+P)</h2>
          <p>
            Traditional track planning software prints or exports the whole layout as one overlapping tangle. Railbench introduces
            <strong>true layer-separated export</strong> designed for building real layouts benchwork by benchwork.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. One Page Per Layer</h3>
              <p>
                Every layer is exported on its own dedicated sheet. Automatic pagination guarantees clean page breaks
                without overlapping content.
              </p>
            </div>

            <div class="help-card">
              <h3>2. 100% Identical Scale & Bounding Box</h3>
              <p>
                All exported layer sheets share the exact same bounding box, origin, and scale. You can view or print transparent
                or paper sheets, stack them on top of each other, and verify that bridge pillars, ramp portals, and track curves align perfectly!
              </p>
            </div>

            <div class="help-card">
              <h3>3. Ghosted Reference Outlines</h3>
              <p>
                Optionally show faint, dimmed outlines of the layers above and below on the current sheet so you have full spatial context.
              </p>
            </div>

            <div class="help-card">
              <h3>4. Layer-Specific Bill of Materials</h3>
              <p>
                Each sheet includes a bottom inventory banner showing exactly which track pieces and quantities are required to construct
                that specific layer (e.g., <code>24188: 8×, 24172: 4×</code>).
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>How to Export / Print:</strong> Press <code>Ctrl+P</code> (or <code>Cmd+P</code> on macOS), choose your layers in the preview dialog, and click <strong>"System Print..."</strong> or <strong>"Export as PDF"</strong>.
          </div>
        </div>
      `;
      break;

    case "power":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Traction Voltage Drop & Analog Simulation</h2>
          <p>
            Ensure reliable operation of digital and analog locomotives with Railbench's integrated
            electrical simulation engine.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Color Heatmap (P Key)</h3>
              <p>
                Press <code>P</code> to toggle the voltage heatmap:
                <br>• <strong style="color: #10b981;">Green</strong>: Optimal voltage (&gt; 92% of supply)
                <br>• <strong style="color: #f59e0b;">Amber</strong>: Moderate drop (&gt; 80% of supply)
                <br>• <strong style="color: #ef4444;">Red</strong>: Critical drop (&lt; 80% — add feed point!)
                <br>• <strong style="color: #dc2626;">Dashed Red</strong>: Isolated / disconnected block
              </p>
            </div>

            <div class="help-card">
              <h3>Power Feeder Points (F Key)</h3>
              <p>
                Select a track and press <code>F</code> to attach a power feed point (e.g. from your booster or transformer).
                Railbench immediately recomputes voltage distribution across the entire rail network.
              </p>
            </div>

            <div class="help-card">
              <h3>Track Isolators (I Key)</h3>
              <p>
                Select a track and press <code>I</code> to place an electrical gap (Trennstelle). Use isolators to simulate separate
                booster districts, analog block control, signal stopping sections, or staging yard siding cutoffs.
              </p>
            </div>

            <div class="help-card">
              <h3>Supply Voltage & Traction Current</h3>
              <p>
                Click the arrow next to the Voltage button in the toolbar and select <strong>Custom Voltage & Current...</strong> to configure voltage and traction currents:
                <br>• <strong>16V AC / Digital</strong>: Märklin H0 Standard
                <br>• <strong>14V DC / DCC</strong>: 2-Rail H0 (Piko, Roco, Trix)
                <br>• <strong>12V DC</strong>: N Scale & TT Scale
                <br>• <strong>18V DC</strong>: Gauge 1 & LGB Garden Railways
                <br>• <strong>Locomotive Current Presets</strong>:
                <br>&nbsp;&nbsp;• <strong>N / Z</strong>: 50–150 mA (typ. 100 mA)
                <br>&nbsp;&nbsp;• <strong>TT</strong>: 200–400 mA (typ. 300 mA)
                <br>&nbsp;&nbsp;• <strong>H0</strong>: 300–600 mA (typ. 450 mA)
                <br>&nbsp;&nbsp;• <strong>Max. per Loco</strong>: 1 A (1000 mA)
                <br>• <strong>Multi-Train Load</strong>: Multiply current by the number of simultaneously operating locomotives.
              </p>
            </div>
          </div>
        </div>
      `;
      break;

    case "baseboard":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Table & Baseboard Benchwork Planning</h2>
          <p>
            Design your layout to fit your actual room and carpentry constraints using realistic baseboard boundaries.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Modular Table Shapes</h3>
              <p>
                Choose between four common model railway benchwork forms:
                <br>• <strong>Rectangular</strong>: Classic flat table or island layout
                <br>• <strong>L-Shape</strong>: Corner layouts with left or right return wing
                <br>• <strong>U-Shape</strong>: Walk-in operator pit layout
                <br>• <strong>T-Shape</strong>: Peninsula or branch line extension
              </p>
            </div>

            <div class="help-card">
              <h3>Wood Finishes & Materials</h3>
              <p>
                Select realistic table textures including <strong>Natural Birch Wood</strong>, <strong>Dark Walnut</strong>,
                <strong>Plywood</strong>, <strong>Dark Slate</strong>, or clean <strong>Drafting White</strong>.
              </p>
            </div>

            <div class="help-card">
              <h3>Live Dimensions & Rulers</h3>
              <p>
                Displays millimetric dimension arrows directly on the board. Toggle millimeter rulers along canvas borders with <code>Shift+L</code>.
              </p>
            </div>

            <div class="help-card">
              <h3>Fit Table to Tracks</h3>
              <p>
                In the Table tab of the left sidebar, click <strong>"Fit Table to Tracks"</strong> to automatically calculate the minimum table dimensions
                needed to enclose all track pieces with a safe buffer.
              </p>
            </div>
          </div>
        </div>
      `;
      break;

    case "view3d":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>3D Perspective & Elevation View <span class="v3d-alpha-tag">ALPHA</span></h2>
          <p>
            The 3D Viewer renders your entire model railroad layout in real-time three dimensions, including the baseboard table, multi-deck elevations, bridge pillars, ballasting, and tracks.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>Camera Navigation</h3>
              <p>
                • <strong>Orbit / Rotate:</strong> Left-click and drag across the 3D scene.<br>
                • <strong>Pan:</strong> Right-click and drag (or Shift + Left-click).<br>
                • <strong>Zoom:</strong> Scroll wheel or trackpad pinch.<br>
                • <strong>Reset:</strong> Click "⌖ Reset View" to re-center on the benchwork.
              </p>
            </div>

            <div class="help-card">
              <h3>Bridges & Elevated Decks</h3>
              <p>
                Tracks on elevated layers (or custom ramps) are rendered at their true vertical heights. Tracks elevated &gt; 15 mm automatically generate realistic bridge support pillars connecting to the baseboard!
              </p>
            </div>

            <div class="help-card">
              <h3>Modular 3D Benchwork</h3>
              <p>
                The wooden baseboard table (Rectangular, L-shape, U-shape, or T-profile) is extruded in full 3D with authentic wood grain shading and sturdy steel table legs.
              </p>
            </div>

            <div class="help-card">
              <h3>3D Models & Scenery (.glb / .gltf)</h3>
              <p>
                Place built-in open-source models (Passenger Station, Signal Box, Engine Shed, Water Tower, Half-Timbered House, Light Signal, Buffer Stop, Trees, Platform Lamps) or import custom <code>.glb</code> / <code>.gltf</code> files via the <strong>3D Models</strong> sidebar tab or by dragging files onto the canvas!
              </p>
            </div>

            <div class="help-card">
              <h3>3D Models Licensing & Origin</h3>
              <p>
                • <strong>Built-in 3D Scenery:</strong> All 10 included models are <strong>100% procedurally generated pure code</strong> (Three.js procedural geometry in <code>src/render/builtin-models-3d.ts</code>). They contain no third-party binary assets or proprietary textures and are fully covered by the project's open-source license (GPLv3 / royalty-free for personal, hobby, and commercial layout design).<br>
                • <strong>Imported Models (.glb / .gltf):</strong> Custom models imported from external sources remain governed by their respective authors' licenses (e.g. Creative Commons CC0, CC-BY).
              </p>
            </div>

            <div class="help-card">
              <h3>Alpha Status Notice</h3>
              <p>
                The 3D viewer is currently in <strong>Alpha preview</strong>. Features like 3D track connection editing, detailed scenery, and catenary are actively in development. Press <code>3</code>, <code>V</code>, or <code>Esc</code> anytime to return to 2D planning.
              </p>
            </div>
          </div>

          <div class="help-callout">
            <strong>Quick Switch:</strong> Press <code>3</code> or <code>V</code> at any time to toggle between the 2D CAD drafting mode and the 3D perspective visualization.
          </div>
        </div>
      `;
      break;

    case "server":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>Multi-User Collaboration & Server Auto-Start</h2>
          <p>
            Railbench allows multiple modelers to build and design layouts simultaneously in real time across a local area network (LAN) or over the Internet.
          </p>

          <div class="help-feature-grid">
            <div class="help-card">
              <h3>1. Real-Time Synchronization</h3>
              <p>
                Track placements, drags, rotations, gradient ramps, and baseboard adjustments propagate instantly without lag to all connected peers.
              </p>
            </div>

            <div class="help-card">
              <h3>2. Live Cursors & Awareness</h3>
              <p>
                Every participant has a unique color and name tag. See remote mouse movements and track selections live on the canvas.
              </p>
            </div>

            <div class="help-card">
              <h3>3. Password Protection & Rooms</h3>
              <p>
                Rooms can be secured with passwords to prevent unauthorized access and protect layouts during collaborative sessions.
              </p>
            </div>

            <div class="help-card">
              <h3>4. Automatic Persistence</h3>
              <p>
                The server automatically persists all changes to disk under <code>./saved_plans/&lt;room&gt;.gleisplan</code>.
              </p>
            </div>
          </div>

          <div class="help-topic-section" style="margin-top: 20px;">
            <h3>Start Variables for Server Auto-Start (CLI & Environment)</h3>
            <p>
              Launch Railbench with the collaboration server auto-started in the background without needing to toggle menus manually.
            </p>

            <table class="help-shortcuts-table">
              <thead>
                <tr>
                  <th style="width: 35%;">Start Variable / Flag</th>
                  <th style="width: 25%;">Default</th>
                  <th style="width: 40%;">Description</th>
                </tr>
              </thead>
              <tbody>
                <tr><td><code>--server</code> or <code>--collab-server</code></td><td>off</td><td>Automatically starts the embedded WebSocket server on application launch.</td></tr>
                <tr><td><code>--port &lt;Port&gt;</code> or <code>--server-port</code></td><td>3000</td><td>Custom network port (e.g. <code>--port 8080</code>).</td></tr>
                <tr><td><code>--password &lt;PW&gt;</code></td><td>none</td><td>Configures a password requirement for the default room.</td></tr>
                <tr><td><code>--room &lt;Name&gt;</code></td><td>default</td><td>Sets the active collaborative room name (e.g. <code>--room central-station</code>).</td></tr>
                <tr><td><code>--connect</code> or <code>--client</code></td><td>off</td><td>Automatically connects the local window to the server upon launch.</td></tr>
              </tbody>
            </table>

            <h4 style="margin-top: 14px; margin-bottom: 6px;">Environment Variables:</h4>
            <table class="help-shortcuts-table">
              <thead>
                <tr>
                  <th style="width: 35%;">Variable</th>
                  <th style="width: 25%;">Example</th>
                  <th style="width: 40%;">Effect</th>
                </tr>
              </thead>
              <tbody>
                <tr><td><code>RAILBENCH_SERVER</code></td><td><code>1</code> or <code>true</code></td><td>Auto-starts the server on launch.</td></tr>
                <tr><td><code>RAILBENCH_SERVER_PORT</code></td><td><code>3000</code></td><td>Configures the server port (or <code>COLLAB_PORT</code> / <code>PORT</code>).</td></tr>
                <tr><td><code>RAILBENCH_SERVER_PASSWORD</code></td><td><code>secret123</code></td><td>Sets the room password.</td></tr>
                <tr><td><code>RAILBENCH_SERVER_ROOM</code></td><td><code>staging-yard</code></td><td>Configures default room name.</td></tr>
                <tr><td><code>RAILBENCH_CONNECT</code></td><td><code>1</code></td><td>Auto-connects local client on start.</td></tr>
              </tbody>
            </table>
          </div>

          <div class="help-callout" style="margin-top: 16px;">
            <strong>Quickstart Examples:</strong><br>
            • Desktop App with Server & Password: <code>railbench --server --port 3000 --password secret --connect</code><br>
            • Standalone Node.js Server: <code>node server.mjs --port 8080 --password secret --room yard1</code><br>
            • 1-Click Browser Link for Collaborators: <code>http://&lt;IP-Address&gt;:3000/?collab=1&room=default&pass=secret</code>
          </div>
        </div>
      `;
      break;

    case "shortcuts":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>⌨️ Keyboard Shortcuts Reference</h2>
          <p>Master these shortcuts for rapid layout design and editing:</p>

          <table class="help-shortcuts-table">
            <thead>
              <tr>
                <th style="width: 25%;">Shortcut</th>
                <th style="width: 40%;">Action</th>
                <th style="width: 35%;">Category</th>
              </tr>
            </thead>
            <tbody>
              <tr><td><kbd>Ctrl</kbd> + <kbd>N</kbd></td><td>New Layout Plan</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>O</kbd></td><td>Open Plan File (.railbench / .gleisplan / XML)</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>S</kbd></td><td>Save Plan (Quick Save)</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd></td><td>Save Plan As...</td><td>File</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>W</kbd></td><td><strong>Close Plan / Dialog / Window</strong></td><td>File / Navigation</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>P</kbd></td><td><strong>Print Plan (Layer-Separated Pages)</strong></td><td>Print / Export</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>E</kbd></td><td>Export Bill of Materials (CSV)</td><td>Print / Export</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>Z</kbd></td><td>Undo last action</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>Y</kbd></td><td>Redo undone action</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>C</kbd></td><td>Copy selected tracks</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>V</kbd></td><td>Paste tracks from clipboard</td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>D</kbd></td><td><strong>Duplicate selected tracks</strong></td><td>Edit</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>A</kbd></td><td>Select all tracks</td><td>Selection</td></tr>
              <tr><td><kbd>R</kbd></td><td>Rotate selected item +7.5° clockwise</td><td>Editing</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>R</kbd></td><td>Rotate selected item -7.5° counter-clockwise</td><td>Editing</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>Wheel</kbd></td><td>Rotate track under cursor by mouse wheel</td><td>Editing</td></tr>
              <tr><td><kbd>Left Click</kbd> + Drag (empty area)</td><td><strong>Marquee Box Selection (Multiple Tracks)</strong></td><td>Selection</td></tr>
              <tr><td><kbd>Right Click</kbd> / <kbd>Middle Click</kbd> + Drag</td><td>Pan Canvas Viewport</td><td>Navigation</td></tr>
              <tr><td><kbd>Ctrl</kbd> / <kbd>Shift</kbd> + <kbd>Left Click</kbd></td><td>Toggle track in selection (Multi-selection)</td><td>Selection</td></tr>
              <tr><td><kbd>Alt</kbd> + <kbd>L</kbd></td><td><strong>Toggle Selection Lock (Select locked & deleted layer items)</strong></td><td>Workspace / Layers</td></tr>
              <tr><td><kbd>M</kbd></td><td><strong>Change Track Height / Move to Layer</strong></td><td>Layers</td></tr>
              <tr><td><kbd>Del</kbd> / <kbd>Backspace</kbd></td><td>Delete selected tracks</td><td>Editing</td></tr>
              <tr><td><kbd>F</kbd></td><td>Toggle Power Feeder point</td><td>Electrical</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>F</kbd></td><td>Toggle Catenary Power Feeder</td><td>Electrical / Catenary</td></tr>
              <tr><td><kbd>I</kbd></td><td>Toggle Track Isolator / Gap</td><td>Electrical</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>I</kbd></td><td>Toggle Catenary Section Insulator</td><td>Electrical / Catenary</td></tr>
              <tr><td><kbd>O</kbd></td><td>Toggle Catenary (Overhead wire & masts)</td><td>Electrical / 3D</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>D</kbd></td><td>Toggle Digital Turnout Decoder</td><td>Electrical / Turnouts</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>L</kbd></td><td>Toggle Lighted Turnout Lantern</td><td>Electrical / Turnouts</td></tr>
              <tr><td><kbd>P</kbd></td><td>Toggle Traction Voltage Heatmap Simulation</td><td>Electrical</td></tr>
              <tr><td><kbd>3</kbd> or <kbd>V</kbd></td><td><strong>Toggle 3D Elevation & Perspective View (Alpha)</strong></td><td>View / 3D</td></tr>
              <tr><td><kbd>Esc</kbd></td><td>Exit 3D View / Close Dialogs</td><td>View / Navigation</td></tr>
              <tr><td><kbd>S</kbd></td><td>Toggle Automatic Track Snapping</td><td>View / Snapping</td></tr>
              <tr><td><kbd>C</kbd></td><td>Toggle Track Geometry Color-Coding</td><td>View</td></tr>
              <tr><td><kbd>N</kbd></td><td>Toggle Track Item Part Numbers</td><td>View</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>B</kbd></td><td>Toggle Floating Layers Manager Menu</td><td>Workspace</td></tr>
              <tr><td><kbd>Shift</kbd> + <kbd>L</kbd></td><td>Toggle Millimeter Edge Rulers</td><td>View</td></tr>
              <tr><td><kbd>Ctrl</kbd> + <kbd>0</kbd></td><td>Reset View / Center on Layout</td><td>View</td></tr>
              <tr><td><kbd>F1</kbd> or <kbd>?</kbd></td><td>Open this Help & Feature Guide</td><td>Help</td></tr>
            </tbody>
          </table>
        </div>
      `;
      break;

    case "about":
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>About Railbench & Track Libraries</h2>
          <p>
            <strong>Railbench ${getAppVersionTag()}</strong> is an open-source model railway track design and layout planning CAD tool.
            It is built using modern TypeScript, PixiJS WebGL rendering, Vite, and Electron.
          </p>

          <h3>Supported Scale Systems & Catalogs:</h3>
          <ul class="help-systems-list">
            <!-- H0 Scale (1:87) -->
            <li><strong>H0 (1:87)</strong>: Märklin C-Gleis (Full 3-rail with roadbed geometry, curves R1–R5, turnouts, crossings)</li>
            <li><strong>H0 (1:87)</strong>: Märklin K-Gleis (Prototypical 3-rail tie-only track without roadbed, R1–R5, slim turnouts 14° 26' and flex-track)</li>
            <li><strong>H0 (1:87)</strong>: Märklin M-Gleis (Classic sheet-metal roadbed 3-rail track, R1, R2, industrial circle and crossings)</li>
            <li><strong>H0 (1:87)</strong>: Trix C-Gleis (2-rail DC with C-track roadbed geometry, identical footprint)</li>
            <li><strong>H0 (1:87)</strong>: Piko A-Gleis (2-rail standard gauge, flexible geometries, turnouts)</li>
            <li><strong>H0 (1:87)</strong>: Roco Line (2-rail realistic high-fidelity track with roadbed)</li>
            <li><strong>H0 (1:87)</strong>: Peco Streamline Code 100 (Universal 2-rail standard gauge system)</li>
            <li><strong>H0 (1:87)</strong>: Tillig Elite (Fine scale 2-rail prototypical track system)</li>

            <!-- TT Scale (1:120) -->
            <li><strong>TT (1:120)</strong>: Tillig TT Modellgleis</li>
            <li><strong>TT (1:120)</strong>: Tillig TT Bettungsgleis (Track system with integrated roadbed)</li>
            <li><strong>TT (1:120)</strong>: Kuehn TT Gleis (2-rail TT track geometry)</li>

            <!-- N Scale (1:160) -->
            <li><strong>N (1:160)</strong>: Fleischmann Piccolo (N-scale with roadbed ballasting)</li>
            <li><strong>N (1:160)</strong>: Kato Unitrack (Integrated roadbed track system with Unijoiners)</li>
            <li><strong>N (1:160)</strong>: Minitrix (Classic 2-rail tie-only track system)</li>

            <!-- Z Scale (1:220) -->
            <li><strong>Z (1:220)</strong>: Märklin mini-club (Classic Z-scale 2-rail track system)</li>
            <li><strong>Z (1:220)</strong>: Rokuhan (Z-scale integrated roadbed track system)</li>
          </ul>

          <div class="help-callout">
            <strong>Testing & Platform Support:</strong>
            <br>• Tested & Verified on: <strong>Windows x86_64</strong> and <strong>Linux (x86_64 AppImage / Flatpak)</strong>.
            <br>• Built & Available for: <strong>macOS</strong> (Apple Silicon M1-M4 arm64 & Intel x64), <strong>Windows ARM64</strong>, and <strong>Linux ARM64</strong>.
          </div>

          <p style="font-size: 11px; color: var(--text-muted); margin-top: 20px;">
            Licensed under the GNU General Public License v3 (GPLv3). All built-in procedural 3D models are open source and royalty-free. All track product names, numbers, and brands (Märklin, Trix, Piko, Roco, Fleischmann, Tillig) are trademarks of their respective manufacturers.
          </p>
        </div>
      `;
      break;
  }
}
