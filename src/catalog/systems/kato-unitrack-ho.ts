import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./kato-unitrack-ho.json";

export const katoUnitrackH0Catalog: Catalog = catalogData as unknown as Catalog;
export const katoUnitrackH0Entries: CatalogEntry[] = katoUnitrackH0Catalog.entries;
