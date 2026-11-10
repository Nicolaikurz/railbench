import * as THREE from "three";

/**
 * Industrie- & Fabrikhalle (Industrial Factory Hall)
 * Abmessungen BxTxH: 260 x 160 x 110 mm
 *
 * Merkmale:
 * - Großzügige Industrie-Bodenplatte mit Laderampe & Zufahrt
 * - Authentisches Sheddach (Sägezahndach) mit nach Norden geneigten Glas-Lichtbändern
 * - 2 große Industrie-Sektionaltore (Rolltore) für LKW und Güterverladung
 * - Zylindrisches Hochsilo mit Steigleiter und Rohrleitungen
 * - Industrie-Schornstein / Abluftkamin mit Blechabdeckung
 * - Überdachte Laderampe mit Rammschutz-Puffern
 */
export function buildIndustryBuilding3D(group: THREE.Group): void {
  // Materialien
  const groundMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.9 });
  const rampMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.85 });
  const brickWallMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.88 });
  const metalWallMat = new THREE.MeshStandardMaterial({ color: 0x52525b, roughness: 0.6, metalness: 0.4 });
  const trimMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.6 });
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.15, metalness: 0.85 });
  const doorMat = new THREE.MeshStandardMaterial({ color: 0x0369a1, roughness: 0.4, metalness: 0.5 });
  const siloMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.3, metalness: 0.8 });
  const pipeMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.35, metalness: 0.75 });
  const hazardMat = new THREE.MeshStandardMaterial({ color: 0xeab308, roughness: 0.5 });
  const chimneyMat = new THREE.MeshStandardMaterial({ color: 0x1c1917, roughness: 0.7 });

  // 1. Fundamentplatte & Vorplatz
  const basePlaza = new THREE.Mesh(new THREE.BoxGeometry(290, 4, 190), groundMat);
  basePlaza.position.set(5, 2, 0);
  group.add(basePlaza);

  // 2. Erhöhte Laderampe an der Front (LKW & Güterwagen-Andockstelle)
  const rampWidth = 140;
  const rampDepth = 30;
  const rampHeight = 10;
  const ramp = new THREE.Mesh(new THREE.BoxGeometry(rampWidth, rampHeight, rampDepth), rampMat);
  ramp.position.set(-30, 4 + rampHeight / 2, 70);
  group.add(ramp);

  // Auffahrkeil / Schräge für Gabelstapler an der Rampe
  const rampWedgeShape = new THREE.Shape();
  rampWedgeShape.moveTo(0, 0);
  rampWedgeShape.lineTo(24, 0);
  rampWedgeShape.lineTo(0, rampHeight);
  rampWedgeShape.closePath();
  const rampWedgeGeo = new THREE.ExtrudeGeometry(rampWedgeShape, { depth: rampDepth, bevelEnabled: false });
  rampWedgeGeo.rotateY(Math.PI / 2);
  const rampWedge = new THREE.Mesh(rampWedgeGeo, rampMat);
  rampWedge.position.set(rampWidth / 2 - 30, 4, 70 - rampDepth / 2);
  group.add(rampWedge);

  // Rammschutzpuffer an der Rampe
  for (const bx of [-85, -60, -35, -10, 15]) {
    const bumper = new THREE.Mesh(new THREE.BoxGeometry(6, 6, 3), pipeMat);
    bumper.position.set(bx, 4 + rampHeight - 4, 85 + 1.5);
    group.add(bumper);
  }

  // 3. Hallenbaukörper (Zweifarbig: Sockel aus Klinker, Aufbau aus Stahlsandwich-Paneelen)
  const hallWidth = 210;
  const hallDepth = 130;
  const hallWallHeight = 55;

  // Klinkersockel
  const baseWall = new THREE.Mesh(new THREE.BoxGeometry(hallWidth, 16, hallDepth), brickWallMat);
  baseWall.position.set(-15, 4 + 8, 0);
  group.add(baseWall);

  // Oberer Wandaufbau (Profilblechfassade)
  const topWall = new THREE.Mesh(new THREE.BoxGeometry(hallWidth, hallWallHeight - 16, hallDepth), metalWallMat);
  topWall.position.set(-15, 4 + 16 + (hallWallHeight - 16) / 2, 0);
  group.add(topWall);

  // Fassadensimse & Stützen
  for (let x = -hallWidth / 2 - 15; x <= hallWidth / 2 - 15; x += 35) {
    const col = new THREE.Mesh(new THREE.BoxGeometry(3, hallWallHeight, 3), trimMat);
    col.position.set(x, 4 + hallWallHeight / 2, hallDepth / 2);
    group.add(col);
  }

  // Vordach über der Laderampe
  const canopy = new THREE.Mesh(new THREE.BoxGeometry(rampWidth + 10, 2.5, 24), trimMat);
  canopy.position.set(-30, 4 + 38, 72);
  group.add(canopy);

  // Gelbe Warnmarkierung am Vordach
  const hazardStrip = new THREE.Mesh(new THREE.BoxGeometry(rampWidth + 10, 1.2, 0.5), hazardMat);
  hazardStrip.position.set(-30, 4 + 37.4, 84.2);
  group.add(hazardStrip);

  // 4. Zwei Industrie-Sektionaltore an der Rampe
  for (const gx of [-60, -5]) {
    // Rolltor (blau gerillt)
    const gate = new THREE.Mesh(new THREE.BoxGeometry(30, 26, 1.5), doorMat);
    gate.position.set(gx, 4 + rampHeight + 13, 65.5);
    group.add(gate);

    // Torrahmen
    const frame = new THREE.Mesh(new THREE.BoxGeometry(34, 28, 2), trimMat);
    frame.position.set(gx, 4 + rampHeight + 14, 65);
    group.add(frame);
  }

  // Ebenerdiges Werkstor an der Rückseite
  const rearGate = new THREE.Mesh(new THREE.BoxGeometry(28, 34, 1.5), doorMat);
  rearGate.position.set(-15, 4 + 17, -65.5);
  group.add(rearGate);

  // Fensterbänder an den Seitenwänden
  for (let fz = -40; fz <= 40; fz += 28) {
    const sideWin = new THREE.Mesh(new THREE.BoxGeometry(1.5, 14, 20), glassMat);
    sideWin.position.set(-hallWidth / 2 - 15 - 0.5, 4 + 32, fz);
    group.add(sideWin);
  }

  // 5. Authentisches Sheddach (Sägezahndach) mit 4 Segmenten
  const numSheds = 4;
  const shedLen = hallWidth;
  const shedWidth = hallDepth / numSheds; // 130 / 4 = 32.5 mm pro Zacke
  const shedHeight = 22;

  for (let i = 0; i < numSheds; i++) {
    const shedZ = -hallDepth / 2 + shedWidth * i;

    // Sägezahn-Profil (steiles Glasfenster nach Norden/hinten, schräge Dachfläche nach Süden/vorn)
    const shedShape = new THREE.Shape();
    shedShape.moveTo(0, 0);
    shedShape.lineTo(shedWidth, 0);
    shedShape.lineTo(0, shedHeight);
    shedShape.closePath();

    const shedGeo = new THREE.ExtrudeGeometry(shedShape, { depth: shedLen, bevelEnabled: false });
    shedGeo.rotateY(Math.PI / 2);
    shedGeo.translate(-hallWidth / 2 - 15, 4 + hallWallHeight, shedZ);

    const shedMesh = new THREE.Mesh(shedGeo, trimMat);
    group.add(shedMesh);

    // Großes Oberlicht-Fenster an der steilen Flanke jedes Sägezahns
    const winGeo = new THREE.BoxGeometry(shedLen - 8, shedHeight - 3, 1.5);
    const shedWin = new THREE.Mesh(winGeo, glassMat);
    shedWin.position.set(-15, 4 + hallWallHeight + shedHeight / 2, shedZ + 0.8);
    group.add(shedWin);
  }

  // 6. Silo-Turm (Lagertank für Rohstoffe neben der Halle)
  const siloRadius = 18;
  const siloHeight = 85;
  const siloX = 110;
  const siloZ = 0;

  // Zylindrischer Silokörper
  const siloBody = new THREE.Mesh(new THREE.CylinderGeometry(siloRadius, siloRadius, siloHeight, 20), siloMat);
  siloBody.position.set(siloX, 4 + siloHeight / 2, siloZ);
  group.add(siloBody);

  // Silodach (Kuppel / Kegel)
  const siloRoof = new THREE.Mesh(new THREE.ConeGeometry(siloRadius + 1.5, 12, 20), siloMat);
  siloRoof.position.set(siloX, 4 + siloHeight + 6, siloZ);
  group.add(siloRoof);

  // Trichtersockel unten
  const siloHopper = new THREE.Mesh(new THREE.CylinderGeometry(siloRadius, 6, 12, 16), trimMat);
  siloHopper.position.set(siloX, 4 + 6, siloZ);
  group.add(siloHopper);

  // 4 Stützbeine des Silos
  for (let a = 0; a < Math.PI * 2; a += Math.PI / 2) {
    const leg = new THREE.Mesh(new THREE.BoxGeometry(3, 18, 3), pipeMat);
    leg.position.set(siloX + (siloRadius - 2) * Math.cos(a), 4 + 9, siloZ + (siloRadius - 2) * Math.sin(a));
    group.add(leg);
  }

  // Steigleiter mit Rückenschutz am Silo
  const ladder = new THREE.Mesh(new THREE.BoxGeometry(4, siloHeight, 1.5), pipeMat);
  ladder.position.set(siloX, 4 + siloHeight / 2, siloZ + siloRadius + 1);
  group.add(ladder);

  // Rohrverbindung vom Silo in die Industriehalle
  const pipe = new THREE.Mesh(new THREE.CylinderGeometry(2.5, 2.5, 30, 8), pipeMat);
  pipe.rotation.z = Math.PI / 2;
  pipe.position.set(siloX - 15, 4 + 50, siloZ);
  group.add(pipe);

  // 7. Industrieller Schornstein / Abluftturm
  const chimHeight = 65;
  const chim = new THREE.Mesh(new THREE.CylinderGeometry(5, 7, chimHeight, 12), chimneyMat);
  chim.position.set(65, 4 + hallWallHeight + chimHeight / 2 - 10, -45);
  group.add(chim);

  const chimCap = new THREE.Mesh(new THREE.ConeGeometry(10, 6, 12), chimneyMat);
  chimCap.position.set(65, 4 + hallWallHeight + chimHeight - 7, -45);
  group.add(chimCap);
}
