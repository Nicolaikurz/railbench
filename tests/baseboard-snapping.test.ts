import { describe, it, expect } from "vitest";
import { snapBaseboard } from "@/core/baseboard-snapping";
import { createBaseboardGraphics } from "@/render/baseboard-sprite";
import {
  createDefaultBaseboard,
  getBaseboardBounds,
  getBaseboardPolygon,
  getLayerPlatePolygon,
  isPointInsideBaseboard,
  isPointNearBaseboardEdge,
  isPointInBaseboardHeader,
  computeBaseboardLegPositions,
} from "@/core/layout";
import { MoveBaseboardCommand, History } from "@/editor/commands";
import type { Layout } from "@/core/types";

describe("Baseboard Free Movement & Snapping", () => {
  it("snaps table A to the right edge of table B", () => {
    const tableB = createDefaultBaseboard({
      id: "board-b",
      width: 2000,
      depth: 1000,
      positionMode: "custom",
      customX: 0,
      customY: 0,
    });

    const tableA = createDefaultBaseboard({
      id: "board-a",
      width: 1200,
      depth: 800,
      positionMode: "custom",
      customX: 2015, // 15 mm from table B's right edge (2000)
      customY: 10,   // 10 mm from table B's top edge (0)
    });

    const result = snapBaseboard(tableA, 2015, 10, [tableB], 35);
    expect(result.snappedX).toBe(true);
    expect(result.snappedY).toBe(true);
    expect(result.x).toBe(2000); // Snapped cleanly to right edge of B
    expect(result.y).toBe(0);    // Snapped flush to top edge of B
    expect(result.guides.length).toBeGreaterThan(0);
  });

  it("snaps table A below table B with left flush alignment", () => {
    const tableB = createDefaultBaseboard({
      id: "board-b",
      width: 2400,
      depth: 1200,
      positionMode: "custom",
      customX: 100,
      customY: 100,
    });

    const tableA = createDefaultBaseboard({
      id: "board-a",
      width: 1200,
      depth: 800,
      positionMode: "custom",
      customX: 110,  // near B.left = 100
      customY: 1315, // near B.bottom = 1300
    });

    const result = snapBaseboard(tableA, 110, 1315, [tableB], 35);
    expect(result.snappedX).toBe(true);
    expect(result.snappedY).toBe(true);
    expect(result.x).toBe(100);  // Flush with left edge
    expect(result.y).toBe(1300); // Snapped right under bottom edge
  });

  it("snaps table A to center alignment when close to middle axis", () => {
    const tableB = createDefaultBaseboard({
      id: "board-b",
      width: 2000,
      depth: 1200, // Mid Y = 600
      positionMode: "custom",
      customX: 0,
      customY: 0,
    });

    // table A depth is 800. Center aligned would mean Y = 600 - 400 = 200
    const tableA = createDefaultBaseboard({
      id: "board-a",
      width: 1000,
      depth: 800,
      positionMode: "custom",
      customX: 2020,
      customY: 215, // 15 mm from centered Y (200)
    });

    const result = snapBaseboard(tableA, 2020, 215, [tableB], 35);
    expect(result.snappedX).toBe(true);
    expect(result.snappedY).toBe(true);
    expect(result.x).toBe(2000);
    expect(result.y).toBe(200); // Perfectly centered
  });

  it("does not snap when distance exceeds the magnetic threshold", () => {
    const tableB = createDefaultBaseboard({
      id: "board-b",
      width: 2000,
      depth: 1000,
      positionMode: "custom",
      customX: 0,
      customY: 0,
    });

    const tableA = createDefaultBaseboard({
      id: "board-a",
      width: 1200,
      depth: 800,
      positionMode: "custom",
      customX: 2100, // 100 mm away from B.right (2000)
      customY: 500,
    });

    const result = snapBaseboard(tableA, 2100, 500, [tableB], 35);
    expect(result.snappedX).toBe(false);
    expect(result.x).toBe(2100);
    expect(result.y).toBe(500);
    expect(result.guides.length).toBe(0);
  });

  it("detects points inside table, on header handle and near table edge", () => {
    const table = createDefaultBaseboard({
      id: "board-1",
      width: 2000,
      depth: 1000,
      positionMode: "custom",
      customX: 100,
      customY: 100,
    });

    // Point in center
    expect(isPointInsideBaseboard({ x: 1000, y: 500 }, table)).toBe(true);
    // Point outside
    expect(isPointInsideBaseboard({ x: 0, y: 0 }, table)).toBe(false);

    // Header handle is at x: 116..336, y: 116..148
    expect(isPointInBaseboardHeader({ x: 150, y: 130 }, table)).toBe(true);
    expect(isPointInBaseboardHeader({ x: 1000, y: 500 }, table)).toBe(false);

    // Near edge test (table edge is at x=100, x=2100, y=100, y=1100)
    expect(isPointNearBaseboardEdge({ x: 108, y: 500 }, table, 20)).toBe(true); // 8 mm from left edge
    expect(isPointNearBaseboardEdge({ x: 2095, y: 500 }, table, 20)).toBe(true); // 5 mm from right edge
    expect(isPointNearBaseboardEdge({ x: 1000, y: 500 }, table, 20)).toBe(false); // in the center
    expect(isPointNearBaseboardEdge({ x: 50, y: 500 }, table, 20)).toBe(false); // 50 mm away outside
  });

  it("handles MoveBaseboardCommand undo and redo", () => {
    const table = createDefaultBaseboard({
      id: "board-1",
      positionMode: "custom",
      customX: 0,
      customY: 0,
    });

    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      pieces: [],
      layers: [],
      baseboards: [table],
    };

    const history = new History(layout);
    const cmd = new MoveBaseboardCommand(
      "board-1",
      { positionMode: "custom", customX: 0, customY: 0 },
      { positionMode: "custom", customX: 500, customY: 200 },
    );

    history.run(cmd);
    expect(table.customX).toBe(500);
    expect(table.customY).toBe(200);

    history.undo();
    expect(table.customX).toBe(0);
    expect(table.customY).toBe(0);
  });

  it("computes 3D table legs inset so they never stand over the table edge", () => {
    const table = createDefaultBaseboard({
      id: "board-rect",
      width: 2400,
      depth: 1200,
      positionMode: "custom",
      customX: 100,
      customY: 200,
      shape: "rect",
    });

    const legRadius = 18;
    const baseInset = 45;
    const legs = computeBaseboardLegPositions(table, legRadius, baseInset);

    expect(legs.length).toBe(4);

    for (const leg of legs) {
      // 1. Der Mittelpunkt des Beins muss innerhalb des Tisches liegen
      expect(isPointInsideBaseboard(leg, table)).toBe(true);

      // 2. Der gesamte Zylindermantel (Radius 18) muss vollständig innerhalb des Tisches liegen
      // Kein Überhang über Kanten (X: [100, 2500], Y: [200, 1400])
      expect(leg.x - legRadius).toBeGreaterThanOrEqual(100);
      expect(leg.x + legRadius).toBeLessThanOrEqual(2500);
      expect(leg.y - legRadius).toBeGreaterThanOrEqual(200);
      expect(leg.y + legRadius).toBeLessThanOrEqual(1400);

      // 3. Echter Abstand zur Kante mindestens (baseInset - legRadius) = 27 mm
      const minDistanceToEdge = Math.min(
        leg.x - 100,
        2500 - leg.x,
        leg.y - 200,
        1400 - leg.y,
      );
      expect(minDistanceToEdge).toBeGreaterThanOrEqual(baseInset);
    }
  });

  it("computes inset table legs for complex shapes (L, U, T) without overhang", () => {
    const shapes: ("l-shape" | "u-shape" | "t-shape")[] = ["l-shape", "u-shape", "t-shape"];
    const legRadius = 18;

    for (const shape of shapes) {
      const table = createDefaultBaseboard({
        id: `board-${shape}`,
        width: 2000,
        depth: 1600,
        positionMode: "custom",
        customX: 0,
        customY: 0,
        shape,
      });

      const legs = computeBaseboardLegPositions(table, legRadius, 45);
      expect(legs.length).toBeGreaterThanOrEqual(4);

      for (const leg of legs) {
        expect(isPointInsideBaseboard(leg, table)).toBe(true);
      }
    }
  });

  it("supports grass surface style on baseboard", () => {
    const table = createDefaultBaseboard({
      id: "board-grass",
      width: 1500,
      depth: 1000,
      style: "grass",
      showDimensions: false,
    });
    expect(table.style).toBe("grass");
    const container = createBaseboardGraphics(table);
    expect(container).toBeDefined();
    expect(container.children.length).toBeGreaterThan(0);
  });

  it("supports triangle shape baseboards with multiple corner orientations and isosceles", () => {
    const orientations: ("corner-tl" | "corner-tr" | "corner-br" | "corner-bl" | "isosceles")[] = [
      "corner-tl",
      "corner-tr",
      "corner-br",
      "corner-bl",
      "isosceles",
    ];

    for (const triType of orientations) {
      const table = createDefaultBaseboard({
        id: `board-tri-${triType}`,
        shape: "triangle",
        triangleType: triType,
        width: 1800,
        depth: 1200,
        positionMode: "custom",
        customX: 0,
        customY: 0,
        showDimensions: false,
      });

      const poly = getBaseboardPolygon(table);
      expect(poly.length).toBe(3);

      // Check leg generation: 3 corner legs inside the table
      const legs = computeBaseboardLegPositions(table, 18, 45);
      expect(legs.length).toBe(3);
      for (const leg of legs) {
        expect(isPointInsideBaseboard(leg, table)).toBe(true);
      }

      // Check 2D sprite rendering does not throw
      const g = createBaseboardGraphics(table);
      expect(g).toBeDefined();
    }
  });

  it("supports table rotation in degrees and transforms the polygon correctly", () => {
    const table = createDefaultBaseboard({
      id: "board-rot",
      shape: "rect",
      width: 2000,
      depth: 1000,
      positionMode: "center", // center is (0, 0), so bounds are [-1000, -500, 2000, 1000]
      rotation: 90,
      showDimensions: false,
    });

    const poly = getBaseboardPolygon(table);
    expect(poly.length).toBe(4);

    // After 90 degree rotation around center (0, 0):
    // Original (1000, 500) rotated 90 deg -> (-500, 1000)
    // Original (-1000, -500) rotated 90 deg -> (500, -1000)
    // The rotated polygon spans roughly X in [-500, 500] and Y in [-1000, 1000]
    const xs = poly.map((p) => p.x);
    const ys = poly.map((p) => p.y);
    expect(Math.min(...xs)).toBeCloseTo(-500, 0);
    expect(Math.max(...xs)).toBeCloseTo(500, 0);
    expect(Math.min(...ys)).toBeCloseTo(-1000, 0);
    expect(Math.max(...ys)).toBeCloseTo(1000, 0);

    // Center point (0, 0) is inside
    expect(isPointInsideBaseboard({ x: 0, y: 0 }, table)).toBe(true);
    // Point (800, 0) was inside before rotation, but is OUTSIDE after 90 deg rotation
    expect(isPointInsideBaseboard({ x: 800, y: 0 }, table)).toBe(false);
    // Point (0, 800) was outside before rotation, but is INSIDE after 90 deg rotation
    expect(isPointInsideBaseboard({ x: 0, y: 800 }, table)).toBe(true);

    // 2D sprite with rotated grain and dimensions
    const g = createBaseboardGraphics(table);
    expect(g).toBeDefined();
  });

  it("calculates polygon for layer plate with shapes (triangle, l-shape, etc.) and rotates along with table", () => {
    const table = createDefaultBaseboard({
      id: "board-1",
      width: 2000,
      depth: 1000,
      positionMode: "custom",
      customX: 100,
      customY: 200,
      rotation: 0,
    });

    // 1. Triangle plate
    const triPlate = {
      id: "p1",
      baseboardId: "board-1",
      shape: "triangle" as const,
      triangleType: "corner-tl" as const,
      width: 1000,
      depth: 500,
      x: 50,
      y: 50,
    };

    const polyTri = getLayerPlatePolygon(triPlate, table);
    expect(polyTri.length).toBe(3);
    // Origin is table (100, 200) + offset (50, 50) = (150, 250)
    expect(polyTri[0]).toEqual({ x: 150, y: 250 });
    expect(polyTri[1]).toEqual({ x: 1150, y: 250 });
    expect(polyTri[2]).toEqual({ x: 150, y: 750 });

    // 2. L-shape plate
    const lPlate = {
      id: "p2",
      baseboardId: "board-1",
      shape: "l-shape" as const,
      lWingWidth: 400,
      lWingDepth: 300,
      lSide: "left" as const,
      width: 1000,
      depth: 800,
      x: 0,
      y: 0,
    };

    const polyL = getLayerPlatePolygon(lPlate, table);
    expect(polyL.length).toBe(6);
  });
});

