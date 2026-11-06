import { Renderer } from "@/render/renderer";
import { showWelcomeDialog, type StartupChoice, type StartupAction } from "@/ui/welcome-dialog";
import { attachCamera } from "@/render/camera";
import {
  createLayout,
  ensureDefaultLayers,
  ensureBaseboard,
  getBaseboards,
  getBaseboardBounds,
  getBaseboardPolygon,
  isPointInsideBaseboard,
  isPointNearBaseboardEdge,
  isPointInBaseboardHeader,
} from "@/core/layout";
import { snapBaseboard, type BaseboardSnapGuide } from "@/core/baseboard-snapping";
import type { BaseboardConfig } from "@/core/types";
import { Ruler } from "@/ui/ruler";
import { Viewer3D } from "@/render/viewer-3d";
import type { Layout, TrackPiece, Transform, Model3D, ColorMode } from "@/core/types";
import {
  catalogRegistry,
  computeWorldConnectors,
  type Catalog,
  type CatalogEntry,
} from "@/catalog/catalog";
import { findBestSnapForPiece, type BestPieceSnap } from "@/core/snapping";
import {
  simulatePowerDrop,
  type SimulationSummary,
  getDefaultLocoCurrentMa,
  isTurnoutEntry,
  detectShortCircuit,
  type ShortCircuitDetectionResult,
} from "@/core/simulation";
import { Selection } from "@/editor/selection";
import {
  History,
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
  MoveBaseboardCommand,
} from "@/editor/commands";
import {
  isPointInsideModel,
  getModelRotationHandleWorld,
  parseGlbOrGltfFile,
} from "@/core/models-3d";
import { createToolbar, updateToolbarState, type ToolbarEvents } from "@/ui/toolbar";
import { getTranslations, onLanguageChange, getLanguage } from "@/i18n";
import { Sidebar } from "@/ui/sidebar";
import { RightSidebar } from "@/ui/right-sidebar";
import { PlanManager, type PlanFormat } from "@/io/plan-manager";
import { exportPieceListCSV, downloadFile } from "@/io/export";
import { openPrintDialog } from "@/io/print";
import { openChangeHeightDialog } from "@/ui/height-dialog";
import { openHelpDialog, type HelpTab } from "@/ui/help-dialog";
import { openPowerSettingsDialog } from "@/ui/power-dialog";
import { openCollabDialog } from "@/ui/collab-dialog";
import { CollabClient } from "@/collab/collab-client";
import { CollabOverlay } from "@/collab/collab-overlay";
import { PerfMonitor } from "@/ui/perf-monitor";
import { themeManager } from "@/theme/theme-manager";
import {
  dist,
  normAngle,
  rad2deg,
  rotate,
  distanceToTrackPiece,
  getPieceCenterWorld,
  getPieceRotationHandleWorld,
  isPieceInSelectionBox,
  computeGroupCentroid,
  rotateGroupAroundCentroid,
  type RotatableItem,
} from "@/core/geometry";
import { loadAppSettings, saveAppSettings } from "@/settings/settings-manager";

async function main() {
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
  let rightSidebar: RightSidebar | null = null;
  let currentSimulation: SimulationSummary | undefined = undefined;
  let currentShortCircuit: ShortCircuitDetectionResult | undefined = undefined;
  let currentSelectionBox: { start: { x: number; y: number }; end: { x: number; y: number } } | null = null;

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
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
      rulerVisible: ruler?.isVisible() ?? true,
      rightSidebarCollapsed,
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
    return catalog.entries.find((e) => e.id === piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId);
  }

  function isPieceLockedOrHidden(piece: TrackPiece): boolean {
    if (!layout.layers) return false;
    const layer = layout.layers.find((l) => l.id === (piece.layerId || "layer-1"));
    if (!layer) return false;
    return !layer.visible || layer.locked;
  }

  /** Prüft, ob ein Gleis/eine Straße auf der aktiven Ebene liegt und interaktiv (selektierbar & verschiebbar) ist. */
  function isPieceInteractive(piece: TrackPiece): boolean {
    const activeId = getActiveLayerId();
    const pieceLayerId = piece.layerId || "layer-1";
    if (pieceLayerId !== activeId) return false;
    return !isPieceLockedOrHidden(piece);
  }

  /** Prüft, ob ein 3D-Modell auf der aktiven Ebene liegt und interaktiv ist. */
  function isModelInteractive(model: Model3D): boolean {
    const activeId = getActiveLayerId();
    const modelLayerId = model.layerId || layout.layers?.[0]?.id || "layer-1";
    if (modelLayerId !== activeId) return false;
    if (!layout.layers) return true;
    const layer = layout.layers.find((l) => l.id === modelLayerId);
    return !layer || (layer.visible && !layer.locked);
  }

  // Plan-Manager für natives/Web-Speichern und Öffnen
  const planManager = new PlanManager((state) => {
    updateToolbarState({
      canUndo: history.canUndo(),
      canRedo: history.canRedo(),
      hasSelection: selection.all().length > 0,
      snappingEnabled,
      colorCodeEnabled: colorMode !== "off",
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
      totalPieces: layout.pieces.length,
      activeFileName: state.fileName ?? layout.name,
      isDirty: state.isDirty,
    });
  });

  // Renderer initialisieren
  const canvasHost = document.getElementById("canvas-host")!;
  const renderer = new Renderer();
  await renderer.init(canvasHost);

  // Lineal (Ruler) initialisieren
  const ruler = new Ruler({
    host: canvasHost,
    world: renderer.world,
    onCenterOrigin: () => {
      renderer.resetView();
      ruler.update();
      sync();
      saveSettingsState();
    },
    visible: savedSettings.rulerVisible ?? true,
  });

  // Kollaboration (Multi-User Server) initialisieren
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

  // Live Performance & Hardware Monitor (FPS, CPU Render-Zeit, RAM, GPU)
  function generateStressTracks(count: number) {
    const entry = catalog.entries[0] || catalogRegistry.getAll()[0]?.entries[0];
    if (!entry) return;

    const newPieces: TrackPiece[] = [];
    const cols = 40;
    const startX = -2000;
    const startY = -1000;

    for (let i = 0; i < count; i++) {
      const col = i % cols;
      const row = Math.floor(i / cols);
      const pieceId = `stress-${Date.now()}-${i}-${Math.random().toString(36).substring(2, 6)}`;
      const transform: Transform = {
        x: startX + col * 200,
        y: startY + row * 80,
        rotation: 0,
      };
      newPieces.push({
        id: pieceId,
        catalogId: entry.id,
        layerId: layout.activeLayerId || layout.layers?.[0]?.id || "default",
        transform,
        connectors: computeWorldConnectors(entry, transform),
      });
    }

    history.run(new AddPiecesCommand(newPieces));
    sync(true);
  }

  const perfMonitor = new PerfMonitor({
    host: canvasHost,
    renderer,
    getLayout: () => layout,
    onGenerateStressTracks: (count) => generateStressTracks(count),
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

  // 3D Viewer initialisieren
  const viewer3D = new Viewer3D(canvasHost);
  let is3DMode = false;

  function toggle3D(force?: boolean) {
    is3DMode = force !== undefined ? force : !is3DMode;
    if (is3DMode) {
      ruler.setVisible(false);
      viewer3D.show(() => toggle3D(false));
      viewer3D.updateLayout(layout, catalog, { colorMode, showColorCode: colorMode !== "off" });
      viewer3D.resetCamera();
    } else {
      viewer3D.hide();
      ruler.setVisible(true);
    }
    sync();
  }

  // Prüfen, ob ein gespeicherter Plan im LocalStorage vorhanden ist
  const cachedPlan = planManager.loadFromLocalStorage();
  if (cachedPlan && cachedPlan.pieces && cachedPlan.pieces.length > 0) {
    layout = cachedPlan;
    ensureDefaultLayers(layout);
    history = new History(layout);
    planManager.setPlanInfo(layout.name + ".railbench", null, "railbench");
  } else {
    initDemoTracks(layout, catalog);
    planManager.setPlanInfo("Model Railway Layout H0.railbench", null, "railbench");
  }

  // Tisch-Drag & Snap Zustand
  let isDraggingBaseboard = false;
  let activeBaseboard: BaseboardConfig | null = null;
  let baseboardDragStartWorld = { x: 0, y: 0 };
  let baseboardOriginalPos = { positionMode: "custom" as const, customX: 0, customY: 0 };
  let activeBaseboardSnapGuides: BaseboardSnapGuide[] = [];
  let baseboardInitialTrackTransforms = new Map<string, Transform>();

  // Distanz-Messwerkzeug (Measure Distance Tool) Zustand
  let isMeasureToolActive = false;
  let measureStart: { x: number; y: number } | null = null;
  let measureCurrent: { x: number; y: number } | null = null;
  let measureLocked = false;
  let measureSnapPoint: { x: number; y: number } | null = null;
  let measureSnapEnabled = true;
  let measureDraggingPoint: "A" | "B" | null = null;

  function getScaleRatio(scale: string): number {
    switch (scale) {
      case "H0": return 87;
      case "TT": return 120;
      case "N": return 160;
      case "Z": return 220;
      case "1": return 32;
      case "G": return 22.5;
      default: return 87;
    }
  }

  // Floating Measure HUD
  const measureHud = document.createElement("div");
  measureHud.id = "measure-hud";
  measureHud.className = "measure-hud";
  measureHud.style.display = "none";
  measureHud.innerHTML = `
    <div class="measure-hud-header">
      <span class="measure-hud-title">Distanzmessung</span>
      <span class="measure-hud-status" id="measure-hud-status"></span>
      <div class="measure-hud-actions">
        <button type="button" class="measure-hud-btn active" id="measure-hud-snap" title="Objektfang an/aus (Alt)">Fang: AN</button>
        <button type="button" class="measure-hud-btn" id="measure-hud-reset" title="Neu messen (Esc)">Neu</button>
        <button type="button" class="measure-hud-close" id="measure-hud-close" title="Messen beenden (Esc)">✕</button>
      </div>
    </div>
    <div class="measure-hud-body">
      <div class="measure-hud-dist-box">
        <span class="measure-hud-dist" id="measure-hud-dist">0.0 mm</span>
        <span class="measure-hud-proto" id="measure-hud-proto"></span>
      </div>
      <div class="measure-hud-coords">
        <div class="measure-coord-group">
          <span class="measure-coord-tag tag-a">A</span>
          <label class="measure-coord-label">X <input type="number" step="0.5" id="measure-ax" class="measure-coord-input" placeholder="0" /></label>
          <label class="measure-coord-label">Y <input type="number" step="0.5" id="measure-ay" class="measure-coord-input" placeholder="0" /></label>
        </div>
        <div class="measure-coord-group">
          <span class="measure-coord-tag tag-b">B</span>
          <label class="measure-coord-label">X <input type="number" step="0.5" id="measure-bx" class="measure-coord-input" placeholder="0" /></label>
          <label class="measure-coord-label">Y <input type="number" step="0.5" id="measure-by" class="measure-coord-input" placeholder="0" /></label>
        </div>
      </div>
      <div class="measure-hud-deltas" id="measure-hud-deltas">
        ΔX: 0.0 mm • ΔY: 0.0 mm • 0.0°
      </div>
    </div>
  `;
  document.body.appendChild(measureHud);

  const measureSnapBtn = measureHud.querySelector("#measure-hud-snap") as HTMLButtonElement | null;
  const measureResetBtn = measureHud.querySelector("#measure-hud-reset") as HTMLButtonElement | null;
  const measureCloseBtn = measureHud.querySelector("#measure-hud-close") as HTMLButtonElement | null;
  const measureAxInput = measureHud.querySelector("#measure-ax") as HTMLInputElement | null;
  const measureAyInput = measureHud.querySelector("#measure-ay") as HTMLInputElement | null;
  const measureBxInput = measureHud.querySelector("#measure-bx") as HTMLInputElement | null;
  const measureByInput = measureHud.querySelector("#measure-by") as HTMLInputElement | null;

  measureSnapBtn?.addEventListener("click", () => {
    measureSnapEnabled = !measureSnapEnabled;
    const t = getTranslations().toolbar.toggles;
    if (measureSnapBtn) {
      measureSnapBtn.textContent = measureSnapEnabled ? t.measureSnapOn : t.measureSnapOff;
      measureSnapBtn.classList.toggle("active", measureSnapEnabled);
    }
  });

  measureResetBtn?.addEventListener("click", () => {
    measureStart = null;
    measureCurrent = null;
    measureLocked = false;
    measureDraggingPoint = null;
    updateMeasureHud();
    sync();
  });

  measureCloseBtn?.addEventListener("click", () => {
    toggleMeasureTool(false);
  });

  function onMeasureCoordManualInput() {
    const ax = parseFloat(measureAxInput?.value ?? "");
    const ay = parseFloat(measureAyInput?.value ?? "");
    const bx = parseFloat(measureBxInput?.value ?? "");
    const by = parseFloat(measureByInput?.value ?? "");

    if (!isNaN(ax) && !isNaN(ay)) {
      measureStart = { x: ax, y: ay };
    }
    if (!isNaN(bx) && !isNaN(by)) {
      measureCurrent = { x: bx, y: by };
      measureLocked = true;
    }
    updateMeasureHud(false);
    sync();
  }

  [measureAxInput, measureAyInput, measureBxInput, measureByInput].forEach((inp) => {
    inp?.addEventListener("input", onMeasureCoordManualInput);
    inp?.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === "Escape") {
        inp.blur();
      }
    });
  });

  function updateMeasureHud(updateInputs = true) {
    if (!isMeasureToolActive) {
      measureHud.style.display = "none";
      return;
    }
    measureHud.style.display = "flex";
    const t = getTranslations().toolbar.toggles;
    const statusEl = document.getElementById("measure-hud-status");
    const distEl = document.getElementById("measure-hud-dist");
    const protoEl = document.getElementById("measure-hud-proto");
    const deltasEl = document.getElementById("measure-hud-deltas");

    if (measureSnapBtn) {
      measureSnapBtn.textContent = measureSnapEnabled ? t.measureSnapOn : t.measureSnapOff;
      measureSnapBtn.classList.toggle("active", measureSnapEnabled);
    }
    if (measureResetBtn) {
      measureResetBtn.textContent = t.measureReset;
    }

    if (!measureStart) {
      if (statusEl) statusEl.textContent = t.measurePromptA;
      if (distEl) distEl.textContent = "0.0 mm";
      if (protoEl) protoEl.textContent = "";
      if (deltasEl) deltasEl.textContent = "ΔX: 0.0 mm • ΔY: 0.0 mm • 0.0°";
      if (updateInputs) {
        if (measureAxInput && document.activeElement !== measureAxInput) measureAxInput.value = "";
        if (measureAyInput && document.activeElement !== measureAyInput) measureAyInput.value = "";
        if (measureBxInput && document.activeElement !== measureBxInput) measureBxInput.value = "";
        if (measureByInput && document.activeElement !== measureByInput) measureByInput.value = "";
      }
      return;
    }

    const p1 = measureStart;
    const p2 = measureCurrent || p1;
    const dx = p2.x - p1.x;
    const dy = p2.y - p1.y;
    const d = Math.hypot(dx, dy);
    const angle = (Math.atan2(dy, dx) * 180) / Math.PI;
    const scaleRatio = getScaleRatio(layout.scale);

    if (statusEl) {
      statusEl.textContent = measureLocked ? t.measureMeasured : t.measurePromptB;
    }

    if (distEl) {
      distEl.textContent = d >= 1000 ? `${d.toFixed(1)} mm (${(d / 1000).toFixed(2)} m)` : `${d.toFixed(1)} mm`;
    }

    if (protoEl) {
      if (scaleRatio && scaleRatio > 1 && d > 0.1) {
        const protoM = (d * scaleRatio) / 1000;
        protoEl.textContent = `(${t.measurePrototype} ${layout.scale} 1:${scaleRatio}: ${protoM.toFixed(2)} m)`;
      } else {
        protoEl.textContent = "";
      }
    }

    if (deltasEl) {
      deltasEl.textContent = `ΔX: ${Math.abs(dx).toFixed(1)} mm • ΔY: ${Math.abs(dy).toFixed(1)} mm • ${Math.abs(angle).toFixed(1)}°`;
    }

    if (updateInputs) {
      if (measureAxInput && document.activeElement !== measureAxInput) measureAxInput.value = p1.x.toFixed(1);
      if (measureAyInput && document.activeElement !== measureAyInput) measureAyInput.value = p1.y.toFixed(1);
      if (measureCurrent) {
        if (measureBxInput && document.activeElement !== measureBxInput) measureBxInput.value = p2.x.toFixed(1);
        if (measureByInput && document.activeElement !== measureByInput) measureByInput.value = p2.y.toFixed(1);
      } else {
        if (measureBxInput && document.activeElement !== measureBxInput) measureBxInput.value = "";
        if (measureByInput && document.activeElement !== measureByInput) measureByInput.value = "";
      }
    }
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

  function toggleMeasureTool(force?: boolean) {
    isMeasureToolActive = force !== undefined ? force : !isMeasureToolActive;
    if (!isMeasureToolActive) {
      measureStart = null;
      measureCurrent = null;
      measureLocked = false;
      measureDraggingPoint = null;
      measureSnapPoint = null;
      canvas.style.cursor = "default";
      measureHud.style.display = "none";
    } else {
      canvas.style.cursor = "crosshair";
      updateMeasureHud();
    }
    sync();
  }

  // Sync-Funktion für UI und Rendering
  function sync(dirty = false) {
    if (dirty) {
      planManager.markDirty(true);
      planManager.saveToLocalStorage(layout);
      if (!isApplyingRemote) {
        collabClient.broadcastLayout(layout);
      }
    }

    if (powerSimEnabled) {
      currentSimulation = simulatePowerDrop(layout, catalog);
    } else {
      currentSimulation = undefined;
    }

    currentShortCircuit = detectShortCircuit(layout, catalog);

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

    ruler.update();
    collabOverlay.update();
    collabClient.broadcastSelection(selection.all());
    updateSelectionFloatingBar();

    if (is3DMode) {
      viewer3D.updateLayout(layout, catalog, {
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
      measureToolActive: isMeasureToolActive,
      colorCodeEnabled: colorMode !== "off",
      colorMode,
      trackLabelsEnabled,
      powerSimEnabled,
      hasShortCircuit: currentShortCircuit.hasShortCircuit,
      shortCircuitCount: currentShortCircuit.shortCircuitPoints.length,
      rulersEnabled: ruler.isVisible(),
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
      collabStatus: collabClient.getStatus(),
      collabUsersCount: collabClient.getUsers().length,
    });

    sidebar.update(layout, currentSimulation);
    if (rightSidebar) {
      rightSidebar.update(layout);
    }
  }

  // Sidebar initialisieren
  const sidebarHost = document.getElementById("sidebar")!;
  const sidebar = new Sidebar(sidebarHost, catalog, layout, {
    onAddPiece: (entry) => {
      addPieceToLayout(entry);
      sync(true);
    },
    onAddModel: (model) => {
      addModelToLayout(model);
    },
    onExportCSV: () => {
      triggerExportCSV();
    },
    onSelectSystem: (systemId) => {
      const newCat = catalogRegistry.get(systemId);
      if (newCat) {
        catalog = newCat;
        sidebar.setCatalog(catalog);
        if (layout.pieces.length === 0 && (newCat.scale === "H0" || newCat.scale === "N" || newCat.scale === "TT" || newCat.scale === "Z" || newCat.scale === "1")) {
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
      moveSelectedToLayer(layerId, { start: 0, end: 0 });
    },
    selectedPieceIds: () => selection.all(),
  });

  if (savedSettings.activeSidebarTab) {
    sidebar.setActiveTab(savedSettings.activeSidebarTab as any);
  }

  // Funktion zum Hinzufügen eines Gleises
  function addPieceToLayout(entry: CatalogEntry, initialPos?: { x: number; y: number }) {
    let targetTransform: Transform = { x: 0, y: 0, rotation: 0 };

    if (initialPos) {
      targetTransform = { x: initialPos.x, y: initialPos.y, rotation: 0 };
    } else if (layout.pieces.length > 0) {
      const last = layout.pieces[layout.pieces.length - 1]!;
      targetTransform = { x: last.transform.x + 190, y: last.transform.y, rotation: last.transform.rotation };
    }

    if (snappingEnabled && layout.pieces.length > 0) {
      const snap = findBestSnapForPiece(entry, targetTransform, layout.pieces, undefined, 60, 45);
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

  // Export CSV Helper
  async function triggerExportCSV() {
    const csv = exportPieceListCSV(layout, catalog);
    const defaultName = `${(layout.name || "Gleisplan").replace(/\s+/g, "_")}_Stueckliste.csv`;

    if (window.electronAPI) {
      await window.electronAPI.exportCSV(defaultName, csv);
    } else {
      downloadFile(defaultName, csv, "text/csv;charset=utf-8;");
    }
  }

  // Plan öffnen Helper
  async function triggerOpenPlan() {
    const result = await planManager.openPlan();
    if (result) {
      layout = result.layout;
      history = new History(layout);
      selection.clear();
      renderer.resetView();
      sync(false);
    }
  }

  // Plan speichern Helper
  async function triggerSavePlan() {
    await planManager.savePlan(layout);
    sync(false);
  }

  // Plan speichern unter Helper
  async function triggerSavePlanAs(format: PlanFormat = "railbench") {
    await planManager.savePlanAs(layout, format);
    sync(false);
  }

  function addModelToLayout(model: Model3D, customPosition?: { x: number; y: number }) {
    if (!layout.models) layout.models = [];
    const m: Model3D = { ...model };
    if (customPosition) {
      m.transform = { x: Math.round(customPosition.x), y: Math.round(customPosition.y), rotation: m.transform.rotation };
    } else {
      const center = renderer.screenToWorld(window.innerWidth / 2, window.innerHeight / 2);
      m.transform = { x: Math.round(center.x), y: Math.round(center.y), rotation: 0 };
    }
    if (!m.layerId) m.layerId = layout.activeLayerId || layout.layers?.[0]?.id;
    history.run(new AddModelCommand(m));
    selection.set(m.id);
    sync(true);
  }

  function rotateSelected(deltaDegrees = 7.5) {
    const selectedIds = selection.all();
    if (selectedIds.length === 0) return;

    const rotatableItems: RotatableItem[] = [];
    for (const id of selectedIds) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece && isPieceInteractive(piece)) {
        const entry = getCatalogEntryForPiece(piece);
        rotatableItems.push({ id: piece.id, type: "piece", transform: { ...piece.transform }, entry });
      }
      const model = layout.models?.find((m) => m.id === id);
      if (model && isModelInteractive(model)) {
        rotatableItems.push({ id: model.id, type: "model", transform: { ...model.transform } });
      }
    }

    if (rotatableItems.length === 0) return;

    // Dreht ein Einzelstück um sein eigenes Centrum bzw. mehrere Teile starr als eine Einheit um ihren Schwerpunkt
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
    sync(true);
  }

  function deleteSelected() {
    const selectedIds = selection.all();
    for (const id of selectedIds) {
      const piece = layout.pieces.find((x) => x.id === id);
      if (piece && isPieceInteractive(piece)) {
        history.run(new RemovePieceCommand(id));
      }
      const model = layout.models?.find((m) => m.id === id);
      if (model && isModelInteractive(model)) {
        history.run(new RemoveModelCommand(id));
      }
    }
    selection.clear();
    sync(true);
  }

  interface TrackClipboardData {
    pieces: TrackPiece[];
    models: Model3D[];
  }
  let trackClipboard: TrackClipboardData | null = null;
  let pasteOffsetCount = 1;

  function copySelected(): boolean {
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

      // In localStorage & System-Zwischenablage ablegen
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
      showToast(isDe ? `${itemLabel} in Zwischenablage kopiert (Strg+C)` : `${itemLabel} copied to clipboard (Ctrl+C)`, "📋");
      return true;
    }
    return false;
  }

  function pasteClipboard(customOffset?: { x: number; y: number }): boolean {
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
      showToast(isDe ? "Zwischenablage ist leer" : "Clipboard is empty", "⚠️");
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
      const entry = getCatalogEntryForPiece(orig);
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
      sync(true);

      const total = newPieces.length + newModels.length;
      const isDe = getLanguage() === "de";
      const itemLabel = isDe
        ? `${total} Element${total === 1 ? "" : "e"}`
        : `${total} item${total === 1 ? "" : "s"}`;
      showToast(isDe ? `${itemLabel} eingefügt (Strg+V)` : `${itemLabel} pasted (Ctrl+V)`, "📥");
      return true;
    }
    return false;
  }

  function duplicateSelected() {
    const selectedIds = selection.all();
    if (selectedIds.length === 0) {
      const isDe = getLanguage() === "de";
      showToast(isDe ? "Keine Gleise zum Duplizieren markiert" : "No tracks selected to duplicate", "⚠️");
      return;
    }
    copySelected();
    pasteClipboard({ x: 40, y: 40 });
  }

  function selectAll() {
    selection.clear();
    for (const p of layout.pieces) {
      if (isPieceInteractive(p)) {
        selection.add(p.id);
      }
    }
    if (layout.models) {
      for (const m of layout.models) {
        if (isModelInteractive(m)) {
          selection.add(m.id);
        }
      }
    }
    sync();
  }

  function toggleSelectedFeeder() {
    const selected = selection.all();
    if (selected[0]) {
      history.run(new ToggleFeederCommand(selected[0]));
      sync(true);
    }
  }

  function toggleSelectedIsolator() {
    const selected = selection.all();
    if (selected[0]) {
      history.run(new ToggleIsolatorCommand(selected[0]));
      sync(true);
    }
  }

  function toggleSelectedCatenary() {
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleCatenaryCommand(selected));
      sync(true);
    }
  }

  function toggleSelectedCatenaryFeeder() {
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleCatenaryFeederCommand(selected));
      sync(true);
    }
  }

  function toggleSelectedCatenaryIsolator() {
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleCatenaryIsolatorCommand(selected));
      sync(true);
    }
  }

  function toggleSelectedTurnoutLantern() {
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleTurnoutLanternCommand(selected));
      sync(true);
    }
  }

  function toggleSelectedTurnoutDecoder() {
    const selected = selection.all();
    if (selected.length > 0) {
      history.run(new ToggleTurnoutDecoderCommand(selected));
      sync(true);
    }
  }

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

  function moveSelectedToLayer(
    layerId: string,
    elevation?: { start: number; end: number } | Map<string, { start: number; end: number }>,
  ) {
    const selected = selection.all();
    if (selected.length === 0) return;
    history.run(new MovePiecesToLayerCommand(selected, layerId, elevation));
    sync(true);
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
        moveSelectedToLayer(targetLayerId, elevation);
      },
    });
  }

  function injectSelectionBarStyles(): void {
  const STYLE_ID = "selection-bar-overrides";
  if (document.getElementById(STYLE_ID)) return; // only inject once

  const style = document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = `
    .selection-floating-bar {
      gap: 6px;
      padding: 6px 10px;
      max-width: calc(100% - 24px);
      overflow-x: auto;
      scrollbar-width: none;
    }
    .selection-floating-bar .sel-bar-btn {
      padding: 4px 8px;
      font-size: 12px;
    }
    .selection-floating-bar .sel-bar-divider {
      margin: 0 2px;
    }
    .selection-floating-bar .sel-bar-select {
      width: 84px;
      max-width: 84px;
      padding: 2px 4px;
      font-size: 12px;
      text-overflow: ellipsis;
    }
    .selection-floating-bar .sel-bar-angle-input {
      width: 52px;
    }
    #sel-bar-elev-input {
      width: 84px; /* wider for ramp placeholder like +10→+20 */
    }
  `;
  document.head.appendChild(style);
  }
  injectSelectionBarStyles();
  // --- FLOATING SELECTION ACTION BAR ---
  const selectionFloatingBar = document.createElement("div");
  selectionFloatingBar.id = "selection-floating-bar";
  selectionFloatingBar.className = "selection-floating-bar hidden";
  const initialSelT = getTranslations().selectionBar;
  selectionFloatingBar.innerHTML = `
    <span class="sel-bar-count" id="sel-bar-count">0 ${initialSelT.piecePlural}</span>
    <div class="sel-bar-layer-group">
      <select id="sel-bar-layer-select" class="sel-bar-select" title="${initialSelT.layerLabel}"></select>
    </div>
    <div class="sel-bar-angle-group" id="sel-bar-elev-group">
      <button id="sel-bar-btn-height" class="sel-bar-btn" title="${initialSelT.heightRampBtn}">${initialSelT.heightLabel}</button>
      <input type="number" id="sel-bar-elev-input" class="sel-bar-angle-input" step="any" placeholder="0" title="Set exact height in mm (Enter to apply, Up/Down for ±1 mm, Shift for ±0.1)" />
      <span class="sel-bar-unit">mm</span>
    </div>
    <div class="sel-bar-angle-group">
      <span class="sel-bar-angle-label" id="sel-bar-angle-lbl">${initialSelT.angleLabel}</span>
      <input type="number" id="sel-bar-angle-input" class="sel-bar-angle-input" step="any" placeholder="0" title="Set exact rotation angle in degrees (Enter to apply, Up/Down for ±7.5°)" />
      <span class="sel-bar-unit">°</span>
    </div>
    <span class="sel-bar-divider"></span>
    <button id="sel-bar-btn-feeder" class="sel-bar-btn" title="Gleis-Einspeisung an/aus (F)">⚡ Einspeisung</button>
    <button id="sel-bar-btn-isolator" class="sel-bar-btn" title="${initialSelT.isolatorInactiveTitle}">${initialSelT.isolatorInactive}</button>
    <span class="sel-bar-divider"></span>
    <button id="sel-bar-btn-catenary" class="sel-bar-btn" title="${initialSelT.catenaryInactiveTitle}">${initialSelT.catenaryBtn}</button>
    <button id="sel-bar-btn-catenary-feed" class="sel-bar-btn" title="${initialSelT.catenaryFeederInactiveTitle}">${initialSelT.catenaryFeederBtn}</button>
    <button id="sel-bar-btn-catenary-iso" class="sel-bar-btn" title="${initialSelT.catenaryIsolatorInactiveTitle}">${initialSelT.catenaryIsolatorBtn}</button>
    <span class="sel-bar-divider"></span>
    <button id="sel-bar-btn-decoder" class="sel-bar-btn" title="${initialSelT.decoderInactiveTitle}">${initialSelT.decoderBtn}</button>
    <button id="sel-bar-btn-lantern" class="sel-bar-btn" title="${initialSelT.lanternInactiveTitle}">${initialSelT.lanternBtn}</button>
    <span class="sel-bar-divider"></span>
    <button id="sel-bar-btn-delete" class="sel-bar-btn sel-bar-btn-danger" title="${initialSelT.deleteBtn} (Del)">${initialSelT.deleteBtn}</button>
  `;
  canvasHost.appendChild(selectionFloatingBar);

  const selBarCount = selectionFloatingBar.querySelector("#sel-bar-count") as HTMLElement;
  const selBarLayerLbl = selectionFloatingBar.querySelector("#sel-bar-layer-lbl") as HTMLElement;
  const selBarLayerSelect = selectionFloatingBar.querySelector("#sel-bar-layer-select") as HTMLSelectElement;
  const selBarElevGroup = selectionFloatingBar.querySelector("#sel-bar-elev-group") as HTMLElement;
  const selBarElevLabel = selectionFloatingBar.querySelector("#sel-bar-elev-label") as HTMLElement;
  const selBarElevInput = selectionFloatingBar.querySelector("#sel-bar-elev-input") as HTMLInputElement;
  const selBarAngleLabel = selectionFloatingBar.querySelector("#sel-bar-angle-lbl") as HTMLElement;
  const selBarAngleInput = selectionFloatingBar.querySelector("#sel-bar-angle-input") as HTMLInputElement;
  const selBarBtnHeight = selectionFloatingBar.querySelector("#sel-bar-btn-height") as HTMLButtonElement;
  const selBarBtnFeeder = selectionFloatingBar.querySelector("#sel-bar-btn-feeder") as HTMLButtonElement;
  const selBarBtnIsolator = selectionFloatingBar.querySelector("#sel-bar-btn-isolator") as HTMLButtonElement;
  const selBarBtnCatenary = selectionFloatingBar.querySelector("#sel-bar-btn-catenary") as HTMLButtonElement;
  const selBarBtnCatenaryFeed = selectionFloatingBar.querySelector("#sel-bar-btn-catenary-feed") as HTMLButtonElement;
  const selBarBtnCatenaryIso = selectionFloatingBar.querySelector("#sel-bar-btn-catenary-iso") as HTMLButtonElement;
  const selBarBtnDecoder = selectionFloatingBar.querySelector("#sel-bar-btn-decoder") as HTMLButtonElement;
  const selBarBtnLantern = selectionFloatingBar.querySelector("#sel-bar-btn-lantern") as HTMLButtonElement;
  const selBarBtnDelete = selectionFloatingBar.querySelector("#sel-bar-btn-delete") as HTMLButtonElement;

  function updateSelectionFloatingBarStatic(): void {
    const t = getTranslations().selectionBar;
    if (selBarElevLabel) selBarElevLabel.textContent = t.heightLabel;
    if (selBarBtnHeight) selBarBtnHeight.textContent = t.rampPrefix;
    if (selBarLayerLbl) selBarLayerLbl.textContent = t.layerLabel;
    if (selBarAngleLabel) selBarAngleLabel.textContent = t.angleLabel;
    if (selBarBtnHeight) selBarBtnHeight.title = t.heightRampBtn;
    if (selBarBtnCatenary) {
      selBarBtnCatenary.textContent = t.catenaryBtn;
      selBarBtnCatenary.title = t.catenaryInactiveTitle;
    }
    if (selBarBtnCatenaryFeed) {
      selBarBtnCatenaryFeed.textContent = t.catenaryFeederBtn;
      selBarBtnCatenaryFeed.title = t.catenaryFeederInactiveTitle;
    }
    if (selBarBtnCatenaryIso) {
      selBarBtnCatenaryIso.textContent = t.catenaryIsolatorBtn;
      selBarBtnCatenaryIso.title = t.catenaryIsolatorInactiveTitle;
    }
    if (selBarBtnLantern) {
      selBarBtnLantern.textContent = t.lanternBtn;
      selBarBtnLantern.title = t.lanternInactiveTitle;
    }
    if (selBarBtnDecoder) {
      selBarBtnDecoder.textContent = t.decoderBtn;
      selBarBtnDecoder.title = t.decoderInactiveTitle;
    }
    if (selBarBtnDelete) {
      selBarBtnDelete.textContent = t.deleteBtn;
      selBarBtnDelete.title = `${t.deleteBtn} (Del)`;
    }
    updateSelectionFloatingBar();
  }
  function applyHeightInput(): void {
  const target = parseFloat(selBarElevInput.value);
  if (isNaN(target)) return;
  const layers = layout.layers || [];

  // group interactive selected pieces by layer (elevation is relative to the layer height)
  const byLayer = new Map<string, string[]>();
  for (const id of selection.all()) {
    const p = layout.pieces.find((x) => x.id === id);
    if (!p || !isPieceInteractive(p)) continue;
    const lid = p.layerId || "layer-1";
    const base = layers.find((l) => l.id === lid)?.heightMm ?? 0;
    const elev = target - base;
    // skip pieces already at this height (avoids duplicate undo steps on Enter + change)
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
  sync(true);
}

selBarElevInput.addEventListener("change", applyHeightInput);
selBarElevInput.addEventListener("keydown", (e: KeyboardEvent) => {
  if (e.key === "Enter") {
    applyHeightInput();
    selBarElevInput.blur();
  } else if (e.key === "Escape") {
    selBarElevInput.blur();
    updateSelectionFloatingBar();
  } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
    e.preventDefault();
    const step = (e.shiftKey ? 0.1 : 1) * (e.key === "ArrowUp" ? 1 : -1);
    const cur = parseFloat(selBarElevInput.value) || 0;
    selBarElevInput.value = (Math.round((cur + step) * 10) / 10).toString();
    applyHeightInput();
  }
});

  function applyAngleInput(): void {
    const rawVal = parseFloat(selBarAngleInput.value);
    if (isNaN(rawVal)) return;
    const targetAngle = normAngle(rawVal);
    const selectedIds = selection.all();
    if (selectedIds.length === 0) return;

    const firstPiece = layout.pieces.find((p) => p.id === selectedIds[0]);
    const firstModel = layout.models?.find((m) => m.id === selectedIds[0]);
    const currentBaseAngle = firstPiece ? firstPiece.transform.rotation : (firstModel ? firstModel.transform.rotation : 0);
    const delta = targetAngle - currentBaseAngle;

    for (const id of selectedIds) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece && isPieceInteractive(piece)) {
        const oldT = { ...piece.transform };
        const newRot = selectedIds.length === 1 ? targetAngle : normAngle(piece.transform.rotation + delta);
        const newT: Transform = { ...piece.transform, rotation: newRot };
        const entry = getCatalogEntryForPiece(piece);
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
    sync(true);
  }

  selBarAngleInput.addEventListener("change", applyAngleInput);
  selBarAngleInput.addEventListener("keydown", (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      applyAngleInput();
      selBarAngleInput.blur();
    } else if (e.key === "Escape") {
      updateSelectionFloatingBar();
      selBarAngleInput.blur();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.shiftKey ? 1 : 7.5;
      const cur = parseFloat(selBarAngleInput.value) || 0;
      selBarAngleInput.value = (Math.round((cur + step) * 10) / 10).toString();
      applyAngleInput();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const step = e.shiftKey ? 1 : 7.5;
      const cur = parseFloat(selBarAngleInput.value) || 0;
      selBarAngleInput.value = (Math.round((cur - step) * 10) / 10).toString();
      applyAngleInput();
    }
  });

  selBarLayerSelect.addEventListener("change", () => {
    if (selBarLayerSelect.value) {
      moveSelectedToLayer(selBarLayerSelect.value, { start: 0, end: 0 });
    }
  });

  selBarBtnHeight.addEventListener("click", () => {
    openHeightDialogForSelection();
  });

  selBarBtnFeeder.addEventListener("click", () => {
    toggleSelectedFeeder();
  });

  selBarBtnIsolator.addEventListener("click", () => {
    toggleSelectedIsolator();
  });

  selBarBtnCatenary.addEventListener("click", () => {
    toggleSelectedCatenary();
  });

  selBarBtnCatenaryFeed.addEventListener("click", () => {
    toggleSelectedCatenaryFeeder();
  });

  selBarBtnCatenaryIso.addEventListener("click", () => {
    toggleSelectedCatenaryIsolator();
  });

  selBarBtnDecoder.addEventListener("click", () => {
    toggleSelectedTurnoutDecoder();
  });

  selBarBtnLantern.addEventListener("click", () => {
    toggleSelectedTurnoutLantern();
  });

  selBarBtnDelete.addEventListener("click", () => {
    deleteSelected();
  });

  function updateSelectionFloatingBar(): void {
    const selectedIds = selection.all();
    if (selectedIds.length === 0) {
      selectionFloatingBar.classList.add("hidden");
      return;
    }
    selectionFloatingBar.classList.remove("hidden");

    const layers = layout.layers || [];
    const selectedPieces = layout.pieces.filter((p) => selectedIds.includes(p.id));
    const firstPiece = selectedPieces[0];
    const firstModel = layout.models?.find((m) => m.id === selectedIds[0]);

    if (firstModel && selectedPieces.length === 0) {
      selBarCount.textContent = `📦 ${firstModel.name} · ${firstModel.width}×${firstModel.depth} mm (H: ${firstModel.height}mm)`;
      selBarElevGroup.style.display = "none";
      selBarBtnHeight.style.display = "none";
      selBarBtnFeeder.style.display = "none";
      selBarBtnIsolator.style.display = "none";
      selBarBtnCatenary.style.display = "none";
      selBarBtnCatenaryFeed.style.display = "none";
      selBarBtnCatenaryIso.style.display = "none";
      selBarBtnDecoder.style.display = "none";
      selBarBtnLantern.style.display = "none";

      const mLayer = layers.find((l) => l.id === firstModel.layerId) || layers[0];
      selBarLayerSelect.innerHTML = layers
        .map((l) => `<option value="${l.id}" ${l.id === mLayer?.id ? "selected" : ""}>${l.name}</option>`)
        .join("");
      if (document.activeElement !== selBarAngleInput) {
        selBarAngleInput.value = (Math.round(firstModel.transform.rotation * 100) / 100).toString();
      }
      return;
    }

    selBarBtnHeight.style.display = "";
    selBarBtnFeeder.style.display = "";
    selBarBtnIsolator.style.display = "";
    selBarBtnCatenary.style.display = "";
    selBarBtnCatenaryFeed.style.display = "";
    selBarBtnCatenaryIso.style.display = "";
    const tSel = getTranslations().selectionBar;

    // Track Feeder (Einspeisung)
    const allHaveFeeder = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.powerFeed);
    selBarBtnFeeder.classList.toggle("active", allHaveFeeder);

    // Track Isolator
    if (firstPiece?.isIsolator) {
      const idx = (firstPiece.isolatedConnectorIndex ?? 0) + 1;
      const total = firstPiece.connectors?.length || 2;
      selBarBtnIsolator.classList.add("active");
      selBarBtnIsolator.textContent = tSel.isolatorActive.replace("{idx}", String(idx)).replace("{total}", String(total));
      selBarBtnIsolator.title = tSel.isolatorActiveTitle.replace("{idx}", String(idx)).replace("{total}", String(total));
    } else {
      selBarBtnIsolator.classList.remove("active");
      selBarBtnIsolator.textContent = tSel.isolatorInactive;
      selBarBtnIsolator.title = tSel.isolatorInactiveTitle;
    }

    // Oberleitung (Catenary)
    const allHaveCatenary = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.hasCatenary);
    if (allHaveCatenary) {
      selBarBtnCatenary.classList.add("active");
      selBarBtnCatenary.textContent = tSel.catenaryActive;
      selBarBtnCatenary.title = tSel.catenaryActiveTitle;
    } else {
      selBarBtnCatenary.classList.remove("active");
      selBarBtnCatenary.textContent = tSel.catenaryBtn;
      selBarBtnCatenary.title = tSel.catenaryInactiveTitle;
    }

    const allHaveCatenaryFeed = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.catenaryPowerFeed);
    if (allHaveCatenaryFeed) {
      selBarBtnCatenaryFeed.classList.add("active");
      selBarBtnCatenaryFeed.textContent = tSel.catenaryFeederActive;
      selBarBtnCatenaryFeed.title = tSel.catenaryFeederActiveTitle;
    } else {
      selBarBtnCatenaryFeed.classList.remove("active");
      selBarBtnCatenaryFeed.textContent = tSel.catenaryFeederBtn;
      selBarBtnCatenaryFeed.title = tSel.catenaryFeederInactiveTitle;
    }

    if (firstPiece?.catenaryIsolator) {
      const idx = (firstPiece.catenaryIsolatedConnectorIndex ?? 0) + 1;
      const total = firstPiece.connectors?.length || 2;
      selBarBtnCatenaryIso.classList.add("active");
      selBarBtnCatenaryIso.textContent = tSel.catenaryIsolatorActive.replace("{idx}", String(idx)).replace("{total}", String(total));
      selBarBtnCatenaryIso.title = tSel.catenaryIsolatorActiveTitle.replace("{idx}", String(idx)).replace("{total}", String(total));
    } else {
      selBarBtnCatenaryIso.classList.remove("active");
      selBarBtnCatenaryIso.textContent = tSel.catenaryIsolatorBtn;
      selBarBtnCatenaryIso.title = tSel.catenaryIsolatorInactiveTitle;
    }

    // Weichen-Optionen (Decoder & Laterne)
    const turnoutPieces = selectedPieces.filter((p) => {
      const entry = getCatalogEntryForPiece(p);
      return isTurnoutEntry(entry);
    });

    if (turnoutPieces.length > 0) {
      selBarBtnDecoder.style.display = "";
      selBarBtnLantern.style.display = "";

      const allHaveDecoder = turnoutPieces.every((p) => !!p.hasDecoder);
      if (allHaveDecoder) {
        selBarBtnDecoder.classList.add("active");
        selBarBtnDecoder.textContent = tSel.decoderActive;
        selBarBtnDecoder.title = tSel.decoderActiveTitle;
      } else {
        selBarBtnDecoder.classList.remove("active");
        selBarBtnDecoder.textContent = tSel.decoderBtn;
        selBarBtnDecoder.title = tSel.decoderInactiveTitle;
      }

      const allHaveLantern = turnoutPieces.every((p) => !!p.hasLantern);
      if (allHaveLantern) {
        selBarBtnLantern.classList.add("active");
        selBarBtnLantern.textContent = tSel.lanternActive;
        selBarBtnLantern.title = tSel.lanternActiveTitle;
      } else {
        selBarBtnLantern.classList.remove("active");
        selBarBtnLantern.textContent = tSel.lanternBtn;
        selBarBtnLantern.title = tSel.lanternInactiveTitle;
      }
    } else {
      selBarBtnDecoder.style.display = "none";
      selBarBtnLantern.style.display = "none";
    }

    const pieceWord = selectedIds.length === 1 ? tSel.pieceSingular : tSel.piecePlural;
    selBarCount.textContent = `${selectedIds.length} ${pieceWord}`;

    const firstLayerId = firstPiece?.layerId || layout.activeLayerId || layers[0]?.id;
    const allSameLayer = selectedPieces.every((p) => (p.layerId || "layer-1") === (firstPiece?.layerId || "layer-1"));

    selBarLayerSelect.innerHTML = layers
      .map((l) => `<option value="${l.id}" ${allSameLayer && l.id === firstLayerId ? "selected" : ""}>${l.name}</option>`)
      .join("");
    if (!allSameLayer) {
      selBarLayerSelect.insertAdjacentHTML("afterbegin", `<option value="" selected disabled>${tSel.multipleLayers}</option>`);
    }

    const curLayer = layers.find((l) => l.id === firstLayerId) || layers[0];
    const baseH = curLayer?.heightMm ?? 0;

    let minH = Infinity;
    let maxH = -Infinity;
    let anyRamp = false;
    for (const p of selectedPieces) {
      const pLayer = layers.find((l) => l.id === (p.layerId || "layer-1")) || curLayer;
      const bH = pLayer?.heightMm ?? 0;
      const s = bH + (p.elevation?.start ?? 0);
      const e = bH + (p.elevation?.end ?? 0);
      if (Math.abs(s - e) > 0.1) anyRamp = true;
      if (s < minH) minH = s;
      if (e < minH) minH = e;
      if (s > maxH) maxH = s;
      if (e > maxH) maxH = e;
    }
    if (minH === Infinity) { minH = baseH; maxH = baseH; }

    const roundH = (v: number): number => (Math.abs(v) < 0.1 ? 0 : Math.round(v * 10) / 10);
    const minHStr = roundH(minH) > 0 ? `+${roundH(minH)}` : `${roundH(minH)}`;
    const maxHStr = roundH(maxH) > 0 ? `+${roundH(maxH)}` : `${roundH(maxH)}`;

    
    const activeAngle = firstPiece ? firstPiece.transform.rotation : 0;
    if (document.activeElement !== selBarAngleInput) {
      selBarAngleInput.value = (Math.round(activeAngle * 100) / 100).toString();
    }
  }

  // Floating Layers Menu initialisieren
  const rightSidebarHost = document.getElementById("right-sidebar")!;
  rightSidebar = new RightSidebar(rightSidebarHost, layout, {
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
    onMoveSelectedToLayer: (layerId, elevation) => {
      moveSelectedToLayer(layerId, elevation);
    },
    onOpenHeightDialog: (preselectedLayerId) => {
      openHeightDialogForSelection(preselectedLayerId);
    },
    onSelectActiveLayer: (layerId) => {
      layout.activeLayerId = layerId;
      const toKeep = selection.all().filter((id) => {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) return (p.layerId || "layer-1") === layerId;
        const m = layout.models?.find((x) => x.id === id);
        if (m) return (m.layerId || "layer-1") === layerId;
        return false;
      });
      selection.setMany(toKeep);
      sync(true);
    },
    onToggleWoodPlate: () => {
      const active = !viewer3D.isWoodPlateEnabled();
      viewer3D.setWoodPlate(active);
      const isDe = getLanguage() === "de";
      showToast(
        active
          ? (isDe ? "Woodplatte (Trassenbrett) platziert" : "Wood plate placed under tracks & roads")
          : (isDe ? "Woodplatte entfernt" : "Wood plate removed"),
        "🪵"
      );
      sync();
    },
    isWoodPlateActive: () => viewer3D.isWoodPlateEnabled(),
    onClose: () => {
      toggleRightSidebar();
    },
    selectedPieceIds: () => selection.all(),
  });

  // Toolbar initialisieren
  const toolbarHost = document.getElementById("toolbar")!;
  const toolbarEvents: ToolbarEvents = {
    onToggleRightSidebar: () => toggleRightSidebar(),
    onNewPlan: () => {
      if (confirm(getTranslations().dialogs.confirmNewPlan)) {
        layout.pieces = [];
        ensureDefaultLayers(layout);
        selection.clear();
        planManager.setPlanInfo("New Layout.railbench", null, "railbench");
        sync(true);
      }
    },
    onUndo: () => {
      history.undo();
      sync(true);
    },
    onRedo: () => {
      history.redo();
      sync(true);
    },
    onCopy: () => copySelected(),
    onPaste: () => pasteClipboard(),
    onDuplicate: () => duplicateSelected(),
    onSelectAll: () => selectAll(),
    onDeselectAll: () => {
      selection.clear();
      sync();
    },
    onDeleteSelected: () => deleteSelected(),
    onRotateSelected: (step = 7.5) => rotateSelected(step),
    onToggleSnapping: () => {
      snappingEnabled = !snappingEnabled;
      sync();
      saveSettingsState();
    },
    onToggleColorCode: () => {
      cycleColorMode();
      saveSettingsState();
    },
    onSetColorMode: (mode: ColorMode) => {
      cycleColorMode(mode);
      saveSettingsState();
    },
    onToggleTrackLabels: () => {
      trackLabelsEnabled = !trackLabelsEnabled;
      sync();
      saveSettingsState();
    },
    onTogglePowerSim: () => {
      powerSimEnabled = !powerSimEnabled;
      sync();
      saveSettingsState();
    },
    onToggleFeeder: () => toggleSelectedFeeder(),
    onToggleIsolator: () => toggleSelectedIsolator(),
    onToggleCatenary: () => toggleSelectedCatenary(),
    onToggleCatenaryFeeder: () => toggleSelectedCatenaryFeeder(),
    onToggleCatenaryIsolator: () => toggleSelectedCatenaryIsolator(),
    onToggleTurnoutLantern: () => toggleSelectedTurnoutLantern(),
    onToggleTurnoutDecoder: () => toggleSelectedTurnoutDecoder(),
    onSetFeedVoltage: (v: number) => {
      layout.feedVoltage = v;
      if (!powerSimEnabled) powerSimEnabled = true;
      sync(true);
    },
    onSetLocoCurrent: (ma: number) => {
      layout.locoCurrentMa = Math.min(1000, Math.max(50, ma));
      if (!powerSimEnabled) powerSimEnabled = true;
      sync(true);
    },
    onOpenPowerSettings: (initialFocus) => {
      openPowerSettingsDialog({
        layout,
        catalog,
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
              const entry = getCatalogEntryForPiece(p);
              if (isTurnoutEntry(entry)) {
                p.hasLantern = true;
                p.hasDecoder = true;
              }
            }
          } else if (bulkTurnoutAction === "clearAll") {
            for (const p of layout.pieces) {
              const entry = getCatalogEntryForPiece(p);
              if (isTurnoutEntry(entry)) {
                p.hasLantern = false;
                p.hasDecoder = false;
              }
            }
          }

          if (!powerSimEnabled) powerSimEnabled = true;
          sync(true);
        },
      });
    },
    onToggleRulers: () => {
      ruler.toggle();
      sync();
      saveSettingsState();
    },
    onToggleMeasureTool: () => toggleMeasureTool(),
    onToggleWoodPlate: () => {
      const active = !viewer3D.isWoodPlateEnabled();
      viewer3D.setWoodPlate(active);
      const isDe = getLanguage() === "de";
      showToast(
        active
          ? (isDe ? "Woodplatte (Trassenbrett) platziert" : "Wood plate placed under tracks & roads")
          : (isDe ? "Woodplatte entfernt" : "Wood plate removed"),
        "🪵"
      );
      sync();
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
      sync(true);
    },
    onConfigureBaseboard: () => {
      sidebar.setActiveTab("board");
    },
    onToggle3D: () => toggle3D(),
    onImportModel: () => {
      sidebar.setActiveTab("models");
    },
    onToggleTheme: () => {
      themeManager.cycle();
    },
    onSetTheme: (theme) => {
      themeManager.setPreference(theme);
    },
    onOpenPlan: () => triggerOpenPlan(),
    onSavePlan: () => triggerSavePlan(),
    onSavePlanAs: (format?: PlanFormat) => triggerSavePlanAs(format ?? "railbench"),
    //onSaveJSON: () => triggerSavePlanAs("json"),
    onSaveXML: () => triggerSavePlanAs("xml"),
    onExportCSV: () => triggerExportCSV(),
    onPrintPlan: () => openPrintDialog(layout, catalog, renderer),
    onMoveSelectedToLayerPrompt: () => openHeightDialogForSelection(),
    onOpenHelp: (tab) => openHelpDialog((tab as HelpTab) || "overview"),
    onResetView: () => {
      renderer.resetView();
      ruler.update();
      sync();
    },
    onOpenCollab: () => triggerCollabDialog(),
    onTogglePerfMonitor: () => perfMonitor.toggle(),
    onFocusShortCircuit: () => {
      if (currentShortCircuit?.shortCircuitPoints.length) {
        const pt = currentShortCircuit.shortCircuitPoints[0]!;
        renderer.centerOn(pt.x, pt.y);
        ruler.update();
        if (is3DMode) {
          viewer3D.focusOnPoint(pt.x, pt.y);
        }
        if (currentShortCircuit.shortCircuitPieces.size > 0) {
          selection.setMany(Array.from(currentShortCircuit.shortCircuitPieces));
        }
        sync();
      }
    },
  };

  createToolbar(toolbarHost, toolbarEvents);

  themeManager.onChange((resolved, pref) => {
    renderer.setTheme(resolved);
    ruler.setTheme(resolved);
    viewer3D.setTheme(resolved);
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

  onLanguageChange(() => {
    createToolbar(toolbarHost, toolbarEvents);
    updateToolbarState({
      themePref: themeManager.getPreference(),
      resolvedTheme: themeManager.getResolvedTheme(),
    });
    updateSelectionFloatingBarStatic();
    updateMeasureHud();
    sync(false);
  });

  // Native Electron Menü-Aktionen empfangen
  if (window.electronAPI) {
    window.electronAPI.onMenuAction((action) => {
      switch (action) {
        case "newPlan":
          if (confirm(getTranslations().dialogs.confirmNewPlan)) {
            layout.pieces = [];
            selection.clear();
            planManager.setPlanInfo("New Layout.railbench", null, "railbench");
            sync(true);
          }
          break;
        case "openPlan": triggerOpenPlan(); break;
        case "savePlan": triggerSavePlan(); break;
        case "savePlanAs": triggerSavePlanAs("railbench"); break;
        case "savePlanAsJson": triggerSavePlanAs("json"); break;
        case "savePlanAsXml": triggerSavePlanAs("xml"); break;
        case "exportCSV": triggerExportCSV(); break;
        case "copy": copySelected(); break;
        case "paste": pasteClipboard(); break;
        case "duplicate": duplicateSelected(); break;
        case "selectAll": selectAll(); break;
        case "undo": history.undo(); sync(true); break;
        case "redo": history.redo(); sync(true); break;
        case "rotate": rotateSelected(7.5); break;
        case "delete": deleteSelected(); break;
        case "toggleFeeder": toggleSelectedFeeder(); break;
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

  // --- DRAG & DROP AUS DEM KATALOG AUF DIE CANVAS ---
  const canvas = renderer.app.canvas;

  canvas.addEventListener("dragover", (e) => {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "copy";
    }
  });

  canvas.addEventListener("drop", async (e) => {
    e.preventDefault();
    const rawTrack = e.dataTransfer?.getData("application/json");
    const rawModel = e.dataTransfer?.getData("application/railbench-model");
    const worldPos = renderer.screenToWorld(e.clientX, e.clientY);

    if (rawModel) {
      try {
        const model = JSON.parse(rawModel) as Model3D;
        addModelToLayout(model, worldPos);
        return;
      } catch (err) {
        console.error("Drop Model Fehler:", err);
      }
    }

    if (rawTrack) {
      try {
        const entry = JSON.parse(rawTrack) as CatalogEntry;
        addPieceToLayout(entry, worldPos);
        sync(true);
        return;
      } catch (err) {
        console.error("Drop Track Fehler:", err);
      }
    }

    if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0]) {
      const file = e.dataTransfer.files[0];
      if (file.name.toLowerCase().endsWith(".glb") || file.name.toLowerCase().endsWith(".gltf")) {
        try {
          const model = await parseGlbOrGltfFile(file);
          addModelToLayout(model, worldPos);
        } catch (err: any) {
          alert(`Error loading dropped 3D model: ${err.message || err}`);
        }
      }
    }
  });

  // --- INTERAKTIVE AUSWAHL, VERSCHIEBEN & DREHEN AUF DER CANVAS ---
  let isDraggingPiece = false;
  let isRotatingPiece = false;
  let isRotatingModel = false;
  let isBoxSelecting = false;
  let boxStartWorld = { x: 0, y: 0 };
  let initialBoxSelection = new Set<string>();
  let activePiece: TrackPiece | null = null;
  let activeModel: Model3D | null = null;
  let dragStartWorld = { x: 0, y: 0 };
  let originalTransform: Transform = { x: 0, y: 0, rotation: 0 };
  let draggedPiecesMap = new Map<string, Transform>();
  let draggedInitialConnections: Array<{ pieceId: string; connectorIndex: number }> = [];
  let pieceCenterWorld = { x: 0, y: 0 };
  let pendingSingleSelect: string | null = null;
  let hasDragMoved = false;

  canvas.addEventListener("contextmenu", (e) => {
    e.preventDefault();
  });

  canvas.addEventListener("pointerdown", (e) => {
    if (isMeasureToolActive) {
      if (e.button === 0) {
        const wPos = renderer.screenToWorld(e.clientX, e.clientY);
        const forceNoSnap = e.altKey;

        if (measureStart && Math.hypot(wPos.x - measureStart.x, wPos.y - measureStart.y) <= 15) {
          measureDraggingPoint = "A";
          canvas.style.cursor = "grabbing";
          return;
        }
        if (measureCurrent && Math.hypot(wPos.x - measureCurrent.x, wPos.y - measureCurrent.y) <= 15) {
          measureDraggingPoint = "B";
          canvas.style.cursor = "grabbing";
          return;
        }

        const snap = findMeasureSnapPoint(wPos, 20, forceNoSnap);
        const pt = snap ? { ...snap } : { ...wPos };

        if (!measureStart || measureLocked) {
          measureStart = pt;
          measureCurrent = pt;
          measureLocked = false;
        } else {
          measureCurrent = pt;
          measureLocked = true;
        }
        updateMeasureHud();
        sync();
      } else if (e.button === 2) {
        if (measureStart) {
          measureStart = null;
          measureCurrent = null;
          measureLocked = false;
          measureDraggingPoint = null;
          updateMeasureHud();
          sync();
        } else {
          toggleMeasureTool(false);
        }
      }
      return;
    }

    if (e.button !== 0) return;

    const wPos = renderer.screenToWorld(e.clientX, e.clientY);
    const isMultiSelect = e.ctrlKey || e.metaKey || e.shiftKey;
    hasDragMoved = false;
    pendingSingleSelect = null;

    const selectedIds = selection.all();
    const scale = renderer.world.scale.x || 1;
    const handleHitRadius = Math.max(24, 20 / scale);

    if (selectedIds.length > 0 && !isMultiSelect) {
      let clickedHandle = false;
      for (const selId of selectedIds) {
        const selPiece = layout.pieces.find((p) => p.id === selId);
        if (selPiece && isPieceInteractive(selPiece)) {
          const entry = getCatalogEntryForPiece(selPiece);
          if (entry) {
            const handlePos = getPieceRotationHandleWorld(entry, selPiece.transform);
            if (dist(wPos, handlePos) <= handleHitRadius) {
              clickedHandle = true;
              break;
            }
          }
        }
        const selModel = layout.models?.find((m) => m.id === selId);
        if (selModel && isModelInteractive(selModel)) {
          const handlePos = getModelRotationHandleWorld(selModel);
          if (dist(wPos, handlePos) <= handleHitRadius) {
            clickedHandle = true;
            break;
          }
        }
      }

      if (clickedHandle) {
        isRotatingPiece = true;
        isRotatingModel = false;
        activePiece = null;
        activeModel = null;
        dragStartWorld = { ...wPos };

        draggedPiecesMap.clear();
        const rotatableItems: RotatableItem[] = [];
        for (const id of selectedIds) {
          const p = layout.pieces.find((x) => x.id === id);
          if (p && isPieceInteractive(p)) {
            draggedPiecesMap.set(p.id, { ...p.transform });
            const pEntry = getCatalogEntryForPiece(p);
            rotatableItems.push({ id: p.id, type: "piece", transform: { ...p.transform }, entry: pEntry });
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m && isModelInteractive(m)) {
            draggedPiecesMap.set(m.id, { ...m.transform });
            rotatableItems.push({ id: m.id, type: "model", transform: { ...m.transform } });
          }
        }

        pieceCenterWorld = computeGroupCentroid(rotatableItems);
        canvas.style.cursor = "grabbing";
        return;
      }
    }

    let hitModel: Model3D | null = null;
    if (layout.models) {
      for (const m of layout.models) {
        if (!isModelInteractive(m)) continue;
        if (isPointInsideModel(wPos, m)) {
          hitModel = m;
          break;
        }
      }
    }

    if (hitModel) {
      if (isMultiSelect) {
        selection.toggle(hitModel.id);
        if (!selection.has(hitModel.id)) {
          sync();
          return;
        }
      } else {
        if (selection.has(hitModel.id) && selection.all().length > 1) {
          pendingSingleSelect = hitModel.id;
        } else {
          selection.set(hitModel.id);
        }
      }
      isDraggingPiece = true;
      activePiece = null;
      activeModel = hitModel;
      dragStartWorld = { ...wPos };
      originalTransform = { ...hitModel.transform };

      draggedPiecesMap.clear();
      for (const id of selection.all()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p && isPieceInteractive(p)) draggedPiecesMap.set(p.id, { ...p.transform });
        const m = layout.models?.find((x) => x.id === id);
        if (m && isModelInteractive(m)) draggedPiecesMap.set(m.id, { ...m.transform });
      }

      draggedInitialConnections = [];
      for (const [id] of draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          for (const c of p.connectors) {
            if (c.connectedTo && !draggedPiecesMap.has(c.connectedTo.pieceId)) {
              draggedInitialConnections.push({ pieceId: c.connectedTo.pieceId, connectorIndex: c.connectedTo.connectorIndex });
            }
          }
        }
      }

      canvas.style.cursor = "move";
      sync();
      return;
    }

    let hit: TrackPiece | null = null;
    const hitGauge = catalog.gauge ?? 16.5;
    let minD = Infinity;

    for (const piece of layout.pieces) {
      if (!isPieceInteractive(piece)) continue;
      const entry = getCatalogEntryForPiece(piece);
      if (!entry) continue;

      const screenGrabBuffer = 20 / scale;
      const maxAllowedDist = entry.isRoad
        ? Math.max(35, (entry.roadWidth ?? 60) * 0.65, screenGrabBuffer)
        : Math.max(32, hitGauge * 1.8, screenGrabBuffer);

      const d = distanceToTrackPiece(wPos, piece.transform, entry);
      if (d < maxAllowedDist && d < minD) {
        minD = d;
        hit = piece;
      }
    }

    if (hit) {
      if (isMultiSelect) {
        selection.toggle(hit.id);
        if (!selection.has(hit.id)) {
          sync();
          return;
        }
      } else {
        if (selection.has(hit.id) && selection.all().length > 1) {
          pendingSingleSelect = hit.id;
        } else {
          selection.set(hit.id);
        }
      }

      isDraggingPiece = true;
      activePiece = hit;
      activeModel = null;
      dragStartWorld = { ...wPos };
      originalTransform = { ...hit.transform };

      draggedPiecesMap.clear();
      for (const id of selection.all()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p && isPieceInteractive(p)) draggedPiecesMap.set(p.id, { ...p.transform });
        const m = layout.models?.find((x) => x.id === id);
        if (m && isModelInteractive(m)) draggedPiecesMap.set(m.id, { ...m.transform });
      }

      draggedInitialConnections = [];
      for (const [id] of draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          for (const c of p.connectors) {
            if (c.connectedTo && !draggedPiecesMap.has(c.connectedTo.pieceId)) {
              draggedInitialConnections.push({ pieceId: c.connectedTo.pieceId, connectorIndex: c.connectedTo.connectorIndex });
            }
          }
        }
      }

      canvas.style.cursor = "move";
      sync();
    } else {
      if (sidebar?.isBaseboardTabActive()) {
        const baseboards = getBaseboards(layout);
        let hitBoard: BaseboardConfig | null = null;

        for (let i = baseboards.length - 1; i >= 0; i--) {
          const b = baseboards[i]!;
          if (!b.enabled) continue;
          if (
            isPointInBaseboardHeader(wPos, b) ||
            isPointNearBaseboardEdge(wPos, b, 24) ||
            isPointInsideBaseboard(wPos, b)
          ) {
            hitBoard = b;
            break;
          }
        }

        if (hitBoard) {
          isDraggingBaseboard = true;
          activeBaseboard = hitBoard;
          sidebar?.setActiveBoardId(hitBoard.id);
          baseboardDragStartWorld = { ...wPos };
          const bBounds = getBaseboardBounds(hitBoard);
          baseboardOriginalPos = {
            positionMode: "custom",
            customX: bBounds.x,
            customY: bBounds.y,
          };

          baseboardInitialTrackTransforms.clear();
          for (const piece of layout.pieces) {
            const entry = getCatalogEntryForPiece(piece);
            if (entry) {
              const center = getPieceCenterWorld(entry, piece.transform);
              if (isPointInsideBaseboard(center, hitBoard)) {
                baseboardInitialTrackTransforms.set(piece.id, { ...piece.transform });
              }
            }
          }

          selection.clear();
          canvas.style.cursor = "grabbing";
          sync();
          return;
        }
      }

      isBoxSelecting = true;
      boxStartWorld = { ...wPos };
      currentSelectionBox = { start: { ...wPos }, end: { ...wPos } };
      initialBoxSelection = new Set(isMultiSelect ? selection.all() : []);
      if (!isMultiSelect) {
        selection.clear();
      }
      canvas.style.cursor = "crosshair";
      sync();
    }
  });

  window.addEventListener("pointermove", (e) => {
    const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);
    collabClient.broadcastCursor(currentWPos.x, currentWPos.y);

    if (isMeasureToolActive) {
      const forceNoSnap = e.altKey;
      const snap = findMeasureSnapPoint(currentWPos, 20, forceNoSnap);
      measureSnapPoint = snap;

      if (measureDraggingPoint === "A") {
        measureStart = snap ? { ...snap } : { ...currentWPos };
        canvas.style.cursor = "grabbing";
        updateMeasureHud();
      } else if (measureDraggingPoint === "B") {
        measureCurrent = snap ? { ...snap } : { ...currentWPos };
        canvas.style.cursor = "grabbing";
        updateMeasureHud();
      } else if (measureStart && !measureLocked) {
        measureCurrent = snap ? { ...snap } : { ...currentWPos };
        canvas.style.cursor = "crosshair";
        updateMeasureHud();
      } else {
        const nearA = measureStart && Math.hypot(currentWPos.x - measureStart.x, currentWPos.y - measureStart.y) <= 15;
        const nearB = measureCurrent && Math.hypot(currentWPos.x - measureCurrent.x, currentWPos.y - measureCurrent.y) <= 15;
        canvas.style.cursor = nearA || nearB ? "grab" : "crosshair";
      }

      sync();
      return;
    }

    if (isBoxSelecting) {
      currentSelectionBox = { start: boxStartWorld, end: currentWPos };
      const distBox = Math.hypot(currentWPos.x - boxStartWorld.x, currentWPos.y - boxStartWorld.y);
      if (distBox > 3) {
        const newlyFound = new Set<string>(initialBoxSelection);
        for (const piece of layout.pieces) {
          if (!isPieceInteractive(piece)) continue;
          const entry = getCatalogEntryForPiece(piece);
          if (entry && isPieceInSelectionBox(piece, entry, boxStartWorld, currentWPos)) {
            newlyFound.add(piece.id);
          }
        }
        if (layout.models) {
          const minX = Math.min(boxStartWorld.x, currentWPos.x);
          const maxX = Math.max(boxStartWorld.x, currentWPos.x);
          const minY = Math.min(boxStartWorld.y, currentWPos.y);
          const maxY = Math.max(boxStartWorld.y, currentWPos.y);
          for (const model of layout.models) {
            if (!isModelInteractive(model)) continue;
            if (model.transform.x >= minX && model.transform.x <= maxX && model.transform.y >= minY && model.transform.y <= maxY) {
              newlyFound.add(model.id);
            }
          }
        }
        selection.setMany([...newlyFound]);
      }
      sync();
      return;
    }

    if (isRotatingModel && activeModel) {
      const dx = currentWPos.x - activeModel.transform.x;
      const dy = currentWPos.y - activeModel.transform.y;
      const pointerAngleDeg = rad2deg(Math.atan2(dy, dx));
      let newRot = normAngle(pointerAngleDeg + 90);
      if (!e.altKey) {
        newRot = Math.round(newRot / 7.5) * 7.5;
      }
      activeModel.transform.rotation = newRot;
      sync();
      return;
    }

    if (isRotatingPiece && draggedPiecesMap.size > 0) {
      const startAngle = Math.atan2(dragStartWorld.y - pieceCenterWorld.y, dragStartWorld.x - pieceCenterWorld.x);
      const currentAngle = Math.atan2(currentWPos.y - pieceCenterWorld.y, currentWPos.x - pieceCenterWorld.x);
      let deltaDeg = rad2deg(currentAngle - startAngle);

      if (!e.altKey) {
        deltaDeg = Math.round(deltaDeg / 7.5) * 7.5;
      }

      const rotItems: RotatableItem[] = [];
      for (const [id, origT] of draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          const pEntry = getCatalogEntryForPiece(p);
          rotItems.push({ id, type: "piece", transform: origT, entry: pEntry });
        }
        const m = layout.models?.find((x) => x.id === id);
        if (m) {
          rotItems.push({ id, type: "model", transform: origT });
        }
      }

      const newTransforms = rotateGroupAroundCentroid(rotItems, deltaDeg, pieceCenterWorld);

      if (snappingEnabled && !e.altKey) {
        const otherPieces = layout.pieces.filter((p) => !draggedPiecesMap.has(p.id));
        let bestSnap: { itemSnap: BestPieceSnap; origT: Transform } | null = null;
        let bestDist = Infinity;

        for (const [id] of draggedPiecesMap.entries()) {
          const p = layout.pieces.find((x) => x.id === id);
          if (!p) continue;
          const pEntry = getCatalogEntryForPiece(p);
          if (!pEntry) continue;
          const candT = newTransforms.get(id);
          if (!candT) continue;

          const snap = findBestSnapForPiece(pEntry, candT, otherPieces, undefined, 35, 30);
          if (snap && snap.dist < bestDist) {
            bestDist = snap.dist;
            bestSnap = { itemSnap: snap, origT: draggedPiecesMap.get(id)! };
          }
        }

        if (bestSnap) {
          const { itemSnap, origT } = bestSnap;
          const snapDeltaRot = normAngle(itemSnap.snappedTransform.rotation - origT.rotation);
          for (const [id, oT] of draggedPiecesMap.entries()) {
            const relOffset = { x: oT.x - origT.x, y: oT.y - origT.y };
            const rotOffset = rotate(relOffset, snapDeltaRot);
            const snappedPos: Transform = {
              x: itemSnap.snappedTransform.x + rotOffset.x,
              y: itemSnap.snappedTransform.y + rotOffset.y,
              rotation: normAngle(oT.rotation + snapDeltaRot),
            };
            const p = layout.pieces.find((x) => x.id === id);
            if (p) {
              p.transform = snappedPos;
              const pEntry = getCatalogEntryForPiece(p);
              if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
            }
            const m = layout.models?.find((x) => x.id === id);
            if (m) m.transform = snappedPos;
          }
          sync();
          return;
        }
      }

      for (const [id, newT] of newTransforms.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p) {
          p.transform = { ...newT };
          const pEntry = getCatalogEntryForPiece(p);
          if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
        }
        const m = layout.models?.find((x) => x.id === id);
        if (m) m.transform = { ...newT };
      }

      sync();
      return;
    }

    if (isDraggingBaseboard && activeBaseboard) {
      const dx = currentWPos.x - baseboardDragStartWorld.x;
      const dy = currentWPos.y - baseboardDragStartWorld.y;

      let candidateX = baseboardOriginalPos.customX + dx;
      let candidateY = baseboardOriginalPos.customY + dy;

      const otherBoards = getBaseboards(layout).filter((b) => b.id !== activeBaseboard!.id && b.enabled !== false);

      if (!e.altKey && otherBoards.length > 0) {
        const snap = snapBaseboard(activeBaseboard, candidateX, candidateY, otherBoards, 35);
        candidateX = snap.x;
        candidateY = snap.y;
        activeBaseboardSnapGuides = snap.guides;
      } else {
        activeBaseboardSnapGuides = [];
        if (!e.altKey) {
          candidateX = Math.round(candidateX / 20) * 20;
          candidateY = Math.round(candidateY / 20) * 20;
        }
      }

      activeBaseboard.positionMode = "custom";
      activeBaseboard.customX = Math.round(candidateX);
      activeBaseboard.customY = Math.round(candidateY);

      if (e.shiftKey && baseboardInitialTrackTransforms.size > 0) {
        const deltaX = activeBaseboard.customX - baseboardOriginalPos.customX;
        const deltaY = activeBaseboard.customY - baseboardOriginalPos.customY;
        for (const [pId, initT] of baseboardInitialTrackTransforms.entries()) {
          const piece = layout.pieces.find((p) => p.id === pId);
          if (piece) {
            piece.transform.x = initT.x + deltaX;
            piece.transform.y = initT.y + deltaY;
            const entry = getCatalogEntryForPiece(piece);
            if (entry) {
              piece.connectors = computeWorldConnectors(entry, piece.transform);
            }
          }
        }
      }

      sidebar?.update(layout, currentSimulation);
      sync();
      return;
    }

    if (isDraggingPiece && (activePiece || activeModel)) {
      let dx = currentWPos.x - dragStartWorld.x;
      let dy = currentWPos.y - dragStartWorld.y;
      const dragDist = Math.hypot(dx, dy);

      if (!hasDragMoved) {
        if (dragDist < 3.0) return;
        hasDragMoved = true;
        pendingSingleSelect = null;
      }

      let groupSnap: {
        snappedTransform: Transform;
        sourceId: string;
        origTransform: Transform;
      } | null = null;

      if (snappingEnabled) {
        const otherPieces = layout.pieces.filter((p) => !draggedPiecesMap.has(p.id));
        let bestDist = Infinity;
        const ignoreConn = dragDist < 28 ? draggedInitialConnections : undefined;

        for (const [id, origT] of draggedPiecesMap.entries()) {
          const piece = layout.pieces.find((p) => p.id === id);
          if (!piece) continue;
          const entry = getCatalogEntryForPiece(piece);
          if (!entry) continue;

          const candT: Transform = {
            x: origT.x + dx,
            y: origT.y + dy,
            rotation: origT.rotation,
          };

          const snap = findBestSnapForPiece(entry, candT, otherPieces, undefined, 45, 60, ignoreConn);
          if (snap && snap.dist < bestDist) {
            bestDist = snap.dist;
            groupSnap = {
              snappedTransform: snap.snappedTransform,
              sourceId: id,
              origTransform: origT,
            };
          }
        }
      }

      if (groupSnap) {
        const { snappedTransform, origTransform } = groupSnap;
        const deltaRot = normAngle(snappedTransform.rotation - origTransform.rotation);

        for (const [id, origT] of draggedPiecesMap.entries()) {
          const relOffset = { x: origT.x - origTransform.x, y: origT.y - origTransform.y };
          const rotOffset = rotate(relOffset, deltaRot);
          const newPos: Transform = {
            x: snappedTransform.x + rotOffset.x,
            y: snappedTransform.y + rotOffset.y,
            rotation: normAngle(origT.rotation + deltaRot),
          };

          const p = layout.pieces.find((x) => x.id === id);
          if (p) {
            p.transform = newPos;
            const pEntry = getCatalogEntryForPiece(p);
            if (pEntry) p.connectors = computeWorldConnectors(pEntry, p.transform);
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m) m.transform = newPos;
        }
      } else {
        for (const [id, origT] of draggedPiecesMap.entries()) {
          const p = layout.pieces.find((x) => x.id === id);
          if (p) {
            p.transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
            const pEntry = getCatalogEntryForPiece(p);
            if (pEntry) {
              p.connectors = computeWorldConnectors(pEntry, p.transform);
            }
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m) {
            m.transform = { x: origT.x + dx, y: origT.y + dy, rotation: origT.rotation };
          }
        }
      }

      sync();
      return;
    }

    if (!isDraggingPiece && !isRotatingPiece && !isRotatingModel && !isBoxSelecting && !isDraggingBaseboard) {
      const selectedIds = selection.all();
      const scale = renderer.world.scale.x || 1;
      const handleHitRadius = Math.max(24, 20 / scale);

      for (const selId of selectedIds) {
        const selPiece = layout.pieces.find((p) => p.id === selId);
        if (selPiece && isPieceInteractive(selPiece)) {
          const entry = getCatalogEntryForPiece(selPiece);
          if (entry) {
            const handlePos = getPieceRotationHandleWorld(entry, selPiece.transform);
            if (dist(currentWPos, handlePos) <= handleHitRadius) {
              canvas.style.cursor = "grab";
              return;
            }
          }
        }
        const selModel = layout.models?.find((m) => m.id === selId);
        if (selModel && isModelInteractive(selModel)) {
          const handlePos = getModelRotationHandleWorld(selModel);
          if (dist(currentWPos, handlePos) <= handleHitRadius) {
            canvas.style.cursor = "grab";
            return;
          }
        }
      }

      let isOverTrack = false;
      const screenGrabBuffer = 20 / scale;
      for (const piece of layout.pieces) {
        if (!isPieceInteractive(piece)) continue;
        const entry = getCatalogEntryForPiece(piece);
        if (entry) {
          const maxAllowed = entry.isRoad
            ? Math.max(35, (entry.roadWidth ?? 60) * 0.65, screenGrabBuffer)
            : Math.max(32, (catalog.gauge ?? 16.5) * 1.8, screenGrabBuffer);
          if (distanceToTrackPiece(currentWPos, piece.transform, entry) <= maxAllowed) {
            isOverTrack = true;
            break;
          }
        }
      }

      if (isOverTrack) {
        canvas.style.cursor = "pointer";
      } else {
        let isOverTable = false;
        if (sidebar?.isBaseboardTabActive()) {
          const baseboards = getBaseboards(layout);
          for (const b of baseboards) {
            if (!b.enabled) continue;
            if (
              isPointInBaseboardHeader(currentWPos, b) ||
              isPointNearBaseboardEdge(currentWPos, b, 24) ||
              isPointInsideBaseboard(currentWPos, b)
            ) {
              isOverTable = true;
              break;
            }
          }
        }
        canvas.style.cursor = isOverTable ? "grab" : "default";
      }
    }
  });

  window.addEventListener("pointerup", (e) => {
    if (isMeasureToolActive) {
      if (measureDraggingPoint) {
        measureDraggingPoint = null;
        measureLocked = true;
        canvas.style.cursor = "crosshair";
        updateMeasureHud();
        sync();
      } else if (measureStart && !measureLocked) {
        const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);
        const forceNoSnap = e.altKey;
        const snap = findMeasureSnapPoint(currentWPos, 20, forceNoSnap);
        const pt = snap ? { ...snap } : { ...currentWPos };
        if (Math.hypot(pt.x - measureStart.x, pt.y - measureStart.y) > 8) {
          measureCurrent = pt;
          measureLocked = true;
          updateMeasureHud();
          sync();
        }
      }
      return;
    }

    if (isBoxSelecting) {
      const currentWPos = renderer.screenToWorld(e.clientX, e.clientY);
      const distBox = Math.hypot(currentWPos.x - boxStartWorld.x, currentWPos.y - boxStartWorld.y);
      const isMultiSelect = e.ctrlKey || e.metaKey || e.shiftKey;

      if (distBox <= 5) {
        if (!isMultiSelect) {
          selection.clear();
        }
      }

      isBoxSelecting = false;
      currentSelectionBox = null;
      canvas.style.cursor = "default";
      sync();
      return;
    }

    if (isRotatingModel && activeModel) {
      const finalT = { ...activeModel.transform };
      if (finalT.rotation !== originalTransform.rotation) {
        history.run(new MoveModelCommand(activeModel.id, originalTransform, finalT));
        sync(true);
      }
      isRotatingModel = false;
      activeModel = null;
      canvas.style.cursor = "default";
      sync();
    } else if (isRotatingPiece) {
      let movedAny = false;
      for (const [id, origT] of draggedPiecesMap.entries()) {
        const p = layout.pieces.find((x) => x.id === id);
        if (p && (p.transform.x !== origT.x || p.transform.y !== origT.y || p.transform.rotation !== origT.rotation)) {
          history.run(new MovePieceCommand(p.id, origT, { ...p.transform }));
          movedAny = true;
        }
        const m = layout.models?.find((x) => x.id === id);
        if (m && (m.transform.x !== origT.x || m.transform.y !== origT.y || m.transform.rotation !== origT.rotation)) {
          history.run(new MoveModelCommand(m.id, origT, { ...m.transform }));
          movedAny = true;
        }
      }
      if (movedAny) sync(true);
      isRotatingPiece = false;
      activePiece = null;
      draggedPiecesMap.clear();
      canvas.style.cursor = "default";
      sync();
    } else if (isDraggingPiece) {
      if (!hasDragMoved && pendingSingleSelect) {
        selection.set(pendingSingleSelect);
      } else if (hasDragMoved) {
        let movedAny = false;
        for (const [id, origT] of draggedPiecesMap.entries()) {
          const p = layout.pieces.find((x) => x.id === id);
          if (p && (p.transform.x !== origT.x || p.transform.y !== origT.y || p.transform.rotation !== origT.rotation)) {
            history.run(new MovePieceCommand(p.id, origT, { ...p.transform }));
            movedAny = true;
          }
          const m = layout.models?.find((x) => x.id === id);
          if (m && (m.transform.x !== origT.x || m.transform.y !== origT.y || m.transform.rotation !== origT.rotation)) {
            history.run(new MoveModelCommand(m.id, origT, { ...m.transform }));
            movedAny = true;
          }
        }
        if (movedAny) {
          sync(true);
        }
      }
      isDraggingPiece = false;
      activePiece = null;
      activeModel = null;
      pendingSingleSelect = null;
      hasDragMoved = false;
      draggedPiecesMap.clear();
      canvas.style.cursor = "default";
      sync();
    } else if (isDraggingBaseboard && activeBaseboard) {
      const finalX = activeBaseboard.customX ?? 0;
      const finalY = activeBaseboard.customY ?? 0;
      if (finalX !== baseboardOriginalPos.customX || finalY !== baseboardOriginalPos.customY) {
        history.run(
          new MoveBaseboardCommand(
            activeBaseboard.id ?? "board-1",
            { ...baseboardOriginalPos },
            { positionMode: "custom", customX: finalX, customY: finalY },
          ),
        );
        sync(true);
      }
      isDraggingBaseboard = false;
      activeBaseboardSnapGuides = [];
      baseboardInitialTrackTransforms.clear();
      canvas.style.cursor = "default";
      sync();
    }
  });

  canvas.addEventListener(
    "wheel",
    (e) => {
      if (e.shiftKey || e.altKey) {
        const sel = selection.all()[0];
        if (sel) {
          e.preventDefault();
          e.stopPropagation();
          const step = e.deltaY < 0 ? -7.5 : 7.5;
          rotateSelected(step);
        }
      }
    },
    { passive: false },
  );

  // --- TASTATUR-SHORTCUTS ---
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

    const ctrlOrMeta = e.ctrlKey || e.metaKey;

    if (ctrlOrMeta && (e.key.toLowerCase() === "s" || e.code === "KeyS")) {
      e.preventDefault();
      if (e.shiftKey) {
        triggerSavePlanAs("railbench");
      } else {
        triggerSavePlan();
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "n" || e.code === "KeyN")) {
      e.preventDefault();
      if (confirm(getTranslations().dialogs.confirmNewPlan)) {
        layout.pieces = [];
        ensureDefaultLayers(layout);
        selection.clear();
        planManager.setPlanInfo("New Layout.railbench", null, "railbench");
        sync(true);
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "o" || e.code === "KeyO")) {
      e.preventDefault();
      triggerOpenPlan();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "e" || e.code === "KeyE")) {
      e.preventDefault();
      triggerExportCSV();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "p" || e.code === "KeyP")) {
      e.preventDefault();
      openPrintDialog(layout, catalog, renderer);
    } else if (e.key.toLowerCase() === "m" && !ctrlOrMeta) {
      e.preventDefault();
      if (selection.all().length > 0 && !isMeasureToolActive) {
        openHeightDialogForSelection();
      } else {
        toggleMeasureTool();
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "z" || e.code === "KeyZ")) {
      e.preventDefault();
      if (e.shiftKey) history.redo();
      else history.undo();
      sync(true);
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "y" || e.code === "KeyY")) {
      e.preventDefault();
      history.redo();
      sync(true);
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "c" || e.code === "KeyC")) {
      const textSelected = window.getSelection()?.toString();
      if (!textSelected || textSelected.trim().length === 0) {
        e.preventDefault();
        const ok = copySelected();
        if (!ok) {
          const isDe = getLanguage() === "de";
          showToast(isDe ? "Keine Gleise zum Kopieren markiert" : "No tracks selected to copy", "⚠️");
        }
      }
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "v" || e.code === "KeyV")) {
      e.preventDefault();
      pasteClipboard();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "d" || e.code === "KeyD")) {
      e.preventDefault();
      duplicateSelected();
    } else if (ctrlOrMeta && (e.key.toLowerCase() === "a" || e.code === "KeyA")) {
      e.preventDefault();
      selectAll();
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      deleteSelected();
    } else if (e.key.toLowerCase() === "r" || e.code === "KeyR") {
      e.preventDefault();
      const step = e.shiftKey ? -7.5 : 7.5;
      rotateSelected(step);
    } else if ((e.key.toLowerCase() === "w" || e.code === "KeyW") && !ctrlOrMeta) {
      e.preventDefault();
      const selPieces = selection.all().map((id) => layout.pieces.find((p) => p.id === id)).filter(Boolean) as TrackPiece[];
      const turnoutPieces = selPieces.filter((p) => {
        const entry = getCatalogEntryForPiece(p);
        return isTurnoutEntry(entry);
      });
      if (turnoutPieces.length > 0) {
        if (e.shiftKey) {
          toggleSelectedTurnoutDecoder();
        } else {
          toggleSelectedTurnoutLantern();
        }
      } else {
        const isDe = getLanguage() === "de";
        showToast(
          isDe
            ? "Weichen-Taste W: Bitte zuerst eine Weiche markieren (Laterne: W, Decoder: Umschalt+W)"
            : "Turnout key W: Please select a turnout first (Lantern: W, Decoder: Shift+W)",
          "💡",
        );
      }
    } else if (e.key.toLowerCase() === "f" && !ctrlOrMeta) {
      e.preventDefault();
      if (e.shiftKey) {
        toggleSelectedCatenaryFeeder();
      } else {
        toggleSelectedFeeder();
      }
    } else if (e.key.toLowerCase() === "i" && !ctrlOrMeta) {
      e.preventDefault();
      if (e.shiftKey) {
        toggleSelectedCatenaryIsolator();
      } else {
        toggleSelectedIsolator();
      }
    } else if (e.key.toLowerCase() === "o" && !ctrlOrMeta) {
      e.preventDefault();
      toggleSelectedCatenary();
    } else if (e.key.toLowerCase() === "s" && !ctrlOrMeta) {
      e.preventDefault();
      snappingEnabled = !snappingEnabled;
      sync();
    } else if (e.key.toLowerCase() === "c" && !ctrlOrMeta) {
      e.preventDefault();
      cycleColorMode();
    } else if (e.key.toLowerCase() === "n" && !ctrlOrMeta) {
      e.preventDefault();
      trackLabelsEnabled = !trackLabelsEnabled;
      sync();
    } else if (e.key.toLowerCase() === "p" && !ctrlOrMeta) {
      e.preventDefault();
      powerSimEnabled = !powerSimEnabled;
      sync();
    } else if (e.key.toLowerCase() === "b" && e.shiftKey && !ctrlOrMeta) {
      e.preventDefault();
      toggleRightSidebar();
    } else if (e.key.toLowerCase() === "l" && !ctrlOrMeta) {
      e.preventDefault();
      if (e.shiftKey) {
        toggleSelectedTurnoutLantern();
      } else {
        ruler.toggle();
        sync();
      }
    } else if (e.key.toLowerCase() === "d" && e.shiftKey && !ctrlOrMeta) {
      e.preventDefault();
      toggleSelectedTurnoutDecoder();
    } else if (e.key === "Escape" && isMeasureToolActive) {
      e.preventDefault();
      if (measureStart) {
        measureStart = null;
        measureCurrent = null;
        measureLocked = false;
        measureDraggingPoint = null;
        updateMeasureHud();
        sync();
      } else {
        toggleMeasureTool(false);
      }
    } else if (e.key === "Escape" && is3DMode) {
      e.preventDefault();
      toggle3D(false);
    } else if ((e.key === "3" || (e.key.toLowerCase() === "v" && !ctrlOrMeta))) {
      e.preventDefault();
      toggle3D();
    } else if (e.key === "F1" || (e.key === "?" && !ctrlOrMeta)) {
      e.preventDefault();
      openHelpDialog("overview");
    } else if (e.key === "F3") {
      e.preventDefault();
      perfMonitor.toggle();
    }
  });

  // Initialer Render
  sync();
}

/** Erzeugt eine kleine Start-Gleiskonfiguration (Oval / Gerade mit Weiche), damit die App sofort lebendig ist. */
function initDemoTracks(layout: Layout, catalog: Catalog) {
  ensureDefaultLayers(layout);
  if (!catalog?.entries || catalog.entries.length === 0) return;

  const g188 = catalog.entries.find((e) => e.id === "24188") || catalog.entries[0];
  const wLeft = catalog.entries.find((e) => e.id === "24611") || catalog.entries[1] || catalog.entries[0];
  const r2_30 = catalog.entries.find((e) => e.id === "24230") || catalog.entries[2] || catalog.entries[0];

  if (!g188) return;

  const p1: TrackPiece = {
    id: "demo-1",
    catalogId: g188.id,
    transform: { x: -200, y: 0, rotation: 0 },
    connectors: computeWorldConnectors(g188, { x: -200, y: 0, rotation: 0 }),
    powerFeed: true,
    layerId: "layer-1",
  };
  layout.pieces.push(p1);

  if (wLeft) {
    const p2Pos = { x: -200 + (g188.length ?? 188.3), y: 0, rotation: 0 };
    const p2: TrackPiece = {
      id: "demo-2",
      catalogId: wLeft.id,
      transform: p2Pos,
      connectors: computeWorldConnectors(wLeft, p2Pos),
      layerId: "layer-1",
    };
    layout.pieces.push(p2);
  }
}
main();