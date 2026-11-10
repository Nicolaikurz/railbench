import type { Connector, TrackPiece, Transform, Layer, Vec2 } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";
import { getLocalConnectors, catalogRegistry } from "@/catalog/catalog";
import { dist, normAngle, rotate, transformPoint, distanceToTrackPiece } from "./geometry";

export interface SnapResult {
  pieceId: string;
  connectorIndex: number;
  target: Connector;
}

/**
 * Berechnet die absolute Höhe (in mm) eines Gleisanschlusses unter Berücksichtigung
 * der Ebenen-Basishöhe (layer.heightMm) und der relativen Gleis-Höhenlage (elevation.start / end).
 */
export function getPieceConnectorHeight(
  piece: { layerId?: string; elevation?: { start: number; end: number } },
  connectorIndex: number,
  layers?: Layer[],
): number {
  const layer = layers?.find((l) => l.id === piece.layerId);
  const baseH = layer?.heightMm ?? 0;
  const relH = connectorIndex === 0
    ? (piece.elevation?.start ?? 0)
    : (piece.elevation?.end ?? (piece.elevation?.start ?? 0));
  return baseH + relH;
}

export type SnapPointStatus = "connected" | "above" | "free";
export type SnapPointColor = "grey" | "yellow" | "green";

export interface ConnectorSnapInfo {
  status: SnapPointStatus;
  color: SnapPointColor;
  height: number;
  connectedTo?: { pieceId: string; connectorIndex: number };
  abovePieceId?: string;
}

/**
 * Prüft, ob zwei Gleisanschlüsse geometrisch und höhenmäßig miteinander verbunden sind:
 * - Liegen an derselben Position (Abstand <= distTolerance)
 * - Sind gegenläufig ausgerichtet (Winkel-Differenz ~ 180°)
 * - Liegen auf derselben Höhe (Höhendifferenz <= heightTolerance)
 */
export function areConnectorsConnected(
  c1: Connector,
  c2: Connector,
  h1: number,
  h2: number,
  distTolerance = 4.0,
  angleTolerance = 35.0,
  heightTolerance = 1.0,
): boolean {
  if (Math.abs(h1 - h2) > heightTolerance) return false;
  const d = dist(c1, c2);
  if (d > distTolerance) return false;
  let diff = Math.abs(normAngle(c1.angle - c2.angle) - 180);
  if (diff > 180) diff = 360 - diff;
  return diff <= angleTolerance;
}

/**
 * Ermittelt den Snap-Status und die Farbe für einen Gleisanschluss:
 * - "connected" (grau): Zwei Gleise sind an diesem Anschluss auf gleicher Höhe verbunden.
 * - "above" (gelb): Ein Gleis verläuft über diesem Anschlusspunkt (Höhenunterschied).
 * - "free" (grün): Freier, unbesetzter Andockpunkt.
 */
export function getConnectorSnapStatus(
  piece: TrackPiece,
  connectorIndex: number,
  allPieces: TrackPiece[],
  layers?: Layer[],
  distTolerance = 4.0,
  angleTolerance = 35.0,
  heightTolerance = 1.0,
): ConnectorSnapInfo {
  const conn = piece.connectors[connectorIndex];
  if (!conn) {
    return { status: "free", color: "green", height: 0 };
  }

  const h = getPieceConnectorHeight(piece, connectorIndex, layers);

  // 1. Prüfen, ob zwei Gleise hier verbunden sind (gleiche Höhe, passende Position & Winkel)
  for (const other of allPieces) {
    if (other.id === piece.id) continue;
    for (let oi = 0; oi < other.connectors.length; oi++) {
      const otherConn = other.connectors[oi]!;
      const otherH = getPieceConnectorHeight(other, oi, layers);
      if (areConnectorsConnected(conn, otherConn, h, otherH, distTolerance, angleTolerance, heightTolerance)) {
        return {
          status: "connected",
          color: "grey",
          height: h,
          connectedTo: { pieceId: other.id, connectorIndex: oi },
        };
      }
    }
  }

  // 2. Prüfen, ob ein anderes Gleis ÜBER diesem Anschlusspunkt liegt (Gleis liegt höher)
  for (const other of allPieces) {
    if (other.id === piece.id) continue;
    const otherLayer = layers?.find((l) => l.id === other.layerId);
    const otherBaseH = otherLayer?.heightMm ?? 0;
    const otherStartH = otherBaseH + (other.elevation?.start ?? 0);
    const otherEndH = otherBaseH + (other.elevation?.end ?? (other.elevation?.start ?? 0));
    const otherMaxH = Math.max(otherStartH, otherEndH);

    // Liegt das andere Gleis vertikal über diesem Connector?
    if (otherMaxH > h + heightTolerance) {
      // Prüfe, ob ein Connector des anderen Gleises direkt darüber liegt
      let isNear = false;
      for (const oc of other.connectors) {
        if (dist(conn, oc) <= 18) {
          isNear = true;
          break;
        }
      }
      // Oder ob der Gleiskörper des anderen Gleises über diesem Punkt verläuft
      if (!isNear) {
        const otherEntry = catalogRegistry.findEntry(other.catalogId);
        if (otherEntry) {
          const d = distanceToTrackPiece(conn, other.transform, otherEntry);
          if (d <= 20) {
            isNear = true;
          }
        }
      }

      if (isNear) {
        return {
          status: "above",
          color: "yellow",
          height: h,
          abovePieceId: other.id,
        };
      }
    }
  }

  // 3. Weder verbunden noch verdeckt: freier Snap-Punkt
  return {
    status: "free",
    color: "green",
    height: h,
  };
}

/**
 * Aktualisiert die connectedTo-Verbindungen aller Gleise im Layout basierend auf
 * geometrischem Snapping und Höhengleichheit.
 */
export function updatePieceConnections(
  pieces: TrackPiece[],
  layers?: Layer[],
  distTolerance = 4.0,
  angleTolerance = 35.0,
  heightTolerance = 1.0,
): void {
  for (let i = 0; i < pieces.length; i++) {
    const p1 = pieces[i]!;
    for (let c1 = 0; c1 < p1.connectors.length; c1++) {
      const conn1 = p1.connectors[c1]!;
      const h1 = getPieceConnectorHeight(p1, c1, layers);
      let matched = false;

      for (let j = 0; j < pieces.length; j++) {
        if (i === j) continue;
        const p2 = pieces[j]!;
        for (let c2 = 0; c2 < p2.connectors.length; c2++) {
          const conn2 = p2.connectors[c2]!;
          const h2 = getPieceConnectorHeight(p2, c2, layers);
          if (areConnectorsConnected(conn1, conn2, h1, h2, distTolerance, angleTolerance, heightTolerance)) {
            conn1.connectedTo = { pieceId: p2.id, connectorIndex: c2 };
            matched = true;
            break;
          }
        }
        if (matched) break;
      }

      if (!matched && conn1.connectedTo) {
        conn1.connectedTo = undefined;
      }
    }
  }
}

/** Findet den nächsten freien, gegenläufigen Connector im Radius unter Berücksichtigung der gleichen Höhe. */
export function findSnap(
  moving: Connector,
  pieces: TrackPiece[],
  radius = 20,
  angleTolerance = 10,
  movingHeight?: number,
  layers?: Layer[],
  maxHeightDiff = 1.0,
): SnapResult | null {
  let best: SnapResult | null = null;
  let bestDist = radius;
  for (const p of pieces) {
    p.connectors.forEach((c, i) => {
      if (c.connectedTo) return;
      if (movingHeight !== undefined) {
        const targetH = getPieceConnectorHeight(p, i, layers);
        if (Math.abs(movingHeight - targetH) > maxHeightDiff) return;
      }
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
  targetHeight?: number;
  movingHeight?: number;
}

export interface SnapAboveResult {
  movingConnectorIndex: number;
  targetPieceId: string;
  targetConnectorIndex: number;
  point: Vec2;
  movingHeight: number;
  targetHeight: number;
  dist: number;
}

/**
 * Prüft, ob ein bewegtes Gleis über einem existierenden Andockpunkt liegt
 * (Höhenunterschied: bewegtes Gleis ist höher als der Ziel-Andockpunkt).
 */
export function findSnapAboveForPiece(
  entry: CatalogEntry,
  candidateTransform: Transform,
  pieces: TrackPiece[],
  candidatePieceOrElevation?: { layerId?: string; elevation?: { start: number; end: number } },
  layers?: Layer[],
  radius = 35,
  maxHeightDiff = 1.0,
): SnapAboveResult | null {
  if (!candidatePieceOrElevation) return null;
  const locals = getLocalConnectors(entry);

  let best: SnapAboveResult | null = null;
  let bestDist = radius;

  for (let mi = 0; mi < locals.length; mi++) {
    const local = locals[mi]!;
    const movingWorldPos = transformPoint(local, candidateTransform);
    const movingH = getPieceConnectorHeight(candidatePieceOrElevation, mi, layers);

    for (const piece of pieces) {
      for (let ti = 0; ti < piece.connectors.length; ti++) {
        const target = piece.connectors[ti]!;
        const targetH = getPieceConnectorHeight(piece, ti, layers);

        // Bewegtes Gleis muss ÜBER dem Ziel-Andockpunkt liegen
        if (movingH > targetH + maxHeightDiff) {
          const d = dist(movingWorldPos, target);
          if (d < bestDist) {
            bestDist = d;
            best = {
              movingConnectorIndex: mi,
              targetPieceId: piece.id,
              targetConnectorIndex: ti,
              point: { x: target.x, y: target.y },
              movingHeight: movingH,
              targetHeight: targetH,
              dist: d,
            };
          }
        }
      }
    }
  }

  // Falls kein Connector direkt drüber liegt, prüfe ob der Gleiskörper über einem Ziel-Andockpunkt liegt
  if (!best) {
    const candidateBaseH = layers?.find((l) => l.id === candidatePieceOrElevation.layerId)?.heightMm ?? 0;
    const cStartH = candidateBaseH + (candidatePieceOrElevation.elevation?.start ?? 0);
    const cEndH = candidateBaseH + (candidatePieceOrElevation.elevation?.end ?? (candidatePieceOrElevation.elevation?.start ?? 0));
    const candMinH = Math.min(cStartH, cEndH);

    for (const piece of pieces) {
      for (let ti = 0; ti < piece.connectors.length; ti++) {
        const target = piece.connectors[ti]!;
        const targetH = getPieceConnectorHeight(piece, ti, layers);
        if (candMinH > targetH + maxHeightDiff) {
          const d = distanceToTrackPiece(target, candidateTransform, entry);
          if (d <= 18 && d < bestDist) {
            bestDist = d;
            best = {
              movingConnectorIndex: 0,
              targetPieceId: piece.id,
              targetConnectorIndex: ti,
              point: { x: target.x, y: target.y },
              movingHeight: candMinH,
              targetHeight: targetH,
              dist: d,
            };
          }
        }
      }
    }
  }

  return best;
}

/**
 * Sucht für ein bewegtes/zu platzierendes Gleisstück den optimalen Andockpunkt an bereits existierende Gleise.
 * Berechnet direkt die exakte Position und Rotation für nahtloses Einrasten ("snap in place").
 * Berücksichtigt die gleiche Höhe: Gleise rasten nur ein, wenn sie auf derselben Höhe liegen.
 */
export function findBestSnapForPiece(
  entry: CatalogEntry,
  candidateTransform: Transform,
  pieces: TrackPiece[],
  ignorePieceIds?: string | Set<string> | string[],
  radius = 45,
  angleTolerance = 60,
  ignoreTargetConnectors?: Array<{ pieceId: string; connectorIndex: number }>,
  candidatePieceOrElevation?: { layerId?: string; elevation?: { start: number; end: number } },
  layers?: Layer[],
  maxHeightDiff = 1.0,
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

        // Höhenprüfung ("the connection should be on the same hight"):
        if (candidatePieceOrElevation) {
          const movingH = getPieceConnectorHeight(candidatePieceOrElevation, mi, layers);
          const targetH = getPieceConnectorHeight(piece, ti, layers);
          if (Math.abs(movingH - targetH) > maxHeightDiff) {
            continue; // Höhenunterschied -> kein Verbindungs-Snap
          }
        }

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

          let targetH: number | undefined;
          let movingH: number | undefined;
          if (candidatePieceOrElevation) {
            movingH = getPieceConnectorHeight(candidatePieceOrElevation, mi, layers);
            targetH = getPieceConnectorHeight(piece, ti, layers);
          }

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
            targetHeight: targetH,
            movingHeight: movingH,
          };
        }
      }
    }
  }

  return best;
}
