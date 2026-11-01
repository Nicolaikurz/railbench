import type { Connector, TrackPiece, Transform } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";
import { getLocalConnectors } from "@/catalog/catalog";
import { dist, normAngle, rotate, transformPoint } from "./geometry";

export interface SnapResult {
  pieceId: string;
  connectorIndex: number;
  target: Connector;
}

/** Findet den nächsten freien, gegenläufigen Connector im Radius. */
export function findSnap(
  moving: Connector,
  pieces: TrackPiece[],
  radius = 20,
  angleTolerance = 10,
): SnapResult | null {
  let best: SnapResult | null = null;
  let bestDist = radius;
  for (const p of pieces) {
    p.connectors.forEach((c, i) => {
      if (c.connectedTo) return;
      const d = dist(moving, c);
      const diff = Math.abs(normAngle(moving.angle - c.angle) - 180);
      if (d < bestDist && diff <= angleTolerance) {
        bestDist = d;
        best = { pieceId: p.id, connectorIndex: i, target: c };
      }
    });
  }
  return best;
}

export interface BestPieceSnap {
  movingConnectorIndex: number;
  targetPieceId: string;
  targetConnectorIndex: number;
  snappedTransform: Transform;
  dist: number;
}

/**
 * Sucht für ein bewegtes/zu platzierendes Gleisstück den optimalen Andockpunkt an bereits existierende Gleise.
 * Berechnet direkt die exakte Position und Rotation für nahtloses Einrasten.
 */
export function findBestSnapForPiece(
  entry: CatalogEntry,
  candidateTransform: Transform,
  pieces: TrackPiece[],
  ignorePieceId?: string,
  radius = 35,
  angleTolerance = 25,
): BestPieceSnap | null {
  const locals = getLocalConnectors(entry);
  let best: BestPieceSnap | null = null;
  let bestDist = radius;

  for (let mi = 0; mi < locals.length; mi++) {
    const local = locals[mi]!;
    // Weltposition des bewegten Connectors bei aktuellem Transform:
    const movingWorldPos = transformPoint(local, candidateTransform);
    const movingWorldAngle = normAngle(candidateTransform.rotation + local.angle);

    for (const piece of pieces) {
      if (piece.id === ignorePieceId) continue;

      for (let ti = 0; ti < piece.connectors.length; ti++) {
        const target = piece.connectors[ti]!;
        // Falls Connector schon mit einem anderen Teil (außer eventuell dem gerade bewegten) belegt ist
        if (target.connectedTo && target.connectedTo.pieceId !== ignorePieceId) continue;

        const d = dist(movingWorldPos, target);
        // Winkel-Differenz zur Gegenrichtung (180°)
        const diff = Math.abs(normAngle(movingWorldAngle - target.angle) - 180);

        if (d < bestDist && diff <= angleTolerance) {
          // Exakte Ausrichtung berechnen:
          // target.angle + 180° = snappedRotation + local.angle
          const snappedRotation = normAngle(target.angle + 180 - local.angle);
          const rotatedOffset = rotate(local, snappedRotation);
          const snappedPos = {
            x: target.x - rotatedOffset.x,
            y: target.y - rotatedOffset.y,
          };

          bestDist = d;
          best = {
            movingConnectorIndex: mi,
            targetPieceId: piece.id,
            targetConnectorIndex: ti,
            snappedTransform: {
              x: snappedPos.x,
              y: snappedPos.y,
              rotation: snappedRotation,
            },
            dist: d,
          };
        }
      }
    }
  }

  return best;
}
