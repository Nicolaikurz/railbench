import * as THREE from "three";

/** 1. BAHNHOF NEUSTADT (320 x 140 x 120 mm) */
export function buildStation3D(group: THREE.Group): void {
  // Landschaftsplatte / Vorplatz & Grünfläche um den Bahnhof
  const grassMat = new THREE.MeshStandardMaterial({ color: 0x4d7c0f, roughness: 0.88 });
  const grassBase = new THREE.Mesh(new THREE.BoxGeometry(380, 3, 200), grassMat);
  grassBase.position.set(0, 1.5, 5);
  group.add(grassBase);

  // Weg / Vorplatz zum Bahnhofs-Eingang
  const pathMat = new THREE.MeshStandardMaterial({ color: 0xd4b996, roughness: 0.75 });
  const curbMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.8 });

  // Hauptweg / Fußweg von außen zum Haupteingang
  const mainPath = new THREE.Mesh(new THREE.BoxGeometry(60, 1, 95), pathMat);
  mainPath.position.set(-20, 3.2, -55);
  group.add(mainPath);

  // Randsteine des Weges
  const curbL = new THREE.Mesh(new THREE.BoxGeometry(3, 2, 95), curbMat);
  curbL.position.set(-51, 3.5, -55);
  group.add(curbL);

  const curbR = new THREE.Mesh(new THREE.BoxGeometry(3, 2, 95), curbMat);
  curbR.position.set(11, 3.5, -55);
  group.add(curbR);

  // Blumenbeet & Büsche am Weg
  const bushMat1 = new THREE.MeshStandardMaterial({ color: 0x166534, roughness: 0.9 });
  const bushMat2 = new THREE.MeshStandardMaterial({ color: 0x15803d, roughness: 0.9 });
  const bushOffsets = [-70, -85, 30, 45, 60];
  for (const bx of bushOffsets) {
    const bush = new THREE.Mesh(new THREE.DodecahedronGeometry(8, 1), bx > 0 ? bushMat1 : bushMat2);
    bush.position.set(bx, 8, -55);
    group.add(bush);
  }

  // Bäume am Vorplatz
  const treeTrunk = new THREE.Mesh(new THREE.CylinderGeometry(2.5, 4, 30, 8), new THREE.MeshStandardMaterial({ color: 0x451a03 }));
  treeTrunk.position.set(-135, 15, -60);
  group.add(treeTrunk);
  const treeCrown = new THREE.Mesh(new THREE.DodecahedronGeometry(22, 1), new THREE.MeshStandardMaterial({ color: 0x15803d }));
  treeCrown.position.set(-135, 42, -60);
  group.add(treeCrown);

  // Fundament des Gebäudes
  const baseMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.9 });
  const base = new THREE.Mesh(new THREE.BoxGeometry(320, 4, 140), baseMat);
  base.position.set(0, 3.5, 0);
  group.add(base);

  // Hauptgebäude (Backstein / Sandstein)
  const wallMat = new THREE.MeshStandardMaterial({ color: 0xc87d55, roughness: 0.75 });
  const mainBuilding = new THREE.Mesh(new THREE.BoxGeometry(200, 70, 90), wallMat);
  mainBuilding.position.set(-20, 40.5, -10);
  group.add(mainBuilding);

  // Giebeldach Hauptgebäude
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x991b1b, roughness: 0.65 });
  const roofShape = new THREE.Shape();
  roofShape.moveTo(-50, 0);
  roofShape.lineTo(0, 35);
  roofShape.lineTo(50, 0);
  roofShape.closePath();
  const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 216, bevelEnabled: false });
  roofGeo.rotateY(Math.PI / 2);
  roofGeo.translate(-128, 75.5, -10);
  const roof = new THREE.Mesh(roofGeo, roofMat);
  group.add(roof);

  // Uhrturm
  const tower = new THREE.Mesh(new THREE.BoxGeometry(45, 100, 45), wallMat);
  tower.position.set(-20, 55.5, -10);
  group.add(tower);

  // Uhrturm-Spitzdach
  const spireGeo = new THREE.ConeGeometry(32, 38, 4);
  spireGeo.rotateY(Math.PI / 4);
  const spireMat = new THREE.MeshStandardMaterial({ color: 0x1e3a5f, roughness: 0.4, metalness: 0.3 });
  const spire = new THREE.Mesh(spireGeo, spireMat);
  spire.position.set(-20, 124.5, -10);
  group.add(spire);

  // Bahnsteig-Vordach mit Stützen (Track side)
  const canopyMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.4 });
  const canopy = new THREE.Mesh(new THREE.BoxGeometry(280, 3, 40), canopyMat);
  canopy.position.set(0, 43.5, 45);
  group.add(canopy);

  // Vordach-Pfeiler
  const pillarMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.7 });
  for (let x = -120; x <= 120; x += 60) {
    const pillar = new THREE.Mesh(new THREE.CylinderGeometry(2, 2, 40, 8), pillarMat);
    pillar.position.set(x, 23.5, 58);
    group.add(pillar);
  }

 // Fenster & Eingänge auf beiden Seiten (Gleisseite +Z, Vorplatzseite -Z)
  const winMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.1, metalness: 0.9 });
  const frameMat = new THREE.MeshStandardMaterial({ color: 0x3f2a1d, roughness: 0.7 });
  const doorMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });

  const entranceX = -20;                  // Eingang mittig unter dem Uhrturm
  const windowOffsets = [-90, -60, -30, 30, 60, 90]; // symmetrisch um den Eingang

  for (const side of [1, -1]) {
    const wallZ = -10 + side * 45;        // Wandfläche (Gebäude: Mitte z=-10, Tiefe 90)
    const faceZ = wallZ + side * 0.5;     // Fenster bündig an der Wand

    // Fenster
    for (const dx of windowOffsets) {
      const win = new THREE.Mesh(new THREE.BoxGeometry(14, 20, 1), winMat);
      win.position.set(entranceX + dx, 49.5, faceZ);
      group.add(win);
    }

    // Türrahmen
    const frame = new THREE.Mesh(new THREE.BoxGeometry(20, 34, 1), frameMat);
    frame.position.set(entranceX, 22.5, faceZ);
    group.add(frame);

    // Zwei Türflügel
    for (const dx of [-4, 4]) {
      const leaf = new THREE.Mesh(new THREE.BoxGeometry(7.6, 30, 1), doorMat);
      leaf.position.set(entranceX + dx, 20.5, wallZ + side * 1.5);
      group.add(leaf);
    }

    // Oberlicht über der Tür
    const transom = new THREE.Mesh(new THREE.BoxGeometry(16, 3, 1), winMat);
    transom.position.set(entranceX, 37.5, wallZ + side * 1.5);
    group.add(transom);

    // Eingangsstufe
    const step = new THREE.Mesh(new THREE.BoxGeometry(26, 1.5, 6), curbMat);
    step.position.set(entranceX, 6.25, wallZ + side * 3);
    group.add(step);
  }
}
