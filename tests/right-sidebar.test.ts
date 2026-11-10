import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { RightSidebar } from "@/ui/right-sidebar";
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

  it("renders Auswahlsperre and Woodplatte as proper buttons without switch knobs", () => {
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

    expect(lockBtn).toBeTruthy();
    expect(woodBtn).toBeTruthy();

    // Verify both are styled as buttons
    expect(lockBtn?.className).toContain("cl-btn");
    expect(woodBtn?.className).toContain("cl-btn");

    // Verify there are no old switch sliders
    expect(host.querySelector(".cl-switch")).toBeNull();
    expect(host.querySelector(".cl-switch-knob")).toBeNull();

    // Verify click triggers callback
    lockBtn?.dispatchEvent({ type: "click" });
    expect(onToggleSelectionLock).toHaveBeenCalledTimes(1);

    woodBtn?.dispatchEvent({ type: "click" });
    expect(onToggleWoodPlate).toHaveBeenCalledTimes(1);
  });

  it("shows active state and badge on buttons when toggled on", () => {
    let lockDisabled = true;
    let woodActive = true;

    new RightSidebar(host as unknown as HTMLElement, layout, {
      onLayerChange: vi.fn(),
      selectedPieceIds: () => [],
      onToggleSelectionLock: vi.fn(),
      isSelectionLockDisabled: () => lockDisabled,
      onToggleWoodPlate: vi.fn(),
      isWoodPlateActive: () => woodActive,
    });

    const lockBtn = host.querySelector("#cl-toggle-selection-lock");
    const woodBtn = host.querySelector("#cl-toggle-woodplate");

    expect(lockBtn?.className).toContain("is-active");
    expect(woodBtn?.className).toContain("is-active");
    expect(lockBtn?.innerHTML).toMatch(/Aufgehoben|Unlocked/);
    expect(woodBtn?.innerHTML).toMatch(/An|On/);
  });
});
