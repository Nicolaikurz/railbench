import { describe, it, expect } from "vitest";
import { isBufferStopPiece, getLocalConnectors, type CatalogEntry } from "@/catalog/catalog";
import { distanceToTrackPiece, getPieceLocalCenter, isStraightTrackEntry } from "@/core/geometry";
import { createTrackGraphics } from "@/render/track-sprite";
import { buildBufferStop3D } from "@/render/builtin-models-3d";
import type { TrackPiece } from "@/core/types";
import * as THREE from "three";

describe("Track Buffer Stop & 3D Model Bufferstop Integration", () => {
  const maerklinCBufferStop: CatalogEntry = {
    id: "24977",
    name: "Prellbock-Gleis 77,5 mm",
    kind: "straight",
    category: "Zubehör",
    description: "Gleisstück 77,5 mm mit fest montiertem Prellbock",
    length: 77.5,
    gauge: 16.5,
  };

  const maerklinCIlluminatedBufferStop: CatalogEntry = {
    id: "24978",
    name: "Prellbock-Gleis beleuchtet 77,5 mm",
    kind: "straight",
    category: "Zubehör",
    description: "Gleisstück 77,5 mm mit beleuchtetem Sperrsignal und Prellbock",
    length: 77.5,
    gauge: 16.5,
  };

  const fleischmannBufferStop: CatalogEntry = {
    id: "9116",
    name: "Prellbock-Gleis 57,5 mm",
    kind: "buffer-stop",
    category: "Zubehör",
    description: "Prellbockgleis 57,5 mm",
    length: 57.5,
    gauge: 9,
  };

  const standardStraight: CatalogEntry = {
    id: "24188",
    name: "Gerades Gleis 188,3 mm",
    kind: "straight",
    category: "Geraden",
    description: "Standardgleis",
    length: 188.3,
    gauge: 16.5,
  };

  const roadPiece: CatalogEntry = {
    id: "ST-LOOP",
    name: "Wendeplatz",
    kind: "accessory",
    category: "Straßen",
    description: "Wendehammer",
    length: 200,
    isRoad: true,
  };

  it("identifies buffer stop tracks accurately across catalogs", () => {
    expect(isBufferStopPiece(maerklinCBufferStop)).toBe(true);
    expect(isBufferStopPiece(maerklinCIlluminatedBufferStop)).toBe(true);
    expect(isBufferStopPiece(fleischmannBufferStop)).toBe(true);
    expect(isBufferStopPiece(standardStraight)).toBe(false);
    expect(isBufferStopPiece(roadPiece)).toBe(false);
  });

  it("returns only the entry connector (single open end) for buffer-stop tracks", () => {
    const connectors = getLocalConnectors(fleischmannBufferStop);
    expect(connectors.length).toBe(1);
    expect(connectors[0]).toEqual({ x: 0, y: 0, angle: 180 });
  });

  it("renders 2D track graphics with buffer stop beam and Sh0 plate", () => {
    const piece: TrackPiece = {
      id: "p-buf-1",
      catalogId: "24977",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
    };

    const container = createTrackGraphics(piece, maerklinCBufferStop);
    expect(container).toBeDefined();
    expect(container.children.length).toBeGreaterThan(0);
  });

  it("builds the 3D buffer stop model with A-beams, wooden bumper plank and red disk", () => {
    const group = new THREE.Group();
    buildBufferStop3D(group);

    expect(group.children.length).toBe(4); // beamLeft, beamRight, bufferPlank, disk

    const meshes = group.children.filter((c) => c instanceof THREE.Mesh) as THREE.Mesh[];
    expect(meshes.length).toBe(4);

    // Bumper plank exists
    const plank = meshes.find((m) => m.geometry instanceof THREE.BoxGeometry && (m.geometry.parameters as any).width === 26);
    expect(plank).toBeDefined();

    // Red signal disk exists
    const disk = meshes.find((m) => m.geometry instanceof THREE.CylinderGeometry);
    expect(disk).toBeDefined();
  });

  it("renders LGB 10005 feeder track (300 mm, gauge 45 mm) with full length, centered marker, and hit-testing", () => {
    const lgbFeeder: CatalogEntry = {
      id: "10005",
      name: "Anschlussgleis 300 mm",
      kind: "feeder",
      category: "Zubehör",
      description: "Einspeisegleis 300 mm mit Schraubklemmen",
      length: 300,
      gauge: 45.0,
    };

    expect(isStraightTrackEntry(lgbFeeder)).toBe(true);
    expect(getPieceLocalCenter(lgbFeeder)).toEqual({ x: 150, y: 0 });
    expect(distanceToTrackPiece({ x: 250, y: 0 }, { x: 0, y: 0, rotation: 0 }, lgbFeeder)).toBeCloseTo(0, 5);

    const piece: TrackPiece = {
      id: "p-lgb-feeder",
      catalogId: "10005",
      transform: { x: 0, y: 0, rotation: 0 },
      powerFeed: true,
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 300, y: 0, angle: 0 },
      ],
    };

    const container = createTrackGraphics(piece, lgbFeeder, undefined, { isSelected: true });
    expect(container).toBeDefined();
    // Feeder marker container is centered at x = 150 and placed outside the G-scale ballast (y < -49)
    const markerChild = container.children.find((c) => c.position.x === 150 && c.position.y < -49);
    expect(markerChild).toBeDefined();
  });
});
