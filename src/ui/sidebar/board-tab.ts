import type {
  BaseboardConfig,
  BaseboardShape,
  BaseboardStyle,
  Layer,
  LayerPlate,
  LayerWoodPlateMode,
} from "@/core/types";
import {
  getBaseboards,
  addBaseboard,
  removeBaseboard,
  duplicateBaseboard,
  getBaseboardBounds,
  isPointInsideBaseboard,
  sortLayers,
} from "@/core/layout";
import { getPieceCenterWorld } from "@/core/geometry";
import { catalogRegistry } from "@/catalog/registry";
import { getLanguage, getTranslations } from "@/i18n";
import type { Sidebar } from "../sidebar";
import { buildMountainCard } from "./mountain-card";

// =========================================================================
// BOARD TAB HELPERS
// =========================================================================
const SHAPE_KEYS = [
  "triangleType", "lWingWidth", "lWingDepth", "lSide",
  "uLeftWingWidth", "uRightWingWidth", "uBackDepth", "tBarDepth", "tStemWidth",
] as const;

export const BOARD_PRESETS: Array<{ w: number; d: number; shape: BaseboardShape }> = [
  { w: 2400, d: 1200, shape: "rect" },
  { w: 2000, d: 1000, shape: "rect" },
  { w: 1200, d: 800, shape: "rect" },
  { w: 2000, d: 2000, shape: "triangle" },
  { w: 1500, d: 1500, shape: "triangle" },
  { w: 2600, d: 2000, shape: "l-shape" },
  { w: 3200, d: 2400, shape: "u-shape" },
  { w: 2500, d: 2000, shape: "t-shape" },
];

/** Copies shape + size from a table to a plate (or vice versa). */
export function copyShape(src: any, dst: any) {
  dst.shape = src.shape ?? "rect";
  dst.width = src.width;
  dst.depth = src.depth;
  for (const k of SHAPE_KEYS) dst[k] = src[k];
}

export function applyShapeDefaults(t: any) {
  switch (t.shape) {
    case "l-shape":
      t.lWingWidth ??= 800; t.lWingDepth ??= 800; t.lSide ??= "left"; break;
    case "u-shape":
      t.uLeftWingWidth ??= 700; t.uRightWingWidth ??= 700; t.uBackDepth ??= 600; break;
    case "t-shape":
      t.tBarDepth ??= 600; t.tStemWidth ??= 800; break;
    case "triangle":
      t.triangleType ??= "corner-tl"; break;
  }
}

export const getSurfaceStyleOptions = (isDe: boolean): [BaseboardStyle, string][] => [
  ["wood", `🌲 ${isDe ? "Sperrholz hell" : "Birch Plywood"}`],
  ["dark-wood", `🪵 ${isDe ? "Eiche dunkel" : "Dark Walnut"}`],
  ["plywood", `🪵 ${isDe ? "Kiefer" : "Multiplex Pine"}`],
  ["slate", `🪨 ${isDe ? "Schiefergrau" : "Slate Anthracite"}`],
  ["white", `⬜ ${isDe ? "Weiß" : "Blueprint White"}`],
  ["grass", `🌿 ${isDe ? "Gras / Wiese" : "Grass / Meadow"}`],
];

export function makePlate(bb: BaseboardConfig, layer: Layer, name: string, thicknessMm?: number): LayerPlate {
  const plate: any = {
    id: `plate-${Date.now()}-${Math.floor(Math.random() * 100000)}`,
    name,
    baseboardId: bb.id,
    x: 0,
    y: 0,
    thicknessMm: thicknessMm ?? layer.woodPlateThicknessMm ?? 12,
    positionMode: "custom",
    style: bb.style ?? "wood",
  };
  copyShape(bb, plate);
  return plate as LayerPlate;
}

export function syncWoodMode(layer: Layer) {
  // Ground table layer (height 0) represents the baseboard table itself.
  // It must always remain in "table" mode so the main table plate stays visible when plates are added to other layers.
  if ((layer.heightMm ?? 0) === 0) {
    if (!layer.woodPlateMode || layer.woodPlateMode === "none") {
      layer.woodPlateMode = "table";
    }
    return;
  }
  if ((layer.plates?.length ?? 0) > 0) {
    layer.woodPlateMode = "table";
  } else if (layer.woodPlateMode === "table") {
    layer.woodPlateMode = "none";
  }
}

export type ElOpts = { cls?: string; text?: string; html?: string; title?: string; style?: string };

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  opts: ElOpts = {},
  kids: Array<Node | null | false | undefined> = [],
): HTMLElementTagNameMap[K] {
  const n = document.createElement(tag);
  if (opts.cls) n.className = opts.cls;
  if (opts.text !== undefined) n.textContent = opts.text;
  if (opts.html !== undefined) n.innerHTML = opts.html;
  if (opts.title) n.title = opts.title;
  if (opts.style) n.style.cssText = opts.style;
  for (const k of kids) if (k) n.appendChild(k);
  return n;
}

export function btn(label: string, title: string, onClick: () => void, cls = "rb-btn rb-btn-secondary", style = ""): HTMLButtonElement {
  const b = el("button", { cls, text: label, title, style });
  b.type = "button";
  b.addEventListener("click", onClick);
  return b;
}

export function row(kids: Node[]): HTMLElement {
  return el("div", { cls: "board-inputs-row" }, kids);
}

export function numberField(
  label: string,
  value: number,
  o: { min?: number; max?: number; step?: number; unit?: string },
  onChange: (v: number) => void,
): HTMLElement {
  const input = el("input");
  input.type = "number";
  input.value = String(value);
  if (o.min !== undefined) input.min = String(o.min);
  if (o.max !== undefined) input.max = String(o.max);
  if (o.step !== undefined) input.step = String(o.step);
  input.addEventListener("change", () => {
    let v = parseFloat(input.value);
    if (!Number.isFinite(v)) { input.value = String(value); return; }
    if (o.min !== undefined) v = Math.max(o.min, v);
    if (o.max !== undefined) v = Math.min(o.max, v);
    input.value = String(v);
    onChange(v);
  });
  return el("div", { cls: "board-input-field" }, [
    label ? el("label", { text: label }) : null,
    el("div", { cls: "board-input-with-unit" }, [input, o.unit ? el("span", { cls: "unit-tag", text: o.unit }) : null]),
  ]);
}

export function selectField<T extends string>(
  label: string,
  options: Array<[T, string]>,
  value: T,
  onChange: (v: T) => void,
): HTMLElement {
  const sel = el("select", { cls: "board-select" });
  for (const [v, l] of options) {
    const opt = el("option", { text: l });
    opt.value = v;
    opt.selected = v === value;
    sel.appendChild(opt);
  }
  sel.addEventListener("change", () => onChange(sel.value as T));
  return el("div", { cls: "board-field" }, [el("label", { text: label }), sel]);
}

export interface BoardCtx {
  isDe: boolean;
  baseboards: BaseboardConfig[];
  bb: BaseboardConfig;
  notifyBoard: () => void;
  notifyLayer: () => void;
  refresh: () => void;
}

export function renderBoardTab(sidebar: Sidebar, container: HTMLElement) {
  const baseboards = getBaseboards(sidebar.layout);
  if (!sidebar.activeBoardId || !baseboards.some((b) => b.id === sidebar.activeBoardId)) {
    sidebar.activeBoardId = baseboards[0]?.id;
  }
  const bb = baseboards.find((b) => b.id === sidebar.activeBoardId) ?? baseboards[0]!;

  const notifyBoard = () => sidebar.callbacks.onBaseboardChange?.();
  const ctx: BoardCtx = {
    isDe: getLanguage() === "de",
    baseboards,
    bb,
    notifyBoard,
    notifyLayer: () => {
      sidebar.callbacks.onLayerChange?.();
      notifyBoard();
    },
    refresh: () => sidebar.render(),
  };

  const anyEnabled = baseboards.some((b) => b.enabled);
  container.appendChild(buildTableCard(sidebar, ctx, anyEnabled));
  if (anyEnabled) {
    container.appendChild(buildShapeCard(sidebar, ctx));
    container.appendChild(buildSurfaceCard(sidebar, ctx));
  }
  container.appendChild(buildWoodCard(sidebar, ctx));
  container.appendChild(buildMountainCard(sidebar, ctx));
}

// ---- Card 1: enable, add, select, name ----------------------------------
export function buildTableCard(sidebar: Sidebar, ctx: BoardCtx, anyEnabled: boolean): HTMLElement {
  const { isDe, baseboards, bb } = ctx;
  const tb = getTranslations().sidebar.board;
  const card = el("div", { cls: "board-card" });

  const enable = el("input");
  enable.type = "checkbox";
  enable.checked = anyEnabled;
  enable.addEventListener("change", () => {
    for (const b of baseboards) b.enabled = enable.checked;
    ctx.notifyBoard();
    ctx.refresh();
  });

  card.appendChild(
    el("div", { style: "display:flex;justify-content:space-between;align-items:center;gap:6px;" }, [
      el("label", { cls: "board-toggle-label" }, [enable, el("span", { text: tb.enableToggle })]),
      btn(`+ ${isDe ? "Tisch" : "Table"}`, isDe ? "Tisch hinzufügen" : "Add table", () => {
        sidebar.activeBoardId = addBaseboard(sidebar.layout).id;
        ctx.notifyBoard();
        ctx.refresh();
      }, "rb-btn rb-btn-secondary", "font-size:11px;padding:4px 8px;"),
    ]),
  );

  if (!anyEnabled) {
    card.appendChild(
      el("div", {
        cls: "empty-sub",
        style: "font-size:11px;margin-top:6px;",
        text: isDe
          ? "Offene Rahmenbauweise – Holzplatten pro Ebene unten konfigurieren."
          : "Open benchwork – configure wood plates per layer below.",
      }),
    );
    return card;
  }

  if (baseboards.length > 1) {
    const sel = el("select", { cls: "board-select", style: "margin-top:6px;" });
    baseboards.forEach((b, i) => {
      const o = el("option", {
        text: `${b.name ?? (isDe ? `Tisch ${i + 1}` : `Table ${i + 1}`)} (${b.width} × ${b.depth} mm)`,
      });
      o.value = b.id!;
      o.selected = b.id === bb.id;
      sel.appendChild(o);
    });
    sel.addEventListener("change", () => {
      sidebar.activeBoardId = sel.value;
      ctx.refresh();
    });
    card.appendChild(sel);
  }

  const name = el("input", { cls: "board-text-input", style: "flex:1;min-width:0;" });
  name.type = "text";
  name.value = bb.name ?? "Tisch";
  name.placeholder = tb.tableName;
  name.addEventListener("input", () => {
    bb.name = name.value;
    ctx.notifyBoard();
  });

  card.appendChild(
    el("div", { style: "display:flex;gap:6px;align-items:center;margin-top:6px;" }, [
      name,
      btn("📑", tb.duplicateTable, () => {
        const clone = duplicateBaseboard(sidebar.layout, bb.id!);
        if (clone) {
          sidebar.activeBoardId = clone.id;
          ctx.notifyBoard();
          ctx.refresh();
        }
      }),
      baseboards.length > 1
        ? btn("🗑️", tb.deleteTable, () => {
            removeBaseboard(sidebar.layout, bb.id!);
            sidebar.activeBoardId = sidebar.layout.baseboards?.[0]?.id;
            ctx.notifyBoard();
            ctx.refresh();
          }, "rb-btn rb-btn-secondary", "color:#ef4444;")
        : null,
    ]),
  );
  return card;
}

// ---- Card 2: shape, presets, size, fit ----------------------------------
export function buildShapeCard(sidebar: Sidebar, ctx: BoardCtx): HTMLElement {
  const { isDe, baseboards, bb } = ctx;
  const tb = getTranslations().sidebar.board;
  const card = el("div", { cls: "board-card", style: "display:flex;flex-direction:column;gap:6px;" }, [
    el("div", { cls: "board-section-title", text: `${tb.shape} & ${isDe ? "Maße" : "Dimensions"}` }),
  ]);

  card.appendChild(buildShapeEditor(bb, isDe, ctx.notifyBoard, ctx.refresh, 200));

  const presets = el("select", { cls: "board-select" });
  const ph = el("option", { text: isDe ? "Vorlage…" : "Preset…" });
  ph.value = "";
  presets.appendChild(ph);
  BOARD_PRESETS.forEach((p, i) => {
    const o = el("option", { text: `${p.w} × ${p.d} mm (${shapeLabel(p.shape)})` });
    o.value = String(i);
    presets.appendChild(o);
  });
  presets.addEventListener("change", () => {
    const p = BOARD_PRESETS[Number(presets.value)];
    if (!p) return;
    bb.width = p.w;
    bb.depth = p.d;
    bb.shape = p.shape;
    applyShapeDefaults(bb);
    ctx.notifyBoard();
    ctx.refresh();
  });
  card.appendChild(presets);

  card.appendChild(
    btn(`📐 ${tb.fitTracks}`, isDe ? "Tisch an alle Gleise anpassen" : "Fit table to placed tracks", () => {
      if (!fitBoardToTracks(sidebar, bb, baseboards)) {
        alert(isDe ? "Noch keine Gleise im Plan vorhanden." : "No tracks on the layout yet!");
        return;
      }
      ctx.notifyBoard();
      ctx.refresh();
    }, "board-btn-fit"),
  );
  return card;
}

export function shapeLabel(shape: BaseboardShape): string {
  const s = getTranslations().sidebar.board.shapes;
  return ({ rect: s.rect, "l-shape": s.lShape, "u-shape": s.uShape, "t-shape": s.tShape, triangle: s.triangle } as Record<string, string>)[shape] ?? shape;
}

/** Shape select + width/depth + shape-specific fields. Works for tables AND plates. */
export function buildShapeEditor(
  t: any,
  isDe: boolean,
  notify: () => void,
  refresh: () => void,
  minSize: number,
): HTMLElement {
  const s = getTranslations().sidebar.board.shapes;
  const box = el("div", { style: "display:flex;flex-direction:column;gap:6px;" });

  box.appendChild(
    selectField<BaseboardShape>(
      "",
      [
        ["rect", `⬛ ${s.rect}`],
        ["l-shape", `⌐ ${s.lShape}`],
        ["u-shape", `⊔ ${s.uShape}`],
        ["t-shape", `⊤ ${s.tShape}`],
        ["triangle", `▲ ${s.triangle}`],
      ],
      t.shape ?? "rect",
      (v) => {
        t.shape = v;
        applyShapeDefaults(t);
        notify();
        refresh();
      },
    ),
  );

  box.appendChild(
    row([
      numberField(isDe ? "Breite (X)" : "Width (X)", t.width, { min: minSize, max: 20000, step: 50, unit: "mm" }, (v) => {
        t.width = v;
        notify();
      }),
      numberField(isDe ? "Tiefe (Y)" : "Depth (Y)", t.depth, { min: minSize, max: 20000, step: 50, unit: "mm" }, (v) => {
        t.depth = v;
        notify();
      }),
    ]),
  );

  if (t.shape === "l-shape") {
    box.appendChild(
      row([
        numberField(isDe ? "Flügelbreite" : "Wing Width", t.lWingWidth ?? 800, { min: 50, step: 50, unit: "mm" }, (v) => { t.lWingWidth = v; notify(); }),
        numberField(isDe ? "Haupttiefe" : "Main Depth", t.lWingDepth ?? 800, { min: 50, step: 50, unit: "mm" }, (v) => { t.lWingDepth = v; notify(); }),
      ]),
    );
    box.appendChild(
      selectField<"left" | "right">(
        isDe ? "Flügelposition" : "Wing Position",
        [["left", isDe ? "Links" : "Left"], ["right", isDe ? "Rechts" : "Right"]],
        t.lSide ?? "left",
        (v) => { t.lSide = v; notify(); },
      ),
    );
  } else if (t.shape === "u-shape") {
    box.appendChild(
      row([
        numberField(isDe ? "Schenkel links" : "Left Wing", t.uLeftWingWidth ?? 700, { min: 50, step: 50, unit: "mm" }, (v) => { t.uLeftWingWidth = v; notify(); }),
        numberField(isDe ? "Schenkel rechts" : "Right Wing", t.uRightWingWidth ?? 700, { min: 50, step: 50, unit: "mm" }, (v) => { t.uRightWingWidth = v; notify(); }),
      ]),
    );
    box.appendChild(
      numberField(isDe ? "Quertiefe" : "Bridge Depth", t.uBackDepth ?? 600, { min: 50, step: 50, unit: "mm" }, (v) => { t.uBackDepth = v; notify(); }),
    );
  } else if (t.shape === "t-shape") {
    box.appendChild(
      row([
        numberField(isDe ? "Kopfteil-Tiefe" : "Top Bar Depth", t.tBarDepth ?? 600, { min: 50, step: 50, unit: "mm" }, (v) => { t.tBarDepth = v; notify(); }),
        numberField(isDe ? "Zungenbreite" : "Stem Width", t.tStemWidth ?? 800, { min: 50, step: 50, unit: "mm" }, (v) => { t.tStemWidth = v; notify(); }),
      ]),
    );
  } else if (t.shape === "triangle") {
    box.appendChild(
      selectField<string>(
        isDe ? "Ausrichtung" : "Orientation",
        [
          ["corner-tl", `📐 ${isDe ? "Ecke oben links" : "Corner top-left"}`],
          ["corner-tr", `📐 ${isDe ? "Ecke oben rechts" : "Corner top-right"}`],
          ["corner-br", `📐 ${isDe ? "Ecke unten rechts" : "Corner bottom-right"}`],
          ["corner-bl", `📐 ${isDe ? "Ecke unten links" : "Corner bottom-left"}`],
          ["isosceles", `▲ ${isDe ? "Gleichschenklig" : "Isosceles"}`],
        ],
        t.triangleType ?? "corner-tl",
        (v) => { t.triangleType = v; notify(); },
      ),
    );
  }
  return box;
}

/** Fits the table around the tracks (in the table's rotated frame). Returns false if there are no tracks. */
export function fitBoardToTracks(sidebar: Sidebar, bb: BaseboardConfig, baseboards: BaseboardConfig[]): boolean {
  const pieces = sidebar.layout.pieces;
  if (pieces.length === 0) return false;

  let candidates = pieces;
  if (baseboards.length > 1) {
    const onBoard = pieces.filter(
      (p) =>
        isPointInsideBaseboard({ x: p.transform.x, y: p.transform.y }, bb) ||
        (p.connectors?.some((c) => isPointInsideBaseboard(c, bb)) ?? false),
    );
    if (onBoard.length > 0) candidates = onBoard;
  }

  const allEntries = catalogRegistry.getAll().flatMap((c) => c.entries);
  const pts: { x: number; y: number }[] = [];
  for (const piece of candidates) {
    pts.push({ x: piece.transform.x, y: piece.transform.y });
    for (const c of piece.connectors ?? []) pts.push({ x: c.x, y: c.y });
    const entry =
      sidebar.catalog.entries.find((e) => e.id === piece.catalogId) ?? allEntries.find((e) => e.id === piece.catalogId);
    if (entry) pts.push(getPieceCenterWorld(entry, piece.transform));
  }
  if (pts.length === 0) return false;

  const rad = ((bb.rotation ?? 0) * Math.PI) / 180;
  const cosN = Math.cos(-rad), sinN = Math.sin(-rad);
  let minU = Infinity, maxU = -Infinity, minV = Infinity, maxV = -Infinity;
  for (const p of pts) {
    const u = p.x * cosN - p.y * sinN;
    const v = p.x * sinN + p.y * cosN;
    minU = Math.min(minU, u); maxU = Math.max(maxU, u);
    minV = Math.min(minV, v); maxV = Math.max(maxV, v);
  }

  const padding = 100;
  const w = Math.max(400, Math.ceil((maxU - minU + padding * 2) / 50) * 50);
  const d = Math.max(400, Math.ceil((maxV - minV + padding * 2) / 50) * 50);
  const midU = (minU + maxU) / 2, midV = (minV + maxV) / 2;
  const cx = midU * Math.cos(rad) - midV * Math.sin(rad);
  const cy = midU * Math.sin(rad) + midV * Math.cos(rad);

  bb.width = w;
  bb.depth = d;
  bb.positionMode = "custom";
  bb.customX = Math.round(cx - w / 2);
  bb.customY = Math.round(cy - d / 2);
  return true;
}

// ---- Card 3: position & surface (collapsed by default) -------------------
export function buildSurfaceCard(sidebar: Sidebar, ctx: BoardCtx): HTMLElement {
  const { isDe, baseboards, bb } = ctx;
  const tb = getTranslations().sidebar.board;

  const card = el("details", { cls: "board-card" });
  card.open = sidebar.surfaceOpen;
  card.addEventListener("toggle", () => { sidebar.surfaceOpen = card.open; });
  card.appendChild(
    el("summary", { cls: "board-section-title", style: "cursor:pointer;", text: isDe ? "Position & Oberfläche" : "Position & Surface" }),
  );

  const body = el("div", { style: "display:flex;flex-direction:column;gap:6px;margin-top:6px;" });

  body.appendChild(
    row([
      selectField<string>(
        tb.positioning,
        [
          ["center", isDe ? "Zentriert (0, 0)" : "Centered (0, 0)"],
          ["top-left", isDe ? "Oben-Links (0, 0)" : "Top-Left (0, 0)"],
          ["custom", isDe ? "Benutzerdefiniert" : "Custom Offset"],
        ],
        bb.positionMode ?? "center",
        (v) => { bb.positionMode = v as any; ctx.notifyBoard(); ctx.refresh(); },
      ),
      selectField<BaseboardStyle>(
        tb.style,
        getSurfaceStyleOptions(isDe),
        bb.style ?? "wood",
        (v) => { bb.style = v; ctx.notifyBoard(); },
      ),
    ]),
  );

  if (bb.positionMode === "custom") {
    body.appendChild(
      row([
        numberField("Offset X", bb.customX ?? 0, { step: 20, unit: "mm" }, (v) => { bb.customX = v; ctx.notifyBoard(); }),
        numberField("Offset Y", bb.customY ?? 0, { step: 20, unit: "mm" }, (v) => { bb.customY = v; ctx.notifyBoard(); }),
      ]),
    );
  }

  if (baseboards.length > 1) {
    body.appendChild(
      btn(tb.snapToRight, tb.snapToRight, () => {
        const other = baseboards.find((x) => x.id !== bb.id);
        if (!other) return;
        const ob = getBaseboardBounds(other);
        bb.positionMode = "custom";
        bb.customX = Math.round(ob.x + ob.width + 20);
        bb.customY = Math.round(ob.y);
        ctx.notifyBoard();
        ctx.refresh();
      }, "board-btn-fit"),
    );
  }

  // Rotation
  const setRotation = (deg: number) => {
    bb.rotation = Math.round((((deg % 360) + 360) % 360) * 10) / 10;
    ctx.notifyBoard();
    ctx.refresh();
  };
  const stepBtn = (label: string, title: string, fn: () => void) =>
    btn(label, title, fn, "board-btn-fit", "padding:4px 6px;font-size:11px;white-space:nowrap;");
  body.appendChild(
    el("div", { cls: "board-field" }, [
      el("label", { text: isDe ? "Drehung" : "Rotation" }),
      el("div", { style: "display:flex;gap:4px;align-items:flex-end;" }, [
        el("div", { style: "flex:1;min-width:70px;" }, [
          numberField("", bb.rotation ?? 0, { step: 15, unit: "°" }, setRotation),
        ]),
        stepBtn("⟲ 45°", isDe ? "45° gegen den Uhrzeigersinn" : "Rotate 45° CCW", () => setRotation((bb.rotation ?? 0) - 45)),
        stepBtn("⟳ 45°", isDe ? "45° im Uhrzeigersinn" : "Rotate 45° CW", () => setRotation((bb.rotation ?? 0) + 45)),
        stepBtn("0°", isDe ? "Zurücksetzen" : "Reset", () => setRotation(0)),
      ]),
    ]),
  );

  // Corner radius + dimensions toggle
  const dims = el("input");
  dims.type = "checkbox";
  dims.checked = bb.showDimensions !== false;
  dims.addEventListener("change", () => { bb.showDimensions = dims.checked; ctx.notifyBoard(); });
  body.appendChild(
    row([
      numberField(isDe ? "Eckenrundung" : "Corner Radius", bb.cornerRadius ?? 12, { min: 0, max: 100, unit: "mm" }, (v) => {
        bb.cornerRadius = v;
        ctx.notifyBoard();
      }),
      el("label", { cls: "board-toggle-label", style: "font-size:11px;align-self:flex-end;padding-bottom:5px;" }, [
        dims,
        el("span", { text: isDe ? "Maße anzeigen" : "Show Dimensions" }),
      ]),
    ]),
  );

  card.appendChild(body);
  return card;
}

// ---- Card 4: wood plates, one accordion entry per layer ------------------
export function buildWoodCard(sidebar: Sidebar, ctx: BoardCtx): HTMLElement {
  const { isDe, baseboards, bb } = ctx;
  const layers = sortLayers(sidebar.layout.layers || []);
  const defaultTableId = baseboards[0]?.id || "bb-1";
  const platesOf = (l: Layer) => (l.plates ?? []).filter((p) => (p.baseboardId || defaultTableId) === bb.id);
  const ensureWoodVisible = () => {
    if (sidebar.callbacks.isWoodPlateActive?.() === false) sidebar.callbacks.onToggleWoodPlate?.();
  };
  const commit = (showWood: boolean) => {
    layers.forEach(syncWoodMode);
    if (showWood) ensureWoodVisible();
    ctx.notifyLayer();
    ctx.refresh();
  };

  const card = el("div", { cls: "board-card board-woodplate-section", style: "display:flex;flex-direction:column;gap:6px;" });

  // Header
  const woodOn = sidebar.callbacks.isWoodPlateActive?.() ?? false;
  card.appendChild(
    el("div", { style: "display:flex;align-items:center;justify-content:space-between;gap:6px;" }, [
      el("div", { cls: "board-section-title", style: "margin:0;font-size:11px;", text: `🪵 ${isDe ? "Holzplatten" : "Wood Plates"}` }),
      btn(
        woodOn ? (isDe ? "👁️ sichtbar" : "👁️ visible") : (isDe ? "🚫 ausgeblendet" : "🚫 hidden"),
        isDe ? "Holzplatten in der 3D-Ansicht ein-/ausblenden" : "Show/hide wood plates in 3D",
        () => { sidebar.callbacks.onToggleWoodPlate?.(); ctx.refresh(); },
        `rb-btn ${woodOn ? "rb-btn-primary" : "rb-btn-secondary"}`,
        "font-size:10px;padding:2px 7px;",
      ),
    ]),
  );

  // One accordion entry per layer
  for (const layer of layers) {
    const plates = platesOf(layer);
    const det = el("details", { cls: "board-layer-wood-card" });
    det.open = sidebar.openPlateLayers.has(layer.id);
    det.addEventListener("toggle", () => {
      if (det.open) sidebar.openPlateLayers.add(layer.id);
      else sidebar.openPlateLayers.delete(layer.id);
    });

    const h = layer.heightMm ?? 0;
    det.appendChild(
      el("summary", { style: "display:flex;align-items:center;gap:6px;cursor:pointer;" }, [
        el("span", { cls: "board-layer-dot", style: `background:${layer.color || "#38bdf8"};` }),
        el("span", { text: layer.name, style: "font-weight:700;flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;" }),
        el("span", { cls: "board-layer-elev-badge", text: `${h >= 0 ? "+" : ""}${h} mm` }),
        el("span", { text: plates.length ? `🪵 ${plates.length}` : "–", style: "font-size:10px;color:var(--text-muted);" }),
      ]),
    );

    const body = el("div", { style: "display:flex;flex-direction:column;gap:6px;margin-top:6px;" });
    for (const plate of plates) body.appendChild(buildPlateEditor(sidebar, plate, layer, ctx, commit));

    body.appendChild(
      btn(
        plates.length === 0
          ? `📐 ${isDe ? `Platte im Tischmaß (${bb.width} × ${bb.depth} mm)` : `Plate at table size (${bb.width} × ${bb.depth} mm)`}`
          : `+ ${isDe ? "Weitere Platte" : "Add another plate"}`,
        isDe ? "Platte zu diesem Tisch und dieser Ebene hinzufügen" : "Add a plate to this table and layer",
        () => {
          const n = plates.length + 1;
          (layer.plates ??= []).push(makePlate(bb, layer, isDe ? `Platte ${n}` : `Plate ${n}`));
          sidebar.openPlateLayers.add(layer.id);
          commit(true);
        },
        "board-btn-fit",
      ),
    );
    det.appendChild(body);
    card.appendChild(det);
  }
  return card;
}

export function buildPlateEditor(
  sidebar: Sidebar,
  plate: LayerPlate,
  layer: Layer,
  ctx: BoardCtx,
  commit: (showWood: boolean) => void,
): HTMLElement {
  const { isDe, bb } = ctx;
  const tb = getTranslations().sidebar.board;
  const box = el("div", { cls: "board-plate-item-card", style: "display:flex;flex-direction:column;gap:6px;" });

  const name = el("input", { cls: "board-text-input", style: "flex:1;min-width:0;height:23px;font-size:11px;font-weight:600;" });
  name.type = "text";
  name.value = plate.name ?? "";
  name.addEventListener("input", () => { plate.name = name.value; ctx.notifyLayer(); });

  box.appendChild(
    el("div", { cls: "board-plate-header" }, [
      name,
      btn("🗑️", isDe ? "Platte entfernen" : "Delete plate", () => {
        layer.plates = (layer.plates ?? []).filter((p) => p.id !== plate.id);
        commit(false);
      }, "rb-btn rb-btn-secondary", "color:#ef4444;padding:2px 6px;height:23px;"),
    ]),
  );

  box.appendChild(buildShapeEditor(plate, isDe, ctx.notifyLayer, ctx.refresh, 50));

  box.appendChild(
    row([
      selectField<BaseboardStyle>(
        tb.style,
        getSurfaceStyleOptions(isDe),
        plate.style ?? bb.style ?? "wood",
        (v) => {
          plate.style = v;
          ctx.notifyLayer();
        },
      ),
      numberField(isDe ? "Stärke" : "Thickness", plate.thicknessMm ?? layer.woodPlateThicknessMm ?? 10, { min: 2, max: 50, step: 1, unit: "mm" }, (v) => {
        plate.thicknessMm = v;
        ctx.notifyLayer();
      }),
    ]),
  );
  box.appendChild(
    row([
      numberField("Offset X", plate.x ?? 0, { step: 20, unit: "mm" }, (v) => { plate.x = v; ctx.notifyLayer(); }),
      numberField("Offset Y", plate.y ?? 0, { step: 20, unit: "mm" }, (v) => { plate.y = v; ctx.notifyLayer(); }),
    ]),
  );

  box.appendChild(
    btn(
      `📐 ${isDe ? "An Tisch anpassen" : "Fit to Table"} (${bb.width} × ${bb.depth} mm)`,
      isDe ? "Maße und Position exakt an den Tisch anpassen" : "Fit plate size and position to table",
      () => {
        copyShape(bb, plate);
        plate.baseboardId = bb.id;
        plate.x = 0;
        plate.y = 0;
        plate.positionMode = "custom";
        if (bb.style) plate.style = bb.style;
        commit(true);
      },
      "board-btn-fit",
    ),
  );
  return box;
}
