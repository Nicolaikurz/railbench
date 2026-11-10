import * as THREE from "three";

/** Fallback generisches Gebäude */
export function buildGenericBuilding3D(group: THREE.Group): void {
  const mat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.7 });
  const box = new THREE.Mesh(new THREE.BoxGeometry(60, 50, 60), mat);
  box.position.set(0, 25, 0);
  group.add(box);
}
