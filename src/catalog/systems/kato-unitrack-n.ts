import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./kato-unitrack-n.json";

export const katoUnitrackNCatalog: Catalog = catalogData as unknown as Catalog;
export const katoUnitrackNEntries: CatalogEntry[] = katoUnitrackNCatalog.entries;
