import type { Layout, TrackPiece, ColorMode } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import type { Selection } from "@/editor/selection";
import type { History } from "@/editor/commands";
import type { PlanManager } from "@/io/plan-manager";
import type { Ruler } from "@/ui/ruler";
import type { PerfMonitor } from "@/ui/perf-monitor";
import type { TrackOperations } from "./track-operations";
import { getLanguage, getTranslations } from "@/i18n";
import { isTurnoutEntry } from "@/core/simulation";

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
  isMeasureToolActive: () => boolean;
  toggleMeasureTool: (force?: boolean) => void;
  getMeasureState: () => { start: { x: number; y: number } | null };
  setMeasureState: (upd: any) => void;
  updateMeasureHud: () => void;
  isSnappingEnabled: () => boolean;
  setSnappingEnabled: (val: boolean) => void;
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

  constructor(deps: KeyboardControllerDeps) {
    this.deps = deps;
    this.handler = this.onKeyDown.bind(this);
    window.addEventListener("keydown", this.handler);
  }

  public destroy(): void {
    window.removeEventListener("keydown", this.handler);
  }

  private onKeyDown(e: KeyboardEvent): void {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

    const ctrlOrMeta = e.ctrlKey || e.metaKey;
    const ops = this.deps.getOperations();
    const layout = this.deps.getLayout();
    const selection = this.deps.getSelection();

    if (ctrlOrMeta && (e.key.toLowerCase() === "s" || e.code === "KeyS")) {
      e.preventDefault();
      if (e.shiftKey) {
        ops.triggerSavePlanAs("railbench");
      } else {
        ops.triggerSavePlan();
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "n" || e.code === "KeyN")) {
      e.preventDefault();
      if (confirm(getTranslations().dialogs.confirmNewPlan)) {
        layout.pieces = [];
        selection.clear();
        this.deps.getPlanManager().setPlanInfo("New Layout.railbench", null, "railbench");
        this.deps.sync(true);
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "o" || e.code === "KeyO")) {
      e.preventDefault();
      ops.triggerOpenPlan();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "e" || e.code === "KeyE")) {
      e.preventDefault();
      ops.triggerExportCSV();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "p" || e.code === "KeyP")) {
      e.preventDefault();
      this.deps.openPrintDialog();
    } else if (e.key.toLowerCase() === "m" && !ctrlOrMeta) {
      e.preventDefault();
      if (selection.all().length > 0 && !this.deps.isMeasureToolActive()) {
        this.deps.openHeightDialogForSelection();
      } else {
        this.deps.toggleMeasureTool();
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "z" || e.code === "KeyZ")) {
      e.preventDefault();
      if (e.shiftKey) this.deps.getHistory().redo();
      else this.deps.getHistory().undo();
      this.deps.sync(true);
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "y" || e.code === "KeyY")) {
      e.preventDefault();
      this.deps.getHistory().redo();
      this.deps.sync(true);
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "c" || e.code === "KeyC")) {
      const textSelected = window.getSelection()?.toString();
      if (!textSelected || textSelected.trim().length === 0) {
        e.preventDefault();
        const ok = ops.copySelected();
        if (!ok) {
          const isDe = getLanguage() === "de";
          this.deps.showToast(isDe ? "Keine Gleise zum Kopieren markiert" : "No tracks selected to copy", "⚠️");
        }
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "v" || e.code === "KeyV")) {
      e.preventDefault();
      ops.pasteClipboard();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "d" || e.code === "KeyD")) {
      e.preventDefault();
      ops.duplicateSelected();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "a" || e.code === "KeyA")) {
      e.preventDefault();
      ops.selectAll();
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      ops.deleteSelected();
    } else if (e.key.toLowerCase() === "r" || e.code === "KeyR") {
      e.preventDefault();
      const step = e.shiftKey ? -7.5 : 7.5;
      ops.rotateSelected(step);
    } else if ((e.key.toLowerCase() === "w" || e.code === "KeyW") && !ctrlOrMeta) {
      e.preventDefault();
      const selPieces = selection.all().map((id) => layout.pieces.find((p) => p.id === id)).filter(Boolean) as TrackPiece[];
      const turnoutPieces = selPieces.filter((p) => {
        const entry = this.deps.getCatalogEntryForPiece(p);
        return isTurnoutEntry(entry);
      });
      if (turnoutPieces.length > 0) {
        if (e.shiftKey) {
          ops.toggleSelectedTurnoutDecoder();
        } else {
          ops.toggleSelectedTurnoutLantern();
        }
      } else {
        const isDe = getLanguage() === "de";
        this.deps.showToast(
          isDe
            ? "Weichen-Taste W: Bitte zuerst eine Weiche markieren (Laterne: W, Decoder: Umschalt+W)"
            : "Turnout key W: Please select a turnout first (Lantern: W, Decoder: Shift+W)",
          "💡",
        );
      }
    } else if (e.key.toLowerCase() === "f" && !ctrlOrMeta) {
      e.preventDefault();
      if (e.shiftKey) {
        ops.toggleSelectedCatenaryFeeder();
      } else {
        ops.toggleSelectedFeeder();
      }
    } else if (e.key.toLowerCase() === "i" && !ctrlOrMeta) {
      e.preventDefault();
      if (e.shiftKey) {
        ops.toggleSelectedCatenaryIsolator();
      } else {
        ops.toggleSelectedIsolator();
      }
    } else if (e.key.toLowerCase() === "o" && !ctrlOrMeta) {
      e.preventDefault();
      ops.toggleSelectedCatenary();
    } else if (e.key.toLowerCase() === "s" && !ctrlOrMeta) {
      e.preventDefault();
      this.deps.setSnappingEnabled(!this.deps.isSnappingEnabled());
      this.deps.sync();
    } else if (e.key.toLowerCase() === "c" && !ctrlOrMeta) {
      e.preventDefault();
      this.deps.cycleColorMode();
    } else if (e.key.toLowerCase() === "n" && !ctrlOrMeta) {
      e.preventDefault();
      this.deps.setTrackLabelsEnabled(!this.deps.isTrackLabelsEnabled());
      this.deps.sync();
    } else if (e.key.toLowerCase() === "p" && !ctrlOrMeta) {
      e.preventDefault();
      this.deps.setPowerSimEnabled(!this.deps.isPowerSimEnabled());
      this.deps.sync();
    } else if (e.key.toLowerCase() === "b" && e.shiftKey && !ctrlOrMeta) {
      e.preventDefault();
      this.deps.toggleRightSidebar();
    } else if (e.altKey && (e.key.toLowerCase() === "l" || e.code === "KeyL")) {
      e.preventDefault();
      this.deps.toggleSelectionLock?.();
    } else if (e.key.toLowerCase() === "l" && !ctrlOrMeta) {
      e.preventDefault();
      if (e.shiftKey) {
        ops.toggleSelectedTurnoutLantern();
      } else {
        this.deps.getRuler().toggle();
        this.deps.sync();
      }
    } else if (e.key.toLowerCase() === "d" && e.shiftKey && !ctrlOrMeta) {
      e.preventDefault();
      ops.toggleSelectedTurnoutDecoder();
    } else if (e.key === "Escape" && this.deps.isMeasureToolActive()) {
      e.preventDefault();
      const mState = this.deps.getMeasureState();
      if (mState.start) {
        this.deps.setMeasureState({ start: null, current: null, locked: false, draggingPoint: null });
        this.deps.updateMeasureHud();
        this.deps.sync();
      } else {
        this.deps.toggleMeasureTool(false);
      }
    } else if (e.key === "Escape" && this.deps.is3DMode()) {
      e.preventDefault();
      this.deps.toggle3D(false);
    } else if (e.key === "3" || (e.key.toLowerCase() === "v" && !ctrlOrMeta)) {
      e.preventDefault();
      this.deps.toggle3D();
    } else if (e.key === "F1" || (e.key === "?" && !ctrlOrMeta)) {
      e.preventDefault();
      this.deps.openHelpDialog("overview");
    } else if (e.key === "F3") {
      e.preventDefault();
      this.deps.getPerfMonitor().toggle();
    }
  }
}
