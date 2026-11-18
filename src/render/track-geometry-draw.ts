import { Graphics } from "pixi.js";
import type { CatalogEntry } from "@/catalog/catalog";
import { isStraightTrackEntry, isCurvedTrackEntry, deg2rad, evaluateFlexSpline, computeFlexSplineLength } from "@/core/geometry";
import { sampleFlexSplinePoints } from "@/core/flextrack";
import { drawArcStroke } from "./track-render-utils";

export function drawBallast(g: Graphics, entry: CatalogEntry, color: number, ballastWidth = 36): void {
  const kind = entry.kind;
  const ballastHalf = ballastWidth / 2;

  if (entry.customOffsets && entry.customOffsets.length >= 3) {
    const chordLen = entry.customChordDist ?? entry.length ?? 900;
    const pts = sampleFlexSplinePoints(chordLen, entry.customOffsets);
    if (pts.length > 1) {
      g.moveTo(pts[0]!.x, pts[0]!.y);
      for (let i = 1; i < pts.length; i++) {
        g.lineTo(pts[i]!.x, pts[i]!.y);
      }
      g.stroke({ width: ballastWidth, color, alpha: 0.95, cap: "butt", join: "round" });
    }
    return;
  }

  if (isStraightTrackEntry(entry)) {
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

  if (isCurvedTrackEntry(entry)) {
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

  if (kind === "turnout-right") {
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

  if (kind === "curved-turnout-left") {
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

  if (kind === "curved-turnout-right") {
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

  if (kind === "three-way") {
    const l = entry.length ?? 188.3;
    const br = entry.branchRadius ?? 437.5;
    const ba = entry.branchAngle ?? 24.3;
    g.rect(0, -ballastHalf, l, ballastWidth);
    g.fill({ color, alpha: 0.95 });
    g.stroke({ width: 1, color: 0x6c655b, alpha: 0.6, join: "miter" });
    drawArcStroke(g, 0, -br, br, Math.PI / 2, Math.PI / 2 - deg2rad(ba), ballastWidth, color, true);
    drawArcStroke(g, 0, br, br, -Math.PI / 2, -Math.PI / 2 + deg2rad(ba), ballastWidth, color);
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

export function drawTies(
  g: Graphics,
  entry: CatalogEntry,
  color: number,
  tieLenHalf = 13,
  tieWidth = 2.6,
  tieSpacing = 6.5,
): void {
  const kind = entry.kind;

  if (entry.customOffsets && entry.customOffsets.length >= 3) {
    const chordLen = entry.customChordDist ?? entry.length ?? 900;
    const totalLen = computeFlexSplineLength(chordLen, entry.customOffsets);
    const count = Math.max(3, Math.floor(totalLen / tieSpacing));
    for (let i = 1; i < count; i++) {
      const t = i / count;
      const { h, dh } = evaluateFlexSpline(entry.customOffsets, t);
      const x = t * chordLen;
      const y = h;
      const ang = Math.atan2(dh, chordLen);
      const nx = -Math.sin(ang);
      const ny = Math.cos(ang);
      g.moveTo(x - nx * tieLenHalf, y - ny * tieLenHalf);
      g.lineTo(x + nx * tieLenHalf, y + ny * tieLenHalf);
    }
    g.stroke({ width: tieWidth, color, cap: "butt" });
    return;
  }

  if (isStraightTrackEntry(entry)) {
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

  if (isCurvedTrackEntry(entry)) {
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

  if (kind === "curved-turnout-right" || (kind === "turnout-right" && entry.radius && !entry.length)) {
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
      const curAng = Math.PI / 2 - ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((br - tieLenHalf) * cos, -br + (br - tieLenHalf) * sin);
      g.lineTo((br + tieLenHalf) * cos, -br + (br + tieLenHalf) * sin);
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
      const curAng = -Math.PI / 2 + ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((br - tieLenHalf) * cos, br + (br - tieLenHalf) * sin);
      g.lineTo((br + tieLenHalf) * cos, br + (br + tieLenHalf) * sin);
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
      const curAng = Math.PI / 2 - ang;
      const cos = Math.cos(curAng);
      const sin = Math.sin(curAng);
      g.moveTo((br - tieLenHalf) * cos, -br + (br - tieLenHalf) * sin);
      g.lineTo((br + tieLenHalf) * cos, -br + (br + tieLenHalf) * sin);
    }

    // 3. Schwellen für Abzweigbogen R2 rechts (24224)
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
