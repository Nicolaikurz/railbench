import * as THREE from "three";
import type { Model3D, TrackPiece } from "@/core/types";
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
  centerline: Array<{ x: number; y: number; elevation: number }>;
}

/** Querschnitt eines Tunnels: senkrechte Wände + Kreisbogen (Segmentbogen bei Straßen). */
interface TunnelProfile {
  isRoad: boolean;
  innerWidth: number; // lichte Breite
  wallH: number; // Höhe der senkrechten Wände
  archRise: number; // Stich des Bogens
  pier: number; // Breite der Portalpfeiler links/rechts
  totalH: number; // wallH + archRise
}

const RAIL_PROFILE: TunnelProfile = {
  isRoad: false, innerWidth: 60, wallH: 56, archRise: 28, pier: 16, totalH: 84,
};
const ROAD_PROFILE: TunnelProfile = {
  isRoad: true, innerWidth: 80, wallH: 52, archRise: 20, pier: 14, totalH: 72,
};

const profileFor = (isRoad: boolean): TunnelProfile => (isRoad ? ROAD_PROFILE : RAIL_PROFILE);

/** Relative Berggrenze (0..1, elliptisch), an der die Portale sitzen. */
export const R_PORTAL = 0.82;
const ELLIPSE_TOLERANCE = 1.05;
const APPROACH_SLOPE = 30; // Böschungsbreite des Einschnitts vor dem Portal

/* ------------------------------------------------------------------ */
/*  Kollisionserkennung                                                */
/* ------------------------------------------------------------------ */

/**
 * Ermittelt alle Gleise und Straßen, die durch die Grundfläche des Berges verlaufen.
 * Gleise und Straßen werden getrennt zu Trassenketten gruppiert, damit eine Kette
 * nie gemischt ist und jeweils das passende Tunnelprofil bekommt.
 */
export function findCollidingTracksForMountain(
  mountain: Model3D,
  pieces: TrackPiece[],
  findEntry: (catalogId: string) => CatalogEntry | undefined,
): MountainTrackCollision[] {
  const mPos = mountain.transform;
  const mRotRad = deg2rad(mPos.rotation);
  const cosR = Math.cos(-mRotRad);
  const sinR = Math.sin(-mRotRad);

  const halfW = mountain.width / 2;
  const halfD = mountain.depth / 2;
  const boundR = Math.max(halfW, halfD) * 1.15;

  // 1. Grober Filter, getrennt nach Gleis / Straße
  const nearbyRail: TrackPiece[] = [];
  const nearbyRoad: TrackPiece[] = [];
  for (const piece of pieces) {
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

    for (const chain of groupConnectedPieces(group)) {
      if (chain.length === 0) continue;

      const centerline: Array<{ x: number; y: number; elevation: number }> = [];
      let insideCount = 0;
      let representativeEntry: CatalogEntry | undefined;

      for (const item of chain) {
        const piece = item.piece;
        const entry = findEntry(piece.catalogId);
        if (!representativeEntry && entry) representativeEntry = entry;

        const startElev = piece.elevation?.start ?? 0;
        const endElev = piece.elevation?.end ?? 0;
        const pts = getPieceCenterlinePoints(piece, entry, item.fromConn, item.toConn, 10);

        for (let i = 0; i < pts.length; i++) {
          const pt = pts[i]!;
          const last = centerline[centerline.length - 1];
          if (last && Math.hypot(pt.x - last.x, pt.y - last.y) < 1.0) continue;

          const t = pts.length > 1 ? i / (pts.length - 1) : 0;
          const dx = pt.x - mPos.x;
          const dy = pt.y - mPos.y;
          const lx = dx * cosR - dy * sinR;
          const lz = dx * sinR + dy * cosR;
          if (Math.hypot(lx / halfW, lz / halfD) <= ELLIPSE_TOLERANCE) insideCount++;

          centerline.push({ x: pt.x, y: pt.y, elevation: startElev + (endElev - startElev) * t });
        }
      }

      if (insideCount > 0 && centerline.length >= 2) {
        collisions.push({
          piece: chain[0]!.piece,
          entry: representativeEntry,
          isRoad,
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

  const stonePortalMat = new THREE.MeshStandardMaterial({
    color: 0x52525b, roughness: 0.85, metalness: 0.12, side: THREE.DoubleSide,
  });
  const tunnelLiningMat = new THREE.MeshStandardMaterial({
    color: 0x27272a, roughness: 0.96, metalness: 0.05, side: THREE.DoubleSide,
  });
  const mountainMat = new THREE.MeshStandardMaterial({
    vertexColors: true, roughness: 0.9, metalness: 0.08, flatShading: true,
  });

  // lokal -> Welt
  const mRotRad = model ? deg2rad(model.transform.rotation) : 0;
  const cosF = Math.cos(mRotRad);
  const sinF = Math.sin(mRotRad);
  const toWorld = (lx: number, lz: number) => ({
    wx: (model?.transform.x ?? 0) + (lx * cosF - lz * sinF),
    wy: (model?.transform.y ?? 0) + (lx * sinF + lz * cosF),
  });

  function mountainNoise(nx: number, nz: number): number {
    const s1 = Math.sin(nx * 3.1415 + 0.5) * Math.cos(nz * 3.1415 + 0.3);
    const s2 = Math.sin(nx * 6.28 + nz * 4.71) * 0.35;
    const s3 = Math.cos(nx * 9.42 - nz * 7.85) * 0.15;
    return s1 + s2 + s3;
  }

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
      let baseH = 0;
      if (rDist < 1.0) {
        const profile = Math.cos(rDist * (Math.PI / 2)) ** 1.35;
        baseH = Math.max(0, height * profile * (1 + 0.3 * mountainNoise(nx, nz)));
      }

      if (hasCollisions) {
        const { wx, wy } = toWorld(lx, lz);
        const insidePortal = rDist <= R_PORTAL;

        for (const col of collisions!) {
          const prof = profileFor(col.isRoad);
          const inner = prof.innerWidth / 2 + prof.pier; // = Breite der Portal-Stirnwand
          for (const pt of col.centerline) {
            const d = Math.hypot(wx - pt.x, wy - pt.y);

            if (insidePortal) {
              // Berginneres: Vollständige Deckung über der Tunnelröhre und dem Kragen
              const outer = inner + 45;
              if (d > outer) continue;
              const w = 1 - smoothstep(inner, outer, d);
              const target = pt.elevation + prof.totalH + 16;
              if (baseH < target) baseH += (target - baseH) * w;
            } else {
              // Vorfeld vor dem Portal: Einschnitt mit sanfter Böschung zur Trasse hin
              const outer = inner + APPROACH_SLOPE;
              if (d > outer) continue;
              const w = 1 - smoothstep(inner, outer, d);
              const target = Math.max(0, pt.elevation);
              if (baseH > target) baseH += (target - baseH) * w;
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

  // Erkennt Quads, die als steile Erdwand direkt in der Portalöffnung hängen würden
  function isEntranceCurtainQuad(
    lx0: number, lz0: number, y0: number,
    lx1: number, lz1: number, y1: number,
    lx2: number, lz2: number, y2: number,
    lx3: number, lz3: number, y3: number,
  ): boolean {
    if (!hasCollisions) return false;
    const minY = Math.min(y0, y1, y2, y3);
    const maxY = Math.max(y0, y1, y2, y3);
    if (maxY - minY < 32) return false;

    const cx = (lx0 + lx1 + lx2 + lx3) * 0.25;
    const cz = (lz0 + lz1 + lz2 + lz3) * 0.25;
    const rQuad = Math.hypot(cx / (width / 2), cz / (depth / 2));
    if (Math.abs(rQuad - R_PORTAL) > 0.12) return false;

    const { wx, wy } = toWorld(cx, cz);
    for (const col of collisions!) {
      const prof = profileFor(col.isRoad);
      const clearW = prof.innerWidth / 2 + 3;
      for (const pt of col.centerline) {
        if (Math.hypot(wx - pt.x, wy - pt.y) <= clearW) {
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

  // 2. Portale + Tunnelröhren
  if (hasCollisions) {
    for (const col of collisions!) {
      buildTunnel(mountainGroup, model!, col, stonePortalMat, tunnelLiningMat, R_PORTAL);
    }
  }

  group.add(mountainGroup);
}

/* ------------------------------------------------------------------ */
/*  Tunnel (Portale + Röhre)                                           */
/* ------------------------------------------------------------------ */

interface LocalPt { lx: number; lz: number; elev: number; normR: number }
interface InsideRun { pts: LocalPt[]; openStart: boolean; openEnd: boolean }

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
  if (cur) runs.push(cur);
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

function buildPortal(
  parent: THREE.Group,
  prof: TunnelProfile,
  at: LocalPt,
  dir: { x: number; z: number },
  mat: THREE.Material,
): void {
  const g = new THREE.Group();
  g.position.set(at.lx, at.elev, at.lz);
  g.rotation.y = Math.atan2(dir.x, dir.z); // lokale +Z zeigt in den Berg

  const c = prof.innerWidth / 2;
  const outerHalfW = c + prof.pier;
  const lintel = prof.isRoad ? 12 : 16;
  const headwallH = prof.isRoad ? 22 : 28;
  const faceDepth = 14;
  const collarDepth = 65;

  // Stirnwand mit Bogenöffnung (ein Stück, keine Lücken zwischen Pfeilern und Sturz)
  const faceH = prof.totalH + lintel + headwallH;
  const face = new THREE.Mesh(
    new THREE.ExtrudeGeometry(portalShape(prof, outerHalfW, faceH), { depth: faceDepth, bevelEnabled: false }),
    mat,
  );
  g.add(face);

  // Sims / Zinnenkranz
  const crown = new THREE.Mesh(
    new THREE.BoxGeometry(outerHalfW * 2 + 6, 6, faceDepth + 6),
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

  // Flügelmauern (links +angle, rechts -angle, damit sie in den Hang laufen)
  const wingW = 48;
  const wingH = prof.totalH + 4;
  const wingD = 10;
  const wingAngle = Math.PI * 0.16;
  const dx = outerHalfW + (wingW / 2) * Math.cos(wingAngle);
  const dz = (wingW / 2) * Math.sin(wingAngle);

  const wingL = new THREE.Mesh(new THREE.BoxGeometry(wingW, wingH, wingD), mat);
  wingL.position.set(-dx, wingH / 2, dz);
  wingL.rotation.y = wingAngle;
  g.add(wingL);

  const wingR = new THREE.Mesh(new THREE.BoxGeometry(wingW, wingH, wingD), mat);
  wingR.position.set(dx, wingH / 2, dz);
  wingR.rotation.y = -wingAngle;
  g.add(wingR);

  // Fundamente unter den Pfeilern & Flügelmauern (Gleisbereich [-c, c] bleibt frei!)
  const sideW = outerHalfW - c + wingW * 0.95;
  const foundH = 14;
  const foundD = faceDepth + 24;

  const foundLeft = new THREE.Mesh(new THREE.BoxGeometry(sideW, foundH, foundD), mat);
  foundLeft.position.set(-(c + sideW / 2), -foundH / 2, foundD / 2 - 4);
  g.add(foundLeft);

  const foundRight = new THREE.Mesh(new THREE.BoxGeometry(sideW, foundH, foundD), mat);
  foundRight.position.set(c + sideW / 2, -foundH / 2, foundD / 2 - 4);
  g.add(foundRight);

  parent.add(g);
}

function buildTunnel(
  parent: THREE.Group,
  mountain: Model3D,
  collision: MountainTrackCollision,
  portalMat: THREE.Material,
  liningMat: THREE.Material,
  rPortal: number,
): void {
  const pts = collision.centerline;
  if (pts.length < 2) return;

  const mPos = mountain.transform;
  const mRotRad = deg2rad(mPos.rotation);
  const cosR = Math.cos(-mRotRad);
  const sinR = Math.sin(-mRotRad);
  const halfW = mountain.width / 2;
  const halfD = mountain.depth / 2;
  const prof = profileFor(collision.isRoad);

  const local: LocalPt[] = pts.map((p) => {
    const dx = p.x - mPos.x;
    const dy = p.y - mPos.y;
    const lx = dx * cosR - dy * sinR;
    const lz = dx * sinR + dy * cosR;
    return { lx, lz, elev: p.elevation || 0, normR: Math.hypot(lx / halfW, lz / halfD) };
  });

  for (const run of extractInsideRuns(local, rPortal, halfW, halfD)) {
    if (run.openStart) buildPortal(parent, prof, run.pts[0]!, inwardDir(run.pts, true), portalMat);
    if (run.openEnd) {
      buildPortal(parent, prof, run.pts[run.pts.length - 1]!, inwardDir(run.pts, false), portalMat);
    }

    const path = run.pts.map((p) => new THREE.Vector3(p.lx, p.elev, p.lz));
    const tube = buildTunnelSweep(path, prof, liningMat);
    if (tube) parent.add(tube);
  }
}