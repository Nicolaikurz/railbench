import type { Sidebar } from "@/ui/sidebar";
import { el, btn, row, numberField, selectField } from "./board-tab";
import { findCollidingTracksForMountain } from "@/render/builtin-models-3d";
import { catalogRegistry } from "@/catalog/catalog";
import type { GroundTerrainType, Model3D, MountainStyle } from "@/core/types";
import { getLanguage } from "@/i18n";
import {
  terrainBrush,
  applyAutoTunnelForMountain,
  type TerrainBrushMode,
} from "@/editor/terrain-brush-controller";

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
 * Aufgeräumtes, minimalistisches Gelände-Menü (Tab "Gelände" / "Terrain"):
 * - 1 einheitliches Pinsel-Werkzeug mit 3 Modi: Berg, Untergrund (Wasser/Sand/Erde/Hügel) und Radierer
 * - 1 kompakter Objekt-Inspektor für vorhandene Gelände-Elemente
 */
export function renderTerrainTab(sidebar: Sidebar, container: HTMLElement): void {
  const isDe = getLanguage() === "de";
  const wrap = container;

  const notifyAndSync = () => {
    sidebar.callbacks.onBaseboardChange?.();
  };
  const refresh = () => {
    sidebar.render();
  };

  /* ================================================================
   * 1. GELÄNDE-PINSEL (Berg / Untergrund / Radierer)
   * ================================================================ */
  const brushCard = el("div", { cls: "board-card" });
  brushCard.appendChild(
    el("div", {
      cls: "board-section-title",
      text: isDe ? "Gelände-Pinsel" : "Terrain Brush",
    }),
  );

  const toolRow = el("div", {
    style: "display:grid;grid-template-columns:1fr 1fr 1fr;gap:5px;margin-bottom:8px;",
  });

  const makeToolBtn = (mode: TerrainBrushMode, label: string, title: string) => {
    const active = terrainBrush.isActive && terrainBrush.mode === mode;
    return btn(
      active ? `${label} ✓` : label,
      title,
      () => {
        terrainBrush.toggle(mode);
        notifyAndSync();
        refresh();
      },
      active ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
      `font-size:11px;padding:6px 6px;${active ? "font-weight:700;" : ""}`,
    );
  };

  toolRow.appendChild(
    makeToolBtn(
      "paint",
      isDe ? "▲ Berg" : "▲ Mountain",
      isDe
        ? "Klicken für Einzelgipfel oder ziehen für Gebirgskamm"
        : "Click for a peak or drag to paint a mountain ridge",
    ),
  );
  toolRow.appendChild(
    makeToolBtn(
      "ground",
      isDe ? "◠ Grund" : "◠ Ground",
      isDe
        ? "Wasser, Sand, Erde oder Hügel per Pinsel malen (passt sich an Gebäude, Gleise & Straßen an)"
        : "Paint water, sand, dirt, or rolling hills (flattens under buildings, tracks & streets)",
    ),
  );
  toolRow.appendChild(
    makeToolBtn(
      "erase",
      isDe ? "✕ Löschen" : "✕ Erase",
      isDe ? "Berge oder Untergrund per Pinsel entfernen" : "Erase mountains or ground terrain by dragging",
    ),
  );
  brushCard.appendChild(toolRow);

  const activeMode: TerrainBrushMode = terrainBrush.mode;

  if (terrainBrush.isActive) {
    const hintText =
      activeMode === "ground"
        ? isDe
          ? "Im 2D-Plan ziehen oder klicken, um Untergrund zu malen. Rechtsklick beendet."
          : "Drag or click on the 2D plan to paint ground. Right-click to exit."
        : activeMode === "erase"
          ? isDe
            ? "Über Berge oder Untergrund ziehen zum Löschen. Rechtsklick beendet."
            : "Drag over mountains or ground to erase. Right-click to exit."
          : isDe
            ? "Klicken (Gipfel) oder Ziehen (Kamm) im 2D-Plan. Shift+Rad = Radius. Rechtsklick beendet."
            : "Click (peak) or drag (ridge) on the 2D plan. Shift+Wheel = radius. Right-click to exit.";
    brushCard.appendChild(
      el("div", {
        cls: "board-field-note",
        style:
          "font-size:10px;color:var(--text-main);padding:5px 7px;border-radius:4px;background:rgba(255,255,255,0.06);margin-bottom:8px;line-height:1.35;",
        text: hintText,
      }),
    );
  }

  // Kontextabhängige Pinsel-Einstellungen (nur das Nötigste für den gewählten Modus)
  if (activeMode === "ground") {
    brushCard.appendChild(
      selectField<GroundTerrainType>(
        isDe ? "Untergrund-Typ" : "Ground Type",
        [
          ["hill", isDe ? "Hügel (Gras)" : "Hill (Grass)"],
          ["water", isDe ? "Wasser" : "Water"],
          ["sand", "Sand"],
          ["dirt", isDe ? "Erde" : "Dirt"],
        ],
        terrainBrush.groundType,
        (t) => {
          terrainBrush.groundType = t;
          terrainBrush.groundHeight = t === "hill" ? 26 : t === "dirt" ? 14 : t === "sand" ? 12 : 6;
          notifyAndSync();
          refresh();
        },
      ),
    );
    brushCard.appendChild(
      row([
        numberField(
          isDe ? "Radius" : "Radius",
          Math.round(terrainBrush.brushRadius),
          { min: 60, max: 600, step: 20, unit: "mm" },
          (v) => {
            terrainBrush.brushRadius = Math.max(60, Math.min(600, v));
            notifyAndSync();
          },
        ),
        numberField(
          isDe ? "Höhe" : "Height",
          Math.round(terrainBrush.groundHeight),
          { min: 2, max: 120, step: 2, unit: "mm" },
          (v) => {
            terrainBrush.groundHeight = Math.max(2, Math.min(120, v));
            notifyAndSync();
          },
        ),
      ]),
    );
    brushCard.appendChild(
      el("div", {
        cls: "board-field-note",
        style: "font-size:10px;opacity:0.75;margin-top:4px;line-height:1.3;",
        text: isDe
          ? "Gebäude, Gleise & Straßen bleiben automatisch flach."
          : "Buildings, tracks & streets stay automatically flat.",
      }),
    );
  } else if (activeMode === "erase") {
    brushCard.appendChild(
      numberField(
        isDe ? "Radier-Radius" : "Eraser Radius",
        Math.round(terrainBrush.brushRadius),
        { min: 60, max: 600, step: 20, unit: "mm" },
        (v) => {
          terrainBrush.brushRadius = Math.max(60, Math.min(600, v));
          notifyAndSync();
        },
      ),
    );
  } else {
    // Berg-Pinsel ("paint" / "extend" / "stamp")
    brushCard.appendChild(
      row([
        numberField(
          isDe ? "Radius" : "Radius",
          Math.round(terrainBrush.brushRadius),
          { min: 60, max: 600, step: 20, unit: "mm" },
          (v) => {
            terrainBrush.brushRadius = Math.max(60, Math.min(600, v));
            notifyAndSync();
          },
        ),
        numberField(
          isDe ? "Höhe" : "Height",
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
      selectField<MountainStyle>(
        isDe ? "Oberfläche" : "Surface",
        [
          ["alpine", isDe ? "Alpen (Schnee & Fels)" : "Alpine (Snow & Rock)"],
          ["rocky", isDe ? "Felsmassiv" : "Rocky Massif"],
          ["forest", isDe ? "Waldgebirge" : "Forested Ridge"],
          ["meadow", isDe ? "Gras-Hügel" : "Meadow Hill"],
        ],
        terrainBrush.brushStyle,
        (style) => {
          terrainBrush.brushStyle = style;
          notifyAndSync();
          refresh();
        },
      ),
    );
    brushCard.appendChild(
      checkboxField(
        isDe ? "Gleise & Straßen automatisch tunneln" : "Auto-tunnel intersecting tracks & streets",
        terrainBrush.autoTunnel,
        (checked) => {
          terrainBrush.autoTunnel = checked;
        },
      ),
    );
  }

  wrap.appendChild(brushCard);

  /* ================================================================
   * 2. GELÄNDE-OBJEKTE (Kompakter Inspektor für Berge & Untergrund)
   * ================================================================ */
  const terrainItems: Model3D[] = (sidebar.layout.models || []).filter(
    (m) => m.builtinId === "mountain" || m.builtinId === "ground-terrain",
  );

  if (terrainItems.length === 0) return;

  const inspectorCard = el("div", { cls: "board-card" });
  inspectorCard.appendChild(
    el("div", {
      cls: "board-section-title",
      text: isDe ? `Gelände-Objekte (${terrainItems.length})` : `Terrain Items (${terrainItems.length})`,
    }),
  );

  let currentItem = terrainItems.find((m) => m.id === sidebar.activeMountainId) ?? terrainItems[terrainItems.length - 1]!;
  sidebar.activeMountainId = currentItem.id;

  if (terrainItems.length > 1) {
    inspectorCard.appendChild(
      selectField<string>(
        isDe ? "Auswählen" : "Select",
        terrainItems.map((item) => [
          item.id,
          `${item.builtinId === "ground-terrain" ? "◠" : "▲"} ${item.name}`,
        ]),
        currentItem.id,
        (id) => {
          sidebar.activeMountainId = id;
          sidebar.callbacks.onSelectModel?.(id);
          refresh();
        },
      ),
    );
  }

  if (currentItem.builtinId === "ground-terrain") {
    const typeLabels: Record<GroundTerrainType, string> = isDe
      ? { water: "Wasser", sand: "Sand", dirt: "Erde", hill: "Hügel" }
      : { water: "Water", sand: "Sand", dirt: "Dirt", hill: "Hill" };

    inspectorCard.appendChild(
      selectField<GroundTerrainType>(
        isDe ? "Typ" : "Type",
        [
          ["hill", typeLabels.hill],
          ["water", typeLabels.water],
          ["sand", typeLabels.sand],
          ["dirt", typeLabels.dirt],
        ],
        currentItem.groundTerrainType ?? "hill",
        (t) => {
          currentItem.groundTerrainType = t;
          notifyAndSync();
          refresh();
        },
      ),
    );
    inspectorCard.appendChild(
      row([
        numberField(
          isDe ? "Breite" : "Width",
          Math.round(currentItem.width),
          { min: 100, max: 6000, step: 50, unit: "mm" },
          (v) => {
            currentItem.width = Math.max(100, v);
            notifyAndSync();
          },
        ),
        numberField(
          isDe ? "Tiefe" : "Depth",
          Math.round(currentItem.depth),
          { min: 100, max: 6000, step: 50, unit: "mm" },
          (v) => {
            currentItem.depth = Math.max(100, v);
            notifyAndSync();
          },
        ),
      ]),
    );
    inspectorCard.appendChild(
      numberField(
        isDe ? "Höhe" : "Height",
        Math.round(currentItem.height),
        { min: 2, max: 120, step: 2, unit: "mm" },
        (v) => {
          currentItem.height = Math.max(2, v);
          notifyAndSync();
        },
      ),
    );
  } else {
    inspectorCard.appendChild(
      selectField<MountainStyle>(
        isDe ? "Oberfläche" : "Surface",
        [
          ["alpine", isDe ? "Alpen (Schnee & Fels)" : "Alpine (Snow & Rock)"],
          ["rocky", isDe ? "Felsmassiv" : "Rocky Massif"],
          ["forest", isDe ? "Waldgebirge" : "Forested Ridge"],
          ["meadow", isDe ? "Gras-Hügel" : "Meadow Hill"],
        ],
        currentItem.mountainStyle ?? "alpine",
        (style) => {
          currentItem.mountainStyle = style;
          notifyAndSync();
          refresh();
        },
      ),
    );
    inspectorCard.appendChild(
      row([
        numberField(
          isDe ? "Breite" : "Width",
          Math.round(currentItem.width),
          { min: 120, max: 4000, step: 50, unit: "mm" },
          (v) => {
            currentItem.width = Math.max(120, v);
            if (terrainBrush.autoTunnel) applyAutoTunnelForMountain(currentItem, sidebar.layout);
            notifyAndSync();
          },
        ),
        numberField(
          isDe ? "Tiefe" : "Depth",
          Math.round(currentItem.depth),
          { min: 120, max: 4000, step: 50, unit: "mm" },
          (v) => {
            currentItem.depth = Math.max(120, v);
            if (terrainBrush.autoTunnel) applyAutoTunnelForMountain(currentItem, sidebar.layout);
            notifyAndSync();
          },
        ),
      ]),
    );
    inspectorCard.appendChild(
      numberField(
        isDe ? "Höhe" : "Height",
        Math.round(currentItem.height),
        { min: 40, max: 600, step: 20, unit: "mm" },
        (v) => {
          currentItem.height = Math.max(40, v);
          notifyAndSync();
        },
      ),
    );

    const collisions = findCollidingTracksForMountain(
      currentItem,
      sidebar.layout.pieces || [],
      (id) => sidebar.catalog?.entries?.find((e) => e.id === id) ?? catalogRegistry.findEntry(id),
      sidebar.layout.layers,
    );
    if (collisions.length > 0) {
      inspectorCard.appendChild(
        el("div", {
          cls: "board-field-note",
          style: "font-size:10.5px;margin-top:4px;color:#10b981;",
          text: isDe
            ? `● Tunnel aktiv (${collisions.length} Abschnitte)`
            : `● Tunnel active (${collisions.length} segments)`,
        }),
      );
    }
  }

  inspectorCard.appendChild(
    btn(
      isDe ? "✕ Löschen" : "✕ Delete",
      isDe ? "Ausgewähltes Gelände-Objekt entfernen" : "Delete selected terrain item",
      () => {
        const idx = sidebar.layout.models?.findIndex((m) => m.id === currentItem.id) ?? -1;
        if (idx !== -1 && sidebar.layout.models) {
          sidebar.layout.models.splice(idx, 1);
          sidebar.activeMountainId = undefined;
          notifyAndSync();
          refresh();
        }
      },
      "rb-btn rb-btn-danger",
      "width:100%;margin-top:8px;font-size:11px;",
    ),
  );

  wrap.appendChild(inspectorCard);
}
