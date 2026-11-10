import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { TrackPiece, ColorMode } from "@/core/types";
import { Catalog, getLocalConnectors, type CatalogEntry } from "@/catalog/catalog";
import type { PiecePowerResult } from "@/core/simulation";
import { deg2rad, getPieceLocalCenter, getUprightTextRotation } from "@/core/geometry";
import { getTrackBlueprintColor, type BlueprintColor } from "./blueprint-colors";
import { getEffectiveFlexEntry } from "@/core/flextrack";

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
  catalog?: Catalog, // <-- Hier den Katalog des Gleises übergeben
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

  // Spurweite & Dimensionen (H0 = 16.5mm, TT = 12mm, N = 9mm, etc.)
  const gauge = entry.gauge ?? 16.5;
  const gaugeHalf = gauge / 2;
  const hasRoadbed = entry.hasRoadbed ?? true;
  const ballastWidth = hasRoadbed ? (gauge >= 16 ? 36 : gauge * 2.2) : gauge * 1.5;
  const tieLenHalf = gauge * 0.78;
  const tieWidth = gauge >= 16 ? 2.6 : (gauge >= 12 ? 2.0 : 1.5);
  const tieSpacing = gauge >= 16 ? 6.5 : (gauge >= 12 ? 4.8 : 3.5);

  // Elevation & Rampen-Berechnung
  const baseElev = options.layerHeightMm ?? 0;
  const startElev = baseElev + (piece.elevation?.start ?? 0);
  const endElev = baseElev + (piece.elevation?.end ?? 0);
  const maxElev = Math.max(startElev, endElev);
  const isElevated = maxElev >= 10;
  const heightDiff = endElev - startElev;
  const isRamp = Math.abs(heightDiff) >= 0.5;
  const pieceLength = getPieceRunningLength(entry);
  const gradPercent = pieceLength > 0 ? (Math.abs(heightDiff) / pieceLength) * 100 : 0;

  // --- STRAßEN-SPEZIFISCHES RENDERING (Asphalt, Fahrbahnmarkierungen & Kontur) ---
  if (entry.isRoad) {
    const roadWidth = entry.roadWidth ?? 60;
    if (isElevated) {
      drawRoadElevationShadow(g, entry, roadWidth, maxElev);
    }
    if (isSelected) {
      drawRoadSelectionContour(g, entry, roadWidth + 8);
    }
    drawRoadPiece2D(g, entry, isSelected);

    if (isNumberLabeled) {
      drawTrackNumberLabel(container, entry, piece.transform.rotation, activeBpColor, false);
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

  // --- ELEVATION TIEFEN-SCHATTEN (Wenn Gleis über Grundplatte angehoben ist) ---
  if (isElevated) {
    drawElevationShadow(g, entry, ballastWidth, maxElev);
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
    drawVoltageBadge(container, entry, piece.transform.rotation, powerResult, isNumberLabeled);
  }

  // --- 5. EINSPEISEPUNKT MARKER (⚡) ---
  if (piece.powerFeed) {
    drawPowerFeederMarker(container);
  }

  // --- 5B. SPANNUNGSTRENNSTELLE MARKER (⛔) ---
  if (piece.isIsolator) {
    drawIsolatorMarker(container, entry, piece.isolatedConnectorIndex ?? 0);
  }

  // --- 5C. OBERLEITUNG (Fahrdraht & Masten) ---
  if (piece.hasCatenary) {
    drawCatenary2D(container, entry, showPowerDrop ? powerResult : undefined);
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

  // --- 6. GLEISNUMMER / BAUPLAN-LABEL (z. B. 24188, 24611, 24624) ---
  if (isNumberLabeled) {
    drawTrackNumberLabel(container, entry, piece.transform.rotation, activeBpColor, showPowerDrop && !!powerResult);
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

function drawTrackNumberLabel(
  container: Container,
  entry: CatalogEntry,
  pieceRotation: number,
  bpColor?: BlueprintColor,
  hasVoltageLabel = false,
) {
  const center = getPieceLocalCenter(entry);
  const labelCont = new Container();
  labelCont.position.set(center.x, center.y);
  // Stets horizontal und lesbar (kein Kopfstand bei gedrehten Gleisen per Gegen-Rotation)
  labelCont.rotation = getUprightTextRotation(pieceRotation);

  const yOffset = entry.kind === "turntable" ? -55 : entry.kind === "transfer-table" ? -70 : hasVoltageLabel ? -9 : 0;

  const text = new Text({
    text: entry.id,
    style: new TextStyle({
      fontFamily: "Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
      fontSize: 10,
      fontWeight: "bold",
      fill: 0xffffff,
      align: "center",
      letterSpacing: 0.4,
    }),
  });
  text.anchor.set(0.5);
  text.position.set(0, yOffset);

  const padX = 5;
  const padY = 2.5;
  let textW = 32;
  let textH = 11;
  try {
    if (typeof document !== "undefined") {
      textW = text.width || 32;
      textH = text.height || 11;
    }
  } catch {
    textW = 32;
    textH = 11;
  }
  const bgW = textW + padX * 2;
  const bgH = textH + padY * 2;

  const bg = new Graphics();
  // Schatten
  bg.roundRect(-bgW / 2 + 0.5, -bgH / 2 + 1 + yOffset, bgW, bgH, 3);
  bg.fill({ color: 0x000000, alpha: 0.55 });
  // Hintergrund-Badge mit Farbkontur passend zum Bauplan-Farbcode
  bg.roundRect(-bgW / 2, -bgH / 2 + yOffset, bgW, bgH, 3);
  bg.fill({ color: 0x0f172a, alpha: 0.95 });
  bg.stroke({ width: 1.2, color: bpColor ? bpColor.hex : 0x38bdf8, alpha: 1.0 });

  labelCont.addChild(bg);
  labelCont.addChild(text);
  container.addChild(labelCont);
}

function drawVoltageBadge(
  container: Container,
  entry: CatalogEntry,
  pieceRotation: number,
  powerResult: PiecePowerResult,
  hasNumberLabel: boolean,
) {
  const center = getPieceLocalCenter(entry);
  const badgeCont = new Container();
  badgeCont.position.set(center.x, center.y);
  badgeCont.rotation = getUprightTextRotation(pieceRotation);
  const yOffset = hasNumberLabel ? 9 : 0;

  const isShort = Boolean(powerResult.isShortCircuit);
  const isDisc = powerResult.status === "disconnected";
  let voltText = isShort
    ? "⚠️⚡ KURZSCHLUSS"
    : isDisc
      ? "⛔ 0.0 V"
      : `${powerResult.isFeeder ? "⚡ " : ""}${powerResult.voltage.toFixed(1)} V`;

  if (powerResult.hasCatenary && powerResult.catenaryVoltage !== undefined) {
    const isCatDisc = powerResult.catenaryStatus === "disconnected";
    const catText = isCatDisc
      ? "OL: ⛔ 0V"
      : `OL: ${powerResult.isCatenaryFeeder ? "⚡" : ""}${powerResult.catenaryVoltage.toFixed(1)}V`;
    voltText = `${voltText} • ${catText}`;
  }

  let bgColor = 0x064e3b;
  let strokeColor = 0x10b981;
  let textColor = 0xecfdf5;

  if (isShort) {
    bgColor = 0x7f1d1d;
    strokeColor = 0xef4444;
    textColor = 0xfff1f2;
  } else if (isDisc) {
    bgColor = 0x450a0a;
    strokeColor = 0xdc2626;
    textColor = 0xfecaca;
  } else if (powerResult.status === "critical") {
    bgColor = 0x7f1d1d;
    strokeColor = 0xef4444;
    textColor = 0xfef2f2;
  } else if (powerResult.status === "warning") {
    bgColor = 0x78350f;
    strokeColor = 0xf59e0b;
    textColor = 0xfffbeb;
  }

  const text = new Text({
    text: voltText,
    style: new TextStyle({
      fontFamily: "Inter, system-ui, -apple-system, sans-serif",
      fontSize: 9.5,
      fontWeight: "bold",
      fill: textColor,
      align: "center",
      letterSpacing: 0.3,
    }),
  });
  text.anchor.set(0.5);
  text.position.set(0, yOffset);

  const padX = 5;
  const padY = 2;
  const bgW = text.width + padX * 2;
  const bgH = text.height + padY * 2;

  const bg = new Graphics();
  // Schatten
  bg.roundRect(-bgW / 2 + 0.5, -bgH / 2 + 1 + yOffset, bgW, bgH, 3);
  bg.fill({ color: 0x000000, alpha: 0.55 });
  // Badge Hintergrund & Rahmen
  bg.roundRect(-bgW / 2, -bgH / 2 + yOffset, bgW, bgH, 3);
  bg.fill({ color: bgColor, alpha: 0.95 });
  bg.stroke({ width: 1.2, color: strokeColor, alpha: 1.0 });

  badgeCont.addChild(bg);
  badgeCont.addChild(text);
  container.addChild(badgeCont);
}

function getPieceRunningLength(entry: CatalogEntry): number {
  if (entry.length) return entry.length;
  if (entry.radius && entry.angle) {
    return (2 * Math.PI * entry.radius * Math.abs(entry.angle)) / 360;
  }
  return 188.3;
}

function drawElevationShadow(g: Graphics, entry: CatalogEntry, ballastWidth: number, elevation: number) {
  const kind = entry.kind;
  const ballastHalf = ballastWidth / 2;
  const shadowDist = Math.min(10, 2 + elevation * 0.08);
  const shadowAlpha = Math.min(0.4, 0.12 + elevation * 0.003);
  const shadowColor = 0x000000;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? 188.3;
    g.rect(shadowDist * 0.5, -ballastHalf + shadowDist, l, ballastWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (kind === "turntable") {
    const r = entry.radius ?? 189;
    g.circle(shadowDist * 0.5, shadowDist, r);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const plateH = 420;
    const topY = -55.2;
    g.rect(shadowDist * 0.5, topY + shadowDist, l, plateH);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      drawArcStroke(
        g,
        shadowDist * 0.5,
        -r + shadowDist,
        r,
        Math.PI / 2,
        Math.PI / 2 - deg2rad(Math.abs(a)),
        ballastWidth,
        shadowColor,
        true,
        shadowAlpha,
      );
      return;
    }
    drawArcStroke(
      g,
      shadowDist * 0.5,
      r + shadowDist,
      r,
      -Math.PI / 2,
      -Math.PI / 2 + deg2rad(a),
      ballastWidth,
      shadowColor,
      false,
      shadowAlpha,
    );
    return;
  }

  const l = entry.length ?? 188.3;
  g.rect(shadowDist * 0.5, -ballastHalf + shadowDist, l, ballastWidth);
  g.fill({ color: shadowColor, alpha: shadowAlpha });
}

function drawChevron(g: Graphics, x: number, y: number, angleRad: number, size = 6.5, color = 0x38bdf8) {
  const dx = Math.cos(angleRad) * size;
  const dy = Math.sin(angleRad) * size;
  const nx = -Math.sin(angleRad) * (size * 0.8);
  const ny = Math.cos(angleRad) * (size * 0.8);

  g.moveTo(x - dx + nx, y - dy + ny);
  g.lineTo(x + dx * 0.6, y + dy * 0.6);
  g.lineTo(x - dx - nx, y - dy - ny);
  g.stroke({ width: 2.2, color, alpha: 0.95, cap: "round", join: "round" });
}

function drawRampChevrons(
  g: Graphics,
  entry: CatalogEntry,
  heightDiff: number,
  gradPercent: number,
) {
  let chevronColor = 0x38bdf8;
  if (gradPercent > 4.5) {
    chevronColor = 0xef4444;
  } else if (gradPercent > 3.0) {
    chevronColor = 0xf59e0b;
  }

  const kind = entry.kind;
  const climbingForward = heightDiff > 0;

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const absA = Math.abs(a);
    const rad = deg2rad(absA);
    const fractions = [0.25, 0.5, 0.75];
    for (const u of fractions) {
      const phi = u * rad;
      const cx = r * Math.sin(phi);
      const cy = a < 0 ? -r * (1 - Math.cos(phi)) : r * (1 - Math.cos(phi));
      const baseTangent = a < 0 ? -phi : phi;
      const tangentAngle = climbingForward ? baseTangent : baseTangent + Math.PI;
      drawChevron(g, cx, cy, tangentAngle, 6.5, chevronColor);
    }
    return;
  }

  const l = entry.length ?? 188.3;
  const arrowAngle = climbingForward ? 0 : Math.PI;
  const fractions = [0.25, 0.5, 0.75];
  for (const frac of fractions) {
    drawChevron(g, l * frac, 0, arrowAngle, 6.5, chevronColor);
  }
}

function drawRampBadge(
  container: Container,
  entry: CatalogEntry,
  pieceRotation: number,
  startElev: number,
  endElev: number,
  gradPercent: number,
  hasNumberLabel: boolean,
  hasVoltageLabel: boolean,
) {
  const center = getPieceLocalCenter(entry);
  const badgeCont = new Container();
  badgeCont.position.set(center.x, center.y);
  badgeCont.rotation = getUprightTextRotation(pieceRotation);

  let yOffset = 0;
  if (hasNumberLabel && hasVoltageLabel) {
    yOffset = 18;
  } else if (hasNumberLabel && !hasVoltageLabel) {
    yOffset = 10;
  } else if (!hasNumberLabel && hasVoltageLabel) {
    yOffset = -10;
  }

  const deltaH = endElev - startElev;
  const arrow = deltaH >= 0 ? "▲" : "▼";
  const s0 = startElev >= 0 ? `+${Math.round(startElev)}` : `${Math.round(startElev)}`;
  const s1 = endElev >= 0 ? `+${Math.round(endElev)}` : `${Math.round(endElev)}`;
  const badgeText = `${arrow} ${gradPercent.toFixed(1)}% (${s0}➔${s1})`;

  let strokeColor = 0x10b981;
  let textColor = 0xecfdf5;
  if (gradPercent > 4.5) {
    strokeColor = 0xef4444;
    textColor = 0xfef2f2;
  } else if (gradPercent > 3.0) {
    strokeColor = 0xf59e0b;
    textColor = 0xfffbeb;
  }

  const text = new Text({
    text: badgeText,
    style: new TextStyle({
      fontFamily: "Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
      fontSize: 9,
      fontWeight: "bold",
      fill: textColor,
      align: "center",
      letterSpacing: 0.3,
    }),
  });
  text.anchor.set(0.5);
  text.position.set(0, yOffset);

  const padX = 4;
  const padY = 2;
  const bgW = text.width + padX * 2;
  const bgH = text.height + padY * 2;

  const bg = new Graphics();
  // Schatten
  bg.roundRect(-bgW / 2 + 0.5, -bgH / 2 + 1 + yOffset, bgW, bgH, 3);
  bg.fill({ color: 0x000000, alpha: 0.55 });
  // Hintergrund-Kapsel
  bg.roundRect(-bgW / 2, -bgH / 2 + yOffset, bgW, bgH, 3);
  bg.fill({ color: 0x0f172a, alpha: 0.92 });
  bg.stroke({ width: 1.1, color: strokeColor, alpha: 0.95 });

  badgeCont.addChild(bg);
  badgeCont.addChild(text);
  container.addChild(badgeCont);
}

function drawSelectionContour(g: Graphics, entry: CatalogEntry, contourWidth = 44) {
  const kind = entry.kind;
  const glowColor = 0x0284c7;
  const glowAlpha = 0.5;
  const contourHalf = contourWidth / 2;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? 188.3;
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });
    return;
  }

  if (kind === "turntable") {
    const r = (entry.radius ?? 189) + 4;
    g.circle(0, 0, r);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9 });
    g.fill({ color: glowColor, alpha: 0.15 });
    return;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const plateH = 420;
    const topY = -55.2;
    g.rect(-4, topY - 4, l + 8, plateH + 8);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      drawArcStroke(g, 0, -r, r, Math.PI / 2, Math.PI / 2 - deg2rad(Math.abs(a)), contourWidth, glowColor, true, glowAlpha);
      return;
    }
    drawArcStroke(g, 0, r, r, -Math.PI / 2, -Math.PI / 2 + deg2rad(a), contourWidth, glowColor, false, glowAlpha);
    return;
  }


  if (kind === "turnout-left") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const end1 = -Math.PI / 2 + deg2rad(a1);
      const end2 = -Math.PI / 2 + deg2rad(a2);
      drawArcStroke(g, 0, r1, r1, -Math.PI / 2, end1, contourWidth, glowColor, false, 0.15);
      drawArcStroke(g, 0, r1, r1 - contourHalf, -Math.PI / 2, end1, 3.5, glowColor, false, 0.9);
      drawArcStroke(g, 0, r1, r1 + contourHalf, -Math.PI / 2, end1, 3.5, glowColor, false, 0.9);
      if (lead > 0) {
        g.rect(-4, -contourHalf, lead + 8, contourWidth);
        g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
        g.fill({ color: glowColor, alpha: 0.15 });
      }
      drawArcStroke(g, lead, r2, r2, -Math.PI / 2, end2, contourWidth, glowColor, false, 0.15);
      drawArcStroke(g, lead, r2, r2 - contourHalf, -Math.PI / 2, end2, 3.5, glowColor, false, 0.9);
      drawArcStroke(g, lead, r2, r2 + contourHalf, -Math.PI / 2, end2, 3.5, glowColor, false, 0.9);
      return;
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), contourWidth, glowColor, false, 0.15);
    drawArcStroke(g, 0, br, br - contourHalf, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), 3.5, glowColor, false, 0.9);
    drawArcStroke(g, 0, br, br + contourHalf, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), 3.5, glowColor, false, 0.9);
    return;
  }

  if (kind === "turnout-right") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const end1 = Math.PI / 2 - deg2rad(a1);
      const end2 = Math.PI / 2 - deg2rad(a2);
      drawArcStroke(g, 0, -r1, r1, Math.PI / 2, end1, contourWidth, glowColor, true, 0.15);
      drawArcStroke(g, 0, -r1, r1 - contourHalf, Math.PI / 2, end1, 3.5, glowColor, true, 0.9);
      drawArcStroke(g, 0, -r1, r1 + contourHalf, Math.PI / 2, end1, 3.5, glowColor, true, 0.9);
      if (lead > 0) {
        g.rect(-4, -contourHalf, lead + 8, contourWidth);
        g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
        g.fill({ color: glowColor, alpha: 0.15 });
      }
      drawArcStroke(g, lead, -r2, r2, Math.PI / 2, end2, contourWidth, glowColor, true, 0.15);
      drawArcStroke(g, lead, -r2, r2 - contourHalf, Math.PI / 2, end2, 3.5, glowColor, true, 0.9);
      drawArcStroke(g, lead, -r2, r2 + contourHalf, Math.PI / 2, end2, 3.5, glowColor, true, 0.9);
      return;
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), contourWidth, glowColor, true, 0.15);
    drawArcStroke(g, 0, -br, br - contourHalf, Math.PI / 2, Math.PI / 2 - deg2rad(ba), 3.5, glowColor, true, 0.9);
    drawArcStroke(g, 0, -br, br + contourHalf, Math.PI / 2, Math.PI / 2 - deg2rad(ba), 3.5, glowColor, true, 0.9);
    return;
  }

  if (kind === "curved-turnout-left") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const end1 = -Math.PI / 2 + deg2rad(a1);
    const end2 = -Math.PI / 2 + deg2rad(a2);
    drawArcStroke(g, 0, r1, r1, -Math.PI / 2, end1, contourWidth, glowColor, false, 0.15);
    drawArcStroke(g, 0, r1, r1 - contourHalf, -Math.PI / 2, end1, 3.5, glowColor, false, 0.9);
    drawArcStroke(g, 0, r1, r1 + contourHalf, -Math.PI / 2, end1, 3.5, glowColor, false, 0.9);
    if (lead > 0) {
      g.rect(-4, -contourHalf, lead + 8, contourWidth);
      g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
      g.fill({ color: glowColor, alpha: 0.15 });
    }
    drawArcStroke(g, lead, r2, r2, -Math.PI / 2, end2, contourWidth, glowColor, false, 0.15);
    drawArcStroke(g, lead, r2, r2 - contourHalf, -Math.PI / 2, end2, 3.5, glowColor, false, 0.9);
    drawArcStroke(g, lead, r2, r2 + contourHalf, -Math.PI / 2, end2, 3.5, glowColor, false, 0.9);
    return;
  }

  if (kind === "curved-turnout-right") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const end1 = Math.PI / 2 - deg2rad(a1);
    const end2 = Math.PI / 2 - deg2rad(a2);
    drawArcStroke(g, 0, -r1, r1, Math.PI / 2, end1, contourWidth, glowColor, true, 0.15);
    drawArcStroke(g, 0, -r1, r1 - contourHalf, Math.PI / 2, end1, 3.5, glowColor, true, 0.9);
    drawArcStroke(g, 0, -r1, r1 + contourHalf, Math.PI / 2, end1, 3.5, glowColor, true, 0.9);
    if (lead > 0) {
      g.rect(-4, -contourHalf, lead + 8, contourWidth);
      g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
      g.fill({ color: glowColor, alpha: 0.15 });
    }
    drawArcStroke(g, lead, -r2, r2, Math.PI / 2, end2, contourWidth, glowColor, true, 0.15);
    drawArcStroke(g, lead, -r2, r2 - contourHalf, Math.PI / 2, end2, 3.5, glowColor, true, 0.9);
    drawArcStroke(g, lead, -r2, r2 + contourHalf, Math.PI / 2, end2, 3.5, glowColor, true, 0.9);
    return;
  }

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });
    // Abzweig links (24224)
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), contourWidth, glowColor, false, 0.15);
    drawArcStroke(g, 0, br, br - contourHalf, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), 3.5, glowColor, false, 0.9);
    drawArcStroke(g, 0, br, br + contourHalf, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), 3.5, glowColor, false, 0.9);
    // Abzweig rechts (24224)
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), contourWidth, glowColor, true, 0.15);
    drawArcStroke(g, 0, -br, br - contourHalf, Math.PI / 2, Math.PI / 2 - deg2rad(ba), 3.5, glowColor, true, 0.9);
    drawArcStroke(g, 0, -br, br + contourHalf, Math.PI / 2, Math.PI / 2 - deg2rad(ba), 3.5, glowColor, true, 0.9);
    return;
  }

  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    // Hauptstrang
    g.rect(-h - 4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });

    // Kreuzender Strang
    const nx = -sin * contourHalf;
    const ny = cos * contourHalf;
    const hCap = h + 4;
    g.moveTo(-hCap * cos, -hCap * sin).lineTo(hCap * cos, hCap * sin);
    g.stroke({ width: contourWidth, color: glowColor, alpha: 0.15, cap: "butt", join: "miter" });

    g.moveTo(-hCap * cos + nx, -hCap * sin + ny).lineTo(hCap * cos + nx, hCap * sin + ny);
    g.moveTo(-hCap * cos - nx, -hCap * sin - ny).lineTo(hCap * cos - nx, hCap * sin - ny);
    g.moveTo(-hCap * cos + nx, -hCap * sin + ny).lineTo(-hCap * cos - nx, -hCap * sin - ny);
    g.moveTo(hCap * cos + nx, hCap * sin + ny).lineTo(hCap * cos - nx, hCap * sin - ny);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    return;
  }

  if (kind === "dkw") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    // 1. Hauptstrang (horizontal)
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });

    // 2. Kreuzender Strang
    const nx = -sin * contourHalf;
    const ny = cos * contourHalf;
    const hCap = h + 4;
    g.moveTo(cx - hCap * cos, -hCap * sin).lineTo(cx + hCap * cos, hCap * sin);
    g.stroke({ width: contourWidth, color: glowColor, alpha: 0.15, cap: "butt", join: "miter" });

    g.moveTo(cx - hCap * cos + nx, -hCap * sin + ny).lineTo(cx + hCap * cos + nx, hCap * sin + ny);
    g.moveTo(cx - hCap * cos - nx, -hCap * sin - ny).lineTo(cx + hCap * cos - nx, hCap * sin - ny);
    g.moveTo(cx - hCap * cos + nx, -hCap * sin + ny).lineTo(cx - hCap * cos - nx, -hCap * sin - ny);
    g.moveTo(cx + hCap * cos + nx, hCap * sin + ny).lineTo(cx + hCap * cos - nx, hCap * sin - ny);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });

    // 3. Bogen 1 (Abzweig nach rechts/unten)
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), contourWidth, glowColor, false, 0.15);
    drawArcStroke(g, 0, br, br + contourHalf, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), 3.5, glowColor, false, 0.9);

    // 4. Bogen 2 (Abzweig nach links/oben)
    drawArcStroke(g, l, -br, br, Math.PI / 2, Math.PI / 2 + deg2rad(ba), contourWidth, glowColor, false, 0.15);
    drawArcStroke(g, l, -br, br + contourHalf, Math.PI / 2, Math.PI / 2 + deg2rad(ba), 3.5, glowColor, false, 0.9);
    return;
  }

  // Fallback
  drawTrackCenterline(g, entry, glowColor, contourWidth, glowAlpha);
}

function drawSelectionGizmo(container: Container, entry: CatalogEntry) {
  const gizmo = new Container();
  gizmo.label = "selection-gizmo";

  const g = new Graphics();
  const center = getPieceLocalCenter(entry);
  const stemOffset = entry.kind === "turntable" ? 220 : entry.kind === "transfer-table" ? 240 : entry.kind === "dkw" ? 62 : entry.isRoad ? 50 : 48;
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
    style: new TextStyle({
      fontSize: 16,
      fill: 0xffffff,
      fontWeight: "bold",
    }),
  });
  rotText.anchor.set(0.5);
  rotText.position.set(handlePos.x, handlePos.y);
  rotText.rotation = -container.rotation;

  gizmo.addChild(g);
  gizmo.addChild(rotText);
  container.addChild(gizmo);
}

function drawBallast(g: Graphics, entry: CatalogEntry, color: number, ballastWidth = 36) {
  const kind = entry.kind;
  const ballastHalf = ballastWidth / 2;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? 188.3;
    g.rect(0, -ballastHalf, l, ballastWidth);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    return;
  }

  if (kind === "turntable") {
    const outerR = entry.radius ?? 189;
    const pitR = 139;

    // Äußerer Schotterring / Bettungskranz (R=139 bis R=189)
    g.circle(0, 0, outerR);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1.5, color: 0x6c655b, alpha: 0.8 });

    // Abgesetzter Grubenrand / Mauerwerk
    g.circle(0, 0, pitR + 3);
    g.stroke({ width: 3, color: 0x475569, alpha: 0.95 });

    // Abgesenkter Grubenboden
    g.circle(0, 0, pitR);
    g.fill({ color: 0x1e293b, alpha: 0.98 });

    // Kreisförmige Laufschiene für die Brücke (Radius 126 mm)
    g.circle(0, 0, 126);
    g.stroke({ width: 1.6, color: 0x94a3b8, alpha: 0.85 });

    // Mittiger Drehzapfen / Königsstuhl
    g.circle(0, 0, 14);
    g.fill({ color: 0x334155 });
    g.circle(0, 0, 6);
    g.fill({ color: 0x64748b });
    return;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const plateH = 420;
    const topY = -55.2;
    const pitLeft = 36;
    const pitRight = l - 36;
    const pitWidth = pitRight - pitLeft;

    // 1. Grundplatte
    g.rect(0, topY, l, plateH);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1.5, color: 0x6c655b, alpha: 0.8 });

    // 2. Vertiefte Grube
    g.rect(pitLeft, topY + 4, pitWidth, plateH - 8);
    g.fill({ color: 0x1e293b, alpha: 0.98 });
    g.stroke({ width: 2, color: 0x475569, alpha: 0.95 });

    // 3. Quer-Laufschienen für die Bühne (quer zur Gleisrichtung, parallel zu Y)
    for (const qx of [80, 180, 280]) {
      g.moveTo(qx, topY + 6).lineTo(qx, topY + plateH - 6);
      g.stroke({ width: 1.6, color: 0x94a3b8, alpha: 0.85 });
    }
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      drawArcStroke(g, 0, -r, r, Math.PI / 2, Math.PI / 2 - deg2rad(Math.abs(a)), ballastWidth, color, true);
      return;
    }
    drawArcStroke(g, 0, r, r, -Math.PI / 2, -Math.PI / 2 + deg2rad(a), ballastWidth, color);
    return;
  }


  if (kind === "turnout-left") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      drawArcStroke(g, 0, r1, r1, -Math.PI / 2, -Math.PI / 2 + deg2rad(a1), ballastWidth, color);
      if (lead > 0) {
        g.rect(0, -ballastHalf, lead, ballastWidth);
        g.fill({ color, alpha: 0.95 });
        g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
      }
      drawArcStroke(g, lead, r2, r2, -Math.PI / 2, -Math.PI / 2 + deg2rad(a2), ballastWidth, color);
      return;
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(0, -ballastHalf, l, ballastWidth);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), ballastWidth, color);
    return;
  }

  if (kind === "turnout-right") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      drawArcStroke(g, 0, -r1, r1, Math.PI / 2, Math.PI / 2 - deg2rad(a1), ballastWidth, color, true);
      if (lead > 0) {
        g.rect(0, -ballastHalf, lead, ballastWidth);
        g.fill({ color, alpha: 0.95 });
        g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
      }
      drawArcStroke(g, lead, -r2, r2, Math.PI / 2, Math.PI / 2 - deg2rad(a2), ballastWidth, color, true);
      return;
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(0, -ballastHalf, l, ballastWidth);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), ballastWidth, color, true);
    return;
  }

  if (kind === "curved-turnout-left") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    drawArcStroke(g, 0, r1, r1, -Math.PI / 2, -Math.PI / 2 + deg2rad(a1), ballastWidth, color);
    if (lead > 0) {
      g.rect(0, -ballastHalf, lead, ballastWidth);
      g.fill({ color, alpha: 0.95 });
      g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    }
    drawArcStroke(g, lead, r2, r2, -Math.PI / 2, -Math.PI / 2 + deg2rad(a2), ballastWidth, color);
    return;
  }

  if (kind === "curved-turnout-right") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    drawArcStroke(g, 0, -r1, r1, Math.PI / 2, Math.PI / 2 - deg2rad(a1), ballastWidth, color, true);
    if (lead > 0) {
      g.rect(0, -ballastHalf, lead, ballastWidth);
      g.fill({ color, alpha: 0.95 });
      g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    }
    drawArcStroke(g, lead, -r2, r2, Math.PI / 2, Math.PI / 2 - deg2rad(a2), ballastWidth, color, true);
    return;
  }

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(0, -ballastHalf, l, ballastWidth);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), ballastWidth, color);
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), ballastWidth, color, true);
    return;
  }

  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    g.rect(-h, -ballastHalf, l, ballastWidth);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    g.moveTo(-h * Math.cos(rad), -h * Math.sin(rad));
    g.lineTo(h * Math.cos(rad), h * Math.sin(rad));
    g.stroke({ width: ballastWidth, color, alpha: 0.95, cap: "butt", join: "miter" });
    return;
  }

  if (kind === "dkw") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    g.rect(0, -ballastHalf, l, ballastWidth);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    g.moveTo(cx - h * cos, -h * sin);
    g.lineTo(cx + h * cos, h * sin);
    g.stroke({ width: ballastWidth, color, alpha: 0.95, cap: "butt", join: "miter" });
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), ballastWidth, color);
    drawArcStroke(g, l, -br, br, Math.PI / 2, Math.PI / 2 + deg2rad(ba), ballastWidth, color, false);
    return;
  }

  // Fallback
  g.rect(0, -ballastHalf, 100, ballastWidth);
  g.fill({ color });
}

function drawTies(g: Graphics, entry: CatalogEntry, color: number, tieLenHalf = 13, tieWidth = 2.6, tieSpacing = 6.5) {
  const kind = entry.kind;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? 188.3;
    for (let x = 4; x <= l - 4; x += tieSpacing) {
      g.moveTo(x, -tieLenHalf);
      g.lineTo(x, tieLenHalf);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "turntable") {
    // 1. Schwellen auf der Drehscheibenbühne (Länge 263 mm, von -131.5 bis +131.5)
    const bridgeHalf = 131.5;
    for (let x = -bridgeHalf + 4; x <= bridgeHalf - 4; x += tieSpacing) {
      g.moveTo(x, -tieLenHalf);
      g.lineTo(x, tieLenHalf);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });

    // 2. Schwellen an den 30 Gleisabgängen am Außenring
    for (let a = 0; a < 360; a += 12) {
      const rad = deg2rad(a);
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);
      for (const r of [152, 162, 172, 182]) {
        const cx = r * cos;
        const cy = r * sin;
        const nx = -sin * tieLenHalf * 0.9;
        const ny = cos * tieLenHalf * 0.9;
        g.moveTo(cx - nx, cy - ny);
        g.lineTo(cx + nx, cy + ny);
        g.stroke({ width: tieWidth, color, cap: "round" });
      }
    }
    return;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const trackYs = [0, 77.4, 154.8, 232.2, 309.6];

    // Schwellen an den 10 Anschluss-Stümpfen links und rechts
    for (const y of trackYs) {
      // Links (x: 4 .. 32)
      for (let x = 4; x <= 32; x += tieSpacing) {
        g.moveTo(x, y - tieLenHalf);
        g.lineTo(x, y + tieLenHalf);
      }
      // Rechts (x: l - 32 .. l - 4)
      for (let x = l - 32; x <= l - 4; x += tieSpacing) {
        g.moveTo(x, y - tieLenHalf);
        g.lineTo(x, y + tieLenHalf);
      }
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });

    // Schwellen auf der Schiebebühne (Mittelstellung y = 154.8)
    const bridgeY = 154.8;
    for (let x = 40; x <= l - 40; x += tieSpacing) {
      g.moveTo(x, bridgeY - tieLenHalf);
      g.lineTo(x, bridgeY + tieLenHalf);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const absA = Math.abs(a);
    const totalRad = deg2rad(absA);
    const stepRad = deg2rad(1.2 * (tieSpacing / 6.5));
    if (a < 0) {
      for (let ang = stepRad / 2; ang < totalRad; ang += stepRad) {
        const curAng = Math.PI / 2 - ang;
        const cos = Math.cos(curAng);
        const sin = Math.sin(curAng);
        g.moveTo((r - tieLenHalf) * cos, -r + (r - tieLenHalf) * sin);
        g.lineTo((r + tieLenHalf) * cos, -r + (r + tieLenHalf) * sin);
      }
    } else {
      for (let ang = stepRad / 2; ang < totalRad; ang += stepRad) {
        const curAng = -Math.PI / 2 + ang;
        const cos = Math.cos(curAng);
        const sin = Math.sin(curAng);
        g.moveTo((r - tieLenHalf) * cos, r + (r - tieLenHalf) * sin);
        g.lineTo((r + tieLenHalf) * cos, r + (r + tieLenHalf) * sin);
      }
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "curved-turnout-left" || (kind === "turnout-left" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;

    const totalRad1 = deg2rad(a1);
    const stepRad1 = deg2rad(1.2 * (tieSpacing / 6.5));
    for (let ang = stepRad1 / 2; ang < totalRad1; ang += stepRad1) {
      const curAng = -Math.PI / 2 + ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((r1 - tieLenHalf) * cos, r1 + (r1 - tieLenHalf) * sin);
      g.lineTo((r1 + tieLenHalf) * cos, r1 + (r1 + tieLenHalf) * sin);
    }

    if (lead > 0) {
      for (let x = 4; x <= lead - 2; x += tieSpacing) {
        g.moveTo(x, -tieLenHalf);
        g.lineTo(x, tieLenHalf);
      }
    }

    const totalRad2 = deg2rad(a2);
    const stepRad2 = deg2rad(1.2 * (tieSpacing / 6.5));
    const startAng2 = lead > 0 ? stepRad2 / 2 : stepRad2 * 3;
    for (let ang = startAng2; ang < totalRad2; ang += stepRad2) {
      const curAng = -Math.PI / 2 + ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo(lead + (r2 - tieLenHalf) * cos, r2 + (r2 - tieLenHalf) * sin);
      g.lineTo(lead + (r2 + tieLenHalf) * cos, r2 + (r2 + tieLenHalf) * sin);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;

    const totalRad1 = deg2rad(a1);
    const stepRad1 = deg2rad(1.2 * (tieSpacing / 6.5));
    for (let ang = stepRad1 / 2; ang < totalRad1; ang += stepRad1) {
      const curAng = Math.PI / 2 - ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((r1 - tieLenHalf) * cos, -r1 + (r1 - tieLenHalf) * sin);
      g.lineTo((r1 + tieLenHalf) * cos, -r1 + (r1 + tieLenHalf) * sin);
    }

    if (lead > 0) {
      for (let x = 4; x <= lead - 2; x += tieSpacing) {
        g.moveTo(x, -tieLenHalf);
        g.lineTo(x, tieLenHalf);
      }
    }

    const totalRad2 = deg2rad(a2);
    const stepRad2 = deg2rad(1.2 * (tieSpacing / 6.5));
    const startAng2 = lead > 0 ? stepRad2 / 2 : stepRad2 * 3;
    for (let ang = startAng2; ang < totalRad2; ang += stepRad2) {
      const curAng = Math.PI / 2 - ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo(lead + (r2 - tieLenHalf) * cos, -r2 + (r2 - tieLenHalf) * sin);
      g.lineTo(lead + (r2 + tieLenHalf) * cos, -r2 + (r2 + tieLenHalf) * sin);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "turnout-left") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;

    // 1. Gerade Schwellen (24188)
    for (let x = 4; x <= l - 4; x += tieSpacing) {
      g.moveTo(x, -tieLenHalf);
      g.lineTo(x, tieLenHalf);
    }

    // 2. Schwellen für Abzweigbogen R2 (24224)
    const totalRad = deg2rad(ba);
    const stepRad = deg2rad(1.2 * (tieSpacing / 6.5));
    for (let ang = stepRad * 2.5; ang < totalRad; ang += stepRad) {
      const curAng = -Math.PI / 2 + ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((br - tieLenHalf) * cos, br + (br - tieLenHalf) * sin);
      g.lineTo((br + tieLenHalf) * cos, br + (br + tieLenHalf) * sin);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "turnout-right") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;

    // 1. Gerade Schwellen (24188)
    for (let x = 4; x <= l - 4; x += tieSpacing) {
      g.moveTo(x, -tieLenHalf);
      g.lineTo(x, tieLenHalf);
    }

    // 2. Schwellen für Abzweigbogen R2 (24224)
    const totalRad = deg2rad(ba);
    const stepRad = deg2rad(1.2 * (tieSpacing / 6.5));
    for (let ang = stepRad * 2.5; ang < totalRad; ang += stepRad) {
      const curAng = Math.PI / 2 - ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((br - tieLenHalf) * cos, -br + (br - tieLenHalf) * sin);
      g.lineTo((br + tieLenHalf) * cos, -br + (br + tieLenHalf) * sin);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;

    // 1. Gerade Schwellen (24188)
    for (let x = 4; x <= l - 4; x += tieSpacing) {
      g.moveTo(x, -tieLenHalf);
      g.lineTo(x, tieLenHalf);
    }

    // 2. Schwellen für Abzweigbogen R2 links (24224)
    const totalRad = deg2rad(ba);
    const stepRad = deg2rad(1.2 * (tieSpacing / 6.5));
    for (let ang = stepRad * 2.5; ang < totalRad; ang += stepRad) {
      const curAng = -Math.PI / 2 + ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((br - tieLenHalf) * cos, br + (br - tieLenHalf) * sin);
      g.lineTo((br + tieLenHalf) * cos, br + (br + tieLenHalf) * sin);
    }

    // 3. Schwellen für Abzweigbogen R2 rechts (24224)
    for (let ang = stepRad * 2.5; ang < totalRad; ang += stepRad) {
      const curAng = Math.PI / 2 - ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((br - tieLenHalf) * cos, -br + (br - tieLenHalf) * sin);
      g.lineTo((br + tieLenHalf) * cos, -br + (br + tieLenHalf) * sin);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    for (let x = -h + 4; x <= h - 4; x += tieSpacing) {
      g.moveTo(x, -tieLenHalf);
      g.lineTo(x, tieLenHalf);
    }
    for (let s = -h + 4; s <= h - 4; s += tieSpacing) {
      const cx = s * cos;
      const cy = s * sin;
      g.moveTo(cx - sin * tieLenHalf, cy + cos * tieLenHalf);
      g.lineTo(cx + sin * tieLenHalf, cy - cos * tieLenHalf);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (kind === "dkw") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    // Hauptstrang
    for (let x = 4; x <= l - 4; x += tieSpacing) {
      g.moveTo(x, -tieLenHalf);
      g.lineTo(x, tieLenHalf);
    }
    // Kreuzender Strang
    for (let s = -h + 4; s <= h - 4; s += tieSpacing) {
      const px = cx + s * cos;
      const py = s * sin;
      g.moveTo(px - sin * tieLenHalf, py + cos * tieLenHalf);
      g.lineTo(px + sin * tieLenHalf, py - cos * tieLenHalf);
    }
    // Bogen 1 (Abzweig nach rechts/unten)
    const totalRad1 = deg2rad(ba);
    const stepRad = deg2rad(2.5 * (tieSpacing / 6.5));
    for (let ang = stepRad / 2; ang < totalRad1; ang += stepRad) {
      const curAng = -Math.PI / 2 + ang;
      const c = Math.cos(curAng);
      const s = Math.sin(curAng);
      g.moveTo(0 + (br - tieLenHalf) * c, br + (br - tieLenHalf) * s);
      g.lineTo(0 + (br + tieLenHalf) * c, br + (br + tieLenHalf) * s);
    }
    // Bogen 2 (Abzweig nach links/oben)
    for (let ang = stepRad / 2; ang < totalRad1; ang += stepRad) {
      const curAng = Math.PI / 2 + ang;
      const c = Math.cos(curAng);
      const s = Math.sin(curAng);
      g.moveTo(l + (br - tieLenHalf) * c, -br + (br - tieLenHalf) * s);
      g.lineTo(l + (br + tieLenHalf) * c, -br + (br + tieLenHalf) * s);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  for (let x = 5; x <= 95; x += tieSpacing) {
    g.moveTo(x, -tieLenHalf);
    g.lineTo(x, tieLenHalf);
  }
  g.stroke({ width: tieWidth, color, cap: "butt" });
}

function drawRails(g: Graphics, entry: CatalogEntry, color: number, gaugeHalf: number) {
  const kind = entry.kind;
  const railWidth = Math.max(1.1, gaugeHalf * 0.22);

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? 188.3;
    g.moveTo(0, -gaugeHalf).lineTo(l, -gaugeHalf);
    g.moveTo(0, gaugeHalf).lineTo(l, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // Prellbock-Gleis: Pufferbohle und Puffer am Gleisende (x = l)
    if (kind === "accessory") {
      const bohleW = 4.5;
      const bohleH = gaugeHalf * 2 + 8;
      const bohleX = l - 6;
      // Bohle
      g.rect(bohleX, -bohleH / 2, bohleW, bohleH);
      g.fill({ color: 0x334155 });
      g.stroke({ width: 0.8, color: 0x0f172a, join: "miter" });

      // Linker und rechter Pufferteller (eckig / rechtwinklig)
      const pLen = 3;
      const pSize = 4.5;
      const pOff = gaugeHalf * 0.75;
      g.rect(l - 1.5, -pOff - pSize / 2, pLen, pSize);
      g.rect(l - 1.5, pOff - pSize / 2, pLen, pSize);
      g.fill({ color: 0x1e293b });
      g.stroke({ width: 0.6, color: 0x0f172a, join: "miter" });

      // Rotes Sh0 Signal-Schild in der Mitte
      g.rect(bohleX - 0.5, -2.5, 2, 5);
      g.fill({ color: 0xef4444 });
    }
    return;
  }

  if (kind === "turntable") {
    const bridgeHalf = 131.5;

    // 1. Schienenstränge auf der Drehscheibenbühne
    g.moveTo(-bridgeHalf, -gaugeHalf).lineTo(bridgeHalf, -gaugeHalf);
    g.moveTo(-bridgeHalf, gaugeHalf).lineTo(bridgeHalf, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // 2. Bühnenkanten / Träger / Geländer
    g.moveTo(-bridgeHalf, -17).lineTo(bridgeHalf, -17);
    g.moveTo(-bridgeHalf, 17).lineTo(bridgeHalf, 17);
    g.stroke({ width: 1.4, color: 0x475569 });

    // Wärterhäuschen an der Bühne (Bedienstand)
    g.roundRect(25, 17, 32, 16, 2);
    g.fill({ color: 0x0f172a });
    g.roundRect(25, 17, 32, 16, 2);
    g.stroke({ width: 1.2, color: 0x38bdf8 });

    // Dach des Wärterhäuschens
    g.rect(23, 16, 36, 4);
    g.fill({ color: 0x334155 });

    // 3. Schienen an den 30 Gleisabgängen am Außenring
    for (let a = 0; a < 360; a += 12) {
      const rad = deg2rad(a);
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);
      const px = -sin * gaugeHalf;
      const py = cos * gaugeHalf;

      g.moveTo(142 * cos + px, 142 * sin + py);
      g.lineTo(189 * cos + px, 189 * sin + py);
      g.stroke({ width: railWidth, color });

      g.moveTo(142 * cos - px, 142 * sin - py);
      g.lineTo(189 * cos - px, 189 * sin - py);
      g.stroke({ width: railWidth, color });
    }
    return;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const trackYs = [0, 77.4, 154.8, 232.2, 309.6];
    const bridgeY = 154.8;

    // 1. Schienen an den 10 Anschlussstutzen
    for (const y of trackYs) {
      // Links
      g.moveTo(0, y - gaugeHalf).lineTo(36, y - gaugeHalf);
      g.moveTo(0, y + gaugeHalf).lineTo(36, y + gaugeHalf);
      // Rechts
      g.moveTo(l - 36, y - gaugeHalf).lineTo(l, y - gaugeHalf);
      g.moveTo(l - 36, y + gaugeHalf).lineTo(l, y + gaugeHalf);
    }
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // 2. Schienen auf der Schiebebühne
    g.moveTo(36, bridgeY - gaugeHalf).lineTo(l - 36, bridgeY - gaugeHalf);
    g.moveTo(36, bridgeY + gaugeHalf).lineTo(l - 36, bridgeY + gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // 3. Bühnenkanten / Träger / Geländer
    g.moveTo(36, bridgeY - 18).lineTo(l - 36, bridgeY - 18);
    g.moveTo(36, bridgeY + 18).lineTo(l - 36, bridgeY + 18);
    g.stroke({ width: 1.4, color: 0x475569 });

    // Wärterhäuschen an der Schiebebühne (Bedienstand)
    g.roundRect(50, bridgeY + 18, 34, 16, 2);
    g.fill({ color: 0x0f172a });
    g.roundRect(50, bridgeY + 18, 34, 16, 2);
    g.stroke({ width: 1.2, color: 0x38bdf8 });

    // Dach des Wärterhäuschens
    g.rect(48, bridgeY + 17, 38, 4);
    g.fill({ color: 0x334155 });
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      const end = Math.PI / 2 - deg2rad(Math.abs(a));
      drawArcStroke(g, 0, -r, r - gaugeHalf, Math.PI / 2, end, railWidth, color, true);
      drawArcStroke(g, 0, -r, r + gaugeHalf, Math.PI / 2, end, railWidth, color, true);
      return;
    }
    const end = -Math.PI / 2 + deg2rad(a);
    drawArcStroke(g, 0, r, r - gaugeHalf, -Math.PI / 2, end, railWidth, color);
    drawArcStroke(g, 0, r, r + gaugeHalf, -Math.PI / 2, end, railWidth, color);
    return;
  }


  if (kind === "turnout-left") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const end1 = -Math.PI / 2 + deg2rad(a1);
      const end2 = -Math.PI / 2 + deg2rad(a2);
      drawArcStroke(g, 0, r1, r1 - gaugeHalf, -Math.PI / 2, end1, railWidth, color);
      drawArcStroke(g, 0, r1, r1 + gaugeHalf, -Math.PI / 2, end1, railWidth, color);
      if (lead > 0) {
        g.moveTo(0, -gaugeHalf).lineTo(lead, -gaugeHalf);
        g.moveTo(0, gaugeHalf).lineTo(lead, gaugeHalf);
        g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
      }
      drawArcStroke(g, lead, r2, r2 - gaugeHalf, -Math.PI / 2, end2, railWidth, color);
      drawArcStroke(g, lead, r2, r2 + gaugeHalf, -Math.PI / 2, end2, railWidth, color);
      return;
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.moveTo(0, -gaugeHalf).lineTo(l, -gaugeHalf);
    g.moveTo(0, gaugeHalf).lineTo(l, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
    const end = -Math.PI / 2 + deg2rad(ba);
    drawArcStroke(g, 0, br, br - gaugeHalf, -Math.PI / 2, end, railWidth, color);
    drawArcStroke(g, 0, br, br + gaugeHalf, -Math.PI / 2, end, railWidth, color);
    return;
  }

  if (kind === "turnout-right") {
    if (entry.radius && !entry.length) {
      const r1 = entry.radius;
      const a1 = entry.angle ?? 30;
      const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
      const a2 = entry.branchAngle ?? 30;
      const end1 = Math.PI / 2 - deg2rad(a1);
      const end2 = Math.PI / 2 - deg2rad(a2);
      drawArcStroke(g, 0, -r1, r1 - gaugeHalf, Math.PI / 2, end1, railWidth, color, true);
      drawArcStroke(g, 0, -r1, r1 + gaugeHalf, Math.PI / 2, end1, railWidth, color, true);
      if (lead > 0) {
        g.moveTo(0, -gaugeHalf).lineTo(lead, -gaugeHalf);
        g.moveTo(0, gaugeHalf).lineTo(lead, gaugeHalf);
        g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
      }
      drawArcStroke(g, lead, -r2, r2 - gaugeHalf, Math.PI / 2, end2, railWidth, color, true);
      drawArcStroke(g, lead, -r2, r2 + gaugeHalf, Math.PI / 2, end2, railWidth, color, true);
      return;
    }
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.moveTo(0, -gaugeHalf).lineTo(l, -gaugeHalf);
    g.moveTo(0, gaugeHalf).lineTo(l, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
    const end = Math.PI / 2 - deg2rad(ba);
    drawArcStroke(g, 0, -br, br - gaugeHalf, Math.PI / 2, end, railWidth, color, true);
    drawArcStroke(g, 0, -br, br + gaugeHalf, Math.PI / 2, end, railWidth, color, true);
    return;
  }

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    // Stammgleis (24188)
    g.moveTo(0, -gaugeHalf).lineTo(l, -gaugeHalf);
    g.moveTo(0, gaugeHalf).lineTo(l, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // Abzweigbogen R2 links (24224)
    const endL = -Math.PI / 2 + deg2rad(ba);
    drawArcStroke(g, 0, br, br - gaugeHalf, -Math.PI / 2, endL, railWidth, color);
    drawArcStroke(g, 0, br, br + gaugeHalf, -Math.PI / 2, endL, railWidth, color);

    // Abzweigbogen R2 rechts (24224)
    const endR = Math.PI / 2 - deg2rad(ba);
    drawArcStroke(g, 0, -br, br - gaugeHalf, Math.PI / 2, endR, railWidth, color, true);
    drawArcStroke(g, 0, -br, br + gaugeHalf, Math.PI / 2, endR, railWidth, color, true);
    return;
  }

  if (kind === "curved-turnout-left") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const end1 = -Math.PI / 2 + deg2rad(a1);
    const end2 = -Math.PI / 2 + deg2rad(a2);
    drawArcStroke(g, 0, r1, r1 - gaugeHalf, -Math.PI / 2, end1, railWidth, color);
    drawArcStroke(g, 0, r1, r1 + gaugeHalf, -Math.PI / 2, end1, railWidth, color);
    if (lead > 0) {
      g.moveTo(0, -gaugeHalf).lineTo(lead, -gaugeHalf);
      g.moveTo(0, gaugeHalf).lineTo(lead, gaugeHalf);
      g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
    }
    drawArcStroke(g, lead, r2, r2 - gaugeHalf, -Math.PI / 2, end2, railWidth, color);
    drawArcStroke(g, lead, r2, r2 + gaugeHalf, -Math.PI / 2, end2, railWidth, color);
    return;
  }

  if (kind === "curved-turnout-right") {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const end1 = Math.PI / 2 - deg2rad(a1);
    const end2 = Math.PI / 2 - deg2rad(a2);
    drawArcStroke(g, 0, -r1, r1 - gaugeHalf, Math.PI / 2, end1, railWidth, color, true);
    drawArcStroke(g, 0, -r1, r1 + gaugeHalf, Math.PI / 2, end1, railWidth, color, true);
    if (lead > 0) {
      g.moveTo(0, -gaugeHalf).lineTo(lead, -gaugeHalf);
      g.moveTo(0, gaugeHalf).lineTo(lead, gaugeHalf);
      g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
    }
    drawArcStroke(g, lead, -r2, r2 - gaugeHalf, Math.PI / 2, end2, railWidth, color, true);
    drawArcStroke(g, lead, -r2, r2 + gaugeHalf, Math.PI / 2, end2, railWidth, color, true);
    return;
  }

  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    // Track 1
    g.moveTo(-h, -gaugeHalf).lineTo(h, -gaugeHalf);
    g.moveTo(-h, gaugeHalf).lineTo(h, gaugeHalf);
    // Track 2
    const nx = -sin * gaugeHalf;
    const ny = cos * gaugeHalf;
    g.moveTo(-h * cos + nx, -h * sin + ny).lineTo(h * cos + nx, h * sin + ny);
    g.moveTo(-h * cos - nx, -h * sin - ny).lineTo(h * cos - nx, h * sin - ny);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
    return;
  }

  if (kind === "dkw") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    // Hauptstrang
    g.moveTo(0, -gaugeHalf).lineTo(l, -gaugeHalf);
    g.moveTo(0, gaugeHalf).lineTo(l, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // Kreuzender Strang
    const nx = -sin * gaugeHalf;
    const ny = cos * gaugeHalf;
    g.moveTo(cx - h * cos + nx, -h * sin + ny).lineTo(cx + h * cos + nx, h * sin + ny);
    g.moveTo(cx - h * cos - nx, -h * sin - ny).lineTo(cx + h * cos - nx, h * sin - ny);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // Zwei Abzweigbögen
    const end1 = -Math.PI / 2 + deg2rad(ba);
    drawArcStroke(g, 0, br, br - gaugeHalf, -Math.PI / 2, end1, railWidth, color);
    drawArcStroke(g, 0, br, br + gaugeHalf, -Math.PI / 2, end1, railWidth, color);

    const end2 = Math.PI / 2 + deg2rad(ba);
    drawArcStroke(g, l, -br, br - gaugeHalf, Math.PI / 2, end2, railWidth, color, false);
    drawArcStroke(g, l, -br, br + gaugeHalf, Math.PI / 2, end2, railWidth, color, false);
    return;
  }

  g.moveTo(0, -gaugeHalf).lineTo(100, -gaugeHalf);
  g.moveTo(0, gaugeHalf).lineTo(100, gaugeHalf);
  g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });
}

function drawTrackCenterline(g: Graphics, entry: CatalogEntry, color: number, width: number, alpha: number) {
  const kind = entry.kind;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? 188.3;
    g.moveTo(0, 0).lineTo(l, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    return;
  }

  if (kind === "turntable") {
    g.moveTo(-131.5, 0).lineTo(131.5, 0);
    g.stroke({ width, color, alpha, cap: "butt" });
    for (let a = 0; a < 360; a += 12) {
      const rad = deg2rad(a);
      g.moveTo(140 * Math.cos(rad), 140 * Math.sin(rad));
      g.lineTo(189 * Math.cos(rad), 189 * Math.sin(rad));
      g.stroke({ width: width * 0.8, color, alpha: alpha * 0.7 });
    }
    return;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const trackYs = [0, 77.4, 154.8, 232.2, 309.6];
    for (const y of trackYs) {
      g.moveTo(0, y).lineTo(36, y);
      g.moveTo(l - 36, y).lineTo(l, y);
      g.stroke({ width: width * 0.8, color, alpha: alpha * 0.7 });
    }
    // Mittellinie der Bühne
    g.moveTo(36, 154.8).lineTo(l - 36, 154.8);
    g.stroke({ width, color, alpha, cap: "butt" });
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      drawArcStroke(g, 0, -r, r, Math.PI / 2, Math.PI / 2 - deg2rad(Math.abs(a)), width, color, true, alpha);
      return;
    }
    drawArcStroke(g, 0, r, r, -Math.PI / 2, -Math.PI / 2 + deg2rad(a), width, color, false, alpha);
    return;
  }

  if (kind === "curved-turnout-left" || (kind === "turnout-left" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    drawArcStroke(g, 0, r1, r1, -Math.PI / 2, -Math.PI / 2 + deg2rad(a1), width, color, false, alpha);
    if (lead > 0) {
      g.moveTo(0, 0).lineTo(lead, 0);
      g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    }
    drawArcStroke(g, lead, r2, r2, -Math.PI / 2, -Math.PI / 2 + deg2rad(a2), width, color, false, alpha);
    return;
  }

  if (kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    drawArcStroke(g, 0, -r1, r1, Math.PI / 2, Math.PI / 2 - deg2rad(a1), width, color, true, alpha);
    if (lead > 0) {
      g.moveTo(0, 0).lineTo(lead, 0);
      g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    }
    drawArcStroke(g, lead, -r2, r2, Math.PI / 2, Math.PI / 2 - deg2rad(a2), width, color, true, alpha);
    return;
  }


  if (kind === "turnout-left") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.moveTo(0, 0).lineTo(l, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), width, color, false, alpha);
    return;
  }

  if (kind === "turnout-right") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.moveTo(0, 0).lineTo(l, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), width, color, true, alpha);
    return;
  }

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.moveTo(0, 0).lineTo(l, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), width, color, false, alpha);
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), width, color, true, alpha);
    return;
  }

  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    g.moveTo(-h, 0).lineTo(h, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    g.moveTo(-h * Math.cos(rad), -h * Math.sin(rad)).lineTo(h * Math.cos(rad), h * Math.sin(rad));
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    return;
  }

  if (kind === "dkw") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);

    // 1. Hauptstrang
    g.moveTo(0, 0).lineTo(l, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });

    // 2. Kreuzender Strang
    g.moveTo(cx - h * cos, -h * sin).lineTo(cx + h * cos, h * sin);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });

    // 3. Zwei Abzweigbögen
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), width, color, false, alpha);
    drawArcStroke(g, l, -br, br, Math.PI / 2, Math.PI / 2 + deg2rad(ba), width, color, false, alpha);
    return;
  }

  g.moveTo(0, 0).lineTo(100, 0);
  g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
}

function drawArcStroke(
  g: Graphics,
  cx: number,
  cy: number,
  radius: number,
  startAngle: number,
  endAngle: number,
  strokeWidth: number,
  color: number,
  counterclockwise = false,
  alpha = 1,
) {
  const sx = cx + radius * Math.cos(startAngle);
  const sy = cy + radius * Math.sin(startAngle);
  g.moveTo(sx, sy);
  g.arc(cx, cy, radius, startAngle, endAngle, counterclockwise);
  g.stroke({ width: strokeWidth, color, alpha, cap: "butt", join: "miter" });
}

function drawPowerFeederMarker(parent: Container) {
  const marker = new Container();
  marker.position.set(20, -25);
  marker.rotation = -parent.rotation;

  const bg = new Graphics();
  bg.circle(0, 0, 11);
  bg.fill({ color: 0xeab308, alpha: 0.95 });
  bg.stroke({ width: 2, color: 0xffffff });
  marker.addChild(bg);

  const txt = new Text({
    text: "⚡",
    style: new TextStyle({ fontSize: 13, fill: 0x1e293b, fontWeight: "bold" }),
  });
  txt.anchor.set(0.5);
  marker.addChild(txt);

  parent.addChild(marker);
}

function drawIsolatorMarker(parent: Container, entry: CatalogEntry, isoIdx = 0) {
  const locals = getLocalConnectors(entry);
  const lc = locals[isoIdx] ?? locals[0] ?? { x: 0, y: 0, angle: 180 };

  const connAngleRad = deg2rad(lc.angle);
  const inwardX = -Math.cos(connAngleRad);
  const inwardY = -Math.sin(connAngleRad);
  const perpX = -Math.sin(connAngleRad);
  const perpY = Math.cos(connAngleRad);

  // Position barrier 5mm inward from connector edge
  const cx = lc.x + inwardX * 5;
  const cy = lc.y + inwardY * 5;

  const cut = new Graphics();
  // Markante rote Trennlinie quer über das Gleisprofil
  cut.moveTo(cx - perpX * 18, cy - perpY * 18).lineTo(cx + perpX * 18, cy + perpY * 18);
  cut.stroke({ width: 4, color: 0xdc2626, alpha: 0.95 });
  // Helle Reflexionslinie
  cut.moveTo(cx + inwardX * 1.5 - perpX * 18, cy + inwardY * 1.5 - perpY * 18)
    .lineTo(cx + inwardX * 1.5 + perpX * 18, cy + inwardY * 1.5 + perpY * 18);
  cut.stroke({ width: 1.5, color: 0xffffff, alpha: 0.85 });
  parent.addChild(cut);

  // Markante runde ⛔ Plakette neben der Trennstelle
  const marker = new Container();
  const badgeX = cx + perpX * 24;
  const badgeY = cy + perpY * 24;
  marker.position.set(badgeX, badgeY);
  marker.rotation = -parent.rotation;

  const bg = new Graphics();
  bg.circle(1, 1, 11);
  bg.fill({ color: 0x000000, alpha: 0.45 });
  bg.circle(0, 0, 11);
  bg.fill({ color: 0xdc2626, alpha: 0.95 });
  bg.stroke({ width: 2, color: 0xffffff });
  marker.addChild(bg);

  const txt = new Text({
    text: "⛔",
    style: new TextStyle({ fontSize: 11, fill: 0xffffff, fontWeight: "bold" }),
  });
  txt.anchor.set(0.5);
  marker.addChild(txt);

  parent.addChild(marker);
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

function drawCatenary2D(parent: Container, entry: CatalogEntry, powerResult?: PiecePowerResult) {
  const g = new Graphics();
  let wireColor = 0x0ea5e9; // Leuchtendes Cyan / Fahrdraht-Blau
  let wireWidth = 1.8;
  let wireAlpha = 0.95;

  if (powerResult && powerResult.catenaryVoltage !== undefined) {
    if (powerResult.catenaryStatus === "optimal") {
      wireColor = 0x10b981; // Grün / Optimal
      wireWidth = 2.4;
    } else if (powerResult.catenaryStatus === "warning") {
      wireColor = 0xf59e0b; // Bernstein / Warnung
      wireWidth = 2.4;
    } else if (powerResult.catenaryStatus === "critical") {
      wireColor = 0xef4444; // Rot / Kritisch
      wireWidth = 2.4;
    } else if (powerResult.catenaryStatus === "disconnected") {
      wireColor = 0xdc2626; // Abgetrennt
      wireWidth = 1.4;
      wireAlpha = 0.45;
    }
  }

  const mastColor = 0x334155; // Anthrazit für Gittermast
  const insulatorColor = 0xb45309; // Bernstein / Keramik-Isolator

  // 1. Fahrdraht als Mittellinie
  drawTrackCenterline(g, entry, wireColor, wireWidth, wireAlpha);

  // 2. Streckenmasten mit Auslegern
  const drawMast = (x: number, y: number, angleRad: number, side: 1 | -1 = 1) => {
    const mastDist = 22 * side;
    const sin = Math.sin(angleRad);
    const cos = Math.cos(angleRad);
    const perpX = -sin * mastDist;
    const perpY = cos * mastDist;

    const mx = x + perpX;
    const my = y + perpY;

    // Beton-Fundament (Quadrat)
    g.rect(mx - 2.5, my - 2.5, 5, 5);
    g.fill({ color: 0x64748b });
    g.stroke({ width: 1, color: 0x1e293b });

    // Stahl-Gittermast
    g.circle(mx, my, 2.2);
    g.fill({ color: mastColor });

    // Auslegerarm (horizontal zur Gleismitte)
    g.moveTo(mx, my).lineTo(x, y);
    g.stroke({ width: 1.5, color: mastColor, alpha: 0.9 });

    // Isolator
    const insX = x + perpX * 0.25;
    const insY = y + perpY * 0.25;
    g.circle(insX, insY, 1.8);
    g.fill({ color: insulatorColor });
  };

  const kind = entry.kind;
  if (kind === "turntable" || kind === "transfer-table") return;
  if (kind === "straight" || kind === "accessory" || kind === "flex") {
    const l = entry.length ?? 188.3;
    drawMast(0, 0, 0, 1);
    if (l >= 150) {
      drawMast(l, 0, 0, 1);
    }
  } else if (kind === "curve") {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const absA = Math.abs(a);
    const rad = deg2rad(absA);
    drawMast(0, 0, 0, 1);
    const endX = r * Math.sin(rad);
    const endY = a < 0 ? -r * (1 - Math.cos(rad)) : r * (1 - Math.cos(rad));
    drawMast(endX, endY, a < 0 ? -rad : rad, a < 0 ? -1 : 1);
  } else {
    drawMast(0, 0, 0, 1);
  }

  parent.addChild(g);
}

function drawCatenaryPowerFeederMarker(parent: Container, _entry: CatalogEntry) {
  const marker = new Container();
  // Platzierung auf der Mast-Seite bei y = +26
  marker.position.set(16, 26);
  marker.rotation = -parent.rotation;

  const g = new Graphics();
  // Zuleitungskabel von der Plakette zum Mast/Fahrdraht
  g.moveTo(0, 0).lineTo(-16, -6).lineTo(-16, -26);
  g.stroke({ width: 1.8, color: 0x0284c7, alpha: 0.9 });

  // Schatten
  g.circle(1, 1, 11);
  g.fill({ color: 0x000000, alpha: 0.4 });

  // Cyan-Goldene Einspeise-Plakette
  g.circle(0, 0, 11);
  g.fill({ color: 0x0284c7, alpha: 0.95 });
  g.stroke({ width: 2, color: 0x38bdf8 });
  marker.addChild(g);

  const txt = new Text({
    text: "⚡",
    style: new TextStyle({ fontSize: 13, fill: 0xffffff, fontWeight: "bold" }),
  });
  txt.anchor.set(0.5);
  marker.addChild(txt);

  // Kleines "OL" (Oberleitung) Badge
  const olBadge = new Graphics();
  olBadge.rect(2, 2, 14, 9);
  olBadge.fill({ color: 0x0f172a, alpha: 0.95 });
  olBadge.stroke({ width: 1, color: 0x38bdf8 });
  marker.addChild(olBadge);

  const subTxt = new Text({
    text: "OL",
    style: new TextStyle({ fontSize: 7, fill: 0x38bdf8, fontWeight: "bold" }),
  });
  subTxt.anchor.set(0.5);
  subTxt.position.set(9, 6.5);
  marker.addChild(subTxt);

  parent.addChild(marker);
}

function drawCatenaryIsolatorMarker(parent: Container, entry: CatalogEntry, isoIdx = 0) {
  const locals = getLocalConnectors(entry);
  const lc = locals[isoIdx] ?? locals[0] ?? { x: 0, y: 0, angle: 180 };

  const connAngleRad = deg2rad(lc.angle);
  const inwardX = -Math.cos(connAngleRad);
  const inwardY = -Math.sin(connAngleRad);
  const perpX = -Math.sin(connAngleRad);
  const perpY = Math.cos(connAngleRad);

  // Position Streckentrenner 8mm eingerückt vom Anschluss am Fahrdraht
  const cx = lc.x + inwardX * 8;
  const cy = lc.y + inwardY * 8;

  const cut = new Graphics();
  // Keramischer Streckentrenner quer über die Fahrleitung
  cut.moveTo(cx - perpX * 7, cy - perpY * 7).lineTo(cx + perpX * 7, cy + perpY * 7);
  cut.stroke({ width: 3.5, color: 0xb45309, alpha: 0.95 }); // Bernstein Keramik

  // Trennspalt
  cut.moveTo(cx + inwardX * 2 - perpX * 7, cy + inwardY * 2 - perpY * 7)
    .lineTo(cx + inwardX * 2 + perpX * 7, cy + inwardY * 2 + perpY * 7);
  cut.stroke({ width: 3.5, color: 0x0284c7, alpha: 0.95 }); // Cyan

  parent.addChild(cut);

  // Markante runde Plakette neben der Oberleitungs-Trennstelle
  const marker = new Container();
  const badgeDist = 26;
  const badgeX = cx - perpX * badgeDist;
  const badgeY = cy - perpY * badgeDist;
  marker.position.set(badgeX, badgeY);
  marker.rotation = -parent.rotation;

  const bg = new Graphics();
  bg.circle(1, 1, 11);
  bg.fill({ color: 0x000000, alpha: 0.45 });
  bg.circle(0, 0, 11);
  bg.fill({ color: 0x0369a1, alpha: 0.95 });
  bg.stroke({ width: 2, color: 0xef4444 });
  marker.addChild(bg);

  const txt = new Text({
    text: "⚡⛔",
    style: new TextStyle({ fontSize: 10, fill: 0xffffff, fontWeight: "bold" }),
  });
  txt.anchor.set(0.5);
  marker.addChild(txt);

  // Kleines "OL" Badge
  const olBadge = new Graphics();
  olBadge.rect(-13, 2, 13, 9);
  olBadge.fill({ color: 0x0f172a, alpha: 0.95 });
  olBadge.stroke({ width: 1, color: 0xef4444 });
  marker.addChild(olBadge);

  const subTxt = new Text({
    text: "OL",
    style: new TextStyle({ fontSize: 7, fill: 0xf87171, fontWeight: "bold" }),
  });
  subTxt.anchor.set(0.5);
  subTxt.position.set(-6.5, 6.5);
  marker.addChild(subTxt);

  parent.addChild(marker);
}

function drawTurnoutAccessories2D(container: Container, entry: CatalogEntry, piece: TrackPiece) {
  const kind = entry.kind;
  const isLeft = kind === "turnout-left" || kind === "curved-turnout-left";
  const lanternY = isLeft ? -22 : 22;
  const lanternX = 26;

  const accGroup = new Container();

  // 1. Stellstange / Verbindungsschiene zur Stellschwelle
  const rod = new Graphics();
  rod.moveTo(lanternX, 0).lineTo(lanternX, lanternY);
  rod.stroke({ width: 1.6, color: 0x334155, alpha: 0.85 });
  accGroup.addChild(rod);

  if (piece.hasLantern) {
    // Weichenlaternen-Gehäuse (Schwarzer Metallkasten mit Sockel)
    const lanternBox = new Graphics();
    // Sockel
    lanternBox.circle(lanternX, lanternY, 5.5);
    lanternBox.fill({ color: 0x0f172a });
    lanternBox.stroke({ width: 1.2, color: 0x475569 });

    // Laternenwürfel (Gehäuse 7x7 mm)
    lanternBox.rect(lanternX - 3.5, lanternY - 3.5, 7, 7);
    lanternBox.fill({ color: 0x1e293b });
    lanternBox.stroke({ width: 1, color: 0x94a3b8 });

    // Leuchtendes Weichensignal / Milchglas-Laternenfenster (warmgelb/weiß leuchtend)
    lanternBox.circle(lanternX, lanternY, 2.2);
    lanternBox.fill({ color: 0xfef08a, alpha: 0.95 });
    // Strahlender Halo / Glüh-Effekt
    lanternBox.circle(lanternX, lanternY, 4.2);
    lanternBox.stroke({ width: 1.2, color: 0xfef08a, alpha: 0.5 });

    accGroup.addChild(lanternBox);
  }

  if (piece.hasDecoder) {
    // Weichendecoder (Digitalbaustein / Chip)
    const decX = piece.hasLantern ? lanternX + 11 : lanternX;
    const decY = lanternY;

    const decBox = new Graphics();
    // Schatten
    decBox.roundRect(decX - 5.5 + 0.5, decY - 4.5 + 0.5, 11, 9, 2);
    decBox.fill({ color: 0x000000, alpha: 0.4 });

    // Decoder-Gehäuse (Dunkelblau/Cyan)
    decBox.roundRect(decX - 5.5, decY - 4.5, 11, 9, 2);
    decBox.fill({ color: 0x0f172a, alpha: 0.95 });
    decBox.stroke({ width: 1.2, color: 0x0284c7 });

    // Digitale Kupfer-Anschlusspins
    decBox.rect(decX - 4, decY - 5.5, 2, 1);
    decBox.rect(decX + 2, decY - 5.5, 2, 1);
    decBox.rect(decX - 4, decY + 4.5, 2, 1);
    decBox.rect(decX + 2, decY + 4.5, 2, 1);
    decBox.fill({ color: 0xf59e0b });

    accGroup.addChild(decBox);

    // Mini-Beschriftung "DEC"
    const decTxt = new Text({
      text: "DEC",
      style: new TextStyle({
        fontSize: 5,
        fontWeight: "bold",
        fill: 0x38bdf8,
        fontFamily: "monospace",
      }),
    });
    decTxt.anchor.set(0.5);
    decTxt.position.set(decX, decY);
    accGroup.addChild(decTxt);
  }

  container.addChild(accGroup);
}

/**
 * Zeichnet den Schatten einer angehobenen Straße
 */
function drawRoadElevationShadow(
  g: Graphics,
  entry: CatalogEntry,
  roadWidth: number,
  maxElev: number,
) {
  const shadowDist = Math.min(maxElev * 0.35, 22);
  const shadowAlpha = Math.min(0.35, 0.12 + (maxElev / 100) * 0.2);
  const roadHalf = roadWidth / 2;
  const shadowColor = 0x000000;
  const kind = entry.kind;

  if (kind === "straight" || kind === "accessory") {
    const l = entry.length ?? 200;
    if (entry.id === "ST-LOOP") {
      g.rect(shadowDist * 0.5, -roadHalf + shadowDist, 35, roadWidth);
      g.circle(55 + shadowDist * 0.5, shadowDist, 42);
      g.fill({ color: shadowColor, alpha: shadowAlpha });
      return;
    }
    g.rect(shadowDist * 0.5, -roadHalf + shadowDist, l, roadWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 200;
    const a = entry.angle ?? 90;
    drawArcStroke(
      g,
      shadowDist * 0.5,
      r + shadowDist,
      r,
      -Math.PI / 2,
      -Math.PI / 2 + deg2rad(a),
      roadWidth,
      shadowColor,
      false,
      shadowAlpha,
    );
    return;
  }

  if (kind === "turnout-left") {
    const l = entry.length ?? 200;
    g.rect(shadowDist * 0.5, -roadHalf + shadowDist, l, roadWidth);
    if (entry.branchLength !== undefined) {
      const midX = l / 2;
      g.rect(midX - roadHalf + shadowDist * 0.5, shadowDist, roadWidth, entry.branchLength);
    } else if (entry.branchRadius !== undefined) {
      const br = entry.branchRadius;
      const ba = entry.branchAngle ?? 45;
      drawArcStroke(
        g,
        shadowDist * 0.5,
        br + shadowDist,
        br,
        -Math.PI / 2,
        -Math.PI / 2 + deg2rad(ba),
        roadWidth,
        shadowColor,
        false,
        shadowAlpha,
      );
    }
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (kind === "crossing") {
    const l = entry.length ?? 200;
    const midX = l / 2;
    g.rect(shadowDist * 0.5, -roadHalf + shadowDist, l, roadWidth);
    g.rect(midX - roadHalf + shadowDist * 0.5, -l / 2 + shadowDist, roadWidth, l);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }
}

/**
 * Zeichnet die Selektionskontur für ein Straßenstück
 */
function drawRoadSelectionContour(
  g: Graphics,
  entry: CatalogEntry,
  contourWidth = 68,
) {
  const kind = entry.kind;
  const glowColor = 0x0284c7;
  const glowAlpha = 0.5;
  const contourHalf = contourWidth / 2;

  if (kind === "straight" || kind === "accessory") {
    const l = entry.length ?? 200;
    if (entry.id === "ST-LOOP") {
      g.rect(-4, -contourHalf, 45, contourWidth);
      g.circle(55, 0, 42 + 4);
      g.stroke({ width: 3.5, color: glowColor, alpha: 0.9 });
      g.fill({ color: glowColor, alpha: 0.15 });
      return;
    }
    if (entry.id === "ST-BUS") {
      g.rect(-4, -contourHalf, l + 8, contourWidth);
      g.rect(33, contourHalf - 4, 134, 19);
      g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
      g.fill({ color: glowColor, alpha: 0.15 });
      return;
    }
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });
    return;
  }

  if (kind === "curve") {
    const r = entry.radius ?? 200;
    const a = entry.angle ?? 90;
    const rad = deg2rad(a);
    drawArcStroke(g, 0, r, r, -Math.PI / 2, -Math.PI / 2 + rad, contourWidth, glowColor, false, 0.15);
    drawArcStroke(g, 0, r, r - contourHalf, -Math.PI / 2, -Math.PI / 2 + rad, 3.5, glowColor, false, 0.9);
    drawArcStroke(g, 0, r, r + contourHalf, -Math.PI / 2, -Math.PI / 2 + rad, 3.5, glowColor, false, 0.9);
    return;
  }

  if (kind === "turnout-left") {
    const l = entry.length ?? 200;
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });

    if (entry.branchLength !== undefined) {
      const midX = l / 2;
      g.rect(midX - contourHalf, 0, contourWidth, entry.branchLength + 4);
      g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
      g.fill({ color: glowColor, alpha: 0.15 });
    } else if (entry.branchRadius !== undefined) {
      const br = entry.branchRadius;
      const ba = entry.branchAngle ?? 45;
      const rad = deg2rad(ba);
      drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + rad, contourWidth, glowColor, false, 0.15);
      drawArcStroke(g, 0, br, br - contourHalf, -Math.PI / 2, -Math.PI / 2 + rad, 3.5, glowColor, false, 0.9);
      drawArcStroke(g, 0, br, br + contourHalf, -Math.PI / 2, -Math.PI / 2 + rad, 3.5, glowColor, false, 0.9);
    }
    return;
  }

  if (kind === "crossing") {
    const l = entry.length ?? 200;
    const midX = l / 2;
    g.rect(-4, -contourHalf, l + 8, contourWidth);
    g.rect(midX - contourHalf, -l / 2 - 4, contourWidth, l + 8);
    g.stroke({ width: 3.5, color: glowColor, alpha: 0.9, join: "miter" });
    g.fill({ color: glowColor, alpha: 0.15 });
    return;
  }
}

/**
 * Zeichnet ein Straßenstück realistisch mit 2D PixiJS Graphics:
 * Asphaltfahrbahn, solide weiße Begrenzungslinien, gestrichelte Mittellinie
 * und fahrbahnspezifische Markierungen (Bushaltestelle, Bahnübergang, etc.)
 */
function drawRoadPiece2D(g: Graphics, entry: CatalogEntry, _isSelected: boolean) {
  const kind = entry.kind;
  const roadWidth = entry.roadWidth ?? 60;
  const roadHalf = roadWidth / 2;

  const asphaltColor = 0x383a42;
  const curbColor = 0x22242a;
  const markingColor = 0xf8fafc;
  const lineW = 1.5;

  if (kind === "straight" || kind === "accessory") {
    const l = entry.length ?? 200;

    // --- WENDEPLATZ / SACKGASSE (ST-LOOP) ---
    if (entry.id === "ST-LOOP") {
      const neckL = 35;
      const loopCenterX = 55;
      const loopRadius = 42;

      // Asphaltkörper
      g.rect(0, -roadHalf, neckL, roadWidth);
      g.circle(loopCenterX, 0, loopRadius);
      g.fill({ color: asphaltColor, alpha: 0.98 });
      g.stroke({ width: 1, color: curbColor, alpha: 0.8 });

      // Weiße Randlinien
      g.moveTo(0, -roadHalf + 2.5).lineTo(neckL, -roadHalf + 2.5);
      g.moveTo(0, roadHalf - 2.5).lineTo(neckL, roadHalf - 2.5);
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      g.arc(loopCenterX, 0, loopRadius - 2.5, -Math.PI * 0.65, Math.PI * 0.65, false);
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      // Zentrierter Wendekreis / Mittelinsel
      g.circle(loopCenterX, 0, 14);
      g.fill({ color: 0x27272a, alpha: 0.98 });
      g.stroke({ width: 1.5, color: markingColor, alpha: 0.9 });

      // Kurzer Mittellinien-Ansatz
      for (let x = 4; x + 8 <= neckL; x += 14) {
        g.moveTo(x, 0).lineTo(x + 8, 0);
      }
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });
      return;
    }

    // --- BUSHALTESTELLE / PARKMÖGLICHKEIT (ST-BUS) ---
    if (entry.id === "ST-BUS") {
      const bayStartX = 35;
      const bayEndX = 165;
      const bayW = 15;

      // Hauptfahrbahn
      g.rect(0, -roadHalf, l, roadWidth);
      // Ausbuchtung Haltebucht (unten bei y = +roadHalf)
      g.roundRect(bayStartX, roadHalf, bayEndX - bayStartX, bayW, 3);
      g.fill({ color: asphaltColor, alpha: 0.98 });
      g.stroke({ width: 1, color: curbColor, alpha: 0.8 });

      // Randlinie oben
      g.moveTo(0, -roadHalf + 2.5).lineTo(l, -roadHalf + 2.5);
      // Randlinie unten vor und nach der Haltebucht
      g.moveTo(0, roadHalf - 2.5).lineTo(bayStartX, roadHalf - 2.5);
      g.moveTo(bayEndX, roadHalf - 2.5).lineTo(l, roadHalf - 2.5);
      // Äußere Kante der Haltebucht
      g.moveTo(bayStartX, roadHalf + bayW - 2.5).lineTo(bayEndX, roadHalf + bayW - 2.5);
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      // Gelbe/Weiße Trennlinie zur Bucht (unterbrochen)
      for (let bx = bayStartX + 4; bx + 6 <= bayEndX - 4; bx += 10) {
        g.moveTo(bx, roadHalf - 0.5).lineTo(bx + 6, roadHalf - 0.5);
      }
      g.stroke({ width: lineW, color: 0xfbbf24, alpha: 0.92 });

      // Symbolische Zickzack-/Wartelinie
      const midBayX = (bayStartX + bayEndX) / 2;
      for (let zx = midBayX - 25; zx <= midBayX + 25; zx += 8) {
        g.moveTo(zx - 3, roadHalf + 3).lineTo(zx + 3, roadHalf + bayW - 5);
      }
      g.stroke({ width: 1.2, color: 0xfbbf24, alpha: 0.65 });

      // Mittellinie Hauptstraße
      for (let x = 4; x + 8 <= l - 2; x += 14) {
        g.moveTo(x, 0).lineTo(x + 8, 0);
      }
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });
      return;
    }

    // --- BAHNÜBERGANG (ST-CROSS) ---
    if (entry.id === "ST-CROSS") {
      // Asphaltfahrbahn
      g.rect(0, -roadHalf, l, roadWidth);
      g.fill({ color: asphaltColor, alpha: 0.98 });
      g.stroke({ width: 1, color: curbColor, alpha: 0.8 });

      // Randlinien vor und hinter Übergang
      g.moveTo(0, -roadHalf + 2.5).lineTo(76, -roadHalf + 2.5);
      g.moveTo(124, -roadHalf + 2.5).lineTo(l, -roadHalf + 2.5);
      g.moveTo(0, roadHalf - 2.5).lineTo(76, roadHalf - 2.5);
      g.moveTo(124, roadHalf - 2.5).lineTo(l, roadHalf - 2.5);
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      // Mittellinie
      for (let x = 4; x + 8 <= 74; x += 14) {
        g.moveTo(x, 0).lineTo(x + 8, 0);
      }
      for (let x = 126; x + 8 <= l - 2; x += 14) {
        g.moveTo(x, 0).lineTo(x + 8, 0);
      }
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      // Stop- / Haltelinien vor Gleisübergang
      g.moveTo(76, -roadHalf + 2.5).lineTo(76, roadHalf - 2.5);
      g.moveTo(124, -roadHalf + 2.5).lineTo(124, roadHalf - 2.5);
      g.stroke({ width: 2.5, color: markingColor, alpha: 0.95 });

      // Rot-weiße Gefahren-Randmarkierungen vor und nach Bahnübergang
      for (let rx = 58; rx < 76; rx += 4.5) {
        g.rect(rx, -roadHalf + 0.5, 2.2, 2.2).fill({ color: 0xdc2626 });
        g.rect(rx + 2.2, -roadHalf + 0.5, 2.3, 2.2).fill({ color: 0xf8fafc });
        g.rect(rx, roadHalf - 2.7, 2.2, 2.2).fill({ color: 0xdc2626 });
        g.rect(rx + 2.2, roadHalf - 2.7, 2.3, 2.2).fill({ color: 0xf8fafc });
      }
      for (let rx = 124; rx < 142; rx += 4.5) {
        g.rect(rx, -roadHalf + 0.5, 2.2, 2.2).fill({ color: 0xdc2626 });
        g.rect(rx + 2.2, -roadHalf + 0.5, 2.3, 2.2).fill({ color: 0xf8fafc });
        g.rect(rx, roadHalf - 2.7, 2.2, 2.2).fill({ color: 0xdc2626 });
        g.rect(rx + 2.2, roadHalf - 2.7, 2.3, 2.2).fill({ color: 0xf8fafc });
      }

      // Gleisüberfahrt: Holz-/Gummibohlen
      g.rect(82, -roadHalf - 1.5, 36, roadWidth + 3);
      g.fill({ color: 0x221f1d, alpha: 0.98 });
      g.stroke({ width: 1.2, color: 0x475569 });

      // Einzelne Bohlenfugen quer zur Fahrbahn
      for (let bx = 86; bx <= 114; bx += 5.5) {
        g.moveTo(bx, -roadHalf - 1).lineTo(bx, roadHalf + 1);
      }
      g.stroke({ width: 1, color: 0x0f172a, alpha: 0.85 });

      // Durchgehende Schienenstränge des kreuzenden Gleises (Spurweite 16.5 mm quer zur Straße bei x=100)
      const rL = 100 - 8.25;
      const rR = 100 + 8.25;
      g.moveTo(rL, -roadHalf - 3).lineTo(rL, roadHalf + 3);
      g.moveTo(rR, -roadHalf - 3).lineTo(rR, roadHalf + 3);
      g.stroke({ width: 2, color: 0xc8d0d8, alpha: 0.95 });
      return;
    }

    // --- STANDARD GERADE STRASSE (ST-S200, ST-S100, ST-S050) ---
    g.rect(0, -roadHalf, l, roadWidth);
    g.fill({ color: asphaltColor, alpha: 0.98 });
    g.stroke({ width: 1, color: curbColor, alpha: 0.8 });

    // Weiße Begrenzungslinien
    g.moveTo(0, -roadHalf + 2.5).lineTo(l, -roadHalf + 2.5);
    g.moveTo(0, roadHalf - 2.5).lineTo(l, roadHalf - 2.5);
    g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

    // Gestrichelte Mittellinie
    for (let x = 4; x + 8 <= l - 2; x += 14) {
      g.moveTo(x, 0).lineTo(x + 8, 0);
    }
    g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });
    return;
  }

  // --- KURVE (ST-C200-90, ST-C200-45, ST-C300-45) ---
  if (kind === "curve") {
    const r = entry.radius ?? 200;
    const a = entry.angle ?? 90;
    const rad = deg2rad(a);

    // Asphalt Bogen
    drawArcStroke(g, 0, r, r, -Math.PI / 2, -Math.PI / 2 + rad, roadWidth, asphaltColor);

    // Weiße Randlinien
    drawArcStroke(g, 0, r, r - roadHalf + 2.5, -Math.PI / 2, -Math.PI / 2 + rad, lineW, markingColor, false, 0.92);
    drawArcStroke(g, 0, r, r + roadHalf - 2.5, -Math.PI / 2, -Math.PI / 2 + rad, lineW, markingColor, false, 0.92);

    // Gestrichelte Mittellinie
    const totalArc = rad * r;
    const period = 14;
    const numDashes = Math.max(1, Math.floor(totalArc / period));
    const angStep = rad / numDashes;
    const dashAng = (8 / totalArc) * rad;
    for (let i = 0; i < numDashes; i++) {
      const startA = -Math.PI / 2 + i * angStep + (angStep - dashAng) * 0.5;
      const endA = startA + dashAng;
      drawArcStroke(g, 0, r, r, startA, endA, lineW, markingColor, false, 0.92);
    }
    return;
  }

  // --- T-KREUZUNG / Y-ABZWEIG (ST-TJ-90, ST-Y-45) ---
  if (kind === "turnout-left") {
    const l = entry.length ?? 200;
    const midX = l / 2;

    // T-Kreuzung mit rechtwinkligem Abzweig (ST-TJ-90)
    if (entry.branchLength !== undefined) {
      const bLen = entry.branchLength;

      // Asphalt
      g.rect(0, -roadHalf, l, roadWidth);
      g.rect(midX - roadHalf, 0, roadWidth, bLen);
      g.fill({ color: asphaltColor, alpha: 0.98 });
      g.stroke({ width: 1, color: curbColor, alpha: 0.8 });

      // Durchgehende Außenlinie oben
      g.moveTo(0, -roadHalf + 2.5).lineTo(l, -roadHalf + 2.5);
      // Ecke links unten
      g.moveTo(0, roadHalf - 2.5).lineTo(midX - roadHalf + 2.5, roadHalf - 2.5);
      g.lineTo(midX - roadHalf + 2.5, bLen);
      // Ecke rechts unten
      g.moveTo(l, roadHalf - 2.5).lineTo(midX + roadHalf - 2.5, roadHalf - 2.5);
      g.lineTo(midX + roadHalf - 2.5, bLen);
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      // Mittellinie Hauptstraße
      for (let x = 4; x + 8 <= l - 2; x += 14) {
        g.moveTo(x, 0).lineTo(x + 8, 0);
      }
      // Mittellinie Einmündung
      for (let y = bLen - 4; y - 8 >= 14; y -= 14) {
        g.moveTo(midX, y).lineTo(midX, y - 8);
      }
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });
      return;
    }

    // Y-Einmündung mit 45° Bogen (ST-Y-45)
    if (entry.branchRadius !== undefined) {
      const br = entry.branchRadius;
      const ba = entry.branchAngle ?? 45;
      const rad = deg2rad(ba);

      // Asphalt Hauptstraße
      g.rect(0, -roadHalf, l, roadWidth);
      g.fill({ color: asphaltColor, alpha: 0.98 });
      g.stroke({ width: 1, color: curbColor, alpha: 0.8 });

      // Asphalt Abzweig
      drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + rad, roadWidth, asphaltColor);

      // Weiße Randlinien Hauptstraße
      g.moveTo(0, -roadHalf + 2.5).lineTo(l, -roadHalf + 2.5);
      g.moveTo(0, roadHalf - 2.5).lineTo(l, roadHalf - 2.5);
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      // Weiße Randlinie Abzweig
      drawArcStroke(g, 0, br, br + roadHalf - 2.5, -Math.PI / 2, -Math.PI / 2 + rad, lineW, markingColor, false, 0.92);

      // Mittellinie Hauptstraße
      for (let x = 4; x + 8 <= l - 2; x += 14) {
        g.moveTo(x, 0).lineTo(x + 8, 0);
      }
      g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

      // Gestrichelte Mittellinie Abzweig
      const totalArc = rad * br;
      const numDashes = Math.max(1, Math.floor(totalArc / 14));
      const angStep = rad / numDashes;
      const dashAng = (8 / totalArc) * rad;
      for (let i = 0; i < numDashes; i++) {
        const startA = -Math.PI / 2 + i * angStep + (angStep - dashAng) * 0.5;
        const endA = startA + dashAng;
        drawArcStroke(g, 0, br, br, startA, endA, lineW, markingColor, false, 0.92);
      }
      return;
    }
  }

  // --- 4-WEGE STRASSENKREUZUNG (ST-X-90) ---
  if (kind === "crossing") {
    const l = entry.length ?? 200;
    const midX = l / 2;
    const halfL = l / 2;

    // Asphalt horizontal & vertikal
    g.rect(0, -roadHalf, l, roadWidth);
    g.rect(midX - roadHalf, -halfL, roadWidth, l);
    g.fill({ color: asphaltColor, alpha: 0.98 });
    g.stroke({ width: 1, color: curbColor, alpha: 0.8 });

    // 4 Ecken mit Begrenzungslinien
    const leftX = midX - roadHalf + 2.5;
    const rightX = midX + roadHalf - 2.5;
    const topY = -roadHalf + 2.5;
    const bottomY = roadHalf - 2.5;

    // Oben-Links
    g.moveTo(0, topY).lineTo(leftX, topY).lineTo(leftX, -halfL);
    // Oben-Rechts
    g.moveTo(l, topY).lineTo(rightX, topY).lineTo(rightX, -halfL);
    // Unten-Links
    g.moveTo(0, bottomY).lineTo(leftX, bottomY).lineTo(leftX, halfL);
    // Unten-Rechts
    g.moveTo(l, bottomY).lineTo(rightX, bottomY).lineTo(rightX, halfL);
    g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });

    // 4 Zufahrt-Mittellinien bis zur Kreuzungsmitte
    for (let x = 4; x + 8 <= leftX - 4; x += 14) {
      g.moveTo(x, 0).lineTo(x + 8, 0);
    }
    for (let x = rightX + 4; x + 8 <= l - 2; x += 14) {
      g.moveTo(x, 0).lineTo(x + 8, 0);
    }
    for (let y = -halfL + 4; y + 8 <= topY - 4; y += 14) {
      g.moveTo(midX, y).lineTo(midX, y + 8);
    }
    for (let y = bottomY + 4; y + 8 <= halfL - 2; y += 14) {
      g.moveTo(midX, y).lineTo(midX, y + 8);
    }
    g.stroke({ width: lineW, color: markingColor, alpha: 0.92 });
    return;
  }
}
