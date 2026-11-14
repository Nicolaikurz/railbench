import type { Layout, Model3D, MountainBrushNode, MountainStyle } from "@/core/types";
import { isPointInsideModel } from "@/core/models-3d";
import { evaluateMountainLocal, worldToMountainLocal, mountainLocalToWorld } from "@/render/models-3d/mountain-tunnel";
import { getLanguage } from "@/i18n";

export type TerrainBrushMode = "paint" | "extend" | "stamp" | "erase";
export type TerrainRidgeProfile = "uniform" | "peak-center" | "varied";

/**
 * Prüft alle Gleise und Straßen im Layout gegen einen Berg und markiert
 * durchquerende Segmente automatisch mit `hasTunnel = true`.
 */
export function applyAutoTunnelForMountain(mountain: Model3D, layout: Layout): number {
  const mHeight = mountain.height ?? 220;
  const layerMap = new Map((layout.layers ?? []).map((l) => [l.id, l]));
  const mLayer = mountain.layerId ? layerMap.get(mountain.layerId) : undefined;
  const mBaseElev = (mLayer?.heightMm ?? 0) + (mountain.elevation ?? 0);

  let count = 0;
  for (const piece of layout.pieces) {
    const pLayer = piece.layerId ? layerMap.get(piece.layerId) : undefined;
    if (pLayer && pLayer.visible === false) continue;
    const baseH = pLayer?.heightMm ?? 0;
    const sRel = baseH + (piece.elevation?.start ?? 0) - mBaseElev;
    const eRel = baseH + (piece.elevation?.end ?? (piece.elevation?.start ?? 0)) - mBaseElev;
    if (Math.max(sRel, eRel) < -35 || Math.min(sRel, eRel) > mHeight - 10) continue;

    // Prüfe Transform-Ursprung, Anschlüsse und Zwischenpunkte
    const testPts: Array<{ x: number; y: number }> = [{ x: piece.transform.x, y: piece.transform.y }];
    for (const c of piece.connectors) {
      testPts.push({ x: c.x, y: c.y });
    }
    if (piece.connectors.length >= 2) {
      const c0 = piece.connectors[0]!;
      const c1 = piece.connectors[1]!;
      testPts.push({ x: (c0.x + c1.x) * 0.5, y: (c0.y + c1.y) * 0.5 });
      testPts.push({ x: c0.x * 0.75 + c1.x * 0.25, y: c0.y * 0.75 + c1.y * 0.25 });
      testPts.push({ x: c0.x * 0.25 + c1.x * 0.75, y: c0.y * 0.25 + c1.y * 0.75 });
    }

    const inside = testPts.some((pt) => {
      const { lx, lz } = worldToMountainLocal(pt.x, pt.y, mountain);
      return evaluateMountainLocal(lx, lz, mountain).normR <= 0.85;
    });

    if (inside) {
      piece.hasTunnel = true;
      piece.hasBridge = false;
      count++;
    }
  }
  return count;
}

/**
 * Markiert für alle Berge im Layout die durchquerenden Gleise und Straßen als Tunnel.
 */
export function applyAutoTunnelAllMountains(layout: Layout): number {
  const mountains = (layout.models ?? []).filter((m) => m.builtinId === "mountain");
  let total = 0;
  for (const m of mountains) {
    total += applyAutoTunnelForMountain(m, layout);
  }
  return total;
}

/**
 * Berechnet Zentrum, Breite, Tiefe und maximale Höhe eines Berges anhand seiner Welt-Knoten neu.
 */
export function recomputeMountainBoundsFromWorldNodes(
  mountain: Model3D,
  worldNodes: Array<{ x: number; y: number; radius: number; height: number }>,
): void {
  if (worldNodes.length === 0) return;
  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;
  let maxR = 60;
  let maxH = 40;

  for (const n of worldNodes) {
    if (n.x - n.radius < minX) minX = n.x - n.radius;
    if (n.x + n.radius > maxX) maxX = n.x + n.radius;
    if (n.y - n.radius < minY) minY = n.y - n.radius;
    if (n.y + n.radius > maxY) maxY = n.y + n.radius;
    if (n.radius > maxR) maxR = n.radius;
    if (n.height > maxH) maxH = n.height;
  }

  const cx = Math.round((minX + maxX) / 2);
  const cy = Math.round((minY + maxY) / 2);
  mountain.transform.x = cx;
  mountain.transform.y = cy;
  mountain.transform.rotation = 0;
  mountain.height = Math.round(maxH);

  if (worldNodes.length === 1) {
    const only = worldNodes[0]!;
    mountain.width = Math.round(only.radius * 2);
    mountain.depth = Math.round(only.radius * 1.75);
    mountain.mountainPath = undefined;
  } else {
    mountain.width = Math.round(Math.max(maxR * 2, maxX - minX + maxR * 0.25));
    mountain.depth = Math.round(Math.max(maxR * 2, maxY - minY + maxR * 0.25));
    mountain.mountainPath = worldNodes.map((n) => ({
      x: Math.round(n.x - cx),
      y: Math.round(n.y - cy),
      radius: Math.round(n.radius),
      height: Math.round(n.height),
    }));
  }
}

/**
 * Glättet den Freiform-Pfad (`mountainPath`) eines Berges für organischere Gebirgskämme.
 */
export function smoothMountainPath(mountain: Model3D): boolean {
  const path = mountain.mountainPath;
  if (!path || path.length < 3) return false;

  const worldNodes = path.map((n) => {
    const w = mountainLocalToWorld(n.x, n.y, mountain);
    return { x: w.wx, y: w.wy, radius: n.radius, height: n.height };
  });

  const smoothed = worldNodes.map((curr, idx) => {
    if (idx === 0 || idx === worldNodes.length - 1) return { ...curr };
    const prev = worldNodes[idx - 1]!;
    const next = worldNodes[idx + 1]!;
    return {
      x: Math.round(prev.x * 0.25 + curr.x * 0.5 + next.x * 0.25),
      y: Math.round(prev.y * 0.25 + curr.y * 0.5 + next.y * 0.25),
      radius: Math.round(prev.radius * 0.2 + curr.radius * 0.6 + next.radius * 0.2),
      height: Math.round(prev.height * 0.2 + curr.height * 0.6 + next.height * 0.2),
    };
  });

  recomputeMountainBoundsFromWorldNodes(mountain, smoothed);
  return true;
}

export class TerrainBrushController {
  public isActive = false;
  public mode: TerrainBrushMode = "paint";
  public brushRadius = 220;
  public brushHeight = 220;
  public brushStyle: MountainStyle = "alpine";
  public ridgeProfile: TerrainRidgeProfile = "uniform";
  public autoTunnel = true;
  public targetMountainId?: string;

  public isDrawing = false;
  public cursorWorld: { x: number; y: number } | null = null;
  public strokePoints: Array<{ x: number; y: number }> = [];

  private listeners = new Set<() => void>();

  public subscribe(cb: () => void): () => void {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  public notify(): void {
    for (const cb of this.listeners) cb();
  }

  public setActive(active: boolean, mode?: TerrainBrushMode, targetMountainId?: string): void {
    this.isActive = active;
    if (mode) this.mode = mode;
    if (targetMountainId !== undefined) this.targetMountainId = targetMountainId;
    if (!active) {
      this.isDrawing = false;
      this.strokePoints = [];
      this.cursorWorld = null;
    }
    this.notify();
  }

  public toggle(mode: TerrainBrushMode = "paint", targetMountainId?: string): void {
    if (this.isActive && this.mode === mode) {
      this.setActive(false);
    } else {
      this.setActive(true, mode, targetMountainId);
    }
  }

  public adjustBrushRadius(delta: number): void {
    this.brushRadius = Math.max(60, Math.min(600, Math.round(this.brushRadius + delta)));
    this.notify();
  }

  /**
   * Knoten-genauer & Ganzberg-Radierer:
   * - Hat ein Berg einen `mountainPath` mit mehreren Knoten, werden nur die getroffenen Knoten gelöscht
   *   und die Bounding-Box neu berechnet.
   * - Bleibt kein Knoten übrig oder ist es ein Einzelberg, wird der gesamte Berg entfernt.
   */
  public eraseMountainsAt(wPos: { x: number; y: number }, layout: Layout): boolean {
    if (!layout.models || layout.models.length === 0) return false;
    let changed = false;
    const eraseHitR = Math.max(40, this.brushRadius * 0.6);
    const remainingModels: Model3D[] = [];

    for (const m of layout.models) {
      if (m.builtinId !== "mountain") {
        remainingModels.push(m);
        continue;
      }

      if (m.mountainPath && m.mountainPath.length >= 2) {
        const worldNodes = m.mountainPath.map((n) => {
          const w = mountainLocalToWorld(n.x, n.y, m);
          return { x: w.wx, y: w.wy, radius: n.radius, height: n.height };
        });
        const kept = worldNodes.filter((n) => Math.hypot(wPos.x - n.x, wPos.y - n.y) > Math.max(eraseHitR, n.radius * 0.45));
        if (kept.length !== worldNodes.length) {
          changed = true;
          if (kept.length >= 1) {
            recomputeMountainBoundsFromWorldNodes(m, kept);
            remainingModels.push(m);
          }
          continue;
        }
      }

      if (isPointInsideModel(wPos, m) || Math.hypot(wPos.x - m.transform.x, wPos.y - m.transform.y) <= eraseHitR) {
        changed = true;
        continue;
      }

      remainingModels.push(m);
    }

    if (changed) {
      layout.models = remainingModels;
    }
    return changed;
  }

  public startStroke(wPos: { x: number; y: number }, layout: Layout): boolean {
    if (!this.isActive) return false;
    this.cursorWorld = { x: wPos.x, y: wPos.y };
    this.isDrawing = true;

    if (this.mode === "erase") {
      this.strokePoints = [];
      return this.eraseMountainsAt(wPos, layout);
    }

    if (this.mode === "stamp") {
      this.strokePoints = [{ x: Math.round(wPos.x), y: Math.round(wPos.y) }];
      return true;
    }

    this.strokePoints = [{ x: Math.round(wPos.x), y: Math.round(wPos.y) }];
    return true;
  }

  public moveStroke(wPos: { x: number; y: number }, layout: Layout): boolean {
    this.cursorWorld = { x: wPos.x, y: wPos.y };
    if (!this.isActive || !this.isDrawing) return false;

    if (this.mode === "erase") {
      return this.eraseMountainsAt(wPos, layout);
    }

    if (this.mode === "stamp") {
      this.strokePoints = [{ x: Math.round(wPos.x), y: Math.round(wPos.y) }];
      return true;
    }

    const last = this.strokePoints[this.strokePoints.length - 1];
    const minStep = Math.max(24, this.brushRadius * 0.22);
    if (!last || Math.hypot(wPos.x - last.x, wPos.y - last.y) >= minStep) {
      this.strokePoints.push({ x: Math.round(wPos.x), y: Math.round(wPos.y) });
      return true;
    }
    return false;
  }

  private computeNodeHeightAndRadius(idx: number, total: number, baseR: number, baseH: number): { radius: number; height: number } {
    if (total <= 2 || this.ridgeProfile === "uniform") {
      return { radius: baseR, height: baseH };
    }
    const t = idx / (total - 1); // 0..1
    if (this.ridgeProfile === "peak-center") {
      // Sanfter Anstieg von den Ausläufern (68%) zum Hauptgipfel in der Mitte (108%)
      const bell = Math.sin(t * Math.PI);
      return {
        radius: Math.round(baseR * (0.82 + 0.22 * bell)),
        height: Math.round(baseH * (0.68 + 0.38 * bell)),
      };
    }
    // "varied": Organisch wechselnde Alpengipfel entlang des Kamms
    const wave = Math.sin(idx * 1.9 + 0.7) * 0.16 + Math.cos(idx * 3.1) * 0.08;
    return {
      radius: Math.round(baseR * (1 + wave * 0.5)),
      height: Math.round(baseH * (1 + wave)),
    };
  }

  private findExtendTargetMountain(firstPt: { x: number; y: number }, layout: Layout): Model3D | undefined {
    const mountains = (layout.models ?? []).filter((m) => m.builtinId === "mountain");
    if (mountains.length === 0) return undefined;
    if (this.targetMountainId) {
      const explicit = mountains.find((m) => m.id === this.targetMountainId);
      if (explicit) return explicit;
    }
    // Nächstgelegenen Berg finden
    let best: Model3D | undefined;
    let bestDist = Infinity;
    for (const m of mountains) {
      const d = Math.hypot(firstPt.x - m.transform.x, firstPt.y - m.transform.y);
      if (d < bestDist) {
        bestDist = d;
        best = m;
      }
    }
    return best;
  }

  public finishStroke(layout: Layout, activeLayerId?: string): Model3D | null {
    if (!this.isDrawing) return null;
    this.isDrawing = false;

    if (this.mode === "erase" || this.strokePoints.length === 0) {
      this.strokePoints = [];
      this.notify();
      return null;
    }

    const pts = this.mode === "stamp" ? [this.strokePoints[this.strokePoints.length - 1]!] : this.strokePoints;
    this.strokePoints = [];

    const r = Math.max(60, Math.round(this.brushRadius));
    const h = Math.max(40, Math.round(this.brushHeight));
    if (!layout.models) layout.models = [];

    // Modus "extend": Bestehenden Berg um die neuen Pinsel-Knoten erweitern
    if (this.mode === "extend") {
      const target = this.findExtendTargetMountain(pts[0]!, layout);
      if (target) {
        const existingWorldNodes: Array<{ x: number; y: number; radius: number; height: number }> =
          target.mountainPath && target.mountainPath.length > 0
            ? target.mountainPath.map((n) => {
                const w = mountainLocalToWorld(n.x, n.y, target);
                return { x: w.wx, y: w.wy, radius: n.radius, height: n.height };
              })
            : [
                {
                  x: target.transform.x,
                  y: target.transform.y,
                  radius: Math.round(Math.max(target.width ?? 600, target.depth ?? 450) * 0.45),
                  height: target.height ?? h,
                },
              ];

        const addedNodes = pts.map((p, idx) => {
          const nh = this.computeNodeHeightAndRadius(idx, pts.length, r, h);
          return { x: Math.round(p.x), y: Math.round(p.y), radius: nh.radius, height: nh.height };
        });

        recomputeMountainBoundsFromWorldNodes(target, [...existingWorldNodes, ...addedNodes]);
        if (this.autoTunnel) {
          applyAutoTunnelForMountain(target, layout);
        }
        this.targetMountainId = target.id;
        this.notify();
        return target;
      }
    }

    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;
    for (const p of pts) {
      if (p.x < minX) minX = p.x;
      if (p.x > maxX) maxX = p.x;
      if (p.y < minY) minY = p.y;
      if (p.y > maxY) maxY = p.y;
    }

    const cx = Math.round((minX + maxX) / 2);
    const cy = Math.round((minY + maxY) / 2);

    const isDe = getLanguage() === "de";
    const existingCount = layout.models.filter((m) => m.builtinId === "mountain").length;

    let width = r * 2;
    let depth = Math.round(r * 1.75);
    let maxNodeH = h;
    let mountainPath: MountainBrushNode[] | undefined = undefined;

    if (pts.length >= 2) {
      width = Math.round(Math.max(r * 2, maxX - minX + r * 2.15));
      depth = Math.round(Math.max(r * 2, maxY - minY + r * 2.15));
      mountainPath = pts.map((p, idx) => {
        const nh = this.computeNodeHeightAndRadius(idx, pts.length, r, h);
        if (nh.height > maxNodeH) maxNodeH = nh.height;
        return {
          x: Math.round(p.x - cx),
          y: Math.round(p.y - cy),
          radius: nh.radius,
          height: nh.height,
        };
      });
    }

    const newMountain: Model3D = {
      id: `model-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      name:
        pts.length >= 2
          ? `${isDe ? "Bergkette" : "Mountain Ridge"} ${existingCount + 1}`
          : `${isDe ? "Berg" : "Mountain"} ${existingCount + 1}`,
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: cx, y: cy, rotation: 0 },
      width,
      depth,
      height: maxNodeH,
      mountainStyle: this.brushStyle,
      mountainPath,
      layerId: activeLayerId || layout.activeLayerId || layout.layers?.[0]?.id,
    };

    layout.models.push(newMountain);
    this.targetMountainId = newMountain.id;

    if (this.autoTunnel) {
      applyAutoTunnelForMountain(newMountain, layout);
    }

    this.notify();
    return newMountain;
  }
}

export const terrainBrush = new TerrainBrushController();
