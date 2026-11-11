import type { Catalog } from "@/catalog/catalog";
import type { ModelScale } from "@/core/model-scale";

export const CATEGORY_MAP: Record<string, string[]> = {
  Favorites: ["Favorites", "Favoriten"],
  All: ["All", "Alle"],
  Straights: ["Straights", "Geraden"],
  Curves: ["Curves", "Bögen"],
  Turnouts: ["Turnouts", "Weichen"],
  Crossings: ["Crossings", "Kreuzungen"],
  Accessories: ["Accessories", "Zubehör"],
};

export const AVAILABLE_SCALES: ModelScale[] = ["H0", "TT", "N", "Z", "0", "1", "G"];

export const SCALE_DESCRIPTIONS: Record<ModelScale, { de: string; en: string }> = {
  H0: { de: "1:87 (Fahrbahn ~60 mm)", en: "1:87 (Roadway ~60 mm)" },
  TT: { de: "1:120 (Fahrbahn ~45 mm)", en: "1:120 (Roadway ~45 mm)" },
  N: { de: "1:160 (Fahrbahn ~35 mm)", en: "1:160 (Roadway ~35 mm)" },
  Z: { de: "1:220 (Fahrbahn ~25 mm)", en: "1:220 (Roadway ~25 mm)" },
  "0": { de: "1:45 (Fahrbahn ~115 mm)", en: "1:45 (Roadway ~115 mm)" },
  "1": { de: "1:32 (Fahrbahn ~160 mm)", en: "1:32 (Roadway ~160 mm)" },
  G: { de: "1:22,5 (Fahrbahn ~230 mm)", en: "1:22.5 (Roadway ~230 mm)" },
};

/** Roadway width per scale in mm (full 2-lane width). */
export const ROAD_WIDTH_MM: Record<ModelScale, number> = {
  H0: 60,
  TT: 45,
  N: 35,
  Z: 25,
  "0": 115,
  "1": 160,
  G: 230,
};

export function getRoadWidthMm(scale?: string): number {
  return ROAD_WIDTH_MM[((scale || "H0").toUpperCase() as ModelScale)] ?? 60;
}

import { catalogRegistry } from "@/catalog/registry";

/**
 * Finds the road catalog for a scale. Matches the catalog's `scale` field (case-insensitive),
 * and falls back to the id convention `generic-streets-<scale>`.
 */
export function findRoadCatalogForScale(scale: string): Catalog | undefined {
  const sc = scale.toUpperCase();
  const roads = catalogRegistry.getAll().filter((c) => c.isRoadSystem);
  return (
    roads.find((c) => c.id.toLowerCase() === `generic-streets-${sc.toLowerCase()}`) ??
    roads.find((c) => (c.scale || "").toUpperCase() === sc) ??
    roads.find((c) => c.id.toLowerCase().endsWith(`-${sc.toLowerCase()}`))
  );
}

export const MODEL_CATEGORIES = ["All", "Buildings", "Railway", "Nature", "Custom"];

export const MODEL_CATEGORY_LABELS_DE: Record<string, string> = {
  All: "Alle",
  Buildings: "Gebäude",
  Railway: "Bahn",
  Nature: "Natur",
  Custom: "Eigene",
};

export function formatCatalogTitle(cat: Catalog): string {
  if (cat.id === "favorites") {
    return cat.name;
  }
  if (cat.isRoadSystem) {
    return cat.name || `${cat.manufacturer} ${cat.system}`;
  }
  return `${cat.manufacturer} ${cat.system} ${cat.scale}`;
}

export function formatCatalogSubtitle(cat: Catalog, isDe: boolean, t: any): string {
  if (cat.id === "favorites") {
    const count = cat.entries.length;
    return isDe ? `${count} gespeicherte Favoriten` : `${count} saved favorites`;
  }
  if (cat.isRoadSystem) {
    const width = getRoadWidthMm(cat.scale);
    return isDe ? `Fahrbahnbreite ${width} mm, 2-spurig` : `Road width ${width} mm, 2-lane`;
  }
  const gauge = cat.gauge ?? 16.5;
  const roadbedText =
    cat.hasRoadbed === false
      ? isDe
        ? "ohne Bettung"
        : "without roadbed"
      : isDe
      ? "mit Bettung"
      : "roadbed";

  let conductorsText = "";
  if (cat.conductors === 3) {
    conductorsText = t.catalog.threeConductors;
  } else if (cat.conductors === 2) {
    conductorsText = t.catalog.twoConductors;
  } else if (cat.conductors) {
    conductorsText = isDe ? `${cat.conductors} Leiter` : `${cat.conductors}-conductor`;
  }

  return conductorsText ? `${gauge} mm, ${roadbedText}, ${conductorsText}` : `${gauge} mm, ${roadbedText}`;
}
