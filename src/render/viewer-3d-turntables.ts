import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import type { TrackPiece } from "@/core/types";
import { deg2rad } from "@/core/geometry";

interface SharedMats {
  ballast: THREE.MeshStandardMaterial;
  concrete: THREE.MeshStandardMaterial;
  concreteDark: THREE.MeshStandardMaterial;
  pitWall: THREE.MeshStandardMaterial;
  pitFloor: THREE.MeshStandardMaterial;
  sump: THREE.MeshStandardMaterial;
  steelGirder: THREE.MeshStandardMaterial;
  steelDark: THREE.MeshStandardMaterial;
  steelDeck: THREE.MeshStandardMaterial;
  wheelSteel: THREE.MeshStandardMaterial;
  bronze: THREE.MeshStandardMaterial;
  wood: THREE.MeshStandardMaterial;
  rail: THREE.MeshStandardMaterial;
  cabWall: THREE.MeshStandardMaterial;
  cabTrim: THREE.MeshStandardMaterial;
  cabRoof: THREE.MeshStandardMaterial;
  glass: THREE.MeshStandardMaterial;
  lampWarm: THREE.MeshStandardMaterial;
  signalRed: THREE.MeshStandardMaterial;
  hazardYellow: THREE.MeshStandardMaterial;
  insulator: THREE.MeshStandardMaterial;
}

function createSharedMats(ballastColor: number): SharedMats {
  return {
    ballast: new THREE.MeshStandardMaterial({ color: ballastColor, roughness: 0.9 }),
    concrete: new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.88 }),
    concreteDark: new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.90 }),
    pitWall: new THREE.MeshStandardMaterial({ color: 0x526071, roughness: 0.88, side: THREE.DoubleSide }),
    pitFloor: new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.94 }),
    sump: new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.95 }),
    steelGirder: new THREE.MeshStandardMaterial({ color: 0x364a46, roughness: 0.46, metalness: 0.68 }),
    steelDark: new THREE.MeshStandardMaterial({ color: 0x1f2937, roughness: 0.42, metalness: 0.76 }),
    steelDeck: new THREE.MeshStandardMaterial({ color: 0x2d3748, roughness: 0.56, metalness: 0.58 }),
    wheelSteel: new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.28, metalness: 0.88 }),
    bronze: new THREE.MeshStandardMaterial({ color: 0xb45309, roughness: 0.34, metalness: 0.82 }),
    wood: new THREE.MeshStandardMaterial({ color: 0x3e2723, roughness: 0.85 }),
    rail: new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25 }),
    cabWall: new THREE.MeshStandardMaterial({ color: 0x2f453f, roughness: 0.72 }),
    cabTrim: new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.48, metalness: 0.62 }),
    cabRoof: new THREE.MeshStandardMaterial({ color: 0x3f4c59, roughness: 0.52, metalness: 0.48 }),
    glass: new THREE.MeshStandardMaterial({
      color: 0x93c5fd,
      emissive: 0xfde68a,
      emissiveIntensity: 0.22,
      roughness: 0.16,
      metalness: 0.75,
    }),
    lampWarm: new THREE.MeshStandardMaterial({
      color: 0xfef9c3,
      emissive: 0xfef08a,
      emissiveIntensity: 0.95,
      roughness: 0.25,
    }),
    signalRed: new THREE.MeshStandardMaterial({
      color: 0xef4444,
      emissive: 0xef4444,
      emissiveIntensity: 0.85,
      roughness: 0.25,
    }),
    hazardYellow: new THREE.MeshStandardMaterial({ color: 0xeab308, roughness: 0.58 }),
    insulator: new THREE.MeshStandardMaterial({ color: 0x7c2d12, roughness: 0.55 }),
  };
}

function addBox(
  group: THREE.Group,
  x: number,
  y: number,
  z: number,
  w: number,
  h: number,
  d: number,
  mat: THREE.Material,
  castShadow = true,
  receiveShadow = true,
): THREE.Mesh {
  const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat);
  m.position.set(x, y, z);
  m.castShadow = castShadow;
  m.receiveShadow = receiveShadow;
  group.add(m);
  return m;
}

function addStrut(
  group: THREE.Group,
  p0: THREE.Vector3,
  p1: THREE.Vector3,
  width: number,
  depth: number,
  mat: THREE.Material,
) {
  const dir = new THREE.Vector3().subVectors(p1, p0);
  const len = dir.length();
  if (len < 1e-3) return;
  const m = new THREE.Mesh(new THREE.BoxGeometry(width, len, depth), mat);
  m.position.copy(p0).add(p1).multiplyScalar(0.5);
  m.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), dir.divideScalar(len));
  group.add(m);
}

function addSafetyHandrailX(
  group: THREE.Group,
  x0: number,
  x1: number,
  yBase: number,
  z: number,
  gap: [number, number] | null,
  scaleF: number,
  steel: THREE.Material,
) {
  const postH = 12.2 * scaleF;
  const topY = yBase + postH;
  const midY = yBase + postH * 0.52;
  const postW = 1.0 * scaleF;
  const railT = 0.85 * scaleF;

  const emitSpan = (sa: number, sb: number) => {
    if (sb - sa < 4 * scaleF) return;
    addBox(group, (sa + sb) * 0.5, topY, z, sb - sa, railT, railT, steel, false, false);
    addBox(group, (sa + sb) * 0.5, midY, z, sb - sa, railT * 0.8, railT * 0.8, steel, false, false);
    const steps = Math.max(1, Math.round((sb - sa) / (24 * scaleF)));
    for (let i = 0; i <= steps; i++) {
      const px = sa + (sb - sa) * (i / steps);
      addBox(group, px, yBase + postH * 0.5, z, postW, postH, postW, steel, false, false);
    }
  };

  if (gap) {
    emitSpan(x0, Math.min(gap[0], x1));
    emitSpan(Math.max(gap[1], x0), x1);
  } else {
    emitSpan(x0, x1);
  }
}

function addGleissperrsignal(
  group: THREE.Group,
  x: number,
  yBase: number,
  z: number,
  scaleF: number,
  m: SharedMats,
) {
  const postH = 5.6 * scaleF;
  addBox(group, x, yBase + postH * 0.5, z, 1.6 * scaleF, postH, 1.6 * scaleF, m.steelDark);
  const boxY = yBase + postH + 2.8 * scaleF;
  addBox(group, x, boxY, z, 5.2 * scaleF, 5.6 * scaleF, 4.2 * scaleF, m.steelDark);
  addBox(group, x, boxY + 3.2 * scaleF, z, 5.8 * scaleF, 0.8 * scaleF, 4.8 * scaleF, m.steelDark);
  for (const sx of [-1, 1]) {
    addBox(group, x + sx * 2.65 * scaleF, boxY - 0.5 * scaleF, z, 0.6 * scaleF, 2.2 * scaleF, 2.2 * scaleF, m.lampWarm, false, false);
    addBox(group, x + sx * 2.65 * scaleF, boxY + 1.4 * scaleF, z, 0.6 * scaleF, 1.1 * scaleF, 2.4 * scaleF, m.signalRed, false, false);
  }
}

function addOperatorCabin(
  group: THREE.Group,
  cx: number,
  cz: number,
  cabW: number,
  cabD: number,
  girderZ: number,
  yDeck: number,
  yGirderBot: number,
  scaleF: number,
  m: SharedMats,
) {
  const cabH = 28.5 * scaleF;
  const waistH = 11.2 * scaleF;
  const fasciaH = 3.2 * scaleF;
  const winY0 = yDeck + waistH;
  const winY1 = yDeck + cabH - fasciaH;
  const winH = winY1 - winY0;
  const winCY = 0.5 * (winY0 + winY1);
  const wallT = 1.4 * scaleF;
  const innerZ = cz - cabD * 0.5;
  const outerZ = cz + cabD * 0.5;
  const spanZ = outerZ - girderZ + 1.0 * scaleF;

  // Steel cantilever floor plate + 3 diagonal knee-braces anchored to main girder
  addBox(group, cx, yDeck - 0.65 * scaleF, 0.5 * (girderZ + outerZ), cabW + 3 * scaleF, 1.3 * scaleF, spanZ, m.steelDark);
  for (const kx of [cx - cabW * 0.4, cx, cx + cabW * 0.4]) {
    addBox(group, kx, yDeck - 1.4 * scaleF, 0.5 * (girderZ + outerZ), 1.6 * scaleF, 1.8 * scaleF, spanZ, m.steelDark);
    addStrut(
      group,
      new THREE.Vector3(kx, yDeck - 1.5 * scaleF, outerZ - 1.5 * scaleF),
      new THREE.Vector3(kx, yGirderBot + 1.0 * scaleF, girderZ + 0.5 * scaleF),
      1.7 * scaleF,
      1.7 * scaleF,
      m.steelDark,
    );
  }

  // Lower wainscoting & upper fascia
  addBox(group, cx, yDeck + waistH * 0.5, cz, cabW, waistH, cabD, m.cabWall);
  addBox(group, cx, yDeck + cabH - fasciaH * 0.5, cz, cabW + 0.8 * scaleF, fasciaH, cabD + 0.8 * scaleF, m.cabWall);
  addBox(group, cx, yDeck + waistH, cz, cabW + 1.2 * scaleF, 1.1 * scaleF, cabD + 1.2 * scaleF, m.cabTrim);
  for (const sx of [-1, 1]) {
    for (const sz of [-1, 1]) {
      addBox(
        group,
        cx + sx * (cabW * 0.5 - 0.8 * scaleF),
        yDeck + cabH * 0.5,
        cz + sz * (cabD * 0.5 - 0.8 * scaleF),
        2.2 * scaleF,
        cabH,
        2.2 * scaleF,
        m.cabTrim,
      );
    }
  }

  // Interior control desk
  addBox(group, cx, winY0 + 1.8 * scaleF, cz - 1.5 * scaleF, cabW * 0.52, 4.2 * scaleF, cabD * 0.35, m.steelDark);

  // Outer (+Z) & side (-X/+X) framed panoramic windows
  addBox(group, cx, winCY, outerZ - wallT * 0.5, cabW - 4 * scaleF, winH, 0.9 * scaleF, m.glass);
  for (const mx of [-cabW * 0.16, cabW * 0.16]) {
    addBox(group, cx + mx, winCY, outerZ - wallT * 0.3, 1.1 * scaleF, winH + 0.4 * scaleF, 1.3 * scaleF, m.cabTrim);
  }
  for (const sx of [-1, 1]) {
    const wx = cx + sx * (cabW * 0.5 - wallT * 0.5);
    addBox(group, wx, winCY, cz, 0.9 * scaleF, winH, cabD - 4.4 * scaleF, m.glass);
    addBox(group, wx + sx * 0.2 * scaleF, winCY, cz, 1.2 * scaleF, winH + 0.4 * scaleF, 1.1 * scaleF, m.cabTrim);
  }

  // Bridge-facing (-Z) observation window & walkway doorway with warm sconce lamp
  const obsX = cx - cabW * 0.16;
  addBox(group, obsX, winCY, innerZ + wallT * 0.5, cabW * 0.46, winH, 0.9 * scaleF, m.glass);
  addBox(group, obsX, winCY, innerZ + wallT * 0.3, 1.1 * scaleF, winH + 0.4 * scaleF, 1.3 * scaleF, m.cabTrim);

  const doorX = cx + cabW * 0.25;
  const doorW = cabW * 0.26;
  const doorH = cabH - 4.2 * scaleF;
  addBox(group, doorX, yDeck + cabH * 0.5, innerZ + wallT * 0.5, doorW + 3 * scaleF, cabH, wallT, m.cabWall);
  addBox(group, doorX, yDeck + doorH * 0.5 + 0.4 * scaleF, innerZ + 0.15 * scaleF, doorW, doorH, 1.0 * scaleF, m.cabTrim);
  addBox(group, doorX, yDeck + doorH * 0.68, innerZ - 0.2 * scaleF, doorW * 0.55, 5.5 * scaleF, 0.6 * scaleF, m.glass);
  addBox(group, doorX, yDeck + doorH + 1.8 * scaleF, innerZ - 1.1 * scaleF, 2.2 * scaleF, 1.6 * scaleF, 1.6 * scaleF, m.lampWarm, false, false);

  // Overhanging pitched roof + ventilator chimney
  const roofY = yDeck + cabH;
  const ridgeH = 6.2 * scaleF;
  const halfSpan = cabD * 0.5 + 2.4 * scaleF;
  const roofLen = cabW + 4.8 * scaleF;
  const roofShape = new THREE.Shape();
  roofShape.moveTo(-halfSpan, 0);
  roofShape.lineTo(0, ridgeH);
  roofShape.lineTo(halfSpan, 0);
  roofShape.lineTo(halfSpan, 1.2 * scaleF);
  roofShape.lineTo(0, ridgeH + 1.2 * scaleF);
  roofShape.lineTo(-halfSpan, 1.2 * scaleF);
  roofShape.closePath();
  const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: roofLen, bevelEnabled: false });
  roofGeo.rotateY(Math.PI / 2);
  const roofMesh = new THREE.Mesh(roofGeo, m.cabRoof);
  roofMesh.position.set(cx - roofLen * 0.5, roofY, cz);
  roofMesh.castShadow = true;
  group.add(roofMesh);

  addBox(group, cx, roofY + ridgeH + 1.2 * scaleF, cz, roofLen + 0.8 * scaleF, 1.8 * scaleF, 2.8 * scaleF, m.cabTrim);
  const stack = new THREE.Mesh(new THREE.CylinderGeometry(1.5 * scaleF, 1.7 * scaleF, 4.8 * scaleF, 8), m.steelDark);
  stack.position.set(cx + cabW * 0.26, roofY + ridgeH + 1.2 * scaleF, cz + cabD * 0.12);
  group.add(stack);
  const cap = new THREE.Mesh(new THREE.CylinderGeometry(2.5 * scaleF, 2.8 * scaleF, 0.9 * scaleF, 8), m.cabTrim);
  cap.position.set(cx + cabW * 0.26, roofY + ridgeH + 3.8 * scaleF, cz + cabD * 0.12);
  group.add(cap);
}

// ============================================================================
//  1. TURNTABLE (DB 23 m Einheitsdrehscheibe — Märklin 74862 / Trix 66862)
// ============================================================================

export function buildTurntable3D(
  group: THREE.Group,
  _piece: TrackPiece,
  entry: CatalogEntry,
  ballastColor: number,
) {
  const outerR = entry.radius ?? 189;
  const scaleF = Math.max(0.4, Math.min(2.5, outerR / 189));
  const pitR = 139 * scaleF;
  const pitDepth = 15 * scaleF;
  const ringRailR = 124 * scaleF;
  const bridgeL = pitR * 2 - 2; // 1 mm clearance at both ends — eliminates old 7.5 mm rail gap
  const gauge = entry.gauge ?? 16.5;
  const hg = gauge * 0.5;
  const girderZ = Math.max(12.6, hg + 4.2);

  // Standard TS track heights: ballast top 6.0, ties 6.0..8.2 (center 7.1), rails 7.1..9.3 (center 8.2, top 9.3)
  const ballastTop = 6.0;
  const tieY = 7.1;
  const railY = 8.2;
  const m = createSharedMats(ballastColor);

  // 1. Outer gravel ballast collar & concrete pit coping
  const ringShape = new THREE.Shape();
  ringShape.absarc(0, 0, outerR, 0, Math.PI * 2, false);
  const holePath = new THREE.Path();
  holePath.absarc(0, 0, pitR + 5.6 * scaleF, 0, Math.PI * 2, true);
  ringShape.holes.push(holePath);
  const ringGeo = new THREE.ExtrudeGeometry(ringShape, { depth: ballastTop - 0.02, bevelEnabled: false, curveSegments: 48 });
  ringGeo.rotateX(Math.PI / 2);
  const ringMesh = new THREE.Mesh(ringGeo, m.ballast);
  ringMesh.position.set(0, ballastTop, 0);
  ringMesh.receiveShadow = true;
  group.add(ringMesh);

  const rimShape = new THREE.Shape();
  rimShape.absarc(0, 0, pitR + 6.4 * scaleF, 0, Math.PI * 2, false);
  const rimHole = new THREE.Path();
  rimHole.absarc(0, 0, pitR - 0.4 * scaleF, 0, Math.PI * 2, true);
  rimShape.holes.push(rimHole);
  const rimGeo = new THREE.ExtrudeGeometry(rimShape, { depth: 6.7, bevelEnabled: false, curveSegments: 48 });
  rimGeo.rotateX(Math.PI / 2);
  const rimMesh = new THREE.Mesh(rimGeo, m.concrete);
  rimMesh.position.set(0, 6.7, 0);
  rimMesh.receiveShadow = true;
  group.add(rimMesh);

  // 2. Concrete pit floor, drainage sump ring & grates, pit walls, 24 pilasters, ladder
  const floorMesh = new THREE.Mesh(new THREE.CylinderGeometry(pitR + 4 * scaleF, pitR + 4 * scaleF, 3, 48), m.pitFloor);
  floorMesh.position.set(0, -pitDepth - 1.5, 0);
  floorMesh.receiveShadow = true;
  group.add(floorMesh);

  const sumpShape = new THREE.Shape();
  sumpShape.absarc(0, 0, 35 * scaleF, 0, Math.PI * 2, false);
  const sumpHole = new THREE.Path();
  sumpHole.absarc(0, 0, 25 * scaleF, 0, Math.PI * 2, true);
  sumpShape.holes.push(sumpHole);
  const sumpGeo = new THREE.ExtrudeGeometry(sumpShape, { depth: 0.35, bevelEnabled: false, curveSegments: 24 });
  sumpGeo.rotateX(Math.PI / 2);
  const sumpMesh = new THREE.Mesh(sumpGeo, m.sump);
  sumpMesh.position.set(0, -pitDepth + 0.35, 0);
  group.add(sumpMesh);
  for (let g = 0; g < 8; g++) {
    const ga = (g * Math.PI * 2) / 8;
    const grate = addBox(group, 30 * scaleF * Math.cos(ga), -pitDepth + 0.55, 30 * scaleF * Math.sin(ga), 8 * scaleF, 0.4, 3 * scaleF, m.steelDeck, false, false);
    grate.rotation.y = -ga;
  }

  const wallH = pitDepth + 6.0;
  const wallMesh = new THREE.Mesh(new THREE.CylinderGeometry(pitR, pitR, wallH, 48, 1, true), m.pitWall);
  wallMesh.position.set(0, -pitDepth + wallH * 0.5, 0);
  group.add(wallMesh);

  for (let i = 0; i < 24; i++) {
    const deg = i * 15;
    if (Math.abs(deg - 250) < 8) continue;
    const rad = deg2rad(deg);
    const pil = addBox(
      group,
      (pitR - 0.8 * scaleF) * Math.cos(rad),
      -pitDepth + wallH * 0.5,
      (pitR - 0.8 * scaleF) * Math.sin(rad),
      2.0 * scaleF,
      wallH,
      3.2 * scaleF,
      m.concreteDark,
      false,
      true,
    );
    pil.rotation.y = -rad;
  }

  // Pit maintenance ladder at 250 deg
  const ladA = deg2rad(250);
  const ladR = pitR - 1.6 * scaleF;
  const ladH = pitDepth + 5.0;
  for (const sz of [-2.0 * scaleF, 2.0 * scaleF]) {
    addBox(
      group,
      ladR * Math.cos(ladA) - sz * Math.sin(ladA),
      -pitDepth + ladH * 0.5,
      ladR * Math.sin(ladA) + sz * Math.cos(ladA),
      0.7 * scaleF,
      ladH,
      0.7 * scaleF,
      m.steelDark,
      false,
      false,
    );
  }
  for (let r = 0; r < 6; r++) {
    const ry = -pitDepth + 2.2 * scaleF + (r * (ladH - 3 * scaleF)) / 5;
    const rung = addBox(group, ladR * Math.cos(ladA), ry, ladR * Math.sin(ladA), 0.6 * scaleF, 0.6 * scaleF, 4.4 * scaleF, m.steelDark, false, false);
    rung.rotation.y = -ladA;
  }

  // 3. Raised concrete rail curb + circular metallic running rail
  const curbH = 2.4 * scaleF;
  const curbShape = new THREE.Shape();
  curbShape.absarc(0, 0, ringRailR + 4.0 * scaleF, 0, Math.PI * 2, false);
  const curbHole = new THREE.Path();
  curbHole.absarc(0, 0, ringRailR - 4.0 * scaleF, 0, Math.PI * 2, true);
  curbShape.holes.push(curbHole);
  const curbGeo = new THREE.ExtrudeGeometry(curbShape, { depth: curbH, bevelEnabled: false, curveSegments: 48 });
  curbGeo.rotateX(Math.PI / 2);
  const curbMesh = new THREE.Mesh(curbGeo, m.concrete);
  curbMesh.position.set(0, -pitDepth + curbH, 0);
  group.add(curbMesh);

  const railMinorR = 1.15 * scaleF;
  const pitRailCY = -pitDepth + curbH + railMinorR;
  const pitRailTop = pitRailCY + railMinorR;
  const pitRailGeo = new THREE.TorusGeometry(ringRailR, railMinorR, 8, 48);
  pitRailGeo.rotateX(Math.PI / 2);
  const pitRail = new THREE.Mesh(pitRailGeo, m.rail);
  pitRail.position.set(0, pitRailCY, 0);
  group.add(pitRail);

  // 4. Multi-tier Königsstuhl center pivot
  const kBase = new THREE.Mesh(new THREE.CylinderGeometry(21 * scaleF, 24 * scaleF, 3.6 * scaleF, 12), m.concreteDark);
  kBase.position.set(0, -pitDepth + 1.8 * scaleF, 0);
  group.add(kBase);
  const kCast = new THREE.Mesh(new THREE.CylinderGeometry(15 * scaleF, 18 * scaleF, 3.0 * scaleF, 16), m.steelDark);
  kCast.position.set(0, -pitDepth + 4.8 * scaleF, 0);
  group.add(kCast);
  const kBronze = new THREE.Mesh(new THREE.CylinderGeometry(11.5 * scaleF, 12.5 * scaleF, 1.6 * scaleF, 16), m.bronze);
  kBronze.position.set(0, -pitDepth + 7.1 * scaleF, 0);
  group.add(kBronze);
  const kSaddle = new THREE.Mesh(new THREE.CylinderGeometry(10.2 * scaleF, 10.2 * scaleF, 3.2 * scaleF, 16), m.steelGirder);
  kSaddle.position.set(0, -pitDepth + 9.2 * scaleF, 0);
  group.add(kSaddle);

  // 5. Exit track stubs around circumference (sleepers + flush 9.3 mm top rails + locking chairs)
  const localConns = entry.localConnectors ?? [];
  const slotAngles = localConns.length > 0 ? localConns.map((c) => c.angle) : Array.from({ length: 30 }, (_, i) => i * 12);
  const stubLen = outerR - pitR;
  const stubCenterR = 0.5 * (pitR + outerR);
  const rTieStart = pitR + 8.2 * scaleF;
  const rTieEnd = outerR - 3.8 * scaleF;
  const tieWStd = 22 * (gauge / 16.5);

  for (const deg of slotAngles) {
    const rad = deg2rad(deg);
    const ca = Math.cos(rad);
    const sa = Math.sin(rad);

    for (let ti = 0; ti < 5; ti++) {
      const rT = rTieStart + ((rTieEnd - rTieStart) * ti) / 4;
      const tie = addBox(group, rT * ca, tieY, rT * sa, 2.6 * scaleF, 2.2, tieWStd, m.wood, false, true);
      tie.rotation.y = -rad;
    }

    for (const side of [-1, 1]) {
      const oz = side * hg;
      const sRail = addBox(group, stubCenterR * ca - oz * sa, railY, stubCenterR * sa + oz * ca, stubLen, 2.2, 1.2, m.rail, true, true);
      sRail.rotation.y = -rad;

      const chairR = pitR + 2.6 * scaleF;
      const chair = addBox(group, chairR * ca - oz * sa, 6.9, chairR * sa + oz * ca, 3.4 * scaleF, 0.9, 2.8, m.steelDark, false, false);
      chair.rotation.y = -rad;
    }

    const lockR = pitR + 1.8 * scaleF;
    const lock = addBox(group, lockR * ca, 6.9, lockR * sa, 2.4 * scaleF, 0.8, gauge * 0.52, m.bronze, false, false);
    lock.rotation.y = -rad;
  }

  // 6. DB 23 m Fish-Belly Tapered Plate-Girder Bridge
  const halfB = bridgeL * 0.5;
  const girderEnd = halfB - 1.2 * scaleF;
  const girderShape = new THREE.Shape();
  girderShape.moveTo(-girderEnd, 8.6);
  girderShape.lineTo(girderEnd, 8.6);
  girderShape.lineTo(girderEnd, -1.8 * scaleF);
  girderShape.lineTo(ringRailR, -3.6 * scaleF);
  girderShape.lineTo(44 * scaleF, -8.6 * scaleF);
  girderShape.lineTo(-44 * scaleF, -8.6 * scaleF);
  girderShape.lineTo(-ringRailR, -3.6 * scaleF);
  girderShape.lineTo(-girderEnd, -1.8 * scaleF);
  girderShape.closePath();

  const girderGeo = new THREE.ExtrudeGeometry(girderShape, { depth: 2.0, bevelEnabled: false });
  girderGeo.translate(0, 0, -1.0);
  for (const sz of [-1, 1]) {
    const gz = sz * girderZ;
    const gMesh = new THREE.Mesh(girderGeo, m.steelGirder);
    gMesh.position.set(0, 0, gz);
    gMesh.castShadow = true;
    group.add(gMesh);

    addBox(group, 0, 8.9, gz, girderEnd * 2, 1.0, 3.4, m.steelDark);
    const segs: [[number, number], [number, number]][] = [
      [[-girderEnd, -1.8 * scaleF], [-ringRailR, -3.6 * scaleF]],
      [[-ringRailR, -3.6 * scaleF], [-44 * scaleF, -8.6 * scaleF]],
      [[-44 * scaleF, -8.6 * scaleF], [44 * scaleF, -8.6 * scaleF]],
      [[44 * scaleF, -8.6 * scaleF], [ringRailR, -3.6 * scaleF]],
      [[ringRailR, -3.6 * scaleF], [girderEnd, -1.8 * scaleF]],
    ];
    for (const [pa, pb] of segs) {
      addStrut(group, new THREE.Vector3(pa[0], pa[1], gz), new THREE.Vector3(pb[0], pb[1], gz), 1.4, 3.4, m.steelDark);
    }

    for (const vx of [-102, -74, -44, -22, 0, 22, 44, 74, 102]) {
      const svx = vx * scaleF;
      const ax = Math.abs(svx);
      const yb =
        ax <= 44 * scaleF
          ? -8.6 * scaleF
          : (-8.6 + 5.0 * Math.min(1, Math.max(0, (ax - 44 * scaleF) / Math.max(1, ringRailR - 44 * scaleF)))) * scaleF;
      addBox(group, svx, 0.5 * (yb + 8.6), gz, 1.5, 8.6 - yb, 3.2, m.steelDark, false, false);
    }
  }

  for (let i = 0; i <= 12; i++) {
    const cx = -girderEnd + (girderEnd * 2 * i) / 12;
    addBox(group, cx, 3.8, 0, 2.0, 2.4, girderZ * 2, m.steelDark, false, false);
  }
  for (const sx of [-1, 1]) {
    addBox(group, sx * (halfB - 1.2), 5.6, 0, 2.4, 2.2, (girderZ + 1.5) * 2, m.steelDark);
    addBox(group, sx * (halfB - 0.3), 6.2, 0, 1.6, 1.1, gauge * 0.5, m.bronze, false, false);
  }

  // 7. 4 Flanged bogie wheels sitting tangentially on circular pit rail
  const wheelR = 3.4 * scaleF;
  const flangeR = 4.05 * scaleF;
  const wheelY = pitRailTop + wheelR;
  const xW = Math.sqrt(Math.max(100, ringRailR * ringRailR - girderZ * girderZ));
  const treadGeo = new THREE.CylinderGeometry(wheelR, wheelR, 2.2, 16);
  treadGeo.rotateZ(Math.PI / 2);
  const flangeGeo = new THREE.CylinderGeometry(flangeR, flangeR, 0.5, 16);
  flangeGeo.rotateZ(Math.PI / 2);

  for (const sx of [-1, 1]) {
    for (const sz of [-1, 1]) {
      const wx = sx * xW;
      const wz = sz * girderZ;
      const axleAng = Math.atan2(wz, wx);
      const rLen = Math.hypot(wx, wz);
      const ux = wx / rLen;
      const uz = wz / rLen;

      const tread = new THREE.Mesh(treadGeo, m.wheelSteel);
      tread.position.set(wx, wheelY, wz);
      tread.rotation.y = -axleAng;
      group.add(tread);

      const flange = new THREE.Mesh(flangeGeo, m.steelDark);
      flange.position.set(wx - ux * 1.2, wheelY, wz - uz * 1.2);
      flange.rotation.y = -axleAng;
      group.add(flange);

      const yoke = addBox(group, wx, wheelY + 2.8 * scaleF, wz, 4.2 * scaleF, 5.2 * scaleF, 6.0 * scaleF, m.steelDark, false, false);
      yoke.rotation.y = -axleAng;
    }
  }

  // 8. Center deck plate, symmetrically centered ties & flush 9.3 mm top rails
  addBox(group, 0, 5.65, 0, bridgeL - 1, 0.7, girderZ * 2 - 1.2, m.steelDeck);
  const tieWBridge = 21 * (gauge / 16.5);
  const tieSpan = bridgeL - 10;
  const nTies = Math.max(4, Math.round(tieSpan / 8.0));
  for (let i = 0; i <= nTies; i++) {
    const tx = -tieSpan * 0.5 + (tieSpan * i) / nTies;
    addBox(group, tx, tieY, 0, 2.6, 2.2, tieWBridge, m.wood, false, true);
  }
  addBox(group, 0, railY, -hg, bridgeL, 2.2, 1.2, m.rail);
  addBox(group, 0, railY, hg, bridgeL, 2.2, 1.2, m.rail);

  // 9. Side steel-grated catwalks, kickplates, 2-bar handrails & Gleissperrsignale
  const walkW = 7.4;
  const walkZ = girderZ + 4.4;
  const railZ = girderZ + 7.7;
  const cabCX = -16 * scaleF;
  const cabW = 34 * scaleF;
  const cabD = 20.5 * scaleF;
  const cabCZ = railZ + cabD * 0.5;

  const walkL = bridgeL - 9 * scaleF;
  for (const sz of [-1, 1]) {
    addBox(group, 0, 5.65, sz * walkZ, walkL, 0.7, walkW, m.steelDeck);
    addBox(group, 0, 6.6, sz * (walkZ + walkW * 0.5 - 0.35), walkL, 1.5, 0.7, m.steelDark, false, false);
    for (let bx = -112; bx <= 112; bx += 28) {
      const x = bx * scaleF;
      addStrut(group, new THREE.Vector3(x, 5.4, sz * (railZ - 0.4)), new THREE.Vector3(x, 1.0, sz * girderZ), 1.2, 1.2, m.steelDark);
    }
  }
  addSafetyHandrailX(group, -walkL * 0.5 + 1.5, walkL * 0.5 - 1.5, 6.0, -railZ, null, scaleF, m.steelDark);
  addSafetyHandrailX(group, -walkL * 0.5 + 1.5, walkL * 0.5 - 1.5, 6.0, railZ, [cabCX - cabW * 0.5, cabCX + cabW * 0.5], scaleF, m.steelDark);
  addGleissperrsignal(group, halfB - 10 * scaleF, 6.0, -(girderZ + 5.2), scaleF, m);
  addGleissperrsignal(group, -(halfB - 10 * scaleF), 6.0, girderZ + 5.2, scaleF, m);

  // 10. Cantilevered steel-braced Wärterhaus operator cabin
  addOperatorCabin(group, cabCX, cabCZ, cabW, cabD, girderZ, 6.0, -8.2 * scaleF, scaleF, m);
}

// ============================================================================
//  2. TRANSFER TABLE (Schiebebühne — Märklin 72941)
// ============================================================================

export function buildTransferTable3D(
  group: THREE.Group,
  _piece: TrackPiece,
  entry: CatalogEntry,
  ballastColor: number,
) {
  const l = entry.length ?? 360;
  const gauge = entry.gauge ?? 16.5;
  const hg = gauge * 0.5;

  const rawZs = (entry.localConnectors ?? []).map((c) => c.y).sort((a, b) => a - b);
  const trackZs: number[] = [];
  for (const z of rawZs) {
    const prev = trackZs[trackZs.length - 1];
    if (prev === undefined || Math.abs(z - prev) > 1.0) trackZs.push(z);
  }
  if (trackZs.length === 0) trackZs.push(0, 77.4, 154.8, 232.2, 309.6);

  const minTZ = trackZs[0] ?? 0;
  const maxTZ = trackZs[trackZs.length - 1] ?? 309.6;
  const centerZ = 0.5 * (minTZ + maxTZ);
  const topZ = minTZ - 55.2;
  const botZ = maxTZ + 55.2;
  const plateD = botZ - topZ;

  const pitLeft = 36;
  const pitRight = l - 36;
  const pitW = pitRight - pitLeft;
  const pitTop = topZ + 6;
  const pitBot = botZ - 6;
  const pitD = pitBot - pitTop;
  const pitDepth = 12;
  const bridgeL = pitW - 2;
  const bridgeCX = l * 0.5;
  const bridgeZ = trackZs[Math.floor(trackZs.length / 2)] ?? centerZ;
  const railXs = [pitLeft + 44, l * 0.5, pitRight - 44];
  const girderZOff = Math.max(12.6, hg + 4.2);

  // Standard TS heights: ballast 6.0, ties 7.1, rails 8.2 (top 9.3)
  const ballastTop = 6.0;
  const tieY = 7.1;
  const railY = 8.2;
  const m = createSharedMats(ballastColor);

  // 1. Left & right gravel aprons + concrete coping + top/bottom headwalls
  const apronW = Math.max(8, pitLeft - 3.5);
  addBox(group, apronW * 0.5, ballastTop * 0.5, centerZ, apronW, ballastTop - 0.02, plateD, m.ballast, false, true);
  addBox(group, l - apronW * 0.5, ballastTop * 0.5, centerZ, apronW, ballastTop - 0.02, plateD, m.ballast, false, true);
  addBox(group, pitLeft - 1.75, 3.25, centerZ, 3.5, 6.5, pitD + 2, m.concrete, false, true);
  addBox(group, pitRight + 1.75, 3.25, centerZ, 3.5, 6.5, pitD + 2, m.concrete, false, true);
  addBox(group, bridgeCX, 3.35, topZ + 3, pitW + 7, 6.7, 6, m.concrete, false, true);
  addBox(group, bridgeCX, 3.35, botZ - 3, pitW + 7, 6.7, 6, m.concrete, false, true);

  // 2. Recessed concrete transfer pit: floor slab, 4 walls, pilasters, busbar, ladder, sump
  addBox(group, bridgeCX, -pitDepth - 1.25, centerZ, pitW + 6, 2.5, pitD + 6, m.pitFloor, false, true);
  const wallH = pitDepth + 6.0;
  const wallCY = -pitDepth + wallH * 0.5;
  addBox(group, pitLeft - 1.5, wallCY, centerZ, 3, wallH, pitD + 4, m.pitWall, false, true);
  addBox(group, pitRight + 1.5, wallCY, centerZ, 3, wallH, pitD + 4, m.pitWall, false, true);
  addBox(group, bridgeCX, wallCY, pitTop - 1.5, pitW + 4, wallH, 3, m.pitWall, false, true);
  addBox(group, bridgeCX, wallCY, pitBot + 1.5, pitW + 4, wallH, 3, m.pitWall, false, true);

  for (let i = 0; i + 1 < trackZs.length; i++) {
    const pz = 0.5 * ((trackZs[i] ?? 0) + (trackZs[i + 1] ?? 0));
    addBox(group, pitLeft + 1.0, wallCY - 0.2, pz, 2.2, wallH - 0.6, 5, m.concreteDark, false, true);
    addBox(group, pitRight - 1.0, wallCY - 0.2, pz, 2.2, wallH - 0.6, 5, m.concreteDark, false, true);
  }
  addBox(group, pitLeft + 1.2, -3.8, centerZ, 0.9, 1.2, pitD - 14, m.bronze, false, false);
  const ladX = bridgeCX + 34;
  for (const lx of [ladX - 2, ladX + 2]) addBox(group, lx, wallCY, pitTop + 1.1, 0.7, wallH - 1, 0.7, m.steelDark, false, false);
  for (let r = 0; r < 5; r++) {
    addBox(group, ladX, -pitDepth + 2.2 + (r * (wallH - 4)) / 4, pitTop + 1.1, 4.4, 0.6, 0.6, m.steelDark, false, false);
  }
  for (const dx of [-14, 14]) {
    addBox(group, bridgeCX + dx, -pitDepth + 0.18, centerZ, 3.8, 0.36, pitD - 12, m.sump, false, false);
    for (let k = 0; k < 8; k++) {
      const gz = pitTop + 26 + ((pitD - 52) * k) / 7;
      addBox(group, bridgeCX + dx, -pitDepth + 0.42, gz, 3.4, 0.25, 6, m.steelDeck, false, false);
    }
  }

  // 3. 3 Raised concrete traverse-rail beams, metallic traverse rails & 6 end wheel-stop bumpers
  const railTopY = -pitDepth + 4.2; // -7.8 mm
  const curbLen = pitD - 6;
  for (const rx of railXs) {
    addBox(group, rx, -pitDepth + 1.1, centerZ, 7.2, 2.2, curbLen, m.concrete, false, true);
    addBox(group, rx, -pitDepth + 3.2, centerZ, 1.8, 2.0, curbLen - 4, m.rail);
    for (const ez of [pitTop + 6.5, pitBot - 6.5]) {
      addBox(group, rx, -pitDepth + 4.6, ez, 6.4, 5.0, 5.2, m.steelDark);
      addBox(group, rx, -pitDepth + 6.4, ez, 6.8, 1.5, 5.6, m.hazardYellow);
    }
  }

  // 4. 10 Approach track stubs (ties strictly on aprons + flush 9.3 mm rails + locking shoes)
  const tieWStd = 22 * (gauge / 16.5);
  for (const tz of trackZs) {
    for (let ti = 0; ti < 4; ti++) {
      const u = ti / 3;
      addBox(group, 5.5 + (pitLeft - 11) * u, tieY, tz, 2.6, 2.2, tieWStd, m.wood, false, true);
      addBox(group, pitRight + 5.5 + (l - pitRight - 11) * u, tieY, tz, 2.6, 2.2, tieWStd, m.wood, false, true);
    }
    for (const side of [-1, 1]) {
      const rz = tz + side * hg;
      addBox(group, pitLeft * 0.5, railY, rz, pitLeft, 2.2, 1.2, m.rail);
      addBox(group, pitRight + (l - pitRight) * 0.5, railY, rz, l - pitRight, 2.2, 1.2, m.rail);
      addBox(group, pitLeft - 1.5, 6.9, rz, 3.0, 0.9, 2.8, m.steelDark, false, false);
      addBox(group, pitRight + 1.5, 6.9, rz, 3.0, 0.9, 2.8, m.steelDark, false, false);
    }
    addBox(group, pitLeft - 1.4, 6.9, tz, 2.4, 0.8, gauge * 0.5, m.bronze, false, false);
    addBox(group, pitRight + 1.4, 6.9, tz, 2.4, 0.8, gauge * 0.5, m.bronze, false, false);
  }

  // 5. Six flanged traverse wheels (axles along X!) + drive shaft + motor/gearbox
  const wheelR = 3.2;
  const flangeR = 3.85;
  const wheelY = railTopY + wheelR; // -4.6 mm -> rides cleanly on -7.8 mm traverse rails
  const treadGeo = new THREE.CylinderGeometry(wheelR, wheelR, 2.0, 16);
  treadGeo.rotateZ(Math.PI / 2);
  const flangeGeo = new THREE.CylinderGeometry(flangeR, flangeR, 0.5, 16);
  flangeGeo.rotateZ(Math.PI / 2);

  for (const rx of railXs) {
    for (const sz of [-1, 1]) {
      const wz = bridgeZ + sz * girderZOff;
      const tread = new THREE.Mesh(treadGeo, m.wheelSteel);
      tread.position.set(rx, wheelY, wz);
      group.add(tread);
      for (const fx of [-1.15, 1.15]) {
        const fl = new THREE.Mesh(flangeGeo, m.steelDark);
        fl.position.set(rx + fx, wheelY, wz);
        group.add(fl);
      }
      addBox(group, rx, wheelY + 2.4, wz, 5.4, 4.4, 4.4, m.steelDark, false, false);
    }
  }
  const shaftZ = bridgeZ + girderZOff;
  const shaftLen = (railXs[2] ?? pitRight - 44) - (railXs[0] ?? pitLeft + 44) + 10;
  const shaft = new THREE.Mesh(new THREE.CylinderGeometry(0.95, 0.95, shaftLen, 10), m.wheelSteel);
  shaft.rotation.z = Math.PI / 2;
  shaft.position.set(bridgeCX, wheelY, shaftZ);
  group.add(shaft);
  const motorX = bridgeCX - 28;
  addBox(group, motorX + 7, wheelY + 0.5, shaftZ + 2.2, 6.5, 5.2, 6.2, m.steelDark);
  const motor = new THREE.Mesh(new THREE.CylinderGeometry(3.4, 3.4, 10.5, 12), m.steelGirder);
  motor.rotation.z = Math.PI / 2;
  motor.position.set(motorX - 2, wheelY + 0.6, shaftZ + 4.4);
  group.add(motor);

  // 6. Flanged plate girders, cross-beams, deck & symmetrically centered bridge ties/rails (zero overhang!)
  const girderL = bridgeL - 2;
  for (const sz of [-1, 1]) {
    const gz = bridgeZ + sz * girderZOff;
    addBox(group, bridgeCX, 3.2, gz, girderL, 10.8, 2.0, m.steelGirder);
    addBox(group, bridgeCX, 8.9, gz, girderL + 1, 1.1, 3.4, m.steelDark);
    addBox(group, bridgeCX, -2.3, gz, girderL + 1, 1.2, 3.6, m.steelDark);
    for (let st = 0; st <= 12; st++) {
      const sx = pitLeft + 4 + ((bridgeL - 8) * st) / 12;
      addBox(group, sx, 3.2, gz, 1.5, 10.8, 3.2, m.steelDark, false, false);
    }
  }
  for (let cb = 0; cb <= 10; cb++) {
    const bx = pitLeft + 3 + ((bridgeL - 6) * cb) / 10;
    addBox(group, bx, 3.6, bridgeZ, 2.2, 2.4, girderZOff * 2, m.steelDark, false, false);
  }
  for (const ex of [pitLeft + 2.2, pitRight - 2.2]) {
    addBox(group, ex, 5.6, bridgeZ, 2.4, 2.4, (girderZOff + 8) * 2, m.steelDark);
    addBox(group, ex, 6.5, bridgeZ, 1.8, 1.0, gauge * 0.48, m.bronze, false, false);
  }
  addBox(group, bridgeCX, 5.65, bridgeZ, bridgeL - 1, 0.7, girderZOff * 2 - 1.2, m.steelDeck);

  const tieWBridge = 21 * (gauge / 16.5);
  const tieSpan = bridgeL - 10;
  const nTies = Math.max(4, Math.round(tieSpan / 8.0));
  for (let i = 0; i <= nTies; i++) {
    const tx = bridgeCX - tieSpan * 0.5 + (tieSpan * i) / nTies;
    addBox(group, tx, tieY, bridgeZ, 2.6, 2.2, tieWBridge, m.wood, false, true);
  }
  addBox(group, bridgeCX, railY, bridgeZ - hg, bridgeL, 2.2, 1.2, m.rail);
  addBox(group, bridgeCX, railY, bridgeZ + hg, bridgeL, 2.2, 1.2, m.rail);

  // 7. Side walkways, 2-bar handrails, hazard ends & Gleissperrsignale
  const walkW = 7.4;
  const walkZOff = girderZOff + 4.4;
  const railZOff = girderZOff + 7.7;
  const cabCX = bridgeCX - 34;
  const cabW = 36;
  const cabD = 21;
  const cabCZ = bridgeZ + railZOff + cabD * 0.5;

  for (const sz of [-1, 1]) {
    const wz = bridgeZ + sz * walkZOff;
    addBox(group, bridgeCX, 5.65, wz, bridgeL - 4, 0.7, walkW, m.steelDeck);
    addBox(group, bridgeCX, 6.6, bridgeZ + sz * (walkZOff + walkW * 0.5 - 0.35), bridgeL - 4, 1.5, 0.7, m.steelDark, false, false);
    for (let bx = 0; bx <= 8; bx++) {
      const x = pitLeft + 12 + ((bridgeL - 24) * bx) / 8;
      addStrut(group, new THREE.Vector3(x, 5.4, bridgeZ + sz * (railZOff - 0.4)), new THREE.Vector3(x, 1.0, bridgeZ + sz * girderZOff), 1.2, 1.2, m.steelDark);
    }
    for (const ex of [pitLeft + 2, pitRight - 2]) {
      addBox(group, ex, 6.0, wz, 2.2, 2.0, walkW, m.hazardYellow);
    }
  }
  addSafetyHandrailX(group, pitLeft + 5, pitRight - 5, 6.0, bridgeZ - railZOff, null, 1, m.steelDark);
  addSafetyHandrailX(group, pitLeft + 5, pitRight - 5, 6.0, bridgeZ + railZOff, [cabCX - cabW * 0.5, cabCX + cabW * 0.5], 1, m.steelDark);
  addGleissperrsignal(group, pitRight - 7.5, 6.0, bridgeZ - (girderZOff + 5.2), 1, m);
  addGleissperrsignal(group, pitLeft + 7.5, 6.0, bridgeZ + (girderZOff + 5.2), 1, m);

  // 8. Cantilevered steel-braced Wärterhaus operator cabin
  addOperatorCabin(group, cabCX, cabCZ, cabW, cabD, bridgeZ + girderZOff, 6.0, -2.2, 1, m);

  // 9. Signature overhead power-collector portal arch at bridge center (top crossbeam at y = 89.5,
  // clearing TS catenary contactH = 72 and messengerH = 85!)
  const archH = 84.5;
  const archTopY = 6.0 + archH; // 90.5 mm
  for (const sz of [-1, 1]) {
    const pz = bridgeZ + sz * railZOff;
    addBox(group, bridgeCX, 6.0 + archH * 0.5, pz, 3.6, archH, 3.2, m.steelGirder);
    addBox(group, bridgeCX, 7.6, pz, 5.8, 3.4, 5.0, m.steelDark);
    addStrut(
      group,
      new THREE.Vector3(bridgeCX, archTopY - 12, pz),
      new THREE.Vector3(bridgeCX, archTopY - 2, bridgeZ + sz * (railZOff - 10)),
      1.6,
      1.6,
      m.steelDark,
    );
  }
  const spanTotal = railZOff * 2 + 4;
  addBox(group, bridgeCX, archTopY - 1.2, bridgeZ, 3.8, 2.8, spanTotal, m.steelGirder);
  addBox(group, bridgeCX, archTopY - 4.2, bridgeZ, 2.4, 1.4, spanTotal - 4, m.steelDark);
  for (const sz of [-1, 1]) {
    const ins = new THREE.Mesh(new THREE.CylinderGeometry(1.4, 1.8, 2.8, 8), m.insulator);
    ins.position.set(bridgeCX, archTopY + 1.5, bridgeZ + sz * 8);
    group.add(ins);
  }
  addBox(group, bridgeCX, archTopY + 3.4, bridgeZ, 2.2, 1.0, 20, m.steelDark);
  addBox(group, bridgeCX, archTopY + 5.6, bridgeZ, 1.6, 0.9, 14, m.bronze);
  addStrut(group, new THREE.Vector3(bridgeCX, archTopY + 3.4, bridgeZ - 7), new THREE.Vector3(bridgeCX, archTopY + 5.6, bridgeZ - 4), 0.7, 0.7, m.wheelSteel);
  addStrut(group, new THREE.Vector3(bridgeCX, archTopY + 3.4, bridgeZ + 7), new THREE.Vector3(bridgeCX, archTopY + 5.6, bridgeZ + 4), 0.7, 0.7, m.wheelSteel);
}
