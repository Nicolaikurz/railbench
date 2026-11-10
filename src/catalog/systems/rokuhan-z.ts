import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./rokuhan-z.json";

export const rokuhanZCatalog: Catalog = catalogData as unknown as Catalog;
export const rokuhanZEntries: CatalogEntry[] = rokuhanZCatalog.entries;
