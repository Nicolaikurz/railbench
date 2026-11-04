import type { Connector, TrackPiece, Transform } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";
import { getLocalConnectors, catalogRegistry } from "@/catalog/catalog";
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
 * Berechnet direkt die exakte Position und Rotation für nahtloses Einrasten ("snap in place").
 */
export function findBestSnapForPiece(
  entry: CatalogEntry,
  candidateTransform: Transform,
  pieces: TrackPiece[],
  ignorePieceIds?: string | Set<string> | string[],
  radius = 45,
  angleTolerance = 60,
  ignoreTargetConnectors?: Array<{ pieceId: string; connectorIndex: number }>,
): BestPieceSnap | null {
  const ignoreSet =
    ignorePieceIds instanceof Set
      ? ignorePieceIds
      : Array.isArray(ignorePieceIds)
        ? new Set(ignorePieceIds)
        : ignorePieceIds
          ? new Set([ignorePieceIds])
          : new Set<string>();

  const locals = getLocalConnectors(entry);
  let best: BestPieceSnap | null = null;
  let bestScore = Infinity;

  for (let mi = 0; mi < locals.length; mi++) {
    const local = locals[mi]!;
    // Weltposition des bewegten Connectors bei aktuellem Transform:
    const movingWorldPos = transformPoint(local, candidateTransform);
    const movingWorldAngle = normAngle(candidateTransform.rotation + local.angle);

    for (const piece of pieces) {
      if (ignoreSet.has(piece.id)) continue;

      // Straßen nur an Straßen, Gleise nur an Gleise snappen
      const targetEntry = catalogRegistry.findEntry(piece.catalogId);
      if (Boolean(targetEntry?.isRoad) !== Boolean(entry.isRoad)) continue;

      for (let ti = 0; ti < piece.connectors.length; ti++) {
        // Falls dieser Zielanschluss ignoriert werden soll (z. B. beim Wegziehen von der bestehenden Verbindung)
        if (ignoreTargetConnectors && ignoreTargetConnectors.some((ic) => ic.pieceId === piece.id && ic.connectorIndex === ti)) {
          continue;
        }

        const target = piece.connectors[ti]!;
        // Falls Connector schon mit einem anderen Teil belegt ist
        if (target.connectedTo && !ignoreSet.has(target.connectedTo.pieceId)) continue;

        const d = dist(movingWorldPos, target);
        if (d > radius) continue;

        // Winkel-Differenz zur Gegenrichtung (180°)
        let diff = Math.abs(normAngle(movingWorldAngle - target.angle) - 180);
        if (diff > 180) diff = 360 - diff;

        // Snap-in-place Toleranz:
        // Bei nahem Heranführen (<= 28mm) rastet das Gleis direkt passgenau ein,
        // auch wenn der Anstellwinkel noch grob abweicht.
        const effectiveAngleTolerance = d <= 28 ? 180 : angleTolerance;
        if (diff > effectiveAngleTolerance) continue;

        // Bewertungsgewichtung: Nähere Anschlüsse und besser ausgerichtete Anschlüsse bevorzugen
        const score = d + (diff / 180) * 20;

        if (score < bestScore) {
          // Exakte Ausrichtung berechnen:
          // target.angle + 180° = snappedRotation + local.angle
          const snappedRotation = normAngle(target.angle + 180 - local.angle);
          const rotatedOffset = rotate(local, snappedRotation);
          const snappedPos = {
            x: target.x - rotatedOffset.x,
            y: target.y - rotatedOffset.y,
          };

          bestScore = score;
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
