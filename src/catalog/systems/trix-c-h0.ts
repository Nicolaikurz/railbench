import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./trix-c-h0.json";

export const trixCCatalog: Catalog = catalogData as unknown as Catalog;
export const trixCEntries: CatalogEntry[] = trixCCatalog.entries;
