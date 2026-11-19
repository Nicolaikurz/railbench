import * as THREE from "three";
import * as BufferGeometryUtils from "three/examples/jsm/utils/BufferGeometryUtils.js";
import type { CatalogEntry } from "@/catalog/catalog";
import type { Layout, TrackPiece } from "@/core/types";
import { deg2rad, isCurvedTrackEntry, isStraightTrackEntry } from "@/core/geometry";

// Shared materials for all catenary elements to minimize memory & state switches
const SHARED_WIRE_MAT = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.3, metalness: 0.85 });
SHARED_WIRE_MAT.userData.shared = true;

const SHARED_MESSENGER_MAT = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.4, metalness: 0.7 });
SHARED_MESSENGER_MAT.userData.shared = true;

const SHARED_MAST_MAT = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.6 });
SHARED_MAST_MAT.userData.shared = true;

const SHARED_BASE_MAT = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.9 });
SHARED_BASE_MAT.userData.shared = true;

const SHARED_INS_MAT = new THREE.MeshStandardMaterial({ color: 0x854d0e, roughness: 0.3 });
SHARED_INS_MAT.userData.shared = true;

// Shared geometries for masts and accessories
const SHARED_BASE_GEO = new THREE.BoxGeometry(6, 8, 6);
SHARED_BASE_GEO.userData.shared = true;

const SHARED_POLE_GEO = new THREE.BoxGeometry(3, 86, 3);
SHARED_POLE_GEO.userData.shared = true;

const SHARED_ARM_GEO = new THREE.BoxGeometry(1.6, 1.6, 24);
SHARED_ARM_GEO.userData.shared = true;

const SHARED_DIAG_GEO = new THREE.BoxGeometry(1.2, 1.2, Math.hypot(13, 16));
SHARED_DIAG_GEO.userData.shared = true;

const SHARED_INS_GEO = new THREE.CylinderGeometry(1.2, 1.2, 3, 8);
SHARED_INS_GEO.userData.shared = true;

function createMastBracketGeo(side: 1 | -1): THREE.BufferGeometry {
  const mastDist = 24 * side;
  const contactH = 72;
  const messengerH = 85;
  const parts: THREE.BufferGeometry[] = [];

  // 1. Oberer Spitzenausleger (vom Mast z=24*side bis Tragseil z=0 auf y=85)
  const arm = new THREE.BoxGeometry(1.5, 1.5, 24);
  arm.translate(0, messengerH, mastDist / 2);
  parts.push(arm);

  // Tragseilklemme am Spitzenausleger
  const topClamp = new THREE.BoxGeometry(2.0, 1.8, 1.8);
  topClamp.translate(0, messengerH, 0);
  parts.push(topClamp);

  // 2. Schräges Auslegerrohr vom Mast (z=24*side, y=69) zum oberen Arm (z=4*side, y=85)
  const diagLen = Math.hypot(20, 16);
  const diag = new THREE.BoxGeometry(1.2, 1.2, diagLen);
  diag.rotateX(Math.atan2(16, 20) * side);
  diag.translate(0, 77, 14 * side);
  parts.push(diag);

  // 3. Unterer Seitenhalter: verbindet Auslegerrohr (z=14*side, y=77) exakt mit dem unteren Fahrdraht (z=0, y=72)
  const steadyLen = Math.hypot(14, 5);
  const steadyArm = new THREE.BoxGeometry(0.95, 0.95, steadyLen);
  steadyArm.rotateX(-Math.atan2(5, 14) * side);
  steadyArm.translate(0, 74.5, 7 * side);
  parts.push(steadyArm);

  // Fahrdrahtklemme am unteren Fahrdraht (z=0, y=72)
  const botClamp = new THREE.BoxGeometry(2.2, 1.5, 1.5);
  botClamp.translate(0, contactH + 0.3, 0);
  parts.push(botClamp);

  const merged = BufferGeometryUtils.mergeGeometries(parts);
  merged.userData.shared = true;
  parts.forEach((p) => p.dispose());
  return merged;
}

function addRibbedInsulatorParts(
  parts: THREE.BufferGeometry[],
  y: number,
  z: number,
  rotX: number,
): void {
  const core = new THREE.CylinderGeometry(0.85, 0.85, 4.8, 8);
  core.rotateX(Math.PI / 2 + rotX);
  core.translate(0, y, z);
  parts.push(core);

  for (const offset of [-1.5, 0, 1.5]) {
    const disc = new THREE.CylinderGeometry(1.55, 1.55, 0.55, 8);
    disc.translate(0, offset, 0);
    disc.rotateX(Math.PI / 2 + rotX);
    disc.translate(0, y, z);
    parts.push(disc);
  }
}

function createMastInsGeo(side: 1 | -1): THREE.BufferGeometry {
  const parts: THREE.BufferGeometry[] = [];
  // 1. Oberer Stabisolator am Spitzenausleger nahe Mast
  addRibbedInsulatorParts(parts, 85, 20.5 * side, 0);
  // 2. Unterer Stabisolator am schrägen Auslegerrohr nahe Mast
  addRibbedInsulatorParts(parts, 71.8, 20.5 * side, Math.atan2(16, 20) * side);
  // 3. Isolator am unteren Seitenhalter
  addRibbedInsulatorParts(parts, 75.5, 9.8 * side, -Math.atan2(5, 14) * side);

  const merged = BufferGeometryUtils.mergeGeometries(parts);
  merged.userData.shared = true;
  parts.forEach((p) => p.dispose());
  return merged;
}

function createTensionHardwareGeo(side: 1 | -1): THREE.BufferGeometry {
  const parts: THREE.BufferGeometry[] = [];
  const outerZ = 28.0 * side;

  // Oberer Rollenhalter am Mast
  const topBracket = new THREE.BoxGeometry(1.4, 1.4, 5.2);
  topBracket.translate(0, 82, 26.2 * side);
  parts.push(topBracket);

  // Spannrad / Umlenkrolle
  const pulley = new THREE.CylinderGeometry(2.3, 2.3, 1.0, 12);
  pulley.rotateZ(Math.PI / 2);
  pulley.translate(0, 80, outerZ);
  parts.push(pulley);

  // Spannseil von der Umlenkrolle hinab zu den Gewichten
  const cable = new THREE.CylinderGeometry(0.28, 0.28, 48, 6);
  cable.translate(0, 56, outerZ);
  parts.push(cable);

  // Führungsstange für die Gewichte entlang des Masts
  const guideRod = new THREE.CylinderGeometry(0.32, 0.32, 60, 6);
  guideRod.translate(0, 39, 26.6 * side);
  parts.push(guideRod);

  // Unterer Halter am Mastfuß
  const botBracket = new THREE.BoxGeometry(1.4, 1.2, 4.2);
  botBracket.translate(0, 10, 25.8 * side);
  parts.push(botBracket);

  const merged = BufferGeometryUtils.mergeGeometries(parts);
  merged.userData.shared = true;
  parts.forEach((p) => p.dispose());
  return merged;
}

function createTensionWeightGeo(side: 1 | -1): THREE.BufferGeometry {
  const parts: THREE.BufferGeometry[] = [];
  const outerZ = 28.0 * side;
  // 6 gestapelte Beton-/Gusseisen-Spanngewichte (Radspannwerk)
  for (let i = 0; i < 6; i++) {
    const disc = new THREE.CylinderGeometry(2.15, 2.15, 2.0, 10);
    disc.translate(0, 18 + i * 2.45, outerZ);
    parts.push(disc);
  }
  const merged = BufferGeometryUtils.mergeGeometries(parts);
  merged.userData.shared = true;
  parts.forEach((p) => p.dispose());
  return merged;
}

const SHARED_MAST_BRACKET_POS = createMastBracketGeo(1);
const SHARED_MAST_BRACKET_NEG = createMastBracketGeo(-1);
const SHARED_MAST_INS_POS = createMastInsGeo(1);
const SHARED_MAST_INS_NEG = createMastInsGeo(-1);
const SHARED_TENSION_HW_POS = createTensionHardwareGeo(1);
const SHARED_TENSION_HW_NEG = createTensionHardwareGeo(-1);
const SHARED_TENSION_WEIGHT_POS = createTensionWeightGeo(1);
const SHARED_TENSION_WEIGHT_NEG = createTensionWeightGeo(-1);

const SHARED_UNIT_DROPPER_GEO = new THREE.CylinderGeometry(0.28, 0.28, 1, 6);
SHARED_UNIT_DROPPER_GEO.userData.shared = true;
const SHARED_DROPPER_GEO = new THREE.CylinderGeometry(0.3, 0.3, 13, 6);
SHARED_DROPPER_GEO.userData.shared = true;

const SHARED_UNIT_WIRE_GEO = new THREE.BoxGeometry(1, 0.8, 0.8);
SHARED_UNIT_WIRE_GEO.userData.shared = true;

const SHARED_UNIT_MSG_GEO = new THREE.BoxGeometry(1, 0.7, 0.7);
SHARED_UNIT_MSG_GEO.userData.shared = true;

const _wireGeoCache = new Map<number, THREE.BoxGeometry>();
function getSharedWireGeometry(len: number): THREE.BoxGeometry {
  const key = Math.round(len * 10);
  let geo = _wireGeoCache.get(key);
  if (!geo) {
    geo = new THREE.BoxGeometry(len, 0.8, 0.8);
    geo.userData.shared = true;
    _wireGeoCache.set(key, geo);
  }
  return geo;
}

const _msgGeoCache = new Map<number, THREE.BoxGeometry>();
function getSharedMessengerGeometry(len: number): THREE.BoxGeometry {
  const key = Math.round(len * 10);
  let geo = _msgGeoCache.get(key);
  if (!geo) {
    geo = new THREE.BoxGeometry(len, 0.7, 0.7);
    geo.userData.shared = true;
    _msgGeoCache.set(key, geo);
  }
  return geo;
}

export interface CatenaryTrackItem {
  piece: TrackPiece;
  entry: CatalogEntry;
  startElev: number;
  endElev: number;
}

interface TrackSamplePoint {
  x: number;
  y: number;
  z: number;
  r: number;
}

interface PlacedMastPoint {
  x: number;
  y: number;
  z: number;
}

/**
 * Globaler Kontext für die 3D-Oberleitungsmasten einer Szene:
 * 1. Verhindert doppelte Masten ("Träger") an derselben Stelle (z. B. an Gleisübergängen oder Weichenwurzeln).
 * 2. Prüft per 2D-Raumgitter, ob ein Mastfuß auf einem Gleis / Weichenzweig stünde,
 *    wechselt automatisch auf die freie Gleisseite oder lässt den Mast weg, wenn beide Seiten blockiert sind.
 */
export class CatenaryMastContext {
  private static readonly CELL = 60;
  private trackGrid = new Map<string, TrackSamplePoint[]>();
  private placedGrid = new Map<string, PlacedMastPoint[]>();

  // Globale Instancing-Batch-Matrizen für alle Oberleitungen der gesamten Anlage (~13 Draw-Calls gesamt!)
  private wireMatrices: THREE.Matrix4[] = [];
  private msgMatrices: THREE.Matrix4[] = [];
  private dropperMatrices: THREE.Matrix4[] = [];
  private mastBaseMatrices: THREE.Matrix4[] = [];
  private mastPoleMatrices: THREE.Matrix4[] = [];
  private mastBracketPosMatrices: THREE.Matrix4[] = [];
  private mastBracketNegMatrices: THREE.Matrix4[] = [];
  private mastInsPosMatrices: THREE.Matrix4[] = [];
  private mastInsNegMatrices: THREE.Matrix4[] = [];
  private tensionHwPosMatrices: THREE.Matrix4[] = [];
  private tensionHwNegMatrices: THREE.Matrix4[] = [];
  private tensionWeightPosMatrices: THREE.Matrix4[] = [];
  private tensionWeightNegMatrices: THREE.Matrix4[] = [];
  private mastCount = 0;

  private readonly _unitX = new THREE.Vector3(1, 0, 0);
  private readonly _upY = new THREE.Vector3(0, 1, 0);
  private readonly _tmpPos = new THREE.Vector3();
  private readonly _tmpQuat = new THREE.Quaternion();
  private readonly _tmpScale = new THREE.Vector3(1, 1, 1);
  private readonly _tmpDir = new THREE.Vector3();
  private readonly _tmpPieceMat = new THREE.Matrix4();
  private readonly _tmpLocalMat = new THREE.Matrix4();
  private readonly _tmpOffsetMat = new THREE.Matrix4();

  constructor(items: CatenaryTrackItem[]) {
    for (const item of items) {
      if (item.entry.isRoad) continue;
      this.indexTrackItem(item);
    }
  }

  private getPieceWorldMatrix(piece: TrackPiece, startElev: number): THREE.Matrix4 {
    this._tmpPos.set(piece.transform.x, startElev, piece.transform.y);
    this._tmpQuat.setFromAxisAngle(this._upY, -deg2rad(piece.transform.rotation));
    this._tmpScale.set(1, 1, 1);
    return this._tmpPieceMat.compose(this._tmpPos, this._tmpQuat, this._tmpScale);
  }

  addBatchedContactSegment(
    piece: TrackPiece,
    startElev: number,
    x1: number,
    y1: number,
    z1: number,
    x2: number,
    y2: number,
    z2: number,
    contactH = 72,
  ): void {
    const w1 = this.toWorld(piece, startElev, x1, y1 + contactH, z1);
    const w2 = this.toWorld(piece, startElev, x2, y2 + contactH, z2);
    const dx = w2.x - w1.x;
    const dy = w2.y - w1.y;
    const dz = w2.z - w1.z;
    const totalLen = Math.hypot(dx, dy, dz);
    if (totalLen < 0.01) return;

    this._tmpDir.set(dx, dy, dz).divideScalar(totalLen);
    this._tmpQuat.setFromUnitVectors(this._unitX, this._tmpDir);
    this._tmpScale.set(totalLen, 1, 1);
    this._tmpPos.set((w1.x + w2.x) * 0.5, (w1.y + w2.y) * 0.5, (w1.z + w2.z) * 0.5);
    this.wireMatrices.push(new THREE.Matrix4().compose(this._tmpPos, this._tmpQuat, this._tmpScale));
  }

  addBatchedMessengerSegment(
    piece: TrackPiece,
    startElev: number,
    x1: number,
    y1AbsLocal: number,
    z1: number,
    x2: number,
    y2AbsLocal: number,
    z2: number,
  ): void {
    const w1 = this.toWorld(piece, startElev, x1, y1AbsLocal, z1);
    const w2 = this.toWorld(piece, startElev, x2, y2AbsLocal, z2);
    const dx = w2.x - w1.x;
    const dy = w2.y - w1.y;
    const dz = w2.z - w1.z;
    const totalLen = Math.hypot(dx, dy, dz);
    if (totalLen < 0.01) return;

    this._tmpDir.set(dx, dy, dz).divideScalar(totalLen);
    this._tmpQuat.setFromUnitVectors(this._unitX, this._tmpDir);
    this._tmpScale.set(totalLen, 1, 1);
    this._tmpPos.set((w1.x + w2.x) * 0.5, (w1.y + w2.y) * 0.5, (w1.z + w2.z) * 0.5);
    this.msgMatrices.push(new THREE.Matrix4().compose(this._tmpPos, this._tmpQuat, this._tmpScale));
  }

  addBatchedWireSegment(
    piece: TrackPiece,
    startElev: number,
    x1: number,
    y1: number,
    z1: number,
    x2: number,
    y2: number,
    z2: number,
    contactH = 72,
    messengerH = 85,
  ): void {
    this.addBatchedContactSegment(piece, startElev, x1, y1, z1, x2, y2, z2, contactH);
    this.addBatchedMessengerSegment(piece, startElev, x1, y1 + messengerH, z1, x2, y2 + messengerH, z2);
  }

  addBatchedDropper(
    piece: TrackPiece,
    startElev: number,
    lx: number,
    ly: number,
    lz: number,
    midH = 78.5,
    length = 13,
  ): void {
    const w = this.toWorld(piece, startElev, lx, ly, lz);
    this._tmpPos.set(w.x, w.y + midH, w.z);
    this._tmpQuat.identity();
    this._tmpScale.set(1, Math.max(0.5, length), 1);
    this.dropperMatrices.push(new THREE.Matrix4().compose(this._tmpPos, this._tmpQuat, this._tmpScale));
  }

  addBatchedMast(
    piece: TrackPiece,
    startElev: number,
    lx: number,
    ly: number,
    lz: number,
    rotY: number,
    side: 1 | -1,
  ): void {
    const pieceMat = this.getPieceWorldMatrix(piece, startElev);
    this._tmpPos.set(lx, ly, lz);
    this._tmpQuat.setFromAxisAngle(this._upY, rotY);
    this._tmpScale.set(1, 1, 1);
    this._tmpLocalMat.compose(this._tmpPos, this._tmpQuat, this._tmpScale);
    const mastWorld = new THREE.Matrix4().multiplyMatrices(pieceMat, this._tmpLocalMat);

    const mastDist = 24 * side;
    this._tmpOffsetMat.makeTranslation(0, 4, mastDist);
    this.mastBaseMatrices.push(new THREE.Matrix4().multiplyMatrices(mastWorld, this._tmpOffsetMat));

    this._tmpOffsetMat.makeTranslation(0, 47, mastDist);
    this.mastPoleMatrices.push(new THREE.Matrix4().multiplyMatrices(mastWorld, this._tmpOffsetMat));

    const hasTensionWeights = this.mastCount % 5 === 0;
    this.mastCount++;

    if (side === -1) {
      this.mastBracketNegMatrices.push(mastWorld.clone());
      this.mastInsNegMatrices.push(mastWorld.clone());
      if (hasTensionWeights) {
        this.tensionHwNegMatrices.push(mastWorld.clone());
        this.tensionWeightNegMatrices.push(mastWorld.clone());
      }
    } else {
      this.mastBracketPosMatrices.push(mastWorld.clone());
      this.mastInsPosMatrices.push(mastWorld.clone());
      if (hasTensionWeights) {
        this.tensionHwPosMatrices.push(mastWorld.clone());
        this.tensionWeightPosMatrices.push(mastWorld.clone());
      }
    }
  }

  /**
   * Erstellt am Ende von buildAllTracks3D gebatchte InstancedMeshes
   * für sämtliche Oberleitungen der gesamten Anlage.
   */
  flush(targetGroup: THREE.Group): void {
    const emitBatch = (
      geo: THREE.BufferGeometry,
      mat: THREE.Material,
      matrices: THREE.Matrix4[],
      castShadow: boolean,
      receiveShadow: boolean,
    ) => {
      if (matrices.length === 0) return;
      const inst = new THREE.InstancedMesh(geo, mat, matrices.length);
      inst.castShadow = castShadow;
      inst.receiveShadow = receiveShadow;
      for (let i = 0; i < matrices.length; i++) {
        inst.setMatrixAt(i, matrices[i]!);
      }
      inst.instanceMatrix.needsUpdate = true;
      inst.computeBoundingSphere?.();
      targetGroup.add(inst);
    };

    emitBatch(SHARED_UNIT_WIRE_GEO, SHARED_WIRE_MAT, this.wireMatrices, false, false);
    emitBatch(SHARED_UNIT_MSG_GEO, SHARED_MESSENGER_MAT, this.msgMatrices, false, false);
    emitBatch(SHARED_UNIT_DROPPER_GEO, SHARED_MESSENGER_MAT, this.dropperMatrices, false, false);
    emitBatch(SHARED_BASE_GEO, SHARED_BASE_MAT, this.mastBaseMatrices, true, true);
    emitBatch(SHARED_POLE_GEO, SHARED_MAST_MAT, this.mastPoleMatrices, true, false);
    emitBatch(SHARED_MAST_BRACKET_POS, SHARED_MAST_MAT, this.mastBracketPosMatrices, true, false);
    emitBatch(SHARED_MAST_BRACKET_NEG, SHARED_MAST_MAT, this.mastBracketNegMatrices, true, false);
    emitBatch(SHARED_MAST_INS_POS, SHARED_INS_MAT, this.mastInsPosMatrices, false, false);
    emitBatch(SHARED_MAST_INS_NEG, SHARED_INS_MAT, this.mastInsNegMatrices, false, false);
    emitBatch(SHARED_TENSION_HW_POS, SHARED_MAST_MAT, this.tensionHwPosMatrices, true, false);
    emitBatch(SHARED_TENSION_HW_NEG, SHARED_MAST_MAT, this.tensionHwNegMatrices, true, false);
    emitBatch(SHARED_TENSION_WEIGHT_POS, SHARED_BASE_MAT, this.tensionWeightPosMatrices, true, true);
    emitBatch(SHARED_TENSION_WEIGHT_NEG, SHARED_BASE_MAT, this.tensionWeightNegMatrices, true, true);
  }

  private cellKey(cx: number, cz: number): string {
    return `${cx}:${cz}`;
  }

  private addTrackSample(wx: number, wy: number, wz: number, r: number): void {
    const cx = Math.floor(wx / CatenaryMastContext.CELL);
    const cz = Math.floor(wz / CatenaryMastContext.CELL);
    const key = this.cellKey(cx, cz);
    let list = this.trackGrid.get(key);
    if (!list) {
      list = [];
      this.trackGrid.set(key, list);
    }
    list.push({ x: wx, y: wy, z: wz, r });
  }

  private toWorld(
    piece: TrackPiece,
    startElev: number,
    lx: number,
    ly: number,
    lz: number,
  ): { x: number; y: number; z: number } {
    const ry = -deg2rad(piece.transform.rotation);
    const cos = Math.cos(ry);
    const sin = Math.sin(ry);
    return {
      x: piece.transform.x + lx * cos + lz * sin,
      y: startElev + ly,
      z: piece.transform.y - lx * sin + lz * cos,
    };
  }

  private indexStraight(
    piece: TrackPiece,
    startElev: number,
    x1: number,
    z1: number,
    x2: number,
    z2: number,
    heightDiff: number,
    clearR: number,
  ): void {
    const len = Math.hypot(x2 - x1, z2 - z1);
    const steps = Math.max(2, Math.ceil(len / 18));
    for (let i = 0; i <= steps; i++) {
      const u = i / steps;
      const w = this.toWorld(piece, startElev, x1 + (x2 - x1) * u, heightDiff * u, z1 + (z2 - z1) * u);
      this.addTrackSample(w.x, w.y, w.z, clearR);
    }
  }

  private indexArc(
    piece: TrackPiece,
    startElev: number,
    r: number,
    rad: number,
    dir: 1 | -1,
    leadX: number,
    heightDiff: number,
    clearR: number,
  ): void {
    const arcLen = Math.abs(r * rad);
    const steps = Math.max(3, Math.ceil(arcLen / 18));
    for (let i = 0; i <= steps; i++) {
      const u = i / steps;
      const th = rad * u;
      const lx = leadX + r * Math.sin(th);
      const lz = dir * r * (1 - Math.cos(th));
      const w = this.toWorld(piece, startElev, lx, heightDiff * u, lz);
      this.addTrackSample(w.x, w.y, w.z, clearR);
    }
  }

  private indexTrackItem(item: CatenaryTrackItem): void {
    const { piece, entry, startElev, endElev } = item;
    const heightDiff = endElev - startElev;
    const gauge = entry.gauge ?? 16.5;
    const hasRoadbed = entry.hasRoadbed !== false;
    const ballastW = hasRoadbed ? (gauge / 16.5) * 36 : (gauge / 16.5) * 26;
    // Halbe Schotterbreite + 2 mm Toleranz (~20 mm bei H0 < 24 mm Mastabstand am eigenen Gleis)
    const clearR = ballastW / 2 + 2;
    const kind = entry.kind;

    if (kind === "turntable") {
      const pitR = 142;
      const w = this.toWorld(piece, startElev, 0, 0, 0);
      this.addTrackSample(w.x, w.y, w.z, pitR);
      return;
    }

    const isCurvedTurnout =
      kind === "curved-turnout-left" ||
      kind === "curved-turnout-right" ||
      ((kind === "turnout-left" || kind === "turnout-right") && !!entry.radius && !entry.length);

    if (isStraightTrackEntry(entry)) {
      const len = entry.length ?? (kind === "flex" ? 900 : 188.3);
      this.indexStraight(piece, startElev, 0, 0, len, 0, heightDiff, clearR);
    } else if (isCurvedTrackEntry(entry)) {
      const r = entry.radius ?? 437.5;
      const a = entry.angle ?? 30;
      const dir: 1 | -1 = a < 0 ? -1 : 1;
      this.indexArc(piece, startElev, r, deg2rad(Math.abs(a)), dir, 0, heightDiff, clearR);
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
      this.indexArc(piece, startElev, r1, deg2rad(Math.abs(a1)), dir, 0, heightDiff, clearR);
      if (lead > 0) this.indexStraight(piece, startElev, 0, 0, lead, 0, 0, clearR);
      this.indexArc(piece, startElev, r2, deg2rad(Math.abs(a2)), dir, lead, heightDiff, clearR);
    } else if (kind === "turnout-left" || kind === "turnout-right") {
      const len = entry.length ?? 188.3;
      const br = entry.branchRadius ?? 437.5;
      const ba = entry.branchAngle ?? 24.3;
      const dir: 1 | -1 = kind === "turnout-left" ? -1 : 1;
      this.indexStraight(piece, startElev, 0, 0, len, 0, heightDiff, clearR);
      this.indexArc(piece, startElev, br, deg2rad(Math.abs(ba)), dir, 0, heightDiff, clearR);
    } else if (kind === "three-way") {
      const len = entry.length ?? 188.3;
      const br = entry.branchRadius ?? 437.5;
      const ba = entry.branchAngle ?? 24.3;
      this.indexStraight(piece, startElev, 0, 0, len, 0, heightDiff, clearR);
      this.indexArc(piece, startElev, br, deg2rad(Math.abs(ba)), -1, 0, heightDiff, clearR);
      this.indexArc(piece, startElev, br, deg2rad(Math.abs(ba)), 1, 0, heightDiff, clearR);
    } else if (kind === "crossing") {
      const len = entry.length ?? 188.3;
      const rad = deg2rad(Math.abs(entry.angle ?? 24.3));
      const h = len / 2;
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);
      this.indexStraight(piece, startElev, -h, 0, h, 0, heightDiff, clearR);
      this.indexStraight(piece, startElev, -h * cos, -h * sin, h * cos, h * sin, heightDiff, clearR);
    } else if (kind === "double-slip") {
      const len = entry.length ?? 188.3;
      const rad = deg2rad(Math.abs(entry.angle ?? 24.3));
      const h = len / 2;
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);
      this.indexStraight(piece, startElev, 0, 0, len, 0, heightDiff, clearR);
      this.indexStraight(piece, startElev, h - h * cos, -h * sin, h + h * cos, h * sin, heightDiff, clearR);
    } else {
      const len = entry.length ?? 188.3;
      this.indexStraight(piece, startElev, 0, 0, len, 0, heightDiff, clearR);
    }
  }

  private hasPlacedMastNear(wx: number, wy: number, wz: number, horizTol = 35, vertTol = 40): boolean {
    const cx = Math.floor(wx / CatenaryMastContext.CELL);
    const cz = Math.floor(wz / CatenaryMastContext.CELL);
    const tolSq = horizTol * horizTol;
    for (let dx = -1; dx <= 1; dx++) {
      for (let dz = -1; dz <= 1; dz++) {
        const list = this.placedGrid.get(this.cellKey(cx + dx, cz + dz));
        if (!list) continue;
        for (const p of list) {
          if (Math.abs(p.y - wy) > vertTol) continue;
          const ddx = p.x - wx;
          const ddz = p.z - wz;
          if (ddx * ddx + ddz * ddz <= tolSq) return true;
        }
      }
    }
    return false;
  }

  private recordPlacedMast(wx: number, wy: number, wz: number): void {
    const cx = Math.floor(wx / CatenaryMastContext.CELL);
    const cz = Math.floor(wz / CatenaryMastContext.CELL);
    const key = this.cellKey(cx, cz);
    let list = this.placedGrid.get(key);
    if (!list) {
      list = [];
      this.placedGrid.set(key, list);
    }
    list.push({ x: wx, y: wy, z: wz });
  }

  private isPoleCollidingWithTrack(wx: number, wy: number, wz: number): boolean {
    const cx = Math.floor(wx / CatenaryMastContext.CELL);
    const cz = Math.floor(wz / CatenaryMastContext.CELL);
    for (let dx = -1; dx <= 1; dx++) {
      for (let dz = -1; dz <= 1; dz++) {
        const list = this.trackGrid.get(this.cellKey(cx + dx, cz + dz));
        if (!list) continue;
        for (const pt of list) {
          if (Math.abs(pt.y - wy) > 55) continue;
          const ddx = pt.x - wx;
          const ddz = pt.z - wz;
          if (ddx * ddx + ddz * ddz < pt.r * pt.r) return true;
        }
      }
    }
    return false;
  }

  /**
   * Prüft Duplikate und Gleiskollisionen für einen Oberleitungsmast.
   * Gibt die freie Seite (1 oder -1) zurück oder null, wenn der Mast weggelassen werden soll.
   */
  resolveMastSide(
    piece: TrackPiece,
    startElev: number,
    localX: number,
    localY: number,
    localZ: number,
    rotY: number,
    preferredSide: 1 | -1,
  ): 1 | -1 | null {
    const attachWorld = this.toWorld(piece, startElev, localX, localY, localZ);
    if (this.hasPlacedMastNear(attachWorld.x, attachWorld.y, attachWorld.z)) {
      return null;
    }

    const candidates: Array<1 | -1> = [preferredSide, (preferredSide === 1 ? -1 : 1)];
    for (const side of candidates) {
      const mastDist = 24 * side;
      const poleLx = localX + mastDist * Math.sin(rotY);
      const poleLz = localZ + mastDist * Math.cos(rotY);
      const poleWorld = this.toWorld(piece, startElev, poleLx, localY, poleLz);
      if (!this.isPoleCollidingWithTrack(poleWorld.x, poleWorld.y, poleWorld.z)) {
        this.recordPlacedMast(attachWorld.x, attachWorld.y, attachWorld.z);
        return side;
      }
    }

    return null;
  }
}

/**
 * Zeichnet ein auf dem Brückendeck (Drehscheibe / Schiebebühne) montiertes
 * Oberleitungs-Brückenportal mit zwei Stützen am Bühnenrand (z = ±15.5),
 * oberem Tragseil-Joch, unterem Fahrdraht-Seitenhalter und gerippten Isolatoren.
 */
function buildBridgeCatenaryPortal(
  catGroup: THREE.Group,
  x: number,
  z: number,
  deckY = 6.2,
  postOffsetZ = 15.5,
  contactH = 72,
  messengerH = 85,
  rotY = 0,
): void {
  const portal = new THREE.Group();
  portal.position.set(x, 0, z);
  portal.rotation.y = rotY;

  const postH = messengerH + 2 - deckY;
  const postCenterY = deckY + postH / 2;
  const postGeo = new THREE.BoxGeometry(2.2, postH, 2.2);

  for (const s of [-1, 1]) {
    const post = new THREE.Mesh(postGeo, SHARED_MAST_MAT);
    post.position.set(0, postCenterY, s * postOffsetZ);
    post.castShadow = true;
    portal.add(post);

    // Fußplatte direkt auf dem Brückendeck
    const foot = new THREE.Mesh(new THREE.BoxGeometry(4.0, 1.4, 3.6), SHARED_BASE_MAT);
    foot.position.set(0, deckY + 0.7, s * postOffsetZ);
    portal.add(foot);
  }

  // Oberer Querträger (Tragseil-Joch)
  const topSpan = postOffsetZ * 2 + 2;
  const topBeam = new THREE.Mesh(new THREE.BoxGeometry(1.6, 1.6, topSpan), SHARED_MAST_MAT);
  topBeam.position.set(0, messengerH, 0);
  topBeam.castShadow = true;
  portal.add(topBeam);

  // Tragseilklemme mittig
  const topClamp = new THREE.Mesh(new THREE.BoxGeometry(2.0, 1.8, 1.8), SHARED_MAST_MAT);
  topClamp.position.set(0, messengerH, 0);
  portal.add(topClamp);

  // Unterer Querträger / Seitenhalter für den Fahrdraht
  const botBeamY = contactH + 3.5;
  const botBeam = new THREE.Mesh(new THREE.BoxGeometry(1.1, 1.1, topSpan - 1), SHARED_MAST_MAT);
  botBeam.position.set(0, botBeamY, 0);
  portal.add(botBeam);

  // Vertikale Fahrdrahtklemme nach unten zum Fahrdraht (y = contactH)
  const clampStem = new THREE.Mesh(new THREE.BoxGeometry(1.0, botBeamY - contactH + 0.6, 1.0), SHARED_MAST_MAT);
  clampStem.position.set(0, (botBeamY + contactH) / 2, 0);
  portal.add(clampStem);

  const botClamp = new THREE.Mesh(new THREE.BoxGeometry(2.2, 1.5, 1.5), SHARED_MAST_MAT);
  botClamp.position.set(0, contactH + 0.3, 0);
  portal.add(botClamp);

  // Gerippte Keramik-Isolatoren auf beiden Querträgern
  const insParts: THREE.BufferGeometry[] = [];
  for (const s of [-1, 1]) {
    addRibbedInsulatorParts(insParts, messengerH, s * (postOffsetZ - 4.2), 0);
    addRibbedInsulatorParts(insParts, botBeamY, s * (postOffsetZ - 4.2), 0);
  }
  const insGeo = BufferGeometryUtils.mergeGeometries(insParts);
  insParts.forEach((p) => p.dispose());
  portal.add(new THREE.Mesh(insGeo, SHARED_INS_MAT));

  catGroup.add(portal);
}

/**
 * Zeichnet eine gerade Oberleitungsspanne mit durchhängendem oberen Tragseil,
 * geradem unteren Fahrdraht und dynamisch langen Hängern.
 */
function addSaggingStraightSpan(
  catGroup: THREE.Group,
  x1: number,
  z1: number,
  x2: number,
  z2: number,
  contactH = 72,
  messengerH = 85,
): void {
  const dx = x2 - x1;
  const dz = z2 - z1;
  const horizLen = Math.hypot(dx, dz);
  if (horizLen < 0.1) return;

  const unitX = new THREE.Vector3(1, 0, 0);
  const mx = (x1 + x2) / 2;
  const mz = (z1 + z2) / 2;
  const dirVec = new THREE.Vector3(dx, 0, dz).normalize();
  const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);

  // Gerader unterer Fahrdraht
  const wireMesh = new THREE.Mesh(getSharedWireGeometry(horizLen), SHARED_WIRE_MAT);
  wireMesh.position.set(mx, contactH, mz);
  wireMesh.quaternion.copy(quat);
  catGroup.add(wireMesh);

  // Durchhängendes oberes Tragseil
  const sagMax = horizLen >= 40 ? Math.min(6.0, horizLen * 0.028) : 0;
  const msgSegs = sagMax > 0.1 ? Math.max(4, Math.min(10, Math.round(horizLen / 35))) : 1;
  for (let s = 0; s < msgSegs; s++) {
    const uA = s / msgSegs;
    const uB = (s + 1) / msgSegs;
    const sagA = 4 * sagMax * uA * (1 - uA);
    const sagB = 4 * sagMax * uB * (1 - uB);
    const ax = x1 + dx * uA;
    const ay = messengerH - sagA;
    const az = z1 + dz * uA;
    const bx = x1 + dx * uB;
    const by = messengerH - sagB;
    const bz = z1 + dz * uB;
    const segLen = Math.hypot(bx - ax, by - ay, bz - az);
    if (segLen < 0.01) continue;
    const segDir = new THREE.Vector3(bx - ax, by - ay, bz - az).divideScalar(segLen);
    const segQuat = new THREE.Quaternion().setFromUnitVectors(unitX, segDir);
    const msgMesh = new THREE.Mesh(getSharedMessengerGeometry(segLen), SHARED_MESSENGER_MAT);
    msgMesh.position.set((ax + bx) / 2, (ay + by) / 2, (az + bz) / 2);
    msgMesh.quaternion.copy(segQuat);
    catGroup.add(msgMesh);
  }

  // Variable Hänger zwischen Tragseil und Fahrdraht
  const dropperCount = Math.max(1, Math.floor(horizLen / 35));
  if (dropperCount > 0) {
    const droppersInst = new THREE.InstancedMesh(SHARED_UNIT_DROPPER_GEO, SHARED_MESSENGER_MAT, dropperCount);
    const dummy = new THREE.Object3D();
    for (let i = 1; i <= dropperCount; i++) {
      const u = i / (dropperCount + 1);
      const sag = 4 * sagMax * u * (1 - u);
      const dropLen = Math.max(1, messengerH - sag - contactH);
      dummy.position.set(x1 + dx * u, contactH + dropLen / 2, z1 + dz * u);
      dummy.scale.set(1, dropLen, 1);
      dummy.updateMatrix();
      droppersInst.setMatrixAt(i - 1, dummy.matrix);
    }
    droppersInst.instanceMatrix.needsUpdate = true;
    catGroup.add(droppersInst);
  }
}

export function buildTurntableCatenary3D(
  group: THREE.Group,
  _piece: TrackPiece,
  entry: CatalogEntry,
  _currentLayout?: Layout,
  _mastCtx?: CatenaryMastContext,
  _startElev = 0,
) {
  const catGroup = new THREE.Group();
  const contactH = 72;
  const messengerH = 85;

  // Keine Oberleitung auf der drehenden Bühne selbst – stattdessen zentrale Oberleitungsspinne
  // mit Haltern am Außenende der Drehscheibe und radialen Abspannseilen für alle 30 Anschlüsse
  const pitR = 139;
  const outerR = entry.radius ?? 189;
  const outerHolderR = outerR - 4;
  const spiderRingR = 18;
  const spiderRingY = messengerH + 4;

  // Zentraler Oberleitungsring ("Drehscheiben-Spinne"): oberer Tragring + unterer Fahrdrahtring
  const spiderRingGeo = new THREE.TorusGeometry(spiderRingR, 0.75, 8, 28);
  spiderRingGeo.rotateX(Math.PI / 2);
  const spiderRing = new THREE.Mesh(spiderRingGeo, SHARED_MAST_MAT);
  spiderRing.position.set(0, spiderRingY, 0);
  catGroup.add(spiderRing);

  const contactRingGeo = new THREE.TorusGeometry(spiderRingR, 0.55, 8, 28);
  contactRingGeo.rotateX(Math.PI / 2);
  const contactRing = new THREE.Mesh(contactRingGeo, SHARED_WIRE_MAT);
  contactRing.position.set(0, contactH, 0);
  catGroup.add(contactRing);

  // Vertikale Hänger zwischen oberem Tragring und unterem Fahrdrahtring
  const ringDropH = spiderRingY - contactH;
  const ringDropDummy = new THREE.Object3D();
  const ringDroppers = new THREE.InstancedMesh(SHARED_UNIT_DROPPER_GEO, SHARED_MESSENGER_MAT, 6);
  for (let i = 0; i < 6; i++) {
    const a = (i * Math.PI * 2) / 6;
    ringDropDummy.position.set(spiderRingR * Math.cos(a), contactH + ringDropH / 2, spiderRingR * Math.sin(a));
    ringDropDummy.scale.set(1, ringDropH, 1);
    ringDropDummy.updateMatrix();
    ringDroppers.setMatrixAt(i, ringDropDummy.matrix);
  }
  ringDroppers.instanceMatrix.needsUpdate = true;
  catGroup.add(ringDroppers);

  const unitX = new THREE.Vector3(1, 0, 0);
  const addRadialLine = (
    ax: number,
    ay: number,
    az: number,
    bx: number,
    by: number,
    bz: number,
    isContactWire: boolean,
  ) => {
    const dx = bx - ax;
    const dy = by - ay;
    const dz = bz - az;
    const len = Math.hypot(dx, dy, dz);
    if (len < 0.1) return;
    const dirVec = new THREE.Vector3(dx, dy, dz).divideScalar(len);
    const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);
    const mesh = new THREE.Mesh(
      isContactWire ? getSharedWireGeometry(len) : getSharedMessengerGeometry(len),
      isContactWire ? SHARED_WIRE_MAT : SHARED_MESSENGER_MAT,
    );
    mesh.position.set((ax + bx) / 2, (ay + by) / 2, (az + bz) / 2);
    mesh.quaternion.copy(quat);
    catGroup.add(mesh);
  };

  // Alle 30 Anschluss-Stümpfe am Außenring der Drehscheibe mit durchgehendem Fahrdraht bis zum
  // zentralen Fahrdrahtring, Außenrand-Halter (am Ende der Drehscheibe) & Spinnen-Abspannseil
  const localConns = entry.localConnectors || [];
  const slotAngles: number[] =
    localConns.length > 0
      ? localConns.map((c) => c.angle)
      : Array.from({ length: 30 }, (_, i) => i * 12);

  // Hänger entlang aller 30 radialen Spinnen-Abschnitte gebatcht in einem InstancedMesh
  const radialDropperFracs = [0.25, 0.55, 0.85];
  const radialDroppers = new THREE.InstancedMesh(
    SHARED_UNIT_DROPPER_GEO,
    SHARED_MESSENGER_MAT,
    slotAngles.length * radialDropperFracs.length,
  );
  let rdIdx = 0;

  for (const angleDeg of slotAngles) {
    const rad = deg2rad(angleDeg);
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    const xRing = spiderRingR * cos;
    const zRing = spiderRingR * sin;
    const xPit = pitR * cos;
    const zPit = pitR * sin;
    const xOut = outerR * cos;
    const zOut = outerR * sin;

    // 1. Durchgehender unterer Kupfer-Fahrdraht vom zentralen Fahrdrahtring bis zum Außenende
    addRadialLine(xRing, contactH, zRing, xOut, contactH, zOut, true);

    // 2. Äußeres Tragseil (Grubenrand bis Außenende) + oberes Spinnen-Abspannseil (Grubenrand bis Tragring)
    addRadialLine(xPit, messengerH, zPit, xOut, messengerH, zOut, false);
    addRadialLine(xPit, messengerH, zPit, xRing, spiderRingY, zRing, false);

    // 3. Hänger zwischen Spinnen-Tragseil und radialem Fahrdraht über der Grube
    for (const u of radialDropperFracs) {
      const rx = xRing + (xPit - xRing) * u;
      const rz = zRing + (zPit - zRing) * u;
      const topY = spiderRingY + (messengerH - spiderRingY) * u;
      const dropLen = Math.max(1, topY - contactH);
      ringDropDummy.position.set(rx, contactH + dropLen / 2, rz);
      ringDropDummy.scale.set(1, dropLen, 1);
      ringDropDummy.updateMatrix();
      radialDroppers.setMatrixAt(rdIdx++, ringDropDummy.matrix);
    }

    // 4. Oberleitungs-Halter (Portal) direkt am Außenende der Drehscheibe (r = outerR - 4)
    buildBridgeCatenaryPortal(
      catGroup,
      outerHolderR * cos,
      outerHolderR * sin,
      6.0,
      13.0,
      contactH,
      messengerH,
      -rad,
    );
  }

  radialDroppers.instanceMatrix.needsUpdate = true;
  catGroup.add(radialDroppers);

  group.add(catGroup);
}

export function buildTransferTableCatenary3D(
  group: THREE.Group,
  piece: TrackPiece,
  entry: CatalogEntry,
  currentLayout?: Layout,
  mastCtx?: CatenaryMastContext,
  startElev = 0,
) {
  const catGroup = new THREE.Group();
  const contactH = 72;
  const messengerH = 85;

  const buildOuterMast = (x: number, z: number, rotY = 0, side: 1 | -1 = 1) => {
    let resolvedSide: 1 | -1 = side;
    if (mastCtx) {
      const res = mastCtx.resolveMastSide(piece, startElev, x, 0, z, rotY, side);
      if (res === null) return;
      resolvedSide = res;
    }

    const mastGroup = new THREE.Group();
    mastGroup.position.set(x, 0, z);
    mastGroup.rotation.y = rotY;

    const mastDist = 24 * resolvedSide;
    const baseMesh = new THREE.Mesh(SHARED_BASE_GEO, SHARED_BASE_MAT);
    baseMesh.position.set(0, 4, mastDist);
    baseMesh.castShadow = true;
    baseMesh.receiveShadow = true;
    mastGroup.add(baseMesh);

    const poleMesh = new THREE.Mesh(SHARED_POLE_GEO, SHARED_MAST_MAT);
    poleMesh.position.set(0, 47, mastDist);
    poleMesh.castShadow = true;
    mastGroup.add(poleMesh);

    const bracketGeo = resolvedSide === -1 ? SHARED_MAST_BRACKET_NEG : SHARED_MAST_BRACKET_POS;
    const bracketMesh = new THREE.Mesh(bracketGeo, SHARED_MAST_MAT);
    bracketMesh.castShadow = true;
    mastGroup.add(bracketMesh);

    const insGeo = resolvedSide === -1 ? SHARED_MAST_INS_NEG : SHARED_MAST_INS_POS;
    mastGroup.add(new THREE.Mesh(insGeo, SHARED_INS_MAT));

    catGroup.add(mastGroup);
  };

  // Schiebebühne: Brückenportale stehen direkt auf dem Bühnendeck (z = bridgeZ ± 15.5)
  const l = entry.length ?? 360;
  const pitLeft = 36;
  const pitRight = l - 36;
  const bridgeZ = 154.8;

  addSaggingStraightSpan(catGroup, pitLeft, bridgeZ, pitRight, bridgeZ, contactH, messengerH);
  buildBridgeCatenaryPortal(catGroup, pitLeft + 12, bridgeZ, 6.2, 15.5, contactH, messengerH);
  buildBridgeCatenaryPortal(catGroup, pitRight - 12, bridgeZ, 6.2, 15.5, contactH, messengerH);

  // Anschluss-Stutzen links und rechts (nur wenn angeschlossenes Gleis Oberleitung hat)
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
        addSaggingStraightSpan(catGroup, 0, z, pitLeft, z, contactH, messengerH);
        buildOuterMast(0, z, -Math.PI, 1);
        // Oberleitungs-Halter (Portal) direkt am linken Grubenrand für den angeschlossenen Gleis-Stutzen
        buildBridgeCatenaryPortal(catGroup, pitLeft - 3, z, 6.0, 15.5, contactH, messengerH);
      } else {
        addSaggingStraightSpan(catGroup, pitRight, z, l, z, contactH, messengerH);
        buildOuterMast(l, z, 0, 1);
        // Oberleitungs-Halter (Portal) direkt am rechten Grubenrand für den angeschlossenen Gleis-Stutzen
        buildBridgeCatenaryPortal(catGroup, pitRight + 3, z, 6.0, 15.5, contactH, messengerH);
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
  mastCtx?: CatenaryMastContext,
) {
  const catGroup = new THREE.Group();

  const wireMat = SHARED_WIRE_MAT;
  const messengerMat = SHARED_MESSENGER_MAT;
  const mastMat = SHARED_MAST_MAT;
  const baseMat = SHARED_BASE_MAT;
  const insMat = SHARED_INS_MAT;

  const contactH = 72;
  const messengerH = 85;
  const localPlacedMasts: Array<{ x: number; y: number; z: number }> = [];

  const buildMast = (x: number, z: number, rotY = 0, side: 1 | -1 = 1, y = 0) => {
    // Lokale Duplikatprüfung (z. B. gemeinsame Weichenwurzel bei Standalone-Aufruf)
    for (const pm of localPlacedMasts) {
      if (Math.hypot(pm.x - x, pm.z - z) < 30 && Math.abs(pm.y - y) < 40) {
        return;
      }
    }

    let resolvedSide: 1 | -1 = side;
    if (mastCtx && piece) {
      const res = mastCtx.resolveMastSide(piece, startElev, x, y, z, rotY, side);
      if (res === null) return;
      resolvedSide = res;
      localPlacedMasts.push({ x, y, z });
      mastCtx.addBatchedMast(piece, startElev, x, y, z, rotY, resolvedSide);
      return;
    }

    localPlacedMasts.push({ x, y, z });

    const mastGroup = new THREE.Group();
    mastGroup.position.set(x, y, z);
    mastGroup.rotation.y = rotY;

    const mastDist = 24 * resolvedSide;

    const baseMesh = new THREE.Mesh(SHARED_BASE_GEO, baseMat);
    baseMesh.position.set(0, 4, mastDist);
    baseMesh.castShadow = true;
    baseMesh.receiveShadow = true;
    mastGroup.add(baseMesh);

    const poleMesh = new THREE.Mesh(SHARED_POLE_GEO, mastMat);
    poleMesh.position.set(0, 47, mastDist);
    poleMesh.castShadow = true;
    mastGroup.add(poleMesh);

    const bracketGeo = resolvedSide === -1 ? SHARED_MAST_BRACKET_NEG : SHARED_MAST_BRACKET_POS;
    const bracketMesh = new THREE.Mesh(bracketGeo, mastMat);
    bracketMesh.castShadow = true;
    mastGroup.add(bracketMesh);

    const insGeo = resolvedSide === -1 ? SHARED_MAST_INS_NEG : SHARED_MAST_INS_POS;
    const insMesh = new THREE.Mesh(insGeo, insMat);
    mastGroup.add(insMesh);

    catGroup.add(mastGroup);
  };

  const unitX = new THREE.Vector3(1, 0, 0);

  const addStraightWire = (
    x1: number,
    z1: number,
    x2: number,
    z2: number,
    withMasts = true,
    mastSide: 1 | -1 = 1,
    y1 = 0,
    y2 = 0,
    placeStartMast = true,
    placeEndMast = true,
  ) => {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const dz = z2 - z1;
    const horizLen = Math.hypot(dx, dz);
    if (horizLen < 0.1) return;
    const angle = Math.atan2(dz, dx);

    const sagMax = horizLen >= 30 ? Math.min(6.2, horizLen * 0.032) : 0;
    const msgSegs = sagMax > 0.1 ? Math.max(4, Math.min(10, Math.round(horizLen / 35))) : 1;

    if (mastCtx && piece) {
      // Gerader unterer Fahrdraht (konstante Höhe für Stromabnehmer)
      mastCtx.addBatchedContactSegment(piece, startElev, x1, y1, z1, x2, y2, z2, contactH);

      // Durchhängendes oberes Tragseil (Kettenlinie / Parabel zwischen den Masten)
      for (let s = 0; s < msgSegs; s++) {
        const uA = s / msgSegs;
        const uB = (s + 1) / msgSegs;
        const sagA = 4 * sagMax * uA * (1 - uA);
        const sagB = 4 * sagMax * uB * (1 - uB);
        mastCtx.addBatchedMessengerSegment(
          piece,
          startElev,
          x1 + dx * uA,
          y1 + dy * uA + messengerH - sagA,
          z1 + dz * uA,
          x1 + dx * uB,
          y1 + dy * uB + messengerH - sagB,
          z1 + dz * uB,
        );
      }

      // Hänger mit variabler Länge exakt zwischen durchhängendem Tragseil und geradem Fahrdraht
      const dropperCount = Math.max(1, Math.floor(horizLen / 35));
      for (let i = 1; i <= dropperCount; i++) {
        const u = i / (dropperCount + 1);
        const sag = 4 * sagMax * u * (1 - u);
        const dropLen = Math.max(1, messengerH - sag - contactH);
        mastCtx.addBatchedDropper(
          piece,
          startElev,
          x1 + dx * u,
          y1 + dy * u,
          z1 + dz * u,
          contactH + dropLen / 2,
          dropLen,
        );
      }
    } else {
      const totalLen = Math.hypot(dx, dy, dz);
      const mx = (x1 + x2) / 2;
      const my = (y1 + y2) / 2;
      const mz = (z1 + z2) / 2;

      const dirVec = new THREE.Vector3(dx, dy, dz).normalize();
      const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);

      const wireMesh = new THREE.Mesh(getSharedWireGeometry(totalLen), wireMat);
      wireMesh.position.set(mx, contactH + my, mz);
      wireMesh.quaternion.copy(quat);
      catGroup.add(wireMesh);

      for (let s = 0; s < msgSegs; s++) {
        const uA = s / msgSegs;
        const uB = (s + 1) / msgSegs;
        const sagA = 4 * sagMax * uA * (1 - uA);
        const sagB = 4 * sagMax * uB * (1 - uB);
        const ax = x1 + dx * uA;
        const ay = y1 + dy * uA + messengerH - sagA;
        const az = z1 + dz * uA;
        const bx = x1 + dx * uB;
        const by = y1 + dy * uB + messengerH - sagB;
        const bz = z1 + dz * uB;
        const segLen = Math.hypot(bx - ax, by - ay, bz - az);
        if (segLen < 0.01) continue;
        const segDir = new THREE.Vector3(bx - ax, by - ay, bz - az).divideScalar(segLen);
        const segQuat = new THREE.Quaternion().setFromUnitVectors(unitX, segDir);
        const msgMesh = new THREE.Mesh(getSharedMessengerGeometry(segLen), messengerMat);
        msgMesh.position.set((ax + bx) / 2, (ay + by) / 2, (az + bz) / 2);
        msgMesh.quaternion.copy(segQuat);
        catGroup.add(msgMesh);
      }

      const dropperCount = Math.max(1, Math.floor(horizLen / 35));
      if (dropperCount > 0) {
        const droppersInst = new THREE.InstancedMesh(SHARED_UNIT_DROPPER_GEO, messengerMat, dropperCount);
        const dummy = new THREE.Object3D();
        for (let i = 1; i <= dropperCount; i++) {
          const u = i / (dropperCount + 1);
          const sag = 4 * sagMax * u * (1 - u);
          const dropLen = Math.max(1, messengerH - sag - contactH);
          dummy.position.set(x1 + dx * u, contactH + dropLen / 2 + (y1 + dy * u), z1 + dz * u);
          dummy.scale.set(1, dropLen, 1);
          dummy.updateMatrix();
          droppersInst.setMatrixAt(i - 1, dummy.matrix);
        }
        droppersInst.instanceMatrix.needsUpdate = true;
        catGroup.add(droppersInst);
      }
    }

    if (withMasts) {
      if (placeStartMast) {
        buildMast(x1, z1, -angle, mastSide, y1);
      }
      if (placeEndMast && horizLen >= 120) {
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
    mastSide: 1 | -1 = 1,
    startH = 0,
    heightDiff = 0,
    placeStartMast = true,
    placeEndMast = true,
  ) => {
    const segs = Math.max(4, Math.floor(rad / deg2rad(5)));
    const arcLen = Math.abs(r * rad);
    const sagMax = arcLen >= 30 ? Math.min(6.2, arcLen * 0.032) : 0;

    for (let i = 0; i < segs; i++) {
      const u1 = i / segs;
      const u2 = (i + 1) / segs;
      const uMid = (u1 + u2) * 0.5;
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

      const sag1 = 4 * sagMax * u1 * (1 - u1);
      const sag2 = 4 * sagMax * u2 * (1 - u2);
      const sagMid = 4 * sagMax * uMid * (1 - uMid);
      const dropLen = Math.max(1, messengerH - sagMid - contactH);

      const mx = (x1 + x2) / 2;
      const my = (y1 + y2) / 2;
      const mz = (z1 + z2) / 2;

      if (mastCtx && piece) {
        mastCtx.addBatchedContactSegment(piece, startElev, x1, y1, z1, x2, y2, z2, contactH);
        mastCtx.addBatchedMessengerSegment(piece, startElev, x1, y1 + messengerH - sag1, z1, x2, y2 + messengerH - sag2, z2);
        mastCtx.addBatchedDropper(piece, startElev, mx, my, mz, contactH + dropLen / 2, dropLen);
      } else {
        const dirVec = new THREE.Vector3(dx, dy, dz).normalize();
        const quat = new THREE.Quaternion().setFromUnitVectors(unitX, dirVec);

        const wMesh = new THREE.Mesh(getSharedWireGeometry(segLen), wireMat);
        wMesh.position.set(mx, contactH + my, mz);
        wMesh.quaternion.copy(quat);
        catGroup.add(wMesh);

        const mDy = (y2 + messengerH - sag2) - (y1 + messengerH - sag1);
        const mLen = Math.hypot(dx, mDy, dz);
        const mDir = new THREE.Vector3(dx, mDy, dz).divideScalar(Math.max(0.01, mLen));
        const mQuat = new THREE.Quaternion().setFromUnitVectors(unitX, mDir);
        const mMesh = new THREE.Mesh(getSharedMessengerGeometry(mLen), messengerMat);
        mMesh.position.set(mx, (y1 + y2) / 2 + messengerH - (sag1 + sag2) / 2, mz);
        mMesh.quaternion.copy(mQuat);
        catGroup.add(mMesh);

        const dropMesh = new THREE.Mesh(SHARED_UNIT_DROPPER_GEO, messengerMat);
        dropMesh.position.set(mx, contactH + dropLen / 2 + my, mz);
        dropMesh.scale.set(1, dropLen, 1);
        catGroup.add(dropMesh);
      }
    }

    if (withMasts) {
      if (placeStartMast) {
        buildMast(leadX, 0, 0, mastSide, startH);
      }
      if (placeEndMast) {
        const endX = leadX + r * Math.sin(rad);
        const endZ = dir * r * (1 - Math.cos(rad));
        buildMast(endX, endZ, -dir * rad, mastSide, startH + heightDiff);
      }
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
    addCurvedWire(r, absRad, dir, 0, true, -dir as 1 | -1, 0, heightDiff);
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

    // Nur ein Mast an der Wurzel (0,0) und je ein äußerer Mast an den beiden Bogenenden
    // Kein Mast bei (lead, 0) mitten zwischen den Weichenzweigen!
    addCurvedWire(r1, deg2rad(Math.abs(a1)), dir, 0, true, -dir as 1 | -1, 0, heightDiff, true, true);
    if (lead > 0) {
      addStraightWire(0, 0, lead, 0, false, 1, 0, heightDiff);
    }
    addCurvedWire(r2, deg2rad(Math.abs(a2)), dir, lead, true, dir, 0, heightDiff, false, true);
  } else if (kind === "turnout-left" || kind === "turnout-right") {
    const len = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const dir: 1 | -1 = kind === "turnout-left" ? -1 : 1;

    // Gerade: Mast an Wurzel & Ende auf der Außenseite (-dir, weg vom Abzweig)
    // Abzweig: kein doppelter Mast an der Wurzel, nur am Bogenende auf der Außenseite (+dir)
    addStraightWire(0, 0, len, 0, true, -dir as 1 | -1, 0, heightDiff, true, true);
    addCurvedWire(br, deg2rad(Math.abs(ba)), dir, 0, true, dir, 0, heightDiff, false, true);
  } else if (kind === "three-way") {
    const len = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;

    // Ein Mast an der Wurzel, kein Mast am Mittelende (eingeschlossen zwischen den beiden Bögen),
    // je ein äußerer Mast an den beiden Außenbögen
    addStraightWire(0, 0, len, 0, true, 1, 0, heightDiff, true, false);
    addCurvedWire(br, deg2rad(Math.abs(ba)), -1, 0, true, -1, 0, heightDiff, false, true);
    addCurvedWire(br, deg2rad(Math.abs(ba)), 1, 0, true, 1, 0, heightDiff, false, true);
  } else if (kind === "crossing") {
    const len = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(Math.abs(a));
    const h = len / 2;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    addStraightWire(-h, 0, h, 0, true, 1, 0, heightDiff);
    addStraightWire(-h * cos, -h * sin, h * cos, h * sin, true, -1, 0, heightDiff);
  } else if (kind === "double-slip") {
    const len = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(Math.abs(a));
    const h = len / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    addStraightWire(0, 0, len, 0, true, 1, 0, heightDiff);
    addStraightWire(cx - h * cos, -h * sin, cx + h * cos, h * sin, true, -1, 0, heightDiff);
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
