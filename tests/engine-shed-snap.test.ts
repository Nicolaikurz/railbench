import { describe, it, expect } from "vitest";
import { findBuiltinModelDef } from "@/core/models-3d";
import { buildEngineShed3D } from "@/render/models-3d/engine-shed";
import { buildTurntableTrainhall3D } from "@/render/models-3d/turntable-trainhall";
import {
  getEngineShedDoorConnector,
  getBuildingDoorConnectors,
  isConnectorConnectedToBuilding,
  isBuildingSnapped,
  snapEngineShedToTrack,
  snapTrainhallToTrack,
  snapTrackPieceToBuilding,
  snapTrackPieceToEngineShed,
} from "@/core/engine-shed-snap";
import type { TrackPiece, Transform, Model3D } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import * as THREE from "three";

describe("Engine Shed (Lokschuppen) Snapping, Clearance & Open Doors", () => {
  it("has slimmed dimensions in BUILTIN_MODELS to prevent side track collision", () => {
    const def = findBuiltinModelDef("engine-shed");
    expect(def).toBeDefined();
    expect(def?.width).toBe(240);
    expect(def?.depth).toBe(72);
    expect(def?.height).toBe(105);

    // Half-depth is 36 mm.
    // In H0, standard parallel track spacing:
    // - Märklin C-track: 64.3 mm -> distance to shed wall = 64.3 - 36 = 28.3 mm (> 24 mm NEM 102)
    // - Roco GeoLine: 61.6 mm -> distance to shed wall = 61.6 - 36 = 25.6 mm (> 24 mm NEM 102)
    const halfDepth = def!.depth / 2;
    expect(64.3 - halfDepth).toBeGreaterThanOrEqual(24.0);
    expect(61.6 - halfDepth).toBeGreaterThanOrEqual(24.0);
  });

  it("builds 3D engine shed with all elements within lateral width (|Z| <= 37 mm)", () => {
    const group = new THREE.Group();
    buildEngineShed3D(group);
    group.updateMatrixWorld(true);

    let maxAbsZ = 0;
    group.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry) {
        const geo = obj.geometry;
        const pos = geo.getAttribute("position");
        if (!pos) return;
        const v = new THREE.Vector3();
        for (let i = 0; i < pos.count; i++) {
          v.fromBufferAttribute(pos, i);
          v.applyMatrix4(obj.matrixWorld);
          maxAbsZ = Math.max(maxAbsZ, Math.abs(v.z));
        }
      }
    });

    // Max lateral extension must be <= 37.5 mm so adjacent parallel tracks never collide
    expect(maxAbsZ).toBeLessThanOrEqual(37.5);
  });

  it("builds 3D engine shed with wide open doors (~80° rotation)", () => {
    const group = new THREE.Group();
    buildEngineShed3D(group);

    // Find groups representing the door hinges
    const hinges: THREE.Group[] = [];
    group.traverse((obj) => {
      if (obj instanceof THREE.Group && Math.abs(obj.rotation.y) >= 1.0) {
        hinges.push(obj);
      }
    });

    // Should have 2 wide open door leaves (left & right)
    expect(hinges.length).toBeGreaterThanOrEqual(2);
    for (const h of hinges) {
      expect(Math.abs(h.rotation.y)).toBeGreaterThanOrEqual(1.2); // ~70°-80°
    }
  });

  it("builds turntable-trainhall with wide open doors on all 3 stalls", () => {
    const group = new THREE.Group();
    buildTurntableTrainhall3D(group);

    const openDoorHinges: THREE.Group[] = [];
    group.traverse((obj) => {
      if (obj instanceof THREE.Group && Math.abs(obj.rotation.y) >= 1.0) {
        openDoorHinges.push(obj);
      }
    });

    // 3 stalls * 2 doors = 6 wide open door leaves
    expect(openDoorHinges.length).toBeGreaterThanOrEqual(6);
    for (const h of openDoorHinges) {
      expect(Math.abs(h.rotation.y)).toBeGreaterThanOrEqual(1.2);
    }
  });

  it("provides sufficient clearance for catenary (wire height 72-85mm) at entry and under roof", () => {
    // Contact wire is at Y = 72mm, messenger wire at Y = 85mm
    const catenaryWireH = 85;

    // Test Engine Shed (Lokschuppen)
    const shedGroup = new THREE.Group();
    buildEngineShed3D(shedGroup);
    shedGroup.updateMatrixWorld(true);

    let maxRoofApexY = 0;
    shedGroup.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry) {
        const geo = obj.geometry;
        const pos = geo.getAttribute("position");
        if (!pos) return;
        const v = new THREE.Vector3();
        for (let i = 0; i < pos.count; i++) {
          v.fromBufferAttribute(pos, i);
          v.applyMatrix4(obj.matrixWorld);
          maxRoofApexY = Math.max(maxRoofApexY, v.y);
        }
      }
    });

    // Roof ridge reaches > 120 mm (well above 85 mm catenary)
    expect(maxRoofApexY).toBeGreaterThanOrEqual(120);

    // Test Turntable Trainhall (Ringlokschuppen)
    const hallGroup = new THREE.Group();
    buildTurntableTrainhall3D(hallGroup);
    hallGroup.updateMatrixWorld(true);

    let maxHallRoofY = 0;
    hallGroup.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry) {
        const geo = obj.geometry;
        const pos = geo.getAttribute("position");
        if (!pos) return;
        const v = new THREE.Vector3();
        for (let i = 0; i < pos.count; i++) {
          v.fromBufferAttribute(pos, i);
          v.applyMatrix4(obj.matrixWorld);
          maxHallRoofY = Math.max(maxHallRoofY, v.y);
        }
      }
    });

    // Ringlokschuppen roof ridge reaches > 120 mm
    expect(maxHallRoofY).toBeGreaterThanOrEqual(120);
  });

  it("calculates engine shed door connector correctly", () => {
    const shed = {
      transform: { x: 300, y: 200, rotation: 0 },
      width: 240,
    };
    const conn = getEngineShedDoorConnector(shed);
    // Door is at x = 300 - 120 = 180, y = 200, angle = 0 + 180 = 180
    expect(conn.x).toBe(180);
    expect(conn.y).toBe(200);
    expect(conn.angle).toBe(180);
  });

  it("snaps engine shed to track connector and aligns rotation", () => {
    // Track piece ending at (100, 200) with connector pointing east (angle 0)
    const trackPiece: TrackPiece = {
      id: "p-1",
      catalogId: "24188",
      transform: { x: 50, y: 200, rotation: 0 },
      connectors: [
        { x: 0, y: 200, angle: 180 },
        { x: 100, y: 200, angle: 0 },
      ],
    };

    // Candidate shed positioned near (100, 200)
    const candTransform: Transform = { x: 215, y: 205, rotation: 10 };
    const result = snapEngineShedToTrack(candTransform, [trackPiece]);

    expect(result).not.toBeNull();
    expect(result?.targetPiece.id).toBe("p-1");
    expect(result?.connectorIndex).toBe(1);

    // Snapped shed: door (-120, 0) at (100, 200) -> shed center at (220, 200), rotation 0
    expect(result?.snappedTransform.rotation).toBe(0);
    expect(result?.snappedTransform.x).toBe(220);
    expect(result?.snappedTransform.y).toBe(200);
  });

  it("snaps track piece to engine shed doorway", () => {
    const shed: Model3D = {
      id: "m-shed",
      name: "Lokschuppen",
      category: "railway",
      modelType: "builtin",
      builtinId: "engine-shed",
      transform: { x: 220, y: 200, rotation: 0 },
      width: 240,
      depth: 72,
      height: 105,
    };

    // Straight track piece of length 188 mm
    const trackEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188.3 mm",
      category: "Geraden",
      description: "Gerades Gleis 188.3 mm",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Candidate track near shed door (100, 200)
    const candTrackTransform: Transform = { x: -85, y: 205, rotation: 5 };
    const snap = snapTrackPieceToEngineShed(trackEntry, candTrackTransform, [shed]);

    expect(snap).not.toBeNull();
    expect(snap?.movingConnectorIndex).toBe(1); // Connector 1 (x: 188.3, angle: 0) docks into door
    expect(snap?.snappedTransform.rotation).toBe(0);
    expect(snap?.snappedTransform.x).toBeCloseTo(-88.3, 0);
    expect(snap?.snappedTransform.y).toBe(200);
  });

  it("verifies 3D engine shed roof has proper gable pitch (ridge higher than eaves)", () => {
    const group = new THREE.Group();
    buildEngineShed3D(group);
    group.updateMatrixWorld(true);

    let maxY = -Infinity;
    let minY = Infinity;
    let foundExtrudedRoof = false;

    group.traverse((obj) => {
      if (obj instanceof THREE.Mesh && obj.geometry instanceof THREE.ExtrudeGeometry) {
        foundExtrudedRoof = true;
        const pos = obj.geometry.getAttribute("position");
        const v = new THREE.Vector3();
        for (let i = 0; i < pos.count; i++) {
          v.fromBufferAttribute(pos, i);
          v.applyMatrix4(obj.matrixWorld);
          maxY = Math.max(maxY, v.y);
          minY = Math.min(minY, v.y);
        }
      }
    });

    expect(foundExtrudedRoof).toBe(true);
    // Ridge at Y ~ 124 mm, eaves at Y ~ 97-100 mm (elevated so catenary at 72-85mm fits inside)
    expect(maxY).toBeGreaterThanOrEqual(120);
    expect(minY).toBeLessThanOrEqual(100);
    expect(maxY - minY).toBeGreaterThanOrEqual(20); // clear gable pitch
  });

  it("snaps track piece INSIDE the engine shed (connector 0 docks at doorway heading in)", () => {
    const shed: Model3D = {
      id: "m-shed",
      name: "Lokschuppen",
      category: "railway",
      modelType: "builtin",
      builtinId: "engine-shed",
      transform: { x: 220, y: 200, rotation: 0 },
      width: 240,
      depth: 72,
      height: 105,
    };

    const trackEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188.3 mm",
      category: "Geraden",
      description: "Gerades Gleis 188.3 mm",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Candidate track placed near door (100, 200) extending eastward into the shed
    const candInsideTransform: Transform = { x: 105, y: 202, rotation: 2 };
    const snap = snapTrackPieceToBuilding(trackEntry, candInsideTransform, [shed]);

    expect(snap).not.toBeNull();
    expect(snap?.movingConnectorIndex).toBe(0); // Connector 0 docks at door
    expect(snap?.snappedTransform.rotation).toBe(0);
    // Doorway is at (100, 200) heading inward along angle 0
    expect(snap?.snappedTransform.x).toBeCloseTo(100, 0);
    expect(snap?.snappedTransform.y).toBeCloseTo(200, 0);
  });

  it("calculates 3 doorway connectors for turntable-trainhall (Ringlokschuppen)", () => {
    const trainhall: Model3D = {
      id: "m-th",
      name: "Ringlokschuppen",
      category: "railway",
      modelType: "builtin",
      builtinId: "turntable-trainhall",
      transform: { x: 400, y: 500, rotation: 0 },
      width: 280,
      depth: 260,
      height: 110,
    };

    const doors = getBuildingDoorConnectors(trainhall);
    expect(doors.length).toBe(3);

    // Stall 0: -12°
    expect(doors[0]!.stallIndex).toBe(0);
    expect(doors[0]!.x).toBeCloseTo(400 - 40.54, 0);
    expect(doors[0]!.y).toBeCloseTo(500 - 159.26, 0);

    // Stall 1: 0° (center)
    expect(doors[1]!.stallIndex).toBe(1);
    expect(doors[1]!.x).toBeCloseTo(400, 0);
    expect(doors[1]!.y).toBeCloseTo(500 - 155.0, 0);

    // Stall 2: +12°
    expect(doors[2]!.stallIndex).toBe(2);
    expect(doors[2]!.x).toBeCloseTo(400 + 40.54, 0);
    expect(doors[2]!.y).toBeCloseTo(500 - 159.26, 0);
  });

  it("snaps track pieces to turntable-trainhall stalls (both approaching and inside)", () => {
    const trainhall: Model3D = {
      id: "m-th",
      name: "Ringlokschuppen",
      category: "railway",
      modelType: "builtin",
      builtinId: "turntable-trainhall",
      transform: { x: 400, y: 500, rotation: 0 },
      width: 280,
      depth: 260,
      height: 110,
    };

    const trackEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188.3 mm",
      category: "Geraden",
      description: "Gerades Gleis 188.3 mm",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Center stall doorway is at (400, 345) pointing south (angle 270°)
    // Test approaching center stall from outside: connector 1 docks at (400, 345)
    const candApproach: Transform = { x: 395, y: 160, rotation: 85 };
    const snapApproach = snapTrackPieceToBuilding(trackEntry, candApproach, [trainhall]);
    expect(snapApproach).not.toBeNull();
    expect(snapApproach?.movingConnectorIndex).toBe(1);
    expect(snapApproach?.snappedTransform.rotation).toBe(90);

    // Test inside center stall: connector 0 docks at (400, 345) heading south (rotation 90°)
    const candInside: Transform = { x: 405, y: 350, rotation: 95 };
    const snapInside = snapTrackPieceToBuilding(trackEntry, candInside, [trainhall]);
    expect(snapInside).not.toBeNull();
    expect(snapInside?.movingConnectorIndex).toBe(0);
    expect(snapInside?.snappedTransform.rotation).toBe(90);
    expect(snapInside?.snappedTransform.x).toBeCloseTo(400, 0);
    expect(snapInside?.snappedTransform.y).toBeCloseTo(345, 0);
  });

  it("snaps turntable-trainhall to existing track piece", () => {
    // Track piece ending at (400, 345) pointing south (angle 90°)
    const trackPiece: TrackPiece = {
      id: "p-in",
      catalogId: "24188",
      transform: { x: 400, y: 156.7, rotation: 90 },
      connectors: [
        { x: 400, y: 156.7, angle: 270 },
        { x: 400, y: 345, angle: 90 },
      ],
    };

    // Trainhall placed near (400, 500)
    const candTransform: Transform = { x: 403, y: 502, rotation: 1 };
    const snap = snapTrainhallToTrack(candTransform, [trackPiece]);

    expect(snap).not.toBeNull();
    expect(snap?.stallIndex).toBe(1); // Center stall
    expect(snap?.snappedTransform.x).toBeCloseTo(400, 0);
    expect(snap?.snappedTransform.y).toBeCloseTo(500, 0);
    expect(snap?.snappedTransform.rotation).toBe(0);
  });

  it("identifies connector connected to building (for blue snappoint display)", () => {
    const shed: Model3D = {
      id: "m-shed",
      name: "Lokschuppen",
      category: "railway",
      modelType: "builtin",
      builtinId: "engine-shed",
      transform: { x: 220, y: 200, rotation: 0 },
      width: 240,
      depth: 72,
      height: 105,
    };

    const connectedConn = { x: 100, y: 200, angle: 0 };
    const farConn = { x: 0, y: 200, angle: 180 };

    expect(isConnectorConnectedToBuilding(connectedConn, [shed])).not.toBeNull();
    expect(isConnectorConnectedToBuilding(farConn, [shed])).toBeNull();
    expect(isBuildingSnapped(shed, [{ id: "p1", catalogId: "24188", transform: { x: 0, y: 0, rotation: 0 }, connectors: [connectedConn] }])).toBe(true);
  });

  it("snaps engine shed to track connector even if it is already connected to another track", () => {
    // Two connected track pieces meeting at (100, 200)
    const p1: TrackPiece = {
      id: "p1",
      catalogId: "24188",
      transform: { x: 50, y: 200, rotation: 0 },
      connectors: [
        { x: 0, y: 200, angle: 180 },
        { x: 100, y: 200, angle: 0, connectedTo: { pieceId: "p2", connectorIndex: 0 } },
      ],
    };
    const p2: TrackPiece = {
      id: "p2",
      catalogId: "24188",
      transform: { x: 150, y: 200, rotation: 0 },
      connectors: [
        { x: 100, y: 200, angle: 180, connectedTo: { pieceId: "p1", connectorIndex: 1 } },
        { x: 200, y: 200, angle: 0 },
      ],
    };

    // Candidate shed placed near the connected joint (100, 200)
    const candTransform: Transform = { x: 220, y: 205, rotation: 5 };
    const snap = snapEngineShedToTrack(candTransform, [p1, p2]);

    expect(snap).not.toBeNull();
    expect(snap?.snappedTransform.x).toBe(220);
    expect(snap?.snappedTransform.y).toBe(200);
    expect(snap?.snappedTransform.rotation).toBe(0);
  });

  it("snaps turntable-trainhall to track piece even if it is already connected to another track", () => {
    // Lead track connected to another piece at (400, 345)
    const p1: TrackPiece = {
      id: "p-lead",
      catalogId: "24188",
      transform: { x: 400, y: 156.7, rotation: 90 },
      connectors: [
        { x: 400, y: 156.7, angle: 270 },
        { x: 400, y: 345, angle: 90, connectedTo: { pieceId: "p-next", connectorIndex: 0 } },
      ],
    };
    const p2: TrackPiece = {
      id: "p-next",
      catalogId: "24188",
      transform: { x: 400, y: 345, rotation: 90 },
      connectors: [
        { x: 400, y: 345, angle: 270, connectedTo: { pieceId: "p-lead", connectorIndex: 1 } },
        { x: 400, y: 533.3, angle: 90 },
      ],
    };

    const candTransform: Transform = { x: 402, y: 502, rotation: 3 };
    const snap = snapTrainhallToTrack(candTransform, [p1, p2]);

    expect(snap).not.toBeNull();
    expect(snap?.stallIndex).toBe(1);
    expect(snap?.snappedTransform.x).toBeCloseTo(400, 0);
    expect(snap?.snappedTransform.y).toBeCloseTo(500, 0);
  });
});
