import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedCylinderGeometry,
  getSharedMaterial,
  getSharedSphereGeometry,
} from "./primitive-cache";

let _sharedNeckGeo: THREE.TorusGeometry | null = null;
let _sharedShadeGeo: THREE.ConeGeometry | null = null;

function getSharedNeckGeo(): THREE.TorusGeometry {
  if (!_sharedNeckGeo) {
    _sharedNeckGeo = new THREE.TorusGeometry(8.5, 1.1, 8, 24, Math.PI);
    _sharedNeckGeo.userData.shared = true;
  }
  return _sharedNeckGeo;
}

function getSharedShadeGeo(): THREE.ConeGeometry {
  if (!_sharedShadeGeo) {
    _sharedShadeGeo = new THREE.ConeGeometry(6.2, 3.6, 16);
    _sharedShadeGeo.userData.shared = true;
  }
  return _sharedShadeGeo;
}

/**
 * 10. BAHNSTEIGLEUCHTE — Klassische Gusseisen-Peitschenleuchte (20 × 20 × 74 mm)
 * Gestufter Gusseisensockel, Zierkragen, schlanker Mast, Schwanenhals-Ausleger,
 * Emaille-Schirm (außen dunkel, innen weiß) und warmweißer Glasdiffusor.
 */
export function buildPlatformLamp3D(group: THREE.Group): void {
  const ironMat = getSharedMaterial(0x1e293b, 0.38, 0.75);
  const enamelDarkMat = getSharedMaterial(0x334155, 0.45, 0.4);
  const enamelWhiteMat = getSharedMaterial(0xf8fafc, 0.4);
  const glowMat = getSharedMaterial(0xfef08a, 0.15, 0.05, 0xfef08a, 0.95);

  // Gestufter Gusseisen-Sockel
  const plinth = new THREE.Mesh(getSharedBoxGeometry(7.5, 2.2, 7.5), ironMat);
  plinth.position.set(0, 1.1, 0);
  group.add(plinth);

  const baseCone = new THREE.Mesh(getSharedCylinderGeometry(2.4, 3.6, 11, 10), ironMat);
  baseCone.position.set(0, 7.5, 0);
  group.add(baseCone);

  const collar1 = new THREE.Mesh(getSharedCylinderGeometry(3.0, 3.0, 1.6, 10), ironMat);
  collar1.position.set(0, 13.5, 0);
  group.add(collar1);

  // Schlanker verjüngter Mast
  const mast = new THREE.Mesh(getSharedCylinderGeometry(1.15, 1.85, 50, 10), ironMat);
  mast.position.set(0, 38.5, 0);
  group.add(mast);

  const collar2 = new THREE.Mesh(getSharedCylinderGeometry(2.0, 2.0, 1.4, 10), ironMat);
  collar2.position.set(0, 62.5, 0);
  group.add(collar2);

  // Schwanenhals-Bogen
  const neck = new THREE.Mesh(getSharedNeckGeo(), ironMat);
  neck.position.set(8.5, 63, 0);
  group.add(neck);

  // Emaille-Lampenschirm + weißer Reflektor (direkt am Bogenende (17, 63, 0))
  const shade = new THREE.Mesh(getSharedShadeGeo(), enamelDarkMat);
  shade.position.set(17, 61.8, 0);
  group.add(shade);

  const reflector = new THREE.Mesh(getSharedCylinderGeometry(5.6, 5.6, 0.6, 16), enamelWhiteMat);
  reflector.position.set(17, 60.0, 0);
  group.add(reflector);

  // Warmweißer Glasdiffusor
  const bulb = new THREE.Mesh(getSharedSphereGeometry(2.3, 12, 10), glowMat);
  bulb.position.set(17, 58.8, 0);
  group.add(bulb);
}
