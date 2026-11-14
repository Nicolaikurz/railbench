import * as THREE from "three";
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

export const RAIL_PROFILE: TunnelProfile = {
  isRoad: false, hasCatenary: false, numTracks: 1, innerWidth: 60, wallH: 56, archRise: 28, pier: 16, totalH: 84,
};
export const ROAD_PROFILE: TunnelProfile = {
  isRoad: true, hasCatenary: false, numTracks: 1, innerWidth: 80, wallH: 52, archRise: 20, pier: 14, totalH: 72,
};

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

export const profileFor = (isRoad: boolean, hasCatenary = false, gauge = 16.5): TunnelProfile =>
  createTunnelProfile(isRoad, hasCatenary, gauge, 1, 0);

/** Relative Berggrenze (0..1, elliptisch), an der die Portale sitzen. */
export const R_PORTAL = 0.82;
const ELLIPSE_TOLERANCE = 1.05;
const APPROACH_SLOPE = 30; // Böschungsbreite des Einschnitts vor dem Portal
const PORTAL_LIFT = 1.5; // Portale + Röhre leicht anheben gegen Z-Fighting mit Gelände/Tischplatte

function mountainNoise(nx: number, nz: number): number {
  const s1 = Math.sin(nx * 3.1415 + 0.5) * Math.cos(nz * 3.1415 + 0.3);
  const s2 = Math.sin(nx * 6.28 + nz * 4.71) * 0.35;
  const s3 = Math.cos(nx * 9.42 - nz * 7.85) * 0.15;
  return s1 + s2 + s3;
}

function naturalMountainHeightAt(nx: number, nz: number, height: number): number {
  const rDist = Math.hypot(nx, nz);
  if (rDist >= 1.0) return 0;
  const profile = Math.cos(rDist * (Math.PI / 2)) ** 1.35;
  return Math.max(0, height * profile * (1 + 0.3 * mountainNoise(nx, nz)));
}

/* ------------------------------------------------------------------ */
/*  Kollisionserkennung                                                */
/* ------------------------------------------------------------------ */

/**
 * Ermittelt alle Gleise und Straßen, die tatsächlich durch den Bergkörper verlaufen
 * (nicht unterhalb der Bergbasis / Tischplatte und nicht oberhalb des Berges).
 * Gleise und Straßen werden getrennt zu Trassenketten gruppiert, damit eine Kette
 * nie gemischt ist und jeweils das passende Tunnelprofil bekommt.
 */
export function findCollidingTracksForMountain(
  mountain: Model3D,
  pieces: TrackPiece[],
  findEntry: (catalogId: string) => CatalogEntry | undefined,
  layers?: Layer[],
): MountainTrackCollision[] {
  const mPos = mountain.transform;
  const mRotRad = deg2rad(mPos.rotation);
  const cosR = Math.cos(-mRotRad);
  const sinR = Math.sin(-mRotRad);

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
    // Tief unterhalb der Bergbasis liegende Gleise (z. B. Schattenbahnhof <= -35 mm) oder oberhalb des Berges ignorieren
    if (Math.max(rel.start, rel.end) < -35) continue;
    if (Math.min(rel.start, rel.end) > mHeight - 10) continue;

    const entry = findEntry(piece.catalogId);
    const maxLen = entry?.length ?? 250;
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
          // Punkte tief unter der Bergbasis oder über dem Berggipfel gehören nicht zum Bergtunnel
          if (elev < -35 || elev > mHeight - 10) continue;

          const last = centerline[centerline.length - 1];
          if (last && Math.hypot(pt.x - last.x, pt.y - last.y) < 1.0) continue;

          const dx = pt.x - mPos.x;
          const dy = pt.y - mPos.y;
          const lx = dx * cosR - dy * sinR;
          const lz = dx * sinR + dy * cosR;
          const nx = lx / halfW;
          const nz = lz / halfD;
          const normR = Math.hypot(nx, nz);
          const terrainH = naturalMountainHeightAt(nx, nz, mHeight);
          // Zählen, wenn der Punkt im Bergkörper liegt
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
/*  Geometrie-Helfer                                                   */
/* ------------------------------------------------------------------ */

/** Querschnittspunkte von links unten über den Bogen nach rechts unten. */
function profilePoints(p: TunnelProfile, inset = 0, arcSteps = 14): Array<[number, number]> {
  const c = p.innerWidth / 2 - inset;
  const rise = Math.max(1, p.archRise - inset);
  const wallH = p.wallH;

  const R = (c * c + rise * rise) / (2 * rise);
  const cy = wallH + rise - R;
  const thL = Math.atan2(wallH - cy, -c);
  const thR = Math.atan2(wallH - cy, c);

  const pts: Array<[number, number]> = [[-c, 0], [-c, wallH]];
  for (let i = 1; i < arcSteps; i++) {
    const th = thL + (thR - thL) * (i / arcSteps);
    pts.push([R * Math.cos(th), cy + R * Math.sin(th)]);
  }
  pts.push([c, wallH], [c, 0]);
  return pts;
}

/** Portal-/Kragen-Umriss als offene Form: Außenrechteck mit Bogenöffnung (unten offen). */
function portalShape(p: TunnelProfile, outerHalfW: number, outerH: number): THREE.Shape {
  const c = p.innerWidth / 2;
  const prof = profilePoints(p, 0);
  const s = new THREE.Shape();
  s.moveTo(-outerHalfW, 0);
  s.lineTo(-outerHalfW, outerH);
  s.lineTo(outerHalfW, outerH);
  s.lineTo(outerHalfW, 0);
  s.lineTo(c, 0);
  for (let i = prof.length - 2; i >= 0; i--) s.lineTo(prof[i]![0], prof[i]![1]);
  s.closePath();
  return s;
}

/** Gewölbte Tunnelröhre entlang eines Pfades (Querschnitt = Tunnelprofil). */
function buildTunnelSweep(path: THREE.Vector3[], p: TunnelProfile, mat: THREE.Material): THREE.Mesh | null {
  if (path.length < 2) return null;
  const section = profilePoints(p, 1.0); // leicht kleiner -> kein Z-Fighting mit dem Kragen
  const ns = section.length;
  const positions: number[] = [];
  const indices: number[] = [];

  let prevSx = 1;
  let prevSz = 0;
  for (let i = 0; i < path.length; i++) {
    const a = path[Math.max(i - 1, 0)]!;
    const b = path[Math.min(i + 1, path.length - 1)]!;
    const tx = b.x - a.x;
    const tz = b.z - a.z;
    const len = Math.hypot(tx, tz);
    let sx = prevSx;
    let sz = prevSz;
    if (len > 1e-6) {
      // Normale nach rechts bezüglich der Tangente (passend zum lokalen Koordinatensystem des Portals)
      sx = tz / len;
      sz = -tx / len;
    }
    prevSx = sx;
    prevSz = sz;

    const c = path[i]!;
    for (const [x, y] of section) positions.push(c.x + sx * x, c.y + y, c.z + sz * x);
  }

  for (let i = 0; i < path.length - 1; i++) {
    for (let j = 0; j < ns - 1; j++) {
      const a = i * ns + j;
      const b = a + 1;
      const c = (i + 1) * ns + j;
      const d = c + 1;
      indices.push(a, c, b, b, c, d);
    }
  }

  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geo.setIndex(indices);
  geo.computeVertexNormals();
  return new THREE.Mesh(geo, mat);
}

/* ------------------------------------------------------------------ */
/*  Berg                                                               */
/* ------------------------------------------------------------------ */

const smoothstep = (a: number, b: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

/**
 * Erstellt die 3D-Geometrie für einen parametrischen Berg mit Tunnelportalen
 * (Gleise: Rundbogen, Straßen: breiterer, flacher Segmentbogen).
 * Vor dem Portal wird das Gelände als Einschnitt mit Böschung abgesenkt,
 * hinter dem Portal steigt es schnell auf Deckungshöhe über der Röhre an.
 */
export function buildMountain3D(
  group: THREE.Group,
  model?: Model3D,
  collisions?: MountainTrackCollision[],
): void {
  const width = model?.width ?? 600;
  const depth = model?.depth ?? 450;
  const height = model?.height ?? 220;
  const hasCollisions = !!(collisions && collisions.length > 0 && model);

  const mountainGroup = new THREE.Group();
  // Höhen-Offset in der vertikalen Z-Achse zur Beseitigung von Z-Fighting und Artefakten mit der Tischplatte
  mountainGroup.position.y = 0.5;

  const stonePortalMat = new THREE.MeshStandardMaterial({
    color: 0x52525b,
    roughness: 0.85,
    metalness: 0.12,
    side: THREE.DoubleSide,
    polygonOffset: true,
    polygonOffsetFactor: -1,
    polygonOffsetUnits: -1,
  });
  const tunnelLiningMat = new THREE.MeshStandardMaterial({
    color: 0x27272a,
    roughness: 0.96,
    metalness: 0.05,
    side: THREE.DoubleSide,
    polygonOffset: true,
    polygonOffsetFactor: -1,
    polygonOffsetUnits: -1,
  });
  const mountainMat = new THREE.MeshStandardMaterial({
    vertexColors: true,
    roughness: 0.9,
    metalness: 0.08,
    flatShading: true,
    polygonOffset: true,
    polygonOffsetFactor: -1,
    polygonOffsetUnits: -1,
  });

  // lokal -> Welt
  const mRotRad = model ? deg2rad(model.transform.rotation) : 0;
  const cosF = Math.cos(mRotRad);
  const sinF = Math.sin(mRotRad);
  const toWorld = (lx: number, lz: number) => ({
    wx: (model?.transform.x ?? 0) + (lx * cosF - lz * sinF),
    wy: (model?.transform.y ?? 0) + (lx * sinF + lz * cosF),
  });

  // 1. Geländegitter
  const segsX = 40;
  const segsZ = 40;
  const positions: number[] = [];
  const colors: number[] = [];
  const indices: number[] = [];

  for (let iz = 0; iz <= segsZ; iz++) {
    const lz = (iz / segsZ - 0.5) * depth;
    const nz = (iz / segsZ - 0.5) * 2;

    for (let ix = 0; ix <= segsX; ix++) {
      const lx = (ix / segsX - 0.5) * width;
      const nx = (ix / segsX - 0.5) * 2;

      const rDist = Math.hypot(nx, nz);
      let baseH = naturalMountainHeightAt(nx, nz, height);

      if (hasCollisions) {
        const { wx, wy } = toWorld(lx, lz);
        const insidePortal = rDist <= R_PORTAL;

        for (const col of collisions!) {
          const prof = col.prof ?? createTunnelProfile(col.isRoad, col.hasCatenary, col.gauge);
          const inner = prof.innerWidth / 2 + prof.pier; // halbe Breite der Portal-Stirnwand
          for (const pt of col.centerline) {
            if (pt.elevation < -35 || pt.elevation > height - 10) continue;
            const d = Math.hypot(wx - pt.x, wy - pt.y);

            if (insidePortal) {
              // Berginneres: Vollständige Deckung über der Tunnelröhre und dem Kragen
              const outer = inner + 45;
              if (d > outer) continue;
              const w = 1 - smoothstep(inner, outer, d);
              const target = pt.elevation + prof.totalH + 16;
              if (baseH < target) baseH += (target - baseH) * w;
            } else {
              // Vorfeld vor dem Portal: Einschnitt mit sanfter Böschung zur Trasse hin (nur absenken, falls Gelände höher als Trasse)
              const outer = inner + APPROACH_SLOPE;
              if (d > outer) continue;
              const w = 1 - smoothstep(inner, outer, d);
              const target = pt.elevation;
              if (baseH > target) {
                baseH += (target - baseH) * w;
              }
            }
          }
        }
      }

      positions.push(lx, baseH, lz);

      const hRatio = height > 0 ? baseH / height : 0;
      if (hRatio > 0.82) colors.push(0.96, 0.98, 1.0); // Schnee
      else if (hRatio > 0.35) colors.push(0.36, 0.34, 0.32); // Fels
      else colors.push(0.24, 0.38, 0.12); // Wiese
    }
  }

  // Tunnel-Routen und Portale vorberechnen
  const tunnelRuns = hasCollisions ? computeTunnelRuns(model, collisions, R_PORTAL) : [];
  const uniquePortals = new Set<TunnelPortalInfo>();
  for (const tr of tunnelRuns) {
    if (tr.startPortal) uniquePortals.add(tr.startPortal);
    if (tr.endPortal) uniquePortals.add(tr.endPortal);
  }
  const allPortals = Array.from(uniquePortals);

  // Erkennt Quads, die als steile Erdwand direkt in der Portalöffnung hängen würden
  function isEntranceCurtainQuad(
    lx0: number, lz0: number, y0: number,
    lx1: number, lz1: number, y1: number,
    lx2: number, lz2: number, y2: number,
    lx3: number, lz3: number, y3: number,
  ): boolean {
    if (allPortals.length === 0) return false;
    const minY = Math.min(y0, y1, y2, y3);
    const maxY = Math.max(y0, y1, y2, y3);
    const cx = (lx0 + lx1 + lx2 + lx3) * 0.25;
    const cz = (lz0 + lz1 + lz2 + lz3) * 0.25;
    const cy = (y0 + y1 + y2 + y3) * 0.25;

    for (const po of allPortals) {
      // Liegt komplett über der Decke / Krone des Portals -> Bergdach bleibt vollständig erhalten
      if (minY >= po.at.elev + po.prof.totalH) continue;
      // Liegt komplett unter der Schienenunterkante / Fundament -> bleibt erhalten
      if (maxY <= po.at.elev - 2) continue;

      const dx = cx - po.at.lx;
      const dz = cz - po.at.lz;
      const distFwd = dx * po.dir.x + dz * po.dir.z;
      const distLat = Math.abs(dx * (-po.dir.z) + dz * po.dir.x);

      // Lichtraumprofil: Querschnittsbreite + Tiefenkorridor an der Portalstirnwand
      // Quads im Portalmund (von kurz vor der Stirnwand bis in den Kragen)
      if (distLat <= po.prof.innerWidth / 2 + 3 && distFwd >= -16 && distFwd <= 45) {
        if (cy >= po.at.elev - 2 && cy <= po.at.elev + po.prof.totalH + 4) {
          return true;
        }
      }
    }
    return false;
  }

  // Quads erzeugen; nur die Wand im Portalmund freihalten
  for (let iz = 0; iz < segsZ; iz++) {
    for (let ix = 0; ix < segsX; ix++) {
      const i0 = iz * (segsX + 1) + ix;
      const i1 = i0 + 1;
      const i2 = (iz + 1) * (segsX + 1) + ix;
      const i3 = i2 + 1;

      if (
        isEntranceCurtainQuad(
          positions[i0 * 3]!, positions[i0 * 3 + 2]!, positions[i0 * 3 + 1]!,
          positions[i1 * 3]!, positions[i1 * 3 + 2]!, positions[i1 * 3 + 1]!,
          positions[i2 * 3]!, positions[i2 * 3 + 2]!, positions[i2 * 3 + 1]!,
          positions[i3 * 3]!, positions[i3 * 3 + 2]!, positions[i3 * 3 + 1]!,
        )
      ) {
        continue;
      }

      indices.push(i0, i2, i1, i1, i2, i3);
    }
  }

  const mountainGeo = new THREE.BufferGeometry();
  mountainGeo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  mountainGeo.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
  mountainGeo.setIndex(indices);
  mountainGeo.computeVertexNormals();

  const mountainMesh = new THREE.Mesh(mountainGeo, mountainMat);
  mountainMesh.castShadow = true;
  mountainMesh.receiveShadow = true;
  mountainGroup.add(mountainMesh);

  // 2. Portale (jedes Portal exakt einmal zeichnen)
  for (const po of allPortals) {
    buildPortal(mountainGroup, po.prof, po.at, po.dir, stonePortalMat);
  }

  // 3. Tunnelröhren
  for (const tr of tunnelRuns) {
    const path = tr.pts.map((p) => new THREE.Vector3(p.lx, p.elev + PORTAL_LIFT, p.lz));
    const tube = buildTunnelSweep(path, tr.prof, tunnelLiningMat);
    if (tube) mountainGroup.add(tube);
  }

  group.add(mountainGroup);
}

/* ------------------------------------------------------------------ */
/*  Tunnel (Portale + Röhre)                                           */
/* ------------------------------------------------------------------ */

interface LocalPt { lx: number; lz: number; elev: number; normR: number }
interface InsideRun { pts: LocalPt[]; openStart: boolean; openEnd: boolean }

export interface TunnelPortalInfo {
  at: LocalPt;
  dir: { x: number; z: number };
  prof: TunnelProfile;
  numTracks?: number;
}

export interface TunnelRun {
  prof: TunnelProfile;
  pts: LocalPt[];
  startPortal?: TunnelPortalInfo;
  endPortal?: TunnelPortalInfo;
}

/** Schnittpunkt Segment/Ellipse (normR = rPortal) per Bisektion. */
function crossing(a: LocalPt, b: LocalPt, rPortal: number, halfW: number, halfD: number): LocalPt {
  let lo = 0;
  let hi = 1;
  const aInside = a.normR <= rPortal;
  for (let k = 0; k < 10; k++) {
    const t = (lo + hi) / 2;
    const lx = a.lx + (b.lx - a.lx) * t;
    const lz = a.lz + (b.lz - a.lz) * t;
    const inside = Math.hypot(lx / halfW, lz / halfD) <= rPortal;
    if (inside === aInside) lo = t;
    else hi = t;
  }
  const t = (lo + hi) / 2;
  const lx = a.lx + (b.lx - a.lx) * t;
  const lz = a.lz + (b.lz - a.lz) * t;
  return {
    lx, lz,
    elev: a.elev + (b.elev - a.elev) * t,
    normR: Math.hypot(lx / halfW, lz / halfD),
  };
}

/** Zerlegt eine Mittellinie in zusammenhängende Abschnitte innerhalb des Berges. */
function extractInsideRuns(pts: LocalPt[], rPortal: number, halfW: number, halfD: number): InsideRun[] {
  const runs: InsideRun[] = [];
  let cur: InsideRun | null = null;

  for (let i = 0; i < pts.length; i++) {
    const p = pts[i]!;
    if (p.normR <= rPortal) {
      if (!cur) {
        cur = { pts: [], openStart: false, openEnd: false };
        if (i > 0) {
          cur.pts.push(crossing(pts[i - 1]!, p, rPortal, halfW, halfD));
          cur.openStart = true;
        } else if (p.normR >= rPortal - 0.25) {
          cur.openStart = true;
        }
      }
      cur.pts.push(p);
    } else if (cur) {
      cur.pts.push(crossing(pts[i - 1]!, p, rPortal, halfW, halfD));
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
}

/** Berechnet die zusammenhängenden Tunnelröhren und Portalstandorte für einen Berg. */
export function computeTunnelRuns(
  mountain: Model3D | undefined,
  collisions: MountainTrackCollision[] | undefined,
  rPortal: number,
): TunnelRun[] {
  if (!mountain || !collisions || collisions.length === 0) return [];
  const mPos = mountain.transform;
  const mRotRad = deg2rad(mPos.rotation);
  const cosR = Math.cos(-mRotRad);
  const sinR = Math.sin(-mRotRad);
  const halfW = mountain.width / 2;
  const halfD = mountain.depth / 2;
  const mHeight = mountain.height ?? 220;

  const runsWithCol: Array<{ run: TunnelRun; col: MountainTrackCollision }> = [];

  for (const col of collisions) {
    if (col.centerline.length < 2) continue;
    const prof = col.prof ?? createTunnelProfile(col.isRoad, col.hasCatenary, col.gauge);
    const local: LocalPt[] = col.centerline
      .filter((p) => (p.elevation ?? 0) >= -35 && (p.elevation ?? 0) <= mHeight - 10)
      .map((p) => {
        const dx = p.x - mPos.x;
        const dy = p.y - mPos.y;
        const lx = dx * cosR - dy * sinR;
        const lz = dx * sinR + dy * cosR;
        return { lx, lz, elev: p.elevation ?? 0, normR: Math.hypot(lx / halfW, lz / halfD) };
      });

    for (const run of extractInsideRuns(local, rPortal, halfW, halfD)) {
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
      candidates.push({
        run,
        isStart: true,
        at: startPt,
        dir: inwardDir(run.pts, true),
        col,
        prof: run.prof,
      });
    }
    if (endPt.elev >= -35 && endPt.elev <= mHeight - 10) {
      candidates.push({
        run,
        isStart: false,
        at: endPt,
        dir: inwardDir(run.pts, false),
        col,
        prof: run.prof,
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

    // Straßen-Portale werden nicht zusammengefasst
    if (!c1.col.isRoad) {
      const gauge = c1.col.gauge ?? 16.5;
      const scale = Math.max(0.4, Math.min(3.0, gauge / 16.5));
      const maxGroupingDist = 115 * scale; // Typischer Parallelgleisabstand

      for (let j = i + 1; j < candidates.length; j++) {
        const c2 = candidates[j]!;
        if (grouped.has(c2) || c2.col.isRoad) continue;
        if (c1.run === c2.run) continue; // Start und Ende desselben Gleises nicht gruppieren

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
      };
      if (cand.isStart) cand.run.startPortal = portalInfo;
      else cand.run.endPortal = portalInfo;
    } else {
      // Mehrgleisiges Portal für 2 oder mehr Gleise
      const k = cluster.length;
      const hasCatenary = cluster.some((c) => c.col.hasCatenary || c.prof.hasCatenary);
      const gauge = cluster[0]!.col.gauge ?? 16.5;

      const avgLx = cluster.reduce((sum, c) => sum + c.at.lx, 0) / k;
      const avgLz = cluster.reduce((sum, c) => sum + c.at.lz, 0) / k;
      const avgElev = cluster.reduce((sum, c) => sum + c.at.elev, 0) / k;

      const sumDirX = cluster.reduce((sum, c) => sum + c.dir.x, 0);
      const sumDirZ = cluster.reduce((sum, c) => sum + c.dir.z, 0);
      const dirLen = Math.hypot(sumDirX, sumDirZ) || 1;
      const combinedDir = { x: sumDirX / dirLen, z: sumDirZ / dirLen };

      // Querachse zur Portal-Blickrichtung
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
          normR: Math.hypot(avgLx / halfW, avgLz / halfD),
        },
        dir: combinedDir,
        prof: multiProf,
        numTracks: k,
      };

      for (const cand of cluster) {
        if (cand.isStart) cand.run.startPortal = combinedPortal;
        else cand.run.endPortal = combinedPortal;
      }
    }
  }

  return runsWithCol.map((rc) => rc.run);
}

function buildPortal(
  parent: THREE.Group,
  prof: TunnelProfile,
  at: LocalPt,
  dir: { x: number; z: number },
  mat: THREE.Material,
): void {
  const g = new THREE.Group();
  g.position.set(at.lx, at.elev + PORTAL_LIFT, at.lz);
  g.rotation.y = Math.atan2(dir.x, dir.z); // lokale +Z zeigt in den Berg

  const c = prof.innerWidth / 2;
  const outerHalfW = c + prof.pier;
  const isMulti = (prof.numTracks ?? 1) > 1;
  const lintel = prof.isRoad ? 12 : (isMulti ? 18 : 16);
  const headwallH = prof.isRoad ? 22 : (isMulti ? 32 : 28);
  const faceDepth = isMulti ? 16 : 14;
  const collarDepth = isMulti ? 70 : 65;

  // Stirnwand mit Bogenöffnung (ein Stück, keine Lücken zwischen Pfeilern und Sturz)
  const faceH = prof.totalH + lintel + headwallH;
  const face = new THREE.Mesh(
    new THREE.ExtrudeGeometry(portalShape(prof, outerHalfW, faceH), { depth: faceDepth, bevelEnabled: false }),
    mat,
  );
  g.add(face);

  // Sims / Zinnenkranz
  const crown = new THREE.Mesh(
    new THREE.BoxGeometry(outerHalfW * 2 + 8, 6, faceDepth + 6),
    mat,
  );
  crown.position.set(0, faceH + 3, faceDepth / 2);
  g.add(crown);

  // Tiefer Kragen hinter der Stirnwand – schließt den Berg lückenlos an
  const collar = new THREE.Mesh(
    new THREE.ExtrudeGeometry(portalShape(prof, outerHalfW - 4, prof.totalH + 14), {
      depth: collarDepth,
      bevelEnabled: false,
    }),
    mat,
  );
  collar.position.z = faceDepth;
  g.add(collar);

  // Flügelmauern (flügeln nach vorn außen in den Geländeeinschnitt)
  const wingW = isMulti ? 56 : 48;
  const wingH = prof.totalH + 4;
  const wingD = 10;
  const wingAngle = Math.PI * 0.16;
  const dx = outerHalfW + (wingW / 2) * Math.cos(wingAngle);
  const dz = -(wingW / 2) * Math.sin(wingAngle);

  const wingL = new THREE.Mesh(new THREE.BoxGeometry(wingW, wingH, wingD), mat);
  wingL.position.set(-dx, wingH / 2, dz);
  wingL.rotation.y = -wingAngle;
  g.add(wingL);

  const wingR = new THREE.Mesh(new THREE.BoxGeometry(wingW, wingH, wingD), mat);
  wingR.position.set(dx, wingH / 2, dz);
  wingR.rotation.y = wingAngle;
  g.add(wingR);

  // Fundamente unter den Pfeilern & Flügelmauern (Gleisbereich [-c, c] bleibt frei!)
  const sideW = outerHalfW - c + wingW * Math.cos(wingAngle) + 6;
  const foundH = Math.max(24, (at.elev ?? 0) + 16);
  const minZ = -wingW * Math.sin(wingAngle) - 2;
  const maxZ = faceDepth + 6;
  const foundD = maxZ - minZ;
  const foundZ = (minZ + maxZ) / 2;

  const foundLeft = new THREE.Mesh(new THREE.BoxGeometry(sideW, foundH, foundD), mat);
  foundLeft.position.set(-(c + sideW / 2), -foundH / 2, foundZ);
  g.add(foundLeft);

  const foundRight = new THREE.Mesh(new THREE.BoxGeometry(sideW, foundH, foundD), mat);
  foundRight.position.set(c + sideW / 2, -foundH / 2, foundZ);
  g.add(foundRight);

  parent.add(g);
}