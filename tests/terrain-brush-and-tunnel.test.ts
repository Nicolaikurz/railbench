import { describe, it, expect } from "vitest";
import * as THREE from "three";
import { Container } from "pixi.js";
import type { Layout, Model3D, TrackPiece } from "@/core/types";
import { ToggleTunnelCommand, ToggleBridgeCommand } from "@/editor/commands";
import {
  TerrainBrushController,
  applyAutoTunnelAllMountains,
  smoothMountainPath,
} from "@/editor/terrain-brush-controller";
import {
  buildMountain3D,
  computeTunnelRuns,
  findCollidingTracksForMountain,
  R_PORTAL,
} from "@/render/models-3d/mountain";
import { drawTunnel2D, buildPieceTunnel3D } from "@/render/track-tunnel";
import { createBuiltinModel, isPointInsideModel } from "@/core/models-3d";
import { createBuiltinModel3DMesh } from "@/render/builtin-models-3d";
import { createModel2DGraphics } from "@/render/model-sprite";

import type { CatalogEntry } from "@/catalog/catalog";

const straightEntry: CatalogEntry = {
  id: "24188",
  name: "Straight Track 188.3mm",
  category: "straight",
  description: "Straight",
  kind: "straight" as const,
  length: 188.3,
  gauge: 16.5,
  localConnectors: [
    { x: 0, y: 0, angle: 180 },
    { x: 188.3, y: 0, angle: 0 },
  ],
};

const roadEntry: CatalogEntry = {
  id: "road-str-200",
  name: "Straight Street 200mm",
  category: "straight",
  description: "Street",
  kind: "straight" as const,
  isRoad: true,
  roadWidth: 66,
  length: 200,
  localConnectors: [
    { x: 0, y: 0, angle: 180 },
    { x: 200, y: 0, angle: 0 },
  ],
};

describe("Terrain Brush, Overlapping Mountain Tunnels & Track/Street Tunnel Toggle", () => {
  it("suppresses mid-mountain portals when two mountains overlap along a track chain", () => {
    const m1: Model3D = {
      id: "m-left",
      name: "Mountain Left",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 350, y: 500, rotation: 0 },
      width: 500,
      depth: 400,
      height: 220,
    };
    const m2: Model3D = {
      id: "m-right",
      name: "Mountain Right",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 650, y: 500, rotation: 0 },
      width: 500,
      depth: 400,
      height: 220,
    };

    const tracks: TrackPiece[] = [0, 1, 2, 3, 4, 5].map((i) => ({
      id: `t-${i}`,
      catalogId: "24188",
      transform: { x: 50 + i * 188.3, y: 500, rotation: 0 },
      connectors: [
        {
          x: 50 + i * 188.3,
          y: 500,
          angle: 180,
          connectedTo: i > 0 ? { pieceId: `t-${i - 1}`, connectorIndex: 1 } : undefined,
        },
        {
          x: 50 + (i + 1) * 188.3,
          y: 500,
          angle: 0,
          connectedTo: i < 5 ? { pieceId: `t-${i + 1}`, connectorIndex: 0 } : undefined,
        },
      ],
    }));

    const cols1 = findCollidingTracksForMountain(m1, tracks, () => straightEntry);
    const cols2 = findCollidingTracksForMountain(m2, tracks, () => straightEntry);
    expect(cols1.length).toBe(1);
    expect(cols2.length).toBe(1);

    const runs1 = computeTunnelRuns(m1, cols1, R_PORTAL, [m1, m2]);
    const runs2 = computeTunnelRuns(m2, cols2, R_PORTAL, [m1, m2]);
    expect(runs1.length).toBe(1);
    expect(runs2.length).toBe(1);

    // Outer entrance of m1 is not suppressed; inner exit of m1 (inside m2) IS suppressed
    expect(runs1[0]!.startPortal?.suppressedByOverlap).toBe(false);
    expect(runs1[0]!.endPortal?.suppressedByOverlap).toBe(true);

    // Inner entrance of m2 (inside m1) IS suppressed; outer exit of m2 is not suppressed
    expect(runs2[0]!.startPortal?.suppressedByOverlap).toBe(true);
    expect(runs2[0]!.endPortal?.suppressedByOverlap).toBe(false);

    // Building 3D meshes should only construct 1 visible stone portal per mountain (2 total for the massif)
    const g1 = new THREE.Group();
    buildMountain3D(g1, m1, { collisions: cols1, allMountains: [m1, m2] });
    const sub1 = g1.children[0] as THREE.Group;
    const portals1 = sub1.children.filter((c) => c instanceof THREE.Group && c.children.length >= 4);
    expect(portals1.length).toBe(1);
  });

  it("toggles hasTunnel on tracks and streets via ToggleTunnelCommand and clears hasBridge", () => {
    const layout: Layout = {
      name: "Test",
      scale: "H0",
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [],
          hasBridge: true,
        },
        {
          id: "r1",
          catalogId: "road-str-200",
          transform: { x: 200, y: 0, rotation: 0 },
          connectors: [],
        },
      ],
      models: [],
    };

    const cmd = new ToggleTunnelCommand(["p1", "r1"]);
    cmd.do(layout);
    expect(layout.pieces[0]!.hasTunnel).toBe(true);
    expect(layout.pieces[0]!.hasBridge).toBe(false);
    expect(layout.pieces[1]!.hasTunnel).toBe(true);

    // Undo restores previous states including hasBridge
    cmd.undo(layout);
    expect(layout.pieces[0]!.hasTunnel).toBeUndefined();
    expect(layout.pieces[0]!.hasBridge).toBe(true);
    expect(layout.pieces[1]!.hasTunnel).toBeUndefined();

    // Enabling bridge clears tunnel
    cmd.do(layout);
    const bridgeCmd = new ToggleBridgeCommand(["p1"]);
    bridgeCmd.do(layout);
    expect(layout.pieces[0]!.hasBridge).toBe(true);
    expect(layout.pieces[0]!.hasTunnel).toBe(false);
  });

  it("paints freeform mountain ridges with TerrainBrushController and auto-tunnels intersected tracks & streets", () => {
    const brush = new TerrainBrushController();
    brush.setActive(true, "paint");
    brush.brushRadius = 180;
    brush.brushHeight = 240;
    brush.brushStyle = "rocky";
    brush.autoTunnel = true;

    const layout: Layout = {
      name: "Test",
      scale: "H0",
      pieces: [
        {
          id: "rail-under-brush",
          catalogId: "24188",
          transform: { x: 200, y: 200, rotation: 0 },
          connectors: [
            { x: 100, y: 200, angle: 180 },
            { x: 300, y: 200, angle: 0 },
          ],
        },
        {
          id: "road-under-brush",
          catalogId: "road-str-200",
          transform: { x: 450, y: 210, rotation: 0 },
          connectors: [
            { x: 350, y: 210, angle: 180 },
            { x: 550, y: 210, angle: 0 },
          ],
        },
      ],
      models: [],
    };

    brush.startStroke({ x: 150, y: 200 }, layout);
    brush.moveStroke({ x: 320, y: 205 }, layout);
    brush.moveStroke({ x: 500, y: 210 }, layout);
    const created = brush.finishStroke(layout, "layer-1");

    expect(created).not.toBeNull();
    expect(created!.builtinId).toBe("mountain");
    expect(created!.mountainStyle).toBe("rocky");
    expect(created!.mountainPath?.length).toBeGreaterThanOrEqual(2);

    // Hit test works along the painted ridge path
    expect(isPointInsideModel({ x: 200, y: 200 }, created!)).toBe(true);
    expect(isPointInsideModel({ x: 480, y: 210 }, created!)).toBe(true);
    expect(isPointInsideModel({ x: 1200, y: 1200 }, created!)).toBe(false);

    // Intersected rail track and street were automatically marked as tunnels
    expect(layout.pieces[0]!.hasTunnel).toBe(true);
    expect(layout.pieces[1]!.hasTunnel).toBe(true);

    // 3D mountain mesh builds cleanly from freeform mountainPath
    const group = new THREE.Group();
    const cols = findCollidingTracksForMountain(created!, layout.pieces, (id) =>
      id === "road-str-200" ? roadEntry : straightEntry,
    );
    buildMountain3D(group, created!, cols);
    expect(group.children.length).toBe(1);

    // Node-granular eraser removes individual nodes first, then the mountain when all nodes are erased
    brush.setActive(true, "erase");
    const erasedNode = brush.startStroke({ x: 320, y: 205 }, layout);
    expect(erasedNode).toBe(true);
    expect(layout.models?.length).toBe(1);
    brush.moveStroke({ x: 150, y: 200 }, layout);
    brush.moveStroke({ x: 500, y: 210 }, layout);
    brush.finishStroke(layout);
    expect(layout.models?.length).toBe(0);
  });

  it("supports extend mode, stamp mode, ridge smoothing, and radius adjustment", () => {
    const brush = new TerrainBrushController();
    brush.setActive(true, "stamp");
    brush.brushRadius = 200;
    brush.brushHeight = 180;

    const layout: Layout = {
      name: "Test",
      scale: "H0",
      pieces: [],
      models: [],
    };

    // 1. Stamp single peak
    brush.startStroke({ x: 100, y: 100 }, layout);
    brush.moveStroke({ x: 120, y: 110 }, layout);
    const stamped = brush.finishStroke(layout);
    expect(stamped).not.toBeNull();
    expect(stamped!.mountainPath).toBeUndefined();
    expect(stamped!.transform.x).toBe(120);
    expect(layout.models?.length).toBe(1);

    // 2. Extend existing mountain with a ridge stroke
    brush.setActive(true, "extend", stamped!.id);
    brush.ridgeProfile = "peak-center";
    brush.startStroke({ x: 250, y: 110 }, layout);
    brush.moveStroke({ x: 420, y: 150 }, layout);
    brush.moveStroke({ x: 600, y: 180 }, layout);
    const extended = brush.finishStroke(layout);
    expect(extended?.id).toBe(stamped!.id);
    expect(layout.models?.length).toBe(1);
    expect(extended!.mountainPath?.length).toBeGreaterThanOrEqual(3);

    // 3. Smooth mountain path
    const smoothed = smoothMountainPath(extended!);
    expect(smoothed).toBe(true);

    // 4. Adjust brush radius with clamping
    brush.adjustBrushRadius(50);
    expect(brush.brushRadius).toBe(250);
    brush.adjustBrushRadius(-500);
    expect(brush.brushRadius).toBe(60);
  });

  it("renders standalone 2D and 3D tunnels for tracks and streets with hasTunnel = true", () => {
    const piece: TrackPiece = {
      id: "t-standalone",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
      hasTunnel: true,
    };

    const container2D = new Container();
    drawTunnel2D(container2D, straightEntry, piece, [piece]);
    expect(container2D.children.length).toBeGreaterThanOrEqual(2);

    const group3D = new THREE.Group();
    buildPieceTunnel3D(group3D, straightEntry, piece, 0, 0, {
      name: "Test",
      scale: "H0",
      pieces: [piece],
      models: [],
    });
    // Inner lining mesh + outer cover shell + 2 stone portal groups
    expect(group3D.children.length).toBe(4);
  });

  it("applyAutoTunnelAllMountains marks tracks crossing existing mountains", () => {
    const layout: Layout = {
      name: "Test",
      scale: "H0",
      pieces: [
        {
          id: "p-in",
          catalogId: "24188",
          transform: { x: 500, y: 500, rotation: 0 },
          connectors: [
            { x: 400, y: 500, angle: 180 },
            { x: 600, y: 500, angle: 0 },
          ],
        },
      ],
      models: [
        {
          id: "m-1",
          name: "Berg 1",
          category: "nature",
          modelType: "builtin",
          builtinId: "mountain",
          transform: { x: 500, y: 500, rotation: 0 },
          width: 600,
          depth: 450,
          height: 220,
        },
      ],
    };

    const count = applyAutoTunnelAllMountains(layout);
    expect(count).toBe(1);
    expect(layout.pieces[0]!.hasTunnel).toBe(true);
  });

  it("builds 3D meshes and 2D top-view graphics for coaling-station and diesel-fuel-station", () => {
    const coaling = createBuiltinModel("coaling-station", { x: 100, y: 200, rotation: 0 });
    expect(coaling.category).toBe("railway");
    const coalingMesh = createBuiltinModel3DMesh("coaling-station", coaling, "H0");
    expect(coalingMesh.children.length).toBeGreaterThanOrEqual(1);
    expect(coalingMesh.children.length).toBeLessThanOrEqual(8);
    const coaling2D = createModel2DGraphics(coaling, true);
    expect(coaling2D.children.length).toBeGreaterThanOrEqual(1);

    const diesel = createBuiltinModel("diesel-fuel-station", { x: 300, y: 200, rotation: 0 });
    expect(diesel.category).toBe("railway");
    const dieselMesh = createBuiltinModel3DMesh("diesel-fuel-station", diesel, "H0");
    expect(dieselMesh.children.length).toBeGreaterThanOrEqual(1);
    expect(dieselMesh.children.length).toBeLessThanOrEqual(8);
    const diesel2D = createModel2DGraphics(diesel, true);
    expect(diesel2D.children.length).toBeGreaterThanOrEqual(1);
  });
});
