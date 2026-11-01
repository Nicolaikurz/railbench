import { describe, it, expect } from "vitest";
import { snapBaseboard } from "@/core/baseboard-snapping";
import {
  createDefaultBaseboard,
  getBaseboardBounds,
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
});

