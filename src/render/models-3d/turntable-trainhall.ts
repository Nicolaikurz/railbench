import * as THREE from "three";

/**
 * 3-ständiger Ringlokschuppen für Drehscheiben (Turntable Train Hall)
 * Ausgelegt für 12°-Teilung (Standard bei DB-Drehscheiben wie Märklin 74862).
 * Abmessungen BxTxH: 320 x 310 x 105 mm.
 *
 * Koordinatenraum:
 * - Drehscheiben-Mittelpunkt liegt bei (0, 0, -350) in lokalen Koordinaten
 * - Y = 0 mm ist die Tischoberkante (alle Bauteile liegen bei Y >= 0)
 * - 3 Gleisstände bei -12°, 0° und +12°
 * - Segmentierte Giebeldächer mit Firstlinien entlang der Standachsen
 */
export function buildTurntableTrainhall3D(group: THREE.Group): void {
  const DEG2RAD = Math.PI / 180;
  const TT_Z = -350;        // Drehscheiben-Mittelpunkt im lokalen Modellraum
  const R_FRONT = 195;      // Torfront-Radius vom Drehscheiben-Zentrum
  const R_REAR = 505;       // Rückwand-Radius vom Drehscheiben-Zentrum
  const STALL_CENTERS = [-12 * DEG2RAD, 0, 12 * DEG2RAD];
  const PARTITION_ANGLES = [-18 * DEG2RAD, -6 * DEG2RAD, 6 * DEG2RAD, 18 * DEG2RAD];

  const Y_GROUND = 0;       // Tischoberfläche
  const Y_FLOOR = 3.5;      // Oberkante Hallenboden / Schienenbasis
  const Y_EAVE = 100;       // Traufhöhe der Wände (erhöht für vollständige Oberleitungs-Freiheit)
  const Y_RIDGE = 122;      // Firsthöhe der Giebeldächer

  // Materialien
  const groundMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.9 });
  const brickMat = new THREE.MeshStandardMaterial({ color: 0x8b3a2b, roughness: 0.82 });
  const trimMat = new THREE.MeshStandardMaterial({ color: 0xd4b996, roughness: 0.75 });
  const roofMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.7, side: THREE.DoubleSide });
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.15, metalness: 0.85 });
  const doorMat = new THREE.MeshStandardMaterial({ color: 0x1e3a5f, roughness: 0.8 });
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.4, metalness: 0.7 });
  const railMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.3, metalness: 0.8 });
  const chimMat = new THREE.MeshStandardMaterial({ color: 0x1c1917, roughness: 0.6 });
  const lampMat = new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.8 });

  // Hilfsfunktion: Berechnet einen 3D-Punkt aus Polar-Radius und Winkel um das Drehscheibenzentrum
  function radialPt(r: number, angleRad: number, y = 0): THREE.Vector3 {
    return new THREE.Vector3(
      r * Math.sin(angleRad),
      y,
      r * Math.cos(angleRad) + TT_Z,
    );
  }

  // Hilfsfunktion: Fügt zwei Dreiecke (Quad) zu Positions- und Normalenarrays hinzu
  function addQuad(
    positions: number[],
    normals: number[],
    p1: THREE.Vector3,
    p2: THREE.Vector3,
    p3: THREE.Vector3,
    p4: THREE.Vector3,
  ) {
    const vA = new THREE.Vector3().subVectors(p2, p1);
    const vB = new THREE.Vector3().subVectors(p3, p1);
    const n = new THREE.Vector3().crossVectors(vA, vB).normalize();

    positions.push(
      p1.x, p1.y, p1.z,  p2.x, p2.y, p2.z,  p3.x, p3.y, p3.z,
      p1.x, p1.y, p1.z,  p3.x, p3.y, p3.z,  p4.x, p4.y, p4.z,
    );
    for (let i = 0; i < 6; i++) {
      normals.push(n.x, n.y, n.z);
    }
  }

  function addTri(
    positions: number[],
    normals: number[],
    p1: THREE.Vector3,
    p2: THREE.Vector3,
    p3: THREE.Vector3,
  ) {
    const vA = new THREE.Vector3().subVectors(p2, p1);
    const vB = new THREE.Vector3().subVectors(p3, p1);
    const n = new THREE.Vector3().crossVectors(vA, vB).normalize();

    positions.push(p1.x, p1.y, p1.z,  p2.x, p2.y, p2.z,  p3.x, p3.y, p3.z);
    for (let i = 0; i < 3; i++) {
      normals.push(n.x, n.y, n.z);
    }
  }

  // 1. FUNDAMENTPLATTE (Massiver Betonboden von Y=0 bis Y=3.5 mm)
  const floorPos: number[] = [];
  const floorNorm: number[] = [];
  const numFloorSteps = 16;

  for (let j = 0; j < numFloorSteps; j++) {
    const a1 = -18 * DEG2RAD + (36 * DEG2RAD * j) / numFloorSteps;
    const a2 = -18 * DEG2RAD + (36 * DEG2RAD * (j + 1)) / numFloorSteps;

    // Obere Bodenfläche
    const t1 = radialPt(R_FRONT, a1, Y_FLOOR);
    const t2 = radialPt(R_FRONT, a2, Y_FLOOR);
    const t3 = radialPt(R_REAR, a2, Y_FLOOR);
    const t4 = radialPt(R_REAR, a1, Y_FLOOR);
    addQuad(floorPos, floorNorm, t1, t2, t3, t4);

    // Vorderkante senkrecht
    const b1 = radialPt(R_FRONT, a1, Y_GROUND);
    const b2 = radialPt(R_FRONT, a2, Y_GROUND);
    addQuad(floorPos, floorNorm, b1, t1, t2, b2);

    // Hinterkante senkrecht
    const b4 = radialPt(R_REAR, a1, Y_GROUND);
    const b3 = radialPt(R_REAR, a2, Y_GROUND);
    addQuad(floorPos, floorNorm, b3, t3, t4, b4);
  }

  // Seitenkanten der Bodenplatte links (-18°) und rechts (+18°)
  const flL_t1 = radialPt(R_FRONT, -18 * DEG2RAD, Y_FLOOR);
  const flL_t2 = radialPt(R_REAR, -18 * DEG2RAD, Y_FLOOR);
  const flL_b1 = radialPt(R_FRONT, -18 * DEG2RAD, Y_GROUND);
  const flL_b2 = radialPt(R_REAR, -18 * DEG2RAD, Y_GROUND);
  addQuad(floorPos, floorNorm, flL_b2, flL_t2, flL_t1, flL_b1);

  const flR_t1 = radialPt(R_FRONT, 18 * DEG2RAD, Y_FLOOR);
  const flR_t2 = radialPt(R_REAR, 18 * DEG2RAD, Y_FLOOR);
  const flR_b1 = radialPt(R_FRONT, 18 * DEG2RAD, Y_GROUND);
  const flR_b2 = radialPt(R_REAR, 18 * DEG2RAD, Y_GROUND);
  addQuad(floorPos, floorNorm, flR_b1, flR_t1, flR_t2, flR_b2);

  const floorGeo = new THREE.BufferGeometry();
  floorGeo.setAttribute("position", new THREE.Float32BufferAttribute(floorPos, 3));
  floorGeo.setAttribute("normal", new THREE.Float32BufferAttribute(floorNorm, 3));
  const floorMesh = new THREE.Mesh(floorGeo, groundMat);
  floorMesh.receiveShadow = true;
  group.add(floorMesh);

  // 2. GLEISE & UNTERSUCHUNGSGRUBEN IN DEN 3 STÄNDEN (Y=3.5 bis 5.5 mm)
  const trackGauge = 16.5;
  const trackLength = R_REAR - R_FRONT - 25;

  for (const angle of STALL_CENTERS) {
    const stallGroup = new THREE.Group();
    stallGroup.position.set(0, 0, TT_Z);
    stallGroup.rotation.y = -angle;

    // Schienenprofile (Höhe 2.0 mm auf dem Hallenboden bei Y=3.5 mm -> Y=4.5 mm)
    const railGeo = new THREE.BoxGeometry(1.6, 2.0, trackLength);
    const railLeft = new THREE.Mesh(railGeo, railMat);
    railLeft.position.set(-trackGauge / 2, Y_FLOOR + 1.0, R_FRONT + trackLength / 2);
    stallGroup.add(railLeft);

    const railRight = new THREE.Mesh(railGeo, railMat);
    railRight.position.set(trackGauge / 2, Y_FLOOR + 1.0, R_FRONT + trackLength / 2);
    stallGroup.add(railRight);

    // Untersuchungsgrube (dunkle Einbuchtung zwischen den Schienen, Y=3.5 bis 3.8 mm)
    const pitMat = new THREE.MeshStandardMaterial({ color: 0x1f2937, roughness: 0.95 });
    const pit = new THREE.Mesh(new THREE.BoxGeometry(trackGauge - 3.2, 0.6, trackLength - 30), pitMat);
    pit.position.set(0, Y_FLOOR + 0.3, R_FRONT + trackLength / 2 - 10);
    stallGroup.add(pit);

    // Prellbock am Hallenende
    const stopMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });
    const bufferStop = new THREE.Mesh(new THREE.BoxGeometry(trackGauge + 6, 7, 4), stopMat);
    bufferStop.position.set(0, Y_FLOOR + 3.5, R_FRONT + trackLength - 6);
    stallGroup.add(bufferStop);

    group.add(stallGroup);
  }

  // 3. SEITENWÄNDE (Flanken links bei -18° und rechts bei +18°)
  const wallH = Y_EAVE - Y_FLOOR; // 60.5 mm
  const flankLen = R_REAR - R_FRONT;

  for (const side of [-1, 1]) {
    const flankGroup = new THREE.Group();
    flankGroup.position.set(0, 0, TT_Z);
    flankGroup.rotation.y = -side * 18 * DEG2RAD;

    // Backsteinwand von Y=3.5 bis Y=64 mm
    const wall = new THREE.Mesh(new THREE.BoxGeometry(4, wallH, flankLen), brickMat);
    wall.position.set(0, Y_FLOOR + wallH / 2, R_FRONT + flankLen / 2);
    flankGroup.add(wall);

    // Werkstattfenster an den Seiten
    for (let fz = R_FRONT + 45; fz < R_REAR - 45; fz += 55) {
      const windowMesh = new THREE.Mesh(new THREE.BoxGeometry(4.4, 40, 30), glassMat);
      windowMesh.position.set(0, Y_FLOOR + 46, fz);
      flankGroup.add(windowMesh);

      const sill = new THREE.Mesh(new THREE.BoxGeometry(5.2, 2.5, 34), trimMat);
      sill.position.set(0, Y_FLOOR + 25, fz);
      flankGroup.add(sill);
    }

    group.add(flankGroup);
  }

  // 4. RÜCKWAND (Segmentierte Backsteinwand mit Fenstern)
  for (let i = 0; i < 3; i++) {
    const angle = STALL_CENTERS[i]!;
    const segGroup = new THREE.Group();
    segGroup.position.set(0, 0, TT_Z);
    segGroup.rotation.y = -angle;

    const segW = R_REAR * 12 * DEG2RAD;
    const rearWall = new THREE.Mesh(new THREE.BoxGeometry(segW + 2, wallH, 4), brickMat);
    rearWall.position.set(0, Y_FLOOR + wallH / 2, R_REAR - 2);
    segGroup.add(rearWall);

    // Großes Sprossenfenster
    const rearWin = new THREE.Mesh(new THREE.BoxGeometry(segW * 0.65, 46, 4.4), glassMat);
    rearWin.position.set(0, Y_FLOOR + 48, R_REAR - 2);
    segGroup.add(rearWin);

    // Pfeiler an den Segmentgrenzen
    const pillarL = new THREE.Mesh(new THREE.BoxGeometry(5, wallH + 2, 6), trimMat);
    pillarL.position.set(-segW / 2, Y_FLOOR + (wallH + 2) / 2, R_REAR - 1);
    segGroup.add(pillarL);

    const pillarR = new THREE.Mesh(new THREE.BoxGeometry(5, wallH + 2, 6), trimMat);
    pillarR.position.set(segW / 2, Y_FLOOR + (wallH + 2) / 2, R_REAR - 1);
    segGroup.add(pillarR);

    group.add(segGroup);
  }

  // 5. FRONTFASSADE MIT LOKTOREN (Y = 3.5 bis Y = 100 mm)
  // Durchfahrtsöffnung: Breite 34 mm, Höhe 88 mm (Y von 3.5 bis 91.5 mm für volle Oberleitungs-Freiheit)
  const portalW = 34;
  const portalH = 88;

  for (let i = 0; i < 3; i++) {
    const angle = STALL_CENTERS[i]!;
    const portalGroup = new THREE.Group();
    portalGroup.position.set(0, 0, TT_Z);
    portalGroup.rotation.y = -angle;

    const frontSegW = R_FRONT * 12 * DEG2RAD + 2;

    // Torsturzwand oben (über der Durchfahrt bis zur Traufe Y=64 mm)
    const sturzH = wallH - portalH; // 14.5 mm
    const frontTop = new THREE.Mesh(new THREE.BoxGeometry(frontSegW, sturzH, 4), brickMat);
    frontTop.position.set(0, Y_FLOOR + portalH + sturzH / 2, R_FRONT + 2);
    portalGroup.add(frontTop);

    // Seitliche Pfeiler links und rechts der Tordurchfahrt (vom Boden Y=3.5 bis Y=64 mm)
    const sidePillarW = (frontSegW - portalW) / 2;
    const pillarLeft = new THREE.Mesh(new THREE.BoxGeometry(sidePillarW + 1, wallH, 4.5), trimMat);
    pillarLeft.position.set(-frontSegW / 2 + sidePillarW / 2, Y_FLOOR + wallH / 2, R_FRONT + 2);
    portalGroup.add(pillarLeft);

    const pillarRight = new THREE.Mesh(new THREE.BoxGeometry(sidePillarW + 1, wallH, 4.5), trimMat);
    pillarRight.position.set(frontSegW / 2 - sidePillarW / 2, Y_FLOOR + wallH / 2, R_FRONT + 2);
    portalGroup.add(pillarRight);

    // Sandstein-Torbogen / Sturz
    const archTrim = new THREE.Mesh(new THREE.BoxGeometry(portalW + 4, 3.5, 5), trimMat);
    archTrim.position.set(0, Y_FLOOR + portalH + 1.7, R_FRONT + 2);
    portalGroup.add(archTrim);

    // Torflügel links & rechts weit geöffnet an den Außenpfosten
    const leafW = portalW / 2 - 0.5;
    const doorLeafGeo = new THREE.BoxGeometry(leafW, portalH - 1, 1.8);

    // Linker Torflügel: Scharnier am linken Portalpfeiler, weit nach außen geöffnet (~77°)
    const hingeL = new THREE.Group();
    hingeL.position.set(-portalW / 2, Y_FLOOR + (portalH - 1) / 2, R_FRONT + 2);
    hingeL.rotation.y = 1.35;

    const doorL = new THREE.Mesh(doorLeafGeo, doorMat);
    doorL.position.set(leafW / 2, 0, 0);
    hingeL.add(doorL);

    const doorWinL = new THREE.Mesh(new THREE.BoxGeometry(9, 20, 2.2), glassMat);
    doorWinL.position.set(leafW / 2, (portalH - 1) / 2 - 16, 0);
    hingeL.add(doorWinL);

    portalGroup.add(hingeL);

    // Rechter Torflügel: Scharnier am rechten Portalpfeiler, weit nach außen geöffnet (~ -77°)
    const hingeR = new THREE.Group();
    hingeR.position.set(portalW / 2, Y_FLOOR + (portalH - 1) / 2, R_FRONT + 2);
    hingeR.rotation.y = -1.35;

    const doorR = new THREE.Mesh(doorLeafGeo, doorMat);
    doorR.position.set(-leafW / 2, 0, 0);
    hingeR.add(doorR);

    const doorWinR = new THREE.Mesh(new THREE.BoxGeometry(9, 20, 2.2), glassMat);
    doorWinR.position.set(-leafW / 2, (portalH - 1) / 2 - 16, 0);
    hingeR.add(doorWinR);

    portalGroup.add(hingeR);

    // Laterne über dem Tor
    const lamp = new THREE.Mesh(new THREE.SphereGeometry(2.2, 8, 8), lampMat);
    lamp.position.set(0, Y_FLOOR + portalH + 7, R_FRONT - 1);
    portalGroup.add(lamp);

    group.add(portalGroup);
  }

  // 6. ECHTES FÄCHERFÖRMIGES GIEBELDACH (Exakte Ausrichtung, nahtlos von Traufe Y=64 zu First Y=82 mm)
  const roofPos: number[] = [];
  const roofNorm: number[] = [];
  const gablePos: number[] = [];
  const gableNorm: number[] = [];

  const R_ROOF_FRONT = R_FRONT - 3;
  const R_ROOF_REAR = R_REAR + 5;

  for (let s = 0; s < 3; s++) {
    const aL = PARTITION_ANGLES[s]!;
    const aC = STALL_CENTERS[s]!;
    const aR = PARTITION_ANGLES[s + 1]!;

    // 6 Eckpunkte des Stand-Daches
    const fL = radialPt(R_ROOF_FRONT, aL, Y_EAVE);
    const fC = radialPt(R_ROOF_FRONT, aC, Y_RIDGE);
    const fR = radialPt(R_ROOF_FRONT, aR, Y_EAVE);

    const rL = radialPt(R_ROOF_REAR, aL, Y_EAVE);
    const rC = radialPt(R_ROOF_REAR, aC, Y_RIDGE);
    const rR = radialPt(R_ROOF_REAR, aR, Y_EAVE);

    // Linke Dachhälfte (vom First zur linken Traufe)
    addQuad(roofPos, roofNorm, fL, rL, rC, fC);

    // Rechte Dachhälfte (vom First zur rechten Traufe)
    addQuad(roofPos, roofNorm, fC, rC, rR, fR);

    // Vorderer Giebeldreieck-Abschluss (über den Toren, Backstein)
    const fL_wall = radialPt(R_FRONT + 2, aL, Y_EAVE);
    const fC_wall = radialPt(R_FRONT + 2, aC, Y_RIDGE);
    const fR_wall = radialPt(R_FRONT + 2, aR, Y_EAVE);
    addTri(gablePos, gableNorm, fL_wall, fC_wall, fR_wall);

    // Hinterer Giebeldreieck-Abschluss (über der Rückwand, Backstein)
    const rL_wall = radialPt(R_REAR - 2, aL, Y_EAVE);
    const rC_wall = radialPt(R_REAR - 2, aC, Y_RIDGE);
    const rR_wall = radialPt(R_REAR - 2, aR, Y_EAVE);
    addTri(gablePos, gableNorm, rR_wall, rC_wall, rL_wall);
  }

  // Schieferdach-Mesh
  const roofGeo = new THREE.BufferGeometry();
  roofGeo.setAttribute("position", new THREE.Float32BufferAttribute(roofPos, 3));
  roofGeo.setAttribute("normal", new THREE.Float32BufferAttribute(roofNorm, 3));
  const roofMesh = new THREE.Mesh(roofGeo, roofMat);
  roofMesh.castShadow = true;
  roofMesh.receiveShadow = true;
  group.add(roofMesh);

  // Giebelwand-Mesh (Backstein)
  const gableGeo = new THREE.BufferGeometry();
  gableGeo.setAttribute("position", new THREE.Float32BufferAttribute(gablePos, 3));
  gableGeo.setAttribute("normal", new THREE.Float32BufferAttribute(gableNorm, 3));
  const gableMesh = new THREE.Mesh(gableGeo, brickMat);
  group.add(gableMesh);

  // 7. OBERLICHTER & SCHORNSTEINE AUF DEM DACHFIRST
  for (let s = 0; s < 3; s++) {
    const angle = STALL_CENTERS[s]!;
    const ridgeGroup = new THREE.Group();
    ridgeGroup.position.set(0, 0, TT_Z);
    ridgeGroup.rotation.y = -angle;

    const ridgeLen = R_ROOF_REAR - R_ROOF_FRONT - 40;
    const midR = (R_ROOF_FRONT + R_ROOF_REAR) / 2;

    // Verglaster Dachreiter / Oberlichtstreifen auf dem First
    const skylight = new THREE.Mesh(new THREE.BoxGeometry(10, 5, ridgeLen), glassMat);
    skylight.position.set(0, Y_RIDGE + 2.5, midR);
    ridgeGroup.add(skylight);

    const skylightFrame = new THREE.Mesh(new THREE.BoxGeometry(11, 1.2, ridgeLen + 2), steelMat);
    skylightFrame.position.set(0, Y_RIDGE + 5.2, midR);
    ridgeGroup.add(skylightFrame);

    // Rauchabzug / Schornstein für Dampflokomotiven über dem Stand
    const chimneyR = R_FRONT + 140;
    const chimBase = new THREE.Mesh(new THREE.CylinderGeometry(5, 6, 20, 12), chimMat);
    chimBase.position.set(0, Y_RIDGE + 10, chimneyR);
    ridgeGroup.add(chimBase);

    const chimCap = new THREE.Mesh(new THREE.ConeGeometry(8, 5, 12), chimMat);
    chimCap.position.set(0, Y_RIDGE + 22.5, chimneyR);
    ridgeGroup.add(chimCap);

    group.add(ridgeGroup);
  }

  // 8. INNENSTÜTZEN ZWISCHEN DEN STÄNDEN
  for (const pillarAngle of [-6 * DEG2RAD, 6 * DEG2RAD]) {
    for (const pr of [R_FRONT + 75, R_FRONT + 155, R_FRONT + 235]) {
      const colGroup = new THREE.Group();
      colGroup.position.set(0, 0, TT_Z);
      colGroup.rotation.y = -pillarAngle;

      const column = new THREE.Mesh(new THREE.BoxGeometry(3.5, Y_EAVE - Y_FLOOR, 3.5), steelMat);
      column.position.set(0, Y_FLOOR + (Y_EAVE - Y_FLOOR) / 2, pr);
      colGroup.add(column);

      group.add(colGroup);
    }
  }
}
