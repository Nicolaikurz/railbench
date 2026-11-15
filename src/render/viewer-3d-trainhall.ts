import * as THREE from "three";
import type { Layout } from "@/core/types";
import { getBaseboardBounds, getBaseboards } from "@/core/layout";
import { buildTrainHallForest, buildTrainHallLake } from "./viewer-3d-trainhall-scenery";

export interface TrainHallBuildResult {
  sceneCenter: { cx: number; floorY: number; cz: number };
  nightLampMats: THREE.MeshStandardMaterial[];
}

export function buildTrainHall(
  trainHallGroup: THREE.Group,
  layout: Layout,
  isDark: boolean,
): TrainHallBuildResult {
  const nightLampMats: THREE.MeshStandardMaterial[] = [];

  // 1. Ausdehnung des Tisches / der Anlage ermitteln
  let minX = -1200;
  let maxX = 1200;
  let minZ = -800;
  let maxZ = 800;

  const baseboards = getBaseboards(layout);
  let hasBounds = false;
  for (const bb of baseboards) {
    if (!bb.enabled) continue;
    const b = getBaseboardBounds(bb);
    const bMinX = b.x;
    const bMaxX = b.x + b.width;
    const bMinZ = b.y;
    const bMaxZ = b.y + b.depth;
    if (!hasBounds) {
      minX = bMinX;
      maxX = bMaxX;
      minZ = bMinZ; // 2D Y ist 3D Z
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
  const wTable = Math.max(1200, maxX - minX);
  const lTable = Math.max(1000, maxZ - minZ);

  // Dimensionen der Lokhalle
  const wHall = Math.max(9000, wTable + 4800);
  const lHall = Math.max(10000, lTable + 5600);
  const floorY = -755; // Niveau unter den 720mm Tischbeinen
  const wallH = floorY + 5200; // Traufhöhe / Stützenhöhe (Y = 1445)
  const apexH = floorY + 6400; // Firsthöhe / Bogenkamm (Y = 3645)

  const steelColor = isDark ? 0x1e293b : 0x2c3e50;
  const brickColor = isDark ? 0x4a2820 : 0x6e382b;
  const concreteColor = isDark ? 0x334155 : 0x64748b;

  // Gemeinsame Materialien
  const floorMat = new THREE.MeshStandardMaterial({
    color: concreteColor,
    roughness: 0.88,
    metalness: 0.08,
    polygonOffset: true,
    polygonOffsetFactor: -1,
    polygonOffsetUnits: -1,
  });
  const yellowLineMat = new THREE.MeshStandardMaterial({
    color: 0xf59e0b,
    roughness: 0.4,
    metalness: 0.2,
  });
  const steelMat = new THREE.MeshStandardMaterial({
    color: steelColor,
    roughness: 0.45,
    metalness: 0.75,
  });
  const craneYellowMat = new THREE.MeshStandardMaterial({
    color: 0xf59e0b,
    roughness: 0.35,
    metalness: 0.4,
  });
  const brickMat = new THREE.MeshStandardMaterial({
    color: brickColor,
    roughness: 0.9,
    metalness: 0.05,
  });
  const skylightGlassMat = new THREE.MeshStandardMaterial({
    color: 0xcffafe,
    transparent: true,
    opacity: 0.32,
    roughness: 0.1,
    metalness: 0.1,
  });
  const windowGlassMat = new THREE.MeshStandardMaterial({
    color: 0x93c5fd,
    transparent: true,
    opacity: 0.38,
    roughness: 0.15,
    metalness: 0.1,
  });
  const roofMat = new THREE.MeshStandardMaterial({
    color: isDark ? 0x1e293b : 0x334155,
    roughness: 0.7,
    metalness: 0.4,
  });
  const sleeperMat = new THREE.MeshStandardMaterial({
    color: 0x27272a,
    roughness: 0.8,
  });
  const railMat = new THREE.MeshStandardMaterial({
    color: 0x94a3b8,
    roughness: 0.25,
    metalness: 0.85,
  });

  // 1. Hallenboden, Sicherheitsmarkierungen & Bodengleise
  buildTrainHallFloor(trainHallGroup, cx, cz, wHall, lHall, floorY, minX, maxX, minZ, maxZ, floorMat, yellowLineMat);

  // 2. Stahl-Fachwerkbogen & Stützen (Hallenbogen)
  buildTrainHallArches(trainHallGroup, cx, cz, wHall, lHall, floorY, wallH, apexH, steelMat);

  // 3. Glas-Oberlicht & Zinkdach
  buildTrainHallRoof(trainHallGroup, cx, cz, wHall, lHall, wallH, apexH, skylightGlassMat, roofMat, steelMat);

  // 4. Giebelwände mit Rundbogenfenstern, Hallentoren & Bahnhofsuhr
  buildTrainHallEndWalls(trainHallGroup, cx, cz, wHall, lHall, floorY, wallH, apexH, brickMat, windowGlassMat, steelMat, nightLampMats);

  // 5. Seitenwände mit Industriefenstern
  buildTrainHallSideWalls(trainHallGroup, cx, cz, wHall, lHall, floorY, wallH, brickMat, windowGlassMat, steelMat);

  // 6. Decken-Brückenkran (Laufkran mit Haken)
  buildTrainHallCrane(trainHallGroup, cx, cz, wHall, lHall, floorY, craneYellowMat, steelMat);

  // 7. Industrie-Pendelleuchten mit warmem Schein
  buildTrainHallLamps(trainHallGroup, cx, cz, wHall, lHall, floorY, apexH, steelMat, nightLampMats);

  // 8. Dichter Wald rund um die Lokhalle
  buildTrainHallForest(trainHallGroup, cx, cz, wHall, lHall, floorY, isDark);

  // 9. Idyllischer kleiner See vor den Hügeln
  buildTrainHallLake(trainHallGroup, cx, cz, wHall, lHall, floorY, isDark);

  return {
    sceneCenter: { cx, floorY, cz },
    nightLampMats,
  };
}

function buildTrainHallFloor(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  floorY: number,
  minX: number,
  maxX: number,
  minZ: number,
  maxZ: number,
  floorMat: THREE.Material,
  yellowLineMat: THREE.Material,
) {
  const floorH = 50;
  const floorElev = 16;
  const floorGeo = new THREE.BoxGeometry(wHall + 80, floorH, lHall + 80);
  const floorMesh = new THREE.Mesh(floorGeo, floorMat);
  floorMesh.position.set(cx, floorY + floorElev - floorH / 2, cz);
  floorMesh.receiveShadow = true;
  trainHallGroup.add(floorMesh);

  // Gelbe Sicherheitslinien um den Modellbahntisch
  const margin = 220;
  const boxW = (maxX - minX) + margin * 2;
  const boxL = (maxZ - minZ) + margin * 2;
  const lineThick = 35;
  const lineY = floorY + floorElev + 2.5;

  const lineN = new THREE.Mesh(new THREE.BoxGeometry(boxW, 2, lineThick), yellowLineMat);
  lineN.position.set(cx, lineY, minZ - margin);
  trainHallGroup.add(lineN);

  const lineS = new THREE.Mesh(new THREE.BoxGeometry(boxW, 2, lineThick), yellowLineMat);
  lineS.position.set(cx, lineY, maxZ + margin);
  trainHallGroup.add(lineS);

  const lineW = new THREE.Mesh(new THREE.BoxGeometry(lineThick, 2, boxL), yellowLineMat);
  lineW.position.set(minX - margin, lineY, cz);
  trainHallGroup.add(lineW);

  const lineE = new THREE.Mesh(new THREE.BoxGeometry(lineThick, 2, boxL), yellowLineMat);
  lineE.position.set(maxX + margin, lineY, cz);
  trainHallGroup.add(lineE);
}

function buildTrainHallArches(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  floorY: number,
  wallH: number,
  apexH: number,
  steelMat: THREE.Material,
) {
  const numBays = 6;
  const stepZ = lHall / numBays;
  const halfW = wHall / 2;
  const colHeight = wallH - floorY;

  const colGeo = new THREE.BoxGeometry(70, colHeight, 70);

  const outerCurve = new THREE.CatmullRomCurve3([
    new THREE.Vector3(-halfW, wallH, 0),
    new THREE.Vector3(-halfW * 0.65, (wallH + apexH) / 2 + 260, 0),
    new THREE.Vector3(0, apexH, 0),
    new THREE.Vector3(halfW * 0.65, (wallH + apexH) / 2 + 260, 0),
    new THREE.Vector3(halfW, wallH, 0),
  ]);
  const outerGeo = new THREE.TubeGeometry(outerCurve, 24, 20, 8, false);

  const innerCurve = new THREE.CatmullRomCurve3([
    new THREE.Vector3(-halfW + 160, wallH + 10, 0),
    new THREE.Vector3((-halfW + 160) * 0.65, (wallH + apexH) / 2 + 70, 0),
    new THREE.Vector3(0, apexH - 220, 0),
    new THREE.Vector3((halfW - 160) * 0.65, (wallH + apexH) / 2 + 70, 0),
    new THREE.Vector3(halfW - 160, wallH + 10, 0),
  ]);
  const innerGeo = new THREE.TubeGeometry(innerCurve, 24, 15, 8, false);

  const strutSteps = 9;
  const strutGeos: THREE.CylinderGeometry[] = [];
  const strutTransforms: { midX: number; midY: number; quat: THREE.Quaternion }[] = [];
  const unitY = new THREE.Vector3(0, 1, 0);

  for (let s = 1; s < strutSteps; s++) {
    const t = s / strutSteps;
    const ptOut = outerCurve.getPoint(t);
    const ptIn = innerCurve.getPoint(t);
    const vLen = ptOut.distanceTo(ptIn);

    const sGeo = new THREE.CylinderGeometry(8, 8, vLen, 6);
    strutGeos.push(sGeo);

    const dir = ptOut.clone().sub(ptIn).normalize();
    const quat = new THREE.Quaternion().setFromUnitVectors(unitY, dir);
    strutTransforms.push({
      midX: (ptOut.x + ptIn.x) / 2,
      midY: (ptOut.y + ptIn.y) / 2,
      quat,
    });
  }

  for (let i = 0; i <= numBays; i++) {
    const z = cz - lHall / 2 + i * stepZ;

    const colL = new THREE.Mesh(colGeo, steelMat);
    colL.position.set(cx - halfW, (floorY + wallH) / 2, z);
    trainHallGroup.add(colL);

    const colR = new THREE.Mesh(colGeo, steelMat);
    colR.position.set(cx + halfW, (floorY + wallH) / 2, z);
    trainHallGroup.add(colR);

    const outerMesh = new THREE.Mesh(outerGeo, steelMat);
    outerMesh.position.set(cx, 0, z);
    trainHallGroup.add(outerMesh);

    const innerMesh = new THREE.Mesh(innerGeo, steelMat);
    innerMesh.position.set(cx, 0, z);
    trainHallGroup.add(innerMesh);

    for (let s = 0; s < strutGeos.length; s++) {
      const sGeo = strutGeos[s]!;
      const st = strutTransforms[s]!;
      const strut = new THREE.Mesh(sGeo, steelMat);
      strut.position.set(cx + st.midX, st.midY, z);
      strut.quaternion.copy(st.quat);
      trainHallGroup.add(strut);
    }
  }

  const purlinPositions = [
    { x: 0, y: apexH },
    { x: -halfW * 0.35, y: (wallH + apexH) / 2 + 250 },
    { x: halfW * 0.35, y: (wallH + apexH) / 2 + 250 },
    { x: -halfW, y: wallH },
    { x: halfW, y: wallH },
    { x: -halfW + 80, y: floorY + 1600 },
    { x: halfW - 80, y: floorY + 1600 },
  ];

  const purlinGeo = new THREE.BoxGeometry(24, 28, lHall + 100);
  for (const p of purlinPositions) {
    const purlin = new THREE.Mesh(purlinGeo, steelMat);
    purlin.position.set(cx + p.x, p.y, cz);
    trainHallGroup.add(purlin);
  }
}

function buildTrainHallRoof(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  wallH: number,
  apexH: number,
  skylightGlassMat: THREE.Material,
  roofMat: THREE.Material,
  steelMat: THREE.Material,
) {
  const halfW = wHall / 2;
  const skylightW = wHall * 0.34;
  const shoulderY = (wallH + apexH) / 2 + 260;

  const glassPlaneGeoL = new THREE.BoxGeometry(skylightW / 2 + 10, 8, lHall);
  const glassMeshL = new THREE.Mesh(glassPlaneGeoL, skylightGlassMat);
  const angleL = Math.atan2(apexH - shoulderY, skylightW / 2);
  glassMeshL.position.set(cx - skylightW / 4, (apexH + shoulderY) / 2, cz);
  glassMeshL.rotation.z = angleL;
  trainHallGroup.add(glassMeshL);

  const glassMeshR = new THREE.Mesh(glassPlaneGeoL, skylightGlassMat);
  glassMeshR.position.set(cx + skylightW / 4, (apexH + shoulderY) / 2, cz);
  glassMeshR.rotation.z = -angleL;
  trainHallGroup.add(glassMeshR);

  const mullionSteps = Math.floor(lHall / 500);
  const mullionGeo = new THREE.BoxGeometry(10, 14, 12);
  for (let i = 0; i < mullionSteps; i++) {
    const mz = cz - lHall / 2 + i * 500;
    const mRidge = new THREE.Mesh(mullionGeo, steelMat);
    mRidge.position.set(cx, apexH + 12, mz);
    trainHallGroup.add(mRidge);
  }

  const sideW = halfW - skylightW / 2;
  const sideSlopeGeo = new THREE.BoxGeometry(sideW + 20, 10, lHall);
  const sideAngle = Math.atan2(shoulderY - wallH, sideW);

  const roofL = new THREE.Mesh(sideSlopeGeo, roofMat);
  roofL.position.set(cx - (skylightW / 2 + sideW / 2), (shoulderY + wallH) / 2, cz);
  roofL.rotation.z = sideAngle;
  trainHallGroup.add(roofL);

  const roofR = new THREE.Mesh(sideSlopeGeo, roofMat);
  roofR.position.set(cx + (skylightW / 2 + sideW / 2), (shoulderY + wallH) / 2, cz);
  roofR.rotation.z = -sideAngle;
  trainHallGroup.add(roofR);
}

function buildTrainHallEndWalls(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  floorY: number,
  wallH: number,
  apexH: number,
  brickMat: THREE.Material,
  windowGlassMat: THREE.Material,
  steelMat: THREE.Material,
  nightLampMats: THREE.MeshStandardMaterial[],
) {
  const endZPositions = [cz - lHall / 2, cz + lHall / 2];
  const halfW = wHall / 2;

  for (const z of endZPositions) {
    const isSouthWall = z > cz;
    const lowerWallH = wallH - floorY;

    if (!isSouthWall) {
      const wallMesh = new THREE.Mesh(new THREE.BoxGeometry(wHall, lowerWallH, 50), brickMat);
      wallMesh.position.set(cx, (floorY + wallH) / 2, z);
      wallMesh.receiveShadow = true;
      trainHallGroup.add(wallMesh);
    } else {
      const doorW = 1250;
      const doorH = 2250;
      const sideWallW = (wHall - doorW) / 2;

      const wallL = new THREE.Mesh(new THREE.BoxGeometry(sideWallW, lowerWallH, 50), brickMat);
      wallL.position.set(cx - halfW + sideWallW / 2, (floorY + wallH) / 2, z);
      wallL.receiveShadow = true;
      trainHallGroup.add(wallL);

      const wallR = new THREE.Mesh(new THREE.BoxGeometry(sideWallW, lowerWallH, 50), brickMat);
      wallR.position.set(cx + halfW - sideWallW / 2, (floorY + wallH) / 2, z);
      wallR.receiveShadow = true;
      trainHallGroup.add(wallR);

      const wallAboveH = lowerWallH - doorH;
      const wallAbove = new THREE.Mesh(new THREE.BoxGeometry(doorW, wallAboveH, 50), brickMat);
      wallAbove.position.set(cx, floorY + doorH + wallAboveH / 2, z);
      wallAbove.receiveShadow = true;
      trainHallGroup.add(wallAbove);

      const timberFrameMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.85 });
      const frameL = new THREE.Mesh(new THREE.BoxGeometry(70, doorH + 30, 80), timberFrameMat);
      frameL.position.set(cx - doorW / 2 - 30, floorY + (doorH + 30) / 2, z);
      trainHallGroup.add(frameL);

      const frameR = new THREE.Mesh(new THREE.BoxGeometry(70, doorH + 30, 80), timberFrameMat);
      frameR.position.set(cx + doorW / 2 + 30, floorY + (doorH + 30) / 2, z);
      trainHallGroup.add(frameR);

      const frameTop = new THREE.Mesh(new THREE.BoxGeometry(doorW + 140, 70, 85), timberFrameMat);
      frameTop.position.set(cx, floorY + doorH + 35, z);
      trainHallGroup.add(frameTop);

      const stepMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.75 });
      const step = new THREE.Mesh(new THREE.BoxGeometry(doorW + 180, 24, 420), stepMat);
      step.position.set(cx, floorY + 12, z + 180);
      step.receiveShadow = true;
      trainHallGroup.add(step);

      const doorLeafW = doorW - 30;
      const doorLeafH = doorH - 20;
      const doorGroup = new THREE.Group();
      doorGroup.position.set(cx - doorW / 2 + 20, floorY + 16, z);
      doorGroup.rotation.y = 1.35;

      const woodDoorMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });
      const ironMat = new THREE.MeshStandardMaterial({ color: 0x18181b, roughness: 0.5, metalness: 0.85 });

      const doorLeaf = new THREE.Mesh(new THREE.BoxGeometry(doorLeafW, doorLeafH, 38), woodDoorMat);
      doorLeaf.position.set(doorLeafW / 2, doorLeafH / 2, 0);
      doorLeaf.castShadow = true;
      doorGroup.add(doorLeaf);

      for (let bx = 160; bx < doorLeafW; bx += 180) {
        const groove = new THREE.Mesh(new THREE.BoxGeometry(4, doorLeafH - 20, 42), timberFrameMat);
        groove.position.set(bx, doorLeafH / 2, 0);
        doorGroup.add(groove);
      }

      const hingeOffsetsY = [doorLeafH * 0.2, doorLeafH * 0.5, doorLeafH * 0.82];
      for (const hy of hingeOffsetsY) {
        const hinge = new THREE.Mesh(new THREE.BoxGeometry(doorLeafW * 0.75, 28, 46), ironMat);
        hinge.position.set(doorLeafW * 0.38, hy, 0);
        doorGroup.add(hinge);
      }

      const handlePlate = new THREE.Mesh(new THREE.BoxGeometry(22, 90, 48), ironMat);
      handlePlate.position.set(doorLeafW - 80, doorLeafH * 0.48, 0);
      doorGroup.add(handlePlate);

      const handleRing = new THREE.Mesh(new THREE.TorusGeometry(18, 4, 8, 16), ironMat);
      handleRing.position.set(doorLeafW - 80, doorLeafH * 0.48, 25);
      doorGroup.add(handleRing);

      trainHallGroup.add(doorGroup);

      const lampMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, metalness: 0.85, roughness: 0.35 });
      const lanternBulbMat = new THREE.MeshStandardMaterial({
        color: 0xfef08a,
        emissive: 0xfef08a,
        emissiveIntensity: 1.5,
      });
      nightLampMats.push(lanternBulbMat);

      const wallArm = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 75), lampMat);
      wallArm.position.set(cx, floorY + doorH + 180, z + 40);
      trainHallGroup.add(wallArm);

      const lanternHead = new THREE.Mesh(new THREE.ConeGeometry(24, 28, 6), lampMat);
      lanternHead.position.set(cx, floorY + doorH + 195, z + 75);
      trainHallGroup.add(lanternHead);

      const lanternBulb = new THREE.Mesh(new THREE.SphereGeometry(12, 12, 12), lanternBulbMat);
      lanternBulb.position.set(cx, floorY + doorH + 175, z + 75);
      trainHallGroup.add(lanternBulb);
    }

    const gableH = apexH - wallH;
    const gableGeo = new THREE.BoxGeometry(wHall - 40, gableH, 40);
    const gableMesh = new THREE.Mesh(gableGeo, brickMat);
    gableMesh.position.set(cx, wallH + gableH / 2, z);
    gableMesh.receiveShadow = true;
    trainHallGroup.add(gableMesh);

    const winRadius = wHall * 0.22;
    const winCenterY = wallH + gableH * 0.42;

    const winGlassGeo = new THREE.RingGeometry(0, winRadius, 32, 1, 0, Math.PI);
    const winGlass = new THREE.Mesh(winGlassGeo, windowGlassMat);
    winGlass.position.set(cx, winCenterY, z + (z < cz ? 26 : -26));
    trainHallGroup.add(winGlass);

    const winFrameGeo = new THREE.TorusGeometry(winRadius, 16, 8, 32, Math.PI);
    const winFrame = new THREE.Mesh(winFrameGeo, steelMat);
    winFrame.position.set(cx, winCenterY, z + (z < cz ? 28 : -28));
    trainHallGroup.add(winFrame);

    for (let a = 0; a <= 6; a++) {
      const rad = (a * Math.PI) / 6;
      const spokeGeo = new THREE.BoxGeometry(winRadius, 8, 12);
      const spoke = new THREE.Mesh(spokeGeo, steelMat);
      spoke.position.set(
        cx + (Math.cos(rad) * winRadius) / 2,
        winCenterY + (Math.sin(rad) * winRadius) / 2,
        z + (z < cz ? 28 : -28),
      );
      spoke.rotation.z = rad;
      trainHallGroup.add(spoke);
    }
  }

  // Schweizer Bahnhofsuhr an der Stirnwand
  const clockZ = cz - lHall / 2 + 32;
  const clockY = apexH - 360;

  const clockRim = new THREE.Mesh(
    new THREE.RingGeometry(170, 195, 32),
    new THREE.MeshStandardMaterial({ color: 0x18181b, metalness: 0.8, roughness: 0.3 }),
  );
  clockRim.position.set(cx, clockY, clockZ);
  trainHallGroup.add(clockRim);

  const clockFace = new THREE.Mesh(new THREE.CircleGeometry(170, 32), new THREE.MeshBasicMaterial({ color: 0xf8fafc }));
  clockFace.position.set(cx, clockY, clockZ + 1);
  trainHallGroup.add(clockFace);

  const tickMat = new THREE.MeshBasicMaterial({ color: 0x09090b });
  for (let h = 0; h < 12; h++) {
    const rad = (h * Math.PI) / 6;
    const isQuarter = h % 3 === 0;
    const tick = new THREE.Mesh(new THREE.BoxGeometry(isQuarter ? 8 : 4, isQuarter ? 26 : 16, 2), tickMat);
    tick.position.set(cx + Math.cos(rad) * 145, clockY + Math.sin(rad) * 145, clockZ + 2);
    tick.rotation.z = rad + Math.PI / 2;
    trainHallGroup.add(tick);
  }

  const handH = new THREE.Mesh(new THREE.BoxGeometry(7, 85, 2), tickMat);
  handH.position.set(cx + 25, clockY + 25, clockZ + 3);
  handH.rotation.z = Math.PI / 4;
  trainHallGroup.add(handH);

  const handM = new THREE.Mesh(new THREE.BoxGeometry(5, 120, 2), tickMat);
  handM.position.set(cx - 35, clockY + 35, clockZ + 4);
  handM.rotation.z = -Math.PI / 6;
  trainHallGroup.add(handM);

  const redMat = new THREE.MeshBasicMaterial({ color: 0xef4444 });
  const handS = new THREE.Mesh(new THREE.BoxGeometry(3, 130, 2), redMat);
  handS.position.set(cx, clockY + 45, clockZ + 5);
  handS.rotation.z = -Math.PI / 1.5;
  trainHallGroup.add(handS);

  const secondDot = new THREE.Mesh(new THREE.CircleGeometry(12, 16), redMat);
  secondDot.position.set(cx - 45, clockY + 80, clockZ + 6);
  trainHallGroup.add(secondDot);
}

function buildTrainHallSideWalls(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  floorY: number,
  wallH: number,
  brickMat: THREE.Material,
  windowGlassMat: THREE.Material,
  steelMat: THREE.Material,
) {
  const halfW = wHall / 2;
  const sideXPositions = [cx - halfW, cx + halfW];

  for (const x of sideXPositions) {
    const plinthH = 500;
    const plinth = new THREE.Mesh(new THREE.BoxGeometry(40, plinthH, lHall), brickMat);
    plinth.position.set(x, floorY + plinthH / 2, cz);
    plinth.receiveShadow = true;
    trainHallGroup.add(plinth);

    const winH = wallH - (floorY + plinthH);
    const winY = floorY + plinthH + winH / 2;

    const glassWall = new THREE.Mesh(new THREE.BoxGeometry(10, winH, lHall - 100), windowGlassMat);
    glassWall.position.set(x, winY, cz);
    trainHallGroup.add(glassWall);

    const count = Math.floor(lHall / 750);
    for (let i = 0; i < count; i++) {
      const wz = cz - lHall / 2 + i * 750;
      const barV = new THREE.Mesh(new THREE.BoxGeometry(16, winH, 16), steelMat);
      barV.position.set(x, winY, wz);
      trainHallGroup.add(barV);
    }
  }
}

function buildTrainHallCrane(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  floorY: number,
  craneYellowMat: THREE.Material,
  steelMat: THREE.Material,
) {
  const craneZ = cz - lHall * 0.22;
  const craneY = floorY + 1600;
  const craneSpan = wHall - 140;

  const beamGeo = new THREE.BoxGeometry(craneSpan, 110, 36);

  const beam1 = new THREE.Mesh(beamGeo, craneYellowMat);
  beam1.position.set(cx, craneY, craneZ - 90);
  beam1.castShadow = true;
  trainHallGroup.add(beam1);

  const beam2 = new THREE.Mesh(beamGeo, craneYellowMat);
  beam2.position.set(cx, craneY, craneZ + 90);
  beam2.castShadow = true;
  trainHallGroup.add(beam2);

  const truckGeo = new THREE.BoxGeometry(80, 80, 320);
  const truckL = new THREE.Mesh(truckGeo, steelMat);
  truckL.position.set(cx - craneSpan / 2, craneY, craneZ);
  trainHallGroup.add(truckL);

  const truckR = new THREE.Mesh(truckGeo, steelMat);
  truckR.position.set(cx + craneSpan / 2, craneY, craneZ);
  trainHallGroup.add(truckR);

  const trolleyX = cx + 500;
  const trolley = new THREE.Mesh(
    new THREE.BoxGeometry(180, 75, 150),
    new THREE.MeshStandardMaterial({ color: 0x1f2937, roughness: 0.5, metalness: 0.8 }),
  );
  trolley.position.set(trolleyX, craneY + 50, craneZ);
  trainHallGroup.add(trolley);

  const cableGeo = new THREE.CylinderGeometry(2, 2, 450, 4);
  const cableMat = new THREE.MeshBasicMaterial({ color: 0x475569 });
  const cable1 = new THREE.Mesh(cableGeo, cableMat);
  cable1.position.set(trolleyX - 25, craneY - 180, craneZ);
  trainHallGroup.add(cable1);

  const cable2 = new THREE.Mesh(cableGeo, cableMat);
  cable2.position.set(trolleyX + 25, craneY - 180, craneZ);
  trainHallGroup.add(cable2);

  const block = new THREE.Mesh(
    new THREE.BoxGeometry(70, 60, 45),
    new THREE.MeshStandardMaterial({ color: 0xf59e0b, roughness: 0.4 }),
  );
  block.position.set(trolleyX, craneY - 420, craneZ);
  trainHallGroup.add(block);

  const hookGeo = new THREE.TorusGeometry(32, 9, 8, 20, Math.PI * 1.35);
  const hook = new THREE.Mesh(hookGeo, new THREE.MeshStandardMaterial({ color: 0x111827, metalness: 0.9 }));
  hook.position.set(trolleyX, craneY - 480, craneZ);
  hook.rotation.z = Math.PI * 0.75;
  trainHallGroup.add(hook);
}

function buildTrainHallLamps(
  trainHallGroup: THREE.Group,
  cx: number,
  cz: number,
  wHall: number,
  lHall: number,
  floorY: number,
  apexH: number,
  steelMat: THREE.Material,
  nightLampMats: THREE.MeshStandardMaterial[],
) {
  const lampBays = [cz - lHall * 0.32, cz, cz + lHall * 0.32];
  const lampOffsetsX = [-wHall * 0.22, wHall * 0.22];
  const lampY = floorY + 2400;

  const shadeMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.6 });
  const bulbMat = new THREE.MeshStandardMaterial({
    color: 0xffedd5,
    emissive: 0xfb923c,
    emissiveIntensity: 0.8,
    roughness: 0.2,
  });
  nightLampMats.push(bulbMat);

  for (const z of lampBays) {
    for (const offX of lampOffsetsX) {
      const lx = cx + offX;

      const cableH = apexH - lampY;
      const cableGeo = new THREE.CylinderGeometry(2, 2, cableH, 4);
      const cable = new THREE.Mesh(cableGeo, steelMat);
      cable.position.set(lx, lampY + cableH / 2, z);
      trainHallGroup.add(cable);

      const shadeGeo = new THREE.CylinderGeometry(18, 75, 55, 16, 1, true);
      const shade = new THREE.Mesh(shadeGeo, shadeMat);
      shade.position.set(lx, lampY + 20, z);
      trainHallGroup.add(shade);

      const bulb = new THREE.Mesh(new THREE.SphereGeometry(18, 12, 12), bulbMat);
      bulb.position.set(lx, lampY, z);
      trainHallGroup.add(bulb);

      const pointLight = new THREE.PointLight(0xffedd5, 0.45, 4500, 1.4);
      pointLight.position.set(lx, lampY - 10, z);
      trainHallGroup.add(pointLight);
    }
  }
}
