import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./piko-a-h0.json";

export const pikoACatalog: Catalog = catalogData as unknown as Catalog;
export const pikoAEntries: CatalogEntry[] = pikoACatalog.entries;
