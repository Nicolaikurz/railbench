import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./maerklin-m-h0.json";

export const maerklinMCatalog: Catalog = catalogData as unknown as Catalog;
export const maerklinMEntries: CatalogEntry[] = maerklinMCatalog.entries;
