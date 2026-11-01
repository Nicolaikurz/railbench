import type { Language, Translations } from "./types";
import { en } from "./en";
import { de } from "./de";

export * from "./types";
export * from "./catalog";

const STORAGE_KEY = "railbench_language";

const dictionaries: Record<Language, Translations> = {
  en,
  de,
};

let currentLanguage: Language = detectInitialLanguage();
const listeners = new Set<(lang: Language) => void>();

function detectInitialLanguage(): Language {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en" || saved === "de") {
      return saved;
    }
  } catch {}

  // System/Browser-Sprache prüfen
  if (typeof navigator !== "undefined" && navigator.language) {
    if (navigator.language.toLowerCase().startsWith("de")) {
      return "de";
    }
  }
  return "en";
}

/** Aktuell gewählte Sprache ("en" oder "de") abfragen */
export function getLanguage(): Language {
  return currentLanguage;
}

/** Übersetzungs-Wörterbuch für die aktive Sprache abrufen */
export function getTranslations(): Translations {
  return dictionaries[currentLanguage] ?? en;
}

/** Sprache aktiv setzen und alle registrierten Listener benachrichtigen */
export function setLanguage(lang: Language): void {
  if (currentLanguage === lang) return;
  currentLanguage = lang;
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {}

  // HTML-Lang-Attribut aktualisieren
  if (typeof document !== "undefined" && document.documentElement) {
    document.documentElement.lang = lang;
  }

  for (const listener of listeners) {
    try {
      listener(lang);
    } catch (e) {
      console.error("Error in language listener:", e);
    }
  }
}

/** Sprache zwischen "en" und "de" umschalten */
export function toggleLanguage(): Language {
  const next: Language = currentLanguage === "en" ? "de" : "en";
  setLanguage(next);
  return next;
}

/** Listener registrieren, der bei Sprachwechsel aufgerufen wird */
export function onLanguageChange(listener: (lang: Language) => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/**
 * Hilfsfunktion zum Abrufen eines übersetzten Strings mit optionaler Parameterersetzung.
 * Beispiel: `t((x) => x.bom.totalSummary, { count: 12, length: "2.4" })`
 */
export function t(
  accessor: (dict: Translations) => string,
  params?: Record<string, string | number>,
): string {
  const dict = getTranslations();
  let str = accessor(dict);
  if (params) {
    for (const [key, val] of Object.entries(params)) {
      str = str.replace(new RegExp(`\\{${key}\\}`, "g"), String(val));
    }
  }
  return str;
}

