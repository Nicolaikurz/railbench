import type { Sidebar } from "@/ui/sidebar";
import type { BoardCtx } from "./board-tab";
import { el, btn, row, numberField, selectField } from "./board-tab";
import { createBuiltinModel } from "@/core/models-3d";
import { findCollidingTracksForMountain } from "@/render/builtin-models-3d";
import { catalogRegistry } from "@/catalog/catalog";
import { getBaseboardBounds } from "@/core/layout";
import type { Model3D, MountainStyle } from "@/core/types";
import {
  terrainBrush,
  applyAutoTunnelAllMountains,
  applyAutoTunnelForMountain,
} from "@/editor/terrain-brush-controller";

/**
 * Erstellt die komplett überarbeitete Sidebar-Karte für Gebirge, Berge,
 * interaktiven Berg-Pinsel/Stift und Gleis-/Straßen-Tunnelsteuerung.
 */
export function buildMountainCard(sidebar: Sidebar, ctx: BoardCtx): HTMLElement {
  const { isDe, bb } = ctx;

  const card = el("details", { cls: "board-card" });
  card.open = sidebar.mountainsOpen;
  card.addEventListener("toggle", () => {
    sidebar.mountainsOpen = card.open;
  });

  card.appendChild(
    el("summary", {
      cls: "board-section-title",
      style: "cursor:pointer;",
      text: isDe ? "▲ Gebirge, Pinsel & Tunnel" : "▲ Mountains, Brush & Tunnels",
    }),
  );

  const body = el("div", { style: "display:flex;flex-direction:column;gap:8px;margin-top:6px;" });

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
      ctx.notifyBoard();
    }
    sidebar.activeMountainId = newMountain.id;
    sidebar.mountainsOpen = true;
    sidebar.callbacks.onSelectModel?.(newMountain.id);
    ctx.refresh();
  };

  // 1. Oberste Zeile: Standard-Berg hinzufügen (erster Button für Test-Kompatibilität)
  const topActionRow = el("div", { cls: "board-btn-row", style: "display:flex;gap:5px;flex-wrap:wrap;" });
  const addBtn = btn(
    isDe ? "+ Berg hinzufügen" : "+ Add Mountain",
    isDe ? "Neues Gebirgsmassiv auf die Platte setzen" : "Place a new mountain on the board",
    () => {
      const count = mountains.length;
      const bounds = bb ? getBaseboardBounds(bb) : { x: 0, y: 0, width: 1000, depth: 800 };
      const centerX = Math.round(bounds.x + bounds.width / 2);
      const centerY = Math.round(bounds.y + bounds.depth / 2);
      const offset = (count % 5) * 60;

      const newMountain = createBuiltinModel(
        "mountain",
        { x: centerX + offset, y: centerY + offset, rotation: 0 },
        sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
      );
      newMountain.name = isDe ? `Berg ${count + 1}` : `Mountain ${count + 1}`;
      newMountain.width = 600;
      newMountain.depth = 450;
      newMountain.height = 220;
      newMountain.mountainStyle = terrainBrush.brushStyle;

      insertMountain(newMountain);
    },
    "rb-btn rb-btn-secondary",
    "flex: 1 1 100%;",
  );
  topActionRow.appendChild(addBtn);

  // Schnell-Vorlagen: Eck-Gebirge (L-Form) & Kulissen-Kette
  const addCornerBtn = btn(
    isDe ? "⌐ Eck-Massiv" : "⌐ Corner Massif",
    isDe
      ? "Erstellt ein L-förmiges Eckgebirge in der linken oberen Anlagen-Ecke"
      : "Create an L-shaped mountain massif in the top-left corner",
    () => {
      const count = mountains.length;
      const bounds = bb ? getBaseboardBounds(bb) : { x: -1000, y: -600, width: 2000, depth: 1200 };
      const cx = Math.round(bounds.x + Math.min(420, bounds.width * 0.25));
      const cy = Math.round(bounds.y + Math.min(340, bounds.depth * 0.28));
      const newMountain = createBuiltinModel(
        "mountain",
        { x: cx, y: cy, rotation: 0 },
        sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
      );
      newMountain.name = isDe ? `Eck-Gebirge ${count + 1}` : `Corner Massif ${count + 1}`;
      newMountain.width = 960;
      newMountain.depth = 780;
      newMountain.height = 240;
      newMountain.mountainStyle = terrainBrush.brushStyle;
      newMountain.mountainPath = [
        { x: -200, y: 180, radius: 220, height: 210 },
        { x: -200, y: -140, radius: 250, height: 240 },
        { x: 220, y: -160, radius: 220, height: 220 },
      ];
      insertMountain(newMountain);
    },
    "rb-btn rb-btn-secondary",
    "flex: 1; font-size: 11px;",
  );

  const addRidgeBtn = btn(
    isDe ? "▲ Kulissen-Kette" : "▲ Backdrop Ridge",
    isDe
      ? "Erstellt eine langgestreckte Gebirgskette entlang der hinteren Anlagenkante"
      : "Create a long mountain ridge along the back edge of the table",
    () => {
      const count = mountains.length;
      const bounds = bb ? getBaseboardBounds(bb) : { x: -1000, y: -600, width: 2000, depth: 1200 };
      const cx = Math.round(bounds.x + bounds.width / 2);
      const cy = Math.round(bounds.y + Math.min(200, bounds.depth * 0.18));
      const w = Math.round(Math.max(800, bounds.width * 0.72));
      const halfSpan = Math.round(w * 0.34);

      const newMountain = createBuiltinModel(
        "mountain",
        { x: cx, y: cy, rotation: 0 },
        sidebar.layout.activeLayerId || sidebar.layout.layers?.[0]?.id,
      );
      newMountain.name = isDe ? `Gebirgskette ${count + 1}` : `Backdrop Ridge ${count + 1}`;
      newMountain.width = w;
      newMountain.depth = 420;
      newMountain.height = 250;
      newMountain.mountainStyle = terrainBrush.brushStyle;
      newMountain.mountainPath = [
        { x: -halfSpan, y: 15, radius: 190, height: 210 },
        { x: 0, y: -15, radius: 210, height: 250 },
        { x: halfSpan, y: 10, radius: 190, height: 220 },
      ];
      insertMountain(newMountain);
    },
    "rb-btn rb-btn-secondary",
    "flex: 1; font-size: 11px;",
  );

  topActionRow.appendChild(addCornerBtn);
  topActionRow.appendChild(addRidgeBtn);
  body.appendChild(topActionRow);

  // 2. Interaktives Berg-Pinsel / Stift Werkzeug (2D-Plan)
  const brushSection = el("div", {
    style:
      "display:flex;flex-direction:column;gap:6px;padding:7px 8px;border-radius:6px;background:rgba(15,23,42,0.35);border:1px solid var(--border-main);",
  });
  brushSection.appendChild(
    el("div", {
      style: "font-size:11px;font-weight:700;color:var(--text-main);display:flex;justify-content:space-between;align-items:center;",
      text: isDe ? "Berg-Pinsel / Stift (2D-Plan)" : "Terrain Brush / Pencil (2D Plan)",
    }),
  );

  const isPaintActive = terrainBrush.isActive && terrainBrush.mode === "paint";
  const isExtendActive = terrainBrush.isActive && terrainBrush.mode === "extend";
  const isStampActive = terrainBrush.isActive && terrainBrush.mode === "stamp";
  const isEraseActive = terrainBrush.isActive && terrainBrush.mode === "erase";

  const brushBtnRow = el("div", { cls: "board-btn-row", style: "display:flex;gap:5px;flex-wrap:wrap;" });
  const paintBtn = btn(
    isPaintActive
      ? (isDe ? "✎ Zeichnen ✓" : "✎ Paint ✓")
      : (isDe ? "✎ Berg zeichnen" : "✎ Draw Mountain"),
    isDe
      ? "Mit der Maus im 2D-Plan Berge und Gebirgszüge aufmalen"
      : "Paint mountains and ridges directly on the 2D canvas",
    () => {
      terrainBrush.toggle("paint");
      ctx.notifyBoard();
      ctx.refresh();
    },
    isPaintActive ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
    "flex: 1; font-size: 11px;",
  );

  const extendBtn = btn(
    isExtendActive
      ? (isDe ? "+ Erweitern ✓" : "+ Extend ✓")
      : (isDe ? "+ Erweitern" : "+ Extend"),
    isDe ? "Bestehenden Berg nahtlos mit dem Pinsel erweitern" : "Extend existing mountain massif with brush",
    () => {
      terrainBrush.toggle("extend", sidebar.activeMountainId);
      ctx.notifyBoard();
      ctx.refresh();
    },
    isExtendActive ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
    "flex: 0 0 auto; font-size: 11px;",
  );

  const stampBtn = btn(
    isStampActive
      ? (isDe ? "▲ Stempel ✓" : "▲ Stamp ✓")
      : (isDe ? "▲ Stempel" : "▲ Stamp"),
    isDe ? "Einzelklick setzt einen Berggipfel" : "Single click stamps a mountain peak",
    () => {
      terrainBrush.toggle("stamp");
      ctx.notifyBoard();
      ctx.refresh();
    },
    isStampActive ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
    "flex: 0 0 auto; font-size: 11px;",
  );

  const eraseBtn = btn(
    isEraseActive
      ? (isDe ? "✕ Radierer ✓" : "✕ Eraser ✓")
      : (isDe ? "✕ Radierer" : "✕ Eraser"),
    isDe ? "Berge oder Pinsel-Knoten im 2D-Plan durch Überfahren entfernen" : "Click or drag on mountains in 2D to erase nodes or mountains",
    () => {
      terrainBrush.toggle("erase");
      ctx.notifyBoard();
      ctx.refresh();
    },
    isEraseActive ? "rb-btn rb-btn-danger" : "rb-btn rb-btn-secondary",
    "flex: 0 0 auto; font-size: 11px;",
  );

  brushBtnRow.appendChild(paintBtn);
  brushBtnRow.appendChild(extendBtn);
  brushBtnRow.appendChild(stampBtn);
  brushBtnRow.appendChild(eraseBtn);
  brushSection.appendChild(brushBtnRow);

  if (terrainBrush.isActive) {
    brushSection.appendChild(
      el("div", {
        cls: "board-field-note",
        style: "font-size:10.5px;color:var(--text-main);line-height:1.35;",
        text: isPaintActive
          ? (isDe
              ? "Im 2D-Plan klicken oder ziehen, um ein Gebirgsmassiv zu zeichnen. Rechtsklick beendet das Werkzeug."
              : "Click or drag on the 2D plan to paint a mountain ridge. Right-click to exit tool.")
          : (isDe
              ? "Im 2D-Plan auf Berge klicken/ziehen, um sie zu löschen. Rechtsklick beendet."
              : "Click or drag over mountains on the 2D plan to erase them. Right-click to exit."),
      }),
    );
  }

  body.appendChild(brushSection);

  // 3. Separates Tunnel-Werkzeug für Gleise & Straßen
  const tunnelSection = el("div", {
    style:
      "display:flex;flex-direction:column;gap:6px;padding:7px 8px;border-radius:6px;background:rgba(15,23,42,0.35);border:1px solid var(--border-main);",
  });
  const tunneledPiecesCount = (sidebar.layout.pieces || []).filter((p) => !!p.hasTunnel).length;
  const selectedIds = sidebar.callbacks.selectedPieceIds?.() ?? [];
  const selectedTrackPieces = (sidebar.layout.pieces || []).filter((p) => selectedIds.includes(p.id));
  const allSelectedHaveTunnel =
    selectedTrackPieces.length > 0 && selectedTrackPieces.every((p) => !!p.hasTunnel);

  tunnelSection.appendChild(
    el("div", {
      style: "font-size:11px;font-weight:700;color:var(--text-main);",
      text: isDe
        ? `Gleis- & Straßen-Tunnel (${tunneledPiecesCount} aktiv)`
        : `Track & Street Tunnels (${tunneledPiecesCount} active)`,
    }),
  );

  const tunnelBtnRow = el("div", { cls: "board-btn-row", style: "display:flex;gap:5px;flex-wrap:wrap;" });
  const toggleSelTunnelBtn = btn(
    selectedTrackPieces.length > 0
      ? allSelectedHaveTunnel
        ? (isDe ? `Tunnel entfernen (${selectedTrackPieces.length})` : `Remove Tunnel (${selectedTrackPieces.length})`)
        : (isDe ? `Als Tunnel setzen (${selectedTrackPieces.length})` : `Set as Tunnel (${selectedTrackPieces.length})`)
      : (isDe ? "Auswahl als Tunnel (T)" : "Toggle Tunnel on Selection (T)"),
    isDe
      ? "Markierte Gleise oder Straßen als Tunnel ein-/ausschalten (Tastenkürzel: T)"
      : "Toggle tunnel on selected tracks or streets (Shortcut: T)",
    () => {
      if (selectedTrackPieces.length === 0) return;
      const shouldEnable = !allSelectedHaveTunnel;
      for (const p of selectedTrackPieces) {
        p.hasTunnel = shouldEnable;
        if (shouldEnable) p.hasBridge = false;
      }
      ctx.notifyBoard();
      ctx.refresh();
    },
    allSelectedHaveTunnel ? "rb-btn rb-btn-primary" : "rb-btn rb-btn-secondary",
    "flex: 1 1 100%; font-size: 11px;",
  );
  tunnelBtnRow.appendChild(toggleSelTunnelBtn);

  const autoTunnelBtn = btn(
    isDe ? "Auto-Tunnel (Berge)" : "Auto-Tunnel (Mountains)",
    isDe
      ? "Markiert automatisch alle Gleise und Straßen, die durch einen Berg führen, als Tunnel"
      : "Automatically mark all tracks and streets passing through any mountain as tunnels",
    () => {
      applyAutoTunnelAllMountains(sidebar.layout);
      ctx.notifyBoard();
      ctx.refresh();
    },
    "rb-btn rb-btn-secondary",
    "flex: 1; font-size: 11px;",
  );
  tunnelBtnRow.appendChild(autoTunnelBtn);

  if (tunneledPiecesCount > 0) {
    const clearTunnelsBtn = btn(
      isDe ? "✕ Alle lösen" : "✕ Clear All",
      isDe ? "Entfernt die manuelle Tunnel-Markierung von allen Gleisen und Straßen" : "Clear explicit tunnel flag from all tracks and streets",
      () => {
        for (const p of sidebar.layout.pieces || []) {
          p.hasTunnel = false;
        }
        ctx.notifyBoard();
        ctx.refresh();
      },
      "rb-btn rb-btn-secondary",
      "flex: 0 0 auto; font-size: 11px;",
    );
    tunnelBtnRow.appendChild(clearTunnelsBtn);
  }

  tunnelSection.appendChild(tunnelBtnRow);
  body.appendChild(tunnelSection);

  // 4. Ausgewählter Berg – Inspektor (WICHTIG: Erste 6 <input>-Felder bleiben X, Y, Breite, Tiefe, Höhe, Drehung)
  if (mountains.length > 0) {
    if (!sidebar.activeMountainId || !mountains.some((m) => m.id === sidebar.activeMountainId)) {
      sidebar.activeMountainId = mountains[0]!.id;
    }
    const currentMountain = mountains.find((m) => m.id === sidebar.activeMountainId) ?? mountains[0]!;

    if (mountains.length > 1) {
      body.appendChild(
        selectField<string>(
          isDe ? "Ausgewählter Berg" : "Selected Mountain",
          mountains.map((m) => [m.id, m.name || (isDe ? "Berg" : "Mountain")]),
          currentMountain.id,
          (id) => {
            sidebar.activeMountainId = id;
            sidebar.callbacks.onSelectModel?.(id);
            ctx.refresh();
          },
        ),
      );
    }

    body.appendChild(
      selectField<MountainStyle>(
        isDe ? "Biom / Gelände-Stil" : "Biome / Terrain Style",
        [
          ["alpine", isDe ? "Alpen (Schneegipfel & Fels)" : "Alpine (Snow & Rock)"],
          ["rocky", isDe ? "Felsmassiv (Granit)" : "Rocky Massif (Granite)"],
          ["forest", isDe ? "Mittelgebirge (Wald)" : "Forested Ridge"],
          ["meadow", isDe ? "Gras-Hügel / Voralpen" : "Green Meadow Hill"],
        ],
        currentMountain.mountainStyle ?? "alpine",
        (style) => {
          currentMountain.mountainStyle = style;
          terrainBrush.brushStyle = style;
          ctx.notifyBoard();
          ctx.refresh();
        },
      ),
    );

    // 4.1 Position X & Y (inputs[0], inputs[1])
    body.appendChild(
      row([
        numberField("Position X", Math.round(currentMountain.transform.x), { step: 10, unit: "mm" }, (v) => {
          currentMountain.transform.x = v;
          ctx.notifyBoard();
        }),
        numberField("Position Y", Math.round(currentMountain.transform.y), { step: 10, unit: "mm" }, (v) => {
          currentMountain.transform.y = v;
          ctx.notifyBoard();
        }),
      ]),
    );

    // 4.2 Abmessungen: Breite (X) & Tiefe/Länge (Y) (inputs[2], inputs[3])
    body.appendChild(
      row([
        numberField(
          isDe ? "Breite (X)" : "Width (X)",
          Math.round(currentMountain.width ?? 600),
          { min: 50, step: 20, unit: "mm" },
          (v) => {
            currentMountain.width = v;
            ctx.notifyBoard();
          },
        ),
        numberField(
          isDe ? "Länge / Tiefe (Y)" : "Length / Depth (Y)",
          Math.round(currentMountain.depth ?? 450),
          { min: 50, step: 20, unit: "mm" },
          (v) => {
            currentMountain.depth = v;
            ctx.notifyBoard();
          },
        ),
      ]),
    );

    // 4.3 Höhe (Z) & Drehung (inputs[4], inputs[5])
    body.appendChild(
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
            ctx.notifyBoard();
          },
        ),
        numberField(
          isDe ? "Drehung" : "Rotation",
          Math.round(currentMountain.transform.rotation ?? 0),
          { step: 5, unit: "°" },
          (v) => {
            currentMountain.transform.rotation = ((v % 360) + 360) % 360;
            ctx.notifyBoard();
          },
        ),
      ]),
    );

    // 4.4 Kollisionsprüfung & Tunnel-Status
    const collisions = findCollidingTracksForMountain(
      currentMountain,
      sidebar.layout.pieces || [],
      (id) => sidebar.catalog?.entries?.find((e) => e.id === id) ?? catalogRegistry.findEntry(id),
      sidebar.layout.layers,
    );

    const statusBox = el("div", {
      cls: "board-field-note",
      style:
        "font-size: 11px; padding: 6px 8px; border-radius: 4px; background: rgba(0,0,0,0.18); border-left: 3px solid " +
        (collisions.length > 0 ? "#10b981" : "#64748b") +
        ";",
      html:
        collisions.length > 0
          ? `<strong>● ${isDe ? "Tunnel aktiv" : "Tunnel active"}:</strong> ${collisions.length} ${
              isDe ? "Gleis-/Straßenabschnitte kreuzen den Berg." : "track/road segments intersect mountain."
            }<br><span style="opacity: 0.8;">${
              isDe ? "Steinportale und Tunnelausschnitte werden im 3D-Modus dargestellt." : "Stone portals and cutouts rendered in 3D."
            }</span>`
          : `<strong>○ ${isDe ? "Keine Gleiskreuzung" : "Solid mountain"}:</strong> ${
              isDe
                ? "Vollmassiv. Verschiebe den Berg über Gleise, um Tunnels zu erzeugen."
                : "Move mountain over tracks or roads to create tunnels."
            }`,
    });
    body.appendChild(statusBox);

    // 4.5 Aktionen: Zentrieren & Löschen
    const actionsRow = el("div", { cls: "board-btn-row", style: "display:flex;gap:6px;" });

    const centerBtn = btn(
      isDe ? "Auf Tisch zentrieren" : "Center on Table",
      isDe ? "Platziert den Berg in der Mitte der aktuellen Tischplatte" : "Place mountain in center of active table",
      () => {
        if (bb) {
          const bounds = getBaseboardBounds(bb);
          currentMountain.transform.x = Math.round(bounds.x + bounds.width / 2);
          currentMountain.transform.y = Math.round(bounds.y + bounds.depth / 2);
          ctx.notifyBoard();
          ctx.refresh();
        }
      },
      "rb-btn rb-btn-secondary",
      "flex: 1;",
    );

    const deleteBtn = btn(
      isDe ? "Berg löschen" : "Delete Mountain",
      isDe ? "Entfernt diesen Berg von der Anlage" : "Remove this mountain from layout",
      () => {
        const idx = sidebar.layout.models?.findIndex((m) => m.id === currentMountain.id) ?? -1;
        if (idx !== -1 && sidebar.layout.models) {
          sidebar.layout.models.splice(idx, 1);
          sidebar.activeMountainId = undefined;
          ctx.notifyBoard();
          ctx.refresh();
        }
      },
      "rb-btn rb-btn-danger",
      "flex: 1;",
    );

    actionsRow.appendChild(centerBtn);
    actionsRow.appendChild(deleteBtn);
    body.appendChild(actionsRow);
  } else {
    const hint = el("div", {
      cls: "board-field-note",
      style: "font-size: 11px; opacity: 0.85; line-height: 1.4;",
      text: isDe
        ? "Noch kein Berg vorhanden. Klicke auf '+ Berg hinzufügen' oder nutze den Berg-Pinsel, um direkt auf dem 2D-Plan ein Gebirgsmassiv zu zeichnen."
        : "No mountains yet. Click '+ Add Mountain' or use the Terrain Brush to paint a mountain ridge directly on the 2D plan.",
    });
    body.appendChild(hint);
  }

  // 5. Pinsel-Parameter (am Ende platziert, damit inputs[0..5] beim Berg-Inspektor unverändert bleiben)
  body.appendChild(
    row([
      numberField(
        isDe ? "Pinsel-Radius" : "Brush Radius",
        Math.round(terrainBrush.brushRadius),
        { min: 60, max: 600, step: 20, unit: "mm" },
        (v) => {
          terrainBrush.brushRadius = Math.max(60, Math.min(600, v));
          ctx.notifyBoard();
        },
      ),
      numberField(
        isDe ? "Pinsel-Höhe" : "Brush Height",
        Math.round(terrainBrush.brushHeight),
        { min: 40, max: 500, step: 20, unit: "mm" },
        (v) => {
          terrainBrush.brushHeight = Math.max(40, Math.min(500, v));
          ctx.notifyBoard();
        },
      ),
    ]),
  );

  card.appendChild(body);
  return card;
}
