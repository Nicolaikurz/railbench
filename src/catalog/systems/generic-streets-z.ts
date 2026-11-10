import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./generic-streets-z.json";

export const genericStreetsZCatalog: Catalog = catalogData as unknown as Catalog;
export const genericStreetZEntries: CatalogEntry[] = genericStreetsZCatalog.entries;
