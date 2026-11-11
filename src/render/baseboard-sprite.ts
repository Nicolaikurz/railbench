import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { BaseboardConfig, BaseboardStyle, Layout } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import { getBaseboardBounds, getBaseboardPolygon } from "@/core/layout";
import type { BaseboardSnapGuide } from "@/core/baseboard-snapping";
import { computeTableTrackCutouts } from "@/core/table-cutout";

export interface BaseboardRenderOptions {
  isSelected?: boolean;
  isDragging?: boolean;
  snapGuides?: BaseboardSnapGuide[];
  layout?: Layout;
  catalog?: Catalog;
}

interface StyleColorConfig {
  surfaceColor: number;
  edgeColor: number;
  highlightColor: number;
  grainColor: number;
  hasGrain: boolean;
  isGrass?: boolean;
}

function getStyleColors(style: BaseboardStyle): StyleColorConfig {
  switch (style) {
    case "grass":
    case "gras" as any:
      return {
        surfaceColor: 0x4d7c0f,
        edgeColor: 0x365314,
        highlightColor: 0xa3e635,
        grainColor: 0x3f6212,
        hasGrain: false,
        isGrass: true,
      };
    case "dark-wood":
      return {
        surfaceColor: 0x4a2e1b,
        edgeColor: 0x2b170a,
        highlightColor: 0x73482a,
        grainColor: 0x331e11,
        hasGrain: true,
      };
    case "plywood":
      return {
        surfaceColor: 0xecdcb9,
        edgeColor: 0xb59e75,
        highlightColor: 0xffffff,
        grainColor: 0xcbb58c,
        hasGrain: true,
      };
    case "slate":
      return {
        surfaceColor: 0x27272a,
        edgeColor: 0x3f3f46,
        highlightColor: 0x52525b,
        grainColor: 0x18181b,
        hasGrain: false,
      };
    case "white":
      return {
        surfaceColor: 0xf8fafc,
        edgeColor: 0xcbd5e1,
        highlightColor: 0xffffff,
        grainColor: 0xe2e8f0,
        hasGrain: false,
      };
    case "wood":
    default:
      return {
        surfaceColor: 0xd4a373, // Warmer Kiefer/Birke Holzton
        edgeColor: 0x8c5828,    // Solider Umleimer
        highlightColor: 0xffe6c2,// Glanzkante oben
        grainColor: 0xa8713d,   // Holzmaserung
        hasGrain: true,
      };
  }
}

/** Erzeugt das Pixi-Container-Objekt für die Holzplatte / den Anlagentisch. */
export function createBaseboardGraphics(
  config: BaseboardConfig,
  options: BaseboardRenderOptions = {},
): Container {
  const container = new Container();
  if (!config.enabled) return container;

  const { x, y, width, depth } = getBaseboardBounds(config);
  const radius = Math.max(0, Math.min(config.cornerRadius ?? 12, width / 2, depth / 2));
  const styleConfig = getStyleColors(config.style);
  const polygon = getBaseboardPolygon(config);
  const isRect = !config.shape || config.shape === "rect";
  const isSelected = options.isSelected ?? false;
  const isDragging = options.isDragging ?? false;

  // 1. Weicher Raum-Schatten
  const shadow = new Graphics();
  if (isRect) {
    shadow.roundRect(x + 6, y + 10, width, depth, radius + 2);
  } else {
    const shadowPoly = polygon.map((pt) => ({ x: pt.x + 6, y: pt.y + 10 }));
    shadow.poly(shadowPoly);
  }
  shadow.fill({ color: 0x000000, alpha: 0.22 });
  container.addChild(shadow);

  // 1B. Selektions- / Drag-Glühen um den gesamten Tisch
  if (isSelected || isDragging) {
    const glow = new Graphics();
    const glowColor = isDragging ? 0x06b6d4 : 0x0284c7;
    if (isRect) {
      glow.roundRect(x - 5, y - 5, width + 10, depth + 10, radius + 4);
    } else {
      glow.poly(polygon);
    }
    glow.stroke({ width: 4.0, color: glowColor, alpha: 0.85 });
    glow.fill({ color: glowColor, alpha: 0.08 });
    container.addChild(glow);
  }

  // 2. Tischplatte Oberfläche
  const g = new Graphics();
  if (isRect) {
    g.roundRect(x, y, width, depth, radius);
  } else {
    g.poly(polygon);
  }
  g.fill({ color: styleConfig.surfaceColor });

  // Gras- / Wiesenstruktur oder Holzmaserung
  if (styleConfig.isGrass) {
    const step = 70;
    for (let gy = y + 25; gy < y + depth - 20; gy += step) {
      const offsetX = ((gy - y) % (step * 2) === 0) ? 0 : step / 2;
      for (let gx = x + 25 + offsetX; gx < x + width - 20; gx += step) {
        g.moveTo(gx - 3, gy + 3).lineTo(gx - 4, gy - 4);
        g.moveTo(gx, gy + 3).lineTo(gx, gy - 6);
        g.moveTo(gx + 3, gy + 3).lineTo(gx + 4, gy - 4);
      }
    }
    g.stroke({ width: 0.9, color: styleConfig.highlightColor, alpha: 0.35 });

    for (let gy = y + 55; gy < y + depth - 30; gy += step) {
      const offsetX = ((gy - y) % (step * 2) === 0) ? step / 2 : 0;
      for (let gx = x + 40 + offsetX; gx < x + width - 30; gx += step) {
        g.circle(gx, gy, 1.8);
      }
    }
    g.fill({ color: styleConfig.grainColor, alpha: 0.25 });
  } else if (styleConfig.hasGrain) {
    const plankHeight = 80; // mm
    const startY = y;
    const endY = y + depth;

    // Dielenstöße / Fugen
    for (let py = startY + plankHeight; py < endY; py += plankHeight) {
      g.moveTo(x + 2, py).lineTo(x + width - 2, py);
    }
    g.stroke({ width: 0.9, color: styleConfig.grainColor, alpha: 0.28 });

    // Feine Holzmaserungs-Wellen
    for (let py = startY + 20; py < endY; py += 38) {
      g.moveTo(x + 8, py).lineTo(x + width - 8, py);
    }
    g.stroke({ width: 0.5, color: styleConfig.grainColor, alpha: 0.14 });
  }

  // 3. Tischkante / Multiplex-Umleimer (Border & Fase)
  if (isRect) {
    g.roundRect(x, y, width, depth, radius);
    g.stroke({ width: 4.0, color: styleConfig.edgeColor, alpha: 0.95 });

    // 3D-Lichtkante oben/innen
    const inset = 2;
    if (width > inset * 2 && depth > inset * 2) {
      g.roundRect(x + inset, y + inset, width - inset * 2, depth - inset * 2, Math.max(0, radius - inset));
      g.stroke({ width: 1.2, color: styleConfig.highlightColor, alpha: 0.45 });
    }
  } else {
    g.poly(polygon);
    g.stroke({ width: 4.0, color: styleConfig.edgeColor, alpha: 0.95 });
  }

  container.addChild(g);

  // 3B. Tischdurchbrüche / Ausschnitte für Rampen und Strecken unter der Platte
  if (options.layout && options.catalog) {
    const cutouts = computeTableTrackCutouts(options.layout, options.catalog, config);
    if (cutouts.length > 0) {
      const cutoutGraphics = new Graphics();
      for (const cutout of cutouts) {
        if (cutout.points.length < 3) continue;

        // Dunkle Aussparung / Durchbruch nach unten
        cutoutGraphics.poly(cutout.points);
        cutoutGraphics.fill({ color: 0x18181b, alpha: 0.72 });

        // Umleimer- / Schnittkante des Holzausschnitts
        cutoutGraphics.poly(cutout.points);
        cutoutGraphics.stroke({ width: 2.2, color: styleConfig.edgeColor, alpha: 0.95 });

        // Schattierte Innenkante
        cutoutGraphics.poly(cutout.points);
        cutoutGraphics.stroke({ width: 1.0, color: 0x09090b, alpha: 0.5 });
      }
      container.addChild(cutoutGraphics);
    }
  }

  // 4. Bemaßungslinien und -beschriftungen
  if (config.showDimensions !== false) {
    drawDimensionLines(container, x, y, width, depth);
  }

  // 5. Magnetische Snap-Führungslinien (Cyan-Laser-Guides)
  if (options.snapGuides && options.snapGuides.length > 0) {
    drawSnapGuides(container, options.snapGuides);
  }

  return container;
}

function drawDimensionLines(parent: Container, x: number, y: number, w: number, h: number) {
  const dim = new Graphics();
  const lineDist = 32; // mm Abstand zur Platte
  const extOver = 8;
  const tickLen = 6;
  const dimColor = 0x0284c7;

  // --- HORIZONTALE BEMASSUNG (OBEN) ---
  const lineY = y - lineDist;
  // Maßhilfslinien links und rechts
  dim.moveTo(x, y - 4).lineTo(x, lineY - extOver);
  dim.moveTo(x + w, y - 4).lineTo(x + w, lineY - extOver);
  // Maßlinie
  dim.moveTo(x, lineY).lineTo(x + w, lineY);
  // Architekten-Schrägstriche (45°)
  dim.moveTo(x - tickLen, lineY + tickLen).lineTo(x + tickLen, lineY - tickLen);
  dim.moveTo(x + w - tickLen, lineY + tickLen).lineTo(x + w + tickLen, lineY - tickLen);

  // --- VERTIKALE BEMASSUNG (LINKS) ---
  const lineX = x - lineDist;
  // Maßhilfslinien oben und unten
  dim.moveTo(x - 4, y).lineTo(lineX - extOver, y);
  dim.moveTo(x - 4, y + h).lineTo(lineX - extOver, y + h);
  // Maßlinie
  dim.moveTo(lineX, y).lineTo(lineX, y + h);
  // Architekten-Schrägstriche
  dim.moveTo(lineX - tickLen, y + tickLen).lineTo(lineX + tickLen, y - tickLen);
  dim.moveTo(lineX - tickLen, y + h + tickLen).lineTo(lineX + tickLen, y + h - tickLen);

  dim.stroke({ width: 1.5, color: dimColor, alpha: 0.85 });
  parent.addChild(dim);

  // Text-Labels für Maße
  const wLabelText = `${Math.round(w)} mm (${(w / 1000).toFixed(2)} m)`;
  const hLabelText = `${Math.round(h)} mm (${(h / 1000).toFixed(2)} m)`;

  const topBadge = createDimBadge(wLabelText);
  topBadge.position.set(x + w / 2, lineY);
  parent.addChild(topBadge);

  const leftBadge = createDimBadge(hLabelText);
  leftBadge.position.set(lineX, y + h / 2);
  leftBadge.rotation = -Math.PI / 2;
  parent.addChild(leftBadge);
}

function createDimBadge(textString: string): Container {
  const container = new Container();

  const text = new Text({
    text: textString,
    style: new TextStyle({
      fontFamily: "Inter, system-ui, -apple-system, sans-serif",
      fontSize: 11,
      fontWeight: "bold",
      fill: 0xffffff,
      align: "center",
      letterSpacing: 0.5,
    }),
  });
  text.anchor.set(0.5);

  const padX = 7;
  const padY = 3;
  const bgW = text.width + padX * 2;
  const bgH = text.height + padY * 2;

  const bg = new Graphics();
  // Schatten
  bg.roundRect(-bgW / 2 + 1, -bgH / 2 + 1, bgW, bgH, 4);
  bg.fill({ color: 0x000000, alpha: 0.4 });
  // Badge
  bg.roundRect(-bgW / 2, -bgH / 2, bgW, bgH, 4);
  bg.fill({ color: 0x0f172a, alpha: 0.92 });
  bg.stroke({ width: 1.2, color: 0x38bdf8, alpha: 0.95 });

  container.addChild(bg);
  container.addChild(text);
  return container;
}

function drawSnapGuides(parent: Container, guides: BaseboardSnapGuide[]) {
  if (guides.length === 0) return;
  const g = new Graphics();

  for (const guide of guides) {
    const col = guide.color ?? 0x06b6d4;
    if (guide.type === "x") {
      // Glüh-Linie dahinter
      g.moveTo(guide.pos, guide.from).lineTo(guide.pos, guide.to);
      g.stroke({ width: 4.5, color: col, alpha: 0.35 });
      // Laser-Linie im Zentrum
      g.moveTo(guide.pos, guide.from).lineTo(guide.pos, guide.to);
      g.stroke({ width: 1.8, color: 0xffffff, alpha: 0.95 });

      // Endpunkt-Marker
      g.circle(guide.pos, guide.from, 4).fill({ color: col });
      g.circle(guide.pos, guide.to, 4).fill({ color: col });
    } else {
      // Glüh-Linie dahinter
      g.moveTo(guide.from, guide.pos).lineTo(guide.to, guide.pos);
      g.stroke({ width: 4.5, color: col, alpha: 0.35 });
      // Laser-Linie im Zentrum
      g.moveTo(guide.from, guide.pos).lineTo(guide.to, guide.pos);
      g.stroke({ width: 1.8, color: 0xffffff, alpha: 0.95 });

      // Endpunkt-Marker
      g.circle(guide.from, guide.pos, 4).fill({ color: col });
      g.circle(guide.to, guide.pos, 4).fill({ color: col });
    }
  }

  parent.addChild(g);
}

