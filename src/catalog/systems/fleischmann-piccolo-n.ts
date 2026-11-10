import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./fleischmann-piccolo-n.json";

export const fleischmannPiccoloCatalog: Catalog = catalogData as unknown as Catalog;
export const fleischmannPiccoloEntries: CatalogEntry[] = fleischmannPiccoloCatalog.entries;
