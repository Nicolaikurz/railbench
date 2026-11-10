import type { Connector, Transform } from "@/core/types";
import { deg2rad, normAngle, transformPoint } from "@/core/geometry";
import type { LocalConnector, CatalogEntry, Catalog } from "./types";
import { maerklinCCatalog, maerklinCEntries } from "./systems/maerklin-c-h0";

// Re-export aller Typen und Gleissysteme
export * from "./types";
export * from "./registry";
export * from "./systems/maerklin-c-h0";
export * from "./systems/trix-c-h0";
export * from "./systems/minitrix-n";
export * from "./systems/rokuhan-z"
export * from "./systems/maerklin-m-h0";
export * from "./systems/maerklin-k-h0";
export * from "./systems/maerklin-z";
export * from "./systems/piko-a-h0";
export * from "./systems/roco-line-h0";
export * from "./systems/fleischmann-piccolo-n";
export * from "./systems/tillig-tt";

export * from "./systems/generic-streets-h0";
export * from "./systems/generic-streets-n";
export * from "./systems/generic-streets-tt";
export * from "./systems/generic-streets-z";


// Abwärtskompatibilität
export const cGleisCatalogEntries = maerklinCEntries;

export interface C_Gleise {
  id: string;
  name: string;
  description: string;
}

export const cGleisCatalog: C_Gleise[] = maerklinCEntries.map((e) => ({
  id: e.id,
  name: e.name,
  description: e.description,
}));

/**
 * Berechnet die relativen Connector-Positionen und -Winkel (im lokalen Koordinatensystem des Gleisstücks).
 * Funktioniert universell für alle Gleissysteme und Spurweiten.
 */
export function getLocalConnectors(entry?: CatalogEntry | null): LocalConnector[] {
  if (!entry) {
    return [
      { x: 0, y: 0, angle: 180 },
      { x: 100, y: 0, angle: 0 },
    ];
  }

  if (entry.kind !== "flex" && entry.localConnectors && entry.localConnectors.length > 0) {
    return entry.localConnectors;
  }

  const kind = entry.kind;

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const spacing = 77.4;
    const count = 5;
    const connectors: LocalConnector[] = [];
    for (let i = 0; i < count; i++) {
      connectors.push({ x: 0, y: i * spacing, angle: 180 });
      connectors.push({ x: l, y: i * spacing, angle: 0 });
    }
    return connectors;
  }

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    if (kind === "flex" && entry.radius && entry.angle && Math.abs(entry.angle) > 0.1) {
      const r = entry.radius;
      const a = entry.angle;
      if (a < 0) {
        const rad = deg2rad(Math.abs(a));
        return [
          { x: 0, y: 0, angle: 180 },
          { x: r * Math.sin(rad), y: -r * (1 - Math.cos(rad)), angle: normAngle(a) },
        ];
      }
      const rad = deg2rad(a);
      return [
        { x: 0, y: 0, angle: 180 },
        { x: r * Math.sin(rad), y: r * (1 - Math.cos(rad)), angle: normAngle(a) },
      ];
    }
    const l = entry.length ?? (kind === "flex" ? 900 : 188.3);
    return [
      { x: 0, y: 0, angle: 180 },
      { x: l, y: 0, angle: 0 },
    ];
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      const rad = deg2rad(Math.abs(a));
      return [
        { x: 0, y: 0, angle: 180 },
        { x: r * Math.sin(rad), y: -r * (1 - Math.cos(rad)), angle: normAngle(a) },
      ];
    }
    const rad = deg2rad(a);
    // Standardbogen biegt nach links (positiver y-Bereich):
    const ex = r * Math.sin(rad);
    const ey = r * (1 - Math.cos(rad));
    return [
      { x: 0, y: 0, angle: 180 },
      { x: ex, y: ey, angle: normAngle(a) },
    ];
  }

  if (kind === "turnout-left") {
    // Falls keine Länge, aber Radius definiert ist, als Bogenweiche behandeln
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const rad1 = deg2rad(a1);
      const rad2 = deg2rad(a2);
      return [
        { x: 0, y: 0, angle: 180 },
        { x: r1 * Math.sin(rad1), y: r1 * (1 - Math.cos(rad1)), angle: normAngle(a1) },
        { x: lead + r2 * Math.sin(rad2), y: r2 * (1 - Math.cos(rad2)), angle: normAngle(a2) },
      ];
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const rad = deg2rad(ba);
    const bx = br * Math.sin(rad);
    const by = br * (1 - Math.cos(rad));
    return [
      { x: 0, y: 0, angle: 180 },
      { x: l, y: 0, angle: 0 },
      { x: bx, y: by, angle: normAngle(ba) },
    ];
  }

  if (kind === "turnout-right") {
    // Falls keine Länge, aber Radius definiert ist, als Bogenweiche behandeln
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const rad1 = deg2rad(a1);
      const rad2 = deg2rad(a2);
      return [
        { x: 0, y: 0, angle: 180 },
        { x: r1 * Math.sin(rad1), y: -r1 * (1 - Math.cos(rad1)), angle: normAngle(-a1) },
        { x: lead + r2 * Math.sin(rad2), y: -r2 * (1 - Math.cos(rad2)), angle: normAngle(-a2) },
      ];
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const rad = deg2rad(ba);
    const bx = br * Math.sin(rad);
    const by = -br * (1 - Math.cos(rad));
    return [
      { x: 0, y: 0, angle: 180 },
      { x: l, y: 0, angle: 0 },
      { x: bx, y: by, angle: normAngle(-ba) },
    ];
  }

  if (kind === "curved-turnout-left") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const rad1 = deg2rad(a1);
    const rad2 = deg2rad(a2);
    return [
      { x: 0, y: 0, angle: 180 },
      { x: r1 * Math.sin(rad1), y: r1 * (1 - Math.cos(rad1)), angle: normAngle(a1) },
      { x: lead + r2 * Math.sin(rad2), y: r2 * (1 - Math.cos(rad2)), angle: normAngle(a2) },
    ];
  }

  if (kind === "curved-turnout-right") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const rad1 = deg2rad(a1);
    const rad2 = deg2rad(a2);
    return [
      { x: 0, y: 0, angle: 180 },
      { x: r1 * Math.sin(rad1), y: -r1 * (1 - Math.cos(rad1)), angle: normAngle(-a1) },
      { x: lead + r2 * Math.sin(rad2), y: -r2 * (1 - Math.cos(rad2)), angle: normAngle(-a2) },
    ];
  }

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const rad = deg2rad(ba);
    const bx = br * Math.sin(rad);
    const byL = br * (1 - Math.cos(rad));
    const byR = -br * (1 - Math.cos(rad));
    return [
      { x: 0, y: 0, angle: 180 },
      { x: l, y: 0, angle: 0 },
      { x: bx, y: byL, angle: normAngle(ba) },
      { x: bx, y: byR, angle: normAngle(-ba) },
    ];
  }

  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    return [
      { x: -h, y: 0, angle: 180 },
      { x: h, y: 0, angle: 0 },
      { x: -h * Math.cos(rad), y: -h * Math.sin(rad), angle: normAngle(180 + a) },
      { x: h * Math.cos(rad), y: h * Math.sin(rad), angle: normAngle(a) },
    ];
  }

  if (kind === "dkw") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    return [
      { x: 0, y: 0, angle: 180 },
      { x: l, y: 0, angle: 0 },
      { x: cx - h * Math.cos(rad), y: -h * Math.sin(rad), angle: normAngle(180 + a) },
      { x: cx + h * Math.cos(rad), y: h * Math.sin(rad), angle: normAngle(a) },
    ];
  }

  // Fallback
  return [
    { x: 0, y: 0, angle: 180 },
    { x: 100, y: 0, angle: 0 },
  ];
}

/** Transformiert die lokalen Connectors in Weltkoordinaten basierend auf der Position und Rotation des Gleisstücks. */
export function computeWorldConnectors(
  entry: CatalogEntry | undefined | null,
  transform: Transform,
  piece?: { customLength?: number; customRadius?: number; customAngle?: number },
): Connector[] {
  if (!entry) return [];
  let effectiveEntry = entry;
  if (piece && (piece.customLength !== undefined || piece.customRadius !== undefined || piece.customAngle !== undefined)) {
    const isCurved = Boolean(piece.customRadius && piece.customAngle && Math.abs(piece.customAngle) > 0.1);
    effectiveEntry = {
      ...entry,
      localConnectors: undefined,
      length: piece.customLength ?? entry.length,
      radius: piece.customRadius ?? entry.radius,
      angle: piece.customAngle ?? entry.angle,
      kind: isCurved ? "curve" : entry.kind === "flex" ? "straight" : entry.kind,
    };
  }
  const locals = getLocalConnectors(effectiveEntry);
  if (!transform) return [];
  return locals.map((lc) => {
    const wp = transformPoint(lc, transform);
    return {
      x: wp.x,
      y: wp.y,
      angle: normAngle((transform.rotation ?? 0) + lc.angle),
    };
  });
}

/** Lädt einen Katalog über eine URL */
export async function loadCatalog(url: string): Promise<Catalog> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Katalog nicht ladbar: ${url}`);
  return res.json();
}