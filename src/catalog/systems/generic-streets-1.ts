import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./generic-streets-1.json";

export const genericStreets1Catalog: Catalog = catalogData as unknown as Catalog;
export const genericStreet1Entries: CatalogEntry[] = genericStreets1Catalog.entries;
