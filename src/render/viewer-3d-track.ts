import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import { isBufferStopPiece } from "@/catalog/catalog";
import type { ColorMode, Layout, TrackPiece } from "@/core/types";
import { deg2rad, isCurvedTrackEntry, isStraightTrackEntry } from "@/core/geometry";
import { getTrackBlueprintColor } from "@/render/blueprint-colors";
import { buildBufferStop3D } from "@/render/builtin-models-3d";
import { applyCurveElevation, buildRoadPiece3D } from "./viewer-3d-roads";
import { buildTransferTable3D, buildTurntable3D } from "./viewer-3d-turntables";
import { buildCatenary3D, buildTransferTableCatenary3D, buildTurntableCatenary3D } from "./viewer-3d-catenary";
import { buildTrackBridge3D, buildTrackSignal3D, buildTurnoutAccessories3D } from "./viewer-3d-bridges";

export function buildTrackPiece3D(
  trackGroup: THREE.Group,
  piece: TrackPiece,
  entry: CatalogEntry,
  startElev: number,
  endElev: number,
  colorMode: ColorMode,
  layerColor?: string,
  layout?: Layout,
) {
  const group = new THREE.Group();
  group.position.set(piece.transform.x, startElev, piece.transform.y);
  group.rotation.y = -deg2rad(piece.transform.rotation);

  if (entry.isRoad) {
    buildRoadPiece3D(group, piece, entry, endElev - startElev, colorMode, layerColor);
    trackGroup.add(group);
    return;
  }

  const bpColor = getTrackBlueprintColor(entry.id);
  let ballastColor = 0x78716c;
  if (colorMode === "types" && bpColor) {
    ballastColor = bpColor.hex;
  } else if (colorMode === "layers") {
    const hexStr = layerColor || "#38bdf8";
    const parsedHex = parseInt(hexStr.replace(/^#/, ""), 16);
    ballastColor = isNaN(parsedHex) ? 0x38bdf8 : parsedHex;
  }

  const kind = entry.kind;
  if (kind === "turntable") {
    buildTurntable3D(group, piece, entry, ballastColor);
    if (piece.hasCatenary) {
      buildTurntableCatenary3D(group, piece, entry, layout);
    }
    trackGroup.add(group);
    return;
  }
  if (kind === "transfer-table") {
    buildTransferTable3D(group, piece, entry, ballastColor);
    if (piece.hasCatenary) {
      buildTransferTableCatenary3D(group, piece, entry, layout);
    }
    trackGroup.add(group);
    return;
  }

  const gauge = entry.gauge ?? 16.5;
  const gaugeHalf = gauge / 2;
  const hasRoadbed = entry.hasRoadbed !== false;
  const ballastH = hasRoadbed ? 6 : 1.5;
  const ballastW = hasRoadbed ? (gauge / 16.5) * 36 : (gauge / 16.5) * 26;
  const railH = 2.2;
  const railW = 1.2;
  const railY = ballastH + railH;
  const heightDiff = endElev - startElev;

  const ballastMat = new THREE.MeshStandardMaterial({ color: ballastColor, roughness: 0.9 });
  const tieMat = new THREE.MeshStandardMaterial({ color: 0x3e2723, roughness: 0.85 });
  const railMat = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25 });

  const addStraightSection = (
    targetGroup: THREE.Group,
    len: number,
    startX = 0,
    isAccessory = false,
  ) => {
    const ballastGeo = new THREE.BoxGeometry(len, ballastH, ballastW);
    const ballastMesh = new THREE.Mesh(ballastGeo, ballastMat);
    ballastMesh.position.set(startX + len / 2, ballastH / 2, 0);
    ballastMesh.receiveShadow = true;
    ballastMesh.castShadow = true;
    targetGroup.add(ballastMesh);

    const tieW = Math.max(1.5, (gauge / 16.5) * 2.6);
    const tieStep = Math.max(3.5, (gauge / 16.5) * 6.5);
    const tieGeo = new THREE.BoxGeometry(tieW, 2.2, gauge * 1.55);
    const tieCount = Math.floor(len / tieStep);
    for (let i = 0; i < tieCount; i++) {
      const tie = new THREE.Mesh(tieGeo, tieMat);
      tie.position.set(startX + i * tieStep + tieStep / 2, ballastH + 1.1, 0);
      tie.castShadow = true;
      targetGroup.add(tie);
    }

    const railGeo = new THREE.BoxGeometry(len, railH, railW);
    const rL = new THREE.Mesh(railGeo, railMat);
    rL.position.set(startX + len / 2, railY, -gaugeHalf);
    const rR = new THREE.Mesh(railGeo, railMat);
    rR.position.set(startX + len / 2, railY, gaugeHalf);
    targetGroup.add(rL);
    targetGroup.add(rR);

    if (isAccessory) {
      const bufferGroup = new THREE.Group();
      buildBufferStop3D(bufferGroup);

      const scale = gauge / 16.5;
      bufferGroup.scale.set(scale, scale, scale);
      bufferGroup.rotation.y = Math.PI / 2;

      const endX = startX + len;
      const backOffset = 15 * scale;
      bufferGroup.position.set(endX - backOffset, ballastH + 2.2, 0);

      const isIlluminated =
        /beleuchtet|illuminat/i.test(entry.name || "") ||
        /beleuchtet|illuminat/i.test(entry.description || "");
      if (isIlluminated) {
        bufferGroup.traverse((obj) => {
          if (obj instanceof THREE.Mesh && obj.geometry instanceof THREE.CylinderGeometry) {
            obj.material = new THREE.MeshStandardMaterial({
              color: 0xef4444,
              emissive: 0xef4444,
              emissiveIntensity: 0.95,
              roughness: 0.25,
            });
          }
        });
      }

      bufferGroup.traverse((obj) => {
        if (obj instanceof THREE.Mesh) {
          obj.castShadow = true;
          obj.receiveShadow = true;
        }
      });

      targetGroup.add(bufferGroup);
    }
  };

  const addCurvedSection = (
    targetGroup: THREE.Group,
    r: number,
    a: number,
    dir: 1 | -1 = 1,
    leadX = 0,
    customHDiff = heightDiff,
  ) => {
    const rad = deg2rad(a);
    const rIn = r - ballastW / 2;
    const rOut = r + ballastW / 2;

    const curveShape = new THREE.Shape();
    if (dir === 1) {
      curveShape.absarc(0, 0, rOut, -Math.PI / 2, -Math.PI / 2 + rad, false);
      curveShape.absarc(0, 0, rIn, -Math.PI / 2 + rad, -Math.PI / 2, true);
    } else {
      curveShape.absarc(0, 0, rOut, Math.PI / 2, Math.PI / 2 - rad, true);
      curveShape.absarc(0, 0, rIn, Math.PI / 2 - rad, Math.PI / 2, false);
    }
    curveShape.closePath();

    const curveGeo = new THREE.ExtrudeGeometry(curveShape, { depth: ballastH, bevelEnabled: false });
    curveGeo.rotateX(Math.PI / 2);
    curveGeo.translate(leadX, ballastH, dir * r);
    applyCurveElevation(curveGeo, r, rad, customHDiff, dir, leadX);

    const ballastMesh = new THREE.Mesh(curveGeo, ballastMat);
    ballastMesh.receiveShadow = true;
    ballastMesh.castShadow = true;
    targetGroup.add(ballastMesh);

    for (const offset of [-gaugeHalf, gaugeHalf]) {
      const railShape = new THREE.Shape();
      const rr = r + offset;
      if (dir === 1) {
        railShape.absarc(0, 0, rr + railW / 2, -Math.PI / 2, -Math.PI / 2 + rad, false);
        railShape.absarc(0, 0, rr - railW / 2, -Math.PI / 2 + rad, -Math.PI / 2, true);
      } else {
        railShape.absarc(0, 0, rr + railW / 2, Math.PI / 2, Math.PI / 2 - rad, true);
        railShape.absarc(0, 0, rr - railW / 2, -Math.PI / 2, Math.PI / 2, false);
      }
      railShape.closePath();

      const bRailGeo = new THREE.ExtrudeGeometry(railShape, { depth: railH, bevelEnabled: false });
      bRailGeo.rotateX(Math.PI / 2);
      bRailGeo.translate(leadX, railY, dir * r);
      applyCurveElevation(bRailGeo, r, rad, customHDiff, dir, leadX);

      const bRailMesh = new THREE.Mesh(bRailGeo, railMat);
      targetGroup.add(bRailMesh);
    }

    const arcLen = r * rad;
    const tieW = Math.max(1.5, (gauge / 16.5) * 2.6);
    const tieStep = Math.max(3.5, (gauge / 16.5) * 6.5);
    const tieCount = Math.max(3, Math.floor(arcLen / tieStep));
    const tieGeo = new THREE.BoxGeometry(tieW, 2.2, gauge * 1.55);
    for (let i = 0; i < tieCount; i++) {
      const u = (i + 0.5) / tieCount;
      const phi = u * rad;
      const tx = leadX + r * Math.sin(phi);
      const tz = dir * r * (1 - Math.cos(phi));
      const ty = ballastH + 1.1 + u * customHDiff;

      const tieMesh = new THREE.Mesh(tieGeo, tieMat);
      tieMesh.position.set(tx, ty, tz);
      tieMesh.rotation.y = -dir * phi;
      tieMesh.castShadow = true;
      targetGroup.add(tieMesh);
    }
  };

  const isCurvedTurnout =
    kind === "curved-turnout-left" ||
    kind === "curved-turnout-right" ||
    ((kind === "turnout-left" || kind === "turnout-right") && !!entry.radius && !entry.length);

  const isBufferStop = isBufferStopPiece(entry);
  if (isStraightTrackEntry(entry)) {
    const len = entry.length ?? (kind === "flex" ? 900 : 188.3);
    const straightGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      straightGroup.rotation.z = Math.atan2(heightDiff, len);
    }
    addStraightSection(straightGroup, len, 0, isBufferStop);
    group.add(straightGroup);
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const dir: 1 | -1 = a < 0 ? -1 : 1;
    addCurvedSection(group, r, Math.abs(a), dir, 0, heightDiff);
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

    const cGroup = new THREE.Group();
    addCurvedSection(cGroup, r1, a1, dir, 0, heightDiff);
    if (lead > 0) {
      addStraightSection(cGroup, lead, 0, false);
    }
    addCurvedSection(cGroup, r2, a2, dir, lead, heightDiff);
    group.add(cGroup);
  } else if (kind === "turnout-left" || kind === "turnout-right") {
    const len = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const dir: 1 | -1 = kind === "turnout-left" ? -1 : 1;

    const turnoutGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      turnoutGroup.rotation.z = Math.atan2(heightDiff, len);
    }

    addStraightSection(turnoutGroup, len, 0, false);
    addCurvedSection(turnoutGroup, br, ba, dir, 0, heightDiff);
    group.add(turnoutGroup);
  } else if (kind === "three-way") {
    const len = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;

    const threeWayGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      threeWayGroup.rotation.z = Math.atan2(heightDiff, len);
    }

    addStraightSection(threeWayGroup, len, 0, false);
    addCurvedSection(threeWayGroup, br, ba, -1, 0, heightDiff);
    addCurvedSection(threeWayGroup, br, ba, 1, 0, heightDiff);
    group.add(threeWayGroup);
  } else if (kind === "crossing") {
    const len = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(Math.abs(a));
    const h = len / 2;

    const crossingGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      crossingGroup.rotation.z = Math.atan2(heightDiff, len);
    }

    addStraightSection(crossingGroup, len, -h, false);

    const track2Group = new THREE.Group();
    track2Group.rotation.y = -rad;
    addStraightSection(track2Group, len, -h, false);
    crossingGroup.add(track2Group);

    group.add(crossingGroup);
  } else if (kind === "double-slip") {
    const len = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(Math.abs(a));
    const h = len / 2;
    const cx = h;

    const dkwGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      dkwGroup.rotation.z = Math.atan2(heightDiff, len);
    }

    addStraightSection(dkwGroup, len, 0, false);

    const track2Group = new THREE.Group();
    track2Group.position.set(cx, 0, 0);
    track2Group.rotation.y = -rad;
    addStraightSection(track2Group, len, -h, false);
    dkwGroup.add(track2Group);

    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;

    addCurvedSection(dkwGroup, br, ba, 1, 0, heightDiff);

    const rightBranch = new THREE.Group();
    rightBranch.position.set(len, 0, 0);
    rightBranch.rotation.y = Math.PI;
    addCurvedSection(rightBranch, br, ba, 1, 0, heightDiff);
    dkwGroup.add(rightBranch);

    group.add(dkwGroup);
  } else {
    const len = entry.length ?? 188.3;
    const fallbackGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      fallbackGroup.rotation.z = Math.atan2(heightDiff, len);
    }
    addStraightSection(fallbackGroup, len, 0, false);
    group.add(fallbackGroup);
  }

  if (piece.hasCatenary) {
    buildCatenary3D(group, entry, startElev, endElev, piece);
  }

  if (piece.hasLantern || piece.hasDecoder) {
    buildTurnoutAccessories3D(group, entry, piece);
  }

  if (piece.hasSignal) {
    buildTrackSignal3D(group, entry, piece);
  }

  if (piece.hasBridge) {
    buildTrackBridge3D(group, entry, piece, startElev, endElev, layout);
  }

  trackGroup.add(group);
}
