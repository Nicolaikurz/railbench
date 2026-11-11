import type { Layout, Layer, TrackPiece, BaseboardConfig, LayerPlate } from "./types";
import { distToSegment } from "./geometry";

export function createDefaultBaseboard(partial?: Partial<BaseboardConfig>): BaseboardConfig {
  return {
    id: partial?.id ?? `board-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
    name: partial?.name ?? "Tisch 1 (Hauptplatte)",
    enabled: partial?.enabled ?? true,
    shape: partial?.shape ?? "rect",
    triangleType: partial?.triangleType,
    width: partial?.width ?? 2400,
    depth: partial?.depth ?? 1200,
    rotation: partial?.rotation ?? 0,
    lWingWidth: partial?.lWingWidth ?? 800,
    lWingDepth: partial?.lWingDepth ?? 800,
    lSide: partial?.lSide ?? "left",
    uLeftWingWidth: partial?.uLeftWingWidth ?? 700,
    uRightWingWidth: partial?.uRightWingWidth ?? 700,
    uBackDepth: partial?.uBackDepth ?? 600,
    tBarDepth: partial?.tBarDepth ?? 600,
    tStemWidth: partial?.tStemWidth ?? 800,
    positionMode: partial?.positionMode ?? "center",
    customX: partial?.customX ?? 0,
    customY: partial?.customY ?? 0,
    style: partial?.style ?? "wood",
    cornerRadius: partial?.cornerRadius ?? 12,
    showDimensions: partial?.showDimensions ?? true,
  };
}

export function getBaseboards(layout: Layout): BaseboardConfig[] {
  if (layout.baseboards && layout.baseboards.length > 1) {
    layout.baseboard = layout.baseboards[0];
    return layout.baseboards;
  }

  if (layout.baseboards && layout.baseboards.length === 1) {
    if (layout.baseboard && layout.baseboard !== layout.baseboards[0]) {
      layout.baseboards = [layout.baseboard];
    }
    layout.baseboard = layout.baseboards[0];
    return layout.baseboards;
  }

  if (layout.baseboard) {
    if (!layout.baseboard.id) layout.baseboard.id = "board-1";
    if (!layout.baseboard.name) layout.baseboard.name = "Tisch 1 (Hauptplatte)";
    layout.baseboards = [layout.baseboard];
  } else {
    const def = createDefaultBaseboard();
    layout.baseboards = [def];
    layout.baseboard = def;
  }

  layout.baseboard = layout.baseboards[0];
  return layout.baseboards;
}

export function ensureBaseboard(layout: Layout): BaseboardConfig {
  const boards = getBaseboards(layout);
  return boards[0]!;
}

export function addBaseboard(layout: Layout, config?: Partial<BaseboardConfig>): BaseboardConfig {
  const boards = getBaseboards(layout);
  const nextNum = boards.length + 1;
  const lastBoard = boards[boards.length - 1];
  const lastBounds = lastBoard ? getBaseboardBounds(lastBoard) : { x: 0, y: 0, width: 2400, depth: 1200 };

  const newBoard = createDefaultBaseboard({
    id: `board-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
    name: `Tisch ${nextNum}`,
    width: 1200,
    depth: 800,
    positionMode: "custom",
    customX: Math.round(lastBounds.x + lastBounds.width + 50),
    customY: Math.round(lastBounds.y),
    ...config,
  });

  boards.push(newBoard);
  layout.baseboards = boards;
  layout.baseboard = boards[0];
  return newBoard;
}

export function removeBaseboard(layout: Layout, id: string): boolean {
  const boards = getBaseboards(layout);
  const idx = boards.findIndex((b) => b.id === id);
  if (idx !== -1) {
    boards.splice(idx, 1);
    if (boards.length === 0) {
      boards.push(createDefaultBaseboard());
    }
    layout.baseboards = boards;
    layout.baseboard = boards[0];
    return true;
  }
  return false;
}

export function duplicateBaseboard(layout: Layout, id: string): BaseboardConfig | undefined {
  const boards = getBaseboards(layout);
  const source = boards.find((b) => b.id === id);
  if (!source) return undefined;

  const bounds = getBaseboardBounds(source);
  const clone: BaseboardConfig = {
    ...source,
    id: `board-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
    name: `${source.name ?? "Tisch"} (Kopie)`,
    positionMode: "custom",
    customX: Math.round(bounds.x + bounds.width + 50),
    customY: Math.round(bounds.y),
  };

  boards.push(clone);
  layout.baseboards = boards;
  layout.baseboard = boards[0];
  return clone;
}

export function getBaseboardBounds(config: BaseboardConfig): { x: number; y: number; width: number; depth: number } {
  let x = 0;
  let y = 0;
  if (config.positionMode === "center") {
    x = -config.width / 2;
    y = -config.depth / 2;
  } else if (config.positionMode === "top-left") {
    x = 0;
    y = 0;
  } else {
    x = config.customX ?? 0;
    y = config.customY ?? 0;
  }
  return { x, y, width: config.width, depth: config.depth };
}

export function getBaseboardPolygon(config: BaseboardConfig): { x: number; y: number }[] {
  const { x, y, width: W, depth: D } = getBaseboardBounds(config);
  const shape = config.shape ?? "rect";
  let poly: { x: number; y: number }[];

  if (shape === "rect") {
    poly = [
      { x, y },
      { x: x + W, y },
      { x: x + W, y: y + D },
      { x, y: y + D },
    ];
  } else if (shape === "triangle") {
    const triType = config.triangleType ?? "corner-tl";
    if (triType === "corner-tl") {
      poly = [
        { x, y },
        { x: x + W, y },
        { x, y: y + D },
      ];
    } else if (triType === "corner-tr") {
      poly = [
        { x, y },
        { x: x + W, y },
        { x: x + W, y: y + D },
      ];
    } else if (triType === "corner-br") {
      poly = [
        { x: x + W, y },
        { x: x + W, y: y + D },
        { x, y: y + D },
      ];
    } else if (triType === "corner-bl") {
      poly = [
        { x, y },
        { x: x + W, y: y + D },
        { x, y: y + D },
      ];
    } else {
      // "isosceles" peak at top-center, base at bottom
      poly = [
        { x: x + W / 2, y },
        { x: x + W, y: y + D },
        { x, y: y + D },
      ];
    }
  } else if (shape === "l-shape") {
    const isLeft = (config.lSide ?? "left") === "left";
    const wWing = Math.min(config.lWingWidth ?? 800, W * 0.85);
    const dMain = Math.min(config.lWingDepth ?? 800, D * 0.85);

    if (isLeft) {
      poly = [
        { x, y },
        { x: x + W, y },
        { x: x + W, y: y + dMain },
        { x: x + wWing, y: y + dMain },
        { x: x + wWing, y: y + D },
        { x, y: y + D },
      ];
    } else {
      poly = [
        { x, y },
        { x: x + W, y },
        { x: x + W, y: y + D },
        { x: x + W - wWing, y: y + D },
        { x: x + W - wWing, y: y + dMain },
        { x, y: y + dMain },
      ];
    }
  } else if (shape === "u-shape") {
    const wLeft = Math.min(config.uLeftWingWidth ?? 700, W * 0.45);
    const wRight = Math.min(config.uRightWingWidth ?? 700, W * 0.45);
    const dBack = Math.min(config.uBackDepth ?? 600, D * 0.85);

    poly = [
      { x, y },
      { x: x + W, y },
      { x: x + W, y: y + D },
      { x: x + W - wRight, y: y + D },
      { x: x + W - wRight, y: y + dBack },
      { x: x + wLeft, y: y + dBack },
      { x: x + wLeft, y: y + D },
      { x, y: y + D },
    ];
  } else if (shape === "t-shape") {
    const dBar = Math.min(config.tBarDepth ?? 600, D * 0.85);
    const wStem = Math.min(config.tStemWidth ?? 800, W * 0.85);
    const stemLeft = x + (W - wStem) / 2;
    const stemRight = stemLeft + wStem;

    poly = [
      { x, y },
      { x: x + W, y },
      { x: x + W, y: y + dBar },
      { x: stemRight, y: y + dBar },
      { x: stemRight, y: y + D },
      { x: stemLeft, y: y + D },
      { x: stemLeft, y: y + dBar },
      { x, y: y + dBar },
    ];
  } else {
    poly = [
      { x, y },
      { x: x + W, y },
      { x: x + W, y: y + D },
      { x, y: y + D },
    ];
  }

  const rot = config.rotation ?? 0;
  if (rot !== 0) {
    const cx = x + W / 2;
    const cy = y + D / 2;
    const rad = (rot * Math.PI) / 180;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    poly = poly.map((pt) => {
      const dx = pt.x - cx;
      const dy = pt.y - cy;
      return {
        x: Math.round((cx + dx * cos - dy * sin) * 10) / 10,
        y: Math.round((cy + dx * sin + dy * cos) * 10) / 10,
      };
    });
  }

  return poly;
}

/** Berechnet die Polygonecken einer Ebenen-Holzplatte unter Berücksichtigung ihrer Form (Rechteck, Dreieck, L, U, T)
 * und der Rotation bzw. Position des zugeordneten Tisches. */
export function getLayerPlatePolygon(
  plate: LayerPlate,
  targetBoard?: BaseboardConfig,
): { x: number; y: number }[] {
  const tableBounds = targetBoard ? getBaseboardBounds(targetBoard) : { x: 0, y: 0 };
  const W = Math.max(50, plate.width);
  const D = Math.max(50, plate.depth);
  const x = tableBounds.x + (plate.x ?? 0);
  const y = tableBounds.y + (plate.y ?? 0);

  const dummyBb: BaseboardConfig = {
    enabled: true,
    width: W,
    depth: D,
    positionMode: "custom",
    customX: x,
    customY: y,
    shape: plate.shape ?? "rect",
    triangleType: plate.triangleType,
    lWingWidth: plate.lWingWidth,
    lWingDepth: plate.lWingDepth,
    lSide: plate.lSide,
    uLeftWingWidth: plate.uLeftWingWidth,
    uRightWingWidth: plate.uRightWingWidth,
    uBackDepth: plate.uBackDepth,
    tBarDepth: plate.tBarDepth,
    tStemWidth: plate.tStemWidth,
    rotation: plate.rotation ?? 0,
    style: "wood",
  };

  let poly = getBaseboardPolygon(dummyBb);

  const rot = targetBoard?.rotation ?? 0;
  if (rot !== 0 && targetBoard) {
    const cx = tableBounds.x + targetBoard.width / 2;
    const cy = tableBounds.y + targetBoard.depth / 2;
    const rad = (rot * Math.PI) / 180;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    poly = poly.map((pt) => {
      const dx = pt.x - cx;
      const dy = pt.y - cy;
      return {
        x: Math.round((cx + dx * cos - dy * sin) * 10) / 10,
        y: Math.round((cy + dx * sin + dy * cos) * 10) / 10,
      };
    });
  }

  return poly;
}

/** Berechnet die eingerückten Montagepositionen der Tischbeine (in mm Weltkoordinaten),
 * sodass kein Bein über die Tischkanten oder Tischecken hinausragt. */
export function computeBaseboardLegPositions(
  config: BaseboardConfig,
  legRadius = 18,
  baseInset = 45,
): { x: number; y: number }[] {
  const polygon = getBaseboardPolygon(config);
  if (polygon.length < 3) return [];

  const placedLegs: { x: number; y: number }[] = [];

  for (let i = 0; i < polygon.length; i++) {
    const pPrev = polygon[(i - 1 + polygon.length) % polygon.length]!;
    const pCurr = polygon[i]!;
    const pNext = polygon[(i + 1) % polygon.length]!;

    const v1x = pCurr.x - pPrev.x;
    const v1y = pCurr.y - pPrev.y;
    const l1 = Math.hypot(v1x, v1y);

    const v2x = pNext.x - pCurr.x;
    const v2y = pNext.y - pCurr.y;
    const l2 = Math.hypot(v2x, v2y);

    if (l1 < 1e-4 || l2 < 1e-4) continue;

    const u1x = v1x / l1;
    const u1y = v1y / l1;
    const u2x = v2x / l2;
    const u2y = v2y / l2;

    // Einwärts gerichtete Normalen für im Uhrzeigersinn orientierte Polygone
    const n1x = -u1y;
    const n1y = u1x;
    const n2x = -u2y;
    const n2y = u2x;

    // Dynamischer Inset: Bei kurzen Kanten skalieren, aber mindestens legRadius + 5 mm
    const minEdge = Math.min(l1, l2);
    const inset = Math.min(baseInset, Math.max(legRadius + 5, minEdge * 0.25));

    const bisX = n1x + n2x;
    const bisY = n1y + n2y;
    const bisLen = Math.hypot(bisX, bisY);

    let legX = pCurr.x + bisX * inset;
    let legY = pCurr.y + bisY * inset;
    if (bisLen > 1e-4) {
      const bisScale = Math.min(inset * 2.8, (2 * inset) / bisLen);
      legX = pCurr.x + (bisX / bisLen) * bisScale;
      legY = pCurr.y + (bisY / bisLen) * bisScale;
    }

    // Mindestabstand zu vorherigen Beinen
    const tooClose = placedLegs.some((p) => Math.hypot(p.x - legX, p.y - legY) < 80);
    if (tooClose) continue;

    // Nur Beine zulassen, deren Mittelpunkt innerhalb der Platte liegt
    if (!isPointInsideBaseboard({ x: legX, y: legY }, config)) continue;

    placedLegs.push({ x: legX, y: legY });
  }

  return placedLegs;
}

export function createDefaultLayers(): Layer[] {
  return [
    {
      id: "layer-1",
      name: "Ground Level (0 mm)",
      color: "#38bdf8",
      visible: true,
      locked: false,
      heightMm: 0,
      woodPlateMode: "table",
      woodPlateThicknessMm: 12,
    },
  ];
}

/**
 * Sortiert Ebenen automatisch:
 * 1. Primär nach Höhe absteigend (höchste Ebene ganz oben)
 * 2. Sekundär nach Name aufsteigend (bei gleicher Höhe)
 */
export function sortLayers(layers: Layer[]): Layer[] {
  return [...layers].sort((a, b) => {
    const hA = a.heightMm ?? 0;
    const hB = b.heightMm ?? 0;
    if (hB !== hA) {
      return hB - hA;
    }
    return (a.name || "").localeCompare(b.name || "", undefined, { numeric: true, sensitivity: "base" });
  });
}

export function ensureDefaultLayers(layout: Layout): Layout {
  ensureBaseboard(layout);
  if (!layout.layers || layout.layers.length === 0) {
    layout.layers = createDefaultLayers();
  }
  for (const layer of layout.layers) {
    if ((layer.heightMm ?? 0) === 0 && !layer.woodPlateMode) {
      layer.woodPlateMode = "table";
    }
  }
  layout.layers = sortLayers(layout.layers);
  if (!layout.activeLayerId || !layout.layers.some((l) => l.id === layout.activeLayerId)) {
    layout.activeLayerId = layout.layers[0]!.id;
  }
  for (const piece of layout.pieces) {
    if (!piece.layerId) {
      piece.layerId = layout.activeLayerId;
    }
  }
  return layout;
}

export function createLayout(name = "New Layout"): Layout {
  const defBoard = createDefaultBaseboard();
  const l: Layout = {
    name,
    scale: "H0",
    pieces: [],
    models: [],
    layers: createDefaultLayers(),
    activeLayerId: "layer-1",
    baseboard: defBoard,
    baseboards: [defBoard],
  };
  return l;
}

export function addPiece(layout: Layout, piece: TrackPiece): void {
  if (!piece.layerId) {
    piece.layerId = layout.activeLayerId ?? "layer-1";
  }
  layout.pieces.push(piece);
}

export function removePiece(layout: Layout, id: string): void {
  layout.pieces = layout.pieces.filter((p) => p.id !== id);
}

/** Prüft, ob ein Weltpunkt innerhalb der Tischplatte liegt. */
export function isPointInsideBaseboard(point: { x: number; y: number }, config: BaseboardConfig): boolean {
  if (!config.enabled) return false;
  const poly = getBaseboardPolygon(config);
  if (poly.length < 3) return false;

  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const xi = poly[i]!.x;
    const yi = poly[i]!.y;
    const xj = poly[j]!.x;
    const yj = poly[j]!.y;
    const intersect = yi > point.y !== yj > point.y && point.x < ((xj - xi) * (point.y - yi)) / (yj - yi) + xi;
    if (intersect) inside = !inside;
  }
  return inside;
}

/** Berechnet die Bounding Box des Tisch-Header-Griffs (Verschiebegriff). */
export function getBaseboardHeaderBounds(config: BaseboardConfig): { x: number; y: number; width: number; height: number } {
  const b = getBaseboardBounds(config);
  return {
    x: b.x + 16,
    y: b.y + 16,
    width: 220,
    height: 32,
  };
}

/** Prüft, ob ein Klick auf den Verschiebegriff / Header des Tisches zielt. */
export function isPointInBaseboardHeader(point: { x: number; y: number }, config: BaseboardConfig): boolean {
  if (!config.enabled) return false;
  const h = getBaseboardHeaderBounds(config);
  return point.x >= h.x && point.x <= h.x + h.width && point.y >= h.y && point.y <= h.y + h.height;
}

/** Prüft, ob ein Klick nahe der Außenkante der Tischplatte liegt (Umleimer-Greifzone). */
export function isPointNearBaseboardEdge(point: { x: number; y: number }, config: BaseboardConfig, tolerance = 24): boolean {
  if (!config.enabled) return false;
  const poly = getBaseboardPolygon(config);
  if (poly.length < 3) return false;

  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const d = distToSegment(point, poly[j]!, poly[i]!);
    if (d <= tolerance) return true;
  }
  return false;
}
