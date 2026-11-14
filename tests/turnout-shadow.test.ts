import { describe, it, expect } from "vitest";
import type { CatalogEntry } from "@/catalog/catalog";
import type { TrackPiece } from "@/core/types";
import { createTrackGraphics } from "@/render/track-sprite";

describe("Turnout Elevation Shadow Rendering", () => {
  const curvedTurnoutLeft: CatalogEntry = {
    id: "24671",
    name: "Bogenweiche links",
    kind: "curved-turnout-left",
    category: "Weichen",
    description: "Bogenweiche links R1 360 mm / 30°",
    radius: 360,
    angle: 30,
    branchRadius: 360,
    branchAngle: 30,
    gauge: 16.5,
  };

  const curvedTurnoutRight: CatalogEntry = {
    id: "24672",
    name: "Bogenweiche rechts",
    kind: "curved-turnout-right",
    category: "Weichen",
    description: "Bogenweiche rechts R1 360 mm / 30°",
    radius: 360,
    angle: 30,
    branchRadius: 360,
    branchAngle: 30,
    gauge: 16.5,
  };

  const standardTurnoutLeft: CatalogEntry = {
    id: "24611",
    name: "Weiche links",
    kind: "turnout-left",
    category: "Weichen",
    description: "Standardweiche links",
    length: 188.3,
    branchRadius: 437.5,
    branchAngle: 24.3,
    gauge: 16.5,
  };

  const standardTurnoutRight: CatalogEntry = {
    id: "24612",
    name: "Weiche rechts",
    kind: "turnout-right",
    category: "Weichen",
    description: "Standardweiche rechts",
    length: 188.3,
    branchRadius: 437.5,
    branchAngle: 24.3,
    gauge: 16.5,
  };

  const threeWayTurnout: CatalogEntry = {
    id: "24630",
    name: "Dreiwegweiche",
    kind: "three-way",
    category: "Weichen",
    description: "Dreiwegweiche symmetrisch",
    length: 188.3,
    branchRadius: 437.5,
    branchAngle: 24.3,
    gauge: 16.5,
  };

  const crossingEntry: CatalogEntry = {
    id: "24640",
    name: "Kreuzung 24,3°",
    kind: "crossing",
    category: "Kreuzungen",
    description: "Kreuzung 188,3 mm",
    length: 188.3,
    angle: 24.3,
    gauge: 16.5,
  };

  const doubleSlipEntry: CatalogEntry = {
    id: "24624",
    name: "Doppelkreuzungsweiche",
    kind: "double-slip",
    category: "Weichen",
    description: "DKW 188,3 mm",
    length: 188.3,
    angle: 24.3,
    branchRadius: 437.5,
    branchAngle: 24.3,
    gauge: 16.5,
  };

  const roadTurnoutLeft: CatalogEntry = {
    id: "ST-TJ-90",
    name: "T-Kreuzung 90°",
    kind: "turnout-left",
    category: "Weichen",
    description: "T-Einmündung",
    length: 200,
    branchLength: 100,
    branchAngle: 90,
    isRoad: true,
    roadWidth: 60,
  };

  const roadTurnoutRight: CatalogEntry = {
    id: "ST-TJ-90-R",
    name: "T-Kreuzung rechts",
    kind: "turnout-right",
    category: "Weichen",
    description: "T-Einmündung gespiegelt",
    length: 200,
    branchLength: 100,
    branchAngle: 90,
    isRoad: true,
    roadWidth: 60,
  };

  it("renders elevation shadow for elevated curved-turnout-left", () => {
    const flatPiece: TrackPiece = {
      id: "ctl-flat",
      catalogId: curvedTurnoutLeft.id,
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      elevation: { start: 0, end: 0 },
    };
    const elevPiece: TrackPiece = {
      id: "ctl-elev",
      catalogId: curvedTurnoutLeft.id,
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      elevation: { start: 30, end: 30 },
    };

    const flatCont = createTrackGraphics(flatPiece, curvedTurnoutLeft);
    const elevCont = createTrackGraphics(elevPiece, curvedTurnoutLeft);

    const flatG = flatCont.children[0] as any;
    const elevG = elevCont.children[0] as any;

    expect(elevG.context.instructions.length).toBeGreaterThan(flatG.context.instructions.length);
  });

  it("renders elevation shadow for elevated curved-turnout-right", () => {
    const flatPiece: TrackPiece = {
      id: "ctr-flat",
      catalogId: curvedTurnoutRight.id,
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      elevation: { start: 0, end: 0 },
    };
    const elevPiece: TrackPiece = {
      id: "ctr-elev",
      catalogId: curvedTurnoutRight.id,
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      elevation: { start: 30, end: 30 },
    };

    const flatCont = createTrackGraphics(flatPiece, curvedTurnoutRight);
    const elevCont = createTrackGraphics(elevPiece, curvedTurnoutRight);

    const flatG = flatCont.children[0] as any;
    const elevG = elevCont.children[0] as any;

    expect(elevG.context.instructions.length).toBeGreaterThan(flatG.context.instructions.length);
  });

  it("renders elevation shadow for standard turnout-left and turnout-right", () => {
    for (const entry of [standardTurnoutLeft, standardTurnoutRight]) {
      const flatPiece: TrackPiece = {
        id: "t-flat",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: 0, end: 0 },
      };
      const elevPiece: TrackPiece = {
        id: "t-elev",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: 25, end: 25 },
      };

      const flatCont = createTrackGraphics(flatPiece, entry);
      const elevCont = createTrackGraphics(elevPiece, entry);

      const flatG = flatCont.children[0] as any;
      const elevG = elevCont.children[0] as any;

      expect(elevG.context.instructions.length).toBeGreaterThan(flatG.context.instructions.length);
    }
  });

  it("renders elevation shadow for three-way, crossing, and double-slip tracks", () => {
    for (const entry of [threeWayTurnout, crossingEntry, doubleSlipEntry]) {
      const flatPiece: TrackPiece = {
        id: "m-flat",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: 0, end: 0 },
      };
      const elevPiece: TrackPiece = {
        id: "m-elev",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: 35, end: 35 },
      };

      const flatCont = createTrackGraphics(flatPiece, entry);
      const elevCont = createTrackGraphics(elevPiece, entry);

      const flatG = flatCont.children[0] as any;
      const elevG = elevCont.children[0] as any;

      expect(elevG.context.instructions.length).toBeGreaterThan(flatG.context.instructions.length);
    }
  });

  it("renders road elevation shadow for elevated road turnouts", () => {
    for (const entry of [roadTurnoutLeft, roadTurnoutRight]) {
      const flatPiece: TrackPiece = {
        id: "r-flat",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: 0, end: 0 },
      };
      const elevPiece: TrackPiece = {
        id: "r-elev",
        catalogId: entry.id,
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [],
        elevation: { start: 40, end: 40 },
      };

      const flatCont = createTrackGraphics(flatPiece, entry);
      const elevCont = createTrackGraphics(elevPiece, entry);

      const flatG = flatCont.children[0] as any;
      const elevG = elevCont.children[0] as any;

      expect(elevG.context.instructions.length).toBeGreaterThan(flatG.context.instructions.length);
    }
  });
});
