import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./kuehn-gleis-tt.json";

export const kuehnGleisTTCatalog: Catalog = catalogData as unknown as Catalog;
export const kuehnGleisTTEntries: CatalogEntry[] = kuehnGleisTTCatalog.entries;
