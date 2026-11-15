import * as THREE from "three";

const SHARED_LAMP_STEEL_MAT = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.7 });
SHARED_LAMP_STEEL_MAT.userData.shared = true;

const SHARED_LAMP_SHADE_MAT = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.5 });
SHARED_LAMP_SHADE_MAT.userData.shared = true;

const SHARED_LAMP_GLOW_MAT = new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.9 });
SHARED_LAMP_GLOW_MAT.userData.shared = true;

const SHARED_LAMP_MAST_GEO = new THREE.CylinderGeometry(1.5, 2, 60, 8);
SHARED_LAMP_MAST_GEO.userData.shared = true;

const SHARED_LAMP_NECK_GEO = new THREE.TorusGeometry(8, 1.2, 8, 16, Math.PI / 2);
SHARED_LAMP_NECK_GEO.rotateZ(Math.PI / 2);
SHARED_LAMP_NECK_GEO.userData.shared = true;

const SHARED_LAMP_SHADE_GEO = new THREE.ConeGeometry(5, 3, 12);
SHARED_LAMP_SHADE_GEO.userData.shared = true;

const SHARED_LAMP_BULB_GEO = new THREE.SphereGeometry(1.8, 8, 8);
SHARED_LAMP_BULB_GEO.userData.shared = true;

/** 10. BAHNSTEIGLEUCHTE (20 x 20 x 70 mm) */
export function buildPlatformLamp3D(group: THREE.Group): void {
  // Mast
  const mast = new THREE.Mesh(SHARED_LAMP_MAST_GEO, SHARED_LAMP_STEEL_MAT);
  mast.position.set(0, 30, 0);
  group.add(mast);

  // Schwanenhals-Bogen
  const neck = new THREE.Mesh(SHARED_LAMP_NECK_GEO, SHARED_LAMP_STEEL_MAT);
  neck.position.set(8, 60, 0);
  group.add(neck);

  // Lampenschirm
  const shade = new THREE.Mesh(SHARED_LAMP_SHADE_GEO, SHARED_LAMP_SHADE_MAT);
  shade.position.set(16, 62, 0);
  group.add(shade);

  // Warmweißes Leuchtmittel
  const bulb = new THREE.Mesh(SHARED_LAMP_BULB_GEO, SHARED_LAMP_GLOW_MAT);
  bulb.position.set(16, 60.5, 0);
  group.add(bulb);
}
