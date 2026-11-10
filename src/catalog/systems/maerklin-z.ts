import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./maerklin-z.json";

export const maerklinZCatalog: Catalog = catalogData as unknown as Catalog;
export const maerklinZEntries: CatalogEntry[] = maerklinZCatalog.entries;
