import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { RightSidebar } from "@/ui/right-sidebar";
import { showTrackContextMenu } from "@/ui/track-context-menu";
import type { Layout } from "@/core/types";

class MockElement {
  tagName: string;
  id: string = "";
  className: string = "";
  style: Record<string, string> & { setProperty?: (k: string, v: string) => void } = {};
  title: string = "";
  dataset: Record<string, string> = {};
  children: MockElement[] = [];
  parentNode: MockElement | null = null;
  private listeners: Record<string, ((e: any) => void)[]> = {};
  private _innerHTML: string = "";
  textContent: string = "";
  type: string = "button";
  private attrs: Record<string, string> = {};
  classList: {
    add: (c: string) => void;
    remove: (c: string) => void;
    contains: (c: string) => boolean;
  };

  setAttribute(name: string, value: string) {
    this.attrs[name] = value;
  }

  getAttribute(name: string): string | null {
    return this.attrs[name] ?? null;
  }

  constructor(tagName: string) {
    this.tagName = tagName.toUpperCase();
    this.style.setProperty = (k: string, v: string) => {
      this.style[k] = v;
    };
    const classes = new Set<string>();
    this.classList = {
      add: (c: string) => {
        classes.add(c);
        this.className = Array.from(classes).join(" ");
      },
      remove: (c: string) => {
        classes.delete(c);
        this.className = Array.from(classes).join(" ");
      },
      contains: (c: string) => classes.has(c),
    };
  }

  get innerHTML(): string {
    return this._innerHTML;
  }

  set innerHTML(val: string) {
    this._innerHTML = val;
    this.children = [];
    const matches = val.match(/id="([^"]+)"|class="([^"]+)"/g) || [];
    for (const m of matches) {
      if (m.startsWith('id="')) {
        const id = m.slice(4, -1);
        const child = new MockElement("div");
        child.id = id;
        this.appendChild(child);
      } else if (m.startsWith('class="')) {
        const cls = m.slice(7, -1);
        const child = new MockElement("div");
        child.className = cls;
        this.appendChild(child);
      }
    }
  }

  appendChild(child: MockElement) {
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  append(...items: (MockElement | string)[]) {
    for (const item of items) {
      if (item instanceof MockElement) {
        this.appendChild(item);
      }
    }
  }

  remove() {
    if (this.parentNode) {
      const idx = this.parentNode.children.indexOf(this);
      if (idx !== -1) this.parentNode.children.splice(idx, 1);
      this.parentNode = null;
    }
  }

  addEventListener(type: string, listener: (e: any) => void) {
    if (!this.listeners[type]) this.listeners[type] = [];
    this.listeners[type].push(listener);
  }

  removeEventListener(type: string, listener: (e: any) => void) {
    if (this.listeners[type]) {
      this.listeners[type] = this.listeners[type].filter((l) => l !== listener);
    }
  }

  dispatchEvent(event: any): boolean {
    const list = this.listeners[event.type] || [];
    for (const listener of [...list]) {
      listener(event);
    }
    return true;
  }

  querySelector(selector: string): MockElement | null {
    if (selector.startsWith("#")) {
      const id = selector.slice(1);
      return this.find((el) => el.id === id);
    }
    if (selector.startsWith(".")) {
      const cls = selector.slice(1);
      return this.find((el) => el.className.split(/\s+/).includes(cls));
    }
    return null;
  }

  querySelectorAll(selector: string): MockElement[] {
    const result: MockElement[] = [];
    if (selector.startsWith(".")) {
      const cls = selector.slice(1);
      this.findAll((el) => el.className.split(/\s+/).includes(cls), result);
    }
    return result;
  }

  private find(predicate: (el: MockElement) => boolean): MockElement | null {
    for (const child of this.children) {
      if (predicate(child)) return child;
      const found = child.find(predicate);
      if (found) return found;
    }
    return null;
  }

  private findAll(predicate: (el: MockElement) => boolean, out: MockElement[]) {
    for (const child of this.children) {
      if (predicate(child)) out.push(child);
      child.findAll(predicate, out);
    }
  }
}

describe("RightSidebar - Buttons for Auswahlsperre & Woodplatte", () => {
  let host: MockElement;
  let layout: Layout;
  let originalDocument: any;

  beforeEach(() => {
    originalDocument = (globalThis as any).document;

    (globalThis as any).document = {
      head: new MockElement("head"),
      body: new MockElement("body"),
      getElementById: () => null,
      createElement: (tag: string) => new MockElement(tag),
    };

    host = new MockElement("div");
    layout = {
      pieces: [],
      baseboards: [],
      models: [],
      nextPieceId: 1,
      layers: [
        { id: "layer-1", name: "Ebene 1", color: "#38bdf8", visible: true, locked: false, heightMm: 0 },
      ],
      activeLayerId: "layer-1",
    } as unknown as Layout;
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
  });

  it("renders Woodplatte button and verifies selection lock button was removed from right sidebar", () => {
    const onToggleSelectionLock = vi.fn();
    const onToggleWoodPlate = vi.fn();
    let lockDisabled = false;
    let woodActive = false;

    new RightSidebar(host as unknown as HTMLElement, layout, {
      onLayerChange: vi.fn(),
      selectedPieceIds: () => [],
      onToggleSelectionLock,
      isSelectionLockDisabled: () => lockDisabled,
      onToggleWoodPlate,
      isWoodPlateActive: () => woodActive,
    });

    const lockBtn = host.querySelector("#cl-toggle-selection-lock");
    const woodBtn = host.querySelector("#cl-toggle-woodplate");

    // Selection lock button should not be present in the right sidebar
    expect(lockBtn).toBeNull();
    expect(woodBtn).toBeTruthy();

    // Verify wood button is styled as button
    expect(woodBtn?.className).toContain("cl-btn");

    // Verify there are no old switch sliders
    expect(host.querySelector(".cl-switch")).toBeNull();
    expect(host.querySelector(".cl-switch-knob")).toBeNull();

    // Verify click triggers callback
    woodBtn?.dispatchEvent({ type: "click" });
    expect(onToggleWoodPlate).toHaveBeenCalledTimes(1);
  });

  it("shows active state and badge on Woodplatte button when toggled on", () => {
    let woodActive = true;

    new RightSidebar(host as unknown as HTMLElement, layout, {
      onLayerChange: vi.fn(),
      selectedPieceIds: () => [],
      onToggleWoodPlate: vi.fn(),
      isWoodPlateActive: () => woodActive,
    });

    const lockBtn = host.querySelector("#cl-toggle-selection-lock");
    const woodBtn = host.querySelector("#cl-toggle-woodplate");

    expect(lockBtn).toBeNull();
    expect(woodBtn?.className).toContain("is-active");
    expect(woodBtn?.innerHTML).toMatch(/An|On/);
  });

  it("auto-fits every layer in the menu when layers are added or when height is too small", () => {
    host.style.height = "120px";
    layout.layers = [
      { id: "layer-2", name: "Bergstrecke", color: "#4ade80", visible: true, locked: false, heightMm: 80 },
      { id: "layer-1", name: "Hauptebene", color: "#38bdf8", visible: true, locked: false, heightMm: 0 },
      { id: "layer-0", name: "Schattenbahnhof", color: "#94a3b8", visible: true, locked: false, heightMm: -80 },
    ];

    const sb = new RightSidebar(host as unknown as HTMLElement, layout, {
      onLayerChange: vi.fn(),
      selectedPieceIds: () => [],
      onToggleWoodPlate: vi.fn(),
      isWoodPlateActive: () => false,
    });

    // Too-small fixed height (120px for 3 layers) should be cleared so it auto-fits all layers
    expect(host.style.height).toBe("");
    expect(host.querySelectorAll(".cl-card").length).toBe(3);

    // If user stretches sidebar taller (400px) and then a 4th layer is added, it resets to auto-fit all 4 layers
    host.style.height = "400px";
    layout.layers.push({
      id: "layer-3",
      name: "Gipfelbahn",
      color: "#f59e0b",
      visible: true,
      locked: false,
      heightMm: 160,
    });
    sb.update(layout);

    expect(host.style.height).toBe("");
    expect(host.querySelectorAll(".cl-card").length).toBe(4);
  });

  it("moves the track context menu up when opened near the bottom of the screen so every entry is visible", () => {
    const prevWin = (globalThis as any).window;
    (globalThis as any).window = {
      innerWidth: 1280,
      innerHeight: 600,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    };

    try {
      const body = (globalThis as any).document.body as MockElement;
      const origCreate = (globalThis as any).document.createElement;
      (globalThis as any).document.createElement = (tag: string) => {
        const el = origCreate(tag);
        (el as any).scrollHeight = 410;
        (el as any).offsetHeight = 410;
        (el as any).scrollWidth = 285;
        (el as any).offsetWidth = 285;
        return el;
      };

      showTrackContextMenu({
        piece: {
          id: "p1",
          catalogId: "24612",
          layerId: "layer-1",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [{ x: 0, y: 0, angle: 180 }],
        },
        screenX: 400,
        screenY: 520,
        layout,
        onOpenRampGenerator: vi.fn(),
        onAutoRamp: vi.fn(),
        onOpenHeightDialog: vi.fn(),
        onCopy: vi.fn(),
        onDelete: vi.fn(),
      });

      const menu = body.children.find((c) => c.id === "track-context-menu");
      expect(menu).toBeDefined();
      const topPx = parseFloat(menu!.style.top ?? "");
      // 410 + 4 = 414 height; 600 - 414 - 12 = 174px so the entire 410px menu fits above 600px
      expect(topPx + 410).toBeLessThanOrEqual(600 - 12);
    } finally {
      (globalThis as any).window = prevWin;
    }
  });
});
