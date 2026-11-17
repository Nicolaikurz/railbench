import { describe, it, expect } from "vitest";
import { TrackOperations, type EditorContext } from "@/editor/track-operations";
import { History } from "@/editor/commands";
import { Selection } from "@/editor/selection";
import { maerklinCCatalog, computeWorldConnectors, type CatalogEntry } from "@/catalog/catalog";
import { updatePieceConnections } from "@/core/snapping";
import type { Layout, TrackPiece } from "@/core/types";
import { findOpenConnectorAt, checkFlexHandleHit } from "@/editor/canvas-hit-test";

describe("Open track plus - appendTrackToConnector", () => {
  function createMockEditor(initialPieces: TrackPiece[] = []) {
    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      pieces: [...initialPieces],
      layers: [{ id: "layer-1", name: "Ebene 1", color: "#3b82f6", visible: true, locked: false, heightMm: 0 }],
      baseboards: [],
      models: [],
    };
    const history = new History(layout);
    const selection = new Selection();
    let toastMsg = "";

    const ctx: EditorContext = {
      getLayout: () => layout,
      setLayout: (l) => {
        Object.assign(layout, l);
      },
      getCatalog: () => maerklinCCatalog,
      getSelection: () => selection,
      getHistory: () => history,
      setHistory: () => {},
      getRenderer: () => ({} as any),
      getPlanManager: () => ({} as any),
      isSnappingEnabled: () => true,
      isPieceInteractive: () => true,
      isModelInteractive: () => true,
      getCatalogEntry: (p) => maerklinCCatalog.entries.find((e) => e.id === p.catalogId),
      sync: () => {},
      showToast: (msg) => {
        toastMsg = msg;
      },
    };

    const operations = new TrackOperations(ctx);
    return { layout, history, selection, operations, getToast: () => toastMsg };
  }

  it("appends default straight track to open connector 1 and connects them", () => {
    const entry24188 = maerklinCCatalog.entries.find((e) => e.id === "24188")!;
    const p1: TrackPiece = {
      id: "p-1",
      catalogId: "24188",
      layerId: "layer-1",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: computeWorldConnectors(entry24188, { x: 100, y: 100, rotation: 0 }),
    };

    const { layout, operations, selection, history } = createMockEditor([p1]);

    // Connector 1 is at (100 + 188.3, 100) = (288.3, 100) with angle 0
    const newPiece = operations.appendTrackToConnector(p1, 1);
    expect(newPiece).not.toBeNull();
    expect(layout.pieces).toHaveLength(2);

    // Should be a 24188 by fallback to anchor catalogId
    expect(newPiece!.catalogId).toBe("24188");
    expect(selection.all()).toEqual([newPiece!.id]);

    // Position should match connector 1 of p1
    expect(newPiece!.transform.x).toBeCloseTo(288.3, 1);
    expect(newPiece!.transform.y).toBeCloseTo(100, 1);
    expect(newPiece!.transform.rotation).toBeCloseTo(0, 1);

    // Mutual connection should be established
    expect(p1.connectors[1]!.connectedTo?.pieceId).toBe(newPiece!.id);
    expect(newPiece!.connectors[0]!.connectedTo?.pieceId).toBe(p1.id);

    // Undo should cleanly revert
    history.undo();
    updatePieceConnections(layout.pieces, layout.layers);
    expect(layout.pieces).toHaveLength(1);
    expect(p1.connectors[1]!.connectedTo).toBeUndefined();

    // Redo should restore
    history.redo();
    updatePieceConnections(layout.pieces, layout.layers);
    expect(layout.pieces).toHaveLength(2);
    expect(p1.connectors[1]!.connectedTo?.pieceId).toBe(newPiece!.id);
  });

  it("places the last placed track when lastPlacedCatalogId is set", () => {
    const entry24188 = maerklinCCatalog.entries.find((e) => e.id === "24188")!;
    const entry24130 = maerklinCCatalog.entries.find((e) => e.id === "24130")!; // Curve R1 30°

    const p1: TrackPiece = {
      id: "p-1",
      catalogId: "24188",
      layerId: "layer-1",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: computeWorldConnectors(entry24188, { x: 100, y: 100, rotation: 0 }),
    };

    const { layout, operations } = createMockEditor([p1]);

    // Simulate placing a 24130 curve previously
    operations.setLastPlacedCatalogId("24130");

    const newPiece = operations.appendTrackToConnector(p1, 1);
    expect(newPiece).not.toBeNull();
    expect(newPiece!.catalogId).toBe("24130");
    expect(layout.pieces).toHaveLength(2);
    expect(p1.connectors[1]!.connectedTo?.pieceId).toBe(newPiece!.id);
  });

  it("appends to connector 0 (rear) with opposing orientation", () => {
    const entry24188 = maerklinCCatalog.entries.find((e) => e.id === "24188")!;
    const p1: TrackPiece = {
      id: "p-1",
      catalogId: "24188",
      layerId: "layer-1",
      transform: { x: 200, y: 100, rotation: 0 },
      connectors: computeWorldConnectors(entry24188, { x: 200, y: 100, rotation: 0 }),
    };

    const { layout, operations } = createMockEditor([p1]);

    // Connector 0 is at (200, 100) pointing left (180°)
    const newPiece = operations.appendTrackToConnector(p1, 0);
    expect(newPiece).not.toBeNull();
    expect(layout.pieces).toHaveLength(2);

    // New piece should be oriented at 180° extending leftwards
    expect(newPiece!.transform.x).toBeCloseTo(200, 1);
    expect(newPiece!.transform.y).toBeCloseTo(100, 1);
    expect(newPiece!.transform.rotation).toBeCloseTo(180, 1);

    expect(p1.connectors[0]!.connectedTo?.pieceId).toBe(newPiece!.id);
  });

  it("returns null if connector is already connected", () => {
    const entry24188 = maerklinCCatalog.entries.find((e) => e.id === "24188")!;
    const p1: TrackPiece = {
      id: "p-1",
      catalogId: "24188",
      layerId: "layer-1",
      transform: { x: 100, y: 100, rotation: 0 },
      connectors: computeWorldConnectors(entry24188, { x: 100, y: 100, rotation: 0 }),
    };
    p1.connectors[1]!.connectedTo = { pieceId: "other", connectorIndex: 0 };

    const { operations } = createMockEditor([p1]);
    const res = operations.appendTrackToConnector(p1, 1);
    expect(res).toBeNull();
  });

  it("smoothly continues curve curvature", () => {
    const entry24130 = maerklinCCatalog.entries.find((e) => e.id === "24130")!; // Curve R1 30°
    const p1: TrackPiece = {
      id: "c-1",
      catalogId: "24130",
      layerId: "layer-1",
      transform: { x: 300, y: 300, rotation: 0 },
      connectors: computeWorldConnectors(entry24130, { x: 300, y: 300, rotation: 0 }),
    };

    const { layout, operations } = createMockEditor([p1]);
    operations.setLastPlacedCatalogId("24130");

    const p2 = operations.appendTrackToConnector(p1, 1);
    expect(p2).not.toBeNull();
    expect(layout.pieces).toHaveLength(2);
    expect(p1.connectors[1]!.connectedTo?.pieceId).toBe(p2!.id);

    // Angle of p1 connector 1 is 30°. p2 should be rotated by 30° to smoothly continue the arc
    expect(p2!.transform.rotation).toBeCloseTo(30, 1);
  });

  it("places add (+) button further away (28mm) and allows dragging flex track connector handle", () => {
    const flexEntry: CatalogEntry = {
      id: "24900",
      name: "Flexgleis",
      kind: "flex",
      category: "Flexgleise",
      description: "Flexibles Gleis 900 mm",
      length: 900,
      gauge: 16.5,
    };
    const pFlex: TrackPiece = {
      id: "flex-1",
      catalogId: "24900",
      layerId: "layer-1",
      customLength: 900,
      transform: { x: 0, y: 0, rotation: 0 },
      connectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 900, y: 0, angle: 0 },
      ],
    };

    const layout: Layout = {
      name: "Flex Layout",
      scale: "H0",
      pieces: [pFlex],
      layers: [{ id: "layer-1", name: "Ebene 1", color: "#3b82f6", visible: true, locked: false, heightMm: 0 }],
      baseboards: [],
      models: [],
    };
    const selection = new Selection();
    selection.set("flex-1");

    // 1. Klick direkt auf den Connector-Endpunkt (900, 0): darf KEIN Plus-Treffer sein
    const connHit = findOpenConnectorAt(
      layout,
      selection,
      () => true,
      { x: 900, y: 0 },
      1.0,
      () => flexEntry,
    );
    expect(connHit).toBeNull();

    // 2. Klick auf den abgesetzten Plus-Button bei (900 + 28, 0): wird als Plus-Button erkannt
    const plusHit = findOpenConnectorAt(
      layout,
      selection,
      () => true,
      { x: 928, y: 0 },
      1.0,
      () => flexEntry,
    );
    expect(plusHit).not.toBeNull();
    expect(plusHit?.piece.id).toBe("flex-1");
    expect(plusHit?.connectorIndex).toBe(1);

    // 3. Klick auf das offene Ende triggert den Ziehgriff zum Biegen/Kürzen/Strecken
    const flexHandleHit = checkFlexHandleHit(
      layout,
      selection,
      () => true,
      () => flexEntry,
      { x: 900, y: 0 },
      15,
    );
    expect(flexHandleHit).not.toBeNull();
    expect(flexHandleHit?.piece.id).toBe("flex-1");
  });
});
