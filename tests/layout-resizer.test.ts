import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  LayoutResizer,
  DEFAULT_SIDEBAR_WIDTH,
  DEFAULT_TOOLBAR_HEIGHT,
  DEFAULT_RIGHT_SIDEBAR_WIDTH,
  MIN_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
  MIN_TOOLBAR_HEIGHT,
  MAX_TOOLBAR_HEIGHT,
  MIN_RIGHT_SIDEBAR_WIDTH,
  MAX_RIGHT_SIDEBAR_WIDTH,
} from "@/ui/layout-resizer";

class MockElement {
  tagName: string;
  id: string = "";
  className: string = "";
  style: any;
  title: string = "";
  dataset: Record<string, string> = {};
  children: MockElement[] = [];
  parentNode: MockElement | null = null;
  private listeners: Record<string, ((e: any) => void)[]> = {};
  offsetWidth: number = 0;
  offsetHeight: number = 0;
  classList: {
    add: (c: string) => void;
    remove: (c: string) => void;
    contains: (c: string) => boolean;
    toggle: (c: string, force?: boolean) => boolean;
  };

  constructor(tagName: string) {
    this.tagName = tagName.toUpperCase();
    const styleProps: Record<string, string> = {};
    this.style = {
      ...styleProps,
      setProperty: (k: string, v: string) => {
        styleProps[k] = v;
      },
      getPropertyValue: (k: string) => {
        return styleProps[k] ?? "";
      },
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
      toggle: (c: string, force?: boolean) => {
        const has = classes.has(c);
        const shouldAdd = force !== undefined ? force : !has;
        if (shouldAdd) classes.add(c);
        else classes.delete(c);
        this.className = Array.from(classes).join(" ");
        return shouldAdd;
      },
    };
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

  private find(predicate: (el: MockElement) => boolean): MockElement | null {
    for (const child of this.children) {
      if (predicate(child)) return child;
      const found = child.find(predicate);
      if (found) return found;
    }
    return null;
  }
}

describe("LayoutResizer - Adjustable Toolbar, Sidebar & Right Sidebar", () => {
  let appEl: MockElement;
  let toolbarEl: MockElement;
  let sidebarEl: MockElement;
  let rightSidebarEl: MockElement;
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

    appEl = new MockElement("div");
    appEl.id = "app";

    toolbarEl = new MockElement("div");
    toolbarEl.id = "toolbar";
    toolbarEl.offsetHeight = 42;

    sidebarEl = new MockElement("div");
    sidebarEl.id = "sidebar";
    sidebarEl.offsetWidth = 280;

    rightSidebarEl = new MockElement("div");
    rightSidebarEl.id = "right-sidebar";
    rightSidebarEl.offsetWidth = 290;

    appEl.appendChild(toolbarEl);
    appEl.appendChild(sidebarEl);
    appEl.appendChild(rightSidebarEl);

    (globalThis as any).document = {
      body: new MockElement("body"),
      getElementById: (id: string) => {
        if (id === "app") return appEl;
        if (id === "toolbar") return toolbarEl;
        if (id === "sidebar") return sidebarEl;
        if (id === "right-sidebar") return rightSidebarEl;
        return null;
      },
      createElement: (tag: string) => new MockElement(tag),
    };

    const windowListeners: Record<string, ((e: any) => void)[]> = {};
    (globalThis as any).window = {
      innerWidth: 1920,
      innerHeight: 1080,
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

    (globalThis as any).getComputedStyle = (el: MockElement) => ({
      getPropertyValue: (prop: string) => el.style.getPropertyValue(prop),
    });
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
    (globalThis as any).window = originalWindow;
  });

  it("attaches resizer splitters to sidebar, toolbar and right sidebar", () => {
    const resizer = new LayoutResizer({
      appEl: appEl as unknown as HTMLElement,
      toolbarEl: toolbarEl as unknown as HTMLElement,
      sidebarEl: sidebarEl as unknown as HTMLElement,
      rightSidebarEl: rightSidebarEl as unknown as HTMLElement,
    });

    expect(appEl.querySelector("#sidebar-resizer")).toBeTruthy();
    expect(appEl.querySelector("#toolbar-resizer")).toBeTruthy();
    expect(rightSidebarEl.querySelector("#right-sidebar-resizer-left")).toBeTruthy();
    expect(rightSidebarEl.querySelector("#right-sidebar-resizer-bottom")).toBeTruthy();
    expect(rightSidebarEl.querySelector("#right-sidebar-resizer-corner")).toBeTruthy();

    resizer.destroy();
  });

  it("adjusts and clamps sidebar width", () => {
    const resizer = new LayoutResizer({
      appEl: appEl as unknown as HTMLElement,
      toolbarEl: toolbarEl as unknown as HTMLElement,
      sidebarEl: sidebarEl as unknown as HTMLElement,
      rightSidebarEl: rightSidebarEl as unknown as HTMLElement,
    });

    resizer.setSidebarWidth(350, true);
    expect(appEl.style.getPropertyValue("--sidebar-width")).toBe("350px");
    expect(localStorage.getItem("railbench_sidebar_width")).toBe("350");

    // Clamping min width
    resizer.setSidebarWidth(50, true);
    expect(appEl.style.getPropertyValue("--sidebar-width")).toBe(`${MIN_SIDEBAR_WIDTH}px`);

    // Resetting
    resizer.resetSidebarWidth();
    expect(appEl.style.getPropertyValue("--sidebar-width")).toBe(`${DEFAULT_SIDEBAR_WIDTH}px`);
    expect(localStorage.getItem("railbench_sidebar_width")).toBeNull();

    resizer.destroy();
  });

  it("adjusts and clamps toolbar height", () => {
    const resizer = new LayoutResizer({
      appEl: appEl as unknown as HTMLElement,
      toolbarEl: toolbarEl as unknown as HTMLElement,
      sidebarEl: sidebarEl as unknown as HTMLElement,
      rightSidebarEl: rightSidebarEl as unknown as HTMLElement,
    });

    resizer.setToolbarHeight(55, true);
    expect(appEl.style.getPropertyValue("--toolbar-height")).toBe("55px");
    expect(localStorage.getItem("railbench_toolbar_height")).toBe("55");

    // Clamping max height
    resizer.setToolbarHeight(200, true);
    expect(appEl.style.getPropertyValue("--toolbar-height")).toBe(`${MAX_TOOLBAR_HEIGHT}px`);

    // Resetting
    resizer.resetToolbarHeight();
    expect(appEl.style.getPropertyValue("--toolbar-height")).toBe(`${DEFAULT_TOOLBAR_HEIGHT}px`);
    expect(localStorage.getItem("railbench_toolbar_height")).toBeNull();

    resizer.destroy();
  });

  it("adjusts and clamps right sidebar width and height", () => {
    const resizer = new LayoutResizer({
      appEl: appEl as unknown as HTMLElement,
      toolbarEl: toolbarEl as unknown as HTMLElement,
      sidebarEl: sidebarEl as unknown as HTMLElement,
      rightSidebarEl: rightSidebarEl as unknown as HTMLElement,
    });

    resizer.setRightSidebarWidth(420, true);
    expect(rightSidebarEl.style.width).toBe("420px");
    expect(localStorage.getItem("railbench_right_sidebar_width")).toBe("420");

    resizer.setRightSidebarHeight(500, true);
    expect(rightSidebarEl.style.height).toBe("500px");
    expect(localStorage.getItem("railbench_right_sidebar_height")).toBe("500");

    // Reset all right sidebar sizes
    resizer.resetRightSidebarAll();
    expect(rightSidebarEl.style.width).toBe(`${DEFAULT_RIGHT_SIDEBAR_WIDTH}px`);
    expect(rightSidebarEl.style.height).toBe("");
    expect(localStorage.getItem("railbench_right_sidebar_width")).toBeNull();
    expect(localStorage.getItem("railbench_right_sidebar_height")).toBeNull();

    resizer.destroy();
  });

  it("restores saved sizes from localStorage on initialization", () => {
    localStorage.setItem("railbench_sidebar_width", "360");
    localStorage.setItem("railbench_toolbar_height", "50");
    localStorage.setItem("railbench_right_sidebar_width", "340");
    localStorage.setItem("railbench_right_sidebar_height", "450");

    const resizer = new LayoutResizer({
      appEl: appEl as unknown as HTMLElement,
      toolbarEl: toolbarEl as unknown as HTMLElement,
      sidebarEl: sidebarEl as unknown as HTMLElement,
      rightSidebarEl: rightSidebarEl as unknown as HTMLElement,
    });

    expect(appEl.style.getPropertyValue("--sidebar-width")).toBe("360px");
    expect(appEl.style.getPropertyValue("--toolbar-height")).toBe("50px");
    expect(rightSidebarEl.style.width).toBe("340px");
    expect(rightSidebarEl.style.height).toBe("450px");

    // resetAll should reset all 3
    resizer.resetAll();
    expect(appEl.style.getPropertyValue("--sidebar-width")).toBe(`${DEFAULT_SIDEBAR_WIDTH}px`);
    expect(appEl.style.getPropertyValue("--toolbar-height")).toBe(`${DEFAULT_TOOLBAR_HEIGHT}px`);
    expect(rightSidebarEl.style.width).toBe(`${DEFAULT_RIGHT_SIDEBAR_WIDTH}px`);
    expect(rightSidebarEl.style.height).toBe("");

    resizer.destroy();
  });

  it("handles double-click to reset individual splitters", () => {
    const resizer = new LayoutResizer({
      appEl: appEl as unknown as HTMLElement,
      toolbarEl: toolbarEl as unknown as HTMLElement,
      sidebarEl: sidebarEl as unknown as HTMLElement,
      rightSidebarEl: rightSidebarEl as unknown as HTMLElement,
    });

    resizer.setSidebarWidth(400, true);
    const sidebarHandle = appEl.querySelector("#sidebar-resizer")!;
    sidebarHandle.dispatchEvent({ type: "dblclick" });
    expect(appEl.style.getPropertyValue("--sidebar-width")).toBe(`${DEFAULT_SIDEBAR_WIDTH}px`);

    resizer.setToolbarHeight(60, true);
    const toolbarHandle = appEl.querySelector("#toolbar-resizer")!;
    toolbarHandle.dispatchEvent({ type: "dblclick" });
    expect(appEl.style.getPropertyValue("--toolbar-height")).toBe(`${DEFAULT_TOOLBAR_HEIGHT}px`);

    resizer.destroy();
  });

  it("defaults to locked, can be unlocked, toggled and persisted", () => {
    const resizer = new LayoutResizer({
      appEl: appEl as unknown as HTMLElement,
      toolbarEl: toolbarEl as unknown as HTMLElement,
      sidebarEl: sidebarEl as unknown as HTMLElement,
      rightSidebarEl: rightSidebarEl as unknown as HTMLElement,
    });

    expect(resizer.isLocked()).toBe(true);
    expect(appEl.classList.contains("bars-locked")).toBe(true);

    resizer.toggleLocked();
    expect(resizer.isLocked()).toBe(false);
    expect(appEl.classList.contains("bars-locked")).toBe(false);
    expect(localStorage.getItem("railbench_bars_locked")).toBe("false");

    resizer.setLocked(true);
    expect(resizer.isLocked()).toBe(true);
    expect(localStorage.getItem("railbench_bars_locked")).toBe("true");

    resizer.destroy();
  });
});
