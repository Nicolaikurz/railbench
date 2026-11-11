import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./generic-streets-g.json";

export const genericStreetsGCatalog: Catalog = catalogData as unknown as Catalog;
export const genericStreetGEntries: CatalogEntry[] = genericStreetsGCatalog.entries;
