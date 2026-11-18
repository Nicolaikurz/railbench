import { describe, it, expect } from "vitest";
import { getUprightTextRotation, deg2rad } from "@/core/geometry";
import { createTrackGraphics } from "@/render/track-sprite";
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

  it("scales track number label height proportionally to track gauge (Z < N < H0 ≈ 18.5 mm)", () => {
    // 1. Z-Spurweite (6.5 mm): ca. 7.3 mm
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
    expect(labelZ.badgeHeight).toBeCloseTo(7.3, 1);

    // 2. N-Spurweite (9.0 mm): ca. 10.1 mm (kleiner als H0, größer als Z)
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
    expect(labelN.badgeHeight).toBeCloseTo(10.1, 1);
    expect(labelN.badgeHeight).toBeGreaterThan(labelZ.badgeHeight);

    // 3. H0-Spurweite (16.5 mm): ca. 18.5 mm (16.5 + 2 mm)
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
    expect(labelH0.badgeHeight).toBeCloseTo(18.5, 1);
    expect(labelH0.badgeHeight).toBeGreaterThan(labelN.badgeHeight);
  });
});
