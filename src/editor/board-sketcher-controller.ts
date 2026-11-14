import type { BaseboardConfig, Layer, LayerPlate, Layout, Vec2 } from "@/core/types";
import {
  addBaseboard,
  getBaseboardBounds,
  getBaseboardPolygon,
  getBaseboards,
  getLayerPlatePolygon,
} from "@/core/layout";
import { terrainBrush } from "./terrain-brush-controller";

export type SketchToolMode = "poly" | "rect" | "edit";

export type SketchTarget =
  | { kind: "table"; boardId?: string; createNew?: boolean }
  | { kind: "plate"; layerId: string; boardId: string; plateId?: string; createNew?: boolean };

export interface SketchGuideLine {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

/**
 * Applies a world-coordinate polygon (at least 3 vertices) to a BaseboardConfig.
 */
export function applyPolygonToTable(
  bb: BaseboardConfig,
  worldPts: Vec2[],
  asRect = false,
): void {
  if (worldPts.length < 3) return;
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (const p of worldPts) {
    if (p.x < minX) minX = p.x;
    if (p.x > maxX) maxX = p.x;
    if (p.y < minY) minY = p.y;
    if (p.y > maxY) maxY = p.y;
  }
  const w = Math.max(100, Math.round(maxX - minX));
  const d = Math.max(100, Math.round(maxY - minY));

  bb.enabled = true;
  bb.positionMode = "custom";
  bb.customX = Math.round(minX);
  bb.customY = Math.round(minY);
  bb.width = w;
  bb.depth = d;
  bb.rotation = 0;

  if (asRect) {
    bb.shape = "rect";
    bb.customPolygon = undefined;
  } else {
    bb.shape = "custom";
    bb.customPolygon = worldPts.map((p) => ({
      x: Math.round((p.x - minX) * 10) / 10,
      y: Math.round((p.y - minY) * 10) / 10,
    }));
  }
}

/**
 * Applies a world-coordinate polygon (at least 3 vertices) to a LayerPlate.
 */
export function applyPolygonToPlate(
  plate: LayerPlate,
  targetBoard: BaseboardConfig | undefined,
  worldPts: Vec2[],
  asRect = false,
): void {
  if (worldPts.length < 3) return;

  const tableBounds = targetBoard ? getBaseboardBounds(targetBoard) : { x: 0, y: 0, width: 2400, depth: 1200 };
  const rot = targetBoard?.rotation ?? 0;

  let localWorldPts = worldPts;
  if (rot !== 0 && targetBoard) {
    const cx = tableBounds.x + targetBoard.width / 2;
    const cy = tableBounds.y + targetBoard.depth / 2;
    const rad = (-rot * Math.PI) / 180;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    localWorldPts = worldPts.map((pt) => {
      const dx = pt.x - cx;
      const dy = pt.y - cy;
      return {
        x: cx + dx * cos - dy * sin,
        y: cy + dx * sin + dy * cos,
      };
    });
  }

  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (const p of localWorldPts) {
    if (p.x < minX) minX = p.x;
    if (p.x > maxX) maxX = p.x;
    if (p.y < minY) minY = p.y;
    if (p.y > maxY) maxY = p.y;
  }

  const w = Math.max(50, Math.round(maxX - minX));
  const d = Math.max(50, Math.round(maxY - minY));

  plate.positionMode = "custom";
  plate.x = Math.round(minX - tableBounds.x);
  plate.y = Math.round(minY - tableBounds.y);
  plate.width = w;
  plate.depth = d;
  plate.rotation = 0;

  if (asRect) {
    plate.shape = "rect";
    plate.customPolygon = undefined;
  } else {
    plate.shape = "custom";
    plate.customPolygon = localWorldPts.map((p) => ({
      x: Math.round((p.x - minX) * 10) / 10,
      y: Math.round((p.y - minY) * 10) / 10,
    }));
  }
}

export class BoardSketcherController {
  public isActive = false;
  public mode: SketchToolMode = "poly";
  public target: SketchTarget | null = null;

  public snapStepMm = 50;
  public orthoSnap = true;

  // Polygon / Rectangle drawing state
  public points: Vec2[] = [];
  public rectStart: Vec2 | null = null;
  public isDraggingRect = false;
  public cursorWorld: Vec2 | null = null;
  public snappedCursor: Vec2 | null = null;
  public isNearStartPoint = false;
  public activeGuides: SketchGuideLine[] = [];

  // Interactive vertex & edge editing state ("edit" mode)
  public editPoints: Vec2[] = [];
  public draggingVertexIndex: number | null = null;
  public hoveredVertexIndex: number | null = null;
  public hoveredMidpointIndex: number | null = null;
  private lastClickTime = 0;

  public onStateChange?: (committed?: boolean, createdBoardId?: string) => void;

  public startTableSketch(
    layout: Layout,
    mode: SketchToolMode,
    opts?: { boardId?: string; createNew?: boolean },
  ): void {
    if (terrainBrush.isActive) terrainBrush.setActive(false);
    this.isActive = true;
    this.mode = mode;
    const baseboards = getBaseboards(layout);
    const boardId = opts?.boardId ?? baseboards[0]?.id;
    this.target = {
      kind: "table",
      boardId,
      createNew: opts?.createNew ?? false,
    };
    this.resetDrawingState();
    if (mode === "edit") {
      const bb = baseboards.find((b) => b.id === boardId) ?? baseboards[0];
      if (bb) {
        this.editPoints = getBaseboardPolygon(bb).map((p) => ({ x: p.x, y: p.y }));
      }
    }
    this.onStateChange?.(false);
  }

  public startPlateSketch(
    layout: Layout,
    mode: SketchToolMode,
    opts: { layerId: string; boardId: string; plateId?: string; createNew?: boolean },
  ): void {
    if (terrainBrush.isActive) terrainBrush.setActive(false);
    this.isActive = true;
    this.mode = mode;
    this.target = {
      kind: "plate",
      layerId: opts.layerId,
      boardId: opts.boardId,
      plateId: opts.plateId,
      createNew: opts.createNew ?? !opts.plateId,
    };
    this.resetDrawingState();
    if (mode === "edit" && opts.plateId) {
      const layer = layout.layers?.find((l) => l.id === opts.layerId);
      const plate = layer?.plates?.find((p) => p.id === opts.plateId);
      const baseboards = getBaseboards(layout);
      const bb = baseboards.find((b) => b.id === (plate?.baseboardId || opts.boardId)) ?? baseboards[0];
      if (plate) {
        this.editPoints = getLayerPlatePolygon(plate, bb).map((p) => ({ x: p.x, y: p.y }));
      }
    }
    this.onStateChange?.(false);
  }

  public setMode(layout: Layout, mode: SketchToolMode): void {
    if (!this.isActive || !this.target) return;
    if (this.target.kind === "table") {
      this.startTableSketch(layout, mode, {
        boardId: this.target.boardId,
        createNew: mode === "edit" ? false : this.target.createNew,
      });
    } else {
      this.startPlateSketch(layout, mode, {
        layerId: this.target.layerId,
        boardId: this.target.boardId,
        plateId: this.target.plateId,
        createNew: mode === "edit" ? false : this.target.createNew,
      });
    }
  }

  public stop(): void {
    if (!this.isActive) return;
    this.isActive = false;
    this.target = null;
    this.resetDrawingState();
    this.onStateChange?.(false);
  }

  private resetDrawingState(): void {
    this.points = [];
    this.rectStart = null;
    this.isDraggingRect = false;
    this.activeGuides = [];
    this.isNearStartPoint = false;
    this.editPoints = [];
    this.draggingVertexIndex = null;
    this.hoveredVertexIndex = null;
    this.hoveredMidpointIndex = null;
  }

  /**
   * Computes grid-snapped and ortho-aligned world point.
   */
  public computeSnappedPoint(rawWorld: Vec2, refPoints: Vec2[], disableSnap = false): Vec2 {
    this.activeGuides = [];
    let x = rawWorld.x;
    let y = rawWorld.y;

    const step = disableSnap ? 0 : this.snapStepMm;
    if (step > 0) {
      x = Math.round(x / step) * step;
      y = Math.round(y / step) * step;
    } else {
      x = Math.round(x);
      y = Math.round(y);
    }

    if (this.orthoSnap && !disableSnap && refPoints.length > 0) {
      const tol = Math.max(25, step * 0.8);
      const last = refPoints[refPoints.length - 1]!;
      const first = refPoints[0]!;

      if (Math.abs(x - last.x) <= tol) {
        x = last.x;
        this.activeGuides.push({ x1: last.x, y1: last.y, x2: x, y2: y });
      } else if (refPoints.length >= 2 && Math.abs(x - first.x) <= tol) {
        x = first.x;
        this.activeGuides.push({ x1: first.x, y1: first.y, x2: x, y2: y });
      }

      if (Math.abs(y - last.y) <= tol) {
        y = last.y;
        this.activeGuides.push({ x1: last.x, y1: last.y, x2: x, y2: y });
      } else if (refPoints.length >= 2 && Math.abs(y - first.y) <= tol) {
        y = first.y;
        this.activeGuides.push({ x1: first.x, y1: first.y, x2: x, y2: y });
      }
    }

    return { x, y };
  }

  public onPointerDown(
    wPos: Vec2,
    layout: Layout,
    scale: number,
    button: number,
    altKey = false,
  ): boolean {
    if (!this.isActive || !this.target) return false;

    const hitRadius = Math.max(22, 16 / Math.max(0.15, scale));

    // Right click: delete vertex in edit mode, or undo/exit in poly/rect mode
    if (button === 2) {
      if (this.mode === "edit") {
        const vIdx = this.findVertexAt(wPos, hitRadius);
        if (vIdx !== null && this.editPoints.length > 3) {
          this.editPoints.splice(vIdx, 1);
          this.commitPointsToTarget(layout, this.editPoints, false, false);
          this.onStateChange?.(true);
          return true;
        }
        this.stop();
        return true;
      }
      if (this.mode === "poly" && this.points.length > 0) {
        this.points.pop();
        this.onStateChange?.(false);
        return true;
      }
      this.stop();
      return true;
    }

    if (button !== 0) return false;

    const now = Date.now();
    const isDoubleClick = now - this.lastClickTime < 300;
    this.lastClickTime = now;

    if (this.mode === "edit") {
      // Alt+Click on a vertex deletes it (if > 3 vertices)
      const vIdx = this.findVertexAt(wPos, hitRadius);
      if (vIdx !== null) {
        if (altKey && this.editPoints.length > 3) {
          this.editPoints.splice(vIdx, 1);
          this.commitPointsToTarget(layout, this.editPoints, false, false);
          this.onStateChange?.(true);
          return true;
        }
        this.draggingVertexIndex = vIdx;
        return true;
      }

      // Click on an edge midpoint [+] inserts a new vertex and starts dragging it
      const midIdx = this.findMidpointAt(wPos, hitRadius);
      if (midIdx !== null) {
        const p1 = this.editPoints[midIdx]!;
        const p2 = this.editPoints[(midIdx + 1) % this.editPoints.length]!;
        const midPt = { x: Math.round((p1.x + p2.x) / 2), y: Math.round((p1.y + p2.y) / 2) };
        this.editPoints.splice(midIdx + 1, 0, midPt);
        this.draggingVertexIndex = midIdx + 1;
        this.commitPointsToTarget(layout, this.editPoints, false, false);
        return true;
      }
      return true;
    }

    if (this.mode === "rect") {
      const snapped = this.computeSnappedPoint(wPos, [], altKey);
      if (!this.rectStart) {
        this.rectStart = snapped;
        this.snappedCursor = snapped;
        this.isDraggingRect = true;
      } else {
        this.finishRect(layout, this.rectStart, snapped);
      }
      return true;
    }

    // "poly" mode
    const snapped = this.computeSnappedPoint(wPos, this.points, altKey);
    const last = this.points[this.points.length - 1];
    if (this.points.length >= 3) {
      const first = this.points[0]!;
      const nearLast = Boolean(last && Math.hypot(wPos.x - last.x, wPos.y - last.y) <= hitRadius);
      if (Math.hypot(wPos.x - first.x, wPos.y - first.y) <= hitRadius || (isDoubleClick && nearLast)) {
        this.finishPoly(layout);
        return true;
      }
    }

    if (!last || Math.hypot(snapped.x - last.x, snapped.y - last.y) > 5) {
      this.points.push(snapped);
    }
    return true;
  }

  public onPointerMove(wPos: Vec2, layout: Layout, scale: number, altKey = false): void {
    if (!this.isActive) return;
    this.cursorWorld = { ...wPos };
    const hitRadius = Math.max(22, 16 / Math.max(0.15, scale));

    if (this.mode === "edit") {
      if (this.draggingVertexIndex !== null && this.editPoints[this.draggingVertexIndex]) {
        const neighbors: Vec2[] = [];
        const n = this.editPoints.length;
        if (n >= 2) {
          neighbors.push(this.editPoints[(this.draggingVertexIndex - 1 + n) % n]!);
          neighbors.push(this.editPoints[(this.draggingVertexIndex + 1) % n]!);
        }
        const snapped = this.computeSnappedPoint(wPos, neighbors, altKey);
        this.editPoints[this.draggingVertexIndex] = snapped;
        this.snappedCursor = snapped;
        this.commitPointsToTarget(layout, this.editPoints, false, false);
      } else {
        this.hoveredVertexIndex = this.findVertexAt(wPos, hitRadius);
        this.hoveredMidpointIndex =
          this.hoveredVertexIndex === null ? this.findMidpointAt(wPos, hitRadius) : null;
        this.snappedCursor = { x: Math.round(wPos.x), y: Math.round(wPos.y) };
        this.activeGuides = [];
      }
      return;
    }

    if (this.mode === "rect") {
      this.snappedCursor = this.computeSnappedPoint(
        wPos,
        this.rectStart ? [this.rectStart] : [],
        altKey,
      );
      return;
    }

    // "poly" mode
    this.snappedCursor = this.computeSnappedPoint(wPos, this.points, altKey);
    if (this.points.length >= 3) {
      const first = this.points[0]!;
      this.isNearStartPoint = Math.hypot(wPos.x - first.x, wPos.y - first.y) <= hitRadius;
      if (this.isNearStartPoint) {
        this.snappedCursor = { ...first };
      }
    } else {
      this.isNearStartPoint = false;
    }
  }

  public onPointerUp(wPos: Vec2, layout: Layout, altKey = false): boolean {
    if (!this.isActive) return false;

    if (this.mode === "edit" && this.draggingVertexIndex !== null) {
      this.draggingVertexIndex = null;
      this.activeGuides = [];
      this.onStateChange?.(true);
      return true;
    }

    if (this.mode === "rect" && this.isDraggingRect && this.rectStart) {
      const snapped = this.computeSnappedPoint(wPos, [this.rectStart], altKey);
      const dx = Math.abs(snapped.x - this.rectStart.x);
      const dy = Math.abs(snapped.y - this.rectStart.y);
      this.isDraggingRect = false;
      if (dx >= 80 && dy >= 80) {
        this.finishRect(layout, this.rectStart, snapped);
        return true;
      }
    }
    return false;
  }

  public undoLastPoint(): boolean {
    if (!this.isActive || this.mode !== "poly" || this.points.length === 0) return false;
    this.points.pop();
    this.onStateChange?.(false);
    return true;
  }

  public finishPoly(layout: Layout): boolean {
    if (!this.isActive || !this.target || this.points.length < 3) return false;
    const pts = [...this.points];
    const createdBoardId = this.commitPointsToTarget(layout, pts, false, true);
    this.isActive = false;
    this.target = null;
    this.resetDrawingState();
    this.onStateChange?.(true, createdBoardId);
    return true;
  }

  private finishRect(layout: Layout, p1: Vec2, p2: Vec2): boolean {
    const minX = Math.min(p1.x, p2.x);
    const maxX = Math.max(p1.x, p2.x);
    const minY = Math.min(p1.y, p2.y);
    const maxY = Math.max(p1.y, p2.y);
    if (maxX - minX < 50 || maxY - minY < 50) return false;

    const rectPts: Vec2[] = [
      { x: minX, y: minY },
      { x: maxX, y: minY },
      { x: maxX, y: maxY },
      { x: minX, y: maxY },
    ];
    const createdBoardId = this.commitPointsToTarget(layout, rectPts, true, true);
    this.isActive = false;
    this.target = null;
    this.resetDrawingState();
    this.onStateChange?.(true, createdBoardId);
    return true;
  }

  private commitPointsToTarget(
    layout: Layout,
    worldPts: Vec2[],
    asRect: boolean,
    allowCreate: boolean,
  ): string | undefined {
    if (!this.target || worldPts.length < 3) return undefined;
    const baseboards = getBaseboards(layout);

    if (this.target.kind === "table") {
      let bb: BaseboardConfig | undefined;
      if (this.target.createNew && allowCreate) {
        bb = addBaseboard(layout);
        this.target.boardId = bb.id;
        this.target.createNew = false;
      } else {
        bb = baseboards.find((b) => b.id === this.target!.boardId) ?? baseboards[0];
      }
      if (bb) {
        applyPolygonToTable(bb, worldPts, asRect);
        return bb.id;
      }
    } else if (this.target.kind === "plate") {
      const layer: Layer | undefined = layout.layers?.find((l) => l.id === (this.target as any).layerId);
      if (!layer) return undefined;
      const targetBoard =
        baseboards.find((b) => b.id === (this.target as any).boardId) ?? baseboards[0];
      let plate = layer.plates?.find((p) => p.id === (this.target as any).plateId);

      if (!plate && allowCreate) {
        const nextNum = (layer.plates?.length ?? 0) + 1;
        plate = {
          id: `plate-${Date.now()}-${Math.floor(Math.random() * 100000)}`,
          name: `Platte ${nextNum}`,
          baseboardId: targetBoard?.id,
          width: 1000,
          depth: 600,
          x: 0,
          y: 0,
          thicknessMm: layer.woodPlateThicknessMm ?? 12,
          positionMode: "custom",
          style: targetBoard?.style ?? "wood",
        };
        (layer.plates ??= []).push(plate);
        this.target.plateId = plate.id;
        this.target.createNew = false;
        if ((layer.heightMm ?? 0) !== 0) {
          layer.woodPlateMode = "table";
        }
      }
      if (plate) {
        applyPolygonToPlate(plate, targetBoard, worldPts, asRect);
      }
    }
    return undefined;
  }

  private findVertexAt(wPos: Vec2, radius: number): number | null {
    for (let i = 0; i < this.editPoints.length; i++) {
      const p = this.editPoints[i]!;
      if (Math.hypot(wPos.x - p.x, wPos.y - p.y) <= radius) {
        return i;
      }
    }
    return null;
  }

  private findMidpointAt(wPos: Vec2, radius: number): number | null {
    const n = this.editPoints.length;
    if (n < 3) return null;
    for (let i = 0; i < n; i++) {
      const p1 = this.editPoints[i]!;
      const p2 = this.editPoints[(i + 1) % n]!;
      const mx = (p1.x + p2.x) / 2;
      const my = (p1.y + p2.y) / 2;
      if (Math.hypot(wPos.x - mx, wPos.y - my) <= radius) {
        return i;
      }
    }
    return null;
  }
}

export const boardSketcher = new BoardSketcherController();
