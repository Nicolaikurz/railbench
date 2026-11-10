import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./maerklin-k-h0.json";

export const maerklinKCatalog: Catalog = catalogData as unknown as Catalog;
export const maerklinKEntries: CatalogEntry[] = maerklinKCatalog.entries;
