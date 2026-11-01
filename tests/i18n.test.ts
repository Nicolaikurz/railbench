import { describe, it, expect, beforeEach } from "vitest";
import { getLanguage, setLanguage, toggleLanguage, getTranslations, onLanguageChange } from "@/i18n";
import { en } from "@/i18n/en";
import { de } from "@/i18n/de";

describe("i18n Module", () => {
  beforeEach(() => {
    const store = new Map<string, string>();
    (globalThis as any).localStorage = {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, val: string) => store.set(key, val),
      removeItem: (key: string) => store.delete(key),
      clear: () => store.clear(),
    };
    setLanguage("en");
  });

  it("should initialize with English by default when set", () => {
    expect(getLanguage()).toBe("en");
    expect(getTranslations().toolbar.fileMenu.newPlan).toBe("New Plan");
  });

  it("should switch to German and update translations", () => {
    let triggered = false;
    let receivedLang = "";
    const unsub = onLanguageChange((lang) => {
      triggered = true;
      receivedLang = lang;
    });

    try {
      setLanguage("de");
      expect(getLanguage()).toBe("de");
      expect(triggered).toBe(true);
      expect(receivedLang).toBe("de");
      expect(getTranslations().toolbar.fileMenu.newPlan).toBe("Neuer Plan");
      expect(localStorage.getItem("railbench_language")).toBe("de");
    } finally {
      unsub();
    }
  });

  it("should toggle language between EN and DE", () => {
    setLanguage("en");
    const nextLang = toggleLanguage();
    expect(nextLang).toBe("de");
    expect(getLanguage()).toBe("de");

    const backLang = toggleLanguage();
    expect(backLang).toBe("en");
    expect(getLanguage()).toBe("en");
  });

  it("should have complete and matching keys in both EN and DE dictionaries", () => {
    function getDeepKeys(obj: Record<string, any>, prefix = ""): string[] {
      return Object.keys(obj).flatMap((key) => {
        const val = obj[key];
        const currentPath = prefix ? `${prefix}.${key}` : key;
        if (typeof val === "object" && val !== null && !Array.isArray(val)) {
          return getDeepKeys(val, currentPath);
        }
        return [currentPath];
      });
    }

    const enKeys = getDeepKeys(en);
    const deKeys = getDeepKeys(de);

    expect(enKeys.sort()).toEqual(deKeys.sort());

    // Verify none of the keys have empty string values
    for (const key of enKeys) {
      const parts = key.split(".");
      let valEn: any = en;
      let valDe: any = de;
      for (const p of parts) {
        valEn = valEn[p];
        valDe = valDe[p];
      }
      expect(typeof valEn).toBe("string");
      expect(typeof valDe).toBe("string");
      expect(valEn.trim().length).toBeGreaterThan(0);
      expect(valDe.trim().length).toBeGreaterThan(0);
    }
  });

  it("should correctly translate catalog track names between English and German", async () => {
    const { getTrackDisplayName, getTrackDisplayCategory } = await import("@/i18n");

    const straightPiece = { id: "24188", name: "Gerades Gleis 188,3 mm" };
    const curvedPiece = { id: "24130", name: "Gebogenes Gleis R1 / 30°" };
    const turnoutPiece = { id: "24611", name: "Weiche links 24,3°" };
    const slimTurnout = { id: "24712", name: "Schlanke Weiche rechts 236,1 mm" };
    const bufferPiece = { id: "24977", name: "Prellbock-Gleis 77,5 mm" };

    // In English
    setLanguage("en");
    expect(getTrackDisplayName(straightPiece)).toBe("Straight track 188.3 mm");
    expect(getTrackDisplayName(curvedPiece)).toBe("Curved track R1 / 30°");
    expect(getTrackDisplayName(turnoutPiece)).toBe("Turnout left 24.3°");
    expect(getTrackDisplayName(slimTurnout)).toBe("Slim turnout right 236.1 mm");
    expect(getTrackDisplayName(bufferPiece)).toBe("Buffer stop track 77.5 mm");
    expect(getTrackDisplayCategory("Geraden")).toBe("Straights");
    expect(getTrackDisplayCategory("Weichen")).toBe("Turnouts");

    // In German
    setLanguage("de");
    expect(getTrackDisplayName(straightPiece)).toBe("Gerades Gleis 188,3 mm");
    expect(getTrackDisplayName(curvedPiece)).toBe("Gebogenes Gleis R1 / 30°");
    expect(getTrackDisplayName(turnoutPiece)).toBe("Weiche links 24,3°");
    expect(getTrackDisplayName(slimTurnout)).toBe("Schlanke Weiche rechts 236,1 mm");
    expect(getTrackDisplayName(bufferPiece)).toBe("Prellbock-Gleis 77,5 mm");
    expect(getTrackDisplayCategory("Geraden")).toBe("Geraden");
    expect(getTrackDisplayCategory("Weichen")).toBe("Weichen");
  });
});
