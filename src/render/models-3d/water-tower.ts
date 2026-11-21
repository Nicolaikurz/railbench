import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedCylinderGeometry,
  getSharedMaterial,
} from "./primitive-cache";

let _sharedConeRoofGeo: THREE.ConeGeometry | null = null;
let _sharedCupolaRoofGeo: THREE.ConeGeometry | null = null;

function getSharedConeRoofGeo(): THREE.ConeGeometry {
  if (!_sharedConeRoofGeo) {
    _sharedConeRoofGeo = new THREE.ConeGeometry(44, 30, 20);
    _sharedConeRoofGeo.userData.shared = true;
  }
  return _sharedConeRoofGeo;
}

function getSharedCupolaRoofGeo(): THREE.ConeGeometry {
  if (!_sharedCupolaRoofGeo) {
    _sharedCupolaRoofGeo = new THREE.ConeGeometry(12, 11, 12);
    _sharedCupolaRoofGeo.userData.shared = true;
  }
  return _sharedCupolaRoofGeo;
}

/**
 * 5. WASSERTURM — Historischer Eisenbahn-Wasserturm (85 × 85 × 180 mm)
 * Gestufter Natursteinsockel, Backsteinschaft mit Rundbogen-Tür & Turmfenstern,
 * auskragender genieteter Hochbehälter auf Konsolring, Schiefer-Kegeldach mit Laterne,
 * Wasserstandsanzeiger, Steigleiter & schwenkbarer Wasserkran.
 */
export function buildWaterTower3D(group: THREE.Group): void {
  const stoneMat = getSharedMaterial(0x64748b, 0.9);
  const corniceMat = getSharedMaterial(0xcbd5e1, 0.78);
  const brickMat = getSharedMaterial(0x9a3412, 0.82);
  const tankWallMat = getSharedMaterial(0x475569, 0.58, 0.35);
  const tankBandMat = getSharedMaterial(0x1e293b, 0.4, 0.72);
  const roofMat = getSharedMaterial(0x334155, 0.55, 0.25);
  const woodDoorMat = getSharedMaterial(0x5c2e14, 0.8);
  const glassWarmMat = getSharedMaterial(0xfde68a, 0.2, 0.2, 0xf59e0b, 0.4);
  const glassDarkMat = getSharedMaterial(0x1e3a5f, 0.15, 0.8);
  const brassMat = getSharedMaterial(0xd97706, 0.3, 0.8);
  const gaugeBgMat = getSharedMaterial(0xf8fafc, 0.5);
  const gaugeRedMat = getSharedMaterial(0xdc2626, 0.45);

  // 1. Gestufter Natursteinsockel
  const basePlinth = new THREE.Mesh(getSharedCylinderGeometry(39, 41, 6, 16), stoneMat);
  basePlinth.position.set(0, 3, 0);
  group.add(basePlinth);

  const lowerStone = new THREE.Mesh(getSharedCylinderGeometry(35, 37, 16, 16), stoneMat);
  lowerStone.position.set(0, 14, 0);
  group.add(lowerStone);

  const plinthBelt = new THREE.Mesh(getSharedCylinderGeometry(36.5, 36.5, 2.5, 16), corniceMat);
  plinthBelt.position.set(0, 22.5, 0);
  group.add(plinthBelt);

  // 2. Konischer Backsteinschaft
  const shaft = new THREE.Mesh(getSharedCylinderGeometry(28.5, 34, 68, 16), brickMat);
  shaft.position.set(0, 57, 0);
  group.add(shaft);

  // Eingangsportal (Stahl-/Holztür mit Natursteinrahmen)
  const doorFrame = new THREE.Mesh(getSharedBoxGeometry(14, 24, 4), corniceMat);
  doorFrame.position.set(0, 17, 34.5);
  group.add(doorFrame);
  const doorLeaf = new THREE.Mesh(getSharedBoxGeometry(10.5, 21, 4.4), woodDoorMat);
  doorLeaf.position.set(0, 16, 34.8);
  group.add(doorLeaf);

  // Schmale Turmfenster (4 Seiten, 2 Ebenen)
  const winFrameGeo = getSharedBoxGeometry(6.5, 11, 2.2);
  const winPaneGeo = getSharedBoxGeometry(4.5, 8.8, 2.5);
  for (let i = 0; i < 4; i++) {
    const ang = (i * Math.PI) / 2;
    for (const wy of [46, 72]) {
      const r = wy === 46 ? 31.8 : 29.8;
      const wg = new THREE.Group();
      wg.position.set(Math.sin(ang) * r, wy, Math.cos(ang) * r);
      wg.rotation.y = ang;
      wg.add(new THREE.Mesh(winFrameGeo, corniceMat));
      wg.add(new THREE.Mesh(winPaneGeo, wy === 72 && i % 2 === 0 ? glassWarmMat : glassDarkMat));
      group.add(wg);
    }
  }

  // 3. Auskragender Konsolring unter dem Hochbehälter
  const corbelRing = new THREE.Mesh(getSharedCylinderGeometry(39, 29.5, 10, 16), corniceMat);
  corbelRing.position.set(0, 95, 0);
  group.add(corbelRing);

  const bracketGeo = getSharedBoxGeometry(3.2, 10, 9);
  for (let i = 0; i < 12; i++) {
    const ang = (i * Math.PI * 2) / 12;
    const br = new THREE.Mesh(bracketGeo, stoneMat);
    br.position.set(Math.sin(ang) * 33, 93, Math.cos(ang) * 33);
    br.rotation.y = ang;
    group.add(br);
  }

  // 4. Verkleideter Wasserbehälter mit Stahlbändern & Fensterband
  const tankBody = new THREE.Mesh(getSharedCylinderGeometry(40, 39.5, 44, 20), tankWallMat);
  tankBody.position.set(0, 122, 0);
  group.add(tankBody);

  // Stahl-Spannringe um den Behälter
  for (const by of [104, 118, 132, 143]) {
    const band = new THREE.Mesh(getSharedCylinderGeometry(40.7, 40.7, 1.8, 20), tankBandMat);
    band.position.set(0, by, 0);
    group.add(band);
  }

  // Kleine Kontrollfenster rund um das Behältergeschoss
  const tankWinGeo = getSharedBoxGeometry(7, 8, 1.5);
  for (let i = 0; i < 8; i++) {
    const ang = (i * Math.PI * 2) / 8 + Math.PI / 8;
    const tw = new THREE.Mesh(tankWinGeo, i % 3 === 0 ? glassWarmMat : glassDarkMat);
    tw.position.set(Math.sin(ang) * 40.2, 125, Math.cos(ang) * 40.2);
    tw.rotation.y = ang;
    group.add(tw);
  }

  // Wasserstandsanzeiger an der Vorderseite
  const gaugeBoard = new THREE.Mesh(getSharedBoxGeometry(4.5, 34, 1.6), gaugeBgMat);
  gaugeBoard.position.set(0, 122, 40.6);
  group.add(gaugeBoard);
  const gaugeMarker = new THREE.Mesh(getSharedBoxGeometry(5.2, 3.2, 2.2), gaugeRedMat);
  gaugeMarker.position.set(0, 130, 40.9);
  group.add(gaugeMarker);

  // 5. Schiefer-Kegeldach + Dachlaterne + Blitzableiter
  const roof = new THREE.Mesh(getSharedConeRoofGeo(), roofMat);
  roof.position.set(0, 159, 0);
  group.add(roof);

  const cupolaBase = new THREE.Mesh(getSharedCylinderGeometry(9.6, 10, 3.5, 12), tankBandMat);
  cupolaBase.position.set(0, 170.75, 0);
  group.add(cupolaBase);

  const cupolaGlass = new THREE.Mesh(getSharedCylinderGeometry(8.4, 8.4, 6, 12), glassWarmMat);
  cupolaGlass.position.set(0, 175.5, 0);
  group.add(cupolaGlass);

  const cupolaTopRing = new THREE.Mesh(getSharedCylinderGeometry(9.6, 9.6, 1.2, 12), tankBandMat);
  cupolaTopRing.position.set(0, 178.8, 0);
  group.add(cupolaTopRing);

  const mullionGeo = getSharedCylinderGeometry(0.45, 0.45, 6.2, 6);
  for (let i = 0; i < 6; i++) {
    const a = (i * Math.PI * 2) / 6;
    const m = new THREE.Mesh(mullionGeo, tankBandMat);
    m.position.set(Math.sin(a) * 8.9, 175.5, Math.cos(a) * 8.9);
    group.add(m);
  }

  const cupolaRoof = new THREE.Mesh(getSharedCupolaRoofGeo(), roofMat);
  cupolaRoof.position.set(0, 184, 0);
  group.add(cupolaRoof);

  const finial = new THREE.Mesh(getSharedCylinderGeometry(0.6, 1.0, 12, 8), brassMat);
  finial.position.set(0, 194, 0);
  group.add(finial);

  // 6. Steigleiter außen auf der Rückseite (-Z = -41.2, frei vor dem Behälter)
  const railGeo = getSharedCylinderGeometry(0.6, 0.6, 132, 6);
  for (const lx of [-2.4, 2.4]) {
    const rail = new THREE.Mesh(railGeo, tankBandMat);
    rail.position.set(lx, 78, -41.2);
    group.add(rail);
  }
  const rungGeo = getSharedBoxGeometry(5.2, 0.8, 0.8);
  for (let ry = 18; ry <= 140; ry += 8) {
    const rung = new THREE.Mesh(rungGeo, tankBandMat);
    rung.position.set(0, ry, -41.2);
    group.add(rung);
  }
  for (const sy of [28, 60, 92, 124]) {
    for (const lx of [-2.4, 2.4]) {
      const standOff = new THREE.Mesh(getSharedBoxGeometry(0.8, 0.8, 9.5), tankBandMat);
      standOff.position.set(lx, sy, -36.5);
      group.add(standOff);
    }
  }

  // 7. Wasserkran-Ausleger (+X Seite) mit Steigrohr, Flansch & Trichter
  const standPipe = new THREE.Mesh(getSharedCylinderGeometry(3.2, 3.8, 86, 10), tankBandMat);
  standPipe.position.set(36, 46, 0);
  group.add(standPipe);

  const arm = new THREE.Mesh(getSharedCylinderGeometry(2.6, 2.8, 30, 10), tankBandMat);
  arm.rotation.z = Math.PI / 2;
  arm.position.set(49, 86, 0);
  group.add(arm);

  const flange = new THREE.Mesh(getSharedCylinderGeometry(4.2, 4.2, 2.2, 10), tankBandMat);
  flange.position.set(36, 87, 0);
  group.add(flange);

  const spout = new THREE.Mesh(getSharedCylinderGeometry(2.8, 2.0, 18, 10), tankBandMat);
  spout.position.set(62, 78, 0);
  group.add(spout);
}
