import * as THREE from "three";
import {
  getSharedCylinderGeometry,
  getSharedDodecahedronGeometry,
  getSharedMaterial,
} from "./primitive-cache";

/**
 * 9. LAUBBAUM — Organischer Laubbaum (80 × 80 × 125 mm)
 * Wurzelanlauf, gegabelte Äste und 6 mehrfarbig abgestufte Laubcluster.
 */
export function buildDeciduousTree3D(group: THREE.Group): void {
  const barkMat = getSharedMaterial(0x4b3621, 0.92);
  const leafDarkMat = getSharedMaterial(0x14532d, 0.92);
  const leafMidMat = getSharedMaterial(0x15803d, 0.9);
  const leafLightMat = getSharedMaterial(0x22c55e, 0.88);

  // Wurzelanlauf + Stamm
  const rootFlare = new THREE.Mesh(getSharedCylinderGeometry(7.5, 12, 12, 8), barkMat);
  rootFlare.position.set(0, 6, 0);
  group.add(rootFlare);

  const trunk = new THREE.Mesh(getSharedCylinderGeometry(4.5, 7.8, 48, 8), barkMat);
  trunk.position.set(0, 32, 0);
  group.add(trunk);

  // 3 gegabelte Hauptäste
  const branchGeo = getSharedCylinderGeometry(2.0, 3.6, 26, 7);
  const branches: Array<{ x: number; y: number; z: number; rx: number; rz: number }> = [
    { x: 9, y: 58, z: 6, rx: 0.35, rz: -0.5 },
    { x: -10, y: 60, z: -5, rx: -0.3, rz: 0.52 },
    { x: 2, y: 62, z: -10, rx: -0.5, rz: -0.15 },
  ];
  for (const b of branches) {
    const m = new THREE.Mesh(branchGeo, barkMat);
    m.position.set(b.x, b.y, b.z);
    m.rotation.set(b.rx, 0, b.rz);
    group.add(m);
  }

  // 6 mehrfarbige, organisch versetzte Laubcluster
  const clusters: Array<{ r: number; x: number; y: number; z: number; mat: THREE.Material }> = [
    { r: 30, x: 0, y: 80, z: 0, mat: leafMidMat },
    { r: 24, x: 18, y: 74, z: 12, mat: leafLightMat },
    { r: 23, x: -17, y: 77, z: -11, mat: leafDarkMat },
    { r: 21, x: -14, y: 88, z: 13, mat: leafLightMat },
    { r: 20, x: 15, y: 86, z: -14, mat: leafMidMat },
    { r: 19, x: 0, y: 100, z: 3, mat: leafLightMat },
  ];
  for (const c of clusters) {
    const mesh = new THREE.Mesh(getSharedDodecahedronGeometry(c.r, 1), c.mat);
    mesh.position.set(c.x, c.y, c.z);
    group.add(mesh);
  }
}

/** Low-Poly LOD Variante des Laubbaums (1 Kugel statt 6) */
export function buildDeciduousTreeLowPoly3D(group: THREE.Group): void {
  const barkMat = getSharedMaterial(0x4b3621, 0.92);
  const leafMidMat = getSharedMaterial(0x15803d, 0.9);

  const trunk = new THREE.Mesh(getSharedCylinderGeometry(5, 9, 50, 8), barkMat);
  trunk.position.set(0, 25, 0);
  group.add(trunk);

  const mainCrown = new THREE.Mesh(getSharedDodecahedronGeometry(36, 1), leafMidMat);
  mainCrown.position.set(0, 78, 0);
  group.add(mainCrown);
}
