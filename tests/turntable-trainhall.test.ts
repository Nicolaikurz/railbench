import { describe, it, expect } from "vitest";
import { BUILTIN_MODELS, findBuiltinModelDef } from "@/core/models-3d";
import { createBuiltinModel3DMesh } from "@/render/builtin-models-3d";
import {
  buildStation3D,
  buildSignalBox3D,
  buildEngineShed3D,
  buildTownHouse3D,
  buildWaterTower3D,
  buildLightSignal3D,
  buildBufferStop3D,
  buildPineTree3D,
  buildDeciduousTree3D,
  buildPlatformLamp3D,
  buildGenericBuilding3D,
  buildTurntableTrainhall3D,
  buildSkyscraper3D,
  buildIndustryBuilding3D,
} from "@/render/models-3d";
import { snapModelToTurntable } from "@/core/turntable-snap";
import { distanceToTrackPiece, getPieceRotationHandleWorld, isPieceInSelectionBox } from "@/core/geometry";
import type { TrackPiece, Transform } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import * as THREE from "three";

describe("Turntable Trainhall & Modular 3D Models", () => {
  it("includes turntable-trainhall in BUILTIN_MODELS", () => {
    const def = findBuiltinModelDef("turntable-trainhall");
    expect(def).toBeDefined();
    expect(def?.name).toContain("Ringlokschuppen");
    expect(def?.category).toBe("railway");
    expect(def?.width).toBe(320);
    expect(def?.depth).toBe(310);
    expect(def?.height).toBe(115);
  });

  it("builds 3D meshes for all modular builders", () => {
    const builders = [
      buildStation3D,
      buildSignalBox3D,
      buildEngineShed3D,
      buildTownHouse3D,
      buildWaterTower3D,
      buildLightSignal3D,
      buildBufferStop3D,
      buildPineTree3D,
      buildDeciduousTree3D,
      buildPlatformLamp3D,
      buildGenericBuilding3D,
      buildTurntableTrainhall3D,
      buildSkyscraper3D,
      buildIndustryBuilding3D,
    ];

    for (const builder of builders) {
      const group = new THREE.Group();
      builder(group);
      expect(group.children.length).toBeGreaterThan(0);
    }
  });

  it("includes skyscraper and industry-building in BUILTIN_MODELS", () => {
    const sky = findBuiltinModelDef("skyscraper");
    expect(sky).toBeDefined();
    expect(sky?.name).toContain("Wolkenkratzer");
    expect(sky?.height).toBe(280);

    const ind = findBuiltinModelDef("industry-building");
    expect(ind).toBeDefined();
    expect(ind?.name).toContain("Industriehalle");
    expect(ind?.width).toBe(260);
  });

  it("builds 3D mesh for turntable-trainhall via createBuiltinModel3DMesh", () => {
    const fakeModel = {
      id: "m-1",
      name: "Ringlokschuppen",
      category: "railway" as const,
      modelType: "builtin" as const,
      builtinId: "turntable-trainhall",
      transform: { x: 0, y: 0, rotation: 0 },
      width: 320,
      depth: 310,
      height: 115,
    };
    const mesh = createBuiltinModel3DMesh("turntable-trainhall", fakeModel, "H0");
    expect(mesh).toBeInstanceOf(THREE.Group);
    expect(mesh.children.length).toBeGreaterThan(0);
  });

  it("ensures no part of turntable-trainhall goes below table (Y >= 0)", () => {
    const group = new THREE.Group();
    buildTurntableTrainhall3D(group);
    group.updateMatrixWorld(true);

    let minY = Infinity;
    group.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry) {
        const geo = obj.geometry;
        const pos = geo.getAttribute("position");
        if (pos) {
          const v = new THREE.Vector3();
          for (let i = 0; i < pos.count; i++) {
            v.fromBufferAttribute(pos, i);
            v.applyMatrix4(obj.matrixWorld);
            if (v.y < minY) minY = v.y;
          }
        }
      }
    });

    expect(minY).toBeGreaterThanOrEqual(-0.001);
  });

  it("snaps turntable-trainhall to a turntable piece", () => {
    const turntablePiece: TrackPiece = {
      id: "tt-1",
      catalogId: "74862",
      transform: { x: 500, y: 500, rotation: 0 },
      connectors: [],
      elevation: { start: 0, end: 0 },
    };

    const mockTurntableEntry: CatalogEntry = {
      id: "74862",
      name: "C-Gleis Drehscheibe",
      description: "C-Gleis Drehscheibe",
      kind: "turntable",
      category: "Zubehör",
      radius: 189,
      localConnectors: [
        { x: 189, y: 0, angle: 0 },
        { x: 184.9, y: 39.3, angle: 12 },
        { x: 172.7, y: 76.9, angle: 24 },
        { x: 0, y: 189, angle: 90 },
      ],
    };

    const getCatalogEntry = (p: TrackPiece) => {
      if (p.id === "tt-1") return mockTurntableEntry;
      return undefined;
    };

    // Turntable center is (500, 500). Target dist ~ 189 + 6 + 155 = 350 mm.
    // Place candidate near exit at 0°: candidate near (500 + 350, 500) = (850, 500)
    const candNear0: Transform = { x: 840, y: 510, rotation: 0 };
    const snapResult0 = snapModelToTurntable(candNear0, [turntablePiece], getCatalogEntry);

    expect(snapResult0).not.toBeNull();
    expect(snapResult0?.exitAngle).toBe(0);
    expect(snapResult0?.snappedTransform.x).toBe(850);
    expect(snapResult0?.snappedTransform.y).toBe(500);
    // At exit 0°, front facing turntable center (-X) has rotation normAngle(0 - 90) = 270°
    expect(snapResult0?.snappedTransform.rotation).toBe(270);

    // Place candidate near exit at 90°: candidate near (500, 500 + 350) = (500, 850)
    const candNear90: Transform = { x: 505, y: 845, rotation: 45 };
    const snapResult90 = snapModelToTurntable(candNear90, [turntablePiece], getCatalogEntry);

    expect(snapResult90).not.toBeNull();
    expect(snapResult90?.exitAngle).toBe(90);
    expect(snapResult90?.snappedTransform.x).toBe(500);
    expect(snapResult90?.snappedTransform.y).toBe(850);
    // At exit 90°, front facing turntable center (-Y) has rotation normAngle(90 - 90) = 0°
    expect(snapResult90?.snappedTransform.rotation).toBe(0);

    // If candidate is too far away, snapping should return null
    const candTooFar: Transform = { x: 1200, y: 1200, rotation: 0 };
    const snapTooFar = snapModelToTurntable(candTooFar, [turntablePiece], getCatalogEntry);
    expect(snapTooFar).toBeNull();
  });

  it("handles transfer-table moving field, distance and rotation handle", () => {
    const entry: CatalogEntry = {
      id: "72941",
      name: "Schiebebühne",
      description: "Schiebebühne",
      kind: "transfer-table",
      category: "Zubehör",
      length: 360,
    };

    const transform: Transform = { x: 100, y: 100, rotation: 0 };

    // Point in center of the transfer table: (100 + 180, 100 + 154.8) = (280, 254.8)
    const dCenter = distanceToTrackPiece({ x: 280, y: 254.8 }, transform, entry);
    expect(dCenter).toBe(0);

    // Point near top edge: (100 + 50, 100 - 50) = (150, 50) -> inside [-55.2, 364.8]
    const dTop = distanceToTrackPiece({ x: 150, y: 50 }, transform, entry);
    expect(dTop).toBe(0);

    // Point outside by 20 mm to the right: (100 + 380, 100 + 100) -> 20 mm outside
    const dOutside = distanceToTrackPiece({ x: 480, y: 200 }, transform, entry);
    expect(Math.round(dOutside)).toBe(20);

    // Rotation handle must be positioned with stemOffset = 240, placing it above the top edge
    const rotHandle = getPieceRotationHandleWorld(entry, transform);
    // Center is (100 + 180, 100 + 154.8) = (280, 254.8). Handle Y = 254.8 - 240 = 14.8 (well above top edge at 44.8)
    expect(rotHandle.x).toBe(280);
    expect(Math.round(rotHandle.y)).toBe(15);

    // Selection box enclosing the transfer table
    const piece: TrackPiece = {
      id: "tt-piece-1",
      catalogId: "72941",
      transform,
      connectors: [],
    };
    const inBox = isPieceInSelectionBox(piece, entry, { x: 50, y: 0 }, { x: 500, y: 500 });
    expect(inBox).toBe(true);
  });
});
