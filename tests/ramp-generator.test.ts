import { describe, it, expect } from "vitest";
import type { TrackPiece } from "@/core/types";
import { maerklinCCatalog } from "@/catalog/catalog";
import {
  MIN_CLEARANCE_HEIGHT_BY_SCALE,
  getMinClearanceHeightForScale,
  getOpenConnectors,
  findStandardTrackEntries,
  getAvailableCurveRadii,
  generateRampPieces,
} from "@/core/ramp-generator";

describe("Ramp Generator & Clearance Heights", () => {
  describe("Minimum clearance heights across model railway scales", () => {
    it("returns correct NEM clearance heights with safety buffer for all major scales", () => {
      expect(getMinClearanceHeightForScale("Z")).toBe(40);
      expect(getMinClearanceHeightForScale("N")).toBe(50);
      expect(getMinClearanceHeightForScale("TT")).toBe(60);
      expect(getMinClearanceHeightForScale("H0")).toBe(80);
      expect(getMinClearanceHeightForScale("0")).toBe(120);
      expect(getMinClearanceHeightForScale("1")).toBe(170);
      expect(getMinClearanceHeightForScale("G")).toBe(220);
    });

    it("falls back to H0 clearance (80mm) for unknown or undefined scale", () => {
      expect(getMinClearanceHeightForScale(undefined)).toBe(80);
      expect(getMinClearanceHeightForScale("XYZ")).toBe(80);
    });
  });

  describe("Open connectors detection", () => {
    it("identifies both ends as open when track piece has no neighbors", () => {
      const piece: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const openConns = getOpenConnectors(piece, [piece]);
      expect(openConns.length).toBe(2);
      expect(openConns[0]).toBe(0);
      expect(openConns[1]).toBe(1);
    });

    it("identifies only the free end when one end is connected to another piece", () => {
      const p1: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
        ],
      };
      const p2: TrackPiece = {
        id: "p2",
        catalogId: "24188",
        transform: { x: 188.3, y: 0, rotation: 0 },
        connectors: [
          { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
          { x: 376.6, y: 0, angle: 0 },
        ],
      };

      const p1Open = getOpenConnectors(p1, [p1, p2]);
      expect(p1Open.length).toBe(1);
      expect(p1Open[0]).toBe(0);

      const p2Open = getOpenConnectors(p2, [p1, p2]);
      expect(p2Open.length).toBe(1);
      expect(p2Open[0]).toBe(1);
    });

    it("identifies 0 open connectors when all ends are connected", () => {
      const p1: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180, connectedTo: { pieceId: "p0", connectorIndex: 1 } },
          { x: 188.3, y: 0, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
        ],
      };
      const p0: TrackPiece = {
        id: "p0",
        catalogId: "24188",
        transform: { x: -188.3, y: 0, rotation: 0 },
        connectors: [
          { x: -188.3, y: 0, angle: 180 },
          { x: 0, y: 0, angle: 0, connectedTo: { pieceId: "p1", connectorIndex: 0 } },
        ],
      };
      const p2: TrackPiece = {
        id: "p2",
        catalogId: "24188",
        transform: { x: 188.3, y: 0, rotation: 0 },
        connectors: [
          { x: 188.3, y: 0, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
          { x: 376.6, y: 0, angle: 0 },
        ],
      };

      const p1Open = getOpenConnectors(p1, [p0, p1, p2]);
      expect(p1Open.length).toBe(0);
    });
  });

  describe("Standard track entries selection", () => {
    it("finds standard straight (24188) and curve (24130 / 24230) in Märklin C-Gleis catalog", () => {
      const { straight, curve } = findStandardTrackEntries(maerklinCCatalog);
      expect(straight).toBeDefined();
      expect(straight!.id).toBe("24188");
      expect(curve).toBeDefined();
      expect(curve!.angle).toBe(30);
    });
  });

  describe("Straight ramp generation", () => {
    it("generates a continuous straight ramp climbing 80mm with safe gradient <= 3%", () => {
      const anchor: TrackPiece = {
        id: "anchor",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 0, end: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const result = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1, // Exit towards positive X
        targetHeightMm: 80,
        shape: "straight",
        maxGradientPercent: 2.8,
        targetLayerId: "layer-2",
      });

      expect(result.pieces.length).toBeGreaterThanOrEqual(15); // ~2857mm required / 188.3mm ~ 16 pieces
      expect(result.gradientPercent).toBeLessThanOrEqual(2.8);
      expect(result.startHeightMm).toBe(0);
      expect(result.targetHeightMm).toBe(80);

      // Verify continuous snapping and elevation
      let prevX = 188.3;
      let prevElev = 0;
      for (let i = 0; i < result.pieces.length; i++) {
        const piece = result.pieces[i]!;
        expect(piece.layerId).toBe("layer-2");
        expect(piece.transform.rotation).toBe(0);
        expect(piece.connectors[0]!.x).toBeCloseTo(prevX, 0.1);
        expect(piece.elevation).toBeDefined();
        expect(piece.elevation!.start).toBeCloseTo(prevElev, 0.5);
        expect(piece.elevation!.end).toBeGreaterThan(piece.elevation!.start!);

        prevX = piece.connectors[1]!.x;
        prevElev = piece.elevation!.end!;
      }

      // Final piece must reach exactly target height 80mm
      const lastPiece = result.pieces[result.pieces.length - 1]!;
      expect(lastPiece.elevation!.end).toBeCloseTo(80, 0.5);
    });

    it("generates downward straight ramp when target height is below start height", () => {
      const anchor: TrackPiece = {
        id: "anchor-high",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 100, end: 100 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const result = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 20,
        shape: "straight",
        maxGradientPercent: 3.0,
      });

      expect(result.pieces.length).toBeGreaterThan(0);
      expect(result.targetHeightMm).toBe(20);
      expect(result.pieces[0]!.elevation!.start).toBeCloseTo(100, 0.5);
      expect(result.pieces[result.pieces.length - 1]!.elevation!.end).toBeCloseTo(20, 0.5);
    });
  });

  describe("Curved ramp generation", () => {
    it("generates 90 degree curved ramp turning left", () => {
      const anchor: TrackPiece = {
        id: "anchor",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 0, end: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const result = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 15, // 15mm rise over ~565mm curve length = ~2.65% safe gradient
        shape: "curved",
        direction: "left",
        curveAngleDeg: 90,
      });

      // 90 deg / 30 deg per piece = 3 pieces
      expect(result.pieces.length).toBe(3);
      expect(["24130", "24230", "24330", "24430", "24530"]).toContain(result.pieces[0]!.catalogId);
      expect(result.targetHeightMm).toBe(15);

      // Verify pieces turn left: exit connector points north (270 deg) with negative Y
      const lastPiece = result.pieces[result.pieces.length - 1]!;
      const exitConn = lastPiece.connectors[0]!; // Exit is connector 0 for left turns
      expect(exitConn.y).toBeLessThan(0); // Left turn heads north (-Y)
      expect(exitConn.angle).toBeCloseTo(270, 0.5);

      // For left turn, local connector 0 is the exit at target height 15mm
      expect(lastPiece.elevation!.start).toBeCloseTo(15, 0.5);
    });

    it("generates 180 degree curved ramp turning right", () => {
      const anchor: TrackPiece = {
        id: "anchor",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 0, end: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const result = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 30, // 30mm rise over ~1131mm curve length = ~2.65% safe gradient
        shape: "curved",
        direction: "right",
        curveAngleDeg: 180,
      });

      // 180 deg / 30 deg per piece = 6 pieces
      expect(result.pieces.length).toBe(6);
      const lastPiece = result.pieces[result.pieces.length - 1]!;

      // Verify pieces turn right: exit connector points west (180 deg) with positive Y (south)
      const exitConn = lastPiece.connectors[1]!; // Exit is connector 1 for right turns
      expect(exitConn.y).toBeGreaterThan(0); // Right turn heads south (+Y)
      expect(exitConn.angle).toBeCloseTo(180, 0.5);

      // For right turn, local connector 1 is the exit at target height 30mm
      expect(lastPiece.elevation!.end).toBeCloseTo(30, 0.5);
    });
  });

  describe("Radius Type Selection", () => {
    it("extracts all available curve radius types with human-readable labels from catalog", () => {
      const radii = getAvailableCurveRadii(maerklinCCatalog);
      expect(radii.length).toBeGreaterThanOrEqual(5);

      const r1 = radii.find((r) => Math.round(r.radiusMm) === 360);
      expect(r1).toBeDefined();
      expect(r1!.catalogId).toBe("24130");
      expect(r1!.label).toContain("R1");
      expect(r1!.label).toContain("360 mm");

      const r2 = radii.find((r) => Math.round(r.radiusMm) === 438);
      expect(r2).toBeDefined();
      expect(r2!.catalogId).toBe("24230");
      expect(r2!.label).toContain("R2");
      expect(r2!.label).toContain("437.5 mm");
    });

    it("generates ramp using specifically selected curve radius type (e.g. R2 24230 or R1 24130)", () => {
      const anchor: TrackPiece = {
        id: "anchor",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 0, end: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      // Explicitly select R2 (24230)
      const resultR2 = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 30,
        shape: "curved",
        direction: "right",
        curveAngleDeg: 180,
        selectedCurveCatalogId: "24230",
      });

      expect(resultR2.pieces.length).toBe(6);
      expect(resultR2.pieces.every((p) => p.catalogId === "24230")).toBe(true);

      // Explicitly select R1 (24130)
      const resultR1 = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 30,
        shape: "curved",
        direction: "left",
        curveAngleDeg: 180,
        selectedCurveCatalogId: "24130",
      });

      expect(resultR1.pieces.length).toBe(6);
      expect(resultR1.pieces.every((p) => p.catalogId === "24130")).toBe(true);
    });
  });

  describe("Helix (Wendel) ramp generation", () => {
    it("generates a multi-turn helix climbing at least minimum clearance per loop", () => {
      const anchor: TrackPiece = {
        id: "anchor",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 0, end: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const result = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 160, // 2 full loops of 80mm
        shape: "helix",
        direction: "right",
      });

      // 360 deg = 12 pieces (30 deg each). 2 loops = 24 pieces.
      expect(result.pieces.length).toBe(24);
      expect(result.pieces[0]!.elevation!.start).toBeCloseTo(0, 0.5);
      expect(result.pieces[23]!.elevation!.end).toBeCloseTo(160, 0.5);

      // Verify each turn (12 pieces) climbs by ~80mm
      expect(result.pieces[11]!.elevation!.end).toBeCloseTo(80, 0.5);
    });

    it("adjusts helix piece count and gradient dynamically when targetGradientPercent changes", () => {
      const anchor: TrackPiece = {
        id: "anchor",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 0, end: 0 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const res40 = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 80,
        shape: "helix",
        direction: "right",
        selectedCurveCatalogId: "24230", // R2 = 437.5mm
        targetGradientPercent: 4.0,
      });

      const res35 = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 80,
        shape: "helix",
        direction: "right",
        selectedCurveCatalogId: "24230",
        targetGradientPercent: 3.5,
      });

      const res28 = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 80,
        shape: "helix",
        direction: "right",
        selectedCurveCatalogId: "24230",
        targetGradientPercent: 2.8,
      });

      const res20 = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 80,
        shape: "helix",
        direction: "right",
        selectedCurveCatalogId: "24230",
        targetGradientPercent: 2.0,
      });

      // 4.0% target: <= 4.0%
      expect(res40.gradientPercent).toBeLessThanOrEqual(4.0);
      expect(res40.pieces.length).toBe(9);

      // 3.5% target: <= 3.5%
      expect(res35.gradientPercent).toBeLessThanOrEqual(3.5);
      expect(res35.pieces.length).toBe(10);

      // 2.8% target: <= 2.8%
      expect(res28.gradientPercent).toBeLessThanOrEqual(2.8);
      expect(res28.pieces.length).toBe(13);

      // 2.0% target: <= 2.0%
      expect(res20.gradientPercent).toBeLessThanOrEqual(2.0);
      expect(res20.pieces.length).toBe(18);

      // Verify that changing target gradient creates strictly more pieces as gradient gets gentler
      expect(res20.pieces.length).toBeGreaterThan(res28.pieces.length);
      expect(res28.pieces.length).toBeGreaterThan(res35.pieces.length);
      expect(res35.pieces.length).toBeGreaterThan(res40.pieces.length);
    });

    it("supports downward helix (Gleiswendel abwärts) with target gradient", () => {
      const anchor: TrackPiece = {
        id: "anchor",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        elevation: { start: 100, end: 100 },
        connectors: [
          { x: 0, y: 0, angle: 180 },
          { x: 188.3, y: 0, angle: 0 },
        ],
      };

      const result = generateRampPieces({
        catalog: maerklinCCatalog,
        anchorPiece: anchor,
        connectorIndex: 1,
        targetHeightMm: 20, // descending 80mm down
        shape: "helix",
        direction: "left",
        selectedCurveCatalogId: "24230",
        targetGradientPercent: 2.8,
      });

      expect(result.success).toBe(true);
      expect(result.gradientPercent).toBeLessThanOrEqual(2.8);
      expect(result.pieces.length).toBe(13);
      // For left turn, connector 1 is entry, connector 0 is exit
      expect(result.pieces[0]!.elevation!.end).toBeCloseTo(100, 0.5);
      expect(result.pieces[12]!.elevation!.start).toBeCloseTo(20, 0.5);
    });
  });
});
