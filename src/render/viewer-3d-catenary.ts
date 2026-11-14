import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import type { Layout, TrackPiece } from "@/core/types";
import { deg2rad, isCurvedTrackEntry, isStraightTrackEntry } from "@/core/geometry";

export function buildTurntableCatenary3D(
  group: THREE.Group,
  piece: TrackPiece,
  entry: CatalogEntry,
  currentLayout?: Layout,
) {
  const catGroup = new THREE.Group();

  const wireMat = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.3, metalness: 0.85 });
  const messengerMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.4, metalness: 0.7 });
  const mastMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.6 });
  const baseMat = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.9 });
  const insMat = new THREE.MeshStandardMaterial({ color: 0x854d0e, roughness: 0.3 });

  const contactH = 72;
  const messengerH = 85;

  const buildMast = (x: number, z: number, rotY = 0, side = 1) => {
    const mastGroup = new THREE.Group();
    mastGroup.position.set(x, 0, z);
    mastGroup.rotation.y = rotY;

    const mastDist = 24 * side;

    const baseGeo = new THREE.BoxGeometry(6, 8, 6);
    const baseMesh = new THREE.Mesh(baseGeo, baseMat);
    baseMesh.position.set(0, 4, mastDist);
    baseMesh.castShadow = true;
    baseMesh.receiveShadow = true;
    mastGroup.add(baseMesh);

    const poleGeo = new THREE.BoxGeometry(3, 86, 3);
    const poleMesh = new THREE.Mesh(poleGeo, mastMat);
    poleMesh.position.set(0, 47, mastDist);
    poleMesh.castShadow = true;
    mastGroup.add(poleMesh);

    const armGeo = new THREE.BoxGeometry(1.6, 1.6, 24);
    const armMesh = new THREE.Mesh(armGeo, mastMat);
    armMesh.position.set(0, messengerH, mastDist / 2);
    mastGroup.add(armMesh);

    const diagLen = Math.hypot(13, 16);
    const diagGeo = new THREE.BoxGeometry(1.2, 1.2, diagLen);
    const diagMesh = new THREE.Mesh(diagGeo, mastMat);
    diagMesh.position.set(0, (contactH + messengerH) / 2, mastDist * 0.6);
    diagMesh.rotation.x = Math.atan2(13, 16) * side;
    mastGroup.add(diagMesh);

    const insGeo = new THREE.CylinderGeometry(1.2, 1.2, 3, 8);
    const ins1 = new THREE.Mesh(insGeo, insMat);
    ins1.position.set(0, messengerH, 3 * side);
    mastGroup.add(ins1);

    const ins2 = new THREE.Mesh(insGeo, insMat);
    ins2.position.set(0, contactH, 1 * side);
    mastGroup.add(ins2);

    catGroup.add(mastGroup);
  };

  const unitX = new THREE.Vector3(1, 0, 0);

  const addStraightWire = (
    x1: number,
    z1: number,
    x2: number,
    z2: number,
    withMasts = true,
    mastSide = 1,
  ) => {
    const dx = x2 - x1;
    const dz = z2 - z1;
    const horizLen = Math.hypot(dx, dz);
    if (horizLen < 0.1) return;
    const angle = Math.atan2(dz, dx);
    const mx = (x1 + x2) / 2;
    const mz = (z1 + z2) / 2;

    const dirVec = new THREE.Vector3(dx, 0, dz).normalize();
    const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);

    const wireGeo = new THREE.BoxGeometry(horizLen, 0.8, 0.8);
    const wireMesh = new THREE.Mesh(wireGeo, wireMat);
    wireMesh.position.set(mx, contactH, mz);
    wireMesh.quaternion.copy(quat);
    catGroup.add(wireMesh);

    const msgGeo = new THREE.BoxGeometry(horizLen, 0.7, 0.7);
    const msgMesh = new THREE.Mesh(msgGeo, messengerMat);
    msgMesh.position.set(mx, messengerH, mz);
    msgMesh.quaternion.copy(quat);
    catGroup.add(msgMesh);

    const dropperCount = Math.max(1, Math.floor(horizLen / 35));
    for (let i = 1; i <= dropperCount; i++) {
      const u = i / (dropperCount + 1);
      const hx = x1 + dx * u;
      const hz = z1 + dz * u;
      const dropperGeo = new THREE.CylinderGeometry(0.3, 0.3, 13, 6);
      const dropperMesh = new THREE.Mesh(dropperGeo, messengerMat);
      dropperMesh.position.set(hx, (contactH + messengerH) / 2, hz);
      catGroup.add(dropperMesh);
    }

    if (withMasts) {
      buildMast(x1, z1, -angle, mastSide);
      if (horizLen >= 120) {
        buildMast(x2, z2, -angle, mastSide);
      }
    }
  };

  const bridgeHalf = 131.5;
  addStraightWire(-bridgeHalf, 0, bridgeHalf, 0, true, 1);

  if (piece.connectors && currentLayout?.pieces) {
    const localConns = entry.localConnectors || [];
    const outerR = entry.radius ?? 189;
    const pitR = 139;

    piece.connectors.forEach((conn, idx) => {
      if (!conn.connectedTo) return;
      const other = currentLayout.pieces.find((p) => p.id === conn.connectedTo!.pieceId);
      if (!other || !other.hasCatenary) return;

      const localConn = localConns[idx];
      const angleDeg = localConn ? localConn.angle : idx * 12;
      const rad = deg2rad(angleDeg);
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);

      const x1 = pitR * cos;
      const z1 = pitR * sin;
      const x2 = outerR * cos;
      const z2 = outerR * sin;

      addStraightWire(x1, z1, x2, z2, false);

      const mastAngle = Math.atan2(z2 - z1, x2 - x1);
      buildMast(x2, z2, -mastAngle, 1);
    });
  }

  group.add(catGroup);
}

export function buildTransferTableCatenary3D(
  group: THREE.Group,
  piece: TrackPiece,
  entry: CatalogEntry,
  currentLayout?: Layout,
) {
  const catGroup = new THREE.Group();

  const wireMat = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.3, metalness: 0.85 });
  const messengerMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.4, metalness: 0.7 });
  const mastMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.6 });
  const baseMat = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.9 });
  const insMat = new THREE.MeshStandardMaterial({ color: 0x854d0e, roughness: 0.3 });

  const contactH = 72;
  const messengerH = 85;

  const buildMast = (x: number, z: number, rotY = 0, side = 1) => {
    const mastGroup = new THREE.Group();
    mastGroup.position.set(x, 0, z);
    mastGroup.rotation.y = rotY;

    const mastDist = 24 * side;

    const baseGeo = new THREE.BoxGeometry(6, 8, 6);
    const baseMesh = new THREE.Mesh(baseGeo, baseMat);
    baseMesh.position.set(0, 4, mastDist);
    baseMesh.castShadow = true;
    baseMesh.receiveShadow = true;
    mastGroup.add(baseMesh);

    const poleGeo = new THREE.BoxGeometry(3, 86, 3);
    const poleMesh = new THREE.Mesh(poleGeo, mastMat);
    poleMesh.position.set(0, 47, mastDist);
    poleMesh.castShadow = true;
    mastGroup.add(poleMesh);

    const armGeo = new THREE.BoxGeometry(1.6, 1.6, 24);
    const armMesh = new THREE.Mesh(armGeo, mastMat);
    armMesh.position.set(0, messengerH, mastDist / 2);
    mastGroup.add(armMesh);

    const diagLen = Math.hypot(13, 16);
    const diagGeo = new THREE.BoxGeometry(1.2, 1.2, diagLen);
    const diagMesh = new THREE.Mesh(diagGeo, mastMat);
    diagMesh.position.set(0, (contactH + messengerH) / 2, mastDist * 0.6);
    diagMesh.rotation.x = Math.atan2(13, 16) * side;
    mastGroup.add(diagMesh);

    const insGeo = new THREE.CylinderGeometry(1.2, 1.2, 3, 8);
    const ins1 = new THREE.Mesh(insGeo, insMat);
    ins1.position.set(0, messengerH, 3 * side);
    mastGroup.add(ins1);

    const ins2 = new THREE.Mesh(insGeo, insMat);
    ins2.position.set(0, contactH, 1 * side);
    mastGroup.add(ins2);

    catGroup.add(mastGroup);
  };

  const unitX = new THREE.Vector3(1, 0, 0);

  const addStraightWire = (
    x1: number,
    z1: number,
    x2: number,
    z2: number,
    withMasts = true,
    mastSide = 1,
  ) => {
    const dx = x2 - x1;
    const dz = z2 - z1;
    const horizLen = Math.hypot(dx, dz);
    if (horizLen < 0.1) return;
    const angle = Math.atan2(dz, dx);
    const mx = (x1 + x2) / 2;
    const mz = (z1 + z2) / 2;

    const dirVec = new THREE.Vector3(dx, 0, dz).normalize();
    const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);

    const wireGeo = new THREE.BoxGeometry(horizLen, 0.8, 0.8);
    const wireMesh = new THREE.Mesh(wireGeo, wireMat);
    wireMesh.position.set(mx, contactH, mz);
    wireMesh.quaternion.copy(quat);
    catGroup.add(wireMesh);

    const msgGeo = new THREE.BoxGeometry(horizLen, 0.7, 0.7);
    const msgMesh = new THREE.Mesh(msgGeo, messengerMat);
    msgMesh.position.set(mx, messengerH, mz);
    msgMesh.quaternion.copy(quat);
    catGroup.add(msgMesh);

    const dropperCount = Math.max(1, Math.floor(horizLen / 35));
    for (let i = 1; i <= dropperCount; i++) {
      const u = i / (dropperCount + 1);
      const hx = x1 + dx * u;
      const hz = z1 + dz * u;
      const dropperGeo = new THREE.CylinderGeometry(0.3, 0.3, 13, 6);
      const dropperMesh = new THREE.Mesh(dropperGeo, messengerMat);
      dropperMesh.position.set(hx, (contactH + messengerH) / 2, hz);
      catGroup.add(dropperMesh);
    }

    if (withMasts) {
      buildMast(x1, z1, -angle, mastSide);
      if (horizLen >= 120) {
        buildMast(x2, z2, -angle, mastSide);
      }
    }
  };

  const l = entry.length ?? 360;
  const pitLeft = 36;
  const pitRight = l - 36;
  const bridgeZ = 154.8;

  addStraightWire(pitLeft, bridgeZ, pitRight, bridgeZ, true, 1);

  if (piece.connectors && currentLayout?.pieces) {
    const localConns = entry.localConnectors || [];
    piece.connectors.forEach((conn, idx) => {
      if (!conn.connectedTo) return;
      const other = currentLayout.pieces.find((p) => p.id === conn.connectedTo!.pieceId);
      if (!other || !other.hasCatenary) return;

      const localConn = localConns[idx];
      const isLeft = !localConn || localConn.x === 0;
      const z = localConn ? localConn.y : (idx < 5 ? idx * 77.4 : (idx - 5) * 77.4);

      if (isLeft) {
        addStraightWire(0, z, pitLeft, z, false);
        buildMast(0, z, -Math.PI, 1);
      } else {
        addStraightWire(pitRight, z, l, z, false);
        buildMast(l, z, 0, 1);
      }
    });
  }

  group.add(catGroup);
}

export function buildCatenary3D(
  group: THREE.Group,
  entry: CatalogEntry,
  startElev: number,
  endElev: number,
  piece?: TrackPiece,
) {
  const catGroup = new THREE.Group();

  const wireMat = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.3, metalness: 0.85 });
  const messengerMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.4, metalness: 0.7 });
  const mastMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.6 });
  const baseMat = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.9 });
  const insMat = new THREE.MeshStandardMaterial({ color: 0x854d0e, roughness: 0.3 });

  const contactH = 72;
  const messengerH = 85;

  const buildMast = (x: number, z: number, rotY = 0, side = 1, y = 0) => {
    const mastGroup = new THREE.Group();
    mastGroup.position.set(x, y, z);
    mastGroup.rotation.y = rotY;

    const mastDist = 24 * side;

    const baseGeo = new THREE.BoxGeometry(6, 8, 6);
    const baseMesh = new THREE.Mesh(baseGeo, baseMat);
    baseMesh.position.set(0, 4, mastDist);
    baseMesh.castShadow = true;
    baseMesh.receiveShadow = true;
    mastGroup.add(baseMesh);

    const poleGeo = new THREE.BoxGeometry(3, 86, 3);
    const poleMesh = new THREE.Mesh(poleGeo, mastMat);
    poleMesh.position.set(0, 47, mastDist);
    poleMesh.castShadow = true;
    mastGroup.add(poleMesh);

    const armGeo = new THREE.BoxGeometry(1.6, 1.6, 24);
    const armMesh = new THREE.Mesh(armGeo, mastMat);
    armMesh.position.set(0, messengerH, mastDist / 2);
    mastGroup.add(armMesh);

    const diagLen = Math.hypot(13, 16);
    const diagGeo = new THREE.BoxGeometry(1.2, 1.2, diagLen);
    const diagMesh = new THREE.Mesh(diagGeo, mastMat);
    diagMesh.position.set(0, (contactH + messengerH) / 2, mastDist * 0.6);
    diagMesh.rotation.x = Math.atan2(13, 16) * side;
    mastGroup.add(diagMesh);

    const insGeo = new THREE.CylinderGeometry(1.2, 1.2, 3, 8);
    const ins1 = new THREE.Mesh(insGeo, insMat);
    ins1.position.set(0, messengerH, 3 * side);
    mastGroup.add(ins1);

    const ins2 = new THREE.Mesh(insGeo, insMat);
    ins2.position.set(0, contactH, 1 * side);
    mastGroup.add(ins2);

    catGroup.add(mastGroup);
  };

  const unitX = new THREE.Vector3(1, 0, 0);

  const addStraightWire = (
    x1: number,
    z1: number,
    x2: number,
    z2: number,
    withMasts = true,
    mastSide = 1,
    y1 = 0,
    y2 = 0,
  ) => {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const dz = z2 - z1;
    const horizLen = Math.hypot(dx, dz);
    if (horizLen < 0.1) return;
    const totalLen = Math.hypot(dx, dy, dz);
    const angle = Math.atan2(dz, dx);
    const mx = (x1 + x2) / 2;
    const my = (y1 + y2) / 2;
    const mz = (z1 + z2) / 2;

    const dirVec = new THREE.Vector3(dx, dy, dz).normalize();
    const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);

    const wireGeo = new THREE.BoxGeometry(totalLen, 0.8, 0.8);
    const wireMesh = new THREE.Mesh(wireGeo, wireMat);
    wireMesh.position.set(mx, contactH + my, mz);
    wireMesh.quaternion.copy(quat);
    catGroup.add(wireMesh);

    const msgGeo = new THREE.BoxGeometry(totalLen, 0.7, 0.7);
    const msgMesh = new THREE.Mesh(msgGeo, messengerMat);
    msgMesh.position.set(mx, messengerH + my, mz);
    msgMesh.quaternion.copy(quat);
    catGroup.add(msgMesh);

    const dropperCount = Math.max(1, Math.floor(horizLen / 35));
    for (let i = 1; i <= dropperCount; i++) {
      const u = i / (dropperCount + 1);
      const hx = x1 + dx * u;
      const hy = y1 + dy * u;
      const hz = z1 + dz * u;
      const dropperGeo = new THREE.CylinderGeometry(0.3, 0.3, 13, 6);
      const dropperMesh = new THREE.Mesh(dropperGeo, messengerMat);
      dropperMesh.position.set(hx, (contactH + messengerH) / 2 + hy, hz);
      catGroup.add(dropperMesh);
    }

    if (withMasts) {
      buildMast(x1, z1, -angle, mastSide, y1);
      if (horizLen >= 120) {
        buildMast(x2, z2, -angle, mastSide, y2);
      }
    }
  };

  const addCurvedWire = (
    r: number,
    rad: number,
    dir: 1 | -1 = 1,
    leadX = 0,
    withMasts = true,
    mastSide = 1,
    startH = 0,
    heightDiff = 0,
  ) => {
    const segs = Math.max(4, Math.floor(rad / deg2rad(5)));
    for (let i = 0; i < segs; i++) {
      const u1 = i / segs;
      const u2 = (i + 1) / segs;
      const th1 = rad * u1;
      const th2 = rad * u2;

      const x1 = leadX + r * Math.sin(th1);
      const z1 = dir * r * (1 - Math.cos(th1));
      const y1 = startH + u1 * heightDiff;

      const x2 = leadX + r * Math.sin(th2);
      const z2 = dir * r * (1 - Math.cos(th2));
      const y2 = startH + u2 * heightDiff;

      const dx = x2 - x1;
      const dy = y2 - y1;
      const dz = z2 - z1;
      const segLen = Math.hypot(dx, dy, dz);
      if (segLen < 0.01) continue;

      const mx = (x1 + x2) / 2;
      const my = (y1 + y2) / 2;
      const mz = (z1 + z2) / 2;
      const dirVec = new THREE.Vector3(dx, dy, dz).normalize();
      const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);

      const wGeo = new THREE.BoxGeometry(segLen, 0.8, 0.8);
      const wMesh = new THREE.Mesh(wGeo, wireMat);
      wMesh.position.set(mx, contactH + my, mz);
      wMesh.quaternion.copy(quat);
      catGroup.add(wMesh);

      const mGeo = new THREE.BoxGeometry(segLen, 0.7, 0.7);
      const mMesh = new THREE.Mesh(mGeo, messengerMat);
      mMesh.position.set(mx, messengerH + my, mz);
      mMesh.quaternion.copy(quat);
      catGroup.add(mMesh);

      const dropGeo = new THREE.CylinderGeometry(0.3, 0.3, 13, 6);
      const dropMesh = new THREE.Mesh(dropGeo, messengerMat);
      dropMesh.position.set(mx, (contactH + messengerH) / 2 + my, mz);
      catGroup.add(dropMesh);
    }

    if (withMasts) {
      buildMast(leadX, 0, 0, mastSide, startH);
      const endX = leadX + r * Math.sin(rad);
      const endZ = dir * r * (1 - Math.cos(rad));
      buildMast(endX, endZ, -dir * rad, mastSide, startH + heightDiff);
    }
  };

  const kind = entry.kind;
  const isCurvedTurnout =
    kind === "curved-turnout-left" ||
    kind === "curved-turnout-right" ||
    ((kind === "turnout-left" || kind === "turnout-right") && !!entry.radius && !entry.length);
  const heightDiff = endElev - startElev;

  if (isStraightTrackEntry(entry)) {
    const len = entry.length ?? (kind === "flex" ? 900 : 188.3);
    addStraightWire(0, 0, len, 0, true, 1, 0, heightDiff);
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const dir: 1 | -1 = a < 0 ? -1 : 1;
    const absRad = deg2rad(Math.abs(a));
    addCurvedWire(r, absRad, dir, 0, true, -dir, 0, heightDiff);
  } else if (isCurvedTurnout) {
    const dir: 1 | -1 = kind === "curved-turnout-left" || kind === "turnout-left" ? -1 : 1;
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead =
      entry.branchLength ??
      (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 =
      entry.branchLength !== undefined
        ? (entry.branchRadius ?? r1)
        : entry.branchRadius && entry.branchRadius > r1
          ? r1
          : (entry.branchRadius ?? r1);
    const a2 = entry.branchAngle ?? 30;

    addCurvedWire(r1, deg2rad(Math.abs(a1)), dir, 0, true, -dir, 0, heightDiff);
    if (lead > 0) {
      addStraightWire(0, 0, lead, 0, false, 1, 0, heightDiff);
    }
    addCurvedWire(r2, deg2rad(Math.abs(a2)), dir, lead, true, dir, 0, heightDiff);
  } else if (kind === "turnout-left" || kind === "turnout-right") {
    const len = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const dir: 1 | -1 = kind === "turnout-left" ? -1 : 1;

    addStraightWire(0, 0, len, 0, true, -dir, 0, heightDiff);
    addCurvedWire(br, deg2rad(Math.abs(ba)), dir, 0, true, dir, 0, heightDiff);
  } else if (kind === "three-way") {
    const len = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;

    addStraightWire(0, 0, len, 0, true, 1, 0, heightDiff);
    addCurvedWire(br, deg2rad(Math.abs(ba)), -1, 0, true, -1, 0, heightDiff);
    addCurvedWire(br, deg2rad(Math.abs(ba)), 1, 0, true, 1, 0, heightDiff);
  } else if (kind === "crossing") {
    const len = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(Math.abs(a));
    const h = len / 2;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    addStraightWire(-h, 0, h, 0, true, 1, 0, heightDiff);
    addStraightWire(-h * cos, -h * sin, h * cos, h * sin, true, 1, 0, heightDiff);
  } else if (kind === "double-slip") {
    const len = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(Math.abs(a));
    const h = len / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    addStraightWire(0, 0, len, 0, true, 1, 0, heightDiff);
    addStraightWire(cx - h * cos, -h * sin, cx + h * cos, h * sin, true, 1, 0, heightDiff);
  } else {
    const len = entry.length ?? 188.3;
    addStraightWire(0, 0, len, 0, true, 1, 0, heightDiff);
  }

  // 3D-OBERLEITUNGS-EINSPEISUNG
  if (piece?.catenaryPowerFeed) {
    const feedBoxGeo = new THREE.BoxGeometry(4, 9, 5);
    const feedBoxMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.4 });
    const feedBox = new THREE.Mesh(feedBoxGeo, feedBoxMat);
    feedBox.position.set(0, 5, 27);
    catGroup.add(feedBox);

    const conduitGeo = new THREE.CylinderGeometry(0.6, 0.6, 85, 6);
    const conduitMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.5 });
    const conduit = new THREE.Mesh(conduitGeo, conduitMat);
    conduit.position.set(0, 43, 25.5);
    catGroup.add(conduit);

    const clampGeo = new THREE.CylinderGeometry(0.5, 0.5, 24, 6);
    const clampMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.3, metalness: 0.8 });
    const feedWire = new THREE.Mesh(clampGeo, clampMat);
    feedWire.position.set(0, messengerH, 12);
    feedWire.rotation.x = Math.PI / 2;
    catGroup.add(feedWire);

    const dropFeedGeo = new THREE.CylinderGeometry(0.5, 0.5, 13, 6);
    const dropFeed = new THREE.Mesh(dropFeedGeo, clampMat);
    dropFeed.position.set(0, (contactH + messengerH) / 2, 0);
    catGroup.add(dropFeed);
  }

  // 3D-OBERLEITUNGS-TRENNSTELLE
  if (piece?.catenaryIsolator) {
    const isoIdx = piece.catenaryIsolatedConnectorIndex ?? 0;
    let isoX = 25;
    let isoZ = 0;
    let isoRotY = 0;
    let isoY = 0;

    if (kind === "curve") {
      const r = entry.radius ?? 437.5;
      const a = entry.angle ?? 30;
      const dir: 1 | -1 = a < 0 ? -1 : 1;
      const rad = deg2rad(Math.abs(a));
      const frac = isoIdx === 1 ? 0.82 : 0.18;
      const phi = frac * rad;
      isoX = r * Math.sin(phi);
      isoZ = dir * r * (1 - Math.cos(phi));
      isoRotY = -dir * phi;
      isoY = frac * heightDiff;
    } else {
      const len = entry.length ?? 188.3;
      const frac = isoIdx === 1 ? Math.max(20, len - 25) / len : Math.min(len - 20, 25) / len;
      isoX = frac * len;
      isoY = frac * heightDiff;
    }

    const isoGroup = new THREE.Group();
    isoGroup.position.set(isoX, isoY, isoZ);
    isoGroup.rotation.y = isoRotY;

    const secInsGeo = new THREE.CylinderGeometry(1.5, 1.5, 14, 8);
    const secInsMat = new THREE.MeshStandardMaterial({ color: 0x9a3412, roughness: 0.25 });
    const insMesh1 = new THREE.Mesh(secInsGeo, secInsMat);
    insMesh1.position.set(0, contactH, 1.2);
    insMesh1.rotation.z = Math.PI / 2;
    isoGroup.add(insMesh1);

    const insMesh2 = new THREE.Mesh(secInsGeo, secInsMat);
    insMesh2.position.set(0, contactH, -1.2);
    insMesh2.rotation.z = Math.PI / 2;
    isoGroup.add(insMesh2);

    const topInsMesh = new THREE.Mesh(secInsGeo, secInsMat);
    topInsMesh.position.set(0, messengerH, 0);
    topInsMesh.rotation.z = Math.PI / 2;
    isoGroup.add(topInsMesh);

    const markGeo = new THREE.BoxGeometry(4, 3, 4);
    const markMat = new THREE.MeshStandardMaterial({ color: 0xdc2626, roughness: 0.4 });
    const markMesh = new THREE.Mesh(markGeo, markMat);
    markMesh.position.set(0, contactH + 2.5, 0);
    isoGroup.add(markMesh);

    catGroup.add(isoGroup);
  }

  group.add(catGroup);
}
