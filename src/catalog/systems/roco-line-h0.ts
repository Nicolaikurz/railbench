import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./roco-line-h0.json";

export const rocoLineCatalog: Catalog = catalogData as unknown as Catalog;
export const rocoLineEntries: CatalogEntry[] = rocoLineCatalog.entries;
