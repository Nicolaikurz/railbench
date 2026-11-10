import * as THREE from "three";

/** 7. PRELLBOCK (25 x 35 x 25 mm) */
export function buildBufferStop3D(group: THREE.Group): void {
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x374151, roughness: 0.6, metalness: 0.6 });

  // A-Träger links & rechts
  const beamLeft = new THREE.Mesh(new THREE.BoxGeometry(3, 24, 30), steelMat);
  beamLeft.rotation.x = -0.4;
  beamLeft.position.set(-8.5, 12, 0);
  group.add(beamLeft);

  const beamRight = new THREE.Mesh(new THREE.BoxGeometry(3, 24, 30), steelMat);
  beamRight.rotation.x = -0.4;
  beamRight.position.set(8.5, 12, 0);
  group.add(beamRight);

  // Holz-Pufferbohle
  const woodMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.85 });
  const bufferPlank = new THREE.Mesh(new THREE.BoxGeometry(26, 7, 6), woodMat);
  bufferPlank.position.set(0, 16, -10);
  group.add(bufferPlank);

  // Rote Signal-Kreisscheibe
  const diskMat = new THREE.MeshStandardMaterial({ color: 0xdc2626, roughness: 0.5 });
  const disk = new THREE.Mesh(new THREE.CylinderGeometry(4.5, 4.5, 1, 16), diskMat);
  disk.rotation.x = Math.PI / 2;
  disk.position.set(0, 16, -13.5);
  group.add(disk);
}
