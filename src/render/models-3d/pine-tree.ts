import * as THREE from "three";
import { getSharedCylinderGeometry, getSharedMaterial } from "./primitive-cache";

const SHARED_PINE_CONE_GEOS = [
  new THREE.ConeGeometry(28, 34, 11),
  new THREE.ConeGeometry(25, 32, 11),
  new THREE.ConeGeometry(21, 30, 11),
  new THREE.ConeGeometry(17, 28, 10),
  new THREE.ConeGeometry(13, 24, 10),
  new THREE.ConeGeometry(8.5, 22, 9),
];
SHARED_PINE_CONE_GEOS.forEach((g) => (g.userData.shared = true));

const SHARED_PINE_LOWPOLY_CONE_GEO = new THREE.ConeGeometry(26, 98, 8);
SHARED_PINE_LOWPOLY_CONE_GEO.userData.shared = true;

const PINE_Y_OFFSETS = [36, 54, 72, 90, 107, 122];
const PINE_ROTATIONS = [0, 0.55, 1.1, 0.3, 0.9, 1.5];

/**
 * 8. FICHTE / TANNE — Gestufte Nadelbaum-Krone (55 × 55 × 135 mm)
 * Wurzelanlauf + 6 leicht verdrehte, farblich abgestufte Nadelkränze.
 */
export function buildPineTree3D(group: THREE.Group): void {
  const barkMat = getSharedMaterial(0x3f2e21, 0.92);
  const tierMats = [
    getSharedMaterial(0x14532d, 0.95),
    getSharedMaterial(0x166534, 0.94),
    getSharedMaterial(0x14532d, 0.94),
    getSharedMaterial(0x15803d, 0.93),
    getSharedMaterial(0x166534, 0.92),
    getSharedMaterial(0x16a34a, 0.9),
  ];

  // Wurzelanlauf & Stamm
  const rootFlare = new THREE.Mesh(getSharedCylinderGeometry(5.5, 8.5, 10, 8), barkMat);
  rootFlare.position.set(0, 5, 0);
  group.add(rootFlare);

  const trunk = new THREE.Mesh(getSharedCylinderGeometry(3.2, 5.8, 36, 8), barkMat);
  trunk.position.set(0, 22, 0);
  group.add(trunk);

  // 6 gestufte, leicht versetzte Nadelkegel
  for (let i = 0; i < 6; i++) {
    const cone = new THREE.Mesh(SHARED_PINE_CONE_GEOS[i]!, tierMats[i]!);
    cone.position.set(0, PINE_Y_OFFSETS[i]!, 0);
    cone.rotation.y = PINE_ROTATIONS[i]!;
    group.add(cone);
  }
}

/** Low-Poly LOD Variante der Tanne (1 vereinfachter Kegel statt 6) */
export function buildPineTreeLowPoly3D(group: THREE.Group): void {
  const barkMat = getSharedMaterial(0x3f2e21, 0.92);
  const foliageMat = getSharedMaterial(0x166534, 0.94);

  const trunk = new THREE.Mesh(getSharedCylinderGeometry(3.5, 6, 35, 8), barkMat);
  trunk.position.set(0, 17.5, 0);
  group.add(trunk);

  const cone = new THREE.Mesh(SHARED_PINE_LOWPOLY_CONE_GEO, foliageMat);
  cone.position.set(0, 76, 0);
  group.add(cone);
}
