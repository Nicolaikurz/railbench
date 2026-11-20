import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedMaterial,
} from "./primitive-cache";

let _sharedRoofGeo: THREE.ExtrudeGeometry | null = null;
let _sharedGableGeo: THREE.ExtrudeGeometry | null = null;

function getSharedTownHouseRoofGeo(): THREE.ExtrudeGeometry {
  if (!_sharedRoofGeo) {
    const roofShape = new THREE.Shape();
    roofShape.moveTo(-46, 0);
    roofShape.lineTo(0, 44);
    roofShape.lineTo(46, 0);
    roofShape.closePath();
    _sharedRoofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 112, bevelEnabled: false });
    _sharedRoofGeo.rotateY(Math.PI / 2);
    _sharedRoofGeo.translate(-56, 76, 0);
    _sharedRoofGeo.userData.shared = true;
  }
  return _sharedRoofGeo;
}

function getSharedTownHouseGableGeo(): THREE.ExtrudeGeometry {
  if (!_sharedGableGeo) {
    const gableShape = new THREE.Shape();
    gableShape.moveTo(-41, 0);
    gableShape.lineTo(0, 39.2);
    gableShape.lineTo(41, 0);
    gableShape.closePath();
    _sharedGableGeo = new THREE.ExtrudeGeometry(gableShape, { depth: 1.2, bevelEnabled: false });
    _sharedGableGeo.rotateY(Math.PI / 2);
    _sharedGableGeo.userData.shared = true;
  }
  return _sharedGableGeo;
}

/**
 * 4. FACHWERKHAUS — Traditionelles deutsches Bürger-/Fachwerkhaus (110 × 90 × 125 mm)
 * Sauberer Natursteinsockel (ohne überstehende Grasplatte), warmer cremefarbener Putz
 * mit dunklem Holzfachwerk, Sprossenfenstern, grünen Fensterläden, Blumenkästen,
 * Dachgaube & Backsteinschornstein.
 */
export function buildTownHouse3D(group: THREE.Group): void {
  const stoneMat = getSharedMaterial(0x64748b, 0.9);
  const plasterMat = getSharedMaterial(0xf3ead8, 0.82); // Warmer Kalkputz statt Grellgelb
  const beamMat = getSharedMaterial(0x451a03, 0.88);
  const roofMat = getSharedMaterial(0x991b1b, 0.68);
  const ridgeMat = getSharedMaterial(0x7c2d12, 0.72);
  const brickMat = getSharedMaterial(0x9a3412, 0.82);
  const doorMat = getSharedMaterial(0x5c2e14, 0.78);
  const frameMat = getSharedMaterial(0xf8fafc, 0.65);
  const shutterMat = getSharedMaterial(0x166534, 0.78);
  const glassDarkMat = getSharedMaterial(0x1e3a5f, 0.14, 0.82);
  const glassWarmMat = getSharedMaterial(0xfde68a, 0.2, 0.25, 0xf59e0b, 0.42);
  const flowerBoxMat = getSharedMaterial(0x78350f, 0.85);
  const flowerRedMat = getSharedMaterial(0xdc2626, 0.7);

  // 1. Naturstein-Sockel (exakt innerhalb 110 × 90 mm)
  const plinth = new THREE.Mesh(getSharedBoxGeometry(108, 9, 86), stoneMat);
  plinth.position.set(0, 4.5, 0);
  group.add(plinth);

  // 2. Erdgeschoss & leicht auskragendes Obergeschoss
  const egBody = new THREE.Mesh(getSharedBoxGeometry(102, 33, 78), plasterMat);
  egBody.position.set(0, 25.5, 0);
  group.add(egBody);

  const ogBody = new THREE.Mesh(getSharedBoxGeometry(106, 35, 82), plasterMat);
  ogBody.position.set(0, 59, 0);
  group.add(ogBody);

  // Giebeldreiecke links & rechts
  for (const sx of [-1, 1]) {
    const gable = new THREE.Mesh(getSharedTownHouseGableGeo(), plasterMat);
    gable.position.set(sx * 52.4, 76, 0);
    group.add(gable);
  }

  // 3. Fachwerk-Balken (horizontale Riegel & vertikale Ständer)
  const hBeamLongGeo = getSharedBoxGeometry(107.6, 2.6, 83.6);
  for (const by of [42.5, 59, 75.2]) {
    const hb = new THREE.Mesh(hBeamLongGeo, beamMat);
    hb.position.set(0, by, 0);
    group.add(hb);
  }

  const vPostGeo = getSharedBoxGeometry(2.6, 35, 2.6);
  const egPostGeo = getSharedBoxGeometry(2.6, 33, 2.6);
  for (const sz of [-1, 1]) {
    for (const bx of [-52.2, -26, 0, 26, 52.2]) {
      const ogPost = new THREE.Mesh(vPostGeo, beamMat);
      ogPost.position.set(bx, 59, sz * 40.5);
      group.add(ogPost);
    }
    for (const bx of [-50.2, -25, 25, 50.2]) {
      const egPost = new THREE.Mesh(egPostGeo, beamMat);
      egPost.position.set(bx, 25.5, sz * 38.5);
      group.add(egPost);
    }
  }

  // Diagonale Fachwerk-Streben an den Ecken des OG
  const braceGeo = getSharedBoxGeometry(2.2, 22, 1.8);
  for (const sz of [-1, 1]) {
    for (const sx of [-1, 1]) {
      const br = new THREE.Mesh(braceGeo, beamMat);
      br.position.set(sx * 39, 59, sz * 41.1);
      br.rotation.z = sx * 0.65;
      group.add(br);
    }
  }

  // 4. Steiles Satteldach + Firstkappe + Dachgaube + Schornstein
  const roof = new THREE.Mesh(getSharedTownHouseRoofGeo(), roofMat);
  group.add(roof);

  const ridge = new THREE.Mesh(getSharedBoxGeometry(113, 2.4, 3.4), ridgeMat);
  ridge.position.set(0, 120.2, 0);
  group.add(ridge);

  // Dachgaube vorne (+Z)
  const dormerWall = new THREE.Mesh(getSharedBoxGeometry(20, 14, 18), plasterMat);
  dormerWall.position.set(0, 91, 22);
  group.add(dormerWall);
  const dormerRoof = new THREE.Mesh(getSharedBoxGeometry(23, 3.2, 21), roofMat);
  dormerRoof.position.set(0, 99, 22);
  group.add(dormerRoof);
  const dormerWin = new THREE.Mesh(getSharedBoxGeometry(10, 8.5, 1.2), glassWarmMat);
  dormerWin.position.set(0, 91.5, 31.2);
  group.add(dormerWin);

  // Backstein-Schornstein
  const chimney = new THREE.Mesh(getSharedBoxGeometry(10, 26, 10), brickMat);
  chimney.position.set(28, 112, -14);
  group.add(chimney);
  const chimneyCap = new THREE.Mesh(getSharedBoxGeometry(12, 2.2, 12), stoneMat);
  chimneyCap.position.set(28, 125.5, -14);
  group.add(chimneyCap);

  // 5. Sprossenfenster mit grünen Fensterläden & Blumenkästen
  const winFrameGeo = getSharedBoxGeometry(12.5, 15.5, 1.2);
  const winGlassGeo = getSharedBoxGeometry(10, 13, 1.4);
  const shutterGeo = getSharedBoxGeometry(4.2, 15.5, 1.1);
  const fboxGeo = getSharedBoxGeometry(13, 2.8, 3.2);
  const flowerGeo = getSharedBoxGeometry(11.5, 2.2, 2.2);

  const addHouseWindow = (wx: number, wy: number, wz: number, side: 1 | -1, warm: boolean, withFlowers: boolean) => {
    const f = new THREE.Mesh(winFrameGeo, frameMat);
    f.position.set(wx, wy, wz + side * 0.5);
    group.add(f);

    const g = new THREE.Mesh(winGlassGeo, warm ? glassWarmMat : glassDarkMat);
    g.position.set(wx, wy, wz + side * 0.7);
    group.add(g);

    for (const sx of [-1, 1]) {
      const sh = new THREE.Mesh(shutterGeo, shutterMat);
      sh.position.set(wx + sx * 8.6, wy, wz + side * 0.6);
      group.add(sh);
    }

    if (withFlowers) {
      const fb = new THREE.Mesh(fboxGeo, flowerBoxMat);
      fb.position.set(wx, wy - 9.2, wz + side * 1.8);
      group.add(fb);
      const fl = new THREE.Mesh(flowerGeo, flowerRedMat);
      fl.position.set(wx, wy - 7.3, wz + side * 1.8);
      group.add(fl);
    }
  };

  for (const side of [1, -1] as const) {
    // OG Fenster (3 Stück)
    for (const dx of [-36, 0, 36]) {
      addHouseWindow(dx, 59, side * 41, side, dx === 0 || (side === 1 && dx === -36), side === 1);
    }
    // EG Fenster (links & rechts der Tür)
    for (const dx of [-36, 36]) {
      addHouseWindow(dx, 26, side * 39, side, side === 1 && dx === 36, false);
    }
  }

  // Giebelfenster (links & rechts)
  const gableFrameGeo = getSharedBoxGeometry(1.4, 13, 11);
  const gableGlassGeo = getSharedBoxGeometry(1.6, 10.6, 8.8);
  for (const sx of [-1, 1]) {
    const gf = new THREE.Mesh(gableFrameGeo, frameMat);
    gf.position.set(sx * 53.2, 90, 0);
    group.add(gf);
    const gg = new THREE.Mesh(gableGlassGeo, glassWarmMat);
    gg.position.set(sx * 53.4, 90, 0);
    group.add(gg);
  }

  // 6. Haustür mit Steintreppe (Vorderseite +Z)
  const doorFrame = new THREE.Mesh(getSharedBoxGeometry(16, 25, 1.6), beamMat);
  doorFrame.position.set(0, 21.5, 39.2);
  group.add(doorFrame);
  const door = new THREE.Mesh(getSharedBoxGeometry(13, 22.5, 1.8), doorMat);
  door.position.set(0, 20.5, 39.5);
  group.add(door);
  const step = new THREE.Mesh(getSharedBoxGeometry(20, 3.2, 6), stoneMat);
  step.position.set(0, 7.6, 42);
  group.add(step);
}

/** Vereinfachte Low-Poly Variante des Fachwerkhauses (LOD >= 3500 mm) */
export function buildTownHouseLowPoly3D(group: THREE.Group): void {
  const stoneMat = getSharedMaterial(0x64748b, 0.9);
  const plasterMat = getSharedMaterial(0xf3ead8, 0.82);
  const roofMat = getSharedMaterial(0x991b1b, 0.68);

  const plinth = new THREE.Mesh(getSharedBoxGeometry(108, 9, 86), stoneMat);
  plinth.position.set(0, 4.5, 0);
  group.add(plinth);

  const body = new THREE.Mesh(getSharedBoxGeometry(106, 68, 82), plasterMat);
  body.position.set(0, 43, 0);
  group.add(body);

  const roof = new THREE.Mesh(getSharedTownHouseRoofGeo(), roofMat);
  group.add(roof);
}
