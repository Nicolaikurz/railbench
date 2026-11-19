import { describe, it, expect } from "vitest";
import { Text } from "pixi.js";
import { getUprightTextRotation, deg2rad } from "@/core/geometry";
import { createTrackGraphics } from "@/render/track-sprite";
import { getTrackNumberBadgeHeight, computeLabelTextResolution } from "@/render/track-badges";
import { createModel2DGraphics } from "@/render/model-sprite";
import type { TrackPiece, Model3D } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";

describe("Upright Labels & Text Rotation (CAD-style horizontal orientation)", () => {
  it("calculates exact counter-rotation angle to cancel container rotation", () => {
    // 0 deg -> 0 rad
    expect(getUprightTextRotation(0)).toBeCloseTo(0);

    // 90 deg -> -PI / 2 rad
    expect(getUprightTextRotation(90)).toBeCloseTo(-Math.PI / 2);

    // 180 deg -> -PI rad
    expect(getUprightTextRotation(180)).toBeCloseTo(-Math.PI);

    // 270 deg -> -3*PI / 2 rad (or PI / 2 rad)
    expect(getUprightTextRotation(270)).toBeCloseTo(-1.5 * Math.PI);

    // Any arbitrary angle (e.g. 45 deg)
    const rot45 = 45;
    const parentRad = deg2rad(rot45);
    const counterRad = getUprightTextRotation(rot45);
    // World rotation = parentRad + counterRad must be 0 (strictly horizontal)
    expect(parentRad + counterRad).toBeCloseTo(0);
  });

  it("ensures track article number label is counter-rotated to remain horizontal at any piece rotation", () => {
    const entry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188,3 mm",
      kind: "straight",
      category: "Geraden",
      description: "Standardgleis",
      length: 188.3,
      gauge: 16.5,
    };

    const angles = [0, 45, 90, 135, 180, 225, 270, 315];

    for (const angle of angles) {
      const piece: TrackPiece = {
        id: `p-${angle}`,
        catalogId: "24188",
        transform: { x: 200, y: 200, rotation: angle },
        connectors: [],
      };

      const container = createTrackGraphics(piece, entry, undefined, { showTrackLabels: true });
      expect(container.rotation).toBeCloseTo(deg2rad(angle));

      // Find the label container child
      const labelCont = container.children.find(
        (c) => c !== container && typeof (c as any).rotation === "number" && (c as any).children?.length === 2,
      ) as any;

      expect(labelCont).toBeDefined();
      expect(labelCont.rotation).toBeCloseTo(getUprightTextRotation(angle));

      // World angle must equal 0 (perfectly horizontal, left-to-right)
      const worldRotation = container.rotation + labelCont.rotation;
      expect(worldRotation).toBeCloseTo(0);
    }
  });

  it("ensures 2D building and model labels remain horizontal and upright at any model rotation", () => {
    const angles = [0, 60, 90, 180, 270];

    for (const angle of angles) {
      const model: Model3D = {
        id: `building-${angle}`,
        name: "Bahnhof Neustadt",
        category: "buildings",
        modelType: "builtin",
        builtinId: "station-medium",
        transform: { x: 500, y: 500, rotation: angle },
        width: 150,
        depth: 80,
        height: 60,
      };

      const container = createModel2DGraphics(model, false);
      expect(container.rotation).toBeCloseTo(deg2rad(angle));

      // Find the label container child (contains background badge + text)
      const labelCont = container.children.find(
        (c) => c !== container && typeof (c as any).rotation === "number" && (c as any).children?.length === 2,
      ) as any;

      expect(labelCont).toBeDefined();
      expect(labelCont.rotation).toBeCloseTo(getUprightTextRotation(angle));

      // World angle must equal 0 (perfectly horizontal, never upside down)
      const worldRotation = container.rotation + labelCont.rotation;
      expect(worldRotation).toBeCloseTo(0);
    }
  });

  it("fits track number label height slightly below ballast width (~80%) with minimum gauge (Z: 11.4 mm, N: 15.8 mm, H0: 28.8 mm)", () => {
    // 1. Z-Spurweite (6.5 mm mit Bettung): 14.3 * 0.8 = 11.4 mm (>= 6.5 mm gauge)
    const entryZ: CatalogEntry = {
      id: "8500",
      name: "Gerades Gleis 110 mm",
      kind: "straight",
      category: "Geraden",
      description: "Märklin Spur Z",
      length: 110,
      gauge: 6.5,
    };
    const pieceZ: TrackPiece = {
      id: "piece-z",
      catalogId: "8500",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: [],
    };
    const containerZ = createTrackGraphics(pieceZ, entryZ, undefined, { showTrackLabels: true });
    const labelZ = containerZ.children.find((c: any) => c.label === "track-number-label") as any;
    expect(labelZ).toBeDefined();
    expect(labelZ.badgeHeight).toBeCloseTo(getTrackNumberBadgeHeight(entryZ), 1);
    expect(labelZ.badgeHeight).toBeCloseTo(11.4, 1);
    expect(labelZ.badgeHeight).toBeGreaterThanOrEqual(6.5);

    // 2. N-Spurweite (9.0 mm mit Bettung): 19.8 * 0.8 = 15.8 mm (>= 9.0 mm gauge)
    const entryN: CatalogEntry = {
      id: "9100",
      name: "Gerades Gleis 222 mm",
      kind: "straight",
      category: "Geraden",
      description: "Fleischmann Piccolo Spur N",
      length: 222,
      gauge: 9.0,
    };
    const pieceN: TrackPiece = {
      id: "piece-n",
      catalogId: "9100",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: [],
    };
    const containerN = createTrackGraphics(pieceN, entryN, undefined, { showTrackLabels: true });
    const labelN = containerN.children.find((c: any) => c.label === "track-number-label") as any;
    expect(labelN).toBeDefined();
    expect(labelN.badgeHeight).toBeCloseTo(getTrackNumberBadgeHeight(entryN), 1);
    expect(labelN.badgeHeight).toBeCloseTo(15.8, 1);
    expect(labelN.badgeHeight).toBeGreaterThan(labelZ.badgeHeight);

    // 3. H0-Spurweite (16.5 mm mit Bettung / Schotter): 36.0 * 0.8 = 28.8 mm (>= 16.5 mm gauge)
    const entryH0: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188,3 mm",
      kind: "straight",
      category: "Geraden",
      description: "Märklin C-Gleis H0",
      length: 188.3,
      gauge: 16.5,
    };
    const pieceH0: TrackPiece = {
      id: "piece-h0",
      catalogId: "24188",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: [],
    };
    const containerH0 = createTrackGraphics(pieceH0, entryH0, undefined, { showTrackLabels: true });
    const labelH0 = containerH0.children.find((c: any) => c.label === "track-number-label") as any;
    expect(labelH0).toBeDefined();
    expect(labelH0.badgeHeight).toBeCloseTo(getTrackNumberBadgeHeight(entryH0), 1);
    expect(labelH0.badgeHeight).toBeCloseTo(28.8, 1);
    expect(labelH0.badgeHeight).toBeGreaterThan(labelN.badgeHeight);

    // 4. H0 ohne Bettung (z.B. K-Gleis, hasRoadbed: false): 24.8 * 0.8 = 19.8 mm (>= 16.5 mm gauge)
    const entryH0NoBed: CatalogEntry = {
      ...entryH0,
      id: "2200",
      hasRoadbed: false,
    };
    const containerH0NoBed = createTrackGraphics(pieceH0, entryH0NoBed, undefined, { showTrackLabels: true });
    const labelH0NoBed = containerH0NoBed.children.find((c: any) => c.label === "track-number-label") as any;
    expect(labelH0NoBed.badgeHeight).toBeCloseTo(19.8, 1);
    expect(labelH0NoBed.badgeHeight).toBeGreaterThanOrEqual(16.5);
  });

  it("increases track number label Text resolution when zoomed in so labels stay sharp", () => {
    const entryH0: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188,3 mm",
      kind: "straight",
      category: "Geraden",
      description: "Märklin C-Gleis H0",
      length: 188.3,
      gauge: 16.5,
    };
    const pieceH0: TrackPiece = {
      id: "piece-zoom",
      catalogId: "24188",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: [],
    };

    const container1x = createTrackGraphics(pieceH0, entryH0, undefined, {
      showTrackLabels: true,
      worldScale: 1,
    });
    const label1x = container1x.children.find((c: any) => c.label === "track-number-label") as any;
    const text1x = label1x.children.find((c: any) => c instanceof Text) as Text;

    const container8x = createTrackGraphics(pieceH0, entryH0, undefined, {
      showTrackLabels: true,
      worldScale: 8,
    });
    const label8x = container8x.children.find((c: any) => c.label === "track-number-label") as any;
    const text8x = label8x.children.find((c: any) => c instanceof Text) as Text;

    expect(text1x.resolution).toBeGreaterThanOrEqual(2);
    expect(text8x.resolution).toBeGreaterThan(text1x.resolution);
    expect(computeLabelTextResolution(12, 10)).toBeGreaterThan(computeLabelTextResolution(12, 1));
  });
});
