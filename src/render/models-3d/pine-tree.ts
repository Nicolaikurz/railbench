import * as THREE from "three";

const SHARED_PINE_TRUNK_MAT = new THREE.MeshStandardMaterial({ color: 0x3f2e21, roughness: 0.9 });
SHARED_PINE_TRUNK_MAT.userData.shared = true;

const SHARED_PINE_TRUNK_GEO = new THREE.CylinderGeometry(3.5, 6, 35, 8);
SHARED_PINE_TRUNK_GEO.userData.shared = true;

const SHARED_PINE_CONE_MATS = [
  new THREE.MeshStandardMaterial({ color: 0x14532d, roughness: 0.95 }),
  new THREE.MeshStandardMaterial({ color: 0x166534, roughness: 0.95 }),
  new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.95 }),
  new THREE.MeshStandardMaterial({ color: 0x16a34a, roughness: 0.95 }),
];
SHARED_PINE_CONE_MATS.forEach((m) => (m.userData.shared = true));

const SHARED_PINE_CONE_GEOS = [
  new THREE.ConeGeometry(27, 40, 12),
  new THREE.ConeGeometry(23, 36, 12),
  new THREE.ConeGeometry(18, 32, 12),
  new THREE.ConeGeometry(12, 28, 12),
];
SHARED_PINE_CONE_GEOS.forEach((g) => (g.userData.shared = true));

const Y_OFFSETS = [42, 65, 88, 110];

/** 8. FICHTE / TANNE (55 x 55 x 135 mm) */
export function buildPineTree3D(group: THREE.Group): void {
  // Stamm
  const trunk = new THREE.Mesh(SHARED_PINE_TRUNK_GEO, SHARED_PINE_TRUNK_MAT);
  trunk.position.set(0, 17.5, 0);
  group.add(trunk);

  // 4 gestufte Nadelkegel
  for (let i = 0; i < 4; i++) {
    const cone = new THREE.Mesh(SHARED_PINE_CONE_GEOS[i]!, SHARED_PINE_CONE_MATS[i]!);
    cone.position.set(0, Y_OFFSETS[i]!, 0);
    group.add(cone);
  }
}

const SHARED_PINE_LOWPOLY_CONE_GEO = new THREE.ConeGeometry(26, 95, 8);
SHARED_PINE_LOWPOLY_CONE_GEO.userData.shared = true;

/** Low-Poly LOD Variante der Tanne (1 vereinfachter Kegel statt 4) */
export function buildPineTreeLowPoly3D(group: THREE.Group): void {
  const trunk = new THREE.Mesh(SHARED_PINE_TRUNK_GEO, SHARED_PINE_TRUNK_MAT);
  trunk.position.set(0, 17.5, 0);
  group.add(trunk);

  const cone = new THREE.Mesh(SHARED_PINE_LOWPOLY_CONE_GEO, SHARED_PINE_CONE_MATS[1]!);
  cone.position.set(0, 75, 0);
  group.add(cone);
}
