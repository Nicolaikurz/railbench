import type { Layout, TrackPiece, Transform } from "@/core/types";
import { computeWorldConnectors, catalogRegistry, type Catalog } from "@/catalog/catalog";
import { ensureDefaultLayers } from "@/core/layout";
import { fromJSON } from "@/io/serialize";
import { AddPiecesCommand } from "@/editor/commands";
import type { History } from "@/editor/commands";
import rawDemoLayout from "./demo-layout.railbench?raw";

export const DEMO_LAYOUT_NAME = "Alpenland H0 Demo";

/**
 * Erzeugt eine frische Kopie des gespeicherten Alpenland H0 Demo-Plans.
 * Das Layout wird direkt aus der Datei `demo-layout.railbench` (JSON-Format) geladen.
 */
export function createDemoLayout(_catalog?: Catalog): Layout {
  return fromJSON(rawDemoLayout);
}

/**
 * Initialisiert das Layout mit dem aus `demo-layout.railbench` geladenen Plan.
 */
export function initDemoTracks(layout: Layout, catalog?: Catalog): void {
  const demo = createDemoLayout(catalog);
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
