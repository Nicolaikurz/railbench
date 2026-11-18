import type { Layout, TrackPiece, Model3D, Transform, SignalSide, SignalDirection } from "@/core/types";
import { createLayout } from "@/core/layout";
import {
  computeWorldConnectors,
  getLocalConnectors,
  isFeederEntry,
  isIsolatingEntry,
  catalogRegistry,
  type Catalog,
  type CatalogEntry,
} from "@/catalog/catalog";
import { findBestSnapForPiece, updatePieceConnections } from "@/core/snapping";
import { snapModelToTurntable } from "@/core/turntable-snap";
import { snapEngineShedToTrack, snapTrainhallToTrack } from "@/core/engine-shed-snap";
import { cutFlexTrack, bendFlexTrack, flipFlexTrackPiece, isFlexTrack, resetFlexTrackToOriginal } from "@/core/flextrack";
import {
  AddPieceCommand,
  RemovePieceCommand,
  MovePieceCommand,
  AddModelCommand,
  RemoveModelCommand,
  MoveModelCommand,
  History,
} from "@/editor/commands";
import type { Selection } from "@/editor/selection";
import {
  normAngle,
  rotate,
  rotateGroupAroundCentroid,
  type RotatableItem,
} from "@/core/geometry";
import { getLanguage, getTranslations } from "@/i18n";
import { exportPieceListCSV, downloadFile } from "@/io/export";
import type { PlanManager, PlanFormat } from "@/io/plan-manager";
import type { Renderer } from "@/render/renderer";

import {
  copySelected,
  pasteClipboard,
  duplicateSelected,
  selectConnectedTracks,
  selectLayerTracks,
  toggleGroupSelected,
  splitTrack,
  replaceTrack,
} from "./track-clipboard-ops";
import {
  toggleSelectedFeeder,
  toggleSelectedIsolator,
  toggleSelectedCatenary,
  toggleSelectedCatenaryFeeder,
  toggleSelectedCatenaryIsolator,
  toggleSelectedTurnoutLantern,
  toggleSelectedTurnoutDecoder,
  toggleSelectedSignal,
  cycleSelectedSignalSide,
  cycleSelectedSignalDirection,
  setSignalConfig,
  toggleSelectedBridge,
  toggleSelectedTunnel,
  moveSelectedToLayer,
  applyHeightInput,
  applyAutoRamp,
  applyAngleInput,
} from "./track-attribute-ops";

export interface EditorContext {
  getLayout: () => Layout;
  setLayout: (l: Layout) => void;
  getCatalog: () => Catalog;
  getSelection: () => Selection;
  getHistory: () => History;
  setHistory: (h: History) => void;
  getRenderer: () => Renderer;
  getPlanManager: () => PlanManager;
  isSnappingEnabled: () => boolean;
  isPieceInteractive: (piece: TrackPiece) => boolean;
  isModelInteractive: (model: Model3D) => boolean;
  getCatalogEntry: (piece: TrackPiece) => CatalogEntry | undefined;
  sync: (dirty?: boolean) => void;
  showToast: (msg: string, icon?: string) => void;
}

/**
 * Ermittelt, ob der Gleisverlauf an einem bestimmten Anschluss nach links oder rechts biegt.
 */
function detectTrackTurn(
  anchorPiece: TrackPiece,
  chosenIndex: number,
  layoutPieces: TrackPiece[],
  getCatalogEntry: (piece: TrackPiece) => CatalogEntry | undefined,
  visited = new Set<string>()
): "left" | "right" | "straight" {
  if (visited.has(anchorPiece.id)) return "straight";
  visited.add(anchorPiece.id);

  const chosenConn = anchorPiece.connectors[chosenIndex];
  if (!chosenConn) return "straight";

  const incoming = anchorPiece.connectors
    .map((c, index) => ({ c, index }))
    .find((item) => item.index !== chosenIndex && Boolean(item.c.connectedTo));

  if (incoming) {
    const inDir = normAngle(incoming.c.angle + 180);
    const outDir = chosenConn.angle;
    let turnDelta = normAngle(outDir - inDir);
    if (turnDelta > 180) turnDelta -= 360;

    if (turnDelta > 1) return "left";
    if (turnDelta < -1) return "right";

    const prevPieceId = incoming.c.connectedTo?.pieceId;
    const prevPiece = layoutPieces.find((p) => p.id === prevPieceId);
    if (prevPiece) {
      const prevConnectedIdx = prevPiece.connectors.findIndex(
        (c) => c.connectedTo?.pieceId === anchorPiece.id
      );
      if (prevConnectedIdx >= 0) {
        return detectTrackTurn(prevPiece, prevConnectedIdx, layoutPieces, getCatalogEntry, visited);
      }
    }
    return "straight";
  }

  const anchorEntry = getCatalogEntry(anchorPiece);
  if (anchorEntry?.kind === "curve") {
    return chosenIndex === 1 ? "left" : "right";
  }
  if (anchorEntry?.kind === "turnout-right" || anchorEntry?.kind === "curved-turnout-right") {
    if (chosenIndex === 2) return "right";
  }
  if (anchorEntry?.kind === "turnout-left" || anchorEntry?.kind === "curved-turnout-left") {
    if (chosenIndex === 2) return "left";
  }

  return "straight";
}

export class TrackOperations {
  private ctx: EditorContext;
  private lastPlacedCatalogId: string | null = null;

  constructor(ctx: EditorContext) {
    this.ctx = ctx;
  }

  public getLastPlacedCatalogId(): string | null {
    return this.lastPlacedCatalogId;
  }

  public setLastPlacedCatalogId(id: string | null): void {
    this.lastPlacedCatalogId = id;
  }

  public addPieceToLayout(entry: CatalogEntry, initialPos?: { x: number; y: number }): void {
    this.lastPlacedCatalogId = entry.id;
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();

    let targetTransform: Transform = { x: 0, y: 0, rotation: 0 };
    let targetLayerId = layout.activeLayerId ?? "layer-1";
    let targetElevation: { start: number; end: number } | undefined = undefined;

    // 1. Identify anchor piece: selected piece ONLY
    const selectedIds = selection.all();
    let anchorPiece: TrackPiece | undefined = undefined;
    if (selectedIds.length > 0) {
      anchorPiece = layout.pieces.find((p) => p.id === selectedIds[selectedIds.length - 1]);
      if (anchorPiece) {
        const anchorEntry = this.ctx.getCatalogEntry(anchorPiece);
        if (Boolean(anchorEntry?.isRoad) !== Boolean(entry.isRoad)) {
          anchorPiece = undefined;
        }
      }
    }

    let snappedToAnchor = false;

    if (anchorPiece && !initialPos) {
      // Find open/empty connectors on the selected anchor piece only
      const openConnectors = anchorPiece.connectors
        .map((c, index) => ({ c, index }))
        .filter((item) => !item.c.connectedTo);

      if (openConnectors.length > 0) {
        // Choose best connector: prefer exit/forward connectors (index > 0)
        let chosen = openConnectors.find((item) => item.index > 0) ?? openConnectors[0]!;
        if (entry.kind === "curve" && openConnectors.some((item) => item.index === 2)) {
          chosen = openConnectors.find((item) => item.index === 2)!;
        } else if (entry.kind === "straight" && openConnectors.some((item) => item.index === 1)) {
          chosen = openConnectors.find((item) => item.index === 1)!;
        }

        const locals = getLocalConnectors(entry);
        let movingLocal = locals[0] ?? { x: 0, y: 0, angle: 180 };

        // Check the turn of the track: if turning right, use locals[1] to continue turning right
        if (entry.kind === "curve" && locals.length >= 2) {
          const trackTurn = detectTrackTurn(
            anchorPiece,
            chosen.index,
            layout.pieces,
            (p) => this.ctx.getCatalogEntry(p)
          );
          if (trackTurn === "right") {
            movingLocal = locals[1]!;
          } else {
            movingLocal = locals[0]!;
          }
        }

        // Calculate exact snapped transform
        const snappedRotation = normAngle(chosen.c.angle + 180 - movingLocal.angle);
        const rotatedOffset = rotate(movingLocal, snappedRotation);
        targetTransform = {
          x: chosen.c.x - rotatedOffset.x,
          y: chosen.c.y - rotatedOffset.y,
          rotation: snappedRotation,
        };

        targetLayerId = anchorPiece.layerId || targetLayerId;
        if (anchorPiece.elevation) {
          const elevVal = chosen.index === 0
            ? (anchorPiece.elevation.start ?? 0)
            : (anchorPiece.elevation.end ?? (anchorPiece.elevation.start ?? 0));
          if (Math.abs(elevVal) > 0.01) {
            targetElevation = { start: elevVal, end: elevVal };
          }
        }
        snappedToAnchor = true;
      }
    }

    if (!snappedToAnchor) {
      if (initialPos) {
        targetTransform = { x: initialPos.x, y: initialPos.y, rotation: 0 };
        if (this.ctx.isSnappingEnabled() && layout.pieces.length > 0) {
          const candidateLayer = { layerId: targetLayerId, elevation: targetElevation };
          const snap = findBestSnapForPiece(entry, targetTransform, layout.pieces, undefined, 60, 45, undefined, candidateLayer, layout.layers);
          if (snap) {
            targetTransform = snap.snappedTransform;
          }
        }
      } else if (anchorPiece) {
        targetTransform = {
          x: anchorPiece.transform.x + 200,
          y: anchorPiece.transform.y + 100,
          rotation: anchorPiece.transform.rotation,
        };
      } else if (layout.pieces.length > 0) {
        const last = layout.pieces[layout.pieces.length - 1]!;
        targetTransform = {
          x: last.transform.x + 200,
          y: last.transform.y + 100,
          rotation: last.transform.rotation,
        };
      }
    }

    const id = `track-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`;
    const connectors = computeWorldConnectors(entry, targetTransform);
    const isFeeder = isFeederEntry(entry);
    const isIso = isIsolatingEntry(entry);

    let powerFeed = false;
    if ((entry as any)._initialPowerFeed !== undefined) {
      powerFeed = Boolean((entry as any)._initialPowerFeed);
    } else if (isFeeder) {
      powerFeed = true;
    }

    let isIsolator = false;
    let isolatedConnectorIndex: number | undefined = undefined;
    if ((entry as any)._initialIsolator !== undefined) {
      isIsolator = Boolean((entry as any)._initialIsolator);
      if (isIsolator) isolatedConnectorIndex = 0;
    } else if (isIso) {
      isIsolator = true;
      isolatedConnectorIndex = 0;
    }

    const piece: TrackPiece = {
      id,
      catalogId: entry.id,
      transform: targetTransform,
      connectors,
      powerFeed,
      isIsolator: isIsolator || undefined,
      isolatedConnectorIndex,
      layerId: targetLayerId,
      elevation: targetElevation,
    };

    history.run(new AddPieceCommand(piece));
    updatePieceConnections(layout.pieces, layout.layers);
    selection.set(id);
  }

  /**
   * Hängt das zuletzt platzierte Gleis (oder das Gleis selbst) an einen freien Anschluss an.
   */
  public appendTrackToConnector(
    anchorPiece: TrackPiece,
    connectorIndex: number,
    specificCatalogId?: string
  ): TrackPiece | null {
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();

    const anchorConn = anchorPiece.connectors[connectorIndex];
    if (!anchorConn || anchorConn.connectedTo) return null;

    const catalog = this.ctx.getCatalog();
    const targetCatalogId = specificCatalogId || this.lastPlacedCatalogId || anchorPiece.catalogId;
    let entry =
      catalog.entries.find((e) => e.id === targetCatalogId) ??
      catalogRegistry.findEntry(targetCatalogId) ??
      this.ctx.getCatalogEntry(anchorPiece);

    if (!entry) return null;

    const locals = getLocalConnectors(entry);
    let movingLocal = locals[0] ?? { x: 0, y: 0, angle: 180 };

    // Kurven-Richtung: Fortführung einer Kurve nach links oder rechts
    if (entry.kind === "curve" && locals.length >= 2) {
      const trackTurn = detectTrackTurn(
        anchorPiece,
        connectorIndex,
        layout.pieces,
        (p) => this.ctx.getCatalogEntry(p)
      );
      if (trackTurn === "right") {
        movingLocal = locals[1]!;
      } else {
        movingLocal = locals[0]!;
      }
    }

    const snappedRotation = normAngle(anchorConn.angle + 180 - movingLocal.angle);
    const rotatedOffset = rotate(movingLocal, snappedRotation);
    const targetTransform: Transform = {
      x: anchorConn.x - rotatedOffset.x,
      y: anchorConn.y - rotatedOffset.y,
      rotation: snappedRotation,
    };

    let targetLayerId = anchorPiece.layerId || layout.activeLayerId || "layer-1";
    let targetElevation: { start: number; end: number } | undefined = undefined;
    if (anchorPiece.elevation) {
      const elevVal = connectorIndex === 0
        ? (anchorPiece.elevation.start ?? 0)
        : (anchorPiece.elevation.end ?? (anchorPiece.elevation.start ?? 0));
      if (Math.abs(elevVal) > 0.01) {
        targetElevation = { start: elevVal, end: elevVal };
      }
    }

    const id = `track-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`;
    const connectors = computeWorldConnectors(entry, targetTransform);
    const isFeeder = isFeederEntry(entry);
    const isIso = isIsolatingEntry(entry);

    const piece: TrackPiece = {
      id,
      catalogId: entry.id,
      transform: targetTransform,
      connectors,
      powerFeed: isFeeder,
      isIsolator: isIso || undefined,
      isolatedConnectorIndex: isIso ? 0 : undefined,
      layerId: targetLayerId,
      elevation: targetElevation,
      hasCatenary: anchorPiece.hasCatenary ? true : undefined,
    };

    history.run(new AddPieceCommand(piece));
    updatePieceConnections(layout.pieces, layout.layers);
    selection.set(id);
    this.lastPlacedCatalogId = entry.id;

    const isDe = getLanguage() === "de";
    this.ctx.showToast(
      isDe ? `Gleis ${entry.name || entry.id} hinzugefügt` : `Added track ${entry.name || entry.id}`,
      "➕"
    );

    return piece;
  }

  public addModelToLayout(model: Model3D, customPosition?: { x: number; y: number }): void {
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();

    if (!layout.models) layout.models = [];
    const m: Model3D = { ...model };
    if (customPosition) {
      m.transform = { x: Math.round(customPosition.x), y: Math.round(customPosition.y), rotation: m.transform.rotation };
    } else {
      const center = this.ctx.getRenderer().screenToWorld(window.innerWidth / 2, window.innerHeight / 2);
      m.transform = { x: Math.round(center.x), y: Math.round(center.y), rotation: 0 };
    }
    if (!m.layerId) m.layerId = layout.activeLayerId || layout.layers?.[0]?.id;
    if (m.builtinId === "turntable-trainhall" && this.ctx.isSnappingEnabled()) {
      const ttSnap = snapModelToTurntable(m.transform, layout.pieces, (p) => this.ctx.getCatalogEntry(p));
      if (ttSnap) {
        m.transform = ttSnap.snappedTransform;
      } else {
        const thSnap = snapTrainhallToTrack(m.transform, layout.pieces);
        if (thSnap) m.transform = thSnap.snappedTransform;
      }
    } else if (m.builtinId === "engine-shed" && this.ctx.isSnappingEnabled()) {
      const shedSnap = snapEngineShedToTrack(m.transform, layout.pieces, (p) => this.ctx.getCatalogEntry(p));
      if (shedSnap) m.transform = shedSnap.snappedTransform;
    }
    history.run(new AddModelCommand(m));
    selection.set(m.id);
    this.ctx.sync(true);
  }

  public rotateSelected(deltaDegrees = 7.5): void {
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();
    const selectedIds = selection.all();
    if (selectedIds.length === 0) return;

    const rotatableItems: RotatableItem[] = [];
    for (const id of selectedIds) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece && this.ctx.isPieceInteractive(piece)) {
        const entry = this.ctx.getCatalogEntry(piece);
        rotatableItems.push({ id: piece.id, type: "piece", transform: { ...piece.transform }, entry });
      }
      const model = layout.models?.find((m) => m.id === id);
      if (model && this.ctx.isModelInteractive(model)) {
        rotatableItems.push({ id: model.id, type: "model", transform: { ...model.transform } });
      }
    }

    if (rotatableItems.length === 0) return;

    const newTransforms = rotateGroupAroundCentroid(rotatableItems, deltaDegrees);

    for (const item of rotatableItems) {
      const newT = newTransforms.get(item.id);
      if (!newT) continue;
      const oldT = item.transform;

      if (item.type === "piece") {
        const piece = layout.pieces.find((p) => p.id === item.id);
        if (piece) {
          piece.transform = { ...newT };
          if (item.entry) {
            piece.connectors = computeWorldConnectors(item.entry, newT);
          }
          history.run(new MovePieceCommand(piece.id, oldT, newT));
        }
      } else {
        const model = layout.models?.find((m) => m.id === item.id);
        if (model) {
          model.transform = { ...newT };
          history.run(new MoveModelCommand(model.id, oldT, newT));
        }
      }
    }
    this.ctx.sync(true);
  }

  public deleteSelected(): void {
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();
    const selectedIds = selection.all();

    for (const id of selectedIds) {
      const piece = layout.pieces.find((x) => x.id === id);
      if (piece && this.ctx.isPieceInteractive(piece)) {
        history.run(new RemovePieceCommand(id));
      }
      const model = layout.models?.find((m) => m.id === id);
      if (model && this.ctx.isModelInteractive(model)) {
        history.run(new RemoveModelCommand(id));
      }
    }
    selection.clear();
    this.ctx.sync(true);
  }

  public copySelected(): boolean {
    return copySelected(this.ctx);
  }

  public pasteClipboard(customOffset?: { x: number; y: number }): boolean {
    return pasteClipboard(this.ctx, customOffset, (id) => {
      this.lastPlacedCatalogId = id;
    });
  }

  public duplicateSelected(): void {
    duplicateSelected(this.ctx, (id) => {
      this.lastPlacedCatalogId = id;
    });
  }

  public selectAll(): void {
    const layout = this.ctx.getLayout();
    const selection = this.ctx.getSelection();
    selection.clear();
    for (const p of layout.pieces) {
      if (this.ctx.isPieceInteractive(p)) {
        selection.add(p.id);
      }
    }
    if (layout.models) {
      for (const m of layout.models) {
        if (this.ctx.isModelInteractive(m)) {
          selection.add(m.id);
        }
      }
    }
    this.ctx.sync();
  }

  public flipSelected(): void {
    const layout = this.ctx.getLayout();
    const selection = this.ctx.getSelection();
    const selected = selection.all();
    if (selected.length === 0) return;

    const catalog = this.ctx.getCatalog();
    for (const id of selected) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (!piece) continue;
      const entry = this.ctx.getCatalogEntry(piece);
      if (!entry) continue;

      if (isFlexTrack(entry, piece)) {
        flipFlexTrackPiece(piece, entry);
      } else {
        // Check for opposite partner in catalog (e.g. turnout-left <-> turnout-right)
        let counterpart: CatalogEntry | undefined;
        if (entry.kind === "turnout-left") {
          counterpart = catalog.entries.find((e) => e.kind === "turnout-right" && Math.abs((e.length || 0) - (entry.length || 0)) < 5);
        } else if (entry.kind === "turnout-right") {
          counterpart = catalog.entries.find((e) => e.kind === "turnout-left" && Math.abs((e.length || 0) - (entry.length || 0)) < 5);
        } else if (entry.kind === "curved-turnout-left") {
          counterpart = catalog.entries.find((e) => e.kind === "curved-turnout-right" && Math.abs((e.radius || 0) - (entry.radius || 0)) < 5);
        } else if (entry.kind === "curved-turnout-right") {
          counterpart = catalog.entries.find((e) => e.kind === "curved-turnout-left" && Math.abs((e.radius || 0) - (entry.radius || 0)) < 5);
        }

        if (counterpart) {
          piece.catalogId = counterpart.id;
        } else {
          // Standard piece / curve / straight: rotate 180°
          piece.transform.rotation = normAngle(piece.transform.rotation + 180);
        }
        piece.connectors = computeWorldConnectors(counterpart || entry, piece.transform);
      }
    }
    updatePieceConnections(layout.pieces, layout.layers);
    this.ctx.sync(true);
    const isDe = getLanguage() === "de";
    this.ctx.showToast(isDe ? "Gleise gespiegelt / gewendet" : "Tracks mirrored / flipped", "⇄");
  }

  public selectConnectedTracks(startPieceId?: string): void {
    selectConnectedTracks(this.ctx, startPieceId);
  }

  public selectLayerTracks(layerId?: string): void {
    selectLayerTracks(this.ctx, layerId);
  }

  public toggleGroupSelected(): void {
    toggleGroupSelected(this.ctx);
  }

  public splitTrack(pieceId: string): boolean {
    return splitTrack(this.ctx, pieceId, (id, len) => this.adjustFlexTrack(id, len));
  }

  public replaceTrack(pieceId: string, newCatalogId: string): boolean {
    return replaceTrack(this.ctx, pieceId, newCatalogId);
  }

  public toggleSelectedFeeder(): void {
    toggleSelectedFeeder(this.ctx);
  }

  public toggleSelectedIsolator(): void {
    toggleSelectedIsolator(this.ctx);
  }

  public toggleSelectedCatenary(): void {
    toggleSelectedCatenary(this.ctx);
  }

  public toggleSelectedCatenaryFeeder(): void {
    toggleSelectedCatenaryFeeder(this.ctx);
  }

  public toggleSelectedCatenaryIsolator(): void {
    toggleSelectedCatenaryIsolator(this.ctx);
  }

  public toggleSelectedTurnoutLantern(): void {
    toggleSelectedTurnoutLantern(this.ctx);
  }

  public toggleSelectedTurnoutDecoder(): void {
    toggleSelectedTurnoutDecoder(this.ctx);
  }

  public toggleSelectedSignal(targetState?: boolean): void {
    toggleSelectedSignal(this.ctx, targetState);
  }

  public cycleSelectedSignalSide(): void {
    cycleSelectedSignalSide(this.ctx);
  }

  public cycleSelectedSignalDirection(): void {
    cycleSelectedSignalDirection(this.ctx);
  }

  public setSignalConfig(
    pieceIds: string[],
    config: { hasSignal?: boolean; signalSide?: SignalSide; signalDirection?: SignalDirection },
  ): void {
    setSignalConfig(this.ctx, pieceIds, config);
  }

  public toggleSelectedBridge(targetState?: boolean): void {
    toggleSelectedBridge(this.ctx, targetState);
  }

  public toggleSelectedTunnel(targetState?: boolean): void {
    toggleSelectedTunnel(this.ctx, targetState);
  }

  public moveSelectedToLayer(
    layerId: string,
    elevation?: { start: number; end: number } | Map<string, { start: number; end: number }>,
  ): void {
    moveSelectedToLayer(this.ctx, layerId, elevation);
  }

  public applyHeightInput(target: number): void {
    applyHeightInput(this.ctx, target);
  }

  public applyAutoRamp(onFallbackToDialog?: () => void): boolean {
    return applyAutoRamp(this.ctx, onFallbackToDialog);
  }

  public applyAngleInput(rawVal: number): void {
    applyAngleInput(this.ctx, rawVal);
  }

  public async triggerExportCSV(): Promise<void> {
    const layout = this.ctx.getLayout();
    const catalog = this.ctx.getCatalog();
    const csv = exportPieceListCSV(layout, catalog);
    const defaultName = `${(layout.name || "Gleisplan").replace(/\s+/g, "_")}_Stueckliste.csv`;

    if (window.electronAPI) {
      await window.electronAPI.exportCSV(defaultName, csv);
    } else {
      downloadFile(defaultName, csv, "text/csv;charset=utf-8;");
    }
  }

  public triggerNewPlan(): void {
    const planManager = this.ctx.getPlanManager();
    const planState = planManager.getState();
    const layout = this.ctx.getLayout();
    const hasUnsavedChanges = planState.isDirty && (layout.pieces.length > 0 || (layout.models && layout.models.length > 0));
    if (hasUnsavedChanges) {
      if (!confirm(getTranslations().dialogs.confirmNewPlan)) {
        return;
      }
    }
    const newName = getLanguage() === "de" ? "Neuer Plan" : "New Layout";
    const freshLayout = createLayout(newName);
    freshLayout.scale = layout.scale || "H0";
    this.ctx.setLayout(freshLayout);
    this.ctx.setHistory(new History(freshLayout));
    this.ctx.getSelection().clear();
    this.ctx.getRenderer().invalidateCaches?.();
    this.ctx.getRenderer().resetView();
    const planFileName = getLanguage() === "de" ? "Neuer Plan.railbench" : "New Layout.railbench";
    planManager.setPlanInfo(planFileName, null, "railbench");
    planManager.markDirty(false);
    this.ctx.sync(false);
  }

  public async triggerOpenPlan(): Promise<void> {
    const planManager = this.ctx.getPlanManager();
    const planState = planManager.getState();
    const layout = this.ctx.getLayout();
    const hasUnsavedChanges = planState.isDirty && layout.pieces.length > 0;
    if (hasUnsavedChanges) {
      if (!confirm(getTranslations().dialogs.confirmNewPlan)) {
        return;
      }
    }
    const result = await planManager.openPlan();
    if (result) {
      this.ctx.setLayout(result.layout);
      this.ctx.setHistory(new (this.ctx.getHistory().constructor as any)(result.layout));
      this.ctx.getSelection().clear();
      this.ctx.getRenderer().resetView();
      this.ctx.sync(false);
    }
  }

  public async triggerSavePlan(): Promise<void> {
    await this.ctx.getPlanManager().savePlan(this.ctx.getLayout());
    this.ctx.sync(false);
  }

  public async triggerSavePlanAs(format: PlanFormat = "railbench"): Promise<void> {
    await this.ctx.getPlanManager().savePlanAs(this.ctx.getLayout(), format);
    this.ctx.sync(false);
  }

  public adjustFlexTrack(pieceId: string, cutLength?: number, radius?: number, angle?: number): void {
    const layout = this.ctx.getLayout();
    const piece = layout.pieces.find((p) => p.id === pieceId);
    if (!piece || !this.ctx.isPieceInteractive(piece)) return;
    const entry = this.ctx.getCatalogEntry(piece);
    if (!entry) return;

    if (cutLength !== undefined && cutLength > 0) {
      cutFlexTrack(piece, entry, cutLength);
    }
    bendFlexTrack(piece, entry, radius, angle);
    updatePieceConnections(layout.pieces, layout.layers);
    this.ctx.sync(true);
  }

  public resetFlexTrack(pieceId: string): void {
    const layout = this.ctx.getLayout();
    const piece = layout.pieces.find((p) => p.id === pieceId);
    if (!piece || !this.ctx.isPieceInteractive(piece)) return;
    const entry = this.ctx.getCatalogEntry(piece);
    if (!entry) return;

    resetFlexTrackToOriginal(piece, entry, layout.pieces, layout.layers);
    this.ctx.sync(true);
  }
}
