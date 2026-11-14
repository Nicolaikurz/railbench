import { describe, it, expect } from "vitest";
import * as THREE from "three";
import { Viewer3D } from "@/render/viewer-3d";
import type { TrackPiece } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";

describe("3D Catenary Rendering on Ramps and Spirals", () => {
  const straightEntry: CatalogEntry = {
    id: "24188",
    name: "Gerades Gleis 188.3 mm",
    kind: "straight",
    category: "Gleise",
    description: "C-Gleis gerade",
    length: 188.3,
    gauge: 16.5,
  };

  const curveRightEntry: CatalogEntry = {
    id: "24130",
    name: "Gebogenes Gleis R1 30°",
    kind: "curve",
    category: "Kurven",
    description: "C-Gleis Kurve rechts",
    radius: 360,
    angle: 30,
    gauge: 16.5,
  };

  const curveLeftEntry: CatalogEntry = {
    id: "24130-L",
    name: "Gebogenes Gleis R1 30° links",
    kind: "curve",
    category: "Kurven",
    description: "C-Gleis Kurve links",
    radius: 360,
    angle: -30,
    gauge: 16.5,
  };

  const invokeBuildCatenary = (
    entry: CatalogEntry,
    startElev: number,
    endElev: number,
    piece?: Partial<TrackPiece>,
  ): THREE.Group => {
    const parent = new THREE.Group();
    const buildCatenary = (Viewer3D.prototype as any).buildCatenary3D;
    buildCatenary.call(null, parent, entry, startElev, endElev, piece as any);
    return parent.children[0] as THREE.Group;
  };

  it("renders 3D catenary on a straight ramp down (Gefälle) with vertical masts and sloped wires", () => {
    const startElev = 50;
    const endElev = 20; // heightDiff = -30
    const catGroup = invokeBuildCatenary(straightEntry, startElev, endElev);

    expect(catGroup).toBeDefined();
    // catGroup itself should NOT be tilted so coordinate axes remain plumb
    expect(catGroup.rotation.z).toBeCloseTo(0, 5);

    // Find contact wires (boxes with Y around contactH=72 + my)
    const wireMeshes: THREE.Mesh[] = [];
    const mastGroups: THREE.Group[] = [];

    catGroup.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry instanceof THREE.BoxGeometry) {
        // Wire meshes have width ~188, height ~0.8
        const p = obj.geometry.parameters;
        if (p.height === 0.8 && p.width > 100) {
          wireMeshes.push(obj);
        }
      } else if (obj instanceof THREE.Group && obj !== catGroup) {
        // Masts are nested groups
        mastGroups.push(obj);
      }
    });

    expect(wireMeshes.length).toBeGreaterThan(0);
    const wire = wireMeshes[0]!;
    // Midpoint Y should be contactH (72) + heightDiff / 2 (-15) = 57
    expect(wire.position.y).toBeCloseTo(72 - 15, 1);

    // Check masts: both should be standing plumb (vertical)
    // Start mast at x=0, y=0
    // End mast at x=188.3, y=-30
    expect(mastGroups.length).toBeGreaterThanOrEqual(2);
    const startMast = mastGroups.find((m) => Math.abs(m.position.x) < 5);
    const endMast = mastGroups.find((m) => Math.abs(m.position.x - 188.3) < 5);

    expect(startMast).toBeDefined();
    expect(startMast?.position.y).toBeCloseTo(0, 1);
    expect(startMast?.rotation.z).toBeCloseTo(0, 5);

    expect(endMast).toBeDefined();
    expect(endMast?.position.y).toBeCloseTo(-30, 1);
    expect(endMast?.rotation.z).toBeCloseTo(0, 5);
  });

  it("renders 3D catenary on a spiral down (Gleiswendel abwärts)", () => {
    const startElev = 100;
    const endElev = 80; // heightDiff = -20
    const catGroup = invokeBuildCatenary(curveRightEntry, startElev, endElev);

    expect(catGroup).toBeDefined();

    // Verify wire segments descend continuously along the curve
    const wireSegments: THREE.Mesh[] = [];
    const mastGroups: THREE.Group[] = [];

    catGroup.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry instanceof THREE.BoxGeometry) {
        const p = obj.geometry.parameters;
        if (p.height === 0.8 && p.depth === 0.8) {
          wireSegments.push(obj);
        }
      } else if (obj instanceof THREE.Group && obj !== catGroup) {
        mastGroups.push(obj);
      }
    });

    expect(wireSegments.length).toBeGreaterThanOrEqual(4);

    // Sort segments by X (or angle)
    wireSegments.sort((a, b) => a.position.x - b.position.x);

    // First segment should be near Y = 72 + (0.5/N) * (-20)
    expect(wireSegments[0]!.position.y).toBeLessThan(72);
    expect(wireSegments[0]!.position.y).toBeGreaterThan(72 - 5);

    // Last segment should be near Y = 72 - 20 = 52
    const lastSeg = wireSegments[wireSegments.length - 1]!;
    expect(lastSeg.position.y).toBeLessThan(72 - 15);
    expect(lastSeg.position.y).toBeGreaterThanOrEqual(52);

    // Each consecutive segment must be lower than the previous one
    for (let i = 1; i < wireSegments.length; i++) {
      expect(wireSegments[i]!.position.y).toBeLessThanOrEqual(wireSegments[i - 1]!.position.y);
    }

    // Check masts:
    // Start mast at Y = 0
    // End mast at Y = -20 (following track slope down)
    const startMast = mastGroups.find((m) => Math.abs(m.position.x) < 5);
    const endMast = mastGroups.find((m) => m.position.x > 50);

    expect(startMast).toBeDefined();
    expect(startMast?.position.y).toBeCloseTo(0, 1);

    expect(endMast).toBeDefined();
    expect(endMast?.position.y).toBeCloseTo(-20, 1);
  });

  it("renders 3D catenary for left-turning spirals (angle < 0) curving into -Z", () => {
    const startElev = 60;
    const endElev = 40; // heightDiff = -20
    const catGroup = invokeBuildCatenary(curveLeftEntry, startElev, endElev);

    const wireSegments: THREE.Mesh[] = [];
    catGroup.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry instanceof THREE.BoxGeometry) {
        const p = obj.geometry.parameters;
        if (p.height === 0.8 && p.depth === 0.8) {
          wireSegments.push(obj);
        }
      }
    });

    expect(wireSegments.length).toBeGreaterThanOrEqual(4);
    // For a left turn, Z coordinates must be negative
    for (const seg of wireSegments) {
      expect(seg.position.z).toBeLessThanOrEqual(0);
      expect(seg.position.x).toBeGreaterThanOrEqual(0);
    }
  });

  it("elevates catenary section insulator on a spiral down", () => {
    const startElev = 90;
    const endElev = 60; // heightDiff = -30
    const catGroup = invokeBuildCatenary(curveRightEntry, startElev, endElev, {
      catenaryIsolator: true,
      catenaryIsolatedConnectorIndex: 1, // frac = 0.82
    });

    // Find insulator group
    let isoGroup: THREE.Group | undefined;
    catGroup.traverse((obj) => {
      if (obj instanceof THREE.Group && obj.children.some((c) => (c as any).geometry instanceof THREE.BoxGeometry && (c as any).position.y > 70)) {
        isoGroup = obj;
      }
    });

    expect(isoGroup).toBeDefined();
    // In a spiral down with frac = 0.82 and heightDiff = -30, isoGroup.position.y should be ~ 0.82 * -30 = -24.6
    expect(isoGroup?.position.y).toBeCloseTo(0.82 * -30, 0.5);
  });

  it("renders 3D catenary on turntable: bridge wire always, snappoints only when track with catenary is connected", () => {
    const turntableEntry: CatalogEntry = {
      id: "7286",
      name: "Drehscheibe",
      kind: "turntable",
      category: "Drehscheiben",
      description: "Drehscheibe",
      radius: 189,
      gauge: 16.5,
    };

    const connectedCatTrack: TrackPiece = {
      id: "cat-p",
      catalogId: "24188",
      transform: { x: 189, y: 0, rotation: 0 },
      connectors: [],
      hasCatenary: true,
    };

    const connectedNoCatTrack: TrackPiece = {
      id: "nocat-p",
      catalogId: "24188",
      transform: { x: 0, y: 189, rotation: 90 },
      connectors: [],
      hasCatenary: false,
    };

    const ttPiece: TrackPiece = {
      id: "tt-main",
      catalogId: "7286",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 189, y: 0, angle: 0, connectedTo: { pieceId: "cat-p", connectorIndex: 0 } },
        { x: 0, y: 189, angle: 90, connectedTo: { pieceId: "nocat-p", connectorIndex: 0 } },
      ],
      hasCatenary: true,
    };

    const viewer = Object.create(Viewer3D.prototype);
    viewer.currentLayout = {
      version: 1,
      scale: "H0",
      pieces: [ttPiece, connectedCatTrack, connectedNoCatTrack],
      baseboard: { shape: "rectangle", width: 1000, length: 1000 },
    };

    const group = new THREE.Group();
    viewer.buildTurntableCatenary3D(group, ttPiece, turntableEntry);

    const wireMeshes: THREE.Mesh[] = [];
    const mastGroups: THREE.Group[] = [];

    group.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry instanceof THREE.BoxGeometry) {
        const p = obj.geometry.parameters;
        if (p.height === 0.8 && p.depth === 0.8) {
          wireMeshes.push(obj);
        }
      } else if (obj instanceof THREE.Group && (obj as any).children.some((c: any) => c.geometry instanceof THREE.BoxGeometry && c.geometry.parameters.height === 86)) {
        mastGroups.push(obj);
      }
    });

    // We should have bridge wire (~263 length) and 1 snappoint wire (for cat-p, length ~50)
    // The nocat-p snappoint should NOT have a wire!
    expect(wireMeshes.length).toBe(2);

    const bridgeWire = wireMeshes.find((w) => (w.geometry as THREE.BoxGeometry).parameters.width > 200);
    expect(bridgeWire).toBeDefined();

    const snappointWire = wireMeshes.find((w) => (w.geometry as THREE.BoxGeometry).parameters.width < 100);
    expect(snappointWire).toBeDefined();

    // Masts: 2 on bridge + 1 at connected catenary snappoint = 3 masts total
    expect(mastGroups.length).toBe(3);
  });

  it("renders 3D catenary on transfer table: bridge wire always, snappoints only when track with catenary is connected", () => {
    const transferEntry: CatalogEntry = {
      id: "7294",
      name: "Schiebebühne",
      kind: "transfer-table",
      category: "Schiebebühnen",
      description: "Schiebebühne",
      length: 360,
      gauge: 16.5,
    };

    const connectedCatTrack: TrackPiece = {
      id: "cat-tr",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasCatenary: true,
    };

    const tablePiece: TrackPiece = {
      id: "tt-main",
      catalogId: "7294",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180, connectedTo: { pieceId: "cat-tr", connectorIndex: 0 } },
      ],
      hasCatenary: true,
    };

    const viewer = Object.create(Viewer3D.prototype);
    viewer.currentLayout = {
      version: 1,
      scale: "H0",
      pieces: [tablePiece, connectedCatTrack],
      baseboard: { shape: "rectangle", width: 1000, length: 1000 },
    };

    const group = new THREE.Group();
    viewer.buildTransferTableCatenary3D(group, tablePiece, transferEntry);

    const wireMeshes: THREE.Mesh[] = [];
    group.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry instanceof THREE.BoxGeometry) {
        const p = obj.geometry.parameters;
        if (p.height === 0.8 && p.depth === 0.8) {
          wireMeshes.push(obj);
        }
      }
    });

    // 1 bridge wire + 1 snappoint wire
    expect(wireMeshes.length).toBe(2);
  });
});
