import type { Sidebar } from "@/ui/sidebar";
import type { BoardCtx } from "./board-tab";
import { el, btn, row, numberField, selectField } from "./board-tab";
import { createBuiltinModel } from "@/core/models-3d";
import { findCollidingTracksForMountain } from "@/render/builtin-models-3d";
import { catalogRegistry } from "@/catalog/catalog";
import { getBaseboardBounds } from "@/core/layout";
import type { Model3D } from "@/core/types";

/**
 * Erstellt die Sidebar-Karte zur Erstellung und Konfiguration von Bergen / Gebirgsmassiven
 * mit anpassbarer Position (X, Y), Abmessungen (Breite, Tiefe, Höhe), Drehung und
 * automatischer Erkennung von Tunnelausschnitten bei kreuzenden Gleisen/Straßen.
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
      text: isDe ? "⛰️ Gebirge & Berge" : "⛰️ Mountains & Terrain",
    }),
  );

  const body = el("div", { style: "display:flex;flex-direction:column;gap:8px;margin-top:6px;" });

  const mountains: Model3D[] = (sidebar.layout.models || []).filter((m) => m.builtinId === "mountain");

  // Oberste Zeile: Hinzufügen-Button
  const topActionRow = el("div", { cls: "board-btn-row" });
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

      if (sidebar.callbacks.onAddModel) {
        sidebar.callbacks.onAddModel(newMountain);
      } else {
        if (!sidebar.layout.models) sidebar.layout.models = [];
        sidebar.layout.models.push(newMountain);
        ctx.notifyBoard();
      }

      sidebar.activeMountainId = newMountain.id;
      sidebar.mountainsOpen = true;
      ctx.refresh();
    },
    "rb-btn rb-btn-secondary",
    "width: 100%;",
  );
  topActionRow.appendChild(addBtn);
  body.appendChild(topActionRow);

  if (mountains.length === 0) {
    const hint = el("div", {
      cls: "board-field-note",
      style: "font-size: 11px; opacity: 0.8; line-height: 1.4;",
      text: isDe
        ? "Noch kein Berg vorhanden. Klicke auf '+ Berg hinzufügen', um ein Gebirgsmassiv zu erstellen. Kreuzende Gleise und Straßen erzeugen automatisch Tunnelausschnitte und Steinportale."
        : "No mountains yet. Click '+ Add Mountain' to generate terrain. Intersecting tracks and roads automatically receive tunnel portals and cutouts.",
    });
    body.appendChild(hint);
    card.appendChild(body);
    return card;
  }

  // Aktiven Berg bestimmen
  if (!sidebar.activeMountainId || !mountains.some((m) => m.id === sidebar.activeMountainId)) {
    sidebar.activeMountainId = mountains[0]!.id;
  }
  const currentMountain = mountains.find((m) => m.id === sidebar.activeMountainId) ?? mountains[0]!;

  // Dropdown zur Auswahl, falls mehrere Berge existieren
  if (mountains.length > 1) {
    body.appendChild(
      selectField<string>(
        isDe ? "Ausgewählter Berg" : "Selected Mountain",
        mountains.map((m) => [m.id, m.name || (isDe ? "Berg" : "Mountain")]),
        currentMountain.id,
        (id) => {
          sidebar.activeMountainId = id;
          ctx.refresh();
        },
      ),
    );
  }

  // 1. Position X & Y
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

  // 2. Abmessungen: Breite (X) & Tiefe/Länge (Y)
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

  // 3. Höhe (Z) & Drehung
  body.appendChild(
    row([
      numberField(
        isDe ? "Höhe (Z)" : "Height (Z)",
        Math.round(currentMountain.height ?? 220),
        { min: 10, step: 10, unit: "mm" },
        (v) => {
          currentMountain.height = v;
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

  // 4. Kollisionsprüfung & Tunnel-Status
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
        ? `<strong>🚇 ${isDe ? "Tunnel aktiv" : "Tunnel active"}:</strong> ${collisions.length} ${
            isDe ? "Gleis-/Straßenabschnitte kreuzen den Berg." : "track/road segments intersect mountain."
          }<br><span style="opacity: 0.8;">${
            isDe ? "Steinportale und Tunnelausschnitte werden im 3D-Modus dargestellt." : "Stone portals and cutouts rendered in 3D."
          }</span>`
        : `<strong>⛰️ ${isDe ? "Keine Gleiskreuzung" : "Solid mountain"}:</strong> ${
            isDe
              ? "Vollmassiv. Verschiebe den Berg über Gleise, um Tunnels zu erzeugen."
              : "Move mountain over tracks or roads to create tunnels."
          }`,
  });
  body.appendChild(statusBox);

  // 5. Aktionen: Zentrieren & Löschen
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

  card.appendChild(body);
  return card;
}
