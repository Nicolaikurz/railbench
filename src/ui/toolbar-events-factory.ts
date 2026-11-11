import type { ToolbarEvents } from "@/ui/toolbar";
import type { Layout, ColorMode } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import type { TrackOperations } from "@/editor/track-operations";
import type { Selection } from "@/editor/selection";
import type { History } from "@/editor/commands";
import type { PlanManager, PlanFormat } from "@/io/plan-manager";
import type { Renderer } from "@/render/renderer";
import type { Ruler } from "@/ui/ruler";
import type { Viewer3D } from "@/render/viewer-3d";
import type { Sidebar } from "@/ui/sidebar";
import type { PerfMonitor } from "@/ui/perf-monitor";
import { themeManager } from "@/theme/theme-manager";
import { openPowerSettingsDialog } from "@/ui/power-dialog";
import { openHelpDialog, type HelpTab } from "@/ui/help-dialog";
import { openPrintDialog } from "@/io/print";
import { getTranslations, getLanguage } from "@/i18n";
import { ensureDefaultLayers, getBaseboards } from "@/core/layout";
import { isTurnoutEntry, type ShortCircuitDetectionResult } from "@/core/simulation";
import { resetPersonalConfig } from "@/settings/settings-manager";

export interface ToolbarFactoryDeps {
  getLayout: () => Layout;
  getCatalog: () => Catalog;
  getSelection: () => Selection;
  getHistory: () => History;
  getOperations: () => TrackOperations;
  getPlanManager: () => PlanManager;
  getRenderer: () => Renderer;
  getRuler: () => Ruler;
  getViewer3D: () => Viewer3D;
  getSidebar: () => Sidebar | null;
  getPerfMonitor: () => PerfMonitor;
  getCurrentShortCircuit: () => ShortCircuitDetectionResult | undefined;
  is3DMode: () => boolean;
  toggle3D: () => void;
  toggleMeasureTool: () => void;
  toggleRightSidebar: () => void;
  onResetUiSizes?: () => void;
  onToggleLockBars?: () => void;
  saveSettingsState: () => void;
  sync: (dirty?: boolean) => void;
  isSnappingEnabled: () => boolean;
  setSnappingEnabled: (val: boolean) => void;
  toggleSelectionLock?: () => void;
  cycleColorMode: (mode?: ColorMode) => void;
  isTrackLabelsEnabled: () => boolean;
  setTrackLabelsEnabled: (val: boolean) => void;
  isPowerSimEnabled: () => boolean;
  setPowerSimEnabled: (val: boolean) => void;
  openHeightDialogForSelection: () => void;
  triggerCollabDialog: () => void;
  triggerWelcomeDialog?: () => void;
  triggerDemoTracksDialog?: () => void;
  triggerImportCatalog?: () => void;
  showToast: (msg: string, icon?: string) => void;
}

export function buildToolbarEvents(deps: ToolbarFactoryDeps): ToolbarEvents {
  const layout = deps.getLayout();
  const operations = deps.getOperations();
  const selection = deps.getSelection();
  const planManager = deps.getPlanManager();
  const history = deps.getHistory();
  const renderer = deps.getRenderer();
  const ruler = deps.getRuler();
  const viewer3D = deps.getViewer3D();

  return {
    onToggleRightSidebar: () => deps.toggleRightSidebar(),
    onNewPlan: () => operations.triggerNewPlan(),
    onOpenDemoTracks: () => deps.triggerDemoTracksDialog?.(),
    onUndo: () => {
      history.undo();
      deps.sync(true);
    },
    onRedo: () => {
      history.redo();
      deps.sync(true);
    },
    onCopy: () => operations.copySelected(),
    onPaste: () => operations.pasteClipboard(),
    onDuplicate: () => operations.duplicateSelected(),
    onSelectAll: () => operations.selectAll(),
    onDeselectAll: () => {
      selection.clear();
      deps.sync();
    },
    onDeleteSelected: () => operations.deleteSelected(),
    onRotateSelected: (step = 7.5) => operations.rotateSelected(step),
    onToggleSnapping: () => {
      deps.setSnappingEnabled(!deps.isSnappingEnabled());
      deps.sync();
      deps.saveSettingsState();
    },
    onToggleSelectionLock: () => {
      deps.toggleSelectionLock?.();
    },
    onToggleColorCode: () => {
      deps.cycleColorMode();
      deps.saveSettingsState();
    },
    onSetColorMode: (mode: ColorMode) => {
      deps.cycleColorMode(mode);
      deps.saveSettingsState();
    },
    onToggleTrackLabels: () => {
      deps.setTrackLabelsEnabled(!deps.isTrackLabelsEnabled());
      deps.sync();
      deps.saveSettingsState();
    },
    onTogglePowerSim: () => {
      deps.setPowerSimEnabled(!deps.isPowerSimEnabled());
      deps.sync();
      deps.saveSettingsState();
    },
    onToggleFeeder: () => operations.toggleSelectedFeeder(),
    onToggleIsolator: () => operations.toggleSelectedIsolator(),
    onToggleCatenary: () => operations.toggleSelectedCatenary(),
    onToggleCatenaryFeeder: () => operations.toggleSelectedCatenaryFeeder(),
    onToggleCatenaryIsolator: () => operations.toggleSelectedCatenaryIsolator(),
    onToggleTurnoutLantern: () => operations.toggleSelectedTurnoutLantern(),
    onToggleTurnoutDecoder: () => operations.toggleSelectedTurnoutDecoder(),
    onSetFeedVoltage: (v: number) => {
      layout.feedVoltage = v;
      if (!deps.isPowerSimEnabled()) deps.setPowerSimEnabled(true);
      deps.sync(true);
    },
    onSetLocoCurrent: (ma: number) => {
      layout.locoCurrentMa = Math.min(1000, Math.max(50, ma));
      if (!deps.isPowerSimEnabled()) deps.setPowerSimEnabled(true);
      deps.sync(true);
    },
    onOpenPowerSettings: (initialFocus) => {
      openPowerSettingsDialog({
        layout,
        catalog: deps.getCatalog(),
        initialFocus,
        onApply: ({
          feedVoltage,
          catenaryFeedVoltage,
          catenaryCircuitMode,
          locoCurrentMa,
          locoCount,
          turnoutLanternCurrentMa,
          turnoutDecoderCurrentMa,
          bulkTurnoutAction,
        }) => {
          layout.feedVoltage = feedVoltage;
          if (catenaryFeedVoltage !== undefined) layout.catenaryFeedVoltage = catenaryFeedVoltage;
          if (catenaryCircuitMode !== undefined) layout.catenaryCircuitMode = catenaryCircuitMode;
          layout.locoCurrentMa = locoCurrentMa;
          layout.locoCount = locoCount;
          if (turnoutLanternCurrentMa !== undefined) layout.turnoutLanternCurrentMa = turnoutLanternCurrentMa;
          if (turnoutDecoderCurrentMa !== undefined) layout.turnoutDecoderCurrentMa = turnoutDecoderCurrentMa;

          if (bulkTurnoutAction === "equipAll") {
            for (const p of layout.pieces) {
              const entry = deps.getCatalog().entries.find((e) => e.id === p.catalogId);
              if (isTurnoutEntry(entry)) {
                p.hasLantern = true;
                p.hasDecoder = true;
              }
            }
          } else if (bulkTurnoutAction === "clearAll") {
            for (const p of layout.pieces) {
              const entry = deps.getCatalog().entries.find((e) => e.id === p.catalogId);
              if (isTurnoutEntry(entry)) {
                delete p.hasLantern;
                delete p.hasDecoder;
              }
            }
          }
          if (!deps.isPowerSimEnabled()) deps.setPowerSimEnabled(true);
          deps.sync(true);
        },
      });
    },
    onToggleRulers: () => {
      ruler.toggle();
      deps.sync();
      deps.saveSettingsState();
    },
    onToggleMeasureTool: () => deps.toggleMeasureTool(),
    onToggleWoodPlate: () => {
      const active = !viewer3D.isWoodPlateEnabled();
      viewer3D.setWoodPlate(active);
      const isDe = getLanguage() === "de";
      deps.showToast(
        active
          ? (isDe ? "Woodplatte (Trassenbrett) platziert" : "Wood plate placed under tracks & roads")
          : (isDe ? "Woodplatte entfernt" : "Wood plate removed"),
        "🪵"
      );
      deps.sync();
    },
    onToggleBaseboard: () => {
      const baseboards = getBaseboards(layout);
      const anyEnabled = baseboards.some((b) => b.enabled);
      for (const b of baseboards) {
        b.enabled = !anyEnabled;
      }
      if (layout.baseboard) {
        layout.baseboard.enabled = !anyEnabled;
      }
      deps.sync(true);
    },
    onConfigureBaseboard: () => {
      deps.getSidebar()?.setActiveTab("board");
    },
    onToggle3D: () => deps.toggle3D(),
    onImportModel: () => {
      deps.getSidebar()?.setActiveTab("models");
    },
    onImportCatalog: () => {
      deps.triggerImportCatalog?.();
    },
    onToggleTheme: () => {
      themeManager.cycle();
    },
    onSetTheme: (theme) => {
      themeManager.setPreference(theme);
    },
    onOpenPlan: () => operations.triggerOpenPlan(),
    onSavePlan: () => operations.triggerSavePlan(),
    onSavePlanAs: (format?: PlanFormat) => operations.triggerSavePlanAs(format ?? "railbench"),
    onSaveXML: () => operations.triggerSavePlanAs("xml"),
    onExportCSV: () => operations.triggerExportCSV(),
    onPrintPlan: () => openPrintDialog(layout, deps.getCatalog(), renderer),
    onMoveSelectedToLayerPrompt: () => deps.openHeightDialogForSelection(),
    onOpenHelp: (tab?: string) => openHelpDialog((tab as HelpTab) || "overview"),
    onOpenWelcome: () => deps.triggerWelcomeDialog?.(),
    onResetView: () => {
      renderer.resetView();
      ruler.update();
      deps.sync();
    },
    onOpenCollab: () => deps.triggerCollabDialog(),
    onTogglePerfMonitor: () => deps.getPerfMonitor().toggle(),
    onResetUiSizes: () => deps.onResetUiSizes?.(),
    onToggleLockBars: () => deps.onToggleLockBars?.(),
    onResetPersonalConfig: () => {
      const t = getTranslations();
      if (confirm(t.dialogs.confirmResetConfig)) {
        resetPersonalConfig();
        deps.showToast(t.dialogs.configResetSuccess, "🧹");
        if (typeof window !== "undefined" && typeof window.location?.reload === "function") {
          setTimeout(() => {
            window.location.reload();
          }, 200);
        }
      }
    },
    onFocusShortCircuit: () => {
      const currentShortCircuit = deps.getCurrentShortCircuit();
      if (currentShortCircuit?.shortCircuitPoints.length) {
        const pt = currentShortCircuit.shortCircuitPoints[0]!;
        renderer.centerOn(pt.x, pt.y);
        ruler.update();
        if (deps.is3DMode()) {
          viewer3D.focusOnPoint(pt.x, pt.y);
        }
        if (currentShortCircuit.shortCircuitPieces.size > 0) {
          selection.setMany(Array.from(currentShortCircuit.shortCircuitPieces));
        }
        deps.sync();
      }
    },
  };
}
