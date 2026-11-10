import * as THREE from "three";

/** 9. LAUBBAUM (80 x 80 x 120 mm) */
export function buildDeciduousTree3D(group: THREE.Group): void {
  const trunkMat = new THREE.MeshStandardMaterial({ color: 0x443525, roughness: 0.9 });
  const trunk = new THREE.Mesh(new THREE.CylinderGeometry(5, 9, 50, 8), trunkMat);
  trunk.position.set(0, 25, 0);
  group.add(trunk);

  // Organische Laubkrone aus mehreren Kugeln
  const crownMat = new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.9 });
  const mainCrown = new THREE.Mesh(new THREE.DodecahedronGeometry(36, 1), crownMat);
  mainCrown.position.set(0, 75, 0);
  group.add(mainCrown);

  const subCrown1 = new THREE.Mesh(new THREE.DodecahedronGeometry(22, 1), crownMat);
  subCrown1.position.set(16, 70, 12);
  group.add(subCrown1);

  const subCrown2 = new THREE.Mesh(new THREE.DodecahedronGeometry(20, 1), crownMat);
  subCrown2.position.set(-14, 82, -10);
  group.add(subCrown2);
}
