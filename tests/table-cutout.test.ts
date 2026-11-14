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

    it("returns false for track 100mm under the table (-100 mm) and shadow station tracks (-80 mm)", () => {
      const piece100: TrackPiece = {
        id: "p-100",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: -100, end: -100 },
      };
      const piece80: TrackPiece = {
        id: "p-80",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: -80, end: -80 },
      };
      const deepRampSegment: TrackPiece = {
        id: "p-ramp-deep",
        catalogId: "24188",
        transform: { x: 500, y: 500, rotation: 0 },
        connectors: [],
        elevation: { start: -80, end: -100 },
      };
      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [piece100, piece80, deepRampSegment],
      };
      expect(isPiecePassingThroughTable(piece100, layout, mockCatalog).passesThrough).toBe(false);
      expect(isPiecePassingThroughTable(piece80, layout, mockCatalog).passesThrough).toBe(false);
      expect(isPiecePassingThroughTable(deepRampSegment, layout, mockCatalog).passesThrough).toBe(false);
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

    it("supports previewElevations for live ramp generation without saved piece elevation", () => {
      const p1: TrackPiece = {
        id: "ramp1",
        catalogId: "24188",
        transform: { x: 200, y: 200, rotation: 0 },
        connectors: [
          { x: 200, y: 200, angle: 180 },
          { x: 388, y: 200, angle: 0 },
        ],
        // Flat saved elevation
        elevation: { start: 0, end: 0 },
      };

      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [p1],
      };

      // Without preview: no cutouts because piece is flat
      const noCutouts = computeTableTrackCutouts(layout, mockCatalog, baseboard);
      expect(noCutouts).toHaveLength(0);

      // With previewElevations dipping down to -50mm: cutouts generated live!
      const previewMap = new Map<string, { start: number; end: number }>();
      previewMap.set("ramp1", { start: 0, end: -50 });

      const withPreview = computeTableTrackCutouts(
        layout,
        mockCatalog,
        baseboard,
        0,
        35,
        previewMap,
      );
      expect(withPreview).toHaveLength(1);
      expect(withPreview[0]!.pieceIds).toContain("ramp1");
      expect(withPreview[0]!.depthRange.min).toBe(-50);
    });

    it("renders clean 2D baseboard graphics with cutout aperture", async () => {
      const { createBaseboardGraphics } = await import("@/render/baseboard-sprite");
      const p1: TrackPiece = {
        id: "ramp1",
        catalogId: "24188",
        transform: { x: 200, y: 200, rotation: 0 },
        connectors: [
          { x: 200, y: 200, angle: 180 },
          { x: 388, y: 200, angle: 0 },
        ],
        elevation: { start: 0, end: -60 },
      };

      const layout: Layout = {
        name: "Test",
        scale: "H0",
        pieces: [p1],
      };

      const container = createBaseboardGraphics(baseboard, {
        layout,
        catalog: mockCatalog,
      });

      expect(container.children.length).toBeGreaterThan(2);
    });

    it("does not cast elevation drop shadow for descending or underground tracks", async () => {
      const { createTrackGraphics } = await import("@/render/track-sprite");
      const entry = mockCatalog.entries[0]!;

      // Descending track under table (-40 to -80 mm)
      const subterraneanPiece: TrackPiece = {
        id: "ramp-sub",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: -40, end: -80 },
      };

      const subContainer = createTrackGraphics(subterraneanPiece, entry, mockCatalog, {
        minLayoutElev: -80,
      });

      // Elevated bridge track (+20 to +40 mm)
      const elevatedPiece: TrackPiece = {
        id: "ramp-elev",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: 20, end: 40 },
      };

      const elevContainer = createTrackGraphics(elevatedPiece, entry, mockCatalog, {
        minLayoutElev: -80,
      });

      // Elevated track draws elevation shadow instructions to graphics context
      const subG = subContainer.children[0] as any;
      const elevG = elevContainer.children[0] as any;
      expect(elevG.context.instructions.length).toBeGreaterThan(subG.context.instructions.length);
    });
  });

  describe("isPointInsideCutout", () => {
    it("computes cutouts for Alpenland H0 demo layout", async () => {
      const { createDemoLayout } = await import("@/editor/demo-tracks");
      const { maerklinCCatalog } = await import("@/catalog/catalog");
      const { getBaseboardPolygon, getBaseboardBounds } = await import("@/core/layout");
      const THREE = await import("three");

      const layout = createDemoLayout();
      const catalog = maerklinCCatalog;
      const baseboard = layout.baseboard!;
      const cutouts = computeTableTrackCutouts(layout, catalog, baseboard);
      expect(cutouts.length).toBeGreaterThan(0);
      console.log("Cutout points count:", cutouts[0]!.points.length);
      console.log("First 5 points:", cutouts[0]!.points.slice(0, 5));
      console.log("Points at turnaround (80-84):", cutouts[0]!.points.slice(80, 84));
      console.log("Baseboard bounds:", getBaseboardBounds(baseboard));

      const polygon = getBaseboardPolygon(baseboard);
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

      for (const cutout of cutouts) {
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

      const extrudeSettings = {
        depth: 35,
        bevelEnabled: true,
        bevelSegments: 3,
        steps: 1,
        bevelSize: 2.5,
        bevelThickness: 2.5,
      };

      const geometry = new THREE.ExtrudeGeometry(shape, extrudeSettings);
      console.log("Vertices:", geometry.attributes.position?.count);
    });
  });
});
