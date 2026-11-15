import * as THREE from "three";
import * as BufferGeometryUtils from "three/examples/jsm/utils/BufferGeometryUtils.js";
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
import { buildPieceTunnel3D } from "./track-tunnel";

const SHARED_TIE_MAT = new THREE.MeshStandardMaterial({ color: 0x3e2723, roughness: 0.85 });
SHARED_TIE_MAT.userData.shared = true;

const SHARED_RAIL_MAT = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25 });
SHARED_RAIL_MAT.userData.shared = true;

const _ballastMatCache = new Map<number, THREE.MeshStandardMaterial>();
function getSharedBallastMaterial(color: number): THREE.MeshStandardMaterial {
  let mat = _ballastMatCache.get(color);
  if (!mat) {
    mat = new THREE.MeshStandardMaterial({
      color,
      roughness: 0.9,
      polygonOffset: true,
      polygonOffsetFactor: 1,
      polygonOffsetUnits: 1,
    });
    mat.userData.shared = true;
    _ballastMatCache.set(color, mat);
  }
  return mat;
}

const _tieGeoCache = new Map<number, THREE.BoxGeometry>();
function getSharedTieGeometry(gauge: number): THREE.BoxGeometry {
  const key = Math.round(gauge * 100);
  let geo = _tieGeoCache.get(key);
  if (!geo) {
    const tieW = Math.max(1.5, (gauge / 16.5) * 2.6);
    geo = new THREE.BoxGeometry(tieW, 2.2, gauge * 1.55);
    geo.userData.shared = true;
    _tieGeoCache.set(key, geo);
  }
  return geo;
}

const _straightBallastGeoCache = new Map<string, THREE.BoxGeometry>();
function getSharedStraightBallastGeometry(len: number, h: number, w: number): THREE.BoxGeometry {
  const key = `${Math.round(len * 10)}:${Math.round(h * 10)}:${Math.round(w * 10)}`;
  let geo = _straightBallastGeoCache.get(key);
  if (!geo) {
    geo = new THREE.BoxGeometry(len, h, w);
    geo.userData.shared = true;
    _straightBallastGeoCache.set(key, geo);
  }
  return geo;
}

const _straightRailGeoCache = new Map<string, THREE.BoxGeometry>();
function getSharedStraightRailGeometry(len: number, h: number, w: number): THREE.BoxGeometry {
  const key = `${Math.round(len * 10)}:${Math.round(h * 10)}:${Math.round(w * 10)}`;
  let geo = _straightRailGeoCache.get(key);
  if (!geo) {
    geo = new THREE.BoxGeometry(len, h, w);
    geo.userData.shared = true;
    _straightRailGeoCache.set(key, geo);
  }
  return geo;
}

const _straightDualRailGeoCache = new Map<string, THREE.BufferGeometry>();
function getSharedStraightDualRailGeometry(len: number, h: number, w: number, gaugeHalf: number): THREE.BufferGeometry {
  const key = `${Math.round(len * 10)}:${Math.round(h * 10)}:${Math.round(w * 10)}:${Math.round(gaugeHalf * 100)}`;
  let geo = _straightDualRailGeoCache.get(key);
  if (!geo) {
    const rL = new THREE.BoxGeometry(len, h, w);
    rL.translate(0, 0, -gaugeHalf);
    const rR = new THREE.BoxGeometry(len, h, w);
    rR.translate(0, 0, gaugeHalf);
    geo = BufferGeometryUtils.mergeGeometries([rL, rR]);
    geo.userData.shared = true;
    rL.dispose();
    rR.dispose();
    _straightDualRailGeoCache.set(key, geo);
  }
  return geo;
}

const _curvedBallastGeoCache = new Map<string, THREE.BufferGeometry>();
const _curvedRailGeoCache = new Map<string, THREE.BufferGeometry>();
const _curvedDualRailGeoCache = new Map<string, THREE.BufferGeometry>();

const SHARED_BUFFER_LAMP_MAT = new THREE.MeshStandardMaterial({
  color: 0xef4444,
  emissive: 0xef4444,
  emissiveIntensity: 0.95,
  roughness: 0.25,
});
SHARED_BUFFER_LAMP_MAT.userData.shared = true;

const _tieDummy = new THREE.Object3D();

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
    if (piece.hasBridge) {
      buildTrackBridge3D(group, entry, piece, startElev, endElev, layout);
    }
    if (piece.hasTunnel) {
      buildPieceTunnel3D(group, entry, piece, startElev, endElev, layout);
    }
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

  const ballastMat = getSharedBallastMaterial(ballastColor);
  const tieMat = SHARED_TIE_MAT;
  const railMat = SHARED_RAIL_MAT;
  const tieGeo = getSharedTieGeometry(gauge);

  const addStraightSection = (
    targetGroup: THREE.Group,
    len: number,
    startX = 0,
    isAccessory = false,
  ) => {
    const ballastGeo = getSharedStraightBallastGeometry(len, ballastH, ballastW);
    const ballastMesh = new THREE.Mesh(ballastGeo, ballastMat);
    ballastMesh.position.set(startX + len / 2, ballastH / 2, 0);
    ballastMesh.receiveShadow = true;
    ballastMesh.castShadow = true;
    targetGroup.add(ballastMesh);

    const tieStep = Math.max(3.5, (gauge / 16.5) * 6.5);
    const tieCount = Math.floor(len / tieStep);
    if (tieCount > 0) {
      const tiesInstanced = new THREE.InstancedMesh(tieGeo, tieMat, tieCount);
      tiesInstanced.castShadow = true;
      _tieDummy.rotation.set(0, 0, 0);
      for (let i = 0; i < tieCount; i++) {
        _tieDummy.position.set(startX + i * tieStep + tieStep / 2, ballastH + 1.1, 0);
        _tieDummy.updateMatrix();
        tiesInstanced.setMatrixAt(i, _tieDummy.matrix);
      }
      tiesInstanced.instanceMatrix.needsUpdate = true;
      targetGroup.add(tiesInstanced);
    }

    const dualRailGeo = getSharedStraightDualRailGeometry(len, railH, railW, gaugeHalf);
    const railsMesh = new THREE.Mesh(dualRailGeo, railMat);
    railsMesh.position.set(startX + len / 2, railY, 0);
    targetGroup.add(railsMesh);

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
            obj.material = SHARED_BUFFER_LAMP_MAT;
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
    const isFlat = Math.abs(customHDiff) < 0.01;
    const curveCacheKey = isFlat ? `${Math.round(r * 10)}:${Math.round(rad * 1000)}:${dir}:${leadX}:${ballastH}:${ballastW}` : null;

    let curveGeo = curveCacheKey ? _curvedBallastGeoCache.get(curveCacheKey) : undefined;
    if (!curveGeo) {
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

      curveGeo = new THREE.ExtrudeGeometry(curveShape, { depth: ballastH, bevelEnabled: false });
      curveGeo.rotateX(Math.PI / 2);
      curveGeo.translate(leadX, ballastH, dir * r);
      if (!isFlat) {
        applyCurveElevation(curveGeo, r, rad, customHDiff, dir, leadX);
      } else {
        curveGeo.userData.shared = true;
        _curvedBallastGeoCache.set(curveCacheKey!, curveGeo);
      }
    }

    const ballastMesh = new THREE.Mesh(curveGeo, ballastMat);
    ballastMesh.receiveShadow = true;
    ballastMesh.castShadow = true;
    targetGroup.add(ballastMesh);

    const railCacheKey = isFlat ? `${Math.round(r * 10)}:${Math.round(gaugeHalf * 100)}:${Math.round(rad * 1000)}:${dir}:${leadX}:${railH}:${railW}` : null;
    let dualCurvedGeo = railCacheKey ? _curvedDualRailGeoCache.get(railCacheKey) : undefined;

    if (!dualCurvedGeo) {
      const railGeos: THREE.BufferGeometry[] = [];
      for (const offset of [-gaugeHalf, gaugeHalf]) {
        const railShape = new THREE.Shape();
        const rr = r + offset;
        if (dir === 1) {
          railShape.absarc(0, 0, rr + railW / 2, -Math.PI / 2, -Math.PI / 2 + rad, false);
          railShape.absarc(0, 0, rr - railW / 2, -Math.PI / 2 + rad, -Math.PI / 2, true);
        } else {
          railShape.absarc(0, 0, rr + railW / 2, Math.PI / 2, Math.PI / 2 - rad, true);
          railShape.absarc(0, 0, rr - railW / 2, Math.PI / 2 - rad, Math.PI / 2, false);
        }
        railShape.closePath();

        const bRailGeo = new THREE.ExtrudeGeometry(railShape, { depth: railH, bevelEnabled: false });
        bRailGeo.rotateX(Math.PI / 2);
        bRailGeo.translate(leadX, railY, dir * r);
        if (!isFlat) {
          applyCurveElevation(bRailGeo, r, rad, customHDiff, dir, leadX);
        }
        railGeos.push(bRailGeo);
      }
      dualCurvedGeo = BufferGeometryUtils.mergeGeometries(railGeos);
      railGeos.forEach((g) => g.dispose());
      if (isFlat) {
        dualCurvedGeo.userData.shared = true;
        _curvedDualRailGeoCache.set(railCacheKey!, dualCurvedGeo);
      }
    }

    const bRailMesh = new THREE.Mesh(dualCurvedGeo, railMat);
    targetGroup.add(bRailMesh);

    const arcLen = r * rad;
    const tieStep = Math.max(3.5, (gauge / 16.5) * 6.5);
    const tieCount = Math.max(3, Math.floor(arcLen / tieStep));
    const tiesInstanced = new THREE.InstancedMesh(tieGeo, tieMat, tieCount);
    tiesInstanced.castShadow = true;
    for (let i = 0; i < tieCount; i++) {
      const u = (i + 0.5) / tieCount;
      const phi = u * rad;
      const tx = leadX + r * Math.sin(phi);
      const tz = dir * r * (1 - Math.cos(phi));
      const ty = ballastH + 1.1 + u * customHDiff;

      _tieDummy.position.set(tx, ty, tz);
      _tieDummy.rotation.set(0, -dir * phi, 0);
      _tieDummy.updateMatrix();
      tiesInstanced.setMatrixAt(i, _tieDummy.matrix);
    }
    tiesInstanced.instanceMatrix.needsUpdate = true;
    targetGroup.add(tiesInstanced);
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
  if (piece.hasTunnel) {
    buildPieceTunnel3D(group, entry, piece, startElev, endElev, layout);
  }

  trackGroup.add(group);
}
