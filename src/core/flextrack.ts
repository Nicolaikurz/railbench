import type { Connector, TrackPiece, Layer, Vec2 } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";
import { computeWorldConnectors } from "@/catalog/catalog";
import {
  dist,
  deg2rad,
  transformPoint,
  untransformPoint,
  normAngle,
  evaluateFlexSpline,
  computeFlexSplineLength,
} from "./geometry";
import { getPieceConnectorHeight, updatePieceConnections } from "./snapping";

export { evaluateFlexSpline, computeFlexSplineLength, isSplineTrackEntry } from "./geometry";

export interface FlexSplineSample {
  x: number;
  y: number;
  nx: number;
  ny: number;
  tangentAngleRad: number;
}

/**
 * Erzeugt Stützpunkte entlang des Splines eines Flexgleises im lokalen Koordinatensystem.
 */
export function sampleFlexSplinePoints(
  chordDist: number,
  offsets: number[],
  numSamples = 60,
): FlexSplineSample[] {
  const samples: FlexSplineSample[] = [];
  for (let i = 0; i <= numSamples; i++) {
    const t = i / numSamples;
    const { h, dh } = evaluateFlexSpline(offsets, t);
    const x = t * chordDist;
    const y = h;
    const tangentAngleRad = Math.atan2(dh, chordDist);
    const nx = -Math.sin(tangentAngleRad);
    const ny = Math.cos(tangentAngleRad);
    samples.push({ x, y, nx, ny, tangentAngleRad });
  }
  return samples;
}

/**
 * Prüft, ob ein Gleisstück ein Flexgleis ist (aus dem Katalog oder mit flexibler Geometrie).
 */
export function isFlexTrack(entry?: CatalogEntry | null, piece?: TrackPiece | null): boolean {
  if (entry?.kind === "flex") return true;
  if (entry?.name && entry.name.toLowerCase().includes("flex")) return true;
  if (entry?.description && entry.description.toLowerCase().includes("flex")) return true;
  if (
    piece &&
    (piece.customLength !== undefined ||
      piece.customRadius !== undefined ||
      piece.customAngle !== undefined ||
      piece.customOffsets !== undefined)
  ) {
    return true;
  }
  return false;
}

/**
 * Dreht ein Flexgleis um, sodass Connector 0 und 1 getauscht werden.
 * Dies ermöglicht das interaktive Anpassen von jedem offenen Ende aus,
 * während das verbundene Ende sicher an Position und Ausrichtung verankert bleibt.
 */
export function flipFlexTrackPiece(piece: TrackPiece, entry: CatalogEntry, allPieces?: TrackPiece[]): void {
  const c0 = piece.connectors[0];
  const c1 = piece.connectors[1];
  if (!c1) return;

  const oldC0ConnectedTo = c0?.connectedTo;
  const oldC1ConnectedTo = c1?.connectedTo;

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

  // Spline-Offsets umkehren
  if (piece.customOffsets && piece.customOffsets.length >= 3) {
    piece.customOffsets = [
      -piece.customOffsets[2]!,
      -piece.customOffsets[1]!,
      -piece.customOffsets[0]!,
    ];
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

  // Verbindungen sauber vertauschen (c0 <-> c1)
  if (piece.connectors[0]) piece.connectors[0].connectedTo = oldC1ConnectedTo;
  if (piece.connectors[1]) piece.connectors[1].connectedTo = oldC0ConnectedTo;

  // Auch bei allen anderen Gleisen, die auf piece.id verweisen, den Index vertauschen
  if (allPieces) {
    for (const other of allPieces) {
      if (other.id === piece.id) continue;
      for (const conn of other.connectors) {
        if (conn.connectedTo && conn.connectedTo.pieceId === piece.id) {
          conn.connectedTo.connectorIndex = conn.connectedTo.connectorIndex === 0 ? 1 : 0;
        }
      }
    }
  }
}

/**
 * Setzt ein Flexgleis vollständig in seine ursprüngliche gerade Form und Originallänge zurück.
 * Falls das Gleis an einem Ende (z.B. Connector 1) angeschlossen ist, bleibt dieser Ankerpunkt erhalten.
 */
export function resetFlexTrackToOriginal(
  piece: TrackPiece,
  entry: CatalogEntry,
  allPieces?: TrackPiece[],
  layers?: Layer[],
): void {
  // Falls Connector 1 verbunden ist und Connector 0 frei ist:
  // Drehe das Gleis um, damit der verbundene Anschluss der feste Ursprung (piece.transform) bleibt!
  if (piece.connectors[1]?.connectedTo && !piece.connectors[0]?.connectedTo) {
    flipFlexTrackPiece(piece, entry, allPieces);
  }

  const oldConn0ConnectedTo = piece.connectors[0]?.connectedTo;

  piece.customLength = undefined;
  piece.customRadius = undefined;
  piece.customAngle = undefined;
  piece.customOffsets = undefined;
  piece.customChordDist = undefined;
  piece.connectors = computeWorldConnectors(entry, piece.transform, piece);

  // Der feste Ursprungsanschluss (Connector 0) behält seine bestehende Verbindung bei
  if (piece.connectors[0] && oldConn0ConnectedTo) {
    piece.connectors[0].connectedTo = oldConn0ConnectedTo;
  }

  // Verbindungen für alle Gleise im Layout aktualisieren
  if (allPieces) {
    updatePieceConnections(allPieces, layers);
  }
}

/**
 * Liefert den effektiven Katalogeintrag für ein Flexgleis (mit angepasster Länge, Radius und Winkel).
 */
export function getEffectiveFlexEntry(entry: CatalogEntry, piece?: TrackPiece | null): CatalogEntry {
  if (!piece) return entry;
  if (
    piece.customLength !== undefined ||
    piece.customRadius !== undefined ||
    piece.customAngle !== undefined ||
    piece.customOffsets !== undefined
  ) {
    const isCurved = Boolean(piece.customRadius && piece.customAngle && Math.abs(piece.customAngle) > 0.1);
    return {
      ...entry,
      localConnectors: undefined,
      length: piece.customLength ?? entry.length,
      radius: piece.customRadius ?? entry.radius,
      angle: piece.customAngle ?? entry.angle,
      customOffsets: piece.customOffsets,
      customChordDist: piece.customChordDist,
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
  const isRoad = (cid: string) => cid.startsWith("road-") || cid.startsWith("ST-");
  const pieceIsRoad = isRoad(piece.catalogId);

  for (const other of allPieces) {
    if (other.id === piece.id) continue;
    if (isRoad(other.catalogId) !== pieceIsRoad) continue;
    for (let i = 0; i < other.connectors.length; i++) {
      const conn = other.connectors[i]!;
      // Nicht an Anschlüsse snappen, die bereits mit einem anderen Gleis verbunden sind
      if (conn.connectedTo && conn.connectedTo.pieceId !== piece.id) continue;
      // Nicht an den Anschluss snappen, der bereits mit Connector 0 dieses Gleises verbunden ist
      if (conn.connectedTo && conn.connectedTo.pieceId === piece.id && conn.connectedTo.connectorIndex === 0) continue;

      // Prüfen, ob der Anschluss innerhalb der physischen Reichweite des Flexgleises liegt
      const directDist = dist(piece.transform, conn);
      if (directDist > maxLen + snapRadius * 0.75) continue;

      const d = dist(targetWorldPos, conn);
      if (d < closestDist) {
        // Prüfen, ob der Winkel kompatibel ist:
        // Der Zielconnector zeigt in conn.angle. Ein ankommendes Gleis muss grob gegenläufig sein (conn.angle + 180).
        const chordAngle = Math.atan2(conn.y - piece.transform.y, conn.x - piece.transform.x) * (180 / Math.PI);
        let angleDiff = Math.abs(normAngle(conn.angle - chordAngle) - 180);
        if (angleDiff > 180) angleDiff = 360 - angleDiff;
        if (angleDiff > 85) continue;

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
  let snappedInfo: { pieceId: string; connectorIndex: number; point: Vec2; angle: number } | undefined;
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
        angle: nearby.connector.angle,
      };
      targetHeight = getPieceConnectorHeight(nearby.piece, nearby.connectorIndex, layers);
    }
  }

  // 2. Zielkoordinaten in das lokale Koordinatensystem von Connector 0 (piece.transform) transformieren
  const local = untransformPoint(finalPos, piece.transform);
  let x = local.x;
  let y = local.y;

  let chordDist = Math.hypot(x, y);
  if (chordDist < 15) {
    const scale = 15 / (chordDist || 1);
    x *= scale;
    y *= scale;
    chordDist = 15;
  }

  let customLength: number;
  let customRadius: number | undefined;
  let customAngle: number | undefined;

  // 3. Gerades oder gebogenes Flexgleis berechnen
  const absY = Math.abs(y);
  const chordAngle = Math.atan2(y, x);
  const alphaRad = 2 * chordAngle;
  const alphaDeg = (alphaRad * 180) / Math.PI;
  const d2 = x * x + y * y;
  const R = absY > 0.001 ? Math.max(50, Math.round((d2 / (2 * absY)) * 10) / 10) : 100000;

  // Gerades Flexgleis:
  // Snappt zu gerade, wenn die Krümmung sehr gering ist (Winkel < 1.5° oder Radius > 4000 mm)
  // Bei freiem Ziehen (ohne Snap) auch bei minimaler seitlicher Abweichung (< 2.5 mm bei > 40 mm Distanz)
  const isStraight = Math.abs(alphaDeg) < 1.5 || R > 4000 || (!snappedInfo && absY < 2.5 && chordDist > 40);

  if (isStraight) {
    // Nahezu gerade: Geradeaus-Flexgleis
    // Wenn frei gezogen und nahe an der vollen Originallänge: auf 900 mm einrasten
    if (!snappedInfo && chordDist >= maxLen - 20) {
      chordDist = maxLen;
    }
    const len = Math.min(maxLen, Math.max(15, Math.round(chordDist * 10) / 10));
    customLength = len;
    customRadius = undefined;
    customAngle = undefined;
  } else {
    // Gebogenes Flexgleis: Kreisbogen durch (0,0) tangential zur X-Achse und durch (x, y)
    let arcLength = R * Math.abs(alphaRad);

    if (arcLength > maxLen) {
      // Bogenlänge auf maxLen begrenzen, aber snappedInfo erhalten
      customLength = maxLen;
      const maxAlphaDeg = (maxLen / R) * (180 / Math.PI);
      customAngle = Math.round(Math.sign(alphaRad) * maxAlphaDeg * 10) / 10;
      customRadius = R;
    } else {
      customLength = Math.max(15, Math.round(arcLength * 10) / 10);
      customRadius = R;
      customAngle = Math.round(((alphaRad * 180) / Math.PI) * 10) / 10;
    }
  }

  // 4. Rampen-Höhenanpassung
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
  const oldConn0ConnectedTo = piece.connectors[0]?.connectedTo;
  const oldConn1ConnectedTo = piece.connectors[1]?.connectedTo;
  piece.connectors = computeWorldConnectors(entry, piece.transform, piece);
  if (piece.connectors[0] && oldConn0ConnectedTo) {
    piece.connectors[0].connectedTo = oldConn0ConnectedTo;
  }

  // Bei Snapping: Connector 1 exakt an den Zielanschluss anheften und beidseitig verbinden
  if (snappedInfo && piece.connectors[1]) {
    piece.connectors[1].x = Math.round(finalPos.x * 10) / 10;
    piece.connectors[1].y = Math.round(finalPos.y * 10) / 10;
    piece.connectors[1].angle = normAngle(snappedInfo.angle + 180);
    piece.connectors[1].connectedTo = { pieceId: snappedInfo.pieceId, connectorIndex: snappedInfo.connectorIndex };

    if (allPieces) {
      const targetPiece = allPieces.find((p) => p.id === snappedInfo!.pieceId);
      if (targetPiece && targetPiece.connectors[snappedInfo.connectorIndex]) {
        targetPiece.connectors[snappedInfo.connectorIndex]!.connectedTo = {
          pieceId: piece.id,
          connectorIndex: 1,
        };
      }
    }
  } else if (piece.connectors[1] && oldConn1ConnectedTo) {
    // Unhook: Wenn zuvor verbunden war und nun ins Leere gezogen wurde
    if (allPieces) {
      const oldTargetId = oldConn1ConnectedTo.pieceId;
      const oldTargetIdx = oldConn1ConnectedTo.connectorIndex;
      const oldTarget = allPieces.find((p) => p.id === oldTargetId);
      if (oldTarget?.connectors[oldTargetIdx]?.connectedTo?.pieceId === piece.id) {
        oldTarget.connectors[oldTargetIdx]!.connectedTo = undefined;
      }
    }
    piece.connectors[1].connectedTo = undefined;
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

/**
 * Berechnet die Weltkoordinaten eines Punkts entlang des Flexgleises bei Parameter t in [0, 1].
 * t = 0.0 ist Connector 0, t = 0.5 ist der Mittelpunkt, t = 1.0 ist Connector 1.
 */
export function getFlexTrackPointAt(piece: TrackPiece, entry: CatalogEntry, t: number): Vec2 {
  const clampedT = Math.max(0, Math.min(1, t));

  if (piece.customOffsets && piece.customOffsets.length >= 3) {
    const { h } = evaluateFlexSpline(piece.customOffsets, clampedT);
    const c0 = piece.connectors[0];
    const c1 = piece.connectors[1];
    if (c0 && c1) {
      const ux = c1.x - c0.x;
      const uy = c1.y - c0.y;
      const d = Math.hypot(ux, uy);
      if (d > 0.001) {
        const nx = -uy / d;
        const ny = ux / d;
        return {
          x: c0.x + clampedT * ux + h * nx,
          y: c0.y + clampedT * uy + h * ny,
        };
      }
    }
    const chordLen = piece.customChordDist ?? piece.customLength ?? entry.length ?? 900;
    return transformPoint({ x: clampedT * chordLen, y: h }, piece.transform);
  }

  const len = piece.customLength ?? entry.length ?? 900;
  const radius = piece.customRadius;
  const angle = piece.customAngle;

  if (radius && angle && Math.abs(angle) > 0.1 && radius > 0) {
    const curAngle = clampedT * angle;
    let localX: number;
    let localY: number;
    if (curAngle < 0) {
      const rad = deg2rad(Math.abs(curAngle));
      localX = radius * Math.sin(rad);
      localY = -radius * (1 - Math.cos(rad));
    } else {
      const rad = deg2rad(curAngle);
      localX = radius * Math.sin(rad);
      localY = radius * (1 - Math.cos(rad));
    }
    return transformPoint({ x: localX, y: localY }, piece.transform);
  }

  // Gerades Gleis: Punkt liegt bei t * len entlang der lokalen X-Achse
  return transformPoint({ x: clampedT * len, y: 0 }, piece.transform);
}

/**
 * Berechnet die Weltkoordinaten des Mittelpunkts eines Flexgleises (t = 0.5).
 */
export function getFlexTrackMidpoint(piece: TrackPiece, entry: CatalogEntry): Vec2 {
  return getFlexTrackPointAt(piece, entry, 0.5);
}

/**
 * Biegt ein Flexgleis über einen seiner Pivot-Punkte (z. B. t = 0.25, 0.5 oder 0.75),
 * während BEIDE Enden (Connector 0 und Connector 1) exakt an ihren festen Weltpositionen verankert bleiben.
 * Jeder Pivot-Punkt verformt die Krümmung an dieser Stelle zwischen den Nachbarpunkten unabhängig,
 * sodass auch gegenläufige Kurven (S-Kurven) in verschiedene Richtungen möglich sind.
 */
export function bendFlexTrackBetweenFixedEnds(
  piece: TrackPiece,
  entry: CatalogEntry,
  fixedC0: Vec2,
  fixedC1: Vec2,
  targetWorldPos: Vec2,
  _layers?: Layer[],
  pivotFraction = 0.5,
): void {
  const maxLen = entry.length ?? 900;
  const ux = fixedC1.x - fixedC0.x;
  const uy = fixedC1.y - fixedC0.y;
  const chordDist = Math.hypot(ux, uy);
  if (chordDist < 10) return;

  const chordAngleRad = Math.atan2(uy, ux);
  const chordAngleDeg = normAngle((chordAngleRad * 180) / Math.PI);

  // Normaleneinheitsvektor zur Sehne (zeigt 90° gegen den Uhrzeigersinn)
  const nx = -uy / chordDist;
  const ny = ux / chordDist;

  // Bestimme, welcher Pivot-Punkt gezogen wird (0: ~0.25, 1: ~0.50, 2: ~0.75)
  const pivotIdx = pivotFraction <= 0.35 ? 0 : pivotFraction >= 0.65 ? 2 : 1;
  const frac = pivotIdx === 0 ? 0.25 : pivotIdx === 2 ? 0.75 : 0.5;

  // Ruhelage des gezogenen Pivot-Punkts auf der ungebogenen Sehne
  const p0x = fixedC0.x + frac * ux;
  const p0y = fixedC0.y + frac * uy;

  // Vorzeichenbehaftete Auslenkung des Zielpunkts senkrecht zur Sehne
  const deltaPerp = (targetWorldPos.x - p0x) * nx + (targetWorldPos.y - p0y) * ny;

  // Bestehende Verbindungen sichern
  const oldConn0ConnectedTo = piece.connectors[0]?.connectedTo;
  const oldConn1ConnectedTo = piece.connectors[1]?.connectedTo;

  // Bisherige Offsets initialisieren oder übernehmen
  let offsets = piece.customOffsets ? [...piece.customOffsets] : undefined;
  if (!offsets || offsets.length < 3) {
    if (piece.customRadius && piece.customAngle && Math.abs(piece.customAngle) > 0.1) {
      const halfA = deg2rad(Math.abs(piece.customAngle) / 2);
      const sign = -Math.sign(piece.customAngle);
      const hMid = sign * piece.customRadius * (1 - Math.cos(halfA));
      offsets = [0.75 * hMid, hMid, 0.75 * hMid];
    } else {
      offsets = [0, 0, 0];
    }
  }

  const isCurrentlyStraight =
    Math.abs(offsets[0]!) < 2.5 && Math.abs(offsets[1]!) < 2.5 && Math.abs(offsets[2]!) < 2.5;

  if (pivotIdx === 1 && isCurrentlyStraight) {
    // Wenn an einem geraden Gleis der Mittelpunkt gezogen wird: gleichmäßige Standard-Wölbung
    offsets[1] = Math.round(deltaPerp * 10) / 10;
    offsets[0] = Math.round(deltaPerp * 0.75 * 10) / 10;
    offsets[2] = Math.round(deltaPerp * 0.75 * 10) / 10;
  } else {
    // Spezifischen Pivot-Punkt an dieser Stelle verformen (unterstützt auch gegenläufige Richtung)
    offsets[pivotIdx] = Math.round(deltaPerp * 10) / 10;
  }

  const allNearZero =
    Math.abs(offsets[0]!) < 2.5 && Math.abs(offsets[1]!) < 2.5 && Math.abs(offsets[2]!) < 2.5;

  if (allNearZero) {
    // Snappen auf gerade Linie: wenn alle Punkte nah an der Sehne liegen
    piece.customOffsets = undefined;
    piece.customChordDist = undefined;
    piece.customRadius = undefined;
    piece.customAngle = undefined;
    piece.customLength = Math.min(maxLen, Math.max(15, Math.round(chordDist * 10) / 10));
    piece.transform = {
      x: Math.round(fixedC0.x * 10) / 10,
      y: Math.round(fixedC0.y * 10) / 10,
      rotation: Math.round(chordAngleDeg * 10) / 10,
    };
  } else {
    // Gesamte Bogenlänge berechnen und ggf. auf maxLen begrenzen
    let splineLen = computeFlexSplineLength(chordDist, offsets);
    if (splineLen > maxLen) {
      const scale = maxLen / splineLen;
      offsets = offsets.map((h) => Math.round(h * scale * 10) / 10);
      splineLen = maxLen;
    }

    piece.customOffsets = offsets;
    piece.customChordDist = Math.round(chordDist * 10) / 10;
    piece.customLength = Math.min(maxLen, Math.max(15, Math.round(splineLen * 10) / 10));

    // Falls symmetrischer Einzelbogen (z. B. durch Mittelpunktzug): Radius & Winkel für Kompatibilität mitberechnen
    const isSymmetricArc =
      Math.abs(offsets[0]! - offsets[2]!) < 1.0 &&
      Math.abs(offsets[0]! - 0.75 * offsets[1]!) < 2.0;
    if (isSymmetricArc && Math.abs(offsets[1]!) >= 2.5) {
      const absH = Math.abs(offsets[1]!);
      const halfD = chordDist / 2;
      let R = (halfD * halfD + absH * absH) / (2 * absH);
      if (R < 80) R = 80;
      const sinHalfAlpha = Math.min(1.0, halfD / R);
      const alphaRad = 2 * Math.asin(sinHalfAlpha);
      const alphaDeg = (alphaRad * 180) / Math.PI;
      const signedAngle = -Math.sign(offsets[1]!) * alphaDeg;
      piece.customRadius = Math.round(R * 10) / 10;
      piece.customAngle = Math.round(signedAngle * 10) / 10;
    } else {
      piece.customRadius = undefined;
      piece.customAngle = undefined;
    }

    piece.transform = {
      x: Math.round(fixedC0.x * 10) / 10,
      y: Math.round(fixedC0.y * 10) / 10,
      rotation: Math.round(chordAngleDeg * 10) / 10,
    };
  }

  // Connectors neu in Weltkoordinaten berechnen
  piece.connectors = computeWorldConnectors(entry, piece.transform, piece);

  // Beide Endpunkte exakt auf den fixierten Koordinaten halten
  if (piece.connectors[0]) {
    piece.connectors[0].x = Math.round(fixedC0.x * 10) / 10;
    piece.connectors[0].y = Math.round(fixedC0.y * 10) / 10;
    if (oldConn0ConnectedTo) piece.connectors[0].connectedTo = oldConn0ConnectedTo;
  }
  if (piece.connectors[1]) {
    piece.connectors[1].x = Math.round(fixedC1.x * 10) / 10;
    piece.connectors[1].y = Math.round(fixedC1.y * 10) / 10;
    if (oldConn1ConnectedTo) piece.connectors[1].connectedTo = oldConn1ConnectedTo;
  }
}
