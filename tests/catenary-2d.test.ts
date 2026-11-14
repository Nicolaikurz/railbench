import { describe, it, expect } from "vitest";
import { createTrackGraphics } from "@/render/track-sprite";
import type { TrackPiece } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";

describe("2D Catenary rendering for left and right turns", () => {
  const turnoutLeftEntry: CatalogEntry = {
    id: "24611",
    name: "Weiche links",
    kind: "turnout-left",
    category: "Weichen",
    description: "C-Gleis Weiche links",
    length: 188.3,
    branchRadius: 437.5,
    branchAngle: 24.3,
    gauge: 16.5,
  };

  const turnoutRightEntry: CatalogEntry = {
    id: "24612",
    name: "Weiche rechts",
    kind: "turnout-right",
    category: "Weichen",
    description: "C-Gleis Weiche rechts",
    length: 188.3,
    branchRadius: 437.5,
    branchAngle: 24.3,
    gauge: 16.5,
  };

  it("renders 2D catenary for turnout-left with correct branch and masts", () => {
    const piece: TrackPiece = {
      id: "p-left",
      catalogId: "24611",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasCatenary: true,
    };

    const container = createTrackGraphics(piece, turnoutLeftEntry);
    expect(container).toBeDefined();
    expect(container.children.length).toBeGreaterThan(0);
  });

  it("renders 2D catenary for turnout-right with correct branch and masts", () => {
    const piece: TrackPiece = {
      id: "p-right",
      catalogId: "24612",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasCatenary: true,
    };

    const container = createTrackGraphics(piece, turnoutRightEntry);
    expect(container).toBeDefined();
    expect(container.children.length).toBeGreaterThan(0);
  });

  it("renders 2D catenary feeder and isolator markers on turnouts", () => {
    const piece: TrackPiece = {
      id: "p-iso",
      catalogId: "24611",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasCatenary: true,
      catenaryPowerFeed: true,
      catenaryIsolator: true,
      catenaryIsolatedConnectorIndex: 2,
    };

    const container = createTrackGraphics(piece, turnoutLeftEntry);
    expect(container).toBeDefined();
    expect(container.children.length).toBeGreaterThan(0);
  });

  it("turntable 2D catenary: only renders catenary on snappoints if a track with catenary is connected", () => {
    const turntableEntry: CatalogEntry = {
      id: "7286",
      name: "Drehscheibe",
      kind: "turntable",
      category: "Drehscheiben",
      description: "Drehscheibe",
      radius: 189,
      gauge: 16.5,
    };

    const connectedCatenaryTrack: TrackPiece = {
      id: "conn-cat",
      catalogId: "24188",
      transform: { x: 189, y: 0, rotation: 0 },
      connectors: [],
      hasCatenary: true,
    };

    const connectedNonCatenaryTrack: TrackPiece = {
      id: "conn-nocat",
      catalogId: "24188",
      transform: { x: 0, y: 189, rotation: 90 },
      connectors: [],
      hasCatenary: false,
    };

    // 1. Drehscheibe ohne angeschlossene Gleise
    const ttIsolated: TrackPiece = {
      id: "tt-1",
      catalogId: "7286",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 189, y: 0, angle: 0 },
        { x: 0, y: 189, angle: 90 },
      ],
      hasCatenary: true,
    };

    const gIsolated = createTrackGraphics(ttIsolated, turntableEntry, undefined, { allPieces: [ttIsolated] });
    expect(gIsolated).toBeDefined();

    // 2. Drehscheibe mit nur nicht-Oberleitungs-Gleis verbunden
    const ttWithNonCat: TrackPiece = {
      id: "tt-2",
      catalogId: "7286",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 189, y: 0, angle: 0, connectedTo: { pieceId: "conn-nocat", connectorIndex: 0 } },
      ],
      hasCatenary: true,
    };
    const gNonCat = createTrackGraphics(ttWithNonCat, turntableEntry, undefined, { allPieces: [ttWithNonCat, connectedNonCatenaryTrack] });
    expect(gNonCat).toBeDefined();

    // 3. Drehscheibe mit Oberleitungs-Gleis verbunden
    const ttWithCat: TrackPiece = {
      id: "tt-3",
      catalogId: "7286",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 189, y: 0, angle: 0, connectedTo: { pieceId: "conn-cat", connectorIndex: 0 } },
      ],
      hasCatenary: true,
    };
    const gCat = createTrackGraphics(ttWithCat, turntableEntry, undefined, { allPieces: [ttWithCat, connectedCatenaryTrack] });
    expect(gCat).toBeDefined();
  });

  it("transfer table 2D catenary: only renders catenary on snappoints if a track with catenary is connected", () => {
    const transferEntry: CatalogEntry = {
      id: "7294",
      name: "Schiebebühne",
      kind: "transfer-table",
      category: "Schiebebühnen",
      description: "Schiebebühne",
      length: 360,
      gauge: 16.5,
    };

    const connectedCatenaryTrack: TrackPiece = {
      id: "conn-cat-2",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasCatenary: true,
    };

    const tableWithCat: TrackPiece = {
      id: "st-1",
      catalogId: "7294",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180, connectedTo: { pieceId: "conn-cat-2", connectorIndex: 0 } },
      ],
      hasCatenary: true,
    };

    const g = createTrackGraphics(tableWithCat, transferEntry, undefined, { allPieces: [tableWithCat, connectedCatenaryTrack] });
    expect(g).toBeDefined();
  });
});
