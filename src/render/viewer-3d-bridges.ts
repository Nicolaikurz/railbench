import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import type { Layout, Layer, TrackPiece, Vec2 } from "@/core/types";
import { deg2rad, distanceToTrackPiece, isCurvedTrackEntry, isStraightTrackEntry, transformPoint, untransformPoint } from "@/core/geometry";
import { buildLightSignal3D } from "@/render/builtin-models-3d";
import { applyCurveElevation } from "./viewer-3d-roads";

// Shared materials for turnout accessories and bridges
const SHARED_TURNOUT_METAL_MAT = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.5, metalness: 0.8 });
SHARED_TURNOUT_METAL_MAT.userData.shared = true;

const SHARED_TURNOUT_BASE_MAT = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.8 });
SHARED_TURNOUT_BASE_MAT.userData.shared = true;

const SHARED_TURNOUT_HOUSING_MAT = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.7 });
SHARED_TURNOUT_HOUSING_MAT.userData.shared = true;

const SHARED_TURNOUT_LENS_MAT = new THREE.MeshStandardMaterial({
  color: 0xfef08a,
  emissive: 0xfef08a,
  emissiveIntensity: 0.8,
  roughness: 0.3,
});
SHARED_TURNOUT_LENS_MAT.userData.shared = true;

const SHARED_TURNOUT_DEC_MAT = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.4, metalness: 0.5 });
SHARED_TURNOUT_DEC_MAT.userData.shared = true;

const SHARED_TURNOUT_LED_MAT = new THREE.MeshStandardMaterial({
  color: 0x38bdf8,
  emissive: 0x0284c7,
  emissiveIntensity: 0.9,
});
SHARED_TURNOUT_LED_MAT.userData.shared = true;

const SHARED_BRIDGE_STEEL_MAT = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.7 });
SHARED_BRIDGE_STEEL_MAT.userData.shared = true;

const SHARED_BRIDGE_DECK_MAT = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.9 });
SHARED_BRIDGE_DECK_MAT.userData.shared = true;

const SHARED_BRIDGE_RAIL_MAT = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.4, metalness: 0.8 });
SHARED_BRIDGE_RAIL_MAT.userData.shared = true;

const SHARED_BRIDGE_PILLAR_MAT = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.85 });
SHARED_BRIDGE_PILLAR_MAT.userData.shared = true;

// Shared turnout accessory geometries
const SHARED_ROD_GEO = new THREE.CylinderGeometry(0.8, 0.8, 22, 6);
SHARED_ROD_GEO.userData.shared = true;

const SHARED_TURNOUT_BASE_GEO = new THREE.CylinderGeometry(3.5, 4, 3, 10);
SHARED_TURNOUT_BASE_GEO.userData.shared = true;

const SHARED_TURNOUT_HOUSING_GEO = new THREE.BoxGeometry(5.5, 6, 5.5);
SHARED_TURNOUT_HOUSING_GEO.userData.shared = true;

const SHARED_TURNOUT_CAP_GEO = new THREE.ConeGeometry(4.2, 2.0, 4);
SHARED_TURNOUT_CAP_GEO.rotateY(Math.PI / 4);
SHARED_TURNOUT_CAP_GEO.userData.shared = true;

type LanternSymbol = "straight" | "round" | "left" | "right";
const _lanternFaceMatCache = new Map<LanternSymbol, THREE.MeshStandardMaterial>();

/**
 * Erzeugt eine gecachte Weichenlaternen-Textur nach DB-Weichensignal (Wn 1 / Wn 2):
 * - "straight": senkrechter weißer Balken (Geradeaus)
 * - "round":    weißer Kreis / Punkt (Abzweig Fersenansicht)
 * - "left":     weißer Pfeil nach links (Abzweig links)
 * - "right":    weißer Pfeil nach rechts (Abzweig rechts)
 */
function getLanternFaceMaterial(symbol: LanternSymbol): THREE.MeshStandardMaterial {
  let mat = _lanternFaceMatCache.get(symbol);
  if (mat) return mat;

  let tex: THREE.CanvasTexture | null = null;
  if (typeof document !== "undefined") {
    const canvas = document.createElement("canvas");
    canvas.width = 64;
    canvas.height = 64;
    const ctx = canvas.getContext("2d");
    if (ctx) {
      // Schwarzes Laternengehäuse
      ctx.fillStyle = "#0b1120";
      ctx.fillRect(0, 0, 64, 64);

      // Feiner Metallrahmen
      ctx.strokeStyle = "#334155";
      ctx.lineWidth = 4;
      ctx.strokeRect(2, 2, 60, 60);

      // Leuchtendes Milchglas-Weichensignal (warmweiß)
      ctx.fillStyle = "#fef9c3";
      if (symbol === "straight") {
        // Wn 1: Senkrechter Balken
        ctx.fillRect(25, 10, 14, 44);
      } else if (symbol === "round") {
        // Wn 2b: Weißer Kreis (Fersenansicht)
        ctx.beginPath();
        ctx.arc(32, 32, 16, 0, Math.PI * 2);
        ctx.fill();
      } else if (symbol === "left") {
        // Wn 2a: Nach links geneigter/fallender weißer Balken (\)
        ctx.save();
        ctx.translate(32, 32);
        ctx.rotate(-Math.PI / 4);
        ctx.fillRect(-7, -23, 14, 46);
        ctx.restore();
      } else if (symbol === "right") {
        // Wn 2a: Nach rechts geneigter/fallender weißer Balken (/)
        ctx.save();
        ctx.translate(32, 32);
        ctx.rotate(Math.PI / 4);
        ctx.fillRect(-7, -23, 14, 46);
        ctx.restore();
      }

      tex = new THREE.CanvasTexture(canvas);
      tex.colorSpace = THREE.SRGBColorSpace;
      tex.needsUpdate = true;
    }
  }

  mat = tex
    ? new THREE.MeshStandardMaterial({
        map: tex,
        emissiveMap: tex,
        emissive: 0xfffbeb,
        emissiveIntensity: 0.95,
        roughness: 0.35,
      })
    : SHARED_TURNOUT_LENS_MAT;
  mat.userData.shared = true;
  _lanternFaceMatCache.set(symbol, mat);
  return mat;
}

const SHARED_TURNOUT_DEC_GEO = new THREE.BoxGeometry(9, 3, 12);
SHARED_TURNOUT_DEC_GEO.userData.shared = true;

const SHARED_TURNOUT_LED_GEO = new THREE.CylinderGeometry(0.8, 0.8, 1, 8);
SHARED_TURNOUT_LED_GEO.userData.shared = true;

export function buildTurnoutAccessories3D(group: THREE.Group, entry: CatalogEntry, piece: TrackPiece) {
  const kind = entry.kind;
  const isLeft = kind === "turnout-left" || kind === "curved-turnout-left";
  const isRight = kind === "turnout-right" || kind === "curved-turnout-right";
  // Auf der Abzweigseite (Turn-Side) platzieren: in 3D ist links = -Z, rechts = +Z
  const lanternZ = isLeft ? -22 : 22;
  const lanternX = 26;

  const accGroup = new THREE.Group();

  // 1. Stellstange (metallisch)
  const rod = new THREE.Mesh(SHARED_ROD_GEO, SHARED_TURNOUT_METAL_MAT);
  rod.rotation.x = Math.PI / 2;
  rod.position.set(lanternX, 3.5, lanternZ / 2);
  accGroup.add(rod);

  if (piece.hasLantern) {
    // 2. Sockel & Laternengehäuse mit authentischen Weichensignal-Bildern
    const base = new THREE.Mesh(SHARED_TURNOUT_BASE_GEO, SHARED_TURNOUT_BASE_MAT);
    base.position.set(lanternX, 2.5, lanternZ);
    base.castShadow = true;
    accGroup.add(base);

    // 6 Flächen der BoxGeometry: [+X (Ferse), -X (Spitze), +Y (Dach), -Y (Boden), +Z (Seite), -Z (Seite)]
    const matStraight = getLanternFaceMaterial("straight");
    const matRound = getLanternFaceMaterial("round");
    const matLeft = getLanternFaceMaterial("left");
    const matRight = getLanternFaceMaterial("right");

    const faceMats: THREE.Material[] = isLeft
      ? [matRound, matLeft, SHARED_TURNOUT_HOUSING_MAT, SHARED_TURNOUT_HOUSING_MAT, matStraight, matStraight]
      : isRight
        ? [matRound, matRight, SHARED_TURNOUT_HOUSING_MAT, SHARED_TURNOUT_HOUSING_MAT, matStraight, matStraight]
        : [matStraight, matStraight, SHARED_TURNOUT_HOUSING_MAT, SHARED_TURNOUT_HOUSING_MAT, matRight, matLeft];

    const housing = new THREE.Mesh(SHARED_TURNOUT_HOUSING_GEO, faceMats);
    housing.position.set(lanternX, 6.8, lanternZ);
    housing.castShadow = true;
    accGroup.add(housing);

    // Dachhaube der Weichenlaterne
    const cap = new THREE.Mesh(SHARED_TURNOUT_CAP_GEO, SHARED_TURNOUT_HOUSING_MAT);
    cap.position.set(lanternX, 10.6, lanternZ);
    cap.castShadow = true;
    accGroup.add(cap);
  }

  if (piece.hasDecoder) {
    // 3. Digitaldecoder-Kasten
    const decX = piece.hasLantern ? lanternX + 11 : lanternX;
    const decZ = lanternZ;
    const dec = new THREE.Mesh(SHARED_TURNOUT_DEC_GEO, SHARED_TURNOUT_DEC_MAT);
    dec.position.set(decX, 2, decZ);
    dec.castShadow = true;
    accGroup.add(dec);

    // Mini-LED Kontrollleuchte auf dem Decoder
    const led = new THREE.Mesh(SHARED_TURNOUT_LED_GEO, SHARED_TURNOUT_LED_MAT);
    led.position.set(decX + 2.5, 3.8, decZ);
    accGroup.add(led);
  }

  group.add(accGroup);
}

export function buildTrackSignal3D(group: THREE.Group, entry: CatalogEntry, piece: TrackPiece): void {
  const gauge = entry.gauge ?? 16.5;
  const dist = gauge * 0.95 + 7;
  const l = entry.length ?? (entry.radius ? (entry.radius * deg2rad(Math.abs(entry.angle ?? 30))) : 188.3);
  const sideConfig = piece.signalSide ?? "right";
  const dirConfig = piece.signalDirection ?? "forward";

  const sides: Array<1 | -1> =
    sideConfig === "both" ? [-1, 1] : sideConfig === "left" ? [1] : [-1];
  const directions: boolean[] =
    dirConfig === "both" ? [true, false] : [dirConfig === "forward"];

  for (const sideFactor of sides) {
    for (const isForward of directions) {
      const signalGroup = new THREE.Group();
      buildLightSignal3D(signalGroup);

      // Scale realistically next to the track (buildLightSignal3D is 85mm tall, scale ~0.6 -> ~50mm)
      signalGroup.scale.set(0.6, 0.6, 0.6);

      if (isCurvedTrackEntry(entry)) {
        const r = entry.radius ?? 437.5;
        const a = entry.angle ?? 30;
        const deg = Math.abs(a);
        const dir: 1 | -1 = a < 0 ? -1 : 1;
        const rad = deg2rad(deg);
        const u = isForward ? 0.35 : 0.65;
        const phi = u * rad;
        const rSig = r - sideFactor * dir * dist;
        const px = rSig * Math.sin(phi);
        const pz = dir * (r - rSig * Math.cos(phi));

        signalGroup.position.set(px, 0, pz);
        signalGroup.rotation.y = isForward ? (-dir * phi - Math.PI / 2) : (-dir * phi + Math.PI / 2);
      } else {
        const sigX = isForward ? Math.min(l * 0.35, 45) : l - Math.min(l * 0.35, 45);
        const sigZ = sideFactor * dist;
        signalGroup.position.set(sigX, 0, sigZ);
        signalGroup.rotation.y = isForward ? -Math.PI / 2 : Math.PI / 2;
      }

      group.add(signalGroup);
    }
  }
}

export function isPillarColliding(
  worldPos: Vec2,
  pillarTopElev: number,
  currentPieceId: string,
  currentLayout?: Layout,
): boolean {
  if (!currentLayout) return false;

  const layerMap = new Map<string, Layer>();
  if (currentLayout.layers) {
    for (const lay of currentLayout.layers) layerMap.set(lay.id, lay);
  }

  // 1. Kollision mit anderen Gleisen und Straßen (layout.pieces)
  for (const otherPiece of currentLayout.pieces) {
    if (otherPiece.id === currentPieceId) continue;

    const isConnected = otherPiece.connectors?.some((c) => c.connectedTo?.pieceId === currentPieceId);
    if (isConnected) continue;

    const layer = otherPiece.layerId ? layerMap.get(otherPiece.layerId) : undefined;
    const baseElevation = layer?.heightMm ?? 0;
    const otherStartElev = baseElevation + (otherPiece.elevation?.start ?? 0);
    const otherEndElev = baseElevation + (otherPiece.elevation?.end ?? (otherPiece.elevation?.start ?? 0));
    const otherMinElev = Math.min(otherStartElev, otherEndElev);

    if (otherMinElev > pillarTopElev + 5) continue;

    let otherEntry: CatalogEntry | undefined = catalogRegistry.findEntry(otherPiece.catalogId);
    if (!otherEntry) {
      const isRoad = otherPiece.catalogId.startsWith("ST-");
      otherEntry = {
        id: otherPiece.catalogId,
        name: otherPiece.catalogId,
        kind: isRoad && otherPiece.catalogId.includes("-C") ? "curve" : "straight",
        category: isRoad ? "Straßen" : "Gleise",
        description: "",
        length: 188.3,
        isRoad,
        roadWidth: isRoad ? 60 : undefined,
      };
    }

    const dist = distanceToTrackPiece(worldPos, otherPiece.transform, otherEntry);
    const isRoad = otherEntry.isRoad === true || otherPiece.catalogId.startsWith("ST-");
    const clearance = isRoad ? ((otherEntry.roadWidth ?? 60) / 2 + 15) : 34;

    if (dist < clearance) {
      return true;
    }
  }

  // 2. Kollision mit Gebäuden und Modellen (layout.models)
  if (currentLayout.models) {
    for (const model of currentLayout.models) {
      if (model.builtinId === "mountain") {
        const localP = untransformPoint(worldPos, model.transform);
        const scale = model.scale ?? 1;
        const halfW = (model.width * scale) / 2;
        const halfD = (model.depth * scale) / 2;
        if (halfW > 0 && halfD > 0) {
          const normR = Math.hypot(localP.x / halfW, localP.y / halfD);
          if (normR <= 0.95) {
            return true;
          }
        }
        continue;
      }

      const layer = model.layerId ? layerMap.get(model.layerId) : undefined;
      const modelBase = layer?.heightMm ?? 0;
      const modelElev = modelBase + (model.elevation ?? 0);

      if (modelElev > pillarTopElev) continue;

      const localP = untransformPoint(worldPos, model.transform);
      const scale = model.scale ?? 1;
      const halfW = (model.width * scale) / 2 + 11;
      const halfD = (model.depth * scale) / 2 + 11;

      if (Math.abs(localP.x) <= halfW && Math.abs(localP.y) <= halfD) {
        return true;
      }
    }
  }

  return false;
}

export function buildTrackBridge3D(
  group: THREE.Group,
  entry: CatalogEntry,
  piece: TrackPiece,
  startElev: number,
  endElev: number,
  currentLayout?: Layout,
): void {
  const l = entry.length ?? (entry.radius ? (entry.radius * deg2rad(Math.abs(entry.angle ?? 30))) : 188.3);
  const gauge = entry.gauge ?? 16.5;
  const bridgeW = Math.max(34, gauge + 18);
  const bridgeH = 3.5;
  const heightDiff = endElev - startElev;
  const steelMat = SHARED_BRIDGE_STEEL_MAT;
  const deckMat = SHARED_BRIDGE_DECK_MAT;
  const railMat = SHARED_BRIDGE_RAIL_MAT;
  const pillarMat = SHARED_BRIDGE_PILLAR_MAT;

  const bridgeGroup = new THREE.Group();

  const tryAddStraightPillar = (baseU: number) => {
    const uOptions = [baseU, baseU - 0.08, baseU + 0.08].filter((u) => u >= 0.08 && u <= 0.92);
    for (const testU of uOptions) {
      const localX = l * testU;
      const localZ = 0;
      const elevAtU = startElev + testU * heightDiff;
      const pillarTopElev = elevAtU - bridgeH;
      if (pillarTopElev < 12) return;

      const worldPos = transformPoint({ x: localX, y: localZ }, piece.transform);
      if (!isPillarColliding(worldPos, pillarTopElev, piece.id, currentLayout)) {
        const pillarH = pillarTopElev;
        const pillarGeo = new THREE.CylinderGeometry(8, 11, pillarH, 14);
        const p = new THREE.Mesh(pillarGeo, pillarMat);
        const localMidY = (testU * heightDiff - bridgeH - startElev) / 2;
        p.position.set(localX, localMidY, localZ);
        p.castShadow = true;
        p.receiveShadow = true;
        bridgeGroup.add(p);
        return;
      }
    }
  };

  const tryAddCurvedPillar = (baseU: number, r: number, rad: number, dir: 1 | -1) => {
    const uOptions = [baseU, baseU - 0.08, baseU + 0.08].filter((u) => u >= 0.08 && u <= 0.92);
    for (const testU of uOptions) {
      const phi = testU * rad;
      const px = r * Math.sin(phi);
      const pz = dir * r * (1 - Math.cos(phi));
      const elevAtU = startElev + testU * heightDiff;
      const pillarTopElev = elevAtU - bridgeH;
      if (pillarTopElev < 12) return;

      const worldPos = transformPoint({ x: px, y: pz }, piece.transform);
      if (!isPillarColliding(worldPos, pillarTopElev, piece.id, currentLayout)) {
        const pillarH = pillarTopElev;
        const pillarGeo = new THREE.CylinderGeometry(8, 11, pillarH, 14);
        const p = new THREE.Mesh(pillarGeo, pillarMat);
        const localMidY = (testU * heightDiff - bridgeH - startElev) / 2;
        p.position.set(px, localMidY, pz);
        p.castShadow = true;
        p.receiveShadow = true;
        bridgeGroup.add(p);
        return;
      }
    }
  };

  if (isStraightTrackEntry(entry)) {
    const straightDeckGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      straightDeckGroup.rotation.z = Math.atan2(heightDiff, l);
    }

    const deckGeo = new THREE.BoxGeometry(l, bridgeH, bridgeW);
    const deck = new THREE.Mesh(deckGeo, deckMat);
    deck.position.set(l / 2, -bridgeH / 2, 0);
    deck.castShadow = true;
    deck.receiveShadow = true;
    straightDeckGroup.add(deck);

    for (const zSide of [-bridgeW / 2 + 1, bridgeW / 2 - 1]) {
      const girderGeo = new THREE.BoxGeometry(l, 7, 2.2);
      const girder = new THREE.Mesh(girderGeo, steelMat);
      girder.position.set(l / 2, 2.5, zSide);
      girder.castShadow = true;
      straightDeckGroup.add(girder);

      const railBar = new THREE.Mesh(new THREE.BoxGeometry(l, 1.2, 1.2), railMat);
      railBar.position.set(l / 2, 7, zSide);
      straightDeckGroup.add(railBar);

      const step = 20;
      for (let x = 6; x <= l - 6; x += step) {
        const post = new THREE.Mesh(new THREE.BoxGeometry(1.2, 7, 1.2), steelMat);
        post.position.set(x, 3.5, zSide);
        straightDeckGroup.add(post);
      }
    }
    bridgeGroup.add(straightDeckGroup);

    if (l < 130) {
      tryAddStraightPillar(0.5);
    } else if (l < 260) {
      tryAddStraightPillar(0.25);
      tryAddStraightPillar(0.75);
    } else {
      const count = Math.floor(l / 110);
      for (let i = 1; i <= count; i++) {
        tryAddStraightPillar(i / (count + 1));
      }
    }
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const angleVal = entry.angle ?? 30;
    const deg = Math.abs(angleVal);
    const dir: 1 | -1 = angleVal < 0 ? -1 : 1;
    const rad = deg2rad(deg);
    const innerR = Math.max(10, r - bridgeW / 2);
    const outerR = r + bridgeW / 2;
    const segments = Math.max(12, Math.round(deg / 2));

    const curveShape = new THREE.Shape();
    if (dir === 1) {
      curveShape.absarc(0, 0, outerR, -Math.PI / 2, -Math.PI / 2 + rad, false);
      curveShape.absarc(0, 0, innerR, -Math.PI / 2 + rad, -Math.PI / 2, true);
    } else {
      curveShape.absarc(0, 0, outerR, Math.PI / 2, Math.PI / 2 - rad, true);
      curveShape.absarc(0, 0, innerR, Math.PI / 2 - rad, Math.PI / 2, false);
    }
    curveShape.closePath();

    const deckGeo = new THREE.ExtrudeGeometry(curveShape, { depth: bridgeH, bevelEnabled: false, curveSegments: segments });
    deckGeo.rotateX(Math.PI / 2);
    deckGeo.translate(0, 0, dir * r);
    if (Math.abs(heightDiff) > 0.01) {
      applyCurveElevation(deckGeo, r, rad, heightDiff, dir, 0);
    }
    const deck = new THREE.Mesh(deckGeo, deckMat);
    deck.position.set(0, 0, 0);
    deck.castShadow = true;
    deck.receiveShadow = true;
    bridgeGroup.add(deck);

    for (const rSide of [innerR + 1, outerR - 1]) {
      const girderShape = new THREE.Shape();
      const gIn = rSide - 1.1;
      const gOut = rSide + 1.1;
      if (dir === 1) {
        girderShape.absarc(0, 0, gOut, -Math.PI / 2, -Math.PI / 2 + rad, false);
        girderShape.absarc(0, 0, gIn, -Math.PI / 2 + rad, -Math.PI / 2, true);
      } else {
        girderShape.absarc(0, 0, gOut, Math.PI / 2, Math.PI / 2 - rad, true);
        girderShape.absarc(0, 0, gIn, Math.PI / 2 - rad, Math.PI / 2, false);
      }
      girderShape.closePath();

      const girderGeo = new THREE.ExtrudeGeometry(girderShape, { depth: 7, bevelEnabled: false, curveSegments: segments });
      girderGeo.rotateX(Math.PI / 2);
      girderGeo.translate(0, 6, dir * r);
      if (Math.abs(heightDiff) > 0.01) {
        applyCurveElevation(girderGeo, r, rad, heightDiff, dir, 0);
      }
      const girderMesh = new THREE.Mesh(girderGeo, steelMat);
      girderMesh.castShadow = true;
      bridgeGroup.add(girderMesh);

      const railShape = new THREE.Shape();
      const rIn = rSide - 0.6;
      const rOut = rSide + 0.6;
      if (dir === 1) {
        railShape.absarc(0, 0, rOut, -Math.PI / 2, -Math.PI / 2 + rad, false);
        railShape.absarc(0, 0, rIn, -Math.PI / 2 + rad, -Math.PI / 2, true);
      } else {
        railShape.absarc(0, 0, rOut, Math.PI / 2, Math.PI / 2 - rad, true);
        railShape.absarc(0, 0, rIn, Math.PI / 2 - rad, Math.PI / 2, false);
      }
      railShape.closePath();

      const railGeo = new THREE.ExtrudeGeometry(railShape, { depth: 1.2, bevelEnabled: false, curveSegments: segments });
      railGeo.rotateX(Math.PI / 2);
      railGeo.translate(0, 7.2, dir * r);
      if (Math.abs(heightDiff) > 0.01) {
        applyCurveElevation(railGeo, r, rad, heightDiff, dir, 0);
      }
      const railMesh = new THREE.Mesh(railGeo, railMat);
      bridgeGroup.add(railMesh);

      const sideArcLen = rSide * rad;
      const postStep = 20;
      const postCount = Math.max(2, Math.floor(sideArcLen / postStep));
      const postGeo = new THREE.BoxGeometry(1.2, 7, 1.2);
      for (let i = 0; i <= postCount; i++) {
        const u = i / postCount;
        const phi = u * rad;
        const px = rSide * Math.sin(phi);
        const pz = dir === 1 ? r - rSide * Math.cos(phi) : -r + rSide * Math.cos(phi);
        const py = 2.5 + u * heightDiff;
        const postMesh = new THREE.Mesh(postGeo, steelMat);
        postMesh.position.set(px, py, pz);
        postMesh.rotation.y = -dir * phi;
        postMesh.castShadow = true;
        bridgeGroup.add(postMesh);
      }
    }

    const arcLen = r * rad;
    if (arcLen < 130) {
      tryAddCurvedPillar(0.5, r, rad, dir);
    } else if (arcLen < 260) {
      tryAddCurvedPillar(0.25, r, rad, dir);
      tryAddCurvedPillar(0.75, r, rad, dir);
    } else {
      const count = Math.floor(arcLen / 110);
      for (let i = 1; i <= count; i++) {
        tryAddCurvedPillar(i / (count + 1), r, rad, dir);
      }
    }
  } else {
    const straightDeckGroup = new THREE.Group();
    if (Math.abs(heightDiff) > 0.01) {
      straightDeckGroup.rotation.z = Math.atan2(heightDiff, l);
    }
    const deckGeo = new THREE.BoxGeometry(l, bridgeH, bridgeW);
    const deck = new THREE.Mesh(deckGeo, deckMat);
    deck.position.set(l / 2, -bridgeH / 2, 0);
    deck.castShadow = true;
    deck.receiveShadow = true;
    straightDeckGroup.add(deck);

    for (const zSide of [-bridgeW / 2 + 1, bridgeW / 2 - 1]) {
      const girderGeo = new THREE.BoxGeometry(l, 7, 2.2);
      const girder = new THREE.Mesh(girderGeo, steelMat);
      girder.position.set(l / 2, 2.5, zSide);
      girder.castShadow = true;
      straightDeckGroup.add(girder);

      const railBar = new THREE.Mesh(new THREE.BoxGeometry(l, 1.2, 1.2), railMat);
      railBar.position.set(l / 2, 7, zSide);
      straightDeckGroup.add(railBar);
    }
    bridgeGroup.add(straightDeckGroup);
    tryAddStraightPillar(0.5);
  }

  group.add(bridgeGroup);
}
