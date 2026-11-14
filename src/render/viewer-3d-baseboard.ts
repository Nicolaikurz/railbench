import * as THREE from "three";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import type { BaseboardConfig, Layer, LayerPlate, LayerWoodPlateMode, Layout } from "@/core/types";
import {
  computeBaseboardLegPositions,
  getBaseboardBounds,
  getBaseboardPolygon,
  getBaseboards,
  getLayerPlatePolygon,
  isPointInsideBaseboard,
} from "@/core/layout";
import { computeTableTrackCutouts, isPointInsideCutout } from "@/core/table-cutout";
import { transformPoint } from "@/core/geometry";

export function getStyleColor(style: BaseboardConfig["style"]): number {
  switch (style) {
    case "grass": return 0x4d7c0f;
    case "dark-wood": return 0x4a2e1b;
    case "plywood": return 0xecdcb9;
    case "slate": return 0x27272a;
    case "white": return 0xf8fafc;
    case "wood":
    default: return 0xd4a373;
  }
}

export function buildBaseboard3D(
  tableGroup: THREE.Group,
  config: BaseboardConfig,
  layout?: Layout,
  catalog?: Catalog,
  tablePlateMode: LayerWoodPlateMode = "table",
  plateThickness?: number,
) {
  const polygon = getBaseboardPolygon(config);
  if (polygon.length < 3) return;

  // 1. Zargenrahmen / Unterbau um den Umfang des Tisches
  const frameH = 45;
  const frameThick = 20;
  const frameMat = new THREE.MeshStandardMaterial({
    color: 0x8c6d48,
    roughness: 0.78,
    metalness: 0.05,
  });

  for (let i = 0; i < polygon.length; i++) {
    const p1 = polygon[i]!;
    const p2 = polygon[(i + 1) % polygon.length]!;
    const dx = p2.x - p1.x;
    const dy = p2.y - p1.y;
    const len = Math.hypot(dx, dy);
    if (len < 2) continue;

    const frameGeo = new THREE.BoxGeometry(len, frameH, frameThick);
    const frameMesh = new THREE.Mesh(frameGeo, frameMat);
    frameMesh.position.set((p1.x + p2.x) / 2, -frameH / 2, (p1.y + p2.y) / 2);
    frameMesh.rotation.y = -Math.atan2(dy, dx);
    frameMesh.castShadow = true;
    frameMesh.receiveShadow = true;
    tableGroup.add(frameMesh);
  }

  // 2. Tischbeine an markanten Eckpunkten nach unten
  const legRadius = 18;
  const legGeo = new THREE.CylinderGeometry(legRadius, legRadius, 720, 16);
  const legMat = new THREE.MeshStandardMaterial({ color: 0x27272a, roughness: 0.4, metalness: 0.7 });

  const trackCutouts = layout && catalog ? computeTableTrackCutouts(layout, catalog, config) : [];
  const legPositions = computeBaseboardLegPositions(config, legRadius, 45);
  for (const pt of legPositions) {
    if (trackCutouts.length > 0 && isPointInsideCutout(pt, trackCutouts, legRadius + 15)) {
      continue;
    }
    const leg = new THREE.Mesh(legGeo, legMat);
    leg.position.set(pt.x, -360 - 35 - 2.5, pt.y);
    leg.castShadow = true;
    leg.receiveShadow = true;
    tableGroup.add(leg);
  }

  // 3. Tischplatte (nur bei tablePlateMode === "table")
  if (tablePlateMode !== "table") {
    return;
  }

  let outerPts = polygon.map((p) => new THREE.Vector2(p.x, p.y));
  if (THREE.ShapeUtils.isClockWise(outerPts)) {
    outerPts = outerPts.reverse();
  }

  const shape = new THREE.Shape();
  shape.moveTo(outerPts[0]!.x, outerPts[0]!.y);
  for (let i = 1; i < outerPts.length; i++) {
    shape.lineTo(outerPts[i]!.x, outerPts[i]!.y);
  }
  shape.closePath();

  // Vertiefungen / Ausschnitte für Drehscheiben und Schiebebühnen
  if (layout?.pieces) {
    const entryMap = new Map<string, CatalogEntry>();
    if (catalog?.entries) {
      for (const e of catalog.entries) entryMap.set(e.id, e);
    }

    for (const piece of layout.pieces) {
      const pieceLayer = layout.layers?.find((l) => l.id === piece.layerId);
      const baseElev = pieceLayer?.heightMm ?? 0;
      const startElev = baseElev + (piece.elevation?.start ?? 0);
      const endElev = baseElev + (piece.elevation?.end ?? 0);
      if (Math.abs(startElev) > 2 || Math.abs(endElev) > 2) continue;

      const entry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId);
      if (!entry) continue;

      if (entry.kind === "turntable") {
        const pt = { x: piece.transform.x, y: piece.transform.y };
        if (!isPointInsideBaseboard(pt, config)) continue;

        const cutoutR = 146;
        const holePath = new THREE.Path();
        holePath.absarc(pt.x, pt.y, cutoutR, 0, Math.PI * 2, true);
        shape.holes.push(holePath);
      } else if (entry.kind === "transfer-table") {
        const l = entry.length ?? 360;
        const center = transformPoint({ x: l / 2, y: 154.8 }, piece.transform);
        if (!isPointInsideBaseboard(center, config)) continue;

        const pLeft = 34;
        const pRight = l - 34;
        const pTop = -51.2;
        const pBottom = 360.8;

        const corners = [
          transformPoint({ x: pLeft, y: pTop }, piece.transform),
          transformPoint({ x: pRight, y: pTop }, piece.transform),
          transformPoint({ x: pRight, y: pBottom }, piece.transform),
          transformPoint({ x: pLeft, y: pBottom }, piece.transform),
        ];

        let holeVecs = corners.map((c) => new THREE.Vector2(c.x, c.y));
        if (!THREE.ShapeUtils.isClockWise(holeVecs)) {
          holeVecs = holeVecs.reverse();
        }

        const holePath = new THREE.Path();
        holePath.moveTo(holeVecs[0]!.x, holeVecs[0]!.y);
        for (let i = 1; i < holeVecs.length; i++) {
          holePath.lineTo(holeVecs[i]!.x, holeVecs[i]!.y);
        }
        holePath.closePath();
        shape.holes.push(holePath);
      }
    }
  }

  // Tischdurchbrüche für Rampen und Strecken unter der Platte
  for (const cutout of trackCutouts) {
    if (cutout.points.length < 3) continue;
    let holeVecs = cutout.points.map((p) => new THREE.Vector2(p.x, p.y));
    if (!THREE.ShapeUtils.isClockWise(holeVecs)) {
      holeVecs = holeVecs.reverse();
    }

    const holePath = new THREE.Path();
    holePath.moveTo(holeVecs[0]!.x, holeVecs[0]!.y);
    for (let i = 1; i < holeVecs.length; i++) {
      holePath.lineTo(holeVecs[i]!.x, holeVecs[i]!.y);
    }
    holePath.closePath();
    shape.holes.push(holePath);
  }

  const bevelThickness = 2.0;
  const depth = plateThickness ?? 35;
  const extrudeSettings: THREE.ExtrudeGeometryOptions = {
    depth,
    bevelEnabled: true,
    bevelSegments: 2,
    steps: 1,
    bevelSize: 2.0,
    bevelThickness,
  };

  const geometry = new THREE.ExtrudeGeometry(shape, extrudeSettings);
  geometry.rotateX(Math.PI / 2);
  geometry.translate(0, -bevelThickness, 0);

  const woodColor = getStyleColor(config.style);
  const isGrass = config.style === "grass";
  const material = new THREE.MeshStandardMaterial({
    color: woodColor,
    roughness: isGrass ? 0.9 : 0.65,
    metalness: isGrass ? 0.02 : 0.05,
  });

  const tableMesh = new THREE.Mesh(geometry, material);
  tableMesh.receiveShadow = true;
  tableMesh.castShadow = true;
  tableGroup.add(tableMesh);
}

export function buildLayerDeckPlate3D(
  woodPlateGroup: THREE.Group,
  layer: Layer,
  config: BaseboardConfig,
  layout: Layout,
  catalog: Catalog,
) {
  const polygon = getBaseboardPolygon(config);
  if (polygon.length < 3) return;

  let outerPts = polygon.map((p) => new THREE.Vector2(p.x, p.y));
  if (THREE.ShapeUtils.isClockWise(outerPts)) {
    outerPts = outerPts.reverse();
  }

  const shape = new THREE.Shape();
  shape.moveTo(outerPts[0]!.x, outerPts[0]!.y);
  for (let i = 1; i < outerPts.length; i++) {
    shape.lineTo(outerPts[i]!.x, outerPts[i]!.y);
  }
  shape.closePath();

  const depth = layer.woodPlateThicknessMm ?? 12;
  const layerH = layer.heightMm ?? 0;

  const cutouts = computeTableTrackCutouts(layout, catalog, config, layerH, depth);
  for (const cutout of cutouts) {
    if (cutout.points.length < 3) continue;
    let holeVecs = cutout.points.map((p) => new THREE.Vector2(p.x, p.y));
    if (!THREE.ShapeUtils.isClockWise(holeVecs)) {
      holeVecs = holeVecs.reverse();
    }
    const holePath = new THREE.Path();
    holePath.moveTo(holeVecs[0]!.x, holeVecs[0]!.y);
    for (let i = 1; i < holeVecs.length; i++) {
      holePath.lineTo(holeVecs[i]!.x, holeVecs[i]!.y);
    }
    holePath.closePath();
    shape.holes.push(holePath);
  }

  const bevelThickness = 1.5;
  const extrudeSettings: THREE.ExtrudeGeometryOptions = {
    depth,
    bevelEnabled: true,
    bevelSegments: 2,
    steps: 1,
    bevelSize: 1.5,
    bevelThickness,
  };

  const geometry = new THREE.ExtrudeGeometry(shape, extrudeSettings);
  geometry.rotateX(Math.PI / 2);
  geometry.translate(0, layerH - bevelThickness, 0);

  const plateStyle = layer.woodPlateStyle ?? config.style ?? "wood";
  const woodColor = getStyleColor(plateStyle);
  const isGrass = plateStyle === "grass";
  const material = new THREE.MeshStandardMaterial({
    color: woodColor,
    roughness: isGrass ? 0.9 : 0.65,
    metalness: isGrass ? 0.02 : 0.05,
  });

  const deckMesh = new THREE.Mesh(geometry, material);
  deckMesh.receiveShadow = true;
  deckMesh.castShadow = true;
  woodPlateGroup.add(deckMesh);

  if (Math.abs(layerH) >= 20) {
    const riserRadius = 12;
    const riserPositions = computeBaseboardLegPositions(config, riserRadius, 55);
    const riserHeight = Math.abs(layerH);
    const riserGeo = new THREE.CylinderGeometry(riserRadius, riserRadius, riserHeight, 12);
    const riserMat = new THREE.MeshStandardMaterial({ color: 0x8c6d48, roughness: 0.8, metalness: 0.05 });

    for (const pt of riserPositions) {
      if (cutouts.length > 0 && isPointInsideCutout(pt, cutouts, riserRadius + 15)) continue;
      const riser = new THREE.Mesh(riserGeo, riserMat);
      const midY = layerH > 0 ? layerH / 2 - depth / 2 : layerH / 2;
      riser.position.set(pt.x, midY, pt.y);
      riser.castShadow = true;
      riser.receiveShadow = true;
      woodPlateGroup.add(riser);
    }
  }
}

export function buildCustomLayerPlate3D(
  woodPlateGroup: THREE.Group,
  layer: Layer,
  plate: LayerPlate,
  layout: Layout,
  catalog: Catalog,
) {
  const width = Math.max(50, plate.width);
  const depth = Math.max(50, plate.depth);
  const thickness = plate.thicknessMm ?? layer.woodPlateThicknessMm ?? 10;
  const layerH = layer.heightMm ?? 0;

  const baseboards = getBaseboards(layout);
  const targetBoard = (plate.baseboardId ? baseboards.find((b) => b.id === plate.baseboardId) : undefined) || baseboards[0];
  const tableBounds = targetBoard ? getBaseboardBounds(targetBoard) : { x: 0, y: 0 };
  const originX = tableBounds.x + (plate.x ?? 0);
  const originY = tableBounds.y + (plate.y ?? 0);
  const rot = targetBoard?.rotation ?? 0;

  const polygon = getLayerPlatePolygon(plate, targetBoard);

  let outerPts = polygon.map((p) => new THREE.Vector2(p.x, p.y));
  if (THREE.ShapeUtils.isClockWise(outerPts)) {
    outerPts = outerPts.reverse();
  }

  const shape = new THREE.Shape();
  shape.moveTo(outerPts[0]!.x, outerPts[0]!.y);
  for (let i = 1; i < outerPts.length; i++) {
    shape.lineTo(outerPts[i]!.x, outerPts[i]!.y);
  }
  shape.closePath();

  const plateStyle = plate.style ?? targetBoard?.style ?? "wood";

  const dummyConfig: BaseboardConfig = {
    enabled: true,
    width,
    depth,
    shape: plate.shape ?? "rect",
    customPolygon: plate.customPolygon,
    triangleType: plate.triangleType,
    lWingWidth: plate.lWingWidth,
    lWingDepth: plate.lWingDepth,
    lSide: plate.lSide,
    uLeftWingWidth: plate.uLeftWingWidth,
    uRightWingWidth: plate.uRightWingWidth,
    uBackDepth: plate.uBackDepth,
    tBarDepth: plate.tBarDepth,
    tStemWidth: plate.tStemWidth,
    positionMode: "custom",
    customX: originX,
    customY: originY,
    rotation: rot,
    style: plateStyle,
  };

  const cutouts = computeTableTrackCutouts(layout, catalog, dummyConfig, layerH, thickness);
  for (const cutout of cutouts) {
    if (cutout.points.length < 3) continue;
    let holeVecs = cutout.points.map((p) => new THREE.Vector2(p.x, p.y));
    if (!THREE.ShapeUtils.isClockWise(holeVecs)) {
      holeVecs = holeVecs.reverse();
    }
    const holePath = new THREE.Path();
    holePath.moveTo(holeVecs[0]!.x, holeVecs[0]!.y);
    for (let i = 1; i < holeVecs.length; i++) {
      holePath.lineTo(holeVecs[i]!.x, holeVecs[i]!.y);
    }
    holePath.closePath();
    shape.holes.push(holePath);
  }

  const bevelThickness = 1.5;
  const extrudeSettings: THREE.ExtrudeGeometryOptions = {
    depth: thickness,
    bevelEnabled: true,
    bevelSegments: 2,
    steps: 1,
    bevelSize: 1.5,
    bevelThickness,
  };

  const geometry = new THREE.ExtrudeGeometry(shape, extrudeSettings);
  geometry.rotateX(Math.PI / 2);
  geometry.translate(0, layerH - bevelThickness, 0);

  const woodColor = getStyleColor(plateStyle);
  const isGrass = plateStyle === "grass";
  const material = new THREE.MeshStandardMaterial({
    color: woodColor,
    roughness: isGrass ? 0.9 : 0.65,
    metalness: isGrass ? 0.02 : 0.05,
  });

  const deckMesh = new THREE.Mesh(geometry, material);
  deckMesh.receiveShadow = true;
  deckMesh.castShadow = true;
  woodPlateGroup.add(deckMesh);

  if (Math.abs(layerH) >= 20) {
    const riserRadius = 12;
    const riserPositions = computeBaseboardLegPositions(dummyConfig, riserRadius, 55);
    const riserHeight = Math.abs(layerH);
    const riserGeo = new THREE.CylinderGeometry(riserRadius, riserRadius, riserHeight, 12);
    const riserMat = new THREE.MeshStandardMaterial({ color: 0x8c6d48, roughness: 0.8, metalness: 0.05 });

    for (const pt of riserPositions) {
      if (cutouts.length > 0 && isPointInsideCutout(pt, cutouts, riserRadius + 15)) continue;
      const riser = new THREE.Mesh(riserGeo, riserMat);
      const midY = layerH > 0 ? layerH / 2 - thickness / 2 : layerH / 2;
      riser.position.set(pt.x, midY, pt.y);
      riser.castShadow = true;
      riser.receiveShadow = true;
      woodPlateGroup.add(riser);
    }
  }
}
