import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import { isBufferStopPiece } from "@/catalog/catalog";
import type { ColorMode, TrackPiece } from "@/core/types";
import { deg2rad, isCurvedTrackEntry, isStraightTrackEntry } from "@/core/geometry";

/**
 * Hebt die Vertices einer gebogenen Drei.js-Geometrie stufenlos entlang des Kreisbogens an,
 * um realistische 3D-Gleiswendeln und Rampen ohne Brüche darzustellen.
 */
export function applyCurveElevation(
  geo: THREE.BufferGeometry,
  r: number,
  rad: number,
  heightDiff: number,
  dir: 1 | -1 = 1,
  leadX = 0,
) {
  if (Math.abs(heightDiff) < 0.01) return;
  const pos = geo.attributes.position;
  if (!pos) return;
  const centerZ = dir * r;
  for (let i = 0; i < pos.count; i++) {
    const vx = pos.getX(i) - leadX;
    const vy = pos.getY(i);
    const vz = pos.getZ(i);
    const theta =
      dir === 1
        ? Math.atan2(vz - centerZ, vx) + Math.PI / 2
        : Math.PI / 2 - Math.atan2(vz - centerZ, vx);
    const u = Math.max(0, Math.min(1, theta / rad));
    pos.setY(i, vy + u * heightDiff);
  }
  geo.computeVertexNormals();
}

export const SHARED_WOOD_PLATE_MAT = new THREE.MeshStandardMaterial({
  color: 0xcbb89d,
  roughness: 0.85,
  metalness: 0.05,
});
SHARED_WOOD_PLATE_MAT.userData.shared = true;

const _straightWoodPlateGeoCache = new Map<string, THREE.BoxGeometry>();
function getSharedStraightWoodPlateGeo(len: number, thickness: number, width: number): THREE.BoxGeometry {
  const key = `${Math.round(len * 10)}:${Math.round(thickness * 10)}:${Math.round(width * 10)}`;
  let geo = _straightWoodPlateGeoCache.get(key);
  if (!geo) {
    geo = new THREE.BoxGeometry(len, thickness, width);
    geo.userData.shared = true;
    _straightWoodPlateGeoCache.set(key, geo);
  }
  return geo;
}

const _curvedWoodPlateGeoCache = new Map<string, THREE.BufferGeometry>();

const SHARED_ASPHALT_DEFAULT_MAT = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.92, side: THREE.DoubleSide });
SHARED_ASPHALT_DEFAULT_MAT.userData.shared = true;

const SHARED_LINE_MAT = new THREE.MeshStandardMaterial({ color: 0xf8fafc, roughness: 0.4, side: THREE.DoubleSide });
SHARED_LINE_MAT.userData.shared = true;

const SHARED_YELLOW_MAT = new THREE.MeshStandardMaterial({ color: 0xfbbf24, roughness: 0.4, side: THREE.DoubleSide });
SHARED_YELLOW_MAT.userData.shared = true;

const SHARED_PLANK_MAT = new THREE.MeshStandardMaterial({ color: 0x272422, roughness: 0.85, side: THREE.DoubleSide });
SHARED_PLANK_MAT.userData.shared = true;

const SHARED_ROAD_RAIL_MAT = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25, side: THREE.DoubleSide });
SHARED_ROAD_RAIL_MAT.userData.shared = true;

const _layerAsphaltMatCache = new Map<number, THREE.MeshStandardMaterial>();

export function buildRoadPiece3D(
  group: THREE.Group,
  _piece: TrackPiece,
  entry: CatalogEntry,
  heightDiff: number,
  colorMode: ColorMode,
  layerColor?: string,
) {
  const roadW = entry.roadWidth ?? 60;
  const roadH = 2.4;
  const roadBaseY = 0.08;
  const kind = entry.kind;

  let asphaltMat = SHARED_ASPHALT_DEFAULT_MAT;
  if (colorMode === "layers" && layerColor) {
    const parsed = parseInt(layerColor.replace(/^#/, ""), 16);
    if (!isNaN(parsed)) {
      let cached = _layerAsphaltMatCache.get(parsed);
      if (!cached) {
        cached = new THREE.MeshStandardMaterial({ color: parsed, roughness: 0.92, side: THREE.DoubleSide });
        cached.userData.shared = true;
        _layerAsphaltMatCache.set(parsed, cached);
      }
      asphaltMat = cached;
    }
  }

  const lineMat = SHARED_LINE_MAT;
  const yellowMat = SHARED_YELLOW_MAT;
  const plankMat = SHARED_PLANK_MAT;
  const railMat = SHARED_ROAD_RAIL_MAT;

  const addStraightRoad = (target: THREE.Group, len: number, startX = 0, customZ = 0) => {
    const roadGeo = new THREE.BoxGeometry(len, roadH, roadW);
    const roadMesh = new THREE.Mesh(roadGeo, asphaltMat);
    roadMesh.position.set(startX + len / 2, roadBaseY + roadH / 2, customZ);
    roadMesh.receiveShadow = true;
    roadMesh.castShadow = true;
    target.add(roadMesh);

    const edgeLineGeo = new THREE.BoxGeometry(len, 0.12, 1.4);
    const edgeZ1 = customZ - roadW / 2 + 2.5;
    const edgeZ2 = customZ + roadW / 2 - 2.5;

    const edgeL = new THREE.Mesh(edgeLineGeo, lineMat);
    edgeL.position.set(startX + len / 2, roadBaseY + roadH + 0.06, edgeZ1);
    target.add(edgeL);

    const edgeR = new THREE.Mesh(edgeLineGeo, lineMat);
    edgeR.position.set(startX + len / 2, roadBaseY + roadH + 0.06, edgeZ2);
    target.add(edgeR);

    const dashGeo = new THREE.BoxGeometry(8, 0.12, 1.4);
    const period = 14;
    for (let x = startX + 4; x + 8 <= startX + len - 2; x += period) {
      const dash = new THREE.Mesh(dashGeo, lineMat);
      dash.position.set(x + 4, roadBaseY + roadH + 0.06, customZ);
      target.add(dash);
    }
  };

  const addCurvedRoad = (
    target: THREE.Group,
    r: number,
    a: number,
    dir: 1 | -1 = 1,
    customHDiff = heightDiff,
  ) => {
    const rad = deg2rad(a);
    const rIn = r - roadW / 2;
    const rOut = r + roadW / 2;

    const curveShape = new THREE.Shape();
    if (dir === 1) {
      curveShape.absarc(0, 0, rOut, -Math.PI / 2, -Math.PI / 2 + rad, false);
      curveShape.absarc(0, 0, rIn, -Math.PI / 2 + rad, -Math.PI / 2, true);
    } else {
      curveShape.absarc(0, 0, rOut, Math.PI / 2, Math.PI / 2 - rad, true);
      curveShape.absarc(0, 0, rIn, Math.PI / 2 - rad, Math.PI / 2, false);
    }
    curveShape.closePath();

    const curveGeo = new THREE.ExtrudeGeometry(curveShape, { depth: roadH, bevelEnabled: false });
    curveGeo.rotateX(Math.PI / 2);
    curveGeo.translate(0, roadBaseY + roadH, dir * r);
    applyCurveElevation(curveGeo, r, rad, customHDiff, dir, 0);

    const roadMesh = new THREE.Mesh(curveGeo, asphaltMat);
    roadMesh.receiveShadow = true;
    roadMesh.castShadow = true;
    target.add(roadMesh);

    for (const off of [-roadW / 2 + 2.5, roadW / 2 - 2.5]) {
      const lineShape = new THREE.Shape();
      const rr = r + off;
      const lineW = 1.4;
      if (dir === 1) {
        lineShape.absarc(0, 0, rr + lineW / 2, -Math.PI / 2, -Math.PI / 2 + rad, false);
        lineShape.absarc(0, 0, rr - lineW / 2, -Math.PI / 2 + rad, -Math.PI / 2, true);
      } else {
        lineShape.absarc(0, 0, rr + lineW / 2, Math.PI / 2, Math.PI / 2 - rad, true);
        lineShape.absarc(0, 0, rr - lineW / 2, Math.PI / 2 - rad, Math.PI / 2, false);
      }
      lineShape.closePath();

      const lineGeo = new THREE.ExtrudeGeometry(lineShape, { depth: 0.12, bevelEnabled: false });
      lineGeo.rotateX(Math.PI / 2);
      lineGeo.translate(0, roadBaseY + roadH + 0.06, dir * r);
      applyCurveElevation(lineGeo, r, rad, customHDiff, dir, 0);
      target.add(new THREE.Mesh(lineGeo, lineMat));
    }

    const totalArc = rad * r;
    const numDashes = Math.max(1, Math.floor(totalArc / 14));
    const angStep = rad / numDashes;
    const dashBoxGeo = new THREE.BoxGeometry(8, 0.12, 1.4);

    for (let i = 0; i < numDashes; i++) {
      const phi = (i + 0.5) * angStep;
      const tx = r * Math.sin(phi);
      const tz = dir * r * (1 - Math.cos(phi));
      const u = phi / rad;
      const ty = roadBaseY + roadH + 0.06 + u * customHDiff;

      const dash = new THREE.Mesh(dashBoxGeo, lineMat);
      dash.position.set(tx, ty, tz);
      dash.rotation.y = -dir * phi;
      target.add(dash);
    }
  };

  if (kind === "straight" || kind === "accessory" || kind === "flex" || kind === "buffer-stop" || isBufferStopPiece(entry)) {
    const len = entry.length ?? 200;
    const straightGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      straightGroup.rotation.z = Math.atan2(heightDiff, len);
    }

    if (entry.id === "ST-LOOP") {
      const neckL = 35;
      const loopCenterX = 55;
      const loopRadius = 42;

      const neckGeo = new THREE.BoxGeometry(neckL, roadH, roadW);
      const neck = new THREE.Mesh(neckGeo, asphaltMat);
      neck.position.set(neckL / 2, roadBaseY + roadH / 2, 0);
      neck.receiveShadow = true;
      straightGroup.add(neck);

      const circleGeo = new THREE.CylinderGeometry(loopRadius, loopRadius, roadH, 32);
      const circleMesh = new THREE.Mesh(circleGeo, asphaltMat);
      circleMesh.position.set(loopCenterX, roadBaseY + roadH / 2, 0);
      circleMesh.receiveShadow = true;
      straightGroup.add(circleMesh);

      const islandGeo = new THREE.CylinderGeometry(14, 14, roadH + 0.4, 24);
      const islandMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.9, side: THREE.DoubleSide });
      const islandMesh = new THREE.Mesh(islandGeo, islandMat);
      islandMesh.position.set(loopCenterX, roadBaseY + (roadH + 0.4) / 2, 0);
      straightGroup.add(islandMesh);

      const edgeLineGeo = new THREE.BoxGeometry(neckL, 0.12, 1.4);
      const eL = new THREE.Mesh(edgeLineGeo, lineMat);
      eL.position.set(neckL / 2, roadBaseY + roadH + 0.06, -roadW / 2 + 2.5);
      straightGroup.add(eL);
      const eR = new THREE.Mesh(edgeLineGeo, lineMat);
      eR.position.set(neckL / 2, roadBaseY + roadH + 0.06, roadW / 2 - 2.5);
      straightGroup.add(eR);

      group.add(straightGroup);
      return;
    }

    if (entry.id === "ST-BUS") {
      addStraightRoad(straightGroup, len);
      const bayGeo = new THREE.BoxGeometry(130, roadH, 15);
      const bayMesh = new THREE.Mesh(bayGeo, asphaltMat);
      bayMesh.position.set(100, roadBaseY + roadH / 2, roadW / 2 + 7.5);
      bayMesh.receiveShadow = true;
      straightGroup.add(bayMesh);

      const yDashGeo = new THREE.BoxGeometry(6, 0.12, 1.4);
      for (let bx = 38; bx + 6 <= 162; bx += 10) {
        const yd = new THREE.Mesh(yDashGeo, yellowMat);
        yd.position.set(bx + 3, roadBaseY + roadH + 0.06, roadW / 2);
        straightGroup.add(yd);
      }
      group.add(straightGroup);
      return;
    }

    if (entry.id === "ST-CROSS") {
      addStraightRoad(straightGroup, len);

      const plankGeo = new THREE.BoxGeometry(36, roadH + 0.3, roadW + 2);
      const plankMesh = new THREE.Mesh(plankGeo, plankMat);
      plankMesh.position.set(100, roadBaseY + (roadH + 0.3) / 2, 0);
      straightGroup.add(plankMesh);

      const railCrossGeo = new THREE.BoxGeometry(1.2, 2.0, roadW + 4);
      const rL = new THREE.Mesh(railCrossGeo, railMat);
      rL.position.set(100 - 8.25, roadBaseY + roadH + 1.0, 0);
      straightGroup.add(rL);
      const rR = new THREE.Mesh(railCrossGeo, railMat);
      rR.position.set(100 + 8.25, roadBaseY + roadH + 1.0, 0);
      straightGroup.add(rR);

      const stopGeo = new THREE.BoxGeometry(2.5, 0.12, roadW - 4);
      const sL = new THREE.Mesh(stopGeo, lineMat);
      sL.position.set(78, roadBaseY + roadH + 0.06, 0);
      straightGroup.add(sL);
      const sR = new THREE.Mesh(stopGeo, lineMat);
      sR.position.set(122, roadBaseY + roadH + 0.06, 0);
      straightGroup.add(sR);

      group.add(straightGroup);
      return;
    }

    addStraightRoad(straightGroup, len);
    group.add(straightGroup);
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 200;
    const a = entry.angle ?? 90;
    addCurvedRoad(group, r, Math.abs(a), a < 0 ? -1 : 1, heightDiff);
    return;
  }

  if (kind === "turnout-left" || kind === "turnout-right") {
    const len = entry.length ?? 200;
    const turnGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      turnGroup.rotation.z = Math.atan2(heightDiff, len);
    }
    addStraightRoad(turnGroup, len);

    const isRight = kind === "turnout-right";
    const branchSign = isRight ? 1 : -1;

    if (entry.branchLength !== undefined) {
      const bLen = entry.branchLength;
      const branchGeo = new THREE.BoxGeometry(roadW, roadH, bLen);
      const branchMesh = new THREE.Mesh(branchGeo, asphaltMat);
      branchMesh.position.set(len / 2, roadBaseY + roadH / 2, branchSign * (bLen / 2));
      branchMesh.receiveShadow = true;
      turnGroup.add(branchMesh);

      const edgeZGeo = new THREE.BoxGeometry(1.4, 0.12, bLen);
      const ezL = new THREE.Mesh(edgeZGeo, lineMat);
      ezL.position.set(len / 2 - roadW / 2 + 2.5, roadBaseY + roadH + 0.06, branchSign * (bLen / 2));
      turnGroup.add(ezL);
      const ezR = new THREE.Mesh(edgeZGeo, lineMat);
      ezR.position.set(len / 2 + roadW / 2 - 2.5, roadBaseY + roadH + 0.06, branchSign * (bLen / 2));
      turnGroup.add(ezR);

      const bDashGeo = new THREE.BoxGeometry(1.4, 0.12, 8);
      for (let z = 14; z + 8 <= bLen - 4; z += 14) {
        const bd = new THREE.Mesh(bDashGeo, lineMat);
        bd.position.set(len / 2, roadBaseY + roadH + 0.06, branchSign * (z + 4));
        turnGroup.add(bd);
      }
      group.add(turnGroup);
      return;
    }

    if (entry.branchRadius !== undefined) {
      const br = entry.branchRadius;
      const ba = entry.branchAngle ?? 45;
      addCurvedRoad(turnGroup, br, ba, branchSign, heightDiff);
      group.add(turnGroup);
      return;
    }
    group.add(turnGroup);
    return;
  }

  if (kind === "crossing") {
    const len = entry.length ?? 200;
    const crossGroup = new THREE.Group();
    addStraightRoad(crossGroup, len);

    const crossGeo = new THREE.BoxGeometry(roadW, roadH, len);
    const crossMesh = new THREE.Mesh(crossGeo, asphaltMat);
    crossMesh.position.set(len / 2, roadBaseY + roadH / 2, 0);
    crossMesh.receiveShadow = true;
    crossGroup.add(crossMesh);

    const crossEdgeGeo = new THREE.BoxGeometry(1.4, 0.12, (len - roadW) / 2);
    const ce1 = new THREE.Mesh(crossEdgeGeo, lineMat);
    ce1.position.set(len / 2 - roadW / 2 + 2.5, roadBaseY + roadH + 0.06, -len / 4 - roadW / 4);
    crossGroup.add(ce1);
    const ce2 = new THREE.Mesh(crossEdgeGeo, lineMat);
    ce2.position.set(len / 2 + roadW / 2 - 2.5, roadBaseY + roadH + 0.06, -len / 4 - roadW / 4);
    crossGroup.add(ce2);
    const ce3 = new THREE.Mesh(crossEdgeGeo, lineMat);
    ce3.position.set(len / 2 - roadW / 2 + 2.5, roadBaseY + roadH + 0.06, len / 4 + roadW / 4);
    crossGroup.add(ce3);
    const ce4 = new THREE.Mesh(crossEdgeGeo, lineMat);
    ce4.position.set(len / 2 + roadW / 2 - 2.5, roadBaseY + roadH + 0.06, len / 4 + roadW / 4);
    crossGroup.add(ce4);

    const crossDashGeo = new THREE.BoxGeometry(1.4, 0.12, 8);
    for (let z = -len / 2 + 4; z + 8 <= -roadW / 2 - 2; z += 14) {
      const cd = new THREE.Mesh(crossDashGeo, lineMat);
      cd.position.set(len / 2, roadBaseY + roadH + 0.06, z + 4);
      crossGroup.add(cd);
    }
    for (let z = roadW / 2 + 4; z + 8 <= len / 2 - 2; z += 14) {
      const cd = new THREE.Mesh(crossDashGeo, lineMat);
      cd.position.set(len / 2, roadBaseY + roadH + 0.06, z + 4);
      crossGroup.add(cd);
    }
    group.add(crossGroup);
    return;
  }

  // Fallback
  const fallbackLen = entry.length ?? 200;
  const fallbackGroup = new THREE.Group();
  addStraightRoad(fallbackGroup, fallbackLen);
  group.add(fallbackGroup);
}

/**
 * Erzeugt ein Trassenbrett (Wood Plate) unter Gleisen oder Straßen für realistische Modelltisch-Unterbauten.
 */
export function buildWoodPlatePiece3D(
  group: THREE.Group,
  _piece: TrackPiece,
  entry: CatalogEntry,
  heightDiff: number,
  customThickness?: number,
) {
  const isRoad = entry.isRoad === true || entry.id.startsWith("ST-");
  const roadW = entry.roadWidth ?? 60;
  const plateWidth = isRoad ? roadW + 8 : (entry.gauge ?? 16.5) * 2.4;
  const thickness = customThickness ?? 6;
  const woodPlateMat = SHARED_WOOD_PLATE_MAT;

  if (isStraightTrackEntry(entry)) {
    const len = entry.length ?? (entry.kind === "flex" ? 900 : isRoad ? 200 : 180);
    const geo = getSharedStraightWoodPlateGeo(len, thickness, plateWidth);
    const mesh = new THREE.Mesh(geo, woodPlateMat);
    mesh.position.set(len / 2, -thickness / 2, 0);
    if (Math.abs(heightDiff) > 0.01) {
      mesh.rotation.z = Math.atan2(heightDiff, len);
    }
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    group.add(mesh);
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 360;
    const angleVal = entry.angle ?? 30;
    const dir: 1 | -1 = angleVal < 0 ? -1 : 1;
    const deg = Math.abs(angleVal);
    const rad = deg2rad(deg);
    const isFlat = Math.abs(heightDiff) < 0.01;
    const cacheKey = isFlat ? `${Math.round(r * 10)}:${Math.round(rad * 1000)}:${dir}:${thickness}:${plateWidth}` : null;

    let geo = cacheKey ? _curvedWoodPlateGeoCache.get(cacheKey) : undefined;
    if (!geo) {
      const innerR = Math.max(10, r - plateWidth / 2);
      const outerR = r + plateWidth / 2;
      const segments = Math.max(8, Math.round(deg / 3));

      const curveShape = new THREE.Shape();
      if (dir === 1) {
        curveShape.absarc(0, 0, outerR, -Math.PI / 2, -Math.PI / 2 + rad, false);
        curveShape.absarc(0, 0, innerR, -Math.PI / 2 + rad, -Math.PI / 2, true);
      } else {
        curveShape.absarc(0, 0, outerR, Math.PI / 2, Math.PI / 2 - rad, true);
        curveShape.absarc(0, 0, innerR, Math.PI / 2 - rad, Math.PI / 2, false);
      }
      curveShape.closePath();

      const extrudeSettings = { depth: thickness, bevelEnabled: false, curveSegments: segments };
      geo = new THREE.ExtrudeGeometry(curveShape, extrudeSettings);
      geo.rotateX(Math.PI / 2);
      geo.translate(0, 0, dir * r);
      if (!isFlat) {
        applyCurveElevation(geo, r, rad, heightDiff, dir, 0);
      } else {
        geo.userData.shared = true;
        _curvedWoodPlateGeoCache.set(cacheKey!, geo);
      }
    }

    const mesh = new THREE.Mesh(geo, woodPlateMat);
    mesh.position.set(0, -thickness, 0);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    group.add(mesh);
  } else {
    const len = entry.length ?? 180;
    const geo = getSharedStraightWoodPlateGeo(len, thickness, plateWidth);
    const mesh = new THREE.Mesh(geo, woodPlateMat);
    mesh.position.set(len / 2, -thickness / 2, 0);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    group.add(mesh);
  }
}

export interface WoodPlateBuildItem {
  piece: TrackPiece;
  entry: CatalogEntry;
  startElev: number;
  endElev: number;
  thickness: number;
}

export function buildAllWoodPlates3D(woodPlateGroup: THREE.Group, items: WoodPlateBuildItem[]) {
  const flatGroups = new Map<string, WoodPlateBuildItem[]>();
  const individualItems: WoodPlateBuildItem[] = [];

  for (const item of items) {
    const isFlat = Math.abs(item.endElev - item.startElev) < 0.01;
    if (isFlat && (isStraightTrackEntry(item.entry) || isCurvedTrackEntry(item.entry))) {
      const key = `${item.entry.id}:${item.thickness}`;
      let list = flatGroups.get(key);
      if (!list) {
        list = [];
        flatGroups.set(key, list);
      }
      list.push(item);
    } else {
      individualItems.push(item);
    }
  }

  const worldMat = new THREE.Matrix4();
  const localMat = new THREE.Matrix4();
  const finalMat = new THREE.Matrix4();
  const pos = new THREE.Vector3();
  const quat = new THREE.Quaternion();
  const scale = new THREE.Vector3(1, 1, 1);
  const upAxis = new THREE.Vector3(0, 1, 0);

  for (const [, list] of flatGroups) {
    if (list.length >= 2) {
      const first = list[0]!;
      const entry = first.entry;
      const thickness = first.thickness;
      const isRoad = entry.isRoad === true || entry.id.startsWith("ST-");
      const roadW = entry.roadWidth ?? 60;
      const plateWidth = isRoad ? roadW + 8 : (entry.gauge ?? 16.5) * 2.4;

      if (isStraightTrackEntry(entry)) {
        const len = entry.length ?? (entry.kind === "flex" ? 900 : isRoad ? 200 : 180);
        const geo = getSharedStraightWoodPlateGeo(len, thickness, plateWidth);
        const instMesh = new THREE.InstancedMesh(geo, SHARED_WOOD_PLATE_MAT, list.length);
        instMesh.castShadow = true;
        instMesh.receiveShadow = true;
        localMat.makeTranslation(len / 2, -thickness / 2, 0);

        for (let i = 0; i < list.length; i++) {
          const it = list[i]!;
          pos.set(it.piece.transform.x, it.startElev, it.piece.transform.y);
          quat.setFromAxisAngle(upAxis, -deg2rad(it.piece.transform.rotation));
          worldMat.compose(pos, quat, scale);
          finalMat.multiplyMatrices(worldMat, localMat);
          instMesh.setMatrixAt(i, finalMat);
        }
        instMesh.instanceMatrix.needsUpdate = true;
        woodPlateGroup.add(instMesh);
      } else if (isCurvedTrackEntry(entry)) {
        const r = entry.radius ?? 360;
        const angleVal = entry.angle ?? 30;
        const dir: 1 | -1 = angleVal < 0 ? -1 : 1;
        const deg = Math.abs(angleVal);
        const rad = deg2rad(deg);
        const cacheKey = `${Math.round(r * 10)}:${Math.round(rad * 1000)}:${dir}:${thickness}:${plateWidth}`;
        let geo = _curvedWoodPlateGeoCache.get(cacheKey);
        if (!geo) {
          const innerR = Math.max(10, r - plateWidth / 2);
          const outerR = r + plateWidth / 2;
          const segments = Math.max(8, Math.round(deg / 3));
          const curveShape = new THREE.Shape();
          if (dir === 1) {
            curveShape.absarc(0, 0, outerR, -Math.PI / 2, -Math.PI / 2 + rad, false);
            curveShape.absarc(0, 0, innerR, -Math.PI / 2 + rad, -Math.PI / 2, true);
          } else {
            curveShape.absarc(0, 0, outerR, Math.PI / 2, Math.PI / 2 - rad, true);
            curveShape.absarc(0, 0, innerR, Math.PI / 2 - rad, Math.PI / 2, false);
          }
          curveShape.closePath();
          geo = new THREE.ExtrudeGeometry(curveShape, { depth: thickness, bevelEnabled: false, curveSegments: segments });
          geo.rotateX(Math.PI / 2);
          geo.translate(0, 0, dir * r);
          geo.userData.shared = true;
          _curvedWoodPlateGeoCache.set(cacheKey, geo);
        }

        const instMesh = new THREE.InstancedMesh(geo, SHARED_WOOD_PLATE_MAT, list.length);
        instMesh.castShadow = true;
        instMesh.receiveShadow = true;
        localMat.makeTranslation(0, -thickness, 0);

        for (let i = 0; i < list.length; i++) {
          const it = list[i]!;
          pos.set(it.piece.transform.x, it.startElev, it.piece.transform.y);
          quat.setFromAxisAngle(upAxis, -deg2rad(it.piece.transform.rotation));
          worldMat.compose(pos, quat, scale);
          finalMat.multiplyMatrices(worldMat, localMat);
          instMesh.setMatrixAt(i, finalMat);
        }
        instMesh.instanceMatrix.needsUpdate = true;
        woodPlateGroup.add(instMesh);
      }
    } else {
      for (const item of list) {
        individualItems.push(item);
      }
    }
  }

  for (const it of individualItems) {
    const plateGroup = new THREE.Group();
    plateGroup.position.set(it.piece.transform.x, it.startElev, it.piece.transform.y);
    plateGroup.rotation.y = -deg2rad(it.piece.transform.rotation);
    buildWoodPlatePiece3D(plateGroup, it.piece, it.entry, it.endElev - it.startElev, it.thickness);
    woodPlateGroup.add(plateGroup);
  }
}
