import type { Layout, TrackPiece, Transform } from "@/core/types";
import {
  computeWorldConnectors,
  catalogRegistry,
  type Catalog,
} from "@/catalog/catalog";
import { ensureDefaultLayers } from "@/core/layout";
import { fromJSON } from "@/io/serialize";
import { AddPiecesCommand } from "@/editor/commands";
import type { History } from "@/editor/commands";
import type { ModelScale } from "@/core/model-scale";

import rawAlpenlandH0 from "../../demotracks/alpenland-h0.railbench?raw";
import rawMainlineH0 from "../../demotracks/mainline-h0.railbench?raw";
import rawSwitchingN from "../../demotracks/switching-n.railbench?raw";
import rawGardenG from "../../demotracks/garden-g.railbench?raw";
import rawShunting0 from "../../demotracks/shunting-0.railbench?raw";
import rawSuitcaseZ from "../../demotracks/suitcase-z.railbench?raw";

export const DEMO_LAYOUT_NAME = "Alpenland H0 Demo";

export interface DemoLayoutDef {
  id: string;
  name: string;
  nameEn: string;
  scale: ModelScale;
  systemName: string;
  catalogId: string;
  description: string;
  descriptionEn: string;
  dimensionsMm: { width: number; height: number };
  pieceCount: number;
  icon: string;
  tags: string[];
  createLayout: () => Layout;
}

/**
 * 1. Alpenland H0 Demo (demotracks/alpenland-h0.railbench)
 */
export function createAlpenlandH0Layout(): Layout {
  return fromJSON(rawAlpenlandH0);
}

/**
 * 2. Zweigleisige Hauptstrecke (demotracks/mainline-h0.railbench)
 */
export function createMainlineH0Layout(): Layout {
  return fromJSON(rawMainlineH0);
}

/**
 * 3. Rangier- & Industrieanlage Spur N (demotracks/switching-n.railbench)
 */
export function createSwitchingNLayout(): Layout {
  return fromJSON(rawSwitchingN);
}

/**
 * 4. Gartenbahn Rundkurs Spur G (demotracks/garden-g.railbench)
 */
export function createGardenGLayout(): Layout {
  return fromJSON(rawGardenG);
}

/**
 * 5. Rangieranlage Spur 0 Inglenook (demotracks/shunting-0.railbench)
 */
export function createShunting0Layout(): Layout {
  return fromJSON(rawShunting0);
}

/**
 * 6. Koffer-Anlage Spur Z (demotracks/suitcase-z.railbench)
 */
export function createSuitcaseZLayout(): Layout {
  return fromJSON(rawSuitcaseZ);
}

/**
 * Katalog aller verfügbaren Demo-Layouts (aus dem Ordner demotracks/).
 */
export const DEMO_LAYOUTS: DemoLayoutDef[] = [
  {
    id: "alpenland-h0",
    name: "Alpenland H0 Demo",
    nameEn: "Alpine H0 Demo",
    scale: "H0",
    systemName: "Märklin C-Gleis (H0)",
    catalogId: "maerklin-c-h0",
    description:
      "Klassischer Alpen-Rundkurs mit Ausweichgleis, Prellbock, Einspeisung, Bergbahnhof und Stellwerk.",
    descriptionEn:
      "Classic alpine circuit with passing siding, buffer stop, power feeder, mountain station, and signal box.",
    dimensionsMm: { width: 1800, height: 1000 },
    pieceCount: 21,
    icon: "🏔️",
    tags: ["H0", "Märklin", "Oval", "Bahnhof", "Klassiker"],
    createLayout: createAlpenlandH0Layout,
  },
  {
    id: "mainline-h0",
    name: "Zweigleisige Hauptstrecke",
    nameEn: "Double-Track Mainline",
    scale: "H0",
    systemName: "Märklin C-Gleis (H0)",
    catalogId: "maerklin-c-h0",
    description:
      "Vollständige 2-gleisige Hauptstrecke (R1/R2) mit Güter- und Lokschuppen-Abstellgleisen, Prellböcken, Hauptbahnhof, BW-Lokschuppen und Stellwerk.",
    descriptionEn:
      "Complete double-track mainline circuit (R1/R2) with freight and engine shed sidings, buffer stops, main station, depot, and signal box.",
    dimensionsMm: { width: 2500, height: 1300 },
    pieceCount: 46,
    icon: "🚂",
    tags: ["H0", "Märklin", "2-Gleisig", "Hauptstrecke", "BW"],
    createLayout: createMainlineH0Layout,
  },
  {
    id: "switching-n",
    name: "Rangier- & Industrieanlage",
    nameEn: "Industrial Shunting Yard",
    scale: "N",
    systemName: "Minitrix (N)",
    catalogId: "minitrix-n",
    description:
      "Kompakte Spur-N Anlage mit R2-Stammoval, 3 parallelen Industrie-Anschlussgleisen, Entkupplungsgleisen, Prellböcken, Industriewerk und Stellwerk.",
    descriptionEn:
      "Compact N-scale layout with R2 main loop, 3 parallel industrial spurs, uncouplers, buffer stops, factory building, and signal box.",
    dimensionsMm: { width: 1200, height: 650 },
    pieceCount: 31,
    icon: "🏭",
    tags: ["N", "Minitrix", "Rangieren", "Industrie", "Kompakt"],
    createLayout: createSwitchingNLayout,
  },
  {
    id: "garden-g",
    name: "Gartenbahn Rundkurs",
    nameEn: "Garden Railway Circuit",
    scale: "G",
    systemName: "LGB Spur G",
    catalogId: "lgb-g",
    description:
      "Großzügiger Gartenbahn-Rundkurs mit R1-Bögen, 2 Abstellgleisen mit beleuchteten Prellböcken, Gartenbahnhof und historischem Wasserturm.",
    descriptionEn:
      "Generous garden railway loop with R1 curves, 2 sidings with illuminated buffer stops, garden station, and historic water tower.",
    dimensionsMm: { width: 3200, height: 2000 },
    pieceCount: 27,
    icon: "🌲",
    tags: ["G", "LGB", "Gartenbahn", "45 mm", "Outdoor"],
    createLayout: createGardenGLayout,
  },
  {
    id: "shunting-0",
    name: "Rangieranlage Spur 0 (Inglenook)",
    nameEn: "Scale 0 Shunting Puzzle",
    scale: "0",
    systemName: "Lenz Spur 0",
    catalogId: "lenz-0",
    description:
      "Klassisches Inglenook-Rangierrätsel in Spur 0: Ausziehgleis mit Weichenstraße zu 3 parallelen Abstellgleisen, Entkupplern, Prellböcken und Güterschuppen.",
    descriptionEn:
      "Classic Inglenook shunting puzzle in Scale 0: Lead track with turnouts into 3 parallel sidings, uncouplers, buffer stops, and goods shed.",
    dimensionsMm: { width: 2600, height: 750 },
    pieceCount: 17,
    icon: "🧩",
    tags: ["0", "Lenz", "Inglenook", "Rangieren", "32 mm"],
    createLayout: createShunting0Layout,
  },
  {
    id: "suitcase-z",
    name: "Koffer-Anlage Spur Z",
    nameEn: "Miniature Suitcase Layout",
    scale: "Z",
    systemName: "Märklin Spur Z",
    catalogId: "maerklin-z-miniclub",
    description:
      "Feine Miniatur-Modellbahn im Alukoffer-Format mit R2-Rundkurs, Trenn- und Schaltgleis, Bahnhofsgleis mit Entkuppler, Haltepunkt und Alpenchalet.",
    descriptionEn:
      "Delicate miniature suitcase layout with R2 loop, isolating and contact tracks, station spur with uncoupler, alpine halt, and chalet.",
    dimensionsMm: { width: 750, height: 500 },
    pieceCount: 16,
    icon: "💼",
    tags: ["Z", "Märklin Z", "Miniatur", "Koffer", "6.5 mm"],
    createLayout: createSuitcaseZLayout,
  },
];

export function getDemoLayout(id: string): DemoLayoutDef | undefined {
  return DEMO_LAYOUTS.find((d) => d.id === id);
}

/**
 * Erzeugt eine frische Kopie des Standard-Demo-Plans (Alpenland H0)
 */
export function createDemoLayout(_catalog?: Catalog): Layout {
  return createAlpenlandH0Layout();
}

/**
 * Initialisiert das Layout mit einem gewählten Demo-Plan (Standard: Alpenland H0).
 */
export function initDemoTracks(layout: Layout, _catalog?: Catalog, demoId?: string): void {
  const def = (demoId ? getDemoLayout(demoId) : undefined) ?? DEMO_LAYOUTS[0]!;
  const demo = def.createLayout();

  layout.name = demo.name;
  layout.scale = demo.scale;
  layout.feedVoltage = demo.feedVoltage;
  layout.locoCurrentMa = demo.locoCurrentMa;
  layout.locoCount = demo.locoCount;
  layout.turnoutLanternCurrentMa = demo.turnoutLanternCurrentMa;
  layout.turnoutDecoderCurrentMa = demo.turnoutDecoderCurrentMa;
  layout.layers = demo.layers;
  layout.activeLayerId = demo.activeLayerId;
  layout.baseboard = demo.baseboard;
  layout.baseboards = demo.baseboards;
  layout.models = demo.models;
  layout.pieces = demo.pieces;
  ensureDefaultLayers(layout);
}

/**
 * Generiert eine vorgegebene Anzahl an Test-Gleisstücken im Gitter für Performance- & Stresstests.
 */
export function generateStressTracks(
  count: number,
  layout: Layout,
  catalog: Catalog,
  history: History,
  sync: (dirty?: boolean) => void,
): void {
  const entry = catalog.entries[0] || catalogRegistry.getAll()[0]?.entries[0];
  if (!entry) return;

  const newPieces: TrackPiece[] = [];
  const cols = 40;
  const startX = -2000;
  const startY = -1000;

  for (let i = 0; i < count; i++) {
    const col = i % cols;
    const row = Math.floor(i / cols);
    const pieceId = `stress-${Date.now()}-${i}-${Math.random().toString(36).substring(2, 6)}`;
    const transform: Transform = {
      x: startX + col * 200,
      y: startY + row * 80,
      rotation: 0,
    };
    newPieces.push({
      id: pieceId,
      catalogId: entry.id,
      layerId: layout.activeLayerId || layout.layers?.[0]?.id || "default",
      transform,
      connectors: computeWorldConnectors(entry, transform),
    });
  }

  history.run(new AddPiecesCommand(newPieces));
  sync(true);
}
