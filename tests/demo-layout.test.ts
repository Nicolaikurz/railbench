import { describe, it, expect, vi } from "vitest";
import {
  createDemoLayout,
  initDemoTracks,
  DEMO_LAYOUT_NAME,
  DEMO_LAYOUTS,
  getDemoLayout,
} from "@/editor/demo-tracks";
import { showDemoTracksDialog } from "@/ui/demo-tracks-dialog";
import { createLayout } from "@/core/layout";
import { toJSON, fromJSON } from "@/io/serialize";
import { dist } from "@/core/geometry";
import rawDemoLayout from "../demotracks/alpenland-h0.railbench?raw";
import rawMainlineH0 from "../demotracks/mainline-h0.railbench?raw";
import rawSwitchingN from "../demotracks/switching-n.railbench?raw";
import rawGardenG from "../demotracks/garden-g.railbench?raw";
import rawShunting0 from "../demotracks/shunting-0.railbench?raw";
import rawSuitcaseZ from "../demotracks/suitcase-z.railbench?raw";
import { isFeederEntry, isIsolatingEntry } from "@/catalog/types";
import { TrackOperations, type EditorContext } from "@/editor/track-operations";
import { catalogRegistry } from "@/catalog/catalog";
import { History } from "@/editor/commands";
import { Selection } from "@/editor/selection";
import type { Layout } from "@/core/types";

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

  it("stellt 6 verschiedene Demo-Layouts über DEMO_LAYOUTS bereit", () => {
    expect(DEMO_LAYOUTS.length).toBe(6);
    const ids = DEMO_LAYOUTS.map((d) => d.id);
    expect(ids).toContain("alpenland-h0");
    expect(ids).toContain("mainline-h0");
    expect(ids).toContain("switching-n");
    expect(ids).toContain("garden-g");
    expect(ids).toContain("shunting-0");
    expect(ids).toContain("suitcase-z");

    const scales = new Set(DEMO_LAYOUTS.map((d) => d.scale));
    expect(scales.has("H0")).toBe(true);
    expect(scales.has("N")).toBe(true);
    expect(scales.has("G")).toBe(true);
    expect(scales.has("0")).toBe(true);
    expect(scales.has("Z")).toBe(true);
  });

  it("erzeugt jedes Demo-Layout fehlerfrei aus dem Ordner demotracks/ ohne unverbundene Gleisenden", () => {
    const rawFiles: Record<string, string> = {
      "alpenland-h0": rawDemoLayout,
      "mainline-h0": rawMainlineH0,
      "switching-n": rawSwitchingN,
      "garden-g": rawGardenG,
      "shunting-0": rawShunting0,
      "suitcase-z": rawSuitcaseZ,
    };

    for (const demo of DEMO_LAYOUTS) {
      expect(typeof rawFiles[demo.id]).toBe("string");
      const def = getDemoLayout(demo.id);
      expect(def).toBeDefined();
      expect(def!.scale).toBe(demo.scale);
      const layout = def!.createLayout();
      expect(layout).toBeDefined();
      expect(layout.scale).toBe(demo.scale);
      expect(layout.pieces.length).toBe(demo.pieceCount);
      expect(layout.baseboards?.length).toBeGreaterThan(0);
      expect(layout.baseboard?.positionMode).toBe("center");
      expect(layout.baseboard?.width).toBe(demo.dimensionsMm.width);
      expect(layout.baseboard?.depth).toBe(demo.dimensionsMm.height);
      expect(layout.layers?.length).toBeGreaterThan(0);
      expect(layout.models?.length).toBeGreaterThan(0);

      // Jedes Gleisstück in jedem Demo-Layout ist an allen Anschlüssen lückenlos verbunden
      const unconnected = layout.pieces.flatMap((p) =>
        p.connectors.filter((c) => !c.connectedTo),
      );
      expect(unconnected).toHaveLength(0);
    }
  });

  it("erkennt Feeder- und Isoliergleise zuverlässig mit isFeederEntry und isIsolatingEntry", () => {
    const cTrackCatalog = catalogRegistry.get("maerklin-c-h0");
    const zTrackCatalog = catalogRegistry.get("maerklin-z-miniclub");
    expect(cTrackCatalog).toBeDefined();
    expect(zTrackCatalog).toBeDefined();

    // 24088 ist ein Anschlussgleis (Feeder)
    const feederEntry = cTrackCatalog?.entries.find((e) => e.id === "24088");
    expect(feederEntry).toBeDefined();
    expect(isFeederEntry(feederEntry)).toBe(true);
    expect(isIsolatingEntry(feederEntry)).toBe(false);

    // 8588 ist ein Trenngleis (Isolator) in Spur Z
    const isoEntry = zTrackCatalog?.entries.find((e) => e.id === "8588");
    expect(isoEntry).toBeDefined();
    expect(isIsolatingEntry(isoEntry)).toBe(true);
    expect(isFeederEntry(isoEntry)).toBe(false);

    // Normales Gleis 24188
    const normalEntry = cTrackCatalog?.entries.find((e) => e.id === "24188");
    expect(normalEntry).toBeDefined();
    expect(isFeederEntry(normalEntry)).toBe(false);
    expect(isIsolatingEntry(normalEntry)).toBe(false);
  });

  it("TrackOperations initialisiert powerFeed und isIsolator entsprechend Initialstatus", () => {
    let layout: Layout = createLayout("Test Layout");
    let history = new History(layout);
    let selection = new Selection();
    const cTrackCatalog = catalogRegistry.get("maerklin-c-h0")!;
    const zTrackCatalog = catalogRegistry.get("maerklin-z-miniclub")!;
    const feederEntry = cTrackCatalog.entries.find((e) => e.id === "24088")!;
    const isoEntry = zTrackCatalog.entries.find((e) => e.id === "8588")!;
    const normalEntry = cTrackCatalog.entries.find((e) => e.id === "24188")!;

    const ctx: EditorContext = {
      getLayout: () => layout,
      setLayout: (l: Layout) => { layout = l; },
      getCatalog: () => cTrackCatalog,
      getSelection: () => selection,
      getHistory: () => history,
      setHistory: (h: History) => { history = h; },
      getRenderer: () => ({} as any),
      getPlanManager: () => ({} as any),
      isSnappingEnabled: () => false,
      isPieceInteractive: () => true,
      isModelInteractive: () => true,
      getCatalogEntry: (p) =>
        cTrackCatalog.entries.find((e) => e.id === p.catalogId) ||
        zTrackCatalog.entries.find((e) => e.id === p.catalogId),
      sync: vi.fn(),
      showToast: vi.fn(),
    };
    const ops = new TrackOperations(ctx);

    // 1. Zuerst ein normales Gleis platzieren, damit das nächste nicht als erstes Gleis automatisch powerFeed erhält
    ops.addPieceToLayout(normalEntry, { x: 50, y: 50 });
    expect(layout.pieces.length).toBe(1);
    expect(layout.pieces[0]?.powerFeed).toBe(true); // Erstes Gleis im leeren Layout

    // 2. Feeder mit aktiviertem Toggle hinzugefügt
    const feederActive = { ...feederEntry, _initialPowerFeed: true } as any;
    ops.addPieceToLayout(feederActive, { x: 100, y: 100 });
    const piece2 = layout.pieces[1]!;
    expect(piece2.powerFeed).toBe(true);

    // 3. Feeder mit deaktiviertem Toggle hinzugefügt (_initialPowerFeed: false)
    const feederInactive = { ...feederEntry, _initialPowerFeed: false } as any;
    ops.addPieceToLayout(feederInactive, { x: 200, y: 100 });
    const piece3 = layout.pieces[2]!;
    expect(piece3.powerFeed).toBe(false);

    // 4. Isoliergleis mit aktiviertem Toggle hinzugefügt (_initialIsolator: true)
    const isoActive = { ...isoEntry, _initialIsolator: true } as any;
    ops.addPieceToLayout(isoActive, { x: 300, y: 100 });
    const piece4 = layout.pieces[3]!;
    expect(piece4.isIsolator).toBe(true);
    expect(piece4.isolatedConnectorIndex).toBe(0);

    // 5. Isoliergleis mit deaktiviertem Toggle hinzugefügt (_initialIsolator: false)
    const isoInactive = { ...isoEntry, _initialIsolator: false } as any;
    ops.addPieceToLayout(isoInactive, { x: 400, y: 100 });
    const piece5 = layout.pieces[4]!;
    expect(piece5.isIsolator).toBeUndefined();

    // 6. Normales zweites Gleis
    ops.addPieceToLayout(normalEntry, { x: 500, y: 100 });
    const piece6 = layout.pieces[5]!;
    expect(piece6.powerFeed).toBe(false);
    expect(piece6.isIsolator).toBeUndefined();
  });

  it("zeigt das vereinfachte Demo-Menü an und lädt den gewählten Plan per Klick", async () => {
    const items: Array<{ dataset: Record<string, string>; click: () => void }> = [];
    let overlayRemoved = false;

    const createEl = (tag: string): any => {
      const listeners: Record<string, Array<(e?: any) => void>> = {};
      const el: any = {
        tagName: tag.toUpperCase(),
        id: "",
        className: "",
        type: "",
        innerHTML: "",
        dataset: {},
        children: [] as any[],
        setAttribute: vi.fn(),
        addEventListener: (ev: string, fn: (e?: any) => void) => {
          (listeners[ev] ??= []).push(fn);
        },
        click: () => {
          for (const fn of listeners["click"] ?? []) fn({ target: el });
        },
        appendChild: (child: any) => {
          el.children.push(child);
          if (child.className === "demo-track-item") items.push(child);
        },
        append: (...children: any[]) => {
          el.children.push(...children);
        },
        querySelector: () => createEl("button"),
        remove: () => {
          overlayRemoved = true;
        },
      };
      return el;
    };

    const prevDoc = (globalThis as any).document;
    (globalThis as any).document = {
      getElementById: () => null,
      createElement: (tag: string) => createEl(tag),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      body: { appendChild: vi.fn() },
    };

    try {
      const onSelect = vi.fn();
      const promise = showDemoTracksDialog({ onSelect });

      expect(items.length).toBe(6);
      expect(items.map((i) => i.dataset.demoId)).toEqual(DEMO_LAYOUTS.map((d) => d.id));

      items[1]!.click();
      const selected = await promise;
      expect(selected?.id).toBe("mainline-h0");
      expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ id: "mainline-h0" }));
      expect(overlayRemoved).toBe(true);
    } finally {
      (globalThis as any).document = prevDoc;
    }
  });
});
