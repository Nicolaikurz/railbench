import type { Layout, TrackPiece, Model3D, Transform } from "@/core/types";
import { computeWorldConnectors, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import { findBestSnapForPiece } from "@/core/snapping";
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

    if (initialPos) {
      targetTransform = { x: initialPos.x, y: initialPos.y, rotation: 0 };
    } else if (layout.pieces.length > 0) {
      const last = layout.pieces[layout.pieces.length - 1]!;
      targetTransform = { x: last.transform.x + 190, y: last.transform.y, rotation: last.transform.rotation };
    }

    if (this.ctx.isSnappingEnabled() && layout.pieces.length > 0) {
      const candidateLayer = { layerId: layout.activeLayerId ?? "layer-1" };
      const snap = findBestSnapForPiece(entry, targetTransform, layout.pieces, undefined, 60, 45, undefined, candidateLayer, layout.layers);
      if (snap) {
        targetTransform = snap.snappedTransform;
      }
    }

    const id = `track-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`;
    const connectors = computeWorldConnectors(entry, targetTransform);
    const isFirst = layout.pieces.length === 0;

    const piece: TrackPiece = {
      id,
      catalogId: entry.id,
      transform: targetTransform,
      connectors,
      powerFeed: isFirst,
      layerId: layout.activeLayerId ?? "layer-1",
    };

    history.run(new AddPieceCommand(piece));
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

  public async triggerOpenPlan(): Promise<void> {
    const planManager = this.ctx.getPlanManager();
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
}
