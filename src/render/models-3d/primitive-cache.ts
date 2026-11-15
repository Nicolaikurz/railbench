import * as THREE from "three";

const _boxCache = new Map<string, THREE.BoxGeometry>();
const _cylinderCache = new Map<string, THREE.CylinderGeometry>();
const _dodecahedronCache = new Map<string, THREE.DodecahedronGeometry>();
const _sphereCache = new Map<string, THREE.SphereGeometry>();
const _matCache = new Map<string, THREE.MeshStandardMaterial>();

/**
 * Gibt eine gecachte BoxGeometry für identische Abmessungen zurück.
 * Markiert mit userData.shared = true, um Redundanzen im RAM zu vermeiden.
 */
export function getSharedBoxGeometry(w: number, h: number, d: number): THREE.BoxGeometry {
  const rw = Math.round(w * 10) / 10;
  const rh = Math.round(h * 10) / 10;
  const rd = Math.round(d * 10) / 10;
  const key = `${rw}:${rh}:${rd}`;
  let geo = _boxCache.get(key);
  if (!geo) {
    geo = new THREE.BoxGeometry(w, h, d);
    geo.userData.shared = true;
    _boxCache.set(key, geo);
  }
  return geo;
}

/**
 * Gibt eine gecachte CylinderGeometry zurück.
 */
export function getSharedCylinderGeometry(
  radiusTop: number,
  radiusBottom: number,
  height: number,
  radialSegments = 8,
): THREE.CylinderGeometry {
  const rt = Math.round(radiusTop * 10) / 10;
  const rb = Math.round(radiusBottom * 10) / 10;
  const rh = Math.round(height * 10) / 10;
  const key = `${rt}:${rb}:${rh}:${radialSegments}`;
  let geo = _cylinderCache.get(key);
  if (!geo) {
    geo = new THREE.CylinderGeometry(radiusTop, radiusBottom, height, radialSegments);
    geo.userData.shared = true;
    _cylinderCache.set(key, geo);
  }
  return geo;
}

/**
 * Gibt eine gecachte DodecahedronGeometry (z.B. für Büsche/Laub) zurück.
 */
export function getSharedDodecahedronGeometry(radius: number, detail = 1): THREE.DodecahedronGeometry {
  const rr = Math.round(radius * 10) / 10;
  const key = `${rr}:${detail}`;
  let geo = _dodecahedronCache.get(key);
  if (!geo) {
    geo = new THREE.DodecahedronGeometry(radius, detail);
    geo.userData.shared = true;
    _dodecahedronCache.set(key, geo);
  }
  return geo;
}

/**
 * Gibt eine gecachte SphereGeometry zurück.
 */
export function getSharedSphereGeometry(
  radius: number,
  widthSegments = 8,
  heightSegments = 8,
): THREE.SphereGeometry {
  const rr = Math.round(radius * 10) / 10;
  const key = `${rr}:${widthSegments}:${heightSegments}`;
  let geo = _sphereCache.get(key);
  if (!geo) {
    geo = new THREE.SphereGeometry(radius, widthSegments, heightSegments);
    geo.userData.shared = true;
    _sphereCache.set(key, geo);
  }
  return geo;
}

/**
 * Gibt ein geteiltes MeshStandardMaterial für standardisierte Farben zurück.
 */
export function getSharedMaterial(
  color: number,
  roughness = 0.8,
  metalness = 0.0,
  emissive?: number,
  emissiveIntensity?: number,
): THREE.MeshStandardMaterial {
  const rc = color.toString(16);
  const rr = Math.round(roughness * 100);
  const rm = Math.round(metalness * 100);
  const em = emissive ? emissive.toString(16) : "0";
  const ei = Math.round((emissiveIntensity ?? 0) * 100);
  const key = `${rc}:${rr}:${rm}:${em}:${ei}`;
  let mat = _matCache.get(key);
  if (!mat) {
    mat = new THREE.MeshStandardMaterial({
      color,
      roughness,
      metalness,
      ...(emissive ? { emissive, emissiveIntensity: emissiveIntensity ?? 1 } : {}),
    });
    mat.userData.shared = true;
    _matCache.set(key, mat);
  }
  return mat;
}
