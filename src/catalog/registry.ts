import type { Catalog, CatalogEntry, TrackKind } from "./types";
import { maerklinCCatalog } from "./systems/maerklin-c-h0";
import { maerklinMCatalog } from "./systems/maerklin-m-h0";
import { maerklinKCatalog } from "./systems/maerklin-k-h0";
import { maerklinZCatalog } from "./systems/maerklin-z";
import { trixCCatalog } from "./systems/trix-c-h0";
import{ minitrixNCatalog } from "./systems/minitrix-n";
import { pikoACatalog } from "./systems/piko-a-h0";
import { rocoLineCatalog } from "./systems/roco-line-h0";
import { fleischmannPiccoloCatalog } from "./systems/fleischmann-piccolo-n";
import { pecoStreamlineH0Catalog } from "./systems/peco-streamline-h0-code100";
import { katoUnitrackNCatalog } from "./systems/kato-unitrack-n";
import { genericStreetsCatalog } from "./systems/generic-streets-h0";
import { genericStreetsNCatalog } from "./systems/generic-streets-n";
import { genericStreetsTTCatalog } from "./systems/generic-streets-tt";
import { genericStreetsZCatalog} from "./systems/generic-streets-z";
import { kuehnGleisTTCatalog } from "./systems/kuehn-gleis-tt";
import { tilligEliteH0Catalog } from "./systems/tillig-elite-h0";
import { tilligBettungsgleisTTCatalog } from "./systems/tillig-bettungsgleis-tt";
import { tilligTTCatalog } from "./systems/tillig-tt";
import { rokuhanZCatalog } from "./systems/rokuhan-z";


function parseNum(val: any): number | undefined {
  if (typeof val === "number" && !isNaN(val)) return val;
  if (typeof val === "string") {
    const n = parseFloat(val.trim().replace(",", "."));
    if (!isNaN(n)) return n;
  }
  return undefined;
}

function normalizeKind(kind: any, entry: { radius?: number; branchRadius?: number; branchAngle?: number }): TrackKind {
  const k = String(kind || "").toLowerCase().trim();
  if (k === "straight" || k === "gerade") return "straight";
  if (k === "curve" || k === "bogen" || k === "kurve") return "curve";
  if (k === "turnout-left" || k === "weiche-links" || k === "weiche_links" || k === "weiche links") return "turnout-left";
  if (k === "turnout-right" || k === "weiche-rechts" || k === "weiche_rechts" || k === "weiche rechts") return "turnout-right";
  if (k === "curved-turnout-left" || k === "bogenweiche-links" || k === "bogenweiche links") return "curved-turnout-left";
  if (k === "curved-turnout-right" || k === "bogenweiche-rechts" || k === "bogenweiche rechts") return "curved-turnout-right";
  if (k === "three-way" || k === "dreiweg" || k === "dreiwegweiche") return "three-way";
  if (k === "crossing" || k === "kreuzung") return "crossing";
  if (k === "dkw" || k === "doppelkreuzungsweiche") return "dkw";
  if (k === "buffer-stop" || k === "prellbock") return "buffer-stop";
  if (k === "uncoupler" || k === "entkuppler" || k === "entkupplungsgleis") return "uncoupler";
  if (k === "feeder" || k === "anschlussgleis" || k === "einspeisung") return "feeder";
  if (k === "isolating" || k === "trennstrecke" || k === "isoliergleis") return "isolating";
  if (k === "turntable" || k === "drehscheibe") return "turntable";
  if (k === "transfer-table" || k === "transfertable" || k === "schiebebühne" || k === "schiebebuehne") return "transfer-table";
  if (k === "flex" || k === "flexgleis") return "flex";
  if (k === "accessory" || k === "zubehör" || k === "zubehoer") return "accessory";

  if (entry.branchRadius || entry.branchAngle) return "turnout-left";
  if (entry.radius) return "curve";
  return "straight";
}

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
    this.register(pecoStreamlineH0Catalog);
    this.register(katoUnitrackNCatalog);
    this.register(tilligTTCatalog);
    this.register(genericStreetsCatalog);
    this.register(genericStreetsTTCatalog);
    this.register(genericStreetsNCatalog);
    this.register(genericStreetsZCatalog);
    this.register(maerklinZCatalog);
    this.register(kuehnGleisTTCatalog);
    this.register(tilligEliteH0Catalog);
    this.register(tilligBettungsgleisTTCatalog);
    this.register(rokuhanZCatalog);
    this.register(minitrixNCatalog);

    // Gespeicherte benutzerdefinierte Kataloge aus localStorage laden
    this.loadCustomCatalogsFromStorage();
  }

  private saveCustomCatalogsToStorage() {
    try {
      if (typeof window === "undefined" || !window.localStorage) return;
      const customList = Array.from(this.catalogs.values()).filter((c) => c.isCustom);
      window.localStorage.setItem("railbench_custom_catalogs", JSON.stringify(customList));
    } catch (e) {
      console.warn("Fehler beim Speichern benutzerdefinierter Kataloge:", e);
    }
  }

  private loadCustomCatalogsFromStorage() {
    try {
      if (typeof window === "undefined" || !window.localStorage) return;
      const raw = window.localStorage.getItem("railbench_custom_catalogs");
      if (!raw) return;
      const list = JSON.parse(raw);
      if (Array.isArray(list)) {
        for (const cat of list) {
          if (cat && cat.id && Array.isArray(cat.entries)) {
            cat.isCustom = true;
            this.catalogs.set(cat.id, cat);
          }
        }
      }
    } catch (e) {
      console.warn("Fehler beim Laden benutzerdefinierter Kataloge:", e);
    }
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
   * Unterstützt vollständige Kataloge oder flache Track-Listen.
   * Registriert das System automatisch und persistiert es.
   */
  loadFromJSON(jsonString: string): Catalog {
    const cleanJson = (jsonString || "").trim().replace(/^\uFEFF/, "");
    let parsed: any;
    try {
      parsed = JSON.parse(cleanJson);
    } catch (err) {
      throw new Error(`Ungültiges JSON-Format: ${(err as Error).message}`);
    }

    if (!parsed || typeof parsed !== "object") {
      throw new Error("Katalog-JSON muss ein Objekt oder Array sein.");
    }

    // Prüfen, ob es sich versehentlich um einen Gleisplan handelt
    if (Array.isArray(parsed.pieces) && !parsed.entries && !parsed.tracks && !parsed.gleise) {
      throw new Error(
        "Die gewählte Datei ist ein Gleisplan (enthält 'pieces'), kein Gleiskatalog. Nutze bitte 'Plan öffnen' im Menü.",
      );
    }

    // Array von Gleisen direkt oder unter alternativen Schlüsseln
    if (Array.isArray(parsed)) {
      parsed = {
        name: "Benutzerdefinierter Gleiskatalog",
        manufacturer: "Custom",
        system: "Gleise",
        scale: "H0",
        gauge: 16.5,
        entries: parsed,
      };
    } else if (!Array.isArray(parsed.entries)) {
      if (Array.isArray(parsed.tracks)) parsed.entries = parsed.tracks;
      else if (Array.isArray(parsed.gleise)) parsed.entries = parsed.gleise;
      else if (Array.isArray(parsed.schienen)) parsed.entries = parsed.schienen;
      else if (Array.isArray(parsed.artikel)) parsed.entries = parsed.artikel;
      else if (Array.isArray(parsed.items)) parsed.entries = parsed.items;
      else if (Array.isArray(parsed.elements)) parsed.entries = parsed.elements;
    }

    if (!Array.isArray(parsed.entries)) {
      throw new Error("Katalog muss ein Array 'entries', 'tracks' oder 'gleise' mit Elementen enthalten.");
    }

    const scale = String(parsed.scale || "H0");
    let defaultGauge = 16.5;
    if (scale === "N") defaultGauge = 9.0;
    else if (scale === "TT") defaultGauge = 12.0;
    else if (scale === "Z") defaultGauge = 6.5;
    else if (scale === "1" || scale === "G") defaultGauge = 45.0;

    const catalogGauge = parseNum(parsed.gauge ?? parsed.spurweite) ?? defaultGauge;
    const manufacturer = String(parsed.manufacturer || parsed.hersteller || "Custom");
    const system = String(parsed.system || parsed.name || "Eigenes Gleissystem");

    const catalog: Catalog = {
      id: parsed.id || `custom-${manufacturer}-${system}-${scale}-${Date.now()}`.toLowerCase().replace(/[^a-z0-9_-]/g, "-"),
      name: parsed.name || `${manufacturer} ${scale} ${system}`,
      manufacturer,
      system,
      scale,
      gauge: catalogGauge,
      hasRoadbed: typeof parsed.hasRoadbed === "boolean" ? parsed.hasRoadbed : Boolean(parsed.bettung),
      conductors: typeof parsed.conductors === "number" ? parsed.conductors : (parsed.leiter ?? 2),
      isRoadSystem: Boolean(parsed.isRoadSystem || parsed.istStrasse),
      isCustom: true,
      description: parsed.description ? String(parsed.description) : undefined,
      entries: parsed.entries.map((e: any, index: number) => {
        const length = parseNum(e.length ?? e.laenge ?? e.länge);
        const radius = parseNum(e.radius ?? e.bogenradius);
        const angle = parseNum(e.angle ?? e.winkel ?? e.bogenwinkel);
        const branchRadius = parseNum(e.branchRadius ?? e.abzweigradius);
        const branchAngle = parseNum(e.branchAngle ?? e.abzweigwinkel);
        const branchLength = parseNum(e.branchLength ?? e.abzweiglaenge ?? e.abzweiglänge);
        const itemGauge = parseNum(e.gauge ?? e.spurweite) ?? catalogGauge;
        const kind = normalizeKind(e.kind ?? e.typ ?? e.art ?? e.type, { radius, branchRadius, branchAngle });

        let category = e.category || e.kategorie;
        if (!category) {
          if (kind.includes("turnout") || kind === "three-way" || kind === "dkw") category = "Weichen";
          else if (kind === "curve") category = "Bögen";
          else if (kind === "crossing") category = "Kreuzungen";
          else if (kind === "buffer-stop" || kind === "uncoupler" || kind === "feeder" || kind === "accessory") category = "Zubehör";
          else category = "Geraden";
        }

        return {
          id: String(e.id ?? e.artnr ?? e.artikelnummer ?? e.nr ?? e.nummer ?? `track-${index + 1}`),
          name: String(e.name ?? e.bezeichnung ?? e.titel ?? e.title ?? e.id ?? `Gleis ${index + 1}`),
          nameEn: e.nameEn ? String(e.nameEn) : undefined,
          kind,
          category,
          description: String(e.description ?? e.beschreibung ?? ""),
          descriptionEn: e.descriptionEn ? String(e.descriptionEn) : undefined,
          length,
          radius,
          angle,
          branchRadius,
          branchAngle,
          branchLength,
          gauge: itemGauge,
          hasRoadbed: typeof e.hasRoadbed === "boolean" ? e.hasRoadbed : Boolean(e.bettung ?? parsed.hasRoadbed),
          isRoad: Boolean(e.isRoad || parsed.isRoadSystem),
          roadWidth: parseNum(e.roadWidth ?? e.breite),
          localConnectors: Array.isArray(e.localConnectors) ? e.localConnectors : undefined,
        };
      }),
    };

    this.register(catalog);
    this.saveCustomCatalogsToStorage();
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
