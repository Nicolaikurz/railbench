import { Graphics } from "pixi.js";
import type { CatalogEntry } from "@/catalog/catalog";
import { isStraightTrackEntry, isCurvedTrackEntry, deg2rad } from "@/core/geometry";
import { isBufferStopPiece, isFeederEntry } from "@/catalog/catalog";
import { drawArcStroke } from "./track-render-utils";

export function drawTrackCenterline(g: Graphics, entry: CatalogEntry, color: number, width: number, alpha: number): void {
  const kind = entry.kind;

  if (isStraightTrackEntry(entry)) {
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

  if (isCurvedTrackEntry(entry)) {
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
    drawArcStroke(g, 0, -r1, r1, Math.PI / 2, Math.PI / 2 - deg2rad(a1), width, color, true, alpha);
    if (lead > 0) {
      g.moveTo(0, 0).lineTo(lead, 0);
      g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    }
    drawArcStroke(g, lead, -r2, r2, Math.PI / 2, Math.PI / 2 - deg2rad(a2), width, color, true, alpha);
    return;
  }

  if (kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length)) {
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

  if (kind === "turnout-left") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.moveTo(0, 0).lineTo(l, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), width, color, true, alpha);
    return;
  }

  if (kind === "turnout-right") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.moveTo(0, 0).lineTo(l, 0);
    g.stroke({ width, color, alpha, cap: "butt", join: "miter" });
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), width, color, false, alpha);
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

export function drawRails(g: Graphics, entry: CatalogEntry, color: number, gaugeHalf: number): void {
  const kind = entry.kind;
  const railWidth = Math.max(1.1, gaugeHalf * 0.22);

  if (isStraightTrackEntry(entry)) {
    const l = entry.length ?? 188.3;
    g.moveTo(0, -gaugeHalf).lineTo(l, -gaugeHalf);
    g.moveTo(0, gaugeHalf).lineTo(l, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // Prellbock-Gleis: Pufferbohle und Puffer am Gleisende (x = l)
    if (isBufferStopPiece(entry)) {
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
    } else if (isFeederEntry(entry)) {
      // Anschlussgleis: Anschlussklemmen / Schraubklemmen am Gleisrand in der Mitte
      const midX = l / 2;
      const boxW = Math.max(8, gaugeHalf * 0.65);
      const boxH = Math.max(4.5, gaugeHalf * 0.42);
      const boxY = gaugeHalf + Math.max(1.5, gaugeHalf * 0.14);
      const termOffset = boxW * 0.26;
      const termRad = Math.max(1.2, boxH * 0.26);

      // Zuleitungen zu den beiden Schienenprofilen
      g.moveTo(midX - termOffset, boxY).lineTo(midX - termOffset, -gaugeHalf);
      g.moveTo(midX + termOffset, boxY).lineTo(midX + termOffset, gaugeHalf);
      g.stroke({ width: Math.max(1, railWidth * 0.55), color: 0xb45309, alpha: 0.85 });

      // Klemmengehäuse
      g.roundRect(midX - boxW / 2, boxY, boxW, boxH, 1.5);
      g.fill({ color: 0x1e293b });
      g.stroke({ width: 0.9, color: 0x475569 });

      // Zwei Messing-/Kupfer-Schraubklemmen
      g.circle(midX - termOffset, boxY + boxH / 2, termRad);
      g.circle(midX + termOffset, boxY + boxH / 2, termRad);
      g.fill({ color: 0xf59e0b });
    } else if (kind === "uncoupler") {
      // Entkupplungsgleis: Entkupplungsbohle zwischen den Schienen
      const plateLen = Math.min(l * 0.42, Math.max(22, gaugeHalf * 3.2));
      const plateH = Math.max(4, gaugeHalf * 1.15);
      g.roundRect((l - plateLen) / 2, -plateH / 2, plateLen, plateH, 1.5);
      g.fill({ color: 0x475569, alpha: 0.9 });
      g.stroke({ width: 0.8, color: 0x94a3b8 });
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

  if (isCurvedTrackEntry(entry)) {
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

  if (kind === "turnout-right") {
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

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    // Stammgleis (24188)
    g.moveTo(0, -gaugeHalf).lineTo(l, -gaugeHalf);
    g.moveTo(0, gaugeHalf).lineTo(l, gaugeHalf);
    g.stroke({ width: railWidth, color, cap: "butt", join: "miter" });

    // Abzweigbogen R2 links (24224)
    const endL = Math.PI / 2 - deg2rad(ba);
    drawArcStroke(g, 0, -br, br - gaugeHalf, Math.PI / 2, endL, railWidth, color, true);
    drawArcStroke(g, 0, -br, br + gaugeHalf, Math.PI / 2, endL, railWidth, color, true);

    // Abzweigbogen R2 rechts (24224)
    const endR = -Math.PI / 2 + deg2rad(ba);
    drawArcStroke(g, 0, br, br - gaugeHalf, -Math.PI / 2, endR, railWidth, color);
    drawArcStroke(g, 0, br, br + gaugeHalf, -Math.PI / 2, endR, railWidth, color);
    return;
  }

  if (kind === "curved-turnout-left") {
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

  if (kind === "curved-turnout-right") {
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

export function drawSelectionContour(g: Graphics, entry: CatalogEntry, contourWidth = 44): void {
  const kind = entry.kind;
  const glowColor = 0x0284c7;
  const glowAlpha = 0.5;
  const contourHalf = contourWidth / 2;

  if (isStraightTrackEntry(entry)) {
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

  if (isCurvedTrackEntry(entry)) {
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

  if (kind === "turnout-right") {
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

  if (kind === "curved-turnout-left") {
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

  if (kind === "curved-turnout-right") {
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
