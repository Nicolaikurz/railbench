import { rad2deg, normAngle, rotate } from "@/core/geometry";
import type { Layout, TrackPiece, Model3D, Transform } from "@/core/types";
import type { Selection } from "@/editor/selection";
import type { History } from "@/editor/commands";
import type { CatalogEntry } from "@/catalog/catalog";
import { computeWorldConnectors } from "@/catalog/catalog";
import { findBestSnapForPiece, type BestPieceSnap } from "@/core/snapping";
import { snapTrackPieceToBuilding, snapEngineShedToTrack, snapTrainhallToTrack } from "@/core/engine-shed-snap";
import { snapModelToTurntable } from "@/core/turntable-snap";
import { rotateGroupAroundCentroid, type RotatableItem } from "@/core/geometry";
import { MovePieceCommand, MoveModelCommand } from "@/editor/commands";

export function populateDraggedPiecesMap(
  layout: Layout,
  selection: Selection,
  isPieceInteractive: (piece: TrackPiece) => boolean,
  isModelInteractive: (model: Model3D) => boolean,
  draggedPiecesMap: Map<string, Transform>,
  draggedInitialConnections: { pieceId: string; connectorIndex: number }[],
): void {
  draggedPiecesMap.clear();
  for (const id of selection.all()) {
    const p = layout.pieces.find((x) => x.id === id);
    if (p && isPieceInteractive(p)) draggedPiecesMap.set(p.id, { ...p.transform });
    const m = layout.models?.find((x) => x.id === id);
    if (m && isModelInteractive(m)) draggedPiecesMap.set(m.id, { ...m.transform });
  }
  draggedInitialConnections.length = 0;
  for (const [id] of draggedPiecesMap.entries()) {
    const p = layout.pieces.find((x) => x.id === id);
    if (p) {
      for (const c of p.connectors) {
        if (c.connectedTo && !draggedPiecesMap.has(c.connectedTo.pieceId)) {
          draggedInitialConnections.push({ pieceId: c.connectedTo.pieceId, connectorIndex: c.connectedTo.connectorIndex });
        }
      }
    }
  }
}

export function commitMovedItems(
  layout: Layout,
  history: History,
  draggedPiecesMap: Map<string, Transform>,
): boolean {
  let movedAny = false;
  for (const [id, origT] of draggedPiecesMap.entries()) {
    const p = layout.pieces.find((x) => x.id === id);
    if (p && (p.transform.x !== origT.x || p.transform.y !== origT.y || p.transform.rotation !== origT.rotation)) {
      history.run(new MovePieceCommand(p.id, origT, { ...p.transform }));
      movedAny = true;
    }
    const m = layout.models?.find((x) => x.id === id);
    if (m && (m.transform.x !== origT.x || m.transform.y !== origT.y || m.transform.rotation !== origT.rotation)) {
      history.run(new MoveModelCommand(m.id, origT, { ...m.transform }));
      movedAny = true;
    }
  }
  return movedAny;
}

export function applyGroupRotationWithSnapping(
  layout: Layout,
  draggedPiecesMap: Map<string, Transform>,
  pieceCenterWorld: { x: number; y: number },
  deltaDeg: number,
  snappingEnabled: boolean,
  getCatalogEntryForPiece: (piece: TrackPiece) => CatalogEntry | undefined,
): void {
  const rotItems: RotatableItem[] = [];
  for (const [id, origT] of draggedPiecesMap.entries()) {
    const p = layout.pieces.find((x) => x.id === id);
    if (p) {
      const pEntry = getCatalogEntryForPiece(p);
      rotItems.push({ id, type: "piece", transform: origT, entry: pEntry });
    }
    const m = layout.models?.find((x) => x.id === id);
    if (m) {
      rotItems.push({ id, type: "model", transform: origT });
    }
  }

  const newTransforms = rotateGroupAroundCentroid(rotItems, deltaDeg, pieceCenterWorld);

  if (snappingEnabled) {
    const otherPieces = layout.pieces.filter((p) => !draggedPiecesMap.has(p.id));
    let bestSnap: { itemSnap: BestPieceSnap; origT: Transform } | null = null;
    let bestDist = Infinity;

    for (const [id] of draggedPiecesMap.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) {
        const pEntry = getCatalogEntryForPiece(p);
        if (!pEntry) continue;
        const candT = newTransforms.get(id);
        if (!candT) continue;

        let snap = findBestSnapForPiece(pEntry, candT, otherPieces, undefined, 35, 30, undefined, p, layout.layers);
        if (layout.models) {
          const bSnap = snapTrackPieceToBuilding(pEntry, candT, layout.models);
          if (bSnap && (!snap || bSnap.dist < snap.dist)) {
            snap = bSnap;
          }
        }
        if (snap && snap.dist < bestDist) {
          bestDist = snap.dist;
          bestSnap = { itemSnap: snap, origT: draggedPiecesMap.get(id)! };
        }
      } else {
        const m = layout.models?.find((x) => x.id === id);
        if (m && (m.builtinId === "engine-shed" || m.builtinId === "turntable-trainhall")) {
          const candT = newTransforms.get(id);
          if (!candT) continue;
          const snap = m.builtinId === "engine-shed"
            ? snapEngineShedToTrack(candT, layout.pieces, (pc: TrackPiece) => getCatalogEntryForPiece(pc))
            : snapTrainhallToTrack(candT, layout.pieces);
          if (snap && snap.dist < bestDist) {
            bestDist = snap.dist;
            bestSnap = {
              itemSnap: {
                movingConnectorIndex: 0,
                targetPieceId: snap.targetPiece.id,
                targetConnectorIndex: snap.connectorIndex,
                snappedTransform: snap.snappedTransform,
                dist: snap.dist,
              },
              origT: draggedPiecesMap.get(id)!,
            };
          }
        }
      }
    }

    if (bestSnap) {
      const { itemSnap, origT } = bestSnap;
      const snapDeltaRot = normAngle(itemSnap.snappedTransform.rotation - origT.rotation);
      for (const [id, oT] of draggedPiecesMap.entries()) {
        const relOffset = { x: oT.x - origT.x, y: oT.y - origT.y };
        const rotOffset = rotate(relOffset, snapDeltaRot);
        const snappedPos: Transform = {
          x: itemSnap.snappedTransform.x + rotOffset.x,
          y: itemSnap.snappedTransform.y + rotOffset.y,
          rotation: normAngle(oT.rotation + snapDeltaRot),
        };
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          p.transform = snappedPos;
          const pEntry = getCatalogEntryForPiece(p);
          if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
        }
        const m = layout.models?.find((x) => x.id === id);
        if (m) m.transform = snappedPos;
      }
      return;
    }
  }

  for (const [id, newT] of newTransforms.entries()) {
    const p = layout.pieces.find((x) => x.id === id);
    if (p) {
      p.transform = { ...newT };
      const pEntry = getCatalogEntryForPiece(p);
      if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
    }
    const m = layout.models?.find((x) => x.id === id);
    if (m) m.transform = { ...newT };
  }
}

export function applyGroupDragWithSnapping(
  layout: Layout,
  draggedPiecesMap: Map<string, Transform>,
  dx: number,
  dy: number,
  dragDist: number,
  snappingEnabled: boolean,
  hasUnlatchedFromInitial: boolean,
  draggedInitialConnections: { pieceId: string; connectorIndex: number }[],
  getCatalogEntryForPiece: (piece: TrackPiece) => CatalogEntry | undefined,
): void {
  let groupSnap: {
    snappedTransform: Transform;
    sourceId: string;
    origTransform: Transform;
  } | null = null;

  if (snappingEnabled) {
    const otherPieces = layout.pieces.filter((p) => !draggedPiecesMap.has(p.id));
    let bestDist = Infinity;
    const ignoreConn = !hasUnlatchedFromInitial ? draggedInitialConnections : undefined;

    for (const [id, origT] of draggedPiecesMap.entries()) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece) {
        const entry = getCatalogEntryForPiece(piece);
        if (!entry) continue;
        const candT: Transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
        let snap = findBestSnapForPiece(entry, candT, otherPieces, undefined, 45, 60, ignoreConn, piece, layout.layers);
        if (layout.models) {
          const bSnap = snapTrackPieceToBuilding(entry, candT, layout.models);
          if (bSnap && (!snap || bSnap.dist < snap.dist)) {
            snap = bSnap;
          }
        }
        if (snap && snap.dist < bestDist) {
          bestDist = snap.dist;
          groupSnap = { snappedTransform: snap.snappedTransform, sourceId: id, origTransform: origT };
        }
      } else {
        const m = layout.models?.find((x) => x.id === id);
        if (m && m.builtinId === "turntable-trainhall") {
          const candT: Transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
          let snap: any = snapModelToTurntable(candT, layout.pieces, (p: TrackPiece) => getCatalogEntryForPiece(p));
          if (!snap) {
            snap = snapTrainhallToTrack(candT, layout.pieces);
          }
          if (snap && snap.dist < bestDist) {
            bestDist = snap.dist;
            groupSnap = { snappedTransform: snap.snappedTransform, sourceId: id, origTransform: origT };
          }
        } else if (m && m.builtinId === "engine-shed") {
          const candT: Transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
          const snap = snapEngineShedToTrack(candT, layout.pieces, (p: TrackPiece) => getCatalogEntryForPiece(p));
          if (snap && snap.dist < bestDist) {
            bestDist = snap.dist;
            groupSnap = { snappedTransform: snap.snappedTransform, sourceId: id, origTransform: origT };
          }
        }
      }
    }
  }

  if (groupSnap) {
    const { snappedTransform, origTransform } = groupSnap;
    const deltaRot = normAngle(snappedTransform.rotation - origTransform.rotation);

    for (const [id, origT] of draggedPiecesMap.entries()) {
      const relOffset = { x: origT.x - origTransform.x, y: origT.y - origTransform.y };
      const rotOffset = rotate(relOffset, deltaRot);
      const newPos: Transform = {
        x: snappedTransform.x + rotOffset.x,
        y: snappedTransform.y + rotOffset.y,
        rotation: normAngle(origT.rotation + deltaRot),
      };

      const p = layout.pieces.find((x) => x.id === id);
      if (p) {
        p.transform = newPos;
        const pEntry = getCatalogEntryForPiece(p);
        if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
      }
      const m = layout.models?.find((x) => x.id === id);
      if (m) m.transform = newPos;
    }
  } else {
    for (const [id, origT] of draggedPiecesMap.entries()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) {
        p.transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
        const pEntry = getCatalogEntryForPiece(p);
        if (pEntry) {
          p.connectors = computeWorldConnectors(pEntry, p.transform);
        }
      }
      const m = layout.models?.find((x) => x.id === id);
      if (m) {
        m.transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
      }
    }
  }
}
