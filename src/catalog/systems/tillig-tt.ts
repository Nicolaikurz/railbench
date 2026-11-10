import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./tillig-tt.json";

export const tilligTTCatalog: Catalog = catalogData as unknown as Catalog;
export const tilligTTEntries: CatalogEntry[] = tilligTTCatalog.entries;
