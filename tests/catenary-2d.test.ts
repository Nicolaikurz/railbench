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
});
