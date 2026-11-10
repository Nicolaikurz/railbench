import type { Connector, TrackPiece, Transform, Layer, Vec2 } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";
import { computeWorldConnectors } from "@/catalog/catalog";
import { dist, deg2rad, untransformPoint } from "./geometry";
import { getPieceConnectorHeight } from "./snapping";

/**
 * Prüft, ob ein Gleisstück ein Flexgleis ist (aus dem Katalog oder mit flexibler Geometrie).
 */
export function isFlexTrack(entry?: CatalogEntry | null, piece?: TrackPiece | null): boolean {
  if (entry?.kind === "flex") return true;
  if (entry?.name && entry.name.toLowerCase().includes("flex")) return true;
  if (entry?.description && entry.description.toLowerCase().includes("flex")) return true;
  if (piece && (piece.customLength !== undefined || piece.customRadius !== undefined || piece.customAngle !== undefined)) {
    return true;
  }
  return false;
}

/**
 * Dreht ein Flexgleis um, sodass Connector 0 und 1 getauscht werden.
 * Dies ermöglicht das interaktive Anpassen von jedem offenen Ende aus,
 * während das verbundene Ende sicher an Position und Ausrichtung verankert bleibt.
 */
export function flipFlexTrackPiece(piece: TrackPiece, entry: CatalogEntry): void {
  const c1 = piece.connectors[1];
  if (!c1) return;

  // Neuer Ursprung ist das bisherige Connector-1-Ende, Ausrichtung zeigt rückwärts
  piece.transform = {
    x: Math.round(c1.x * 10) / 10,
    y: Math.round(c1.y * 10) / 10,
    rotation: Math.round(((c1.angle - 180 + 3600) % 360) * 10) / 10,
  };

  // Kurvenwinkel umkehren, da der Kreisbogen nun von der Gegenseite durchlaufen wird
  if (piece.customAngle !== undefined) {
    piece.customAngle = -piece.customAngle;
  }

  // Steigung umkehren (Start- und Endhöhe tauschen)
  if (piece.elevation) {
    piece.elevation = {
      start: piece.elevation.end,
      end: piece.elevation.start,
    };
  }

  // Connectors neu in Weltkoordinaten berechnen
  piece.connectors = computeWorldConnectors(entry, piece.transform, piece);
}

/**
 * Liefert den effektiven Katalogeintrag für ein Flexgleis (mit angepasster Länge, Radius und Winkel).
 */
export function getEffectiveFlexEntry(entry: CatalogEntry, piece?: TrackPiece | null): CatalogEntry {
  if (!piece) return entry;
  if (piece.customLength !== undefined || piece.customRadius !== undefined || piece.customAngle !== undefined) {
    const isCurved = Boolean(piece.customRadius && piece.customAngle && Math.abs(piece.customAngle) > 0.1);
    return {
      ...entry,
      length: piece.customLength ?? entry.length,
      radius: piece.customRadius ?? entry.radius,
      angle: piece.customAngle ?? entry.angle,
      kind: isCurved ? "curve" : entry.kind === "flex" ? "straight" : entry.kind,
    };
  }
  return entry;
}

export interface FlexAdjustmentResult {
  customLength: number;
  customRadius?: number;
  customAngle?: number;
  elevation?: { start: number; end: number };
  snappedConnector?: { pieceId: string; connectorIndex: number; point: Vec2 };
}

/**
 * Sucht einen naheliegenden offenen Connector eines anderen Gleises,
 * an den der offene Andockpunkt des Flexgleises "geheftet" (gesnappt) werden kann.
 */
export function findNearbyConnectorForFlex(
  piece: TrackPiece,
  targetWorldPos: Vec2,
  allPieces: TrackPiece[],
  snapRadius = 25,
  maxLen = 900,
): { piece: TrackPiece; connectorIndex: number; connector: Connector } | null {
  let closest: { piece: TrackPiece; connectorIndex: number; connector: Connector } | null = null;
  let closestDist = snapRadius;

  for (const other of allPieces) {
    if (other.id === piece.id) continue;
    for (let i = 0; i < other.connectors.length; i++) {
      const conn = other.connectors[i]!;
      if (conn.connectedTo && conn.connectedTo.pieceId !== piece.id) continue;
      // Prüfen, ob der Anschluss innerhalb der physischen Reichweite des Flexgleises liegt
      const directDist = dist(piece.transform, conn);
      if (directDist > maxLen + snapRadius) continue;

      const d = dist(targetWorldPos, conn);
      if (d < closestDist) {
        closestDist = d;
        closest = { piece: other, connectorIndex: i, connector: conn };
      }
    }
  }

  return closest;
}

/**
 * Passt das offene Gleisende eines Flexgleises interaktiv an:
 * - Berechnet exakte Länge und Bogenradius / Bogenwinkel zur Zielposition
 * - Begrenzt die Länge strikt auf die maximale Kataloglänge des Flexgleises (kann nicht überdehnt werden)
 * - Unterstützt das "Anheften" (Snappen) an offene Anschlüsse anderer Gleise im zulässigen Längenbereich
 * - Unterstützt Steigungen/Rampen: passt bei Bedarf die Endhöhe an das Zielgleis an
 */
export function adjustFlexTrackOpenConnector(
  piece: TrackPiece,
  entry: CatalogEntry,
  targetWorldPos: Vec2,
  allPieces?: TrackPiece[],
  layers?: Layer[],
  snapRadius = 25,
): FlexAdjustmentResult {
  const maxLen = entry.length ?? 900;
  let finalPos: Vec2 = { ...targetWorldPos };
  let snappedInfo: { pieceId: string; connectorIndex: number; point: Vec2 } | undefined;
  let targetHeight: number | undefined;

  // 1. Snapping an benachbarten offenen Anschluss prüfen ("tack the open snappoint")
  if (allPieces && allPieces.length > 0) {
    const nearby = findNearbyConnectorForFlex(piece, targetWorldPos, allPieces, snapRadius, maxLen);
    if (nearby) {
      finalPos = { x: nearby.connector.x, y: nearby.connector.y };
      snappedInfo = {
        pieceId: nearby.piece.id,
        connectorIndex: nearby.connectorIndex,
        point: finalPos,
      };
      targetHeight = getPieceConnectorHeight(nearby.piece, nearby.connectorIndex, layers);
    }
  }

  // 2. Zielkoordinaten in das lokale Koordinatensystem von Connector 0 (piece.transform) transformieren
  const local = untransformPoint(finalPos, piece.transform);
  let x = local.x;
  const y = local.y;

  // Mindestlänge für Gleisabschnitte
  if (x < 15) x = 15;

  let customLength: number;
  let customRadius: number | undefined;
  let customAngle: number | undefined;

  // 3. Gerades oder gebogenes Flexgleis berechnen
  if (Math.abs(y) < 1.5) {
    // Nahezu gerade: Geradeaus-Flexgleis
    if (x > maxLen) {
      x = maxLen;
      snappedInfo = undefined;
    }
    customLength = Math.min(maxLen, Math.max(15, Math.round(x * 10) / 10));
    customRadius = undefined;
    customAngle = undefined;
  } else {
    // Gebogenes Flexgleis: Kreisbogen durch (0,0) tangential zur X-Achse und durch (x, y)
    // Kreisgleichung: x^2 + (y - R)^2 = R^2 => R = (x^2 + y^2) / (2 * |y|)
    const d2 = x * x + y * y;
    const absY = Math.abs(y);
    const R = Math.max(50, Math.round((d2 / (2 * absY)) * 10) / 10);
    const chordAngle = Math.atan2(y, x);
    const alphaRad = 2 * chordAngle;
    const arcLength = R * Math.abs(alphaRad);

    if (arcLength > maxLen) {
      // Flexgleis kann seine maximale Länge nicht überschreiten: Bogenlänge auf maxLen begrenzen
      customLength = maxLen;
      const maxAlphaDeg = (maxLen / R) * (180 / Math.PI);
      customAngle = Math.round(Math.sign(alphaRad) * maxAlphaDeg * 10) / 10;
      customRadius = R;
      snappedInfo = undefined;
    } else {
      customLength = Math.max(15, Math.round(arcLength * 10) / 10);
      customRadius = R;
      customAngle = Math.round(((alphaRad * 180) / Math.PI) * 10) / 10;
    }
  }

  // 4. Rampen-Höhenanpassung ("flexible track for ramps they can change over their length the ramp")
  if (targetHeight !== undefined && layers) {
    const pieceLayer = layers.find((l) => l.id === piece.layerId);
    const baseH = pieceLayer?.heightMm ?? 0;
    const startRel = piece.elevation?.start ?? 0;
    const endRel = targetHeight - baseH;
    piece.elevation = {
      start: Math.round(startRel * 10) / 10,
      end: Math.round(endRel * 10) / 10,
    };
  }

  // Parameter am Gleisstück aktualisieren
  piece.customLength = customLength;
  piece.customRadius = customRadius;
  piece.customAngle = customAngle;

  // Welt-Connectors für das neue Flexgleis aktualisieren
  piece.connectors = computeWorldConnectors(entry, piece.transform, piece);
  if (snappedInfo && piece.connectors[1]) {
    piece.connectors[1].x = Math.round(finalPos.x * 10) / 10;
    piece.connectors[1].y = Math.round(finalPos.y * 10) / 10;
  }

  return {
    customLength,
    customRadius,
    customAngle,
    elevation: piece.elevation,
    snappedConnector: snappedInfo,
  };
}

/**
 * Schneidet ein Flexgleis auf eine neue Länge (in mm) zu (begrenzt auf maxLen).
 */
export function cutFlexTrack(piece: TrackPiece, entry: CatalogEntry, newLengthMm: number): void {
  const maxLen = entry.length ?? 900;
  const len = Math.min(maxLen, Math.max(15, Math.round(newLengthMm * 10) / 10));
  const oldLen = piece.customLength ?? entry.length ?? 900;

  piece.customLength = len;
  // Falls bereits gebogen, den Winkel proportional an die neue Länge anpassen
  if (piece.customRadius && piece.customAngle && oldLen > 0) {
    const ratio = len / oldLen;
    piece.customAngle = Math.round(piece.customAngle * ratio * 10) / 10;
  }

  piece.connectors = computeWorldConnectors(entry, piece.transform, piece);
}

/**
 * Biegt ein Flexgleis auf einen festen Radius (in mm) und Winkel (in Grad),
 * ohne dass die Bogenlänge die maximale Gleislänge überschreitet.
 */
export function bendFlexTrack(
  piece: TrackPiece,
  entry: CatalogEntry,
  radiusMm?: number,
  angleDeg?: number,
): void {
  const maxLen = entry.length ?? 900;
  if (radiusMm && angleDeg && Math.abs(angleDeg) > 0.1 && radiusMm > 0) {
    piece.customRadius = Math.max(50, Math.round(radiusMm * 10) / 10);
    let requestedAngle = Math.round(angleDeg * 10) / 10;
    let arcLength = (Math.PI * piece.customRadius * Math.abs(requestedAngle)) / 180;

    if (arcLength > maxLen) {
      requestedAngle = Math.sign(requestedAngle) * Math.round(((maxLen * 180) / (Math.PI * piece.customRadius)) * 10) / 10;
      arcLength = maxLen;
    }

    piece.customAngle = requestedAngle;
    piece.customLength = Math.max(15, Math.round(arcLength * 10) / 10);
  } else {
    // Wieder als gerades Gleis
    piece.customRadius = undefined;
    piece.customAngle = undefined;
    piece.customLength = Math.min(maxLen, piece.customLength ?? maxLen);
  }

  piece.connectors = computeWorldConnectors(entry, piece.transform, piece);
}

/**
 * Berechnet Rampen- und Steigungsinformationen für ein Flexgleis.
 */
export function getFlexTrackRampInfo(
  piece: TrackPiece,
  entry: CatalogEntry,
  layers?: Layer[],
): {
  lengthMm: number;
  startElevMm: number;
  endElevMm: number;
  riseMm: number;
  gradientPercent: number;
  isRamp: boolean;
} {
  const layer = layers?.find((l) => l.id === piece.layerId);
  const baseH = layer?.heightMm ?? 0;
  const startElev = baseH + (piece.elevation?.start ?? 0);
  const endElev = baseH + (piece.elevation?.end ?? (piece.elevation?.start ?? 0));
  const rise = Math.round((endElev - startElev) * 10) / 10;
  const length = piece.customLength ?? entry.length ?? 900;
  const grad = length > 0 ? (Math.abs(rise) / length) * 100 : 0;

  return {
    lengthMm: length,
    startElevMm: startElev,
    endElevMm: endElev,
    riseMm: rise,
    gradientPercent: Math.round(grad * 10) / 10,
    isRamp: Math.abs(rise) >= 0.5,
  };
}
