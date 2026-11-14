import * as THREE from "three";

/**
 * BEKOHLUNGSANLAGE FÜR DAMPFLOKOMOTIVEN (180 x 75 x 125 mm)
 * Detailreiches Dampflok-BW-Modell mit Beton-Kohlenbansen, Steinkohle-Halde,
 * Stahl-Hochbunker mit beweglicher Kohlenschütte, Drehkran mit Zweischalengreifer
 * sowie seitlichem Gelenk-Wasserkran.
 */
export function buildCoalingStation3D(group: THREE.Group): void {
  const concreteMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.9, metalness: 0.05 });
  const darkConcreteMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.92, metalness: 0.05 });
  const coalMat = new THREE.MeshStandardMaterial({
    color: 0x141417,
    roughness: 0.65,
    metalness: 0.25,
    flatShading: true,
  });
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.45, metalness: 0.65 });
  const darkSteelMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.75 });
  const craneCabMat = new THREE.MeshStandardMaterial({ color: 0x1e3a5f, roughness: 0.5, metalness: 0.4 });
  const rustMat = new THREE.MeshStandardMaterial({ color: 0x7c2d12, roughness: 0.85, metalness: 0.2 });
  const warningMat = new THREE.MeshStandardMaterial({ color: 0xea580c, roughness: 0.5, metalness: 0.3 });
  const glassMat = new THREE.MeshStandardMaterial({
    color: 0xbae6fd,
    roughness: 0.15,
    metalness: 0.8,
    emissive: 0xfef08a,
    emissiveIntensity: 0.25,
  });

  // 1. Fundamentplatte (180 x 75 x 4 mm)
  const basePlate = new THREE.Mesh(new THREE.BoxGeometry(180, 4, 75), darkConcreteMat);
  basePlate.position.set(0, 2, 0);
  group.add(basePlate);

  // 2. Großer Kohlenbansen (Beton-Einfassung links/Mitte, X = -82 bis +22, Z = -32 bis +18)
  const bansenMinX = -82;
  const bansenMaxX = 22;
  const bansenW = bansenMaxX - bansenMinX; // 104 mm
  const bansenCX = (bansenMinX + bansenMaxX) / 2; // -30 mm
  const bansenCZ = -7;
  const bansenD = 50;
  const wallH = 22;
  const wallT = 3.5;

  // Rückwand (-Z), Vorderwand (+Z) und Seitenwände
  const backWall = new THREE.Mesh(new THREE.BoxGeometry(bansenW, wallH, wallT), concreteMat);
  backWall.position.set(bansenCX, 4 + wallH / 2, bansenCZ - bansenD / 2 + wallT / 2);
  group.add(backWall);

  const frontWall = new THREE.Mesh(new THREE.BoxGeometry(bansenW, wallH * 0.82, wallT), concreteMat);
  frontWall.position.set(bansenCX, 4 + (wallH * 0.82) / 2, bansenCZ + bansenD / 2 - wallT / 2);
  group.add(frontWall);

  const leftWall = new THREE.Mesh(new THREE.BoxGeometry(wallT, wallH, bansenD), concreteMat);
  leftWall.position.set(bansenMinX + wallT / 2, 4 + wallH / 2, bansenCZ);
  group.add(leftWall);

  const rightWall = new THREE.Mesh(new THREE.BoxGeometry(wallT, wallH, bansenD), concreteMat);
  rightWall.position.set(bansenMaxX - wallT / 2, 4 + wallH / 2, bansenCZ);
  group.add(rightWall);

  // Beton-Strebepfeiler außen am Kohlenbansen
  for (let i = 0; i <= 5; i++) {
    const px = bansenMinX + 4 + i * ((bansenW - 8) / 5);
    const ribBack = new THREE.Mesh(new THREE.BoxGeometry(2.8, wallH + 1, 3.5), darkConcreteMat);
    ribBack.position.set(px, 4 + wallH / 2, bansenCZ - bansenD / 2 - 1);
    group.add(ribBack);

    const ribFront = new THREE.Mesh(new THREE.BoxGeometry(2.8, wallH * 0.82 + 1, 3.5), darkConcreteMat);
    ribFront.position.set(px, 4 + (wallH * 0.82) / 2, bansenCZ + bansenD / 2 + 1);
    group.add(ribFront);
  }

  // 3. Steinkohle-Halde im Bansen (Grundfüllung + facettierte Kohlehügel)
  const coalBed = new THREE.Mesh(new THREE.BoxGeometry(bansenW - 6, 14, bansenD - 6), coalMat);
  coalBed.position.set(bansenCX, 4 + 7, bansenCZ);
  group.add(coalBed);

  const coalMounds: Array<{ x: number; z: number; r: number; h: number }> = [
    { x: -64, z: -10, r: 22, h: 18 },
    { x: -42, z: -4, r: 25, h: 22 },
    { x: -18, z: -9, r: 23, h: 20 },
    { x: 2, z: -5, r: 20, h: 17 },
    { x: -52, z: 6, r: 19, h: 15 },
    { x: -26, z: 7, r: 21, h: 16 },
  ];
  for (const m of coalMounds) {
    const cone = new THREE.Mesh(new THREE.ConeGeometry(m.r, m.h, 7), coalMat);
    cone.position.set(m.x, 16 + m.h / 2, m.z);
    cone.rotation.y = (m.x * 0.17) % Math.PI;
    group.add(cone);
  }

  // 4. Stahl-Hochbunker / Wiegebunker (rechts bei X = +52 mm, Z = -2 mm)
  const towerCX = 52;
  const towerCZ = -2;
  const towerSpanX = 36;
  const towerSpanZ = 34;
  const legH = 56;

  // 4 Stahlstützen mit Betonsockeln
  for (const sx of [-1, 1]) {
    for (const sz of [-1, 1]) {
      const lx = towerCX + sx * (towerSpanX / 2);
      const lz = towerCZ + sz * (towerSpanZ / 2);

      const plinth = new THREE.Mesh(new THREE.BoxGeometry(6, 6, 6), concreteMat);
      plinth.position.set(lx, 7, lz);
      group.add(plinth);

      const leg = new THREE.Mesh(new THREE.BoxGeometry(3.2, legH, 3.2), steelMat);
      leg.position.set(lx, 10 + legH / 2, lz);
      group.add(leg);
    }
  }

  // Querriegel & Fachwerkstreben am Hochbunker-Gerüst
  for (const yLevel of [28, 48, 64]) {
    const beamX1 = new THREE.Mesh(new THREE.BoxGeometry(towerSpanX + 4, 2.5, 2.5), steelMat);
    beamX1.position.set(towerCX, yLevel, towerCZ - towerSpanZ / 2);
    group.add(beamX1);

    const beamX2 = new THREE.Mesh(new THREE.BoxGeometry(towerSpanX + 4, 2.5, 2.5), steelMat);
    beamX2.position.set(towerCX, yLevel, towerCZ + towerSpanZ / 2);
    group.add(beamX2);

    const beamZ1 = new THREE.Mesh(new THREE.BoxGeometry(2.5, 2.5, towerSpanZ + 4), steelMat);
    beamZ1.position.set(towerCX - towerSpanX / 2, yLevel, towerCZ);
    group.add(beamZ1);

    const beamZ2 = new THREE.Mesh(new THREE.BoxGeometry(2.5, 2.5, towerSpanZ + 4), steelMat);
    beamZ2.position.set(towerCX + towerSpanX / 2, yLevel, towerCZ);
    group.add(beamZ2);
  }

  // Trichter-Unterteil (Pyramidenstumpf nach unten)
  const funnel = new THREE.Mesh(new THREE.CylinderGeometry(24, 9, 20, 4), steelMat);
  funnel.rotation.y = Math.PI / 4;
  funnel.position.set(towerCX, 56, towerCZ);
  group.add(funnel);

  // Haupt-Kohlenbunker-Aufsatz (oben offen mit Kohlefüllung)
  const hopperBox = new THREE.Mesh(new THREE.BoxGeometry(38, 22, 36), steelMat);
  hopperBox.position.set(towerCX, 76, towerCZ);
  group.add(hopperBox);

  const hopperCoal = new THREE.Mesh(new THREE.ConeGeometry(21, 14, 6), coalMat);
  hopperCoal.position.set(towerCX, 90, towerCZ);
  group.add(hopperCoal);

  // Schwenkbare Kohlenschütte zur Gleisseite (+Z)
  const chuteGroup = new THREE.Group();
  chuteGroup.position.set(towerCX, 50, towerCZ + 14);
  chuteGroup.rotation.x = 0.55; // Nach vorn unten zum Gleis geneigt
  const chuteBed = new THREE.Mesh(new THREE.BoxGeometry(12, 2, 26), darkSteelMat);
  chuteBed.position.set(0, 0, 11);
  chuteGroup.add(chuteBed);
  const chuteSideL = new THREE.Mesh(new THREE.BoxGeometry(1.5, 5, 26), steelMat);
  chuteSideL.position.set(-5.5, 2, 11);
  chuteGroup.add(chuteSideL);
  const chuteSideR = new THREE.Mesh(new THREE.BoxGeometry(1.5, 5, 26), steelMat);
  chuteSideR.position.set(5.5, 2, 11);
  chuteGroup.add(chuteSideR);
  group.add(chuteGroup);

  // Bedienbühne & kleines Wärterhäuschen am Hochbunker
  const walkway = new THREE.Mesh(new THREE.BoxGeometry(46, 2, 44), darkSteelMat);
  walkway.position.set(towerCX, 65, towerCZ);
  group.add(walkway);

  // 5. Drehkran mit Gittermast-Ausleger & Zweischalengreifer (bei X = +12 mm, Z = -12 mm)
  const craneX = 12;
  const craneZ = -12;

  // Massiver runder Kran-Turmsockel
  const cranePedestal = new THREE.Mesh(new THREE.CylinderGeometry(11, 13, 42, 12), concreteMat);
  cranePedestal.position.set(craneX, 4 + 21, craneZ);
  group.add(cranePedestal);

  const slewingRing = new THREE.Mesh(new THREE.CylinderGeometry(12, 12, 3, 16), darkSteelMat);
  slewingRing.position.set(craneX, 47, craneZ);
  group.add(slewingRing);

  // Drehbarer Kranaufbau (zeigt schräg zwischen Bansen und Hochbunker)
  const craneSuper = new THREE.Group();
  craneSuper.position.set(craneX, 48.5, craneZ);
  craneSuper.rotation.y = -0.45;

  const cabBody = new THREE.Mesh(new THREE.BoxGeometry(22, 18, 16), craneCabMat);
  cabBody.position.set(-2, 9, 0);
  craneSuper.add(cabBody);

  const cabRoof = new THREE.Mesh(new THREE.BoxGeometry(24, 2, 18), darkSteelMat);
  cabRoof.position.set(-2, 19, 0);
  craneSuper.add(cabRoof);

  const cabWindow = new THREE.Mesh(new THREE.BoxGeometry(2, 8, 12), glassMat);
  cabWindow.position.set(8.5, 11, 0);
  craneSuper.add(cabWindow);

  // Gegengewicht am Heck
  const counterweight = new THREE.Mesh(new THREE.BoxGeometry(8, 12, 14), darkConcreteMat);
  counterweight.position.set(-15, 7, 0);
  craneSuper.add(counterweight);

  // Schräger Gitter-Ausleger (Länge 72 mm, ragt bis auf ~122 mm Höhe)
  const boomLen = 72;
  const boomAngle = 0.72; // ~41° nach oben
  const boomGroup = new THREE.Group();
  boomGroup.position.set(6, 12, 0);
  boomGroup.rotation.z = boomAngle;

  for (const bz of [-3, 3]) {
    const chord = new THREE.Mesh(new THREE.CylinderGeometry(1.1, 1.4, boomLen, 6), steelMat);
    chord.rotation.z = -Math.PI / 2;
    chord.position.set(boomLen / 2, 0, bz);
    boomGroup.add(chord);
  }
  for (let bStep = 6; bStep < boomLen; bStep += 10) {
    const strut = new THREE.Mesh(new THREE.BoxGeometry(1.2, 3.2, 6.5), steelMat);
    strut.position.set(bStep, 0, 0);
    boomGroup.add(strut);
  }
  craneSuper.add(boomGroup);

  // Auslegerspitze in lokalen Kran-Koordinaten berechnen
  const tipX = 6 + Math.cos(boomAngle) * boomLen;
  const tipY = 12 + Math.sin(boomAngle) * boomLen;

  // Umlenkrolle an der Auslegerspitze
  const pulley = new THREE.Mesh(new THREE.CylinderGeometry(3, 3, 2.5, 12), warningMat);
  pulley.rotation.x = Math.PI / 2;
  pulley.position.set(tipX, tipY, 0);
  craneSuper.add(pulley);

  // Stahlseil & Zweischalengreifer (hängt von der Auslegerspitze herab)
  const cableLen = 42;
  const cable = new THREE.Mesh(new THREE.CylinderGeometry(0.45, 0.45, cableLen, 6), darkSteelMat);
  cable.position.set(tipX, tipY - cableLen / 2, 0);
  craneSuper.add(cable);

  const grabY = tipY - cableLen;
  const grabHead = new THREE.Mesh(new THREE.BoxGeometry(5, 6, 5), darkSteelMat);
  grabHead.position.set(tipX, grabY, 0);
  craneSuper.add(grabHead);

  const jawL = new THREE.Mesh(new THREE.BoxGeometry(6, 7, 8), rustMat);
  jawL.position.set(tipX - 3, grabY - 5, 0);
  jawL.rotation.z = 0.25;
  craneSuper.add(jawL);

  const jawR = new THREE.Mesh(new THREE.BoxGeometry(6, 7, 8), rustMat);
  jawR.position.set(tipX + 3, grabY - 5, 0);
  jawR.rotation.z = -0.25;
  craneSuper.add(jawR);

  const grabCoal = new THREE.Mesh(new THREE.SphereGeometry(4.5, 6, 5), coalMat);
  grabCoal.position.set(tipX, grabY - 4, 0);
  craneSuper.add(grabCoal);

  group.add(craneSuper);

  // 6. Gelenk-Wasserkran für Dampfloks (links vorn bei X = -72 mm, Z = +24 mm)
  const wcX = -72;
  const wcZ = 24;

  const drainGrate = new THREE.Mesh(new THREE.BoxGeometry(14, 1, 14), darkSteelMat);
  drainGrate.position.set(wcX + 18, 4.5, wcZ);
  group.add(drainGrate);

  const wcBase = new THREE.Mesh(new THREE.CylinderGeometry(3.8, 5, 8, 10), darkSteelMat);
  wcBase.position.set(wcX, 8, wcZ);
  group.add(wcBase);

  const wcColumn = new THREE.Mesh(new THREE.CylinderGeometry(2.4, 2.8, 46, 10), darkSteelMat);
  wcColumn.position.set(wcX, 31, wcZ);
  group.add(wcColumn);

  const wcArm = new THREE.Mesh(new THREE.CylinderGeometry(1.8, 1.8, 28, 8), darkSteelMat);
  wcArm.rotation.z = Math.PI / 2;
  wcArm.position.set(wcX + 8, 53, wcZ);
  group.add(wcArm);

  const wcSpout = new THREE.Mesh(new THREE.CylinderGeometry(1.6, 2.0, 12, 8), darkSteelMat);
  wcSpout.position.set(wcX + 20, 48, wcZ);
  group.add(wcSpout);

  const wcWeight = new THREE.Mesh(new THREE.SphereGeometry(3.8, 10, 10), warningMat);
  wcWeight.position.set(wcX - 7, 55, wcZ);
  group.add(wcWeight);
}
