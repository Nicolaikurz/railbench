import { Application, Container, Graphics, Text, TextStyle } from "pixi.js";
import type { Layout, ColorMode } from "@/core/types";
import { catalogRegistry, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import type { SimulationSummary, ShortCircuitDetectionResult } from "@/core/simulation";
import { getTranslations } from "@/i18n";
import { createTrackGraphics } from "./track-sprite";
import { createBaseboardGraphics } from "./baseboard-sprite";
import { createModel2DGraphics } from "./model-sprite";
import { getBaseboards, getBaseboardBounds } from "@/core/layout";
import type { BaseboardSnapGuide } from "@/core/baseboard-snapping";
import { getConnectorSnapStatus } from "@/core/snapping";
import { isFlexTrack } from "@/core/flextrack";
import {
  getBuildingDoorConnectors,
  isConnectorConnectedToBuilding,
} from "@/core/engine-shed-snap";
import { dist } from "@/core/geometry";

export interface MeasureState {
  start: { x: number; y: number } | null;
  current: { x: number; y: number } | null;
  locked?: boolean;
  snapPoint?: { x: number; y: number } | null;
  scaleRatio?: number;
  scaleName?: string;
  showDeltas?: boolean;
}

function drawDashedLine(g: Graphics, x1: number, y1: number, x2: number, y2: number, dash = 4, gap = 4) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.hypot(dx, dy);
  if (len === 0) return;
  const cos = dx / len;
  const sin = dy / len;
  let dist = 0;
  while (dist < len) {
    const curDash = Math.min(dash, len - dist);
    g.moveTo(x1 + cos * dist, y1 + sin * dist);
    g.lineTo(x1 + cos * (dist + curDash), y1 + sin * (dist + curDash));
    dist += dash + gap;
  }
}

export interface RenderStateOptions {
  selectedIds?: Set<string>;
  showConnectors?: boolean;
  powerSimulation?: SimulationSummary;
  shortCircuit?: ShortCircuitDetectionResult;
  showPowerDrop?: boolean;
  showTrackLabels?: boolean;
  showColorCode?: boolean;
  colorMode?: ColorMode;
  snapPreviewTarget?: { x: number; y: number; isAbove?: boolean } | null;
  selectionBox?: { start: { x: number; y: number }; end: { x: number; y: number } } | null;
  activeBaseboardId?: string;
  isDraggingBaseboard?: boolean;
  baseboardSnapGuides?: BaseboardSnapGuide[];
  measureTool?: MeasureState | null;
}

export class Renderer {
  app = new Application();
  world = new Container(); // wird für Pan/Zoom skaliert und verschoben
  private grid = new Graphics();
  private baseboardLayer = new Container();
  private modelsLayer = new Container();
  private tracksLayer = new Container();
  private overlayLayer = new Container();
  private currentTheme: "light" | "dark" = "light";

  async init(host: HTMLElement) {
    const isDark = document.documentElement.getAttribute("data-theme") === "dark";
    this.currentTheme = isDark ? "dark" : "light";
    await this.app.init({
      resizeTo: host,
      background: isDark ? "#131b2e" : "#e4dfd5",
      antialias: true,
      autoDensity: true,
    });
    host.appendChild(this.app.canvas);

    this.world.addChild(this.grid);
    this.world.addChild(this.baseboardLayer);
    this.world.addChild(this.modelsLayer);
    this.world.addChild(this.tracksLayer);
    this.world.addChild(this.overlayLayer);
    this.app.stage.addChild(this.world);

    this.drawGrid();
  }

  setTheme(theme: "light" | "dark") {
    this.currentTheme = theme;
    const bgColor = theme === "dark" ? 0x131b2e : 0xe4dfd5;
    this.app.renderer.background.color = bgColor;
    this.drawGrid();
  }

  drawGrid(size = 50, extent = 6000) {
    this.grid.clear();
    const isDark = this.currentTheme === "dark";

    const fineColor = isDark ? 0x334155 : 0xffffff;
    const fineAlpha = isDark ? 0.45 : 0.35;
    const coarseColor = isDark ? 0x475569 : 0x94a3b8;
    const coarseAlpha = isDark ? 0.6 : 0.3;
    const axisColor = isDark ? 0x38bdf8 : 0x64748b;

    // Feines Hilfsraster (50 mm)
    for (let i = -extent; i <= extent; i += size) {
      this.grid.moveTo(i, -extent).lineTo(i, extent);
      this.grid.moveTo(-extent, i).lineTo(extent, i);
    }
    this.grid.stroke({ width: 0.8, color: fineColor, alpha: fineAlpha });

    // Grobes Hauptraster (200 mm)
    for (let i = -extent; i <= extent; i += size * 4) {
      this.grid.moveTo(i, -extent).lineTo(i, extent);
      this.grid.moveTo(-extent, i).lineTo(extent, i);
    }
    this.grid.stroke({ width: 1.5, color: coarseColor, alpha: coarseAlpha });

    // Nullpunkt-Kreuz
    this.grid.moveTo(-30, 0).lineTo(30, 0);
    this.grid.moveTo(0, -30).lineTo(0, 30);
    this.grid.stroke({ width: 2, color: axisColor, alpha: 0.7 });
  }

  render(layout: Layout, catalog?: Catalog, options: RenderStateOptions = {}) {
    // 0. Baseboard / Tischplatte rendern
    this.baseboardLayer.removeChildren();
    const baseboards = getBaseboards(layout);
    for (const bb of baseboards) {
      if (bb.enabled) {
        const isSelected = options.activeBaseboardId === bb.id;
        const isDragging = isSelected && (options.isDraggingBaseboard ?? false);
        const bbSprite = createBaseboardGraphics(bb, {
          isSelected,
          isDragging,
          snapGuides: isDragging ? options.baseboardSnapGuides : undefined,
          layout,
          catalog,
        });
        this.baseboardLayer.addChild(bbSprite);
      }
    }

    // 0A. Tischplatten / Voll-Zwischenebenen (woodPlateMode === "table")
    if (layout.layers) {
      for (const layer of layout.layers) {
        if (!layer.visible || layer.woodPlateMode !== "table" || (layer.heightMm ?? 0) === 0) continue;
        for (const bb of baseboards) {
          if (!bb.enabled) continue;
          const { x, y, width, depth } = getBaseboardBounds(bb);
          const deckG = new Graphics();
          deckG.roundRect(x + 2, y + 2, width - 4, depth - 4, Math.max(0, (bb.cornerRadius ?? 12) - 2));
          deckG.stroke({ width: 2, color: 0xcbb89d, alpha: 0.65 });
          deckG.fill({ color: 0xcbb89d, alpha: 0.08 });
          this.baseboardLayer.addChild(deckG);
        }
      }
    }

    // 0B. 3D-Modelle / Scenery im 2D-Grundriss rendern
    this.modelsLayer.removeChildren();
    const selectedIds = options.selectedIds ?? new Set<string>();

    const layerMap = new Map<string, any>();
    if (layout.layers) {
      for (const l of layout.layers) layerMap.set(l.id, l);
    }

    if (layout.models && layout.models.length > 0) {
      for (const model of layout.models) {
        const layer = model.layerId ? layerMap.get(model.layerId) : undefined;
        if (layer && !layer.visible) continue;

        // Holzfundamentplatte rendern, wenn Layer im tight-Modus
        if (layer?.woodPlateMode === "tight") {
          const baseG = new Graphics();
          const scale = model.scale ?? 1.0;
          const w = (model.width * scale) + 12;
          const d = (model.depth * scale) + 12;
          baseG.rect(-w / 2, -d / 2, w, d);
          baseG.fill({ color: 0xcbb89d, alpha: 0.35 });
          baseG.stroke({ width: 1.5, color: 0x8c6d48, alpha: 0.7 });
          baseG.position.set(model.transform.x, model.transform.y);
          baseG.rotation = (model.transform.rotation * Math.PI) / 180;
          this.modelsLayer.addChild(baseG);
        }

        const isSelected = selectedIds.has(model.id);
        const modelSprite = createModel2DGraphics(model, isSelected);
        this.modelsLayer.addChild(modelSprite);
      }
    }

    // 1. Tracks rendern
    this.tracksLayer.removeChildren();

    const entryMap = new Map<string, CatalogEntry>();
    if (catalog) {
      for (const e of catalog.entries) {
        entryMap.set(e.id, e);
      }
    }

    const showPowerDrop = options.showPowerDrop ?? false;
    const powerSim = options.powerSimulation;

    for (const piece of layout.pieces) {
      const layer = piece.layerId ? layerMap.get(piece.layerId) : undefined;
      // Ebene ausgeblendet?
      if (layer && !layer.visible) {
        continue;
      }

      const entry: CatalogEntry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId) ?? {
        id: piece.catalogId,
        name: `Gleis ${piece.catalogId}`,
        kind: "straight",
        category: "Geraden",
        description: "",
        length: 188.3,
      };

      const isSelected = selectedIds.has(piece.id);
      const powerResult = powerSim?.results.get(piece.id);
      const isShortCircuit = Boolean(options.shortCircuit?.shortCircuitPieces.has(piece.id));

      const sprite = createTrackGraphics(piece, entry, catalog, {
        isSelected,
        showConnectors: options.showConnectors,
        showPowerDrop,
        powerResult,
        isShortCircuit,
        showTrackLabels: options.showTrackLabels,
        showColorCode: options.showColorCode,
        colorMode: options.colorMode,
        layerColor: layer?.color,
        layerHeightMm: layer?.heightMm ?? 0,
      });

      this.tracksLayer.addChild(sprite);
    }

    // 2. Overlays rendern (Connectors, Snap-Ziele)
    this.overlayLayer.removeChildren();
    const ov = new Graphics();
    this.overlayLayer.addChild(ov);

    if (options.showConnectors) {
      const drawnConnected = new Set<string>();
      for (const piece of layout.pieces) {
        for (let cIdx = 0; cIdx < piece.connectors.length; cIdx++) {
          const conn = piece.connectors[cIdx]!;
          const isBuildingSnap = isConnectorConnectedToBuilding(conn, layout.models);
          const info = getConnectorSnapStatus(piece, cIdx, layout.pieces, layout.layers);

          if (isBuildingSnap) {
            // "change the color to blue if an building is snappped"
            const key = `${Math.round(conn.x * 2) / 2},${Math.round(conn.y * 2) / 2}`;
            if (drawnConnected.has(key)) continue;
            drawnConnected.add(key);

            ov.circle(conn.x, conn.y, 4.5);
            ov.fill({ color: 0x0284c7, alpha: 0.95 });
            ov.circle(conn.x, conn.y, 2);
            ov.fill({ color: 0x38bdf8 });
            ov.circle(conn.x, conn.y, 7);
            ov.stroke({ width: 1.2, color: 0x0369a1, alpha: 0.7 });
          } else if (info.status === "connected") {
            // "if two tracks are connected color it grey"
            const key = `${Math.round(conn.x * 2) / 2},${Math.round(conn.y * 2) / 2}`;
            if (drawnConnected.has(key)) continue;
            drawnConnected.add(key);

            ov.circle(conn.x, conn.y, 4);
            ov.fill({ color: 0x64748b, alpha: 0.9 });
            ov.circle(conn.x, conn.y, 2);
            ov.fill({ color: 0x94a3b8 });
            ov.circle(conn.x, conn.y, 6.5);
            ov.stroke({ width: 1, color: 0x475569, alpha: 0.6 });
          } else if (info.status === "above") {
            // "add yellow snap point if the track is above an snap point"
            ov.circle(conn.x, conn.y, 5);
            ov.fill({ color: 0xeab308, alpha: 0.95 });
            ov.circle(conn.x, conn.y, 2.2);
            ov.fill({ color: 0xfef08a });
            ov.circle(conn.x, conn.y, 7.5);
            ov.stroke({ width: 1.4, color: 0xfacc15, alpha: 0.8 });
          } else {
            // Freier Andockpunkt: leuchtend grün
            ov.circle(conn.x, conn.y, 4.5);
            ov.fill({ color: 0x16a34a, alpha: 0.9 });
            ov.circle(conn.x, conn.y, 2);
            ov.fill({ color: 0xffffff });
            ov.circle(conn.x, conn.y, 7);
            ov.stroke({ width: 1.2, color: 0x22c55e, alpha: 0.5 });
          }
        }
      }

      // Snappoints für Gebäude (Lokschuppen, Ringlokschuppen etc.)
      if (layout.models) {
        for (const m of layout.models) {
          const doors = getBuildingDoorConnectors(m);
          for (const door of doors) {
            const isConnected = layout.pieces.some((p) =>
              p.connectors.some((c) => dist(c, door) <= 5.0),
            );
            const key = `${Math.round(door.x * 2) / 2},${Math.round(door.y * 2) / 2}`;
            if (isConnected) {
              if (drawnConnected.has(key)) continue;
              drawnConnected.add(key);

              // "change the color to blue if an building is snappped"
              ov.circle(door.x, door.y, 4.5);
              ov.fill({ color: 0x0284c7, alpha: 0.95 });
              ov.circle(door.x, door.y, 2);
              ov.fill({ color: 0x38bdf8 });
              ov.circle(door.x, door.y, 7);
              ov.stroke({ width: 1.2, color: 0x0369a1, alpha: 0.7 });
            } else {
              ov.circle(door.x, door.y, 4.5);
              ov.fill({ color: 0x16a34a, alpha: 0.9 });
              ov.circle(door.x, door.y, 2);
              ov.fill({ color: 0xffffff });
              ov.circle(door.x, door.y, 7);
              ov.stroke({ width: 1.2, color: 0x22c55e, alpha: 0.5 });
            }
          }
        }
      }
    }

    // Falls gerade ein Snap-Ziel aktiv ist:
    if (options.snapPreviewTarget) {
      const pt = options.snapPreviewTarget;
      const color = pt.isAbove ? 0xeab308 : 0xf59e0b;
      ov.circle(pt.x, pt.y, 10);
      ov.stroke({ width: 2.5, color, alpha: 0.9 });
      ov.circle(pt.x, pt.y, 4);
      ov.fill({ color });
    }

    // Interaktiver Flexgleis-Ziehgriff für markierte Flexgleise ("tack the open snappoint")
    for (const piece of layout.pieces) {
      if (selectedIds.has(piece.id)) {
        const rawEntry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId);
        if (isFlexTrack(rawEntry, piece)) {
          for (let ci = 0; ci < piece.connectors.length; ci++) {
            const flexConn = piece.connectors[ci];
            if (!flexConn || flexConn.connectedTo) continue;
            ov.circle(flexConn.x, flexConn.y, 9);
            ov.stroke({ width: 2.5, color: 0x06b6d4, alpha: 0.95 });
            ov.circle(flexConn.x, flexConn.y, 6.5);
            ov.fill({ color: 0x0891b2, alpha: 0.9 });
            ov.circle(flexConn.x, flexConn.y, 2.8);
            ov.fill({ color: 0xffffff });
            ov.circle(flexConn.x, flexConn.y, 12);
            ov.stroke({ width: 1, color: 0x38bdf8, alpha: 0.5 });
          }
        }
      }
    }

    // Auswahlrahmen (Marquee Box bei Rechtsklick-Drag)
    if (options.selectionBox) {
      const box = options.selectionBox;
      const minX = Math.min(box.start.x, box.end.x);
      const maxX = Math.max(box.start.x, box.end.x);
      const minY = Math.min(box.start.y, box.end.y);
      const maxY = Math.max(box.start.y, box.end.y);
      const w = maxX - minX;
      const h = maxY - minY;

      if (w > 2 || h > 2) {
        ov.rect(minX, minY, w, h);
        ov.fill({ color: 0x0284c7, alpha: 0.15 });
        ov.stroke({ width: 1.5, color: 0x38bdf8, alpha: 0.95 });

        const cornerSize = Math.min(8, w / 4, h / 4);
        if (cornerSize > 1) {
          ov.moveTo(minX, minY + cornerSize).lineTo(minX, minY).lineTo(minX + cornerSize, minY);
          ov.moveTo(maxX - cornerSize, minY).lineTo(maxX, minY).lineTo(maxX, minY + cornerSize);
          ov.moveTo(minX, maxY - cornerSize).lineTo(minX, maxY).lineTo(minX + cornerSize, maxY);
          ov.moveTo(maxX - cornerSize, maxY).lineTo(maxX, maxY).lineTo(maxX, maxY - cornerSize);
          ov.stroke({ width: 2.5, color: 0x0284c7, alpha: 1 });
        }
      }
    }

    // 3. Messwerkzeug (Measure Tool: Manuelle Distanzmessung zwischen zwei Punkten)
    if (options.measureTool) {
      const { start: p1, current: p2, locked, scaleRatio } = options.measureTool;

      // Punkt A Badge zeichnen (falls gesetzt)
      if (p1) {
        ov.circle(p1.x, p1.y, 8).fill({ color: 0x0284c7, alpha: 0.25 });
        ov.circle(p1.x, p1.y, 5).fill({ color: 0x0284c7 }).stroke({ width: 1.5, color: 0xffffff });

        const txtA = new Text({
          text: "A",
          style: new TextStyle({
            fontSize: 9,
            fill: 0xffffff,
            fontWeight: "bold",
            stroke: { color: 0x0f172a, width: 2 },
          }),
        });
        txtA.anchor.set(0.5, 1);
        txtA.position.set(p1.x, p1.y - 7);
        this.overlayLayer.addChild(txtA);
      }

      // Falls Punkt A und Punkt B vorhanden sind
      if (p1 && p2) {
        const dx = p2.x - p1.x;
        const dy = p2.y - p1.y;
        const dist = Math.hypot(dx, dy);

        // Orthogonale Hilfsdreiecke / Delta-Linien (ΔX und ΔY)
        if (dist > 15 && (Math.abs(dx) > 10 || Math.abs(dy) > 10)) {
          // ΔX: (p1.x, p1.y) -> (p2.x, p1.y)
          drawDashedLine(ov, p1.x, p1.y, p2.x, p1.y, 4, 3);
          ov.stroke({ width: 1, color: 0x94a3b8, alpha: 0.5 });

          // ΔY: (p2.x, p1.y) -> (p2.x, p2.y)
          drawDashedLine(ov, p2.x, p1.y, p2.x, p2.y, 4, 3);
          ov.stroke({ width: 1, color: 0x94a3b8, alpha: 0.5 });
        }

        if (dist > 0.5) {
          const nx = -dy / dist;
          const ny = dx / dist;
          const tickLen = 7;

          // Kontrast-Schatten
          ov.moveTo(p1.x, p1.y).lineTo(p2.x, p2.y);
          ov.stroke({ width: 3.5, color: 0x0f172a, alpha: 0.7 });

          // Haupt-Messlinie
          const lineColor = locked ? 0x10b981 : 0x0284c7;
          ov.moveTo(p1.x, p1.y).lineTo(p2.x, p2.y);
          ov.stroke({ width: 2, color: lineColor, alpha: 0.95 });

          // End-Ticks (Begrenzungslinien an A und B)
          ov.moveTo(p1.x + nx * tickLen, p1.y + ny * tickLen).lineTo(p1.x - nx * tickLen, p1.y - ny * tickLen);
          ov.moveTo(p2.x + nx * tickLen, p2.y + ny * tickLen).lineTo(p2.x - nx * tickLen, p2.y - ny * tickLen);
          ov.stroke({ width: 2, color: lineColor, alpha: 0.95 });

          // Punkt B Badge
          ov.circle(p2.x, p2.y, 8).fill({ color: lineColor, alpha: 0.25 });
          ov.circle(p2.x, p2.y, 5).fill({ color: lineColor }).stroke({ width: 1.5, color: 0xffffff });

          const txtB = new Text({
            text: "B",
            style: new TextStyle({
              fontSize: 9,
              fill: 0xffffff,
              fontWeight: "bold",
              stroke: { color: 0x0f172a, width: 2 },
            }),
          });
          txtB.anchor.set(0.5, 1);
          txtB.position.set(p2.x, p2.y - 7);
          this.overlayLayer.addChild(txtB);

          // Maßbeschriftung in der Mitte
          const midX = (p1.x + p2.x) / 2;
          const midY = (p1.y + p2.y) / 2;

          let distStr = dist >= 1000
            ? `${dist.toFixed(1)} mm (${(dist / 1000).toFixed(2)} m)`
            : `${dist.toFixed(1)} mm`;

          if (scaleRatio && scaleRatio > 1) {
            const protoM = (dist * scaleRatio) / 1000;
            distStr += `\nVorbild: ${protoM.toFixed(2)} m`;
          }

          const txt = new Text({
            text: distStr,
            style: new TextStyle({
              fontSize: 11,
              fill: 0xffffff,
              fontWeight: "bold",
              align: "center",
              stroke: { color: 0x0f172a, width: 3 },
            }),
          });
          txt.anchor.set(0.5, 0.5);
          txt.position.set(midX + nx * 16, midY + ny * 16);
          this.overlayLayer.addChild(txt);
        }
      }

      if (options.measureTool.snapPoint) {
        const sp = options.measureTool.snapPoint;
        ov.circle(sp.x, sp.y, 7).stroke({ width: 2, color: 0x10b981, alpha: 0.9 });
        ov.circle(sp.x, sp.y, 3).fill({ color: 0x10b981 });
      }
    }

    // 4. Kurzschluss-Indikator (2-Leiter-System)
    if (options.shortCircuit?.hasShortCircuit) {
      const t = getTranslations().toolbar.status;
      const overlayText = t.shortCircuitOverlayLabel || "KURZSCHLUSS";

      for (const pt of options.shortCircuit.shortCircuitPoints) {
        // Äußeres pulsierendes Glühen
        ov.circle(pt.x, pt.y, 18);
        ov.fill({ color: 0xef4444, alpha: 0.28 });
        ov.circle(pt.x, pt.y, 12);
        ov.stroke({ width: 2.5, color: 0xdc2626, alpha: 0.95 });

        // Roter Blitz-Zentrumspunkt
        ov.circle(pt.x, pt.y, 4.5);
        ov.fill({ color: 0xffffff });
        ov.stroke({ width: 2, color: 0xb91c1c });

        // Warn-Plakette
        const badgeCont = new Container();
        badgeCont.position.set(pt.x, pt.y - 20);

        const pillBg = new Graphics();
        pillBg.roundRect(-48, -10, 96, 20, 5);
        pillBg.fill({ color: 0x7f1d1d, alpha: 0.95 });
        pillBg.stroke({ width: 1.5, color: 0xef4444 });
        badgeCont.addChild(pillBg);

        const lbl = new Text({
          text: `⚡ ${overlayText}`,
          style: new TextStyle({
            fontSize: 9,
            fill: 0xffffff,
            fontWeight: "bold",
          }),
        });
        lbl.anchor.set(0.5, 0.5);
        badgeCont.addChild(lbl);

        this.overlayLayer.addChild(badgeCont);
      }
    }
  }

  /** Wandelt Bildschirmkoordinaten (Canvas) in Weltkoordinaten um. */
  screenToWorld(clientX: number, clientY: number): { x: number; y: number } {
    const rect = this.app.canvas.getBoundingClientRect();
    const mx = clientX - rect.left;
    const my = clientY - rect.top;
    const wx = (mx - this.world.x) / this.world.scale.x;
    const wy = (my - this.world.y) / this.world.scale.y;
    return { x: wx, y: wy };
  }

  /** Wandelt Weltkoordinaten (mm) in Bildschirmkoordinaten (Pixel relativ zum Canvas-Host) um. */
  worldToScreen(worldX: number, worldY: number): { x: number; y: number } {
    const sx = worldX * this.world.scale.x + this.world.x;
    const sy = worldY * this.world.scale.y + this.world.y;
    return { x: sx, y: sy };
  }

  /** Zentriert die Ansicht auf bestimmte Weltkoordinaten. */
  centerOn(worldX: number, worldY: number) {
    this.world.x = this.app.screen.width / 2 - worldX * this.world.scale.x;
    this.world.y = this.app.screen.height / 2 - worldY * this.world.scale.y;
  }

  /** Zentriert die Ansicht auf den Ursprung oder Layout-Mittelpunkt. */
  resetView() {
    this.world.scale.set(1);
    this.world.x = this.app.screen.width / 2;
    this.world.y = this.app.screen.height / 2;
  }
}
