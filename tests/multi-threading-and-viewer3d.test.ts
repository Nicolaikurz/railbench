import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { getThreadConcurrency, detectShortCircuitAsync } from "@/core/multi-threading";
import { Viewer3D } from "@/render/viewer-3d";
import { catalogRegistry } from "@/catalog/registry";
import { createLayout } from "@/core/layout";

describe("Multi-Threading and 3D Minimal Environment", () => {
  let originalDocument: any;
  let originalWindow: any;

  beforeEach(() => {
    originalDocument = (globalThis as any).document;
    originalWindow = (globalThis as any).window;

    const mockElement = () => ({
      style: {},
      dataset: {},
      classList: { toggle: vi.fn(), add: vi.fn(), remove: vi.fn() },
      appendChild: vi.fn(),
      remove: vi.fn(),
      removeChild: vi.fn(),
      querySelector: vi.fn().mockReturnValue(null),
      querySelectorAll: vi.fn().mockReturnValue([]),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      getAttribute: vi.fn().mockReturnValue(null),
      setAttribute: vi.fn(),
      getContext: vi.fn().mockReturnValue({
        getExtension: vi.fn(),
        getParameter: vi.fn(),
      }),
    });

    (globalThis as any).document = {
      createElement: () => mockElement(),
      createElementNS: () => mockElement(),
      getElementById: vi.fn().mockReturnValue(null),
      documentElement: mockElement(),
      body: mockElement(),
      head: mockElement(),
    };

    (globalThis as any).window = {
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      devicePixelRatio: 1,
    };
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
    (globalThis as any).window = originalWindow;
  });
  it("provides thread concurrency count and async simulation execution", async () => {
    const concurrency = getThreadConcurrency();
    expect(concurrency).toBeGreaterThanOrEqual(1);

    const catalog = catalogRegistry.get("maerklin-c-h0");
    expect(catalog).toBeDefined();

    const emptyLayout = createLayout("Test Layout");

    const res = await detectShortCircuitAsync(emptyLayout, catalog!);
    expect(res).toBeDefined();
    expect(res.hasShortCircuit).toBe(false);
  });

  it("verifies 3D Minimal setting completely clears train hall and stops day/night cycle", () => {
    const host = document.createElement("div");
    document.body.appendChild(host);

    const viewer = new Viewer3D(host);
    const catalog = catalogRegistry.get("maerklin-c-h0")!;
    const layout = createLayout("Test Layout 3D");

    // 1. Start in Hall mode
    viewer.setEnvironment("hall");
    viewer.updateLayout(layout, catalog);
    viewer.setDayNightActive(true);

    const trainHallGroup = (viewer as any).trainHallGroup;
    expect(trainHallGroup.children.length).toBeGreaterThan(0);
    expect((viewer as any).sceneCenter.y).toBeLessThan(-500);

    // 2. Switch to Minimal mode
    viewer.setEnvironment("minimal");

    // Must be completely emptied from memory/scene
    expect(trainHallGroup.children.length).toBe(0);
    expect(trainHallGroup.visible).toBe(false);

    // Day/Night cycle must be disabled in minimal mode
    expect((viewer as any).dayNightActive).toBe(false);

    // Scene center must be at table level y = 0, NOT on the invisible hall floor below -500
    expect((viewer as any).sceneCenter.y).toBe(0);

    // Celestial bodies must be hidden
    expect((viewer as any).celestialGroup.visible).toBe(false);

    // Calling updateDayNight in minimal mode must do nothing
    const timeBefore = (viewer as any).timeOfDay;
    (viewer as any).updateDayNight(1.0);
    expect((viewer as any).timeOfDay).toBe(timeBefore);

    viewer.dispose();
    host.remove();
  });
});
