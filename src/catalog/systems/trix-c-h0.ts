import type { Catalog, CatalogEntry } from "../types";
import { maerklinCEntries } from "./maerklin-c-h0";

/**
 * Trix H0 C-Gleis Bibliothek
 * Basiert auf der bewährten Geometrie und dem Schotterbett des Märklin C-Gleises,
 * jedoch für 2-Leiter Gleichstrom / DCC mit den offiziellen Trix-Artikelnummern (62xxx statt 24xxx).
 */
export const trixCEntries: CatalogEntry[] = maerklinCEntries
  // Märklin-spezifisches 3-Leiter M-Gleis Übergangsgleis herausfiltern
  .filter((entry) => entry.id !== "24951")
  .map((entry) => {
    const newId = entry.id === "74862"
      ? "66862"
      : entry.id.startsWith("24")
      ? "62" + entry.id.slice(2)
      : entry.id;

    let desc = entry.description;
    if (entry.id === "24922") {
      desc = "Übergangsgleis 180 mm von Trix C-Gleis auf Standard 2-Leiter-Gleise (NEM)";
    } else {
      desc = desc.replace(/Märklin/g, "Trix").replace(/C-Gleis/g, "Trix C-Gleis");
    }

    return {
      ...entry,
      id: newId,
      name: entry.name,
      description: desc,
    };
  });

export const trixCCatalog: Catalog = {
  id: "trix-c-h0",
  name: "Trix H0 C-Gleis",
  manufacturer: "Trix",
  system: "C-Gleis",
  scale: "H0",
  gauge: 16.5,
  hasRoadbed: true,
  conductors: 2,
  entries: trixCEntries,
};

