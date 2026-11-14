import { describe, it, expect } from "vitest";
import { Graphics } from "pixi.js";
import { createBaseboardGraphics, getStyleColors } from "@/render/baseboard-sprite";
import { createDefaultBaseboard, getBaseboards } from "@/core/layout";
import { Renderer } from "@/render/renderer";
import type { BaseboardConfig, Layout, Layer, LayerPlate } from "@/core/types";

describe("2D Table & Plate Render (Color and Style)", () => {
  it("provides distinct surface and edge colors for each baseboard/plate style", () => {
    const grass = getStyleColors("grass");
    expect(grass.surfaceColor).toBe(0x4d7c0f); // Grass green
    expect(grass.isGrass).toBe(true);

    const darkWood = getStyleColors("dark-wood");
    expect(darkWood.surfaceColor).toBe(0x4a2e1b); // Walnut
    expect(darkWood.hasGrain).toBe(true);

    const slate = getStyleColors("slate");
    expect(slate.surfaceColor).toBe(0x27272a); // Slate anthracite
    expect(slate.isGrass).toBeFalsy();

    const white = getStyleColors("white");
    expect(white.surfaceColor).toBe(0xf8fafc); // White

    const pine = getStyleColors("plywood");
    expect(pine.surfaceColor).toBe(0xecdcb9);

    const wood = getStyleColors("wood");
    expect(wood.surfaceColor).toBe(0xd4a373); // Birch plywood
  });

  it("renders baseboard table with grass surface color and grass tuft pattern", () => {
    const table = createDefaultBaseboard({
      id: "board-grass",
      style: "grass",
      width: 2000,
      depth: 1000,
    });
    const container = createBaseboardGraphics(table);
    expect(container).toBeDefined();
    expect(container.children.length).toBeGreaterThan(2);
  });

  it("renders baseboard table with wood grain pattern for wood styles", () => {
    const table = createDefaultBaseboard({
      id: "board-dark-wood",
      style: "dark-wood",
      width: 2000,
      depth: 1000,
    });
    const container = createBaseboardGraphics(table);
    expect(container).toBeDefined();
    expect(container.children.length).toBeGreaterThan(2);
  });

  it("resolves plate style from ground level plate to the table rendering", () => {
    const bb = createDefaultBaseboard({
      id: "board-1",
      style: "wood", // Table default was wood
      width: 2400,
      depth: 1200,
    });

    const plate: LayerPlate = {
      id: "plate-1",
      baseboardId: "board-1",
      width: 2400,
      depth: 1200,
      style: "grass", // User chose grass for the plate
    };

    const groundLayer: Layer = {
      id: "ground",
      name: "Ground Level (0 mm)",
      visible: true,
      locked: false,
      heightMm: 0,
      woodPlateMode: "table",
      plates: [plate],
    };

    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      baseboards: [bb],
      layers: [groundLayer],
      pieces: [],
    };

    // Determine effective style as renderer does
    const baseboards = getBaseboards(layout);
    const tableLayer = layout.layers?.find((l) => (l.heightMm ?? 0) === 0);
    const matchingPlate = tableLayer?.plates?.find(
      (p) => (p.baseboardId || baseboards[0]?.id) === bb.id,
    );
    const effectiveStyle = matchingPlate?.style ?? tableLayer?.woodPlateStyle ?? bb.style ?? "wood";

    expect(effectiveStyle).toBe("grass");

    const bbSprite = createBaseboardGraphics({ ...bb, style: effectiveStyle });
    expect(bbSprite).toBeDefined();
    expect(bbSprite.children.length).toBeGreaterThan(0);
  });

  it("renders intermediate layer plates and open benchwork plates using full baseboard graphics", () => {
    // Upper level plate at 100 mm height
    const upperPlate: LayerPlate = {
      id: "plate-upper",
      baseboardId: "board-1",
      width: 1200,
      depth: 600,
      x: 200,
      y: 150,
      style: "slate",
    };

    const plateConfig: BaseboardConfig = {
      enabled: true,
      width: upperPlate.width,
      depth: upperPlate.depth,
      positionMode: "custom",
      customX: upperPlate.x,
      customY: upperPlate.y,
      shape: "rect",
      style: upperPlate.style ?? "wood",
      cornerRadius: 8,
      showDimensions: false,
    };

    const plateSprite = createBaseboardGraphics(plateConfig, {
      layerElevation: 100,
      layerThickness: 12,
    });

    expect(plateSprite).toBeDefined();
    expect(plateSprite.children.length).toBeGreaterThan(0);
  });

  it("cuts holes in PixiJS v8 graphics for table cutouts so surface is not corrupted", () => {
    const table = createDefaultBaseboard({
      id: "board-cutout-test",
      style: "grass",
      width: 2000,
      depth: 1000,
    });

    const container = createBaseboardGraphics(table);
    // Find the surface graphics object (child 1) with green surface color (0x4d7c0f = 5078031)
    const surfaceG = container.children.find((c) => {
      if (!(c instanceof Graphics)) return false;
      const fill = (c as any).context?.instructions?.find((i: any) => i.action === "fill");
      return fill?.data?.style?.color === 0x4d7c0f;
    }) as any;
    expect(surfaceG).toBeDefined();
    const fillInstruction = surfaceG.context.instructions.find((i: any) => i.action === "fill");
    expect(fillInstruction).toBeDefined();
    expect(fillInstruction.data.style.color).toBe(0x4d7c0f);
  });

  it("sorts baseboard and plates by elevation in Renderer so underground plates do not cover the table surface", () => {
    const renderer = new Renderer();

    const tableBoard = createDefaultBaseboard({
      id: "board-1",
      style: "grass",
      width: 2200,
      depth: 1250,
    });

    const groundLayer: Layer = {
      id: "layer-ground",
      name: "Hauptstrecke",
      heightMm: 0,
      visible: true,
      locked: false,
      woodPlateMode: "table",
    };

    const undergroundPlate: LayerPlate = {
      id: "plate-shadow",
      baseboardId: "board-1",
      width: 2200,
      depth: 1250,
      style: "wood",
    };

    const undergroundLayer: Layer = {
      id: "layer-shadow",
      name: "Schattenbahnhof",
      heightMm: -100,
      visible: true,
      locked: false,
      woodPlateMode: "table",
      plates: [undergroundPlate],
    };

    const layout: Layout = {
      name: "Test Multi-Layer Layout",
      scale: "H0",
      activeLayerId: "layer-ground",
      baseboards: [tableBoard],
      layers: [undergroundLayer, groundLayer],
      pieces: [],
    };

    renderer.render(layout);

    const baseboardContainer = renderer.getBaseboardContainer();
    expect(baseboardContainer.children.length).toBe(2);

    // Child 0 must be the underground plate (-100 mm)
    // Child 1 must be the main ground table (0 mm) drawn on top of the underground plate
    const undergroundSprite = baseboardContainer.children[0];
    const tableSprite = baseboardContainer.children[1];

    expect(tableSprite).toBeDefined();
    expect(tableSprite!.alpha).toBe(1.0); // Active layer table is fully opaque
    expect(undergroundSprite).toBeDefined();

    // When switching active layer to Schattenbahnhof (-100 mm), inactive upper table (0 mm) becomes ghosted (alpha 0.4)
    layout.activeLayerId = "layer-shadow";
    renderer.render(layout);

    expect(baseboardContainer.children[1]!.alpha).toBe(0.4);

    // When hiding Hauptstrecke (groundLayer.visible = false), the table is hidden completely
    groundLayer.visible = false;
    renderer.render(layout);

    expect(baseboardContainer.children.length).toBe(1); // Only underground plate remains
  });
});
