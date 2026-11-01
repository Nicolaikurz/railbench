import { getLanguage, type Language } from "./index";
import type { CatalogEntry } from "@/catalog/types";

/**
 * Translates a German track catalog name into natural English.
 */
export function translateTrackNameToEn(name: string): string {
  let res = name;

  // Replacements from most specific to least specific
  const patterns: [RegExp, string][] = [
    [/^Basis-Anschlussgleis\b/i, "Feeder track"],
    [/^Anschlussgleis\b/i, "Feeder track"],
    [/^Schlanke Bogenweiche links\b/i, "Slim curved turnout left"],
    [/^Schlanke Bogenweiche rechts\b/i, "Slim curved turnout right"],
    [/^Bogenweiche links\b/i, "Curved turnout left"],
    [/^Bogenweiche rechts\b/i, "Curved turnout right"],
    [/^Schlanke Weiche links\b/i, "Slim turnout left"],
    [/^Schlanke Weiche rechts\b/i, "Slim turnout right"],
    [/^Weiche EW1 links\b/i, "Turnout EW1 left"],
    [/^Weiche EW1 rechts\b/i, "Turnout EW1 right"],
    [/^Weiche links\b/i, "Turnout left"],
    [/^Weiche rechts\b/i, "Turnout right"],
    [/^Weiche WL links\b/i, "Turnout WL left"],
    [/^Weiche WR rechts\b/i, "Turnout WR right"],
    [/^Dreiweg-Weiche\b/i, "Three-way turnout"],
    [/^Dreiwegweiche\b/i, "Three-way turnout"],
    [/^Schlanke DKW\b/i, "Slim double slip turnout"],
    [/^Doppelkreuzungsweiche \(DKW\)\b/i, "Double slip turnout (DKW)"],
    [/^Doppelkreuzungsweiche DKW\b/i, "Double slip turnout DKW"],
    [/^Doppelkreuzungsweiche\b/i, "Double slip turnout"],
    [/^Schlanke Kreuzung\b/i, "Slim crossing"],
    [/^Kreuzung\b/i, "Crossing"],
    [/^Schaltgleis gebogen\b/i, "Curved contact track"],
    [/^Schaltgleis gerade\b/i, "Straight contact track"],
    [/^Schaltgleis\b/i, "Contact track"],
    [/^Kontaktgleissatz gerade\b/i, "Straight contact track set"],
    [/^Entkupplungsgleis\b/i, "Uncoupler track"],
    [/^Prellbock-Bausatz mit Gleis\b/i, "Buffer stop kit with track"],
    [/^Prellbock-Gleis beleuchtet\b/i, "Buffer stop track (illuminated)"],
    [/^Prellbock-Gleis\b/i, "Buffer stop track"],
    [/^Prellbock\b/i, "Buffer stop"],
    [/^C-Gleis Drehscheibe\b/i, "C Track Turntable"],
    [/^Drehscheibe\b/i, "Turntable"],
    [/^Übergangsgleis K-Gleis\b/i, "Transition track to K-Track"],
    [/^Übergangsgleis M-Gleis\b/i, "Transition track to M-Track"],
    [/^Übergangsgleis\b/i, "Transition track"],
    [/^Böschungsendstück\b/i, "Roadbed end piece"],
    [/^Gerades Gleis\b/i, "Straight track"],
    [/^Gebogenes Gleis\b/i, "Curved track"],
  ];

  for (const [re, repl] of patterns) {
    if (re.test(res)) {
      res = res.replace(re, repl);
      break;
    }
  }

  // Decimal comma in numbers to decimal dot in English (e.g. 188,3 mm -> 188.3 mm, 24,3° -> 24.3°)
  res = res.replace(/(\d+),(\d+)/g, "$1.$2");

  return res;
}

/**
 * Translates German track catalog descriptions into natural English.
 */
export function translateTrackDescriptionToEn(desc: string): string {
  let res = desc;

  res = res
    .replace(/Gerades Standardgleis/gi, "Straight standard track")
    .replace(/Basis-Standardgleis/gi, "Base standard track")
    .replace(/Langes gerades Gleis/gi, "Long straight track")
    .replace(/Großes gerades Gleis/gi, "Large straight track")
    .replace(/doppeltes Standardgleis/gi, "double standard length")
    .replace(/Halbes Standardgleis/gi, "Half standard track")
    .replace(/Gerades Gleis/gi, "Straight track")
    .replace(/Gebogenes Gleis/gi, "Curved track")
    .replace(/Gegenbogen\/Ergänzung/gi, "counter-curve / extension")
    .replace(/Ausgleichsstück für schlanke Weichen/gi, "compensation piece for slim turnouts")
    .replace(/Parallelkreis-Abstandsgleis/gi, "parallel spacing track")
    .replace(/Länge passend zu schlanken Weichen/gi, "length matching slim turnouts")
    .replace(/Bogenwinkel/gi, "curve angle")
    .replace(/Bogen/gi, "curve")
    .replace(/Innenbogen/gi, "inner curve")
    .replace(/Außenbogen/gi, "outer curve")
    .replace(/Weiche links/gi, "Turnout left")
    .replace(/Weiche rechts/gi, "Turnout right")
    .replace(/Handhebel und Weichenlaterne/gi, "manual lever and turnout lantern")
    .replace(/Antrieb/gi, "mechanism")
    .replace(/Kreuzung/gi, "Crossing")
    .replace(/Prellbock/gi, "Buffer stop")
    .replace(/mit Prellbock/gi, "with buffer stop")
    .replace(/beleuchteter Prellbock/gi, "illuminated buffer stop")
    .replace(/beleuchtet/gi, "illuminated")
    .replace(/Gleisanschluss/gi, "feeder connector")
    .replace(/Entstörkondensator/gi, "suppressor capacitor")
    .replace(/Analog und Digital/gi, "analog and digital")
    .replace(/Elektroantrieb/gi, "electric drive")
    .replace(/Digital-Decoder/gi, "digital decoder")
    .replace(/nachrüstbar/gi, "retrofittable")
    .replace(/integriert/gi, "integrated")
    .replace(/Schwellen/gi, "ties")
    .replace(/Bettung/gi, "roadbed");

  res = res.replace(/(\d+),(\d+)/g, "$1.$2");

  return res;
}

/**
 * Returns the localized display name for a catalog track entry.
 */
export function getTrackDisplayName(
  entry: CatalogEntry | { id?: string; name: string; nameEn?: string },
  lang: Language = getLanguage()
): string {
  if (lang === "en") {
    if ("nameEn" in entry && entry.nameEn) {
      return entry.nameEn;
    }
    return translateTrackNameToEn(entry.name);
  }
  return entry.name;
}

/**
 * Returns the localized description for a catalog track entry.
 */
export function getTrackDisplayDescription(
  entry: CatalogEntry,
  lang: Language = getLanguage()
): string {
  if (lang === "en") {
    if (entry.descriptionEn) return entry.descriptionEn;
    return translateTrackDescriptionToEn(entry.description);
  }
  return entry.description;
}

/**
 * Returns the localized category name (e.g. "Geraden" -> "Straights").
 */
export function getTrackDisplayCategory(
  category: string,
  lang: Language = getLanguage()
): string {
  if (lang === "en") {
    switch (category) {
      case "Geraden":
        return "Straights";
      case "Bögen":
        return "Curves";
      case "Weichen":
        return "Turnouts";
      case "Kreuzungen":
        return "Crossings";
      case "Zubehör":
        return "Accessories";
      default:
        return category;
    }
  } else {
    switch (category) {
      case "Straights":
        return "Geraden";
      case "Curves":
        return "Bögen";
      case "Turnouts":
        return "Weichen";
      case "Crossings":
        return "Kreuzungen";
      case "Accessories":
        return "Zubehör";
      default:
        return category;
    }
  }
}

