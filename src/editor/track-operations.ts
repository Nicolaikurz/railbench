import type { Layout, TrackPiece, Model3D, Transform } from "@/core/types";
import { ensureDefaultLayers } from "@/core/layout";
import {
  computeWorldConnectors,
  getLocalConnectors,
  isFeederEntry,
  isIsolatingEntry,
  type Catalog,
  type CatalogEntry,
} from "@/catalog/catalog";
import { findBestSnapForPiece, updatePieceConnections } from "@/core/snapping";
import { snapModelToTurntable } from "@/core/turntable-snap";
import { cutFlexTrack, bendFlexTrack } from "@/core/flextrack";
import {
  AddPieceCommand,
  AddPiecesCommand,
  RemovePieceCommand,
  MovePieceCommand,
  ToggleFeederCommand,
  ToggleIsolatorCommand,
  ToggleCatenaryCommand,
  ToggleCatenaryFeederCommand,
  ToggleCatenaryIsolatorCommand,
  ToggleTurnoutLanternCommand,
  ToggleTurnoutDecoderCommand,
  MovePiecesToLayerCommand,
  AddModelCommand,
  RemoveModelCommand,
  MoveModelCommand,
} from "@/editor/commands";
import type { History } from "@/editor/commands";
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

interface TrackClipboardData {
  pieces: TrackPiece[];
  models: Model3D[];
}

let trackClipboard: TrackClipboardData | null = null;
let pasteOffsetCount = 1;

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

  constructor(ctx: EditorContext) {
    this.ctx = ctx;
  }

  public addPieceToLayout(entry: CatalogEntry, initialPos?: { x: number; y: number }): void {
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
    const isFirst = layout.pieces.length === 0;

    const isFeeder = isFeederEntry(entry);
    const isIso = isIsolatingEntry(entry);

    let powerFeed = isFirst;
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
      if (ttSnap) m.transform = ttSnap.snappedTransform;
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
    const layout = this.ctx.getLayout();
    const selection = this.ctx.getSelection();
    const selectedIds = selection.all();
    if (selectedIds.length === 0) return false;

    const piecesToCopy: TrackPiece[] = [];
    for (const id of selectedIds) {
      const p = layout.pieces.find((x) => x.id === id);
      if (p) {
        piecesToCopy.push(JSON.parse(JSON.stringify(p)));
      }
    }

    const modelsToCopy: Model3D[] = [];
    if (layout.models) {
      for (const id of selectedIds) {
        const m = layout.models.find((x) => x.id === id);
        if (m) {
          modelsToCopy.push(JSON.parse(JSON.stringify(m)));
        }
      }
    }

    if (piecesToCopy.length > 0 || modelsToCopy.length > 0) {
      trackClipboard = { pieces: piecesToCopy, models: modelsToCopy };
      pasteOffsetCount = 1;

      try {
        const payload = JSON.stringify(trackClipboard);
        localStorage.setItem("railbench_clipboard", payload);
        if (navigator.clipboard?.writeText) {
          navigator.clipboard.writeText(payload).catch(() => {});
        }
      } catch {}

      const total = piecesToCopy.length + modelsToCopy.length;
      const isDe = getLanguage() === "de";
      const itemLabel = isDe
        ? `${total} Element${total === 1 ? "" : "e"}`
        : `${total} item${total === 1 ? "" : "s"}`;
      this.ctx.showToast(isDe ? `${itemLabel} in Zwischenablage kopiert (Strg+C)` : `${itemLabel} copied to clipboard (Ctrl+C)`, "📋");
      return true;
    }
    return false;
  }

  public pasteClipboard(customOffset?: { x: number; y: number }): boolean {
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();

    if (!trackClipboard || (trackClipboard.pieces.length === 0 && trackClipboard.models.length === 0)) {
      try {
        const stored = localStorage.getItem("railbench_clipboard");
        if (stored) {
          trackClipboard = JSON.parse(stored);
        }
      } catch {}
    }

    if (!trackClipboard || (trackClipboard.pieces.length === 0 && trackClipboard.models.length === 0)) {
      const isDe = getLanguage() === "de";
      this.ctx.showToast(isDe ? "Zwischenablage ist leer" : "Clipboard is empty", "⚠️");
      return false;
    }

    const offset = customOffset ?? {
      x: 40 * pasteOffsetCount,
      y: 40 * pasteOffsetCount,
    };
    if (!customOffset) {
      pasteOffsetCount++;
    }

    const newPieces: TrackPiece[] = [];
    const newModels: Model3D[] = [];
    const newIds: string[] = [];
    const now = Date.now();

    for (let i = 0; i < trackClipboard.pieces.length; i++) {
      const orig = trackClipboard.pieces[i];
      if (!orig) continue;
      const newId = `p-${now}-${i}-${Math.random().toString(36).substring(2, 7)}`;
      const entry = this.ctx.getCatalogEntry(orig);
      const newTransform = {
        x: orig.transform.x + offset.x,
        y: orig.transform.y + offset.y,
        rotation: orig.transform.rotation,
      };

      const copyPiece: TrackPiece = {
        ...JSON.parse(JSON.stringify(orig)),
        id: newId,
        transform: newTransform,
        connectors: entry ? computeWorldConnectors(entry, newTransform) : orig.connectors,
      };
      newPieces.push(copyPiece);
      newIds.push(newId);
    }

    for (let i = 0; i < trackClipboard.models.length; i++) {
      const orig = trackClipboard.models[i];
      if (!orig) continue;
      const newId = `m-${now}-${i}-${Math.random().toString(36).substring(2, 7)}`;
      const copyModel: Model3D = {
        ...JSON.parse(JSON.stringify(orig)),
        id: newId,
        transform: {
          ...orig.transform,
          x: orig.transform.x + offset.x,
          y: orig.transform.y + offset.y,
        },
      };
      newModels.push(copyModel);
      newIds.push(newId);
    }

    if (newPieces.length > 0 || newModels.length > 0) {
      history.run(new AddPiecesCommand(newPieces, newModels));
      selection.clear();
      for (const id of newIds) {
        selection.add(id);
      }
      this.ctx.sync(true);

      const total = newPieces.length + newModels.length;
      const isDe = getLanguage() === "de";
      const itemLabel = isDe
        ? `${total} Element${total === 1 ? "" : "e"}`
        : `${total} item${total === 1 ? "" : "s"}`;
      this.ctx.showToast(isDe ? `${itemLabel} eingefügt (Strg+V)` : `${itemLabel} pasted (Ctrl+V)`, "📥");
      return true;
    }
    return false;
  }

  public duplicateSelected(): void {
    const selection = this.ctx.getSelection();
    const selectedIds = selection.all();
    if (selectedIds.length === 0) {
      const isDe = getLanguage() === "de";
      this.ctx.showToast(isDe ? "Keine Gleise zum Duplizieren markiert" : "No tracks selected to duplicate", "⚠️");
      return;
    }
    this.copySelected();
    this.pasteClipboard({ x: 40, y: 40 });
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

  public toggleSelectedFeeder(): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected[0]) {
      history.run(new ToggleFeederCommand(selected[0]));
      this.ctx.sync(true);
    }
  }

  public toggleSelectedIsolator(): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected[0]) {
      history.run(new ToggleIsolatorCommand(selected[0]));
      this.ctx.sync(true);
    }
  }

  public toggleSelectedCatenary(): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleCatenaryCommand(selected));
      this.ctx.sync(true);
    }
  }

  public toggleSelectedCatenaryFeeder(): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleCatenaryFeederCommand(selected));
      this.ctx.sync(true);
    }
  }

  public toggleSelectedCatenaryIsolator(): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleCatenaryIsolatorCommand(selected));
      this.ctx.sync(true);
    }
  }

  public toggleSelectedTurnoutLantern(): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleTurnoutLanternCommand(selected));
      this.ctx.sync(true);
    }
  }

  public toggleSelectedTurnoutDecoder(): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleTurnoutDecoderCommand(selected));
      this.ctx.sync(true);
    }
  }

  public moveSelectedToLayer(
    layerId: string,
    elevation?: { start: number; end: number } | Map<string, { start: number; end: number }>,
  ): void {
    const selection = this.ctx.getSelection();
    const history = this.ctx.getHistory();
    const selected = selection.all();
    if (selected.length === 0) return;
    history.run(new MovePiecesToLayerCommand(selected, layerId, elevation));
    this.ctx.sync(true);
  }

  public applyHeightInput(target: number): void {
    if (isNaN(target)) return;
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();
    const layers = layout.layers || [];

    const byLayer = new Map<string, string[]>();
    for (const id of selection.all()) {
      const p = layout.pieces.find((x) => x.id === id);
      if (!p || !this.ctx.isPieceInteractive(p)) continue;
      const lid = p.layerId || "layer-1";
      const base = layers.find((l) => l.id === lid)?.heightMm ?? 0;
      const elev = target - base;
      if (Math.abs((p.elevation?.start ?? 0) - elev) < 0.05 && Math.abs((p.elevation?.end ?? 0) - elev) < 0.05) continue;
      if (!byLayer.has(lid)) byLayer.set(lid, []);
      byLayer.get(lid)!.push(id);
    }
    if (byLayer.size === 0) return;

    for (const [lid, ids] of byLayer) {
      const base = layers.find((l) => l.id === lid)?.heightMm ?? 0;
      const elev = target - base;
      history.run(new MovePiecesToLayerCommand(ids, lid, { start: elev, end: elev }));
    }
    this.ctx.sync(true);
  }

  public applyAngleInput(rawVal: number): void {
    if (isNaN(rawVal)) return;
    const layout = this.ctx.getLayout();
    const history = this.ctx.getHistory();
    const selection = this.ctx.getSelection();
    const targetAngle = normAngle(rawVal);
    const selectedIds = selection.all();
    if (selectedIds.length === 0) return;

    const firstPiece = layout.pieces.find((p) => p.id === selectedIds[0]);
    const firstModel = layout.models?.find((m) => m.id === selectedIds[0]);
    const currentBaseAngle = firstPiece ? firstPiece.transform.rotation : (firstModel ? firstModel.transform.rotation : 0);
    const delta = targetAngle - currentBaseAngle;

    for (const id of selectedIds) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece && this.ctx.isPieceInteractive(piece)) {
        const oldT = { ...piece.transform };
        const newRot = selectedIds.length === 1 ? targetAngle : normAngle(piece.transform.rotation + delta);
        const newT: Transform = { ...piece.transform, rotation: newRot };
        const entry = this.ctx.getCatalogEntry(piece);
        if (entry) {
          piece.transform = newT;
          piece.connectors = computeWorldConnectors(entry, newT);
          history.run(new MovePieceCommand(piece.id, oldT, newT));
        }
      }
      const model = layout.models?.find((m) => m.id === id);
      if (model) {
        const oldT = { ...model.transform };
        const newRot = selectedIds.length === 1 ? targetAngle : normAngle(model.transform.rotation + delta);
        const newT: Transform = { ...oldT, rotation: newRot };
        model.transform = newT;
        history.run(new MoveModelCommand(model.id, oldT, newT));
      }
    }
    this.ctx.sync(true);
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
    layout.pieces = [];
    layout.models = [];
    ensureDefaultLayers(layout);
    this.ctx.getSelection().clear();
    const hist = this.ctx.getHistory();
    if (hist && typeof (hist as any).clear === "function") {
      (hist as any).clear();
    }
    const newName = getLanguage() === "de" ? "Neuer Plan.railbench" : "New Layout.railbench";
    planManager.setPlanInfo(newName, null, "railbench");
    this.ctx.getRenderer().resetView();
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
    if (radius !== undefined || angle !== undefined) {
      bendFlexTrack(piece, entry, radius, angle);
    }
    updatePieceConnections(layout.pieces, layout.layers);
    this.ctx.sync(true);
  }
}
