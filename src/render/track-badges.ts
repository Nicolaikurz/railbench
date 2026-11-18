import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { CatalogEntry } from "@/catalog/catalog";
import type { PiecePowerResult } from "@/core/simulation";
import type { BlueprintColor } from "@/render/blueprint-colors";
import {
  getPieceLocalCenter,
  getUprightTextRotation,
  isStraightTrackEntry,
  isCurvedTrackEntry,
  deg2rad,
} from "@/core/geometry";
import { sampleFlexSplinePoints } from "@/core/flextrack";
import { drawArcStroke } from "./track-render-utils";

export function getPieceRunningLength(entry: CatalogEntry): number {
  if (entry.length) return entry.length;
  if (entry.radius && entry.angle) {
    return (2 * Math.PI * entry.radius * Math.abs(entry.angle)) / 360;
  }
  return 188.3;
}

// Statische Shared TextStyles, um GC-Allokationen bei jedem Render-Durchlauf zu vermeiden
const _trackNumberStyleCache = new Map<number, TextStyle>();
function getTrackNumberStyle(fontSize: number): TextStyle {
  const rounded = Math.round(fontSize * 10) / 10;
  let style = _trackNumberStyleCache.get(rounded);
  if (!style) {
    style = new TextStyle({
      fontFamily: "Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
      fontSize: rounded,
      fontWeight: "bold",
      fill: 0xffffff,
      align: "center",
      letterSpacing: Math.max(0.1, Math.round(rounded * 0.04 * 10) / 10),
    });
    _trackNumberStyleCache.set(rounded, style);
  }
  return style;
}

const TRACK_NUMBER_STYLE = getTrackNumberStyle(10);

const VOLT_STYLE_GOOD = new TextStyle({
  fontFamily: "Inter, system-ui, -apple-system, sans-serif",
  fontSize: 9.5,
  fontWeight: "bold",
  fill: 0xf0fdf4,
  align: "center",
  letterSpacing: 0.3,
});

const VOLT_STYLE_WARN = new TextStyle({
  fontFamily: "Inter, system-ui, -apple-system, sans-serif",
  fontSize: 9.5,
  fontWeight: "bold",
  fill: 0xfffbeb,
  align: "center",
  letterSpacing: 0.3,
});

const VOLT_STYLE_CRIT = new TextStyle({
  fontFamily: "Inter, system-ui, -apple-system, sans-serif",
  fontSize: 9.5,
  fontWeight: "bold",
  fill: 0xfef2f2,
  align: "center",
  letterSpacing: 0.3,
});

const RAMP_STYLE_NORMAL = new TextStyle({
  fontFamily: "Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
  fontSize: 9,
  fontWeight: "bold",
  fill: 0xf0fdf4,
  align: "center",
  letterSpacing: 0.3,
});

const RAMP_STYLE_WARN = new TextStyle({
  fontFamily: "Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
  fontSize: 9,
  fontWeight: "bold",
  fill: 0xfffbeb,
  align: "center",
  letterSpacing: 0.3,
});

const RAMP_STYLE_CRIT = new TextStyle({
  fontFamily: "Inter, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
  fontSize: 9,
  fontWeight: "bold",
  fill: 0xfef2f2,
  align: "center",
  letterSpacing: 0.3,
});

export function drawTrackNumberLabel(
  container: Container,
  entry: CatalogEntry,
  pieceRotation: number,
  bpColor?: BlueprintColor,
  hasVoltageLabel = false,
): void {
  const center = getPieceLocalCenter(entry);
  const labelCont = new Container();
  labelCont.position.set(center.x, center.y);
  labelCont.rotation = getUprightTextRotation(pieceRotation);

  // Spurweite & Schienenabstand berücksichtigen (z.B. Z: 6.5mm, N: 9mm, TT: 12mm, H0: 16.5mm)
  const gauge = entry.gauge ?? 16.5;
  const gaugeHalf = gauge / 2;
  const railWidth = Math.max(1.1, gaugeHalf * 0.22);

  // Maximale Badge-Höhe so begrenzen, dass sie bei geraden Gleisen exakt zwischen die Schienen passt
  // und keinesfalls über die Gleisbegrenzungen (Schieneninnenkanten) hinausragt
  const maxBadgeH = Math.min(13.0, Math.max(4.2, (gauge - railWidth) * 0.88));

  const padY = Math.max(0.6, Math.round(maxBadgeH * 0.16 * 10) / 10);
  const padX = Math.max(1.8, Math.round(maxBadgeH * 0.36 * 10) / 10);
  const cornerR = Math.max(1.2, Math.round(maxBadgeH * 0.22 * 10) / 10);
  const strokeW = Math.max(0.6, Math.min(1.2, Math.round(maxBadgeH * 0.09 * 10) / 10));

  const targetTextH = maxBadgeH - 2 * padY;
  const fontSize = Math.max(3.0, Math.round(targetTextH * 0.82 * 10) / 10);

  const voltOffset = (maxBadgeH / 2) + Math.max(0.8, 1.6 * (gauge / 16.5));
  const yOffset = entry.kind === "turntable" ? -55 : entry.kind === "transfer-table" ? -70 : hasVoltageLabel ? -voltOffset : 0;

  const text = new Text({
    text: entry.id,
    style: getTrackNumberStyle(fontSize),
  });
  text.anchor.set(0.5);
  text.position.set(0, yOffset);

  let textW = Math.max(10, entry.id.length * fontSize * 0.65);
  let textH = targetTextH;
  try {
    if (typeof document !== "undefined" && text.width > 0 && text.height > 0) {
      textW = text.width;
      textH = text.height;
    }
  } catch {
    // fallback
  }

  // Falls gerenderte Texthöhe größer ist als die erlaubte targetTextH: Text herunterskalieren
  if (textH > targetTextH) {
    const scaleRatio = targetTextH / textH;
    text.scale.set(scaleRatio);
    textW *= scaleRatio;
    textH = targetTextH;
  }

  const bgW = textW + padX * 2;
  const bgH = Math.min(maxBadgeH, textH + padY * 2);

  const bg = new Graphics();
  // Schatten
  bg.roundRect(-bgW / 2 + 0.5, -bgH / 2 + 1 + yOffset, bgW, bgH, cornerR);
  bg.fill({ color: 0x000000, alpha: 0.55 });
  // Hintergrund-Badge mit Farbkontur passend zum Bauplan-Farbcode
  bg.roundRect(-bgW / 2, -bgH / 2 + yOffset, bgW, bgH, cornerR);
  bg.fill({ color: 0x0f172a, alpha: 0.95 });
  bg.stroke({ width: strokeW, color: bpColor ? bpColor.hex : 0x38bdf8, alpha: 1.0 });

  labelCont.addChild(bg);
  labelCont.addChild(text);
  labelCont.label = "track-number-label";
  (labelCont as any).badgeWidth = bgW;
  (labelCont as any).badgeHeight = bgH;
  container.addChild(labelCont);
}

export function drawVoltageBadge(
  container: Container,
  entry: CatalogEntry,
  pieceRotation: number,
  powerResult: PiecePowerResult,
  hasNumberLabel: boolean,
): void {
  const center = getPieceLocalCenter(entry);
  const badgeCont = new Container();
  badgeCont.position.set(center.x, center.y);
  badgeCont.rotation = getUprightTextRotation(pieceRotation);
  const yOffset = hasNumberLabel ? 9 : 0;

  const isShort = Boolean(powerResult.isShortCircuit);
  const isDisc = powerResult.status === "disconnected";
  let voltText: string;
  if (isShort) {
    voltText = "⚠️⚡ KURZSCHLUSS";
  } else if (isDisc) {
    voltText = "⛔ 0.0 V";
  } else if (powerResult.isFeeder) {
    voltText = `⚡ ${powerResult.voltage.toFixed(2)} V`;
  } else {
    voltText = `${powerResult.voltage.toFixed(2)} V`;
  }

  if (powerResult.hasCatenary && powerResult.catenaryVoltage !== undefined) {
    const isCatDisc = powerResult.catenaryStatus === "disconnected";
    let catText: string;
    if (isCatDisc) {
      catText = "OL: ⛔ 0.0 V";
    } else if (powerResult.isCatenaryFeeder) {
      catText = `OL: ⚡ ${powerResult.catenaryVoltage.toFixed(2)} V`;
    } else {
      catText = `OL: ${powerResult.catenaryVoltage.toFixed(2)} V`;
    }
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

  let textStyle = VOLT_STYLE_GOOD;
  if (isShort || isDisc || powerResult.status === "critical") {
    textStyle = VOLT_STYLE_CRIT;
  } else if (powerResult.status === "warning") {
    textStyle = VOLT_STYLE_WARN;
  }

  const text = new Text({
    text: voltText,
    style: textStyle,
  });
  text.anchor.set(0.5);
  text.position.set(0, yOffset);

  const padX = 5;
  const padY = 2;
  let textW = Math.max(20, voltText.length * 6);
  let textH = 12;
  try {
    if (typeof document !== "undefined" && text.width > 0 && text.height > 0) {
      textW = text.width;
      textH = text.height;
    }
  } catch {
    // fallback
  }
  const bgW = textW + padX * 2;
  const bgH = textH + padY * 2;

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

export function drawElevationShadow(
  g: Graphics,
  entry: CatalogEntry,
  ballastWidth: number,
  elevation: number,
): void {
  const kind = entry.kind;
  const ballastHalf = ballastWidth / 2;
  const shadowDist = Math.min(10, 2 + elevation * 0.08);
  const shadowAlpha = Math.min(0.4, 0.12 + elevation * 0.003);
  const shadowColor = 0x000000;
  const ox = shadowDist * 0.5;
  const oy = shadowDist;

  if (entry.customOffsets && entry.customOffsets.length >= 3) {
    const chordLen = entry.customChordDist ?? entry.length ?? 900;
    const pts = sampleFlexSplinePoints(chordLen, entry.customOffsets);
    if (pts.length > 1) {
      g.moveTo(pts[0]!.x + ox, pts[0]!.y + oy);
      for (let i = 1; i < pts.length; i++) {
        g.lineTo(pts[i]!.x + ox, pts[i]!.y + oy);
      }
      g.stroke({ width: ballastWidth, color: shadowColor, alpha: shadowAlpha, cap: "butt", join: "round" });
    }
    return;
  }

  if (isStraightTrackEntry(entry)) {
    const l = entry.length ?? 188.3;
    g.rect(ox, -ballastHalf + oy, l, ballastWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (kind === "turntable") {
    const r = entry.radius ?? 189;
    g.circle(ox, oy, r);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const plateH = 420;
    const topY = -55.2;
    g.rect(ox, topY + oy, l, plateH);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    return;
  }

  if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    if (a < 0) {
      drawArcStroke(
        g,
        ox,
        -r + oy,
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
      ox,
      r + oy,
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

  // Bogenweichen (Links / Rechts)
  if (kind === "curved-turnout-left" || (kind === "turnout-left" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    drawArcStroke(g, ox, -r1 + oy, r1, Math.PI / 2, Math.PI / 2 - deg2rad(a1), ballastWidth, shadowColor, true, shadowAlpha);
    if (lead > 0) {
      g.rect(ox, -ballastHalf + oy, lead, ballastWidth);
      g.fill({ color: shadowColor, alpha: shadowAlpha });
    }
    drawArcStroke(g, lead + ox, -r2 + oy, r2, Math.PI / 2, Math.PI / 2 - deg2rad(a2), ballastWidth, shadowColor, true, shadowAlpha);
    return;
  }

  if (kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    drawArcStroke(g, ox, r1 + oy, r1, -Math.PI / 2, -Math.PI / 2 + deg2rad(a1), ballastWidth, shadowColor, false, shadowAlpha);
    if (lead > 0) {
      g.rect(ox, -ballastHalf + oy, lead, ballastWidth);
      g.fill({ color: shadowColor, alpha: shadowAlpha });
    }
    drawArcStroke(g, lead + ox, r2 + oy, r2, -Math.PI / 2, -Math.PI / 2 + deg2rad(a2), ballastWidth, shadowColor, false, shadowAlpha);
    return;
  }

  // Standard-Weichen
  if (kind === "turnout-left") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(ox, -ballastHalf + oy, l, ballastWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    drawArcStroke(g, ox, -br + oy, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), ballastWidth, shadowColor, true, shadowAlpha);
    return;
  }

  if (kind === "turnout-right") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(ox, -ballastHalf + oy, l, ballastWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    drawArcStroke(g, ox, br + oy, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), ballastWidth, shadowColor, false, shadowAlpha);
    return;
  }

  // Dreiwegweiche
  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(ox, -ballastHalf + oy, l, ballastWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    drawArcStroke(g, ox, -br + oy, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), ballastWidth, shadowColor, true, shadowAlpha);
    drawArcStroke(g, ox, br + oy, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), ballastWidth, shadowColor, false, shadowAlpha);
    return;
  }

  // Kreuzung
  if (kind === "crossing") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const rad = deg2rad(a);
    const h = l / 2;
    g.rect(-h + ox, -ballastHalf + oy, l, ballastWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    g.moveTo(-h * Math.cos(rad) + ox, -h * Math.sin(rad) + oy);
    g.lineTo(h * Math.cos(rad) + ox, h * Math.sin(rad) + oy);
    g.stroke({ width: ballastWidth, color: shadowColor, alpha: shadowAlpha, cap: "butt", join: "miter" });
    return;
  }

  // Doppelkreuzungsweiche
  if (kind === "double-slip") {
    const l = entry.length ?? 188.3;
    const a = entry.angle ?? 24.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? a;
    const rad = deg2rad(a);
    const h = l / 2;
    const cx = h;
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    g.rect(ox, -ballastHalf + oy, l, ballastWidth);
    g.fill({ color: shadowColor, alpha: shadowAlpha });
    g.moveTo(cx - h * cos + ox, -h * sin + oy);
    g.lineTo(cx + h * cos + ox, h * sin + oy);
    g.stroke({ width: ballastWidth, color: shadowColor, alpha: shadowAlpha, cap: "butt", join: "miter" });
    drawArcStroke(g, ox, br + oy, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), ballastWidth, shadowColor, false, shadowAlpha);
    drawArcStroke(g, l + ox, -br + oy, br, Math.PI / 2, Math.PI / 2 + deg2rad(ba), ballastWidth, shadowColor, false, shadowAlpha);
    return;
  }

  const l = entry.length ?? 188.3;
  g.rect(ox, -ballastHalf + oy, l, ballastWidth);
  g.fill({ color: shadowColor, alpha: shadowAlpha });
}

function drawChevron(g: Graphics, x: number, y: number, angleRad: number, size = 6.5, color = 0x38bdf8): void {
  const dx = Math.cos(angleRad) * size;
  const dy = Math.sin(angleRad) * size;
  const nx = -Math.sin(angleRad) * (size * 0.8);
  const ny = Math.cos(angleRad) * (size * 0.8);

  g.moveTo(x - dx + nx, y - dy + ny);
  g.lineTo(x + dx * 0.6, y + dy * 0.6);
  g.lineTo(x - dx - nx, y - dy - ny);
  g.stroke({ width: 2.2, color, alpha: 0.95, cap: "round", join: "round" });
}

export function drawRampChevrons(
  g: Graphics,
  entry: CatalogEntry,
  heightDiff: number,
  gradPercent: number,
): void {
  let chevronColor = 0x38bdf8;
  if (gradPercent > 4.5) {
    chevronColor = 0xef4444;
  } else if (gradPercent > 3.0) {
    chevronColor = 0xf59e0b;
  }

  const climbingForward = heightDiff > 0;

  if (isCurvedTrackEntry(entry)) {
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

export function drawRampBadge(
  container: Container,
  entry: CatalogEntry,
  pieceRotation: number,
  startElev: number,
  endElev: number,
  gradPercent: number,
  hasNumberLabel: boolean,
  hasVoltageLabel: boolean,
): void {
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
  let textStyle = RAMP_STYLE_NORMAL;
  if (gradPercent > 4.5) {
    strokeColor = 0xef4444;
    textStyle = RAMP_STYLE_CRIT;
  } else if (gradPercent > 3.0) {
    strokeColor = 0xf59e0b;
    textStyle = RAMP_STYLE_WARN;
  }

  const text = new Text({
    text: badgeText,
    style: textStyle,
  });
  text.anchor.set(0.5);
  text.position.set(0, yOffset);

  const padX = 4;
  const padY = 2;
  const textW = typeof document !== "undefined" ? (text.width || 40) : 40;
  const textH = typeof document !== "undefined" ? (text.height || 14) : 14;
  const bgW = textW + padX * 2;
  const bgH = textH + padY * 2;

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
