import type { Layout, TrackPiece, Connector } from "./types";
import { catalogRegistry, computeWorldConnectors, isFeederEntry, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import { dist, deg2rad, normAngle } from "./geometry";
import { isWasmReady, solvePowerDijkstraKernel, solveShortCircuitDsuKernel } from "./wasm-bridge";

export type PowerStatus = "optimal" | "warning" | "critical" | "disconnected";

export interface ShortCircuitPoint {
  pieceId1: string;
  connectorIndex1: number;
  pieceId2: string;
  connectorIndex2: number;
  x: number;
  y: number;
}

export interface ShortCircuitDetectionResult {
  hasShortCircuit: boolean;
  isTwoWireSystem: boolean;
  shortCircuitPieces: Set<string>;
  shortCircuitPoints: ShortCircuitPoint[];
}

export const MIN_LOCO_CURRENT_MA = 50;
export const MAX_LOCO_CURRENT_MA = 1000; // 1A Maximal pro Lok

export interface ScaleCurrentPreset {
  scale: string;
  label: string;
  minMa: number;
  maxMa: number;
  defaultMa: number;
  description: string;
}

export const SCALE_CURRENT_PRESETS: Record<string, ScaleCurrentPreset> = {
  Z: { scale: "Z", label: "Spur Z", minMa: 50, maxMa: 150, defaultMa: 100, description: "50–150 mA pro Lok" },
  N: { scale: "N", label: "Spur N", minMa: 50, maxMa: 150, defaultMa: 100, description: "50–150 mA pro Lok" },
  TT: { scale: "TT", label: "Spur TT", minMa: 200, maxMa: 400, defaultMa: 300, description: "200–400 mA pro Lok" },
  H0: { scale: "H0", label: "Spur H0", minMa: 300, maxMa: 600, defaultMa: 450, description: "300–600 mA pro Lok" },
  "0": { scale: "0", label: "Spur 0", minMa: 400, maxMa: 800, defaultMa: 600, description: "400–800 mA pro Lok" },
  "1": { scale: "1", label: "Spur 1", minMa: 600, maxMa: 1000, defaultMa: 800, description: "600–1000 mA pro Lok" },
  G: { scale: "G", label: "Spur G", minMa: 600, maxMa: 1000, defaultMa: 1000, description: "Max. 1000 mA (1A) pro Lok" },
};

export const SCALE_VOLTAGE_PRESETS: Record<string, number> = {
  Z: 10.0,
  N: 12.0,
  TT: 14.0,
  H0: 16.0,
  "0": 16.0,
  "1": 18.0,
  G: 24.0,
};

export function getDefaultFeedVoltage(scale?: string): number {
  if (!scale) return 16.0;
  return SCALE_VOLTAGE_PRESETS[scale.toUpperCase()] ?? 16.0;
}

export function getDefaultLocoCurrentMa(scale?: string): number {
  if (!scale) return 450;
  return SCALE_CURRENT_PRESETS[scale]?.defaultMa ?? 450;
}

/**
 * Erkennt, ob ein Katalog-Eintrag eine Weiche / Kreuzungsweiche ist.
 */
export function isTurnoutEntry(entry?: CatalogEntry): boolean {
  if (!entry) return false;
  if (entry.isRoad) return false;
  if (
    entry.kind === "turnout-left" ||
    entry.kind === "turnout-right" ||
    entry.kind === "curved-turnout-left" ||
    entry.kind === "curved-turnout-right" ||
    entry.kind === "three-way" ||
    entry.kind === "double-slip"
  ) {
    return true;
  }
  const cat = entry.category?.toLowerCase() || "";
  return cat.includes("weich") || cat.includes("turnout");
}

export interface PiecePowerResult {
  pieceId: string;
  voltage: number;
  voltageDrop: number;
  voltageDropPercent: number;
  status: PowerStatus;
  resistanceOhm: number;
  distanceFromFeederMeters: number;
  isFeeder: boolean;
  isShortCircuit?: boolean;
  // Weichen-Zubehör (Turnout accessories):
  isTurnout?: boolean;
  hasLantern?: boolean;
  hasDecoder?: boolean;
  accessoryCurrentMa?: number;
  accessoryDrop?: number;
  // Oberleitungs-Stromkreis (Catenary circuit):
  hasCatenary?: boolean;
  catenaryVoltage?: number;
  catenaryDrop?: number;
  catenaryDropPercent?: number;
  catenaryStatus?: PowerStatus;
  catenaryResistanceOhm?: number;
  isCatenaryFeeder?: boolean;
}

export interface SimulationSummary {
  enabled: boolean;
  feedVoltage: number;
  catenaryFeedVoltage?: number;
  catenaryCircuitMode?: "separate" | "parallel";
  locoCurrentMa: number;
  locoCount: number;
  trainCurrentAmps: number;
  // Weichen-Zubehör & Gesamtstrom:
  turnoutCount: number;
  turnoutLanternCount: number;
  turnoutDecoderCount: number;
  turnoutAccessoryCurrentAmps: number;
  totalCurrentAmps: number;
  turnoutLanternCurrentMa: number;
  turnoutDecoderCurrentMa: number;
  feederCount: number;
  isolatorCount: number;
  minVoltage: number;
  maxDropPercent: number;
  optimalCount: number;
  warningCount: number;
  criticalCount: number;
  disconnectedCount: number;
  results: Map<string, PiecePowerResult>;
  recommendation: string;

  // 2-Leiter Kurzschluss-Erkennung (Short Circuit Detection):
  hasShortCircuit?: boolean;
  shortCircuitPieces?: string[];
  shortCircuitPoints?: ShortCircuitPoint[];

  // Oberleitungs-Statistiken:
  catenaryPieceCount?: number;
  catenaryFeederCount?: number;
  catenaryIsolatorCount?: number;
  catenaryMinVoltage?: number;
  catenaryMaxDropPercent?: number;
  catenaryOptimalCount?: number;
  catenaryWarningCount?: number;
  catenaryCriticalCount?: number;
  catenaryDisconnectedCount?: number;
  catenaryRecommendation?: string;
}

/** Ermittelt die ungefähre Gleislänge eines Stücks in Metern. */
function getPieceLengthMeters(entry?: CatalogEntry): number {
  if (!entry) return 0.188;
  if (entry.length) return entry.length / 1000;
  if (entry.radius && entry.angle) {
    const r = entry.radius / 1000;
    const rad = deg2rad(entry.angle);
    return r * rad;
  }
  return 0.188;
}

/** Prüft, ob ein Katalog ein 2-Leiter-System ist (z. B. Trix C, Piko A, Roco Line, Fleischmann Piccolo, Tillig TT) */
export function isTwoWireCatalog(catalog: Catalog): boolean {
  if (catalog.isRoadSystem) return false;
  if (catalog.conductors === 3) return false;
  if (catalog.conductors === 2) return true;
  return true;
}

class DisjointSet {
  private parent = new Map<string, string>();
  private rank = new Map<string, number>();

  find(x: string): string {
    let p = this.parent.get(x);
    if (!p) {
      this.parent.set(x, x);
      this.rank.set(x, 0);
      return x;
    }
    if (p !== x) {
      p = this.find(p);
      this.parent.set(x, p);
    }
    return p;
  }

  union(x: string, y: string): boolean {
    const rx = this.find(x);
    const ry = this.find(y);
    if (rx === ry) return false;
    const rankX = this.rank.get(rx) ?? 0;
    const rankY = this.rank.get(ry) ?? 0;
    if (rankX < rankY) {
      this.parent.set(rx, ry);
    } else if (rankX > rankY) {
      this.parent.set(ry, rx);
    } else {
      this.parent.set(ry, rx);
      this.rank.set(rx, rankX + 1);
    }
    return true;
  }
}

/** Liefert die Schienen-ID für einen bestimmten Anschluss (cIdx) und Seite (L = links, R = rechts) in Blickrichtung des Anschlusses */
function getRailTerminal(piece: TrackPiece, entry: CatalogEntry | undefined, cIdx: number, side: "L" | "R"): string {
  if (entry?.kind === "crossing") {
    if (cIdx < 2) {
      const isRail1 = cIdx === 0 ? side === "L" : side === "R";
      return isRail1 ? `${piece.id}:1` : `${piece.id}:2`;
    } else {
      const isRail3 = cIdx === 2 ? side === "L" : side === "R";
      return isRail3 ? `${piece.id}:3` : `${piece.id}:4`;
    }
  }

  // Für reguläre Gleise (Gerade, Bogen, Weiche, DKW, Prellbock etc.):
  // Anschluss 0 (Stamm / Anfang): links ist Schiene 1, rechts ist Schiene 2
  // Abgänge k >= 1: rechts ist Schiene 1, links ist Schiene 2
  const isRail1 = cIdx === 0 ? side === "L" : side === "R";
  return isRail1 ? `${piece.id}:1` : `${piece.id}:2`;
}

/**
 * Erkennt Kurzschlüsse in 2-Leiter-Gleissystemen (2-wire / 2-rail systems, DCC/DC),
 * wenn linke Schiene und rechte Schiene durch Kehrschleifen, Gleisdreiecke
 * oder fehlerhafte Polung ohne isolierende Trennstellen verbunden sind.
 */
export function detectShortCircuit(layout: Layout, catalog: Catalog): ShortCircuitDetectionResult {
  const isTwoWire = isTwoWireCatalog(catalog);
  if (!isTwoWire) {
    return {
      hasShortCircuit: false,
      isTwoWireSystem: false,
      shortCircuitPieces: new Set<string>(),
      shortCircuitPoints: [],
    };
  }

  const pieces = layout.pieces || [];
  if (pieces.length === 0) {
    return {
      hasShortCircuit: false,
      isTwoWireSystem: true,
      shortCircuitPieces: new Set<string>(),
      shortCircuitPoints: [],
    };
  }

  const entryMap = new Map<string, CatalogEntry>();
  for (const e of catalog.entries) {
    entryMap.set(e.id, e);
  }

  for (const p of pieces) {
    const entry = entryMap.get(p.catalogId) ?? catalogRegistry.findEntry(p.catalogId);
    if (entry && (!p.connectors || p.connectors.length === 0)) {
      p.connectors = computeWorldConnectors(entry, p.transform);
    }
  }

  interface ConnectionPair {
    p1: TrackPiece;
    cIdx1: number;
    c1: Connector;
    p2: TrackPiece;
    cIdx2: number;
    c2: Connector;
  }

  const connections: ConnectionPair[] = [];
  const visitedJoints = new Set<string>();

  for (let i = 0; i < pieces.length; i++) {
    const p1 = pieces[i]!;
    for (let cIdx1 = 0; cIdx1 < p1.connectors.length; cIdx1++) {
      const c1 = p1.connectors[cIdx1]!;

      if (c1.connectedTo) {
        const targetPieceId = c1.connectedTo.pieceId;
        const targetConnectorIndex = c1.connectedTo.connectorIndex;
        const p2 = pieces.find((p) => p.id === targetPieceId);
        if (p2 && targetConnectorIndex < p2.connectors.length) {
          const c2 = p2.connectors[targetConnectorIndex]!;
          const key =
            p1.id < p2.id
              ? `${p1.id}:${cIdx1}_${p2.id}:${targetConnectorIndex}`
              : `${p2.id}:${targetConnectorIndex}_${p1.id}:${cIdx1}`;
          if (!visitedJoints.has(key)) {
            visitedJoints.add(key);
            const c1Iso = Boolean(p1.isIsolator && (p1.isolatedConnectorIndex ?? 0) === cIdx1);
            const c2Iso = Boolean(p2.isIsolator && (p2.isolatedConnectorIndex ?? 0) === targetConnectorIndex);
            if (!c1Iso && !c2Iso) {
              connections.push({ p1, cIdx1, c1, p2, cIdx2: targetConnectorIndex, c2 });
            }
          }
        }
      } else {
        // Geometrische Nachbarschaftsprüfung
        for (let j = i + 1; j < pieces.length; j++) {
          const p2 = pieces[j]!;
          for (let cIdx2 = 0; cIdx2 < p2.connectors.length; cIdx2++) {
            const c2 = p2.connectors[cIdx2]!;
            const d = dist(c1, c2);
            const diffAngle = Math.abs(normAngle(c1.angle - c2.angle) - 180);
            if (d <= 12 && diffAngle <= 35) {
              const key =
                p1.id < p2.id
                  ? `${p1.id}:${cIdx1}_${p2.id}:${cIdx2}`
                  : `${p2.id}:${cIdx2}_${p1.id}:${cIdx1}`;
              if (!visitedJoints.has(key)) {
                visitedJoints.add(key);
                const c1Iso = Boolean(p1.isIsolator && (p1.isolatedConnectorIndex ?? 0) === cIdx1);
                const c2Iso = Boolean(p2.isIsolator && (p2.isolatedConnectorIndex ?? 0) === cIdx2);
                if (!c1Iso && !c2Iso) {
                  connections.push({ p1, cIdx1, c1, p2, cIdx2, c2 });
                }
              }
            }
          }
        }
      }
    }
  }

  const shortCircuitPoints: ShortCircuitPoint[] = [];
  const shortCircuitPieces = new Set<string>();

  if (isWasmReady() && connections.length > 0) {
    const termMap = new Map<string, number>();
    const getTermIdx = (key: string): number => {
      let id = termMap.get(key);
      if (id === undefined) {
        id = termMap.size;
        termMap.set(key, id);
      }
      return id;
    };

    const edgeArr = new Uint32Array(connections.length * 4);
    for (let i = 0; i < connections.length; i++) {
      const conn = connections[i]!;
      const entry1 = entryMap.get(conn.p1.catalogId) ?? catalogRegistry.findEntry(conn.p1.catalogId);
      const entry2 = entryMap.get(conn.p2.catalogId) ?? catalogRegistry.findEntry(conn.p2.catalogId);
      edgeArr[i * 4] = getTermIdx(getRailTerminal(conn.p1, entry1, conn.cIdx1, "L"));
      edgeArr[i * 4 + 1] = getTermIdx(getRailTerminal(conn.p1, entry1, conn.cIdx1, "R"));
      edgeArr[i * 4 + 2] = getTermIdx(getRailTerminal(conn.p2, entry2, conn.cIdx2, "L"));
      edgeArr[i * 4 + 3] = getTermIdx(getRailTerminal(conn.p2, entry2, conn.cIdx2, "R"));
    }

    const pairPieceIds: string[] = [];
    const pairIndices: number[] = [];
    for (const p of pieces) {
      pairPieceIds.push(p.id);
      pairIndices.push(getTermIdx(`${p.id}:1`), getTermIdx(`${p.id}:2`));
      const entry = entryMap.get(p.catalogId) ?? catalogRegistry.findEntry(p.catalogId);
      if (entry?.kind === "crossing") {
        pairPieceIds.push(p.id);
        pairIndices.push(getTermIdx(`${p.id}:3`), getTermIdx(`${p.id}:4`));
      }
    }

    const res = solveShortCircuitDsuKernel(termMap.size, edgeArr, new Uint32Array(pairIndices));
    if (res) {
      for (let i = 0; i < connections.length; i++) {
        if (res[i]) {
          const conn = connections[i]!;
          shortCircuitPoints.push({
            pieceId1: conn.p1.id,
            connectorIndex1: conn.cIdx1,
            pieceId2: conn.p2.id,
            connectorIndex2: conn.cIdx2,
            x: (conn.c1.x + conn.c2.x) / 2,
            y: (conn.c1.y + conn.c2.y) / 2,
          });
        }
      }
      for (let i = 0; i < pairPieceIds.length; i++) {
        if (res[connections.length + i]) {
          shortCircuitPieces.add(pairPieceIds[i]!);
        }
      }
    }
  } else {
    const dsu = new DisjointSet();

    for (const conn of connections) {
      const entry1 = entryMap.get(conn.p1.catalogId) ?? catalogRegistry.findEntry(conn.p1.catalogId);
      const entry2 = entryMap.get(conn.p2.catalogId) ?? catalogRegistry.findEntry(conn.p2.catalogId);

      const node1L = getRailTerminal(conn.p1, entry1, conn.cIdx1, "L");
      const node1R = getRailTerminal(conn.p1, entry1, conn.cIdx1, "R");
      const node2L = getRailTerminal(conn.p2, entry2, conn.cIdx2, "L");
      const node2R = getRailTerminal(conn.p2, entry2, conn.cIdx2, "R");

      const willShortCircuit =
        dsu.find(node1L) === dsu.find(node2L) ||
        dsu.find(node1R) === dsu.find(node2R) ||
        dsu.find(node1L) === dsu.find(node1R) ||
        dsu.find(node2L) === dsu.find(node2R);

      if (willShortCircuit && dsu.find(node1L) !== dsu.find(node2R)) {
        shortCircuitPoints.push({
          pieceId1: conn.p1.id,
          connectorIndex1: conn.cIdx1,
          pieceId2: conn.p2.id,
          connectorIndex2: conn.cIdx2,
          x: (conn.c1.x + conn.c2.x) / 2,
          y: (conn.c1.y + conn.c2.y) / 2,
        });
      }

      dsu.union(node1L, node2R);
      dsu.union(node1R, node2L);
    }

    for (const p of pieces) {
      const r1 = `${p.id}:1`;
      const r2 = `${p.id}:2`;
      if (dsu.find(r1) === dsu.find(r2)) {
        shortCircuitPieces.add(p.id);
      }
      const entry = entryMap.get(p.catalogId) ?? catalogRegistry.findEntry(p.catalogId);
      if (entry?.kind === "crossing") {
        const r3 = `${p.id}:3`;
        const r4 = `${p.id}:4`;
        if (dsu.find(r3) === dsu.find(r4)) {
          shortCircuitPieces.add(p.id);
        }
      }
    }
  }

  // Falls Pieces kurzgeschlossen sind, aber noch kein ShortCircuitPoint erfasst wurde
  if (shortCircuitPieces.size > 0 && shortCircuitPoints.length === 0) {
    for (const conn of connections) {
      if (shortCircuitPieces.has(conn.p1.id) && shortCircuitPieces.has(conn.p2.id)) {
        const isReverseContact =
          (conn.cIdx1 === 0 && conn.cIdx2 === 0) || (conn.cIdx1 >= 1 && conn.cIdx2 >= 1);
        if (isReverseContact) {
          shortCircuitPoints.push({
            pieceId1: conn.p1.id,
            connectorIndex1: conn.cIdx1,
            pieceId2: conn.p2.id,
            connectorIndex2: conn.cIdx2,
            x: (conn.c1.x + conn.c2.x) / 2,
            y: (conn.c1.y + conn.c2.y) / 2,
          });
          break;
        }
      }
    }
    if (shortCircuitPoints.length === 0 && connections.length > 0) {
      const c = connections[0]!;
      shortCircuitPoints.push({
        pieceId1: c.p1.id,
        connectorIndex1: c.cIdx1,
        pieceId2: c.p2.id,
        connectorIndex2: c.cIdx2,
        x: (c.c1.x + c.c2.x) / 2,
        y: (c.c1.y + c.c2.y) / 2,
      });
    }
  }

  return {
    hasShortCircuit: shortCircuitPieces.size > 0,
    isTwoWireSystem: true,
    shortCircuitPieces,
    shortCircuitPoints,
  };
}

export interface SimulatePowerDropOptions {
  trainCurrentAmps?: number;
  feedVoltage?: number;
  catenaryFeedVoltage?: number;
  catenaryCircuitMode?: "separate" | "parallel";
  locoCurrentMa?: number;
  locoCount?: number;
  turnoutLanternCurrentMa?: number;
  turnoutDecoderCurrentMa?: number;
  fallbackFeeder?: boolean;
}

/**
 * Führt die Spannungsabfall-Simulation (Power-Drop Simulation) für das gegebene Layout durch.
 * Berechnet Leitungswiderstände, Übergangswiderstände an Steckverbindungen und Spannungsabfall.
 */
export function simulatePowerDrop(
  layout: Layout,
  catalog: Catalog,
  options: SimulatePowerDropOptions = {},
): SimulationSummary {
  const feedVoltage = options.feedVoltage ?? layout.feedVoltage ?? getDefaultFeedVoltage(layout.scale);

  // Stromaufnahme pro Lok (mA):
  let locoCurrentMa = options.locoCurrentMa ?? layout.locoCurrentMa;
  if (locoCurrentMa === undefined) {
    locoCurrentMa = getDefaultLocoCurrentMa(layout.scale);
  }
  // Clamp locoCurrentMa: Maximal 1A (1000 mA), minimal 50 mA
  locoCurrentMa = Math.min(MAX_LOCO_CURRENT_MA, Math.max(MIN_LOCO_CURRENT_MA, Math.round(locoCurrentMa)));

  // Anzahl Loks / Züge gleichzeitig im Fahrbetrieb:
  let locoCount = options.locoCount ?? layout.locoCount ?? 1;
  locoCount = Math.max(1, Math.min(20, Math.round(locoCount)));

  // Gesamtstrom in Ampere berechnen (Traktion / Züge)
  let current: number;
  if (options.trainCurrentAmps !== undefined) {
    current = options.trainCurrentAmps;
    locoCurrentMa = Math.min(MAX_LOCO_CURRENT_MA, Math.max(MIN_LOCO_CURRENT_MA, Math.round((current / locoCount) * 1000)));
  } else if (layout.trainCurrentAmps !== undefined && layout.locoCurrentMa === undefined) {
    current = layout.trainCurrentAmps;
    locoCurrentMa = Math.min(MAX_LOCO_CURRENT_MA, Math.max(MIN_LOCO_CURRENT_MA, Math.round((current / locoCount) * 1000)));
  } else {
    current = (locoCurrentMa * locoCount) / 1000;
  }

  // Weichen-Zubehör Parameter (Laterne & Digitaldecoder):
  const turnoutLanternCurrentMa = options.turnoutLanternCurrentMa ?? layout.turnoutLanternCurrentMa ?? 20;
  const turnoutDecoderCurrentMa = options.turnoutDecoderCurrentMa ?? layout.turnoutDecoderCurrentMa ?? 15;

  const railOhmPerMeter = 0.08; // Spezifischer Widerstand beider Schienen
  const jointResistanceOhm = 0.05; // Übergangswiderstand je Schienenverbinder

  const entryMap = new Map<string, CatalogEntry>();
  for (const e of catalog.entries) {
    entryMap.set(e.id, e);
  }

  const pieces = layout.pieces;
  const results = new Map<string, PiecePowerResult>();

  // Weichen erfassen und Stromaufnahme pro Stück ermitteln
  let turnoutCount = 0;
  let turnoutLanternCount = 0;
  let turnoutDecoderCount = 0;
  let totalTurnoutAccessoryMa = 0;
  const pieceAccessoryMa = new Map<string, number>();

  for (const p of pieces) {
    const entry = entryMap.get(p.catalogId);
    if (isTurnoutEntry(entry)) {
      turnoutCount++;
      if (p.hasLantern) turnoutLanternCount++;
      if (p.hasDecoder) turnoutDecoderCount++;

      let ma = 0;
      if (p.turnoutCurrentMa !== undefined && p.turnoutCurrentMa > 0) {
        ma = p.turnoutCurrentMa;
      } else {
        if (p.hasLantern) ma += turnoutLanternCurrentMa;
        if (p.hasDecoder) ma += turnoutDecoderCurrentMa;
      }
      pieceAccessoryMa.set(p.id, ma);
      totalTurnoutAccessoryMa += ma;
    }
  }

  const turnoutAccessoryCurrentAmps = totalTurnoutAccessoryMa / 1000;
  const totalCurrentAmps = current + turnoutAccessoryCurrentAmps;

  if (pieces.length === 0) {
    return {
      enabled: true,
      feedVoltage,
      locoCurrentMa,
      locoCount,
      trainCurrentAmps: current,
      turnoutCount: 0,
      turnoutLanternCount: 0,
      turnoutDecoderCount: 0,
      turnoutAccessoryCurrentAmps: 0,
      totalCurrentAmps: current,
      turnoutLanternCurrentMa,
      turnoutDecoderCurrentMa,
      feederCount: 0,
      isolatorCount: 0,
      minVoltage: feedVoltage,
      maxDropPercent: 0,
      optimalCount: 0,
      warningCount: 0,
      criticalCount: 0,
      disconnectedCount: 0,
      hasShortCircuit: false,
      shortCircuitPieces: [],
      shortCircuitPoints: [],
      results,
      recommendation: "Keine Gleise im Plan vorhanden.",
    };
  }

  // 1. Einspeisepunkte ermitteln (nur explizit als Feeder konfigurierte Gleise / Anschlussgleise)
  const explicitFeeders = pieces.filter((p) => p.powerFeed || isFeederEntry(entryMap.get(p.catalogId)));
  const feeders: TrackPiece[] = explicitFeeders;

  // Welt-Connectors für alle Gleise aktualisieren/sicherstellen
  for (const p of pieces) {
    const entry = entryMap.get(p.catalogId);
    if (entry) {
      const prev = p.connectors;
      p.connectors = computeWorldConnectors(entry, p.transform);
      if (prev) {
        for (let i = 0; i < p.connectors.length && i < prev.length; i++) {
          if (prev[i]?.connectedTo) {
            p.connectors[i]!.connectedTo = prev[i]!.connectedTo;
          }
        }
      }
    }
  }

  // 2. Verbindungsgraph aufbauen
  // Adjazenzliste: Map<pieceId, Set<targetPieceId>>
  const adj = new Map<string, Set<string>>();
  for (const p of pieces) {
    adj.set(p.id, new Set<string>());
  }

  // Verbindungen über connectors (entweder explizit connectedTo oder nahestehende Anschlüsse)
  for (let i = 0; i < pieces.length; i++) {
    const p1 = pieces[i]!;
    for (let cIdx1 = 0; cIdx1 < p1.connectors.length; cIdx1++) {
      const c1 = p1.connectors[cIdx1]!;
      if (c1.connectedTo) {
        const targetPieceId = c1.connectedTo.pieceId;
        const targetConnectorIndex = c1.connectedTo.connectorIndex;
        const p2 = pieces.find((p) => p.id === targetPieceId);

        // Trennstelle isoliert nur an genau einem Anschluss (isolatedConnectorIndex, Standard 0):
        const c1Isolated = Boolean(p1.isIsolator && (p1.isolatedConnectorIndex ?? 0) === cIdx1);
        const c2Isolated = Boolean(p2?.isIsolator && (p2.isolatedConnectorIndex ?? 0) === targetConnectorIndex);

        if (!c1Isolated && !c2Isolated) {
          adj.get(p1.id)?.add(targetPieceId);
          adj.get(targetPieceId)?.add(p1.id);
        }
      } else {
        // Geometrische Nachbarschaftsprüfung
        for (let j = i + 1; j < pieces.length; j++) {
          const p2 = pieces[j]!;
          for (let cIdx2 = 0; cIdx2 < p2.connectors.length; cIdx2++) {
            const c2 = p2.connectors[cIdx2]!;
            const d = dist(c1, c2);
            const diffAngle = Math.abs(normAngle(c1.angle - c2.angle) - 180);
            if (d <= 12 && diffAngle <= 35) {
              const c1Isolated = Boolean(p1.isIsolator && (p1.isolatedConnectorIndex ?? 0) === cIdx1);
              const c2Isolated = Boolean(p2.isIsolator && (p2.isolatedConnectorIndex ?? 0) === cIdx2);
              if (!c1Isolated && !c2Isolated) {
                adj.get(p1.id)?.add(p2.id);
                adj.get(p2.id)?.add(p1.id);
              }
            }
          }
        }
      }
    }
  }

  // 3. Multi-Source Dijkstra (Rust-WASM-Kernel wenn gebaut, sonst binärer Min-Heap in JS: O((V+E) log V))
  const nPieces = pieces.length;
  const pieceIdToIdx = new Map<string, number>();
  const pieceLenM = new Float64Array(nPieces);
  for (let i = 0; i < nPieces; i++) {
    const p = pieces[i]!;
    pieceIdToIdx.set(p.id, i);
    pieceLenM[i] = getPieceLengthMeters(entryMap.get(p.catalogId));
  }

  const adjOffsets = new Uint32Array(nPieces + 1);
  let totalEdges = 0;
  for (let i = 0; i < nPieces; i++) {
    const nbrs = adj.get(pieces[i]!.id);
    adjOffsets[i] = totalEdges;
    if (nbrs) totalEdges += nbrs.size;
  }
  adjOffsets[nPieces] = totalEdges;

  const adjNeighbors = new Uint32Array(totalEdges);
  let edgeCursor = 0;
  for (let i = 0; i < nPieces; i++) {
    const nbrs = adj.get(pieces[i]!.id);
    if (nbrs) {
      for (const nbrId of nbrs) {
        const nbrIdx = pieceIdToIdx.get(nbrId);
        if (nbrIdx !== undefined) {
          adjNeighbors[edgeCursor++] = nbrIdx;
        }
      }
    }
    adjOffsets[i + 1] = edgeCursor;
  }

  const feederIndicesArr: number[] = [];
  for (const feeder of feeders) {
    const idx = pieceIdToIdx.get(feeder.id);
    if (idx !== undefined) feederIndicesArr.push(idx);
  }

  const solved = solvePowerDijkstraKernel(
    nPieces,
    adjOffsets,
    adjNeighbors.subarray(0, edgeCursor),
    pieceLenM,
    new Uint32Array(feederIndicesArr),
    railOhmPerMeter,
    jointResistanceOhm,
  );

  const minResistance = new Map<string, number>();
  const minDistance = new Map<string, number>();
  const parent = new Map<string, string | null>();
  const edgeResToParent = new Map<string, number>();

  for (let i = 0; i < nPieces; i++) {
    const pid = pieces[i]!.id;
    minResistance.set(pid, solved.minResistance[i]!);
    minDistance.set(pid, solved.minDistance[i]!);
    const parIdx = solved.parent[i]!;
    parent.set(pid, parIdx >= 0 ? pieces[parIdx]!.id : null);
    if (solved.minResistance[i]! < Infinity) {
      edgeResToParent.set(pid, solved.edgeResToParent[i]!);
    }
  }

  // Subtree-Zubehörstrom aggregieren (von den Blättern zur Einspeisung)
  const connectedPieces = pieces.filter((p) => (minResistance.get(p.id) ?? Infinity) < Infinity);
  const sortedDesc = [...connectedPieces].sort(
    (a, b) => (minResistance.get(b.id) ?? 0) - (minResistance.get(a.id) ?? 0),
  );

  const subtreeLoadAmps = new Map<string, number>();
  for (const p of sortedDesc) {
    const selfAmps = (pieceAccessoryMa.get(p.id) ?? 0) / 1000;
    const currentSub = (subtreeLoadAmps.get(p.id) ?? 0) + selfAmps;
    subtreeLoadAmps.set(p.id, currentSub);

    const parId = parent.get(p.id);
    if (parId) {
      subtreeLoadAmps.set(parId, (subtreeLoadAmps.get(parId) ?? 0) + currentSub);
    }
  }

  // Spannungsabfall durch Weichen-Zubehör entlang des Pfads von der Einspeisung berechnen
  const sortedAsc = [...sortedDesc].reverse();
  const accessoryDropMap = new Map<string, number>();
  for (const p of sortedAsc) {
    const parId = parent.get(p.id);
    const subLoad = subtreeLoadAmps.get(p.id) ?? 0;
    const edgeRes = edgeResToParent.get(p.id) ?? 0;

    if (!parId) {
      accessoryDropMap.set(p.id, edgeRes * subLoad);
    } else {
      const parDrop = accessoryDropMap.get(parId) ?? 0;
      accessoryDropMap.set(p.id, parDrop + edgeRes * subLoad);
    }
  }

  // --- 4. OBERLEITUNGS-STROMKREIS SIMULATION (CATENARY CIRCUIT) ---
  const catenaryCircuitMode = options.catenaryCircuitMode ?? layout.catenaryCircuitMode ?? "separate";
  const catenaryFeedVoltage = catenaryCircuitMode === "parallel"
    ? feedVoltage
    : (options.catenaryFeedVoltage ?? layout.catenaryFeedVoltage ?? feedVoltage);

  const catenaryPieces = pieces.filter((p) => !!p.hasCatenary);
  const catenaryPieceCount = catenaryPieces.length;

  const minCatenaryRes = new Map<string, number>();
  const catenaryFeeders: TrackPiece[] = [];
  const catenaryOhmPerMeter = 0.035; // Hochleitfähiger Kupfer-Fahrdraht + Tragseil
  const catenaryJointResOhm = 0.02; // Fahrleitungs-Steckverbinder

  if (catenaryPieceCount > 0) {
    const explicitCatFeeders = catenaryPieces.filter((p) => !!p.catenaryPowerFeed);
    catenaryFeeders.push(...explicitCatFeeders);

    // Catenary Adjazenzgraph aufbauen
    const catAdj = new Map<string, Set<string>>();
    for (const p of catenaryPieces) {
      catAdj.set(p.id, new Set<string>());
    }

    for (let i = 0; i < catenaryPieces.length; i++) {
      const p1 = catenaryPieces[i]!;
      for (let cIdx1 = 0; cIdx1 < p1.connectors.length; cIdx1++) {
        const c1 = p1.connectors[cIdx1]!;
        if (c1.connectedTo) {
          const targetPieceId = c1.connectedTo.pieceId;
          const targetConnectorIndex = c1.connectedTo.connectorIndex;
          const p2 = catenaryPieces.find((p) => p.id === targetPieceId);
          if (p2) {
            const c1Isolated = Boolean(p1.catenaryIsolator && (p1.catenaryIsolatedConnectorIndex ?? 0) === cIdx1);
            const c2Isolated = Boolean(p2.catenaryIsolator && (p2.catenaryIsolatedConnectorIndex ?? 0) === targetConnectorIndex);
            if (!c1Isolated && !c2Isolated) {
              catAdj.get(p1.id)?.add(targetPieceId);
              catAdj.get(targetPieceId)?.add(p1.id);
            }
          }
        } else {
          for (let j = i + 1; j < catenaryPieces.length; j++) {
            const p2 = catenaryPieces[j]!;
            for (let cIdx2 = 0; cIdx2 < p2.connectors.length; cIdx2++) {
              const c2 = p2.connectors[cIdx2]!;
              const d = dist(c1, c2);
              const diffAngle = Math.abs(normAngle(c1.angle - c2.angle) - 180);
              if (d <= 12 && diffAngle <= 35) {
                const c1Isolated = Boolean(p1.catenaryIsolator && (p1.catenaryIsolatedConnectorIndex ?? 0) === cIdx1);
                const c2Isolated = Boolean(p2.catenaryIsolator && (p2.catenaryIsolatedConnectorIndex ?? 0) === cIdx2);
                if (!c1Isolated && !c2Isolated) {
                  catAdj.get(p1.id)?.add(p2.id);
                  catAdj.get(p2.id)?.add(p1.id);
                }
              }
            }
          }
        }
      }
    }

    // Dijkstra für Oberleitung (Rust-WASM-Kernel wenn geladen, sonst binärer Min-Heap in JS)
    const nCat = catenaryPieces.length;
    const catIdToIdx = new Map<string, number>();
    const catLenM = new Float64Array(nCat);
    for (let i = 0; i < nCat; i++) {
      const cp = catenaryPieces[i]!;
      catIdToIdx.set(cp.id, i);
      catLenM[i] = getPieceLengthMeters(entryMap.get(cp.catalogId));
    }

    const catOffsets = new Uint32Array(nCat + 1);
    let totalCatEdges = 0;
    for (let i = 0; i < nCat; i++) {
      const nbrs = catAdj.get(catenaryPieces[i]!.id);
      catOffsets[i] = totalCatEdges;
      if (nbrs) totalCatEdges += nbrs.size;
    }
    catOffsets[nCat] = totalCatEdges;

    const catNeighbors = new Uint32Array(totalCatEdges);
    let catCursor = 0;
    for (let i = 0; i < nCat; i++) {
      const nbrs = catAdj.get(catenaryPieces[i]!.id);
      if (nbrs) {
        for (const nbrId of nbrs) {
          const idx = catIdToIdx.get(nbrId);
          if (idx !== undefined) catNeighbors[catCursor++] = idx;
        }
      }
      catOffsets[i + 1] = catCursor;
    }

    const catFeederIdx: number[] = [];
    for (const cf of catenaryFeeders) {
      const idx = catIdToIdx.get(cf.id);
      if (idx !== undefined) catFeederIdx.push(idx);
    }

    const catSolved = solvePowerDijkstraKernel(
      nCat,
      catOffsets,
      catNeighbors.subarray(0, catCursor),
      catLenM,
      new Uint32Array(catFeederIdx),
      catenaryOhmPerMeter,
      catenaryJointResOhm,
    );

    for (let i = 0; i < nCat; i++) {
      minCatenaryRes.set(catenaryPieces[i]!.id, catSolved.minResistance[i]!);
    }
  }

  let minV = feedVoltage;
  let maxDropPct = 0;
  let optimalCount = 0;
  let warningCount = 0;
  let criticalCount = 0;
  let disconnectedCount = 0;

  // Oberleitungs-Kennzahlen
  let catMinV = catenaryFeedVoltage;
  let catMaxDropPct = 0;
  let catOptimalCount = 0;
  let catWarningCount = 0;
  let catCriticalCount = 0;
  let catDisconnectedCount = 0;

  // 2-Leiter Kurzschluss-Prüfung (Short Circuit Detection)
  const shortCircuitRes = detectShortCircuit(layout, catalog);

  for (const p of pieces) {
    let res = minResistance.get(p.id) ?? Infinity;
    const distM = minDistance.get(p.id) ?? 0;
    const isFeeder = feeders.some((f) => f.id === p.id);
    const isShortCircuit = shortCircuitRes.shortCircuitPieces.has(p.id);
    const entry = entryMap.get(p.catalogId);
    const isTurnout = isTurnoutEntry(entry);
    const accMa = pieceAccessoryMa.get(p.id) ?? 0;
    const accDrop = accessoryDropMap.get(p.id) ?? 0;
    const hasCatenary = Boolean(p.hasCatenary);
    const catRes = hasCatenary ? (minCatenaryRes.get(p.id) ?? Infinity) : undefined;
    const isCatFeeder = catenaryFeeders.some((cf) => cf.id === p.id);

    // Parallelschaltung (Oberleitung als Ringleitungs-Booster)
    if (catenaryCircuitMode === "parallel" && hasCatenary && catRes !== undefined && res < Infinity && catRes < Infinity) {
      res = (res * catRes) / (res + catRes);
    }

    let catVoltage: number | undefined;
    let catDrop: number | undefined;
    let catDropPercent: number | undefined;
    let catStatus: PowerStatus | undefined;

    if (hasCatenary && catRes !== undefined) {
      if (catRes === Infinity) {
        if (catenaryFeeders.length === 0) {
          catMinV = 0;
          catMaxDropPct = 100;
        }
        catVoltage = 0;
        catDrop = catenaryFeedVoltage;
        catDropPercent = 100;
        catStatus = "disconnected";
        catDisconnectedCount++;
      } else if (isCatFeeder) {
        // Am Einspeisepunkt der Oberleitung liegt stets die volle Nennspannung an (kein Abfall)
        catDrop = 0;
        catVoltage = catenaryFeedVoltage;
        catDropPercent = 0;
        catStatus = "optimal";
        catOptimalCount++;
        if (catVoltage < catMinV) catMinV = catVoltage;
      } else {
        catDrop = current * catRes;
        catVoltage = Math.max(0, catenaryFeedVoltage - catDrop);
        catDropPercent = (catDrop / catenaryFeedVoltage) * 100;

        if (catDropPercent > 15) {
          catStatus = "critical";
          catCriticalCount++;
        } else if (catDropPercent > 5) {
          catStatus = "warning";
          catWarningCount++;
        } else {
          catStatus = "optimal";
          catOptimalCount++;
        }

        if (catVoltage < catMinV) catMinV = catVoltage;
        if (catDropPercent > catMaxDropPct) catMaxDropPct = catDropPercent;
      }
    }

    if (res === Infinity) {
      if (feeders.length === 0) {
        minV = 0;
        maxDropPct = 100;
      }
      results.set(p.id, {
        pieceId: p.id,
        voltage: 0,
        voltageDrop: feedVoltage,
        voltageDropPercent: 100,
        status: isShortCircuit ? "critical" : "disconnected",
        resistanceOhm: Infinity,
        distanceFromFeederMeters: 0,
        isFeeder,
        isShortCircuit,
        isTurnout,
        hasLantern: p.hasLantern,
        hasDecoder: p.hasDecoder,
        accessoryCurrentMa: accMa > 0 ? accMa : undefined,
        accessoryDrop: undefined,
        hasCatenary,
        catenaryVoltage: catVoltage !== undefined ? Math.round(catVoltage * 100) / 100 : undefined,
        catenaryDrop: catDrop !== undefined ? Math.round(catDrop * 100) / 100 : undefined,
        catenaryDropPercent: catDropPercent !== undefined ? Math.round(catDropPercent * 10) / 10 : undefined,
        catenaryStatus: catStatus,
        catenaryResistanceOhm: catRes !== undefined && catRes < Infinity ? Math.round(catRes * 1000) / 1000 : Infinity,
        isCatenaryFeeder: isCatFeeder,
      });
      disconnectedCount++;
    } else if (isFeeder) {
      // Am Einspeisepunkt liegt stets die volle Nennspannung an (kein Abfall am Feeder)
      const v = isShortCircuit ? 0 : feedVoltage;
      const dropV = isShortCircuit ? feedVoltage : 0;
      const dropPct = isShortCircuit ? 100 : 0;
      const status: PowerStatus = isShortCircuit ? "critical" : "optimal";
      if (isShortCircuit) criticalCount++; else optimalCount++;
      if (v < minV) minV = v;

      results.set(p.id, {
        pieceId: p.id,
        voltage: Math.round(v * 100) / 100,
        voltageDrop: Math.round(dropV * 100) / 100,
        voltageDropPercent: Math.round(dropPct * 10) / 10,
        status,
        resistanceOhm: 0,
        distanceFromFeederMeters: 0,
        isFeeder,
        isShortCircuit,
        isTurnout,
        hasLantern: p.hasLantern,
        hasDecoder: p.hasDecoder,
        accessoryCurrentMa: accMa > 0 ? accMa : undefined,
        accessoryDrop: undefined,
        hasCatenary,
        catenaryVoltage: catVoltage !== undefined ? Math.round(catVoltage * 100) / 100 : undefined,
        catenaryDrop: catDrop !== undefined ? Math.round(catDrop * 100) / 100 : undefined,
        catenaryDropPercent: catDropPercent !== undefined ? Math.round(catDropPercent * 10) / 10 : undefined,
        catenaryStatus: catStatus,
        catenaryResistanceOhm: catRes !== undefined && catRes < Infinity ? Math.round(catRes * 1000) / 1000 : Infinity,
        isCatenaryFeeder: isCatFeeder,
      });
    } else {
      const tractionDrop = current * res;
      const dropV = accDrop + tractionDrop;
      const v = isShortCircuit ? 0 : Math.max(0, feedVoltage - dropV);
      const dropPct = isShortCircuit ? 100 : (dropV / feedVoltage) * 100;

      let status: PowerStatus = "optimal";
      if (isShortCircuit || dropPct > 15) {
        status = "critical";
        criticalCount++;
      } else if (dropPct > 5) {
        status = "warning";
        warningCount++;
      } else {
        optimalCount++;
      }

      if (v < minV) minV = v;
      if (dropPct > maxDropPct) maxDropPct = dropPct;

      results.set(p.id, {
        pieceId: p.id,
        voltage: Math.round(v * 100) / 100,
        voltageDrop: Math.round(dropV * 100) / 100,
        voltageDropPercent: Math.round(dropPct * 10) / 10,
        status,
        resistanceOhm: Math.round(res * 1000) / 1000,
        distanceFromFeederMeters: Math.round(distM * 100) / 100,
        isFeeder,
        isShortCircuit,
        isTurnout,
        hasLantern: p.hasLantern,
        hasDecoder: p.hasDecoder,
        accessoryCurrentMa: accMa > 0 ? accMa : undefined,
        accessoryDrop: accDrop > 0 ? Math.round(accDrop * 1000) / 1000 : undefined,
        hasCatenary,
        catenaryVoltage: catVoltage !== undefined ? Math.round(catVoltage * 100) / 100 : undefined,
        catenaryDrop: catDrop !== undefined ? Math.round(catDrop * 100) / 100 : undefined,
        catenaryDropPercent: catDropPercent !== undefined ? Math.round(catDropPercent * 10) / 10 : undefined,
        catenaryStatus: catStatus,
        catenaryResistanceOhm: catRes !== undefined && catRes < Infinity ? Math.round(catRes * 1000) / 1000 : (hasCatenary ? Infinity : undefined),
        isCatenaryFeeder: isCatFeeder,
      });
    }
  }

  const turnoutInfoStr = totalTurnoutAccessoryMa > 0
    ? ` inkl. ${totalTurnoutAccessoryMa} mA Weichen-Zubehör (${turnoutLanternCount}x Laterne, ${turnoutDecoderCount}x Decoder)`
    : "";
  let recommendation = `Spannungsversorgung bei ${totalCurrentAmps.toFixed(2)} A Gesamtlast (${current.toFixed(2)} A Traktion [${locoCount}x ${locoCurrentMa} mA]${turnoutInfoStr}) ist im gesamten Gleisbild optimal.`;
  if (shortCircuitRes.hasShortCircuit) {
    recommendation = `⚠️ ACHTUNG: Kurzschluss im 2-Leiter-System erkannt! Linke und rechte Schiene sind kurzgeschlossen (${shortCircuitRes.shortCircuitPieces.size} Gleise betroffen). Bitte Trennstellen (I) und Kehrschleifenmodul einbauen. ${recommendation}`;
  } else if (feeders.length === 0 && pieces.length > 0) {
    recommendation = `⚠️ Keine Gleiseinspeisung vorhanden! Alle ${pieces.length} Gleise sind stromlos (0 V). Bitte mindestens ein Gleis als Einspeisepunkt setzen (Taste F oder Anschlussgleis).`;
  } else if (disconnectedCount > 0) {
    recommendation = `Achtung: ${disconnectedCount} Gleise haben keine elektrische Verbindung zu einer Einspeisung!`;
  } else if (criticalCount > 0) {
    recommendation = `Kritischer Spannungsabfall bei ${criticalCount} Gleisen (>15%) bei ${totalCurrentAmps.toFixed(2)} A Gesamtlast. Zusätzliche Stromeinspeisung alle 2 bis 3 Meter dringend empfohlen!`;
  } else if (warningCount > 0) {
    recommendation = `Leichter Spannungsabfall bei ${warningCount} Gleisen (5-15%) bei ${totalCurrentAmps.toFixed(2)} A Gesamtlast. Bei Mehrzugbetrieb zusätzliche Einspeisung empfohlen.`;
  }

  let catRecommendation = catenaryPieceCount > 0
    ? `Oberleitungs-Versorgung (${catenaryPieceCount} Gleise): ${catenaryFeeders.length} Einspeisepunkt(e) aktiv.`
    : undefined;
  if (catenaryPieceCount > 0) {
    if (catenaryCircuitMode === "separate" && catenaryFeeders.length === 0) {
      catRecommendation = `⚠️ Keine Oberleitungs-Einspeisung vorhanden! Alle ${catenaryPieceCount} Oberleitungs-Abschnitte sind stromlos (0 V). Bitte Oberleitungs-Einspeisepunkt setzen (Taste Shift+F) oder auf Parallelbetrieb umschalten.`;
    } else if (catDisconnectedCount > 0) {
      catRecommendation = `Achtung: ${catDisconnectedCount} Oberleitungsabschnitte sind von der Fahrleitungs-Einspeisung getrennt (Trennstelle prüfen)!`;
    } else if (catCriticalCount > 0) {
      catRecommendation = `Kritischer Spannungsabfall in der Oberleitung bei ${catCriticalCount} Abschnitten (>15%). Zusätzliche Oberleitungs-Einspeisung erforderlich!`;
    } else if (catWarningCount > 0) {
      catRecommendation = `Leichter Spannungsabfall in der Oberleitung (${catWarningCount} Abschnitte).`;
    }
  }

  const isolatorCount = layout.pieces.filter((p) => p.isIsolator).length;
  const catIsolatorCount = layout.pieces.filter((p) => p.hasCatenary && p.catenaryIsolator).length;

  return {
    enabled: true,
    feedVoltage,
    catenaryFeedVoltage: catenaryPieceCount > 0 ? catenaryFeedVoltage : undefined,
    catenaryCircuitMode: catenaryPieceCount > 0 ? catenaryCircuitMode : undefined,
    locoCurrentMa,
    locoCount,
    trainCurrentAmps: current,
    turnoutCount,
    turnoutLanternCount,
    turnoutDecoderCount,
    turnoutAccessoryCurrentAmps,
    totalCurrentAmps,
    turnoutLanternCurrentMa,
    turnoutDecoderCurrentMa,
    feederCount: feeders.length,
    isolatorCount,
    minVoltage: Math.round(minV * 100) / 100,
    maxDropPercent: Math.round(maxDropPct * 10) / 10,
    optimalCount,
    warningCount,
    criticalCount,
    disconnectedCount,
    results,
    recommendation,
    hasShortCircuit: shortCircuitRes.hasShortCircuit,
    shortCircuitPieces: Array.from(shortCircuitRes.shortCircuitPieces),
    shortCircuitPoints: shortCircuitRes.shortCircuitPoints,
    catenaryPieceCount: catenaryPieceCount > 0 ? catenaryPieceCount : undefined,
    catenaryFeederCount: catenaryPieceCount > 0 ? catenaryFeeders.length : undefined,
    catenaryIsolatorCount: catenaryPieceCount > 0 ? catIsolatorCount : undefined,
    catenaryMinVoltage: catenaryPieceCount > 0 ? Math.round(catMinV * 100) / 100 : undefined,
    catenaryMaxDropPercent: catenaryPieceCount > 0 ? Math.round(catMaxDropPct * 10) / 10 : undefined,
    catenaryOptimalCount: catenaryPieceCount > 0 ? catOptimalCount : undefined,
    catenaryWarningCount: catenaryPieceCount > 0 ? catWarningCount : undefined,
    catenaryCriticalCount: catenaryPieceCount > 0 ? catCriticalCount : undefined,
    catenaryDisconnectedCount: catenaryPieceCount > 0 ? catDisconnectedCount : undefined,
    catenaryRecommendation: catRecommendation,
  };
}
