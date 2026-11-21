import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedCylinderGeometry,
  getSharedMaterial,
} from "./primitive-cache";

/**
 * 2. STELLWERK — Klassisches Fahrdienstleiter-Stellwerk (120 × 75 × 110 mm)
 * Sauberer Steinsockel ohne überstehende Grasplatte, Klinker-Untergeschoss,
 * auskragender verglaster Stellwerksraum mit Fenstersprossen & sichtbarem Stellpult,
 * Stahl-Außentreppe mit Podest & Geländer, Walmdach mit Antenne & Stellwerksschild.
 */
export function buildSignalBox3D(group: THREE.Group): void {
  const stoneMat = getSharedMaterial(0x64748b, 0.88);
  const corniceMat = getSharedMaterial(0xd6d3d1, 0.75);
  const brickMat = getSharedMaterial(0x9a3412, 0.82);
  const upperWallMat = getSharedMaterial(0xe5d6be, 0.76);
  const steelMat = getSharedMaterial(0x1e293b, 0.42, 0.72);
  const roofMat = getSharedMaterial(0x334155, 0.62, 0.3);
  const glassMat = getSharedMaterial(0x38bdf8, 0.12, 0.82);
  const warmGlassMat = getSharedMaterial(0xfde68a, 0.18, 0.25, 0xf59e0b, 0.45);
  const doorMat = getSharedMaterial(0x5c2e14, 0.78);
  const signBlueMat = getSharedMaterial(0x1d4ed8, 0.45, 0.15);
  const signWhiteMat = getSharedMaterial(0xf8fafc, 0.55);
  const deskMat = getSharedMaterial(0x475569, 0.6, 0.3);
  const leverRedMat = getSharedMaterial(0xdc2626, 0.4);
  const leverBlueMat = getSharedMaterial(0x2563eb, 0.4);

  // 1. Sauberer Betonsockel (exakt innerhalb 120 × 75 mm)
  const plinth = new THREE.Mesh(getSharedBoxGeometry(118, 5, 73), stoneMat);
  plinth.position.set(0, 2.5, 0);
  group.add(plinth);

  // 2. Klinker-Untergeschoss (Relaisraum)
  const groundFloor = new THREE.Mesh(getSharedBoxGeometry(92, 42, 58), brickMat);
  groundFloor.position.set(-4, 26, 0);
  group.add(groundFloor);

  // Gesimsband zwischen EG & OG
  const belt = new THREE.Mesh(getSharedBoxGeometry(108, 3.2, 70), corniceMat);
  belt.position.set(-4, 48, 0);
  group.add(belt);

  // EG-Fenster (Vorder- & Rückseite) + Stahltür
  const egFrameGeo = getSharedBoxGeometry(11, 14, 1.4);
  const egGlassGeo = getSharedBoxGeometry(8.6, 11.6, 1.6);
  for (const side of [-1, 1] as const) {
    const wz = side * 29;
    for (const dx of [-32, -8, 16]) {
      const f = new THREE.Mesh(egFrameGeo, corniceMat);
      f.position.set(dx, 27, wz + side * 0.4);
      group.add(f);
      const gl = new THREE.Mesh(egGlassGeo, side === 1 && dx === -8 ? warmGlassMat : glassMat);
      gl.position.set(dx, 27, wz + side * 0.6);
      group.add(gl);
    }
  }

  // Eingangstür EG rechts
  const doorFrame = new THREE.Mesh(getSharedBoxGeometry(1.8, 22, 13), corniceMat);
  doorFrame.position.set(42.2, 16, 0);
  group.add(doorFrame);
  const door = new THREE.Mesh(getSharedBoxGeometry(2.0, 20, 10.5), doorMat);
  door.position.set(42.4, 15.5, 0);
  group.add(door);

  // 3. Auskragendes Obergeschoss (Stellwerksraum mit Panorama-Fensterband)
  const lowerParapet = new THREE.Mesh(getSharedBoxGeometry(106, 11, 68), upperWallMat);
  lowerParapet.position.set(-4, 55, 0);
  group.add(lowerParapet);

  const upperHeader = new THREE.Mesh(getSharedBoxGeometry(106, 6, 68), upperWallMat);
  upperHeader.position.set(-4, 81, 0);
  group.add(upperHeader);

  // Panorama-Verglasung rundum (halbtransparent, damit Stellpult & Hebelbank sichtbar sind)
  const panoGlassMat = new THREE.MeshStandardMaterial({
    color: 0xfde68a,
    roughness: 0.18,
    metalness: 0.2,
    emissive: 0xf59e0b,
    emissiveIntensity: 0.25,
    transparent: true,
    opacity: 0.42,
  });
  const glassCore = new THREE.Mesh(getSharedBoxGeometry(104, 18, 66), panoGlassMat);
  glassCore.position.set(-4, 69.5, 0);
  group.add(glassCore);

  // Innenraum-Stellpult / Hebelbank (durch die Scheibe erkennbar)
  const desk = new THREE.Mesh(getSharedBoxGeometry(70, 9, 10), deskMat);
  desk.position.set(-4, 64, 14);
  group.add(desk);
  const leverGeo = getSharedCylinderGeometry(0.6, 0.6, 7, 6);
  for (let lx = -28; lx <= 20; lx += 8) {
    const lev = new THREE.Mesh(leverGeo, lx % 16 === 0 ? leverRedMat : leverBlueMat);
    lev.position.set(lx, 70, 14);
    lev.rotation.x = 0.25;
    group.add(lev);
  }

  // Vertikale Fenstersprossen rund um den Stellwerksraum
  const mullionGeo = getSharedBoxGeometry(2.2, 19, 2.2);
  for (const side of [-1, 1] as const) {
    for (let mx = -56; mx <= 48; mx += 13) {
      const m = new THREE.Mesh(mullionGeo, steelMat);
      m.position.set(mx, 69.5, side * 33.4);
      group.add(m);
    }
  }
  for (const sideX of [-56, 48]) {
    for (let mz = -22; mz <= 22; mz += 14.5) {
      const m = new THREE.Mesh(mullionGeo, steelMat);
      m.position.set(sideX, 69.5, mz);
      group.add(m);
    }
  }

  // 4. Auskragendes Dach mit Dachrand, Entlüfter & Funkantenne
  const roofLower = new THREE.Mesh(getSharedBoxGeometry(116, 4, 78), roofMat);
  roofLower.position.set(-4, 86, 0);
  group.add(roofLower);

  const roofUpper = new THREE.Mesh(getSharedBoxGeometry(102, 5, 64), roofMat);
  roofUpper.position.set(-4, 90.5, 0);
  group.add(roofUpper);

  const antenna = new THREE.Mesh(getSharedCylinderGeometry(0.5, 0.8, 18, 6), steelMat);
  antenna.position.set(-36, 102, -16);
  group.add(antenna);

  const vent = new THREE.Mesh(getSharedBoxGeometry(10, 6, 10), stoneMat);
  vent.position.set(22, 95, -14);
  group.add(vent);

  // Stellwerks-Schild (DB-Blau/Weiß) an beiden Längsseiten
  const signBgGeo = getSharedBoxGeometry(34, 6.5, 1.2);
  const signTxtGeo = getSharedBoxGeometry(26, 2.2, 1.5);
  for (const side of [-1, 1] as const) {
    const sb = new THREE.Mesh(signBgGeo, signBlueMat);
    sb.position.set(-4, 55, side * 34.4);
    group.add(sb);
    const st = new THREE.Mesh(signTxtGeo, signWhiteMat);
    st.position.set(-4, 55, side * 34.5);
    group.add(st);
  }

  // 5. Stahl-Außentreppe rechts (+X) zum Stellwerksraum
  const landing = new THREE.Mesh(getSharedBoxGeometry(13, 2, 18), steelMat);
  landing.position.set(54, 49, -18);
  group.add(landing);

  const stepGeo = getSharedBoxGeometry(11, 1.4, 4);
  for (let i = 0; i < 11; i++) {
    const st = new THREE.Mesh(stepGeo, steelMat);
    st.position.set(54, 7 + i * 4.1, 24 - i * 3.8);
    group.add(st);
  }

  // Treppenwangen (innen & außen), Handlauf & Podest-Geländer
  const stringerOuter = new THREE.Mesh(getSharedBoxGeometry(1.2, 2.5, 58), steelMat);
  stringerOuter.position.set(59.5, 28, 3);
  stringerOuter.rotation.x = 0.82;
  group.add(stringerOuter);

  const stringerInner = new THREE.Mesh(getSharedBoxGeometry(1.2, 2.5, 58), steelMat);
  stringerInner.position.set(48.5, 28, 3);
  stringerInner.rotation.x = 0.82;
  group.add(stringerInner);

  const handrail = new THREE.Mesh(getSharedBoxGeometry(1.0, 1.2, 58), steelMat);
  handrail.position.set(59.5, 38, 3);
  handrail.rotation.x = 0.82;
  group.add(handrail);

  const postGeo = getSharedBoxGeometry(1.0, 10, 1.0);
  for (const [pz, py] of [
    [22, 14],
    [8, 29],
    [-6, 44],
    [-16, 54],
  ] as const) {
    const p = new THREE.Mesh(postGeo, steelMat);
    p.position.set(59.5, py, pz);
    group.add(p);
  }

  // Sicherheitsgeländer am oberen Podest (+X und -Z)
  for (const [px, pz] of [
    [60, -26.5],
    [48.5, -26.5],
  ] as const) {
    const lp = new THREE.Mesh(postGeo, steelMat);
    lp.position.set(px, 54, pz);
    group.add(lp);
  }
  const landingRailX = new THREE.Mesh(getSharedBoxGeometry(1.0, 1.0, 12), steelMat);
  landingRailX.position.set(60, 59, -21.2);
  group.add(landingRailX);
  const landingRailZ = new THREE.Mesh(getSharedBoxGeometry(12.5, 1.0, 1.0), steelMat);
  landingRailZ.position.set(54.2, 59, -26.5);
  group.add(landingRailZ);
}

/** Vereinfachte Low-Poly Variante des Stellwerks (LOD >= 3500 mm) */
export function buildSignalBoxLowPoly3D(group: THREE.Group): void {
  const stoneMat = getSharedMaterial(0x64748b, 0.88);
  const brickMat = getSharedMaterial(0x9a3412, 0.82);
  const upperWallMat = getSharedMaterial(0xe5d6be, 0.76);
  const roofMat = getSharedMaterial(0x334155, 0.62, 0.3);

  const plinth = new THREE.Mesh(getSharedBoxGeometry(118, 5, 73), stoneMat);
  plinth.position.set(0, 2.5, 0);
  group.add(plinth);

  const groundFloor = new THREE.Mesh(getSharedBoxGeometry(92, 42, 58), brickMat);
  groundFloor.position.set(-4, 26, 0);
  group.add(groundFloor);

  const topFloor = new THREE.Mesh(getSharedBoxGeometry(106, 36, 68), upperWallMat);
  topFloor.position.set(-4, 65, 0);
  group.add(topFloor);

  const roof = new THREE.Mesh(getSharedBoxGeometry(116, 7, 78), roofMat);
  roof.position.set(-4, 87, 0);
  group.add(roof);
}
