import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { Model3D } from "@/core/types";
import { deg2rad, getUprightTextRotation } from "@/core/geometry";
import { findBuiltinModelDef } from "@/core/models-3d";

export function createModel2DGraphics(model: Model3D, isSelected: boolean): Container {
  const container = new Container();
  container.position.set(model.transform.x, model.transform.y);
  container.rotation = deg2rad(model.transform.rotation);

  const scale = model.scale ?? 1.0;
  const w = Math.max(10, model.width * scale);
  const d = Math.max(10, model.depth * scale);

  const halfW = w / 2;
  const halfD = d / 2;

  const g = new Graphics();

  let icon = "🏢";
  if (model.modelType === "builtin" && model.builtinId) {
    const def = findBuiltinModelDef(model.builtinId);
    if (def) icon = def.icon;
  } else if (model.category === "nature") {
    icon = "🌲";
  } else if (model.category === "railway") {
    icon = "🚂";
  } else if (model.category === "custom") {
    icon = "📦";
  }

  // 1. Architektonische Draufsicht (Top View) je Modelltyp zeichnen
  const builtinId = model.builtinId;
  if (builtinId === "turntable-trainhall") {
    drawTrainhallTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "station") {
    drawStationTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "signal-box") {
    drawSignalBoxTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "engine-shed") {
    drawEngineShedTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "town-house") {
    drawTownHouseTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "industry-building") {
    drawIndustryBuildingTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "skyscraper") {
    drawSkyscraperTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "water-tower") {
    drawWaterTowerTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "pine-tree") {
    drawPineTreeTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "deciduous-tree") {
    drawDeciduousTreeTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "light-signal") {
    drawLightSignalTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "buffer-stop") {
    drawBufferStopTopView(g, w, d, halfW, halfD, isSelected);
  } else if (builtinId === "platform-lamp") {
    drawPlatformLampTopView(g, w, d, halfW, halfD, isSelected);
  } else {
    drawGenericBuildingTopView(g, w, d, halfW, halfD, model.category, isSelected);
  }

  // 2. Wenn selektiert: Rotations-Griff zeichnen (oben)
  if (isSelected) {
    const handleDist = halfD + 18;

    // Verbindungslinie
    g.moveTo(0, -halfD);
    g.lineTo(0, -handleDist);
    g.stroke({ width: 1.5, color: 0x38bdf8, alpha: 0.85 });

    // Drehknopf
    g.circle(0, -handleDist, 6);
    g.fill({ color: 0x38bdf8 });
    g.stroke({ width: 1.5, color: 0xffffff });

    // Pfeil-Symbol im Knopf
    g.circle(0, -handleDist, 2);
    g.fill({ color: 0x0f172a });
  }

  container.addChild(g);

  // 3. Text-Beschriftung im Modell-Zentrum (stets horizontal und aufrecht per Gegen-Rotation)
  if (w >= 35 && d >= 22) {
    const rot = model.transform.rotation ?? 0;
    const rad = deg2rad(rot);
    const cosA = Math.abs(Math.cos(rad));
    const sinA = Math.abs(Math.sin(rad));
    // Verfügbare horizontale Spannweite im gedrehten Gebäudekörper
    const maxHorizWidth = Math.min(
      cosA > 0.001 ? w / cosA : Infinity,
      sinA > 0.001 ? d / sinA : Infinity,
    );
    const wrapWidth = Math.max(28, Math.min(maxHorizWidth - 8, Math.max(w, d)));

    const style = new TextStyle({
      fontFamily: "system-ui, -apple-system, sans-serif",
      fontSize: Math.min(10, Math.max(8, Math.min(w / 12, d / 3.5))),
      fontWeight: "bold",
      fill: isSelected ? "#38bdf8" : "#ffffff",
      dropShadow: {
        color: "#000000",
        blur: 3,
        alpha: 0.8,
        distance: 1,
      },
      align: "center",
      wordWrap: true,
      wordWrapWidth: wrapWidth,
    });

    const labelCont = new Container();
    labelCont.rotation = getUprightTextRotation(rot);

    const labelText = new Text({
      text: `${icon} ${model.name}`,
      style,
    });
    labelText.anchor.set(0.5, 0.5);

    // Kompaktes, dezentes dunkles Hintergrund-Badge für saubere Lesbarkeit auf dem Dach
    const padX = 5;
    const padY = 2;
    let textW = 40;
    let textH = 12;
    try {
      if (typeof document !== "undefined") {
        textW = labelText.width || 40;
        textH = labelText.height || 12;
      }
    } catch {
      textW = 40;
      textH = 12;
    }
    const bgW = textW + padX * 2;
    const bgH = textH + padY * 2;
    const bg = new Graphics();
    bg.roundRect(-bgW / 2 + 0.5, -bgH / 2 + 1, bgW, bgH, 3);
    bg.fill({ color: 0x000000, alpha: 0.35 });
    bg.roundRect(-bgW / 2, -bgH / 2, bgW, bgH, 3);
    bg.fill({ color: 0x0f172a, alpha: 0.78 });
    bg.stroke({ width: 1, color: isSelected ? 0x38bdf8 : 0x475569, alpha: 0.85 });

    labelCont.addChild(bg);
    labelCont.addChild(labelText);
    container.addChild(labelCont);
  }

  return container;
}

/** Bahnhof Neustadt: Draufsicht mit Hauptflügel, Seitenflügeln, Uhrenturm & Bahnsteigdach */
function drawStationTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  const mainW = w * 0.44;
  const wingW = (w - mainW) / 2;
  const canopyD = d * 0.26;
  const buildingD = d - canopyD;
  const bHalfD = buildingD / 2;
  const yShift = -canopyD / 2;

  // 1. Weicher Gebäudeschatten
  g.roundRect(-halfW + 3, -halfD + 4, w, d, 4);
  g.fill({ color: 0x000000, alpha: 0.22 });

  const roofRedLight = 0xc2410c;
  const roofRedDark = 0x9a3412;
  const ridgeColor = 0x7c2d12;

  // 2. Seitenflügel links (Satteldach mit First)
  const lX = -halfW;
  g.rect(lX, yShift - bHalfD + 4, wingW, buildingD - 8);
  g.fill({ color: roofRedDark });
  g.poly([lX, yShift - bHalfD + 4, lX + wingW, yShift - bHalfD + 4, lX + wingW, yShift, lX, yShift]);
  g.fill({ color: roofRedLight });
  g.moveTo(lX + 6, yShift).lineTo(lX + wingW, yShift);
  g.stroke({ width: 1.5, color: ridgeColor });

  // 3. Seitenflügel rechts (Satteldach mit First)
  const rX = halfW - wingW;
  g.rect(rX, yShift - bHalfD + 4, wingW, buildingD - 8);
  g.fill({ color: roofRedDark });
  g.poly([rX, yShift - bHalfD + 4, rX + wingW, yShift - bHalfD + 4, rX + wingW, yShift, rX, yShift]);
  g.fill({ color: roofRedLight });
  g.moveTo(rX, yShift).lineTo(rX + wingW - 6, yShift);
  g.stroke({ width: 1.5, color: ridgeColor });

  // 4. Haupttrakt mittig (höheres Walmdach / First)
  const cLeft = -mainW / 2;
  const cRight = mainW / 2;
  const cTop = yShift - bHalfD;
  const cBottom = yShift + bHalfD;

  // Sonnenseite (oben)
  g.poly([cLeft, cTop, cRight, cTop, cRight - 10, yShift, cLeft + 10, yShift]);
  g.fill({ color: 0xea580c });
  // Schattenseite (unten)
  g.poly([cLeft + 10, yShift, cRight - 10, yShift, cRight, cBottom, cLeft, cBottom]);
  g.fill({ color: 0x7c2d12 });
  // Walmdreiecke links & rechts
  g.poly([cLeft, cTop, cLeft + 10, yShift, cLeft, cBottom]);
  g.fill({ color: 0xb45309 });
  g.poly([cRight, cTop, cRight, cBottom, cRight - 10, yShift]);
  g.fill({ color: 0x9a3412 });

  // Dachfirst
  g.moveTo(cLeft + 10, yShift).lineTo(cRight - 10, yShift);
  g.stroke({ width: 2.2, color: 0x431407 });

  // Uhrenturm (Kupferdach mit Spitze)
  const towerS = Math.min(22, mainW * 0.22);
  g.rect(-towerS / 2, yShift - towerS / 2, towerS, towerS);
  g.fill({ color: 0x0f766e });
  g.stroke({ width: 1.2, color: 0x134e4a });
  g.circle(0, yShift, 2.5);
  g.fill({ color: 0xfbbf24 });

  // Schornsteine auf den Flügeln
  for (const cx of [lX + wingW * 0.4, rX + wingW * 0.6]) {
    g.rect(cx - 3, yShift - 8, 6, 5);
    g.fill({ color: 0x475569 });
    g.stroke({ width: 0.8, color: 0x1e293b });
    g.circle(cx, yShift - 5.5, 1.2);
    g.fill({ color: 0x0f172a });
  }

  // 5. Bahnsteigdach (Platform canopy) auf der Gleisseite
  const canY = cBottom;
  const canH = canopyD;
  g.rect(-halfW + 4, canY, w - 8, canH);
  g.fill({ color: 0x0284c7, alpha: 0.35 });
  g.stroke({ width: 1.5, color: 0x0369a1, alpha: 0.85 });

  // Stützenträger / Glasscheiben-Muster
  const beamStep = Math.max(16, w / 12);
  for (let bx = -halfW + 16; bx < halfW - 10; bx += beamStep) {
    g.moveTo(bx, canY).lineTo(bx, canY + canH);
  }
  g.stroke({ width: 0.9, color: 0x38bdf8, alpha: 0.7 });

  // Gesamtkontur
  g.roundRect(-halfW, yShift - bHalfD + 4, w, buildingD - 4, 2);
  g.stroke({ width: isSelected ? 2.5 : 1.2, color: isSelected ? 0x38bdf8 : 0x7c2d12 });
}

/** Stellwerk W1: Draufsicht mit Treppenaufgang, Schieferdach, Firstlinie, Erker & Schornstein */
function drawSignalBoxTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  g.roundRect(-halfW + 2, -halfD + 3, w, d, 3);
  g.fill({ color: 0x000000, alpha: 0.22 });

  const stairW = w * 0.22;
  const sLeft = -halfW;
  g.rect(sLeft, -halfD + 6, stairW, d - 12);
  g.fill({ color: 0x64748b });
  for (let sy = -halfD + 10; sy < halfD - 8; sy += 5) {
    g.moveTo(sLeft, sy).lineTo(sLeft + stairW, sy);
  }
  g.stroke({ width: 0.8, color: 0x334155, alpha: 0.7 });

  const bLeft = sLeft + stairW;
  const bW = w - stairW;
  const bRight = halfW;

  // Schieferdach
  g.poly([bLeft, -halfD, bRight, -halfD, bRight - 6, 0, bLeft + 4, 0]);
  g.fill({ color: 0x475569 });
  g.poly([bLeft + 4, 0, bRight - 6, 0, bRight, halfD - 6, bLeft, halfD - 6]);
  g.fill({ color: 0x1e293b });

  g.moveTo(bLeft + 4, 0).lineTo(bRight - 6, 0);
  g.stroke({ width: 1.5, color: 0x0f172a });

  // Panoramaglas-Kanzelüberstand zum Gleis
  g.rect(bLeft + 6, halfD - 7, bW - 12, 7);
  g.fill({ color: 0x38bdf8, alpha: 0.5 });
  g.stroke({ width: 1.2, color: 0x0284c7 });

  // Schornstein & Funkantenne
  g.rect(bRight - 16, -halfD + 5, 6, 5);
  g.fill({ color: 0xb45309 });
  g.stroke({ width: 0.8, color: 0x78350f });

  g.circle(bLeft + 10, -4, 2);
  g.fill({ color: 0x94a3b8 });
  g.stroke({ width: 0.8, color: 0x0f172a });

  g.rect(bLeft, -halfD, bW, d - 6);
  g.stroke({ width: isSelected ? 2.5 : 1.2, color: isSelected ? 0x38bdf8 : 0x0f172a });
}

/** Lokschuppen: Draufsicht mit Ziegeldach, First-Oberlicht, Rauchabzügen & weit geöffneten Toren */
function drawEngineShedTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  g.roundRect(-halfW + 3, -halfD + 3, w, d, 3);
  g.fill({ color: 0x000000, alpha: 0.22 });

  // Durchgehende Ziegeldachflächen (Nord- & Südhälfte mit First)
  g.rect(-halfW, -halfD, w, halfD);
  g.fill({ color: 0xb45309 });
  g.rect(-halfW, 0, w, halfD);
  g.fill({ color: 0x7c2d12 });

  // Firstlinie
  g.moveTo(-halfW, 0).lineTo(halfW, 0);
  g.stroke({ width: 2.2, color: 0x431407 });

  // First-Oberlichtband (Verglaster Dachreiter)
  const skyW = w * 0.6;
  g.rect(-skyW / 2, -5, skyW, 10);
  g.fill({ color: 0x38bdf8, alpha: 0.5 });
  g.stroke({ width: 1.0, color: 0x0284c7, alpha: 0.85 });

  // Subtil sichtbare Schienen unter dem Oberlicht im Innenraum
  g.moveTo(-skyW / 2 + 4, -3.5).lineTo(skyW / 2 - 4, -3.5);
  g.stroke({ width: 1.0, color: 0x64748b, alpha: 0.6 });
  g.moveTo(-skyW / 2 + 4, 3.5).lineTo(skyW / 2 - 4, 3.5);
  g.stroke({ width: 1.0, color: 0x64748b, alpha: 0.6 });

  // Glassprossen
  for (let sx = -skyW / 2 + 10; sx < skyW / 2; sx += 12) {
    g.moveTo(sx, -5).lineTo(sx, 5);
  }
  g.stroke({ width: 0.6, color: 0x0369a1, alpha: 0.6 });

  // 3 Runde Rauchabzugshauben auf dem Dach
  for (const hx of [-w / 3, 0, w / 3]) {
    g.circle(hx, -halfD * 0.38, 4.5);
    g.fill({ color: 0x334155 });
    g.stroke({ width: 1.0, color: 0x0f172a });
    g.circle(hx, -halfD * 0.38, 1.8);
    g.fill({ color: 0x020617 });
  }

  // Gleisstränge am Einfahrtstor
  g.moveTo(-halfW - 14, -4.5).lineTo(-halfW + 4, -4.5);
  g.stroke({ width: 1.4, color: 0x94a3b8 });
  g.moveTo(-halfW - 14, 4.5).lineTo(-halfW + 4, 4.5);
  g.stroke({ width: 1.4, color: 0x94a3b8 });

  // Sandstein-Torbogen / Schwellen-Kante am Portal
  const doorHalfW = Math.min(22, halfD * 0.62);
  g.moveTo(-halfW, -doorHalfW).lineTo(-halfW, doorHalfW);
  g.stroke({ width: 2.2, color: 0xc4b5a5 });

  // Weit geöffnete Torflügel links & rechts (nach außen geschwenkt vor der Wand)
  // Linker Flügel (nach außen/oben geschwenkt)
  g.moveTo(-halfW, -doorHalfW).lineTo(-halfW - 6, -doorHalfW - 8);
  g.stroke({ width: 2.4, color: 0x78350f });
  g.circle(-halfW, -doorHalfW, 1.8);
  g.fill({ color: 0x0f172a });

  // Rechter Flügel (nach außen/unten geschwenkt)
  g.moveTo(-halfW, doorHalfW).lineTo(-halfW - 6, doorHalfW + 8);
  g.stroke({ width: 2.4, color: 0x78350f });
  g.circle(-halfW, doorHalfW, 1.8);
  g.fill({ color: 0x0f172a });

  // Gesamtkontur
  g.rect(-halfW, -halfD, w, d);
  g.stroke({ width: isSelected ? 2.5 : 1.4, color: isSelected ? 0x38bdf8 : 0x431407 });
}

/** Fachwerkhaus: Draufsicht mit Ziegeldach, Dachgauben, Schornstein & Firstlinie */
function drawTownHouseTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  g.roundRect(-halfW + 3, -halfD + 3, w, d, 2);
  g.fill({ color: 0x000000, alpha: 0.24 });

  // Satteldach Ziegel
  g.rect(-halfW, -halfD, w, halfD);
  g.fill({ color: 0xea580c });
  g.rect(-halfW, 0, w, halfD);
  g.fill({ color: 0x9a3412 });

  g.moveTo(-halfW, 0).lineTo(halfW, 0);
  g.stroke({ width: 2.2, color: 0x431407 });

  // Ziegellinien
  for (let gy = -halfD + 8; gy < halfD; gy += 9) {
    if (Math.abs(gy) < 3) continue;
    g.moveTo(-halfW + 2, gy).lineTo(halfW - 2, gy);
  }
  g.stroke({ width: 0.5, color: 0x431407, alpha: 0.25 });

  // Dachgauben
  for (const gx of [-halfW * 0.45, halfW * 0.45]) {
    g.poly([gx - 7, -halfD + 4, gx + 7, -halfD + 4, gx, -halfD + 16]);
    g.fill({ color: 0xf97316 });
    g.stroke({ width: 0.9, color: 0x7c2d12 });
  }

  // Schornstein
  const cx = halfW * 0.2;
  const cy = -3;
  g.rect(cx - 4, cy - 3, 8, 6);
  g.fill({ color: 0xb45309 });
  g.stroke({ width: 1.0, color: 0x451a03 });
  g.rect(cx - 2, cy - 1.5, 4, 3);
  g.fill({ color: 0x18181b });

  g.rect(-halfW, -halfD, w, d);
  g.stroke({ width: isSelected ? 2.5 : 1.2, color: isSelected ? 0x38bdf8 : 0x431407 });
}

/** Industriehalle / Fabrik: Draufsicht mit Sheddach (Sägezahndach), Laderampe & Stahlsilo */
function drawIndustryBuildingTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  g.roundRect(-halfW + 3, -halfD + 4, w, d, 3);
  g.fill({ color: 0x000000, alpha: 0.22 });

  const siloR = Math.min(halfD * 0.65, 26);
  const hallW = w - siloR * 2 - 8;
  const hLeft = -halfW;
  const hRight = hLeft + hallW;

  // 4 Sheddach-Segmente mit Nord-Fensterstreifen
  const bays = 4;
  const bayW = hallW / bays;
  for (let b = 0; b < bays; b++) {
    const bx = hLeft + b * bayW;
    g.rect(bx, -halfD, bayW * 0.78, d);
    g.fill({ color: b % 2 === 0 ? 0x475569 : 0x334155 });
    g.rect(bx + bayW * 0.78, -halfD, bayW * 0.22, d);
    g.fill({ color: 0x7dd3fc, alpha: 0.6 });
    g.stroke({ width: 0.6, color: 0x0284c7 });
  }

  // Laderampe
  g.rect(hLeft, halfD - 8, hallW, 8);
  g.fill({ color: 0x94a3b8 });
  g.stroke({ width: 1.0, color: 0x475569 });

  // Stahlsilo
  const scX = halfW - siloR;
  const scY = 0;
  g.circle(scX, scY, siloR);
  g.fill({ color: 0x94a3b8 });
  g.stroke({ width: 1.5, color: 0x334155 });
  g.circle(scX, scY, siloR * 0.55);
  g.fill({ color: 0x64748b });
  g.circle(scX, scY, 2.5);
  g.fill({ color: 0x1e293b });

  // Rohrleitung
  g.moveTo(scX - siloR, 0).lineTo(hRight, 0);
  g.stroke({ width: 2.5, color: 0x0f172a });

  g.rect(hLeft, -halfD, hallW, d);
  g.stroke({ width: isSelected ? 2.5 : 1.3, color: isSelected ? 0x38bdf8 : 0x1e293b });
}

/** Wolkenkratzer: Draufsicht mit Attika, Flachdach, Penthouse, Klimaanlagen & Antenne */
function drawSkyscraperTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  g.roundRect(-halfW + 4, -halfD + 4, w, d, 2);
  g.fill({ color: 0x000000, alpha: 0.28 });

  // Glasfassaden-Attika
  g.rect(-halfW, -halfD, w, d);
  g.fill({ color: 0x0284c7, alpha: 0.35 });
  g.stroke({ width: 2.0, color: 0x0369a1 });

  // Teerdachfläche
  const inset = 6;
  g.rect(-halfW + inset, -halfD + inset, w - inset * 2, d - inset * 2);
  g.fill({ color: 0x1e293b });
  g.stroke({ width: 1.0, color: 0x334155 });

  // Aufzugsschacht
  const pW = w * 0.38;
  const pD = d * 0.35;
  g.rect(-pW / 2, -pD / 2 - 4, pW, pD);
  g.fill({ color: 0x475569 });
  g.stroke({ width: 1.2, color: 0x0f172a });

  // Klimaanlagen
  const hvacW = Math.min(18, w * 0.2);
  const hvacD = Math.min(12, d * 0.2);
  g.rect(-halfW + inset + 4, halfD - inset - hvacD - 3, hvacW, hvacD);
  g.fill({ color: 0x94a3b8 });
  g.stroke({ width: 0.8, color: 0x334155 });
  g.rect(-halfW + inset + 4 + hvacW + 4, halfD - inset - hvacD - 3, hvacW, hvacD);
  g.fill({ color: 0x94a3b8 });
  g.stroke({ width: 0.8, color: 0x334155 });

  // Antennenmast mit roter Leuchte
  g.circle(halfW - inset - 8, -halfD + inset + 8, 3);
  g.fill({ color: 0xef4444 });
  g.stroke({ width: 1.0, color: 0xffffff });

  if (isSelected) {
    g.rect(-halfW, -halfD, w, d);
    g.stroke({ width: 2.5, color: 0x38bdf8 });
  }
}

/** Ringlokschuppen: Fächerförmige Draufsicht mit 3 Gleisachsen, weit geöffneten Toren & Rauchabzügen */
function drawTrainhallTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  const fw = w * 0.38;
  g.poly([
    -fw / 2 + 3, -halfD + 4,
    fw / 2 + 3, -halfD + 4,
    halfW + 3, halfD + 4,
    -halfW + 3, halfD + 4,
  ]);
  g.fill({ color: 0x000000, alpha: 0.22 });

  g.poly([
    -fw / 2, -halfD,
    fw / 2, -halfD,
    halfW, halfD,
    -halfW, halfD,
  ]);
  g.fill({ color: 0x0f766e, alpha: 0.85 });
  g.stroke({
    width: isSelected ? 2.5 : 1.5,
    color: isSelected ? 0x38bdf8 : 0x134e4a,
    alpha: 0.95,
  });

  for (const st of [-1, 0, 1]) {
    const xFront = (fw * 0.32) * st;
    const xRear = (halfW * 0.65) * st;

    // Gleisachse durch die Halle
    g.moveTo(xFront, -halfD - 4);
    g.lineTo(xRear, halfD - 6);
    g.stroke({ width: 1.5, color: 0x0f172a, alpha: 0.7 });

    // Rauchabzug auf dem First
    const xMid = (xFront + xRear) / 2;
    g.circle(xMid, 0, 4.5);
    g.fill({ color: 0x334155 });
    g.stroke({ width: 0.9, color: 0x0f172a });
    g.circle(xMid, 0, 1.8);
    g.fill({ color: 0x020617 });

    // Tordurchfahrt (Dunkler Ausschnitt)
    const doorGap = 7;
    g.rect(xFront - doorGap, -halfD - 1, doorGap * 2, 4);
    g.fill({ color: 0x18181b });

    // Weit geöffnete Torflügel links & rechts jedes Standes
    // Linker Torflügel (nach außen geschwenkt)
    g.moveTo(xFront - doorGap, -halfD);
    g.lineTo(xFront - doorGap - 4, -halfD - 5);
    g.stroke({ width: 2.0, color: 0x78350f });
    g.circle(xFront - doorGap, -halfD, 1.3);
    g.fill({ color: 0x0f172a });

    // Rechter Torflügel (nach außen geschwenkt)
    g.moveTo(xFront + doorGap, -halfD);
    g.lineTo(xFront + doorGap + 4, -halfD - 5);
    g.stroke({ width: 2.0, color: 0x78350f });
    g.circle(xFront + doorGap, -halfD, 1.3);
    g.fill({ color: 0x0f172a });
  }
}

/** Wasserturm: Runder Kessel, Kegeldach & schwenkbarer Wasserkranausleger */
function drawWaterTowerTopView(g: Graphics, _w: number, _d: number, halfW: number, halfD: number, isSelected: boolean) {
  const r = Math.min(halfW, halfD);
  g.circle(2, 3, r);
  g.fill({ color: 0x000000, alpha: 0.25 });

  g.circle(0, 0, r);
  g.fill({ color: 0x0369a1 });
  g.stroke({ width: 2.0, color: 0x075985 });

  g.circle(0, 0, r * 0.65);
  g.stroke({ width: 1.0, color: 0x0c4a6e, alpha: 0.6 });
  g.circle(0, 0, r * 0.35);
  g.fill({ color: 0x0284c7 });
  g.stroke({ width: 1.0, color: 0x0c4a6e });

  g.circle(0, 0, 3.5);
  g.fill({ color: 0xf59e0b });
  g.stroke({ width: 1.0, color: 0x78350f });

  g.moveTo(0, 0).lineTo(r + 14, 0);
  g.stroke({ width: 2.5, color: 0x1e293b });
  g.circle(r + 14, 0, 3);
  g.fill({ color: 0x0f172a });

  if (isSelected) {
    g.circle(0, 0, r);
    g.stroke({ width: 2.5, color: 0x38bdf8 });
  }
}

/** Nadelbaum / Fichte: Gestufte, sternförmige Nadelkrone von oben */
function drawPineTreeTopView(g: Graphics, _w: number, _d: number, halfW: number, halfD: number, isSelected: boolean) {
  const r = Math.min(halfW, halfD);
  g.circle(2, 3, r);
  g.fill({ color: 0x000000, alpha: 0.22 });

  const branches = 10;
  const ptsOuter: number[] = [];
  for (let i = 0; i < branches * 2; i++) {
    const angle = (i * Math.PI) / branches;
    const rad = i % 2 === 0 ? r : r * 0.65;
    ptsOuter.push(rad * Math.cos(angle), rad * Math.sin(angle));
  }
  g.poly(ptsOuter);
  g.fill({ color: 0x14532d });
  g.stroke({ width: 1.0, color: 0x052e16 });

  const ptsMid: number[] = [];
  for (let i = 0; i < branches * 2; i++) {
    const angle = (i * Math.PI) / branches + 0.15;
    const rad = i % 2 === 0 ? r * 0.65 : r * 0.42;
    ptsMid.push(rad * Math.cos(angle), rad * Math.sin(angle));
  }
  g.poly(ptsMid);
  g.fill({ color: 0x15803d });

  g.circle(0, 0, r * 0.28);
  g.fill({ color: 0x22c55e });
  g.circle(0, 0, 2);
  g.fill({ color: 0x451a03 });

  if (isSelected) {
    g.circle(0, 0, r + 2);
    g.stroke({ width: 2.0, color: 0x38bdf8 });
  }
}

/** Laubbaum: Organisch überlappende Blattpolster */
function drawDeciduousTreeTopView(g: Graphics, _w: number, _d: number, halfW: number, halfD: number, isSelected: boolean) {
  const r = Math.min(halfW, halfD);
  g.circle(2, 3, r);
  g.fill({ color: 0x000000, alpha: 0.2 });

  g.circle(0, 0, r * 0.95);
  g.fill({ color: 0x15803d });

  const lobes = 6;
  for (let i = 0; i < lobes; i++) {
    const angle = (i * 2 * Math.PI) / lobes;
    const lx = Math.cos(angle) * (r * 0.55);
    const ly = Math.sin(angle) * (r * 0.55);
    g.circle(lx, ly, r * 0.48);
    g.fill({ color: i % 2 === 0 ? 0x16a34a : 0x22c55e });
  }

  g.circle(-r * 0.15, -r * 0.15, r * 0.42);
  g.fill({ color: 0x4ade80, alpha: 0.7 });

  g.circle(0, 0, r);
  g.stroke({ width: isSelected ? 2.5 : 1.2, color: isSelected ? 0x38bdf8 : 0x14532d });
}

/** Lichtsignal: Signalmastfuß mit Signalschirm und Linsenoptiken */
function drawLightSignalTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  g.rect(-halfW + 1, -halfD + 1, w, d);
  g.fill({ color: 0x000000, alpha: 0.2 });

  g.rect(-halfW, -halfD, w, d);
  g.fill({ color: 0x475569 });
  g.stroke({ width: 1.0, color: 0x1e293b });

  const shieldW = w * 0.8;
  const shieldD = d * 0.6;
  g.roundRect(-shieldW / 2, -shieldD / 2, shieldW, shieldD, 2);
  g.fill({ color: 0x09090b });
  g.stroke({ width: 0.8, color: 0x27272a });

  g.circle(-shieldW * 0.25, 0, 2);
  g.fill({ color: 0xef4444 });
  g.circle(shieldW * 0.25, 0, 2);
  g.fill({ color: 0x22c55e });

  if (isSelected) {
    g.rect(-halfW - 2, -halfD - 2, w + 4, d + 4);
    g.stroke({ width: 2.0, color: 0x38bdf8 });
  }
}

/** Prellbock: Schienenklemm-Träger & Pufferbohle mit Sh0-Warnschild */
function drawBufferStopTopView(g: Graphics, w: number, d: number, halfW: number, halfD: number, isSelected: boolean) {
  g.rect(-halfW + 1, -halfD + 1, w, d);
  g.fill({ color: 0x000000, alpha: 0.2 });

  g.rect(-halfW + 2, -halfD + 2, w - 4, d - 8);
  g.fill({ color: 0x334155 });
  g.stroke({ width: 1.0, color: 0x0f172a });

  const beamH = 6;
  g.rect(-halfW, halfD - beamH, w, beamH);
  g.fill({ color: 0x7c2d12 });
  g.stroke({ width: 1.2, color: 0x451a03 });

  g.rect(-4, halfD - beamH - 2, 8, 4);
  g.fill({ color: 0xdc2626 });
  g.stroke({ width: 0.8, color: 0xffffff });

  if (isSelected) {
    g.rect(-halfW - 2, -halfD - 2, w + 4, d + 4);
    g.stroke({ width: 2.0, color: 0x38bdf8 });
  }
}

/** Bahnsteigleuchte: Leuchtenfuß, Mastarm & Leuchtenkopf */
function drawPlatformLampTopView(g: Graphics, _w: number, _d: number, halfW: number, halfD: number, isSelected: boolean) {
  const r = Math.min(halfW, halfD);
  g.circle(0, 0, r);
  g.fill({ color: 0x475569 });
  g.stroke({ width: 1.0, color: 0x1e293b });

  g.moveTo(0, 0).lineTo(0, -halfD - 6);
  g.stroke({ width: 2.0, color: 0x0f172a });

  g.circle(0, -halfD - 6, 4.5);
  g.fill({ color: 0xfef08a });
  g.stroke({ width: 1.0, color: 0xca8a04 });
  g.circle(0, -halfD - 6, 2);
  g.fill({ color: 0xffffff });

  if (isSelected) {
    g.circle(0, 0, r + 2);
    g.stroke({ width: 2.0, color: 0x38bdf8 });
  }
}

/** Standard-Gebäude: Architektonisches Walmdach mit 4-Flächen-Schattierung & Firstlinie */
function drawGenericBuildingTopView(
  g: Graphics,
  w: number,
  d: number,
  halfW: number,
  halfD: number,
  category: string,
  isSelected: boolean,
) {
  g.roundRect(-halfW + 3, -halfD + 3, w, d, 2);
  g.fill({ color: 0x000000, alpha: 0.22 });

  let roofLight = 0xc2410c;
  let roofDark = 0x9a3412;
  let ridgeColor = 0x431407;
  let wallColor = 0x78350f;

  if (category === "railway") {
    roofLight = 0x0284c7;
    roofDark = 0x0369a1;
    ridgeColor = 0x0c4a6e;
    wallColor = 0x075985;
  } else if (category === "nature") {
    roofLight = 0x16a34a;
    roofDark = 0x15803d;
    ridgeColor = 0x052e16;
    wallColor = 0x14532d;
  } else if (category === "custom") {
    roofLight = 0x6366f1;
    roofDark = 0x4f46e5;
    ridgeColor = 0x312e81;
    wallColor = 0x3730a3;
  }

  if (w >= d) {
    const hipOffset = d * 0.45;
    g.poly([-halfW, -halfD, halfW, -halfD, halfW - hipOffset, 0, -halfW + hipOffset, 0]);
    g.fill({ color: roofLight });
    g.poly([-halfW + hipOffset, 0, halfW - hipOffset, 0, halfW, halfD, -halfW, halfD]);
    g.fill({ color: roofDark });
    g.poly([-halfW, -halfD, -halfW + hipOffset, 0, -halfW, halfD]);
    g.fill({ color: roofLight, alpha: 0.85 });
    g.poly([halfW, -halfD, halfW, halfD, halfW - hipOffset, 0]);
    g.fill({ color: roofDark, alpha: 0.9 });
    g.moveTo(-halfW + hipOffset, 0).lineTo(halfW - hipOffset, 0);
    g.stroke({ width: 1.8, color: ridgeColor });
  } else {
    const hipOffset = w * 0.45;
    g.poly([-halfW, -halfD, 0, -halfD + hipOffset, 0, halfD - hipOffset, -halfW, halfD]);
    g.fill({ color: roofLight });
    g.poly([0, -halfD + hipOffset, halfW, -halfD, halfW, halfD, 0, halfD - hipOffset]);
    g.fill({ color: roofDark });
    g.moveTo(0, -halfD + hipOffset).lineTo(0, halfD - hipOffset);
    g.stroke({ width: 1.8, color: ridgeColor });
  }

  g.rect(-halfW, -halfD, w, d);
  g.stroke({ width: isSelected ? 2.5 : 1.3, color: isSelected ? 0x38bdf8 : wallColor });
}
