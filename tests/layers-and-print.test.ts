import { describe, it, expect } from "vitest";
import type { Layout, TrackPiece, Layer } from "@/core/types";
import { MovePiecesToLayerCommand } from "@/editor/commands";
import { sortLayers, ensureDefaultLayers } from "@/core/layout";
import { toXML } from "@/io/serialize";
import { getLayerPieceListSummary } from "@/io/export";
import { computeLayoutPrintBounds, buildPdfFromImages } from "@/io/print";
import { maerklinCCatalog } from "@/catalog/catalog";
import { LAYER_COLOR_PALETTE } from "@/ui/layer-dialog";

describe("Layer Height & Print Features", () => {
  const sampleLayout: Layout = {
    name: "Multi-Deck Layout",
    scale: "H0",
    feedVoltage: 16,
    layers: [
      { id: "layer-1", name: "Ground Level", heightMm: 0, visible: true, locked: false, color: "#38bdf8" },
      { id: "layer-2", name: "Mountain Level", heightMm: 80, visible: true, locked: false, color: "#f472b6" },
      { id: "layer-3", name: "Staging Yard", heightMm: -100, visible: true, locked: false, color: "#34d399" },
    ],
    activeLayerId: "layer-1",
    pieces: [
      {
        id: "p1",
        catalogId: "24188",
        transform: { x: 0, y: 0, rotation: 0 },
        connectors: [{ x: 0, y: 0, angle: 180 }, { x: 188.3, y: 0, angle: 0 }],
        layerId: "layer-1",
      },
      {
        id: "p2",
        catalogId: "24172",
        transform: { x: 188.3, y: 0, rotation: 0 },
        connectors: [{ x: 188.3, y: 0, angle: 180 }, { x: 360, y: 0, angle: 0 }],
        layerId: "layer-1",
      },
      {
        id: "p3",
        catalogId: "24224",
        transform: { x: 360, y: 0, rotation: 0 },
        connectors: [{ x: 360, y: 0, angle: 180 }, { x: 500, y: 50, angle: 24.3 }],
        layerId: "layer-2",
        elevation: { start: 0, end: 0 },
      },
    ],
  };

  it("MovePiecesToLayerCommand moves pieces and updates elevation", () => {
    const layout: Layout = JSON.parse(JSON.stringify(sampleLayout));
    const cmd = new MovePiecesToLayerCommand(["p1", "p2"], "layer-2", { start: 0, end: 0 });

    cmd.do(layout);
    expect(layout.pieces[0]!.layerId).toBe("layer-2");
    expect(layout.pieces[0]!.elevation).toEqual({ start: 0, end: 0 });
    expect(layout.pieces[1]!.layerId).toBe("layer-2");
    expect(layout.pieces[1]!.elevation).toEqual({ start: 0, end: 0 });

    // Undo restores previous states
    cmd.undo(layout);
    expect(layout.pieces[0]!.layerId).toBe("layer-1");
    expect(layout.pieces[0]!.elevation).toBeUndefined();
    expect(layout.pieces[1]!.layerId).toBe("layer-1");

    // Redo re-applies
    cmd.do(layout);
    expect(layout.pieces[0]!.layerId).toBe("layer-2");
    expect(layout.pieces[0]!.elevation).toEqual({ start: 0, end: 0 });
  });

  it("MovePiecesToLayerCommand supports custom incline / ramp elevation", () => {
    const layout: Layout = JSON.parse(JSON.stringify(sampleLayout));
    // Rampe von 0 auf 80mm
    const cmd = new MovePiecesToLayerCommand(["p1"], "layer-1", { start: 0, end: 80 });

    cmd.do(layout);
    expect(layout.pieces[0]!.elevation).toEqual({ start: 0, end: 80 });

    cmd.undo(layout);
    expect(layout.pieces[0]!.elevation).toBeUndefined();
  });

  it("serializes elevation into XML properly", () => {
    const layout: Layout = JSON.parse(JSON.stringify(sampleLayout));
    layout.pieces[0]!.elevation = { start: 10.5, end: 20.75 };

    const xml = toXML(layout);
    expect(xml).toContain('elevStart="10.50"');
    expect(xml).toContain('elevEnd="20.75"');
  });

  it("getLayerPieceListSummary isolates items for a specific layer", () => {
    const summaryL1 = getLayerPieceListSummary(sampleLayout, maerklinCCatalog, "layer-1");
    expect(summaryL1.totalPieces).toBe(2);
    expect(summaryL1.items.length).toBe(2);
    expect(summaryL1.items.some((it) => it.catalogId === "24188")).toBe(true);
    expect(summaryL1.items.some((it) => it.catalogId === "24172")).toBe(true);
    expect(summaryL1.items.some((it) => it.catalogId === "24224")).toBe(false);

    const summaryL2 = getLayerPieceListSummary(sampleLayout, maerklinCCatalog, "layer-2");
    expect(summaryL2.totalPieces).toBe(1);
    expect(summaryL2.items[0]?.catalogId).toBe("24224");

    const summaryL3 = getLayerPieceListSummary(sampleLayout, maerklinCCatalog, "layer-3");
    expect(summaryL3.totalPieces).toBe(0);
    expect(summaryL3.items.length).toBe(0);
  });

  it("supports layer renaming and persists renamed layer in XML", () => {
    const layout: Layout = JSON.parse(JSON.stringify(sampleLayout));
    const layer = layout.layers!.find((l) => l.id === "layer-1")!;
    layer.name = "Hauptbahnhof Ebene 0";

    const xml = toXML(layout);
    expect(xml).toContain('name="Hauptbahnhof Ebene 0"');
  });

  it("computeLayoutPrintBounds computes generous positive dimensions", () => {
    const bounds = computeLayoutPrintBounds(sampleLayout);
    expect(bounds.width).toBeGreaterThanOrEqual(800);
    expect(bounds.height).toBeGreaterThanOrEqual(500);
    expect(bounds.minX).toBeLessThan(0);
    expect(bounds.maxX).toBeGreaterThan(500);
  });

  it("buildPdfFromImages erzeugt eine gültige mehrseitige PDF-1.4 Datei", () => {
    const mockJpeg = new Uint8Array([
      0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xff, 0xd9
    ]);

    const pdfBytes = buildPdfFromImages(
      [
        { bytes: mockJpeg, width: 100, height: 100 },
        { bytes: mockJpeg, width: 100, height: 100 },
      ],
      true,
    );

    expect(pdfBytes).toBeInstanceOf(Uint8Array);
    const pdfText = new TextDecoder().decode(pdfBytes);
    expect(pdfText).toContain("%PDF-1.4");
    expect(pdfText).toContain("/Count 2");
    expect(pdfText).toContain("/MediaBox [ 0 0 841.89 595.28 ]");
    expect(pdfText).toContain("/Filter /DCTDecode");
    expect(pdfText).toContain("%%EOF");
  });

  it("provides rich curated color palette for new layers", () => {
    expect(LAYER_COLOR_PALETTE.length).toBeGreaterThanOrEqual(10);
    expect(LAYER_COLOR_PALETTE[0]).toBe("#38bdf8");
    for (const color of LAYER_COLOR_PALETTE) {
      expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
    }
  });

  it("sortLayers auto-sorts primarily by height (descending) and secondarily by name (ascending)", () => {
    const rawLayers: Layer[] = [
      { id: "l1", name: "Ground Level", heightMm: 0, visible: true, locked: false, color: "#38bdf8" },
      { id: "l2", name: "Mountain High", heightMm: 120, visible: true, locked: false, color: "#f472b6" },
      { id: "l3", name: "Staging Yard B", heightMm: -100, visible: true, locked: false, color: "#34d399" },
      { id: "l4", name: "Bridge Line Beta", heightMm: 50, visible: true, locked: false, color: "#fbbf24" },
      { id: "l5", name: "Bridge Line Alpha", heightMm: 50, visible: true, locked: false, color: "#2dd4bf" },
      { id: "l6", name: "Staging Yard A", heightMm: -100, visible: true, locked: false, color: "#ef4444" },
    ];

    const sorted = sortLayers(rawLayers);

    // Height descending:
    // 120 (Mountain High)
    // 50 (Bridge Line Alpha - 'Alpha' before 'Beta')
    // 50 (Bridge Line Beta)
    // 0 (Ground Level)
    // -100 (Staging Yard A - 'A' before 'B')
    // -100 (Staging Yard B)
    expect(sorted.map((l) => l.id)).toEqual(["l2", "l5", "l4", "l1", "l6", "l3"]);
    expect(sorted[1]!.name).toBe("Bridge Line Alpha");
    expect(sorted[2]!.name).toBe("Bridge Line Beta");
    expect(sorted[4]!.name).toBe("Staging Yard A");
    expect(sorted[5]!.name).toBe("Staging Yard B");
  });

  it("ensureDefaultLayers automatically sorts layout layers", () => {
    const testLayout: Layout = {
      name: "Sort Test",
      scale: "H0",
      pieces: [],
      layers: [
        { id: "l1", name: "Ground", heightMm: 0, visible: true, locked: false, color: "#38bdf8" },
        { id: "l2", name: "Upper Level", heightMm: 100, visible: true, locked: false, color: "#f472b6" },
      ],
    };

    ensureDefaultLayers(testLayout);
    expect(testLayout.layers![0]!.id).toBe("l2"); // Upper Level (100mm) at the top
    expect(testLayout.layers![1]!.id).toBe("l1"); // Ground (0mm)
  });
});

