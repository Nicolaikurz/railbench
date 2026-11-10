import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./minitrix-n.json";

export const minitrixNCatalog: Catalog = catalogData as unknown as Catalog;
export const minitrixNEntries: CatalogEntry[] = minitrixNCatalog.entries;
