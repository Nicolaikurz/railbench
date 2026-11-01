export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export class ThemeManager {
  private preference: ThemePreference = "system";
  private listeners: ((resolved: ResolvedTheme, pref: ThemePreference) => void)[] = [];

  constructor() {
    const saved = localStorage.getItem("railbench_theme_pref") as ThemePreference | null;
    if (saved === "light" || saved === "dark" || saved === "system") {
      this.preference = saved;
    }

    // Auf System-Wechsel (z.B. OS wechselt automatisch auf Dark Mode) lauschen
    if (typeof window !== "undefined" && window.matchMedia) {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      mq.addEventListener("change", () => {
        if (this.preference === "system") {
          this.apply();
        }
      });
    }

    this.apply();
  }

  getPreference(): ThemePreference {
    return this.preference;
  }

  getResolvedTheme(): ResolvedTheme {
    if (typeof window !== "undefined" && window.matchMedia) {
      if (this.preference === "system") {
        return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
      }
    }
    return this.preference === "dark" ? "dark" : "light";
  }

  setPreference(pref: ThemePreference) {
    this.preference = pref;
    localStorage.setItem("railbench_theme_pref", pref);
    this.apply();
  }

  cycle(): ThemePreference {
    if (this.preference === "system") {
      this.setPreference("dark");
    } else if (this.preference === "dark") {
      this.setPreference("light");
    } else {
      this.setPreference("system");
    }
    return this.preference;
  }

  private apply() {
    const resolved = this.getResolvedTheme();
    if (typeof document !== "undefined") {
      document.documentElement.setAttribute("data-theme", resolved);
      document.documentElement.setAttribute("data-theme-pref", this.preference);
    }
    for (const listener of this.listeners) {
      listener(resolved, this.preference);
    }
  }

  onChange(callback: (resolved: ResolvedTheme, pref: ThemePreference) => void) {
    this.listeners.push(callback);
    callback(this.getResolvedTheme(), this.preference);
  }
}

