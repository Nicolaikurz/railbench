import * as THREE from "three";

/**
 * BAHNSTEIG (Station Platform)
 * Abmessungen: 320 x 54 x 55 mm (H0-Standard)
 *
 * Ausstattungsmerkmale:
 * - Massiver Beton-Bahnsteigkörper (Höhe 10 mm) mit beidseitigen Schrägrampen an den Enden
 * - Dunkle Granit-Bahnsteigkanten und taktile weiße Sicherheits-Warnlinien
 * - Treppenabgang zur Personenunterführung mit Edelstahlgeländer und Glasscheiben
 * - Modernes Glas-Stahl-Schmetterlingsdach auf Doppel-T-Trägern und Y-Stützen
 * - Doppelseitige Zugzielanzeiger (ZZA) mit Fahrgastinformation
 * - Blaues Bahnhofs- und Gleisschild ("Neustadt / Gleis 1 & 2")
 * - Doppelseitige Bahnhofsuhr
 * - Klassische Holz-/Metall-Sitzbänke
 * - Zwei moderne Bahnsteigleuchten an den freien Enden
 * - SOS-/Info-Notrufsäule und Abfallbehälter
 */
export function buildPlatform3D(group: THREE.Group): void {
  // -------------------------------------------------------------------------
  // 1. MATERIALIEN
  // -------------------------------------------------------------------------
  // Bahnsteigbelag (Pflaster / Beton hellgrau)
  const deckMat = new THREE.MeshStandardMaterial({
    color: 0x94a3b8,
    roughness: 0.88,
  });

  // Bahnsteigkanten (Granit / Beton dunkelgrau)
  const curbMat = new THREE.MeshStandardMaterial({
    color: 0x475569,
    roughness: 0.9,
  });

  // Taktile Sicherheitslinie (Weiß)
  const safetyLineMat = new THREE.MeshStandardMaterial({
    color: 0xf8fafc,
    roughness: 0.5,
  });

  // Warnstreifen (Gelb)
  const yellowWarnMat = new THREE.MeshStandardMaterial({
    color: 0xeab308,
    roughness: 0.6,
  });

  // Stahlträger & Pfosten (DB Anthrazit / Dunkelgrau)
  const steelMat = new THREE.MeshStandardMaterial({
    color: 0x1e293b,
    roughness: 0.45,
    metalness: 0.65,
  });

  // Dachplatten (Wellblech / Zink dunkel)
  const roofSheetMat = new THREE.MeshStandardMaterial({
    color: 0x334155,
    roughness: 0.55,
    metalness: 0.4,
  });

  // Dachverglasung (Transluzentes Sicherheitsglas)
  const glassRoofMat = new THREE.MeshStandardMaterial({
    color: 0x7dd3fc,
    roughness: 0.15,
    metalness: 0.8,
    transparent: true,
    opacity: 0.65,
  });

  // Geländer-Glas (Treppenabgang)
  const railingGlassMat = new THREE.MeshStandardMaterial({
    color: 0x38bdf8,
    roughness: 0.1,
    metalness: 0.85,
    transparent: true,
    opacity: 0.45,
  });

  // Edelstahl / Chrom
  const metalMat = new THREE.MeshStandardMaterial({
    color: 0x94a3b8,
    roughness: 0.25,
    metalness: 0.85,
  });

  // Holzlatten (Sitzbänke)
  const woodMat = new THREE.MeshStandardMaterial({
    color: 0xb45309,
    roughness: 0.75,
  });

  // DB-Blau (Schilder & ZZA)
  const dbBlueMat = new THREE.MeshStandardMaterial({
    color: 0x0369a1,
    roughness: 0.5,
  });

  // ZZA-Display (Luminiszierendes Cyan/Orange)
  const displayMat = new THREE.MeshStandardMaterial({
    color: 0x38bdf8,
    roughness: 0.2,
    emissive: 0x0284c7,
    emissiveIntensity: 0.6,
  });

  // SOS-Rot
  const redMat = new THREE.MeshStandardMaterial({
    color: 0xdc2626,
    roughness: 0.5,
  });

  // Lampen-Leuchtkopf (Warmweiß)
  const lampGlowMat = new THREE.MeshStandardMaterial({
    color: 0xfef08a,
    emissive: 0xfef08a,
    emissiveIntensity: 0.7,
  });

  // -------------------------------------------------------------------------
  // 2. BAHNSTEIGKÖRPER & DECK
  // -------------------------------------------------------------------------
  // Zentraler Gehbelag (X: -140 bis +140, Z: -26 bis +26, H: 10 mm)
  const mainDeck = new THREE.Mesh(new THREE.BoxGeometry(280, 10, 52), deckMat);
  mainDeck.position.set(0, 5, 0);
  group.add(mainDeck);

  // Schräge End-Rampen für Wartungs-/Personalzugang
  // Linke Rampe (X: -160 bis -140)
  const lRampShape = new THREE.Shape();
  lRampShape.moveTo(-160, 1.2);
  lRampShape.lineTo(-140, 10);
  lRampShape.lineTo(-140, 0);
  lRampShape.lineTo(-160, 0);
  lRampShape.closePath();
  const lRampGeo = new THREE.ExtrudeGeometry(lRampShape, { depth: 52, bevelEnabled: false });
  lRampGeo.translate(0, 0, -26);
  const leftRamp = new THREE.Mesh(lRampGeo, deckMat);
  group.add(leftRamp);

  // Rechte Rampe (X: +140 bis +160)
  const rRampShape = new THREE.Shape();
  rRampShape.moveTo(140, 10);
  rRampShape.lineTo(160, 1.2);
  rRampShape.lineTo(160, 0);
  rRampShape.lineTo(140, 0);
  rRampShape.closePath();
  const rRampGeo = new THREE.ExtrudeGeometry(rRampShape, { depth: 52, bevelEnabled: false });
  rRampGeo.translate(0, 0, -26);
  const rightRamp = new THREE.Mesh(rRampGeo, deckMat);
  group.add(rightRamp);

  // Gelbe Warnmarkierungen an den Rampen-Übergängen
  for (const rx of [-139.5, 139.5]) {
    const rampStripe = new THREE.Mesh(new THREE.BoxGeometry(2, 0.4, 52), yellowWarnMat);
    rampStripe.position.set(rx, 10.2, 0);
    group.add(rampStripe);
  }

  // -------------------------------------------------------------------------
  // 3. BAHNSTEIGKANTEN & TAKTILE SICHERHEITSLINIEN
  // -------------------------------------------------------------------------
  // Bahnsteigkantensteine entlang beider Gleisseiten (Z = -26.5 und Z = +26.5)
  for (const zSide of [-26.5, 26.5]) {
    const curb = new THREE.Mesh(new THREE.BoxGeometry(320, 10.3, 2.2), curbMat);
    curb.position.set(0, 5.15, zSide);
    group.add(curb);
  }

  // Weiße taktile Sicherheitslinien (Blindenleitsystem / Gefahrenstreifen)
  for (const zSide of [-22.5, 22.5]) {
    const safetyLine = new THREE.Mesh(new THREE.BoxGeometry(300, 0.4, 1.8), safetyLineMat);
    safetyLine.position.set(0, 10.22, zSide);
    group.add(safetyLine);
  }

  // -------------------------------------------------------------------------
  // 4. TREPPENABGANG ZUR PERSONENUNTERFÜHRUNG (Center)
  // -------------------------------------------------------------------------
  const stairX = 0;
  const stairW = 34; // Länge X
  const stairD = 18; // Breite Z

  // Dunkle Schachtöffnung
  const stairWell = new THREE.Mesh(
    new THREE.BoxGeometry(stairW, 9.6, stairD),
    new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.95 })
  );
  stairWell.position.set(stairX, 5.1, 0);
  group.add(stairWell);

  // Stufen nach unten (von X = +14 absteigend nach X = -14)
  for (let s = 0; s < 7; s++) {
    const sx = 12 - s * 3.8;
    const sy = 9.2 - s * 1.3;
    const step = new THREE.Mesh(new THREE.BoxGeometry(3.6, 2, stairD - 2), curbMat);
    step.position.set(sx, sy, 0);
    group.add(step);
  }

  // Edelstahlgeländer um 3 Seiten des Schachts (Nord, Süd, Ost)
  // Pfosten
  const railingH = 10;
  const postPositions: Array<[number, number]> = [
    [-17, -9], [-6, -9], [6, -9], [17, -9],
    [-17, 9], [-6, 9], [6, 9], [17, 9],
    [-17, 0],
  ];
  for (const [px, pz] of postPositions) {
    const post = new THREE.Mesh(new THREE.CylinderGeometry(0.5, 0.5, railingH, 8), metalMat);
    post.position.set(px, 10 + railingH / 2, pz);
    group.add(post);
  }

  // Handläufe
  // Längs Geländer Nord & Süd
  for (const pz of [-9, 9]) {
    const rail = new THREE.Mesh(new THREE.BoxGeometry(stairW, 0.8, 0.8), metalMat);
    rail.position.set(stairX, 10 + railingH, pz);
    group.add(rail);

    // Glasscheibe unter Handlauf
    const glassPane = new THREE.Mesh(new THREE.BoxGeometry(stairW - 2, railingH - 2, 0.4), railingGlassMat);
    glassPane.position.set(stairX, 10 + (railingH - 1) / 2, pz);
    group.add(glassPane);
  }

  // Quer Geländer Stirnseite (West bei X = -17)
  const endRail = new THREE.Mesh(new THREE.BoxGeometry(0.8, 0.8, stairD), metalMat);
  endRail.position.set(-17, 10 + railingH, 0);
  group.add(endRail);

  const endGlass = new THREE.Mesh(new THREE.BoxGeometry(0.4, railingH - 2, stairD - 2), railingGlassMat);
  endGlass.position.set(-17, 10 + (railingH - 1) / 2, 0);
  group.add(endGlass);

  // -------------------------------------------------------------------------
  // 5. BAHNSTEIGDACH (Stahl-Glas-Konstruktion)
  // -------------------------------------------------------------------------
  const canopyLen = 210; // X: -105 bis +105
  const canopyWidth = 50; // Z: -25 bis +25
  const roofBaseY = 40;

  // Stützen an 4 Positionen entlang der Mittelachse Z = 0
  const pillarX = [-80, -32, 32, 80];

  for (const px of pillarX) {
    // Sockel
    const pBase = new THREE.Mesh(new THREE.BoxGeometry(5, 1.5, 5), steelMat);
    pBase.position.set(px, 10.75, 0);
    group.add(pBase);

    // Hauptstütze (Doppel-T-Form)
    const pMast = new THREE.Mesh(new THREE.BoxGeometry(3.5, 30, 3.5), steelMat);
    pMast.position.set(px, 10 + 15, 0);
    group.add(pMast);

    // Y-Ausleger / Kragträger nach links und rechts
    for (const zDir of [-1, 1]) {
      const arm = new THREE.Mesh(new THREE.BoxGeometry(2.5, 2.5, 22), steelMat);
      arm.position.set(px, roofBaseY - 1.2, zDir * 11);
      arm.rotation.x = zDir * 0.12;
      group.add(arm);

      // Schrägstrebe (Kopfwinkel)
      const strut = new THREE.Mesh(new THREE.CylinderGeometry(0.8, 0.8, 12, 6), steelMat);
      strut.position.set(px, roofBaseY - 5.5, zDir * 7);
      strut.rotation.x = -zDir * 0.65;
      group.add(strut);
    }
  }

  // Längsträger (First & Außenränder)
  const ridgeBeam = new THREE.Mesh(new THREE.BoxGeometry(canopyLen, 3, 3), steelMat);
  ridgeBeam.position.set(0, roofBaseY, 0);
  group.add(ridgeBeam);

  for (const zSide of [-24, 24]) {
    const edgeBeam = new THREE.Mesh(new THREE.BoxGeometry(canopyLen, 2.5, 2), steelMat);
    edgeBeam.position.set(0, roofBaseY + 2.5, zSide);
    group.add(edgeBeam);
  }

  // Dachflügel (Leichtes Schmetterlings-Gefälle nach innen zur Firstrinne)
  for (const zDir of [-1, 1]) {
    // Dunkle Zink-/Stahldachfläche
    const roofPanel = new THREE.Mesh(new THREE.BoxGeometry(canopyLen, 1.5, 16), roofSheetMat);
    roofPanel.position.set(0, roofBaseY + 1.2, zDir * 16);
    roofPanel.rotation.x = zDir * 0.12;
    group.add(roofPanel);

    // Längs-Glasstreifen (Lichtband für Tageslichteinfall)
    const glassStrip = new THREE.Mesh(new THREE.BoxGeometry(canopyLen - 10, 1.2, 7), glassRoofMat);
    glassStrip.position.set(0, roofBaseY + 0.6, zDir * 4.5);
    glassStrip.rotation.x = zDir * 0.12;
    group.add(glassStrip);
  }

  // Dachabschluss-Blenden an den Enden (X = -105 und X = +105)
  for (const xEnd of [-canopyLen / 2, canopyLen / 2]) {
    const fascia = new THREE.Mesh(new THREE.BoxGeometry(1.5, 4, canopyWidth), steelMat);
    fascia.position.set(xEnd, roofBaseY + 1.5, 0);
    group.add(fascia);
  }

  // -------------------------------------------------------------------------
  // 6. AUSSTATTUNG: ZUGZIELANZEIGER, SCHILDER & BAHNHOFSUHR
  // -------------------------------------------------------------------------
  // Doppelseitige Zugzielanzeiger (ZZA) unter dem Dach bei X = -45 und X = +45
  for (const zzaX of [-45, 45]) {
    // Gehäuse
    const zzaBox = new THREE.Mesh(new THREE.BoxGeometry(18, 6.5, 7), dbBlueMat);
    zzaBox.position.set(zzaX, roofBaseY - 5, 0);
    group.add(zzaBox);

    // Aufhängung
    for (const hx of [-6, 6]) {
      const rod = new THREE.Mesh(new THREE.CylinderGeometry(0.4, 0.4, 4, 6), steelMat);
      rod.position.set(zzaX + hx, roofBaseY - 1, 0);
      group.add(rod);
    }

    // Displays auf beiden Gleisseiten (Nord und Süd)
    for (const zDir of [-1, 1]) {
      const disp = new THREE.Mesh(new THREE.BoxGeometry(16, 4.5, 0.4), displayMat);
      disp.position.set(zzaX, roofBaseY - 5, zDir * 3.6);
      group.add(disp);
    }
  }

  // Großes Bahnhofs- und Gleisschild ("Neustadt / Gleis 1 - 2") mittig über dem Treppenabgang
  const signBoard = new THREE.Mesh(new THREE.BoxGeometry(32, 5.5, 1.6), dbBlueMat);
  signBoard.position.set(0, roofBaseY - 5.5, 0);
  group.add(signBoard);

  const signHanger1 = new THREE.Mesh(new THREE.CylinderGeometry(0.4, 0.4, 4.5, 6), steelMat);
  signHanger1.position.set(-11, roofBaseY - 1.5, 0);
  group.add(signHanger1);
  const signHanger2 = new THREE.Mesh(new THREE.CylinderGeometry(0.4, 0.4, 4.5, 6), steelMat);
  signHanger2.position.set(11, roofBaseY - 1.5, 0);
  group.add(signHanger2);

  // Weiße Schriftband-Attrappe auf beiden Schildseiten
  for (const zDir of [-1, 1]) {
    const textStrip = new THREE.Mesh(new THREE.BoxGeometry(28, 3.2, 0.2), safetyLineMat);
    textStrip.position.set(0, roofBaseY - 5.5, zDir * 0.9);
    group.add(textStrip);
  }

  // Doppelseitige Bahnhofsuhr bei X = 15
  const clockFrame = new THREE.Mesh(new THREE.CylinderGeometry(3.5, 3.5, 1.4, 16), steelMat);
  clockFrame.position.set(16, roofBaseY - 4.5, 0);
  clockFrame.rotation.z = Math.PI / 2;
  group.add(clockFrame);

  const clockRod = new THREE.Mesh(new THREE.CylinderGeometry(0.4, 0.4, 3.5, 6), steelMat);
  clockRod.position.set(16, roofBaseY - 1, 0);
  group.add(clockRod);

  for (const xDir of [-1, 1]) {
    const clockFace = new THREE.Mesh(new THREE.CylinderGeometry(3, 3, 0.2, 16), safetyLineMat);
    clockFace.position.set(16 + xDir * 0.72, roofBaseY - 4.5, 0);
    clockFace.rotation.z = Math.PI / 2;
    group.add(clockFace);
  }

  // -------------------------------------------------------------------------
  // 7. SITZBÄNKE & FAHRGASTAUSSTATTUNG
  // -------------------------------------------------------------------------
  // Sitzbänke bei X = -58 und X = +58 (beidseits des Treppenabgangs)
  for (const bx of [-58, 58]) {
    // Füße / Metallgestell
    for (const fx of [-7, 7]) {
      const leg = new THREE.Mesh(new THREE.BoxGeometry(1.2, 4.5, 7.5), steelMat);
      leg.position.set(bx + fx, 12.25, 0);
      group.add(leg);
    }
    // Sitzfläche (Holzlatten)
    const seat = new THREE.Mesh(new THREE.BoxGeometry(18, 0.8, 7.5), woodMat);
    seat.position.set(bx, 14.5, 0);
    group.add(seat);

    // Rückenlehne
    const backrest = new THREE.Mesh(new THREE.BoxGeometry(18, 3.5, 0.8), woodMat);
    backrest.position.set(bx, 17, 0);
    group.add(backrest);
  }

  // Fahrplankasten / Infovitrine bei X = -32
  const infoVitrine = new THREE.Mesh(new THREE.BoxGeometry(1.2, 12, 14), steelMat);
  infoVitrine.position.set(-32, 16, 0);
  group.add(infoVitrine);

  for (const zDir of [-1, 1]) {
    const poster = new THREE.Mesh(new THREE.BoxGeometry(0.2, 9.5, 11), yellowWarnMat);
    poster.position.set(-32, 16, zDir * 0.7);
    group.add(poster);
  }

  // SOS- / Notrufsäule bei X = 32
  const sosColumn = new THREE.Mesh(new THREE.BoxGeometry(2.5, 14, 2.5), redMat);
  sosColumn.position.set(32, 17, 4);
  group.add(sosColumn);

  const sosHeader = new THREE.Mesh(new THREE.BoxGeometry(3.2, 2.5, 3.2), safetyLineMat);
  sosHeader.position.set(32, 23.5, 4);
  group.add(sosHeader);

  // Abfallbehälter an Stützen montiert
  for (const px of [-80, 80]) {
    const trash = new THREE.Mesh(new THREE.CylinderGeometry(1.5, 1.3, 4, 10), metalMat);
    trash.position.set(px, 14.5, 3.5);
    group.add(trash);
  }

  // -------------------------------------------------------------------------
  // 8. BAHNSTEIGLEUCHTEN AN DEN FREIEN ENDEN
  // -------------------------------------------------------------------------
  // Moderne Peitschenleuchten bei X = -132 und X = +132
  for (const lx of [-132, 132]) {
    // Mast
    const mast = new THREE.Mesh(new THREE.CylinderGeometry(1, 1.4, 30, 8), steelMat);
    mast.position.set(lx, 10 + 15, 0);
    group.add(mast);

    // Mastkopf / Schirm
    const lampHead = new THREE.Mesh(new THREE.BoxGeometry(8, 1.2, 4), steelMat);
    lampHead.position.set(lx, 10 + 30.5, 0);
    group.add(lampHead);

    // Leuchtendes Glas
    const bulb = new THREE.Mesh(new THREE.BoxGeometry(6.5, 0.6, 2.8), lampGlowMat);
    bulb.position.set(lx, 10 + 29.8, 0);
    group.add(bulb);
  }
}
