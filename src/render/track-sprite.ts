import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { TrackPiece, ColorMode } from "@/core/types";
import { Catalog, isFeederEntry, isIsolatingEntry, type CatalogEntry } from "@/catalog/catalog";
import type { PiecePowerResult } from "@/core/simulation";
import { deg2rad, getPieceLocalCenter } from "@/core/geometry";
import { getTrackBlueprintColor, type BlueprintColor } from "./blueprint-colors";
import { getEffectiveFlexEntry } from "@/core/flextrack";
import { desaturateColor, darkenColor } from "./track-render-utils";
import { drawRoadElevationShadow, drawRoadSelectionContour, drawRoadPiece2D } from "./road-sprite";
import { drawBallast, drawTies } from "./track-geometry-draw";
import { drawRails, drawTrackCenterline, drawSelectionContour } from "./track-rails-draw";
import {
  getPieceRunningLength,
  getTrackBallastWidth,
  drawTrackNumberLabel,
  drawVoltageBadge,
  drawElevationShadow,
  drawRampChevrons,
  drawRampBadge,
} from "./track-badges";
import {
  drawPowerFeederMarker,
  drawIsolatorMarker,
  drawCatenary2D,
  CatenaryMast2DContext,
  drawCatenaryPowerFeederMarker,
  drawCatenaryIsolatorMarker,
  drawTurnoutAccessories2D,
  drawBridge2D,
  drawTrackSignal2D,
} from "./track-accessories-sprite";
import { drawTunnel2D } from "./track-tunnel";

export { desaturateColor };

const ROTATION_GIZMO_TEXT_STYLE = new TextStyle({
  fontSize: 16,
  fill: 0xffffff,
  fontWeight: "bold",
});

export interface RenderOptions {
  isSelected?: boolean;
  showConnectors?: boolean;
  powerResult?: PiecePowerResult;
  isShortCircuit?: boolean;
  showPowerDrop?: boolean;
  showTrackLabels?: boolean;
  showColorCode?: boolean;
  colorMode?: ColorMode;
  layerColor?: string;
  layerHeightMm?: number;
  /** True, wenn das Gleis auf einer anderen als der aktuell ausgewählten Ebene liegt */
  isInactiveLayer?: boolean;
  /** Minimale Höhe (mm) aller sichtbaren Gleise im Layout für relativen Überführungs-Schatten */
  minLayoutElev?: number;
  allPieces?: TrackPiece[];
  /** Aktueller Welt-Zoom-Faktor für gestochen scharfe Text-Auflösung auf Labels */
  worldScale?: number;
  /** Gemeinsamer Kontext zur Deduplizierung von 2D-Oberleitungsmasten an Gleisübergängen */
  catenaryMastCtx?: CatenaryMast2DContext;
}

/**
 * Zeichnet ein Modellbahn-Gleisstück realistisch mit PixiJS Graphics:
 * 1. Schotterbett (Grau/Anthrazit oder nach Gleistyp / Ebene)
 * 2. Schwellen (Dunkelbraun, quer zur Gleisachse)
 * 3. Schienenprofil (Stahl/Silber, zwei Schienen mit Spurweite H0 = 16,5 mm)
 * 4. Heatmap-Überlagerung für Power-Drop Simulation
 * 5. Einspeisepunkt-Marker (⚡)
 * 6. Anschlusspunkte (Connectors)
 * 7. Selektions-Rahmen & Drehgriff (Rotation Handle Gizmo)
 */
export function createTrackGraphics(
  piece: TrackPiece,
  rawEntry: CatalogEntry,
  _catalog?: Catalog,
  options: RenderOptions = {},
): Container {
  const entry = getEffectiveFlexEntry(rawEntry, piece);
  const container = new Container();
  container.label = `track-${piece.id}`;
  container.position.set(piece.transform.x, piece.transform.y);
  container.rotation = deg2rad(piece.transform.rotation);

  const g = new Graphics();
  container.addChild(g);

  const isSelected = options.isSelected ?? false;
  const showPowerDrop = options.showPowerDrop ?? false;
  const powerResult = options.powerResult;
  const isNumberLabeled = options.showTrackLabels ?? false;

  // Bestimme Farbmodus: "off" | "types" | "layers"
  let colorMode: ColorMode = "off";
  if (options.colorMode) {
    colorMode = options.colorMode;
  } else if (options.showColorCode) {
    colorMode = "types";
  }

  const bpColor = getTrackBlueprintColor(piece.catalogId);
  const isInactiveLayer = Boolean(options.isInactiveLayer && !isSelected);

  let ballastColor = 0x8a8479;
  let tieColor = 0x3d271d;
  let railColor = 0xc8d0d8;
  let activeBpColor: BlueprintColor | undefined = bpColor;

  if (colorMode === "types") {
    ballastColor = bpColor.hex;
    tieColor = 0x1e293b;
    railColor = 0xf8fafc;
    activeBpColor = bpColor;
  } else if (colorMode === "layers") {
    const hexStr = options.layerColor || "#38bdf8";
    const parsedHex = parseInt(hexStr.replace(/^#/, ""), 16);
    const validHex = isNaN(parsedHex) ? 0x38bdf8 : parsedHex;
    ballastColor = validHex;
    tieColor = 0x1e293b;
    railColor = 0xf8fafc;
    activeBpColor = {
      hex: validHex,
      css: hexStr,
      badgeBg: hexStr,
      badgeText: "#ffffff",
      border: "#ffffff",
    };
  } else {
    // "off": Realistische Farben
    ballastColor = 0x8a8479;
    tieColor = 0x3d271d;
    railColor = 0xc8d0d8;
    activeBpColor = bpColor;
  }

  // Nicht-aktive Ebenen stark entsättigen; aktive Ebene dezent entsättigen (18 %),
  // während selektierte Gleise volle Farbsättigung behalten und klar hervorstechen
  if (isInactiveLayer) {
    ballastColor = desaturateColor(ballastColor, 0.72);
    tieColor = desaturateColor(tieColor, 0.72);
    railColor = desaturateColor(railColor, 0.72);
    if (activeBpColor) {
      const desatHex = desaturateColor(activeBpColor.hex, 0.72);
      const desatCss = `#${desatHex.toString(16).padStart(6, "0")}`;
      activeBpColor = {
        ...activeBpColor,
        hex: desatHex,
        css: desatCss,
        badgeBg: desatCss,
      };
    }
    container.alpha = 0.58;
  } else if (!isSelected) {
    const activeDarken = 0.30;
    ballastColor = darkenColor(ballastColor, activeDarken);
    tieColor = darkenColor(tieColor, activeDarken);
    railColor = darkenColor(railColor, activeDarken);
    if (activeBpColor) {
      const darkHex = darkenColor(activeBpColor.hex, activeDarken);
      const darkCss = `#${darkHex.toString(16).padStart(6, "0")}`;
      activeBpColor = {
        ...activeBpColor,
        hex: darkHex,
        css: darkCss,
        badgeBg: darkCss,
      };
    }
  }

  // Spurweite & Dimensionen (H0 = 16.5mm, TT = 12mm, N = 9mm, etc.)
  const gauge = entry.gauge ?? 16.5;
  const gaugeHalf = gauge / 2;
  const ballastWidth = getTrackBallastWidth(entry);
  const worldScale = options.worldScale ?? 1;
  const tieLenHalf = gauge * 0.78;
  const tieWidth = gauge >= 30 ? 6.5 : (gauge >= 16 ? 2.6 : (gauge >= 12 ? 2.0 : 1.5));
  const tieSpacing = gauge >= 30 ? 16.0 : (gauge >= 16 ? 6.5 : (gauge >= 12 ? 4.8 : 3.5));

  // Elevation & Rampen-Berechnung
  const baseElev = options.layerHeightMm ?? 0;
  const startElev = baseElev + (piece.elevation?.start ?? 0);
  const endElev = baseElev + (piece.elevation?.end ?? 0);
  const maxElev = Math.max(startElev, endElev);
  const minElev = Math.min(startElev, endElev);
  // Elevation-Schatten nur für Gleise werfen, die physisch über der Tischplatte liegen (minElev >= 0 && maxElev >= 6).
  // Gleise, die in Tischausschnitte abtauchen oder unterirdisch verlaufen (maxElev <= 1), werfen keinen Schatten.
  const isElevated = minElev >= 0 && maxElev >= 6;
  const shadowElev = Math.max(0, maxElev);
  const heightDiff = endElev - startElev;
  const isRamp = Math.abs(heightDiff) >= 0.5;
  const pieceLength = getPieceRunningLength(entry);
  const gradPercent = pieceLength > 0 ? (Math.abs(heightDiff) / pieceLength) * 100 : 0;

  // --- STRAßEN-SPEZIFISCHES RENDERING (Asphalt, Fahrbahnmarkierungen & Kontur) ---
  if (entry.isRoad) {
    const roadWidth = entry.roadWidth ?? 60;
    if (isElevated) {
      drawRoadElevationShadow(g, entry, roadWidth, shadowElev);
    }
    if (isSelected) {
      drawRoadSelectionContour(g, entry, roadWidth + 8);
    }
    drawRoadPiece2D(g, entry, isSelected);
    if (piece.hasBridge) {
      drawBridge2D(container, entry, piece);
    }
    if (piece.hasTunnel) {
      drawTunnel2D(container, entry, piece, options.allPieces);
    }

    if (isNumberLabeled) {
      drawTrackNumberLabel(container, entry, piece.transform.rotation, activeBpColor, false, worldScale);
    }
    if (isRamp) {
      drawRampChevrons(g, entry, heightDiff, gradPercent);
      drawRampBadge(
        container,
        entry,
        piece.transform.rotation,
        startElev,
        endElev,
        gradPercent,
        isNumberLabeled,
        false,
        worldScale,
      );
    }
    if (options.showConnectors) {
      drawConnectors(g, piece, entry);
    }
    if (isSelected) {
      drawSelectionGizmo(container, entry);
    }
    return container;
  }

  // --- ELEVATION TIEFEN-SCHATTEN (Wenn Gleis über Grundplatte oder tieferen Gleisen liegt) ---
  if (isElevated) {
    drawElevationShadow(g, entry, ballastWidth, shadowElev);
  }

  // --- SELECTION GLOW (Präzise Kontur um den gesamten Gleiskörper) ---
  if (isSelected) {
    drawSelectionContour(g, entry, ballastWidth + 8);
  }

  // --- 1. SCHOTTERBETT ---
  drawBallast(g, entry, ballastColor, ballastWidth);

  // --- 2. SCHWELLEN ---
  drawTies(g, entry, tieColor, tieLenHalf, tieWidth, tieSpacing);

  // --- 3. SCHIENEN ---
  drawRails(g, entry, railColor, gaugeHalf);

  // --- 4. POWER-DROP HEATMAP & KURZSCHLUSS ---
  const isShortCircuit = Boolean(options.isShortCircuit || powerResult?.isShortCircuit);

  if (isShortCircuit && !showPowerDrop) {
    // Rotes Warn-Glühen bei Kurzschluss (2-Leiter-System)
    drawTrackCenterline(g, entry, 0xef4444, ballastWidth * 0.76, 0.45);
    drawTrackCenterline(g, entry, 0xdc2626, 4, 0.95);
  }

  if (showPowerDrop && powerResult) {
    let powerColor = 0x10b981; // Grün: optimal (> 15V)
    let powerAlpha = 0.35;
    let coreWidth = 6;
    if (powerResult.status === "warning") {
      powerColor = 0xf59e0b; // Gelb/Bernstein: Warnung
    } else if (powerResult.status === "critical") {
      powerColor = 0xef4444; // Rot: Kritischer Abfall
    } else if (powerResult.status === "disconnected") {
      powerColor = 0xdc2626; // Rot für isoliert / spannungslos
      powerAlpha = 0.22;
      coreWidth = 3;
    }

    // Breites Glühen auf dem Schotterbett
    drawTrackCenterline(g, entry, powerColor, ballastWidth * 0.72, powerAlpha);
    // Deutliche Stromlinie im Zentrum
    drawTrackCenterline(g, entry, powerColor, coreWidth, 0.95);

    // Deutliche Spannungs-Plakette auf dem Gleis
    drawVoltageBadge(container, entry, piece.transform.rotation, powerResult, isNumberLabeled, worldScale);
  }

  // --- 5. EINSPEISEPUNKT MARKER (⚡) ---
  if (piece.powerFeed) {
    drawPowerFeederMarker(container, entry);
  } else if (isFeederEntry(entry)) {
    drawPowerFeederMarker(container, entry, true);
  }

  // --- 5B. SPANNUNGSTRENNSTELLE MARKER (⛔) ---
  if (piece.isIsolator) {
    drawIsolatorMarker(container, entry, piece.isolatedConnectorIndex ?? 0);
  } else if (isIsolatingEntry(entry)) {
    drawIsolatorMarker(container, entry, piece.isolatedConnectorIndex ?? 0, true);
  }

  // --- 5C. OBERLEITUNG (Fahrdraht & Masten) ---
  if (piece.hasCatenary) {
    drawCatenary2D(container, entry, showPowerDrop ? powerResult : undefined, piece, options.allPieces, options.catenaryMastCtx);
    if (piece.catenaryPowerFeed) {
      drawCatenaryPowerFeederMarker(container, entry);
    }
    if (piece.catenaryIsolator) {
      drawCatenaryIsolatorMarker(container, entry, piece.catenaryIsolatedConnectorIndex ?? 0);
    }
  }

  // --- 5D. WEICHEN-ZUBEHÖR (Laterne 💡 & Decoder 📟) ---
  if (piece.hasLantern || piece.hasDecoder) {
    drawTurnoutAccessories2D(container, entry, piece);
  }

  // --- 5E. BRÜCKEN-AUFBAU (Träger, Geländer & Widerlager) ---
  if (piece.hasBridge) {
    drawBridge2D(container, entry, piece);
  }
  if (piece.hasTunnel) {
    drawTunnel2D(container, entry, piece, options.allPieces);
  }

  // --- 5F. LICHTSIGNAL / STRECKENSIGNAL (🚦) ---
  if (piece.hasSignal) {
    drawTrackSignal2D(container, entry, piece);
  }

  // --- 6. GLEISNUMMER / BAUPLAN-LABEL (z. B. 24188, 24611, 24624) ---
  if (isNumberLabeled) {
    drawTrackNumberLabel(container, entry, piece.transform.rotation, activeBpColor, showPowerDrop && !!powerResult, worldScale);
  }

  // --- 6B. RAMPEN-VISUALISIERUNG (Chevrons & Steigungs-Plakette) ---
  if (isRamp) {
    drawRampChevrons(g, entry, heightDiff, gradPercent);
    drawRampBadge(
      container,
      entry,
      piece.transform.rotation,
      startElev,
      endElev,
      gradPercent,
      isNumberLabeled,
      showPowerDrop && !!powerResult,
      worldScale,
    );
  }

  // --- 7. CONNECTORS ---
  if (options.showConnectors) {
    drawConnectors(g, piece, entry);
  }

  // --- 8. DREHGRIFF (ROTATION HANDLE GIZMO) WENN SELEKTIERT ---
  if (isSelected) {
    drawSelectionGizmo(container, entry);
  }

  return container;
}

function drawConnectors(g: Graphics, piece: TrackPiece, _entry: CatalogEntry) {
  for (let i = 0; i < piece.connectors.length; i++) {
    const c = piece.connectors[i]!;
    const isFree = !c.connectedTo;
    const color = isFree ? 0x22c55e : 0x94a3b8;
    const rad = isFree ? 4.5 : 2.5;

    if (i === 0) {
      g.circle(0, 0, rad);
      g.fill({ color });
      if (isFree) {
        g.circle(0, 0, 2);
        g.fill({ color: 0xffffff });
      }
    }
  }
}

function drawSelectionGizmo(container: Container, entry: CatalogEntry) {
  const gizmo = new Container();
  gizmo.label = "selection-gizmo";

  const g = new Graphics();
  const center = getPieceLocalCenter(entry);
  const stemOffset = entry.kind === "turntable"
    ? 220
    : entry.kind === "transfer-table"
      ? 240
      : entry.kind === "double-slip"
        ? 62
        : entry.isRoad
          ? 50
          : Math.max(48, Math.round((entry.gauge ?? 16.5) * 1.09 + 20));
  const handlePos = { x: center.x, y: center.y - stemOffset };

  // 1. Verbindungsschaft zum Drehgriff
  g.moveTo(center.x, center.y).lineTo(handlePos.x, handlePos.y);
  g.stroke({ width: 2.5, color: 0x0284c7, alpha: 0.9 });

  // 2. Mittelpunkts-Marker auf der Gleisachse
  g.circle(center.x, center.y, 5);
  g.fill({ color: 0x0284c7 });
  g.circle(center.x, center.y, 2.5);
  g.fill({ color: 0xffffff });

  // 3. Dreh-Knopf (Rotation Handle) mit größerer, griffiger Trefferfläche
  g.circle(handlePos.x, handlePos.y, 14);
  g.fill({ color: 0x0284c7 });
  g.stroke({ width: 2.2, color: 0xffffff });

  // Symbol ↻
  const rotText = new Text({
    text: "↻",
    style: ROTATION_GIZMO_TEXT_STYLE,
  });
  rotText.anchor.set(0.5);
  rotText.position.set(handlePos.x, handlePos.y);
  rotText.rotation = -container.rotation;

  gizmo.addChild(g);
  gizmo.addChild(rotText);
  container.addChild(gizmo);
}
