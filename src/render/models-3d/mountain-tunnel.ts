import type { Layer, Model3D, TrackPiece } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import { getPieceCenterlinePoints, groupConnectedPieces } from "@/core/table-cutout";
import { deg2rad } from "@/core/geometry";

/* ------------------------------------------------------------------ */
/*  Typen & Profile                                                    */
/* ------------------------------------------------------------------ */

export interface MountainTrackCollision {
  piece: TrackPiece;
  entry?: CatalogEntry;
  isRoad: boolean;
  hasCatenary?: boolean;
  gauge?: number;
  prof?: TunnelProfile;
  centerline: Array<{ x: number; y: number; elevation: number }>;
}

/** Querschnitt eines Tunnels: senkrechte Wände + Kreisbogen (Segmentbogen bei Straßen). */
export interface TunnelProfile {
  isRoad: boolean;
  hasCatenary?: boolean;
  numTracks?: number;
  innerWidth: number; // lichte Breite
  wallH: number; // Höhe der senkrechten Wände
  archRise: number; // Stich des Bogens
  pier: number; // Breite der Portalpfeiler links/rechts
  totalH: number; // wallH + archRise
}

export function createTunnelProfile(
  isRoad: boolean,
  hasCatenary = false,
  gauge = 16.5,
  numTracks = 1,
  trackSpacing = 0,
): TunnelProfile {
  if (isRoad) {
    return {
      isRoad: true,
      hasCatenary: false,
      numTracks: 1,
      innerWidth: 80,
      wallH: 52,
      archRise: 20,
      pier: 14,
      totalH: 72,
    };
  }

  // Maßstabsfaktor anhand der Spurweite (H0 = 16.5mm)
  const scale = Math.max(0.4, Math.min(3.0, gauge / 16.5));

  // Eingleisige Basismaße:
  // Ohne Oberleitung: Breite = 60, Wand = 56, Stich = 28 -> Gesamthöhe = 84 mm
  // Mit Oberleitung: Fahrdraht bei 72 mm, Tragseil bei 85 mm -> Wand = 72, Stich = 32 -> Gesamthöhe = 104 mm, Breite = 70 mm
  const baseSingleW = (hasCatenary ? 70 : 60) * scale;
  const baseWallH = (hasCatenary ? 72 : 56) * scale;
  const baseArchRise = (hasCatenary ? 32 : 28) * scale;
  const pier = Math.round(16 * scale);

  if (numTracks <= 1) {
    return {
      isRoad: false,
      hasCatenary,
      numTracks: 1,
      innerWidth: Math.round(baseSingleW),
      wallH: Math.round(baseWallH),
      archRise: Math.round(baseArchRise),
      pier,
      totalH: Math.round(baseWallH + baseArchRise),
    };
  }

  // Mehrgleisiges Portal: Öffnung passt sich der Trassenbreite aller Gleise an
  const totalTrackSpread = Math.max(trackSpacing, (numTracks - 1) * 58 * scale);
  const multiWidth = Math.round(totalTrackSpread + baseSingleW);
  const multiWallH = Math.round((hasCatenary ? 74 : 58) * scale);
  const multiArchRise = Math.round((hasCatenary ? 38 : 32) * scale);

  return {
    isRoad: false,
    hasCatenary,
    numTracks,
    innerWidth: multiWidth,
    wallH: multiWallH,
    archRise: multiArchRise,
    pier: Math.round(pier * 1.15),
    totalH: Math.round(multiWallH + multiArchRise),
  };
}

/** Relative Berggrenze (0..1, elliptisch oder entlang Pinselpfad), an der die Portale sitzen. */
export const R_PORTAL = 0.82;
export const ELLIPSE_TOLERANCE = 1.05;
export const APPROACH_SLOPE = 30; // Böschungsbreite des Einschnitts vor dem Portal
export const PORTAL_LIFT = 0; // Portale + Röhre starten exakt auf Gleishöhe (kein Schwebespalt)

export function mountainNoise(nx: number, nz: number): number {
  const s1 = Math.sin(nx * 3.1415 + 0.5) * Math.cos(nz * 3.1415 + 0.3);
  const s2 = Math.sin(nx * 6.28 + nz * 4.71) * 0.35;
  const s3 = Math.cos(nx * 9.42 - nz * 7.85) * 0.15;
  return s1 + s2 + s3;
}

export function naturalMountainHeightAt(nx: number, nz: number, height: number): number {
  const rDist = Math.hypot(nx, nz);
  if (rDist >= 1.0) return 0;
  const profile = Math.cos(rDist * (Math.PI / 2)) ** 1.35;
  return Math.max(0, height * profile * (1 + 0.3 * mountainNoise(nx, nz)));
}

/**
 * Berechnet für einen Punkt (lx, lz) im lokalen Koordinatensystem eines Berges
 * sowohl den normierten Randabstand `normR` (0 = Gipfel/Grat, 1 = Bergfuß)
 * als auch die natürliche Geländehöhe `baseH` (unterstützt Ellipsen-Berge UND Pinsel-Pfade `mountainPath`).
 */
export function evaluateMountainLocal(
  lx: number,
  lz: number,
  model?: Model3D,
): { normR: number; baseH: number } {
  const width = model?.width ?? 600;
  const depth = model?.depth ?? 450;
  const height = model?.height ?? 220;
  const halfW = Math.max(1, width / 2);
  const halfD = Math.max(1, depth / 2);

  const path = model?.mountainPath;
  if (path && path.length > 0) {
    let minNormR = Infinity;
    let maxBaseH = 0;
    const noiseVal = mountainNoise(lx / 220, lz / 220);

    if (path.length === 1) {
      const n = path[0]!;
      const r = Math.max(25, n.radius || 180);
      const h = Math.max(10, n.height || height);
      const d = Math.hypot(lx - n.x, lz - n.y);
      const normR = d / r;
      if (normR < 1.0) {
        const prof = Math.cos(normR * (Math.PI / 2)) ** 1.35;
        maxBaseH = Math.max(0, h * prof * (1 + 0.26 * noiseVal));
      }
      return { normR, baseH: maxBaseH };
    }

    for (let i = 0; i < path.length - 1; i++) {
      const a = path[i]!;
      const b = path[i + 1]!;
      const abx = b.x - a.x;
      const abz = b.y - a.y;
      const lenSq = abx * abx + abz * abz;
      const t = lenSq > 1e-4 ? Math.max(0, Math.min(1, ((lx - a.x) * abx + (lz - a.y) * abz) / lenSq)) : 0;
      const px = a.x + abx * t;
      const pz = a.y + abz * t;
      const r = Math.max(25, (a.radius || 180) + ((b.radius || 180) - (a.radius || 180)) * t);
      const h = Math.max(10, (a.height || height) + ((b.height || height) - (a.height || height)) * t);
      const d = Math.hypot(lx - px, lz - pz);
      const normR = d / r;
      if (normR < minNormR) minNormR = normR;
      if (normR < 1.0) {
        const prof = Math.cos(normR * (Math.PI / 2)) ** 1.35;
        const segH = Math.max(0, h * prof * (1 + 0.26 * noiseVal));
        if (segH > maxBaseH) maxBaseH = segH;
      }
    }
    return { normR: minNormR, baseH: maxBaseH };
  }

  const nx = lx / halfW;
  const nz = lz / halfD;
  const normR = Math.hypot(nx, nz);
  const baseH = naturalMountainHeightAt(nx, nz, height);
  return { normR, baseH };
}

/** Transformiert Weltkoordinaten (wx, wy) in lokale Bergkoordinaten (lx, lz) */
export function worldToMountainLocal(wx: number, wy: number, mountain: Model3D): { lx: number; lz: number } {
  const mPos = mountain.transform;
  const mRotRad = deg2rad(mPos.rotation);
  const cosR = Math.cos(-mRotRad);
  const sinR = Math.sin(-mRotRad);
  const dx = wx - mPos.x;
  const dy = wy - mPos.y;
  return {
    lx: dx * cosR - dy * sinR,
    lz: dx * sinR + dy * cosR,
  };
}

/** Transformiert lokale Bergkoordinaten (lx, lz) in Weltkoordinaten (wx, wy) */
export function mountainLocalToWorld(lx: number, lz: number, mountain?: Model3D): { wx: number; wy: number } {
  const mRotRad = mountain ? deg2rad(mountain.transform.rotation) : 0;
  const cosF = Math.cos(mRotRad);
  const sinF = Math.sin(mRotRad);
  return {
    wx: (mountain?.transform.x ?? 0) + (lx * cosF - lz * sinF),
    wy: (mountain?.transform.y ?? 0) + (lx * sinF + lz * cosF),
  };
}

/** Prüft, ob ein Weltpunkt tief im Inneren eines anderen überlappenden Berges liegt */
export function isPointInsideOtherMountain(
  wx: number,
  wy: number,
  elev: number,
  currentMountain: Model3D,
  allMountains?: Model3D[],
): boolean {
  if (!allMountains || allMountains.length <= 1) return false;
  for (const other of allMountains) {
    if (other.id === currentMountain.id || other.builtinId !== "mountain") continue;
    const otherHeight = other.height ?? 220;
    if (elev < -35 || elev > otherHeight - 10) continue;
    const { lx, lz } = worldToMountainLocal(wx, wy, other);
    const { normR } = evaluateMountainLocal(lx, lz, other);
    if (normR <= R_PORTAL - 0.04) {
      return true;
    }
  }
  return false;
}

/* ------------------------------------------------------------------ */
/*  Kollisionserkennung                                                */
/* ------------------------------------------------------------------ */

export function findCollidingTracksForMountain(
  mountain: Model3D,
  pieces: TrackPiece[],
  findEntry: (catalogId: string) => CatalogEntry | undefined,
  layers?: Layer[],
): MountainTrackCollision[] {
  const mPos = mountain.transform;
  const halfW = mountain.width / 2;
  const halfD = mountain.depth / 2;
  const mHeight = mountain.height ?? 220;
  const boundR = Math.max(halfW, halfD) * 1.15;

  const layerMap = new Map<string, Layer>();
  if (layers) {
    for (const l of layers) layerMap.set(l.id, l);
  }
  const mountainLayer = mountain.layerId ? layerMap.get(mountain.layerId) : undefined;
  const mountainBaseElev = (mountainLayer?.heightMm ?? 0) + (mountain.elevation ?? 0);

  const getPieceRelElev = (piece: TrackPiece): { start: number; end: number; visible: boolean } => {
    const pLayer = piece.layerId ? layerMap.get(piece.layerId) : undefined;
    if (pLayer && pLayer.visible === false) {
      return { start: 0, end: 0, visible: false };
    }
    const baseH = pLayer?.heightMm ?? 0;
    const sWorld = baseH + (piece.elevation?.start ?? 0);
    const eWorld = baseH + (piece.elevation?.end ?? (piece.elevation?.start ?? 0));
    return {
      start: sWorld - mountainBaseElev,
      end: eWorld - mountainBaseElev,
      visible: true,
    };
  };

  // 1. Grober Filter, getrennt nach Gleis / Straße + Höhenfilter (nicht unter oder über dem Berg)
  const nearbyRail: TrackPiece[] = [];
  const nearbyRoad: TrackPiece[] = [];
  for (const piece of pieces) {
    const rel = getPieceRelElev(piece);
    if (!rel.visible) continue;
    if (Math.max(rel.start, rel.end) < -35) continue;
    if (Math.min(rel.start, rel.end) > mHeight - 10) continue;

    const entry = findEntry(piece.catalogId);
    const maxLen = Math.max(entry?.length ?? 250, piece.customLength ?? 0);
    const d = Math.hypot(piece.transform.x - mPos.x, piece.transform.y - mPos.y);
    if (d > boundR + maxLen) continue;
    (entry?.isRoad ? nearbyRoad : nearbyRail).push(piece);
  }

  // 2. Ketten bilden und Mittellinien prüfen
  const collisions: MountainTrackCollision[] = [];
  const groups: Array<[TrackPiece[], boolean]> = [
    [nearbyRail, false],
    [nearbyRoad, true],
  ];

  for (const [group, isRoad] of groups) {
    if (group.length === 0) continue;

    for (const chain of groupConnectedPieces(group, layers)) {
      if (chain.length === 0) continue;

      const centerline: Array<{ x: number; y: number; elevation: number }> = [];
      let insideCount = 0;
      let representativeEntry: CatalogEntry | undefined;
      let chainHasCatenary = false;
      let chainGauge = 16.5;

      for (const item of chain) {
        const piece = item.piece;
        if (piece.hasCatenary) chainHasCatenary = true;
        const entry = findEntry(piece.catalogId);
        if (entry?.gauge) chainGauge = entry.gauge;
        if (!representativeEntry && entry) representativeEntry = entry;

        const rel = getPieceRelElev(piece);
        const fromElev = item.fromConn === 0 ? rel.start : rel.end;
        const toElev = item.toConn === 0 ? rel.start : rel.end;
        const pts = getPieceCenterlinePoints(piece, entry, item.fromConn, item.toConn, 10);

        for (let i = 0; i < pts.length; i++) {
          const pt = pts[i]!;
          const t = pts.length > 1 ? i / (pts.length - 1) : 0;
          const elev = fromElev + (toElev - fromElev) * t;
          if (elev < -35 || elev > mHeight - 10) continue;

          const last = centerline[centerline.length - 1];
          if (last && Math.hypot(pt.x - last.x, pt.y - last.y) < 1.0) continue;

          const { lx, lz } = worldToMountainLocal(pt.x, pt.y, mountain);
          const { normR, baseH: terrainH } = evaluateMountainLocal(lx, lz, mountain);
          if (normR <= ELLIPSE_TOLERANCE && elev <= Math.max(terrainH, mHeight - 10)) {
            insideCount++;
          }

          centerline.push({ x: pt.x, y: pt.y, elevation: elev });
        }
      }

      if (insideCount > 0 && centerline.length >= 2) {
        const prof = createTunnelProfile(isRoad, chainHasCatenary, chainGauge);
        collisions.push({
          piece: chain[0]!.piece,
          entry: representativeEntry,
          isRoad,
          hasCatenary: chainHasCatenary,
          gauge: chainGauge,
          prof,
          centerline,
        });
      }
    }
  }

  return collisions;
}

/* ------------------------------------------------------------------ */
/*  Tunnel (Portale + Röhre)                                           */
/* ------------------------------------------------------------------ */

export interface LocalPt { lx: number; lz: number; elev: number; normR: number }
interface InsideRun { pts: LocalPt[]; openStart: boolean; openEnd: boolean }

export interface TunnelPortalInfo {
  at: LocalPt;
  dir: { x: number; z: number };
  prof: TunnelProfile;
  numTracks?: number;
  suppressedByOverlap?: boolean;
}

export interface TunnelRun {
  prof: TunnelProfile;
  pts: LocalPt[];
  startPortal?: TunnelPortalInfo;
  endPortal?: TunnelPortalInfo;
}

/** Schnittpunkt Segment/Berggrenze (normR = rPortal) per Bisektion. */
function crossing(a: LocalPt, b: LocalPt, rPortal: number, mountain: Model3D): LocalPt {
  let lo = 0;
  let hi = 1;
  const aInside = a.normR <= rPortal;
  for (let k = 0; k < 10; k++) {
    const t = (lo + hi) / 2;
    const lx = a.lx + (b.lx - a.lx) * t;
    const lz = a.lz + (b.lz - a.lz) * t;
    const inside = evaluateMountainLocal(lx, lz, mountain).normR <= rPortal;
    if (inside === aInside) lo = t;
    else hi = t;
  }
  const t = (lo + hi) / 2;
  const lx = a.lx + (b.lx - a.lx) * t;
  const lz = a.lz + (b.lz - a.lz) * t;
  return {
    lx,
    lz,
    elev: a.elev + (b.elev - a.elev) * t,
    normR: evaluateMountainLocal(lx, lz, mountain).normR,
  };
}

/** Zerlegt eine Mittellinie in zusammenhängende Abschnitte innerhalb des Berges. */
function extractInsideRuns(pts: LocalPt[], rPortal: number, mountain: Model3D): InsideRun[] {
  const runs: InsideRun[] = [];
  let cur: InsideRun | null = null;

  for (let i = 0; i < pts.length; i++) {
    const p = pts[i]!;
    if (p.normR <= rPortal) {
      if (!cur) {
        cur = { pts: [], openStart: false, openEnd: false };
        if (i > 0) {
          cur.pts.push(crossing(pts[i - 1]!, p, rPortal, mountain));
          cur.openStart = true;
        } else if (p.normR >= rPortal - 0.25) {
          cur.openStart = true;
        }
      }
      cur.pts.push(p);
    } else if (cur) {
      cur.pts.push(crossing(pts[i - 1]!, p, rPortal, mountain));
      cur.openEnd = true;
      runs.push(cur);
      cur = null;
    }
  }
  if (cur) {
    if (cur.pts.length > 0 && cur.pts[cur.pts.length - 1]!.normR >= rPortal - 0.25) {
      cur.openEnd = true;
    }
    runs.push(cur);
  }
  return runs.filter((r) => r.pts.length >= 2);
}

/** Einheitsvektor vom Portalpunkt ins Bergesinnere (stabil gegen sehr kurze Segmente). */
function inwardDir(run: LocalPt[], atStart: boolean): { x: number; z: number } {
  const base = atStart ? run[0]! : run[run.length - 1]!;
  const order = atStart ? run : [...run].reverse();
  for (let j = 1; j < order.length; j++) {
    const dx = order[j]!.lx - base.lx;
    const dz = order[j]!.lz - base.lz;
    const len = Math.hypot(dx, dz);
    if (len >= 4) return { x: dx / len, z: dz / len };
  }
  const last = order[order.length - 1]!;
  const len = Math.hypot(last.lx - base.lx, last.lz - base.lz) || 1;
  return { x: (last.lx - base.lx) / len, z: (last.lz - base.lz) / len };
}

interface PortalCandidate {
  run: TunnelRun;
  isStart: boolean;
  at: LocalPt;
  dir: { x: number; z: number };
  col: MountainTrackCollision;
  prof: TunnelProfile;
  suppressedByOverlap: boolean;
}

/** Berechnet die zusammenhängenden Tunnelröhren und Portalstandorte für einen Berg. */
export function computeTunnelRuns(
  mountain: Model3D | undefined,
  collisions: MountainTrackCollision[] | undefined,
  rPortal: number,
  allMountains?: Model3D[],
): TunnelRun[] {
  if (!mountain || !collisions || collisions.length === 0) return [];
  const mHeight = mountain.height ?? 220;

  const runsWithCol: Array<{ run: TunnelRun; col: MountainTrackCollision }> = [];

  for (const col of collisions) {
    if (col.centerline.length < 2) continue;
    const prof = col.prof ?? createTunnelProfile(col.isRoad, col.hasCatenary, col.gauge);
    const local: LocalPt[] = col.centerline
      .filter((p) => (p.elevation ?? 0) >= -35 && (p.elevation ?? 0) <= mHeight - 10)
      .map((p) => {
        const { lx, lz } = worldToMountainLocal(p.x, p.y, mountain);
        const { normR } = evaluateMountainLocal(lx, lz, mountain);
        return { lx, lz, elev: p.elevation ?? 0, normR };
      });

    for (const run of extractInsideRuns(local, rPortal, mountain)) {
      const tRun: TunnelRun = { prof, pts: run.pts };
      runsWithCol.push({ run: tRun, col });
    }
  }

  // Portal-Kandidaten sammeln
  const candidates: PortalCandidate[] = [];
  for (const { run, col } of runsWithCol) {
    const startPt = run.pts[0]!;
    const endPt = run.pts[run.pts.length - 1]!;

    if (startPt.elev >= -35 && startPt.elev <= mHeight - 10) {
      const wStart = mountainLocalToWorld(startPt.lx, startPt.lz, mountain);
      const suppressed = isPointInsideOtherMountain(wStart.wx, wStart.wy, startPt.elev, mountain, allMountains);
      candidates.push({
        run,
        isStart: true,
        at: startPt,
        dir: inwardDir(run.pts, true),
        col,
        prof: run.prof,
        suppressedByOverlap: suppressed,
      });
    }
    if (endPt.elev >= -35 && endPt.elev <= mHeight - 10) {
      const wEnd = mountainLocalToWorld(endPt.lx, endPt.lz, mountain);
      const suppressed = isPointInsideOtherMountain(wEnd.wx, wEnd.wy, endPt.elev, mountain, allMountains);
      candidates.push({
        run,
        isStart: false,
        at: endPt,
        dir: inwardDir(run.pts, false),
        col,
        prof: run.prof,
        suppressedByOverlap: suppressed,
      });
    }
  }

  // Gruppierung von nahe beieinander liegenden Portalen zu mehrgleisigen Portalen
  const grouped = new Set<PortalCandidate>();
  for (let i = 0; i < candidates.length; i++) {
    const c1 = candidates[i]!;
    if (grouped.has(c1)) continue;

    const cluster: PortalCandidate[] = [c1];
    grouped.add(c1);

    if (!c1.col.isRoad) {
      const gauge = c1.col.gauge ?? 16.5;
      const scale = Math.max(0.4, Math.min(3.0, gauge / 16.5));
      const maxGroupingDist = 115 * scale;

      for (let j = i + 1; j < candidates.length; j++) {
        const c2 = candidates[j]!;
        if (grouped.has(c2) || c2.col.isRoad) continue;
        if (c1.run === c2.run) continue;

        const dist = Math.hypot(c1.at.lx - c2.at.lx, c1.at.lz - c2.at.lz);
        const dotDir = c1.dir.x * c2.dir.x + c1.dir.z * c2.dir.z;
        const elevDiff = Math.abs(c1.at.elev - c2.at.elev);

        if (dist <= maxGroupingDist && dotDir > 0.5 && elevDiff < 25) {
          cluster.push(c2);
          grouped.add(c2);
        }
      }
    }

    if (cluster.length === 1) {
      const cand = cluster[0]!;
      const portalInfo: TunnelPortalInfo = {
        at: cand.at,
        dir: cand.dir,
        prof: cand.prof,
        numTracks: 1,
        suppressedByOverlap: cand.suppressedByOverlap,
      };
      if (cand.isStart) cand.run.startPortal = portalInfo;
      else cand.run.endPortal = portalInfo;
    } else {
      const k = cluster.length;
      const hasCatenary = cluster.some((c) => c.col.hasCatenary || c.prof.hasCatenary);
      const gauge = cluster[0]!.col.gauge ?? 16.5;
      const suppressed = cluster.every((c) => c.suppressedByOverlap);

      const avgLx = cluster.reduce((sum, c) => sum + c.at.lx, 0) / k;
      const avgLz = cluster.reduce((sum, c) => sum + c.at.lz, 0) / k;
      const avgElev = cluster.reduce((sum, c) => sum + c.at.elev, 0) / k;

      const sumDirX = cluster.reduce((sum, c) => sum + c.dir.x, 0);
      const sumDirZ = cluster.reduce((sum, c) => sum + c.dir.z, 0);
      const dirLen = Math.hypot(sumDirX, sumDirZ) || 1;
      const combinedDir = { x: sumDirX / dirLen, z: sumDirZ / dirLen };

      const latX = -combinedDir.z;
      const latZ = combinedDir.x;
      const latOffsets = cluster.map((c) => (c.at.lx - avgLx) * latX + (c.at.lz - avgLz) * latZ);
      const minLat = Math.min(...latOffsets);
      const maxLat = Math.max(...latOffsets);
      const trackSpread = maxLat - minLat;

      const multiProf = createTunnelProfile(false, hasCatenary, gauge, k, trackSpread);
      const combinedPortal: TunnelPortalInfo = {
        at: {
          lx: avgLx,
          lz: avgLz,
          elev: avgElev,
          normR: evaluateMountainLocal(avgLx, avgLz, mountain).normR,
        },
        dir: combinedDir,
        prof: multiProf,
        numTracks: k,
        suppressedByOverlap: suppressed,
      };

      for (const cand of cluster) {
        if (cand.isStart) cand.run.startPortal = combinedPortal;
        else cand.run.endPortal = combinedPortal;
      }
    }
  }

  return runsWithCol.map((rc) => rc.run);
}
