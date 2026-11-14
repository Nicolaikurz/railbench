import { boardSketcher, type SketchToolMode } from "@/editor/board-sketcher-controller";
import type { Sidebar } from "../sidebar";
import { btn, el, syncWoodMode, type BoardCtx } from "./board-tab";

/**
 * Renders the active CAD Sketcher control panel at the top of the Table tab
 * whenever the user is sketching or editing a table or layer plate on the 2D canvas.
 */
export function buildSketcherBanner(sidebar: Sidebar, ctx: BoardCtx): HTMLElement | null {
  const { isDe } = ctx;

  // Bind state change callback so finishing a sketch on the canvas updates the sidebar & 3D/2D views
  boardSketcher.onStateChange = (committed, createdBoardId) => {
    if (createdBoardId) {
      sidebar.activeBoardId = createdBoardId;
    }
    if (committed) {
      for (const l of sidebar.layout.layers || []) syncWoodMode(l);
      ctx.notifyLayer();
    } else {
      ctx.notifyBoard();
    }
    ctx.refresh();
  };

  if (!boardSketcher.isActive || !boardSketcher.target) return null;

  const target = boardSketcher.target;
  let targetLabel = "";
  if (target.kind === "table") {
    if (target.createNew) {
      targetLabel = isDe ? "Neuer Tisch" : "New Table";
    } else {
      const b = ctx.baseboards.find((x) => x.id === target.boardId) ?? ctx.bb;
      targetLabel = b?.name ?? (isDe ? "Tisch" : "Table");
    }
  } else {
    const layer = sidebar.layout.layers?.find((l) => l.id === target.layerId);
    const plate = layer?.plates?.find((p) => p.id === target.plateId);
    if (target.createNew || !plate) {
      targetLabel = `${isDe ? "Neue Platte" : "New Plate"} (${layer?.name ?? ""})`;
    } else {
      targetLabel = `${plate.name || (isDe ? "Platte" : "Plate")} (${layer?.name ?? ""})`;
    }
  }

  const card = el("div", {
    cls: "board-card",
    style:
      "display:flex;flex-direction:column;gap:6px;border:1.5px solid var(--theme-accent, #38bdf8);background:rgba(14,165,233,0.08);",
  });

  // Header + Exit button
  card.appendChild(
    el("div", { style: "display:flex;justify-content:space-between;align-items:center;gap:6px;" }, [
      el("div", {
        cls: "board-section-title",
        style: "margin:0;color:var(--theme-accent, #38bdf8);",
        text: `✎ Sketcher: ${targetLabel}`,
      }),
      btn(
        `✕ ${isDe ? "Fertig" : "Done"}`,
        isDe ? "Sketcher beenden (Esc)" : "Exit Sketcher (Esc)",
        () => {
          boardSketcher.stop();
        },
        "rb-btn rb-btn-secondary",
        "font-size:10px;padding:2px 7px;",
      ),
    ]),
  );

  // Mode buttons: Polygon | Rectangle | Edit Vertices
  const canEdit =
    (target.kind === "table" && !target.createNew) ||
    (target.kind === "plate" && !target.createNew && Boolean(target.plateId));

  const modeBtn = (m: SketchToolMode, label: string, title: string, disabled = false) => {
    const active = boardSketcher.mode === m;
    const b = btn(
      label,
      title,
      () => boardSketcher.setMode(sidebar.layout, m),
      `rb-btn ${active ? "rb-btn-primary" : "rb-btn-secondary"}`,
      "flex:1;font-size:11px;padding:4px 6px;",
    );
    b.disabled = disabled;
    return b;
  };

  card.appendChild(
    el("div", { style: "display:flex;gap:4px;" }, [
      modeBtn("poly", `✎ ${isDe ? "Polygon" : "Polygon"}`, isDe ? "Eckpunkte nacheinander klicken" : "Click polygon corners"),
      modeBtn("rect", `▭ ${isDe ? "Rechteck" : "Rect"}`, isDe ? "Rechteck aufziehen" : "Draw rectangle"),
      modeBtn(
        "edit",
        `⊹ ${isDe ? "Punkte" : "Vertices"}`,
        isDe ? "Eckpunkte und Kanten verschieben" : "Drag vertices and edges",
        !canEdit,
      ),
    ]),
  );

  // Grid snap & Ortho toggle row
  const snapSteps = [50, 20, 10, 0];
  const snapRow = el("div", { style: "display:flex;align-items:center;gap:4px;flex-wrap:wrap;" }, [
    el("span", {
      style: "font-size:10px;color:var(--text-muted);margin-right:2px;",
      text: isDe ? "Raster:" : "Grid:",
    }),
  ]);
  for (const step of snapSteps) {
    const active = boardSketcher.snapStepMm === step;
    snapRow.appendChild(
      btn(
        step === 0 ? (isDe ? "Frei" : "Free") : `${step}`,
        isDe ? `Rasterfang ${step} mm` : `Grid snap ${step} mm`,
        () => {
          boardSketcher.snapStepMm = step;
          ctx.refresh();
        },
        `rb-btn ${active ? "rb-btn-primary" : "rb-btn-secondary"}`,
        "font-size:10px;padding:2px 6px;",
      ),
    );
  }
  snapRow.appendChild(
    btn(
      `⊥ ${isDe ? "Ortho" : "Ortho"}`,
      isDe ? "Rechtwinklige Hilfslinien ein/aus" : "Toggle orthogonal alignment snap",
      () => {
        boardSketcher.orthoSnap = !boardSketcher.orthoSnap;
        ctx.refresh();
      },
      `rb-btn ${boardSketcher.orthoSnap ? "rb-btn-primary" : "rb-btn-secondary"}`,
      "font-size:10px;padding:2px 6px;margin-left:auto;",
    ),
  );
  card.appendChild(snapRow);

  // Polygon action buttons (Undo last point / Close & apply polygon)
  if (boardSketcher.mode === "poly") {
    const ptCount = boardSketcher.points.length;
    const polyRow = el("div", { style: "display:flex;align-items:center;gap:4px;" }, [
      el("span", {
        style: "font-size:10px;color:var(--text-muted);flex:1;",
        text: isDe ? `Gesetzte Ecken: ${ptCount}` : `Placed corners: ${ptCount}`,
      }),
      ptCount > 0
        ? btn(
            `↶ ${isDe ? "Zurück" : "Undo"}`,
            isDe ? "Letzten Eckpunkt entfernen (Backspace)" : "Remove last vertex (Backspace)",
            () => boardSketcher.undoLastPoint(),
            "rb-btn rb-btn-secondary",
            "font-size:10px;padding:3px 7px;",
          )
        : null,
      ptCount >= 3
        ? btn(
            `✓ ${isDe ? "Schließen" : "Finish"}`,
            isDe ? "Polygon schließen und übernehmen (Enter)" : "Close and apply polygon (Enter)",
            () => boardSketcher.finishPoly(sidebar.layout),
            "rb-btn rb-btn-primary",
            "font-size:10px;padding:3px 8px;",
          )
        : null,
    ]);
    card.appendChild(polyRow);
  }

  // Help hint
  const hintText =
    boardSketcher.mode === "poly"
      ? isDe
        ? "Klick auf 2D-Fläche = Ecke setzen · Klick auf Startpunkt / Doppelklick / Enter = Schließen · Rechtsklick = Zurück/Abbrechen"
        : "Click on 2D canvas = add corner · Click start point / double-click / Enter = close · Right-click = undo/cancel"
      : boardSketcher.mode === "rect"
        ? isDe
          ? "1. Ecke klicken & aufziehen (oder 2. Ecke klicken) · Rechtsklick = Abbrechen"
          : "Click 1st corner & drag (or click 2nd corner) · Right-click = cancel"
        : isDe
          ? "Ecke [■] ziehen = Verschieben · [+] auf Kante ziehen = Neue Ecke einfügen · Rechtsklick / Alt+Klick auf Ecke = Löschen"
          : "Drag corner [■] = move · Drag edge [+] = insert corner · Right-click / Alt+click corner = delete";

  card.appendChild(
    el("div", {
      style: "font-size:10px;line-height:1.35;color:var(--text-muted);",
      text: hintText,
    }),
  );

  return card;
}
