import { Application, Container, Graphics, Sprite, Text, TexturePool } from "pixi.js";
import type { Layout, ColorMode, TrackPiece, BaseboardConfig } from "@/core/types";
import { catalogRegistry, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import type { SimulationSummary, ShortCircuitDetectionResult } from "@/core/simulation";
import { createTrackGraphics } from "./track-sprite";
import { computeLabelTextResolution } from "./track-badges";
import { CatenaryMast2DContext } from "./track-accessories-sprite";
import { createBaseboardGraphics } from "./baseboard-sprite";
import { createModel2DGraphics } from "./model-sprite";
import { collectTerrainSupportPolygons } from "./models-3d/ground-terrain";
import { getBaseboards, getBaseboardBounds } from "@/core/layout";
import type { BaseboardSnapGuide } from "@/core/baseboard-snapping";
import { getConnectorSnapStatus } from "@/core/snapping";
import { isFlexTrack, getFlexTrackPointAt } from "@/core/flextrack";
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
  private grid = new Container();
  private fineGrid = new Graphics();
  private coarseGrid = new Graphics();
  private baseboardLayer = new Container();
  private modelsLayer = new Container();
  private tracksLayer = new Container();
  private previewTracksLayer = new Container();
  private connectorsOverlay = new Graphics();
  private overlayLayer = new Container();
  private overlayGraphics = new Graphics();
  private currentTheme: "light" | "dark" = "light";

  // On-Demand Rendering (0% GPU-Last im Leerlauf statt 60-144 FPS Dauer-Loop)
  private isPaused = false;
  private renderScheduled = false;
  private rafId = 0;
  private delayedRenderTimer: ReturnType<typeof setTimeout> | null = null;
  private lowGpuMode = false;
  private lastRenderTs = 0;
  private resizeObserver: ResizeObserver | null = null;

  // Per-Track Sprite-Cache: beim Verschieben/Auswählen einzelner Gleise wird nicht die ganze Anlage neu gebaut
  private trackSpriteCache = new Map<string, { sig: string; sprite: Container }>();
  private lastConnectorsSignature = "";

  getBaseboardContainer(): Container {
    return this.baseboardLayer;
  }

  /**
   * Zerstört DisplayObjects rekursiv, ohne geteilte GPU-Ressourcen
   * (_sharedContext für GraphicsContext, _sharedTexture für Label-Texturen) zu zerstören.
   */
  private destroyDisplayObject(obj: any) {
    if (!obj || obj.destroyed) return;
    if (obj.children && obj.children.length > 0) {
      const kids = obj.children.slice();
      obj.removeChildren();
      for (const kid of kids) {
        this.destroyDisplayObject(kid);
      }
    }
    if (obj instanceof Graphics) {
      obj.destroy({ context: !(obj as any)._sharedContext });
    } else if (obj instanceof Sprite) {
      const isShared = Boolean((obj as any)._sharedTexture);
      obj.destroy({ texture: !isShared, textureSource: !isShared });
    } else {
      obj.destroy({ children: false, context: true, texture: true, textureSource: true });
    }
  }

  private clearLayer(layer: Container) {
    while (layer.children.length > 0) {
      const child = layer.children[0]!;
      layer.removeChild(child);
      if (child === this.overlayGraphics || child === this.connectorsOverlay) {
        (child as Graphics).clear();
      } else {
        this.destroyDisplayObject(child);
      }
    }
  }

  private clearTrackCache() {
    this.tracksLayer.removeChildren();
    for (const entry of this.trackSpriteCache.values()) {
      this.destroyDisplayObject(entry.sprite);
    }
    this.trackSpriteCache.clear();
  }

  /** Schaltet den Low-GPU-Modus um (30 FPS Limit + DPR 1.0 für geringere GPU-Last). */
  setLowGpuMode(enabled: boolean) {
    if (this.lowGpuMode === enabled) return;
    this.lowGpuMode = enabled;
    if (this.app?.renderer) {
      const targetDpr = enabled
        ? 1.0
        : Math.min(typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1, 1.25);
      if (this.app.renderer.resolution !== targetDpr) {
        this.app.renderer.resolution = targetDpr;
        this.app.resize();
      }
      this.requestRender();
    }
  }

  /** Plant genau einen WebGL-Frame via requestAnimationFrame (On-Demand, im Low-GPU-Modus auf 30 FPS gedrosselt). */
  requestRender() {
    if (this.isPaused || this.renderScheduled) return;
    if (!this.app?.renderer || !this.app?.stage) return;

    const scheduleFrame = () => {
      this.rafId = requestAnimationFrame(() => {
        this.renderScheduled = false;
        this.rafId = 0;
        if (this.isPaused || !this.app?.renderer || !this.app?.stage) return;
        this.lastRenderTs = performance.now();
        this.app.renderer.render(this.app.stage);
      });
    };

    this.renderScheduled = true;
    if (this.lowGpuMode) {
      const elapsed = performance.now() - this.lastRenderTs;
      const minIntervalMs = 1000 / 30;
      if (elapsed < minIntervalMs) {
        this.delayedRenderTimer = setTimeout(() => {
          this.delayedRenderTimer = null;
          if (this.isPaused) {
            this.renderScheduled = false;
            return;
          }
          scheduleFrame();
        }, Math.max(1, Math.ceil(minIntervalMs - elapsed)));
        return;
      }
    }
    scheduleFrame();
  }

  pause() {
    this.isPaused = true;
    if (this.delayedRenderTimer !== null) {
      clearTimeout(this.delayedRenderTimer);
      this.delayedRenderTimer = null;
    }
    if (this.rafId) {
      cancelAnimationFrame(this.rafId);
      this.rafId = 0;
    }
    this.renderScheduled = false;
    this.app.stop?.();
  }

  resume() {
    this.isPaused = false;
    this.updateLabelResolutions();
    this.requestRender();
  }

  destroy() {
    this.pause();
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    this.clearLayer(this.baseboardLayer);
    this.clearLayer(this.modelsLayer);
    this.clearTrackCache();
    this.clearLayer(this.previewTracksLayer);
    this.clearLayer(this.overlayLayer);
    this.connectorsOverlay.clear();
    try {
      TexturePool.clear();
      if (this.app?.renderer && (this.app.renderer as any).textureGC) {
        (this.app.renderer as any).textureGC.run();
      }
    } catch {}
    this.app.destroy(true, { children: true });
  }

  async init(host: HTMLElement, lowGpuMode = false) {
    this.lowGpuMode = lowGpuMode;
    const isDark = document.documentElement.getAttribute("data-theme") === "dark";
    this.currentTheme = isDark ? "dark" : "light";
    const dpr = lowGpuMode
      ? 1.0
      : Math.min(typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1, 1.25);
    await this.app.init({
      resizeTo: host,
      background: isDark ? "#131b2e" : "#e4dfd5",
      antialias: !lowGpuMode,
      autoDensity: true,
      resolution: dpr,
      autoStart: false,
    });
    this.app.ticker?.stop();
    host.appendChild(this.app.canvas);

    this.grid.addChild(this.fineGrid);
    this.grid.addChild(this.coarseGrid);
    this.world.addChild(this.grid);
    this.world.addChild(this.baseboardLayer);
    this.world.addChild(this.modelsLayer);
    this.world.addChild(this.tracksLayer);
    this.world.addChild(this.previewTracksLayer);
    this.world.addChild(this.connectorsOverlay);
    this.world.addChild(this.overlayLayer);
    this.app.stage.addChild(this.world);

    this.drawGrid();

    if (typeof ResizeObserver !== "undefined") {
      this.resizeObserver = new ResizeObserver(() => {
        if (!this.isPaused && this.app?.renderer) {
          this.app.resize();
          this.updateLabelResolutions();
          this.requestRender();
        }
      });
      this.resizeObserver.observe(host);
    }

    this.requestRender();
  }

  setTheme(theme: "light" | "dark") {
    this.currentTheme = theme;
    const bgColor = theme === "dark" ? 0x131b2e : 0xe4dfd5;
    this.app.renderer.background.color = bgColor;
    this.drawGrid();
    this.invalidateCaches();
    this.requestRender();
  }

  private lastBaseboardSignature = "";
  private lastModelsSignature = "";
  private lastTracksSignature = "";
  private lastVisibleTrackItems: Array<{
    piece: TrackPiece;
    layer?: any;
    baseElev: number;
    minElev: number;
    maxElev: number;
    avgElev: number;
    isInactiveLayer: boolean;
    isSelected: boolean;
    index: number;
  }> = [];

  invalidateCaches() {
    this.lastBaseboardSignature = "";
    this.lastModelsSignature = "";
    this.lastTracksSignature = "";
    this.lastConnectorsSignature = "";
    this.lastVisibleTrackItems = [];
    this.clearTrackCache();
    try {
      TexturePool.clear();
      if (this.app?.renderer && (this.app.renderer as any).textureGC) {
        (this.app.renderer as any).textureGC.run();
      }
    } catch {}
  }

  private computeBaseboardSignature(layout: Layout, options: RenderStateOptions): string {
    const bbs = layout.baseboards ?? [];
    let bbHash = bbs.length;
    for (let i = 0; i < bbs.length; i++) {
      const b = bbs[i]!;
      bbHash = (bbHash * 31 + (b.width | 0) + ((b.depth | 0) << 5) + ((b.style?.length ?? 0) << 10) + (b.enabled ? 1 : 0)) | 0;
    }
    const layers = layout.layers ?? [];
    let layerHash = layers.length;
    for (let i = 0; i < layers.length; i++) {
      const l = layers[i]!;
      const h = l.heightMm ?? 0;
      layerHash = (layerHash * 31 + (h | 0) + ((l.visible === false ? 0 : 1) << 8)) | 0;
    }
    return `${layout.activeLayerId || ""}_${bbHash}_${layerHash}_${options.activeBaseboardId || ""}_${Boolean(options.isDraggingBaseboard)}_${options.baseboardSnapGuides?.length ?? 0}_${options.previewElevations?.size ?? 0}`;
  }

  private computeModelsSignature(layout: Layout, options: RenderStateOptions): string {
    const models = layout.models;
    if (!models || models.length === 0) return "0";
    let hash = models.length;
    let selectedModels = "";
    for (let i = 0; i < models.length; i++) {
      const m = models[i]!;
      hash = (hash * 31 + (m.transform.x | 0) + ((m.transform.y | 0) << 5) + (((m.transform.rotation * 10) | 0) << 10) + (((m.elevation ?? 0) * 10) | 0)) | 0;
      if (options.selectedIds && options.selectedIds.has(m.id)) {
        selectedModels += m.id + ",";
      }
    }
    return `${hash}_${layout.activeLayerId || ""}_${selectedModels}`;
  }

  private computeTracksSignature(layout: Layout, catalog: Catalog | undefined, options: RenderStateOptions): string {
    const pieces = layout.pieces;
    let piecesHash = pieces.length;
    let selectedTracks = "";
    for (let i = 0; i < pieces.length; i++) {
      const p = pieces[i]!;
      const eStart = p.elevation?.start ?? 0;
      const eEnd = p.elevation?.end ?? eStart;
      const cLen = p.customLength ?? 0;
      const cRad = p.customRadius ?? 0;
      const cAng = p.customAngle ?? 0;
      const flags = (p.powerFeed ? 1 : 0) |
                    (p.isIsolator ? 2 : 0) |
                    (p.hasCatenary ? 4 : 0) |
                    (p.catenaryPowerFeed ? 8 : 0) |
                    (p.catenaryIsolator ? 16 : 0) |
                    (p.hasLantern ? 32 : 0) |
                    (p.hasDecoder ? 64 : 0) |
                    (p.hasSignal ? 128 : 0) |
                    (p.hasBridge ? 256 : 0) |
                    (p.hasTunnel ? 512 : 0);
      const isoIdx = (p.isolatedConnectorIndex ?? 0) + 1;
      const flexChord = p.customChordDist ? (p.customChordDist * 10) | 0 : 0;
      let flexOffHash = 0;
      if (p.customOffsets) {
        for (let oi = 0; oi < p.customOffsets.length; oi++) {
          flexOffHash = (flexOffHash * 31 + (((p.customOffsets[oi] ?? 0) * 10) | 0)) | 0;
        }
      }
      piecesHash = (piecesHash * 31 + (p.transform.x | 0) + ((p.transform.y | 0) << 5) + (((p.transform.rotation * 10) | 0) << 10) + (((eStart * 10) | 0) << 15) + (((eEnd * 10) | 0) << 20) + ((cLen * 10) | 0) + ((cRad * 10) | 0) + ((cAng * 10) | 0) + flags + (isoIdx << 10) + flexChord + flexOffHash) | 0;
      for (let ci = 0; ci < p.connectors.length; ci++) {
        const c = p.connectors[ci]!;
        piecesHash = (piecesHash * 31 + (c.x | 0) + ((c.y | 0) << 4) + (c.connectedTo ? 1 : 0)) | 0;
      }
      if (options.selectedIds && options.selectedIds.has(p.id)) {
        selectedTracks += p.id + ",";
      }
    }
    let prevPiecesHash = 0;
    if (options.previewPieces && options.previewPieces.length > 0) {
      prevPiecesHash = options.previewPieces.length;
      for (let pi = 0; pi < options.previewPieces.length; pi++) {
        const pp = options.previewPieces[pi]!;
        prevPiecesHash = (prevPiecesHash * 31 + (pp.transform.x | 0) + ((pp.transform.y | 0) << 4) + (((pp.transform.rotation * 10) | 0) << 8)) | 0;
      }
    }
    let prevElevHash = 0;
    if (options.previewElevations && options.previewElevations.size > 0) {
      for (const [id, val] of options.previewElevations) {
        prevElevHash = (prevElevHash * 31 + id.length + ((val.start | 0) << 4) + ((val.end | 0) << 8)) | 0;
      }
    }
    const layers = layout.layers ?? [];
    let layerHash = layers.length;
    for (let i = 0; i < layers.length; i++) {
      const l = layers[i]!;
      const h = l.heightMm ?? 0;
      layerHash = (layerHash * 31 + (h | 0) + ((l.visible === false ? 0 : 1) << 8)) | 0;
    }
    const catId = catalog?.id ?? "";
    let powerHash = "0";
    if (options.powerSimulation) {
      const ps = options.powerSimulation;
      powerHash = `${ps.feedVoltage}_${ps.catenaryFeedVoltage ?? ""}_${ps.locoCurrentMa}_${ps.locoCount}_${ps.totalCurrentAmps}_${ps.minVoltage}_${ps.maxDropPercent}_${ps.optimalCount}_${ps.warningCount}_${ps.criticalCount}_${ps.disconnectedCount}_${ps.catenaryMinVoltage ?? ""}_${ps.results.size}`;
    }
    let scHash = "0";
    if (options.shortCircuit) {
      scHash = `${options.shortCircuit.hasShortCircuit ? 1 : 0}_${options.shortCircuit.shortCircuitPieces.size}_${options.shortCircuit.shortCircuitPoints.length}`;
    }

    return `${piecesHash}_${selectedTracks}_${prevPiecesHash}_${prevElevHash}_${layerHash}_${layout.activeLayerId || ""}_${catId}_${layout.feedVoltage ?? ""}_${layout.catenaryFeedVoltage ?? ""}_${layout.catenaryCircuitMode || ""}_${layout.locoCurrentMa ?? ""}_${layout.locoCount ?? ""}_${layout.turnoutLanternCurrentMa ?? ""}_${layout.turnoutDecoderCurrentMa ?? ""}_${layout.trainCurrentAmps ?? ""}_${options.colorMode || ""}_${Boolean(options.showColorCode)}_${Boolean(options.showTrackLabels)}_${Boolean(options.showConnectors)}_${Boolean(options.showPowerDrop)}_${powerHash}_${scHash}`;
  }

  drawGrid(size = 50, extent = 6000) {
    this.fineGrid.clear();
    this.coarseGrid.clear();
    const isDark = this.currentTheme === "dark";

    const fineColor = isDark ? 0x334155 : 0xffffff;
    const fineAlpha = isDark ? 0.45 : 0.35;
    const coarseColor = isDark ? 0x475569 : 0x94a3b8;
    const coarseAlpha = isDark ? 0.6 : 0.3;
    const axisColor = isDark ? 0x38bdf8 : 0x64748b;

    // Feines Hilfsraster (50 mm) – wird beim Herauszoomen automatisch ausgeblendet
    for (let i = -extent; i <= extent; i += size) {
      this.fineGrid.moveTo(i, -extent).lineTo(i, extent);
      this.fineGrid.moveTo(-extent, i).lineTo(extent, i);
    }
    this.fineGrid.stroke({ width: 0.8, color: fineColor, alpha: fineAlpha });

    // Grobes Hauptraster (200 mm)
    for (let i = -extent; i <= extent; i += size * 4) {
      this.coarseGrid.moveTo(i, -extent).lineTo(i, extent);
      this.coarseGrid.moveTo(-extent, i).lineTo(extent, i);
    }
    this.coarseGrid.stroke({ width: 1.5, color: coarseColor, alpha: coarseAlpha });

    // Nullpunkt-Kreuz
    this.coarseGrid.moveTo(-30, 0).lineTo(30, 0);
    this.coarseGrid.moveTo(0, -30).lineTo(0, 30);
    this.coarseGrid.stroke({ width: 2, color: axisColor, alpha: 0.7 });
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

    const selectedIds = options.selectedIds ?? new Set<string>();
    const entryMap = new Map<string, CatalogEntry>();
    if (catalog) {
      for (const e of catalog.entries) {
        entryMap.set(e.id, e);
      }
    }

    // 0. Baseboard / Tischplatte & Zwischenebenen rendern (sortiert nach realer 3D-Höhe)
    const baseboardSig = this.computeBaseboardSignature(layout, options);
    if (baseboardSig !== this.lastBaseboardSignature) {
      this.clearLayer(this.baseboardLayer);
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
      this.lastBaseboardSignature = baseboardSig;
    }

    // 0B. 3D-Modelle / Scenery im 2D-Grundriss rendern
    const modelsSig = this.computeModelsSignature(layout, options);
    if (modelsSig !== this.lastModelsSignature) {
      this.clearLayer(this.modelsLayer);

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
          (modelSprite as any)._cullRadius =
            model.builtinId === "ground-terrain" || model.builtinId === "mountain"
              ? Infinity
              : Math.max(model.width || 200, model.depth || 200) * (model.scale ?? 1) * 0.65 + 250;
          if (isInactive && !isSelected) {
            modelSprite.alpha = 0.58;
          }
          // Gelände (Berge & Untergrund) in 2D sauber auf Tisch-/Plattenränder maskieren
          if (model.builtinId === "mountain" || model.builtinId === "ground-terrain") {
            const supportPolys = collectTerrainSupportPolygons(layout, model.layerId);
            if (supportPolys.length > 0) {
              const maskG = new Graphics();
              for (const poly of supportPolys) {
                if (poly.length < 3) continue;
                maskG.moveTo(poly[0]!.x, poly[0]!.y);
                for (let pi = 1; pi < poly.length; pi++) {
                  maskG.lineTo(poly[pi]!.x, poly[pi]!.y);
                }
                maskG.closePath();
                maskG.fill({ color: 0xffffff });
              }
              this.modelsLayer.addChild(maskG);
              modelSprite.mask = maskG;
            }
          }
          this.modelsLayer.addChild(modelSprite);
        }
      }
      this.lastModelsSignature = modelsSig;
    }

    // 1. Tracks rendern (inkrementeller Per-Track-Cache, sortiert nach realer 3D-Höhe)
    const tracksSig = this.computeTracksSignature(layout, catalog, options);
    if (tracksSig !== this.lastTracksSignature) {
      const catenaryMastCtx = new CatenaryMast2DContext();

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

      const activeIds = new Set<string>();
      const orderedSprites: Container[] = [];
      const pieceById = new Map<string, TrackPiece>();
      for (const p of layout.pieces) pieceById.set(p.id, p);

      for (const item of visibleTrackItems) {
        const { piece, layer, baseElev, isInactiveLayer, isSelected } = item;
        activeIds.add(piece.id);

        const powerResult = powerSim?.results.get(piece.id);
        const isShortCircuit = Boolean(options.shortCircuit?.shortCircuitPieces.has(piece.id));

        // Nachbar-Flags für Tunnel/Brücken-Übergänge & Oberleitungsmasten berücksichtigen
        let neighborMask = 0;
        for (let ci = 0; ci < piece.connectors.length; ci++) {
          const ct = piece.connectors[ci]?.connectedTo;
          if (ct) {
            const other = pieceById.get(ct.pieceId);
            if (other?.hasTunnel) neighborMask |= 1 << (ci * 3);
            if (other?.hasBridge) neighborMask |= 2 << (ci * 3);
            if (other?.hasCatenary) neighborMask |= 4 << (ci * 3);
          }
        }

        const prevElev = options.previewElevations?.get(piece.id);
        const eStart = prevElev !== undefined ? prevElev.start : (piece.elevation?.start ?? 0);
        const eEnd = prevElev !== undefined ? prevElev.end : (piece.elevation?.end ?? eStart);
        const flags =
          (piece.powerFeed ? 1 : 0) |
          (piece.isIsolator ? 2 : 0) |
          (piece.hasCatenary ? 4 : 0) |
          (piece.catenaryPowerFeed ? 8 : 0) |
          (piece.catenaryIsolator ? 16 : 0) |
          (piece.hasLantern ? 32 : 0) |
          (piece.hasDecoder ? 64 : 0) |
          (piece.hasSignal ? 128 : 0) |
          (piece.hasBridge ? 256 : 0) |
          (piece.hasTunnel ? 512 : 0);
        const flexSig = piece.customOffsets
          ? `${piece.customChordDist ?? ""}:${piece.customOffsets.join(",")}`
          : `${piece.customLength ?? ""}:${piece.customRadius ?? ""}:${piece.customAngle ?? ""}`;
        const pwrSig = showPowerDrop && powerResult
          ? `${powerResult.status}:${powerResult.voltage.toFixed(2)}:${powerResult.catenaryVoltage?.toFixed(2) ?? ""}`
          : "";
        // Bei Oberleitung auf Nachbargleisen ändert sich die Mast-Deduplizierung bei Verschiebung
        const catPosSig = piece.hasCatenary ? `${Math.round(piece.transform.x)}:${Math.round(piece.transform.y)}` : "";

        const pieceSig =
          `${piece.catalogId}|${Math.round(piece.transform.rotation * 100)}|${Math.round(eStart * 10)}|${Math.round(eEnd * 10)}|${baseElev}|${layer?.color ?? ""}|${flags}|${piece.isolatedConnectorIndex ?? 0}|${piece.catenaryIsolatedConnectorIndex ?? 0}|${piece.signalSide ?? ""}|${piece.signalDirection ?? ""}|${flexSig}|${isSelected ? 1 : 0}|${isInactiveLayer ? 1 : 0}|${options.colorMode ?? ""}|${options.showColorCode ? 1 : 0}|${options.showTrackLabels ? 1 : 0}|${options.showConnectors ? 1 : 0}|${showPowerDrop ? 1 : 0}|${isShortCircuit ? 1 : 0}|${pwrSig}|${neighborMask}|${catPosSig}`;

        const cached = this.trackSpriteCache.get(piece.id);
        let sprite: Container;
        if (cached && cached.sig === pieceSig) {
          sprite = cached.sprite;
          // Nur Position aktualisieren – kein GPU-Geometry-Rebuild beim Verschieben!
          sprite.position.set(piece.transform.x, piece.transform.y);
        } else {
          if (cached) {
            this.destroyDisplayObject(cached.sprite);
          }
          const entry: CatalogEntry = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId) ?? {
            id: piece.catalogId,
            name: `Gleis ${piece.catalogId}`,
            kind: "straight",
            category: "Geraden",
            description: "",
            length: 188.3,
          };

          sprite = createTrackGraphics(piece, entry, catalog, {
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
            worldScale: this.world.scale.x || 1,
            catenaryMastCtx,
          });
          this.trackSpriteCache.set(piece.id, { sig: pieceSig, sprite });
        }

        orderedSprites.push(sprite);
      }

      // Nicht mehr vorhandene oder ausgeblendete Gleise aus dem Cache entfernen
      for (const [id, entry] of this.trackSpriteCache) {
        if (!activeIds.has(id)) {
          if (entry.sprite.parent === this.tracksLayer) {
            this.tracksLayer.removeChild(entry.sprite);
          }
          this.destroyDisplayObject(entry.sprite);
          this.trackSpriteCache.delete(id);
        }
      }

      // Exakte Z-Reihenfolge sicherstellen (ohne gecachte Sprites zu zerstören)
      this.tracksLayer.removeChildren();
      for (const sp of orderedSprites) {
        this.tracksLayer.addChild(sp);
      }

      // Ghost / Live-Vorschau von Rampen-Gleisen (z.B. Rampengenerator) in separater Vorschau-Ebene
      this.clearLayer(this.previewTracksLayer);
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
          this.previewTracksLayer.addChild(sprite);
        }
      }
      this.lastVisibleTrackItems = visibleTrackItems;
      this.lastTracksSignature = tracksSig;
    }

    // 2A. Statisches Connector-Overlay cachen (verhindert O(N^2)-Neuberechnung bei jeder Mausbewegung)
    const hov = options.hoveredOpenConnector;
    const connectorsSig = `${Boolean(options.showConnectors)}_${tracksSig}_${modelsSig}_${hov ? `${hov.pieceId}:${hov.connectorIndex}` : ""}`;
    if (connectorsSig !== this.lastConnectorsSignature) {
      const cov = this.connectorsOverlay;
      cov.clear();

      if (options.showConnectors) {
        const drawnConnected = new Set<string>();
        for (const { piece, isInactiveLayer } of this.lastVisibleTrackItems) {
          const alphaMul = isInactiveLayer ? 0.5 : 1.0;
          const pieceLayerKey = piece.layerId || defaultLayerId;
          for (let cIdx = 0; cIdx < piece.connectors.length; cIdx++) {
            const conn = piece.connectors[cIdx]!;
            const isBuildingSnap = isConnectorConnectedToBuilding(conn, layout.models);
            const info = getConnectorSnapStatus(piece, cIdx, layout.pieces, layout.layers);

            if (isBuildingSnap) {
              const key = `${pieceLayerKey}:${Math.round(conn.x * 2) / 2},${Math.round(conn.y * 2) / 2}`;
              if (drawnConnected.has(key)) continue;
              drawnConnected.add(key);

              cov.circle(conn.x, conn.y, 4.5);
              cov.fill({ color: 0x0284c7, alpha: 0.95 * alphaMul });
              cov.circle(conn.x, conn.y, 2);
              cov.fill({ color: 0x38bdf8, alpha: alphaMul });
              cov.circle(conn.x, conn.y, 7);
              cov.stroke({ width: 1.2, color: 0x0369a1, alpha: 0.7 * alphaMul });
            } else if (info.status === "connected") {
              const key = `${pieceLayerKey}:${Math.round(conn.x * 2) / 2},${Math.round(conn.y * 2) / 2}`;
              if (drawnConnected.has(key)) continue;
              drawnConnected.add(key);

              cov.circle(conn.x, conn.y, 4);
              cov.fill({ color: 0x64748b, alpha: 0.9 * alphaMul });
              cov.circle(conn.x, conn.y, 2);
              cov.fill({ color: 0x94a3b8, alpha: alphaMul });
              cov.circle(conn.x, conn.y, 6.5);
              cov.stroke({ width: 1, color: 0x475569, alpha: 0.6 * alphaMul });
            } else if (info.status === "above") {
              cov.circle(conn.x, conn.y, 5);
              cov.fill({ color: 0xeab308, alpha: 0.95 * alphaMul });
              cov.circle(conn.x, conn.y, 2.2);
              cov.fill({ color: 0xfef08a, alpha: alphaMul });
              cov.circle(conn.x, conn.y, 7.5);
              cov.stroke({ width: 1.4, color: 0xfacc15, alpha: 0.8 * alphaMul });
            } else {
              cov.circle(conn.x, conn.y, 4.5);
              cov.fill({ color: isInactiveLayer ? 0x4b7a5e : 0x16a34a, alpha: 0.9 * alphaMul });
              cov.circle(conn.x, conn.y, 7);
              cov.stroke({ width: 1.2, color: isInactiveLayer ? 0x648c74 : 0x22c55e, alpha: 0.5 * alphaMul });

              const arm = 2.2;
              cov.moveTo(conn.x - arm, conn.y).lineTo(conn.x + arm, conn.y);
              cov.moveTo(conn.x, conn.y - arm).lineTo(conn.x, conn.y + arm);
              cov.stroke({ width: 1.2, color: 0xffffff, alpha: 0.95 * alphaMul });

              const isSelected = selectedIds.has(piece.id);
              const isHovered = Boolean(
                hov && hov.pieceId === piece.id && hov.connectorIndex === cIdx,
              );

              if ((isSelected || isHovered) && !isInactiveLayer) {
                const rad = deg2rad(conn.angle);
                const plusDist = 28;
                const px = conn.x + Math.cos(rad) * plusDist;
                const py = conn.y + Math.sin(rad) * plusDist;

                cov.moveTo(conn.x + Math.cos(rad) * 6, conn.y + Math.sin(rad) * 6)
                  .lineTo(px - Math.cos(rad) * 7.5, py - Math.sin(rad) * 7.5);
                cov.stroke({ width: 1.2, color: 0x38bdf8, alpha: 0.75 });

                cov.circle(px, py, 10);
                cov.fill({ color: 0x0284c7, alpha: isHovered ? 0.35 : 0.2 });

                cov.circle(px, py, 7.5);
                cov.fill({ color: isHovered ? 0x0284c7 : 0x0ea5e9, alpha: 0.95 });
                cov.stroke({ width: 1.5, color: 0xffffff, alpha: 1.0 });

                const bArm = 3.6;
                cov.moveTo(px - bArm, py).lineTo(px + bArm, py);
                cov.moveTo(px, py - bArm).lineTo(px, py + bArm);
                cov.stroke({ width: 1.8, color: 0xffffff, alpha: 1.0 });
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

                cov.circle(door.x, door.y, 4.5);
                cov.fill({ color: 0x0284c7, alpha: 0.95 });
                cov.circle(door.x, door.y, 2);
                cov.fill({ color: 0x38bdf8 });
                cov.circle(door.x, door.y, 7);
                cov.stroke({ width: 1.2, color: 0x0369a1, alpha: 0.7 });
              } else {
                cov.circle(door.x, door.y, 4.5);
                cov.fill({ color: 0x16a34a, alpha: 0.9 });
                cov.circle(door.x, door.y, 2);
                cov.fill({ color: 0xffffff });
                cov.circle(door.x, door.y, 7);
                cov.stroke({ width: 1.2, color: 0x22c55e, alpha: 0.5 });
              }
            }
          }
        }
      }
      this.lastConnectorsSignature = connectorsSig;
    }

    // 2B. Transiente Overlays rendern (Snap-Ziele, Flexgleis-Griffe, Auswahlrahmen, Messwerkzeug)
    this.clearLayer(this.overlayLayer);
    const ov = this.overlayGraphics;
    ov.clear();
    this.overlayLayer.addChild(ov);

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

          if (rawEntry && piece.connectors.length >= 2) {
            const fractions = [0.25, 0.5, 0.75];
            for (const frac of fractions) {
              const pt = getFlexTrackPointAt(piece, rawEntry, frac);
              const isCenter = frac === 0.5;
              const outerR = isCenter ? 11 : 9;
              const midR = isCenter ? 7.5 : 6;
              const innerR = isCenter ? 5 : 4;
              const dotR = isCenter ? 2 : 1.5;

              ov.circle(pt.x, pt.y, outerR);
              ov.stroke({ width: 1.5, color: 0x38bdf8, alpha: isCenter ? 0.7 : 0.45 });
              ov.circle(pt.x, pt.y, midR);
              ov.stroke({ width: 2.2, color: 0x0284c7, alpha: 0.95 });
              ov.circle(pt.x, pt.y, innerR);
              ov.fill({ color: 0x06b6d4, alpha: 0.95 });
              ov.circle(pt.x, pt.y, dotR);
              ov.fill({ color: 0xffffff });
            }
          }
        }
      }
    }

    renderSceneOverlays(this.overlayLayer, ov, options);
    this.updateLabelResolutions();
    this.requestRender();
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
    this.updateLabelResolutions();
  }

  /** Zentriert die Ansicht auf den Ursprung oder Layout-Mittelpunkt. */
  resetView() {
    this.world.scale.set(1);
    this.world.x = this.app.screen.width / 2;
    this.world.y = this.app.screen.height / 2;
    this.updateLabelResolutions();
  }

  /**
   * Aktualisiert die Rasterauflösung aller sichtbaren Gleis-Labels/Badges dynamisch passend zum Zoom.
   * Gleise außerhalb des sichtbaren Bildausschnitts bleiben bei niedriger Basis-Auflösung,
   * um Speicher- und VRAM-Verbrauch auch bei sehr großen Layouts minimal zu halten.
   */
  updateLabelResolutions() {
    const worldScale = this.world.scale.x || 1;
    const screenW = this.app?.screen?.width ?? 0;
    const screenH = this.app?.screen?.height ?? 0;
    const hasViewport = screenW > 0 && screenH > 0;
    const pad = 350;
    const viewMinX = hasViewport ? (-this.world.x) / worldScale - pad : -Infinity;
    const viewMaxX = hasViewport ? (screenW - this.world.x) / worldScale + pad : Infinity;
    const viewMinY = hasViewport ? (-this.world.y) / worldScale - pad : -Infinity;
    const viewMaxY = hasViewport ? (screenH - this.world.y) / worldScale + pad : Infinity;

    for (const trackCont of this.tracksLayer.children) {
      if (!(trackCont instanceof Container)) continue;
      const inView =
        !hasViewport ||
        (trackCont.x >= viewMinX &&
          trackCont.x <= viewMaxX &&
          trackCont.y >= viewMinY &&
          trackCont.y <= viewMaxY);
      const effectiveScale = inView ? worldScale : 1;

      for (const child of trackCont.children) {
        if (
          !(child instanceof Container) ||
          (child.label !== "track-number-label" &&
            child.label !== "track-voltage-badge" &&
            child.label !== "track-ramp-badge")
        ) {
          continue;
        }
        for (const sub of child.children) {
          if (sub instanceof Text) {
            const baseFontSize: number =
              typeof (sub as any)._baseFontSize === "number"
                ? (sub as any)._baseFontSize
                : Number((sub.style as any)?.fontSize) || 12;
            const targetRes = computeLabelTextResolution(baseFontSize, effectiveScale);
            if (sub.resolution !== targetRes) {
              sub.resolution = targetRes;
            }
          }
        }
      }
    }
  }
}
