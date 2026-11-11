import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./maerklin-1.json";

export const maerklin1Catalog: Catalog = catalogData as unknown as Catalog;
export const maerklin1Entries: CatalogEntry[] = maerklin1Catalog.entries;
