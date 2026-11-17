import type { Catalog, CatalogEntry } from "../types";
import catalogData from "./bachmann-ez-track-n.json";

export const bachmannEZTrackNCatalog: Catalog = catalogData as unknown as Catalog;
export const bachmannEZTrackNEntries: CatalogEntry[] = bachmannEZTrackNCatalog.entries;
