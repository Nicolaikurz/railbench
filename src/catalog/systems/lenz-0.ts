import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./lenz-0.json";

export const lenz0Catalog: Catalog = catalogData as unknown as Catalog;
export const lenz0Entries: CatalogEntry[] = lenz0Catalog.entries;
