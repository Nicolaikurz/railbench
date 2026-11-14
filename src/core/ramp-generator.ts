import type { TrackPiece, Layer, Connector } from "./types";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { dist, normAngle, rotate } from "./geometry";
import { getPieceConnectorHeight } from "./snapping";
import { getLocalConnectors, computeWorldConnectors } from "@/catalog/catalog";
import { getTrackPieceLength } from "./ramp";

/**
 * Standard minimum clearance height above tracks (Durchfahrtshöhe / lichte Höhe nach NEM 102)
 * for each model railway scale / ratio. Includes safety margin for roadbed and catenary.
 */
export const MIN_CLEARANCE_HEIGHT_BY_SCALE: Record<string, number> = {
  Z: 40,   // Spur Z (1:220): NEM 102 ~33mm, mit Gleis & Trasse mind. 40mm
  N: 50,   // Spur N (1:160): NEM 102 ~40mm, mit Oberleitung & Bettung mind. 50mm
  TT: 60,  // Spur TT (1:120): NEM 102 ~52mm, mit Oberleitung & Trasse mind. 60mm
  H0: 80,  // Spur H0 (1:87): NEM 102 59mm (ohne OL) / 75-80mm (mit OL & Brückentrassen)
  "0": 120, // Spur 0 (1:45): NEM 102 ~105mm, mit Brückentrasse mind. 120mm
  "1": 170, // Spur 1 (1:32): NEM 102 ~150mm, mit Trasse mind. 170mm
  G: 220,  // Spur G (1:22.5): Großbahn mind. 220mm
};

export function getMinClearanceHeightForScale(scale?: string): number {
  const norm = scale?.toUpperCase().trim() || "H0";
  return MIN_CLEARANCE_HEIGHT_BY_SCALE[norm] ?? 80;
}

export type RampShapeType = "straight" | "curved" | "helix";

export interface RampGeneratorParams {
  anchorPiece: TrackPiece;
  anchorConnectorIndex?: number;
  connectorIndex?: number;
  targetHeightMm: number;
  targetLayerId?: string;
  rampShape?: RampShapeType;
  shape?: RampShapeType;
  curveDirection?: "left" | "right";
  direction?: "left" | "right";
  curveAngleDeg?: number; // e.g. 90, 180, 270, 360, 720
  catalog: Catalog;
  layers?: Layer[];
  allPieces?: TrackPiece[];
  targetGradientPercent?: number; // default ~2.8%
  maxGradientPercent?: number;
  customPieceCount?: number;
  selectedCurveCatalogId?: string;
  selectedCurveId?: string;
}

export interface CurveRadiusOption {
  catalogId: string;
  name: string;
  radiusMm: number;
  angleDeg: number;
  label: string;
  entry: CatalogEntry;
}

export interface GeneratedRampResult {
  success: boolean;
  reason?: "no_open_connector" | "same_height" | "no_suitable_pieces";
  pieces: TrackPiece[];
  totalLengthMm: number;
  totalRiseMm: number;
  gradientPercent: number;
  startHeightMm: number;
  endHeightMm: number;
  targetHeightMm?: number;
  chosenConnectorIndex: number;
}

/**
 * Finds all open (unconnected) connectors on a track piece.
 */
export function getOpenConnectors(
  piece: TrackPiece,
  allPieces: TrackPiece[],
  distTolerance = 8.0,
  layers?: Layer[],
): number[] {
  const openIndices: number[] = [];
  for (let i = 0; i < piece.connectors.length; i++) {
    const conn = piece.connectors[i]!;
    const h1 = getPieceConnectorHeight(piece, i, layers);
    const isConnected = allPieces.some((other) => {
      if (other.id === piece.id) return false;
      return other.connectors.some((c, oi) => {
        if (dist(conn, c) > distTolerance) return false;
        const h2 = getPieceConnectorHeight(other, oi, layers);
        return Math.abs(h1 - h2) <= 2.0;
      });
    });
    if (!isConnected) {
      openIndices.push(i);
    }
  }
  return openIndices;
}

/**
 * Finds the most suitable standard straight and curved track entries in the catalog.
 */
export function findStandardTrackEntries(catalog: Catalog): {
  straight?: CatalogEntry;
  curve?: CatalogEntry;
  straightEntry?: CatalogEntry;
  curveEntry?: CatalogEntry;
} {
  // Straight: prefer standard lengths (180-250 mm in H0, or longest straight <= 350 mm)
  const straights = catalog.entries.filter((e) => e.kind === "straight" && (e.length ?? 0) > 40);
  straights.sort((a, b) => {
    // Prefer lengths close to ~188 mm
    const diffA = Math.abs((a.length ?? 0) - 188.3);
    const diffB = Math.abs((b.length ?? 0) - 188.3);
    return diffA - diffB;
  });
  const straightEntry = straights[0] || catalog.entries.find((e) => e.kind === "straight");

  // Curve: prefer standard radius (>= 350 mm in H0) and ~30 degree angle
  const curves = catalog.entries.filter((e) => e.kind === "curve" && (e.radius ?? 0) > 100);
  curves.sort((a, b) => {
    // Prefer angle of 30 deg
    const aDiff = Math.abs((a.angle ?? 0) - 30);
    const bDiff = Math.abs((b.angle ?? 0) - 30);
    if (aDiff !== bDiff) return aDiff - bDiff;
    return (b.radius ?? 0) - (a.radius ?? 0);
  });
  const curveEntry = curves[0] || catalog.entries.find((e) => e.kind === "curve");

  return { straight: straightEntry, curve: curveEntry, straightEntry, curveEntry };
}

/**
 * Returns all distinct curve radius options available in the catalog.
 */
export function getAvailableCurveRadii(catalog: Catalog): CurveRadiusOption[] {
  const curves = catalog.entries.filter(
    (e) => e.kind === "curve" && (e.radius ?? 0) > 40 && !e.id.includes("switch") && !e.name?.toLowerCase().includes("schalt"),
  );
  if (curves.length === 0) return [];

  const byRadius = new Map<number, CatalogEntry[]>();
  for (const c of curves) {
    const rKey = Math.round((c.radius ?? 0) * 10) / 10;
    const list = byRadius.get(rKey) || [];
    list.push(c);
    byRadius.set(rKey, list);
  }

  const sortedRadii = Array.from(byRadius.keys()).sort((a, b) => a - b);
  const options: CurveRadiusOption[] = [];

  for (const r of sortedRadii) {
    const candidates = byRadius.get(r)!;
    candidates.sort((a, b) => {
      const aIsSpecial = a.name?.toLowerCase().includes("trenn") || a.name?.toLowerCase().includes("schalt");
      const bIsSpecial = b.name?.toLowerCase().includes("trenn") || b.name?.toLowerCase().includes("schalt");
      if (aIsSpecial !== bIsSpecial) return aIsSpecial ? 1 : -1;
      const aDiff = Math.abs((a.angle ?? 30) - 30);
      const bDiff = Math.abs((b.angle ?? 30) - 30);
      if (aDiff !== bDiff) return aDiff - bDiff;
      return (b.angle ?? 0) - (a.angle ?? 0);
    });

    const best = candidates[0]!;
    const angle = best.angle ?? 30;
    const rMatch = best.name ? best.name.match(/\b(R\s*\d+[a-zA-Z]?)\b/i) : null;
    const rTag = rMatch ? rMatch[1]!.toUpperCase().replace(/\s+/, "") : `R (${r} mm)`;
    const label = `${rTag} · ${r} mm (#${best.id}, ${angle}°)`;

    options.push({
      catalogId: best.id,
      name: best.name || label,
      radiusMm: r,
      angleDeg: angle,
      label,
      entry: best,
    });
  }

  return options;
}

/**
 * Automatically creates a ramp with curves, straights, or a helix to climb/descend to another layer.
 */
export function generateRampPieces(params: RampGeneratorParams): GeneratedRampResult {
  const {
    anchorPiece,
    anchorConnectorIndex,
    connectorIndex,
    targetHeightMm,
    targetLayerId,
    rampShape,
    shape,
    curveDirection,
    direction,
    curveAngleDeg = 180,
    catalog,
    layers,
    allPieces,
    targetGradientPercent,
    maxGradientPercent,
    customPieceCount,
    selectedCurveCatalogId,
    selectedCurveId,
  } = params;

  const effectiveShape: RampShapeType = shape || rampShape || "straight";
  const effectiveDirection: "left" | "right" = direction || curveDirection || "left";
  const rawGradient = maxGradientPercent ?? targetGradientPercent ?? 2.8;
  const effectiveGradient = Math.abs(rawGradient) || 2.8;
  const effectiveConnIdx = connectorIndex !== undefined ? connectorIndex : anchorConnectorIndex;
  const pieceList = allPieces && allPieces.length > 0 ? allPieces : [anchorPiece];

  // 1. Identify open connector
  const openConns = getOpenConnectors(anchorPiece, pieceList, 8.0, layers);
  if (openConns.length === 0) {
    return {
      success: false,
      reason: "no_open_connector",
      pieces: [],
      totalLengthMm: 0,
      totalRiseMm: 0,
      gradientPercent: 0,
      startHeightMm: 0,
      endHeightMm: targetHeightMm,
      chosenConnectorIndex: -1,
    };
  }

  const chosenConnIdx =
    effectiveConnIdx !== undefined && openConns.includes(effectiveConnIdx)
      ? effectiveConnIdx
      : openConns[0]!;

  const startConn = anchorPiece.connectors[chosenConnIdx]!;
  const hStart = getPieceConnectorHeight(anchorPiece, chosenConnIdx, layers);
  const totalRise = targetHeightMm - hStart;

  if (Math.abs(totalRise) < 0.1) {
    return {
      success: false,
      reason: "same_height",
      pieces: [],
      totalLengthMm: 0,
      totalRiseMm: 0,
      gradientPercent: 0,
      startHeightMm: hStart,
      endHeightMm: targetHeightMm,
      chosenConnectorIndex: chosenConnIdx,
    };
  }

  // 2. Select track pieces from catalog
  const { straightEntry, curveEntry } = findStandardTrackEntries(catalog);
  const curveOverrideId = selectedCurveCatalogId || selectedCurveId;
  const customCurve = curveOverrideId ? catalog.entries.find((e) => e.id === curveOverrideId) : undefined;
  const resolvedCurve = customCurve || curveEntry;

  if (!straightEntry && !resolvedCurve) {
    return {
      success: false,
      reason: "no_suitable_pieces",
      pieces: [],
      totalLengthMm: 0,
      totalRiseMm: 0,
      gradientPercent: 0,
      startHeightMm: hStart,
      endHeightMm: targetHeightMm,
      chosenConnectorIndex: chosenConnIdx,
    };
  }

  const defaultStraight = straightEntry || resolvedCurve!;
  const defaultCurve = resolvedCurve || straightEntry!;

  // 3. Calculate sequence of pieces based on ramp shape and gradient
  const maxGrad = Math.max(0.5, effectiveGradient);
  const minReqLength = (Math.abs(totalRise) / (maxGrad / 100));

  interface PieceSpec {
    entry: CatalogEntry;
    direction: "left" | "right" | "straight";
  }
  const sequence: PieceSpec[] = [];

  if (effectiveShape === "straight") {
    const sLen = getTrackPieceLength(defaultStraight);
    const count = customPieceCount ?? Math.max(1, Math.ceil(minReqLength / Math.max(10, sLen)));
    for (let i = 0; i < count; i++) {
      sequence.push({ entry: defaultStraight, direction: "straight" });
    }
  } else if (effectiveShape === "curved") {
    const cAngle = defaultCurve.angle ?? 30;
    const targetAngle = curveAngleDeg ?? 180;
    const numCurves = Math.max(1, Math.round(targetAngle / Math.max(1, cAngle)));

    const cLen = getTrackPieceLength(defaultCurve);
    const totalCurveLen = numCurves * cLen;

    if (customPieceCount !== undefined) {
      for (let i = 0; i < customPieceCount; i++) {
        sequence.push({ entry: defaultCurve, direction: effectiveDirection });
      }
    } else if (totalCurveLen >= minReqLength) {
      for (let i = 0; i < numCurves; i++) {
        sequence.push({ entry: defaultCurve, direction: effectiveDirection });
      }
    } else {
      // Add straight segments to achieve required safe length
      const diffLen = minReqLength - totalCurveLen;
      const sLen = getTrackPieceLength(defaultStraight);
      const numStraights = Math.ceil(diffLen / Math.max(10, sLen));

      // Half straights before curves, half after
      const halfS = Math.floor(numStraights / 2);
      for (let i = 0; i < halfS; i++) {
        sequence.push({ entry: defaultStraight, direction: "straight" });
      }
      for (let i = 0; i < numCurves; i++) {
        sequence.push({ entry: defaultCurve, direction: effectiveDirection });
      }
      for (let i = halfS; i < numStraights; i++) {
        sequence.push({ entry: defaultStraight, direction: "straight" });
      }
    }
  } else if (effectiveShape === "helix") {
    // Helix / Gleiswendel
    const scale = catalog.scale || "H0";
    const minClearance = getMinClearanceHeightForScale(scale);
    const cLen = getTrackPieceLength(defaultCurve);
    const cAngle = defaultCurve.angle ?? 30;
    const piecesPerTurn = Math.max(4, Math.round(360 / Math.max(1, cAngle)));
    const turnLength = piecesPerTurn * cLen;

    let totalPieces: number;
    if (customPieceCount !== undefined) {
      totalPieces = customPieceCount;
    } else if (targetGradientPercent !== undefined || maxGradientPercent !== undefined) {
      // User specified target gradient: calculate piece count so gradient <= target gradient
      const piecesByGradient = Math.ceil(minReqLength / Math.max(1, cLen));
      const piecesByAngle = params.curveAngleDeg ? Math.round(params.curveAngleDeg / Math.max(1, cAngle)) : 0;
      totalPieces = Math.max(4, piecesByAngle, piecesByGradient);
    } else {
      // Default: full loops of minClearance height
      const turnsByClearance = Math.ceil(Math.abs(totalRise) / Math.max(10, minClearance));
      const turnsByGradient = Math.ceil(minReqLength / Math.max(10, turnLength));
      const finalTurns = Math.max(1, turnsByClearance, turnsByGradient);
      totalPieces = finalTurns * piecesPerTurn;
    }

    for (let i = 0; i < totalPieces; i++) {
      sequence.push({ entry: defaultCurve, direction: effectiveDirection });
    }
  }

  if (sequence.length === 0) {
    sequence.push({ entry: defaultStraight, direction: "straight" });
  }

  // 4. Position and connect each piece along the chain
  const layerList = layers && layers.length > 0 ? layers : [];
  const targetLayer = layerList.find((l) => l.id === targetLayerId) || layerList[0];
  const targetLayerBaseH = targetLayer?.heightMm ?? 0;

  const totalLengthMm = sequence.reduce((sum, s) => sum + getTrackPieceLength(s.entry), 0);
  const avgGrad = totalLengthMm > 0 ? (Math.abs(totalRise) / totalLengthMm) * 100 : 0;

  let currTargetConn: Connector = { ...startConn };
  let accumDist = 0;
  const newPieces: TrackPiece[] = [];

  for (let k = 0; k < sequence.length; k++) {
    const spec = sequence[k]!;
    const entry = spec.entry;
    const isCurveLeft = spec.direction === "left";

    // Curve turning left: connect at local connector 1, exit at local connector 0
    // Curve turning right or straight: connect at local connector 0, exit at local connector 1
    const inConnIdx = isCurveLeft ? 1 : 0;
    const outConnIdx = isCurveLeft ? 0 : 1;

    const locals = getLocalConnectors(entry);
    const inLocal = locals[inConnIdx] || locals[0]!;

    // Align piece rotation and translation
    const snappedRotation = normAngle(currTargetConn.angle + 180 - inLocal.angle);
    const rotOffset = rotate(inLocal, snappedRotation);
    const snappedPos = {
      x: currTargetConn.x - rotOffset.x,
      y: currTargetConn.y - rotOffset.y,
    };

    const pieceTransform = {
      x: Math.round(snappedPos.x * 100) / 100,
      y: Math.round(snappedPos.y * 100) / 100,
      rotation: Math.round(snappedRotation * 100) / 100,
    };

    const worldConns = computeWorldConnectors(entry, pieceTransform);

    // Calculate elevation for start and end of this piece
    const pLen = getTrackPieceLength(entry);
    const sDist = accumDist;
    const eDist = accumDist + pLen;
    accumDist = eDist;

    const u0 = totalLengthMm > 0 ? sDist / totalLengthMm : 0;
    const u1 = totalLengthMm > 0 ? eDist / totalLengthMm : 1;

    const hEntry = hStart + totalRise * u0;
    const hExit = hStart + totalRise * u1;

    const hC0 = inConnIdx === 0 ? hEntry : hExit;
    const hC1 = inConnIdx === 0 ? hExit : hEntry;

    const pieceElev = {
      start: Math.round((hC0 - targetLayerBaseH) * 100) / 100,
      end: Math.round((hC1 - targetLayerBaseH) * 100) / 100,
    };

    const newPiece: TrackPiece = {
      id: `ramp-${Date.now()}-${k}-${Math.random().toString(36).substring(2, 6)}`,
      catalogId: entry.id,
      layerId: targetLayerId,
      transform: pieceTransform,
      connectors: worldConns,
      elevation: pieceElev,
    };

    newPieces.push(newPiece);
    currTargetConn = worldConns[outConnIdx] || worldConns[worldConns.length - 1]!;
  }

  return {
    success: true,
    pieces: newPieces,
    totalLengthMm: Math.round(totalLengthMm * 10) / 10,
    totalRiseMm: Math.round(totalRise * 10) / 10,
    gradientPercent: Math.round(avgGrad * 10) / 10,
    startHeightMm: Math.round(hStart * 10) / 10,
    endHeightMm: Math.round(targetHeightMm * 10) / 10,
    targetHeightMm: Math.round(targetHeightMm * 10) / 10,
    chosenConnectorIndex: chosenConnIdx,
  };
}
