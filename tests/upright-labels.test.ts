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
});
