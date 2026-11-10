import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./tillig-bettungsgleis-tt.json";

export const tilligBettungsgleisTTCatalog: Catalog = catalogData as unknown as Catalog;
export const tilligBettungsgleisTTEntries: CatalogEntry[] = tilligBettungsgleisTTCatalog.entries;
