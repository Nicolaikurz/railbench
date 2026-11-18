import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { openPowerSettingsDialog } from "@/ui/power-dialog";
import type { Layout } from "@/core/types";

describe("Power Settings Dialog", () => {
  let originalDocument: any;
  let originalWindow: any;
  let elementsById: Record<string, any>;

  function createMockElement(tag: string): any {
    const listeners: Record<string, Function[]> = {};
    const el: any = {
      tagName: tag.toUpperCase(),
      id: "",
      className: "",
      innerHTML: "",
      textContent: "",
      style: {},
      children: [],
      classList: {
        contains: (c: string) => (el.className || "").split(" ").includes(c),
        add: (c: string) => {
          const list = (el.className || "").split(" ").filter(Boolean);
          if (!list.includes(c)) list.push(c);
          el.className = list.join(" ");
        },
        remove: (c: string) => {
          el.className = (el.className || "").split(" ").filter((x: string) => x !== c).join(" ");
        },
        toggle: (c: string, force?: boolean) => {
          const has = (el.className || "").split(" ").includes(c);
          const state = force !== undefined ? force : !has;
          if (state) el.classList.add(c);
          else el.classList.remove(c);
        },
      },
      setAttribute: vi.fn(),
      getAttribute: vi.fn(),
      removeAttribute: vi.fn(),
      appendChild: (child: any) => {
        el.children.push(child);
        child.parentElement = el;
        return child;
      },
      remove: () => {
        if (el.id && elementsById[el.id]) {
          delete elementsById[el.id];
        }
      },
      addEventListener: (event: string, handler: Function) => {
        listeners[event] = listeners[event] || [];
        listeners[event]!.push(handler);
      },
      removeEventListener: (event: string, handler: Function) => {
        if (!listeners[event]) return;
        listeners[event] = listeners[event]!.filter((h) => h !== handler);
      },
      dispatchEvent: (e: any) => {
        listeners[e.type]?.forEach((h) => h(e));
      },
      querySelector: (sel: string) => {
        if (sel.startsWith("#")) {
          const id = sel.slice(1);
          return elementsById[id] || null;
        }
        return null;
      },
      querySelectorAll: (sel: string) => {
        return [];
      },
      focus: vi.fn(),
      select: vi.fn(),
    };
    return el;
  }

  beforeEach(() => {
    elementsById = {};
    originalDocument = (globalThis as any).document;
    originalWindow = (globalThis as any).window;

    const mockBody = createMockElement("body");

    (globalThis as any).document = {
      body: mockBody,
      createElement: (tag: string) => {
        const el = createMockElement(tag);
        return el;
      },
      getElementById: (id: string) => elementsById[id] || null,
      querySelectorAll: () => [],
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    };

    (globalThis as any).window = {
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    };
  });

  afterEach(() => {
    (globalThis as any).document = originalDocument;
    (globalThis as any).window = originalWindow;
  });

  it("handles catenary tracking when catenary voltage was not separately touched", () => {
    const layout: Layout = {
      name: "Test",
      scale: "H0",
      feedVoltage: 16,
      catenaryCircuitMode: "separate",
      // catenaryFeedVoltage is undefined: user hasn't explicitly customized it
      pieces: [
        {
          id: "p1",
          catalogId: "24188",
          transform: { x: 0, y: 0, rotation: 0 },
          connectors: [],
          powerFeed: true,
          hasCatenary: true,
          catenaryPowerFeed: true,
        },
      ],
    };

    let appliedParams: any = null;
    let liveParams: any = null;

    const modal = createMockElement("div");
    modal.id = "power-settings-modal";
    elementsById["power-settings-modal"] = modal;

    const ids = [
      "power-modal-close", "power-btn-cancel", "power-btn-apply",
      "power-slider-voltage", "power-input-voltage", "power-lbl-voltage",
      "power-btn-volt-dec", "power-btn-volt-inc",
      "power-slider-loco", "power-input-loco-ma", "power-lbl-loco-ma",
      "power-btn-loco-dec", "power-btn-loco-inc",
      "power-catenary-voltage-container", "power-slider-catenary-voltage",
      "power-input-catenary-voltage", "power-lbl-catenary-voltage",
      "power-btn-cat-volt-dec", "power-btn-cat-volt-inc",
      "power-input-loco-count", "power-btn-dec-count", "power-btn-inc-count",
      "power-lbl-feeder-status", "power-lbl-catenary-status",
      "power-feeder-warning-banner", "power-catenary-warning-banner",
      "power-turnout-stats-text", "power-btn-equip-all-turnouts", "power-btn-clear-all-turnouts",
      "power-slider-lantern", "power-input-lantern-ma", "power-lbl-lantern-ma",
      "power-slider-decoder", "power-input-decoder-ma", "power-lbl-decoder-ma",
      "power-summary-details", "power-summary-total-amps", "power-summary-total-ma",
      "power-dialog-booster-warning",
    ];

    ids.forEach((id) => {
      const el = createMockElement("div");
      el.id = id;
      el.value = "16";
      elementsById[id] = el;
    });

    (globalThis as any).document.createElement = (tag: string) => {
      const el = createMockElement(tag);
      el.querySelector = (sel: string) => {
        if (sel.startsWith("#")) {
          return elementsById[sel.slice(1)] || null;
        }
        return null;
      };
      return el;
    };

    openPowerSettingsDialog({
      layout,
      onApply: (p) => { appliedParams = p; },
    });

    // Test: Changing track slider should also update catenary voltage because catenary was not touched
    const sliderVolt = elementsById["power-slider-voltage"];
    sliderVolt.value = "12";
    sliderVolt.dispatchEvent({ type: "input" });

    // Click apply
    elementsById["power-btn-apply"].dispatchEvent({ type: "click" });
    expect(appliedParams.feedVoltage).toBe(12);
    expect(appliedParams.catenaryFeedVoltage).toBe(12);
  });
});
