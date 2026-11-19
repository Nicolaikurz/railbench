import * as THREE from "three";
import type { CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import type { GroundTerrainType, Layout, Model3D, TrackPiece, Vec2 } from "@/core/types";
import { deg2rad, isCurvedTrackEntry, isStraightTrackEntry } from "@/core/geometry";
import { getBaseboardPolygon, getBaseboards, getLayerPlatePolygon } from "@/core/layout";
import { getEffectiveFlexEntry } from "@/core/flextrack";
import { evaluateMountainLocal, mountainNoise } from "./mountain-tunnel";

/**
 * Sammelt alle aktiven Tisch- bzw. Ebenenplatten-Polygone für eine Ebene,
 * damit Gelände (Berge, Wasser, Sand, Erde, Hügel) niemals über Tisch- oder Plattenränder hinausragt.
 */
export function collectTerrainSupportPolygons(layout?: Layout, layerId?: string): Vec2[][] {
  if (!layout) return [];
  const baseboards = getBaseboards(layout).filter((b) => b.enabled);
  const layer = layerId
    ? layout.layers?.find((l) => l.id === layerId)
    : layout.layers?.find((l) => l.id === layout.activeLayerId) ?? layout.layers?.[0];

  const polys: Vec2[][] = [];

  if (layer?.plates && layer.plates.length > 0) {
    for (const plate of layer.plates) {
      const targetBoard =
        (plate.baseboardId ? baseboards.find((b) => b.id === plate.baseboardId) : undefined) ?? baseboards[0];
      const poly = getLayerPlatePolygon(plate, targetBoard);
      if (poly.length >= 3) polys.push(poly);
    }
  }

  // Haupttisch-Polygone hinzufügen, wenn keine eigenen Platten definiert sind oder Ebene auf Tischhöhe liegt
  if (polys.length === 0 || (layer?.heightMm ?? 0) === 0 || layer?.woodPlateMode === "table") {
    for (const bb of baseboards) {
      const poly = getBaseboardPolygon(bb);
      if (poly.length >= 3) polys.push(poly);
    }
  }

  return polys;
}

export function isPointInAnyPolygon(x: number, y: number, polygons: Vec2[][]): boolean {
  for (const poly of polygons) {
    let inside = false;
    for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
      const xi = poly[i]!.x;
      const yi = poly[i]!.y;
      const xj = poly[j]!.x;
      const yj = poly[j]!.y;
      const intersect = yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi;
      if (intersect) inside = !inside;
    }
    if (inside) return true;
  }
  return false;
}

export interface GroundTerrainExtraData {
  layout?: Layout;
  entryMap?: Map<string, CatalogEntry>;
}

interface BuildingFootprint {
  wx: number;
  wz: number;
  cos: number;
  sin: number;
  halfW: number;
  halfD: number;
  elev: number;
}

interface TrackCorridor {
  minX: number;
  maxX: number;
  minZ: number;
  maxZ: number;
  halfW: number;
  pts: Array<{ x: number; z: number; elev: number }>;
  isTurntable?: boolean;
}

const SHARED_GROUND_TERRAIN_MAT = new THREE.MeshStandardMaterial({
  vertexColors: true,
  roughness: 0.9,
  metalness: 0.05,
  flatShading: true,
  polygonOffset: true,
  polygonOffsetFactor: -1,
  polygonOffsetUnits: -1,
});
SHARED_GROUND_TERRAIN_MAT.userData.shared = true;

const SHARED_WATER_SURFACE_MAT = new THREE.MeshStandardMaterial({
  color: 0x0ea5e9,
  roughness: 0.14,
  metalness: 0.22,
  transparent: true,
  opacity: 0.78,
  side: THREE.DoubleSide,
  polygonOffset: true,
  polygonOffsetFactor: -1,
  polygonOffsetUnits: -1,
});
SHARED_WATER_SURFACE_MAT.userData.shared = true;

const smoothstep = (a: number, b: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

function samplePieceCenterline(
  piece: TrackPiece,
  rawEntry: CatalogEntry,
  startElev: number,
  endElev: number,
): TrackCorridor {
  const entry = getEffectiveFlexEntry(rawEntry, piece);
  const heightDiff = endElev - startElev;
  const rotRad = deg2rad(piece.transform.rotation);
  const cos = Math.cos(rotRad);
  const sin = Math.sin(rotRad);
  const toWorld = (lx: number, ly: number, elev: number) => ({
    x: piece.transform.x + (lx * cos - ly * sin),
    z: piece.transform.y + (lx * sin + ly * cos),
    elev,
  });

  const pts: Array<{ x: number; z: number; elev: number }> = [];
  const kind = entry.kind;

  if (kind === "turntable") {
    const c = toWorld(0, 0, startElev);
    const r = (entry.radius ?? 189) + 12;
    return {
      minX: c.x - r,
      maxX: c.x + r,
      minZ: c.z - r,
      maxZ: c.z + r,
      halfW: r,
      pts: [c],
      isTurntable: true,
    };
  }

  const gauge = entry.gauge ?? 16.5;
  const halfW = entry.isRoad
    ? (entry.roadWidth ?? 60) / 2 + 8
    : Math.max(20, (gauge / 16.5) * 20) + 6;

  const pushStraight = (x1: number, y1: number, x2: number, y2: number) => {
    const len = Math.hypot(x2 - x1, y2 - y1);
    const steps = Math.max(2, Math.ceil(len / 24));
    for (let i = 0; i <= steps; i++) {
      const u = i / steps;
      pts.push(toWorld(x1 + (x2 - x1) * u, y1 + (y2 - y1) * u, startElev + heightDiff * u));
    }
  };

  const pushArc = (r: number, aDeg: number, dir: 1 | -1, leadX = 0) => {
    const rad = deg2rad(Math.abs(aDeg));
    const arcLen = r * rad;
    const steps = Math.max(3, Math.ceil(arcLen / 24));
    for (let i = 0; i <= steps; i++) {
      const u = i / steps;
      const th = rad * u;
      pts.push(toWorld(leadX + r * Math.sin(th), dir * r * (1 - Math.cos(th)), startElev + heightDiff * u));
    }
  };

  if (isStraightTrackEntry(entry)) {
    pushStraight(0, 0, entry.length ?? 188.3, 0);
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    pushArc(r, a, a < 0 ? -1 : 1, 0);
  } else if (kind === "turnout-left" || kind === "turnout-right") {
    const dir: 1 | -1 = kind === "turnout-left" ? -1 : 1;
    if (entry.radius && !entry.length) {
      pushArc(entry.radius, entry.angle ?? 30, dir, 0);
      pushArc(entry.branchRadius ?? entry.radius, entry.branchAngle ?? 30, dir, entry.branchLength ?? 0);
    } else {
      pushStraight(0, 0, entry.length ?? 188.3, 0);
      pushArc(entry.branchRadius ?? 437.5, entry.branchAngle ?? 24.3, dir, 0);
    }
  } else if (kind === "curved-turnout-left" || kind === "curved-turnout-right") {
    const dir: 1 | -1 = kind === "curved-turnout-left" ? -1 : 1;
    const r1 = entry.radius ?? 360;
    pushArc(r1, entry.angle ?? 30, dir, 0);
    pushArc(entry.branchRadius ?? r1, entry.branchAngle ?? 30, dir, entry.branchLength ?? 0);
  } else if (kind === "three-way") {
    pushStraight(0, 0, entry.length ?? 188.3, 0);
    pushArc(entry.branchRadius ?? 437.5, entry.branchAngle ?? 24.3, -1, 0);
    pushArc(entry.branchRadius ?? 437.5, entry.branchAngle ?? 24.3, 1, 0);
  } else if (kind === "crossing") {
    const h = (entry.length ?? 188.3) / 2;
    const rad = deg2rad(Math.abs(entry.angle ?? 24.3));
    pushStraight(-h, 0, h, 0);
    pushStraight(-h * Math.cos(rad), -h * Math.sin(rad), h * Math.cos(rad), h * Math.sin(rad));
  } else {
    pushStraight(0, 0, entry.length ?? 188.3, 0);
  }

  let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
  const pad = halfW + 48;
  for (const p of pts) {
    if (p.x - pad < minX) minX = p.x - pad;
    if (p.x + pad > maxX) maxX = p.x + pad;
    if (p.z - pad < minZ) minZ = p.z - pad;
    if (p.z + pad > maxZ) maxZ = p.z + pad;
  }

  return { minX, maxX, minZ, maxZ, halfW, pts };
}

function pushGroundColor(
  colors: number[],
  type: GroundTerrainType,
  hRatio: number,
  noiseVal: number,
  nearFeature: number,
): void {
  const n = noiseVal * 0.04;
  if (type === "sand") {
    // Warmer Sand / Uferkies
    const r = Math.min(1, Math.max(0, 0.84 + hRatio * 0.06 + n - nearFeature * 0.04));
    const g = Math.min(1, Math.max(0, 0.74 + hRatio * 0.05 + n - nearFeature * 0.04));
    const b = Math.min(1, Math.max(0, 0.51 + hRatio * 0.03 + n));
    colors.push(r, g, b);
  } else if (type === "dirt") {
    // Natürliche Erde / Lehmboden
    const r = Math.min(1, Math.max(0, 0.44 + hRatio * 0.08 + n));
    const g = Math.min(1, Math.max(0, 0.32 + hRatio * 0.06 + n));
    const b = Math.min(1, Math.max(0, 0.21 + hRatio * 0.04 + n));
    colors.push(r, g, b);
  } else if (type === "water") {
    // Sandiges Ufer & dunklerer Gewässerboden
    if (nearFeature > 0.35 || hRatio > 0.25) {
      colors.push(0.74 + n, 0.66 + n, 0.48 + n);
    } else {
      colors.push(0.14 + n * 0.5, 0.32 + n * 0.5, 0.42 + n * 0.5);
    }
  } else {
    // "hill": Saftige grüne Hügel-Wiese, leicht heller auf Kuppen
    const r = Math.min(1, Math.max(0, 0.24 + hRatio * 0.1 + n));
    const g = Math.min(1, Math.max(0, 0.42 + hRatio * 0.12 + n));
    const b = Math.min(1, Math.max(0, 0.14 + hRatio * 0.04 + n));
    colors.push(r, g, b);
  }
}

/**
 * Erzeugt ein adaptives Untergrund-Gelände (Wasser, Sand, Erde oder Hügel),
 * das sich automatisch an Gebäude, Gleise und Straßen anpasst:
 * - Unter Gebäuden, Gleisen und Straßen bleibt der Boden exakt flach (auf deren Höhe).
 * - Rund um Gebäude und Strecken hebt sich das Gelände sanft an (bzw. bildet Uferböschungen bei Wasser).
 */
export function buildGroundTerrain3D(
  group: THREE.Group,
  model?: Model3D,
  extraData?: GroundTerrainExtraData,
): void {
  const width = model?.width ?? 700;
  const depth = model?.depth ?? 500;
  const type: GroundTerrainType = model?.groundTerrainType ?? "hill";
  const defaultH = type === "hill" ? 26 : type === "dirt" ? 14 : type === "sand" ? 12 : 6;
  const maxHeight = Math.max(2, model?.height ?? defaultH);
  const layout = extraData?.layout;
  const entryMap = extraData?.entryMap;

  const modelX = model?.transform.x ?? 0;
  const modelZ = model?.transform.y ?? 0;
  const modelRotRad = deg2rad(model?.transform.rotation ?? 0);
  const cosM = Math.cos(modelRotRad);
  const sinM = Math.sin(modelRotRad);
  const baseLayerElev =
    (layout?.layers?.find((l) => l.id === model?.layerId)?.heightMm ?? 0) + (model?.elevation ?? 0);

  // 1. Gebäude-Footprints im Umfeld einsammeln (alle Nicht-Gelände-Modelle)
  const buildings: BuildingFootprint[] = [];
  if (layout?.models) {
    for (const other of layout.models) {
      if (other.id === model?.id) continue;
      if (other.builtinId === "mountain" || other.builtinId === "ground-terrain") continue;
      const layer = layout.layers?.find((l) => l.id === other.layerId);
      if (layer && !layer.visible) continue;
      const elev = (layer?.heightMm ?? 0) + (other.elevation ?? 0);
      const scale = other.scale ?? 1;
      const bRot = deg2rad(other.transform.rotation);
      buildings.push({
        wx: other.transform.x,
        wz: other.transform.y,
        cos: Math.cos(-bRot),
        sin: Math.sin(-bRot),
        halfW: (other.width * scale) / 2 + 10,
        halfD: (other.depth * scale) / 2 + 10,
        elev,
      });
    }
  }

  // 2. Gleis- und Straßenkorridore einsammeln
  const corridors: TrackCorridor[] = [];
  if (layout?.pieces) {
    for (const piece of layout.pieces) {
      const layer = layout.layers?.find((l) => l.id === piece.layerId);
      if (layer && !layer.visible) continue;
      const entry = entryMap?.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId);
      if (!entry) continue;
      const layerH = layer?.heightMm ?? 0;
      const sElev = layerH + (piece.elevation?.start ?? 0);
      const eElev = layerH + (piece.elevation?.end ?? 0);
      corridors.push(samplePieceCenterline(piece, entry, sElev, eElev));
    }
  }

  const segsX = 48;
  const segsZ = 48;
  const positions: number[] = [];
  const colors: number[] = [];
  const indices: number[] = [];
  const waterMask: boolean[] = [];
  const insideSupport: boolean[] = [];

  const supportPolys = collectTerrainSupportPolygons(layout, model?.layerId);
  const clipToSupport = supportPolys.length > 0;
  const transitionDist = 48; // Sanfte Böschungsbreite um Gebäude, Gleise und Straßen

  const hasBrushPath = !!(model?.mountainPath && model.mountainPath.length > 0);

  for (let iz = 0; iz <= segsZ; iz++) {
    const lz = (iz / segsZ - 0.5) * depth;
    const nz = lz / (depth * 0.5);

    for (let ix = 0; ix <= segsX; ix++) {
      const lx = (ix / segsX - 0.5) * width;
      const nx = lx / (width * 0.5);

      // Weltkoordinaten dieses Gitterpunkts
      const wx = modelX + (lx * cosM - lz * sinM);
      const wz = modelZ + (lx * sinM + lz * cosM);

      const inBounds = !clipToSupport || isPointInAnyPolygon(wx, wz, supportPolys);
      insideSupport.push(inBounds);

      if (!inBounds) {
        positions.push(lx, 0, lz);
        waterMask.push(false);
        pushGroundColor(colors, type, 0, 0, 1);
        continue;
      }

      // Organische Randabschwächung: entlang des Pinselpfads oder elliptisch
      const rDist = hasBrushPath ? evaluateMountainLocal(lx, lz, model).normR : Math.hypot(nx, nz);
      const edgeFade = 1 - smoothstep(0.68, 1.0, rDist);

      // Natürliche sanfte Bodenwellen
      const nVal = mountainNoise(nx * 1.4, nz * 1.4);
      const wave = 0.55 + 0.45 * Math.sin(nx * 2.4 + 0.7) * Math.cos(nz * 2.1 - 0.4) + nVal * 0.25;
      let h = type === "water" ? edgeFade * 2.2 : Math.max(0, edgeFade * maxHeight * Math.max(0.25, wave));

      // Anpassung an Gebäude, Gleise & Straßen:
      // flattenWeight = 1 direkt unter Gebäude/Gleis/Straße -> exakt flach (y = 0 relativ zur Ebene)
      // flattenWeight = 0 weiter weg -> volle Gelände-/Hügelhöhe
      let flattenWeight = 0;
      let insideTurntable = false;

      for (const b of buildings) {
        const dx = wx - b.wx;
        const dz = wz - b.wz;
        // Schneller Grobtest
        const maxR = Math.max(b.halfW, b.halfD) + transitionDist;
        if (Math.abs(dx) > maxR || Math.abs(dz) > maxR) continue;
        // Lokale Gebäude-Box-Distanz
        const blx = dx * b.cos - dz * b.sin;
        const blz = dx * b.sin + dz * b.cos;
        const qx = Math.max(0, Math.abs(blx) - b.halfW);
        const qz = Math.max(0, Math.abs(blz) - b.halfD);
        const dist = Math.hypot(qx, qz);
        if (dist < transitionDist) {
          const w = 1 - smoothstep(0, transitionDist, dist);
          if (w > flattenWeight) flattenWeight = w;
        }
      }

      for (const cor of corridors) {
        if (wx < cor.minX || wx > cor.maxX || wz < cor.minZ || wz > cor.maxZ) continue;
        for (const pt of cor.pts) {
          if (Math.abs(pt.elev - baseLayerElev) > 45) continue;
          const d = Math.hypot(wx - pt.x, wz - pt.z);
          if (cor.isTurntable && d < cor.halfW) {
            insideTurntable = true;
            flattenWeight = 1;
            break;
          }
          if (d < cor.halfW + transitionDist) {
            const w = 1 - smoothstep(cor.halfW, cor.halfW + transitionDist, d);
            if (w > flattenWeight) flattenWeight = w;
          }
        }
        if (insideTurntable) break;
      }

      if (insideTurntable) {
        h = 0;
        waterMask.push(false);
      } else if (type === "water") {
        // Wasser: Sanfte Uferböschung um Gebäude/Gleise, freies Wasser dazwischen
        const isWaterVert = edgeFade > 0.15 && flattenWeight < 0.45;
        waterMask.push(isWaterVert);
        h = isWaterVert ? 0.35 : edgeFade * (1 - flattenWeight * 0.85) * Math.min(maxHeight, 5);
      } else {
        // Hügel / Sand / Erde: Unter Gebäuden, Gleisen & Straßen flach (0), drumherum sanft erhöht
        h = h * (1 - flattenWeight);
        waterMask.push(false);
      }

      positions.push(lx, h, lz);
      const hRatio = maxHeight > 0 ? Math.min(1, h / maxHeight) : 0;
      pushGroundColor(colors, type, hRatio, nVal, flattenWeight);
    }
  }

  for (let iz = 0; iz < segsZ; iz++) {
    for (let ix = 0; ix < segsX; ix++) {
      const i0 = iz * (segsX + 1) + ix;
      const i1 = i0 + 1;
      const i2 = (iz + 1) * (segsX + 1) + ix;
      const i3 = i2 + 1;

      const y0 = positions[i0 * 3 + 1]!;
      const y1 = positions[i1 * 3 + 1]!;
      const y2 = positions[i2 * 3 + 1]!;
      const y3 = positions[i3 * 3 + 1]!;

      // Flache Randbereiche außerhalb der Geländeform auslassen (keine unnötige Rechteckplatte)
      if (y0 <= 0.08 && y1 <= 0.08 && y2 <= 0.08 && y3 <= 0.08) {
        continue;
      }

      // Nur Dreiecke innerhalb der Tisch-/Plattenfläche ausgeben (kein Überhang über den Rand)
      if (
        (y0 > 0.08 || y2 > 0.08 || y1 > 0.08) &&
        insideSupport[i0] &&
        insideSupport[i2] &&
        insideSupport[i1]
      ) {
        indices.push(i0, i2, i1);
      }
      if (
        (y1 > 0.08 || y2 > 0.08 || y3 > 0.08) &&
        insideSupport[i1] &&
        insideSupport[i2] &&
        insideSupport[i3]
      ) {
        indices.push(i1, i2, i3);
      }
    }
  }

  const terrainGroup = new THREE.Group();
  terrainGroup.position.y = 0.25;

  if (indices.length > 0) {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
    geo.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
    geo.setIndex(indices);
    geo.computeVertexNormals();

    const mesh = new THREE.Mesh(geo, SHARED_GROUND_TERRAIN_MAT);
    mesh.receiveShadow = true;
    mesh.castShadow = type === "hill";
    terrainGroup.add(mesh);
  }

  // Bei Typ "water": Glänzende, halbtransparente Wasseroberfläche (spart Gebäude, Gleise & Straßen aus)
  if (type === "water") {
    const wPositions: number[] = [];
    const wIndices: number[] = [];
    const waterY = 1.6;
    for (let iz = 0; iz <= segsZ; iz++) {
      const lz = (iz / segsZ - 0.5) * depth;
      for (let ix = 0; ix <= segsX; ix++) {
        const lx = (ix / segsX - 0.5) * width;
        wPositions.push(lx, waterY, lz);
      }
    }
    for (let iz = 0; iz < segsZ; iz++) {
      for (let ix = 0; ix < segsX; ix++) {
        const i0 = iz * (segsX + 1) + ix;
        const i1 = i0 + 1;
        const i2 = (iz + 1) * (segsX + 1) + ix;
        const i3 = i2 + 1;
        if (waterMask[i0] && waterMask[i1] && waterMask[i2]) {
          wIndices.push(i0, i2, i1);
        }
        if (waterMask[i1] && waterMask[i2] && waterMask[i3]) {
          wIndices.push(i1, i2, i3);
        }
      }
    }
    if (wIndices.length > 0) {
      const wGeo = new THREE.BufferGeometry();
      wGeo.setAttribute("position", new THREE.Float32BufferAttribute(wPositions, 3));
      wGeo.setIndex(wIndices);
      wGeo.computeVertexNormals();
      const waterMesh = new THREE.Mesh(wGeo, SHARED_WATER_SURFACE_MAT);
      waterMesh.receiveShadow = true;
      terrainGroup.add(waterMesh);
    }
  }

  group.add(terrainGroup);
}
