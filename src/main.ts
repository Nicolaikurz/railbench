import "./styles/main.css";
import { Renderer } from "@/render/renderer";
import { attachCamera } from "@/render/camera";
import { createLayout, ensureDefaultLayers, getBaseboards, getBaseboardPolygon } from "@/core/layout";
import { updatePieceConnections } from "@/core/snapping";
import type { BaseboardSnapGuide } from "@/core/baseboard-snapping";
import type { BaseboardConfig, Layout, TrackPiece, Transform, Model3D, ColorMode } from "@/core/types";
import { Ruler } from "@/ui/ruler";
import { Viewer3D } from "@/render/viewer-3d";
import { catalogRegistry, computeWorldConnectors, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import {
  simulatePowerDrop,
  type SimulationSummary,
  isTurnoutEntry,
  detectShortCircuit,
  type ShortCircuitDetectionResult,
  getDefaultFeedVoltage,
  getDefaultLocoCurrentMa,
} from "@/core/simulation";
import { Selection } from "@/editor/selection";
import { History, AddPiecesCommand } from "@/editor/commands";
import { createToolbar, updateToolbarState, type ToolbarEvents } from "@/ui/toolbar";
import { buildToolbarEvents } from "@/ui/toolbar-events-factory";
import { getTranslations, onLanguageChange, getLanguage, setLanguage } from "@/i18n";
import { Sidebar } from "@/ui/sidebar";
import { RightSidebar } from "@/ui/right-sidebar";
import { PlanManager } from "@/io/plan-manager";
import { openPrintDialog } from "@/io/print";
import { openChangeHeightDialog } from "@/ui/height-dialog";
import { openRampGeneratorDialog } from "@/ui/ramp-generator-dialog";
import { openHeightPanel } from "@/ui/height-panel";
import { openHelpDialog, type HelpTab } from "@/ui/help-dialog";
import { openCatalogImportDialog } from "@/catalog/catalog-importer";
import { openCollabDialog } from "@/ui/collab-dialog";
import { CollabClient } from "@/collab/collab-client";
import { CollabOverlay } from "@/collab/collab-overlay";
import { PerfMonitor } from "@/ui/perf-monitor";
import { themeManager } from "@/theme/theme-manager";
import { loadAppSettings, saveAppSettings } from "@/settings/settings-manager";
import { SelectionBar } from "@/ui/selection-bar";
import { MeasureHud, getScaleRatio } from "@/ui/measure-hud";
import { MeasureController } from "@/editor/measure-controller";
import { showToast, showCollabToast } from "@/ui/toast";
import {
  openHeightPanelUnified,
  openHeightDialogForSelection,
  openRampGeneratorForPiece,
  type HeightPanelDeps,
} from "@/editor/height-panel-integration";
import {
  applyStartupAction,
  triggerDemoTracksDialog,
  triggerWelcomeDialog,
  type AppStartupContext,
} from "@/editor/app-startup";
import { TrackOperations } from "@/editor/track-operations";
import { CanvasController } from "@/editor/canvas-controller";
import { initDemoTracks, generateStressTracks, type DemoLayoutDef } from "@/editor/demo-tracks";
import { showWelcomeDialog, type StartupAction } from "@/ui/welcome-dialog";
import { showDemoTracksDialog } from "@/ui/demo-tracks-dialog";
import { getFullAppTitle } from "@/version";
import { getEffectiveFlexEntry } from "@/core/flextrack";
import { initLayoutResizers } from "@/ui/layout-resizer";

async function main() {
  document.title = `${getFullAppTitle()} – Model Railway Track Planner`;
  const savedSettings = loadAppSettings();

  let layout: Layout = createLayout("Model Railway Layout H0");
  let catalog: Catalog =
    (savedSettings.selectedCatalogId && catalogRegistry.get(savedSettings.selectedCatalogId)) ||
    catalogRegistry.getDefault();
  const selection = new Selection();
  let history = new History(layout);

  let snappingEnabled = savedSettings.snappingEnabled ?? true;
  let colorMode: ColorMode = savedSettings.colorMode ?? "types";
  let trackLabelsEnabled = savedSettings.trackLabelsEnabled ?? true;
  let powerSimEnabled = savedSettings.powerSimEnabled ?? false;
  let rightSidebarCollapsed = savedSettings.rightSidebarCollapsed ?? false;
  let selectionLockDisabled = savedSettings.selectionLockDisabled ?? false;
  let rotationStep = savedSettings.rotationStep ?? 30;
  let rotationSnapLocked = savedSettings.rotationSnapLocked ?? true;
  let sidebar: Sidebar | null = null, rightSidebar: RightSidebar | null = null;
  let ruler: Ruler | null = null, viewer3D: Viewer3D | null = null;
  let currentSimulation: SimulationSummary | undefined = undefined;
  let currentShortCircuit: ShortCircuitDetectionResult | undefined = undefined;
  let currentSelectionBox: { start: { x: number; y: number }; end: { x: number; y: number } } | null = null;

  // Baseboard & Measure state
  let isDraggingBaseboard = false;
  let activeBaseboard: BaseboardConfig | null = null;
  let activeBaseboardSnapGuides: BaseboardSnapGuide[] = [];
  let layoutResizer: any = null;

  if (rightSidebarCollapsed) {
    const app = document.getElementById("app");
    app?.classList.add("right-collapsed");
  }

  function toggleRightSidebar() {
    rightSidebarCollapsed = !rightSidebarCollapsed;
    const app = document.getElementById("app");
    if (app) {
      if (rightSidebarCollapsed) {
        app.classList.add("right-collapsed");
      } else {
        app.classList.remove("right-collapsed");
      }
    }
    updateToolbarState({ rightSidebarCollapsed });
    window.dispatchEvent(new Event("resize"));
    saveSettingsState();
  }

  function saveSettingsState() {
    saveAppSettings({
      snappingEnabled,
      selectionLockDisabled,
      rotationStep,
      rotationSnapLocked,
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
      rulerVisible: ruler?.isVisible() ?? true,
      rightSidebarCollapsed,
      barsLocked: layoutResizer?.isLocked(),
      selectedCatalogId: catalog?.id,
      activeSidebarTab: (sidebar?.getActiveTab() as any) ?? "catalog",
      camera: renderer?.world ? {
        x: Math.round(renderer.world.x * 10) / 10,
        y: Math.round(renderer.world.y * 10) / 10,
        zoom: Math.round(renderer.world.scale.x * 1000) / 1000,
      } : undefined,
    });
  }

  function setRotationStep(val: number) {
    rotationStep = val;
    saveSettingsState();
    const isDe = getLanguage() === "de";
    const angleStr = val === 5.7 ? "5.7°" : `${val}°`;
    showToast(isDe ? `Drehwinkel: ${angleStr}` : `Rotation angle: ${angleStr}`, "↻");
    sync();
  }

  function toggleRotationLock(forcedState?: boolean) {
    rotationSnapLocked = forcedState !== undefined ? forcedState : !rotationSnapLocked;
    saveSettingsState();
    const isDe = getLanguage() === "de";
    showToast(
      rotationSnapLocked
        ? (isDe ? "Drehwinkelsperre aktiviert (Einrasten)" : "Rotation angle lock enabled (Snapping)")
        : (isDe ? "Drehwinkelsperre aufgehoben (Freie Drehung)" : "Rotation angle lock disabled (Free rotation)"),
      rotationSnapLocked ? "🔒" : "🔓"
    );
    sync();
  }

  let saveSettingsTimeout: any = null;
  function debouncedSaveSettings() {
    if (saveSettingsTimeout) clearTimeout(saveSettingsTimeout);
    saveSettingsTimeout = setTimeout(saveSettingsState, 350);
  }
  const handleBeforeUnload = () => {
    saveSettingsState();
    planManager.flush(layout);
  };
  window.addEventListener("beforeunload", handleBeforeUnload);
  window.addEventListener("pagehide", handleBeforeUnload);
  function getActiveLayerId(): string {
    return layout.activeLayerId || layout.layers?.[0]?.id || "layer-1";
  }

  function getCatalogEntryForPiece(piece: TrackPiece): CatalogEntry | undefined {
    const raw = catalog.entries.find((e) => e.id === piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId);
    return raw ? getEffectiveFlexEntry(raw, piece) : undefined;
  }

  function toggleSelectionLock(forcedState?: boolean) {
    selectionLockDisabled = forcedState !== undefined ? forcedState : !selectionLockDisabled;
    const isDe = getLanguage() === "de";
    showToast(
      selectionLockDisabled
        ? (isDe ? "Auswahlsperre aufgehoben (alle Ebenen wählbar)" : "Selection lock disabled (all layers selectable)")
        : (isDe ? "Auswahlsperre aktiviert (nur aktive Ebene)" : "Selection lock enabled (active layer only)"),
      selectionLockDisabled ? "🔓" : "🔒",
    );
    saveSettingsState();
    sync();
  }

  function isPieceLockedOrHidden(piece: TrackPiece): boolean {
    if (selectionLockDisabled) return false;
    if (!layout.layers) return false;
    const layer = layout.layers.find((l) => l.id === (piece.layerId || "layer-1"));
    if (!layer) return false;
    return !layer.visible || layer.locked;
  }

  function isPieceInteractive(piece: TrackPiece): boolean {
    if (selectionLockDisabled) return true;
    const activeId = getActiveLayerId();
    const pieceLayerId = piece.layerId || "layer-1";
    const layerExists = layout.layers?.some((l) => l.id === pieceLayerId);
    if (!layerExists) return true;
    if (pieceLayerId !== activeId) return false;
    return !isPieceLockedOrHidden(piece);
  }

  function isModelInteractive(model: Model3D): boolean {
    if (selectionLockDisabled) return true;
    const activeId = getActiveLayerId();
    const modelLayerId = model.layerId || layout.layers?.[0]?.id || "layer-1";
    const layerExists = layout.layers?.some((l) => l.id === modelLayerId);
    if (!layerExists) return true;
    if (modelLayerId !== activeId) return false;
    if (!layout.layers) return true;
    const layer = layout.layers.find((l) => l.id === modelLayerId);
    return !layer || (layer.visible && !layer.locked);
  }

  const planManager = new PlanManager((state) => {
    updateToolbarState({
      canUndo: history.canUndo(),
      canRedo: history.canRedo(),
      hasSelection: selection.all().length > 0,
      snappingEnabled,
      selectionLockDisabled,
      rotationStep,
      rotationSnapLocked,
      colorCodeEnabled: colorMode !== "off",
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
      feedVoltage: layout.feedVoltage ?? getDefaultFeedVoltage(layout.scale),
      totalPieces: layout.pieces.length,
      activeFileName: state.fileName ?? layout.name,
      isDirty: state.isDirty,
    });
  });

  const canvasHost = document.getElementById("canvas-host")!;
  const renderer = new Renderer();
  await renderer.init(canvasHost);
  ruler = new Ruler({
    host: canvasHost,
    world: renderer.world,
    onCenterOrigin: () => {
      renderer.resetView();
      ruler?.update();
      sync();
      saveSettingsState();
    },
    visible: savedSettings.rulerVisible ?? true,
  });
  const collabOverlay = new CollabOverlay(canvasHost, renderer);
  const collabClient = new CollabClient();
  let isApplyingRemote = false;

  function applyRemoteLayout(remoteLayout: Layout, fromUserName: string) {
    isApplyingRemote = true;
    try {
      layout.pieces = remoteLayout.pieces;
      if (remoteLayout.layers) layout.layers = remoteLayout.layers;
      if (remoteLayout.baseboards) layout.baseboards = remoteLayout.baseboards;
      if (remoteLayout.models) layout.models = remoteLayout.models;
      if (remoteLayout.name) layout.name = remoteLayout.name;
      if (remoteLayout.scale) layout.scale = remoteLayout.scale;

      history = new History(layout);
      sync(false);
      showCollabToast(getTranslations().collabDialog.toastRemoteUpdate.replace("{name}", fromUserName));
    } finally {
      isApplyingRemote = false;
    }
  }

  collabClient.onStatusChange = (status) => {
    updateToolbarState({
      collabStatus: status,
      collabUsersCount: collabClient.getUsers().length,
    });
  };

  collabClient.onUsersChange = (users) => {
    collabOverlay.setUsers(users);
    updateToolbarState({
      collabStatus: collabClient.getStatus(),
      collabUsersCount: users.length,
    });
  };

  collabClient.onRemoteLayout = (remoteLayout, fromUser) => {
    applyRemoteLayout(remoteLayout, fromUser);
  };

  collabClient.onRemoteCursor = (userId, x, y) => {
    collabOverlay.updateCursor(userId, x, y);
  };

  collabClient.onChat = (msg) => {
    showCollabToast(`💬 ${msg.userName}: ${msg.text}`);
  };

  function triggerCollabDialog() {
    openCollabDialog({
      client: collabClient,
      layout,
      onConnect: (url, room, name, color, password) => {
        collabClient.connect(url, room, name, color, layout, password);
      },
      onDisconnect: () => {
        collabClient.disconnect();
        collabOverlay.clear();
      },
      onStartServer: window.electronAPI?.startCollabServer
        ? async (opts) => {
            return await window.electronAPI!.startCollabServer(opts);
          }
        : undefined,
      onStopServer: window.electronAPI?.stopCollabServer
        ? async () => {
            await window.electronAPI!.stopCollabServer();
          }
        : undefined,
      getServerStatus: window.electronAPI?.getCollabServerStatus
        ? async () => {
            return await window.electronAPI!.getCollabServerStatus();
          }
        : undefined,
    });
  }

  const perfMonitor = new PerfMonitor({
    host: canvasHost,
    renderer,
    getLayout: () => layout,
    onGenerateStressTracks: (count) => generateStressTracks(count, layout, catalog, history, sync),
  });

  attachCamera(renderer.app.canvas, renderer.world, () => {
    ruler.update();
    collabOverlay.update();
    debouncedSaveSettings();
  });

  if (savedSettings.camera && typeof savedSettings.camera.zoom === "number") {
    renderer.world.x = savedSettings.camera.x;
    renderer.world.y = savedSettings.camera.y;
    renderer.world.scale.set(savedSettings.camera.zoom);
    ruler.update();
  } else {
    renderer.resetView();
    ruler.update();
  }

  viewer3D = new Viewer3D(canvasHost);
  let is3DMode = false;

  function zoomView(factor: number) {
    if (is3DMode && viewer3D?.isSupported()) {
      viewer3D.zoomBy(factor);
      return;
    }
    const canvas = renderer.app.canvas;
    const rect = canvas.getBoundingClientRect();
    const mx = rect.width / 2;
    const my = rect.height / 2;
    const wx = (mx - renderer.world.x) / renderer.world.scale.x;
    const wy = (my - renderer.world.y) / renderer.world.scale.y;
    const newScale = Math.max(0.04, Math.min(25, renderer.world.scale.x * factor));
    renderer.world.scale.set(newScale);
    renderer.world.x = mx - wx * newScale;
    renderer.world.y = my - wy * newScale;
    ruler?.update();
    collabOverlay.update();
    debouncedSaveSettings();
  }

  function panView(dx: number, dy: number) {
    if (is3DMode && viewer3D?.isSupported()) {
      viewer3D.panBy(dx, dy);
      return;
    }
    renderer.world.x += dx;
    renderer.world.y += dy;
    ruler?.update();
    collabOverlay.update();
    debouncedSaveSettings();
  }

  function resetView() {
    renderer.resetView();
    ruler?.update();
    collabOverlay.update();
    debouncedSaveSettings();
  }

  function toggle3D(force?: boolean) {
    if (!viewer3D?.isSupported()) {
      const msg = getLanguage() === "de"
        ? "3D-Ansicht nicht verfügbar (WebGL-Kontext erforderlich)"
        : "3D view unavailable (WebGL context required)";
      showToast(msg, "⚠️");
      return;
    }
    is3DMode = force !== undefined ? force : !is3DMode;
    if (is3DMode) {
      ruler?.setVisible(false);
      renderer.pause();
      viewer3D.show(() => toggle3D(false));
      viewer3D.updateLayout(layout, catalog, { colorMode, showColorCode: colorMode !== "off" });
      viewer3D.resetCamera();
    } else {
      viewer3D.hide();
      renderer.resume();
      ruler?.setVisible(true);
    }
    sync();
  }

  // Initialisiere Editor Operations
  const operations = new TrackOperations({
    getLayout: () => layout,
    setLayout: (l: Layout) => {
      layout = l;
      renderer.invalidateCaches();
      viewer3D?.resetScene?.();
    },
    getCatalog: () => catalog,
    getSelection: () => selection,
    getHistory: () => history,
    setHistory: (h: History) => { history = h; },
    getRenderer: () => renderer,
    getPlanManager: () => planManager,
    isSnappingEnabled: () => snappingEnabled,
    isPieceInteractive,
    isModelInteractive,
    getCatalogEntry: getCatalogEntryForPiece,
    sync: (dirty?: boolean, transient?: boolean) => sync(dirty, transient),
    showToast,
  });

  function cycleColorMode(targetMode?: ColorMode) {
    if (targetMode) {
      colorMode = targetMode;
    } else {
      if (colorMode === "types") colorMode = "layers";
      else if (colorMode === "layers") colorMode = "off";
      else colorMode = "types";
    }
    sync();
  }

  let activePickHandler: ((wPos: { x: number; y: number }, piece: TrackPiece | null) => void) | null = null;
  let activePinDropHandler: ((pinId: string, wPos: { x: number; y: number }, hitPiece: TrackPiece | null) => void) | null = null;
  let activePointMarkers: Array<{ x: number; y: number; label: string; color: number; id?: string }> | null = null;
  let activeHoveredConnector: { pieceId: string; connectorIndex: number } | null = null;
  let canvasController: CanvasController | null = null;
  let previewRampElevations: Map<string, { start: number; end: number }> | null = null;
  let previewGeneratedPieces: TrackPiece[] | null = null;

  const heightPanelDeps: HeightPanelDeps = {
    getLayout: () => layout,
    getCatalog: () => catalog,
    getSelection: () => selection,
    getHistory: () => history,
    getOperations: () => operations,
    sync,
    setPreviewRampElevations: (map) => { previewRampElevations = map; },
    setPreviewGeneratedPieces: (pieces) => { previewGeneratedPieces = pieces; },
    setActivePointMarkers: (markers) => { activePointMarkers = markers; },
    setActivePickHandler: (handler) => { activePickHandler = handler; },
    setActivePinDropHandler: (handler) => { activePinDropHandler = handler; },
  };

  const measureController = new MeasureController({
    getLayout: () => layout,
    getSnappingEnabled: () => snappingEnabled,
    isPieceLockedOrHidden,
    getCanvas: () => renderer.app.canvas,
    sync: () => sync(),
  });

  // Floating Selection Action Bar (Eigenes Modul)
  const selectionBar = new SelectionBar(canvasHost, {
    onApplyHeight: (h) => operations.applyHeightInput(h),
    onApplyAngle: (deg) => operations.applyAngleInput(deg),
    onAdjustFlexTrack: (id, len, r, a) => operations.adjustFlexTrack(id, len, r, a),
    onResetFlexTrack: (id) => operations.resetFlexTrack(id),
    onSelectLayer: (layerId) => operations.moveSelectedToLayer(layerId, { start: 0, end: 0 }),
    onOpenHeightDialog: () => openHeightDialogForSelection(heightPanelDeps),
    onApplyAutoRamp: () => operations.applyAutoRamp(() => openHeightDialogForSelection(heightPanelDeps)),
    onOpenRampGenerator: () => {
      const sel = selection.all();
      if (sel.length === 1) {
        const piece = layout.pieces.find((p) => p.id === sel[0]);
        if (piece) openRampGeneratorForPiece(heightPanelDeps, piece);
      }
    },
    onToggleFeeder: () => operations.toggleSelectedFeeder(),
    onToggleIsolator: () => operations.toggleSelectedIsolator(),
    onToggleCatenary: () => operations.toggleSelectedCatenary(),
    onToggleCatenaryFeeder: () => operations.toggleSelectedCatenaryFeeder(),
    onToggleCatenaryIsolator: () => operations.toggleSelectedCatenaryIsolator(),
    onToggleTurnoutDecoder: () => operations.toggleSelectedTurnoutDecoder(),
    onToggleTurnoutLantern: () => operations.toggleSelectedTurnoutLantern(),
    onToggleSignal: () => operations.toggleSelectedSignal(),
    onToggleBridge: () => operations.toggleSelectedBridge(),
    onToggleTunnel: () => operations.toggleSelectedTunnel(),
    onRotate: (delta) => operations.rotateSelected(delta ?? rotationStep),
    getRotationStep: () => rotationStep,
    onDelete: () => operations.deleteSelected(),
    getSelectedPieceIds: () => selection.all(),
    getLayout: () => layout,
    getCatalogEntry: getCatalogEntryForPiece,
    getCatalog: () => catalog,
  });

  // Sync-Funktion für UI und Rendering
  function sync(dirty = false, transient = false) {
    if (dirty) {
      updatePieceConnections(layout.pieces, layout.layers);
      planManager.markDirty(true);
      if (!transient) {
        planManager.saveToLocalStorage(layout);
        if (!isApplyingRemote) {
          collabClient.broadcastLayout(layout);
        }
      }
    }

    if (powerSimEnabled) {
      currentSimulation = simulatePowerDrop(layout, catalog);
    } else {
      currentSimulation = undefined;
    }

    if (dirty || !currentShortCircuit) {
      currentShortCircuit = detectShortCircuit(layout, catalog);
    }

    const selectedIds = new Set(selection.all());
    const selectedPiece = layout.pieces.find((p) => selectedIds.has(p.id));
    const selectedModel = layout.models?.find((m) => selectedIds.has(m.id));

    if (!is3DMode) {
      renderer.render(layout, catalog, {
        selectedIds,
        showConnectors: snappingEnabled,
        powerSimulation: currentSimulation,
        shortCircuit: currentShortCircuit,
        showPowerDrop: powerSimEnabled,
        showTrackLabels: trackLabelsEnabled,
        showColorCode: colorMode !== "off",
        colorMode,
        selectionBox: currentSelectionBox,
        activeBaseboardId: activeBaseboard?.id ?? sidebar?.getActiveBoardId(),
        isDraggingBaseboard,
        baseboardSnapGuides: activeBaseboardSnapGuides,
        pointMarkers: activePointMarkers || undefined,
        previewElevations: previewRampElevations || undefined,
        previewPieces: previewGeneratedPieces || undefined,
        hoveredOpenConnector: activeHoveredConnector,
        measureTool: measureController.isActive
          ? {
              start: measureController.start,
              current: measureController.current,
              locked: measureController.locked,
              snapPoint: measureController.snapPoint,
              scaleRatio: getScaleRatio(layout.scale),
              scaleName: layout.scale,
            }
          : null,
      });
      ruler?.update();
    } else {
      viewer3D?.updateLayout(layout, catalog, {
        colorMode,
        showColorCode: colorMode !== "off",
        shortCircuit: currentShortCircuit,
      });
    }
    collabOverlay.update();

    // Während flüchtiger Aktionen (aktives Mausziehen/Drehen) überspringen wir
    // das teure Neuaufbauen des HTML-DOM von Seitenleisten und Toolbar
    if (transient) {
      selectionBar.updateLiveValues(selectedPiece, selectedModel);
      return;
    }

    collabClient.broadcastSelection(selection.all());
    selectionBar.update();

    const planState = planManager.getState();
    updateToolbarState({
      canUndo: history.canUndo(),
      canRedo: history.canRedo(),
      hasSelection: selectedIds.size > 0,
      snappingEnabled,
      selectionLockDisabled,
      rotationStep,
      rotationSnapLocked,
      measureToolActive: measureController.isActive,
      colorCodeEnabled: colorMode !== "off",
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
      feedVoltage: layout.feedVoltage ?? getDefaultFeedVoltage(layout.scale),
      hasShortCircuit: currentShortCircuit.hasShortCircuit,
      shortCircuitCount: currentShortCircuit.shortCircuitPoints.length,
      rulersEnabled: ruler?.isVisible() ?? true,
      baseboardEnabled: getBaseboards(layout).some((b) => b.enabled),
      is3DActive: is3DMode,
      selectedHasFeeder: selectedPiece?.powerFeed ?? false,
      selectedHasIsolator: selectedPiece?.isIsolator ?? false,
      totalPieces: layout.pieces.length,
      activeFileName: planState.fileName ?? layout.name,
      isDirty: planState.isDirty,
      themePref: themeManager.getPreference(),
      resolvedTheme: themeManager.getResolvedTheme(),
      rightSidebarCollapsed,
      barsLocked: layoutResizer?.isLocked(),
      collabStatus: collabClient.getStatus(),
      collabUsersCount: collabClient.getUsers().length,
    });

    sidebar?.update(layout, currentSimulation);
    if (rightSidebar) {
      rightSidebar.update(layout);
    }
  }

  function triggerImportCatalog() {
    openCatalogImportDialog({
      onSuccess: (newCat) => {
        catalog = newCat;
        sidebar?.setCatalog(catalog);
        if (layout.pieces.length === 0 && (newCat.scale === "H0" || newCat.scale === "N" || newCat.scale === "TT" || newCat.scale === "Z" || newCat.scale === "1" || newCat.scale === "G" || newCat.scale === "0")) {
          layout.scale = newCat.scale;
        }
        sync(false);
        saveSettingsState();
      },
      showToast,
    });
  }

  // Sidebar initialisieren
  const sidebarHost = document.getElementById("sidebar")!;
  sidebar = new Sidebar(sidebarHost, catalog, layout, {
    onAddPiece: (entry) => { operations.addPieceToLayout(entry); sync(true); },
    onAddModel: (model) => operations.addModelToLayout(model),
    onExportCSV: () => operations.triggerExportCSV(),
    onImportCatalog: () => triggerImportCatalog(),
    onSelectSystem: (systemId) => {
      const newCat = catalogRegistry.get(systemId);
      if (newCat) {
        catalog = newCat;
        sidebar?.setCatalog(catalog);
        if (layout.pieces.length === 0 && (newCat.scale === "H0" || newCat.scale === "N" || newCat.scale === "TT" || newCat.scale === "Z" || newCat.scale === "1" || newCat.scale === "G" || newCat.scale === "0")) {
          layout.scale = newCat.scale;
        }
        sync(false);
        saveSettingsState();
      }
    },
    onTabChange: () => {
      saveSettingsState();
    },
    onLayerChange: () => {
      const activeId = getActiveLayerId();
      const toKeep = selection.all().filter((id) => {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) return (p.layerId || "layer-1") === activeId && !isPieceLockedOrHidden(p);
        const m = layout.models?.find((x) => x.id === id);
        if (m) return (m.layerId || "layer-1") === activeId;
        return false;
      });
      selection.setMany(toKeep);
      sync(true);
    },
    onBaseboardChange: () => {
      sync(true);
    },
    onMoveSelectedToLayer: (layerId) => {
      operations.moveSelectedToLayer(layerId, { start: 0, end: 0 });
    },
    onToggleSelectionLock: () => toggleSelectionLock(),
    isSelectionLockDisabled: () => selectionLockDisabled,
    onToggleWoodPlate: () => toggleWoodPlate(),
    isWoodPlateActive: () => viewer3D?.isWoodPlateEnabled() ?? false,
    getZoom: () => renderer.world.scale.x || 1.0,
    selectedPieceIds: () => selection.all(),
    onSelectModel: (modelId) => {
      selection.set(modelId);
      sync();
    },
  });

  if (savedSettings.activeSidebarTab) {
    sidebar.setActiveTab(savedSettings.activeSidebarTab as any);
  }

  function toggleWoodPlate() {
    const active = !viewer3D?.isWoodPlateEnabled();
    viewer3D?.setWoodPlate(active);
    const isDe = getLanguage() === "de";
    showToast(active ? (isDe ? "Woodplatte (Trassenbrett) platziert" : "Wood plate placed under tracks & roads") : (isDe ? "Woodplatte entfernt" : "Wood plate removed"), "🪵");
    sync();
  }

  // RightSidebar initialisieren
  const rightSidebarHost = document.getElementById("right-sidebar")!;
  rightSidebar = new RightSidebar(rightSidebarHost, layout, {
    onLayerChange: () => {
      const activeId = getActiveLayerId();
      const toKeep = selection.all().filter((id) => {
        if (selectionLockDisabled) return true;
        const p = layout.pieces.find((x) => x.id === id);
        if (p) return (p.layerId || "layer-1") === activeId && !isPieceLockedOrHidden(p);
        const m = layout.models?.find((x) => x.id === id);
        if (m) return (m.layerId || "layer-1") === activeId;
        return false;
      });
      selection.setMany(toKeep);
      sync(true);
    },
    onMoveSelectedToLayer: (layerId, elevation) => {
      operations.moveSelectedToLayer(layerId, elevation);
    },
    onOpenHeightDialog: (preselectedLayerId) => {
      openHeightDialogForSelection(heightPanelDeps, preselectedLayerId);
    },
    onSelectActiveLayer: (layerId) => {
      layout.activeLayerId = layerId;
      const toKeep = selection.all().filter((id) => {
        if (selectionLockDisabled) return true;
        const p = layout.pieces.find((x) => x.id === id);
        if (p) return (p.layerId || "layer-1") === layerId;
        const m = layout.models?.find((x) => x.id === id);
        if (m) return (m.layerId || "layer-1") === layerId;
        return false;
      });
      selection.setMany(toKeep);
      sync(true);
    },
    onToggleWoodPlate: () => toggleWoodPlate(),
    isWoodPlateActive: () => viewer3D?.isWoodPlateEnabled() ?? false,
    onToggleSelectionLock: () => toggleSelectionLock(),
    isSelectionLockDisabled: () => selectionLockDisabled,
    onClose: () => {
      toggleRightSidebar();
    },
    selectedPieceIds: () => selection.all(),
  });

  layoutResizer = initLayoutResizers();
  if (savedSettings.barsLocked !== undefined) {
    layoutResizer.setLocked(savedSettings.barsLocked);
  }

  // Toolbar initialisieren
  const toolbarHost = document.getElementById("toolbar")!;
  const toolbarEvents: ToolbarEvents = buildToolbarEvents({
    getLayout: () => layout, getCatalog: () => catalog, getSelection: () => selection,
    getHistory: () => history, getOperations: () => operations, getPlanManager: () => planManager,
    getRenderer: () => renderer, getRuler: () => ruler, getViewer3D: () => viewer3D,
    getSidebar: () => sidebar, getPerfMonitor: () => perfMonitor, getCurrentShortCircuit: () => currentShortCircuit,
    is3DMode: () => is3DMode, toggle3D, toggleMeasureTool: () => measureController.toggle(), toggleRightSidebar, saveSettingsState, sync,
    onResetUiSizes: () => layoutResizer.resetAll(),
    onToggleLockBars: () => {
      const locked = layoutResizer.toggleLocked();
      saveSettingsState();
      sync();
      const isDe = getLanguage() === "de";
      showToast(locked ? (isDe ? "Leistengrößen fixiert" : "Panels locked") : (isDe ? "Leistengrößen freigegeben" : "Panels unlocked"), locked ? "🔒" : "🔓");
    },
    isSnappingEnabled: () => snappingEnabled,
    setSnappingEnabled: (val) => { snappingEnabled = val; },
    getRotationStep: () => rotationStep,
    setRotationStep: (val: number) => setRotationStep(val),
    isRotationSnapLocked: () => rotationSnapLocked,
    toggleRotationLock: () => toggleRotationLock(),
    toggleSelectionLock: () => toggleSelectionLock(),
    cycleColorMode,
    isTrackLabelsEnabled: () => trackLabelsEnabled,
    setTrackLabelsEnabled: (val) => { trackLabelsEnabled = val; },
    isPowerSimEnabled: () => powerSimEnabled,
    setPowerSimEnabled: (val) => { powerSimEnabled = val; },
    openHeightDialogForSelection: () => openHeightDialogForSelection(heightPanelDeps),
    triggerCollabDialog,
    triggerWelcomeDialog: () => triggerWelcomeDialog(startupCtx),
    triggerDemoTracksDialog: () => triggerDemoTracksDialog(startupCtx),
    triggerImportCatalog,
    showToast,
  });

  createToolbar(toolbarHost, toolbarEvents);
  sidebar?.render();

  themeManager.onChange((resolved, pref) => {
    renderer.setTheme(resolved);
    ruler?.setTheme(resolved);
    viewer3D?.setTheme(resolved);
    updateToolbarState({ themePref: pref, resolvedTheme: resolved });
  });

  // Auto-Connect bei URL-Parametern (?room=... oder ?collab=1)
  const urlParams = new URLSearchParams(window.location.search);
  if (urlParams.has("room") || urlParams.get("collab") === "1") {
    const room = urlParams.get("room") || "default";
    const userName = urlParams.get("user") || undefined;
    const password = urlParams.get("pass") || urlParams.get("password") || undefined;
    const wsProto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsHost = window.location.host || "localhost:3000";
    const wsUrl = `${wsProto}//${wsHost}`;
    collabClient.connect(wsUrl, room, userName, undefined, layout, password);
  }

  // Auto-Start Check & Auto-Connect in Electron bei gesetzten Startvariablen
  if (window.electronAPI?.getServerLaunchConfig) {
    window.electronAPI.getServerLaunchConfig().then((config) => {
      if (config?.shouldAutoStart) {
        const isDe = getLanguage() === "de";
        showToast(
          isDe
            ? `Multi-User Server aktiv (Port ${config.port}, Raum: ${config.defaultRoom})`
            : `Multi-User Server active (Port ${config.port}, Room: ${config.defaultRoom})`,
          "🚆"
        );
        if (config.autoConnect && collabClient.getStatus() !== "connected") {
          collabClient.connect(
            `ws://localhost:${config.port}`,
            config.defaultRoom,
            "Host",
            undefined,
            layout,
            config.password
          );
        }
      }
    }).catch(() => {});
  }

  // Canvas Interaktion & Shortcuts initialisieren
  canvasController = new CanvasController({
    canvas: renderer.app.canvas,
    renderer,
    getLayout: () => layout,
    getCatalog: () => catalog,
    getSelection: () => selection,
    getHistory: () => history,
    getOperations: () => operations,
    getCollabClient: () => collabClient,
    getSidebar: () => sidebar,
    getPerfMonitor: () => perfMonitor,
    getRuler: () => ruler,
    getPlanManager: () => planManager,
    is3DMode: () => is3DMode,
    toggle3D,
    zoomView,
    panView,
    resetView,
    isMeasureToolActive: () => measureController.isActive,
    toggleMeasureTool: (force) => measureController.toggle(force),
    findMeasureSnapPoint: (pos, snapDist, forceNoSnap) => measureController.findSnapPoint(pos, snapDist, forceNoSnap),
    updateMeasureHud: (updateInputs) => measureController.updateHud(updateInputs),
    getMeasureState: () => ({
      start: measureController.start,
      current: measureController.current,
      locked: measureController.locked,
      snapPoint: measureController.snapPoint,
      snapEnabled: measureController.snapEnabled,
      draggingPoint: measureController.draggingPoint,
    }),
    setMeasureState: (upd) => {
      if (upd.start !== undefined) measureController.start = upd.start;
      if (upd.current !== undefined) measureController.current = upd.current;
      if (upd.locked !== undefined) measureController.locked = upd.locked;
      if (upd.snapPoint !== undefined) measureController.snapPoint = upd.snapPoint;
      if (upd.draggingPoint !== undefined) measureController.draggingPoint = upd.draggingPoint;
    },
    isSnappingEnabled: () => snappingEnabled,
    setSnappingEnabled: (val) => { snappingEnabled = val; },
    getRotationStep: () => rotationStep,
    isRotationSnapLocked: () => rotationSnapLocked,
    toggleSelectionLock: () => toggleSelectionLock(),
    getColorMode: () => colorMode,
    cycleColorMode,
    isTrackLabelsEnabled: () => trackLabelsEnabled,
    setTrackLabelsEnabled: (val) => { trackLabelsEnabled = val; },
    isPowerSimEnabled: () => powerSimEnabled,
    setPowerSimEnabled: (val) => { powerSimEnabled = val; },
    toggleRightSidebar,
    openHeightDialogForSelection: () => openHeightDialogForSelection(heightPanelDeps),
    openHelpDialog,
    openRampGeneratorForPiece: (anchorPiece) => openRampGeneratorForPiece(heightPanelDeps, anchorPiece),
    openPrintDialog: () => openPrintDialog(layout, catalog, renderer),
    isPieceInteractive, isModelInteractive, getCatalogEntryForPiece, sync, saveSettingsState, showToast,
    getCurrentSimulation: () => currentSimulation,
    setSelectionBox: (box) => { currentSelectionBox = box; },
    setDraggingBaseboardState: (isDragging, activeBoard, guides) => {
      isDraggingBaseboard = isDragging; activeBaseboard = activeBoard; activeBaseboardSnapGuides = guides;
    },
    getPickHandler: () => activePickHandler,
    getPointMarkers: () => activePointMarkers,
    getPinDropHandler: () => activePinDropHandler,
    getHoveredOpenConnector: () => activeHoveredConnector,
    setHoveredOpenConnector: (val) => { activeHoveredConnector = val; },
  });

  // Native Electron Menü-Aktionen empfangen
  if (window.electronAPI) {
    window.electronAPI.onMenuAction((action) => {
      switch (action) {
        case "newPlan": operations.triggerNewPlan(); break;
        case "openPlan": operations.triggerOpenPlan(); break;
        case "demoTracks": triggerDemoTracksDialog(startupCtx); break;
        case "savePlan": operations.triggerSavePlan(); break;
        case "savePlanAs": operations.triggerSavePlanAs("railbench"); break;
        case "importCatalog": triggerImportCatalog(); break;
        case "import3d": toolbarEvents.onImportModel?.(); break;
        case "exportXml": toolbarEvents.onSaveXML(); break;
        case "exportCSV": operations.triggerExportCSV(); break;
        case "printPlan": openPrintDialog(layout, catalog, renderer); break;
        case "collaborate": triggerCollabDialog(); break;
        case "copy": operations.copySelected(); break;
        case "paste": operations.pasteClipboard(); break;
        case "duplicate": operations.duplicateSelected(); break;
        case "selectAll": operations.selectAll(); break;
        case "deselectAll": selection.clear(); sync(); break;
        case "undo": history.undo(); sync(true); break;
        case "redo": history.redo(); sync(true); break;
        case "rotate": operations.rotateSelected(7.5); break;
        case "rotateCcw": operations.rotateSelected(-7.5); break;
        case "delete": operations.deleteSelected(); break;
        case "toggleFeeder": operations.toggleSelectedFeeder(); break;
        case "toggleSignal": operations.toggleSelectedSignal(); break;
        case "toggleBridge": operations.toggleSelectedBridge(); break;
        case "resetView": renderer.resetView(); sync(); break;
        case "toggleSnapping": snappingEnabled = !snappingEnabled; sync(); break;
        case "toggleColorCode": cycleColorMode(); break;
        case "toggleTrackLabels": trackLabelsEnabled = !trackLabelsEnabled; sync(); break;
        case "togglePowerSim": powerSimEnabled = !powerSimEnabled; sync(); break;
        case "toggle3D": toggle3D(); break;
        case "toggleRulers": ruler?.toggle(); sync(); break;
        case "toggleWoodPlate": toolbarEvents.onToggleWoodPlate?.(); break;
        case "togglePerfMonitor": perfMonitor?.toggle(); sync(); break;
        case "lockBars": toolbarEvents.onToggleLockBars?.(); break;
        case "resetUiSizes": toolbarEvents.onResetUiSizes?.(); break;
        case "themeSystem": themeManager.setPreference("system"); break;
        case "themeDark": themeManager.setPreference("dark"); break;
        case "themeLight": themeManager.setPreference("light"); break;
        case "configPower": toolbarEvents.onOpenPowerSettings?.("voltage"); break;
        case "langAuto": toolbarEvents.onAutoSelectLanguage?.(); break;
        case "langDe": setLanguage("de"); break;
        case "langEn": setLanguage("en"); break;
        case "resetPersonalConfig": toolbarEvents.onResetPersonalConfig?.(); break;
        case "changeHeight": openHeightDialogForSelection(heightPanelDeps); break;
        case "help": openHelpDialog("overview"); break;
        case "shortcuts": openHelpDialog("shortcuts"); break;
        case "welcome": triggerWelcomeDialog(startupCtx); break;
        case "layersGuide": openHelpDialog("layers"); break;
        case "voltageGuide": openHelpDialog("power"); break;
        case "printGuide": openHelpDialog("print"); break;
        case "about": openHelpDialog("about"); break;
      }
    });
  }

  // Sprachänderungen dynamisch übernehmen
  window.electronAPI?.setMenuLanguage?.(getLanguage());
  onLanguageChange((lang) => {
    window.electronAPI?.setMenuLanguage?.(lang);
    createToolbar(toolbarHost, toolbarEvents);
  sidebar?.render();
    updateToolbarState({
      themePref: themeManager.getPreference(),
      resolvedTheme: themeManager.getResolvedTheme(),
    });
    selectionBar.updateStaticTexts();
    measureController.updateHud();
    sync(false);
  });

  const startupCtx: AppStartupContext = {
    getLayout: () => layout,
    setLayout: (l) => {
      layout = l;
      renderer.invalidateCaches();
      viewer3D?.resetScene?.();
    },
    getCatalog: () => catalog,
    setCatalog: (c) => { catalog = c; },
    getSelection: () => selection,
    getHistory: () => history,
    setHistory: (h) => { history = h; },
    planManager,
    saveSettingsState,
    sync,
    onCatalogChange: (c) => sidebar?.setCatalog(c),
    triggerOpenPlan: () => operations.triggerOpenPlan(),
  };

  // Startverhalten ermitteln (Willkommensdialog vs. gemerkte Auswahl)
  const cachedPlan = planManager.loadFromLocalStorage();
  const hasRecent = Boolean(cachedPlan && cachedPlan.pieces?.length);
  const recentName = cachedPlan?.name ? `${cachedPlan.name}.railbench` : undefined;

  let startupAction: StartupAction = savedSettings.startupAction || "ask";
  if (startupAction === "recent" && !hasRecent) {
    startupAction = "ask";
  }

  if (startupAction === "ask") {
    // Vorab-Initialisierung für ansprechenden Hintergrund hinter dem Weichzeichner
    if (hasRecent && cachedPlan) {
      layout = cachedPlan;
      ensureDefaultLayers(layout);
      history = new History(layout);
      planManager.setPlanInfo(layout.name + ".railbench", null, "railbench");
    } else {
      initDemoTracks(layout, catalog);
      planManager.setPlanInfo(`${layout.name}.railbench`, null, "railbench");
    }
    sync(false);

    const choice = await showWelcomeDialog(hasRecent, recentName);
    if (choice.dontAskAgain) {
      saveAppSettings({ startupAction: choice.action });
    }
    applyStartupAction(startupCtx, choice.action, cachedPlan);
  } else {
    applyStartupAction(startupCtx, startupAction as "recent" | "new" | "demo" | "open", cachedPlan);
  }
}

main();
