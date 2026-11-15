import * as THREE from "three";

const SHARED_DEC_TRUNK_MAT = new THREE.MeshStandardMaterial({ color: 0x443525, roughness: 0.9 });
SHARED_DEC_TRUNK_MAT.userData.shared = true;

const SHARED_DEC_TRUNK_GEO = new THREE.CylinderGeometry(5, 9, 50, 8);
SHARED_DEC_TRUNK_GEO.userData.shared = true;

const SHARED_DEC_CROWN_MAT = new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.9 });
SHARED_DEC_CROWN_MAT.userData.shared = true;

const SHARED_DEC_CROWN_GEO = new THREE.DodecahedronGeometry(36, 1);
SHARED_DEC_CROWN_GEO.userData.shared = true;

const SHARED_DEC_SUBCROWN1_GEO = new THREE.DodecahedronGeometry(22, 1);
SHARED_DEC_SUBCROWN1_GEO.userData.shared = true;

const SHARED_DEC_SUBCROWN2_GEO = new THREE.DodecahedronGeometry(20, 1);
SHARED_DEC_SUBCROWN2_GEO.userData.shared = true;

/** 9. LAUBBAUM (80 x 80 x 120 mm) */
export function buildDeciduousTree3D(group: THREE.Group): void {
  const trunk = new THREE.Mesh(SHARED_DEC_TRUNK_GEO, SHARED_DEC_TRUNK_MAT);
  trunk.position.set(0, 25, 0);
  group.add(trunk);

  // Organische Laubkrone aus mehreren Kugeln
  const mainCrown = new THREE.Mesh(SHARED_DEC_CROWN_GEO, SHARED_DEC_CROWN_MAT);
  mainCrown.position.set(0, 75, 0);
  group.add(mainCrown);

  const subCrown1 = new THREE.Mesh(SHARED_DEC_SUBCROWN1_GEO, SHARED_DEC_CROWN_MAT);
  subCrown1.position.set(16, 70, 12);
  group.add(subCrown1);

  const subCrown2 = new THREE.Mesh(SHARED_DEC_SUBCROWN2_GEO, SHARED_DEC_CROWN_MAT);
  subCrown2.position.set(-14, 82, -10);
  group.add(subCrown2);
}

/** Low-Poly LOD Variante des Laubbaums (1 Kugel statt 3) */
export function buildDeciduousTreeLowPoly3D(group: THREE.Group): void {
  const trunk = new THREE.Mesh(SHARED_DEC_TRUNK_GEO, SHARED_DEC_TRUNK_MAT);
  trunk.position.set(0, 25, 0);
  group.add(trunk);

  const mainCrown = new THREE.Mesh(SHARED_DEC_CROWN_GEO, SHARED_DEC_CROWN_MAT);
  mainCrown.position.set(0, 75, 0);
  group.add(mainCrown);
}
