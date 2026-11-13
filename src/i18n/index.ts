import type { Language, Translations } from "./types";
import { en } from "./en";
import { de } from "./de";

export * from "./types";
export * from "./catalog";
export { helpDe } from "./help-de";
export { helpEn } from "./help-en";

const STORAGE_KEY = "railbench_language";

const dictionaries: Record<Language, Translations> = {
  en,
  de,
};

let currentLanguage: Language = detectInitialLanguage();
const listeners = new Set<(lang: Language) => void>();

/**
 * Detects the system/browser/environment language.
 * If system language is German (e.g. de, de-DE, de-AT, de-CH) returns "de", else "en".
 */
export function detectSystemLanguage(): Language {
  // 1. Check navigator.languages (preferred languages array)
  if (typeof navigator !== "undefined") {
    if (Array.isArray(navigator.languages) && navigator.languages.length > 0) {
      for (const lang of navigator.languages) {
        if (typeof lang === "string" && lang.trim().length > 0) {
          if (lang.toLowerCase().trim().startsWith("de")) {
            return "de";
          }
          break;
        }
      }
    }

    // 2. Check navigator.language
    if (typeof navigator.language === "string" && navigator.language.trim().length > 0) {
      if (navigator.language.toLowerCase().trim().startsWith("de")) {
        return "de";
      }
    }

    // 3. Legacy navigator properties
    const navAny = navigator as Record<string, any>;
    if (typeof navAny.userLanguage === "string" && navAny.userLanguage.toLowerCase().trim().startsWith("de")) {
      return "de";
    }
    if (typeof navAny.browserLanguage === "string" && navAny.browserLanguage.toLowerCase().trim().startsWith("de")) {
      return "de";
    }
  }

  // 4. Check Intl locale
  try {
    const intlLocale = Intl.DateTimeFormat().resolvedOptions().locale;
    if (typeof intlLocale === "string" && intlLocale.toLowerCase().trim().startsWith("de")) {
      return "de";
    }
  } catch {}

  // 5. Check Node / Electron process.env
  if (typeof process !== "undefined" && process?.env) {
    const envLang = process.env.LANG || process.env.LC_ALL || process.env.LC_MESSAGES || process.env.LANGUAGE;
    if (typeof envLang === "string" && envLang.toLowerCase().trim().startsWith("de")) {
      return "de";
    }
  }

  return "en";
}

/**
 * Automatically selects the language based on system language.
 * If system language is German then selects German ("de"), else English ("en").
 *
 * @param persist Whether to persist the selected language to localStorage (default: true)
 * @returns The selected Language ("de" or "en")
 */
export function autoSelectLanguage(persist = true): Language {
  const target = detectSystemLanguage();
  if (persist) {
    setLanguage(target);
  } else {
    if (currentLanguage !== target) {
      currentLanguage = target;
      if (typeof document !== "undefined" && document.documentElement) {
        document.documentElement.lang = target;
      }
      for (const listener of listeners) {
        try {
          listener(target);
        } catch (e) {
          console.error("Error in language listener:", e);
        }
      }
    }
  }
  return target;
}

function detectInitialLanguage(): Language {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en" || saved === "de") {
      return saved;
    }
  } catch {}

  return detectSystemLanguage();
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

