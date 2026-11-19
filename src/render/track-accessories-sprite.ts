import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { CatalogEntry } from "@/catalog/catalog";
import { getLocalConnectors } from "@/catalog/catalog";
import type { TrackPiece } from "@/core/types";
import type { PiecePowerResult } from "@/core/simulation";
import {
  deg2rad,
  getPieceLocalCenter,
  isStraightTrackEntry,
  isCurvedTrackEntry,
} from "@/core/geometry";
import { getEffectiveFlexEntry } from "@/core/flextrack";
import { drawTrackCenterline } from "./track-rails-draw";

const FEEDER_MARKER_TEXT_ACTIVE = new TextStyle({ fontSize: 13, fill: 0x1e293b, fontWeight: "bold" });
const FEEDER_MARKER_TEXT_GREY = new TextStyle({ fontSize: 13, fill: 0xe2e8f0, fontWeight: "bold" });
const BRIDGE_ICON_TEXT_STYLE = new TextStyle({ fontSize: 7, fill: 0xffffff });
const SIGNAL_TEXT_STYLE = new TextStyle({
  fontSize: 4,
  fontWeight: "bold",
  fill: 0xf8fafc,
  fontFamily: "monospace",
});

export function drawPowerFeederMarker(parent: Container, entry: CatalogEntry, isGrey = false): void {
  const center = getPieceLocalCenter(entry);
  const gauge = entry.gauge ?? 16.5;
  const hasRoadbed = entry.hasRoadbed ?? true;
  const ballastWidth = hasRoadbed ? (gauge >= 30 ? gauge * 2.18 : (gauge >= 16 ? 36 : gauge * 2.2)) : gauge * 1.5;
  const ballastHalf = ballastWidth / 2;
  const stemStartOffset = Math.max(12, ballastHalf - 6);
  const badgeOffset = Math.max(26, ballastHalf + 8);
  const badgeX = center.x;
  const badgeY = center.y - badgeOffset;

  // Verbindungssteg senkrecht zum Gleis (von Gleisrand bis Plakette)
  const stem = new Graphics();
  stem.moveTo(badgeX, center.y - stemStartOffset).lineTo(badgeX, badgeY + 11);
  stem.stroke({
    width: 2,
    color: isGrey ? 0x94a3b8 : 0xeab308,
    alpha: isGrey ? 0.6 : 0.9,
  });
  parent.addChild(stem);

  const marker = new Container();
  marker.position.set(badgeX, badgeY);
  marker.rotation = 0; // senkrecht zum Gleis

  const bg = new Graphics();
  bg.circle(1, 1, 11);
  bg.fill({ color: 0x000000, alpha: 0.35 });
  bg.circle(0, 0, 11);
  if (isGrey) {
    bg.fill({ color: 0x64748b, alpha: 0.75 });
    bg.stroke({ width: 1.5, color: 0x94a3b8 });
  } else {
    bg.fill({ color: 0xeab308, alpha: 0.95 });
    bg.stroke({ width: 2, color: 0xffffff });
  }
  marker.addChild(bg);

  const txt = new Text({
    text: "⚡",
    style: isGrey ? FEEDER_MARKER_TEXT_GREY : FEEDER_MARKER_TEXT_ACTIVE,
  });
  txt.anchor.set(0.5);
  marker.addChild(txt);

  parent.addChild(marker);
}

export function drawIsolatorMarker(parent: Container, entry: CatalogEntry, isoIdx = 0, isGrey = false): void {
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

  const gauge = entry.gauge ?? 16.5;
  const hasRoadbed = entry.hasRoadbed ?? true;
  const ballastWidth = hasRoadbed ? (gauge >= 30 ? gauge * 2.18 : (gauge >= 16 ? 36 : gauge * 2.2)) : gauge * 1.5;
  const ballastHalf = ballastWidth / 2;
  const cutHalf = Math.max(18, ballastHalf);
  const stemStartOffset = Math.max(12, ballastHalf - 6);
  const badgeOffset = Math.max(26, ballastHalf + 8);

  const cut = new Graphics();
  if (isGrey) {
    cut.moveTo(cx - perpX * cutHalf, cy - perpY * cutHalf).lineTo(cx + perpX * cutHalf, cy + perpY * cutHalf);
    cut.stroke({ width: 3, color: 0x64748b, alpha: 0.6 });
  } else {
    // Markante rote Trennlinie quer über das Gleisprofil
    cut.moveTo(cx - perpX * cutHalf, cy - perpY * cutHalf).lineTo(cx + perpX * cutHalf, cy + perpY * cutHalf);
    cut.stroke({ width: 4, color: 0xdc2626, alpha: 0.95 });
    // Helle Reflexionslinie
    cut.moveTo(cx + inwardX * 1.5 - perpX * cutHalf, cy + inwardY * 1.5 - perpY * cutHalf)
      .lineTo(cx + inwardX * 1.5 + perpX * cutHalf, cy + inwardY * 1.5 + perpY * cutHalf);
    cut.stroke({ width: 1.5, color: 0xffffff, alpha: 0.85 });
  }
  parent.addChild(cut);

  // Plakette senkrecht zum Gleis und über dem Gleis (oben: negatives y)
  const badgeX = cx;
  const badgeY = cy - badgeOffset;

  // Verbindungssteg senkrecht zum Gleis von der Trennlinie nach oben zur Plakette
  const stem = new Graphics();
  stem.moveTo(badgeX, cy - stemStartOffset).lineTo(badgeX, badgeY + 11);
  stem.stroke({
    width: 2,
    color: isGrey ? 0x94a3b8 : 0xdc2626,
    alpha: isGrey ? 0.6 : 0.9,
  });
  parent.addChild(stem);

  const marker = new Container();
  marker.position.set(badgeX, badgeY);
  marker.rotation = 0; // senkrecht zum Gleis

  const bg = new Graphics();
  bg.circle(1, 1, 11);
  bg.fill({ color: 0x000000, alpha: 0.45 });
  bg.circle(0, 0, 11);
  if (isGrey) {
    bg.fill({ color: 0x64748b, alpha: 0.75 });
    bg.stroke({ width: 1.5, color: 0x94a3b8 });
  } else {
    bg.fill({ color: 0xdc2626, alpha: 0.95 });
    bg.stroke({ width: 2, color: 0xffffff });
  }
  marker.addChild(bg);

  const txt = new Text({
    text: isGrey ? "⊘" : "⛔",
    style: new TextStyle({ fontSize: 13, fill: 0xffffff, fontWeight: "bold" }),
  });
  txt.anchor.set(0.5);
  marker.addChild(txt);

  parent.addChild(marker);
}

/**
 * Verhindert doppelte Oberleitungsmasten in der 2D-Ansicht an derselben Weltposition
 * (z. B. an verbundenen Gleisübergängen, Weichen oder Drehscheiben-Anschlüssen).
 */
export class CatenaryMast2DContext {
  private static readonly CELL = 60;
  private placedGrid = new Map<string, Array<{ x: number; y: number }>>();

  tryClaimMast(piece: TrackPiece, localX: number, localY: number, tol = 35): boolean {
    const rad = deg2rad(piece.transform.rotation);
    const cos = Math.cos(rad);
    const sin = Math.sin(rad);
    const wx = piece.transform.x + localX * cos - localY * sin;
    const wy = piece.transform.y + localX * sin + localY * cos;

    const cx = Math.floor(wx / CatenaryMast2DContext.CELL);
    const cy = Math.floor(wy / CatenaryMast2DContext.CELL);
    const tolSq = tol * tol;

    for (let dx = -1; dx <= 1; dx++) {
      for (let dy = -1; dy <= 1; dy++) {
        const list = this.placedGrid.get(`${cx + dx}:${cy + dy}`);
        if (!list) continue;
        for (const p of list) {
          const ddx = p.x - wx;
          const ddy = p.y - wy;
          if (ddx * ddx + ddy * ddy <= tolSq) {
            return false;
          }
        }
      }
    }

    const key = `${cx}:${cy}`;
    let bucket = this.placedGrid.get(key);
    if (!bucket) {
      bucket = [];
      this.placedGrid.set(key, bucket);
    }
    bucket.push({ x: wx, y: wy });
    return true;
  }
}

export function drawCatenary2D(
  parent: Container,
  entry: CatalogEntry,
  powerResult?: PiecePowerResult,
  piece?: TrackPiece,
  allPieces?: TrackPiece[],
  mastCtx?: CatenaryMast2DContext,
): void {
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

  // Streckenmasten mit Auslegern
  const drawMast = (x: number, y: number, angleRad: number, side: 1 | -1 = 1, deduplicate = true) => {
    if (deduplicate && mastCtx && piece && !mastCtx.tryClaimMast(piece, x, y)) {
      return;
    }

    const mastDist = Math.max(22, (entry.gauge ?? 16.5) * 1.15) * side;
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

  // 1. DREHSCHEIBE: Fahrdraht über die Bühne + NUR an den Snappoints mit angeschlossenem Oberleitungsgleis
  if (kind === "turntable") {
    const bridgeHalf = 131.5;
    // Fahrdraht über die Drehscheibenbühne
    g.moveTo(-bridgeHalf, 0).lineTo(bridgeHalf, 0);
    g.stroke({ width: wireWidth, color: wireColor, alpha: wireAlpha });

    // Masten an den beiden Bühnenenden (fest auf der Bühne, nicht mit Außengleisen deduplizieren)
    drawMast(-bridgeHalf, 0, Math.PI, 1, false);
    drawMast(bridgeHalf, 0, 0, 1, false);

    // Snappoints am Grubenrand: NUR rendern, wenn ein Gleis mit Oberleitung angeschlossen ist!
    if (piece?.connectors && allPieces) {
      const localConns = entry.localConnectors || [];
      piece.connectors.forEach((conn, idx) => {
        if (!conn.connectedTo) return;
        const other = allPieces.find((p) => p.id === conn.connectedTo!.pieceId);
        if (!other || !other.hasCatenary) return;

        const localConn = localConns[idx];
        const angleDeg = localConn ? localConn.angle : idx * 12;
        const rad = deg2rad(angleDeg);
        const cos = Math.cos(rad);
        const sin = Math.sin(rad);

        // Fahrdrahtstück über den Anschlussstutzen vom Grubenrand (142 mm) zum Außenrand (189 mm)
        g.moveTo(142 * cos, 142 * sin).lineTo(189 * cos, 189 * sin);
        g.stroke({ width: wireWidth, color: wireColor, alpha: wireAlpha });

        // Catenary-Mast am Außenrand
        drawMast(189 * cos, 189 * sin, rad, 1);
      });
    }
    parent.addChild(g);
    return;
  }

  // 2. SCHIEBEBÜHNE: Fahrdraht über die Schiebebühne + NUR an den Snappoints mit angeschlossenem Oberleitungsgleis
  if (kind === "transfer-table") {
    const l = entry.length ?? 360;
    const bridgeY = 154.8;

    // Fahrdraht über die Schiebebühne
    g.moveTo(36, bridgeY).lineTo(l - 36, bridgeY);
    g.stroke({ width: wireWidth, color: wireColor, alpha: wireAlpha });

    drawMast(36, bridgeY, Math.PI, 1, false);
    drawMast(l - 36, bridgeY, 0, 1, false);

    // Snappoints: NUR rendern, wenn ein Gleis mit Oberleitung angeschlossen ist!
    if (piece?.connectors && allPieces) {
      const localConns = entry.localConnectors || [];
      piece.connectors.forEach((conn, idx) => {
        if (!conn.connectedTo) return;
        const other = allPieces.find((p) => p.id === conn.connectedTo!.pieceId);
        if (!other || !other.hasCatenary) return;

        const localConn = localConns[idx];
        const isLeft = !localConn || localConn.x === 0;
        const y = localConn ? localConn.y : (idx < 5 ? idx * 77.4 : (idx - 5) * 77.4);

        if (isLeft) {
          g.moveTo(0, y).lineTo(36, y);
          g.stroke({ width: wireWidth, color: wireColor, alpha: wireAlpha });
          drawMast(0, y, Math.PI, 1);
        } else {
          g.moveTo(l - 36, y).lineTo(l, y);
          g.stroke({ width: wireWidth, color: wireColor, alpha: wireAlpha });
          drawMast(l, y, 0, 1);
        }
      });
    }
    parent.addChild(g);
    return;
  }

  // Standardgleise: Fahrdraht als Mittellinie
  drawTrackCenterline(g, entry, wireColor, wireWidth, wireAlpha);
  if (isStraightTrackEntry(entry)) {
    const l = entry.length ?? 188.3;
    drawMast(0, 0, 0, 1);
    if (l >= 150) {
      drawMast(l, 0, 0, 1);
    }
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const absA = Math.abs(a);
    const rad = deg2rad(absA);
    const curveSide: 1 | -1 = a < 0 ? -1 : 1;
    drawMast(0, 0, 0, curveSide);
    const endX = r * Math.sin(rad);
    const endY = a < 0 ? -r * (1 - Math.cos(rad)) : r * (1 - Math.cos(rad));
    drawMast(endX, endY, a < 0 ? -rad : rad, curveSide);
  } else if (kind === "curved-turnout-left" || (kind === "turnout-left" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const rad1 = deg2rad(a1);
    const rad2 = deg2rad(a2);
    const endX1 = r1 * Math.sin(rad1);
    const endY1 = -r1 * (1 - Math.cos(rad1));
    const endX2 = lead + r2 * Math.sin(rad2);
    const endY2 = -r2 * (1 - Math.cos(rad2));
    drawMast(0, 0, 0, 1);
    drawMast(endX1, endY1, -rad1, 1);
    drawMast(endX2, endY2, -rad2, -1);
  } else if (kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length)) {
    const r1 = entry.radius ?? 360;
    const a1 = entry.angle ?? 30;
    const lead = entry.branchLength ?? (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
    const r2 = entry.branchLength !== undefined ? (entry.branchRadius ?? r1) : (entry.branchRadius && entry.branchRadius > r1 ? r1 : (entry.branchRadius ?? r1));
    const a2 = entry.branchAngle ?? 30;
    const rad1 = deg2rad(a1);
    const rad2 = deg2rad(a2);
    const endX1 = r1 * Math.sin(rad1);
    const endY1 = r1 * (1 - Math.cos(rad1));
    const endX2 = lead + r2 * Math.sin(rad2);
    const endY2 = r2 * (1 - Math.cos(rad2));
    drawMast(0, 0, 0, -1);
    drawMast(endX1, endY1, rad1, -1);
    drawMast(endX2, endY2, rad2, 1);
  } else if (kind === "turnout-left") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const rad = deg2rad(ba);
    drawMast(0, 0, 0, 1);
    if (l >= 120) drawMast(l, 0, 0, 1);
    const endX = br * Math.sin(rad);
    const endY = -br * (1 - Math.cos(rad));
    drawMast(endX, endY, -rad, -1);
  } else if (kind === "turnout-right") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const rad = deg2rad(ba);
    drawMast(0, 0, 0, -1);
    if (l >= 120) drawMast(l, 0, 0, -1);
    const endX = br * Math.sin(rad);
    const endY = br * (1 - Math.cos(rad));
    drawMast(endX, endY, rad, 1);
  } else if (kind === "three-way") {
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    const rad = deg2rad(ba);
    drawMast(0, 0, 0, 1);
    const endXL = br * Math.sin(rad);
    const endYL = -br * (1 - Math.cos(rad));
    drawMast(endXL, endYL, -rad, -1);
    const endXR = br * Math.sin(rad);
    const endYR = br * (1 - Math.cos(rad));
    drawMast(endXR, endYR, rad, 1);
  } else {
    drawMast(0, 0, 0, 1);
  }

  parent.addChild(g);
}

export function drawCatenaryPowerFeederMarker(parent: Container, entry: CatalogEntry): void {
  const marker = new Container();
  const center = getPieceLocalCenter(entry);
  marker.position.set(center.x, center.y - 28);
  marker.rotation = 0; // senkrecht zum Gleis

  const g = new Graphics();
  // Zuleitungskabel von der Plakette senkrecht nach unten zum Fahrdraht
  g.moveTo(0, 11).lineTo(0, 28);
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

export function drawCatenaryIsolatorMarker(parent: Container, entry: CatalogEntry, isoIdx = 0): void {
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

  // Markante runde Plakette über der Oberleitungs-Trennstelle (senkrecht zum Gleis, oben)
  const marker = new Container();
  const badgeX = cx;
  const badgeY = cy - 28;
  marker.position.set(badgeX, badgeY);
  marker.rotation = 0; // senkrecht zum Gleis

  const gWire = new Graphics();
  gWire.moveTo(cx, cy - 12).lineTo(cx, cy - 17);
  gWire.stroke({ width: 1.5, color: 0x0284c7, alpha: 0.85 });
  parent.addChild(gWire);

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

export function drawTurnoutAccessories2D(container: Container, entry: CatalogEntry, piece: TrackPiece): void {
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

    // Leuchtendes Weichensignal (Abzweigpfeil links/rechts bzw. gerader Balken)
    const isRight = kind === "turnout-right" || kind === "curved-turnout-right";
    if (isLeft || isRight) {
      const dirY = isLeft ? -1 : 1;
      lanternBox.moveTo(lanternX - 2.2, lanternY + dirY * 1.4);
      lanternBox.lineTo(lanternX + 2.2, lanternY - dirY * 1.4);
      lanternBox.stroke({ width: 1.8, color: 0xfef9c3, alpha: 0.98 });
    } else {
      lanternBox.rect(lanternX - 0.9, lanternY - 2.4, 1.8, 4.8);
      lanternBox.fill({ color: 0xfef9c3, alpha: 0.98 });
    }
    // Strahlender Halo / Glüh-Effekt
    lanternBox.circle(lanternX, lanternY, 4.2);
    lanternBox.stroke({ width: 1.1, color: 0xfef08a, alpha: 0.45 });

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

export function drawBridge2D(container: Container, rawEntry: CatalogEntry, piece: TrackPiece): void {
  const entry = getEffectiveFlexEntry(rawEntry, piece);
  const bridgeG = new Graphics();
  const gauge = entry.gauge ?? 16.5;
  const bridgeHalfW = Math.max(18, gauge * 1.05);
  const girderColor = 0x334155; // Anthrazit / Stahlblau
  const strutColor = 0x64748b;  // Helles Stahlgrau
  const deckColor = 0x1e293b;   // Dunkler Brückendeck-Untergrund

  if (isStraightTrackEntry(entry)) {
    const l = entry.length ?? 188.3;
    // 1. Brückendeckplatte
    bridgeG.rect(0, -bridgeHalfW, l, bridgeHalfW * 2);
    bridgeG.fill({ color: deckColor, alpha: 0.25 });

    // 2. Seitliche Hauptträger / Geländer
    bridgeG.moveTo(0, -bridgeHalfW).lineTo(l, -bridgeHalfW);
    bridgeG.moveTo(0, bridgeHalfW).lineTo(l, bridgeHalfW);
    bridgeG.stroke({ width: 2.4, color: girderColor });

    // 3. Geländerpfosten / Fachwerkstreben
    const postStep = 18;
    for (let x = 4; x <= l - 4; x += postStep) {
      bridgeG.moveTo(x, -bridgeHalfW).lineTo(x, -bridgeHalfW + 3.5);
      bridgeG.moveTo(x, bridgeHalfW).lineTo(x, bridgeHalfW - 3.5);
      bridgeG.stroke({ width: 1.4, color: strutColor, alpha: 0.85 });
    }

    // 4. Widerlager (Start & Ende)
    bridgeG.rect(-2, -bridgeHalfW - 2, 4, bridgeHalfW * 2 + 4);
    bridgeG.rect(l - 2, -bridgeHalfW - 2, 4, bridgeHalfW * 2 + 4);
    bridgeG.fill({ color: 0x475569 });
    bridgeG.stroke({ width: 1.2, color: 0x1e293b });

    container.addChild(bridgeG);
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const absA = Math.abs(a);
    const rad = deg2rad(absA);
    const sign = a < 0 ? -1 : 1;

    const rInner = r - bridgeHalfW;
    const rOuter = r + bridgeHalfW;

    // Seitliche Bogen-Hauptträger
    const arcPoints = Math.max(12, Math.ceil(absA / 2.5));
    bridgeG.moveTo(rInner * Math.sin(0), sign * rInner * (1 - Math.cos(0)));
    for (let i = 1; i <= arcPoints; i++) {
      const curA = (rad * i) / arcPoints;
      bridgeG.lineTo(rInner * Math.sin(curA), sign * rInner * (1 - Math.cos(curA)));
    }
    bridgeG.stroke({ width: 2.4, color: girderColor });

    bridgeG.moveTo(rOuter * Math.sin(0), sign * rOuter * (1 - Math.cos(0)));
    for (let i = 1; i <= arcPoints; i++) {
      const curA = (rad * i) / arcPoints;
      bridgeG.lineTo(rOuter * Math.sin(curA), sign * rOuter * (1 - Math.cos(curA)));
    }
    bridgeG.stroke({ width: 2.4, color: girderColor });

    // Geländerpfosten entlang des Bogens
    const stepCount = Math.max(3, Math.floor(absA / 7.5));
    for (let i = 0; i <= stepCount; i++) {
      const curA = (rad * i) / stepCount;
      const xIn = rInner * Math.sin(curA);
      const yIn = sign * rInner * (1 - Math.cos(curA));
      const xOut = rOuter * Math.sin(curA);
      const yOut = sign * rOuter * (1 - Math.cos(curA));
      bridgeG.moveTo(xIn, yIn).lineTo(xOut, yOut);
      bridgeG.stroke({ width: 1.4, color: strutColor, alpha: 0.75 });
    }

    container.addChild(bridgeG);
  } else {
    // Fallback für Weichen oder sonstige Gleise
    const l = entry.length ?? 188.3;
    bridgeG.rect(0, -bridgeHalfW, l, bridgeHalfW * 2);
    bridgeG.fill({ color: deckColor, alpha: 0.2 });
    bridgeG.moveTo(0, -bridgeHalfW).lineTo(l, -bridgeHalfW);
    bridgeG.moveTo(0, bridgeHalfW).lineTo(l, bridgeHalfW);
    bridgeG.stroke({ width: 2.4, color: girderColor });
    container.addChild(bridgeG);
  }

  // Kleines Brücken-Symbol (🌉) als Orientierung
  const center = getPieceLocalCenter(entry);
  const badgeG = new Graphics();
  badgeG.circle(center.x, center.y - bridgeHalfW - 7, 5.5);
  badgeG.fill({ color: 0x0f172a, alpha: 0.95 });
  badgeG.stroke({ width: 1.2, color: 0x38bdf8 });

  const bridgeIcon = new Text({
    text: "🌉",
    style: BRIDGE_ICON_TEXT_STYLE,
  });
  bridgeIcon.anchor.set(0.5);
  bridgeIcon.position.set(center.x, center.y - bridgeHalfW - 7);
  container.addChild(badgeG);
  container.addChild(bridgeIcon);
}

export function drawTrackSignal2D(container: Container, rawEntry: CatalogEntry, piece: TrackPiece): void {
  const entry = getEffectiveFlexEntry(rawEntry, piece);
  const gauge = entry.gauge ?? 16.5;
  const dist = gauge * 0.95 + 7; // Seitlicher Signalabstand
  const sideConfig = piece.signalSide ?? "right";
  const dirConfig = piece.signalDirection ?? "forward";

  const sides: Array<1 | -1> =
    sideConfig === "both" ? [-1, 1] : sideConfig === "left" ? [1] : [-1];

  const directions: boolean[] =
    dirConfig === "both" ? [true, false] : [dirConfig === "forward"];

  for (const sideFactor of sides) {
    for (const isForward of directions) {
      drawSingleSignal2D(container, entry, dist, sideFactor, isForward);
    }
  }
}

function drawSingleSignal2D(
  container: Container,
  entry: CatalogEntry,
  dist: number,
  side: 1 | -1,
  isForward: boolean,
): void {
  let sigX = 28;
  let sigY = side * dist;
  let sigAngle = 0;

  if (isStraightTrackEntry(entry)) {
    const l = entry.length ?? 188.3;
    sigX = isForward ? Math.min(l * 0.35, 45) : l - Math.min(l * 0.35, 45);
    sigY = side * dist;
    sigAngle = isForward ? 0 : Math.PI;
  } else if (isCurvedTrackEntry(entry)) {
    const r = entry.radius ?? 437.5;
    const a = entry.angle ?? 30;
    const deg = Math.abs(a);
    const u = isForward ? 0.35 : 0.65;
    const rad = deg2rad(deg * u);
    const sign = a < 0 ? -1 : 1;
    const effR = r + side * dist;
    sigX = effR * Math.sin(rad);
    sigY = sign * effR * (1 - Math.cos(rad));
    const tangent = sign < 0 ? -rad : rad;
    sigAngle = isForward ? tangent : tangent + Math.PI;
  } else {
    const l = entry.length ?? 188.3;
    sigX = isForward ? 26 : l - 26;
    sigY = side * dist;
    sigAngle = isForward ? 0 : Math.PI;
  }

  const sigContainer = new Container();
  sigContainer.position.set(sigX, sigY);
  sigContainer.rotation = sigAngle;

  // 1. Ausleger / Verbindung zum Schotterbett
  const mastArm = new Graphics();
  mastArm.moveTo(0, -side * 5).lineTo(0, 0);
  mastArm.stroke({ width: 1.5, color: 0x334155, alpha: 0.85 });
  sigContainer.addChild(mastArm);

  // 2. Betonsockel
  const baseG = new Graphics();
  baseG.rect(-3.5, -3.5, 7, 7);
  baseG.fill({ color: 0x64748b });
  baseG.stroke({ width: 1, color: 0x1e293b });
  sigContainer.addChild(baseG);

  // 3. Signalschirm (Schwarzer, abgerundeter Schirm)
  const shieldG = new Graphics();
  shieldG.roundRect(-4.5, -9, 9, 18, 3.5);
  shieldG.fill({ color: 0x09090b });
  shieldG.stroke({ width: 1.2, color: 0x475569 });

  // 4. Rote Optik (Hp0 - Halt) oben
  shieldG.circle(0, -4.5, 2.4);
  shieldG.fill({ color: 0xef4444 });
  shieldG.circle(0, -4.5, 3.6);
  shieldG.stroke({ width: 0.8, color: 0xef4444, alpha: 0.5 });

  // 5. Grüne Optik (Hp1 - Fahrt) unten
  shieldG.circle(0, 4.5, 2.4);
  shieldG.fill({ color: 0x22c55e });
  shieldG.circle(0, 4.5, 3.6);
  shieldG.stroke({ width: 0.8, color: 0x22c55e, alpha: 0.5 });

  sigContainer.addChild(shieldG);

  // 6. Mini-Label "SIG" mit Pfeil
  const labelText = isForward ? "SIG▶" : "◀SIG";
  const sigTxt = new Text({
    text: labelText,
    style: SIGNAL_TEXT_STYLE,
  });
  sigTxt.anchor.set(0.5);
  sigTxt.position.set(0, 12);
  sigContainer.addChild(sigTxt);

  container.addChild(sigContainer);
}
