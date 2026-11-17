import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import {
  DEFAULT_KEYMAP,
  KEY_ACTION_DEFS,
  loadKeymap,
  saveKeymap,
  resetKeymap,
  matchesKeyBinding,
  matchesAction,
  eventToShortcutString,
} from "@/settings/keymap-manager";
import { KeyboardController } from "@/editor/keyboard-controller";

describe("Keymap Manager & Custom Keybindings", () => {
  const store: Record<string, string> = {};

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
    resetKeymap();
  });

  it("provides complete default keybindings including +/- zoom and arrow pans", () => {
    expect(DEFAULT_KEYMAP.zoomIn).toContain("+");
    expect(DEFAULT_KEYMAP.zoomIn).toContain("=");
    expect(DEFAULT_KEYMAP.zoomOut).toContain("-");
    expect(DEFAULT_KEYMAP.panUp).toContain("ArrowUp");
    expect(DEFAULT_KEYMAP.panDown).toContain("ArrowDown");
    expect(DEFAULT_KEYMAP.panLeft).toContain("ArrowLeft");
    expect(DEFAULT_KEYMAP.panRight).toContain("ArrowRight");
    expect(DEFAULT_KEYMAP.undo).toContain("Ctrl+Z");
    expect(DEFAULT_KEYMAP.rotateCw).toContain("R");
    expect(DEFAULT_KEYMAP.rotateCcw).toContain("Shift+R");
  });

  it("correctly matches key events with bindings", () => {
    // Zoom in (+ and =)
    expect(matchesKeyBinding({ key: "+", code: "Equal" } as any, "+")).toBe(true);
    expect(matchesKeyBinding({ key: "=", code: "Equal" } as any, "=")).toBe(true);
    expect(matchesKeyBinding({ key: "+", code: "NumpadAdd" } as any, "NumpadAdd")).toBe(true);

    // Zoom out (- and _)
    expect(matchesKeyBinding({ key: "-", code: "Minus" } as any, "-")).toBe(true);
    expect(matchesKeyBinding({ key: "-", code: "NumpadSubtract" } as any, "NumpadSubtract")).toBe(true);

    // Pan (Arrow keys)
    expect(matchesKeyBinding({ key: "ArrowUp", code: "ArrowUp" } as any, "ArrowUp")).toBe(true);
    expect(matchesKeyBinding({ key: "ArrowDown", code: "ArrowDown" } as any, "ArrowDown")).toBe(true);
    expect(matchesKeyBinding({ key: "ArrowLeft", code: "ArrowLeft" } as any, "ArrowLeft")).toBe(true);
    expect(matchesKeyBinding({ key: "ArrowRight", code: "ArrowRight" } as any, "ArrowRight")).toBe(true);

    // Modifiers (Ctrl+Z, Shift+R)
    expect(matchesKeyBinding({ key: "z", code: "KeyZ", ctrlKey: true } as any, "Ctrl+Z")).toBe(true);
    expect(matchesKeyBinding({ key: "R", code: "KeyR", shiftKey: true } as any, "Shift+R")).toBe(true);
    expect(matchesKeyBinding({ key: "r", code: "KeyR", shiftKey: false } as any, "Shift+R")).toBe(false);
  });

  it("normalizes KeyboardEvent into human-readable shortcut string", () => {
    expect(eventToShortcutString({ key: "z", code: "KeyZ", ctrlKey: true } as any)).toBe("Ctrl+Z");
    expect(eventToShortcutString({ key: "S", code: "KeyS", ctrlKey: true, shiftKey: true } as any)).toBe("Ctrl+Shift+S");
    expect(eventToShortcutString({ key: "ArrowUp", code: "ArrowUp" } as any)).toBe("ArrowUp");
    expect(eventToShortcutString({ key: "ArrowDown", code: "ArrowDown", shiftKey: true } as any)).toBe("Shift+ArrowDown");
    expect(eventToShortcutString({ key: "+", code: "NumpadAdd" } as any)).toBe("+");
  });

  it("saves and loads custom keymap from localStorage and allows reset", () => {
    const custom = loadKeymap();
    custom.zoomIn = ["PageUp"];
    custom.zoomOut = ["PageDown"];
    saveKeymap(custom);

    const reloaded = loadKeymap();
    expect(reloaded.zoomIn).toEqual(["PageUp"]);
    expect(reloaded.zoomOut).toEqual(["PageDown"]);

    // Test matchesAction with custom map
    expect(matchesAction({ key: "PageUp", code: "PageUp" } as any, "zoomIn", reloaded)).toBe(true);
    expect(matchesAction({ key: "+", code: "Equal" } as any, "zoomIn", reloaded)).toBe(false);

    // Reset back to defaults
    resetKeymap();
    const afterReset = loadKeymap();
    expect(afterReset.zoomIn).toContain("+");
  });
});

describe("KeyboardController Navigation & Zoom", () => {
  const store: Record<string, string> = {};
  let windowListeners: Record<string, ((e: any) => void)[]> = {};
  let zoomViewMock: any;
  let panViewMock: any;
  let resetViewMock: any;
  let controller: KeyboardController;

  function dispatchMockKey(evtProps: any) {
    const e = {
      type: "keydown",
      preventDefault: vi.fn(),
      stopPropagation: vi.fn(),
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      metaKey: false,
      ...evtProps,
    };
    const cbs = windowListeners["keydown"] || [];
    for (const cb of cbs) {
      cb(e);
    }
  }

  beforeEach(() => {
    windowListeners = {};
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

    vi.stubGlobal("window", {
      addEventListener: (evt: string, cb: any) => {
        windowListeners[evt] = windowListeners[evt] || [];
        windowListeners[evt].push(cb);
      },
      removeEventListener: (evt: string, cb: any) => {
        if (!windowListeners[evt]) return;
        windowListeners[evt] = windowListeners[evt].filter((fn) => fn !== cb);
      },
      getSelection: () => null,
    });

    resetKeymap();

    zoomViewMock = vi.fn();
    panViewMock = vi.fn();
    resetViewMock = vi.fn();

    controller = new KeyboardController({
      getLayout: () => ({ name: "Test", pieces: [], scale: "H0" }) as any,
      getSelection: () => ({ all: () => [], clear: () => {} }) as any,
      getHistory: () => ({ undo: () => {}, redo: () => {} }) as any,
      getOperations: () => ({}) as any,
      getPlanManager: () => ({}) as any,
      getRuler: () => ({ toggle: () => {} }) as any,
      getPerfMonitor: () => ({ toggle: () => {} }) as any,
      is3DMode: () => false,
      toggle3D: vi.fn(),
      zoomView: zoomViewMock,
      panView: panViewMock,
      resetView: resetViewMock,
      isMeasureToolActive: () => false,
      toggleMeasureTool: vi.fn(),
      getMeasureState: () => ({ start: null }),
      setMeasureState: vi.fn(),
      updateMeasureHud: vi.fn(),
      isSnappingEnabled: () => true,
      setSnappingEnabled: vi.fn(),
      getColorMode: () => "off",
      cycleColorMode: vi.fn(),
      isTrackLabelsEnabled: () => true,
      setTrackLabelsEnabled: vi.fn(),
      isPowerSimEnabled: () => false,
      setPowerSimEnabled: vi.fn(),
      toggleRightSidebar: vi.fn(),
      openHeightDialogForSelection: vi.fn(),
      openHelpDialog: vi.fn(),
      openPrintDialog: vi.fn(),
      getCatalogEntryForPiece: () => undefined,
      sync: vi.fn(),
      showToast: vi.fn(),
    });
  });

  afterEach(() => {
    controller.destroy();
  });

  it("zooms in with + key and zooms out with - key", () => {
    dispatchMockKey({ key: "+", code: "Equal" });
    expect(zoomViewMock).toHaveBeenCalledWith(1.15);

    dispatchMockKey({ key: "-", code: "Minus" });
    expect(zoomViewMock).toHaveBeenCalledWith(1 / 1.15);
  });

  it("pans viewport with arrow keys and supports faster shift pan", () => {
    // Up
    dispatchMockKey({ key: "ArrowUp", code: "ArrowUp" });
    expect(panViewMock).toHaveBeenCalledWith(0, 60);

    // Down
    dispatchMockKey({ key: "ArrowDown", code: "ArrowDown" });
    expect(panViewMock).toHaveBeenCalledWith(0, -60);

    // Left
    dispatchMockKey({ key: "ArrowLeft", code: "ArrowLeft" });
    expect(panViewMock).toHaveBeenCalledWith(60, 0);

    // Right
    dispatchMockKey({ key: "ArrowRight", code: "ArrowRight" });
    expect(panViewMock).toHaveBeenCalledWith(-60, 0);

    // Shift + ArrowUp (Fast pan)
    dispatchMockKey({ key: "ArrowUp", code: "ArrowUp", shiftKey: true });
    expect(panViewMock).toHaveBeenCalledWith(0, 180);
  });

  it("reacts dynamically to custom keymap changes", () => {
    const custom = loadKeymap();
    custom.zoomIn = ["i"];
    saveKeymap(custom);

    // Pressing old '+' should no longer zoom in
    zoomViewMock.mockClear();
    dispatchMockKey({ key: "+", code: "Equal" });
    expect(zoomViewMock).not.toHaveBeenCalled();

    // Pressing 'i' should now zoom in
    dispatchMockKey({ key: "i", code: "KeyI" });
    expect(zoomViewMock).toHaveBeenCalledWith(1.15);
  });
});
