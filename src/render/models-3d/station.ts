import * as THREE from "three";
import {
  getSharedBoxGeometry,
  getSharedCylinderGeometry,
  getSharedDodecahedronGeometry,
  getSharedMaterial,
} from "./primitive-cache";

let _sharedStationRoofGeo: THREE.ExtrudeGeometry | null = null;
let _sharedSpireGeo: THREE.ConeGeometry | null = null;

function getSharedStationRoofGeo(): THREE.ExtrudeGeometry {
  if (!_sharedStationRoofGeo) {
    const roofShape = new THREE.Shape();
    roofShape.moveTo(-50, 0);
    roofShape.lineTo(0, 35);
    roofShape.lineTo(50, 0);
    roofShape.closePath();
    _sharedStationRoofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 216, bevelEnabled: false });
    _sharedStationRoofGeo.rotateY(Math.PI / 2);
    _sharedStationRoofGeo.translate(-128, 75.5, -10);
    _sharedStationRoofGeo.userData.shared = true;
  }
  return _sharedStationRoofGeo;
}

function getSharedSpireGeo(): THREE.ConeGeometry {
  if (!_sharedSpireGeo) {
    _sharedSpireGeo = new THREE.ConeGeometry(32, 38, 4);
    _sharedSpireGeo.rotateY(Math.PI / 4);
    _sharedSpireGeo.userData.shared = true;
  }
  return _sharedSpireGeo;
}

/** 1. BAHNHOF NEUSTADT (320 x 140 x 120 mm) */
export function buildStation3D(group: THREE.Group): void {
  // Landschaftsplatte / Vorplatz & Grünfläche um den Bahnhof
  const grassMat = getSharedMaterial(0x4d7c0f, 0.88);
  const grassBase = new THREE.Mesh(getSharedBoxGeometry(380, 3, 200), grassMat);
  grassBase.position.set(0, 1.5, 5);
  group.add(grassBase);

  // Weg / Vorplatz zum Bahnhofs-Eingang
  const pathMat = getSharedMaterial(0xd4b996, 0.75);
  const curbMat = getSharedMaterial(0x64748b, 0.8);

  // Hauptweg / Fußweg von außen zum Haupteingang
  const mainPath = new THREE.Mesh(getSharedBoxGeometry(60, 1, 95), pathMat);
  mainPath.position.set(-20, 3.2, -55);
  group.add(mainPath);

  // Randsteine des Weges
  const curbGeo = getSharedBoxGeometry(3, 2, 95);
  const curbL = new THREE.Mesh(curbGeo, curbMat);
  curbL.position.set(-51, 3.5, -55);
  group.add(curbL);

  const curbR = new THREE.Mesh(curbGeo, curbMat);
  curbR.position.set(11, 3.5, -55);
  group.add(curbR);

  // Blumenbeet & Büsche am Weg
  const bushMat1 = getSharedMaterial(0x166534, 0.9);
  const bushMat2 = getSharedMaterial(0x15803d, 0.9);
  const bushGeo = getSharedDodecahedronGeometry(8, 1);
  const bushOffsets = [-70, -85, 30, 45, 60];
  for (const bx of bushOffsets) {
    const bush = new THREE.Mesh(bushGeo, bx > 0 ? bushMat1 : bushMat2);
    bush.position.set(bx, 8, -55);
    group.add(bush);
  }

  // Bäume am Vorplatz
  const treeTrunk = new THREE.Mesh(getSharedCylinderGeometry(2.5, 4, 30, 8), getSharedMaterial(0x451a03));
  treeTrunk.position.set(-135, 15, -60);
  group.add(treeTrunk);
  const treeCrown = new THREE.Mesh(getSharedDodecahedronGeometry(22, 1), getSharedMaterial(0x15803d));
  treeCrown.position.set(-135, 42, -60);
  group.add(treeCrown);

  // Fundament des Gebäudes
  const baseMat = getSharedMaterial(0x475569, 0.9);
  const base = new THREE.Mesh(getSharedBoxGeometry(320, 4, 140), baseMat);
  base.position.set(0, 3.5, 0);
  group.add(base);

  // Hauptgebäude (Backstein / Sandstein)
  const wallMat = getSharedMaterial(0xc87d55, 0.75);
  const mainBuilding = new THREE.Mesh(getSharedBoxGeometry(200, 70, 90), wallMat);
  mainBuilding.position.set(-20, 40.5, -10);
  group.add(mainBuilding);

  // Giebeldach Hauptgebäude
  const roofMat = getSharedMaterial(0x991b1b, 0.65);
  const roof = new THREE.Mesh(getSharedStationRoofGeo(), roofMat);
  group.add(roof);

  // Uhrturm
  const tower = new THREE.Mesh(getSharedBoxGeometry(45, 100, 45), wallMat);
  tower.position.set(-20, 55.5, -10);
  group.add(tower);

  // Uhrturm-Spitzdach
  const spireMat = getSharedMaterial(0x1e3a5f, 0.4, 0.3);
  const spire = new THREE.Mesh(getSharedSpireGeo(), spireMat);
  spire.position.set(-20, 124.5, -10);
  group.add(spire);

  // Bahnsteig-Vordach mit Stützen (Track side)
  const canopyMat = getSharedMaterial(0x334155, 0.5, 0.4);
  const canopy = new THREE.Mesh(getSharedBoxGeometry(280, 3, 40), canopyMat);
  canopy.position.set(0, 43.5, 45);
  group.add(canopy);

  // Vordach-Pfeiler
  const pillarMat = getSharedMaterial(0x1e293b, 0.4, 0.7);
  const pillarGeo = getSharedCylinderGeometry(2, 2, 40, 8);
  for (let x = -120; x <= 120; x += 60) {
    const pillar = new THREE.Mesh(pillarGeo, pillarMat);
    pillar.position.set(x, 23.5, 58);
    group.add(pillar);
  }

  // Fenster & Eingänge auf beiden Seiten (Gleisseite +Z, Vorplatzseite -Z)
  const winMat = getSharedMaterial(0x38bdf8, 0.1, 0.9);
  const frameMat = getSharedMaterial(0x3f2a1d, 0.7);
  const doorMat = getSharedMaterial(0x78350f, 0.8);

  const entranceX = -20;
  const windowOffsets = [-90, -60, -30, 30, 60, 90];

  const winGeo = getSharedBoxGeometry(14, 20, 1);
  const frameGeo = getSharedBoxGeometry(20, 34, 1);
  const leafGeo = getSharedBoxGeometry(7.6, 30, 1);
  const transomGeo = getSharedBoxGeometry(16, 3, 1);
  const stepGeo = getSharedBoxGeometry(26, 1.5, 6);

  for (const side of [1, -1]) {
    const wallZ = -10 + side * 45;
    const faceZ = wallZ + side * 0.5;

    // Fenster
    for (const dx of windowOffsets) {
      const win = new THREE.Mesh(winGeo, winMat);
      win.position.set(entranceX + dx, 49.5, faceZ);
      group.add(win);
    }

    // Türrahmen
    const frame = new THREE.Mesh(frameGeo, frameMat);
    frame.position.set(entranceX, 22.5, faceZ);
    group.add(frame);

    // Zwei Türflügel
    for (const dx of [-4, 4]) {
      const leaf = new THREE.Mesh(leafGeo, doorMat);
      leaf.position.set(entranceX + dx, 20.5, wallZ + side * 1.5);
      group.add(leaf);
    }

    // Oberlicht über der Tür
    const transom = new THREE.Mesh(transomGeo, winMat);
    transom.position.set(entranceX, 37.5, wallZ + side * 1.5);
    group.add(transom);

    // Eingangsstufe
    const step = new THREE.Mesh(stepGeo, curbMat);
    step.position.set(entranceX, 6.25, wallZ + side * 3);
    group.add(step);
  }
}

/** Vereinfachte Low-Poly Variante des Bahnhofs (LOD bei Distanz >= 3500 mm) */
export function buildStationLowPoly3D(group: THREE.Group): void {
  const grassMat = getSharedMaterial(0x4d7c0f, 0.88);
  const baseMat = getSharedMaterial(0x475569, 0.9);
  const wallMat = getSharedMaterial(0xc87d55, 0.75);
  const roofMat = getSharedMaterial(0x991b1b, 0.65);
  const spireMat = getSharedMaterial(0x1e3a5f, 0.4, 0.3);
  const canopyMat = getSharedMaterial(0x334155, 0.5, 0.4);

  // Vorplatz
  const grassBase = new THREE.Mesh(getSharedBoxGeometry(380, 3, 200), grassMat);
  grassBase.position.set(0, 1.5, 5);
  group.add(grassBase);

  // Fundament
  const base = new THREE.Mesh(getSharedBoxGeometry(320, 4, 140), baseMat);
  base.position.set(0, 3.5, 0);
  group.add(base);

  // Hauptgebäude
  const mainBuilding = new THREE.Mesh(getSharedBoxGeometry(200, 70, 90), wallMat);
  mainBuilding.position.set(-20, 40.5, -10);
  group.add(mainBuilding);

  // Giebeldach
  const roof = new THREE.Mesh(getSharedStationRoofGeo(), roofMat);
  group.add(roof);

  // Uhrturm
  const tower = new THREE.Mesh(getSharedBoxGeometry(45, 100, 45), wallMat);
  tower.position.set(-20, 55.5, -10);
  group.add(tower);

  // Uhrturm-Spitzdach
  const spire = new THREE.Mesh(getSharedSpireGeo(), spireMat);
  spire.position.set(-20, 124.5, -10);
  group.add(spire);

  // Bahnsteig-Vordach
  const canopy = new THREE.Mesh(getSharedBoxGeometry(280, 3, 40), canopyMat);
  canopy.position.set(0, 43.5, 45);
  group.add(canopy);
}
