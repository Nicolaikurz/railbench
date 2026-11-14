import { Application, Container, Graphics, Text, TextStyle } from "pixi.js";
import type { Layout, ColorMode, TrackPiece, BaseboardConfig } from "@/core/types";
import { catalogRegistry, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import type { SimulationSummary, ShortCircuitDetectionResult } from "@/core/simulation";
import { getTranslations } from "@/i18n";
import { createTrackGraphics } from "./track-sprite";
import { createBaseboardGraphics, getStyleColors } from "./baseboard-sprite";
import { createModel2DGraphics } from "./model-sprite";
import { getBaseboards, getBaseboardBounds, getBaseboardPolygon, getLayerPlatePolygon } from "@/core/layout";
import { computeTableTrackCutouts } from "@/core/table-cutout";
import type { BaseboardSnapGuide } from "@/core/baseboard-snapping";
import { getConnectorSnapStatus } from "@/core/snapping";
import { isFlexTrack } from "@/core/flextrack";
import {
  getBuildingDoorConnectors,
  isConnectorConnectedToBuilding,
} from "@/core/engine-shed-snap";
import { dist, deg2rad } from "@/core/geometry";
import { renderSceneOverlays } from "./renderer-overlays";

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
  pointMarkers?: Array<{ x: number; y: number; label: string; color: number; id?: string }>;
  previewElevations?: Map<string, { start: number; end: number }>;
  previewPieces?: TrackPiece[];
  hoveredOpenConnector?: { pieceId: string; connectorIndex: number } | null;
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

  getBaseboardContainer(): Container {
    return this.baseboardLayer;
  }

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
    const layerMap = new Map<string, any>();
    const layersList = layout.layers ?? [];
    for (const l of layersList) layerMap.set(l.id, l);
    const defaultLayerId = layersList[0]?.id || "layer-1";
    const activeLayerId = layout.activeLayerId || defaultLayerId;
    const hasMultipleLayers = layersList.length > 1;
    const activeLayer = layerMap.get(activeLayerId);
    const activeLayerElev = activeLayer?.heightMm ?? 0;

    // 0. Baseboard / Tischplatte & Zwischenebenen rendern (sortiert nach realer 3D-Höhe)
    this.baseboardLayer.removeChildren();
    const baseboards = getBaseboards(layout);
    const tableLayer = layout.layers?.find((l) => (l.heightMm ?? 0) === 0);
    const tableVisible = tableLayer ? tableLayer.visible !== false : true;

    interface BaseboardRenderItem {
      elev: number;
      order: number;
      sprite: Container;
    }
    const boardItems: BaseboardRenderItem[] = [];

    if (tableVisible) {
      for (const bb of baseboards) {
        if (bb.enabled) {
          const isSelected = options.activeBaseboardId === bb.id;
          const isDragging = isSelected && (options.isDraggingBaseboard ?? false);
          const matchingPlate = tableLayer?.plates?.find(
            (p) => (p.baseboardId || baseboards[0]?.id) === bb.id,
          );
          const effectiveStyle = matchingPlate?.style ?? tableLayer?.woodPlateStyle ?? bb.style ?? "wood";
          const bbToRender = bb.style === effectiveStyle ? bb : { ...bb, style: effectiveStyle };
          const tableElev = tableLayer?.heightMm ?? 0;
          const tableLayerId = tableLayer?.id || defaultLayerId;
          const isInactive = hasMultipleLayers && tableLayerId !== activeLayerId;

          const bbSprite = createBaseboardGraphics(bbToRender, {
            isSelected,
            isDragging,
            snapGuides: isDragging ? options.baseboardSnapGuides : undefined,
            layout,
            catalog,
            previewElevations: options.previewElevations,
            layerElevation: tableElev,
            layerThickness: tableLayer?.woodPlateThicknessMm ?? 35,
          });

          if (isInactive && tableElev > activeLayerElev) {
            bbSprite.alpha = 0.4;
          }

          boardItems.push({
            elev: tableElev,
            order: 0,
            sprite: bbSprite,
          });
        }
      }
    }

    // 0A. Tischplatten / Voll-Zwischenebenen (woodPlateMode === "table" oder layer.plates)
    if (layout.layers) {
      for (const layer of layout.layers) {
        if (layer.visible === false) continue;
        const layerH = layer.heightMm ?? 0;
        const thickness = layer.woodPlateThicknessMm ?? 12;
        const isInactive = hasMultipleLayers && layer.id !== activeLayerId;

        if (layer.plates && layer.plates.length > 0) {
          for (const plate of layer.plates) {
            const targetBoard = (plate.baseboardId ? baseboards.find((b) => b.id === plate.baseboardId) : undefined) || baseboards[0];
            const isGroundMatchingBoard =
              tableVisible &&
              layerH === (tableLayer?.heightMm ?? 0) &&
              targetBoard &&
              targetBoard.enabled &&
              (plate.baseboardId === targetBoard.id || !plate.baseboardId) &&
              (!plate.x || plate.x === 0) &&
              (!plate.y || plate.y === 0) &&
              plate.width === targetBoard.width &&
              plate.depth === targetBoard.depth &&
              plate.shape !== "custom" &&
              targetBoard.shape !== "custom" &&
              (!plate.shape || plate.shape === (targetBoard.shape ?? "rect"));

            // Hauptplatte auf Tisch-Ebene wurde bereits über den Tisch mit voller Grafik gerendert
            if (isGroundMatchingBoard) {
              continue;
            }

            const style = plate.style ?? targetBoard?.style ?? "wood";
            const tableBounds = targetBoard ? getBaseboardBounds(targetBoard) : { x: 0, y: 0 };
            const originX = tableBounds.x + (plate.x ?? 0);
            const originY = tableBounds.y + (plate.y ?? 0);

            const plateConfig: BaseboardConfig = {
              enabled: true,
              width: Math.max(50, plate.width),
              depth: Math.max(50, plate.depth),
              positionMode: "custom",
              customX: originX,
              customY: originY,
              shape: plate.shape ?? "rect",
              customPolygon: plate.customPolygon,
              triangleType: plate.triangleType,
              lWingWidth: plate.lWingWidth,
              lWingDepth: plate.lWingDepth,
              lSide: plate.lSide,
              uLeftWingWidth: plate.uLeftWingWidth,
              uRightWingWidth: plate.uRightWingWidth,
              uBackDepth: plate.uBackDepth,
              tBarDepth: plate.tBarDepth,
              tStemWidth: plate.tStemWidth,
              rotation: plate.rotation ?? 0,
              style,
              cornerRadius: 8,
              showDimensions: false,
            };

            const plateSprite = createBaseboardGraphics(plateConfig, {
              layout,
              catalog,
              previewElevations: options.previewElevations,
              layerElevation: layerH,
              layerThickness: thickness,
            });

            if (isInactive && layerH > activeLayerElev) {
              plateSprite.alpha = 0.4;
            }

            boardItems.push({
              elev: layerH,
              order: 1,
              sprite: plateSprite,
            });
          }
        } else if (layer.woodPlateMode === "table" && layerH !== (tableLayer?.heightMm ?? 0)) {
          for (const bb of baseboards) {
            if (!bb.enabled) continue;
            const style = layer.woodPlateStyle ?? bb.style ?? "wood";
            const deckConfig: BaseboardConfig = {
              ...bb,
              style,
              showDimensions: false,
            };
            const deckSprite = createBaseboardGraphics(deckConfig, {
              layout,
              catalog,
              previewElevations: options.previewElevations,
              layerElevation: layerH,
              layerThickness: thickness,
            });

            if (isInactive && layerH > activeLayerElev) {
              deckSprite.alpha = 0.4;
            }

            boardItems.push({
              elev: layerH,
              order: 1,
              sprite: deckSprite,
            });
          }
        }
      }
    }

    boardItems.sort((a, b) => {
      if (Math.abs(a.elev - b.elev) > 0.05) return a.elev - b.elev;
      return a.order - b.order;
    });

    for (const item of boardItems) {
      this.baseboardLayer.addChild(item.sprite);
    }

    // 0B. 3D-Modelle / Scenery im 2D-Grundriss rendern
    this.modelsLayer.removeChildren();
    const selectedIds = options.selectedIds ?? new Set<string>();

    if (layout.models && layout.models.length > 0) {
      const sortedModels = layout.models
        .map((model, index) => {
          const layer = model.layerId ? layerMap.get(model.layerId) : undefined;
          const elev = (layer?.heightMm ?? 0) + (model.elevation ?? 0);
          const mLayerId = model.layerId || defaultLayerId;
          const isInactive = hasMultipleLayers && mLayerId !== activeLayerId;
          return { model, layer, elev, isInactive, index };
        })
        .filter((item) => !item.layer || item.layer.visible !== false)
        .sort((a, b) => {
          if (Math.abs(a.elev - b.elev) > 0.05) return a.elev - b.elev;
          if (a.isInactive !== b.isInactive) return a.isInactive ? -1 : 1;
          return a.index - b.index;
        });

      for (const { model, layer, isInactive } of sortedModels) {
        const isSelected = selectedIds.has(model.id);

        // Holzfundamentplatte rendern, wenn Layer im tight-Modus
        if (layer?.woodPlateMode === "tight") {
          const baseG = new Graphics();
          const scale = model.scale ?? 1.0;
          const w = (model.width * scale) + 12;
          const d = (model.depth * scale) + 12;
          baseG.rect(-w / 2, -d / 2, w, d);
          baseG.fill({ color: 0xcbb89d, alpha: isInactive && !isSelected ? 0.18 : 0.35 });
          baseG.stroke({ width: 1.5, color: 0x8c6d48, alpha: isInactive && !isSelected ? 0.38 : 0.7 });
          baseG.position.set(model.transform.x, model.transform.y);
          baseG.rotation = (model.transform.rotation * Math.PI) / 180;
          this.modelsLayer.addChild(baseG);
        }

        const modelSprite = createModel2DGraphics(model, isSelected);
        if (isInactive && !isSelected) {
          modelSprite.alpha = 0.58;
        }
        this.modelsLayer.addChild(modelSprite);
      }
    }

    // 1. Tracks rendern (sortiert nach realer 3D-Höhe: tiefere Gleise unten, höhere Gleise darüber)
    this.tracksLayer.removeChildren();

    const entryMap = new Map<string, CatalogEntry>();
    if (catalog) {
      for (const e of catalog.entries) {
        entryMap.set(e.id, e);
      }
    }

    const showPowerDrop = options.showPowerDrop ?? false;
    const powerSim = options.powerSimulation;

    const visibleTrackItems = layout.pieces
      .map((piece, index) => {
        const layer = piece.layerId ? layerMap.get(piece.layerId) : undefined;
        const baseElev = layer?.heightMm ?? 0;
        const prevElev = options.previewElevations?.get(piece.id);
        const pStart = prevElev !== undefined ? prevElev.start : (piece.elevation?.start ?? 0);
        const pEnd = prevElev !== undefined ? prevElev.end : (piece.elevation?.end ?? (piece.elevation?.start ?? 0));
        const startElev = baseElev + pStart;
        const endElev = baseElev + pEnd;
        const minElev = Math.min(startElev, endElev);
        const maxElev = Math.max(startElev, endElev);
        const avgElev = (startElev + endElev) * 0.5;
        const pieceLayerId = piece.layerId || defaultLayerId;
        const isInactiveLayer = hasMultipleLayers && pieceLayerId !== activeLayerId;
        const isSelected = selectedIds.has(piece.id);
        return {
          piece,
          layer,
          baseElev,
          minElev,
          maxElev,
          avgElev,
          isInactiveLayer,
          isSelected,
          index,
        };
      })
      .filter((item) => !item.layer || item.layer.visible !== false);

    let minLayoutElev = 0;
    for (const item of visibleTrackItems) {
      if (item.minElev < minLayoutElev) minLayoutElev = item.minElev;
    }

    visibleTrackItems.sort((a, b) => {
      if (Math.abs(a.avgElev - b.avgElev) > 0.05) return a.avgElev - b.avgElev;
      if (Math.abs(a.maxElev - b.maxElev) > 0.05) return a.maxElev - b.maxElev;
      if (Math.abs(a.minElev - b.minElev) > 0.05) return a.minElev - b.minElev;
      if (a.isInactiveLayer !== b.isInactiveLayer) return a.isInactiveLayer ? -1 : 1;
      if (a.isSelected !== b.isSelected) return a.isSelected ? 1 : -1;
      return a.index - b.index;
    });

    for (const item of visibleTrackItems) {
      const { piece, layer, baseElev, isInactiveLayer, isSelected } = item;

      const entry: CatalogEntry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId) ?? {
        id: piece.catalogId,
        name: `Gleis ${piece.catalogId}`,
        kind: "straight",
        category: "Geraden",
        description: "",
        length: 188.3,
      };

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
        layerHeightMm: baseElev,
        isInactiveLayer,
        minLayoutElev,
        allPieces: layout.pieces,
      });

      this.tracksLayer.addChild(sprite);
    }

    // Ghost / Live-Vorschau von Rampen-Gleisen (z.B. Rampengenerator)
    if (options.previewPieces && options.previewPieces.length > 0) {
      for (const piece of options.previewPieces) {
        const entry: CatalogEntry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId) ?? {
          id: piece.catalogId,
          name: `Gleis ${piece.catalogId}`,
          kind: "straight",
          category: "Geraden",
          description: "",
          length: 188.3,
        };
        const sprite = createTrackGraphics(piece, entry, catalog, {
          isSelected: true,
          showConnectors: true,
        });
        sprite.alpha = 0.82;
        this.tracksLayer.addChild(sprite);
      }
    }

    // 2. Overlays rendern (Connectors, Snap-Ziele)
    this.overlayLayer.removeChildren();
    const ov = new Graphics();
    this.overlayLayer.addChild(ov);

    if (options.showConnectors) {
      const drawnConnected = new Set<string>();
      for (const { piece, isInactiveLayer } of visibleTrackItems) {
        const alphaMul = isInactiveLayer ? 0.5 : 1.0;
        const pieceLayerKey = piece.layerId || defaultLayerId;
        for (let cIdx = 0; cIdx < piece.connectors.length; cIdx++) {
          const conn = piece.connectors[cIdx]!;
          const isBuildingSnap = isConnectorConnectedToBuilding(conn, layout.models);
          const info = getConnectorSnapStatus(piece, cIdx, layout.pieces, layout.layers);

          if (isBuildingSnap) {
            // "change the color to blue if an building is snappped"
            const key = `${pieceLayerKey}:${Math.round(conn.x * 2) / 2},${Math.round(conn.y * 2) / 2}`;
            if (drawnConnected.has(key)) continue;
            drawnConnected.add(key);

            ov.circle(conn.x, conn.y, 4.5);
            ov.fill({ color: 0x0284c7, alpha: 0.95 * alphaMul });
            ov.circle(conn.x, conn.y, 2);
            ov.fill({ color: 0x38bdf8, alpha: alphaMul });
            ov.circle(conn.x, conn.y, 7);
            ov.stroke({ width: 1.2, color: 0x0369a1, alpha: 0.7 * alphaMul });
          } else if (info.status === "connected") {
            // "if two tracks are connected color it grey"
            const key = `${pieceLayerKey}:${Math.round(conn.x * 2) / 2},${Math.round(conn.y * 2) / 2}`;
            if (drawnConnected.has(key)) continue;
            drawnConnected.add(key);

            ov.circle(conn.x, conn.y, 4);
            ov.fill({ color: 0x64748b, alpha: 0.9 * alphaMul });
            ov.circle(conn.x, conn.y, 2);
            ov.fill({ color: 0x94a3b8, alpha: alphaMul });
            ov.circle(conn.x, conn.y, 6.5);
            ov.stroke({ width: 1, color: 0x475569, alpha: 0.6 * alphaMul });
          } else if (info.status === "above") {
            // "add yellow snap point if the track is above an snap point"
            ov.circle(conn.x, conn.y, 5);
            ov.fill({ color: 0xeab308, alpha: 0.95 * alphaMul });
            ov.circle(conn.x, conn.y, 2.2);
            ov.fill({ color: 0xfef08a, alpha: alphaMul });
            ov.circle(conn.x, conn.y, 7.5);
            ov.stroke({ width: 1.4, color: 0xfacc15, alpha: 0.8 * alphaMul });
          } else {
            // Freier Andockpunkt: leuchtend grün mit sichtbarem Plus-Symbol
            ov.circle(conn.x, conn.y, 4.5);
            ov.fill({ color: isInactiveLayer ? 0x4b7a5e : 0x16a34a, alpha: 0.9 * alphaMul });
            ov.circle(conn.x, conn.y, 7);
            ov.stroke({ width: 1.2, color: isInactiveLayer ? 0x648c74 : 0x22c55e, alpha: 0.5 * alphaMul });

            // Kleines weißes "+" Kreuz im freien Andockpunkt
            const arm = 2.2;
            ov.moveTo(conn.x - arm, conn.y).lineTo(conn.x + arm, conn.y);
            ov.moveTo(conn.x, conn.y - arm).lineTo(conn.x, conn.y + arm);
            ov.stroke({ width: 1.2, color: 0xffffff, alpha: 0.95 * alphaMul });

            // Prominenter Plus-Aktions-Button für markierte Gleise oder gehoverte Anschlüsse
            const isSelected = selectedIds.has(piece.id);
            const isHovered = Boolean(
              options.hoveredOpenConnector &&
              options.hoveredOpenConnector.pieceId === piece.id &&
              options.hoveredOpenConnector.connectorIndex === cIdx
            );

            if ((isSelected || isHovered) && !isInactiveLayer) {
              const rad = deg2rad(conn.angle);
              const plusDist = 15;
              const px = conn.x + Math.cos(rad) * plusDist;
              const py = conn.y + Math.sin(rad) * plusDist;

              // Verbindungs-Linie vom Gleisende zum Plus-Button
              ov.moveTo(conn.x + Math.cos(rad) * 6, conn.y + Math.sin(rad) * 6)
                .lineTo(px - Math.cos(rad) * 7.5, py - Math.sin(rad) * 7.5);
              ov.stroke({ width: 1.2, color: 0x38bdf8, alpha: 0.75 });

              // Äußerer Schein / Halo
              ov.circle(px, py, 10);
              ov.fill({ color: 0x0284c7, alpha: isHovered ? 0.35 : 0.2 });

              // Button-Körper
              ov.circle(px, py, 7.5);
              ov.fill({ color: isHovered ? 0x0284c7 : 0x0ea5e9, alpha: 0.95 });
              ov.stroke({ width: 1.5, color: 0xffffff, alpha: 1.0 });

              // Weißes "+" Symbol
              const bArm = 3.6;
              ov.moveTo(px - bArm, py).lineTo(px + bArm, py);
              ov.moveTo(px, py - bArm).lineTo(px, py + bArm);
              ov.stroke({ width: 1.8, color: 0xffffff, alpha: 1.0 });
            }
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

    renderSceneOverlays(this.overlayLayer, ov, options);
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
