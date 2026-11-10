import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./generic-streets-h0.json";

export const genericStreetsCatalog: Catalog = catalogData as unknown as Catalog;
export const genericStreetEntries: CatalogEntry[] = genericStreetsCatalog.entries;
