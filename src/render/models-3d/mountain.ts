import * as THREE from "three";
import { Layout, Model3D } from "@/core/types";
import { deg2rad } from "@/core/geometry";
import { isWasmReady, buildTunnelSweepPositionsKernel } from "@/core/wasm-bridge";
import { collectTerrainSupportPolygons, isPointInAnyPolygon } from "./ground-terrain";
import {
  TunnelProfile,
  MountainTrackCollision,
  TunnelPortalInfo,
  TunnelRun,
  R_PORTAL,
  APPROACH_SLOPE,
  PORTAL_LIFT,
  createTunnelProfile,
  evaluateMountainLocal,
  worldToMountainLocal,
  computeTunnelRuns,
} from "./mountain-tunnel";

export * from "./mountain-tunnel";

/* ------------------------------------------------------------------ */
/*  Geometrie-Helfer für Portale & Tunnelröhren                        */
/* ------------------------------------------------------------------ */

/** Querschnittspunkte von links unten über den Bogen nach rechts unten. */
export function profilePoints(p: TunnelProfile, inset = 0, arcSteps = 14): Array<[number, number]> {
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
export function portalShape(p: TunnelProfile, outerHalfW: number, outerH: number): THREE.Shape {
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
export function buildTunnelSweep(
  path: THREE.Vector3[],
  p: TunnelProfile,
  mat: THREE.Material,
  inset = 1.0,
): THREE.Mesh | null {
  if (path.length < 2) return null;
  const section = profilePoints(p, inset);
  const ns = section.length;
  let posArray: Float32Array | number[] | null = null;

  if (isWasmReady()) {
    const pathFlat = new Float32Array(path.length * 3);
    for (let i = 0; i < path.length; i++) {
      pathFlat[i * 3] = path[i]!.x;
      pathFlat[i * 3 + 1] = path[i]!.y;
      pathFlat[i * 3 + 2] = path[i]!.z;
    }
    const secFlat = new Float32Array(ns * 2);
    for (let j = 0; j < ns; j++) {
      secFlat[j * 2] = section[j]![0];
      secFlat[j * 2 + 1] = section[j]![1];
    }
    posArray = buildTunnelSweepPositionsKernel(pathFlat, secFlat);
  }

  if (!posArray) {
    const positions: number[] = [];
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
        sx = tz / len;
        sz = -tx / len;
      }
      prevSx = sx;
      prevSz = sz;

      const c = path[i]!;
      for (const [x, y] of section) positions.push(c.x + sx * x, c.y + y, c.z + sz * x);
    }
    posArray = positions;
  }

  const indices: number[] = [];
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
  geo.setAttribute("position", new THREE.Float32BufferAttribute(posArray, 3));
  geo.setIndex(indices);
  geo.computeVertexNormals();
  return new THREE.Mesh(geo, mat);
}

export function buildPortal(
  parent: THREE.Group,
  prof: TunnelProfile,
  at: { lx: number; lz: number; elev: number },
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

/* ------------------------------------------------------------------ */
/*  Berg-Mesh & Gelände-Einschnitt                                     */
/* ------------------------------------------------------------------ */

const smoothstep = (a: number, b: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

function pushMountainVertexColor(colors: number[], hRatio: number, style?: string): void {
  if (style === "rocky") {
    if (hRatio > 0.88) colors.push(0.78, 0.78, 0.8);
    else if (hRatio > 0.22) colors.push(0.34, 0.33, 0.31);
    else colors.push(0.28, 0.34, 0.2);
  } else if (style === "forest") {
    if (hRatio > 0.78) colors.push(0.38, 0.36, 0.33);
    else if (hRatio > 0.28) colors.push(0.14, 0.28, 0.11);
    else colors.push(0.2, 0.35, 0.12);
  } else if (style === "meadow") {
    if (hRatio > 0.65) colors.push(0.32, 0.44, 0.16);
    else colors.push(0.24, 0.4, 0.13);
  } else {
    // "alpine" (Standard)
    if (hRatio > 0.82) colors.push(0.96, 0.98, 1.0); // Schnee
    else if (hRatio > 0.35) colors.push(0.36, 0.34, 0.32); // Fels
    else colors.push(0.24, 0.38, 0.12); // Wiese
  }
}

const SHARED_STONE_PORTAL_MAT = new THREE.MeshStandardMaterial({
  color: 0x52525b,
  roughness: 0.85,
  metalness: 0.12,
  side: THREE.DoubleSide,
  polygonOffset: true,
  polygonOffsetFactor: -1,
  polygonOffsetUnits: -1,
});
SHARED_STONE_PORTAL_MAT.userData.shared = true;

const SHARED_TUNNEL_LINING_MAT = new THREE.MeshStandardMaterial({
  color: 0x27272a,
  roughness: 0.96,
  metalness: 0.05,
  side: THREE.DoubleSide,
  polygonOffset: true,
  polygonOffsetFactor: -1,
  polygonOffsetUnits: -1,
});
SHARED_TUNNEL_LINING_MAT.userData.shared = true;

const SHARED_MOUNTAIN_MAT = new THREE.MeshStandardMaterial({
  vertexColors: true,
  roughness: 0.9,
  metalness: 0.08,
  flatShading: true,
  polygonOffset: true,
  polygonOffsetFactor: -1,
  polygonOffsetUnits: -1,
});
SHARED_MOUNTAIN_MAT.userData.shared = true;

interface PreparedCollisionCorridor {
  prof: TunnelProfile;
  inner: number;
  outerInside: number;
  outerApproach: number;
  minLx: number;
  maxLx: number;
  minLz: number;
  maxLz: number;
  pts: Array<{ lx: number; lz: number; elevation: number }>;
}

/**
 * Erstellt die 3D-Geometrie für einen parametrischen oder per Pinsel gezeichneten Berg
 * mit Tunnelportalen (Gleise: Rundbogen, Straßen: breiterer Segmentbogen).
 */
export function buildMountain3D(
  group: THREE.Group,
  model?: Model3D,
  collisionsOrData?:
    | MountainTrackCollision[]
    | { collisions?: MountainTrackCollision[]; allMountains?: Model3D[]; layout?: Layout },
  allMountainsArg?: Model3D[],
): void {
  const collisions = Array.isArray(collisionsOrData)
    ? collisionsOrData
    : collisionsOrData?.collisions;
  const allMountains = allMountainsArg ?? (!Array.isArray(collisionsOrData) ? collisionsOrData?.allMountains : undefined);
  const layout = !Array.isArray(collisionsOrData) ? collisionsOrData?.layout : undefined;
  const width = model?.width ?? 600;
  const depth = model?.depth ?? 450;
  const height = model?.height ?? 220;
  const hasCollisions = !!(collisions && collisions.length > 0 && model);

  const mountainGroup = new THREE.Group();
  // Höhen-Offset zur Vermeidung von Z-Fighting mit der Tischplatte
  mountainGroup.position.y = 0.5;

  const stonePortalMat = SHARED_STONE_PORTAL_MAT;
  const tunnelLiningMat = SHARED_TUNNEL_LINING_MAT;
  const mountainMat = SHARED_MOUNTAIN_MAT;

  // Tunnel-Routen und Portale vorberechnen
  const tunnelRuns: TunnelRun[] = hasCollisions
    ? computeTunnelRuns(model, collisions, R_PORTAL, allMountains)
    : [];
  const uniquePortals = new Set<TunnelPortalInfo>();
  for (const tr of tunnelRuns) {
    if (tr.startPortal) uniquePortals.add(tr.startPortal);
    if (tr.endPortal) uniquePortals.add(tr.endPortal);
  }
  const allPortals = Array.from(uniquePortals);

  // Kollisions-Korridore in Berg-lokale Koordinaten + Bounding-Boxes vorberechnen (O(N) statt pro Vertex)
  const preparedCollisions: PreparedCollisionCorridor[] = [];
  if (hasCollisions) {
    for (const col of collisions!) {
      const prof = col.prof ?? createTunnelProfile(col.isRoad, col.hasCatenary, col.gauge);
      const inner = prof.innerWidth / 2 + prof.pier;
      const outerInside = inner + 45;
      const outerApproach = inner + APPROACH_SLOPE;
      const maxReach = Math.max(outerInside, outerApproach);

      let minLx = Infinity;
      let maxLx = -Infinity;
      let minLz = Infinity;
      let maxLz = -Infinity;
      const pts: Array<{ lx: number; lz: number; elevation: number }> = [];

      for (const pt of col.centerline) {
        if (pt.elevation < -35 || pt.elevation > height - 10) continue;
        const { lx, lz } = worldToMountainLocal(pt.x, pt.y, model!);
        if (lx - maxReach < minLx) minLx = lx - maxReach;
        if (lx + maxReach > maxLx) maxLx = lx + maxReach;
        if (lz - maxReach < minLz) minLz = lz - maxReach;
        if (lz + maxReach > maxLz) maxLz = lz + maxReach;
        pts.push({ lx, lz, elevation: pt.elevation });
      }

      if (pts.length > 0) {
        preparedCollisions.push({
          prof,
          inner,
          outerInside,
          outerApproach,
          minLx,
          maxLx,
          minLz,
          maxLz,
          pts,
        });
      }
    }
  }

  // 1. Geländegitter (gerade Segmentanzahl garantiert exakte Mittel- und Randknoten)
  const isPathMountain = !!(model?.mountainPath && model.mountainPath.length > 0);
  const segsX = isPathMountain ? 52 : 40;
  const segsZ = isPathMountain ? 52 : 40;
  const positions: number[] = [];
  const colors: number[] = [];
  const indices: number[] = [];
  const insideSupport: boolean[] = [];

  const supportPolys = collectTerrainSupportPolygons(layout, model?.layerId);
  const clipToSupport = supportPolys.length > 0 && !!model;
  const mRotRad = deg2rad(model?.transform.rotation ?? 0);
  const mCos = Math.cos(mRotRad);
  const mSin = Math.sin(mRotRad);
  const mOriginX = model?.transform.x ?? 0;
  const mOriginZ = model?.transform.y ?? 0;

  for (let iz = 0; iz <= segsZ; iz++) {
    const lz = (iz / segsZ - 0.5) * depth;

    for (let ix = 0; ix <= segsX; ix++) {
      const lx = (ix / segsX - 0.5) * width;

      const wx = mOriginX + (lx * mCos - lz * mSin);
      const wz = mOriginZ + (lx * mSin + lz * mCos);
      const inBounds = !clipToSupport || isPointInAnyPolygon(wx, wz, supportPolys);
      insideSupport.push(inBounds);

      if (!inBounds) {
        positions.push(lx, 0, lz);
        pushMountainVertexColor(colors, 0, model?.mountainStyle);
        continue;
      }

      const evalRes = evaluateMountainLocal(lx, lz, model);
      const rDist = evalRes.normR;
      let baseH = evalRes.baseH;

      // Äußere Randpunkte (rDist >= 1.0) bleiben exakt auf Tischplatten-Niveau (0)
      if (preparedCollisions.length > 0 && rDist < 0.999) {
        const insidePortal = rDist <= R_PORTAL;

        for (let ci = 0; ci < preparedCollisions.length; ci++) {
          const col = preparedCollisions[ci]!;
          if (lx < col.minLx || lx > col.maxLx || lz < col.minLz || lz > col.maxLz) continue;

          const inner = col.inner;
          const outer = insidePortal ? col.outerInside : col.outerApproach;
          const pts = col.pts;

          for (let pi = 0; pi < pts.length; pi++) {
            const pt = pts[pi]!;
            const dx = lx - pt.lx;
            if (dx > outer || dx < -outer) continue;
            const dz = lz - pt.lz;
            if (dz > outer || dz < -outer) continue;
            const d = Math.hypot(dx, dz);
            if (d > outer) continue;

            const w = 1 - smoothstep(inner, outer, d);
            if (insidePortal) {
              // Berginneres: Vollständige Deckung über der Tunnelröhre und dem Kragen
              const target = pt.elevation + col.prof.totalH + 16;
              if (baseH < target) baseH += (target - baseH) * w;
            } else {
              // Vorfeld vor dem Portal: Einschnitt mit sanfter Böschung zur Trasse hin
              const target = pt.elevation;
              if (baseH > target) baseH += (target - baseH) * w;
            }
          }
        }
      }

      positions.push(lx, baseH, lz);
      const hRatio = height > 0 ? baseH / height : 0;
      pushMountainVertexColor(colors, hRatio, model?.mountainStyle);
    }
  }

  // Erkennt Quads, die in die Portalöffnung, den Portalkragen oder das unmittelbare Vorfeld ragen
  function isEntranceCurtainQuad(
    lx0: number, lz0: number, y0: number,
    lx1: number, lz1: number, y1: number,
    lx2: number, lz2: number, y2: number,
    lx3: number, lz3: number, y3: number,
  ): boolean {
    if (allPortals.length === 0) return false;
    const minY = Math.min(y0, y1, y2, y3);
    const maxY = Math.max(y0, y1, y2, y3);
    const verts: Array<[number, number]> = [
      [lx0, lz0],
      [lx1, lz1],
      [lx2, lz2],
      [lx3, lz3],
    ];
    const cx = (lx0 + lx1 + lx2 + lx3) * 0.25;
    const cz = (lz0 + lz1 + lz2 + lz3) * 0.25;
    verts.push([cx, cz]);

    for (const po of allPortals) {
      // Liegt komplett über der Decke / Krone des Portals -> Bergdach bleibt vollständig erhalten
      if (minY >= po.at.elev + po.prof.totalH) continue;
      // Liegt komplett unter der Schienenunterkante / Fundament -> bleibt erhalten
      if (maxY <= po.at.elev - 2) continue;

      let minFwd = Infinity;
      let maxFwd = -Infinity;
      let minLat = Infinity;
      let maxLat = -Infinity;
      for (const [vx, vz] of verts) {
        const dx = vx - po.at.lx;
        const dz = vz - po.at.lz;
        const fwd = dx * po.dir.x + dz * po.dir.z;
        const lat = dx * (-po.dir.z) + dz * po.dir.x;
        if (fwd < minFwd) minFwd = fwd;
        if (fwd > maxFwd) maxFwd = fwd;
        if (lat < minLat) minLat = lat;
        if (lat > maxLat) maxLat = lat;
      }

      const halfClearW = po.prof.innerWidth / 2 + 4;
      const latOverlaps = maxLat >= -halfClearW && minLat <= halfClearW;
      const fwdOverlaps = maxFwd >= -42 && minFwd <= 52;

      if (latOverlaps && fwdOverlaps) {
        return true;
      }
    }
    return false;
  }

  for (let iz = 0; iz < segsZ; iz++) {
    for (let ix = 0; ix < segsX; ix++) {
      const i0 = iz * (segsX + 1) + ix;
      const i1 = i0 + 1;
      const i2 = (iz + 1) * (segsX + 1) + ix;
      const i3 = i2 + 1;

      const y0 = positions[i0 * 3 + 1]!;
      const y1 = positions[i1 * 3 + 1]!;
      const y2 = positions[i2 * 3 + 1]!;
      const y3 = positions[i3 * 3 + 1]!;

      // Flache grüne Bodenplatten-Quads außerhalb des Bergfußes (y ≈ 0 an allen 4 Ecken)
      // vollständig auslassen, damit keine ebene Platte über Drehscheiben-Gruben o. ä. liegt
      if (y0 <= 0.1 && y1 <= 0.1 && y2 <= 0.1 && y3 <= 0.1) {
        continue;
      }

      if (
        isEntranceCurtainQuad(
          positions[i0 * 3]!, positions[i0 * 3 + 2]!, y0,
          positions[i1 * 3]!, positions[i1 * 3 + 2]!, y1,
          positions[i2 * 3]!, positions[i2 * 3 + 2]!, y2,
          positions[i3 * 3]!, positions[i3 * 3 + 2]!, y3,
        )
      ) {
        continue;
      }

      // Nur Dreiecke mit tatsächlicher Berghöhe innerhalb der Tisch-/Plattenfläche ausgeben
      if (
        (y0 > 0.1 || y2 > 0.1 || y1 > 0.1) &&
        insideSupport[i0] &&
        insideSupport[i2] &&
        insideSupport[i1]
      ) {
        indices.push(i0, i2, i1);
      }
      if (
        (y1 > 0.1 || y2 > 0.1 || y3 > 0.1) &&
        insideSupport[i1] &&
        insideSupport[i2] &&
        insideSupport[i3]
      ) {
        indices.push(i1, i2, i3);
      }
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

  // 2. Portale (nur nicht-überlappte Portale am echten Gebirgsrand zeichnen, exakt auf Gleishöhe)
  const mountainYOffset = mountainGroup.position.y;
  for (const po of allPortals) {
    if (po.suppressedByOverlap) continue;
    buildPortal(
      mountainGroup,
      po.prof,
      { lx: po.at.lx, lz: po.at.lz, elev: po.at.elev - mountainYOffset },
      po.dir,
      stonePortalMat,
    );
  }

  // 3. Tunnelröhren (exakt auf Gleishöhe)
  for (const tr of tunnelRuns) {
    let pts = tr.pts;
    // Bei unterdrücktem Portal an einer Berg-Überlappung Röhre leicht verlängern, damit kein Spalt entsteht
    if (tr.startPortal?.suppressedByOverlap && pts.length >= 2) {
      const p0 = pts[0]!;
      const d = tr.startPortal.dir;
      pts = [{ lx: p0.lx - d.x * 35, lz: p0.lz - d.z * 35, elev: p0.elev, normR: p0.normR }, ...pts];
    }
    if (tr.endPortal?.suppressedByOverlap && pts.length >= 2) {
      const pN = pts[pts.length - 1]!;
      const d = tr.endPortal.dir;
      pts = [...pts, { lx: pN.lx - d.x * 35, lz: pN.lz - d.z * 35, elev: pN.elev, normR: pN.normR }];
    }
    const path = pts.map((p) => new THREE.Vector3(p.lx, p.elev - mountainYOffset + PORTAL_LIFT, p.lz));
    const tube = buildTunnelSweep(path, tr.prof, tunnelLiningMat);
    if (tube) mountainGroup.add(tube);
  }

  group.add(mountainGroup);
}
