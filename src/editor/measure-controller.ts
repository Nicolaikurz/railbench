import type { Layout, TrackPiece } from "@/core/types";
import { getBaseboards, getBaseboardPolygon } from "@/core/layout";
import { MeasureHud } from "@/ui/measure-hud";

export interface MeasureControllerDeps {
  getLayout: () => Layout;
  getSnappingEnabled: () => boolean;
  isPieceLockedOrHidden: (piece: TrackPiece) => boolean;
  getCanvas: () => HTMLCanvasElement;
  sync: () => void;
}

export class MeasureController {
  public isActive = false;
  public start: { x: number; y: number } | null = null;
  public current: { x: number; y: number } | null = null;
  public locked = false;
  public snapPoint: { x: number; y: number } | null = null;
  public snapEnabled = true;
  public draggingPoint: "A" | "B" | null = null;

  public hud: MeasureHud;

  constructor(private deps: MeasureControllerDeps) {
    this.hud = new MeasureHud({
      onToggleSnap: (enabled) => {
        this.snapEnabled = enabled;
        this.updateHud();
      },
      onReset: () => {
        this.start = null;
        this.current = null;
        this.locked = false;
        this.draggingPoint = null;
        this.updateHud();
        this.deps.sync();
      },
      onClose: () => {
        this.toggle(false);
      },
      onManualCoordsChange: ({ ax, ay, bx, by }) => {
        if (ax !== undefined && ay !== undefined) {
          this.start = { x: ax, y: ay };
        }
        if (bx !== undefined && by !== undefined) {
          this.current = { x: bx, y: by };
          this.locked = true;
        }
        this.updateHud(false);
        this.deps.sync();
      },
    });
  }

  public findSnapPoint(
    pos: { x: number; y: number },
    snapDist = 20,
    forceNoSnap = false,
  ): { x: number; y: number } | null {
    if (!this.deps.getSnappingEnabled() || !this.snapEnabled || forceNoSnap) return null;
    let closestPt: { x: number; y: number } | null = null;
    let closestD = snapDist;
    const layout = this.deps.getLayout();

    for (const piece of layout.pieces) {
      if (this.deps.isPieceLockedOrHidden(piece)) continue;
      for (const conn of piece.connectors) {
        const d = Math.hypot(pos.x - conn.x, pos.y - conn.y);
        if (d < closestD) {
          closestD = d;
          closestPt = { x: conn.x, y: conn.y };
        }
      }
    }

    const baseboards = getBaseboards(layout);
    for (const bb of baseboards) {
      if (!bb.enabled) continue;
      const poly = getBaseboardPolygon(bb);
      for (const pt of poly) {
        const d = Math.hypot(pos.x - pt.x, pos.y - pt.y);
        if (d < closestD) {
          closestD = d;
          closestPt = { x: pt.x, y: pt.y };
        }
      }
    }

    return closestPt;
  }

  public updateHud(updateInputs = true) {
    this.hud.update(
      {
        isActive: this.isActive,
        start: this.start,
        current: this.current,
        locked: this.locked,
        snapEnabled: this.snapEnabled,
        scale: this.deps.getLayout().scale,
      },
      updateInputs,
    );
  }

  public toggle(force?: boolean) {
    this.isActive = force !== undefined ? force : !this.isActive;
    const canvas = this.deps.getCanvas();
    if (!this.isActive) {
      this.start = null;
      this.current = null;
      this.locked = false;
      this.draggingPoint = null;
      this.snapPoint = null;
      if (canvas) canvas.style.cursor = "default";
    } else {
      if (canvas) canvas.style.cursor = "crosshair";
    }
    this.updateHud();
    this.deps.sync();
  }
}
