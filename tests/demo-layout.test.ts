import { describe, it, expect } from "vitest";
import { createDemoLayout, initDemoTracks, DEMO_LAYOUT_NAME } from "@/editor/demo-tracks";
import { createLayout } from "@/core/layout";
import { toJSON, fromJSON } from "@/io/serialize";
import { dist } from "@/core/geometry";
import rawDemoLayout from "@/editor/demo-layout.railbench?raw";

describe("Demo Layout", () => {
  it("lädt den Plan direkt aus der .railbench JSON-Datei", () => {
    expect(typeof rawDemoLayout).toBe("string");
    const parsed = JSON.parse(rawDemoLayout);
    expect(parsed.name).toBe(DEMO_LAYOUT_NAME);
    expect(parsed.pieces.length).toBe(21);
  });

  it("erzeugt das vollständige Alpenland H0 Demo-Layout", () => {
    const layout = createDemoLayout();

    expect(layout.name).toBe(DEMO_LAYOUT_NAME);
    expect(layout.scale).toBe("H0");
    expect(layout.feedVoltage).toBe(16);
    expect(layout.pieces.length).toBe(21);
    expect(layout.layers?.length).toBe(3);
    expect(layout.baseboard?.width).toBe(1800);
    expect(layout.baseboard?.depth).toBe(1000);
    expect(layout.models?.length).toBe(2);
  });

  it("schließt das Hauptoval nahtlos ohne Lücke", () => {
    const layout = createDemoLayout();
    const firstPiece = layout.pieces[0]!;
    const lastLoopPiece = layout.pieces[17]!;

    const firstConn = firstPiece.connectors[0]!;
    const lastConn = lastLoopPiece.connectors[1]!;

    const gap = dist(firstConn, lastConn);
    expect(gap).toBeLessThan(0.01);

    // Gegenseitige Verbindung prüfen
    expect(firstConn.connectedTo).toEqual({ pieceId: lastLoopPiece.id, connectorIndex: 1 });
    expect(lastConn.connectedTo).toEqual({ pieceId: firstPiece.id, connectorIndex: 0 });
  });

  it("enthält Einspeisung, Weichenbeleuchtung und Decoder", () => {
    const layout = createDemoLayout();
    const feeder = layout.pieces.find((p) => p.id === "demo-feed");
    const turnout = layout.pieces.find((p) => p.id === "demo-turnout");
    const bumper = layout.pieces.find((p) => p.id === "demo-bumper");

    expect(feeder?.powerFeed).toBe(true);
    expect(turnout?.hasLantern).toBe(true);
    expect(turnout?.hasDecoder).toBe(true);
    expect(turnout?.turnoutCurrentMa).toBe(35);
    expect(bumper?.catalogId).toBe("24977");
  });

  it("verbindet die Weiche mit dem Abstellgleis bis zum Prellbock", () => {
    const layout = createDemoLayout();
    const turnout = layout.pieces.find((p) => p.id === "demo-turnout")!;
    const siding1 = layout.pieces.find((p) => p.id === "demo-siding-1")!;
    const siding2 = layout.pieces.find((p) => p.id === "demo-siding-2")!;
    const bumper = layout.pieces.find((p) => p.id === "demo-bumper")!;

    expect(turnout.connectors[2]?.connectedTo).toEqual({ pieceId: siding1.id, connectorIndex: 0 });
    expect(siding1.connectors[0]?.connectedTo).toEqual({ pieceId: turnout.id, connectorIndex: 2 });

    expect(siding1.connectors[1]?.connectedTo).toEqual({ pieceId: siding2.id, connectorIndex: 0 });
    expect(siding2.connectors[0]?.connectedTo).toEqual({ pieceId: siding1.id, connectorIndex: 1 });

    expect(siding2.connectors[1]?.connectedTo).toEqual({ pieceId: bumper.id, connectorIndex: 0 });
    expect(bumper.connectors[0]?.connectedTo).toEqual({ pieceId: siding2.id, connectorIndex: 1 });
  });

  it("lässt sich fehlerfrei serialisieren und deserialisieren", () => {
    const original = createDemoLayout();
    const json = toJSON(original);
    const restored = fromJSON(json);

    expect(restored.name).toBe(DEMO_LAYOUT_NAME);
    expect(restored.pieces.length).toBe(21);
    expect(restored.layers?.length).toBe(3);
    expect(restored.models?.length).toBe(2);
    expect(restored.baseboard?.width).toBe(1800);
  });

  it("initDemoTracks überschreibt ein leeres Layout mit dem vollständigen Demo-Plan", () => {
    const emptyLayout = createLayout("New Layout");
    initDemoTracks(emptyLayout);

    expect(emptyLayout.name).toBe(DEMO_LAYOUT_NAME);
    expect(emptyLayout.pieces.length).toBe(21);
    expect(emptyLayout.baseboard?.width).toBe(1800);
  });
});
