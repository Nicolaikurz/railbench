export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export class ThemeManager {
  private preference: ThemePreference = "system";
  private listeners: ((resolved: ResolvedTheme, pref: ThemePreference) => void)[] = [];
  private mediaQuery: MediaQueryList | null = null;
  private mediaQueryHandler: (() => void) | null = null;

  constructor() {
    let saved: string | null = null;
    try {
      if (typeof localStorage !== "undefined") {
        saved = localStorage.getItem("railbench_theme_pref") || localStorage.getItem("theme-pref");
      }
    } catch {}

    if (saved === "light" || saved === "dark" || saved === "system") {
      this.preference = saved;
    } else {
      this.preference = "system";
    }

    // Auf System-Wechsel (z.B. OS wechselt automatisch auf Dark Mode) lauschen
    if (typeof window !== "undefined" && window.matchMedia) {
      try {
        this.mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
        this.mediaQueryHandler = () => {
          if (this.preference === "system") {
            this.apply();
          }
        };
        if (this.mediaQuery.addEventListener) {
          this.mediaQuery.addEventListener("change", this.mediaQueryHandler);
        } else if ((this.mediaQuery as any).addListener) {
          (this.mediaQuery as any).addListener(this.mediaQueryHandler);
        }
      } catch (err) {
        console.warn("Could not register prefers-color-scheme listener", err);
      }
    }

    this.apply();
  }

  getPreference(): ThemePreference {
    return this.preference;
  }

  getResolvedTheme(): ResolvedTheme {
    if (this.preference === "system") {
      if (typeof window !== "undefined" && window.matchMedia) {
        try {
          return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
        } catch {}
      }
      return "light";
    }
    return this.preference === "dark" ? "dark" : "light";
  }

  setPreference(pref: ThemePreference) {
    this.preference = pref;
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("railbench_theme_pref", pref);
        localStorage.setItem("theme-pref", pref);
      }
    } catch {}

    if (typeof window !== "undefined" && window.electronAPI?.setThemeSource) {
      try {
        window.electronAPI.setThemeSource(pref);
      } catch {}
    }

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
    if (typeof document !== "undefined" && document.documentElement) {
      document.documentElement.setAttribute("data-theme", resolved);
      document.documentElement.setAttribute("data-theme-pref", this.preference);
    }
    for (const listener of this.listeners) {
      try {
        listener(resolved, this.preference);
      } catch (e) {
        console.error("Theme listener error:", e);
      }
    }
  }

  onChange(callback: (resolved: ResolvedTheme, pref: ThemePreference) => void): () => void {
    this.listeners.push(callback);
    callback(this.getResolvedTheme(), this.preference);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== callback);
    };
  }
}

export const themeManager = new ThemeManager();
