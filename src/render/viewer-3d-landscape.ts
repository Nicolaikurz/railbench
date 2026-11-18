import * as THREE from "three";
import type { Layout } from "@/core/types";
import { getBaseboardBounds, getBaseboards } from "@/core/layout";

export function buildRealisticLandscape(
  landscapeGroup: THREE.Group,
  layout: Layout,
  isDark: boolean,
) {
  let minX = 0,
    maxX = 0,
    minZ = 0,
    maxZ = 0;
  let hasBounds = false;

  const baseboards = getBaseboards(layout);
  for (const b of baseboards) {
    if (!b.enabled) continue;
    const bounds = getBaseboardBounds(b);
    const bMinX = bounds.x;
    const bMaxX = bounds.x + bounds.width;
    const bMinZ = bounds.y;
    const bMaxZ = bounds.y + bounds.depth;
    if (!hasBounds) {
      minX = bMinX;
      maxX = bMaxX;
      minZ = bMinZ;
      maxZ = bMaxZ;
      hasBounds = true;
    } else {
      if (bMinX < minX) minX = bMinX;
      if (bMaxX > maxX) maxX = bMaxX;
      if (bMinZ < minZ) minZ = bMinZ;
      if (bMaxZ > maxZ) maxZ = bMaxZ;
    }
  }

  if (!hasBounds && layout.pieces.length > 0) {
    for (const p of layout.pieces) {
      if (!hasBounds) {
        minX = p.transform.x;
        maxX = p.transform.x;
        minZ = p.transform.y;
        maxZ = p.transform.y;
        hasBounds = true;
      } else {
        if (p.transform.x < minX) minX = p.transform.x;
        if (p.transform.x > maxX) maxX = p.transform.x;
        if (p.transform.y < minZ) minZ = p.transform.y;
        if (p.transform.y > maxZ) maxZ = p.transform.y;
      }
    }
    minX -= 250;
    maxX += 250;
    minZ -= 250;
    maxZ += 250;
  }

  const cx = (minX + maxX) / 2;
  const cz = (minZ + maxZ) / 2;
  const wTable = Math.max(1400, maxX - minX);
  const lTable = Math.max(1200, maxZ - minZ);
  const floorY = -755; // Niveau unter den 720 mm Tischbeinen

  // Gemeinsame Landschafts-Materialien
  const meadowColor = isDark ? 0x1e3a1f : 0x4d7c0f;
  const hillColor = isDark ? 0x142817 : 0x3f6212;
  const rockColor = isDark ? 0x334155 : 0x64748b;
  const snowColor = isDark ? 0x94a3b8 : 0xf8fafc;
  const riverColor = isDark ? 0x0369a1 : 0x0284c7;
  const roadColor = isDark ? 0x18181b : 0x27272a;
  const treeBarkColor = 0x3f2e21;
  const pineGreen1 = isDark ? 0x052e16 : 0x14532d;
  const leafGreen1 = isDark ? 0x166534 : 0x15803d;

  const meadowMat = new THREE.MeshStandardMaterial({ color: meadowColor, roughness: 0.9, metalness: 0.05 });
  const hillMat = new THREE.MeshStandardMaterial({ color: hillColor, roughness: 0.92, metalness: 0.04 });
  const rockMat = new THREE.MeshStandardMaterial({ color: rockColor, roughness: 0.95, metalness: 0.1 });
  const snowMat = new THREE.MeshStandardMaterial({ color: snowColor, roughness: 0.8, metalness: 0.05 });
  const riverMat = new THREE.MeshStandardMaterial({
    color: riverColor,
    roughness: 0.1,
    metalness: 0.8,
    transparent: true,
    opacity: 0.88,
  });
  const riverBankMat = new THREE.MeshStandardMaterial({ color: 0x78716c, roughness: 0.9 });
  const roadMat = new THREE.MeshStandardMaterial({ color: roadColor, roughness: 0.85, metalness: 0.1 });
  const roadMarkMat = new THREE.MeshBasicMaterial({ color: 0xf8fafc });
  const bridgeStoneMat = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.9 });
  const barkMat = new THREE.MeshStandardMaterial({ color: treeBarkColor, roughness: 0.9 });
  const pineMat1 = new THREE.MeshStandardMaterial({ color: pineGreen1, roughness: 0.95 });
  const leafMat1 = new THREE.MeshStandardMaterial({ color: leafGreen1, roughness: 0.9 });

  // 1. Weitläufiges Talboden-Gelände bis zum Horizont (Radius 180.000 mm, 32 Segmente reichen für den Horizont)
  const worldPlaneGeo = new THREE.CircleGeometry(180000, 32);
  worldPlaneGeo.rotateX(-Math.PI / 2);
  const worldPlane = new THREE.Mesh(worldPlaneGeo, meadowMat);
  worldPlane.position.set(cx, floorY - 2, cz);
  worldPlane.receiveShadow = true;
  landscapeGroup.add(worldPlane);

  // 2. Organischer Geländesockel / Böschungsübergang der Anlage (Plateau)
  buildLandscapeEmbankment(landscapeGroup, minX, maxX, minZ, maxZ, floorY, meadowMat, rockMat);

  // 3. Hügelketten & Berge (Rolling Hills & Mountain Range)
  buildLandscapeHills(landscapeGroup, cx, cz, wTable, lTable, floorY, hillMat, rockMat, snowMat);

  // 4. Gebirgsfluss mit Ufer
  buildLandscapeRiver(landscapeGroup, cx, cz, wTable, lTable, floorY, riverMat, riverBankMat);

  // 5. Steinbogenbrücke über den Fluss
  buildLandscapeBridge(landscapeGroup, cx, cz, wTable, lTable, floorY, bridgeStoneMat, roadMat);

  // 6. Geschwungene Landstraße & Allee
  buildLandscapeRoad(landscapeGroup, cx, cz, wTable, lTable, floorY, roadMat, roadMarkMat);

  // 7. Wälder & Baumgruppen (Mischwälder)
  buildLandscapeForest(landscapeGroup, cx, cz, wTable, lTable, floorY, barkMat, pineMat1, leafMat1);

  // 8. Almhütte in den Hügeln
  buildLandscapeCottage(landscapeGroup, cx, cz, wTable, lTable, floorY);

  // 9. 3D-Wolken am Himmel
  buildLandscapeClouds(landscapeGroup, cx, cz);
}

/** Natürliche Geländeböschung vom Anlagentisch nach unten ins Tal */
function buildLandscapeEmbankment(
  landscapeGroup: THREE.Group,
  minX: number,
  maxX: number,
  minZ: number,
  maxZ: number,
  floorY: number,
  meadowMat: THREE.Material,
  rockMat: THREE.Material,
) {
  const slopeOut = 550;
  const topY = -15;
  const botY = floorY;
  const height = topY - botY;

  // 4 Böschungskeile um den Rand des Anlagentisches
  // Nord-Böschung
  const wN = maxX - minX + slopeOut * 2;
  const slopeNGeo = new THREE.BoxGeometry(wN, height, slopeOut);
  const slopeN = new THREE.Mesh(slopeNGeo, meadowMat);
  slopeN.position.set((minX + maxX) / 2, (topY + botY) / 2, minZ - slopeOut / 2);
  slopeN.rotation.x = -0.35;
  slopeN.receiveShadow = true;
  landscapeGroup.add(slopeN);

  // Süd-Böschung
  const slopeS = new THREE.Mesh(slopeNGeo, meadowMat);
  slopeS.position.set((minX + maxX) / 2, (topY + botY) / 2, maxZ + slopeOut / 2);
  slopeS.rotation.x = 0.35;
  slopeS.receiveShadow = true;
  landscapeGroup.add(slopeS);

  // West-Böschung
  const lW = maxZ - minZ;
  const slopeWGeo = new THREE.BoxGeometry(slopeOut, height, lW);
  const slopeW = new THREE.Mesh(slopeWGeo, meadowMat);
  slopeW.position.set(minX - slopeOut / 2, (topY + botY) / 2, (minZ + maxZ) / 2);
  slopeW.rotation.z = 0.35;
  slopeW.receiveShadow = true;
  landscapeGroup.add(slopeW);

  // Ost-Böschung
  const slopeE = new THREE.Mesh(slopeWGeo, meadowMat);
  slopeE.position.set(maxX + slopeOut / 2, (topY + botY) / 2, (minZ + maxZ) / 2);
  slopeE.rotation.z = -0.35;
  slopeE.receiveShadow = true;
  landscapeGroup.add(slopeE);

  // Felsvorsprünge / Granitblöcke an den Ecken
  const rockCorners = [
    { x: minX - 100, z: minZ - 100, r: 65 },
    { x: maxX + 100, z: minZ - 100, r: 75 },
    { x: minX - 100, z: maxZ + 100, r: 70 },
    { x: maxX + 100, z: maxZ + 100, r: 80 },
  ];
  for (const rc of rockCorners) {
    const rock = new THREE.Mesh(new THREE.DodecahedronGeometry(rc.r, 1), rockMat);
    rock.scale.set(1.4, 0.8, 1.2);
    rock.position.set(rc.x, botY + 45, rc.z);
    rock.castShadow = true;
    rock.receiveShadow = true;
    landscapeGroup.add(rock);
  }
}

/** Hügelketten im Mittelgrund und Bergmassiv im Hintergrund */
function buildLandscapeHills(
  landscapeGroup: THREE.Group,
  cx: number,
  cz: number,
  wTable: number,
  lTable: number,
  floorY: number,
  hillMat: THREE.Material,
  rockMat: THREE.Material,
  snowMat: THREE.Material,
) {
  // A. Sanft geschwungene grüne Hügel im Osten & Westen
  const hills = [
    // Östliche Hügelkette
    { x: cx + wTable * 0.85 + 1800, z: cz - lTable * 0.4, r: 3200, h: 850 },
    { x: cx + wTable * 0.95 + 2600, z: cz + lTable * 0.35, r: 3600, h: 950 },
    { x: cx + wTable * 0.75 + 1200, z: cz + lTable * 0.8, r: 2800, h: 720 },
    // Westliche Hügelkette
    { x: cx - wTable * 0.85 - 1900, z: cz - lTable * 0.3, r: 3400, h: 920 },
    { x: cx - wTable * 0.95 - 2700, z: cz + lTable * 0.4, r: 3800, h: 980 },
    { x: cx - wTable * 0.75 - 1300, z: cz - lTable * 0.75, r: 2900, h: 740 },
  ];

  for (const h of hills) {
    const hillGeo = new THREE.CylinderGeometry(0, h.r, h.h, 14, 1, false);
    const hillMesh = new THREE.Mesh(hillGeo, hillMat);
    hillMesh.position.set(h.x, floorY + h.h / 2, h.z);
    hillMesh.receiveShadow = true;
    hillMesh.castShadow = true;
    landscapeGroup.add(hillMesh);
  }

  // B. Majestätisches Alpen- & Mittelgebirgsmassiv im Norden (Hintergrund bis zum Horizont)
  const northZ = cz - lTable * 0.8 - 4800;
  const mountains = [
    { x: cx - 24000, z: northZ - 12000, r: 8500, h: 5400, snow: true },
    { x: cx - 15000, z: northZ - 6000, r: 7200, h: 4800, snow: true },
    { x: cx - 7500, z: northZ - 2200, r: 5600, h: 3800 },
    { x: cx - 3500, z: northZ - 1200, r: 6200, h: 4400, snow: true },
    { x: cx + 500, z: northZ - 1800, r: 6800, h: 5000, snow: true },
    { x: cx + 4500, z: northZ - 1000, r: 5800, h: 4100, snow: true },
    { x: cx + 8500, z: northZ - 2000, r: 5400, h: 3600 },
    { x: cx + 16000, z: northZ - 7000, r: 7600, h: 4900, snow: true },
    { x: cx + 26000, z: northZ - 14000, r: 9000, h: 5600, snow: true },
  ];

  // C. Sanfte Hügelketten am Horizont rundherum
  for (let a = 0; a < 14; a++) {
    const angle = (a / 14) * Math.PI * 2;
    // Nordbereich ist bereits durch das Alpenmassiv bedeckt
    if (angle > Math.PI * 0.8 && angle < Math.PI * 1.2) continue;
    const dist = 50000 + ((a * 2417) % 35000);
    const hx = cx + Math.cos(angle) * dist;
    const hz = cz + Math.sin(angle) * dist;
    const hr = 10000 + ((a * 1523) % 8000);
    const hh = 3200 + ((a * 911) % 3500);

    const hGeo = new THREE.CylinderGeometry(0, hr, hh, 10, 1, false);
    const hMesh = new THREE.Mesh(hGeo, hillMat);
    hMesh.position.set(hx, floorY + hh / 2 - 100, hz);
    landscapeGroup.add(hMesh);
  }

  for (const m of mountains) {
    // Felsenkegel
    const mountainGeo = new THREE.ConeGeometry(m.r, m.h, 10);
    const mountainMesh = new THREE.Mesh(mountainGeo, rockMat);
    mountainMesh.position.set(m.x, floorY + m.h / 2, m.z);
    mountainMesh.receiveShadow = true;
    mountainMesh.castShadow = true;
    landscapeGroup.add(mountainMesh);

    // Schneekappe auf den höchsten Gipfeln
    if (m.snow) {
      const snowH = m.h * 0.28;
      const snowR = m.r * 0.28;
      const snowGeo = new THREE.ConeGeometry(snowR, snowH, 10);
      const snowMesh = new THREE.Mesh(snowGeo, snowMat);
      snowMesh.position.set(m.x, floorY + m.h - snowH / 2, m.z);
      landscapeGroup.add(snowMesh);
    }
  }
}

/** Wunderschöner, geschwungener Gebirgsfluss durch das Tal */
function buildLandscapeRiver(
  landscapeGroup: THREE.Group,
  cx: number,
  cz: number,
  wTable: number,
  lTable: number,
  floorY: number,
  riverMat: THREE.Material,
  riverBankMat: THREE.Material,
) {
  const curvePoints = [
    new THREE.Vector3(cx + wTable * 0.9 + 3000, floorY + 8, cz - lTable * 1.5),
    new THREE.Vector3(cx + wTable * 0.7 + 1800, floorY + 8, cz - lTable * 0.7),
    new THREE.Vector3(cx + wTable * 0.55 + 1100, floorY + 8, cz + lTable * 0.1), // Hier Brücke
    new THREE.Vector3(cx + wTable * 0.35 + 500, floorY + 8, cz + lTable * 0.8),
    new THREE.Vector3(cx - wTable * 0.15, floorY + 8, cz + lTable * 1.45),
    new THREE.Vector3(cx - wTable * 0.7 - 1200, floorY + 8, cz + lTable * 1.9),
  ];

  const riverCurve = new THREE.CatmullRomCurve3(curvePoints);
  const riverGeo = new THREE.TubeGeometry(riverCurve, 20, 160, 6, false);
  riverGeo.scale(1, 0.08, 1); // Flaches Wasserband
  const riverMesh = new THREE.Mesh(riverGeo, riverMat);
  riverMesh.position.set(0, 0, 0);
  landscapeGroup.add(riverMesh);

  // Kiesel-/Sand-Ufer auf beiden Seiten
  const bankGeo = new THREE.TubeGeometry(riverCurve, 20, 210, 6, false);
  bankGeo.scale(1, 0.05, 1);
  const bankMesh = new THREE.Mesh(bankGeo, riverBankMat);
  bankMesh.position.set(0, -3, 0);
  landscapeGroup.add(bankMesh);
}

/** Historische Stein-Bogenbrücke über den Fluss */
function buildLandscapeBridge(
  landscapeGroup: THREE.Group,
  cx: number,
  cz: number,
  wTable: number,
  lTable: number,
  floorY: number,
  stoneMat: THREE.Material,
  roadMat: THREE.Material,
) {
  const bridgeX = cx + wTable * 0.55 + 1100;
  const bridgeZ = cz + lTable * 0.1;
  const bridgeY = floorY + 85;
  const bridgeW = 160;
  const bridgeLen = 520;
  const rotY = Math.PI / 4;

  const bridgeGroup = new THREE.Group();
  bridgeGroup.position.set(bridgeX, bridgeY, bridgeZ);
  bridgeGroup.rotation.y = rotY;

  // Brückenfahrbahn
  const deckGeo = new THREE.BoxGeometry(bridgeW, 16, bridgeLen);
  const deck = new THREE.Mesh(deckGeo, roadMat);
  bridgeGroup.add(deck);

  // Stein-Wangen / Bögen
  const archMat = stoneMat;
  const sideWallL = new THREE.Mesh(new THREE.BoxGeometry(16, 75, bridgeLen), archMat);
  sideWallL.position.set(-bridgeW / 2 + 8, -25, 0);
  bridgeGroup.add(sideWallL);

  const sideWallR = new THREE.Mesh(new THREE.BoxGeometry(16, 75, bridgeLen), archMat);
  sideWallR.position.set(bridgeW / 2 - 8, -25, 0);
  bridgeGroup.add(sideWallR);

  // 2 Bogen-Gewölbedurchlässe
  const archCut1 = new THREE.Mesh(new THREE.CylinderGeometry(75, 75, bridgeW, 16, 1, false, 0, Math.PI), stoneMat);
  archCut1.rotateZ(Math.PI / 2);
  archCut1.position.set(0, -45, -120);
  bridgeGroup.add(archCut1);

  const archCut2 = new THREE.Mesh(new THREE.CylinderGeometry(75, 75, bridgeW, 16, 1, false, 0, Math.PI), stoneMat);
  archCut2.rotateZ(Math.PI / 2);
  archCut2.position.set(0, -45, 120);
  bridgeGroup.add(archCut2);

  // Brückengeländer oben (Brüstung aus Naturstein)
  const parapetGeo = new THREE.BoxGeometry(14, 28, bridgeLen + 20);
  const parapetL = new THREE.Mesh(parapetGeo, archMat);
  parapetL.position.set(-bridgeW / 2 + 7, 20, 0);
  parapetL.castShadow = true;
  bridgeGroup.add(parapetL);

  const parapetR = new THREE.Mesh(parapetGeo, archMat);
  parapetR.position.set(bridgeW / 2 - 7, 20, 0);
  parapetR.castShadow = true;
  bridgeGroup.add(parapetR);

  landscapeGroup.add(bridgeGroup);
}

/** Malerische Landstraße durch das Tal & Allee mit Birken/Bäumen */
function buildLandscapeRoad(
  landscapeGroup: THREE.Group,
  cx: number,
  cz: number,
  wTable: number,
  lTable: number,
  floorY: number,
  roadMat: THREE.Material,
  markMat: THREE.Material,
) {
  const roadPoints = [
    new THREE.Vector3(cx - wTable * 0.9 - 2200, floorY + 4, cz + lTable * 0.8),
    new THREE.Vector3(cx - wTable * 0.4, floorY + 4, cz + lTable * 0.65),
    new THREE.Vector3(cx + wTable * 0.15, floorY + 4, cz + lTable * 0.45),
    new THREE.Vector3(cx + wTable * 0.55 + 1100, floorY + 80, cz + lTable * 0.1), // Über die Brücke
    new THREE.Vector3(cx + wTable * 0.85 + 1900, floorY + 4, cz - lTable * 0.25),
    new THREE.Vector3(cx + wTable * 1.1 + 3200, floorY + 4, cz - lTable * 0.8),
  ];

  const roadCurve = new THREE.CatmullRomCurve3(roadPoints);
  const roadGeo = new THREE.TubeGeometry(roadCurve, 20, 75, 4, false);
  roadGeo.scale(1, 0.05, 1);
  const roadMesh = new THREE.Mesh(roadGeo, roadMat);
  roadMesh.receiveShadow = true;
  landscapeGroup.add(roadMesh);

  // Weiße Mittellinie (gestrichelt)
  const samples = 35;
  for (let i = 0; i < samples; i++) {
    if (i % 2 === 0) continue;
    const t = i / samples;
    const pt = roadCurve.getPoint(t);
    const mark = new THREE.Mesh(new THREE.BoxGeometry(6, 2, 45), markMat);
    mark.position.set(pt.x, pt.y + 2, pt.z);
    const tangent = roadCurve.getTangent(t);
    mark.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), tangent);
    landscapeGroup.add(mark);
  }
}

/** Mischwälder (Nadel- und Laubbäume) in der Landschaft verteilt */
function buildLandscapeForest(
  landscapeGroup: THREE.Group,
  cx: number,
  cz: number,
  wTable: number,
  lTable: number,
  floorY: number,
  barkMat: THREE.Material,
  pineMat1: THREE.Material,
  leafMat1: THREE.Material,
) {
  // A. Nadelwald-Haine an den Hängen
  const pineClusters = [
    // Nordhang (Waldsaum am Gebirgsfuß)
    { cx: cx - 2200, cz: cz - lTable * 0.7 - 2400, count: 18, spread: 2200 },
    { cx: cx + 2400, cz: cz - lTable * 0.7 - 2600, count: 22, spread: 2500 },
    // Osthang
    { cx: cx + wTable * 0.95 + 2400, cz: cz + 300, count: 16, spread: 1800 },
    // Westhang
    { cx: cx - wTable * 0.95 - 2400, cz: cz - 400, count: 16, spread: 1800 },
  ];

  const pinePoints: { x: number; z: number; scale: number; isMat2: boolean }[] = [];
  for (const cl of pineClusters) {
    for (let i = 0; i < cl.count; i++) {
      const angle = (i / cl.count) * Math.PI * 2 + (i % 3);
      const dist = 300 + Math.sin(i * 1.7) * (cl.spread * 0.45);
      const px = cl.cx + Math.cos(angle) * dist;
      const pz = cl.cz + Math.sin(angle) * dist;
      const scale = 0.85 + (i % 5) * 0.12;
      pinePoints.push({ x: px, z: pz, scale, isMat2: i % 2 !== 0 });
    }
  }

  const trunkGeo = new THREE.CylinderGeometry(4, 7, 45, 8);
  trunkGeo.translate(0, 22.5, 0);
  const c1Geo = new THREE.ConeGeometry(32, 45, 8);
  c1Geo.translate(0, 50, 0);
  const c2Geo = new THREE.ConeGeometry(25, 40, 8);
  c2Geo.translate(0, 75, 0);
  const c3Geo = new THREE.ConeGeometry(17, 35, 8);
  c3Geo.translate(0, 98, 0);

  const dummy = new THREE.Object3D();

  if (pinePoints.length > 0) {
    const trunkInst = new THREE.InstancedMesh(trunkGeo, barkMat, pinePoints.length);
    const c1Inst1 = new THREE.InstancedMesh(c1Geo, pineMat1, pinePoints.length);
    const c2Inst1 = new THREE.InstancedMesh(c2Geo, pineMat1, pinePoints.length);
    const c3Inst1 = new THREE.InstancedMesh(c3Geo, pineMat1, pinePoints.length);

    for (let i = 0; i < pinePoints.length; i++) {
      const p = pinePoints[i]!;
      dummy.position.set(p.x, floorY, p.z);
      dummy.scale.setScalar(p.scale);
      dummy.rotation.set(0, 0, 0);
      dummy.updateMatrix();

      trunkInst.setMatrixAt(i, dummy.matrix);
      c1Inst1.setMatrixAt(i, dummy.matrix);
      c2Inst1.setMatrixAt(i, dummy.matrix);
      c3Inst1.setMatrixAt(i, dummy.matrix);
    }

    trunkInst.instanceMatrix.needsUpdate = true;
    c1Inst1.instanceMatrix.needsUpdate = true;
    c2Inst1.instanceMatrix.needsUpdate = true;
    c3Inst1.instanceMatrix.needsUpdate = true;

    landscapeGroup.add(trunkInst, c1Inst1, c2Inst1, c3Inst1);
  }

  // B. Laubbäume entlang des Flusses & der Straße (Auen- & Alleebäume)
  const deciduousPositions = [
    { x: cx + wTable * 0.5 + 850, z: cz - lTable * 0.4, scale: 1.1 },
    { x: cx + wTable * 0.45 + 750, z: cz + lTable * 0.45, scale: 1.25 },
    { x: cx - wTable * 0.35 - 300, z: cz + lTable * 0.95, scale: 1.15 },
    { x: cx - wTable * 0.7 - 700, z: cz + lTable * 1.2, scale: 1.3 },
    { x: cx + wTable * 0.8 + 1400, z: cz - lTable * 0.85, scale: 1.05 },
    { x: cx - wTable * 0.65 - 1100, z: cz - lTable * 0.5, scale: 1.2 },
  ];

  const decTrunkGeo = new THREE.CylinderGeometry(5, 8, 55, 8);
  decTrunkGeo.translate(0, 27.5, 0);
  const crownMainGeo = new THREE.DodecahedronGeometry(42, 1);
  crownMainGeo.translate(0, 85, 0);
  const crownSubGeo = new THREE.DodecahedronGeometry(28, 1);
  crownSubGeo.translate(18, 78, 14);

  const decTrunkInst = new THREE.InstancedMesh(decTrunkGeo, barkMat, deciduousPositions.length);
  const crownMainInst = new THREE.InstancedMesh(crownMainGeo, leafMat1, deciduousPositions.length);
  const crownSubInst = new THREE.InstancedMesh(crownSubGeo, leafMat1, deciduousPositions.length);

  for (let j = 0; j < deciduousPositions.length; j++) {
    const dp = deciduousPositions[j]!;
    dummy.position.set(dp.x, floorY, dp.z);
    dummy.scale.setScalar(dp.scale);
    dummy.rotation.set(0, 0, 0);
    dummy.updateMatrix();

    decTrunkInst.setMatrixAt(j, dummy.matrix);
    crownMainInst.setMatrixAt(j, dummy.matrix);
    crownSubInst.setMatrixAt(j, dummy.matrix);
  }

  decTrunkInst.instanceMatrix.needsUpdate = true;
  crownMainInst.instanceMatrix.needsUpdate = true;
  crownSubInst.instanceMatrix.needsUpdate = true;

  landscapeGroup.add(decTrunkInst, crownMainInst, crownSubInst);
}

/** Malerische Almhütte / Bauernhaus auf dem Osthügel */
function buildLandscapeCottage(
  landscapeGroup: THREE.Group,
  cx: number,
  cz: number,
  wTable: number,
  lTable: number,
  floorY: number,
) {
  const hutX = cx + wTable * 0.85 + 2100;
  const hutZ = cz - lTable * 0.35;
  const hutY = floorY + 480;

  const cottage = new THREE.Group();
  cottage.position.set(hutX, hutY, hutZ);
  cottage.rotation.y = -Math.PI / 6;

  // Steinfundament
  const stoneBase = new THREE.Mesh(
    new THREE.BoxGeometry(160, 25, 120),
    new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.9 }),
  );
  stoneBase.position.set(0, 12.5, 0);
  cottage.add(stoneBase);

  // Blockhaus-Holzwände
  const woodWalls = new THREE.Mesh(
    new THREE.BoxGeometry(150, 65, 110),
    new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.85 }),
  );
  woodWalls.position.set(0, 57.5, 0);
  woodWalls.castShadow = true;
  cottage.add(woodWalls);

  // Schindeldach (Satteldach)
  const roofShape = new THREE.Shape();
  roofShape.moveTo(-65, 0);
  roofShape.lineTo(0, 48);
  roofShape.lineTo(65, 0);
  roofShape.closePath();
  const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 168, bevelEnabled: false });
  roofGeo.rotateY(Math.PI / 2);
  roofGeo.translate(-84, 90, 0);
  const roof = new THREE.Mesh(
    roofGeo,
    new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.7 }),
  );
  roof.castShadow = true;
  cottage.add(roof);

  // Kamin
  const chimney = new THREE.Mesh(
    new THREE.BoxGeometry(18, 55, 18),
    new THREE.MeshStandardMaterial({ color: 0x52525b, roughness: 0.9 }),
  );
  chimney.position.set(30, 115, 0);
  cottage.add(chimney);

  // Warm leuchtendes Fenster
  const win = new THREE.Mesh(
    new THREE.BoxGeometry(16, 20, 2),
    new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.8 }),
  );
  win.position.set(-25, 62, 56);
  cottage.add(win);

  // Kleiner Holzzaun um die Hütte
  const fenceMat = new THREE.MeshStandardMaterial({ color: 0x92400e, roughness: 0.9 });
  for (let f = -90; f <= 90; f += 25) {
    const post = new THREE.Mesh(new THREE.BoxGeometry(4, 26, 4), fenceMat);
    post.position.set(f, 13, 85);
    cottage.add(post);
  }
  const rail = new THREE.Mesh(new THREE.BoxGeometry(190, 3, 2), fenceMat);
  rail.position.set(0, 18, 85);
  cottage.add(rail);

  landscapeGroup.add(cottage);
}

/** Sanfte weiße 3D-Schäfchenwolken am Himmel */
function buildLandscapeClouds(landscapeGroup: THREE.Group, cx: number, cz: number) {
  const cloudMat = new THREE.MeshStandardMaterial({
    color: 0xffffff,
    roughness: 0.95,
    metalness: 0.0,
    flatShading: true,
  });

  const cloudPuffs = [
    { x: cx - 6000, y: 6800, z: cz - 4000, s: 1.2 },
    { x: cx - 1800, y: 7200, z: cz - 7500, s: 1.5 },
    { x: cx + 4500, y: 6500, z: cz - 5000, s: 1.3 },
    { x: cx + 8500, y: 7400, z: cz - 2500, s: 1.1 },
    { x: cx - 7200, y: 6200, z: cz + 3000, s: 1.0 },
    { x: cx + 2500, y: 6900, z: cz + 5500, s: 1.4 },
  ];

  for (const cp of cloudPuffs) {
    const cloud = new THREE.Group();
    cloud.position.set(cp.x, cp.y, cp.z);
    cloud.scale.setScalar(cp.s);

    // Eine Wolke besteht aus 5-6 weichen überlappenden Kugeln
    const subSpheres = [
      { x: 0, y: 0, z: 0, r: 240 },
      { x: 180, y: 40, z: 20, r: 290 },
      { x: -180, y: 30, z: -20, r: 260 },
      { x: 340, y: -20, z: 10, r: 200 },
      { x: -320, y: -15, z: 15, r: 210 },
      { x: 80, y: 90, z: -10, r: 220 },
    ];

    for (const sp of subSpheres) {
      const puff = new THREE.Mesh(new THREE.DodecahedronGeometry(sp.r, 0), cloudMat);
      puff.position.set(sp.x, sp.y, sp.z);
      cloud.add(puff);
    }

    landscapeGroup.add(cloud);
  }
}
