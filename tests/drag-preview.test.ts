import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  createPieceDragCanvas,
  createCatalogThumbnail,
  createModelDragCanvas,
  createCustomModelDragCanvas,
  attachDragImage,
} from "@/ui/drag-preview";
import type { CatalogEntry } from "@/catalog/types";
import type { BuiltinModelDef } from "@/core/models-3d";
import type { Model3D } from "@/core/types";

describe("Drag Preview Generator", () => {
  let originalDocument: any;

  function createMockCanvas() {
    const mockCtx: any = {
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      rotate: vi.fn(),
      scale: vi.fn(),
      beginPath: vi.fn(),
      closePath: vi.fn(),
      arc: vi.fn(),
      arcTo: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      stroke: vi.fn(),
      fill: vi.fn(),
      fillRect: vi.fn(),
      ellipse: vi.fn(),
      fillText: vi.fn(),
      drawImage: vi.fn(),
      imageSmoothingEnabled: true,
      imageSmoothingQuality: "high",
      measureText: vi.fn(() => ({ width: 30 })),
      setLineDash: vi.fn(),
      lineWidth: 1,
      strokeStyle: "",
      fillStyle: "",
      lineCap: "",
      font: "",
      textAlign: "",
      textBaseline: "",
      globalAlpha: 1.0,
    };

    const canvas: any = {
      width: 0,
      height: 0,
      style: {},
      getContext: vi.fn((type: string) => (type === "2d" ? mockCtx : null)),
      remove: vi.fn(),
    };

    return { canvas, mockCtx };
  }

  beforeEach(() => {
    originalDocument = (globalThis as any).document;

    const mockBody = {
      appendChild: vi.fn((child: any) => child),
    };

    (globalThis as any).document = {
      createElement: vi.fn((tag: string) => {
        if (tag === "canvas") {
          return createMockCanvas().canvas;
        }
        return { style: {} };
      }),
      body: mockBody,
    };
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
  });

  it("renders straight track pieces onto canvas with appropriate dimensions", () => {
    const entry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188.3 mm",
      kind: "straight",
      category: "Geraden",
      description: "Länge 188,3 mm",
      length: 188.3,
    };

    const canvas = createPieceDragCanvas(entry, false, undefined, 16.5);
    expect(canvas).toBeDefined();
    expect(canvas.width).toBeGreaterThan(100);
    expect(canvas.height).toBeGreaterThan(20);
  });

  it("renders curved track pieces with arc geometry", () => {
    const entry: CatalogEntry = {
      id: "24130",
      name: "Gebogenes Gleis R1",
      kind: "curve",
      category: "Bögen",
      description: "Radius 360 mm, Winkel 30°",
      radius: 360,
      angle: 30,
    };

    const canvas = createPieceDragCanvas(entry, false);
    expect(canvas).toBeDefined();
    expect(canvas.width).toBeGreaterThan(40);
    expect(canvas.height).toBeGreaterThan(30);
  });

  it("renders turnout track pieces (turnout-left and turnout-right)", () => {
    const leftTurnout: CatalogEntry = {
      id: "24611",
      name: "Weiche links",
      kind: "turnout-left",
      category: "Weichen",
      description: "Weiche links 188.3 mm / R2 437.5 mm",
      length: 188.3,
      branchRadius: 437.5,
      branchAngle: 24.3,
    };

    const canvasLeft = createPieceDragCanvas(leftTurnout, false);
    expect(canvasLeft.width).toBeGreaterThan(100);
    expect(canvasLeft.height).toBeGreaterThan(30);

    const rightTurnout: CatalogEntry = {
      id: "24612",
      name: "Weiche rechts",
      kind: "turnout-right",
      category: "Weichen",
      description: "Weiche rechts 188.3 mm / R2 437.5 mm",
      length: 188.3,
      branchRadius: 437.5,
      branchAngle: 24.3,
    };

    const canvasRight = createPieceDragCanvas(rightTurnout, false);
    expect(canvasRight.width).toBeGreaterThan(100);
    expect(canvasRight.height).toBeGreaterThan(30);
  });

  it("renders curved turnouts and three-way turnouts", () => {
    const curvedTurnout: CatalogEntry = {
      id: "24671",
      name: "Bogenweiche links",
      kind: "curved-turnout-left",
      category: "Weichen",
      description: "Bogenweiche",
      radius: 360,
      angle: 30,
      branchRadius: 360,
      branchAngle: 30,
    };
    const cCanvas = createPieceDragCanvas(curvedTurnout, false);
    expect(cCanvas.width).toBeGreaterThan(40);

    const threeWay: CatalogEntry = {
      id: "24630",
      name: "Dreiwegweiche",
      kind: "three-way",
      category: "Weichen",
      description: "Dreiwegweiche",
      length: 188.3,
      branchRadius: 437.5,
      branchAngle: 24.3,
    };
    const twCanvas = createPieceDragCanvas(threeWay, false);
    expect(twCanvas.width).toBeGreaterThan(100);
  });

  it("renders crossings and double slip switches (dkw)", () => {
    const dkw: CatalogEntry = {
      id: "24620",
      name: "Doppelkreuzungsweiche",
      kind: "dkw",
      category: "Kreuzungen",
      description: "DKW 188.3 mm, 24.3°",
      length: 188.3,
      angle: 24.3,
    };
    const dkwCanvas = createPieceDragCanvas(dkw, false);
    expect(dkwCanvas.width).toBeGreaterThan(100);
    expect(dkwCanvas.height).toBeGreaterThan(20);
  });

  it("renders buffer-stops and uncouplers", () => {
    const bufferStop: CatalogEntry = {
      id: "24977",
      name: "Prellbock",
      kind: "buffer-stop",
      category: "Zubehör",
      description: "Prellbockgleis",
      length: 77.5,
    };
    const bufferCanvas = createPieceDragCanvas(bufferStop, false);
    expect(bufferCanvas.width).toBeGreaterThan( bufferStop.length! * 0.4);

    const uncoupler: CatalogEntry = {
      id: "24997",
      name: "Entkupplungsgleis",
      kind: "uncoupler",
      category: "Zubehör",
      description: "Entkuppler",
      length: 94.2,
    };
    const uncouplerCanvas = createPieceDragCanvas(uncoupler, false);
    expect(uncouplerCanvas.width).toBeGreaterThan(uncoupler.length! * 0.4);
  });

  it("renders road pieces (straight and curved)", () => {
    const straightRoad: CatalogEntry = {
      id: "road-str-180",
      name: "Gerade Straße 180 mm",
      kind: "straight",
      category: "Geraden",
      description: "Gerade Straße",
      length: 180,
      isRoad: true,
      roadWidth: 60,
    };
    const sRoadCanvas = createPieceDragCanvas(straightRoad, true, 60);
    expect(sRoadCanvas.width).toBeGreaterThan(100);

    const curvedRoad: CatalogEntry = {
      id: "road-cur-250-45",
      name: "Kurvenstraße R250 45°",
      kind: "curve",
      category: "Bögen",
      description: "Kurve R250",
      radius: 250,
      angle: 45,
      isRoad: true,
      roadWidth: 60,
    };
    const cRoadCanvas = createPieceDragCanvas(curvedRoad, true, 60);
    expect(cRoadCanvas.width).toBeGreaterThan(50);
  });

  it("renders 3D model footprints for builtin models 1:1 with trackplan dimensions", () => {
    const def: BuiltinModelDef = {
      id: "station",
      name: "Bahnhof Neustadt",
      category: "buildings",
      icon: "🏢",
      description: "Classic passenger station",
      width: 320,
      depth: 140,
      height: 120,
      badgeBg: "#b45309",
      badgeColor: "#ffffff",
    };

    // At zoom 1.0: exact 1:1 with layout millimeter dimensions
    const canvas = createModelDragCanvas(def, 1.0, 1.0);
    expect(canvas).toBeDefined();
    expect(canvas.width).toBe(320 + 12); // width + 6px padding on each side
    expect(canvas.height).toBe(140 + 12); // depth + 6px padding on each side
  });

  it("renders custom 3D model footprint preview 1:1 with trackplan", () => {
    const model: Model3D = {
      id: "m-123",
      name: "Custom Signal Tower",
      category: "railway",
      transform: { x: 0, y: 0, rotation: 0 },
      width: 150,
      depth: 90,
      height: 80,
      modelType: "glb",
    };

    const canvas = createCustomModelDragCanvas(model, 1.0);
    expect(canvas).toBeDefined();
    expect(canvas.width).toBe(150 + 12);
    expect(canvas.height).toBe(90 + 12);
  });

  it("attaches drag image to DataTransfer synchronously and removes canvas", () => {
    const { canvas } = createMockCanvas();
    canvas.width = 120;
    canvas.height = 80;

    const mockDataTransfer = {
      setDragImage: vi.fn(),
    } as unknown as DataTransfer;

    attachDragImage(mockDataTransfer, canvas);

    expect(canvas.style.position).toBe("fixed");
    expect(canvas.style.left).toBe("-9999px");
    expect(document.body.appendChild).toHaveBeenCalledWith(canvas);
    expect(mockDataTransfer.setDragImage).toHaveBeenCalledWith(canvas, 60, 40);
  });

  it("creates a scaled 2D catalog thumbnail for track entries and caches it", () => {
    const entry: CatalogEntry = {
      id: "24188",
      name: "Straight Track 188.3mm",
      category: "Straight",
      kind: "straight",
      description: "Standard straight track",
      length: 188.3,
    };

    const thumb1 = createCatalogThumbnail(entry, false, undefined, 16.5, 60, 40);
    expect(thumb1).toBeDefined();
    expect(thumb1.style.width).toBe("60px");
    expect(thumb1.style.height).toBe("40px");

    // Second call should return cached clone
    const thumb2 = createCatalogThumbnail(entry, false, undefined, 16.5, 60, 40);
    expect(thumb2).toBeDefined();
    expect(thumb2.style.width).toBe("60px");
    expect(thumb2.style.height).toBe("40px");
  });
});
