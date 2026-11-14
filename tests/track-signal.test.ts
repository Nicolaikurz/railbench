import { describe, it, expect } from "vitest";
import * as THREE from "three";
import { Viewer3D } from "@/render/viewer-3d";
import { createTrackGraphics } from "@/render/track-sprite";
import type { Layout, TrackPiece } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import { History, ToggleSignalCommand, SetSignalConfigCommand } from "@/editor/commands";

describe("Track Signals: Both Sides and Both Directions", () => {
  const straightEntry: CatalogEntry = {
    id: "24188",
    name: "Gerades Gleis 188.3 mm",
    kind: "straight",
    category: "Gleise",
    description: "C-Gleis gerade",
    length: 188.3,
    gauge: 16.5,
  };

  const curveEntry: CatalogEntry = {
    id: "24130",
    name: "Gebogenes Gleis R1 30°",
    kind: "curve",
    category: "Kurven",
    description: "C-Gleis Kurve rechts",
    radius: 360,
    angle: 30,
    gauge: 16.5,
  };

  it("2D rendering: creates signals on right side, left side, and both sides", () => {
    // 1. Right side (default)
    const pieceRight: TrackPiece = {
      id: "p-right",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "right",
      signalDirection: "forward",
    };
    const cRight = createTrackGraphics(pieceRight, straightEntry);
    expect(cRight).toBeDefined();

    // 2. Left side
    const pieceLeft: TrackPiece = {
      id: "p-left",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "left",
      signalDirection: "forward",
    };
    const cLeft = createTrackGraphics(pieceLeft, straightEntry);
    expect(cLeft).toBeDefined();

    // 3. Both sides
    const pieceBoth: TrackPiece = {
      id: "p-both",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "both",
      signalDirection: "forward",
    };
    const cBoth = createTrackGraphics(pieceBoth, straightEntry);
    expect(cBoth).toBeDefined();
    // Container should hold more children when signals are on both sides
    expect(cBoth.children.length).toBeGreaterThan(cRight.children.length);
  });

  it("2D rendering: creates signals in forward, backward, and both directions", () => {
    // Forward
    const pFwd: TrackPiece = {
      id: "p-fwd",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "right",
      signalDirection: "forward",
    };
    const cFwd = createTrackGraphics(pFwd, straightEntry);

    // Backward
    const pBwd: TrackPiece = {
      id: "p-bwd",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "right",
      signalDirection: "backward",
    };
    const cBwd = createTrackGraphics(pBwd, straightEntry);
    expect(cBwd).toBeDefined();

    // Both directions
    const pBothDir: TrackPiece = {
      id: "p-both-dir",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "right",
      signalDirection: "both",
    };
    const cBothDir = createTrackGraphics(pBothDir, straightEntry);
    expect(cBothDir.children.length).toBeGreaterThan(cFwd.children.length);
  });

  it("3D rendering: creates correct number and orientation of signals for both sides and directions", () => {
    const viewer = Object.create(Viewer3D.prototype);

    // 1. Single signal right + forward
    const p1: TrackPiece = {
      id: "p-1",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "right",
      signalDirection: "forward",
    };
    const g1 = new THREE.Group();
    (viewer as any).buildTrackSignal3D(g1, straightEntry, p1);
    expect(g1.children.length).toBe(1);
    expect(g1.children[0]!.position.z).toBeLessThan(0); // Right side (negative Z)
    expect(g1.children[0]!.rotation.y).toBeCloseTo(-Math.PI / 2, 2); // Facing incoming train from start

    // 2. Signal left + backward
    const p2: TrackPiece = {
      id: "p-2",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "left",
      signalDirection: "backward",
    };
    const g2 = new THREE.Group();
    (viewer as any).buildTrackSignal3D(g2, straightEntry, p2);
    expect(g2.children.length).toBe(1);
    expect(g2.children[0]!.position.z).toBeGreaterThan(0); // Left side (positive Z)
    expect(g2.children[0]!.rotation.y).toBeCloseTo(Math.PI / 2, 2); // Facing incoming train from end

    // 3. Both sides + forward
    const p3: TrackPiece = {
      id: "p-3",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "both",
      signalDirection: "forward",
    };
    const g3 = new THREE.Group();
    (viewer as any).buildTrackSignal3D(g3, straightEntry, p3);
    expect(g3.children.length).toBe(2);
    const zPositions = g3.children.map((c) => c.position.z);
    expect(zPositions.some((z) => z < 0)).toBe(true); // Right side
    expect(zPositions.some((z) => z > 0)).toBe(true); // Left side

    // 4. Both sides + both directions (2x2 = 4 signals)
    const p4: TrackPiece = {
      id: "p-4",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "both",
      signalDirection: "both",
    };
    const g4 = new THREE.Group();
    (viewer as any).buildTrackSignal3D(g4, straightEntry, p4);
    expect(g4.children.length).toBe(4);
  });

  it("3D rendering: renders signals on curved tracks correctly", () => {
    const viewer = Object.create(Viewer3D.prototype);

    const pCurve: TrackPiece = {
      id: "p-curve",
      catalogId: "24130",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "both",
      signalDirection: "both",
    };
    const g = new THREE.Group();
    (viewer as any).buildTrackSignal3D(g, curveEntry, pCurve);
    expect(g.children.length).toBe(4);
  });

  it("commands and undo/redo: SetSignalConfigCommand properly updates side and direction", () => {
    const piece: TrackPiece = {
      id: "p-test",
      catalogId: "24188",
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [],
      hasSignal: true,
      signalSide: "right",
      signalDirection: "forward",
    };
    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      pieces: [piece],
    };
    const history = new History(layout);

    // Switch side to left
    history.run(new SetSignalConfigCommand(["p-test"], { signalSide: "left" }));
    expect(piece.signalSide).toBe("left");

    // Switch direction to backward
    history.run(new SetSignalConfigCommand(["p-test"], { signalDirection: "backward" }));
    expect(piece.signalDirection).toBe("backward");

    // Switch to both
    history.run(new SetSignalConfigCommand(["p-test"], { signalSide: "both", signalDirection: "both" }));
    expect(piece.signalSide).toBe("both");
    expect(piece.signalDirection).toBe("both");

    // Undo to backward + left
    history.undo();
    expect(piece.signalSide).toBe("left");
    expect(piece.signalDirection).toBe("backward");

    // Undo to left + forward
    history.undo();
    expect(piece.signalSide).toBe("left");
    expect(piece.signalDirection).toBe("forward");

    // Undo to initial right + forward
    history.undo();
    expect(piece.signalSide).toBe("right");
    expect(piece.signalDirection).toBe("forward");
  });
});
