import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedCylinderGeometry,
  getSharedDodecahedronGeometry,
  getSharedMaterial,
} from "./primitive-cache";

/** 2. STELLWERK (120 x 75 x 110 mm) */
export function buildSignalBox3D(group: THREE.Group): void {
  // Geteilte Materialien
  const grassMat = getSharedMaterial(0x4d7c0f, 0.88);
  const pathMat = getSharedMaterial(0xd4b996, 0.8);
  const bushMat = getSharedMaterial(0x166534, 0.9);
  const brickMat = getSharedMaterial(0x854d0e, 0.85);
  const topWallMat = getSharedMaterial(0xd97706, 0.6);
  const glassMat = getSharedMaterial(0x0284c7, 0.15, 0.85);
  const roofMat = getSharedMaterial(0x1e293b, 0.7);
  const stairMat = getSharedMaterial(0x64748b, 0.5, 0.6);
  const frameMat = getSharedMaterial(0x1f2937, 0.6, 0.3);
  const doorMat = getSharedMaterial(0x78350f, 0.8);

  // Landschaftsplatte um das Stellwerk
  const grass = new THREE.Mesh(getSharedBoxGeometry(180, 3, 130), grassMat);
  grass.position.set(10, 1.5, 0);
  group.add(grass);

  // Weg zur Treppe
  const path = new THREE.Mesh(getSharedBoxGeometry(35, 1, 80), pathMat);
  path.position.set(65, 3.2, 25);
  group.add(path);

  // Büsche & Grasbüschel
  const bush1 = new THREE.Mesh(getSharedDodecahedronGeometry(9, 1), bushMat);
  bush1.position.set(-45, 9, 35);
  group.add(bush1);

  // Untergeschoss (Klinker)
  const groundFloor = new THREE.Mesh(getSharedBoxGeometry(100, 45, 65), brickMat);
  groundFloor.position.set(0, 24, 0);
  group.add(groundFloor);

  // Obergeschoss (Ausstellend mit großen Fenstern)
  const topFloor = new THREE.Mesh(getSharedBoxGeometry(112, 35, 75), topWallMat);
  topFloor.position.set(0, 64, 0);
  group.add(topFloor);

  // Panorama-Fensterband
  const glassGeo = getSharedBoxGeometry(106, 20, 1);
  const frontGlass = new THREE.Mesh(glassGeo, glassMat);
  frontGlass.position.set(0, 67.5, 38);
  group.add(frontGlass);

  const backGlass = new THREE.Mesh(glassGeo, glassMat);
  backGlass.position.set(0, 67.5, -38);
  group.add(backGlass);

  // Flachdach mit Dachrand
  const roof = new THREE.Mesh(getSharedBoxGeometry(122, 5, 85), roofMat);
  roof.position.set(0, 84, 0);
  group.add(roof);

  // --- Außentreppe an der Ostwand (x = 50), steigt Richtung -Z ---
  const steps = 14;
  const stairX = 55;
  const z0 = 24, z1 = -12;
  const y0 = 3, y1 = 46.5;
  const run = z0 - z1;
  const rise = y1 - y0;
  const slope = Math.atan2(rise, run);
  const len = Math.hypot(run, rise);
  const midZ = (z0 + z1) / 2;
  const midY = (y0 + y1) / 2;

  // Wangen
  const stringerGeo = getSharedBoxGeometry(1.5, 3, len);
  for (const dx of [-3.5, 3.5]) {
    const stringer = new THREE.Mesh(stringerGeo, stairMat);
    stringer.rotation.x = slope;
    stringer.position.set(stairX + dx, midY, midZ);
    group.add(stringer);
  }

  // Stufen
  const treadGeo = getSharedBoxGeometry(8, 1, 3.2);
  for (let i = 0; i < steps; i++) {
    const tread = new THREE.Mesh(treadGeo, stairMat);
    tread.position.set(
      stairX,
      y0 + (rise * (i + 1)) / steps - 0.5,
      z0 - (run * (i + 0.5)) / steps,
    );
    group.add(tread);
  }

  // Geländer an der Außenseite der Treppe
  const railX = stairX + 3.8;
  const postGeo = getSharedCylinderGeometry(0.4, 0.4, 9, 6);
  for (const i of [1, 4, 7, 10, 13]) {
    const post = new THREE.Mesh(postGeo, stairMat);
    post.position.set(
      railX,
      y0 + (rise * (i + 1)) / steps + 4,
      z0 - (run * (i + 0.5)) / steps,
    );
    group.add(post);
  }
  const handrail = new THREE.Mesh(getSharedBoxGeometry(0.8, 0.8, len), stairMat);
  handrail.rotation.x = slope;
  handrail.position.set(railX, midY + 10.5, midZ);
  group.add(handrail);

  // Podest vor der Obergeschoss-Tür mit Stützen
  const landing = new THREE.Mesh(getSharedBoxGeometry(12, 1.5, 20), stairMat);
  landing.position.set(56, 45.75, -22);
  group.add(landing);

  const supportGeo = getSharedCylinderGeometry(1.2, 1.2, 42, 8);
  for (const pz of [-31, -13]) {
    const support = new THREE.Mesh(supportGeo, stairMat);
    support.position.set(61, 24, pz);
    group.add(support);
  }

  // Podest-Geländer (Ost- und Nordseite)
  const railEGeo = getSharedBoxGeometry(0.8, 0.8, 20);
  const railNGeo = getSharedBoxGeometry(12, 0.8, 0.8);
  for (const ry of [49, 53]) {
    const railE = new THREE.Mesh(railEGeo, stairMat);
    railE.position.set(61.6, ry, -22);
    group.add(railE);
    const railN = new THREE.Mesh(railNGeo, stairMat);
    railN.position.set(56, ry, -31.6);
    group.add(railN);
  }
  const landingPostGeo = getSharedCylinderGeometry(0.4, 0.4, 7, 6);
  for (const [px, pz] of [[61.6, -31.6], [61.6, -22], [61.6, -12.5]] as const) {
    const post = new THREE.Mesh(landingPostGeo, stairMat);
    post.position.set(px, 50, pz);
    group.add(post);
  }

  // --- Tür im Obergeschoss (Ostwand der Bedienetage, x = 56) ---
  const upperFrame = new THREE.Mesh(getSharedBoxGeometry(1, 28, 15), frameMat);
  upperFrame.position.set(56.5, 60.5, -22);
  group.add(upperFrame);

  const upperDoor = new THREE.Mesh(getSharedBoxGeometry(1, 26, 12), doorMat);
  upperDoor.position.set(57, 59.5, -22);
  group.add(upperDoor);

  const doorGlass = new THREE.Mesh(getSharedBoxGeometry(1.2, 8, 6), glassMat);
  doorGlass.position.set(57.4, 65, -22);
  group.add(doorGlass);

  const upperCanopy = new THREE.Mesh(getSharedBoxGeometry(8, 1.5, 18), roofMat);
  upperCanopy.position.set(60, 77, -22);
  group.add(upperCanopy);

  // --- Eingang im Erdgeschoss (Südwand +Z, z = 32.5) ---
  const entryPatch = new THREE.Mesh(getSharedBoxGeometry(22, 1, 27), pathMat);
  entryPatch.position.set(40, 3.1, 46.5);
  group.add(entryPatch);

  const groundFrame = new THREE.Mesh(getSharedBoxGeometry(13, 33, 1), frameMat);
  groundFrame.position.set(40, 19.5, 33);
  group.add(groundFrame);

  const groundDoor = new THREE.Mesh(getSharedBoxGeometry(10, 30, 1), doorMat);
  groundDoor.position.set(40, 18, 33.5);
  group.add(groundDoor);

  const groundStep = new THREE.Mesh(getSharedBoxGeometry(14, 1.5, 4), stairMat);
  groundStep.position.set(40, 3.75, 35);
  group.add(groundStep);

  const groundCanopy = new THREE.Mesh(getSharedBoxGeometry(18, 1.5, 8), roofMat);
  groundCanopy.position.set(40, 37.5, 36.5);
  group.add(groundCanopy);
}

/** Vereinfachte Low-Poly Variante des Stellwerks (LOD bei Distanz >= 3200 mm) */
export function buildSignalBoxLowPoly3D(group: THREE.Group): void {
  const grassMat = getSharedMaterial(0x4d7c0f, 0.88);
  const brickMat = getSharedMaterial(0x854d0e, 0.85);
  const topWallMat = getSharedMaterial(0xd97706, 0.6);
  const glassMat = getSharedMaterial(0x0284c7, 0.15, 0.85);
  const roofMat = getSharedMaterial(0x1e293b, 0.7);
  const stairMat = getSharedMaterial(0x64748b, 0.5, 0.6);

  // Landschaftsplatte
  const grass = new THREE.Mesh(getSharedBoxGeometry(180, 3, 130), grassMat);
  grass.position.set(10, 1.5, 0);
  group.add(grass);

  // Untergeschoss
  const groundFloor = new THREE.Mesh(getSharedBoxGeometry(100, 45, 65), brickMat);
  groundFloor.position.set(0, 24, 0);
  group.add(groundFloor);

  // Obergeschoss
  const topFloor = new THREE.Mesh(getSharedBoxGeometry(112, 35, 75), topWallMat);
  topFloor.position.set(0, 64, 0);
  group.add(topFloor);

  // Panorama-Fensterband
  const glassGeo = getSharedBoxGeometry(106, 20, 1);
  const frontGlass = new THREE.Mesh(glassGeo, glassMat);
  frontGlass.position.set(0, 67.5, 38);
  group.add(frontGlass);

  const backGlass = new THREE.Mesh(glassGeo, glassMat);
  backGlass.position.set(0, 67.5, -38);
  group.add(backGlass);

  // Flachdach
  const roof = new THREE.Mesh(getSharedBoxGeometry(122, 5, 85), roofMat);
  roof.position.set(0, 84, 0);
  group.add(roof);

  // Treppenblock vereinfacht
  const stairBlock = new THREE.Mesh(getSharedBoxGeometry(8, 43.5, 36), stairMat);
  stairBlock.position.set(55, 24.75, 6);
  group.add(stairBlock);
}
