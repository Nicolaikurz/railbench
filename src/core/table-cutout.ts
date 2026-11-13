import type { BaseboardConfig, Layout, TrackPiece, Layer } from "@/core/types";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { getBaseboardBounds, getBaseboardPolygon, isPointInsideBaseboard } from "@/core/layout";
import { deg2rad, dist, isCurvedTrackEntry, isStraightTrackEntry, normAngle, transformPoint } from "@/core/geometry";
import { getMinClearanceHeightForScale } from "@/core/ramp-generator";
import { getPieceConnectorHeight } from "@/core/snapping";

export interface TrackCutoutPolygon {
  points: Array<{ x: number; y: number }>;
  pieceIds: string[];
  depthRange: { min: number; max: number };
  widthMm: number;
}

/**
 * Returns the clearance corridor width (NEM 102 loading gauge + safety margin) for a track entry.
 */
export function getTrackClearanceWidth(entry?: CatalogEntry): number {
  if (!entry) return 54;
  if (entry.isRoad) return (entry.roadWidth ?? 60) + 16;
  const gauge = entry.gauge ?? 16.5;
  // NEM 102 clearance width + buffer (~53 mm in H0, ~30 mm in N, ~102 mm in Spur 0)
  return Math.max(26, Math.round(gauge * 3.2));
}

/**
 * Checks if a track piece penetrates through the table or runs under the table in the collision zone.
 */
export function isPiecePassingThroughTable(
  piece: TrackPiece,
  layout: Layout,
  catalog: Catalog,
  minClearance?: number,
  deckElevation = 0,
  deckThickness = 35,
): { passesThrough: boolean; minElev: number; maxElev: number } {
  const layer = layout.layers?.find((l) => l.id === piece.layerId);
  if (layer && !layer.visible) {
    return { passesThrough: false, minElev: 0, maxElev: 0 };
  }

  const baseH = layer?.heightMm ?? 0;
  const startH = baseH + (piece.elevation?.start ?? 0);
  const endH = baseH + (piece.elevation?.end ?? 0);
  const minElev = Math.min(startH, endH);
  const maxElev = Math.max(startH, endH);

  const clearance = minClearance ?? getMinClearanceHeightForScale(layout.scale || catalog.scale);

  // Track penetrates below deck top surface
  const goesBelow = minElev < deckElevation - 0.1;
  // Flat underground tracks below the physical table deck (e.g. shadow station / tunnel at -80mm or -100mm)
  // run underneath the table and must not cut open the tabletop above them.
  const isSloped = Math.abs(maxElev - minElev) >= 0.5;
  const intersectsDeckSlab = maxElev > deckElevation - deckThickness;
  // Ramp clearance zone extends from deckElevation down to deckElevation - clearance
  // (clearance already includes track bed & deck thickness per MIN_CLEARANCE_HEIGHT_BY_SCALE).
  const inCollisionZone =
    maxElev > deckElevation - clearance + 0.1 && (isSloped || intersectsDeckSlab);

  return {
    passesThrough: goesBelow && inCollisionZone,
    minElev,
    maxElev,
  };
}

/**
 * Samples 2D world points along the centerline of a track piece from one connector to another.
 */
export function getPieceCenterlinePoints(
  piece: TrackPiece,
  entry: CatalogEntry | undefined,
  fromConnIdx: number,
  toConnIdx: number,
  steps = 8,
  tStart = 0,
  tEnd = 1,
): Array<{ x: number; y: number }> {
  const conns = piece.connectors || [];
  const cFrom = conns[fromConnIdx] || conns[0] || { x: piece.transform.x, y: piece.transform.y, angle: 0 };
  const cTo = conns[toConnIdx] || conns[conns.length - 1] || cFrom;
  const t0 = Math.max(0, Math.min(1, tStart));
  const t1 = Math.max(t0, Math.min(1, tEnd));

  if (!entry || isStraightTrackEntry(entry)) {
    // Linear straight segment
    const pts: Array<{ x: number; y: number }> = [];
    const numSubsteps = Math.max(2, Math.round((dist(cFrom, cTo) * Math.max(0.25, t1 - t0)) / 40));
    for (let i = 0; i <= numSubsteps; i++) {
      const t = t0 + (t1 - t0) * (i / numSubsteps);
      pts.push({
        x: cFrom.x + (cTo.x - cFrom.x) * t,
        y: cFrom.y + (cTo.y - cFrom.y) * t,
      });
    }
    return pts;
  }

  if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const isReverse = fromConnIdx === 1 && toConnIdx === 0;

    const pts: Array<{ x: number; y: number }> = [];
    for (let s = 0; s <= steps; s++) {
      const progress = t0 + (t1 - t0) * (s / steps);
      const t = isReverse ? 1 - progress : progress;
      const ang = a * t;
      const rad = deg2rad(Math.abs(ang));

      const localX = r * Math.sin(rad);
      const localY = a < 0 ? -r * (1 - Math.cos(rad)) : r * (1 - Math.cos(rad));
      pts.push(transformPoint({ x: localX, y: localY }, piece.transform));
    }
    return pts;
  }

  // Turnout or other complex shape: straight chord
  const lerpPt = (u: number) => ({
    x: cFrom.x + (cTo.x - cFrom.x) * u,
    y: cFrom.y + (cTo.y - cFrom.y) * u,
  });
  return [lerpPt(t0), lerpPt((t0 + t1) / 2), lerpPt(t1)];
}

interface PieceEdge {
  neighborId: string;
  myConn: number;
  neighborConn: number;
}

/**
 * Groups connected track pieces that need an outbreak into continuous ordered chains.
 */
export function groupConnectedPieces(
  pieces: TrackPiece[],
  layers?: Layer[],
): Array<Array<{ piece: TrackPiece; fromConn: number; toConn: number }>> {
  if (pieces.length === 0) return [];

  const pieceSet = new Set(pieces.map((p) => p.id));
  const pieceMap = new Map<string, TrackPiece>();
  for (const p of pieces) pieceMap.set(p.id, p);

  // Adjacency graph
  const adj = new Map<string, PieceEdge[]>();
  for (const p of pieces) adj.set(p.id, []);

  for (let i = 0; i < pieces.length; i++) {
    const pA = pieces[i]!;
    for (let j = i + 1; j < pieces.length; j++) {
      const pB = pieces[j]!;
      for (let cA = 0; cA < pA.connectors.length; cA++) {
        const connA = pA.connectors[cA]!;
        const hA = getPieceConnectorHeight(pA, cA, layers);
        for (let cB = 0; cB < pB.connectors.length; cB++) {
          const connB = pB.connectors[cB]!;
          const hB = getPieceConnectorHeight(pB, cB, layers);
          if (dist(connA, connB) <= 8.0 && Math.abs(hA - hB) <= 5.0) {
            adj.get(pA.id)!.push({ neighborId: pB.id, myConn: cA, neighborConn: cB });
            adj.get(pB.id)!.push({ neighborId: pA.id, myConn: cB, neighborConn: cA });
          }
        }
      }
    }
  }

  const visited = new Set<string>();
  const chains: Array<Array<{ piece: TrackPiece; fromConn: number; toConn: number }>> = [];

  for (const startPiece of pieces) {
    if (visited.has(startPiece.id)) continue;

    // Find all pieces in this connected component
    const componentPieces: TrackPiece[] = [];
    const queue = [startPiece.id];
    visited.add(startPiece.id);

    while (queue.length > 0) {
      const curId = queue.shift()!;
      componentPieces.push(pieceMap.get(curId)!);
      for (const edge of adj.get(curId) || []) {
        if (!visited.has(edge.neighborId) && pieceSet.has(edge.neighborId)) {
          visited.add(edge.neighborId);
          queue.push(edge.neighborId);
        }
      }
    }

    if (componentPieces.length === 1) {
      chains.push([{ piece: componentPieces[0]!, fromConn: 0, toConn: 1 }]);
      continue;
    }

    // Find an endpoint piece (degree <= 1 in this component)
    const endPiece = componentPieces.find((p) => (adj.get(p.id)?.filter((e) => pieceSet.has(e.neighborId)).length ?? 0) <= 1) || componentPieces[0]!;

    // Walk linearly from endPiece
    const ordered: Array<{ piece: TrackPiece; fromConn: number; toConn: number }> = [];
    const walkVisited = new Set<string>();
    let currentPiece: TrackPiece | undefined = endPiece;
    let prevExitConn: number | undefined = undefined;

    while (currentPiece && !walkVisited.has(currentPiece.id)) {
      walkVisited.add(currentPiece.id);
      const pieceEdges: PieceEdge[] = adj.get(currentPiece.id) ?? [];
      const edges: PieceEdge[] = pieceEdges.filter((e: PieceEdge) => pieceSet.has(e.neighborId) && !walkVisited.has(e.neighborId));

      let fromConn = 0;
      let toConn = 1;

      if (prevExitConn !== undefined) {
        // Find which connector was closest to previous exit
        // We find the edge that brought us here
        const parentEdges: PieceEdge[] = pieceEdges.filter((e: PieceEdge) => walkVisited.has(e.neighborId));
        if (parentEdges.length > 0) {
          fromConn = parentEdges[0]!.myConn;
          toConn = fromConn === 0 ? 1 : 0;
        }
      } else {
        // First piece
        if (edges.length > 0) {
          toConn = edges[0]!.myConn;
          fromConn = toConn === 0 ? 1 : 0;
        }
      }

      ordered.push({ piece: currentPiece, fromConn, toConn });
      prevExitConn = toConn;
      currentPiece = edges.length > 0 ? pieceMap.get(edges[0]!.neighborId) : undefined;
    }

    chains.push(ordered);
  }

  return chains;
}

/**
 * Computes all breakthrough (cutout) opening polygons in the tabletop where tracks pass through or under the desk.
 */
export function computeTableTrackCutouts(
  layout: Layout,
  catalog: Catalog,
  baseboard: BaseboardConfig,
  deckElevation = 0,
  deckThickness = 35,
): TrackCutoutPolygon[] {
  if (!baseboard.enabled || !layout.pieces || layout.pieces.length === 0) {
    return [];
  }

  const baseboardPoly = getBaseboardPolygon(baseboard);
  if (baseboardPoly.length < 3) return [];
  let polyMinX = Infinity, polyMaxX = -Infinity, polyMinY = Infinity, polyMaxY = -Infinity;
  for (const p of baseboardPoly) {
    if (p.x < polyMinX) polyMinX = p.x;
    if (p.x > polyMaxX) polyMaxX = p.x;
    if (p.y < polyMinY) polyMinY = p.y;
    if (p.y > polyMaxY) polyMaxY = p.y;
  }
  const baseboardBounds = {
    x: polyMinX,
    y: polyMinY,
    width: polyMaxX - polyMinX,
    depth: polyMaxY - polyMinY,
  };

  const entryMap = new Map<string, CatalogEntry>();
  for (const e of catalog.entries) entryMap.set(e.id, e);

  // 1. Filter pieces going through or under the table
  const qualifyingPieces: TrackPiece[] = [];
  const minElevMap = new Map<string, number>();
  const maxElevMap = new Map<string, number>();

  for (const piece of layout.pieces) {
    const { passesThrough, minElev, maxElev } = isPiecePassingThroughTable(
      piece,
      layout,
      catalog,
      undefined,
      deckElevation,
      deckThickness,
    );
    if (!passesThrough) continue;

    // Check if piece is in or near this baseboard
    const pt = { x: piece.transform.x, y: piece.transform.y };
    const nearBaseboard =
      isPointInsideBaseboard(pt, baseboard) ||
      (piece.connectors && piece.connectors.some((c) => isPointInsideBaseboard(c, baseboard))) ||
      (pt.x >= baseboardBounds.x - 50 &&
        pt.x <= baseboardBounds.x + baseboardBounds.width + 50 &&
        pt.y >= baseboardBounds.y - 50 &&
        pt.y <= baseboardBounds.y + baseboardBounds.depth + 50);

    if (nearBaseboard) {
      qualifyingPieces.push(piece);
      minElevMap.set(piece.id, minElev);
      maxElevMap.set(piece.id, maxElev);
    }
  }

  if (qualifyingPieces.length === 0) return [];

  // 2. Group into continuous chains
  const clearance = getMinClearanceHeightForScale(layout.scale || catalog.scale);
  const minCutElev = deckElevation - clearance;
  const maxCutElev = deckElevation;
  const chains = groupConnectedPieces(qualifyingPieces, layout.layers);
  const cutouts: TrackCutoutPolygon[] = [];

  for (const chain of chains) {
    if (chain.length === 0) continue;

    // Collect centerline points
    const fullCenterline: Array<{ x: number; y: number }> = [];
    let overallMinElev = Infinity;
    let overallMaxElev = -Infinity;
    let maxCorridorWidth = 0;
    const pieceIds: string[] = [];

    for (let cIdx = 0; cIdx < chain.length; cIdx++) {
      const item = chain[cIdx]!;
      const piece = item.piece;
      pieceIds.push(piece.id);

      const pMinH = Math.max(minCutElev, minElevMap.get(piece.id) ?? 0);
      const pMaxH = Math.min(maxCutElev, maxElevMap.get(piece.id) ?? 0);
      if (pMinH < overallMinElev) overallMinElev = pMinH;
      if (pMaxH > overallMaxElev) overallMaxElev = pMaxH;

      const entry = entryMap.get(piece.catalogId) ?? catalog.entries.find((e) => e.id === piece.catalogId);
      const pW = getTrackClearanceWidth(entry);
      if (pW > maxCorridorWidth) maxCorridorWidth = pW;

      // Clip segment to the breakthrough elevation band [deckElevation - clearance, deckElevation]
      const layer = layout.layers?.find((l) => l.id === piece.layerId);
      const baseH = layer?.heightMm ?? 0;
      const h0 = baseH + (piece.elevation?.start ?? 0);
      const h1 = baseH + (piece.elevation?.end ?? 0);
      const hFrom = item.fromConn === 0 ? h0 : h1;
      const hTo = item.toConn === 0 ? h0 : h1;
      let tStart = 0;
      let tEnd = 1;
      const dH = hTo - hFrom;
      if (Math.abs(dH) >= 0.5) {
        const tTop = (maxCutElev - hFrom) / dH;
        const tBot = (minCutElev - hFrom) / dH;
        tStart = Math.max(0, Math.min(tTop, tBot));
        tEnd = Math.min(1, Math.max(tTop, tBot));
      }

      if (tEnd <= tStart + 0.01) continue;

      const pCenterPts = getPieceCenterlinePoints(piece, entry, item.fromConn, item.toConn, 8, tStart, tEnd);
      for (let k = 0; k < pCenterPts.length; k++) {
        const pt = pCenterPts[k]!;
        if (
          fullCenterline.length === 0 ||
          dist(pt, fullCenterline[fullCenterline.length - 1]!) >= 1.0
        ) {
          fullCenterline.push(pt);
        }
      }
    }

    if (fullCenterline.length < 2) continue;

    // 3. Compute ribbon boundary points
    const halfWidth = maxCorridorWidth / 2;
    const leftPts: Array<{ x: number; y: number }> = [];
    const rightPts: Array<{ x: number; y: number }> = [];
    const M = fullCenterline.length;

    for (let i = 0; i < M; i++) {
      const cur = fullCenterline[i]!;
      let tx: number;
      let ty: number;

      if (i === 0) {
        tx = fullCenterline[1]!.x - cur.x;
        ty = fullCenterline[1]!.y - cur.y;
      } else if (i === M - 1) {
        tx = cur.x - fullCenterline[M - 2]!.x;
        ty = cur.y - fullCenterline[M - 2]!.y;
      } else {
        tx = fullCenterline[i + 1]!.x - fullCenterline[i - 1]!.x;
        ty = fullCenterline[i + 1]!.y - fullCenterline[i - 1]!.y;
      }

      const len = Math.hypot(tx, ty) || 1;
      const nx = -ty / len;
      const ny = tx / len;

      leftPts.push({
        x: cur.x + nx * halfWidth,
        y: cur.y + ny * halfWidth,
      });
      rightPts.push({
        x: cur.x - nx * halfWidth,
        y: cur.y - ny * halfWidth,
      });
    }

    // Assemble closed polygon: left points forward, then right points backward
    const rawPolygon: Array<{ x: number; y: number }> = [...leftPts, ...rightPts.reverse()];

    // Clamp vertices to stay within baseboard bounding bounds (with 1.5mm inset so holes are strictly internal)
    const inset = 1.5;
    const bMinX = baseboardBounds.x + inset;
    const bMaxX = baseboardBounds.x + baseboardBounds.width - inset;
    const bMinY = baseboardBounds.y + inset;
    const bMaxY = baseboardBounds.y + baseboardBounds.depth - inset;

    const clampedPolygon = rawPolygon.map((p) => ({
      x: Math.max(bMinX, Math.min(bMaxX, Math.round(p.x * 10) / 10)),
      y: Math.max(bMinY, Math.min(bMaxY, Math.round(p.y * 10) / 10)),
    }));

    // Deduplicate consecutive points to prevent zero-length edges
    const dedupedPolygon: Array<{ x: number; y: number }> = [];
    for (const p of clampedPolygon) {
      if (
        dedupedPolygon.length === 0 ||
        dist(p, dedupedPolygon[dedupedPolygon.length - 1]!) > 0.5
      ) {
        dedupedPolygon.push(p);
      }
    }
    if (
      dedupedPolygon.length > 2 &&
      dist(dedupedPolygon[0]!, dedupedPolygon[dedupedPolygon.length - 1]!) <= 0.5
    ) {
      dedupedPolygon.pop();
    }

    if (dedupedPolygon.length < 3) continue;

    cutouts.push({
      points: dedupedPolygon,
      pieceIds,
      depthRange: { min: overallMinElev, max: overallMaxElev },
      widthMm: maxCorridorWidth,
    });
  }

  return cutouts;
}

/**
 * Checks if a point (e.g. table leg) is inside or too close to a cutout polygon.
 */
export function isPointInsideCutout(
  point: { x: number; y: number },
  cutouts: TrackCutoutPolygon[],
  safetyBuffer = 15,
): boolean {
  for (const c of cutouts) {
    const pts = c.points;
    if (pts.length < 3) continue;

    // Point in polygon test
    let inside = false;
    for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
      const xi = pts[i]!.x;
      const yi = pts[i]!.y;
      const xj = pts[j]!.x;
      const yj = pts[j]!.y;
      const intersect = yi > point.y !== yj > point.y && point.x < ((xj - xi) * (point.y - yi)) / (yj - yi) + xi;
      if (intersect) inside = !inside;
    }
    if (inside) return true;

    // Check distance to edges
    for (let i = 0; i < pts.length; i++) {
      const p1 = pts[i]!;
      const p2 = pts[(i + 1) % pts.length]!;
      const d = distToSegment(point, p1, p2);
      if (d <= safetyBuffer) return true;
    }
  }
  return false;
}

function distToSegment(
  p: { x: number; y: number },
  v: { x: number; y: number },
  w: { x: number; y: number },
): number {
  const l2 = (v.x - w.x) * (v.x - w.x) + (v.y - w.y) * (v.y - w.y);
  if (l2 === 0) return Math.hypot(p.x - v.x, p.y - v.y);
  let t = ((p.x - v.x) * (w.x - v.x) + (p.y - v.y) * (w.y - v.y)) / l2;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(p.x - (v.x + t * (w.x - v.x)), p.y - (v.y + t * (w.y - v.y)));
}
