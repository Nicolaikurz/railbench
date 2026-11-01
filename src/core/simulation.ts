import type { Layout, TrackPiece } from "./types";
import { computeWorldConnectors, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import { dist, deg2rad, normAngle } from "./geometry";

export type PowerStatus = "optimal" | "warning" | "critical" | "disconnected";

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
  "1": { scale: "1", label: "Spur 1", minMa: 600, maxMa: 1000, defaultMa: 800, description: "600–1000 mA pro Lok" },
  G: { scale: "G", label: "Spur G", minMa: 600, maxMa: 1000, defaultMa: 1000, description: "Max. 1000 mA (1A) pro Lok" },
};

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
    entry.kind === "dkw"
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

export interface SimulatePowerDropOptions {
  trainCurrentAmps?: number;
  feedVoltage?: number;
  catenaryFeedVoltage?: number;
  catenaryCircuitMode?: "separate" | "parallel";
  locoCurrentMa?: number;
  locoCount?: number;
  turnoutLanternCurrentMa?: number;
  turnoutDecoderCurrentMa?: number;
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
  const feedVoltage = options.feedVoltage ?? layout.feedVoltage ?? 16.0;

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
      results,
      recommendation: "Keine Gleise im Plan vorhanden.",
    };
  }

  // 1. Einspeisepunkte ermitteln (falls keine manuell gesetzt, erstes reguläres Gleis als Standard-Einspeisung)
  const explicitFeeders = pieces.filter((p) => p.powerFeed);
  const fallbackFeeder = pieces.find((p) => !p.isIsolator) ?? pieces[0];
  const feeders: TrackPiece[] = explicitFeeders.length > 0 ? explicitFeeders : (fallbackFeeder ? [fallbackFeeder] : []);

  // Welt-Connectors für alle Gleise aktualisieren/sicherstellen
  for (const p of pieces) {
    const entry = entryMap.get(p.catalogId);
    if (entry) {
      p.connectors = computeWorldConnectors(entry, p.transform);
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

  // 3. Multi-Source Dijkstra zur Berechnung des kürzesten elektrischen Widerstands von Einspeisungen
  const minResistance = new Map<string, number>();
  const minDistance = new Map<string, number>();
  const parent = new Map<string, string | null>();
  const edgeResToParent = new Map<string, number>();
  const pq: { pieceId: string; res: number; distM: number }[] = [];

  for (const p of pieces) {
    minResistance.set(p.id, Infinity);
    minDistance.set(p.id, Infinity);
  }

  for (const feeder of feeders) {
    const entry = entryMap.get(feeder.catalogId);
    const pieceLen = getPieceLengthMeters(entry);
    const initialRes = (pieceLen / 2) * railOhmPerMeter; // Mittlerer Widerstand am Einspeisepunkt
    minResistance.set(feeder.id, initialRes);
    minDistance.set(feeder.id, 0);
    parent.set(feeder.id, null);
    edgeResToParent.set(feeder.id, initialRes);
    pq.push({ pieceId: feeder.id, res: initialRes, distM: 0 });
  }

  // Dijkstra ausführen
  while (pq.length > 0) {
    pq.sort((a, b) => a.res - b.res);
    const curr = pq.shift()!;
    if (curr.res > (minResistance.get(curr.pieceId) ?? Infinity)) continue;

    const currPiece = pieces.find((p) => p.id === curr.pieceId);
    if (!currPiece) continue;

    const neighbors = adj.get(curr.pieceId) ?? new Set();
    for (const nbrId of neighbors) {
      const nbrPiece = pieces.find((p) => p.id === nbrId);
      if (!nbrPiece) continue;

      const nbrEntry = entryMap.get(nbrPiece.catalogId);
      const lenM = getPieceLengthMeters(nbrEntry);
      const edgeRes = lenM * railOhmPerMeter + jointResistanceOhm;
      const newRes = curr.res + edgeRes;
      const newDist = curr.distM + lenM;

      if (newRes < (minResistance.get(nbrId) ?? Infinity)) {
        minResistance.set(nbrId, newRes);
        minDistance.set(nbrId, newDist);
        parent.set(nbrId, curr.pieceId);
        edgeResToParent.set(nbrId, edgeRes);
        pq.push({ pieceId: nbrId, res: newRes, distM: newDist });
      }
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
  const catenaryFeedVoltage = options.catenaryFeedVoltage ?? layout.catenaryFeedVoltage ?? feedVoltage;
  const catenaryCircuitMode = options.catenaryCircuitMode ?? layout.catenaryCircuitMode ?? "separate";

  const catenaryPieces = pieces.filter((p) => !!p.hasCatenary);
  const catenaryPieceCount = catenaryPieces.length;

  const minCatenaryRes = new Map<string, number>();
  const catenaryFeeders: TrackPiece[] = [];
  const catenaryOhmPerMeter = 0.035; // Hochleitfähiger Kupfer-Fahrdraht + Tragseil
  const catenaryJointResOhm = 0.02; // Fahrleitungs-Steckverbinder

  if (catenaryPieceCount > 0) {
    const explicitCatFeeders = catenaryPieces.filter((p) => !!p.catenaryPowerFeed);
    const fallbackCatFeeder = catenaryPieces.find((p) => !p.catenaryIsolator) ?? catenaryPieces[0];
    const resolvedCatFeeders = explicitCatFeeders.length > 0 ? explicitCatFeeders : (fallbackCatFeeder ? [fallbackCatFeeder] : []);
    catenaryFeeders.push(...resolvedCatFeeders);

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

    // Dijkstra für Oberleitung
    const catPq: { pieceId: string; res: number }[] = [];
    for (const p of catenaryPieces) {
      minCatenaryRes.set(p.id, Infinity);
    }

    for (const cf of catenaryFeeders) {
      const entry = entryMap.get(cf.catalogId);
      const pieceLen = getPieceLengthMeters(entry);
      const initRes = (pieceLen / 2) * catenaryOhmPerMeter;
      minCatenaryRes.set(cf.id, initRes);
      catPq.push({ pieceId: cf.id, res: initRes });
    }

    while (catPq.length > 0) {
      catPq.sort((a, b) => a.res - b.res);
      const curr = catPq.shift()!;
      if (curr.res > (minCatenaryRes.get(curr.pieceId) ?? Infinity)) continue;

      const neighbors = catAdj.get(curr.pieceId) ?? new Set();
      for (const nbrId of neighbors) {
        const nbrPiece = catenaryPieces.find((p) => p.id === nbrId);
        if (!nbrPiece) continue;
        const nbrEntry = entryMap.get(nbrPiece.catalogId);
        const lenM = getPieceLengthMeters(nbrEntry);
        const edgeRes = lenM * catenaryOhmPerMeter + catenaryJointResOhm;
        const newRes = curr.res + edgeRes;

        if (newRes < (minCatenaryRes.get(nbrId) ?? Infinity)) {
          minCatenaryRes.set(nbrId, newRes);
          catPq.push({ pieceId: nbrId, res: newRes });
        }
      }
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

  for (const p of pieces) {
    let res = minResistance.get(p.id) ?? Infinity;
    const distM = minDistance.get(p.id) ?? 0;
    const isFeeder = feeders.some((f) => f.id === p.id);
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
        catVoltage = 0;
        catDrop = catenaryFeedVoltage;
        catDropPercent = 100;
        catStatus = "disconnected";
        catDisconnectedCount++;
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
      results.set(p.id, {
        pieceId: p.id,
        voltage: 0,
        voltageDrop: feedVoltage,
        voltageDropPercent: 100,
        status: "disconnected",
        resistanceOhm: Infinity,
        distanceFromFeederMeters: 0,
        isFeeder,
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
    } else {
      const tractionDrop = current * res;
      const dropV = accDrop + tractionDrop;
      const v = Math.max(0, feedVoltage - dropV);
      const dropPct = (dropV / feedVoltage) * 100;

      let status: PowerStatus = "optimal";
      if (dropPct > 15) {
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
  if (disconnectedCount > 0) {
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
    if (catDisconnectedCount > 0) {
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
