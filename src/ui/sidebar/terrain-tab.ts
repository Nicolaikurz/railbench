import type { Sidebar } from "@/ui/sidebar";
import { el, btn, row, numberField, selectField } from "./board-tab";
import { createBuiltinModel } from "@/core/models-3d";
import { findCollidingTracksForMountain } from "@/render/builtin-models-3d";
import { catalogRegistry } from "@/catalog/catalog";
import { getBaseboards, getBaseboardBounds } from "@/core/layout";
import type { Model3D, MountainStyle } from "@/core/types";
import { getLanguage } from "@/i18n";
import {
  terrainBrush,
  applyAutoTunnelAllMountains,
  applyAutoTunnelForMountain,
  smoothMountainPath,
  type TerrainBrushMode,
  type TerrainRidgeProfile,
} from "@/editor/terrain-brush-controller";

function textField(
  label: string,
  value: string,
  placeholder: string,
  onChange: (v: string) => void,
): HTMLElement {
  const input = el("input", { cls: "board-select", style: "width:100%;" });
  input.type = "text";
  input.value = value;
  input.placeholder = placeholder;
  input.addEventListener("change", () => onChange(input.value));
  return el("div", { cls: "board-field" }, [el("label", { text: label }), input]);
}

function checkboxField(label: string, checked: boolean, onChange: (v: boolean) => void): HTMLElement {
  const input = el("input");
  input.type = "checkbox";
  input.checked = checked;
  input.addEventListener("change", () => onChange(input.checked));
  return el("label", { cls: "board-toggle-label", style: "margin-top:4px;font-size:11px;" }, [
    input,
    el("span", { text: label }),
  ]);
}

/**
 * Eigenes Hauptmenü (Tab "Gelände" / "Terrain") für Gebirge, Pinsel-/Stift-Werkzeuge
 * und Gleis-/Straßen-Tunnelsteuerung (ausschließlich monochrome Icons, reduzierte Icon-Dichte).
 */
export function renderTerrainTab(sidebar: Sidebar, container: HTMLElement): void {
  const isDe = getLanguage() === "de";
  const baseboards = getBaseboards(sidebar.layout);
  const bb = baseboards.find((b) => b.id === sidebar.activeBoardId) ?? baseboards[0]!;
  const wrap = container;

  const notifyAndSync = () => {
    sidebar.callbacks.onBaseboardChange?.();
  };
  const refresh = () => {
    sidebar.render();
  };

  const mountains: Model3D[] = (sidebar.layout.models || []).filter((m) => m.builtinId === "mountain");

  const insertMountain = (newMountain: Model3D) => {
    if (terrainBrush.autoTunnel) {
      applyAutoTunnelForMountain(newMountain, sidebar.layout);
    }
    if (sidebar.callbacks.onAddModel) {
      sidebar.callbacks.onAddModel(newMountain);
    } else {
      if (!sidebar.layout.models) sidebar.layout.models = [];
      sidebar.layout.models.push(newMountain);
      notifyAndSync();
    }
    sidebar.activeMountainId = newMountain.id;
    terrainBrush.targetMountainId = newMountain.id;
    sidebar.callbacks.onSelectModel?.(newMountain.id);
    refresh();
  };

  /* ================================================================
   * 1. GELÄNDE-PINSEL & STIFT (2D-Gelände-Editor)
   * ================================================================ */
  const brushCard = el("div", { cls: "board-card" });
  brushCard.appendChild(
    el("div", {
      cls: "board-section-title",
      text: isDe ? "Gelände-Pinsel & Stift" : "Terrain Brush & Pencil",
    }),
  );

  const modeGrid = el("div", {
    style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;margin-bottom:6px;",
  });

  const makeModeBtn = (
    mode: TerrainBrushMode,
    monoSymbol: string,
    labelDe: string,
    labelEn: string,
    tipDe: string,
    tipEn: string,
  ) => {
    const active = terrainBrush.isActive && terrainBrush.mode === mode;
    const text = `${monoSymbol} ${isDe ? labelDe : labelEn}${active ? " ✓" : ""}`;
    return btn(
      text,
      isDe ? tipDe : tipEn,
      () => {
        if (mode === "extend" && sidebar.activeMountainId) {
          terrainBrush.toggle("extend", sidebar.activeMountainId);
        } else {
          terrainBrush.toggle(mode);
        }
        notifyAndSync();
        refresh();
      },
      active ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
      `font-size:11px;padding:6px 8px;${active ? "font-weight:700;" : ""}`,
    );
  };

  modeGrid.appendChild(
    makeModeBtn(
      "paint",
      "✎",
      "Kamm-Pinsel",
      "Ridge Brush",
      "Im 2D-Plan ziehen, um einen neuen Gebirgszug zu zeichnen",
      "Drag on the 2D plan to paint a new mountain ridge",
    ),
  );
  modeGrid.appendChild(
    makeModeBtn(
      "extend",
      "+",
      "Berg erweitern",
      "Extend Massif",
      "Erweitert den ausgewählten (oder nächsten) Berg nahtlos um weitere Pinselstriche",
      "Seamlessly extend the selected or nearest mountain with additional brush strokes",
    ),
  );
  modeGrid.appendChild(
    makeModeBtn(
      "stamp",
      "▲",
      "Gipfel-Stempel",
      "Peak Stamp",
      "Einzelklick setzt sofort einen einzelnen Berggipfel mit aktuellem Radius & Höhe",
      "Single click stamps an individual mountain peak with current radius & height",
    ),
  );
  modeGrid.appendChild(
    makeModeBtn(
      "erase",
      "✕",
      "Radierer",
      "Eraser",
      "Löscht einzelne Pinsel-Knoten eines Gebirgskamms oder ganze Berge beim Überfahren",
      "Erase individual mountain path nodes or entire mountains by dragging over them",
    ),
  );
  brushCard.appendChild(modeGrid);

  if (terrainBrush.isActive) {
    const modeHints: Record<TerrainBrushMode, string> = {
      paint: isDe
        ? "Klicken & Ziehen im 2D-Plan zeichnet einen Gebirgskamm. Shift+Mausrad ändert Radius. Rechtsklick beendet."
        : "Click & drag on the 2D plan to paint a ridge. Shift+Wheel adjusts radius. Right-click to exit.",
      extend: isDe
        ? "Ziehe im 2D-Plan, um neue Ausläufer direkt an das bestehende Gebirgsmassiv anzufügen."
        : "Drag on the 2D plan to merge new ridges into the existing mountain massif.",
      stamp: isDe
        ? "Klicke in den 2D-Plan, um Einzelgipfel zu stempeln. Rechtsklick beendet."
        : "Click on the 2D plan to stamp individual peaks. Right-click to exit.",
      erase: isDe
        ? "Überfahre Gebirgsknoten oder Berge im 2D-Plan, um Teile oder ganze Berge abzutragen."
        : "Drag over mountain nodes or peaks on the 2D plan to erase parts or whole mountains.",
    };
    brushCard.appendChild(
      el("div", {
        cls: "board-field-note",
        style:
          "font-size:10.5px;color:var(--text-main);padding:5px 7px;border-radius:4px;background:rgba(255,255,255,0.06);margin-bottom:6px;line-height:1.35;",
        text: modeHints[terrainBrush.mode],
      }),
    );
  }

  // Schnell-Presets für Pinselgröße
  const sizePresetRow = el("div", { style: "display:flex;gap:4px;margin-bottom:6px;" });
  const sizePresets: Array<{ labelDe: string; labelEn: string; r: number; h: number }> = [
    { labelDe: "Hügel (130)", labelEn: "Hill (130)", r: 130, h: 120 },
    { labelDe: "Mittel (220)", labelEn: "Med (220)", r: 220, h: 220 },
    { labelDe: "Groß (320)", labelEn: "Large (320)", r: 320, h: 290 },
    { labelDe: "Alpin (440)", labelEn: "Alpine (440)", r: 440, h: 360 },
  ];
  for (const sp of sizePresets) {
    const isCur = Math.abs(terrainBrush.brushRadius - sp.r) < 15;
    sizePresetRow.appendChild(
      btn(
        isDe ? sp.labelDe : sp.labelEn,
        `${sp.r} mm Radius / ${sp.h} mm ${isDe ? "Höhe" : "Height"}`,
        () => {
          terrainBrush.brushRadius = sp.r;
          terrainBrush.brushHeight = sp.h;
          notifyAndSync();
          refresh();
        },
        isCur ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
        "flex:1;font-size:10px;padding:3px 4px;",
      ),
    );
  }
  brushCard.appendChild(sizePresetRow);

  brushCard.appendChild(
    row([
      numberField(
        isDe ? "Pinsel-Radius" : "Brush Radius",
        Math.round(terrainBrush.brushRadius),
        { min: 60, max: 600, step: 20, unit: "mm" },
        (v) => {
          terrainBrush.brushRadius = Math.max(60, Math.min(600, v));
          notifyAndSync();
        },
      ),
      numberField(
        isDe ? "Gipfel-Höhe" : "Peak Height",
        Math.round(terrainBrush.brushHeight),
        { min: 40, max: 500, step: 20, unit: "mm" },
        (v) => {
          terrainBrush.brushHeight = Math.max(40, Math.min(500, v));
          notifyAndSync();
        },
      ),
    ]),
  );

  brushCard.appendChild(
    row([
      selectField<MountainStyle>(
        isDe ? "Biom / Oberfläche" : "Biome / Surface",
        [
          ["alpine", isDe ? "Alpen (Schnee & Fels)" : "Alpine (Snow & Rock)"],
          ["rocky", isDe ? "Felsmassiv (Granit)" : "Rocky Massif"],
          ["forest", isDe ? "Mittelgebirge (Wald)" : "Forested Ridge"],
          ["meadow", isDe ? "Gras-Hügel / Voralpen" : "Green Meadow Hill"],
        ],
        terrainBrush.brushStyle,
        (style) => {
          terrainBrush.brushStyle = style;
          notifyAndSync();
          refresh();
        },
      ),
      selectField<TerrainRidgeProfile>(
        isDe ? "Kamm-Höhenprofil" : "Ridge Profile",
        [
          ["uniform", isDe ? "Gleichmäßig" : "Uniform"],
          ["peak-center", isDe ? "Hauptgipfel mittig" : "Center Summit"],
          ["varied", isDe ? "Alpin-Zacken" : "Varied Peaks"],
        ],
        terrainBrush.ridgeProfile,
        (prof) => {
          terrainBrush.ridgeProfile = prof;
          notifyAndSync();
        },
      ),
    ]),
  );

  brushCard.appendChild(
    checkboxField(
      isDe
        ? "Kreuzende Gleise & Straßen automatisch als Tunnel markieren"
        : "Automatically tunnel intersecting tracks & streets",
      terrainBrush.autoTunnel,
      (checked) => {
        terrainBrush.autoTunnel = checked;
      },
    ),
  );

  wrap.appendChild(brushCard);

  /* ================================================================
   * 2. BERG-GENERATOR & VORLAGEN
   * ================================================================ */
  const presetCard = el("div", { cls: "board-card" });
  presetCard.appendChild(
    el("div", {
      cls: "board-section-title",
      text: isDe ? "Berg-Generator & Vorlagen" : "Mountain Generator & Presets",
    }),
  );

  const presetGrid = el("div", {
    style: "display:grid;grid-template-columns:1fr 1fr;gap:6px;",
  });

  presetGrid.appendChild(
    btn(
      isDe ? "+ Einzelberg" : "+ Single Peak",
      isDe ? "Neuen Berg auf die Tischplatte setzen" : "Place a new mountain on the table",
      () => {
        const count = mountains.length;
        const bounds = bb ? getBaseboardBounds(bb) : { x: 0, y: 0, width: 1000, depth: 800 };
        const centerX = Math.round(bounds.x + bounds.width / 2);
        const centerY = Math.round(bounds.y + bounds.depth / 2);
        const offset = (count % 5) * 60;
        const m = createBuiltinModel(
          "mountain",
          { x: centerX + offset, y: centerY + offset, rotation: 0 },
          sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
        );
        m.name = isDe ? `Berg ${count + 1}` : `Mountain ${count + 1}`;
        m.width = 600;
        m.depth = 450;
        m.height = terrainBrush.brushHeight;
        m.mountainStyle = terrainBrush.brushStyle;
        insertMountain(m);
      },
      "rb-btn rb-btn-secondary",
      "font-size:11px;",
    ),
  );

  presetGrid.appendChild(
    btn(
      isDe ? "⌐ Eck-Massiv (L)" : "⌐ Corner Massif (L)",
      isDe ? "L-förmiges Eckgebirge für Anlagen-Ecken" : "L-shaped mountain massif for layout corners",
      () => {
        const count = mountains.length;
        const bounds = bb ? getBaseboardBounds(bb) : { x: -1000, y: -600, width: 2000, depth: 1200 };
        const cx = Math.round(bounds.x + Math.min(420, bounds.width * 0.25));
        const cy = Math.round(bounds.y + Math.min(340, bounds.depth * 0.28));
        const m = createBuiltinModel(
          "mountain",
          { x: cx, y: cy, rotation: 0 },
          sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
        );
        m.name = isDe ? `Eck-Gebirge ${count + 1}` : `Corner Massif ${count + 1}`;
        m.width = 960;
        m.depth = 780;
        m.height = 240;
        m.mountainStyle = terrainBrush.brushStyle;
        m.mountainPath = [
          { x: -200, y: 180, radius: 220, height: 210 },
          { x: -200, y: -140, radius: 250, height: 240 },
          { x: 220, y: -160, radius: 220, height: 220 },
        ];
        insertMountain(m);
      },
      "rb-btn rb-btn-secondary",
      "font-size:11px;",
    ),
  );

  presetGrid.appendChild(
    btn(
      isDe ? "▲ Kulissen-Kette" : "▲ Backdrop Ridge",
      isDe ? "Langgestreckte Gebirgskette entlang des hinteren Anlagenrands" : "Long mountain chain along the rear table edge",
      () => {
        const count = mountains.length;
        const bounds = bb ? getBaseboardBounds(bb) : { x: -1000, y: -600, width: 2000, depth: 1200 };
        const cx = Math.round(bounds.x + bounds.width / 2);
        const cy = Math.round(bounds.y + Math.min(200, bounds.depth * 0.18));
        const w = Math.round(Math.max(800, bounds.width * 0.72));
        const halfSpan = Math.round(w * 0.34);
        const m = createBuiltinModel(
          "mountain",
          { x: cx, y: cy, rotation: 0 },
          sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
        );
        m.name = isDe ? `Gebirgskette ${count + 1}` : `Backdrop Ridge ${count + 1}`;
        m.width = w;
        m.depth = 420;
        m.height = 250;
        m.mountainStyle = terrainBrush.brushStyle;
        m.mountainPath = [
          { x: -halfSpan, y: 15, radius: 190, height: 210 },
          { x: 0, y: -15, radius: 210, height: 250 },
          { x: halfSpan, y: 10, radius: 190, height: 220 },
        ];
        insertMountain(m);
      },
      "rb-btn rb-btn-secondary",
      "font-size:11px;",
    ),
  );

  presetGrid.appendChild(
    btn(
      isDe ? "▲▲ Tal-Pass" : "▲▲ Valley Pass",
      isDe ? "Zwei gegenüberliegende Gebirgsflanken mit Tal-Durchfahrt" : "Two opposing mountain flanks forming a valley pass",
      () => {
        const count = mountains.length;
        const bounds = bb ? getBaseboardBounds(bb) : { x: -1000, y: -600, width: 2000, depth: 1200 };
        const cx = Math.round(bounds.x + bounds.width / 2);
        const cy = Math.round(bounds.y + bounds.depth / 2);
        const leftM = createBuiltinModel(
          "mountain",
          { x: cx - 320, y: cy - 120, rotation: -15 },
          sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
        );
        leftM.name = isDe ? `Talflanke West ${count + 1}` : `West Flank ${count + 1}`;
        leftM.width = 560;
        leftM.depth = 400;
        leftM.height = 240;
        leftM.mountainStyle = terrainBrush.brushStyle;

        const rightM = createBuiltinModel(
          "mountain",
          { x: cx + 320, y: cy + 120, rotation: -15 },
          sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
        );
        rightM.name = isDe ? `Talflanke Ost ${count + 2}` : `East Flank ${count + 2}`;
        rightM.width = 560;
        rightM.depth = 400;
        rightM.height = 220;
        rightM.mountainStyle = terrainBrush.brushStyle;

        insertMountain(leftM);
        insertMountain(rightM);
      },
      "rb-btn rb-btn-secondary",
      "font-size:11px;",
    ),
  );

  presetCard.appendChild(presetGrid);
  wrap.appendChild(presetCard);

  /* ================================================================
   * 3. GLEIS- & STRASSEN-TUNNEL WERKZEUG
   * ================================================================ */
  const tunnelCard = el("div", { cls: "board-card" });
  const tunneledPiecesCount = (sidebar.layout.pieces || []).filter((p) => !!p.hasTunnel).length;
  const selectedIds = sidebar.callbacks.selectedPieceIds?.() ?? [];
  const selectedTrackPieces = (sidebar.layout.pieces || []).filter((p) => selectedIds.includes(p.id));
  const allSelectedHaveTunnel =
    selectedTrackPieces.length > 0 && selectedTrackPieces.every((p) => !!p.hasTunnel);

  tunnelCard.appendChild(
    el("div", {
      cls: "board-section-title",
      text: isDe
        ? `Gleis- & Straßen-Tunnel (${tunneledPiecesCount})`
        : `Track & Street Tunnels (${tunneledPiecesCount})`,
    }),
  );

  const tunnelBtnCol = el("div", { style: "display:flex;flex-direction:column;gap:5px;" });
  tunnelBtnCol.appendChild(
    btn(
      selectedTrackPieces.length > 0
        ? allSelectedHaveTunnel
          ? (isDe ? `Tunnel von Auswahl entfernen (${selectedTrackPieces.length})` : `Remove Tunnel from Selection (${selectedTrackPieces.length})`)
          : (isDe ? `Auswahl als Tunnel setzen (${selectedTrackPieces.length})` : `Set Selection as Tunnel (${selectedTrackPieces.length})`)
        : (isDe ? "Auswahl: Tunnel umschalten (T)" : "Selection: Toggle Tunnel (T)"),
      isDe
        ? "Schaltet die Tunnelröhre & Portale für alle markierten Gleise oder Straßen ein/aus"
        : "Toggle stone tunnel portals and vault on selected tracks or streets",
      () => {
        if (selectedTrackPieces.length === 0) return;
        const shouldEnable = !allSelectedHaveTunnel;
        for (const p of selectedTrackPieces) {
          p.hasTunnel = shouldEnable;
          if (shouldEnable) p.hasBridge = false;
        }
        notifyAndSync();
        refresh();
      },
      allSelectedHaveTunnel ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
      "font-size:11px;",
    ),
  );

  const tunnelSubRow = el("div", { style: "display:flex;gap:5px;" });
  tunnelSubRow.appendChild(
    btn(
      isDe ? "Auto-Tunnel (Alle Berge)" : "Auto-Tunnel (All Mountains)",
      isDe
        ? "Markiert alle Gleise und Straßen, die durch einen Berg verlaufen, automatisch als Tunnel"
        : "Automatically mark all tracks and streets passing through any mountain as tunnels",
      () => {
        applyAutoTunnelAllMountains(sidebar.layout);
        notifyAndSync();
        refresh();
      },
      "rb-btn rb-btn-secondary",
      "flex:1;font-size:11px;",
    ),
  );

  if (tunneledPiecesCount > 0) {
    tunnelSubRow.appendChild(
      btn(
        isDe ? "✕ Alle lösen" : "✕ Clear All",
        isDe ? "Entfernt die Tunnel-Markierung von allen Gleisen und Straßen" : "Remove explicit tunnel flag from all tracks and streets",
        () => {
          for (const p of sidebar.layout.pieces || []) {
            p.hasTunnel = false;
          }
          notifyAndSync();
          refresh();
        },
        "rb-btn rb-btn-secondary",
        "flex:0 0 auto;font-size:11px;",
      ),
    );
  }
  tunnelBtnCol.appendChild(tunnelSubRow);
  tunnelCard.appendChild(tunnelBtnCol);
  wrap.appendChild(tunnelCard);

  /* ================================================================
   * 4. AUSGEWÄHLTER BERG – INSPEKTOR & FEINBEARBEITUNG
   * ================================================================ */
  const inspectorCard = el("div", { cls: "board-card" });
  inspectorCard.appendChild(
    el("div", {
      cls: "board-section-title",
      text: isDe
        ? `Gebirgs-Inspektor (${mountains.length})`
        : `Mountain Inspector (${mountains.length})`,
    }),
  );

  if (mountains.length > 0) {
    if (!sidebar.activeMountainId || !mountains.some((m) => m.id === sidebar.activeMountainId)) {
      sidebar.activeMountainId = mountains[0]!.id;
    }
    const currentMountain = mountains.find((m) => m.id === sidebar.activeMountainId) ?? mountains[0]!;
    terrainBrush.targetMountainId = currentMountain.id;

    const inspBody = el("div", { style: "display:flex;flex-direction:column;gap:8px;" });

    if (mountains.length > 1) {
      inspBody.appendChild(
        selectField<string>(
          isDe ? "Ausgewählter Berg" : "Selected Mountain",
          mountains.map((m) => [m.id, m.name || (isDe ? "Berg" : "Mountain")]),
          currentMountain.id,
          (id) => {
            sidebar.activeMountainId = id;
            terrainBrush.targetMountainId = id;
            sidebar.callbacks.onSelectModel?.(id);
            refresh();
          },
        ),
      );
    }

    inspBody.appendChild(
      row([
        textField(
          isDe ? "Bezeichnung" : "Name",
          currentMountain.name || "",
          isDe ? "z.B. Nordmassiv" : "e.g. North Ridge",
          (val) => {
            currentMountain.name = val;
            notifyAndSync();
          },
        ),
        selectField<MountainStyle>(
          isDe ? "Biom / Stil" : "Biome Style",
          [
            ["alpine", isDe ? "Alpen" : "Alpine"],
            ["rocky", isDe ? "Felsmassiv" : "Rocky"],
            ["forest", isDe ? "Wald" : "Forest"],
            ["meadow", isDe ? "Wiese" : "Meadow"],
          ],
          currentMountain.mountainStyle ?? "alpine",
          (style) => {
            currentMountain.mountainStyle = style;
            terrainBrush.brushStyle = style;
            notifyAndSync();
            refresh();
          },
        ),
      ]),
    );

    inspBody.appendChild(
      row([
        numberField("Position X", Math.round(currentMountain.transform.x), { step: 10, unit: "mm" }, (v) => {
          currentMountain.transform.x = v;
          notifyAndSync();
        }),
        numberField("Position Y", Math.round(currentMountain.transform.y), { step: 10, unit: "mm" }, (v) => {
          currentMountain.transform.y = v;
          notifyAndSync();
        }),
      ]),
    );

    inspBody.appendChild(
      row([
        numberField(
          isDe ? "Breite (X)" : "Width (X)",
          Math.round(currentMountain.width ?? 600),
          { min: 50, step: 20, unit: "mm" },
          (v) => {
            currentMountain.width = v;
            notifyAndSync();
          },
        ),
        numberField(
          isDe ? "Tiefe (Y)" : "Depth (Y)",
          Math.round(currentMountain.depth ?? 450),
          { min: 50, step: 20, unit: "mm" },
          (v) => {
            currentMountain.depth = v;
            notifyAndSync();
          },
        ),
      ]),
    );

    inspBody.appendChild(
      row([
        numberField(
          isDe ? "Höhe (Z)" : "Height (Z)",
          Math.round(currentMountain.height ?? 220),
          { min: 10, step: 10, unit: "mm" },
          (v) => {
            currentMountain.height = v;
            if (currentMountain.mountainPath) {
              for (const n of currentMountain.mountainPath) n.height = v;
            }
            notifyAndSync();
          },
        ),
        numberField(
          isDe ? "Drehung" : "Rotation",
          Math.round(currentMountain.transform.rotation ?? 0),
          { step: 5, unit: "°" },
          (v) => {
            currentMountain.transform.rotation = ((v % 360) + 360) % 360;
            notifyAndSync();
          },
        ),
      ]),
    );

    if (currentMountain.mountainPath && currentMountain.mountainPath.length >= 2) {
      const pathRow = el("div", { style: "display:flex;gap:5px;align-items:center;" });
      pathRow.appendChild(
        btn(
          isDe
            ? `∼ Kamm glätten (${currentMountain.mountainPath.length} Knoten)`
            : `∼ Smooth Ridge (${currentMountain.mountainPath.length} nodes)`,
          isDe
            ? "Glättet den gezeichneten Gebirgspfad für sanftere Konturen"
            : "Smooth the painted mountain ridge path for organic contours",
          () => {
            if (smoothMountainPath(currentMountain)) {
              notifyAndSync();
              refresh();
            }
          },
          "rb-btn rb-btn-secondary",
          "flex:1;font-size:11px;",
        ),
      );
      inspBody.appendChild(pathRow);
    }

    const collisions = findCollidingTracksForMountain(
      currentMountain,
      sidebar.layout.pieces || [],
      (id) => sidebar.catalog?.entries?.find((e) => e.id === id) ?? catalogRegistry.findEntry(id),
      sidebar.layout.layers,
    );

    inspBody.appendChild(
      el("div", {
        cls: "board-field-note",
        style:
          "font-size: 11px; padding: 6px 8px; border-radius: 4px; background: rgba(0,0,0,0.18); border-left: 3px solid " +
          (collisions.length > 0 ? "#10b981" : "#64748b") +
          ";",
        html:
          collisions.length > 0
            ? `<strong>● ${isDe ? "Tunnel aktiv" : "Tunnel active"}:</strong> ${collisions.length} ${
                isDe ? "Gleis-/Straßenabschnitte kreuzen den Berg." : "track/road segments intersect mountain."
              }`
            : `<strong>○ ${isDe ? "Vollmassiv" : "Solid mountain"}:</strong> ${
                isDe ? "Keine Gleiskreuzung." : "No track intersections."
              }`,
      }),
    );

    const actionsRow = el("div", { style: "display:flex;gap:6px;" });
    actionsRow.appendChild(
      btn(
        isDe ? "Auf Tisch zentrieren" : "Center on Table",
        isDe ? "Platziert den Berg in der Mitte der aktuellen Tischplatte" : "Place mountain in center of active table",
        () => {
          if (bb) {
            const bounds = getBaseboardBounds(bb);
            currentMountain.transform.x = Math.round(bounds.x + bounds.width / 2);
            currentMountain.transform.y = Math.round(bounds.y + bounds.depth / 2);
            notifyAndSync();
            refresh();
          }
        },
        "rb-btn rb-btn-secondary",
        "flex:1;",
      ),
    );
    actionsRow.appendChild(
      btn(
        isDe ? "✕ Berg löschen" : "✕ Delete Mountain",
        isDe ? "Entfernt diesen Berg von der Anlage" : "Remove this mountain from layout",
        () => {
          const idx = sidebar.layout.models?.findIndex((m) => m.id === currentMountain.id) ?? -1;
          if (idx !== -1 && sidebar.layout.models) {
            sidebar.layout.models.splice(idx, 1);
            sidebar.activeMountainId = undefined;
            notifyAndSync();
            refresh();
          }
        },
        "rb-btn rb-btn-danger",
        "flex:1;",
      ),
    );
    inspBody.appendChild(actionsRow);
    inspectorCard.appendChild(inspBody);
  } else {
    inspectorCard.appendChild(
      el("div", {
        cls: "board-field-note",
        style: "font-size:11px;opacity:0.85;line-height:1.4;",
        text: isDe
          ? "Noch kein Berg auf der Anlage. Wähle oben den Kamm-Pinsel oder klicke auf '+ Einzelberg'."
          : "No mountains yet. Select the Ridge Brush above or click '+ Single Peak'.",
      }),
    );
  }

  wrap.appendChild(inspectorCard);
}
