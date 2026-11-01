import type { Catalog, CatalogEntry } from "./types";
import { maerklinCCatalog } from "./systems/maerklin-c-h0";
import { trixCCatalog } from "./systems/trix-c-h0";
import { maerklinMCatalog } from "./systems/maerklin-m-h0";
import { maerklinKCatalog } from "./systems/maerklin-k-h0";
import { pikoACatalog } from "./systems/piko-a-h0";
import { rocoLineCatalog } from "./systems/roco-line-h0";
import { fleischmannPiccoloCatalog } from "./systems/fleischmann-piccolo-n";
import { tilligTTCatalog } from "./systems/tillig-tt";
import { genericStreetsCatalog } from "./systems/generic-streets-h0";
import { genericStreetsNCatalog } from "./systems/generic-streets-n";

export class CatalogRegistry {
  private catalogs = new Map<string, Catalog>();
  private defaultCatalogId = "maerklin-c-h0";
  private listeners: Array<(catalogs: Catalog[]) => void> = [];

  constructor() {
    // Standard-Gleissysteme registrieren (Märklin C, K, M, Trix, etc.)
    this.register(maerklinCCatalog);
    this.register(maerklinKCatalog);
    this.register(maerklinMCatalog);
    this.register(trixCCatalog);
    this.register(pikoACatalog);
    this.register(rocoLineCatalog);
    this.register(fleischmannPiccoloCatalog);
    this.register(tilligTTCatalog);
    this.register(genericStreetsCatalog);
    this.register(genericStreetsNCatalog);
  }

  /** Registriert ein neues Gleissystem im Katalog. */
  register(catalog: Catalog) {
    if (!catalog.id) {
      catalog.id = `${catalog.manufacturer.toLowerCase()}-${catalog.system.toLowerCase()}-${catalog.scale.toLowerCase()}`.replace(/\s+/g, "-");
    }
    if (!catalog.name) {
      catalog.name = `${catalog.manufacturer} ${catalog.scale} ${catalog.system}`;
    }
    this.catalogs.set(catalog.id, catalog);
    this.notifyListeners();
  }

  /** Gibt alle registrierten Gleissysteme zurück. */
  getAll(): Catalog[] {
    return Array.from(this.catalogs.values());
  }

  /** Gibt ein bestimmtes Gleissystem anhand seiner ID zurück. */
  get(id: string): Catalog | undefined {
    return this.catalogs.get(id);
  }

  /** Gibt das Standard-Gleissystem zurück. */
  getDefault(): Catalog {
    return this.catalogs.get(this.defaultCatalogId) ?? maerklinCCatalog;
  }

  /**
   * Sucht nach einem Gleisstück anhand seiner Artikelnummer / ID.
   * Gibt das CatalogEntry-Objekt zurück (mit übernommener Spurweite aus dem System).
   */
  findEntry(
    entryId: string,
    preferredCatalogId?: string,
  ): CatalogEntry | undefined {
    if (preferredCatalogId) {
      const cat = this.get(preferredCatalogId);
      if (cat) {
        const found = cat.entries.find((e) => e.id === entryId);
        if (found) {
          return {
            gauge: cat.gauge,
            hasRoadbed: cat.hasRoadbed,
            ...found,
          };
        }
      }
    }

    for (const cat of this.catalogs.values()) {
      const found = cat.entries.find((e) => e.id === entryId);
      if (found) {
        return {
          gauge: cat.gauge,
          hasRoadbed: cat.hasRoadbed,
          ...found,
        };
      }
    }

    return undefined;
  }

  /**
   * Sucht nach einem Gleisstück und liefert sowohl Eintrag als auch zugehöriges System.
   */
  findWithCatalog(
    entryId: string,
    preferredCatalogId?: string,
  ): { entry: CatalogEntry; catalog: Catalog } | undefined {
    if (preferredCatalogId) {
      const cat = this.get(preferredCatalogId);
      if (cat) {
        const found = cat.entries.find((e) => e.id === entryId);
        if (found) {
          return {
            entry: { gauge: cat.gauge, hasRoadbed: cat.hasRoadbed, ...found },
            catalog: cat,
          };
        }
      }
    }

    for (const cat of this.catalogs.values()) {
      const found = cat.entries.find((e) => e.id === entryId);
      if (found) {
        return {
          entry: { gauge: cat.gauge, hasRoadbed: cat.hasRoadbed, ...found },
          catalog: cat,
        };
      }
    }

    return undefined;
  }

  /**
   * Lädt und validiert ein Gleissystem aus einem JSON-String.
   * Registriert das System automatisch.
   */
  loadFromJSON(jsonString: string): Catalog {
    let parsed: any;
    try {
      parsed = JSON.parse(jsonString);
    } catch (err) {
      throw new Error(`Ungültiges JSON-Format: ${(err as Error).message}`);
    }

    if (!parsed || typeof parsed !== "object") {
      throw new Error("Katalog-JSON muss ein Objekt sein.");
    }
    if (!parsed.manufacturer || !parsed.system) {
      throw new Error("Katalog muss mindestens 'manufacturer' und 'system' definieren.");
    }
    if (!Array.isArray(parsed.entries)) {
      throw new Error("Katalog muss ein Array 'entries' mit Gleiselementen enthalten.");
    }

    const scale = parsed.scale || "H0";
    let defaultGauge = 16.5;
    if (scale === "N") defaultGauge = 9.0;
    else if (scale === "TT") defaultGauge = 12.0;
    else if (scale === "Z") defaultGauge = 6.5;
    else if (scale === "1") defaultGauge = 45.0;

    const catalog: Catalog = {
      id: parsed.id || `${parsed.manufacturer}-${parsed.system}-${scale}`.toLowerCase().replace(/[^a-z0-9_-]/g, "-"),
      name: parsed.name || `${parsed.manufacturer} ${scale} ${parsed.system}`,
      manufacturer: String(parsed.manufacturer),
      system: String(parsed.system),
      scale,
      gauge: typeof parsed.gauge === "number" ? parsed.gauge : defaultGauge,
      hasRoadbed: typeof parsed.hasRoadbed === "boolean" ? parsed.hasRoadbed : true,
      entries: parsed.entries.map((e: any) => ({
        id: String(e.id),
        name: String(e.name || e.id),
        kind: e.kind || "straight",
        category: e.category || "Geraden",
        description: e.description || "",
        length: typeof e.length === "number" ? e.length : undefined,
        radius: typeof e.radius === "number" ? e.radius : undefined,
        angle: typeof e.angle === "number" ? e.angle : undefined,
        branchRadius: typeof e.branchRadius === "number" ? e.branchRadius : undefined,
        branchAngle: typeof e.branchAngle === "number" ? e.branchAngle : undefined,
        branchLength: typeof e.branchLength === "number" ? e.branchLength : undefined,
        gauge: typeof e.gauge === "number" ? e.gauge : undefined,
        localConnectors: Array.isArray(e.localConnectors) ? e.localConnectors : undefined,
      })),
    };

    this.register(catalog);
    return catalog;
  }

  /** Lädt ein Gleissystem aus einer Datei (z. B. via File-Input / Drag & Drop). */
  async loadFromFile(file: File): Promise<Catalog> {
    const text = await file.text();
    return this.loadFromJSON(text);
  }

  /** Lädt ein Gleissystem über eine URL. */
  async loadFromUrl(url: string): Promise<Catalog> {
    const res = await fetch(url);
    if (!res.ok) throw new Error(`Katalog nicht ladbar: ${url} (${res.statusText})`);
    const text = await res.text();
    return this.loadFromJSON(text);
  }

  /** Exportiert ein Gleissystem als formatiertes JSON. */
  exportCatalogJSON(catalogId: string): string {
    const cat = this.get(catalogId);
    if (!cat) throw new Error(`Katalog nicht gefunden: ${catalogId}`);
    return JSON.stringify(cat, null, 2);
  }

  /** Registriert einen Callback bei Änderungen der Katalogliste. */
  onChange(listener: (catalogs: Catalog[]) => void): () => void {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter((l) => l !== listener);
    };
  }

  private notifyListeners() {
    const list = this.getAll();
    for (const listener of this.listeners) {
      listener(list);
    }
  }
}

export const catalogRegistry = new CatalogRegistry();
