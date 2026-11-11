import { describe, it, expect } from "vitest";
import type { BaseboardConfig, Layout, TrackPiece } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import {
  getTrackClearanceWidth,
  isPiecePassingThroughTable,
  getPieceCenterlinePoints,
  groupConnectedPieces,
  computeTableTrackCutouts,
  isPointInsideCutout,
} from "@/core/table-cutout";

const mockCatalog: Catalog = {
  id: "test-catalog",
  name: "Test Catalog",
  manufacturer: "Märklin",
  system: "3-rail",
  gauge: 16.5,
  scale: "H0",
  entries: [
    {
      id: "24188",
      name: "Straight 188.3mm",
      category: "Geraden",
      kind: "straight",
      gauge: 16.5,
      length: 188.3,
      description: "",
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    },
    {
      id: "24130",
      name: "Curve R1 30 deg",
      category: "Bögen",
      kind: "curve",
      gauge: 16.5,
      radius: 360,
      angle: 30,
      description: "",
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 360 * Math.sin((30 * Math.PI) / 180), y: 360 * (1 - Math.cos((30 * Math.PI) / 180)), angle: 30 },
      ],
    },
    {
      id: "road-straight",
      name: "Road Straight",
      category: "Straßen",
      kind: "straight",
      isRoad: true,
      roadWidth: 60,
      length: 200,
      description: "",
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 200, y: 0, angle: 0 },
      ],
    },
  ],
};

const baseboard: BaseboardConfig = {
  enabled: true,
  shape: "rect",
  width: 2000,
  depth: 1000,
  positionMode: "center",
  style: "wood",
};

describe("table-cutout", () => {
  describe("getTrackClearanceWidth", () => {
    it("computes clearance width for H0 railway track", () => {
      const entry = mockCatalog.entries[0];
      const w = getTrackClearanceWidth(entry);
      expect(w).toBe(53); // Math.round(16.5 * 3.2)
    });

    it("computes clearance width for road elements", () => {
      const entry = mockCatalog.entries[2];
      const w = getTrackClearanceWidth(entry);
      expect(w).toBe(76); // 60 + 16
    });

    it("returns sensible default when entry is undefined", () => {
      expect(getTrackClearanceWidth(undefined)).toBe(54);
    });
  });

  describe("isPiecePassingThroughTable", () => {
    it("returns false for flat track on table level (0 mm)", () => {
      const piece: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
      };
      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [piece],
      };
      const res = isPiecePassingThroughTable(piece, layout, mockCatalog);
      expect(res.passesThrough).toBe(false);
    });

    it("returns false for elevated track above table level (+60 mm)", () => {
      const piece: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: 60, end: 60 },
      };
      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [piece],
      };
      const res = isPiecePassingThroughTable(piece, layout, mockCatalog);
      expect(res.passesThrough).toBe(false);
    });

    it("returns true for descending ramp piece (0 to -40 mm)", () => {
      const piece: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: 0, end: -40 },
      };
      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [piece],
      };
      const res = isPiecePassingThroughTable(piece, layout, mockCatalog);
      expect(res.passesThrough).toBe(true);
      expect(res.minElev).toBe(-40);
      expect(res.maxElev).toBe(0);
    });

    it("returns false for deep shadow station track (-160 mm, below clearance zone)", () => {
      const piece: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: -160, end: -160 },
      };
      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [piece],
      };
      // Clearance is 65mm in H0 + 35mm table thickness = 100mm collision zone
      const res = isPiecePassingThroughTable(piece, layout, mockCatalog);
      expect(res.passesThrough).toBe(false);
    });

    it("supports custom intermediate deckElevation (e.g. +100 mm deck plate)", () => {
      const piece: TrackPiece = {
        id: "ramp-mountain",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: 120, end: 70 },
      };
      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [piece],
      };
      // Deck at +100 mm with 12 mm thickness
      const res = isPiecePassingThroughTable(piece, layout, mockCatalog, undefined, 100, 12);
      expect(res.passesThrough).toBe(true);
      expect(res.minElev).toBe(70);
      expect(res.maxElev).toBe(120);

      // Track flat on deck at +100 mm: should not cut out
      const flatPiece: TrackPiece = {
        id: "flat-on-deck",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: 100, end: 100 },
      };
      const flatRes = isPiecePassingThroughTable(flatPiece, layout, mockCatalog, undefined, 100, 12);
      expect(flatRes.passesThrough).toBe(false);
    });
  });

  describe("getPieceCenterlinePoints", () => {
    it("samples points along a straight track", () => {
      const entry = mockCatalog.entries[0]!;
      const piece: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 100, y: 200, rotation: 0 },
        connectors: [
          { x: 100, y: 200, angle: 180 },
          { x: 288.3, y: 200, angle: 0 },
        ],
      };
      const pts = getPieceCenterlinePoints(piece, entry, 0, 1);
      expect(pts.length).toBeGreaterThan(2);
      expect(pts[0]!.x).toBeCloseTo(100);
      expect(pts[pts.length - 1]!.x).toBeCloseTo(288.3);
    });

    it("samples points along a curved track", () => {
      const entry = mockCatalog.entries[1]!;
      const piece: TrackPiece = {
        id: "p2",
        catalogId: "24130",
        transform: { x: 300, y: 300, rotation: 0 },
        connectors: [
          { x: 300, y: 300, angle: 180 },
          { x: 480, y: 348, angle: 30 },
        ],
      };
      const pts = getPieceCenterlinePoints(piece, entry, 0, 1);
      expect(pts.length).toBe(9); // steps = 8 => 9 points
      expect(pts[0]!.x).toBeCloseTo(300);
      expect(pts[0]!.y).toBeCloseTo(300);
    });
  });

  describe("groupConnectedPieces", () => {
    it("groups a chain of contiguous pieces into a sequence", () => {
      const p1: TrackPiece = {
        id: "p1",
        catalogId: "24188",
        transform: { x: 100, y: 100, rotation: 0 },
        connectors: [
          { x: 100, y: 100, angle: 180 },
          { x: 288, y: 100, angle: 0 },
        ],
      };
      const p2: TrackPiece = {
        id: "p2",
        catalogId: "24188",
        transform: { x: 288, y: 100, rotation: 0 },
        connectors: [
          { x: 288, y: 100, angle: 180 },
          { x: 476, y: 100, angle: 0 },
        ],
      };

      const chains = groupConnectedPieces([p1, p2]);
      expect(chains.length).toBe(1);
      expect(chains[0]!.length).toBe(2);
      expect(chains[0]![0]!.piece.id).toBe("p1");
      expect(chains[0]![1]!.piece.id).toBe("p2");
    });
  });

  describe("computeTableTrackCutouts", () => {
    it("returns empty array when baseboard is disabled", () => {
      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [
          {
            id: "p1",
            catalogId: "24188",
            transform: { x: 200, y: 200, rotation: 0 },
            connectors: [
              { x: 200, y: 200, angle: 180 },
              { x: 388, y: 200, angle: 0 },
            ],
            elevation: { start: 0, end: -50 },
          },
        ],
      };
      const cutouts = computeTableTrackCutouts(layout, mockCatalog, { ...baseboard, enabled: false });
      expect(cutouts).toHaveLength(0);
    });

    it("creates cutout polygon for penetrating ramp tracks", () => {
      const p1: TrackPiece = {
        id: "ramp1",
        catalogId: "24188",
        transform: { x: 200, y: 200, rotation: 0 },
        connectors: [
          { x: 200, y: 200, angle: 180 },
          { x: 388, y: 200, angle: 0 },
        ],
        elevation: { start: 0, end: -40 },
      };
      const p2: TrackPiece = {
        id: "ramp2",
        catalogId: "24188",
        transform: { x: 388, y: 200, rotation: 0 },
        connectors: [
          { x: 388, y: 200, angle: 180 },
          { x: 576, y: 200, angle: 0 },
        ],
        elevation: { start: -40, end: -80 },
      };

      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [p1, p2],
      };

      const cutouts = computeTableTrackCutouts(layout, mockCatalog, baseboard);
      expect(cutouts).toHaveLength(1);
      const cutout = cutouts[0]!;
      expect(cutout.pieceIds).toEqual(["ramp1", "ramp2"]);
      expect(cutout.depthRange.min).toBe(-80);
      expect(cutout.depthRange.max).toBe(0);
      expect(cutout.widthMm).toBe(53);
      expect(cutout.points.length).toBeGreaterThan(4);

      // Verify all points are inside baseboard bounds
      for (const pt of cutout.points) {
        expect(pt.x).toBeGreaterThanOrEqual(1.5);
        expect(pt.x).toBeLessThanOrEqual(2000 - 1.5);
        expect(pt.y).toBeGreaterThanOrEqual(1.5);
        expect(pt.y).toBeLessThanOrEqual(1000 - 1.5);
      }
    });
  });

  describe("isPointInsideCutout", () => {
    it("detects points inside cutout or within safety buffer", () => {
      const cutout = {
        points: [
          { x: 100, y: 100 },
          { x: 300, y: 100 },
          { x: 300, y: 160 },
          { x: 100, y: 160 },
        ],
        pieceIds: ["p1"],
        depthRange: { min: -50, max: 0 },
        widthMm: 60,
      };

      // Inside center
      expect(isPointInsideCutout({ x: 200, y: 130 }, [cutout])).toBe(true);

      // In safety margin (distance ~10 mm to edge)
      expect(isPointInsideCutout({ x: 200, y: 90 }, [cutout], 15)).toBe(true);

      // Far away outside
      expect(isPointInsideCutout({ x: 600, y: 600 }, [cutout], 15)).toBe(false);
    });
  });
});
