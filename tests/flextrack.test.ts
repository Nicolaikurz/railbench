import { describe, it, expect } from "vitest";
import {
  isFlexTrack,
  getEffectiveFlexEntry,
  adjustFlexTrackOpenConnector,
  cutFlexTrack,
  bendFlexTrack,
  resetFlexTrackToOriginal,
  getFlexTrackRampInfo,
  flipFlexTrackPiece,
  findNearbyConnectorForFlex,
  getFlexTrackMidpoint,
  getFlexTrackPointAt,
  bendFlexTrackBetweenFixedEnds,
} from "@/core/flextrack";
import type { CatalogEntry } from "@/catalog/catalog";
import type { TrackPiece, Layer } from "@/core/types";
import { toXML, toJSON, fromJSON, fromXML } from "@/io/serialize";
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

  it("adjusts and curves flex streets properly", () => {
    const roadEntry = {
      id: "ST-FLEX",
      name: "Flexstraße 800 mm",
      nameEn: "Flex Road 800 mm",
      kind: "flex" as const,
      category: "Geraden",
      description: "2-spurige flexible Fahrbahn",
      length: 800,
      isRoad: true,
      roadWidth: 60,
      gauge: 16.5,
    };

    const roadPiece: TrackPiece = {
      id: "road-flex-1",
      catalogId: "ST-FLEX",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 800, y: 0, angle: 0 },
      ],
    };

    expect(isFlexTrack(roadEntry, roadPiece)).toBe(true);

    // Adjust length to 400 mm
    const adj = adjustFlexTrackOpenConnector(roadPiece, roadEntry, { x: 400, y: 0 });
    expect(adj.customLength).toBeCloseTo(400, 1);
    expect(roadPiece.connectors[1]!.x).toBeCloseTo(400, 1);
    expect(roadPiece.connectors[1]!.y).toBeCloseTo(0, 1);

    // Bend flex street into a curve
    bendFlexTrack(roadPiece, roadEntry, 250, 45);
    expect(roadPiece.customRadius).toBe(250);
    expect(roadPiece.customAngle).toBe(45);
    const eff = getEffectiveFlexEntry(roadEntry, roadPiece);
    expect(eff.kind).toBe("curve");
    expect(eff.isRoad).toBe(true);
    expect(roadPiece.connectors[1]!.x).toBeGreaterThan(0);
    expect(roadPiece.connectors[1]!.y).toBeGreaterThan(0);
  });

  it("stretches a shortened or curved flex track back into its straight original shape", () => {
    const piece = createTestPiece();
    // 1. Zuerst kürzen und biegen
    bendFlexTrack(piece, flexCatalogEntry, 500, 30);
    expect(piece.customRadius).toBe(500);
    expect(piece.customAngle).toBe(30);

    // 2. Mit der Maus gerade ziehen (Richtung x = 900, y = 5 mit geringer Abweichung)
    // Zielpunkt in Weltkoordinaten: (100 + 950, 100 + 4) -> x = 950 >= maxLen, y = 4 (unter 14 mm Schwelle)
    const adj = adjustFlexTrackOpenConnector(piece, flexCatalogEntry, { x: 1050, y: 104 });
    expect(adj.customLength).toBe(900);
    expect(adj.customRadius).toBeUndefined();
    expect(adj.customAngle).toBeUndefined();

    // Stück ist wieder vollständig gerade und auf voller Originallänge gestreckt
    expect(piece.customLength).toBe(900);
    expect(piece.customRadius).toBeUndefined();
    expect(piece.customAngle).toBeUndefined();
    expect(piece.connectors[1]!.x).toBe(1000);
    expect(piece.connectors[1]!.y).toBe(100);
    expect(piece.connectors[1]!.angle).toBe(0);
  });

  it("resets flex track to original shape via resetFlexTrackToOriginal", () => {
    const piece = createTestPiece({ customLength: 400, customRadius: 600, customAngle: 25 });
    resetFlexTrackToOriginal(piece, flexCatalogEntry);

    expect(piece.customLength).toBeUndefined();
    expect(piece.customRadius).toBeUndefined();
    expect(piece.customAngle).toBeUndefined();
    expect(piece.connectors[1]!.x).toBe(1000);
    expect(piece.connectors[1]!.y).toBe(100);
    expect(piece.connectors[1]!.angle).toBe(0);
  });

  it("resets flex track snappoint and keeps connection anchor when connected at connector 1", () => {
    // Gleis 1 an (0, 0), Ende 1 bei (188.3, 0)
    const p1: TrackPiece = {
      id: "p-anchor",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "piece-flex-1", connectorIndex: 1 } },
      ],
    };

    // Flexgleis mit Connector 1 angeschlossen an p1, aber stark gekrümmt/verkürzt
    const pieceFlex: TrackPiece = {
      id: "piece-flex-1",
      catalogId: "2205",
      transform: { x: 300, y: 200, rotation: 45 },
      customLength: 350,
      customRadius: 400,
      customAngle: 30,
      connectors: [
        { x: 300, y: 200, angle: 225 },
        { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p-anchor", connectorIndex: 1 } },
      ],
    };

    const allPieces = [p1, pieceFlex];
    resetFlexTrackToOriginal(pieceFlex, flexCatalogEntry, allPieces);

    // 1. Flexgleis-Verbindung bleibt an p1 verankert (nun als Connector 0 nach dem Flip)
    expect(pieceFlex.connectors[0]!.connectedTo?.pieceId).toBe("p-anchor");
    expect(p1.connectors[1]!.connectedTo?.pieceId).toBe("piece-flex-1");
    expect(p1.connectors[1]!.connectedTo?.connectorIndex).toBe(0);

    // 2. Offenes Ende (Connector 1) ist sauber auf Originallänge 900 mm geradeaus gestreckt
    expect(pieceFlex.customRadius).toBeUndefined();
    expect(pieceFlex.customAngle).toBeUndefined();
    expect(pieceFlex.customLength).toBeUndefined();
    expect(pieceFlex.connectors[1]!.x).toBeCloseTo(188.3 + 900, 1);
    expect(pieceFlex.connectors[1]!.y).toBeCloseTo(0, 1);
    expect(pieceFlex.connectors[1]!.angle).toBeCloseTo(0, 1);
    expect(pieceFlex.connectors[1]!.connectedTo).toBeUndefined();
  });

  it("calculates midpoint of straight and curved flex tracks correctly", () => {
    // Gerades Gleis von (100, 100) nach (1000, 100)
    const straightPiece = createTestPiece();
    const straightMid = getFlexTrackMidpoint(straightPiece, flexCatalogEntry);
    expect(straightMid.x).toBeCloseTo(550, 1);
    expect(straightMid.y).toBeCloseTo(100, 1);

    // Gekrümmtes Gleis
    const curvedPiece = createTestPiece({
      transform: { x: 0, y: 0, rotation: 0 },
      customLength: 300,
      customRadius: 500,
      customAngle: 34.38,
    });
    const curvedMid = getFlexTrackMidpoint(curvedPiece, flexCatalogEntry);
    expect(curvedMid.x).toBeGreaterThan(0);
    expect(curvedMid.y).toBeGreaterThan(0);
  });

  it("bends flex track from middle while keeping both ends fixed", () => {
    const piece = createTestPiece({
      transform: { x: 100, y: 100, rotation: 0 },
      customLength: 400,
      connectors: [
        { x: 100, y: 100, angle: 180, connectedTo: { pieceId: "track-a", connectorIndex: 0 } },
        { x: 500, y: 100, angle: 0, connectedTo: { pieceId: "track-b", connectorIndex: 1 } },
      ],
    });

    const fixedC0 = { x: 100, y: 100 };
    const fixedC1 = { x: 500, y: 100 };

    // Ziehe den Mittelpunkt nach oben (y = 160)
    bendFlexTrackBetweenFixedEnds(piece, flexCatalogEntry, fixedC0, fixedC1, { x: 300, y: 160 });

    // 1. Beide Endpunkte bleiben exakt an fixedC0 und fixedC1 verankert
    expect(piece.connectors[0]!.x).toBeCloseTo(100, 0);
    expect(piece.connectors[0]!.y).toBeCloseTo(100, 0);
    expect(piece.connectors[1]!.x).toBeCloseTo(500, 0);
    expect(piece.connectors[1]!.y).toBeCloseTo(100, 0);

    // 2. Verbindungen zu track-a und track-b bleiben erhalten
    expect(piece.connectors[0]!.connectedTo?.pieceId).toBe("track-a");
    expect(piece.connectors[1]!.connectedTo?.pieceId).toBe("track-b");

    // 3. Das Gleis ist nun gebogen (bewegt sich in Zugrichtung der Maus)
    expect(piece.customRadius).toBeDefined();
    expect(piece.customRadius!).toBeGreaterThan(100);
    expect(piece.customAngle).toBeDefined();
    expect(piece.customAngle!).toBeLessThan(0); // Nach oben gewölbt in Zugrichtung

    // Mittelpunkt liegt tatsächlich oberhalb der Sehne (y > 100)
    const midAfterPull = getFlexTrackMidpoint(piece, flexCatalogEntry);
    expect(midAfterPull.y).toBeGreaterThan(100);

    // 4. Ziehe zurück zur Sehne (y = 100) -> snappt wieder zu gerade
    bendFlexTrackBetweenFixedEnds(piece, flexCatalogEntry, fixedC0, fixedC1, { x: 300, y: 100.5 });
    expect(piece.customRadius).toBeUndefined();
    expect(piece.customAngle).toBeUndefined();
    expect(piece.connectors[0]!.x).toBeCloseTo(100, 0);
    expect(piece.connectors[0]!.y).toBeCloseTo(100, 0);
    expect(piece.connectors[1]!.x).toBeCloseTo(500, 0);
    expect(piece.connectors[1]!.y).toBeCloseTo(100, 0);

    // 5. Ziehe an den zusätzlichen Pivot-Punkten (t = 0.25 und t = 0.75)
    bendFlexTrackBetweenFixedEnds(piece, flexCatalogEntry, fixedC0, fixedC1, { x: 200, y: 140 }, undefined, 0.25);
    const pt25 = getFlexTrackPointAt(piece, flexCatalogEntry, 0.25);
    expect(pt25.y).toBeGreaterThan(100); // Wölbt sich nach oben in Zugrichtung
    expect(piece.connectors[0]!.x).toBeCloseTo(100, 0);
    expect(piece.connectors[1]!.x).toBeCloseTo(500, 0);

    bendFlexTrackBetweenFixedEnds(piece, flexCatalogEntry, fixedC0, fixedC1, { x: 400, y: 140 }, undefined, 0.75);
    const pt75 = getFlexTrackPointAt(piece, flexCatalogEntry, 0.75);
    expect(pt75.y).toBeGreaterThan(100); // Wölbt sich nach oben in Zugrichtung
    expect(piece.connectors[0]!.x).toBeCloseTo(100, 0);
    expect(piece.connectors[1]!.x).toBeCloseTo(500, 0);
  });

  it("snaps reliably near max length (880-900mm) without discarding snap", () => {
    // Zielgleis an x=990, connector index 0 bei x=990, angle=180
    const targetPiece: TrackPiece = {
      id: "track-target-890",
      catalogId: "24188",
      transform: { x: 990, y: 100, rotation: 0 },
      connectors: [
        { x: 990, y: 100, angle: 180 },
        { x: 1178.3, y: 100, angle: 0 },
      ],
    };

    // Flexgleis startet bei (100, 100) -> Abstand zum Ziel = 890 mm (in der 880-900mm Zone!)
    const flexPiece = createTestPiece();
    const allPieces = [flexPiece, targetPiece];

    // Drag flex end near target connector at (990, 100)
    const result = adjustFlexTrackOpenConnector(
      flexPiece,
      flexCatalogEntry,
      { x: 995, y: 102 },
      allPieces,
      undefined,
      25,
    );

    // Snap muss erfolgreich sein und darf NICHT durch x >= maxLen - 20 verworfen werden
    expect(result.snappedConnector).toBeDefined();
    expect(result.snappedConnector?.pieceId).toBe("track-target-890");
    expect(flexPiece.connectors[1]!.connectedTo?.pieceId).toBe("track-target-890");
    expect(targetPiece.connectors[0]!.connectedTo?.pieceId).toBe(flexPiece.id);

    // Wenn ins Leere gezogen (weg vom Ziel): sauber unhooken
    adjustFlexTrackOpenConnector(
      flexPiece,
      flexCatalogEntry,
      { x: 500, y: 100 },
      allPieces,
      undefined,
      25,
    );
    expect(flexPiece.connectors[1]!.connectedTo).toBeUndefined();
    expect(targetPiece.connectors[0]!.connectedTo).toBeUndefined();
  });

  it("supports S-curves by bending different pivot points in opposite directions", () => {
    const fixedC0 = { x: 100, y: 100 };
    const fixedC1 = { x: 700, y: 100 };
    const piece = createTestPiece({
      id: "piece-s-curve",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: [
        { x: 100, y: 100, angle: 180 },
        { x: 700, y: 100, angle: 0 },
      ],
    });

    // 1. Biege die erste Stelle (t = 0.25) nach oben (y = 150 -> Auslenkung +50mm)
    bendFlexTrackBetweenFixedEnds(
      piece,
      flexCatalogEntry,
      fixedC0,
      fixedC1,
      { x: 250, y: 150 },
      undefined,
      0.25,
    );

    expect(piece.customOffsets).toBeDefined();
    expect(piece.customOffsets![0]).toBeCloseTo(50, 0);

    // 2. Biege die zweite Stelle (t = 0.75) in die ENTGEGENGESETZTE Richtung nach unten (y = 50 -> Auslenkung -50mm)
    bendFlexTrackBetweenFixedEnds(
      piece,
      flexCatalogEntry,
      fixedC0,
      fixedC1,
      { x: 550, y: 50 },
      undefined,
      0.75,
    );

    expect(piece.customOffsets).toBeDefined();
    expect(piece.customOffsets![0]).toBeCloseTo(50, 0); // Erste Wölbung bleibt erhalten
    expect(piece.customOffsets![2]).toBeCloseTo(-50, 0); // Zweite Wölbung in Gegenrichtung!

    // 3. Beide Endpunkte bleiben unverrückbar fixiert
    expect(piece.connectors[0]!.x).toBeCloseTo(100, 0);
    expect(piece.connectors[0]!.y).toBeCloseTo(100, 0);
    expect(piece.connectors[1]!.x).toBeCloseTo(700, 0);
    expect(piece.connectors[1]!.y).toBeCloseTo(100, 0);

    // 4. Stützpunkte entlang des Gleises prüfen: S-Kurvenform
    const pt25 = getFlexTrackPointAt(piece, flexCatalogEntry, 0.25);
    const pt50 = getFlexTrackPointAt(piece, flexCatalogEntry, 0.5);
    const pt75 = getFlexTrackPointAt(piece, flexCatalogEntry, 0.75);

    expect(pt25.y).toBeGreaterThan(120); // Erste Wölbung oberhalb der Sehne
    expect(pt50.y).toBeCloseTo(100, 0);  // Nulldurchgang / Wendepunkt in der Mitte
    expect(pt75.y).toBeLessThan(80);     // Zweite Wölbung unterhalb der Sehne (Gegenrichtung)

    // Bogenlänge der S-Kurve muss größer sein als die direkte Sehnendistanz (600 mm)
    expect(piece.customLength).toBeGreaterThan(600);

    // 5. XML- und JSON-Serialisierung und Wiederherstellung
    const layout = createLayout();
    layout.pieces = [piece];
    const xml = toXML(layout);
    expect(xml).toContain('customOffsets="50.0,0.0,-50.0"');

    const json = toJSON(layout);
    const restoredLayout = fromJSON(json);
    const restoredPiece = restoredLayout.pieces.find((p) => p.id === "piece-s-curve");
    expect(restoredPiece).toBeDefined();
    expect(restoredPiece?.customOffsets).toEqual([50, 0, -50]);

    const restoredPt25 = getFlexTrackPointAt(restoredPiece!, flexCatalogEntry, 0.25);
    const restoredPt75 = getFlexTrackPointAt(restoredPiece!, flexCatalogEntry, 0.75);
    expect(restoredPt25.y).toBeGreaterThan(120);
    expect(restoredPt75.y).toBeLessThan(80);

    // 6. Zurücksetzen auf Originallänge & gerade
    resetFlexTrackToOriginal(restoredPiece!, flexCatalogEntry);
    expect(restoredPiece?.customOffsets).toBeUndefined();
    expect(restoredPiece?.customChordDist).toBeUndefined();
    expect(restoredPiece?.customLength).toBeUndefined();
    expect(restoredPiece?.customRadius).toBeUndefined();
    expect(restoredPiece?.customAngle).toBeUndefined();
  });
});
