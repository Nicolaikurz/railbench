import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./generic-streets-tt.json";

export const genericStreetsTTCatalog: Catalog = catalogData as unknown as Catalog;
export const genericStreetTTEntries: CatalogEntry[] = genericStreetsTTCatalog.entries;
