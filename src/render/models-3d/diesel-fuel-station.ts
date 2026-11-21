import * as THREE from "three";

/**
 * Diesel Locomotive Fueling Station (Dieseltankstelle / Diesellok-BW)
 * - Oil-spill containment concrete apron (Beton-Auffangwanne) with safety curb & grates
 * - Main horizontal cylindrical diesel storage tank (Haupt-Dieseltank) on saddle piers
 *   with hazard band, top walkway & vertical safety ladder
 * - Brick/plaster pump & filter house (Pumpenhaus) with piping manifold (Rohrbrücke)
 * - Trackside fueling island with two locomotive diesel dispensers (Zapfsäulen mit Schwenkgalgen)
 *   covered by a cantilevered steel weather canopy (Wetterschutzdach)
 */
export function buildDieselFuelStation3D(group: THREE.Group): void {
  const concreteMat = new THREE.MeshStandardMaterial({
    color: 0x94a3b8,
    roughness: 0.9,
  });
  const darkConcreteMat = new THREE.MeshStandardMaterial({
    color: 0x64748b,
    roughness: 0.92,
  });
  const grateMat = new THREE.MeshStandardMaterial({
    color: 0x334155,
    roughness: 0.6,
    metalness: 0.5,
  });
  const tankMat = new THREE.MeshStandardMaterial({
    color: 0xdbeafe,
    roughness: 0.38,
    metalness: 0.35,
  });
  const redBandMat = new THREE.MeshStandardMaterial({
    color: 0xdc2626,
    roughness: 0.45,
  });
  const darkSteelMat = new THREE.MeshStandardMaterial({
    color: 0x334155,
    roughness: 0.45,
    metalness: 0.55,
  });
  const yellowSafetyMat = new THREE.MeshStandardMaterial({
    color: 0xfacc15,
    roughness: 0.5,
  });
  const pumpWallMat = new THREE.MeshStandardMaterial({
    color: 0xe2e8f0,
    roughness: 0.85,
  });
  const brickBaseMat = new THREE.MeshStandardMaterial({
    color: 0x991b1b,
    roughness: 0.88,
  });
  const canopyMat = new THREE.MeshStandardMaterial({
    color: 0x475569,
    roughness: 0.5,
    metalness: 0.4,
  });
  const dispenserBlueMat = new THREE.MeshStandardMaterial({
    color: 0x0284c7,
    roughness: 0.4,
    metalness: 0.25,
  });
  const pipeMat = new THREE.MeshStandardMaterial({
    color: 0x64748b,
    roughness: 0.35,
    metalness: 0.65,
  });
  const hoseMat = new THREE.MeshStandardMaterial({
    color: 0x111827,
    roughness: 0.85,
  });
  const lightMat = new THREE.MeshStandardMaterial({
    color: 0xfef9c3,
    emissive: 0xfef08a,
    emissiveIntensity: 0.45,
    roughness: 0.3,
  });

  // 1. Concrete Foundation & Oil Containment Apron (168 x 63 mm)
  const baseSlab = new THREE.Mesh(
    new THREE.BoxGeometry(168, 2.5, 63),
    concreteMat,
  );
  baseSlab.position.y = 1.25;
  group.add(baseSlab);

  // Trackside oil-spill collection pan (+Z side) with steel drainage grates
  const spillPan = new THREE.Mesh(
    new THREE.BoxGeometry(84, 0.8, 18),
    darkConcreteMat,
  );
  spillPan.position.set(32, 2.7, 17);
  group.add(spillPan);

  for (const gx of [4, 22, 40, 58]) {
    const grate = new THREE.Mesh(
      new THREE.BoxGeometry(14, 0.5, 12),
      grateMat,
    );
    grate.position.set(gx, 3.15, 17);
    group.add(grate);
  }

  // Yellow safety containment curb along front edge
  const frontCurb = new THREE.Mesh(
    new THREE.BoxGeometry(86, 1.4, 1.4),
    yellowSafetyMat,
  );
  frontCurb.position.set(32, 3.1, 26.5);
  group.add(frontCurb);

  // 2. Main Horizontal Cylindrical Diesel Storage Tank (Left/Rear: x = -40, z = -8)
  const tankGroup = new THREE.Group();
  tankGroup.position.set(-40, 2.5, -8);

  // 3 Concrete Saddle Piers (Sattellager)
  for (const sx of [-24, 0, 24]) {
    const pier = new THREE.Mesh(
      new THREE.BoxGeometry(4.5, 9, 24),
      darkConcreteMat,
    );
    pier.position.set(sx, 4.5, 0);
    tankGroup.add(pier);
  }

  // Horizontal cylinder body (length 68, radius 14.5)
  const tankRadius = 14.5;
  const tankLength = 68;
  const tankCenterY = 21;
  const tankCylinder = new THREE.Mesh(
    new THREE.CylinderGeometry(tankRadius, tankRadius, tankLength, 24),
    tankMat,
  );
  tankCylinder.rotation.z = Math.PI / 2;
  tankCylinder.position.set(0, tankCenterY, 0);
  tankGroup.add(tankCylinder);

  // Dished hemispherical end caps
  const capGeom = new THREE.SphereGeometry(
    tankRadius,
    20,
    12,
    0,
    Math.PI * 2,
    0,
    Math.PI / 2,
  );
  const leftCap = new THREE.Mesh(capGeom, tankMat);
  leftCap.rotation.z = Math.PI / 2;
  leftCap.scale.set(1, 0.35, 1);
  leftCap.position.set(-tankLength / 2, tankCenterY, 0);
  tankGroup.add(leftCap);

  const rightCap = new THREE.Mesh(capGeom, tankMat);
  rightCap.rotation.z = -Math.PI / 2;
  rightCap.scale.set(1, 0.35, 1);
  rightCap.position.set(tankLength / 2, tankCenterY, 0);
  tankGroup.add(rightCap);

  // Red flammability / hazard band around tank center & ends
  for (const bx of [-18, 0, 18]) {
    const band = new THREE.Mesh(
      new THREE.CylinderGeometry(
        tankRadius + 0.25,
        tankRadius + 0.25,
        bx === 0 ? 5.5 : 2.2,
        24,
      ),
      bx === 0 ? redBandMat : darkSteelMat,
    );
    band.rotation.z = Math.PI / 2;
    band.position.set(bx, tankCenterY, 0);
    tankGroup.add(band);
  }

  // Top inspection domes (Domdeckel) & catwalk
  for (const dx of [-16, 16]) {
    const dome = new THREE.Mesh(
      new THREE.CylinderGeometry(3.2, 3.5, 3.5, 12),
      darkSteelMat,
    );
    dome.position.set(dx, tankCenterY + tankRadius + 1.2, 0);
    tankGroup.add(dome);
  }

  const catwalk = new THREE.Mesh(
    new THREE.BoxGeometry(58, 0.8, 6),
    grateMat,
  );
  catwalk.position.set(0, tankCenterY + tankRadius + 2.8, 0);
  tankGroup.add(catwalk);

  // Top safety railing along catwalk
  const railTop = new THREE.Mesh(
    new THREE.BoxGeometry(58, 0.6, 0.6),
    yellowSafetyMat,
  );
  railTop.position.set(0, tankCenterY + tankRadius + 9.5, -2.8);
  tankGroup.add(railTop);
  for (const rx of [-28, -14, 0, 14, 28]) {
    const rPost = new THREE.Mesh(
      new THREE.BoxGeometry(0.6, 7, 0.6),
      yellowSafetyMat,
    );
    rPost.position.set(rx, tankCenterY + tankRadius + 6.2, -2.8);
    tankGroup.add(rPost);
  }

  // Access ladder on front of tank (+Z side) with top bridge onto catwalk
  const ladderTopY = tankCenterY + tankRadius + 9.5;
  const ladderH = ladderTopY - 0.5;
  for (const lx of [-1.8, 1.8]) {
    const rail = new THREE.Mesh(
      new THREE.BoxGeometry(0.6, ladderH, 0.6),
      yellowSafetyMat,
    );
    rail.position.set(lx, 0.5 + ladderH / 2, tankRadius + 1.5);
    tankGroup.add(rail);
  }
  for (let ry = 4; ry <= tankCenterY + tankRadius + 2.8; ry += 4.5) {
    const rung = new THREE.Mesh(
      new THREE.BoxGeometry(3.6, 0.5, 0.5),
      yellowSafetyMat,
    );
    rung.position.set(0, ry, tankRadius + 1.5);
    tankGroup.add(rung);
  }
  const ladderBridge = new THREE.Mesh(
    new THREE.BoxGeometry(5.6, 0.8, 13.4),
    grateMat,
  );
  ladderBridge.position.set(0, tankCenterY + tankRadius + 2.8, 9.5);
  tankGroup.add(ladderBridge);
  for (const lx of [-2.4, 2.4]) {
    const sideRail = new THREE.Mesh(
      new THREE.BoxGeometry(0.6, 0.6, 13.4),
      yellowSafetyMat,
    );
    sideRail.position.set(lx, tankCenterY + tankRadius + 9.5, 9.5);
    tankGroup.add(sideRail);
    const midPost = new THREE.Mesh(
      new THREE.BoxGeometry(0.6, 7, 0.6),
      yellowSafetyMat,
    );
    midPost.position.set(lx, tankCenterY + tankRadius + 6.2, 9.5);
    tankGroup.add(midPost);
  }

  group.add(tankGroup);

  // 3. Pump & Filter House (Pumpenhaus, center-rear: x = 18, z = -12)
  const pumpGroup = new THREE.Group();
  pumpGroup.position.set(18, 2.5, -12);

  const pumpPlinth = new THREE.Mesh(
    new THREE.BoxGeometry(28, 5, 22),
    brickBaseMat,
  );
  pumpPlinth.position.y = 2.5;
  pumpGroup.add(pumpPlinth);

  const pumpBody = new THREE.Mesh(
    new THREE.BoxGeometry(27, 21, 21),
    pumpWallMat,
  );
  pumpBody.position.y = 15.5;
  pumpGroup.add(pumpBody);

  const pumpRoof = new THREE.Mesh(
    new THREE.BoxGeometry(30, 2.2, 24),
    canopyMat,
  );
  pumpRoof.position.y = 27.1;
  pumpGroup.add(pumpRoof);

  // Steel door (bündig auf dem Vorbau vor dem Sockel) & industrial window on front (+Z)
  const pumpDoorThreshold = new THREE.Mesh(
    new THREE.BoxGeometry(8, 1.0, 1.6),
    darkSteelMat,
  );
  pumpDoorThreshold.position.set(-5, 0.5, 11.5);
  pumpGroup.add(pumpDoorThreshold);

  const pumpDoor = new THREE.Mesh(
    new THREE.BoxGeometry(7, 18.5, 1.0),
    darkSteelMat,
  );
  pumpDoor.position.set(-5, 9.8, 11.1);
  pumpGroup.add(pumpDoor);

  const pumpWindow = new THREE.Mesh(
    new THREE.BoxGeometry(8, 7, 0.8),
    lightMat,
  );
  pumpWindow.position.set(6, 15, 10.6);
  pumpGroup.add(pumpWindow);

  // Pipe manifold connecting storage tank to pump house
  for (const py of [10, 14]) {
    const pipe = new THREE.Mesh(
      new THREE.CylinderGeometry(1.1, 1.1, 26, 10),
      pipeMat,
    );
    pipe.rotation.z = Math.PI / 2;
    pipe.position.set(-20, py, 2);
    pumpGroup.add(pipe);
  }

  group.add(pumpGroup);

  // 4. Trackside Fueling Island, Dispenser Columns & Weather Canopy
  const islandGroup = new THREE.Group();
  islandGroup.position.set(32, 2.5, 15);

  // Raised concrete dispenser island
  const islandPlatform = new THREE.Mesh(
    new THREE.BoxGeometry(72, 2.2, 8),
    concreteMat,
  );
  islandPlatform.position.y = 1.1;
  islandGroup.add(islandPlatform);

  // Cantilevered steel weather canopy (Wetterschutzdach)
  const canopyY = 52;
  for (const cx of [-22, 22]) {
    const column = new THREE.Mesh(
      new THREE.BoxGeometry(3.2, canopyY, 3.2),
      darkSteelMat,
    );
    column.position.set(cx, canopyY / 2, -1);
    islandGroup.add(column);

    // Cross-beam under canopy
    const beam = new THREE.Mesh(
      new THREE.BoxGeometry(2.8, 3, 24),
      darkSteelMat,
    );
    beam.position.set(cx, canopyY - 1.5, 1);
    islandGroup.add(beam);
  }

  const canopyRoof = new THREE.Mesh(
    new THREE.BoxGeometry(78, 2.4, 26),
    canopyMat,
  );
  canopyRoof.position.set(0, canopyY + 1.2, 1);
  islandGroup.add(canopyRoof);

  // Blue fascia accent band around canopy roof
  const canopyFascia = new THREE.Mesh(
    new THREE.BoxGeometry(78.6, 1.8, 26.6),
    dispenserBlueMat,
  );
  canopyFascia.position.set(0, canopyY + 1.3, 1);
  islandGroup.add(canopyFascia);

  // Under-canopy fluorescent light strips
  for (const lx of [-18, 18]) {
    const lightStrip = new THREE.Mesh(
      new THREE.BoxGeometry(16, 0.8, 3),
      lightMat,
    );
    lightStrip.position.set(lx, canopyY - 0.4, 2);
    islandGroup.add(lightStrip);
  }

  // Two high-capacity locomotive diesel dispensers (Zapfsäulen mit Schwenkgalgen)
  for (const dx of [-14, 14]) {
    const dispCabinet = new THREE.Mesh(
      new THREE.BoxGeometry(7.5, 19, 5),
      dispenserBlueMat,
    );
    dispCabinet.position.set(dx, 2.2 + 9.5, 0);
    islandGroup.add(dispCabinet);

    // White dial / meter display panel
    const meterFace = new THREE.Mesh(
      new THREE.BoxGeometry(5.2, 5.5, 5.4),
      pumpWallMat,
    );
    meterFace.position.set(dx, 2.2 + 13, 0);
    islandGroup.add(meterFace);

    // High-reach swiveling fuel boom (Schwenkgalgen) for locomotive side/top tanks
    const standPipe = new THREE.Mesh(
      new THREE.CylinderGeometry(0.9, 0.9, 18, 8),
      pipeMat,
    );
    standPipe.position.set(dx + 2.2, 2.2 + 24, 0);
    islandGroup.add(standPipe);

    const boomArm = new THREE.Mesh(
      new THREE.CylinderGeometry(0.7, 0.7, 13, 8),
      yellowSafetyMat,
    );
    boomArm.rotation.x = Math.PI / 2.2;
    boomArm.position.set(dx + 2.2, 2.2 + 32, 5.5);
    islandGroup.add(boomArm);

    // Hanging flexible fuel hose (nahtlos am Schwenkgalgen-Ende bei y = 35,1 mm)
    const hose = new THREE.Mesh(
      new THREE.CylinderGeometry(0.45, 0.45, 20, 8),
      hoseMat,
    );
    hose.position.set(dx + 2.2, 2.2 + 23.0, 11.8);
    islandGroup.add(hose);
  }

  // Yellow protective crash bollards at island ends
  for (const bx of [-33, 33]) {
    for (const bz of [-2.5, 2.5]) {
      const bollard = new THREE.Mesh(
        new THREE.CylinderGeometry(1.1, 1.1, 10, 10),
        yellowSafetyMat,
      );
      bollard.position.set(bx, 6, bz);
      islandGroup.add(bollard);
    }
  }

  group.add(islandGroup);
}
