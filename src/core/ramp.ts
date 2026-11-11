import type { TrackPiece, Layer } from "./types";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { dist } from "./geometry";
import { getPieceConnectorHeight } from "./snapping";

export type RampProfile = "linear" | "eased";

export interface ChainPieceInfo {
  piece: TrackPiece;
  entryConnectorIndex: number; // 0 or 1
  exitConnectorIndex: number;  // 1 or 0
  lengthMm: number;
  startDistMm: number;
  endDistMm: number;
  entryHeightMm: number;
  exitHeightMm: number;
  gradientPercent: number;
  elevation: {
    start: number; // connector 0 relative to layerBaseH
    end: number;   // connector 1 relative to layerBaseH
  };
}

export interface RampCalculationResult {
  chain: ChainPieceInfo[];
  totalLengthMm: number;
  totalRiseMm: number;
  averageGradientPercent: number;
  maxGradientPercent: number;
  isContinuous: boolean;
  pieceElevations: Map<string, { start: number; end: number }>;
  targetMaxGradientPercent?: number;
  exceedsTargetMax?: boolean;
  maxAllowableRiseMm?: number;
  recommendedEndHeightMm?: number;
  requiredLengthMm?: number;
}

/**
 * Calculates the running length in mm of a track catalog entry.
 */
export function getTrackPieceLength(entry?: CatalogEntry): number {
  if (!entry) return 188.3;
  if (entry.length) return entry.length;
  if (entry.radius && entry.angle) {
    return (2 * Math.PI * entry.radius * entry.angle) / 360;
  }
  return 188.3;
}

/**
 * Calculates end height in mm such that the maximum gradient does not exceed targetMaxGradientPercent.
 * In linear mode: peak gradient = average gradient.
 * In eased mode: peak gradient = 1.5 * average gradient, so average gradient must be target / 1.5.
 */
export function calculateEndHeightFromMaxGradient(params: {
  startHeightMm: number;
  totalLengthMm: number;
  targetMaxGradientPercent: number;
  profile: RampProfile;
  direction?: 1 | -1;
}): number {
  const { startHeightMm, totalLengthMm, targetMaxGradientPercent, profile, direction = 1 } = params;
  if (targetMaxGradientPercent <= 0 || totalLengthMm <= 0) return startHeightMm;
  const peakFactor = profile === "eased" ? 1.5 : 1.0;
  const maxAllowableAvgGrad = targetMaxGradientPercent / peakFactor;
  const allowableRiseMm = totalLengthMm * (maxAllowableAvgGrad / 100);
  return Math.round((startHeightMm + direction * allowableRiseMm) * 10) / 10;
}

/**
 * Calculates the required track chain length in mm for a given rise (totalRiseMm)
 * so that peak gradient does not exceed targetMaxGradientPercent.
 */
export function calculateRequiredLengthForGradient(params: {
  totalRiseMm: number;
  targetMaxGradientPercent: number;
  profile: RampProfile;
}): number {
  const { totalRiseMm, targetMaxGradientPercent, profile } = params;
  if (targetMaxGradientPercent <= 0) return 0;
  const peakFactor = profile === "eased" ? 1.5 : 1.0;
  const maxAllowableAvgGrad = targetMaxGradientPercent / peakFactor;
  const reqLength = Math.abs(totalRiseMm) / (maxAllowableAvgGrad / 100);
  return Math.round(reqLength * 10) / 10;
}

/**
 * Evaluates the ramp elevation curve at normalized position u in [0, 1].
 * - "linear": Even slope (Dreisatz) across the entire ramp.
 * - "eased": S-curve vertical easement (Ausrundung: 1% -> 2% -> 3% -> 2% -> 1%).
 */
export function evaluateRampProfile(u: number, profile: RampProfile): number {
  const t = Math.max(0, Math.min(1, u));
  if (profile === "linear") {
    return t;
  }
  // Cubic smoothstep: 3t^2 - 2t^3
  // Gives zero vertical slope at ends and smooth acceleration in the middle.
  return t * t * (3 - 2 * t);
}

/**
 * Orders selected pieces into a contiguous topological chain by matching world connectors.
 */
export function sortPiecesIntoChain(
  pieces: TrackPiece[],
  catalog: Catalog,
  reversed = false,
): { ordered: Array<{ piece: TrackPiece; entryConnector: number; exitConnector: number; length: number }>; isContinuous: boolean } {
  if (pieces.length === 0) return { ordered: [], isContinuous: true };
  if (pieces.length === 1) {
    const p = pieces[0]!;
    const entry = catalog.entries.find((e) => e.id === p.catalogId);
    const len = getTrackPieceLength(entry);
    const entryConn = reversed ? 1 : 0;
    const exitConn = reversed ? 0 : 1;
    return {
      ordered: [{ piece: p, entryConnector: entryConn, exitConnector: exitConn, length: len }],
      isContinuous: true,
    };
  }

  const entryMap = new Map<string, CatalogEntry>();
  for (const e of catalog.entries) entryMap.set(e.id, e);

  // Connection adjacency within selected set
  // neighborMap: Map<pieceId, Array<{ neighborId: string; myConnIdx: number; neighborConnIdx: number }>>
  const neighborMap = new Map<string, Array<{ neighborId: string; myConnIdx: number; neighborConnIdx: number }>>();
  for (const p of pieces) {
    neighborMap.set(p.id, []);
  }

  for (let i = 0; i < pieces.length; i++) {
    const pA = pieces[i]!;
    for (let j = i + 1; j < pieces.length; j++) {
      const pB = pieces[j]!;
      for (let cA = 0; cA < pA.connectors.length; cA++) {
        const connA = pA.connectors[cA]!;
        for (let cB = 0; cB < pB.connectors.length; cB++) {
          const connB = pB.connectors[cB]!;
          if (dist(connA, connB) <= 8.0) {
            neighborMap.get(pA.id)?.push({ neighborId: pB.id, myConnIdx: cA, neighborConnIdx: cB });
            neighborMap.get(pB.id)?.push({ neighborId: pA.id, myConnIdx: cB, neighborConnIdx: cA });
          }
        }
      }
    }
  }

  // Find start piece: preferably a degree-1 endpoint (sorted by spatial coordinate for stable left-to-right order)
  let startPiece = pieces[0]!;
  let endpoints = pieces.filter((p) => (neighborMap.get(p.id)?.length || 0) === 1);
  if (endpoints.length > 0) {
    endpoints.sort((a, b) => {
      if (Math.abs(a.transform.x - b.transform.x) > 1) {
        return a.transform.x - b.transform.x;
      }
      return a.transform.y - b.transform.y;
    });
    startPiece = endpoints[0]!;
  }

  const visited = new Set<string>();
  const ordered: Array<{ piece: TrackPiece; entryConnector: number; exitConnector: number; length: number }> = [];

  let curPiece = startPiece;
  let prevPieceId: string | null = null;
  let entryConnIdx = 0;

  // For the first piece, determine entry connector
  const firstNeighbors = neighborMap.get(curPiece.id) || [];
  if (firstNeighbors.length > 0) {
    const outIdx = firstNeighbors[0]!.myConnIdx;
    entryConnIdx = outIdx === 0 ? 1 : 0;
  }

  while (curPiece && !visited.has(curPiece.id)) {
    visited.add(curPiece.id);
    const entry = entryMap.get(curPiece.catalogId);
    const len = getTrackPieceLength(entry);

    const neighbors = neighborMap.get(curPiece.id) || [];
    const nextNeighborInfo = neighbors.find((n) => n.neighborId !== prevPieceId && !visited.has(n.neighborId));

    let exitConnIdx = 1;
    if (nextNeighborInfo) {
      exitConnIdx = nextNeighborInfo.myConnIdx;
    } else {
      exitConnIdx = entryConnIdx === 0 ? 1 : 0;
    }

    ordered.push({
      piece: curPiece,
      entryConnector: entryConnIdx,
      exitConnector: exitConnIdx,
      length: len,
    });

    if (nextNeighborInfo) {
      prevPieceId = curPiece.id;
      const nextPiece = pieces.find((p) => p.id === nextNeighborInfo.neighborId);
      if (nextPiece) {
        entryConnIdx = nextNeighborInfo.neighborConnIdx;
        curPiece = nextPiece;
      } else {
        break;
      }
    } else {
      break;
    }
  }

  // Append any disconnected pieces that weren't reached
  for (const p of pieces) {
    if (!visited.has(p.id)) {
      const entry = entryMap.get(p.catalogId);
      const len = getTrackPieceLength(entry);
      ordered.push({
        piece: p,
        entryConnector: 0,
        exitConnector: 1,
        length: len,
      });
    }
  }

  const isContinuous = ordered.length === pieces.length && (endpoints.length === 2 || pieces.length === 1);

  if (reversed) {
    ordered.reverse();
    // Invert entry and exit connectors when whole chain is flipped
    for (const item of ordered) {
      const temp = item.entryConnector;
      item.entryConnector = item.exitConnector;
      item.exitConnector = temp;
    }
  }

  return { ordered, isContinuous };
}

/**
 * Calculates continuous ramp heights for all pieces in the chain.
 */
export function calculateRamp(params: {
  pieces: TrackPiece[];
  catalog: Catalog;
  startHeightMm: number;
  endHeightMm: number;
  layerBaseHeightMm: number;
  profile: RampProfile;
  reversed?: boolean;
  targetMaxGradientPercent?: number;
}): RampCalculationResult {
  const { pieces, catalog, startHeightMm, endHeightMm, layerBaseHeightMm, profile, reversed = false, targetMaxGradientPercent } = params;

  const { ordered, isContinuous } = sortPiecesIntoChain(pieces, catalog, reversed);

  const totalLengthMm = ordered.reduce((sum, item) => sum + item.length, 0);
  const totalRiseMm = endHeightMm - startHeightMm;
  const avgGrad = totalLengthMm > 0 ? (Math.abs(totalRiseMm) / totalLengthMm) * 100 : 0;

  const chain: ChainPieceInfo[] = [];
  const pieceElevations = new Map<string, { start: number; end: number }>();

  let accumDist = 0;

  for (let i = 0; i < ordered.length; i++) {
    const item = ordered[i]!;
    const startDist = accumDist;
    const endDist = accumDist + item.length;
    accumDist = endDist;

    const u0 = totalLengthMm > 0 ? startDist / totalLengthMm : 0;
    const u1 = totalLengthMm > 0 ? endDist / totalLengthMm : 1;

    const hEntry = startHeightMm + totalRiseMm * evaluateRampProfile(u0, profile);
    const hExit = startHeightMm + totalRiseMm * evaluateRampProfile(u1, profile);

    const deltaPiece = Math.abs(hExit - hEntry);
    const gradPiece = item.length > 0 ? (deltaPiece / item.length) * 100 : 0;

    let c0Height = 0;
    let c1Height = 0;

    if (item.entryConnector === 0) {
      c0Height = hEntry;
      c1Height = hExit;
    } else {
      c0Height = hExit;
      c1Height = hEntry;
    }

    const elevRel = {
      start: Math.round((c0Height - layerBaseHeightMm) * 100) / 100,
      end: Math.round((c1Height - layerBaseHeightMm) * 100) / 100,
    };

    chain.push({
      piece: item.piece,
      entryConnectorIndex: item.entryConnector,
      exitConnectorIndex: item.exitConnector,
      lengthMm: item.length,
      startDistMm: startDist,
      endDistMm: endDist,
      entryHeightMm: Math.round(hEntry * 10) / 10,
      exitHeightMm: Math.round(hExit * 10) / 10,
      gradientPercent: Math.round(gradPiece * 10) / 10,
      elevation: elevRel,
    });

    pieceElevations.set(item.piece.id, elevRel);
  }

  const maxGrad = chain.reduce((max, c) => Math.max(max, c.gradientPercent), avgGrad);

  let exceedsTargetMax = false;
  let maxAllowableRiseMm: number | undefined;
  let recommendedEndHeightMm: number | undefined;
  let requiredLengthMm: number | undefined;

  if (targetMaxGradientPercent !== undefined && targetMaxGradientPercent > 0) {
    const peakFactor = profile === "eased" ? 1.5 : 1.0;
    const maxAllowableAvgGrad = targetMaxGradientPercent / peakFactor;
    maxAllowableRiseMm = Math.round((totalLengthMm * (maxAllowableAvgGrad / 100)) * 10) / 10;
    const dir = totalRiseMm < 0 ? -1 : 1;
    recommendedEndHeightMm = Math.round((startHeightMm + dir * maxAllowableRiseMm) * 10) / 10;
    requiredLengthMm = calculateRequiredLengthForGradient({
      totalRiseMm,
      targetMaxGradientPercent,
      profile,
    });
    exceedsTargetMax = maxGrad > targetMaxGradientPercent + 0.05;
  }

  return {
    chain,
    totalLengthMm,
    totalRiseMm,
    averageGradientPercent: Math.round(avgGrad * 10) / 10,
    maxGradientPercent: Math.round(maxGrad * 10) / 10,
    isContinuous,
    pieceElevations,
    targetMaxGradientPercent,
    exceedsTargetMax,
    maxAllowableRiseMm,
    recommendedEndHeightMm,
    requiredLengthMm,
  };
}

export interface AutoRampResult {
  success: boolean;
  reason?: "no_selection" | "not_continuous" | "same_height";
  chain?: ChainPieceInfo[];
  startHeightMm?: number;
  endHeightMm?: number;
  totalLengthMm?: number;
  totalRiseMm?: number;
  gradientPercent?: number;
  isContinuous?: boolean;
  pieceElevations?: Map<string, { start: number; end: number }>;
}

/**
 * Automatically calculates a smooth constant-gradient ramp across a chain of track pieces.
 *
 * It inspects the endpoints of the selected chain:
 * - If intermediate pieces are selected and connected to unselected tracks at different elevations (e.g. 0 mm and 80 mm),
 *   it infers startHeightMm and endHeightMm directly from the connected adjacent tracks.
 * - If the endpoints themselves belong to layers or pieces with different elevations, those elevations are used.
 * - Interpolates constant slope (linear profile) across the chain and returns relative elevations for each piece.
 */
export function calculateAutoRamp(params: {
  pieces: TrackPiece[];
  allPieces?: TrackPiece[];
  layers?: Layer[];
  catalog: Catalog;
  profile?: RampProfile;
}): AutoRampResult {
  const { pieces, allPieces, layers, catalog, profile = "linear" } = params;

  if (pieces.length === 0) {
    return { success: false, reason: "no_selection" };
  }

  const { ordered, isContinuous } = sortPiecesIntoChain(pieces, catalog, false);
  if (!isContinuous || ordered.length === 0) {
    return { success: false, reason: "not_continuous" };
  }

  const firstItem = ordered[0]!;
  const lastItem = ordered[ordered.length - 1]!;

  const conn1 = firstItem.piece.connectors[firstItem.entryConnector];
  const conn2 = lastItem.piece.connectors[lastItem.exitConnector];

  const selectedIds = new Set(pieces.map((p) => p.id));
  let extPiece1: TrackPiece | undefined;
  let extConnIdx1: number | undefined;
  let extPiece2: TrackPiece | undefined;
  let extConnIdx2: number | undefined;

  if (allPieces && conn1 && conn2) {
    for (const p of allPieces) {
      if (selectedIds.has(p.id)) continue;
      for (let c = 0; c < p.connectors.length; c++) {
        const conn = p.connectors[c]!;
        if (!extPiece1 && dist(conn, conn1) <= 8.0) {
          extPiece1 = p;
          extConnIdx1 = c;
        }
        if (!extPiece2 && dist(conn, conn2) <= 8.0) {
          extPiece2 = p;
          extConnIdx2 = c;
        }
      }
    }
  }

  const h1 = extPiece1 !== undefined && extConnIdx1 !== undefined
    ? getPieceConnectorHeight(extPiece1, extConnIdx1, layers)
    : getPieceConnectorHeight(firstItem.piece, firstItem.entryConnector, layers);

  const h2 = extPiece2 !== undefined && extConnIdx2 !== undefined
    ? getPieceConnectorHeight(extPiece2, extConnIdx2, layers)
    : getPieceConnectorHeight(lastItem.piece, lastItem.exitConnector, layers);

  const totalLengthMm = ordered.reduce((sum, item) => sum + item.length, 0);

  if (Math.abs(h2 - h1) < 0.1) {
    return {
      success: false,
      reason: "same_height",
      startHeightMm: Math.round(h1 * 10) / 10,
      endHeightMm: Math.round(h2 * 10) / 10,
      totalLengthMm: Math.round(totalLengthMm * 10) / 10,
      isContinuous: true,
    };
  }

  const totalRiseMm = h2 - h1;
  const avgGrad = totalLengthMm > 0 ? (Math.abs(totalRiseMm) / totalLengthMm) * 100 : 0;

  const chain: ChainPieceInfo[] = [];
  const pieceElevations = new Map<string, { start: number; end: number }>();
  let accumDist = 0;

  for (let i = 0; i < ordered.length; i++) {
    const item = ordered[i]!;
    const startDist = accumDist;
    const endDist = accumDist + item.length;
    accumDist = endDist;

    const u0 = totalLengthMm > 0 ? startDist / totalLengthMm : 0;
    const u1 = totalLengthMm > 0 ? endDist / totalLengthMm : 1;

    const hEntry = h1 + totalRiseMm * evaluateRampProfile(u0, profile);
    const hExit = h1 + totalRiseMm * evaluateRampProfile(u1, profile);

    const deltaPiece = Math.abs(hExit - hEntry);
    const gradPiece = item.length > 0 ? (deltaPiece / item.length) * 100 : 0;

    let c0Height = 0;
    let c1Height = 0;
    if (item.entryConnector === 0) {
      c0Height = hEntry;
      c1Height = hExit;
    } else {
      c0Height = hExit;
      c1Height = hEntry;
    }

    const pLayer = layers?.find((l) => l.id === item.piece.layerId);
    const pBaseH = pLayer?.heightMm ?? 0;

    const elevRel = {
      start: Math.round((c0Height - pBaseH) * 100) / 100,
      end: Math.round((c1Height - pBaseH) * 100) / 100,
    };

    chain.push({
      piece: item.piece,
      entryConnectorIndex: item.entryConnector,
      exitConnectorIndex: item.exitConnector,
      lengthMm: item.length,
      startDistMm: startDist,
      endDistMm: endDist,
      entryHeightMm: Math.round(hEntry * 10) / 10,
      exitHeightMm: Math.round(hExit * 10) / 10,
      gradientPercent: Math.round(gradPiece * 10) / 10,
      elevation: elevRel,
    });

    pieceElevations.set(item.piece.id, elevRel);
  }

  return {
    success: true,
    chain,
    startHeightMm: Math.round(h1 * 10) / 10,
    endHeightMm: Math.round(h2 * 10) / 10,
    totalLengthMm: Math.round(totalLengthMm * 10) / 10,
    totalRiseMm: Math.round(totalRiseMm * 10) / 10,
    gradientPercent: Math.round(avgGrad * 10) / 10,
    isContinuous: true,
    pieceElevations,
  };
}
