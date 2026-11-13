import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { createToolbar, isMacOS, updateToolbarState, type ToolbarEvents } from "@/ui/toolbar";

describe("Two-line Toolbar & macOS System Menus", () => {
  let originalWindow: any;
  let originalDocument: any;
  let elementsById: Record<string, any> = {};

  function createMockElement(tag: string = "div"): any {
    const children: any[] = [];
    const classes = new Set<string>();
    const listeners: Record<string, ((e: any) => void)[]> = {};

    const el: any = {
      tagName: tag.toUpperCase(),
      id: "",
      className: "",
      style: {},
      innerHTML: "",
      textContent: "",
      type: "button",
      title: "",
      disabled: false,
      parentElement: null,
      classList: {
        add: (c: string) => {
          classes.add(c);
          el.className = Array.from(classes).join(" ");
        },
        remove: (c: string) => {
          classes.delete(c);
          el.className = Array.from(classes).join(" ");
        },
        contains: (c: string) => classes.has(c),
        toggle: (c: string, force?: boolean) => {
          const next = force !== undefined ? force : !classes.has(c);
          if (next) classes.add(c);
          else classes.delete(c);
          el.className = Array.from(classes).join(" ");
          return next;
        },
      },
      appendChild: vi.fn((child: any) => {
        child.parentElement = el;
        children.push(child);
        return child;
      }),
      closest: (sel: string) => {
        let cur = el;
        while (cur) {
          if (sel === ".toolbar-split-group" && cur.className?.includes("toolbar-split-group")) return cur;
          cur = cur.parentElement;
        }
        return null;
      },
      addEventListener: vi.fn((event: string, cb: any) => {
        listeners[event] = listeners[event] || [];
        listeners[event].push(cb);
      }),
      children,
      querySelector: (selector: string) => {
        if (selector.startsWith("#")) {
          const id = selector.slice(1);
          return elementsById[id] || null;
        }
        return null;
      },
      querySelectorAll: (_selector: string) => [],
    };

    return new Proxy(el, {
      set(target, prop, value) {
        target[prop] = value;
        if (prop === "className" && typeof value === "string") {
          classes.clear();
          value.split(/\s+/).filter(Boolean).forEach((c) => classes.add(c));
        }
        if (prop === "id" && value) {
          elementsById[value] = target;
        }
        return true;
      },
    });
  }

  const dummyEvents: ToolbarEvents = {
    onNewPlan: vi.fn(),
    onOpenPlan: vi.fn(),
    onSavePlan: vi.fn(),
    onSavePlanAs: vi.fn(),
    onSaveXML: vi.fn(),
    onExportCSV: vi.fn(),
    onUndo: vi.fn(),
    onRedo: vi.fn(),
    onRotateSelected: vi.fn(),
    onDeleteSelected: vi.fn(),
    onToggleFeeder: vi.fn(),
    onResetView: vi.fn(),
    onToggleSnapping: vi.fn(),
    onToggleColorCode: vi.fn(),
    onToggleTrackLabels: vi.fn(),
    onTogglePowerSim: vi.fn(),
    onToggleTheme: vi.fn(),
  };

  beforeEach(() => {
    elementsById = {};
    originalDocument = (globalThis as any).document;
    originalWindow = (globalThis as any).window;

    const mockBody = createMockElement("body");
    const mockApp = createMockElement("div");
    mockApp.id = "app";

    (globalThis as any).document = {
      body: mockBody,
      createElement: (tag: string) => createMockElement(tag),
      getElementById: (id: string) => elementsById[id] || (id === "app" ? mockApp : null),
      querySelectorAll: () => [],
      addEventListener: vi.fn(),
    };

    (globalThis as any).window = {
      location: { search: "" },
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    };
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
    (globalThis as any).window = originalWindow;
  });

  it("renders a two-line toolbar on standard desktop/web (non-macOS)", () => {
    (globalThis as any).window.__FORCE_MAC_MENU__ = false;
    expect(isMacOS()).toBe(false);

    const host = createMockElement("div");
    createToolbar(host, dummyEvents);

    // Both rows must be generated
    const container = host.children[0];
    expect(container).toBeDefined();
    expect(container.className).toBe("toolbar-container");
    expect(container.children.length).toBe(2);

    const menubarRow = container.children[0];
    const actionsRow = container.children[1];

    expect(menubarRow.className).toContain("toolbar-menubar-row");
    expect(actionsRow.className).toContain("toolbar-actions-row");
    expect(menubarRow.style.display).not.toBe("none");

    // Menubar contains the dropdown menus: Layout, Edit, View, Settings, Help
    expect(elementsById["tb-layout"]).toBeDefined();
    expect(elementsById["tb-edit"]).toBeDefined();
    expect(elementsById["tb-view"]).toBeDefined();
    expect(elementsById["tb-settings"]).toBeDefined();
    expect(elementsById["tb-help"]).toBeDefined();
    expect(elementsById["tb-brand-trigger"]).toBeDefined();

    // File name and track count are present
    expect(elementsById["tb-file-name"]).toBeDefined();
    expect(elementsById["tb-piece-count"]).toBeDefined();

    // Actions line contains quick actions and tools
    expect(elementsById["tb-new"]).toBeDefined();
    expect(elementsById["tb-open"]).toBeDefined();
    expect(elementsById["tb-save"]).toBeDefined();
    expect(elementsById["tb-snap"]).toBeDefined();
    expect(elementsById["tb-selection-lock"]).toBeDefined();
    expect(elementsById["tb-measure"]).toBeDefined();
    expect(elementsById["tb-colors"]).toBeDefined();
    expect(elementsById["tb-labels"]).toBeDefined();
    expect(elementsById["tb-power"]).toBeDefined();
    expect(elementsById["tb-3d"]).toBeDefined();
  });

  it("hides the in-app menubar row on macOS, relying on native system menus", () => {
    (globalThis as any).window.__FORCE_MAC_MENU__ = true;
    expect(isMacOS()).toBe(true);

    const host = createMockElement("div");
    createToolbar(host, dummyEvents);

    const container = host.children[0];
    const menubarRow = container.children[0];
    const actionsRow = container.children[1];

    // Menubar row is hidden because macOS uses standard system menus
    expect(menubarRow.style.display).toBe("none");

    // Body gets has-system-menus class
    expect((globalThis as any).document.body.classList.contains("has-system-menus")).toBe(true);

    // Actions row still retains file name, count badge, and all action buttons
    expect(elementsById["tb-file-name"]).toBeDefined();
    expect(elementsById["tb-piece-count"]).toBeDefined();
    expect(elementsById["tb-snap"]).toBeDefined();
    expect(elementsById["tb-3d"]).toBeDefined();
    expect(elementsById["tb-save"]).toBeDefined();
  });

  it("syncs active and warning states to split groups for color and voltage", () => {
    const host = createMockElement("div");
    createToolbar(host, dummyEvents);

    const btnColors = elementsById["tb-colors"];
    const btnPower = elementsById["tb-power"];
    expect(btnColors).toBeDefined();
    expect(btnPower).toBeDefined();

    const colorsGroup = btnColors.parentElement;
    const powerGroup = btnPower.parentElement;
    expect(colorsGroup.className).toContain("toolbar-split-group");
    expect(powerGroup.className).toContain("toolbar-split-group");

    // Initially colorsWrapper has active class
    expect(colorsGroup.classList.contains("active")).toBe(true);

    // Update state to layers mode
    updateToolbarState({ colorMode: "layers" });
    expect(btnColors.classList.contains("active")).toBe(true);
    expect(btnColors.classList.contains("mode-layers")).toBe(true);
    expect(colorsGroup.classList.contains("active")).toBe(true);
    expect(colorsGroup.classList.contains("mode-layers")).toBe(true);

    // Update state to off
    updateToolbarState({ colorMode: "off" });
    expect(btnColors.classList.contains("active")).toBe(false);
    expect(colorsGroup.classList.contains("active")).toBe(false);

    // Update power sim enabled
    updateToolbarState({ powerSimEnabled: true });
    expect(btnPower.classList.contains("active-warning")).toBe(true);
    expect(powerGroup.classList.contains("active-warning")).toBe(true);

    // Update power sim disabled
    updateToolbarState({ powerSimEnabled: false });
    expect(btnPower.classList.contains("active-warning")).toBe(false);
    expect(powerGroup.classList.contains("active-warning")).toBe(false);
  });
});
