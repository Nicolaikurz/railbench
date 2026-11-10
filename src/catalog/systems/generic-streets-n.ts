import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./generic-streets-n.json";

export const genericStreetsNCatalog: Catalog = catalogData as unknown as Catalog;
export const genericStreetNEntries: CatalogEntry[] = genericStreetsNCatalog.entries;
