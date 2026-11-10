import * as THREE from "three";

/** 5. WASSERTURM (85 x 85 x 170 mm) */
export function buildWaterTower3D(group: THREE.Group): void {
  // Steinsockel
  const stoneMat = new THREE.MeshStandardMaterial({ color: 0x78716c, roughness: 0.9 });
  const base = new THREE.Mesh(new THREE.CylinderGeometry(30, 36, 90, 16), stoneMat);
  base.position.set(0, 45, 0);
  group.add(base);

  // Wasserbehälter (Stahlkessel)
  const tankMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.45, metalness: 0.35 });
  const tank = new THREE.Mesh(new THREE.CylinderGeometry(40, 40, 50, 20), tankMat);
  tank.position.set(0, 115, 0);
  group.add(tank);

  // Kegeldach
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5 });
  const roof = new THREE.Mesh(new THREE.ConeGeometry(44, 30, 20), roofMat);
  roof.position.set(0, 155, 0);
  group.add(roof);

  // Wasserkran-Schwenkarm
  const pipeMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.3, metalness: 0.8 });
  const arm = new THREE.Mesh(new THREE.CylinderGeometry(3, 3, 40, 8), pipeMat);
  arm.rotation.z = Math.PI / 2;
  arm.position.set(40, 85, 0);
  group.add(arm);

  const spout = new THREE.Mesh(new THREE.CylinderGeometry(3, 3, 20, 8), pipeMat);
  spout.position.set(58, 75, 0);
  group.add(spout);
}
