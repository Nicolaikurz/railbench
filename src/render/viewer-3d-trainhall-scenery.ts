import * as THREE from "three";

/**
 * Baut einen dichten Nadel- und Mischwald rund um die Außenseite der Lokhalle.
 * Die Baumkronen und Stämme ragen an den großen Fabrikfenstern und Toren empor,
 * sodass beim Blick aus den Hallenfenstern eine lebendige Waldlandschaft sichtbar ist.
 */
export function buildTrainHallForest(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  floorY: number,
  isDark: boolean,
) {
  const halfW = wHall / 2;
  const halfL = lHall / 2;

  // A. Weitläufiger Waldboden bis zum Horizont (Radius 160.000 mm)
  const forestFloorGeo = new THREE.CircleGeometry(160000, 64);
  forestFloorGeo.rotateX(-Math.PI / 2);
  const forestFloorMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x132616 : 0x2e5425,
    roughness: 0.95,
    metalness: 0.04,
  });
  const forestFloor = new THREE.Mesh(forestFloorGeo, forestFloorMat);
  forestFloor.position.set(cx, floorY - 3, cz);
  forestFloor.receiveShadow = true;
  trainHallGroup.add(forestFloor);

  // Baum- & Natur-Materialien
  const barkMat = new THREE.MeshStandardMaterial({ color: 0x3f2e21, roughness: 0.9 });
  const pineMat1 = new THREE.MeshStandardMaterial({ color: isDark ? 0x052e16 : 0x14532d, roughness: 0.95 });
  const pineMat2 = new THREE.MeshStandardMaterial({ color: isDark ? 0x064e3b : 0x166534, roughness: 0.95 });
  const leafMat = new THREE.MeshStandardMaterial({ color: isDark ? 0x166534 : 0x15803d, roughness: 0.9 });
  const bushMat = new THREE.MeshStandardMaterial({ color: isDark ? 0x052e16 : 0x166534, roughness: 0.95 });
  const hillHorizonMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x0f1e14 : 0x223e1d,
    roughness: 0.95,
  });

  // Shared Geometrien für performantes Rendering vieler Bäume (ausgedehnte Waldlandschaft)
  const pineTrunkGeo = new THREE.CylinderGeometry(18, 28, 220, 8);
  pineTrunkGeo.translate(0, 110, 0);
  const c1Geo = new THREE.ConeGeometry(170, 240, 8);
  c1Geo.translate(0, 230, 0);
  const c2Geo = new THREE.ConeGeometry(140, 220, 8);
  c2Geo.translate(0, 360, 0);
  const c3Geo = new THREE.ConeGeometry(105, 200, 8);
  c3Geo.translate(0, 480, 0);
  const c4Geo = new THREE.ConeGeometry(70, 180, 8);
  c4Geo.translate(0, 590, 0);

  const decTrunkGeo = new THREE.CylinderGeometry(20, 32, 260, 8);
  decTrunkGeo.translate(0, 130, 0);
  const crown1Geo = new THREE.DodecahedronGeometry(190, 1);
  crown1Geo.translate(0, 390, 0);
  const crown2Geo = new THREE.DodecahedronGeometry(140, 1);
  crown2Geo.translate(70, 350, 40);

  const pineTrees: { x: number; z: number; scale: number }[] = [];
  const deciduousTrees: { x: number; z: number; scale: number }[] = [];

  const addPineTree = (x: number, z: number, scale: number) => {
    pineTrees.push({ x, z, scale });
  };

  const addDeciduousTree = (x: number, z: number, scale: number) => {
    deciduousTrees.push({ x, z, scale });
  };

  // 1. WESTSEITE: Riesiges, tiefes und weitläufiges Waldmassiv
  const countZ = Math.floor(lHall / 650);

  // A. Waldsaum nahe der Halle (mit 1.900–2.400 mm Sicherheitsabstand zum Mauerwerk)
  for (let i = -5; i <= countZ + 5; i++) {
    const z = cz - halfL + i * 650 + ((i * 123) % 210);
    const x1 = cx - halfW - 2000 - ((i * 87) % 350);
    const scale1 = 3.8 + ((i * 17) % 8) * 0.2;
    if (i % 3 === 0) {
      addDeciduousTree(x1, z, scale1 * 0.95);
    } else {
      addPineTree(x1, z, scale1);
    }
  }

  // B. Ausgedehnter Mittel- und Tiefwald nach Westen
  const westForestPoints: [number, number, number, boolean][] = [];
  const seedPoints = 72;
  for (let k = 0; k < seedPoints; k++) {
    const distProgress = (k % 6) / 5;
    const baseX = 3600 + distProgress * 46000 + ((k * 1317) % 3500);
    const x = cx - halfW - baseX;
    const zRatio = (Math.floor(k / 6) / 12) - 0.5;
    const z = cz + zRatio * 68000 + ((k * 789) % 3200);
    const scale = 4.4 + distProgress * 2.2 + ((k * 37) % 7) * 0.25;
    const isDeciduous = k % 4 === 1;
    westForestPoints.push([x, z, scale, isDeciduous]);
  }

  for (const [fx, fz, fscale, fdec] of westForestPoints) {
    if (fdec) {
      addDeciduousTree(fx, fz, fscale * 0.9);
    } else {
      addPineTree(fx, fz, fscale);
    }
  }

  // C. Dichte Fichten-Horizontkette im fernen Nordwesten
  for (let r = 0; r < 24; r++) {
    const angle = Math.PI * 0.85 + (r / 24) * Math.PI * 0.6;
    const dist = 38000 + ((r * 1543) % 22000);
    const hx = cx + Math.cos(angle) * dist;
    const hz = cz + Math.sin(angle) * dist;
    addPineTree(hx, hz, 6.2 + ((r * 29) % 6) * 0.35);
  }

  // 2. OSTSEITE: Weite, offene Wiesenlandschaft mit nur wenigen, malerischen Baumgruppen
  const eastSparseTrees: [number, number, number, boolean][] = [
    [cx + halfW + 5800, cz + 4800, 4.2, false],
    [cx + halfW + 6400, cz + 5400, 3.8, true],
    [cx + halfW + 7100, cz + 4400, 4.6, false],
    [cx + halfW + 7200, cz - 3500, 4.8, false],
    [cx + halfW + 8100, cz - 4100, 4.0, true],
    [cx + halfW + 9000, cz - 3200, 5.2, false],
    [cx + halfW + 10200, cz - 4600, 4.5, false],
    [cx + halfW + 3600, cz - 1400, 4.3, true],
    [cx + halfW + 5800, cz - 7800, 4.6, false],
    [cx + halfW + 12500, cz + 8500, 5.4, false],
    [cx + halfW + 16000, cz - 12000, 5.8, true],
    [cx + halfW + 36000, cz - 6000, 6.5, false],
    [cx + halfW + 39000, cz - 4500, 6.8, false],
    [cx + halfW + 43000, cz - 8000, 7.0, true],
    [cx + halfW + 38000, cz + 14000, 6.6, false],
    [cx + halfW + 42000, cz + 16500, 6.9, false],
  ];

  for (const [ex, ez, escale, edec] of eastSparseTrees) {
    if (edec) {
      addDeciduousTree(ex, ez, escale * 0.9);
    } else {
      addPineTree(ex, ez, escale);
    }
  }

  // 3. STIRNSEITEN (Nord & Süd): Wald schmiegt sich an die Westecke
  const endZPositions = [cz - halfL - 600, cz + halfL + 600];
  for (const ez of endZPositions) {
    const signZ = ez < cz ? -1 : 1;
    for (let j = 0; j < 8; j++) {
      const x = cx - halfW + j * 550 - 1200;
      const z1 = ez + signZ * (2200 + ((j * 113) % 500));
      addPineTree(x, z1, 4.5 + ((j * 17) % 5) * 0.25);

      const z2 = ez + signZ * (5200 + ((j * 179) % 1100));
      addPineTree(x - 300, z2, 5.2 + ((j * 23) % 5) * 0.3);

      const z3 = ez + signZ * (11000 + ((j * 251) % 2200));
      addPineTree(x - 700, z3, 6.0 + ((j * 31) % 5) * 0.35);
    }

    const xOpen = cx + halfW * 0.5 + 1200;
    const zOpen = ez + signZ * 3400;
    addPineTree(xOpen, zOpen, 4.4);
    addDeciduousTree(xOpen + 1800, zOpen + signZ * 1200, 4.0);
  }

  // Erzeuge InstancedMeshes für Hunderte von Bäumen
  const dummy = new THREE.Object3D();

  if (pineTrees.length > 0) {
    const trunkInst = new THREE.InstancedMesh(pineTrunkGeo, barkMat, pineTrees.length);
    const c1Inst = new THREE.InstancedMesh(c1Geo, pineMat1, pineTrees.length);
    const c2Inst = new THREE.InstancedMesh(c2Geo, pineMat2, pineTrees.length);
    const c3Inst = new THREE.InstancedMesh(c3Geo, pineMat1, pineTrees.length);
    const c4Inst = new THREE.InstancedMesh(c4Geo, pineMat2, pineTrees.length);

    for (let i = 0; i < pineTrees.length; i++) {
      const pt = pineTrees[i]!;
      dummy.position.set(pt.x, floorY, pt.z);
      dummy.scale.setScalar(pt.scale);
      dummy.rotation.set(0, 0, 0);
      dummy.updateMatrix();

      trunkInst.setMatrixAt(i, dummy.matrix);
      c1Inst.setMatrixAt(i, dummy.matrix);
      c2Inst.setMatrixAt(i, dummy.matrix);
      c3Inst.setMatrixAt(i, dummy.matrix);
      c4Inst.setMatrixAt(i, dummy.matrix);
    }

    trunkInst.instanceMatrix.needsUpdate = true;
    c1Inst.instanceMatrix.needsUpdate = true;
    c2Inst.instanceMatrix.needsUpdate = true;
    c3Inst.instanceMatrix.needsUpdate = true;
    c4Inst.instanceMatrix.needsUpdate = true;

    trainHallGroup.add(trunkInst, c1Inst, c2Inst, c3Inst, c4Inst);
  }

  if (deciduousTrees.length > 0) {
    const dTrunkInst = new THREE.InstancedMesh(decTrunkGeo, barkMat, deciduousTrees.length);
    const cr1Inst = new THREE.InstancedMesh(crown1Geo, leafMat, deciduousTrees.length);
    const cr2Inst = new THREE.InstancedMesh(crown2Geo, leafMat, deciduousTrees.length);

    for (let i = 0; i < deciduousTrees.length; i++) {
      const dt = deciduousTrees[i]!;
      dummy.position.set(dt.x, floorY, dt.z);
      dummy.scale.setScalar(dt.scale);
      dummy.rotation.set(0, 0, 0);
      dummy.updateMatrix();

      dTrunkInst.setMatrixAt(i, dummy.matrix);
      cr1Inst.setMatrixAt(i, dummy.matrix);
      cr2Inst.setMatrixAt(i, dummy.matrix);
    }

    dTrunkInst.instanceMatrix.needsUpdate = true;
    cr1Inst.instanceMatrix.needsUpdate = true;
    cr2Inst.instanceMatrix.needsUpdate = true;

    trainHallGroup.add(dTrunkInst, cr1Inst, cr2Inst);
  }

  // 4. Sanfte Hügelketten am fernen Horizont
  const horizonHillsCount = 20;
  for (let h = 0; h < horizonHillsCount; h++) {
    const angle = (h / horizonHillsCount) * Math.PI * 2;
    const dist = 55000 + ((h * 1321) % 40000);
    const hx = cx + Math.cos(angle) * dist;
    const hz = cz + Math.sin(angle) * dist;
    const radius = 9000 + ((h * 843) % 7000);
    const height = 2800 + ((h * 617) % 3200);

    const hillGeo = new THREE.CylinderGeometry(0, radius, height, 16, 1, false);
    const hillMesh = new THREE.Mesh(hillGeo, hillHorizonMat);
    hillMesh.position.set(hx, floorY + height / 2 - 100, hz);
    trainHallGroup.add(hillMesh);
  }

  // 5. Kleine Zierbüsche am Hallensockel (InstancedMesh)
  const bushGeo = new THREE.DodecahedronGeometry(55, 1);
  const bushInst = new THREE.InstancedMesh(bushGeo, bushMat, 18);
  for (let k = 0; k < 18; k++) {
    const side = k % 2 === 0 ? -1 : 1;
    const bz = cz - halfL + k * (lHall / 16);
    const bx = cx + side * (halfW + 120);
    dummy.position.set(bx, floorY + 30, bz);
    dummy.rotation.set(0, k * 0.4, 0);
    dummy.scale.set(1, 1, 1);
    dummy.updateMatrix();
    bushInst.setMatrixAt(k, dummy.matrix);
  }
  bushInst.instanceMatrix.needsUpdate = true;
  trainHallGroup.add(bushInst);
}

/**
 * Idyllischer kleiner See / Bergsee vor den Hügeln auf der offenen Wiesenseite (Ostseite)
 */
export function buildTrainHallLake(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  _wHall: number,
  _lHall: number,
  floorY: number,
  isDark: boolean,
) {
  const lakeX = cx + 25000;
  const lakeZ = cz - 2500;
  const lakeRadiusX = 5200;
  const lakeRadiusZ = 3800;

  // 1. Organische Ufer- und Wasserform
  const lakeShape = new THREE.Shape();
  const pointsCount = 32;
  for (let i = 0; i < pointsCount; i++) {
    const angle = (i / pointsCount) * Math.PI * 2;
    const noise = 1 + Math.sin(angle * 2) * 0.14 + Math.cos(angle * 3) * 0.08 + Math.sin(angle * 5) * 0.05;
    const px = Math.cos(angle) * lakeRadiusX * noise;
    const pz = Math.sin(angle) * lakeRadiusZ * noise;
    if (i === 0) {
      lakeShape.moveTo(px, pz);
    } else {
      lakeShape.lineTo(px, pz);
    }
  }
  lakeShape.closePath();

  // 2. Kiesel- und Sandstrand um den See
  const beachMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x3f3f46 : 0xa89f81,
    roughness: 0.95,
    metalness: 0.05,
    polygonOffset: true,
    polygonOffsetFactor: -1,
    polygonOffsetUnits: -1,
  });
  const beachGeo = new THREE.ShapeGeometry(lakeShape);
  beachGeo.rotateX(-Math.PI / 2);
  const beachMesh = new THREE.Mesh(beachGeo, beachMat);
  beachMesh.scale.set(1.15, 1, 1.15);
  beachMesh.position.set(lakeX, floorY + 14, lakeZ);
  beachMesh.receiveShadow = true;
  trainHallGroup.add(beachMesh);

  // 3. Wasserfläche
  const waterMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x0369a1 : 0x0284c7,
    roughness: 0.08,
    metalness: 0.85,
    transparent: true,
    opacity: 0.88,
    polygonOffset: true,
    polygonOffsetFactor: -2,
    polygonOffsetUnits: -2,
  });
  const waterGeo = new THREE.ShapeGeometry(lakeShape);
  waterGeo.rotateX(-Math.PI / 2);
  const waterMesh = new THREE.Mesh(waterGeo, waterMat);
  waterMesh.position.set(lakeX, floorY + 18, lakeZ);
  waterMesh.receiveShadow = true;
  trainHallGroup.add(waterMesh);

  // 4. Ufersteine & Granitblöcke (Findlinge) am Wasser (InstancedMesh)
  const rockMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x334155 : 0x64748b,
    roughness: 0.9,
  });
  const rocksCount = 18;
  const rockGeo = new THREE.DodecahedronGeometry(100, 1);
  const rockInst = new THREE.InstancedMesh(rockGeo, rockMat, rocksCount);
  const dummy = new THREE.Object3D();

  for (let r = 0; r < rocksCount; r++) {
    const angle = (r / rocksCount) * Math.PI * 2;
    const noise = 1 + Math.sin(angle * 2) * 0.14 + Math.cos(angle * 3) * 0.08;
    const distOffset = 1.02 + ((r * 17) % 15) * 0.01;
    const rx = lakeX + Math.cos(angle) * (lakeRadiusX * noise * distOffset);
    const rz = lakeZ + Math.sin(angle) * (lakeRadiusZ * noise * distOffset);
    const rRadius = 90 + ((r * 29) % 120);
    const s = rRadius / 100;

    dummy.position.set(rx, floorY + 16 + rRadius * 0.45, rz);
    dummy.scale.set(1.3 * s, 0.75 * s, 1.1 * s);
    dummy.rotation.set(0, (r * 37) % 3, 0);
    dummy.updateMatrix();
    rockInst.setMatrixAt(r, dummy.matrix);
  }
  rockInst.instanceMatrix.needsUpdate = true;
  trainHallGroup.add(rockInst);

  // 5. Schilfgürtel & Binsen in den seichten Uferbuchten (InstancedMesh)
  const reedMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x064e3b : 0x15803d,
    roughness: 0.9,
  });

  const reedBeds = [
    { baseAngle: Math.PI * 0.25, count: 20 },
    { baseAngle: Math.PI * 0.85, count: 18 },
    { baseAngle: Math.PI * 1.6, count: 22 },
  ];
  const totalReeds = reedBeds.reduce((acc, b) => acc + b.count, 0);

  const stemGeo = new THREE.CylinderGeometry(2, 3.5, 1, 5);
  stemGeo.translate(0, 0.5, 0);
  const stemInst = new THREE.InstancedMesh(stemGeo, reedMat, totalReeds);

  let reedIdx = 0;
  for (const bed of reedBeds) {
    for (let s = 0; s < bed.count; s++) {
      const angle = bed.baseAngle + ((s - bed.count / 2) / bed.count) * 0.35;
      const noise = 1 + Math.sin(angle * 2) * 0.14 + Math.cos(angle * 3) * 0.08;
      const offset = 0.94 + ((s * 13) % 10) * 0.012;
      const sx = lakeX + Math.cos(angle) * (lakeRadiusX * noise * offset);
      const sz = lakeZ + Math.sin(angle) * (lakeRadiusZ * noise * offset);
      const stemH = 160 + ((s * 23) % 80);

      dummy.position.set(sx, floorY + 16, sz);
      dummy.scale.set(1, stemH, 1);
      dummy.rotation.set(0, 0, (((s * 11) % 7) - 3) * 0.04);
      dummy.updateMatrix();
      stemInst.setMatrixAt(reedIdx++, dummy.matrix);
    }
  }
  stemInst.instanceMatrix.needsUpdate = true;
  trainHallGroup.add(stemInst);

  // 6. Kleiner hölzerner Boots- & Badesteg (ragt in den See)
  const pierMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.85 });
  const pierX = lakeX - lakeRadiusX * 0.7;
  const pierZ = lakeZ + lakeRadiusZ * 0.2;
  const pierLen = 1400;
  const pierW = 380;

  const pilingGeo = new THREE.CylinderGeometry(10, 12, 120, 6);
  for (const dx of [pierLen * 0.35, pierLen * 0.85]) {
    for (const dz of [-pierW * 0.35, pierW * 0.35]) {
      const piling = new THREE.Mesh(pilingGeo, pierMat);
      piling.position.set(pierX + dx, floorY + 16 + 45, pierZ + dz);
      trainHallGroup.add(piling);
    }
  }

  const pierBeamGeo = new THREE.BoxGeometry(pierLen, 14, 18);
  for (const dz of [-pierW * 0.32, pierW * 0.32]) {
    const beam = new THREE.Mesh(pierBeamGeo, pierMat);
    beam.position.set(pierX + pierLen / 2, floorY + 18 + 80, pierZ + dz);
    trainHallGroup.add(beam);
  }

  const plankCount = Math.floor(pierLen / 55);
  const plankGeo = new THREE.BoxGeometry(45, 10, pierW);
  const plankInst = new THREE.InstancedMesh(plankGeo, pierMat, plankCount);
  for (let p = 0; p < plankCount; p++) {
    dummy.position.set(pierX + 30 + p * 55, floorY + 18 + 92, pierZ);
    dummy.scale.set(1, 1, 1);
    dummy.rotation.set(0, 0, 0);
    dummy.updateMatrix();
    plankInst.setMatrixAt(p, dummy.matrix);
  }
  plankInst.instanceMatrix.needsUpdate = true;
  trainHallGroup.add(plankInst);

  // 7. Kleines Holz-Ruderboot (am Steg vertäut)
  const boatGroup = new THREE.Group();
  boatGroup.position.set(pierX + pierLen * 0.85, floorY + 18 + 12, pierZ + pierW * 1.15);
  boatGroup.rotation.y = 0.25;

  const hullMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });
  const hull = new THREE.Mesh(new THREE.BoxGeometry(550, 80, 220), hullMat);
  hull.position.set(0, 30, 0);
  boatGroup.add(hull);

  const bow = new THREE.Mesh(new THREE.ConeGeometry(110, 160, 4), hullMat);
  bow.rotation.z = -Math.PI / 2;
  bow.position.set(340, 30, 0);
  boatGroup.add(bow);

  const bench = new THREE.Mesh(new THREE.BoxGeometry(35, 8, 200), pierMat);
  bench.position.set(0, 55, 0);
  boatGroup.add(bench);

  trainHallGroup.add(boatGroup);

  // 8. Seerosenblätter auf der ruhigen Wasseroberfläche
  const lilyMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x166534 : 0x22c55e,
    roughness: 0.8,
  });
  const lilyFlowerMat = new THREE.MeshBasicMaterial({ color: 0xfdf4ff });

  const lilyClusters = [
    { x: lakeX + lakeRadiusX * 0.35, z: lakeZ - lakeRadiusZ * 0.4 },
    { x: lakeX - lakeRadiusX * 0.25, z: lakeZ - lakeRadiusZ * 0.5 },
    { x: lakeX + lakeRadiusX * 0.5, z: lakeZ + lakeRadiusZ * 0.25 },
  ];

  for (const cl of lilyClusters) {
    for (let l = 0; l < 4; l++) {
      const lx = cl.x + ((l * 47) % 110) - 55;
      const lz = cl.z + ((l * 67) % 110) - 55;
      const pad = new THREE.Mesh(new THREE.CircleGeometry(40 + (l % 3) * 12, 8), lilyMat);
      pad.rotation.x = -Math.PI / 2;
      pad.position.set(lx, floorY + 18 + 1.2, lz);
      trainHallGroup.add(pad);

      if (l === 0 || l === 2) {
        const flower = new THREE.Mesh(new THREE.DodecahedronGeometry(12, 0), lilyFlowerMat);
        flower.position.set(lx, floorY + 18 + 8, lz);
        trainHallGroup.add(flower);
      }
    }
  }
}
