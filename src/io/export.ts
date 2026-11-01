import type { Layout } from "@/core/types";
import { catalogRegistry, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import { getLanguage, getTrackDisplayName, getTrackDisplayCategory } from "@/i18n";

export interface PieceSummaryItem {
  catalogId: string;
  name: string;
  category: string;
  kind: string;
  count: number;
  specification: string;
  totalLengthMeters: number;
  itemType: "track" | "road";
}

export interface ModelSummaryItem {
  id: string;
  name: string;
  category: string;
  type: string;
  dimensions: string;
  count: number;
}

export interface PieceListSummary {
  items: PieceSummaryItem[];
  totalPieces: number;
  totalLengthMeters: number;
  totalObjects: number;

  tracks: {
    items: PieceSummaryItem[];
    totalPieces: number;
    totalLengthMeters: number;
  };
  roads: {
    items: PieceSummaryItem[];
    totalPieces: number;
    totalLengthMeters: number;
  };
  buildings: {
    items: ModelSummaryItem[];
    totalPieces: number;
  };
}

/** Erstellt eine aggregierte Stückliste der im Layout verbauten Gleisstücke, Straßen und Gebäude. */
export function getPieceListSummary(layout: Layout, catalog: Catalog): PieceListSummary {
  const entryMap = new Map<string, CatalogEntry>();
  for (const e of catalog.entries) {
    entryMap.set(e.id, e);
  }

  const trackCounts = new Map<string, number>();
  const roadCounts = new Map<string, number>();

  for (const p of layout.pieces) {
    const entry = entryMap.get(p.catalogId) ?? catalogRegistry.findEntry(p.catalogId);
    const isRoad = entry?.isRoad === true || p.catalogId.startsWith("ST-");
    if (isRoad) {
      roadCounts.set(p.catalogId, (roadCounts.get(p.catalogId) ?? 0) + 1);
    } else {
      trackCounts.set(p.catalogId, (trackCounts.get(p.catalogId) ?? 0) + 1);
    }
  }

  const isDe = getLanguage() === "de";

  function processPieceCounts(counts: Map<string, number>, isRoadList: boolean) {
    const resItems: PieceSummaryItem[] = [];
    let pieces = 0;
    let lengthMeters = 0;

    for (const [catId, count] of counts.entries()) {
      const entry = entryMap.get(catId) ?? catalogRegistry.findEntry(catId);
      const isRoad = entry?.isRoad === true || catId.startsWith("ST-");
      const name = entry
        ? getTrackDisplayName(entry)
        : (isDe ? (isRoad ? `Straße ${catId}` : `Gleis ${catId}`) : (isRoad ? `Road ${catId}` : `Track ${catId}`));
      const category = entry
        ? getTrackDisplayCategory(entry.category)
        : (isDe ? (isRoad ? "Straßen" : "Gleise") : (isRoad ? "Roads" : "Tracks"));
      const kind = entry?.kind ?? (isDe ? "unbekannt" : "unknown");

      let spec = "";
      let lenM = 0;
      if (entry?.length) {
        spec = isRoad ? `L = ${entry.length} mm (B = ${entry.roadWidth ?? 60} mm)` : `L = ${entry.length} mm`;
        lenM = (entry.length / 1000) * count;
      } else if (entry?.radius && entry?.angle) {
        spec = `R = ${entry.radius} mm / ${entry.angle}°`;
        lenM = ((2 * Math.PI * entry.radius * (entry.angle / 360)) / 1000) * count;
      }

      pieces += count;
      lengthMeters += lenM;

      resItems.push({
        catalogId: catId,
        name,
        category,
        kind,
        count,
        specification: spec,
        totalLengthMeters: Math.round(lenM * 100) / 100,
        itemType: isRoadList ? "road" : "track",
      });
    }

    resItems.sort((a, b) => a.catalogId.localeCompare(b.catalogId, undefined, { numeric: true }));
    return {
      items: resItems,
      totalPieces: pieces,
      totalLengthMeters: Math.round(lengthMeters * 100) / 100,
    };
  }

  const tracks = processPieceCounts(trackCounts, false);
  const roads = processPieceCounts(roadCounts, true);

  // 3D-Modelle / Gebäude & Ausstattung erfassen
  const modelMap = new Map<string, ModelSummaryItem>();
  if (layout.models) {
    for (const m of layout.models) {
      const key = `${m.name}|${m.category}|${Math.round(m.width)}|${Math.round(m.depth)}|${Math.round(m.height)}`;
      const existing = modelMap.get(key);
      if (existing) {
        existing.count++;
      } else {
        modelMap.set(key, {
          id: m.id,
          name: m.name,
          category: m.category ?? "buildings",
          type: m.modelType,
          dimensions: `${Math.round(m.width)} × ${Math.round(m.depth)} × ${Math.round(m.height)} mm`,
          count: 1,
        });
      }
    }
  }
  const buildingItems = Array.from(modelMap.values()).sort((a, b) => a.name.localeCompare(b.name));
  const totalBuildings = layout.models?.length ?? 0;

  const allItems = [...tracks.items, ...roads.items];
  const totalPieces = tracks.totalPieces + roads.totalPieces;
  const totalLengthMeters = Math.round((tracks.totalLengthMeters + roads.totalLengthMeters) * 100) / 100;

  return {
    items: allItems,
    totalPieces,
    totalLengthMeters,
    totalObjects: totalPieces + totalBuildings,
    tracks,
    roads,
    buildings: {
      items: buildingItems,
      totalPieces: totalBuildings,
    },
  };
}

/** Erstellt eine aggregierte Stückliste für eine bestimmte Ebene. */
export function getLayerPieceListSummary(layout: Layout, catalog: Catalog, layerId: string): PieceListSummary {
  const filteredLayout: Layout = {
    ...layout,
    pieces: layout.pieces.filter((p) => (p.layerId || "layer-1") === layerId),
    models: layout.models?.filter((m) => (m.layerId || "layer-1") === layerId),
  };
  return getPieceListSummary(filteredLayout, catalog);
}

/** Generates a CSV file of the Parts List for Excel or spreadsheet applications. */
export function exportPieceListCSV(layout: Layout, catalog: Catalog): string {
  const summary = getPieceListSummary(layout, catalog);
  const isDe = getLanguage() === "de";

  const lines: string[] = [];
  lines.push(isDe ? `Stückliste für Gleisplan:;${layout.name}` : `Parts List for Layout:;${layout.name}`);
  lines.push(isDe ? `Maßstab:;${layout.scale}` : `Scale:;${layout.scale}`);
  lines.push(isDe ? `Gleise gesamt:;${summary.tracks.totalPieces} (${summary.tracks.totalLengthMeters.toFixed(2)} m)` : `Total tracks:;${summary.tracks.totalPieces} (${summary.tracks.totalLengthMeters.toFixed(2)} m)`);
  lines.push(isDe ? `Straßen gesamt:;${summary.roads.totalPieces} (${summary.roads.totalLengthMeters.toFixed(2)} m)` : `Total roads:;${summary.roads.totalPieces} (${summary.roads.totalLengthMeters.toFixed(2)} m)`);
  lines.push(isDe ? `Gebäude & 3D-Modelle:;${summary.buildings.totalPieces}` : `Buildings & 3D Models:;${summary.buildings.totalPieces}`);
  lines.push("");

  // 1. Gleise / Railway Tracks
  if (summary.tracks.items.length > 0) {
    lines.push(isDe ? "--- 1. Gleise & Weichen ---" : "--- 1. Railway Tracks ---");
    lines.push(
      isDe
        ? "Art.-Nr.;Bezeichnung;Kategorie;Gleistyp;Spezifikation;Anzahl;Länge (m)"
        : "Part No.;Name;Category;Kind;Specification;Qty;Track Length (m)"
    );
    for (const it of summary.tracks.items) {
      lines.push(
        `"${it.catalogId}";"${it.name}";"${it.category}";"${it.kind}";"${it.specification}";${it.count};${it.totalLengthMeters.toFixed(2)}`
      );
    }
    lines.push("");
  }

  // 2. Straßen / Roads
  if (summary.roads.items.length > 0) {
    lines.push(isDe ? "--- 2. Straßen & Fahrbahnen ---" : "--- 2. Roads & Streets ---");
    lines.push(
      isDe
        ? "Art.-Nr.;Bezeichnung;Kategorie;Typ;Spezifikation;Anzahl;Länge (m)"
        : "Part No.;Name;Category;Kind;Specification;Qty;Road Length (m)"
    );
    for (const it of summary.roads.items) {
      lines.push(
        `"${it.catalogId}";"${it.name}";"${it.category}";"${it.kind}";"${it.specification}";${it.count};${it.totalLengthMeters.toFixed(2)}`
      );
    }
    lines.push("");
  }

  // 3. Gebäude & 3D-Modelle
  if (summary.buildings.items.length > 0) {
    lines.push(isDe ? "--- 3. Gebäude & 3D-Objekte ---" : "--- 3. Buildings & 3D Objects ---");
    lines.push(
      isDe
        ? "Modellname;Kategorie;Typ;Maße (B × T × H mm);Anzahl"
        : "Model Name;Category;Type;Dimensions (W × D × H mm);Qty"
    );
    for (const it of summary.buildings.items) {
      lines.push(`"${it.name}";"${it.category}";"${it.type}";"${it.dimensions}";${it.count}`);
    }
  }

  return lines.join("\r\n");
}

/** Startet einen Browser-Download einer Textdatei. */
export function downloadFile(filename: string, content: string, mimeType = "application/json"): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Öffnet einen Standard-Dateidialog zum Laden einer lokalen Datei. */
export function openFileDialog(accept: string, onContent: (content: string, filename: string) => void): void {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = accept;
  input.style.display = "none";
  input.addEventListener("change", () => {
    const file = input.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (e) => {
      const text = e.target?.result as string;
      if (text) onContent(text, file.name);
    };
    reader.readAsText(file);
  });
  document.body.appendChild(input);
  input.click();
  document.body.removeChild(input);
}
