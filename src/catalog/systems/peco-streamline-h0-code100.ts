import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./peco-streamline-h0-code100.json";

export const pecoStreamlineH0Catalog: Catalog = catalogData as unknown as Catalog;
export const pecoStreamlineH0Entries: CatalogEntry[] = pecoStreamlineH0Catalog.entries;
