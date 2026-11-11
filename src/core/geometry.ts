import type { Vec2, Transform, TrackPiece } from "./types";
import type { CatalogEntry } from "@/catalog/catalog";

export const deg2rad = (d: number) => (d * Math.PI) / 180;
export const rad2deg = (r: number) => (r * 180) / Math.PI;
export const normAngle = (a: number) => ((a % 360) + 360) % 360;

/**
 * Berechnet den Gegen-Rotationswinkel (in Radiant) per Vektor-Winkelberechnung,
 * um Beschriftungen (z. B. Gleis-Artikelnummern, Gebäudenamen, Plaketten) innerhalb
 * eines rotierten Canvas/SVG-Containers stets horizontal und von links nach rechts
 * lesbar (0° im Weltkoordinatensystem, kein Kopfstand) darzustellen.
 */
export function getUprightTextRotation(containerRotationDeg: number): number {
  return -deg2rad(normAngle(containerRotationDeg));
}

export function rotate(p: Vec2, deg: number): Vec2 {
  const r = deg2rad(deg);
  return {
    x: p.x * Math.cos(r) - p.y * Math.sin(r),
    y: p.x * Math.sin(r) + p.y * Math.cos(r),
  };
}

export const dist = (a: Vec2, b: Vec2) => Math.hypot(a.x - b.x, a.y - b.y);

export function transformPoint(p: Vec2, t: { x: number; y: number; rotation: number }): Vec2 {
  const rot = rotate(p, t.rotation);
  return { x: t.x + rot.x, y: t.y + rot.y };
}

export function untransformPoint(p: Vec2, t: { x: number; y: number; rotation: number }): Vec2 {
  const dx = p.x - t.x;
  const dy = p.y - t.y;
  return rotate({ x: dx, y: dy }, -t.rotation);
}

export function angleDiff(a: number, b: number): number {
  const diff = Math.abs(normAngle(a) - normAngle(b));
  return diff > 180 ? 360 - diff : diff;
}

/** Berechnet den kürzesten Abstand eines Punkts zu einer Linienstrecke A-B. */
export function distToSegment(p: Vec2, a: Vec2, b: Vec2): number {
  const l2 = (b.x - a.x) ** 2 + (b.y - a.y) ** 2;
  if (l2 === 0) return dist(p, a);
  let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2;
  t = Math.max(0, Math.min(1, t));
  return dist(p, { x: a.x + t * (b.x - a.x), y: a.y + t * (b.y - a.y) });
}

/** Berechnet den kürzesten Abstand eines lokalen Punkts zu einem Kreisbogen. */
export function distToArc(
  p: Vec2,
  cx: number,
  cy: number,
  radius: number,
  startAng: number,
  endAng: number,
  counterclockwise = false,
): number {
  const dx = p.x - cx;
  const dy = p.y - cy;
  const dToCenter = Math.hypot(dx, dy);

  let ang = Math.atan2(dy, dx);

  // Winkel normalisieren
  function normalizeAngle(val: number) {
    let a = val % (2 * Math.PI);
    if (a < 0) a += 2 * Math.PI;
    return a;
  }

  const aNorm = normalizeAngle(ang);
  const sNorm = normalizeAngle(startAng);
  const eNorm = normalizeAngle(endAng);

  let inArc = false;
  if (!counterclockwise) {
    // Im Uhrzeigersinn (start -> end)
    if (sNorm <= eNorm) {
      inArc = aNorm >= sNorm && aNorm <= eNorm;
    } else {
      inArc = aNorm >= sNorm || aNorm <= eNorm;
    }
  } else {
    // Gegen den Uhrzeigersinn
    if (sNorm >= eNorm) {
      inArc = aNorm <= sNorm && aNorm >= eNorm;
    } else {
      inArc = aNorm <= sNorm || aNorm >= eNorm;
    }
  }

  const pStart: Vec2 = { x: cx + radius * Math.cos(startAng), y: cy + radius * Math.sin(startAng) };
  const pEnd: Vec2 = { x: cx + radius * Math.cos(endAng), y: cy + radius * Math.sin(endAng) };

  if (inArc) {
    return Math.abs(dToCenter - radius);
  }
  return Math.min(dist(p, pStart), dist(p, pEnd));
}

/**
 * Berechnet den exakten Abstand eines Welt-Punkts zur Geometrie eines Gleisstücks.
 * Ermöglicht präzises Anklicken/Selektieren auf dem gesamten Schotterbett & Schienenverlauf!
 */
export function distanceToTrackPiece(
  worldPoint: Vec2,
  pieceTransform: Transform,
  entry: CatalogEntry,
): number {
  const local = untransformPoint(worldPoint, pieceTransform);
  const kind = entry.kind;

  if (entry.isRoad) {
    if (kind === "straight" || kind === "accessory") {
      const l = entry.length ?? 200;
      if (entry.id === "ST-LOOP") {
        const dNeck = distToSegment(local, { x: 0, y: 0 }, { x: 45, y: 0 });
        const dDist = Math.hypot(local.x - 45, local.y);
        const dLoop = Math.max(0, dDist - 45);
        return Math.min(dNeck, dLoop);
      }
      return distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
    }
    if (kind === "curve") {
      const r = entry.radius ?? 200;
      const a = entry.angle ?? 90;
      const rad = deg2rad(a);
      return distToArc(local, 0, r, r, -Math.PI / 2, -Math.PI / 2 + rad, false);
    }
    if (kind === "turnout-left") {
      const l = entry.length ?? 200;
      const dStraight = distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
      if (entry.branchLength !== undefined) {
        const midX = l / 2;
        const dBranch = distToSegment(local, { x: midX, y: 0 }, { x: midX, y: entry.branchLength });
        return Math.min(dStraight, dBranch);
      }
      if (entry.branchRadius !== undefined) {
        const br = entry.branchRadius;
        const ba = entry.branchAngle ?? 45;
        const dBranch = distToArc(local, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), false);
        return Math.min(dStraight, dBranch);
      }
      return dStraight;
    }
    if (kind === "crossing") {
      const l = entry.length ?? 200;
      const d1 = distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
      const midX = l / 2;
      const d2 = distToSegment(local, { x: midX, y: -midX }, { x: midX, y: midX });
      return Math.min(d1, d2);
    }
  }

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? (kind === "flex" ? 900 : 188.3);
    return distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      return distToArc(local, 0, -r, r, Math.PI / 2, Math.PI / 2 - deg2rad(Math.abs(a)), true);
    }
    const rad = deg2rad(a);
    return distToArc(local, 0, r, r, -Math.PI / 2, -Math.PI / 2 + rad, false);
  }

  if (kind === "turnout-left") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const d1 = distToArc(local, 0, r1, r1, -Math.PI / 2, -Math.PI / 2 + deg2rad(a1), false);
      const dLead = lead > 0 ? distToSegment(local, { x: 0, y: 0 }, { x: lead, y: 0 }) : Infinity;
      const d2 = distToArc(local, lead, r2, r2, -Math.PI / 2, -Math.PI / 2 + deg2rad(a2), false);
      return Math.min(d1, dLead, d2);
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const dStraight = distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
    const dBranch = distToArc(local, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), false);
    return Math.min(dStraight, dBranch);
  }

  if (kind === "turnout-right") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const d1 = distToArc(local, 0, -r1, r1, Math.PI / 2, Math.PI / 2 - deg2rad(a1), true);
      const dLead = lead > 0 ? distToSegment(local, { x: 0, y: 0 }, { x: lead, y: 0 }) : Infinity;
      const d2 = distToArc(local, lead, -r2, r2, Math.PI / 2, Math.PI / 2 - deg2rad(a2), true);
      return Math.min(d1, dLead, d2);
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const dStraight = distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
    const dBranch = distToArc(local, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), true);
    return Math.min(dStraight, dBranch);
  }

  if (kind === "curved-turnout-left") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const d1 = distToArc(local, 0, r1, r1, -Math.PI / 2, -Math.PI / 2 + deg2rad(a1), false);
    const dLead = lead > 0 ? distToSegment(local, { x: 0, y: 0 }, { x: lead, y: 0 }) : Infinity;
    const d2 = distToArc(local, lead, r2, r2, -Math.PI / 2, -Math.PI / 2 + deg2rad(a2), false);
    return Math.min(d1, dLead, d2);
  }

  if (kind === "curved-turnout-right") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const d1 = distToArc(local, 0, -r1, r1, Math.PI / 2, Math.PI / 2 - deg2rad(a1), true);
    const dLead = lead > 0 ? distToSegment(local, { x: 0, y: 0 }, { x: lead, y: 0 }) : Infinity;
    const d2 = distToArc(local, lead, -r2, r2, Math.PI / 2, Math.PI / 2 - deg2rad(a2), true);
    return Math.min(d1, dLead, d2);
  }

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const dStraight = distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
    const dBranchL = distToArc(local, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), false);
    const dBranchR = distToArc(local, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), true);
    return Math.min(dStraight, dBranchL, dBranchR);
  }

  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    const d1 = distToSegment(local, { x: -h, y: 0 }, { x: h, y: 0 });
    const d2 = distToSegment(
      local,
      { x: -h * Math.cos(rad), y: -h * Math.sin(rad) },
      { x: h * Math.cos(rad), y: h * Math.sin(rad) },
    );
    return Math.min(d1, d2);
  }

  if (kind === "double-slip") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    const d1 = distToSegment(local, { x: 0, y: 0 }, { x: l, y: 0 });
    const d2 = distToSegment(
      local,
      { x: cx - h * cos, y: -h * sin },
      { x: cx + h * cos, y: h * sin },
    );
    const dTurn1 = distToArc(local, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), false);
    const dTurn2 = distToArc(local, l, -br, br, Math.PI / 2, Math.PI / 2 + deg2rad(ba), false);
    return Math.min(d1, d2, dTurn1, dTurn2);
  }

  if (kind === "turntable") {
    const d = Math.hypot(local.x, local.y);
    const outerR = entry.radius ?? 189;
    return d <= outerR ? 0 : d - outerR;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const topY = -55.2;
    const plateH = 420;
    const clampedX = Math.max(0, Math.min(local.x, l));
    const clampedY = Math.max(topY, Math.min(local.y, topY + plateH));
    return Math.hypot(local.x - clampedX, local.y - clampedY);
  }

  return dist(local, { x: 0, y: 0 });
}

/** Ermittelt den geometrischen Mittelpunkt eines Gleis- oder Straßenstücks im lokalen Koordinatensystem. */
export function getPieceLocalCenter(entry: CatalogEntry): Vec2 {
  const kind = entry.kind;

  if (entry.isRoad) {
    if (kind === "straight" || kind === "accessory" || kind === "turnout-left" || kind === "turnout-right" || kind === "crossing") {
      const l = entry.length ?? 200;
      return { x: l / 2, y: 0 };
    }
    if (kind === "curve") {
      const r = entry.radius ?? 200;
      const a = entry.angle ?? 90;
      const halfRad = deg2rad(a / 2);
      return {
        x: r * Math.sin(halfRad),
        y: r * (1 - Math.cos(halfRad)),
      };
    }
  }

  if (kind === "curved-turnout-left" || (kind === "turnout-left" && entry.radius && !entry.length)) {
    const r = ((entry.radius ?? 360) + (entry.branchRadius ?? 437.5)) / 2;
    const a = ((entry.angle ?? 30) + (entry.branchAngle ?? 30)) / 2;
    const halfRad = deg2rad(a / 2);
    return {
      x: r * Math.sin(halfRad),
      y: r * (1 - Math.cos(halfRad)),
    };
  }
  if (kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length)) {
    const r = ((entry.radius ?? 360) + (entry.branchRadius ?? 437.5)) / 2;
    const a = ((entry.angle ?? 30) + (entry.branchAngle ?? 30)) / 2;
    const halfRad = deg2rad(a / 2);
    return {
      x: r * Math.sin(halfRad),
      y: -r * (1 - Math.cos(halfRad)),
    };
  }
  if (kind === "straight" || kind === "accessory" || kind === "flex" || kind === "turnout-left" || kind === "turnout-right" || kind === "three-way" || kind === "double-slip") {
    const l = entry.length ?? (kind === "flex" ? 900 : 188.3);
    return { x: l / 2, y: 0 };
  }
  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const halfRad = deg2rad(Math.abs(a) / 2);
    return {
      x: r * Math.sin(halfRad),
      y: a < 0 ? -r * (1 - Math.cos(halfRad)) : r * (1 - Math.cos(halfRad)),
    };
  }
  if (kind === "crossing" || kind === "turntable") {
    return { x: 0, y: 0 };
  }
  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    return { x: l / 2, y: 154.8 };
  }
  return { x: 50, y: 0 };
}

/** Ermittelt die Weltkoordinate des Dreh-Griffs (Rotation Handle) oberhalb des Gleis- oder Straßenstücks. */
export function getPieceRotationHandleWorld(
  entry: CatalogEntry,
  transform: Transform,
  stemOffset = entry.kind === "turntable" ? 220 : entry.kind === "transfer-table" ? 240 : entry.kind === "double-slip" ? 62 : entry.isRoad ? 50 : 48,
): Vec2 {
  const center = getPieceLocalCenter(entry);
  // Senkrecht zur Fahrtrichtung nach oben (negatives y im lokalen System)
  const localHandle: Vec2 = { x: center.x, y: center.y - stemOffset };
  return transformPoint(localHandle, transform);
}

/** Ermittelt den geometrischen Mittelpunkt in Weltkoordinaten. */
export function getPieceCenterWorld(entry: CatalogEntry, transform: Transform): Vec2 {
  const localCenter = getPieceLocalCenter(entry);
  return transformPoint(localCenter, transform);
}

/** Dreht ein Gleisstück um seinen eigenen geometrischen Mittelpunkt (Centrum). */
export function rotatePieceAroundCenter(
  entry: CatalogEntry,
  currentTransform: Transform,
  deltaDeg: number,
): Transform {
  const center = getPieceCenterWorld(entry, currentTransform);
  const newRotation = normAngle(currentTransform.rotation + deltaDeg);
  const localCenter = getPieceLocalCenter(entry);
  const rotatedOffset = rotate(localCenter, newRotation);
  return {
    x: center.x - rotatedOffset.x,
    y: center.y - rotatedOffset.y,
    rotation: newRotation,
  };
}

export interface RotatableItem {
  id: string;
  type: "piece" | "model";
  transform: Transform;
  entry?: CatalogEntry;
}

/** Berechnet den gemeinsamen Schwerpunkt (Centroid) einer Menge von Gleisstücken und Modellen. */
export function computeGroupCentroid(items: RotatableItem[]): Vec2 {
  if (items.length === 0) return { x: 0, y: 0 };
  let sumX = 0;
  let sumY = 0;
  for (const item of items) {
    if (item.type === "piece" && item.entry) {
      const c = getPieceCenterWorld(item.entry, item.transform);
      sumX += c.x;
      sumY += c.y;
    } else {
      sumX += item.transform.x;
      sumY += item.transform.y;
    }
  }
  return { x: sumX / items.length, y: sumY / items.length };
}

/**
 * Dreht eine Gruppe von Gleisstücken und Modellen starr als eine Einheit ("all as one part")
 * um ein gemeinsames Drehzentrum (standardmäßig der Schwerpunkt der Gruppe).
 */
export function rotateGroupAroundCentroid(
  items: RotatableItem[],
  deltaDeg: number,
  centroid?: Vec2,
): Map<string, Transform> {
  const center = centroid ?? computeGroupCentroid(items);
  const result = new Map<string, Transform>();

  for (const item of items) {
    if (item.type === "piece" && item.entry) {
      const c = getPieceCenterWorld(item.entry, item.transform);
      const offset = { x: c.x - center.x, y: c.y - center.y };
      const rotatedOffset = rotate(offset, deltaDeg);
      const newCenter = { x: center.x + rotatedOffset.x, y: center.y + rotatedOffset.y };

      const newRotation = normAngle(item.transform.rotation + deltaDeg);
      const localCenter = getPieceLocalCenter(item.entry);
      const rotLocal = rotate(localCenter, newRotation);
      result.set(item.id, {
        x: newCenter.x - rotLocal.x,
        y: newCenter.y - rotLocal.y,
        rotation: newRotation,
      });
    } else {
      const offset = { x: item.transform.x - center.x, y: item.transform.y - center.y };
      const rotatedOffset = rotate(offset, deltaDeg);
      result.set(item.id, {
        x: center.x + rotatedOffset.x,
        y: center.y + rotatedOffset.y,
        rotation: normAngle(item.transform.rotation + deltaDeg),
      });
    }
  }

  return result;
}

/** Prüft, ob ein Gleisstück innerhalb einer Auswahlbox (Weltkoordinaten) liegt oder diese schneidet. */
export function isPieceInSelectionBox(
  piece: TrackPiece,
  entry: CatalogEntry,
  boxStart: Vec2,
  boxEnd: Vec2,
): boolean {
  const minX = Math.min(boxStart.x, boxEnd.x);
  const maxX = Math.max(boxStart.x, boxEnd.x);
  const minY = Math.min(boxStart.y, boxEnd.y);
  const maxY = Math.max(boxStart.y, boxEnd.y);

  function isPointInside(p: Vec2): boolean {
    return p.x >= minX && p.x <= maxX && p.y >= minY && p.y <= maxY;
  }

  // 1. Prüfen, ob der Mittelpunkt in der Box liegt
  const center = getPieceCenterWorld(entry, piece.transform);
  if (isPointInside(center)) return true;

  // 2. Prüfen, ob der Ursprung in der Box liegt
  if (isPointInside(piece.transform)) return true;

  // 3. Prüfen, ob mindestens ein Connector (Endpunkt) in der Box liegt
  for (const c of piece.connectors) {
    if (isPointInside(c)) return true;
  }

  // 4. Prüfen, ob die Boxmitte nahe am Gleisstück liegt und das Gleis die Box schneidet
  const boxCenter = { x: (minX + maxX) / 2, y: (minY + maxY) / 2 };
  const dCenter = distanceToTrackPiece(boxCenter, piece.transform, entry);
  const boxRadius = Math.hypot(maxX - minX, maxY - minY) / 2;
  if (dCenter <= Math.min(20, boxRadius)) return true;

  // 5. Stützpunkte entlang des Gleises prüfen (z.B. bei langen Geraden oder Bögen)
  const kind = entry.kind;
  const isStraightTurnout = (kind === "turnout-left" || kind === "turnout-right") && (entry.length !== undefined || !entry.radius);
  const isCurvedTurnoutL = kind === "curved-turnout-left" || (kind === "turnout-left" && entry.radius && !entry.length);
  const isCurvedTurnoutR = kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length);
  const sampleSteps = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

  if (kind === "straight" || kind === "accessory" || isStraightTurnout || kind === "three-way" || kind === "double-slip") {
    const l = entry.length ?? 188.3;
    for (const t of sampleSteps) {
      const pt = transformPoint({ x: l * t, y: 0 }, piece.transform);
      if (isPointInside(pt)) return true;
    }
  } else if (kind === "curve" || isCurvedTurnoutL) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    for (const t of sampleSteps) {
      const ang = -Math.PI / 2 + deg2rad(a * t);
      const pt = transformPoint({ x: r * Math.cos(ang), y: r + r * Math.sin(ang) }, piece.transform);
      if (isPointInside(pt)) return true;
    }
    if (isCurvedTurnoutL) {
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r ? entry.branchRadius - r : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r) : (entry.branchRadius && entry.branchRadius > r ? r : (entry.branchRadius ?? r));
      const a2 = entry.branchAngle ?? 30;
      if (lead > 0) {
        const pt = transformPoint({ x: lead * 0.5, y: 0 }, piece.transform);
        if (isPointInside(pt)) return true;
      }
      for (const t of sampleSteps) {
        const ang = -Math.PI / 2 + deg2rad(a2 * t);
        const pt = transformPoint({ x: lead + r2 * Math.cos(ang), y: r2 + r2 * Math.sin(ang) }, piece.transform);
        if (isPointInside(pt)) return true;
      }
    }
  } else if (isCurvedTurnoutR) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    for (const t of sampleSteps) {
      const ang = Math.PI / 2 - deg2rad(a * t);
      const pt = transformPoint({ x: r * Math.cos(ang), y: -r + r * Math.sin(ang) }, piece.transform);
      if (isPointInside(pt)) return true;
    }
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r ? entry.branchRadius - r : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r) : (entry.branchRadius && entry.branchRadius > r ? r : (entry.branchRadius ?? r));
    const a2 = entry.branchAngle ?? 30;
    if (lead > 0) {
      const pt = transformPoint({ x: lead * 0.5, y: 0 }, piece.transform);
      if (isPointInside(pt)) return true;
    }
    for (const t of sampleSteps) {
      const ang = Math.PI / 2 - deg2rad(a2 * t);
      const pt = transformPoint({ x: lead + r2 * Math.cos(ang), y: -r2 + r2 * Math.sin(ang) }, piece.transform);
      if (isPointInside(pt)) return true;
    }
  } else if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const topY = -55.2;
    const plateH = 420;
    const pts = [
      { x: 0, y: topY },
      { x: l, y: topY },
      { x: l, y: topY + plateH },
      { x: 0, y: topY + plateH },
      { x: l / 2, y: topY + plateH / 2 },
    ];
    for (const p of pts) {
      if (isPointInside(transformPoint(p, piece.transform))) return true;
    }
  }

  return false;
}
