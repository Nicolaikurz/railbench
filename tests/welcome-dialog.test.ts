import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { showWelcomeDialog } from "@/ui/welcome-dialog";

describe("Welcome Dialog", () => {
  let originalDocument: any;

  beforeEach(() => {
    originalDocument = (globalThis as any).document;
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
  });

  function setupMockDom() {
    const listeners: Record<string, ((e: any) => void)[]> = {};
    const elements: Record<string, any> = {};

    const mockCheckbox = {
      checked: false,
    };
    elements["#welcome-remember-check"] = mockCheckbox;

    const createMockButton = (id: string) => {
      const btnListeners: Record<string, (() => void)[]> = {};
      const btn = {
        id,
        addEventListener: (event: string, cb: () => void) => {
          btnListeners[event] = btnListeners[event] || [];
          btnListeners[event]!.push(cb);
        },
        click: () => {
          btnListeners["click"]?.forEach((cb) => cb());
        },
      };
      elements[id] = btn;
      return btn;
    };

    createMockButton("#welcome-btn-recent");
    createMockButton("#welcome-btn-new");
    createMockButton("#welcome-btn-demo");
    createMockButton("#welcome-btn-open");
    createMockButton("#welcome-close-btn");

    const mockModal = {
      className: "",
      innerHTML: "",
      classList: {
        add: (cls: string) => {},
      },
      querySelector: (sel: string) => elements[sel] || null,
      remove: () => {},
    };

    (globalThis as any).document = {
      createElement: (tag: string) => {
        if (tag === "div") return mockModal;
        return {};
      },
      body: {
        appendChild: (child: any) => {},
      },
      addEventListener: (event: string, cb: any) => {
        listeners[event] = listeners[event] || [];
        listeners[event]!.push(cb);
      },
      removeEventListener: (event: string, cb: any) => {
        if (listeners[event]) {
          listeners[event] = listeners[event]!.filter((fn) => fn !== cb);
        }
      },
      dispatchEvent: (e: any) => {
        listeners[e.type]?.forEach((cb) => cb(e));
      },
    };

    return { elements, mockCheckbox };
  }

  it("schließt mit 'demo', wenn der Demo-Button geklickt wird", async () => {
    const { elements } = setupMockDom();
    const promise = showWelcomeDialog(false);

    elements["#welcome-btn-demo"].click();
    const result = await promise;

    expect(result.action).toBe("demo");
    expect(result.dontAskAgain).toBe(false);
  });

  it("merkt sich die Auswahl, wenn die Checkbox aktiviert ist", async () => {
    const { elements, mockCheckbox } = setupMockDom();
    const promise = showWelcomeDialog(true, "MeinPlan.railbench");

    mockCheckbox.checked = true;
    elements["#welcome-btn-recent"].click();
    const result = await promise;

    expect(result.action).toBe("recent");
    expect(result.dontAskAgain).toBe(true);
  });

  it("reagiert auf Escape und wählt bei vorhandenem Plan 'recent'", async () => {
    setupMockDom();
    const promise = showWelcomeDialog(true, "MeinPlan.railbench");

    (globalThis as any).document.dispatchEvent({ type: "keydown", key: "Escape", preventDefault: () => {} });
    const result = await promise;

    expect(result.action).toBe("recent");
  });
});
