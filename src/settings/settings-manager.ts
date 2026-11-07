import type { ColorMode } from "@/core/types";
import type { StartupAction } from "@/ui/welcome-dialog";
export interface AppSettings {
  /** Automatische Einrastfunktion aktiv */
  snappingEnabled?: boolean;
  /** Farbmodus: nach Typ ("types"), nach Ebene ("layers") oder aus ("off") */
  colorMode?: ColorMode;
  /** Gleisbezeichnungen/Artikelnummern im Plan anzeigen */
  trackLabelsEnabled?: boolean;
  /** Spannungsabfall- & Fahrstromsimulation aktiv */
  powerSimEnabled?: boolean;
  /** Lineal / Koordinatenachsen anzeigen */
  rulerVisible?: boolean;
  /** Rechte Sidebar eingeklappt */
  rightSidebarCollapsed?: boolean;
  /** Auswahlsperre aufgehoben (alle Ebenen & gelöschte Ebenen auswählbar) */
  selectionLockDisabled?: boolean;
  /** Startverhalten: "ask" (Willkommensdialog anzeigen), "recent", "new", "demo", "open" */
  startupAction?: StartupAction;
  /** Zuletzt ausgewähltes Gleissystem (z.B. maerklin-c, tillig-tt, fleischmann-n) */
  selectedCatalogId?: string;
  /** Zuletzt aktiver Tab in der linken Seitenleiste */
  activeSidebarTab?: "catalog" | "roads" | "models" | "board" | "bom";
  /** 2D-Kameraansicht (Verschiebung und Zoomfaktor) */
  camera?: {
    x: number;
    y: number;
    zoom: number;
  };
}

const SETTINGS_STORAGE_KEY = "railbench_app_settings";

/**
 * Lädt die zuletzt gespeicherten Benutzereinstellungen aus dem LocalStorage.
 * Gibt ein leeres Objekt zurück, falls noch keine Einstellungen existieren oder ein Fehler auftritt.
 */
export function loadAppSettings(): AppSettings {
  try {
    if (typeof localStorage === "undefined") return {};
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    if (typeof parsed === "object" && parsed !== null) {
      return parsed as AppSettings;
    }
  } catch (err) {
    console.warn("Could not load app settings from localStorage:", err);
  }
  return {};
}

/**
 * Speichert Einstellungen im LocalStorage (führt bestehende und neue Werte zusammen).
 */
export function saveAppSettings(patch: Partial<AppSettings>): void {
  try {
    if (typeof localStorage === "undefined") return;
    const current = loadAppSettings();
    const updated = { ...current, ...patch };
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(updated));
  } catch (err) {
    console.warn("Could not save app settings to localStorage:", err);
  }
}

export const KNOWN_PERSONAL_CONFIG_KEYS = [
  SETTINGS_STORAGE_KEY,
  "railbench_theme_pref",
  "theme-pref",
  "railbench_lang",
  "railbench_collab_username",
  "railbench_collab_color",
  "railbench_collab_userid",
  "railbench_collab_password",
  "railbench_clipboard",
  "railbench_3d_env",
  "railbench_3d_trainhall",
  "railbench_3d_woodplate",
  "railbench_show_perf_monitor",
];

/**
 * Löscht alle persönlichen Einstellungen und Konfigurationen aus dem LocalStorage.
 * Der aktuell geöffnete/gespeicherte Gleisplan bleibt dabei unangetastet.
 */
export function resetPersonalConfig(): void {
  try {
    if (typeof localStorage === "undefined") return;
    const preserveKeys = new Set(["railbench_current_plan", "railbench_last_saved"]);
    const keysToRemove = new Set<string>(KNOWN_PERSONAL_CONFIG_KEYS);

    if (typeof localStorage.length === "number") {
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (!key) continue;
        if (preserveKeys.has(key)) continue;
        if (key.startsWith("railbench_") || key === "theme-pref") {
          keysToRemove.add(key);
        }
      }
    }

    for (const key of keysToRemove) {
      if (!preserveKeys.has(key)) {
        localStorage.removeItem(key);
      }
    }
  } catch (err) {
    console.warn("Could not reset personal config from localStorage:", err);
  }
}

