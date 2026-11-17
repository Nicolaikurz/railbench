import { deg2rad, dist, distanceToTrackPiece } from "@/core/geometry";
import type { Layout, TrackPiece, Model3D } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import type { Selection } from "@/editor/selection";
import { getConnectorSnapStatus } from "@/core/snapping";
import { isConnectorConnectedToBuilding } from "@/core/engine-shed-snap";
import { isFlexTrack, flipFlexTrackPiece, getFlexTrackMidpoint, getFlexTrackPointAt } from "@/core/flextrack";

export interface FlexHandleHit {
  piece: TrackPiece;
  entry: CatalogEntry;
  handleType: "connector" | "middle";
  pivotFraction?: number;
  fixedC0?: { x: number; y: number };
  fixedC1?: { x: number; y: number };
}

export function findOpenConnectorAt(
  layout: Layout,
  selection: Selection,
  isPieceInteractive: (piece: TrackPiece) => boolean,
  pos: { x: number; y: number },
  scale: number,
  getCatalogEntryForPiece?: (piece: TrackPiece) => CatalogEntry | undefined,
): { piece: TrackPiece; connectorIndex: number } | null {
  const selectedIds = new Set(selection.all());
  const plusDist = 28;

  // 1. Markierte Gleise zuerst prüfen
  for (const selId of selectedIds) {
    const piece = layout.pieces.find((p) => p.id === selId);
    if (!piece || !isPieceInteractive(piece)) continue;

    for (let cIdx = 0; cIdx < piece.connectors.length; cIdx++) {
      const conn = piece.connectors[cIdx];
      if (!conn || conn.connectedTo) continue;
      if (isConnectorConnectedToBuilding(conn, layout.models)) continue;
      const snapInfo = getConnectorSnapStatus(piece, cIdx, layout.pieces, layout.layers);
      if (snapInfo.status !== "free") continue;

      const rad = deg2rad(conn.angle);
      const plusX = conn.x + Math.cos(rad) * plusDist;
      const plusY = conn.y + Math.sin(rad) * plusDist;

      const hitDistPlus = Math.hypot(pos.x - plusX, pos.y - plusY);
      const hitRadius = Math.max(14, 12 / scale);

      if (hitDistPlus <= hitRadius) {
        return { piece, connectorIndex: cIdx };
      }
    }
  }

  // 2. Alle anderen interaktiven Gleise prüfen
  for (const piece of layout.pieces) {
    if (selectedIds.has(piece.id) || !isPieceInteractive(piece)) continue;

    for (let cIdx = 0; cIdx < piece.connectors.length; cIdx++) {
      const conn = piece.connectors[cIdx];
      if (!conn || conn.connectedTo) continue;
      if (isConnectorConnectedToBuilding(conn, layout.models)) continue;
      const snapInfo = getConnectorSnapStatus(piece, cIdx, layout.pieces, layout.layers);
      if (snapInfo.status !== "free") continue;

      const rad = deg2rad(conn.angle);
      const plusX = conn.x + Math.cos(rad) * plusDist;
      const plusY = conn.y + Math.sin(rad) * plusDist;

      const hitDistPlus = Math.hypot(pos.x - plusX, pos.y - plusY);
      const hitRadius = Math.max(14, 12 / scale);

      if (hitDistPlus <= hitRadius) {
        return { piece, connectorIndex: cIdx };
      }
    }
  }

  return null;
}

export function findPointMarkerAt(
  markers: Array<{ x: number; y: number; label: string; color: number; id?: string }> | null | undefined,
  wPos: { x: number; y: number },
  scale: number,
): { x: number; y: number; label: string; color: number; id?: string } | null {
  if (!markers || markers.length === 0) return null;
  const hitRadius = Math.max(26, 22 / scale);

  for (let i = markers.length - 1; i >= 0; i--) {
    const m = markers[i]!;
    // Abstand zum Boden-Zielpunkt
    if (Math.hypot(wPos.x - m.x, wPos.y - m.y) <= hitRadius) return m;
    // Abstand zur erhöhten Pin-Plakette (sitzt bei y - 30)
    if (Math.hypot(wPos.x - m.x, wPos.y - (m.y - 30)) <= hitRadius * 1.35) return m;
    // Bounding-Box für Nadelstiel und Plakette
    if (wPos.x >= m.x - 38 && wPos.x <= m.x + 38 && wPos.y >= m.y - 50 && wPos.y <= m.y + 12) return m;
  }
  return null;
}

export function isNearFlexHandle(
  layout: Layout,
  selection: Selection,
  isPieceInteractive: (piece: TrackPiece) => boolean,
  getCatalogEntryForPiece: (piece: TrackPiece) => CatalogEntry | undefined,
  wPos: { x: number; y: number },
  hitRadius: number,
): boolean {
  const selIds = selection.all();
  const candidates = selIds.length === 1
    ? [layout.pieces.find((p) => p.id === selIds[0])].filter(Boolean) as TrackPiece[]
    : layout.pieces;

  for (const p of candidates) {
    if (!isPieceInteractive(p)) continue;
    const entry = getCatalogEntryForPiece(p);
    if (!entry || !isFlexTrack(entry, p)) continue;

    // Drei Pivot-Punkte (25%, 50%, 75%) zum Biegen bei festen Enden
    if (p.connectors.length >= 2) {
      for (const frac of [0.25, 0.5, 0.75]) {
        const pt = getFlexTrackPointAt(p, entry, frac);
        if (dist(wPos, pt) <= hitRadius) return true;
      }
    }

    // Offene Anschlüsse (Kürzen & Biegen des freien Endes)
    for (const c of p.connectors) {
      if (!c.connectedTo && dist(wPos, c) <= hitRadius) return true;
    }
  }
  return false;
}

export function checkFlexHandleHit(
  layout: Layout,
  selection: Selection,
  isPieceInteractive: (piece: TrackPiece) => boolean,
  getCatalogEntryForPiece: (piece: TrackPiece) => CatalogEntry | undefined,
  wPos: { x: number; y: number },
  hitRadius: number,
): FlexHandleHit | null {
  const selIds = selection.all();
  const candidates = selIds.length === 1
    ? [layout.pieces.find((p) => p.id === selIds[0])].filter(Boolean) as TrackPiece[]
    : layout.pieces;

  for (const p of candidates) {
    if (!isPieceInteractive(p)) continue;
    const entry = getCatalogEntryForPiece(p);
    if (!entry || !isFlexTrack(entry, p)) continue;

    // 1. Drei Pivot-Punkte zuerst prüfen (25%, 50%, 75% bei fixierten Enden)
    if (p.connectors.length >= 2) {
      const fractions = [0.5, 0.25, 0.75];
      for (const frac of fractions) {
        const pt = getFlexTrackPointAt(p, entry, frac);
        if (dist(wPos, pt) <= hitRadius) {
          selection.set(p.id);
          return {
            piece: p,
            entry,
            handleType: "middle",
            pivotFraction: frac,
            fixedC0: { x: p.connectors[0]!.x, y: p.connectors[0]!.y },
            fixedC1: { x: p.connectors[1]!.x, y: p.connectors[1]!.y },
          };
        }
      }
    }

    // 2. Offene Anschlüsse (Kürzen & Biegen des freien Endes)
    if (p.connectors[1] && !p.connectors[1].connectedTo && dist(wPos, p.connectors[1]) <= hitRadius) {
      selection.set(p.id);
      return { piece: p, entry, handleType: "connector" };
    }
    if (p.connectors[0] && !p.connectors[0].connectedTo && dist(wPos, p.connectors[0]) <= hitRadius) {
      flipFlexTrackPiece(p, entry, layout.pieces);
      selection.set(p.id);
      return { piece: p, entry, handleType: "connector" };
    }
  }
  return null;
}

export function findTrackPieceAt(
  layout: Layout,
  wPos: { x: number; y: number },
  scale: number,
  gauge: number,
  isPieceInteractive: (piece: TrackPiece) => boolean,
  getCatalogEntryForPiece: (piece: TrackPiece) => CatalogEntry | undefined,
): TrackPiece | null {
  let hit: TrackPiece | null = null;
  let minD = Infinity;
  const screenGrabBuffer = 24 / scale;

  for (const piece of layout.pieces) {
    if (!isPieceInteractive(piece)) continue;
    const entry = getCatalogEntryForPiece(piece);
    if (!entry) continue;

    const maxAllowedDist = entry.isRoad
      ? Math.max(35, (entry.roadWidth ?? 60) * 0.65, screenGrabBuffer)
      : Math.max(32, gauge * 1.8, screenGrabBuffer);

    const d = distanceToTrackPiece(wPos, piece.transform, entry);
    if (d < maxAllowedDist && d < minD) {
      minD = d;
      hit = piece;
    }
  }
  return hit;
}
