import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { Layout } from "@/core/types";
import { Sidebar } from "@/ui/sidebar";
import { maerklinCCatalog } from "@/catalog/catalog";
import { renderCatalogTab } from "@/ui/sidebar/catalog-tab";
import { renderRoadsTab } from "@/ui/sidebar/roads-tab";
import { renderModelsTab } from "@/ui/sidebar/models-tab";
import { renderLayersTab } from "@/ui/sidebar/layers-tab";
import { renderBoardTab, syncWoodMode, makePlate } from "@/ui/sidebar/board-tab";
import { renderBomTab } from "@/ui/sidebar/bom-tab";
import { getPieceListSummary } from "@/io/export";

describe("Sidebar Tab Splitting & Modular Tabs", () => {
  let sampleLayout: Layout;
  let mockHost: HTMLElement;
  let originalDocument: any;
  let originalWindow: any;

  beforeEach(() => {
    originalDocument = (globalThis as any).document;
    originalWindow = (globalThis as any).window;

    (globalThis as any).window = {
      addEventListener: () => {},
      removeEventListener: () => {},
    };

    (globalThis as any).document = {
      head: { appendChild: () => {} },
      getElementById: () => null,
      createElement: (tag: string) => {
        const el: any = {
          tagName: tag.toUpperCase(),
          className: "",
          style: {},
          children: [] as any[],
          setAttribute: () => {},
          getAttribute: () => null,
          appendChild: (c: any) => { el.children.push(c); return c; },
          append: (...items: any[]) => { el.children.push(...items); },
          addEventListener: () => {},
          removeEventListener: () => {},
          querySelector: () => null,
          querySelectorAll: () => [],
          classList: { add: () => {}, remove: () => {}, contains: () => false },
          getContext: () => null,
        };
        return el;
      },
    };

    mockHost = (globalThis as any).document.createElement("div");

    sampleLayout = {
      name: "Test Layout",
      scale: "H0",
      pieces: [],
      baseboard: {
        enabled: true,
        shape: "rect",
        width: 2400,
        depth: 1200,
        positionMode: "center",
        style: "wood",
      },
      layers: [
        { id: "l1", name: "Grundebene", heightMm: 0, visible: true, locked: false, color: "#38bdf8" },
      ],
      activeLayerId: "l1",
    };
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
    (globalThis as any).window = originalWindow;
  });

  it("renders catalog, roads, models, board, bom, and layers tabs via their separate modules", () => {
    const sidebar = new Sidebar(mockHost, maerklinCCatalog, sampleLayout, {
      onExportCSV: vi.fn(),
    });

    expect(sidebar.getActiveTab()).toBe("catalog");

    // Roads tab
    sidebar.setActiveTab("roads");
    expect(sidebar.getActiveTab()).toBe("roads");

    // Models tab
    sidebar.setActiveTab("models");
    expect(sidebar.getActiveTab()).toBe("models");

    // Board tab
    sidebar.setActiveTab("board");
    expect(sidebar.getActiveTab()).toBe("board");
    expect(sidebar.isBaseboardTabActive()).toBe(true);

    // BOM tab
    sidebar.setActiveTab("bom");
    expect(sidebar.getActiveTab()).toBe("bom");

    // Direct invocation of each tab render module
    const container = (globalThis as any).document.createElement("div");

    expect(() => renderCatalogTab(sidebar, container)).not.toThrow();
    expect(() => renderRoadsTab(sidebar, container)).not.toThrow();
    expect(() => renderModelsTab(sidebar, container)).not.toThrow();
    expect(() => renderLayersTab(sidebar, container)).not.toThrow();
    expect(() => renderBoardTab(sidebar, container)).not.toThrow();

    const bomSummary = getPieceListSummary(sidebar.layout, sidebar.catalog);
    expect(() => renderBomTab(sidebar, container, bomSummary)).not.toThrow();
  });

  it("does not cause the ground table plate (height 0) to disappear when a plate is added to another layer", () => {
    const groundLayer: any = { id: "l1", name: "Grundebene (0 mm)", heightMm: 0, woodPlateMode: "table", plates: [] };
    const upperLayer: any = { id: "l2", name: "Ebene 2 (+80 mm)", heightMm: 80, woodPlateMode: "none", plates: [] };
    const layers = [groundLayer, upperLayer];

    // Simulate adding a plate to the upper layer
    const bb: any = { id: "bb-1", width: 2400, depth: 1200, shape: "rect" };
    upperLayer.plates.push(makePlate(bb, upperLayer, "Upper Deck Plate"));

    // Run syncWoodMode across all layers
    layers.forEach(syncWoodMode);

    // Upper layer should have table mode because it has plates
    expect(upperLayer.woodPlateMode).toBe("table");

    // Ground layer MUST keep "table" mode so the main table baseboard does not vanish
    expect(groundLayer.woodPlateMode).toBe("table");
  });

  it("supports texture/style option on plates matching the table", () => {
    const bb: any = { id: "bb-1", width: 2400, depth: 1200, shape: "rect", style: "dark-wood" };
    const layer: any = { id: "l2", name: "Ebene 2", heightMm: 80, woodPlateMode: "table", plates: [] };

    const plate = makePlate(bb, layer, "Texture Test Plate");
    // Should inherit the table's style
    expect(plate.style).toBe("dark-wood");

    // Can be changed to other styles
    plate.style = "grass";
    expect(plate.style).toBe("grass");
  });
});
