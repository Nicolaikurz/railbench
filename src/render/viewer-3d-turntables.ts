import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import type { TrackPiece } from "@/core/types";
import { deg2rad } from "@/core/geometry";

export function buildTurntable3D(
  group: THREE.Group,
  _piece: TrackPiece,
  entry: CatalogEntry,
  ballastColor: number,
) {
  const outerR = entry.radius ?? 189;
  const pitR = 139;
  const pitDepth = 12;
  const bridgeL = 263;
  const bridgeW = 34;
  const gauge = entry.gauge ?? 16.5;
  const gaugeHalf = gauge / 2;

  const ballastMat = new THREE.MeshStandardMaterial({ color: ballastColor, roughness: 0.9 });
  const concreteMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.85 });
  const pitWallMat = new THREE.MeshStandardMaterial({
    color: 0x475569,
    roughness: 0.85,
    side: THREE.DoubleSide,
  });
  const pitFloorMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.95 });
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.7 });
  const woodMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.85 });
  const railMat = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25 });
  const cabMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.6 });
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.2, metalness: 0.9 });

  // 1. Äußerer Schotterkranz (Ring von pitR bis outerR)
  const ringShape = new THREE.Shape();
  ringShape.absarc(0, 0, outerR, 0, Math.PI * 2, false);
  const holePath = new THREE.Path();
  holePath.absarc(0, 0, pitR, 0, Math.PI * 2, true);
  ringShape.holes.push(holePath);

  const ringGeo = new THREE.ExtrudeGeometry(ringShape, { depth: 6, bevelEnabled: false, curveSegments: 36 });
  ringGeo.rotateX(Math.PI / 2);
  const ringMesh = new THREE.Mesh(ringGeo, ballastMat);
  ringMesh.position.set(0, 6, 0);
  ringMesh.receiveShadow = true;
  group.add(ringMesh);

  // 2. Grubenrand-Einfassung
  const rimShape = new THREE.Shape();
  rimShape.absarc(0, 0, pitR + 4, 0, Math.PI * 2, false);
  const rimHole = new THREE.Path();
  rimHole.absarc(0, 0, pitR, 0, Math.PI * 2, true);
  rimShape.holes.push(rimHole);

  const rimGeo = new THREE.ExtrudeGeometry(rimShape, { depth: 7.5, bevelEnabled: false, curveSegments: 36 });
  rimGeo.rotateX(Math.PI / 2);
  const rimMesh = new THREE.Mesh(rimGeo, concreteMat);
  rimMesh.position.set(0, 7.5, 0);
  group.add(rimMesh);

  // 3. Grubenboden
  const floorGeo = new THREE.CylinderGeometry(pitR, pitR, 3, 36);
  const floorMesh = new THREE.Mesh(floorGeo, pitFloorMat);
  floorMesh.position.set(0, -pitDepth, 0);
  floorMesh.receiveShadow = true;
  group.add(floorMesh);

  // 4. Grubenwand
  const wallGeo = new THREE.CylinderGeometry(pitR, pitR, pitDepth + 7.5, 36, 1, true);
  const wallMesh = new THREE.Mesh(wallGeo, pitWallMat);
  wallMesh.position.set(0, (-pitDepth + 7.5) / 2, 0);
  group.add(wallMesh);

  // 5. Kreisförmige Laufschiene für die Bühnenräder in der Grube
  const pitRailGeo = new THREE.TorusGeometry(126, 1.2, 8, 36);
  pitRailGeo.rotateX(Math.PI / 2);
  const pitRail = new THREE.Mesh(pitRailGeo, railMat);
  pitRail.position.set(0, -pitDepth + 1.8, 0);
  group.add(pitRail);

  // 6. Mittiger Drehzapfen / Königsstuhl
  const pivotGeo = new THREE.CylinderGeometry(14, 18, 10, 24);
  const pivot = new THREE.Mesh(pivotGeo, steelMat);
  pivot.position.set(0, -pitDepth + 5, 0);
  group.add(pivot);

  // 7. Drehscheibenbühne
  for (const zSide of [-8, 8]) {
    const girderGeo = new THREE.BoxGeometry(bridgeL - 6, 11, 2.5);
    const girder = new THREE.Mesh(girderGeo, steelMat);
    girder.position.set(0, 0, zSide);
    group.add(girder);
  }

  const deckGeo = new THREE.BoxGeometry(bridgeL, 2.4, bridgeW);
  const deck = new THREE.Mesh(deckGeo, woodMat);
  deck.position.set(0, 6.2, 0);
  deck.receiveShadow = true;
  group.add(deck);

  const railGeo = new THREE.BoxGeometry(bridgeL, 2.2, 1.2);
  const rL = new THREE.Mesh(railGeo, railMat);
  rL.position.set(0, 8.5, -gaugeHalf);
  group.add(rL);

  const rR = new THREE.Mesh(railGeo, railMat);
  rR.position.set(0, 8.5, gaugeHalf);
  group.add(rR);

  const tieGeo = new THREE.BoxGeometry(2.6, 1.6, gauge * 1.55);
  const tieCount = Math.floor(bridgeL / 6.5);
  for (let i = 0; i <= tieCount; i++) {
    const tie = new THREE.Mesh(tieGeo, woodMat);
    tie.position.set(-bridgeL / 2 + i * 6.5 + 3.2, 7.6, 0);
    group.add(tie);
  }

  for (const zSide of [-bridgeW / 2 + 1, bridgeW / 2 - 1]) {
    const railBar = new THREE.Mesh(new THREE.BoxGeometry(bridgeL - 10, 1.2, 1.2), steelMat);
    railBar.position.set(0, 15, zSide);
    group.add(railBar);

    for (let x = -bridgeL / 2 + 15; x <= bridgeL / 2 - 15; x += 35) {
      const post = new THREE.Mesh(new THREE.BoxGeometry(1.2, 8, 1.2), steelMat);
      post.position.set(x, 11, zSide);
      group.add(post);
    }
  }

  // Wärterhaus / Bedienstand
  const cabW = 32;
  const cabD = 20;
  const cabH = 26;
  const cab = new THREE.Mesh(new THREE.BoxGeometry(cabW, cabH, cabD), cabMat);
  cab.position.set(40, 17, bridgeW / 2 + cabD / 2 - 2);
  group.add(cab);

  const win = new THREE.Mesh(new THREE.BoxGeometry(cabW - 6, 9, cabD + 0.5), glassMat);
  win.position.set(40, 20, bridgeW / 2 + cabD / 2 - 2);
  group.add(win);

  const cabRoof = new THREE.Mesh(new THREE.BoxGeometry(cabW + 4, 2.5, cabD + 4), steelMat);
  cabRoof.position.set(40, 31, bridgeW / 2 + cabD / 2 - 2);
  group.add(cabRoof);

  // 8. 30 Anschlussstümpfe am Rand
  const stubLen = outerR - pitR;
  for (let a = 0; a < 360; a += 12) {
    const rad = deg2rad(a);
    const stubCenterR = (pitR + outerR) / 2;

    for (const offZ of [-gaugeHalf, gaugeHalf]) {
      const sRail = new THREE.Mesh(new THREE.BoxGeometry(stubLen, 2.2, 1.2), railMat);
      sRail.position.set(stubCenterR * Math.cos(rad) - offZ * Math.sin(rad), 8.5, stubCenterR * Math.sin(rad) + offZ * Math.cos(rad));
      sRail.rotation.y = -rad;
      group.add(sRail);
    }
  }
}

export function buildTransferTable3D(
  group: THREE.Group,
  _piece: TrackPiece,
  entry: CatalogEntry,
  ballastColor: number,
) {
  const l = entry.length ?? 360;
  const plateD = 420;
  const topZ = -55.2;
  const centerZ = 154.8;
  const pitLeft = 36;
  const pitRight = l - 36;
  const pitWidth = pitRight - pitLeft;
  const pitD = plateD - 12;
  const pitDepth = 8;
  const gauge = entry.gauge ?? 16.5;
  const gaugeHalf = gauge / 2;
  const trackZs = [0, 77.4, 154.8, 232.2, 309.6];
  const bridgeL = 288;
  const bridgeW = 34;
  const bridgeZ = 154.8;

  const ballastMat = new THREE.MeshStandardMaterial({ color: ballastColor, roughness: 0.9 });
  const concreteMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.85 });
  const pitWallMat = new THREE.MeshStandardMaterial({
    color: 0x475569,
    roughness: 0.85,
    side: THREE.DoubleSide,
  });
  const pitFloorMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.95 });
  const steelMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.7 });
  const woodMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.85 });
  const railMat = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25 });
  const cabMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.6 });
  const glassMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.2, metalness: 0.9 });

  // 1. Grundplatte / Schotterrand (Links & Rechts)
  const rimLeft = new THREE.Mesh(new THREE.BoxGeometry(pitLeft, 6, plateD), ballastMat);
  rimLeft.position.set(pitLeft / 2, 3, centerZ);
  rimLeft.receiveShadow = true;
  group.add(rimLeft);

  const rimRight = new THREE.Mesh(new THREE.BoxGeometry(l - pitRight, 6, plateD), ballastMat);
  rimRight.position.set(l - (l - pitRight) / 2, 3, centerZ);
  rimRight.receiveShadow = true;
  group.add(rimRight);

  // Quer-Abschlussränder oben und unten
  const rimTop = new THREE.Mesh(new THREE.BoxGeometry(pitWidth, 6, 6), concreteMat);
  rimTop.position.set(l / 2, 3, topZ + 3);
  group.add(rimTop);

  const rimBottom = new THREE.Mesh(new THREE.BoxGeometry(pitWidth, 6, 6), concreteMat);
  rimBottom.position.set(l / 2, 3, topZ + plateD - 3);
  group.add(rimBottom);

  // 2. Grubenboden
  const floorGeo = new THREE.BoxGeometry(pitWidth, 3, pitD);
  const floorMesh = new THREE.Mesh(floorGeo, pitFloorMat);
  floorMesh.position.set(l / 2, -pitDepth, centerZ);
  floorMesh.receiveShadow = true;
  group.add(floorMesh);

  // 2b. Grubenwände
  const wallH = pitDepth + 6;
  const wallY = (-pitDepth + 6) / 2;

  const wallLeft = new THREE.Mesh(new THREE.BoxGeometry(1.5, wallH, pitD), pitWallMat);
  wallLeft.position.set(pitLeft + 0.75, wallY, centerZ);
  group.add(wallLeft);

  const wallRight = new THREE.Mesh(new THREE.BoxGeometry(1.5, wallH, pitD), pitWallMat);
  wallRight.position.set(pitRight - 0.75, wallY, centerZ);
  group.add(wallRight);

  const wallTop = new THREE.Mesh(new THREE.BoxGeometry(pitWidth, wallH, 1.5), pitWallMat);
  wallTop.position.set(l / 2, wallY, topZ + 6 + 0.75);
  group.add(wallTop);

  const wallBottom = new THREE.Mesh(new THREE.BoxGeometry(pitWidth, wallH, 1.5), pitWallMat);
  wallBottom.position.set(l / 2, wallY, topZ + plateD - 6 - 0.75);
  group.add(wallBottom);

  // 3. Querlaufschienen für die Schiebebühne in der Grube
  for (const qx of [80, 180, 280]) {
    const qRail = new THREE.Mesh(new THREE.BoxGeometry(1.2, 1.8, pitD - 12), railMat);
    qRail.position.set(qx, -pitDepth + 2.4, centerZ);
    group.add(qRail);
  }

  // 4. Schwellen & Schienen an den 10 Anschluss-Stutzen
  for (const z of trackZs) {
    for (let x = 8; x <= 28; x += 6.5) {
      const tieL = new THREE.Mesh(new THREE.BoxGeometry(2.6, 2.2, gauge * 1.55), woodMat);
      tieL.position.set(x, 6 + 1.1, z);
      group.add(tieL);
    }
    for (let x = l - 28; x <= l - 8; x += 6.5) {
      const tieR = new THREE.Mesh(new THREE.BoxGeometry(2.6, 2.2, gauge * 1.55), woodMat);
      tieR.position.set(x, 6 + 1.1, z);
      group.add(tieR);
    }

    const railL1 = new THREE.Mesh(new THREE.BoxGeometry(pitLeft, 2.2, 1.2), railMat);
    railL1.position.set(pitLeft / 2, 8.2, z - gaugeHalf);
    group.add(railL1);
    const railL2 = new THREE.Mesh(new THREE.BoxGeometry(pitLeft, 2.2, 1.2), railMat);
    railL2.position.set(pitLeft / 2, 8.2, z + gaugeHalf);
    group.add(railL2);

    const railR1 = new THREE.Mesh(new THREE.BoxGeometry(l - pitRight, 2.2, 1.2), railMat);
    railR1.position.set(l - (l - pitRight) / 2, 8.2, z - gaugeHalf);
    group.add(railR1);
    const railR2 = new THREE.Mesh(new THREE.BoxGeometry(l - pitRight, 2.2, 1.2), railMat);
    railR2.position.set(l - (l - pitRight) / 2, 8.2, z + gaugeHalf);
    group.add(railR2);
  }

  // 5. Schiebebühne
  for (const zSide of [-14, 14]) {
    const girder = new THREE.Mesh(new THREE.BoxGeometry(bridgeL - 4, 10, 2.5), steelMat);
    girder.position.set(l / 2, -pitDepth / 2 + 1, bridgeZ + zSide);
    group.add(girder);
  }

  const deck = new THREE.Mesh(new THREE.BoxGeometry(bridgeL, 2.4, bridgeW), woodMat);
  deck.position.set(l / 2, 6.2, bridgeZ);
  deck.receiveShadow = true;
  group.add(deck);

  const tieCount = Math.floor(bridgeL / 6.5);
  for (let i = 0; i <= tieCount; i++) {
    const tie = new THREE.Mesh(new THREE.BoxGeometry(2.6, 1.6, gauge * 1.55), woodMat);
    tie.position.set(pitLeft + 6 + i * 6.5, 7.6, bridgeZ);
    group.add(tie);
  }

  const rL = new THREE.Mesh(new THREE.BoxGeometry(bridgeL, 2.2, 1.2), railMat);
  rL.position.set(l / 2, 8.5, bridgeZ - gaugeHalf);
  group.add(rL);

  const rR = new THREE.Mesh(new THREE.BoxGeometry(bridgeL, 2.2, 1.2), railMat);
  rR.position.set(l / 2, 8.5, bridgeZ + gaugeHalf);
  group.add(rR);

  for (const zSide of [-bridgeW / 2 + 1, bridgeW / 2 - 1]) {
    const railBar = new THREE.Mesh(new THREE.BoxGeometry(bridgeL - 10, 1.2, 1.2), steelMat);
    railBar.position.set(l / 2, 15, bridgeZ + zSide);
    group.add(railBar);

    for (let x = pitLeft + 15; x <= pitRight - 15; x += 35) {
      const post = new THREE.Mesh(new THREE.BoxGeometry(1.2, 8, 1.2), steelMat);
      post.position.set(x, 11, bridgeZ + zSide);
      group.add(post);
    }
  }

  // Wärterhaus / Steuerstand auf der Bühne
  const cabW = 34;
  const cabD = 20;
  const cabH = 24;
  const cabX = pitLeft + 40;
  const cabZ = bridgeZ + bridgeW / 2 + cabD / 2 - 2;

  const cab = new THREE.Mesh(new THREE.BoxGeometry(cabW, cabH, cabD), cabMat);
  cab.position.set(cabX, 16, cabZ);
  group.add(cab);

  const win = new THREE.Mesh(new THREE.BoxGeometry(cabW - 6, 8, cabD + 0.5), glassMat);
  win.position.set(cabX, 19, cabZ);
  group.add(win);

  const cabRoof = new THREE.Mesh(new THREE.BoxGeometry(cabW + 4, 2.5, cabD + 4), steelMat);
  cabRoof.position.set(cabX, 29, cabZ);
  group.add(cabRoof);
}
