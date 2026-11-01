import { describe, it, expect } from "vitest";
import type { Layout, TrackPiece } from "@/core/types";
import { maerklinCCatalog } from "@/catalog/catalog";
import {
  getTrackPieceLength,
  evaluateRampProfile,
  sortPiecesIntoChain,
  calculateRamp,
  calculateEndHeightFromMaxGradient,
  calculateRequiredLengthForGradient,
} from "@/core/ramp";
import { MovePiecesToLayerCommand } from "@/editor/commands";

describe("Ramp & Elevation Functionality", () => {
  it("computes track piece length accurately for straights and curves", () => {
    const straightEntry = maerklinCCatalog.entries.find((e) => e.id === "24188");
    expect(straightEntry).toBeDefined();
    expect(getTrackPieceLength(straightEntry)).toBeCloseTo(188.3, 1);

    const curveEntry = maerklinCCatalog.entries.find((e) => e.id === "24130"); // R1 = 360mm, 30 deg
    expect(curveEntry).toBeDefined();
    const expectedCurveLen = (2 * Math.PI * 360 * 30) / 360; // ~188.495 mm
    expect(getTrackPieceLength(curveEntry)).toBeCloseTo(expectedCurveLen, 1);
  });

  it("evaluates ramp profiles (linear vs. cubic easement)", () => {
    // Linear
    expect(evaluateRampProfile(0, "linear")).toBe(0);
    expect(evaluateRampProfile(0.5, "linear")).toBe(0.5);
    expect(evaluateRampProfile(1, "linear")).toBe(1);

    // Eased cubic smoothstep: 3u^2 - 2u^3
    expect(evaluateRampProfile(0, "eased")).toBe(0);
    expect(evaluateRampProfile(0.5, "eased")).toBe(0.5);
    expect(evaluateRampProfile(1, "eased")).toBe(1);

    // Ausrundung: gradual slope at start and end
    // At u = 0.2: linear is 0.2, eased is 3(0.04) - 2(0.008) = 0.104 (flatter entry)
    expect(evaluateRampProfile(0.2, "eased")).toBeLessThan(0.2);
    // At u = 0.8: linear is 0.8, eased is 3(0.64) - 2(0.512) = 0.896 (flatter exit)
    expect(evaluateRampProfile(0.8, "eased")).toBeGreaterThan(0.8);
  });

  it("sorts multiple connected pieces into a continuous chain in topological order", () => {
    // 3 straight tracks connected along X axis: p1 (0..188.3), p2 (188.3..376.6), p3 (376.6..564.9)
    const p1: TrackPiece = {
      id: "p1",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };
    const p2: TrackPiece = {
      id: "p2",
      catalogId: "24188",
      transform: { x: 188.3, y: 0, rotation: 0 },
      connectors: [
        { x: 188.3, y: 0, angle: 180 },
        { x: 376.6, y: 0, angle: 0 },
      ],
    };
    const p3: TrackPiece = {
      id: "p3",
      catalogId: "24188",
      transform: { x: 376.6, y: 0, rotation: 0 },
      connectors: [
        { x: 376.6, y: 0, angle: 180 },
        { x: 564.9, y: 0, angle: 0 },
      ],
    };

    // Passed in shuffled order: [p3, p1, p2]
    const { ordered, isContinuous } = sortPiecesIntoChain([p3, p1, p2], maerklinCCatalog, false);
    expect(isContinuous).toBe(true);
    expect(ordered.length).toBe(3);
    // Should be ordered from endpoint p1 to endpoint p3
    expect(ordered[0]!.piece.id).toBe("p1");
    expect(ordered[1]!.piece.id).toBe("p2");
    expect(ordered[2]!.piece.id).toBe("p3");
  });

  it("calculates continuous ramp heights with Dreisatz (linear proportional length)", () => {
    // 4 straight pieces of length 188.3 mm, total 753.2 mm, rising 80 mm from 0 to 80 mm
    const pieces: TrackPiece[] = [
      {
        id: "t1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
      },
      {
        id: "t2",
        catalogId: "24188",
        transform: { x: 188.3, y: 0, rotation: 0 },
        connectors: [{ x: 188.3, y: 0, angle: 180 }, { x: 376.6, y: 0, angle: 0 }],
      },
      {
        id: "t3",
        catalogId: "24188",
        transform: { x: 376.6, y: 0, rotation: 0 },
        connectors: [{ x: 376.6, y: 0, angle: 180 }, { x: 564.9, y: 0, angle: 0 }],
      },
      {
        id: "t4",
        catalogId: "24188",
        transform: { x: 564.9, y: 0, rotation: 0 },
        connectors: [{ x: 564.9, y: 0, angle: 180 }, { x: 753.2, y: 0, angle: 0 }],
      },
    ];

    const result = calculateRamp({
      pieces,
      catalog: maerklinCCatalog,
      startHeightMm: 0,
      endHeightMm: 80,
      layerBaseHeightMm: 0,
      profile: "linear",
    });

    expect(result.isContinuous).toBe(true);
    expect(result.totalLengthMm).toBeCloseTo(753.2, 1);
    expect(result.totalRiseMm).toBe(80);

    // Each piece must rise by exactly 20 mm (Dreisatz: 80 * 188.3 / 753.2 = 20)
    const t1Elev = result.pieceElevations.get("t1");
    const t2Elev = result.pieceElevations.get("t2");
    const t3Elev = result.pieceElevations.get("t3");
    const t4Elev = result.pieceElevations.get("t4");

    expect(t1Elev?.start).toBeCloseTo(0, 1);
    expect(t1Elev?.end).toBeCloseTo(20, 1);

    expect(t2Elev?.start).toBeCloseTo(20, 1);
    expect(t2Elev?.end).toBeCloseTo(40, 1);

    expect(t3Elev?.start).toBeCloseTo(40, 1);
    expect(t3Elev?.end).toBeCloseTo(60, 1);

    expect(t4Elev?.start).toBeCloseTo(60, 1);
    expect(t4Elev?.end).toBeCloseTo(80, 1);
  });

  it("calculates eased S-curve ramp profile with gentle entry and exit (1% -> 2% -> 3% -> 2% -> 1%)", () => {
    // 5 pieces of 200 mm (total 1000 mm), rising 20 mm
    const pieces: TrackPiece[] = [];
    for (let i = 0; i < 5; i++) {
      pieces.push({
        id: `piece-${i}`,
        catalogId: "24188", // using standard 188.3mm pieces
        transform: { x: i * 188.3, y: 0, rotation: 0 },
        connectors: [
          { x: i * 188.3, y: 0, angle: 180 },
          { x: (i + 1) * 188.3, y: 0, angle: 0 },
        ],
      });
    }

    const linearRes = calculateRamp({
      pieces,
      catalog: maerklinCCatalog,
      startHeightMm: 0,
      endHeightMm: 50,
      layerBaseHeightMm: 0,
      profile: "linear",
    });

    const easedRes = calculateRamp({
      pieces,
      catalog: maerklinCCatalog,
      startHeightMm: 0,
      endHeightMm: 50,
      layerBaseHeightMm: 0,
      profile: "eased",
    });

    // In linear mode, all 5 pieces have identical gradient
    expect(linearRes.chain[0]!.gradientPercent).toBe(linearRes.chain[2]!.gradientPercent);

    // In eased mode, first and last pieces have smaller gradient (gentle entry and exit)
    const firstGrad = easedRes.chain[0]!.gradientPercent;
    const midGrad = easedRes.chain[2]!.gradientPercent;
    const lastGrad = easedRes.chain[4]!.gradientPercent;

    expect(firstGrad).toBeLessThan(midGrad);
    expect(lastGrad).toBeLessThan(midGrad);
    expect(firstGrad).toBeCloseTo(lastGrad, 0.5); // Symmetric S-curve
  });

  it("reverses ramp direction when requested", () => {
    const pieces: TrackPiece[] = [
      {
        id: "a",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
      },
      {
        id: "b",
        catalogId: "24188",
        transform: { x: 188.3, y: 0, rotation: 0 },
        connectors: [{ x: 188.3, y: 0, angle: 180 }, { x: 376.6, y: 0, angle: 0 }],
      },
    ];

    const normal = calculateRamp({
      pieces,
      catalog: maerklinCCatalog,
      startHeightMm: 0,
      endHeightMm: 40,
      layerBaseHeightMm: 0,
      profile: "linear",
      reversed: false,
    });

    const reversed = calculateRamp({
      pieces,
      catalog: maerklinCCatalog,
      startHeightMm: 0,
      endHeightMm: 40,
      layerBaseHeightMm: 0,
      profile: "linear",
      reversed: true,
    });

    // When reversed, piece "b" is at the start (conn 1 at 0, conn 0 at 20)
    // and piece "a" is at the end (conn 1 at 20, conn 0 at 40)
    expect(normal.pieceElevations.get("a")?.start).toBeCloseTo(0, 1);
    expect(normal.pieceElevations.get("a")?.end).toBeCloseTo(20, 1);
    expect(normal.pieceElevations.get("b")?.start).toBeCloseTo(20, 1);
    expect(normal.pieceElevations.get("b")?.end).toBeCloseTo(40, 1);

    expect(reversed.pieceElevations.get("b")?.end).toBeCloseTo(0, 1);
    expect(reversed.pieceElevations.get("b")?.start).toBeCloseTo(20, 1);
    expect(reversed.pieceElevations.get("a")?.end).toBeCloseTo(20, 1);
    expect(reversed.pieceElevations.get("a")?.start).toBeCloseTo(40, 1);
  });

  it("MovePiecesToLayerCommand applies pieceElevation Map and supports undo/redo", () => {
    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      layers: [
        { id: "layer-1", name: "Level 1", heightMm: 0, visible: true, locked: false },
        { id: "layer-2", name: "Level 2", heightMm: 50, visible: true, locked: false },
      ],
      activeLayerId: "layer-1",
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
          layerId: "layer-1",
        },
        {
          id: "p2",
          catalogId: "24188",
          transform: { x: 188.3, y: 0, rotation: 0 },
          connectors: [{ x: 188.3, y: 0, angle: 180 }, { x: 376.6, y: 0, angle: 0 }],
          layerId: "layer-1",
        },
      ],
    };

    const elevMap = new Map<string, { start: number; end: number }>();
    elevMap.set("p1", { start: 0, end: 20 });
    elevMap.set("p2", { start: 20, end: 40 });

    const cmd = new MovePiecesToLayerCommand(["p1", "p2"], "layer-2", elevMap);
    cmd.do(layout);

    expect(layout.pieces[0]!.layerId).toBe("layer-2");
    expect(layout.pieces[0]!.elevation).toEqual({ start: 0, end: 20 });
    expect(layout.pieces[1]!.layerId).toBe("layer-2");
    expect(layout.pieces[1]!.elevation).toEqual({ start: 20, end: 40 });

    // Test Undo
    cmd.undo(layout);
    expect(layout.pieces[0]!.layerId).toBe("layer-1");
    expect(layout.pieces[0]!.elevation).toBeUndefined();
    expect(layout.pieces[1]!.layerId).toBe("layer-1");
    expect(layout.pieces[1]!.elevation).toBeUndefined();
  });

  describe("Target Max Steepness Calculation", () => {
    it("calculates allowable end height for linear ramp", () => {
      // 1000 mm track length, start at 0 mm, target max 3.0%
      // Rise = 1000 * 0.03 = 30 mm
      const endH = calculateEndHeightFromMaxGradient({
        startHeightMm: 0,
        totalLengthMm: 1000,
        targetMaxGradientPercent: 3.0,
        profile: "linear",
      });
      expect(endH).toBeCloseTo(30, 1);

      // Downwards direction with start at 50 mm
      const endHDown = calculateEndHeightFromMaxGradient({
        startHeightMm: 50,
        totalLengthMm: 1000,
        targetMaxGradientPercent: 2.5,
        profile: "linear",
        direction: -1,
      });
      expect(endHDown).toBeCloseTo(25, 1);
    });

    it("calculates allowable end height for eased ramp with 1.5x peak factor", () => {
      // In eased ramp, peak gradient is 1.5 * average gradient.
      // So allowable average gradient = 3.0% / 1.5 = 2.0%.
      // For 1000 mm track length, max rise = 1000 * 0.02 = 20 mm.
      const endHEased = calculateEndHeightFromMaxGradient({
        startHeightMm: 0,
        totalLengthMm: 1000,
        targetMaxGradientPercent: 3.0,
        profile: "eased",
      });
      expect(endHEased).toBeCloseTo(20, 1);
    });

    it("calculates required length for a given rise and target max steepness", () => {
      // Rise = 60 mm, target max 3.0% in linear mode
      // Required length = 60 / 0.03 = 2000 mm
      const reqLenLinear = calculateRequiredLengthForGradient({
        totalRiseMm: 60,
        targetMaxGradientPercent: 3.0,
        profile: "linear",
      });
      expect(reqLenLinear).toBeCloseTo(2000, 1);

      // In eased mode: target max 3.0% -> average is 2.0% -> required length = 60 / 0.02 = 3000 mm
      const reqLenEased = calculateRequiredLengthForGradient({
        totalRiseMm: 60,
        targetMaxGradientPercent: 3.0,
        profile: "eased",
      });
      expect(reqLenEased).toBeCloseTo(3000, 1);
    });

    it("evaluates exceedsTargetMax and recommendedEndHeightMm in calculateRamp", () => {
      // 4 straight pieces = ~753.2 mm
      const pieces: TrackPiece[] = [
        { id: "s1", catalogId: "24188", transform: { x: 0, y: 0, rotation: 0 }, connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }] },
        { id: "s2", catalogId: "24188", transform: { x: 188.3, y: 0, rotation: 0 }, connectors: [{ x: 188.3, y: 0, angle: 180 }, { x: 376.6, y: 0, angle: 0 }] },
        { id: "s3", catalogId: "24188", transform: { x: 376.6, y: 0, rotation: 0 }, connectors: [{ x: 376.6, y: 0, angle: 180 }, { x: 564.9, y: 0, angle: 0 }] },
        { id: "s4", catalogId: "24188", transform: { x: 564.9, y: 0, rotation: 0 }, connectors: [{ x: 564.9, y: 0, angle: 180 }, { x: 753.2, y: 0, angle: 0 }] },
      ];

      // End height 50 mm from 0 mm -> gradient ~ 50 / 753.2 * 100 = 6.64%
      const resExceeding = calculateRamp({
        pieces,
        catalog: maerklinCCatalog,
        startHeightMm: 0,
        endHeightMm: 50,
        layerBaseHeightMm: 0,
        profile: "linear",
        targetMaxGradientPercent: 2.5,
      });

      expect(resExceeding.exceedsTargetMax).toBe(true);
      expect(resExceeding.targetMaxGradientPercent).toBe(2.5);
      // recommended end height for 2.5% over 753.2 mm = 753.2 * 0.025 = 18.8 mm
      expect(resExceeding.recommendedEndHeightMm).toBeCloseTo(18.8, 1);
      expect(resExceeding.requiredLengthMm).toBeCloseTo(50 / 0.025, 0); // ~2000 mm

      // End height 15 mm -> gradient ~ 15 / 753.2 * 100 = 1.99% <= 2.5%
      const resOk = calculateRamp({
        pieces,
        catalog: maerklinCCatalog,
        startHeightMm: 0,
        endHeightMm: 15,
        layerBaseHeightMm: 0,
        profile: "linear",
        targetMaxGradientPercent: 2.5,
      });

      expect(resOk.exceedsTargetMax).toBe(false);
    });
  });
});
