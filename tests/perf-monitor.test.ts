import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { PerfMonitor } from "@/ui/perf-monitor";
import type { Renderer } from "@/render/renderer";
import type { Layout } from "@/core/types";

class MockElement {
  tagName: string;
  id: string = "";
  className: string = "";
  style: Record<string, string> = {};
  title: string = "";
  dataset: Record<string, string> = {};
  children: MockElement[] = [];
  parentNode: MockElement | null = null;
  private listeners: Record<string, ((e: any) => void)[]> = {};
  private _innerHTML: string = "";
  textContent: string = "";
  offsetLeft: number = 0;
  offsetTop: number = 0;
  offsetWidth: number = 250;
  offsetHeight: number = 180;
  clientWidth: number = 250;
  clientHeight: number = 180;

  constructor(tagName: string) {
    this.tagName = tagName.toUpperCase();
  }

  get innerHTML(): string {
    return this._innerHTML;
  }

  set innerHTML(val: string) {
    this._innerHTML = val;
    this.children = [];
    // If it contains certain ids or classes, create child elements so querySelector finds them
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

  closest(selector: string): MockElement | null {
    if (selector.startsWith("#") && this.id === selector.slice(1)) return this;
    if (selector.startsWith(".") && this.className === selector.slice(1)) return this;
    return this.parentNode ? this.parentNode.closest(selector) : null;
  }

  querySelector(selector: string): MockElement | null {
    if (selector.startsWith("#")) {
      const id = selector.slice(1);
      return this.find((el) => el.id === id);
    }
    if (selector.startsWith(".")) {
      const cls = selector.slice(1);
      return this.find((el) => el.className === cls);
    }
    return null;
  }

  querySelectorAll(selector: string): MockElement[] {
    const result: MockElement[] = [];
    if (selector.startsWith(".")) {
      const cls = selector.slice(1);
      this.findAll((el) => el.className === cls, result);
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

  getBoundingClientRect() {
    return {
      left: parseFloat(this.style.left ?? "0") || this.offsetLeft || 0,
      top: parseFloat(this.style.top ?? "0") || this.offsetTop || 0,
      width: this.offsetWidth,
      height: this.offsetHeight,
      right: (parseFloat(this.style.left ?? "0") || 0) + this.offsetWidth,
      bottom: (parseFloat(this.style.top ?? "0") || 0) + this.offsetHeight,
    };
  }
}

describe("PerfMonitor - Draggable Hardware Monitor", () => {
  let host: MockElement;
  let mockRenderer: Renderer;
  let layout: Layout;
  const store: Record<string, string> = {};
  let originalDocument: any;
  let originalWindow: any;

  beforeEach(() => {
    for (const k in store) delete store[k];

    vi.stubGlobal("localStorage", {
      getItem: (key: string) => store[key] ?? null,
      setItem: (key: string, val: string) => {
        store[key] = String(val);
      },
      removeItem: (key: string) => {
        delete store[key];
      },
      clear: () => {
        for (const k in store) delete store[k];
      },
      key: (i: number) => Object.keys(store)[i] ?? null,
      get length() {
        return Object.keys(store).length;
      },
    });

    originalDocument = (globalThis as any).document;
    originalWindow = (globalThis as any).window;

    const mockBody = new MockElement("body");
    (globalThis as any).document = {
      body: mockBody,
      createElement: (tag: string) => new MockElement(tag),
    };

    const windowListeners: Record<string, ((e: any) => void)[]> = {};
    (globalThis as any).window = {
      addEventListener: (type: string, fn: any) => {
        if (!windowListeners[type]) windowListeners[type] = [];
        windowListeners[type].push(fn);
      },
      removeEventListener: (type: string, fn: any) => {
        if (windowListeners[type]) {
          windowListeners[type] = windowListeners[type].filter((l) => l !== fn);
        }
      },
      dispatchEvent: (e: any) => {
        const list = windowListeners[e.type] || [];
        for (const fn of [...list]) fn(e);
        return true;
      },
    };

    host = new MockElement("div");
    host.id = "canvas-host";
    host.clientWidth = 1000;
    host.clientHeight = 800;

    mockRenderer = {
      app: {
        canvas: {
          getContext: vi.fn().mockReturnValue(null),
        },
      },
    } as unknown as Renderer;

    layout = {
      pieces: [],
      baseboards: [],
      models: [],
      nextPieceId: 1,
    } as unknown as Layout;
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
    (globalThis as any).window = originalWindow;
  });

  it("creates the hardware monitor with draggable header and correct elements", () => {
    const monitor = new PerfMonitor({
      host: host as unknown as HTMLElement,
      renderer: mockRenderer,
      getLayout: () => layout,
    });

    const overlay = host.querySelector("#perf-monitor-overlay");
    expect(overlay).toBeTruthy();
    expect(overlay?.style.position).toBe("absolute");
    expect(overlay?.style.bottom).toBe("10px");
    expect(overlay?.style.right).toBe("10px");

    const header = overlay?.querySelector(".perf-monitor-header");
    expect(header).toBeTruthy();
    expect(header?.style.cursor).toBe("grab");
    expect(header?.innerHTML).toContain("HARDWARE MONITOR");

    const closeBtn = header?.querySelector("#perf-close-btn");
    expect(closeBtn).toBeTruthy();

    monitor.destroy();
  });

  it("can set, get and clamp position", () => {
    const monitor = new PerfMonitor({
      host: host as unknown as HTMLElement,
      renderer: mockRenderer,
      getLayout: () => layout,
    });

    monitor.setPosition(200, 150);
    const pos = monitor.getPosition();
    expect(pos).toEqual({ left: 200, top: 150 });

    // Clamp when out of bounds (host 1000x800, overlay 250x180 => max 750, 620)
    monitor.setPosition(2000, 1500);
    const clampedPos = monitor.getPosition();
    expect(clampedPos?.left).toBe(750);
    expect(clampedPos?.top).toBe(620);

    monitor.resetPosition();
    expect(monitor.getPosition()).toBeNull(); // Reverted to top/right CSS defaults
    expect(localStorage.getItem("railbench_perf_monitor_pos")).toBeNull();

    monitor.destroy();
  });

  it("restores saved position from localStorage upon initialization", () => {
    localStorage.setItem("railbench_perf_monitor_pos", JSON.stringify({ left: 320, top: 180 }));

    const monitor = new PerfMonitor({
      host: host as unknown as HTMLElement,
      renderer: mockRenderer,
      getLayout: () => layout,
    });

    const pos = monitor.getPosition();
    expect(pos).toEqual({ left: 320, top: 180 });

    monitor.destroy();
  });

  it("supports dragging via pointer events and persists position to localStorage", () => {
    const monitor = new PerfMonitor({
      host: host as unknown as HTMLElement,
      renderer: mockRenderer,
      getLayout: () => layout,
    });

    const overlay = host.querySelector("#perf-monitor-overlay")!;
    const header = overlay.querySelector(".perf-monitor-header")!;

    // Initial position
    monitor.setPosition(100, 100);

    // Pointer down on header
    header.dispatchEvent({
      type: "pointerdown",
      clientX: 100,
      clientY: 100,
      button: 0,
      preventDefault: vi.fn(),
      target: header,
    });

    // Pointer move
    (globalThis as any).window.dispatchEvent({
      type: "pointermove",
      clientX: 180,
      clientY: 150,
    });

    expect(monitor.getPosition()?.left).toBe(180);
    expect(monitor.getPosition()?.top).toBe(150);

    // Pointer up
    (globalThis as any).window.dispatchEvent({
      type: "pointerup",
      clientX: 180,
      clientY: 150,
    });

    const saved = JSON.parse(localStorage.getItem("railbench_perf_monitor_pos") || "{}");
    expect(saved).toEqual({ left: 180, top: 150 });

    monitor.destroy();
  });

  it("resets position when double clicking the header", () => {
    const monitor = new PerfMonitor({
      host: host as unknown as HTMLElement,
      renderer: mockRenderer,
      getLayout: () => layout,
    });

    monitor.setPosition(250, 250);
    expect(monitor.getPosition()).toEqual({ left: 250, top: 250 });

    const overlay = host.querySelector("#perf-monitor-overlay")!;
    const header = overlay.querySelector(".perf-monitor-header")!;
    header.dispatchEvent({ type: "dblclick" });

    expect(monitor.getPosition()).toBeNull();
    expect(overlay.style.right).toBe("10px");
    expect(overlay.style.bottom).toBe("10px");

    monitor.destroy();
  });

  it("toggles and closes via close button", () => {
    const monitor = new PerfMonitor({
      host: host as unknown as HTMLElement,
      renderer: mockRenderer,
      getLayout: () => layout,
    });

    expect(monitor.isOpen()).toBe(false);
    monitor.show();
    expect(monitor.isOpen()).toBe(true);

    const overlay = host.querySelector("#perf-monitor-overlay")!;
    const closeBtn = overlay.querySelector("#perf-close-btn")!;
    closeBtn.dispatchEvent({
      type: "click",
      stopPropagation: vi.fn(),
    });
    expect(monitor.isOpen()).toBe(false);

    monitor.toggle();
    expect(monitor.isOpen()).toBe(true);

    monitor.destroy();
  });

  it("displays accurate Electron process/app RAM usage when electronAPI is available", async () => {
    (globalThis as any).window.electronAPI = {
      getProcessMemoryInfo: () => ({
        rssMb: 185.4,
        heapUsedMb: 34.2,
        heapTotalMb: 48.0,
        externalMb: 12.1,
        systemTotalMb: 16384,
        systemFreeMb: 8192,
        systemUsedMb: 8192,
      }),
      getMemoryUsage: vi.fn().mockResolvedValue({
        appRamMb: 242.8,
        systemTotalMb: 16384,
        systemFreeMb: 8192,
        systemUsedMb: 8192,
      }),
    };

    const monitor = new PerfMonitor({
      host: host as unknown as HTMLElement,
      renderer: mockRenderer,
      getLayout: () => layout,
    });

    monitor.show();
    const ramEl = host.querySelector("#perf-ram-val")!;
    // Immediately uses synchronous getProcessMemoryInfo
    expect(ramEl.textContent).toBe("185.4 MB (Heap: 34.2 MB)");

    // After async IPC resolves, updates to total multi-process appRamMb
    await Promise.resolve();
    expect(ramEl.textContent).toBe("242.8 MB (Heap: 34.2 MB)");
    expect(ramEl.title).toContain("System RAM: 8.0 / 16.0 GB");

    monitor.destroy();
  });

  it("displays accurate RAM usage in Chromium browsers and non-zero fallback in standard browsers", () => {
    const perfAny = performance as any;
    const origMemory = perfAny.memory;

    try {
      perfAny.memory = {
        usedJSHeapSize: 30 * 1024 * 1024,
        totalJSHeapSize: 45 * 1024 * 1024,
        jsHeapSizeLimit: 2048 * 1024 * 1024,
      };

      const monitor = new PerfMonitor({
        host: host as unknown as HTMLElement,
        renderer: mockRenderer,
        getLayout: () => layout,
      });

      monitor.show();
      const ramEl = host.querySelector("#perf-ram-val")!;
      // 30 MB JS heap + 28 MB WebGL/Canvas scene buffer = 58.0 MB
      expect(ramEl.textContent).toBe("58.0 MB (Heap: 30.0 MB)");

      // Fallback when performance.memory is unavailable (e.g. Firefox/Safari)
      delete perfAny.memory;
      monitor.hide();
      monitor.show();
      expect(ramEl.textContent).toBe("~70.0 MB (App + WebGL)");
      expect(ramEl.textContent).not.toBe("~0.0 MB (Layout Heap)");

      monitor.destroy();
    } finally {
      perfAny.memory = origMemory;
    }
  });
});
