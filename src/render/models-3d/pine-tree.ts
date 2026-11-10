import * as THREE from "three";

/** 8. FICHTE / TANNE (55 x 55 x 135 mm) */
export function buildPineTree3D(group: THREE.Group): void {
  // Stamm
  const trunkMat = new THREE.MeshStandardMaterial({ color: 0x3f2e21, roughness: 0.9 });
  const trunk = new THREE.Mesh(new THREE.CylinderGeometry(3.5, 6, 35, 8), trunkMat);
  trunk.position.set(0, 17.5, 0);
  group.add(trunk);

  // 4 gestufte Nadelkegel
  const needleColors = [0x14532d, 0x166534, 0x15803d, 0x16a34a];
  const radii = [27, 23, 18, 12];
  const heights = [40, 36, 32, 28];
  const yOffsets = [42, 65, 88, 110];

  for (let i = 0; i < 4; i++) {
    const mat = new THREE.MeshStandardMaterial({ color: needleColors[i], roughness: 0.95 });
    const cone = new THREE.Mesh(new THREE.ConeGeometry(radii[i], heights[i], 12), mat);
    cone.position.set(0, yOffsets[i]!, 0);
    group.add(cone);
  }
}
