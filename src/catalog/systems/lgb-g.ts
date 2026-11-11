import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./lgb-g.json";

export const lgbGCatalog: Catalog = catalogData as unknown as Catalog;
export const lgbGEntries: CatalogEntry[] = lgbGCatalog.entries;
