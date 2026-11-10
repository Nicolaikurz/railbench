import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./maerklin-c-h0.json";

export const maerklinCCatalog: Catalog = catalogData as unknown as Catalog;
export const maerklinCEntries: CatalogEntry[] = maerklinCCatalog.entries;
