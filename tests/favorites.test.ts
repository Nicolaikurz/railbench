import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { Sidebar } from "@/ui/sidebar";
import { maerklinCCatalog } from "@/catalog/catalog";
import type { Layout } from "@/core/types";

class MockElement {
  tagName: string;
  id: string = "";
  private _className: string = "";
  style: Record<string, string> & { setProperty?: (k: string, v: string) => void } = {};
  title: string = "";
  draggable: boolean = false;
  children: MockElement[] = [];
  parentNode: MockElement | null = null;
  private listeners: Record<string, ((e: any) => void)[]> = {};
  private _innerHTML: string = "";
  textContent: string = "";
  type: string = "button";
  value: string = "";
  placeholder: string = "";
  private attrs: Record<string, string> = {};
  classList: {
    add: (c: string) => void;
    remove: (c: string) => void;
    contains: (c: string) => boolean;
  };

  get className(): string {
    return this._className;
  }

  set className(val: string) {
    this._className = val;
  }

  setAttribute(name: string, value: string) {
    this.attrs[name] = value;
  }

  getAttribute(name: string): string | null {
    return this.attrs[name] ?? null;
  }

  getContext(type: string) {
    return new Proxy(
      {
        canvas: this,
        measureText: () => ({ width: 10 }),
      },
      {
        get(target: any, prop: string) {
          if (prop in target) return target[prop];
          return () => {};
        },
        set(target: any, prop: string, val: any) {
          target[prop] = val;
          return true;
        },
      }
    );
  }

  constructor(tagName: string) {
    this.tagName = tagName.toUpperCase();
    this.style.setProperty = (k: string, v: string) => {
      this.style[k] = v;
    };
    this.classList = {
      add: (c: string) => {
        const parts = new Set(this._className.split(/\s+/).filter(Boolean));
        parts.add(c);
        this._className = Array.from(parts).join(" ");
      },
      remove: (c: string) => {
        const parts = new Set(this._className.split(/\s+/).filter(Boolean));
        parts.delete(c);
        this._className = Array.from(parts).join(" ");
      },
      contains: (c: string) => {
        return this._className.split(/\s+/).filter(Boolean).includes(c);
      },
    };
  }

  get innerHTML(): string {
    return this._innerHTML;
  }

  set innerHTML(val: string) {
    this._innerHTML = val;
    this.children = [];
    const classMatches = val.match(/class="([^"]+)"/g) || [];
    for (const m of classMatches) {
      const cls = m.slice(7, -1);
      const child = new MockElement("div");
      child.className = cls;
      this.appendChild(child);
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
    if (!this.listeners[type]) return;
    this.listeners[type] = this.listeners[type].filter((l) => l !== listener);
  }

  dispatchEvent(event: { type: string; stopPropagation?: () => void; preventDefault?: () => void }) {
    const list = this.listeners[event.type] || [];
    for (const l of list) l(event);
  }

  click() {
    this.dispatchEvent({ type: "click", stopPropagation: () => {}, preventDefault: () => {} });
  }

  querySelector(selector: string): MockElement | null {
    if (selector.startsWith(".")) {
      const cls = selector.slice(1);
      return this.find((el) => el.classList.contains(cls));
    }
    return null;
  }

  querySelectorAll(selector: string): MockElement[] {
    const results: MockElement[] = [];
    if (selector.startsWith(".")) {
      const cls = selector.slice(1);
      this.findAll((el) => el.classList.contains(cls), results);
    }
    return results;
  }

  contains(other: any): boolean {
    if (other === this) return true;
    for (const c of this.children) {
      if (c.contains(other)) return true;
    }
    return false;
  }

  private find(predicate: (el: MockElement) => boolean): MockElement | null {
    for (const c of this.children) {
      if (predicate(c)) return c;
      const f = c.find(predicate);
      if (f) return f;
    }
    return null;
  }

  private findAll(predicate: (el: MockElement) => boolean, out: MockElement[]) {
    for (const c of this.children) {
      if (predicate(c)) out.push(c);
      c.findAll(predicate, out);
    }
  }
}

function createEmptyLayout(): Layout {
  return {
    name: "Test Layout",
    scale: "H0",
    layers: [{ id: "l1", name: "Default", visible: true, locked: false, color: "#3b82f6" }],
    pieces: [],
    baseboards: [],
  };
}

const dummyCallbacks = {
  onExportCSV: () => {},
};

describe("Favorites Feature", () => {
  const store: Record<string, string> = {};
  let host: MockElement;
  let mockDoc: any;

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
    });

    const head = new MockElement("head");
    const body = new MockElement("body");

    mockDoc = {
      createElement: (tag: string) => new MockElement(tag),
      getElementById: () => null,
      head,
      body,
    };
    vi.stubGlobal("document", mockDoc);

    vi.stubGlobal("window", {
      addEventListener: () => {},
      removeEventListener: () => {},
      localStorage: (globalThis as any).localStorage,
    });

    host = new MockElement("div");
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("should initialize with no favorites and empty storage", () => {
    const layout = createEmptyLayout();
    const sidebar = new Sidebar(host as any, maerklinCCatalog, layout, dummyCallbacks);

    expect(sidebar.getTotalFavoritesCount()).toBe(0);
    expect(sidebar.getFavoritesCount()).toBe(0);
    expect(sidebar.isFavorite("24130")).toBe(false);
  });

  it("should toggle favorites and persist to localStorage", () => {
    const layout = createEmptyLayout();
    const sidebar = new Sidebar(host as any, maerklinCCatalog, layout, dummyCallbacks);

    const added = sidebar.toggleFavorite("24130", maerklinCCatalog.id);
    expect(added).toBe(true);
    expect(sidebar.isFavorite("24130", maerklinCCatalog.id)).toBe(true);
    expect(sidebar.getTotalFavoritesCount()).toBe(1);
    expect(sidebar.getFavoritesCount()).toBe(1);

    const stored = JSON.parse(store["railbench_favorite_tracks"] || "[]");
    expect(stored).toEqual([{ catalogId: maerklinCCatalog.id, trackId: "24130" }]);

    // Toggling again removes it
    const removed = sidebar.toggleFavorite("24130", maerklinCCatalog.id);
    expect(removed).toBe(false);
    expect(sidebar.isFavorite("24130", maerklinCCatalog.id)).toBe(false);
    expect(sidebar.getTotalFavoritesCount()).toBe(0);
    expect(sidebar.getFavoritesCount()).toBe(0);
  });

  it("should load existing favorites from localStorage on initialization", () => {
    store["railbench_favorite_tracks"] = JSON.stringify([
      { catalogId: maerklinCCatalog.id, trackId: "24130" },
      { catalogId: "lgb-g", trackId: "11000" },
    ]);

    const layout = createEmptyLayout();
    const sidebar = new Sidebar(host as any, maerklinCCatalog, layout, dummyCallbacks);

    expect(sidebar.getTotalFavoritesCount()).toBe(2);
    // In Märklin C-Gleis, only 24130 belongs to this catalog
    expect(sidebar.getFavoritesCount()).toBe(1);
    expect(sidebar.isFavorite("24130")).toBe(true);
    expect(sidebar.isFavorite("11000")).toBe(true);
  });

  it("should construct a virtual favorites catalog across multiple systems", () => {
    store["railbench_favorite_tracks"] = JSON.stringify([
      { catalogId: maerklinCCatalog.id, trackId: "24130" },
      { catalogId: "lgb-g", trackId: "11000" },
    ]);

    const layout = createEmptyLayout();
    const sidebar = new Sidebar(host as any, maerklinCCatalog, layout, dummyCallbacks);

    const favCatalog = sidebar.getFavoritesCatalog();
    expect(favCatalog.id).toBe("favorites");
    expect(favCatalog.entries.length).toBe(2);

    const cTrack = favCatalog.entries.find((e) => e.id === "24130");
    const gTrack = favCatalog.entries.find((e) => e.id === "11000");

    expect(cTrack).toBeDefined();
    expect(cTrack?.gauge).toBe(16.5);
    expect((cTrack as any)._originCatalogName).toContain("Märklin");

    expect(gTrack).toBeDefined();
    expect(gTrack?.gauge).toBe(45);
    expect((gTrack as any)._originCatalogName).toContain("LGB");
  });

  it("should render star buttons on track cards and filter-pills in catalog tab", () => {
    store["railbench_favorite_tracks"] = JSON.stringify([
      { catalogId: maerklinCCatalog.id, trackId: "24130" },
    ]);

    const layout = createEmptyLayout();
    const sidebar = new Sidebar(host as any, maerklinCCatalog, layout, dummyCallbacks);

    // Check filter pills
    const pills = host.querySelectorAll(".filter-pill");
    expect(pills.length).toBeGreaterThan(0);
    const favPill = pills[0]; // Favorites is the first pill
    expect(favPill?.textContent).toMatch(/Favorit(en|es) \(1\)/);

    // Check track cards
    const cards = host.querySelectorAll(".track-card");
    expect(cards.length).toBeGreaterThan(0);
    const starBtns = host.querySelectorAll(".favorite-track-btn");
    expect(starBtns.length).toBeGreaterThan(0);

    // 24130 should be marked active
    const activeStar = starBtns.find((b) => b.classList.contains("active"));
    expect(activeStar).toBeDefined();
  });
});
