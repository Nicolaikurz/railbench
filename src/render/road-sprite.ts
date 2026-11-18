import { Graphics } from "pixi.js";
import type { CatalogEntry } from "@/catalog/catalog";
import { deg2rad } from "@/core/geometry";
import { drawArcStroke } from "./track-render-utils";

export function drawRoadElevationShadow(
  g: Graphics,
  entry: CatalogEntry,
  roadWidth: number,
  maxElev: number,
): void {
  const shadowDist = Math.min(maxElev * 0.35, 22);
  const shadowAlpha = Math.min(0.35, 0.12 + (maxElev / 100) * 0.2);
  const roadHalf = roadWidth / 2;
  const shadowColor = 0x000000;
  const kind = entry.kind;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
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
    if (a < 0) {
      drawArcStroke(
        g,
        shadowDist * 0.5,
        -r + shadowDist,
        r,
        Math.PI / 2,
        Math.PI / 2 - deg2rad(Math.abs(a)),
        roadWidth,
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
      roadWidth,
      shadowColor,
      false,
      shadowAlpha,
    );
    return;
  }

  if (kind === "turnout-left" || kind === "turnout-right") {
    const l = entry.length ?? 200;
    g.rect(shadowDist * 0.5, -roadHalf + shadowDist, l, roadWidth);
    if (entry.branchLength !== undefined) {
      const midX = l / 2;
      const bY = kind === "turnout-left" ? shadowDist : -entry.branchLength + shadowDist;
      g.rect(midX - roadHalf + shadowDist * 0.5, bY, roadWidth, entry.branchLength);
    } else if (entry.branchRadius !== undefined) {
      const br = entry.branchRadius;
      const ba = entry.branchAngle ?? 45;
      if (kind === "turnout-left") {
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
      } else {
        drawArcStroke(
          g,
          shadowDist * 0.5,
          -br + shadowDist,
          br,
          Math.PI / 2,
          Math.PI / 2 - deg2rad(ba),
          roadWidth,
          shadowColor,
          true,
          shadowAlpha,
        );
      }
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
export function drawRoadSelectionContour(
  g: Graphics,
  entry: CatalogEntry,
  contourWidth = 68,
): void {
  const kind = entry.kind;
  const glowColor = 0x0284c7;
  const contourHalf = contourWidth / 2;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
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
    if (a < 0) {
      const rad = deg2rad(Math.abs(a));
      drawArcStroke(g, 0, -r, r, Math.PI / 2, Math.PI / 2 - rad, contourWidth, glowColor, true, 0.15);
      drawArcStroke(g, 0, -r, r - contourHalf, Math.PI / 2, Math.PI / 2 - rad, 3.5, glowColor, true, 0.9);
      drawArcStroke(g, 0, -r, r + contourHalf, Math.PI / 2, Math.PI / 2 - rad, 3.5, glowColor, true, 0.9);
      return;
    }
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
export function drawRoadPiece2D(g: Graphics, entry: CatalogEntry, _isSelected: boolean): void {
  const kind = entry.kind;
  const roadWidth = entry.roadWidth ?? 60;
  const roadHalf = roadWidth / 2;

  const asphaltColor = 0x383a42;
  const curbColor = 0x22242a;
  const markingColor = 0xf8fafc;
  const lineW = 1.5;

  if (kind === "straight" || kind === "accessory" || kind === "flex") {
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
    if (a < 0) {
      const absA = Math.abs(a);
      const absRad = deg2rad(absA);
      drawArcStroke(g, 0, -r, r, Math.PI / 2, Math.PI / 2 - absRad, roadWidth, asphaltColor, true);
      drawArcStroke(g, 0, -r, r - roadHalf + 2.5, Math.PI / 2, Math.PI / 2 - absRad, lineW, markingColor, true, 0.92);
      drawArcStroke(g, 0, -r, r + roadHalf - 2.5, Math.PI / 2, Math.PI / 2 - absRad, lineW, markingColor, true, 0.92);

      const totalArc = absRad * r;
      const period = 14;
      const numDashes = Math.max(1, Math.floor(totalArc / period));
      const angStep = absRad / numDashes;
      const dashAng = (8 / totalArc) * absRad;
      for (let i = 0; i < numDashes; i++) {
        const startA = Math.PI / 2 - i * angStep - (angStep - dashAng) * 0.5;
        const endA = startA - dashAng;
        drawArcStroke(g, 0, -r, r, startA, endA, lineW, markingColor, true, 0.92);
      }
      return;
    }

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
