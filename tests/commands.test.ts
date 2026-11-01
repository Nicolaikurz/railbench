import { describe, it, expect } from "vitest";
import {
  History,
  AddPiecesCommand,
  ToggleCatenaryCommand,
  ToggleCatenaryFeederCommand,
  ToggleCatenaryIsolatorCommand,
} from "@/editor/commands";
import type { Layout, TrackPiece, Model3D } from "@/core/types";

describe("AddPiecesCommand", () => {
  function makeSampleLayout(): Layout {
    return {
      name: "Test Layout",
      scale: "H0",
      pieces: [
        {
          id: "p-1",
          catalogId: "24188",
          layerId: "default",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [],
        },
      ],
      layers: [{ id: "default", name: "Default", color: "#3b82f6", visible: true, locked: false, heightMm: 0 }],
      baseboards: [],
      models: [],
    };
  }

  it("should add multiple pieces atomically and support single undo/redo", () => {
    const layout = makeSampleLayout();
    const history = new History(layout);

    const pastedPiece1: TrackPiece = {
      id: "p-2",
      catalogId: "24188",
      layerId: "default",
      transform: { x: 40, y: 40, rotation: 0 },
      connectors: [],
    };
    const pastedPiece2: TrackPiece = {
      id: "p-3",
      catalogId: "24172",
      layerId: "default",
      transform: { x: 80, y: 80, rotation: 0 },
      connectors: [],
    };

    history.run(new AddPiecesCommand([pastedPiece1, pastedPiece2]));
    expect(layout.pieces).toHaveLength(3);
    expect(layout.pieces.map((p) => p.id)).toEqual(["p-1", "p-2", "p-3"]);

    history.undo();
    expect(layout.pieces).toHaveLength(1);
    expect(layout.pieces[0]!.id).toBe("p-1");

    history.redo();
    expect(layout.pieces).toHaveLength(3);
    expect(layout.pieces.map((p) => p.id)).toEqual(["p-1", "p-2", "p-3"]);
  });

  it("should add multiple models atomically alongside tracks and undo/redo cleanly", () => {
    const layout = makeSampleLayout();
    const history = new History(layout);

    const pastedPiece: TrackPiece = {
      id: "p-4",
      catalogId: "24188",
      layerId: "default",
      transform: { x: 50, y: 50, rotation: 0 },
      connectors: [],
    };
    const pastedModel: Model3D = {
      id: "m-1",
      name: "Building",
      category: "buildings",
      modelType: "glb",
      fileData: "data:model/gltf-binary;base64,AAA",
      layerId: "default",
      transform: { x: 50, y: 50, rotation: 0 },
      scale: 1,
      width: 100,
      depth: 100,
      height: 80,
    };

    history.run(new AddPiecesCommand([pastedPiece], [pastedModel]));
    expect(layout.pieces).toHaveLength(2);
    expect(layout.models).toHaveLength(1);

    history.undo();
    expect(layout.pieces).toHaveLength(1);
    expect(layout.models).toHaveLength(0);

    history.redo();
    expect(layout.pieces).toHaveLength(2);
    expect(layout.models).toHaveLength(1);
  });

  it("should support clipboard JSON serialization and offset replication", () => {
    const layout = makeSampleLayout();
    const history = new History(layout);

    const clipData = {
      pieces: [JSON.parse(JSON.stringify(layout.pieces[0]))],
      models: [],
    };
    const serialized = JSON.stringify(clipData);
    expect(serialized).toContain("24188");

    const parsed = JSON.parse(serialized);
    const newPiece: TrackPiece = {
      ...parsed.pieces[0],
      id: "p-copied-1",
      transform: {
        x: parsed.pieces[0].transform.x + 40,
        y: parsed.pieces[0].transform.y + 40,
        rotation: parsed.pieces[0].transform.rotation,
      },
    };

    history.run(new AddPiecesCommand([newPiece]));
    expect(layout.pieces).toHaveLength(2);
    expect(layout.pieces[1]!.id).toBe("p-copied-1");
    expect(layout.pieces[1]!.transform.x).toBe(40);
    expect(layout.pieces[1]!.transform.y).toBe(40);
  });

  it("should toggle catenary on tracks atomically with undo/redo", () => {
    const layout = makeSampleLayout();
    const history = new History(layout);

    expect(layout.pieces[0]!.hasCatenary).toBeFalsy();

    // Enable catenary
    history.run(new ToggleCatenaryCommand(["p-1"]));
    expect(layout.pieces[0]!.hasCatenary).toBe(true);

    // Undo -> off
    history.undo();
    expect(layout.pieces[0]!.hasCatenary).toBe(false);

    // Redo -> on
    history.redo();
    expect(layout.pieces[0]!.hasCatenary).toBe(true);

    // Toggle again -> off
    history.run(new ToggleCatenaryCommand(["p-1"]));
    expect(layout.pieces[0]!.hasCatenary).toBe(false);
  });

  it("should toggle catenary power feeder atomically and ensure catenary is active", () => {
    const layout = makeSampleLayout();
    const history = new History(layout);

    expect(layout.pieces[0]!.catenaryPowerFeed).toBeFalsy();
    expect(layout.pieces[0]!.hasCatenary).toBeFalsy();

    // Toggle catenary power feeder on -> should also activate hasCatenary
    history.run(new ToggleCatenaryFeederCommand(["p-1"]));
    expect(layout.pieces[0]!.catenaryPowerFeed).toBe(true);
    expect(layout.pieces[0]!.hasCatenary).toBe(true);

    // Undo -> reverts both
    history.undo();
    expect(layout.pieces[0]!.catenaryPowerFeed).toBeFalsy();
    expect(layout.pieces[0]!.hasCatenary).toBeFalsy();

    // Redo -> activates both
    history.redo();
    expect(layout.pieces[0]!.catenaryPowerFeed).toBe(true);
    expect(layout.pieces[0]!.hasCatenary).toBe(true);

    // Toggle again -> turns off catenaryPowerFeed
    history.run(new ToggleCatenaryFeederCommand(["p-1"]));
    expect(layout.pieces[0]!.catenaryPowerFeed).toBe(false);
  });

  it("should cycle catenary isolator through connectors and undo/redo cleanly", () => {
    const layout = makeSampleLayout();
    layout.pieces[0]!.connectors = [
      { x: 0, y: 0, angle: 180 },
      { x: 188.3, y: 0, angle: 0 },
    ];
    const history = new History(layout);

    // Step 1: Turn on at connector 0
    history.run(new ToggleCatenaryIsolatorCommand(["p-1"]));
    expect(layout.pieces[0]!.catenaryIsolator).toBe(true);
    expect(layout.pieces[0]!.catenaryIsolatedConnectorIndex).toBe(0);
    expect(layout.pieces[0]!.hasCatenary).toBe(true);

    // Step 2: Cycle to connector 1
    history.run(new ToggleCatenaryIsolatorCommand(["p-1"]));
    expect(layout.pieces[0]!.catenaryIsolator).toBe(true);
    expect(layout.pieces[0]!.catenaryIsolatedConnectorIndex).toBe(1);

    // Step 3: Cycle past connectors -> turns off
    history.run(new ToggleCatenaryIsolatorCommand(["p-1"]));
    expect(layout.pieces[0]!.catenaryIsolator).toBe(false);

    // Undo turns it back on at index 1
    history.undo();
    expect(layout.pieces[0]!.catenaryIsolator).toBe(true);
    expect(layout.pieces[0]!.catenaryIsolatedConnectorIndex).toBe(1);
  });
});
