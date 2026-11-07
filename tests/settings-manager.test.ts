import { describe, it, expect, beforeEach, vi } from "vitest";
import { resetPersonalConfig, saveAppSettings, loadAppSettings, KNOWN_PERSONAL_CONFIG_KEYS } from "@/settings/settings-manager";

describe("settings-manager & personal config reset", () => {
  const store: Record<string, string> = {};

  beforeEach(() => {
    for (const k in store) {
      delete store[k];
    }

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
  });

  it("saves and loads app settings", () => {
    saveAppSettings({ snappingEnabled: false, trackLabelsEnabled: true });
    const loaded = loadAppSettings();
    expect(loaded.snappingEnabled).toBe(false);
    expect(loaded.trackLabelsEnabled).toBe(true);
  });

  it("resets all personal config keys while strictly preserving layout plan", () => {
    // Populate personal settings and credentials
    saveAppSettings({ snappingEnabled: false });
    localStorage.setItem("railbench_theme_pref", "dark");
    localStorage.setItem("theme-pref", "dark");
    localStorage.setItem("railbench_lang", "de");
    localStorage.setItem("railbench_collab_username", "Max");
    localStorage.setItem("railbench_collab_password", "secret123");
    localStorage.setItem("railbench_clipboard", '{"tracks":[]}');
    localStorage.setItem("railbench_3d_env", "hall");
    localStorage.setItem("railbench_show_perf_monitor", "true");
    localStorage.setItem("railbench_custom_user_filter", "foo");

    // Populate project layout plan
    localStorage.setItem("railbench_current_plan", '{"name":"My Layout","pieces":[]}');
    localStorage.setItem("railbench_last_saved", "2026-11-07T12:00:00.000Z");

    // Perform reset
    resetPersonalConfig();

    // Check all known keys are wiped
    for (const key of KNOWN_PERSONAL_CONFIG_KEYS) {
      expect(localStorage.getItem(key)).toBeNull();
    }
    expect(localStorage.getItem("railbench_custom_user_filter")).toBeNull();

    // Verify layout plan is NOT deleted
    expect(localStorage.getItem("railbench_current_plan")).toBe('{"name":"My Layout","pieces":[]}');
    expect(localStorage.getItem("railbench_last_saved")).toBe("2026-11-07T12:00:00.000Z");
  });
});
