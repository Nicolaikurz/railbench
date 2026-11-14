import type { Graphics } from "pixi.js";

/**
 * Bekohlungsanlage (Dampflok-BW):
 * Beton-Kohlenbansen mit schwarzen Steinkohlehaufen, Hochbunker mit Schütte,
 * Drehkran mit Gittermast-Ausleger & Greifer sowie Gelenk-Wasserkran.
 */
export function drawCoalingStationTopView(
  g: Graphics,
  w: number,
  d: number,
  halfW: number,
  halfD: number,
  isSelected: boolean,
): void {
  // Schlagschatten
  g.rect(-halfW + 3, -halfD + 4, w, d);
  g.fill({ color: 0x000000, alpha: 0.25 });

  // Fundamentplatte (Beton / Betriebshofboden)
  g.rect(-halfW, -halfD, w, d);
  g.fill({ color: 0x64748b });
  g.stroke({
    width: isSelected ? 2.5 : 1.4,
    color: isSelected ? 0x38bdf8 : 0x334155,
  });

  // 1. Kohlenbansen (links bis Mitte, Beton-Einfassung mit Steinkohle)
  const bunkerX = -halfW + 4;
  const bunkerY = -halfD + 5;
  const bunkerW = w * 0.56;
  const bunkerD = d * 0.68;

  g.rect(bunkerX, bunkerY, bunkerW, bunkerD);
  g.fill({ color: 0x18181b });
  g.stroke({ width: 2.2, color: 0x94a3b8 });

  // Mittel-Trennwand im Kohlenbansen
  g.moveTo(bunkerX + bunkerW * 0.52, bunkerY);
  g.lineTo(bunkerX + bunkerW * 0.52, bunkerY + bunkerD);
  g.stroke({ width: 1.8, color: 0x94a3b8 });

  // Unregelmäßige Steinkohle-Kegel (Anthrazit-Facetten)
  const coalMounds = [
    { cx: bunkerX + bunkerW * 0.26, cy: bunkerY + bunkerD * 0.48, r: Math.min(bunkerW * 0.2, bunkerD * 0.36) },
    { cx: bunkerX + bunkerW * 0.76, cy: bunkerY + bunkerD * 0.52, r: Math.min(bunkerW * 0.19, bunkerD * 0.35) },
  ];
  for (const m of coalMounds) {
    g.circle(m.cx, m.cy, m.r);
    g.fill({ color: 0x09090b });
    g.circle(m.cx - m.r * 0.2, m.cy - m.r * 0.15, m.r * 0.55);
    g.fill({ color: 0x27272a });
  }

  // 2. Hochbunker-Turm (rechts der Mitte) mit schräger Kohlenschütte nach vorn (+Y)
  const towerW = w * 0.18;
  const towerD = d * 0.42;
  const towerX = halfW * 0.32 - towerW / 2;
  const towerY = -halfD * 0.22 - towerD / 2;

  // Schütte (Rutsche Richtung Gleisseite +Y)
  g.rect(towerX + towerW * 0.22, towerY + towerD - 1, towerW * 0.56, d * 0.28);
  g.fill({ color: 0x334155 });
  g.stroke({ width: 1.0, color: 0x0f172a });

  // Hochbunker-Kasten
  g.rect(towerX, towerY, towerW, towerD);
  g.fill({ color: 0x475569 });
  g.stroke({ width: 1.5, color: 0x1e293b });

  // Innere Kohlenfüllung im Hochbunker
  g.rect(towerX + 2.5, towerY + 2.5, towerW - 5, towerD - 5);
  g.fill({ color: 0x09090b });

  // 3. Bekohlungs-Drehkran (Gittermast-Ausleger vom Kranhaus zum Bansen)
  const craneX = halfW * 0.02;
  const craneY = halfD * 0.22;
  const boomTipX = bunkerX + bunkerW * 0.68;
  const boomTipY = bunkerY + bunkerD * 0.45;

  // Kranhaus / Drehkranz
  g.circle(craneX, craneY, Math.min(w, d) * 0.12);
  g.fill({ color: 0x334155 });
  g.stroke({ width: 1.2, color: 0x0f172a });

  // Ausleger (Gittermast)
  g.moveTo(craneX, craneY);
  g.lineTo(boomTipX, boomTipY);
  g.stroke({ width: 2.2, color: 0x94a3b8 });

  // Zweischalengreifer an Auslegerspitze
  g.circle(boomTipX, boomTipY, 3.2);
  g.fill({ color: 0xd97706 });
  g.stroke({ width: 0.9, color: 0x0f172a });

  // 4. Gelenk-Wasserkran (rechts vorn an der Gleisseite)
  const wcX = halfW * 0.72;
  const wcY = halfD * 0.45;
  g.circle(wcX, wcY, 3.2);
  g.fill({ color: 0x1e293b });
  g.stroke({ width: 1.0, color: 0x94a3b8 });
  g.moveTo(wcX, wcY);
  g.lineTo(wcX - w * 0.07, wcY + d * 0.16);
  g.stroke({ width: 1.8, color: 0x38bdf8 });
}

/**
 * Dieseltankstelle (Diesellok-BW):
 * Beton-Auffangwanne mit Gitterrosten, liegender zylindrischer Haupt-Dieseltank
 * mit rotem Gefahrgutring, Pumpenhaus, Rohrbrücke und Wetterschutzdach über 2 Zapfsäulen.
 */
export function drawDieselFuelStationTopView(
  g: Graphics,
  w: number,
  d: number,
  halfW: number,
  halfD: number,
  isSelected: boolean,
): void {
  // Schlagschatten
  g.rect(-halfW + 3, -halfD + 4, w, d);
  g.fill({ color: 0x000000, alpha: 0.24 });

  // Fundamentplatte (Beton-Auffangwanne)
  g.rect(-halfW, -halfD, w, d);
  g.fill({ color: 0x94a3b8 });
  g.stroke({
    width: isSelected ? 2.5 : 1.4,
    color: isSelected ? 0x38bdf8 : 0x475569,
  });

  // 1. Liegender zylindrischer Haupt-Dieseltank (links hinten)
  const tankW = w * 0.42;
  const tankD = d * 0.46;
  const tankX = -halfW + 7;
  const tankY = -halfD + 6;

  g.roundRect(tankX, tankY, tankW, tankD, tankD * 0.35);
  g.fill({ color: 0xdbeafe });
  g.stroke({ width: 1.4, color: 0x475569 });

  // Roter Gefahrgut-/Warnring in der Tankmitte
  const bandW = Math.max(3.5, tankW * 0.1);
  g.rect(tankX + tankW / 2 - bandW / 2, tankY + 1, bandW, tankD - 2);
  g.fill({ color: 0xdc2626 });

  // Laufsteg oben auf dem Tank
  g.rect(tankX + tankW * 0.12, tankY + tankD * 0.38, tankW * 0.76, tankD * 0.24);
  g.fill({ color: 0x334155 });

  // 2. Pumpenhaus (Mitte-hinten) mit Rohrleitungsbrücke zum Tank
  const pumpW = w * 0.17;
  const pumpD = d * 0.34;
  const pumpX = tankX + tankW + w * 0.06;
  const pumpY = -halfD + 7;

  // Rohrleitungen zwischen Tank und Pumpenhaus
  for (const pyOff of [0.35, 0.65]) {
    g.moveTo(tankX + tankW, tankY + tankD * pyOff);
    g.lineTo(pumpX, tankY + tankD * pyOff);
    g.stroke({ width: 1.5, color: 0x475569 });
  }

  g.rect(pumpX, pumpY, pumpW, pumpD);
  g.fill({ color: 0x475569 });
  g.stroke({ width: 1.2, color: 0x1e293b });

  // 3. Gleisseitige Öl-Auffangwanne & Tankinsel mit Wetterschutzdach (rechts vorn)
  const canopyW = w * 0.46;
  const canopyD = d * 0.42;
  const canopyX = halfW - canopyW - 5;
  const canopyY = halfD - canopyD - 4;

  // Gelbe Sicherheitskante an der Gleisseite (+Y)
  g.moveTo(canopyX, halfD - 2);
  g.lineTo(canopyX + canopyW, halfD - 2);
  g.stroke({ width: 2.0, color: 0xfacc15 });

  // Wetterschutzdach mit blauem Randprofil
  g.rect(canopyX, canopyY, canopyW, canopyD);
  g.fill({ color: 0x475569, alpha: 0.9 });
  g.stroke({ width: 1.8, color: 0x0284c7 });

  // Zwei Zapfsäulen-Markierungen + gelbe Schwenkgalgen
  for (const dxFactor of [0.3, 0.7]) {
    const dispX = canopyX + canopyW * dxFactor;
    const dispY = canopyY + canopyD * 0.5;
    g.rect(dispX - 3.5, dispY - 2.5, 7, 5);
    g.fill({ color: 0x0284c7 });
    g.stroke({ width: 0.8, color: 0xffffff });

    // Schwenkgalgen Richtung Gleis (+Y)
    g.moveTo(dispX + 1.5, dispY);
    g.lineTo(dispX + 3, dispY + canopyD * 0.45);
    g.stroke({ width: 1.4, color: 0xfacc15 });
  }
}
