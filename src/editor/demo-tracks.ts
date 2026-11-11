import type { Layout, TrackPiece, Transform } from "@/core/types";
import {
  computeWorldConnectors,
  getLocalConnectors,
  catalogRegistry,
  type Catalog,
  type CatalogEntry,
} from "@/catalog/catalog";
import { ensureDefaultLayers } from "@/core/layout";
import { fromJSON } from "@/io/serialize";
import { AddPiecesCommand } from "@/editor/commands";
import type { History } from "@/editor/commands";
import { updatePieceConnections } from "@/core/snapping";
import { deg2rad, normAngle } from "@/core/geometry";
import rawDemoLayout from "./demo-layout.railbench?raw";
import type { ModelScale } from "@/core/model-scale";

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
 * Sucht einen Katalogeintrag anhand der Artikelnummer / ID über alle Kataloge hinweg.
 */
function findCatalogEntry(id: string): CatalogEntry | undefined {
  for (const cat of catalogRegistry.getAll()) {
    const found = cat.entries.find((e) => e.id === id);
    if (found) return found;
  }
  return undefined;
}

/**
 * Erzeugt eine Kette von verbundenen Gleisen anhand von Spezifikationen.
 */
function chainPieces(
  specs: Array<{
    catalogId: string;
    powerFeed?: boolean;
    isIsolator?: boolean;
    useBranch?: boolean;
    fromPieceIndex?: number;
    fromConnectorIndex?: number;
  }>,
  startTransform: Transform,
  layerId = "l1"
): TrackPiece[] {
  const pieces: TrackPiece[] = [];

  for (let i = 0; i < specs.length; i++) {
    const spec = specs[i]!;
    const entry = findCatalogEntry(spec.catalogId);
    if (!entry) continue;

    let transform: Transform;

    if (spec.fromPieceIndex !== undefined) {
      const parentPiece = pieces[spec.fromPieceIndex];
      const connIdx = spec.fromConnectorIndex ?? (spec.useBranch ? 2 : 1);
      const parentConn = parentPiece?.connectors[connIdx];
      if (parentConn) {
        const locals = getLocalConnectors(entry);
        const lc0 = locals[0] ?? { x: 0, y: 0, angle: 180 };
        const rot = normAngle(parentConn.angle + 180 - lc0.angle);
        const rad = deg2rad(rot);
        const cos = Math.cos(rad);
        const sin = Math.sin(rad);
        const lx = lc0.x * cos - lc0.y * sin;
        const ly = lc0.x * sin + lc0.y * cos;
        transform = { x: parentConn.x - lx, y: parentConn.y - ly, rotation: rot };
      } else {
        transform = startTransform;
      }
    } else if (i === 0) {
      transform = startTransform;
    } else {
      const prev = pieces[i - 1]!;
      const prevConn = prev.connectors[1] ?? prev.connectors[0];
      if (prevConn) {
        const locals = getLocalConnectors(entry);
        const lc0 = locals[0] ?? { x: 0, y: 0, angle: 180 };
        const rot = normAngle(prevConn.angle + 180 - lc0.angle);
        const rad = deg2rad(rot);
        const cos = Math.cos(rad);
        const sin = Math.sin(rad);
        const lx = lc0.x * cos - lc0.y * sin;
        const ly = lc0.x * sin + lc0.y * cos;
        transform = { x: prevConn.x - lx, y: prevConn.y - ly, rotation: rot };
      } else {
        transform = startTransform;
      }
    }

    const connectors = computeWorldConnectors(entry, transform);
    pieces.push({
      id: `p-${i + 1}-${spec.catalogId}`,
      catalogId: spec.catalogId,
      transform,
      connectors,
      layerId,
      powerFeed: spec.powerFeed,
      isIsolator: spec.isIsolator,
      isolatedConnectorIndex: spec.isIsolator ? 0 : undefined,
    });
  }

  return pieces;
}

/**
 * 1. Alpenland H0 Demo (Original aus demo-layout.railbench)
 */
export function createAlpenlandH0Layout(): Layout {
  return fromJSON(rawDemoLayout);
}

/**
 * 2. Zweigleisige Hauptstrecke (Märklin C-Gleis H0)
 */
export function createMainlineH0Layout(): Layout {
  const layerId = "l1";
  const layers = [{ id: layerId, name: "Hauptstrecke", visible: true, locked: false, color: "#3b82f6" }];

  // Äußerer Kreis R2 (24230) + Geraden 24188
  const outerSpecs = [
    { catalogId: "24188", powerFeed: true }, // 0
    { catalogId: "24188" }, // 1
    { catalogId: "24188" }, // 2
    { catalogId: "24188" }, // 3
    // Bogen rechts R2 (6x 30° = 180°)
    { catalogId: "24230" }, // 4
    { catalogId: "24230" }, // 5
    { catalogId: "24230" }, // 6
    { catalogId: "24230" }, // 7
    { catalogId: "24230" }, // 8
    { catalogId: "24230" }, // 9
    // Untere Gerade mit Weichen
    { catalogId: "24611" }, // 10: Weiche links
    { catalogId: "24188" }, // 11
    { catalogId: "24188" }, // 12
    { catalogId: "24612" }, // 13: Weiche rechts
    // Bogen links R2 (6x 30° = 180°)
    { catalogId: "24230" }, // 14
    { catalogId: "24230" }, // 15
    { catalogId: "24230" }, // 16
    { catalogId: "24230" }, // 17
    { catalogId: "24230" }, // 18
    { catalogId: "24230" }, // 19
    // Abstellgleis vom Weichenabzweig (Weiche 13)
    { catalogId: "24188", fromPieceIndex: 13, useBranch: true }, // 20
    { catalogId: "24977" }, // 21 Prellbock
  ];

  const pieces = chainPieces(outerSpecs, { x: 500, y: 250, rotation: 0 }, layerId);

  const layout: Layout = {
    name: "Zweigleisige Hauptstrecke",
    scale: "H0",
    feedVoltage: 16,
    locoCurrentMa: 450,
    locoCount: 2,
    layers,
    activeLayerId: layerId,
    baseboards: [
      {
        id: "bb-main",
        name: "Hauptplatte",
        enabled: true,
        shape: "rect",
        width: 2500,
        depth: 1300,
        style: "wood",
        positionMode: "custom",
        customX: 0,
        customY: 0,
      },
    ],
    pieces,
  };

  ensureDefaultLayers(layout);
  updatePieceConnections(layout.pieces, layout.layers);
  return layout;
}

/**
 * 3. Rangier- & Industrieanlage (Minitrix N)
 */
export function createSwitchingNLayout(): Layout {
  const layerId = "l1";
  const layers = [{ id: layerId, name: "Gleise", visible: true, locked: false, color: "#10b981" }];

  const specs = [
    { catalogId: "14904", powerFeed: true }, // 0: Gerade 104mm
    { catalogId: "14904" }, // 1
    { catalogId: "14904" }, // 2
    // Bogen rechts
    { catalogId: "14901" }, // 3 (30°)
    { catalogId: "14901" }, // 4
    { catalogId: "14901" }, // 5
    { catalogId: "14901" }, // 6
    { catalogId: "14901" }, // 7
    { catalogId: "14901" }, // 8
    // Untere Gerade mit Weichen
    { catalogId: "14938" }, // 9: Weiche links
    { catalogId: "14904" }, // 10
    { catalogId: "14939" }, // 11: Weiche rechts
    // Bogen links
    { catalogId: "14901" }, // 12
    { catalogId: "14901" }, // 13
    { catalogId: "14901" }, // 14
    { catalogId: "14901" }, // 15
    { catalogId: "14901" }, // 16
    { catalogId: "14901" }, // 17
    // Industrie-Abzweig 1
    { catalogId: "14904", fromPieceIndex: 9, useBranch: true }, // 18
    { catalogId: "14991" }, // 19 Prellbock
    // Industrie-Abzweig 2
    { catalogId: "14904", fromPieceIndex: 11, useBranch: true }, // 20
    { catalogId: "14991" }, // 21 Prellbock
  ];

  const pieces = chainPieces(specs, { x: 300, y: 150, rotation: 0 }, layerId);

  const layout: Layout = {
    name: "Rangier- & Industrieanlage (N)",
    scale: "N",
    feedVoltage: 12,
    locoCurrentMa: 150,
    locoCount: 1,
    layers,
    activeLayerId: layerId,
    baseboards: [
      {
        id: "bb-n",
        name: "Modulplatte",
        enabled: true,
        shape: "rect",
        width: 1200,
        depth: 600,
        style: "wood",
        positionMode: "custom",
        customX: 0,
        customY: 0,
      },
    ],
    pieces,
  };

  ensureDefaultLayers(layout);
  updatePieceConnections(layout.pieces, layout.layers);
  return layout;
}

/**
 * 4. Gartenbahn Rundkurs (LGB Spur G)
 */
export function createGardenGLayout(): Layout {
  const layerId = "l1";
  const layers = [{ id: layerId, name: "Gartenstrecke", visible: true, locked: false, color: "#16a34a" }];

  const specs = [
    { catalogId: "10000", powerFeed: true }, // 0 Gerade 300 mm
    { catalogId: "10000" }, // 1
    { catalogId: "10000" }, // 2
    // Bogen rechts 6x 30°
    { catalogId: "11000" }, // 3
    { catalogId: "11000" }, // 4
    { catalogId: "11000" }, // 5
    { catalogId: "11000" }, // 6
    { catalogId: "11000" }, // 7
    { catalogId: "11000" }, // 8
    // Unten mit Weiche
    { catalogId: "12000" }, // 9 Weiche links
    { catalogId: "10000" }, // 10
    { catalogId: "12100" }, // 11 Weiche rechts
    // Bogen links 6x 30°
    { catalogId: "11000" }, // 12
    { catalogId: "11000" }, // 13
    { catalogId: "11000" }, // 14
    { catalogId: "11000" }, // 15
    { catalogId: "11000" }, // 16
    { catalogId: "11000" }, // 17
    // Überholgleis
    { catalogId: "10000", fromPieceIndex: 9, useBranch: true }, // 18
    { catalogId: "10000" }, // 19
  ];

  const pieces = chainPieces(specs, { x: 800, y: 400, rotation: 0 }, layerId);

  const layout: Layout = {
    name: "Gartenbahn Rundkurs (G)",
    scale: "G",
    feedVoltage: 22,
    locoCurrentMa: 1500,
    locoCount: 2,
    layers,
    activeLayerId: layerId,
    baseboards: [
      {
        id: "bb-g",
        name: "Gartenbeet",
        enabled: true,
        shape: "rect",
        width: 3200,
        depth: 2000,
        style: "grass",
        positionMode: "custom",
        customX: 0,
        customY: 0,
      },
    ],
    pieces,
  };

  ensureDefaultLayers(layout);
  updatePieceConnections(layout.pieces, layout.layers);
  return layout;
}

/**
 * 5. Rangieranlage Spur 0 (Lenz Spur 0)
 */
export function createShunting0Layout(): Layout {
  const layerId = "l1";
  const layers = [{ id: layerId, name: "Rangiergleise", visible: true, locked: false, color: "#d97706" }];

  const specs = [
    { catalogId: "45010", powerFeed: true }, // 0 Ausziehgleis
    { catalogId: "45010" }, // 1
    { catalogId: "45030" }, // 2 Weiche links
    { catalogId: "45010" }, // 3
    { catalogId: "45031" }, // 4 Weiche rechts
    { catalogId: "45010" }, // 5
    { catalogId: "45012" }, // 6 Prellbock Stammgleis
    // Nebengleis 1
    { catalogId: "45010", fromPieceIndex: 2, useBranch: true }, // 7
    { catalogId: "45010" }, // 8
    { catalogId: "45012" }, // 9 Prellbock Nebengleis 1
    // Nebengleis 2
    { catalogId: "45010", fromPieceIndex: 4, useBranch: true }, // 10
    { catalogId: "45010" }, // 11
    { catalogId: "45012" }, // 12 Prellbock Nebengleis 2
  ];

  const pieces = chainPieces(specs, { x: 200, y: 300, rotation: 0 }, layerId);

  const layout: Layout = {
    name: "Rangieranlage Spur 0 (Inglenook)",
    scale: "0",
    feedVoltage: 18,
    locoCurrentMa: 600,
    locoCount: 1,
    layers,
    activeLayerId: layerId,
    baseboards: [
      {
        id: "bb-0",
        name: "Rangierregal",
        enabled: true,
        shape: "rect",
        width: 2200,
        depth: 650,
        style: "wood",
        positionMode: "custom",
        customX: 0,
        customY: 0,
      },
    ],
    pieces,
  };

  ensureDefaultLayers(layout);
  updatePieceConnections(layout.pieces, layout.layers);
  return layout;
}

/**
 * 6. Koffer-Anlage Spur Z (Märklin Spur Z)
 */
export function createSuitcaseZLayout(): Layout {
  const layerId = "l1";
  const layers = [{ id: layerId, name: "Kofferstrecke", visible: true, locked: false, color: "#8b5cf6" }];

  const specs = [
    { catalogId: "8500", powerFeed: true }, // 0 Gerade 110 mm
    { catalogId: "8500" }, // 1
    // Bogen rechts 6x 30°
    { catalogId: "8520" }, // 2
    { catalogId: "8520" }, // 3
    { catalogId: "8520" }, // 4
    { catalogId: "8520" }, // 5
    { catalogId: "8520" }, // 6
    { catalogId: "8520" }, // 7
    // Untere Gerade mit Weiche
    { catalogId: "8562" }, // 8 Weiche links
    { catalogId: "8588", isIsolator: true }, // 9 Trenn- / Unterbrechergleis
    { catalogId: "8563" }, // 10 Weiche rechts
    // Bogen links 6x 30°
    { catalogId: "8520" }, // 11
    { catalogId: "8520" }, // 12
    { catalogId: "8520" }, // 13
    { catalogId: "8520" }, // 14
    { catalogId: "8520" }, // 15
    { catalogId: "8520" }, // 16
    // Abstellgleis
    { catalogId: "8500", fromPieceIndex: 8, useBranch: true }, // 17
    { catalogId: "8598" }, // 18 Prellbock
  ];

  const pieces = chainPieces(specs, { x: 180, y: 120, rotation: 0 }, layerId);

  const layout: Layout = {
    name: "Koffer-Anlage Spur Z",
    scale: "Z",
    feedVoltage: 10,
    locoCurrentMa: 80,
    locoCount: 1,
    layers,
    activeLayerId: layerId,
    baseboards: [
      {
        id: "bb-z",
        name: "Alukoffer",
        enabled: true,
        shape: "rect",
        width: 650,
        depth: 450,
        style: "slate",
        positionMode: "custom",
        customX: 0,
        customY: 0,
      },
    ],
    pieces,
  };

  ensureDefaultLayers(layout);
  updatePieceConnections(layout.pieces, layout.layers);
  return layout;
}

/**
 * Katalog aller verfügbaren Demo-Layouts.
 */
export const DEMO_LAYOUTS: DemoLayoutDef[] = [
  {
    id: "alpenland-h0",
    name: "Alpenland H0 Demo",
    nameEn: "Alpine H0 Demo",
    scale: "H0",
    systemName: "Märklin C-Gleis (H0)",
    catalogId: "maerklin-c-h0",
    description: "Klassischer Alpen-Rundkurs mit Ausweichgleis, Prellbock, Einspeisung, Bergbahnhof und Stellwerk.",
    descriptionEn: "Classic alpine circuit with passing siding, buffer stop, power feeder, mountain station, and signal box.",
    dimensionsMm: { width: 2200, height: 1100 },
    pieceCount: 18,
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
    description: "Große 2-gleisige Hauptstrecke mit Überholgleis, Weichenpaaren (Hosenträger) und Abstellgleis mit Prellbock.",
    descriptionEn: "Spacious double-track mainline circuit with passing siding, crossover turnouts, and terminal spur.",
    dimensionsMm: { width: 2500, height: 1300 },
    pieceCount: 22,
    icon: "🚂",
    tags: ["H0", "Märklin", "2-Gleisig", "Hauptstrecke", "Weichen"],
    createLayout: createMainlineH0Layout,
  },
  {
    id: "switching-n",
    name: "Rangier- & Industrieanlage",
    nameEn: "Industrial Shunting Yard",
    scale: "N",
    systemName: "Minitrix (N)",
    catalogId: "minitrix-n",
    description: "Kompakte Spur-N Anlage mit Umfahrgleis, 2 Industrie-Anschlüssen und Prellböcken für abwechslungsreiches Rangieren.",
    descriptionEn: "Compact N-scale layout with runaround track, 2 industrial spurs, and buffer stops for shunting fun.",
    dimensionsMm: { width: 1200, height: 600 },
    pieceCount: 22,
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
    description: "Großzügiger Gartenbahn-Rundkurs mit R1-Bögen, Ausweichgleis, Einspeisegleis und robusten LGB-Weichen.",
    descriptionEn: "Generous garden railway loop with R1 curves, passing siding, power feed, and sturdy LGB turnouts.",
    dimensionsMm: { width: 3200, height: 2000 },
    pieceCount: 20,
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
    description: "Klassisches Inglenook-Rangierrätsel in Spur 0: Ziehgleis mit Weichen zu 3 Nebengleisen mit Prellböcken.",
    descriptionEn: "Classic Inglenook shunting puzzle in Scale 0: Lead track with turnouts into 3 sidings with buffer stops.",
    dimensionsMm: { width: 2200, height: 650 },
    pieceCount: 13,
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
    description: "Feine Miniatur-Modellbahn im Alukoffer-Format mit Kreis, Ausweichstelle, Trennstrecke und Abstellgleis.",
    descriptionEn: "Delicate miniature suitcase layout with loop, passing siding, isolating section, and stub track.",
    dimensionsMm: { width: 650, height: 450 },
    pieceCount: 19,
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
export function initDemoTracks(layout: Layout, catalog?: Catalog, demoId?: string): void {
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
