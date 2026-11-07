import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import type { Layout, TrackPiece, Model3D } from "@/core/types";
import { createToolbar, updateToolbarState, type ToolbarEvents } from "@/ui/toolbar";
import { SelectionBar } from "@/ui/selection-bar";

describe("Selection Lock & Deleted Layer Recovery", () => {
  let originalDocument: any;
  let originalWindow: any;
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
      value: "",
      type: "button",
      title: "",
      disabled: false,
      classList: {
        add: (c: string) => classes.add(c),
        remove: (c: string) => classes.delete(c),
        toggle: (c: string, force?: boolean) => {
          if (force !== undefined) {
            if (force) classes.add(c);
            else classes.delete(c);
            return force;
          }
          if (classes.has(c)) {
            classes.delete(c);
            return false;
          } else {
            classes.add(c);
            return true;
          }
        },
        contains: (c: string) => classes.has(c),
      },
      appendChild: vi.fn((child: any) => {
        children.push(child);
        return child;
      }),
      append: vi.fn((...args: any[]) => {
        children.push(...args);
      }),
      addEventListener: vi.fn((event: string, cb: (e: any) => void) => {
        listeners[event] = listeners[event] || [];
        listeners[event]!.push(cb);
      }),
      click: () => {
        listeners["click"]?.forEach((cb) => cb({ target: el, stopPropagation: vi.fn() }));
      },
      querySelector: (sel: string): any => {
        const cleanId = sel.startsWith("#") ? sel.slice(1) : sel;
        if (!elementsById[cleanId]) {
          elementsById[cleanId] = createMockElement(sel.includes("select") ? "select" : sel.includes("input") ? "input" : "div");
          elementsById[cleanId].id = cleanId;
        }
        return elementsById[cleanId];
      },
      querySelectorAll: () => [],
      insertAdjacentHTML: vi.fn((pos: string, html: string) => {
        el.innerHTML = html + el.innerHTML;
      }),
    };

    return el;
  }

  beforeEach(() => {
    originalDocument = (globalThis as any).document;
    originalWindow = (globalThis as any).window;
    elementsById = {};

    (globalThis as any).window = {
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    };

    const mockHead = {
      appendChild: vi.fn(),
    };
    const mockBody = {
      appendChild: vi.fn(),
    };

    (globalThis as any).document = {
      createElement: vi.fn((tag: string) => {
        const el = createMockElement(tag);
        let idVal = "";
        Object.defineProperty(el, "id", {
          get: () => idVal,
          set: (val: string) => {
            idVal = val;
            elementsById[val] = el;
          },
        });
        return el;
      }),
      getElementById: vi.fn((id: string) => {
        if (!elementsById[id]) {
          elementsById[id] = createMockElement("div");
          elementsById[id].id = id;
        }
        return elementsById[id];
      }),
      querySelectorAll: vi.fn(() => []),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      head: mockHead,
      body: mockBody,
    };
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
    (globalThis as any).window = originalWindow;
  });

  it("renders the selection lock button in toolbar and updates state correctly", () => {
    const events: ToolbarEvents = {
      onToggleSelectionLock: vi.fn(),
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

    const host = createMockElement("div");
    createToolbar(host, events);

    const btnLock = elementsById["tb-selection-lock"];
    expect(btnLock).toBeDefined();

    // Clicking button invokes onToggleSelectionLock
    btnLock.click();
    expect(events.onToggleSelectionLock).toHaveBeenCalledTimes(1);

    // Update state to selectionLockDisabled = true
    updateToolbarState({ selectionLockDisabled: true });
    expect(btnLock.classList.contains("active")).toBe(true);

    // Update state to selectionLockDisabled = false
    updateToolbarState({ selectionLockDisabled: false });
    expect(btnLock.classList.contains("active")).toBe(false);
  });

  it("allows selecting items on deleted layers and across locked layers when selection lock is disabled", () => {
    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      layers: [
        { id: "layer-1", name: "Ebene 1", visible: true, locked: false, heightMm: 0 },
        { id: "layer-2", name: "Ebene 2 (Gesperrt)", visible: true, locked: true, heightMm: 50 },
      ],
      pieces: [
        { id: "p1", catalogId: "24188", transform: { x: 0, y: 0, rotation: 0 }, layerId: "layer-1", connectors: [] },
        { id: "p2", catalogId: "24188", transform: { x: 100, y: 0, rotation: 0 }, layerId: "layer-2", connectors: [] },
        { id: "p-orphan", catalogId: "24188", transform: { x: 200, y: 0, rotation: 0 }, layerId: "deleted-layer-xyz", connectors: [] },
      ],
      models: [
        {
          id: "m-orphan",
          name: "Station",
          category: "buildings",
          transform: { x: 0, y: 0, rotation: 0 },
          width: 200,
          depth: 100,
          height: 80,
          modelType: "builtin",
          layerId: "deleted-layer-xyz",
        },
      ],
    };

    function isPieceInteractive(piece: TrackPiece, selectionLockDisabled: boolean, activeId: string): boolean {
      if (selectionLockDisabled) return true;
      const pieceLayerId = piece.layerId || "layer-1";
      const layerExists = layout.layers?.some((l) => l.id === pieceLayerId);
      if (!layerExists) return true; // Orphaned piece on deleted layer must be selectable!
      if (pieceLayerId !== activeId) return false;
      const layer = layout.layers?.find((l) => l.id === pieceLayerId);
      return !layer || (layer.visible && !layer.locked);
    }

    function isModelInteractive(model: Model3D, selectionLockDisabled: boolean, activeId: string): boolean {
      if (selectionLockDisabled) return true;
      const modelLayerId = model.layerId || "layer-1";
      const layerExists = layout.layers?.some((l) => l.id === modelLayerId);
      if (!layerExists) return true; // Orphaned model on deleted layer must be selectable!
      if (modelLayerId !== activeId) return false;
      const layer = layout.layers?.find((l) => l.id === modelLayerId);
      return !layer || (layer.visible && !layer.locked);
    }

    // Normal mode: piece on active layer-1 is interactive
    expect(isPieceInteractive(layout.pieces[0]!, false, "layer-1")).toBe(true);

    // Normal mode: piece on locked layer-2 is locked
    expect(isPieceInteractive(layout.pieces[1]!, false, "layer-1")).toBe(false);

    // Normal mode: piece on deleted layer is selectable so user can fix it!
    expect(isPieceInteractive(layout.pieces[2]!, false, "layer-1")).toBe(true);
    expect(isModelInteractive(layout.models![0]!, false, "layer-1")).toBe(true);

    // Unlocked selection mode: ALL pieces on all layers (even locked) are interactive!
    expect(isPieceInteractive(layout.pieces[1]!, true, "layer-1")).toBe(true);
    expect(isPieceInteractive(layout.pieces[2]!, true, "layer-1")).toBe(true);
    expect(isModelInteractive(layout.models![0]!, true, "layer-1")).toBe(true);
  });

  it("displays warning option in SelectionBar when selected piece is on a deleted layer", () => {
    const layout: Layout = {
      name: "Test Layout",
      scale: "H0",
      layers: [
        { id: "layer-1", name: "Ebene 1", visible: true, locked: false, heightMm: 0 },
      ],
      pieces: [
        { id: "p-orphan", catalogId: "24188", transform: { x: 0, y: 0, rotation: 0 }, layerId: "deleted-layer-xyz", connectors: [] },
      ],
    };

    const host = createMockElement("div");
    const bar = new SelectionBar(host, {
      onApplyHeight: vi.fn(),
      onApplyAngle: vi.fn(),
      onSelectLayer: vi.fn(),
      onOpenHeightDialog: vi.fn(),
      onToggleFeeder: vi.fn(),
      onToggleIsolator: vi.fn(),
      onToggleCatenary: vi.fn(),
      onToggleCatenaryFeeder: vi.fn(),
      onToggleCatenaryIsolator: vi.fn(),
      onToggleTurnoutDecoder: vi.fn(),
      onToggleTurnoutLantern: vi.fn(),
      onDelete: vi.fn(),
      getSelectedPieceIds: () => ["p-orphan"],
      getLayout: () => layout,
      getCatalogEntry: () => undefined,
    });

    bar.update();

    const layerSelect = elementsById["sel-bar-layer-select"];
    expect(layerSelect).toBeDefined();
    expect(layerSelect.innerHTML).toMatch(/Deleted layer|Gelöschte Ebene/);
  });
});
