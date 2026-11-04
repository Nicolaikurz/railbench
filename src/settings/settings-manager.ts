import type { ColorMode } from "@/core/types";

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

