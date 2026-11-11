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
} from "@/core/simulation";
import { Selection } from "@/editor/selection";
import { History, AddPiecesCommand } from "@/editor/commands";
import { createToolbar, updateToolbarState, type ToolbarEvents } from "@/ui/toolbar";
import { buildToolbarEvents } from "@/ui/toolbar-events-factory";
import { getTranslations, onLanguageChange, getLanguage } from "@/i18n";
import { Sidebar } from "@/ui/sidebar";
import { RightSidebar } from "@/ui/right-sidebar";
import { PlanManager } from "@/io/plan-manager";
import { openPrintDialog } from "@/io/print";
import { openChangeHeightDialog } from "@/ui/height-dialog";
import { openRampGeneratorDialog } from "@/ui/ramp-generator-dialog";
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
  let sidebar: Sidebar | null = null, rightSidebar: RightSidebar | null = null;
  let ruler: Ruler | null = null, viewer3D: Viewer3D | null = null;
  let currentSimulation: SimulationSummary | undefined = undefined;
  let currentShortCircuit: ShortCircuitDetectionResult | undefined = undefined;
  let currentSelectionBox: { start: { x: number; y: number }; end: { x: number; y: number } } | null = null;

  // Baseboard & Measure state
  let isDraggingBaseboard = false;
  let activeBaseboard: BaseboardConfig | null = null;
  let activeBaseboardSnapGuides: BaseboardSnapGuide[] = [];

  let isMeasureToolActive = false;
  let measureStart: { x: number; y: number } | null = null;
  let measureCurrent: { x: number; y: number } | null = null;
  let measureLocked = false;
  let measureSnapPoint: { x: number; y: number } | null = null;
  let measureSnapEnabled = true;
  let measureDraggingPoint: "A" | "B" | null = null;
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

  let saveSettingsTimeout: any = null;
  function debouncedSaveSettings() {
    if (saveSettingsTimeout) clearTimeout(saveSettingsTimeout);
    saveSettingsTimeout = setTimeout(saveSettingsState, 350);
  }
  const handleBeforeUnload = () => {
    saveSettingsState();
    planManager.saveToLocalStorage(layout);
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
      colorCodeEnabled: colorMode !== "off",
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
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

  function showToast(msg: string, icon = "ℹ️") {
    const existing = document.getElementById("app-global-toast");
    if (existing) existing.remove();
    const toast = document.createElement("div");
    toast.id = "app-global-toast";
    toast.className = "app-global-toast";
    toast.style.position = "fixed";
    toast.style.bottom = "64px";
    toast.style.left = "50%";
    toast.style.transform = "translateX(-50%)";
    toast.style.background = "var(--bg-toolbar, #1e293b)";
    toast.style.color = "var(--toolbar-text, #f8fafc)";
    toast.style.border = "1px solid var(--border-main, #334155)";
    toast.style.borderRadius = "20px";
    toast.style.padding = "7px 18px";
    toast.style.fontSize = "13px";
    toast.style.fontWeight = "600";
    toast.style.boxShadow = "0 8px 24px rgba(0,0,0,0.35)";
    toast.style.zIndex = "999";
    toast.style.pointerEvents = "none";
    toast.style.display = "flex";
    toast.style.alignItems = "center";
    toast.style.gap = "8px";
    toast.style.transition = "opacity 0.2s ease, transform 0.2s ease";
    toast.innerHTML = `<span>${icon}</span> <span>${msg}</span>`;

    document.body.appendChild(toast);
    setTimeout(() => {
      toast.style.opacity = "0";
      toast.style.transform = "translateX(-50%) translateY(6px)";
      setTimeout(() => toast.remove(), 250);
    }, 2200);
  }

  function showCollabToast(msg: string) {
    showToast(msg, "👥");
  }

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
      viewer3D.show(() => toggle3D(false));
      viewer3D.updateLayout(layout, catalog, { colorMode, showColorCode: colorMode !== "off" });
      viewer3D.resetCamera();
    } else {
      viewer3D.hide();
      ruler?.setVisible(true);
    }
    sync();
  }

  // Initialisiere Editor Operations
  const operations = new TrackOperations({
    getLayout: () => layout,
    setLayout: (l: Layout) => { layout = l; },
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
    sync: (dirty?: boolean) => sync(dirty),
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

  function openHeightDialogForSelection(preselectedLayerId?: string) {
    const selected = selection.all();
    if (selected.length === 0) {
      alert(getTranslations().dialogs.selectTracksFirst);
      return;
    }
    openChangeHeightDialog({
      layout,
      catalog,
      selectedPieceIds: selected,
      preselectedLayerId,
      onApply: (targetLayerId, elevation) => {
        operations.moveSelectedToLayer(targetLayerId, elevation);
      },
    });
  }

  function openRampGeneratorForPiece(anchorPiece: TrackPiece) {
    openRampGeneratorDialog({
      layout,
      catalog,
      anchorPiece,
      onApply: (generatedPieces, newLayer) => {
        if (newLayer) {
          if (!layout.layers) layout.layers = [];
          if (!layout.layers.some((l) => l.id === newLayer.id)) {
            layout.layers.push(newLayer);
          }
          layout.activeLayerId = newLayer.id;
        }
        history.run(new AddPiecesCommand(generatedPieces));
        selection.setMany(generatedPieces.map((p) => p.id));
        sync(true);
        const isDe = getLanguage() === "de";
        showToast(
          isDe ? `${generatedPieces.length} Rampengleise erstellt` : `Created ${generatedPieces.length} ramp tracks`,
          "📐",
        );
      },
    });
  }

  function findMeasureSnapPoint(pos: { x: number; y: number }, snapDist = 20, forceNoSnap = false): { x: number; y: number } | null {
    if (!snappingEnabled || !measureSnapEnabled || forceNoSnap) return null;
    let closestPt: { x: number; y: number } | null = null;
    let closestD = snapDist;

    for (const piece of layout.pieces) {
      if (isPieceLockedOrHidden(piece)) continue;
      for (const conn of piece.connectors) {
        const d = Math.hypot(pos.x - conn.x, pos.y - conn.y);
        if (d < closestD) {
          closestD = d;
          closestPt = { x: conn.x, y: conn.y };
        }
      }
    }

    const baseboards = getBaseboards(layout);
    for (const bb of baseboards) {
      if (!bb.enabled) continue;
      const poly = getBaseboardPolygon(bb);
      for (const pt of poly) {
        const d = Math.hypot(pos.x - pt.x, pos.y - pt.y);
        if (d < closestD) {
          closestD = d;
          closestPt = { x: pt.x, y: pt.y };
        }
      }
    }

    return closestPt;
  }

  function updateMeasureHud(updateInputs = true) {
    measureHud.update({
      isActive: isMeasureToolActive,
      start: measureStart,
      current: measureCurrent,
      locked: measureLocked,
      snapEnabled: measureSnapEnabled,
      scale: layout.scale,
    }, updateInputs);
  }

  function toggleMeasureTool(force?: boolean) {
    isMeasureToolActive = force !== undefined ? force : !isMeasureToolActive;
    if (!isMeasureToolActive) {
      measureStart = null;
      measureCurrent = null;
      measureLocked = false;
      measureDraggingPoint = null;
      measureSnapPoint = null;
      renderer.app.canvas.style.cursor = "default";
    } else {
      renderer.app.canvas.style.cursor = "crosshair";
    }
    updateMeasureHud();
    sync();
  }

  // Floating Measure Tool HUD
  const measureHud = new MeasureHud({
    onToggleSnap: (enabled) => {
      measureSnapEnabled = enabled;
      updateMeasureHud();
    },
    onReset: () => {
      measureStart = null;
      measureCurrent = null;
      measureLocked = false;
      measureDraggingPoint = null;
      updateMeasureHud();
      sync();
    },
    onClose: () => {
      toggleMeasureTool(false);
    },
    onManualCoordsChange: ({ ax, ay, bx, by }) => {
      if (ax !== undefined && ay !== undefined) {
        measureStart = { x: ax, y: ay };
      }
      if (bx !== undefined && by !== undefined) {
        measureCurrent = { x: bx, y: by };
        measureLocked = true;
      }
      updateMeasureHud(false);
      sync();
    },
  });

  // Floating Selection Action Bar (Eigenes Modul)
  const selectionBar = new SelectionBar(canvasHost, {
    onApplyHeight: (h) => operations.applyHeightInput(h),
    onApplyAngle: (deg) => operations.applyAngleInput(deg),
    onAdjustFlexTrack: (id, len, r, a) => operations.adjustFlexTrack(id, len, r, a),
    onSelectLayer: (layerId) => operations.moveSelectedToLayer(layerId, { start: 0, end: 0 }),
    onOpenHeightDialog: () => openHeightDialogForSelection(),
    onApplyAutoRamp: () => operations.applyAutoRamp(() => openHeightDialogForSelection()),
    onOpenRampGenerator: () => {
      const sel = selection.all();
      if (sel.length === 1) {
        const piece = layout.pieces.find((p) => p.id === sel[0]);
        if (piece) openRampGeneratorForPiece(piece);
      }
    },
    onToggleFeeder: () => operations.toggleSelectedFeeder(),
    onToggleIsolator: () => operations.toggleSelectedIsolator(),
    onToggleCatenary: () => operations.toggleSelectedCatenary(),
    onToggleCatenaryFeeder: () => operations.toggleSelectedCatenaryFeeder(),
    onToggleCatenaryIsolator: () => operations.toggleSelectedCatenaryIsolator(),
    onToggleTurnoutDecoder: () => operations.toggleSelectedTurnoutDecoder(),
    onToggleTurnoutLantern: () => operations.toggleSelectedTurnoutLantern(),
    onRotate: (delta) => operations.rotateSelected(delta ?? 7.5),
    onDelete: () => operations.deleteSelected(),
    getSelectedPieceIds: () => selection.all(),
    getLayout: () => layout,
    getCatalogEntry: getCatalogEntryForPiece,
    getCatalog: () => catalog,
  });

  // Sync-Funktion für UI und Rendering
  function sync(dirty = false) {
    if (dirty) {
      updatePieceConnections(layout.pieces, layout.layers);
      planManager.markDirty(true);
      planManager.saveToLocalStorage(layout);
      if (!isApplyingRemote) {
        collabClient.broadcastLayout(layout);
      }
    }

    if (powerSimEnabled) {
      if (dirty || !currentSimulation) {
        currentSimulation = simulatePowerDrop(layout, catalog);
      }
    } else {
      currentSimulation = undefined;
    }

    if (dirty || !currentShortCircuit) {
      currentShortCircuit = detectShortCircuit(layout, catalog);
    }

    const selectedIds = new Set(selection.all());
    const selectedPiece = layout.pieces.find((p) => selectedIds.has(p.id));

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
      measureTool: isMeasureToolActive
        ? {
            start: measureStart,
            current: measureCurrent,
            locked: measureLocked,
            snapPoint: measureSnapPoint,
            scaleRatio: getScaleRatio(layout.scale),
            scaleName: layout.scale,
          }
        : null,
    });

    ruler?.update();
    collabOverlay.update();
    collabClient.broadcastSelection(selection.all());
    selectionBar.update();

    if (is3DMode) {
      viewer3D?.updateLayout(layout, catalog, {
        colorMode,
        showColorCode: colorMode !== "off",
        shortCircuit: currentShortCircuit,
      });
    }

    const planState = planManager.getState();
    updateToolbarState({
      canUndo: history.canUndo(),
      canRedo: history.canRedo(),
      hasSelection: selectedIds.size > 0,
      snappingEnabled,
      selectionLockDisabled,
      measureToolActive: isMeasureToolActive,
      colorCodeEnabled: colorMode !== "off",
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
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
      openHeightDialogForSelection(preselectedLayerId);
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
    is3DMode: () => is3DMode, toggle3D, toggleMeasureTool, toggleRightSidebar, saveSettingsState, sync,
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
    toggleSelectionLock: () => toggleSelectionLock(),
    cycleColorMode,
    isTrackLabelsEnabled: () => trackLabelsEnabled,
    setTrackLabelsEnabled: (val) => { trackLabelsEnabled = val; },
    isPowerSimEnabled: () => powerSimEnabled,
    setPowerSimEnabled: (val) => { powerSimEnabled = val; },
    openHeightDialogForSelection,
    triggerCollabDialog,
    triggerWelcomeDialog: () => triggerWelcome(),
    triggerDemoTracksDialog: () => triggerDemoTracks(),
    triggerImportCatalog,
    showToast,
  });

  createToolbar(toolbarHost, toolbarEvents);

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
  new CanvasController({
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
    isMeasureToolActive: () => isMeasureToolActive,
    toggleMeasureTool,
    findMeasureSnapPoint,
    updateMeasureHud,
    getMeasureState: () => ({
      start: measureStart,
      current: measureCurrent,
      locked: measureLocked,
      snapPoint: measureSnapPoint,
      snapEnabled: measureSnapEnabled,
      draggingPoint: measureDraggingPoint,
    }),
    setMeasureState: (upd) => {
      if (upd.start !== undefined) measureStart = upd.start;
      if (upd.current !== undefined) measureCurrent = upd.current;
      if (upd.locked !== undefined) measureLocked = upd.locked;
      if (upd.snapPoint !== undefined) measureSnapPoint = upd.snapPoint;
      if (upd.draggingPoint !== undefined) measureDraggingPoint = upd.draggingPoint;
    },
    isSnappingEnabled: () => snappingEnabled,
    setSnappingEnabled: (val) => { snappingEnabled = val; },
    toggleSelectionLock: () => toggleSelectionLock(),
    getColorMode: () => colorMode,
    cycleColorMode,
    isTrackLabelsEnabled: () => trackLabelsEnabled,
    setTrackLabelsEnabled: (val) => { trackLabelsEnabled = val; },
    isPowerSimEnabled: () => powerSimEnabled,
    setPowerSimEnabled: (val) => { powerSimEnabled = val; },
    toggleRightSidebar, openHeightDialogForSelection, openHelpDialog,
    openRampGeneratorForPiece,
    openPrintDialog: () => openPrintDialog(layout, catalog, renderer),
    isPieceInteractive, isModelInteractive, getCatalogEntryForPiece, sync, saveSettingsState, showToast,
    getCurrentSimulation: () => currentSimulation,
    setSelectionBox: (box) => { currentSelectionBox = box; },
    setDraggingBaseboardState: (isDragging, activeBoard, guides) => {
      isDraggingBaseboard = isDragging; activeBaseboard = activeBoard; activeBaseboardSnapGuides = guides;
    },
  });

  // Native Electron Menü-Aktionen empfangen
  if (window.electronAPI) {
    window.electronAPI.onMenuAction((action) => {
      switch (action) {
        case "newPlan": operations.triggerNewPlan(); break;
        case "openPlan": operations.triggerOpenPlan(); break;
        case "savePlan": operations.triggerSavePlan(); break;
        case "savePlanAs": operations.triggerSavePlanAs("railbench"); break;
        case "exportCSV": operations.triggerExportCSV(); break;
        case "copy": operations.copySelected(); break;
        case "paste": operations.pasteClipboard(); break;
        case "duplicate": operations.duplicateSelected(); break;
        case "selectAll": operations.selectAll(); break;
        case "undo": history.undo(); sync(true); break;
        case "redo": history.redo(); sync(true); break;
        case "rotate": operations.rotateSelected(7.5); break;
        case "delete": operations.deleteSelected(); break;
        case "toggleFeeder": operations.toggleSelectedFeeder(); break;
        case "resetView": renderer.resetView(); sync(); break;
        case "toggleSnapping": snappingEnabled = !snappingEnabled; sync(); break;
        case "toggleColorCode": cycleColorMode(); break;
        case "toggleTrackLabels": trackLabelsEnabled = !trackLabelsEnabled; sync(); break;
        case "togglePowerSim": powerSimEnabled = !powerSimEnabled; sync(); break;
        case "toggle3D": toggle3D(); break;
        case "printPlan": openPrintDialog(layout, catalog, renderer); break;
        case "changeHeight": openHeightDialogForSelection(); break;
        case "help": openHelpDialog("overview"); break;
        case "shortcuts": openHelpDialog("shortcuts"); break;
      }
    });
  }

  // Sprachänderungen dynamisch übernehmen
  onLanguageChange(() => {
    createToolbar(toolbarHost, toolbarEvents);
    updateToolbarState({
      themePref: themeManager.getPreference(),
      resolvedTheme: themeManager.getResolvedTheme(),
    });
    selectionBar.updateStaticTexts();
    updateMeasureHud();
    sync(false);
  });

  function loadDemoLayout(demo: DemoLayoutDef) {
    const chosen = demo.createLayout();
    layout.name = chosen.name;
    layout.scale = chosen.scale;
    layout.feedVoltage = chosen.feedVoltage;
    layout.locoCurrentMa = chosen.locoCurrentMa;
    layout.locoCount = chosen.locoCount;
    layout.turnoutLanternCurrentMa = chosen.turnoutLanternCurrentMa;
    layout.turnoutDecoderCurrentMa = chosen.turnoutDecoderCurrentMa;
    layout.layers = chosen.layers;
    layout.activeLayerId = chosen.activeLayerId;
    layout.baseboard = chosen.baseboard;
    layout.baseboards = chosen.baseboards;
    layout.models = chosen.models;
    layout.pieces = chosen.pieces;
    ensureDefaultLayers(layout);

    const matchedCat = catalogRegistry.get(demo.catalogId);
    if (matchedCat) {
      catalog = matchedCat;
      sidebar?.setCatalog(catalog);
    }

    planManager.setPlanInfo(`${layout.name}.railbench`, null, "railbench");
    history = new History(layout);
    selection.clear();
    sync(false);
    saveSettingsState();
    showToast(
      getLanguage() === "de"
        ? `Demo-Layout "${demo.name}" geladen!`
        : `Demo layout "${demo.nameEn}" loaded!`,
      "🚆"
    );
  }

  function triggerDemoTracks() {
    showDemoTracksDialog({
      currentLayoutDirty: planManager.getState().isDirty,
      onSelect: (demo) => {
        loadDemoLayout(demo);
      },
    });
  }

  function applyStartupAction(action: "recent" | "new" | "demo" | "open", cached?: Layout | null) {
    if (action === "recent" && cached && cached.pieces?.length) {
      layout = cached;
      ensureDefaultLayers(layout);
      planManager.setPlanInfo(layout.name + ".railbench", null, "railbench");
    } else if (action === "new") {
      layout.pieces = [];
      layout.models = [];
      ensureDefaultLayers(layout);
      planManager.setPlanInfo(getLanguage() === "de" ? "Neuer Plan.railbench" : "New Layout.railbench", null, "railbench");
    } else if (action === "demo") {
      initDemoTracks(layout, catalog);
      planManager.setPlanInfo(`${layout.name}.railbench`, null, "railbench");
      triggerDemoTracks();
    } else {
      initDemoTracks(layout, catalog);
      planManager.setPlanInfo(`${layout.name}.railbench`, null, "railbench");
      if (action === "open") setTimeout(() => operations.triggerOpenPlan(), 50);
    }
    history = new History(layout);
    selection.clear();
    sync(false);
  }

  async function triggerWelcome() {
    const cached = planManager.loadFromLocalStorage();
    const hasRec = Boolean(cached && cached.pieces?.length);
    const recName = cached?.name ? `${cached.name}.railbench` : undefined;
    const choice = await showWelcomeDialog(hasRec, recName);
    if (choice.dontAskAgain) {
      saveAppSettings({ startupAction: choice.action });
    } else {
      saveAppSettings({ startupAction: "ask" });
    }
    applyStartupAction(choice.action, cached);
  }

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
    applyStartupAction(choice.action, cachedPlan);
  } else {
    applyStartupAction(startupAction as "recent" | "new" | "demo" | "open", cachedPlan);
  }
}

main();
