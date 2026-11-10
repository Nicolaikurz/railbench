import * as THREE from "three";

/** 10. BAHNSTEIGLEUCHTE (20 x 20 x 70 mm) */
export function buildPlatformLamp3D(group: THREE.Group): void {
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.7 });
  // Mast
  const mast = new THREE.Mesh(new THREE.CylinderGeometry(1.5, 2, 60, 8), steelMat);
  mast.position.set(0, 30, 0);
  group.add(mast);

  // Schwanenhals-Bogen
  const neckGeo = new THREE.TorusGeometry(8, 1.2, 8, 16, Math.PI / 2);
  neckGeo.rotateZ(Math.PI / 2);
  const neck = new THREE.Mesh(neckGeo, steelMat);
  neck.position.set(8, 60, 0);
  group.add(neck);

  // Lampenschirm
  const shadeMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.5 });
  const shade = new THREE.Mesh(new THREE.ConeGeometry(5, 3, 12), shadeMat);
  shade.position.set(16, 62, 0);
  group.add(shade);

  // Warmweißes Leuchtmittel
  const glowMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.9 });
  const bulb = new THREE.Mesh(new THREE.SphereGeometry(1.8, 8, 8), glowMat);
  bulb.position.set(16, 60.5, 0);
  group.add(bulb);
}
