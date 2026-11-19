import type { Layout, TrackPiece, ColorMode } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import type { Selection } from "@/editor/selection";
import type { History } from "@/editor/commands";
import type { PlanManager } from "@/io/plan-manager";
import type { Ruler } from "@/ui/ruler";
import type { PerfMonitor } from "@/ui/perf-monitor";
import type { TrackOperations } from "./track-operations";
import { getLanguage } from "@/i18n";
import { ICONS } from "@/ui/icons";
import { isTurnoutEntry } from "@/core/simulation";
import { boardSketcher } from "./board-sketcher-controller";
import {
  loadKeymap,
  onKeymapChange,
  matchesAction,
  type KeyActionId,
} from "@/settings/keymap-manager";

export interface KeyboardControllerDeps {
  getLayout: () => Layout;
  getSelection: () => Selection;
  getHistory: () => History;
  getOperations: () => TrackOperations;
  getPlanManager: () => PlanManager;
  getRuler: () => Ruler;
  getPerfMonitor: () => PerfMonitor;
  is3DMode: () => boolean;
  toggle3D: (force?: boolean) => void;
  zoomView?: (factor: number) => void;
  panView?: (dx: number, dy: number) => void;
  resetView?: () => void;
  isMeasureToolActive: () => boolean;
  toggleMeasureTool: (force?: boolean) => void;
  getMeasureState: () => { start: { x: number; y: number } | null };
  setMeasureState: (upd: any) => void;
  updateMeasureHud: () => void;
  isSnappingEnabled: () => boolean;
  setSnappingEnabled: (val: boolean) => void;
  getRotationStep?: () => number;
  toggleSelectionLock?: () => void;
  getColorMode: () => ColorMode;
  cycleColorMode: (target?: ColorMode) => void;
  isTrackLabelsEnabled: () => boolean;
  setTrackLabelsEnabled: (val: boolean) => void;
  isPowerSimEnabled: () => boolean;
  setPowerSimEnabled: (val: boolean) => void;
  toggleRightSidebar: () => void;
  openHeightDialogForSelection: () => void;
  openHelpDialog: (tab?: any) => void;
  openPrintDialog: () => void;
  getCatalogEntryForPiece: (piece: TrackPiece) => CatalogEntry | undefined;
  sync: (dirty?: boolean) => void;
  showToast: (msg: string, icon?: string) => void;
}

export class KeyboardController {
  private deps: KeyboardControllerDeps;
  private handler: (e: KeyboardEvent) => void;
  private currentKeymap = loadKeymap();
  private unbindKeymapListener: () => void;

  constructor(deps: KeyboardControllerDeps) {
    this.deps = deps;
    this.handler = this.onKeyDown.bind(this);
    if (typeof window !== "undefined") {
      window.addEventListener("keydown", this.handler);
    }
    this.unbindKeymapListener = onKeymapChange((k) => {
      this.currentKeymap = k;
    });
  }

  public destroy(): void {
    if (typeof window !== "undefined") {
      window.removeEventListener("keydown", this.handler);
    }
    this.unbindKeymapListener();
  }

  private match(e: KeyboardEvent, actionId: KeyActionId): boolean {
    return matchesAction(e, actionId, this.currentKeymap);
  }

  private onKeyDown(e: KeyboardEvent): void {
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || (target as any).isContentEditable)) {
      return;
    }

    if (boardSketcher.isActive) {
      if (e.key === "Escape") {
        e.preventDefault();
        boardSketcher.stop();
        this.deps.sync();
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        if (boardSketcher.mode === "poly" && boardSketcher.points.length >= 3) {
          boardSketcher.finishPoly(this.deps.getLayout());
          this.deps.sync(true);
        } else if (boardSketcher.mode === "edit") {
          boardSketcher.stop();
          this.deps.sync(true);
        }
        return;
      }
      if (e.key === "Backspace" && boardSketcher.mode === "poly") {
        e.preventDefault();
        boardSketcher.undoLastPoint();
        this.deps.sync();
        return;
      }
    }

    const ops = this.deps.getOperations();
    const layout = this.deps.getLayout();
    const selection = this.deps.getSelection();

    // 1. Zooming (+ / -)
    if (this.match(e, "zoomIn")) {
      e.preventDefault();
      this.deps.zoomView?.(1.15);
      return;
    }
    if (this.match(e, "zoomOut")) {
      e.preventDefault();
      this.deps.zoomView?.(1 / 1.15);
      return;
    }

    // 2. View Panning (Arrow Keys)
    if (this.match(e, "panUp")) {
      e.preventDefault();
      const step = e.shiftKey ? 180 : 60;
      this.deps.panView?.(0, step);
      return;
    }
    if (this.match(e, "panDown")) {
      e.preventDefault();
      const step = e.shiftKey ? 180 : 60;
      this.deps.panView?.(0, -step);
      return;
    }
    if (this.match(e, "panLeft")) {
      e.preventDefault();
      const step = e.shiftKey ? 180 : 60;
      this.deps.panView?.(step, 0);
      return;
    }
    if (this.match(e, "panRight")) {
      e.preventDefault();
      const step = e.shiftKey ? 180 : 60;
      this.deps.panView?.(-step, 0);
      return;
    }

    // 3. Reset View
    if (this.match(e, "resetView")) {
      e.preventDefault();
      this.deps.resetView?.();
      return;
    }

    // 4. File operations
    if (this.match(e, "savePlanAs")) {
      e.preventDefault();
      ops.triggerSavePlanAs("railbench");
      return;
    }
    if (this.match(e, "savePlan")) {
      e.preventDefault();
      ops.triggerSavePlan();
      return;
    }
    if (this.match(e, "newPlan")) {
      e.preventDefault();
      ops.triggerNewPlan();
      return;
    }
    if (this.match(e, "openPlan")) {
      e.preventDefault();
      ops.triggerOpenPlan();
      return;
    }
    if (this.match(e, "exportCsv")) {
      e.preventDefault();
      ops.triggerExportCSV();
      return;
    }
    if (this.match(e, "printPlan")) {
      e.preventDefault();
      this.deps.openPrintDialog();
      return;
    }

    // 5. History (Undo / Redo)
    if (this.match(e, "redo")) {
      e.preventDefault();
      this.deps.getHistory().redo();
      this.deps.sync(true);
      return;
    }
    if (this.match(e, "undo")) {
      e.preventDefault();
      this.deps.getHistory().undo();
      this.deps.sync(true);
      return;
    }

    // 6. Clipboard & Selection
    if (this.match(e, "copy")) {
      const textSelected = window.getSelection()?.toString();
      if (!textSelected || textSelected.trim().length === 0) {
        e.preventDefault();
        const ok = ops.copySelected();
        if (!ok) {
          const isDe = getLanguage() === "de";
          this.deps.showToast(isDe ? "Keine Gleise zum Kopieren markiert" : "No tracks selected to copy", ICONS.alertTriangle);
        }
      }
      return;
    }
    if (this.match(e, "paste")) {
      e.preventDefault();
      ops.pasteClipboard();
      return;
    }
    if (this.match(e, "duplicate")) {
      e.preventDefault();
      ops.duplicateSelected();
      return;
    }
    if (this.match(e, "selectAll")) {
      e.preventDefault();
      ops.selectAll();
      return;
    }
    if (this.match(e, "deleteSelected")) {
      e.preventDefault();
      ops.deleteSelected();
      return;
    }

    // 7. Rotation
    if (this.match(e, "rotateCcw")) {
      e.preventDefault();
      const rotStep = this.deps.getRotationStep ? this.deps.getRotationStep() : 7.5;
      ops.rotateSelected(-rotStep);
      return;
    }
    if (this.match(e, "rotateCw")) {
      e.preventDefault();
      const rotStep = this.deps.getRotationStep ? this.deps.getRotationStep() : 7.5;
      ops.rotateSelected(rotStep);
      return;
    }

    // 8. Turnout controls
    if (this.match(e, "toggleTurnoutDecoder")) {
      e.preventDefault();
      ops.toggleSelectedTurnoutDecoder();
      return;
    }
    if (this.match(e, "toggleTurnoutLantern")) {
      e.preventDefault();
      const selPieces = selection.all().map((id) => layout.pieces.find((p) => p.id === id)).filter(Boolean) as TrackPiece[];
      const turnoutPieces = selPieces.filter((p) => {
        const entry = this.deps.getCatalogEntryForPiece(p);
        return isTurnoutEntry(entry);
      });
      if (turnoutPieces.length > 0) {
        ops.toggleSelectedTurnoutLantern();
      } else {
        const isDe = getLanguage() === "de";
        this.deps.showToast(
          isDe
            ? "Weichen-Taste: Bitte zuerst eine Weiche markieren (Laterne: W, Decoder: Umschalt+W)"
            : "Turnout key: Please select a turnout first (Lantern: W, Decoder: Shift+W)",
          ICONS.lantern,
        );
      }
      return;
    }

    // 9. Electrical & Track features
    if (this.match(e, "toggleCatenary")) {
      e.preventDefault();
      ops.toggleSelectedCatenary();
      return;
    }
    if (this.match(e, "toggleSignal")) {
      e.preventDefault();
      ops.toggleSelectedSignal();
      return;
    }
    if (this.match(e, "toggleFeeder")) {
      e.preventDefault();
      if (e.shiftKey) {
        ops.toggleSelectedCatenaryFeeder();
      } else {
        ops.toggleSelectedFeeder();
      }
      return;
    }
    if (this.match(e, "toggleIsolator")) {
      e.preventDefault();
      if (e.shiftKey) {
        ops.toggleSelectedCatenaryIsolator();
      } else {
        ops.toggleSelectedIsolator();
      }
      return;
    }
    if (this.match(e, "toggleBridge")) {
      e.preventDefault();
      ops.toggleSelectedBridge();
      return;
    }
    if (this.match(e, "toggleTunnel")) {
      e.preventDefault();
      ops.toggleSelectedTunnel();
      return;
    }

    // 10. Toggles & Tools
    if (this.match(e, "toggleSnapping")) {
      e.preventDefault();
      this.deps.setSnappingEnabled(!this.deps.isSnappingEnabled());
      this.deps.sync();
      return;
    }
    if (this.match(e, "toggleTrackLabels")) {
      e.preventDefault();
      this.deps.setTrackLabelsEnabled(!this.deps.isTrackLabelsEnabled());
      this.deps.sync();
      return;
    }
    if (this.match(e, "togglePowerSim")) {
      e.preventDefault();
      this.deps.setPowerSimEnabled(!this.deps.isPowerSimEnabled());
      this.deps.sync();
      return;
    }
    if (this.match(e, "toggleMeasure")) {
      e.preventDefault();
      if (e.shiftKey) {
        ops.applyAutoRamp(() => this.deps.openHeightDialogForSelection());
      } else if (selection.all().length > 0 && !this.deps.isMeasureToolActive()) {
        this.deps.openHeightDialogForSelection();
      } else {
        this.deps.toggleMeasureTool();
      }
      return;
    }
    if (this.match(e, "toggleSelectionLock")) {
      e.preventDefault();
      this.deps.toggleSelectionLock?.();
      return;
    }
    if (this.match(e, "toggleRuler")) {
      e.preventDefault();
      this.deps.getRuler().toggle();
      this.deps.sync();
      return;
    }
    if (this.match(e, "toggleLayersSidebar")) {
      e.preventDefault();
      this.deps.toggleRightSidebar();
      return;
    }
    if (this.match(e, "toggle3D")) {
      e.preventDefault();
      this.deps.toggle3D();
      return;
    }
    if (this.match(e, "deselectAll")) {
      e.preventDefault();
      if (this.deps.isMeasureToolActive()) {
        const mState = this.deps.getMeasureState();
        if (mState.start) {
          this.deps.setMeasureState({ start: null, current: null, locked: false, draggingPoint: null });
          this.deps.updateMeasureHud();
          this.deps.sync();
        } else {
          this.deps.toggleMeasureTool(false);
        }
        return;
      }
      if (this.deps.is3DMode()) {
        this.deps.toggle3D(false);
        return;
      }
      selection.clear();
      this.deps.sync();
      return;
    }
    if (this.match(e, "openHelp")) {
      e.preventDefault();
      this.deps.openHelpDialog("overview");
      return;
    }
    if (this.match(e, "togglePerfMonitor")) {
      e.preventDefault();
      this.deps.getPerfMonitor().toggle();
      return;
    }

    // Color mode cycle (C key)
    if (e.key.toLowerCase() === "c" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      this.deps.cycleColorMode();
      return;
    }
  }
}
