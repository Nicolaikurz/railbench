import { describe, it, expect } from "vitest";
import {
  isFlexTrack,
  getEffectiveFlexEntry,
  adjustFlexTrackOpenConnector,
  cutFlexTrack,
  bendFlexTrack,
  getFlexTrackRampInfo,
  flipFlexTrackPiece,
  findNearbyConnectorForFlex,
} from "@/core/flextrack";
import type { CatalogEntry } from "@/catalog/catalog";
import type { TrackPiece, Layer } from "@/core/types";
import { toXML, toJSON, fromJSON } from "@/io/serialize";
import { createLayout } from "@/core/layout";
import { updatePieceConnections } from "@/core/snapping";

describe("Flextrack System", () => {
  const flexCatalogEntry: CatalogEntry = {
    id: "2205",
    name: "Flexgleis 900 mm",
    kind: "flex",
    category: "Flexgleise",
    description: "Flexibles Gleis 900 mm",
    length: 900,
    gauge: 16.5,
  };

  const createTestPiece = (overrides?: Partial<TrackPiece>): TrackPiece => ({
    id: "piece-flex-1",
    catalogId: "2205",
    transform: { x: 100, y: 100, rotation: 0 },
    connectors: [
      { x: 100, y: 100, angle: 180 },
      { x: 1000, y: 100, angle: 0 },
    ],
    ...overrides,
  });

  it("identifies flextrack correctly", () => {
    expect(isFlexTrack(flexCatalogEntry)).toBe(true);

    const normalPiece: TrackPiece = {
      id: "p1",
      catalogId: "2200",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
    };
    expect(isFlexTrack({ id: "2200", name: "Gerade", kind: "straight", category: "Geraden", description: "" }, normalPiece)).toBe(false);

    // Track with customLength/customRadius is also treated as flex
    const customPiece: TrackPiece = {
      ...normalPiece,
      customLength: 450,
    };
    expect(isFlexTrack(undefined, customPiece)).toBe(true);

    // Track with "flex" in name or description is recognized even if kind is "straight"
    expect(isFlexTrack({ id: "55205", name: "Gerades Gleis (Flexgleis)", kind: "straight", category: "Geraden", description: "" })).toBe(true);
    expect(isFlexTrack({ id: "9999", name: "Custom Track", kind: "straight", category: "Geraden", description: "Standard Flex track" })).toBe(true);
  });

  it("returns effective flex entry for straight and curved flex pieces", () => {
    const piece = createTestPiece({ customLength: 500 });
    const effectiveStraight = getEffectiveFlexEntry(flexCatalogEntry, piece);
    expect(effectiveStraight.length).toBe(500);
    expect(effectiveStraight.kind).toBe("straight");

    const curvedPiece = createTestPiece({ customLength: 450, customRadius: 600, customAngle: 30 });
    const effectiveCurved = getEffectiveFlexEntry(flexCatalogEntry, curvedPiece);
    expect(effectiveCurved.kind).toBe("curve");
    expect(effectiveCurved.radius).toBe(600);
    expect(effectiveCurved.angle).toBe(30);

    // Negative angle (right-hand curve)
    const rightCurvedPiece = createTestPiece({ customLength: 450, customRadius: 600, customAngle: -45 });
    const effectiveRight = getEffectiveFlexEntry(flexCatalogEntry, rightCurvedPiece);
    expect(effectiveRight.kind).toBe("curve");
    expect(effectiveRight.radius).toBe(600);
    expect(effectiveRight.angle).toBe(-45);
  });

  it("cuts flextrack to custom length and updates connectors", () => {
    const piece = createTestPiece();
    cutFlexTrack(piece, flexCatalogEntry, 350);

    expect(piece.customLength).toBe(350);
    expect(piece.connectors[1]!.x).toBeCloseTo(450, 0); // 100 + 350
    expect(piece.connectors[1]!.y).toBeCloseTo(100, 0);
  });

  it("bends flextrack to specified radius and angle", () => {
    const piece = createTestPiece();
    bendFlexTrack(piece, flexCatalogEntry, 500, 45);

    expect(piece.customRadius).toBe(500);
    expect(piece.customAngle).toBe(45);
    // Arc length = PI * 500 * 45 / 180 = ~392.7 mm
    expect(piece.customLength).toBeCloseTo(392.7, 0);
    expect(piece.connectors.length).toBe(2);

    // Resetting to straight (no radius/angle)
    bendFlexTrack(piece, flexCatalogEntry, undefined, undefined);
    expect(piece.customRadius).toBeUndefined();
    expect(piece.customAngle).toBeUndefined();
  });

  it("adjusts open connector dynamically when dragged (straight)", () => {
    const piece = createTestPiece();
    // Drag endpoint from (1000, 100) to (550, 100) -> delta x = 450
    const res = adjustFlexTrackOpenConnector(piece, flexCatalogEntry, { x: 550, y: 100 });

    expect(res.customLength).toBe(450);
    expect(res.customRadius).toBeUndefined();
    expect(res.customAngle).toBeUndefined();
    expect(piece.connectors[1]!.x).toBeCloseTo(550, 0);
    expect(piece.connectors[1]!.y).toBeCloseTo(100, 0);
  });

  it("adjusts open connector dynamically when curved (left and right curves)", () => {
    const piece = createTestPiece();
    // Drag endpoint into +Y (left curve)
    const resLeft = adjustFlexTrackOpenConnector(piece, flexCatalogEntry, { x: 400, y: 200 });
    expect(resLeft.customRadius).toBeDefined();
    expect(resLeft.customAngle).toBeGreaterThan(0);
    expect(piece.connectors[1]!.x).toBeCloseTo(400, 0);
    expect(piece.connectors[1]!.y).toBeCloseTo(200, 0);

    // Drag endpoint into -Y (right curve)
    const resRight = adjustFlexTrackOpenConnector(piece, flexCatalogEntry, { x: 400, y: 0 });
    expect(resRight.customRadius).toBeDefined();
    expect(resRight.customAngle).toBeLessThan(0);
    expect(piece.connectors[1]!.x).toBeCloseTo(400, 0);
    expect(piece.connectors[1]!.y).toBeCloseTo(0, 0);
  });

  it("tacks / snaps open connector to nearby other track connector", () => {
    const targetPiece: TrackPiece = {
      id: "target-1",
      catalogId: "2200",
      transform: { x: 600, y: 150, rotation: 0 },
      connectors: [{ x: 600, y: 150, angle: 180 }],
    };

    const flexPiece = createTestPiece();
    // Drag close to target connector (e.g. at 605, 148 within snap radius 25)
    const res = adjustFlexTrackOpenConnector(
      flexPiece,
      flexCatalogEntry,
      { x: 605, y: 148 },
      [flexPiece, targetPiece],
      undefined,
      25,
    );

    expect(res.snappedConnector).toBeDefined();
    expect(res.snappedConnector?.pieceId).toBe("target-1");
    // Flex piece connector 1 should snap exactly onto target connector
    expect(flexPiece.connectors[1]!.x).toBeCloseTo(600, 0);
    expect(flexPiece.connectors[1]!.y).toBeCloseTo(150, 0);
  });

  it("calculates continuous ramp gradients for flextracks", () => {
    const layers: Layer[] = [{ id: "layer-1", name: "Ebene 0", color: "#38bdf8", visible: true, locked: false, heightMm: 0 }];
    const piece = createTestPiece({
      customLength: 1000,
      elevation: { start: 0, end: 35 },
    });

    const rampInfo = getFlexTrackRampInfo(piece, flexCatalogEntry, layers);
    expect(rampInfo.isRamp).toBe(true);
    expect(rampInfo.lengthMm).toBe(1000);
    expect(rampInfo.riseMm).toBe(35);
    expect(rampInfo.gradientPercent).toBe(3.5); // 35 / 1000 * 100 = 3.5%
  });

  it("serializes and deserializes flextrack custom geometry to/from XML and JSON", () => {
    const layout = createLayout("Flex Test");
    const piece = createTestPiece({
      customLength: 620.5,
      customRadius: 750.2,
      customAngle: -28.4,
      elevation: { start: 10, end: 45 },
    });
    layout.pieces.push(piece);

    const xml = toXML(layout);
    expect(xml).toContain('customLength="620.5"');
    expect(xml).toContain('customRadius="750.2"');
    expect(xml).toContain('customAngle="-28.40"');

    const json = toJSON(layout);
    const restored = fromJSON(json);
    const restoredPiece = restored.pieces.find((p) => p.id === piece.id);
    expect(restoredPiece).toBeDefined();
    expect(restoredPiece?.customLength).toBe(620.5);
    expect(restoredPiece?.customRadius).toBe(750.2);
    expect(restoredPiece?.customAngle).toBe(-28.4);
    expect(restoredPiece?.elevation?.start).toBe(10);
    expect(restoredPiece?.elevation?.end).toBe(45);
  });

  it("flips flextrack correctly (swapping connectors and reversing curve and elevation)", () => {
    // 1. Straight flex track
    const straightPiece = createTestPiece({ customLength: 500 });
    // Connectors are initially at (100, 100) and (600, 100)
    straightPiece.connectors = [
      { x: 100, y: 100, angle: 180 },
      { x: 600, y: 100, angle: 0 },
    ];
    flipFlexTrackPiece(straightPiece, flexCatalogEntry);
    // After flip, origin is at (600, 100), rotation is 180
    expect(straightPiece.transform.x).toBeCloseTo(600, 0);
    expect(straightPiece.transform.y).toBeCloseTo(100, 0);
    expect(straightPiece.transform.rotation).toBeCloseTo(180, 0);
    // Connector 0 should now be at (600, 100) pointing at 0 deg
    expect(straightPiece.connectors[0]!.x).toBeCloseTo(600, 0);
    expect(straightPiece.connectors[0]!.y).toBeCloseTo(100, 0);
    // Connector 1 should now be at (100, 100) pointing at 180 deg
    expect(straightPiece.connectors[1]!.x).toBeCloseTo(100, 0);
    expect(straightPiece.connectors[1]!.y).toBeCloseTo(100, 0);

    // 2. Curved flex track with elevation
    const curvedPiece = createTestPiece({
      customLength: 450,
      customRadius: 600,
      customAngle: 30,
      elevation: { start: 10, end: 40 },
    });
    // Set connectors for curved piece
    curvedPiece.connectors = [
      { x: 100, y: 100, angle: 180 },
      { x: 400, y: 200, angle: 30 },
    ];
    flipFlexTrackPiece(curvedPiece, flexCatalogEntry);
    expect(curvedPiece.customAngle).toBe(-30);
    expect(curvedPiece.elevation?.start).toBe(40);
    expect(curvedPiece.elevation?.end).toBe(10);
    expect(curvedPiece.connectors[0]!.x).toBeCloseTo(400, 0);
    expect(curvedPiece.connectors[0]!.y).toBeCloseTo(200, 0);
  });

  it("findNearbyConnectorForFlex skips already connected connectors", () => {
    const p1 = createTestPiece({ id: "p1" });
    const target1: TrackPiece = {
      id: "t1",
      catalogId: "2200",
      transform: { x: 500, y: 100, rotation: 0 },
      connectors: [{ x: 500, y: 100, angle: 180, connectedTo: { pieceId: "other", connectorIndex: 0 } }],
    };
    const target2: TrackPiece = {
      id: "t2",
      catalogId: "2200",
      transform: { x: 505, y: 100, rotation: 0 },
      connectors: [{ x: 505, y: 100, angle: 180 }], // open
    };

    const nearby = findNearbyConnectorForFlex(p1, { x: 500, y: 100 }, [p1, target1, target2], 25);
    // Must skip target1 because it's connected, and choose target2
    expect(nearby).toBeDefined();
    expect(nearby?.piece.id).toBe("t2");
  });

  it("strictly prevents flextracks from extending beyond their max catalog length", () => {
    const maxLen = flexCatalogEntry.length!; // 900 mm
    const piece = createTestPiece();

    // 1. Dragging straight beyond max length (e.g. 1500 mm)
    const resStraight = adjustFlexTrackOpenConnector(piece, flexCatalogEntry, { x: 1600, y: 100 });
    expect(resStraight.customLength).toBe(maxLen);
    expect(piece.customLength).toBe(maxLen);
    // Connector 1 cannot be further than maxLen from piece origin (100)
    expect(piece.connectors[1]!.x).toBeCloseTo(100 + maxLen, 0);

    // 2. Dragging curve with extreme radius and angle that would exceed 900 mm
    const resCurved = adjustFlexTrackOpenConnector(piece, flexCatalogEntry, { x: 1200, y: 800 });
    expect(resCurved.customLength).toBeLessThanOrEqual(maxLen);
    expect(piece.customLength).toBeLessThanOrEqual(maxLen);

    // 3. Cutting cannot exceed max length
    cutFlexTrack(piece, flexCatalogEntry, 1800);
    expect(piece.customLength).toBe(maxLen);

    // 4. Bending cannot exceed max length
    bendFlexTrack(piece, flexCatalogEntry, 1000, 90); // Arc would be ~1570 mm
    expect(piece.customLength).toBe(maxLen);
    expect(piece.customAngle).toBeCloseTo((maxLen * 180) / (Math.PI * 1000), 0);

    // 5. Connectors further away than maxLen cannot be snapped to
    const unreachableTarget: TrackPiece = {
      id: "unreachable-1",
      catalogId: "2200",
      transform: { x: 1500, y: 100, rotation: 0 },
      connectors: [{ x: 1500, y: 100, angle: 180 }],
    };
    const snapResult = adjustFlexTrackOpenConnector(
      piece,
      flexCatalogEntry,
      { x: 1500, y: 100 },
      [piece, unreachableTarget],
      undefined,
      25,
    );
    expect(snapResult.snappedConnector).toBeUndefined();
    expect(snapResult.customLength).toBe(maxLen);
  });
});
