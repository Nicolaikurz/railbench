import heightPanelHtml from "./templates/height-panel.html?raw";
import type { Layout, Layer, TrackPiece } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import { getLanguage } from "@/i18n";
import {
  calculateRamp,
  calculateEndHeightFromMaxGradient,
  findConnectedTrackPath,
  type RampProfile,
} from "@/core/ramp";
import {
  generateRampPieces,
  getMinClearanceHeightForScale,
  getOpenConnectors,
  getAvailableCurveRadii,
  type RampShapeType,
} from "@/core/ramp-generator";
import { getPieceConnectorHeight } from "@/core/snapping";
import { openLayerDialog } from "./layer-dialog";

export interface HeightPanelOptions {
  layout: Layout;
  catalog: Catalog;
  initialTab?: "flat" | "ramp" | "generate";
  selectedPieceIds?: string[];
  anchorPiece?: TrackPiece;
  anchorConnectorIndex?: number;
  preselectedLayerId?: string;
  onApplyFlat?: (targetLayerId: string, customElevationMm?: number) => void;
  onApplyRamp?: (targetLayerId: string, pieceElevations: Map<string, { start: number; end: number }>) => void;
  onApplyGenerate?: (pieces: TrackPiece[], newLayer?: Layer) => void;
  onPreviewRamp?: (pieceElevations: Map<string, { start: number; end: number }> | null) => void;
  onPreviewGenerate?: (pieces: TrackPiece[] | null) => void;
  setPointMarkers?: (markers: Array<{ x: number; y: number; label: string; color: number; id?: string }> | null) => void;
  setPickHandler?: (handler: ((wPos: { x: number; y: number }, piece: TrackPiece | null) => void) | null) => void;
  setPinDropHandler?: (handler: ((pinId: string, wPos: { x: number; y: number }, hitPiece: TrackPiece | null) => void) | null) => void;
  onClose?: () => void;
}

interface StartConnectorOption {
  piece: TrackPiece;
  connIdx: number;
  heightMm: number;
  label: string;
}

let activeHeightPanelCleanup: (() => void) | null = null;

export function openHeightPanel(options: HeightPanelOptions): HTMLElement {
  const {
    layout,
    catalog,
    onApplyFlat,
    onApplyRamp,
    onApplyGenerate,
    onPreviewRamp,
    onPreviewGenerate,
    setPointMarkers,
    setPickHandler,
    setPinDropHandler,
    onClose,
  } = options;

  const isDe = getLanguage() === "de";

  // Remove any existing height panel and its global listeners
  if (activeHeightPanelCleanup) {
    activeHeightPanelCleanup();
    activeHeightPanelCleanup = null;
  }
  const existing = document.getElementById("height-panel-root");
  if (existing) {
    existing.remove();
  }

  const layers: Layer[] = layout.layers && layout.layers.length > 0
    ? [...layout.layers]
    : [{ id: "layer-1", name: isDe ? "Hauptebene" : "Main Level", visible: true, locked: false, heightMm: 0 }];

  // Selected pieces
  let selectedPieceIds = options.selectedPieceIds ? [...options.selectedPieceIds] : [];
  let selectedPieces = layout.pieces.filter((p) => selectedPieceIds.includes(p.id));

  // Determine active/target layer
  const firstPiece = selectedPieces[0] || options.anchorPiece;
  const currentLayerId = firstPiece?.layerId || layout.activeLayerId || layers[0]!.id;
  const currentLayer = layers.find((l) => l.id === currentLayerId) || layers[0]!;
  let targetLayerId = options.preselectedLayerId || currentLayer.id;

  const cleanLayerName = (name: string): string =>
    name.replace(/\s*\(\s*[+-]?[0-9]+(?:[.,][0-9]+)?\s*mm\s*\)$/, "");

  // Panel container
  const panel = document.createElement("div");
  panel.id = "height-panel-root";
  panel.className = "height-panel-card";
  panel.innerHTML = heightPanelHtml;

  // Position below or next to #right-sidebar (or restore saved position)
  const parentEl = document.getElementById("app") || document.body;
  parentEl.appendChild(panel);

  const applyDefaultDockedPosition = () => {
    panel.style.left = "auto";
    panel.style.bottom = "auto";
    panel.style.right = "14px";
    panel.style.top = "76px";
    const rsb = document.getElementById("right-sidebar");
    if (rsb) {
      const rsbRect = rsb.getBoundingClientRect();
      const appEl = document.getElementById("app");
      const isCollapsed = appEl?.classList.contains("right-collapsed");
      if (!isCollapsed && rsbRect.height > 0) {
        if (rsbRect.bottom + 420 <= window.innerHeight) {
          panel.style.top = `${Math.max(76, rsbRect.bottom + 8)}px`;
          panel.style.right = "14px";
        } else {
          panel.style.top = `${Math.max(76, rsbRect.top)}px`;
          panel.style.right = `${Math.round(rsbRect.width + 24)}px`;
        }
      }
    }
  };

  applyDefaultDockedPosition();

  if (typeof localStorage !== "undefined") {
    try {
      const savedPosStr = localStorage.getItem("railbench_height_panel_pos");
      if (savedPosStr) {
        const savedPos = JSON.parse(savedPosStr);
        if (typeof savedPos?.left === "number" && typeof savedPos?.top === "number") {
          const maxLeft = Math.max(10, (window.innerWidth || 1000) - (panel.offsetWidth || 334) - 10);
          const maxTop = Math.max(50, (window.innerHeight || 800) - 80);
          panel.style.right = "auto";
          panel.style.bottom = "auto";
          panel.style.left = `${Math.round(Math.max(10, Math.min(savedPos.left, maxLeft)))}px`;
          panel.style.top = `${Math.round(Math.max(50, Math.min(savedPos.top, maxTop)))}px`;
        }
      }
    } catch {}
  }

  const qs = <T extends HTMLElement = HTMLElement>(sel: string): T => panel.querySelector(sel) as T;

  // Dragging support for header (matching PerfMonitor behavior)
  const header = qs(".hp-header");
  header.style.userSelect = "none";
  header.style.touchAction = "none";
  header.title = isDe
    ? "Ziehen zum Verschieben (Doppelklick zum Zurücksetzen)"
    : "Drag to move (Double-click to reset)";

  let isDraggingPanel = false;
  let dragStartX = 0;
  let dragStartY = 0;
  let initialLeft = 0;
  let initialTop = 0;

  const onPanelDragMove = (e: PointerEvent) => {
    if (!isDraggingPanel) return;
    const dx = e.clientX - dragStartX;
    const dy = e.clientY - dragStartY;
    const panelW = panel.offsetWidth || 334;
    const maxLeft = Math.max(10, (window.innerWidth || 1000) - panelW - 10);
    const maxTop = Math.max(50, (window.innerHeight || 800) - 80);
    const newLeft = Math.max(10, Math.min(initialLeft + dx, maxLeft));
    const newTop = Math.max(50, Math.min(initialTop + dy, maxTop));
    panel.style.left = `${Math.round(newLeft)}px`;
    panel.style.top = `${Math.round(newTop)}px`;
  };

  const onPanelDragEnd = (e?: PointerEvent) => {
    if (!isDraggingPanel) return;
    isDraggingPanel = false;
    header.style.cursor = "grab";
    if (typeof document !== "undefined" && document.body) {
      document.body.style.userSelect = "";
    }
    if (e && typeof header.hasPointerCapture === "function") {
      try {
        if (header.hasPointerCapture(e.pointerId)) {
          header.releasePointerCapture(e.pointerId);
        }
      } catch {}
    }
    window.removeEventListener("pointermove", onPanelDragMove);
    window.removeEventListener("pointerup", onPanelDragEnd);
    window.removeEventListener("pointercancel", onPanelDragEnd);

    const curLeft = parseFloat(panel.style.left);
    const curTop = parseFloat(panel.style.top);
    if (!isNaN(curLeft) && !isNaN(curTop) && typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_height_panel_pos", JSON.stringify({ left: curLeft, top: curTop }));
      } catch {}
    }
  };

  header.addEventListener("pointerdown", (e) => {
    if ((e.target as HTMLElement)?.closest?.("#hp-close-btn, button")) return;
    if (e.button !== 0) return;
    e.preventDefault();
    isDraggingPanel = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;

    const parentRect = typeof parentEl.getBoundingClientRect === "function"
      ? parentEl.getBoundingClientRect()
      : { left: 0, top: 0, width: 0, height: 0 };
    const rect = typeof panel.getBoundingClientRect === "function"
      ? panel.getBoundingClientRect()
      : { left: 0, top: 0, width: 0, height: 0 };

    if (rect.width && parentRect.width) {
      initialLeft = rect.left - parentRect.left;
      initialTop = rect.top - parentRect.top;
    } else {
      const hostW = parentEl.clientWidth || window.innerWidth || 1000;
      const panelW = panel.offsetWidth || 334;
      initialLeft =
        parseFloat(panel.style.left) ||
        panel.offsetLeft ||
        (panel.style.right ? hostW - panelW - (parseFloat(panel.style.right) || 14) : 100);
      initialTop = parseFloat(panel.style.top) || panel.offsetTop || 76;
    }

    panel.style.right = "auto";
    panel.style.bottom = "auto";
    panel.style.left = `${Math.round(initialLeft)}px`;
    panel.style.top = `${Math.round(initialTop)}px`;
    header.style.cursor = "grabbing";
    if (typeof document !== "undefined" && document.body) {
      document.body.style.userSelect = "none";
    }
    if (typeof header.setPointerCapture === "function") {
      try {
        header.setPointerCapture(e.pointerId);
      } catch {}
    }
    window.addEventListener("pointermove", onPanelDragMove);
    window.addEventListener("pointerup", onPanelDragEnd);
    window.addEventListener("pointercancel", onPanelDragEnd);
  });

  header.addEventListener("dblclick", () => {
    applyDefaultDockedPosition();
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem("railbench_height_panel_pos");
      } catch {}
    }
  });

  // Cleanup helper
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closePanel();
    }
  };

  const closePanel = () => {
    if (isDraggingPanel) {
      onPanelDragEnd();
    }
    window.removeEventListener("keydown", onKeyDown);
    if (activeHeightPanelCleanup === closePanel) {
      activeHeightPanelCleanup = null;
    }
    setPickHandler?.(null);
    setPinDropHandler?.(null);
    setPointMarkers?.(null);
    onPreviewRamp?.(null);
    onPreviewGenerate?.(null);
    panel.remove();
    onClose?.();
  };
  activeHeightPanelCleanup = closePanel;

  const closeBtn = qs<HTMLButtonElement>("#hp-close-btn");
  const cancelBtn = qs<HTMLButtonElement>("#hp-btn-cancel");
  const applyBtn = qs<HTMLButtonElement>("#hp-btn-apply");
  closeBtn.addEventListener("click", closePanel);
  cancelBtn.addEventListener("click", closePanel);
  window.addEventListener("keydown", onKeyDown);

  // Tab switcher
  let activeTab: "flat" | "ramp" | "generate" = options.initialTab || (selectedPieces.length > 1 ? "ramp" : "flat");
  if (options.initialTab === "generate" || (!selectedPieces.length && options.anchorPiece)) {
    activeTab = "generate";
  }

  const tabBtns = panel.querySelectorAll<HTMLButtonElement>(".hp-nav-tab");
  const tabContents = {
    flat: qs("#hp-tab-content-flat"),
    ramp: qs("#hp-tab-content-ramp"),
    generate: qs("#hp-tab-content-generate"),
  };

  const switchTab = (tab: "flat" | "ramp" | "generate") => {
    activeTab = tab;
    tabBtns.forEach((btn) => btn.classList.toggle("active", btn.dataset.tab === tab));
    tabContents.flat.style.display = tab === "flat" ? "flex" : "none";
    tabContents.ramp.style.display = tab === "ramp" ? "flex" : "none";
    tabContents.generate.style.display = tab === "generate" ? "flex" : "none";

    // Clear pick state & update live preview for the active tab
    stopPicking();
    if (tab === "flat") {
      setPointMarkers?.(null);
      onPreviewRamp?.(null);
      onPreviewGenerate?.(null);
      applyBtn.textContent = isDe ? "Höhe anwenden" : "Apply Height";
    } else if (tab === "ramp") {
      onPreviewGenerate?.(null);
      applyBtn.textContent = isDe ? "Steigung anwenden" : "Apply Ramp";
      updateRampCalculation();
    } else if (tab === "generate") {
      onPreviewRamp?.(null);
      applyBtn.textContent = isDe ? "Rampe erstellen" : "Create Ramp";
      updateGenerateCalculation();
    }
  };

  tabBtns.forEach((btn) => {
    btn.addEventListener("click", () => switchTab(btn.dataset.tab as any));
  });

  // Pick handler state
  type PickingMode = null | "ramp-a" | "ramp-b" | "gen-start";
  let pickingMode: PickingMode = null;

  const stopPicking = () => {
    pickingMode = null;
    panel.querySelectorAll(".hp-pick-btn, .hp-pick-btn-compact").forEach((el) => {
      el.classList.remove("picking");
    });
    setPickHandler?.(null);
  };

  // =========================================================================
  // TAB 1: FLAT / CUSTOM HEIGHT
  // =========================================================================
  const flatMetaCount = qs("#hp-flat-count-badge");
  const flatMetaLayer = qs("#hp-flat-layer-badge");
  const flatLayerSelect = qs<HTMLSelectElement>("#hp-flat-layer-select");
  const flatModeLayerLbl = qs("#hp-flat-mode-layer-lbl");
  const flatModeCustomLbl = qs("#hp-flat-mode-custom-lbl");
  const flatCustomGroup = qs("#hp-flat-custom-group");
  const flatCustomHeightInput = qs<HTMLInputElement>("#hp-flat-custom-height");
  const flatStatusText = qs("#hp-flat-status-text");

  const renderLayerOptions = (selectEl: HTMLSelectElement, selectedId: string, withCreate = true) => {
    let html = layers
      .map((l) => `<option value="${l.id}" ${l.id === selectedId ? "selected" : ""}>${cleanLayerName(l.name)} (${l.heightMm ?? 0} mm)</option>`)
      .join("");
    if (withCreate) html += `<option value="__create_new__">+ ${isDe ? "Neue Ebene erstellen..." : "Create new layer..."}</option>`;
    selectEl.innerHTML = html;
  };

  renderLayerOptions(flatLayerSelect, targetLayerId);
  flatMetaCount.textContent = `${selectedPieces.length} ${isDe ? "Gleise" : "tracks"}`;
  flatMetaLayer.textContent = cleanLayerName(currentLayer.name);

  const promptCreateNewLayer = (onCreated: (l: Layer) => void) => {
    openLayerDialog({
      layout,
      targetHeightMm: (currentLayer.heightMm ?? 0) + 80,
      position: "above",
      onApply: (newLayer) => {
        layers.push(newLayer);
        if (!layout.layers) layout.layers = layers;
        renderLayerOptions(flatLayerSelect, newLayer.id);
        renderLayerOptions(rampLayerSelect, newLayer.id);
        onCreated(newLayer);
      },
    });
  };

  let flatMode: "layer" | "custom" = "layer";

  const updateFlatUI = () => {
    flatModeLayerLbl.classList.toggle("active", flatMode === "layer");
    flatModeCustomLbl.classList.toggle("active", flatMode === "custom");
    flatCustomGroup.style.display = flatMode === "custom" ? "flex" : "none";
    const selLayer = layers.find((l) => l.id === flatLayerSelect.value) || currentLayer;
    const h = flatMode === "layer" ? (selLayer.heightMm ?? 0) : (parseFloat(flatCustomHeightInput.value) || 0);
    flatStatusText.textContent = flatMode === "layer"
      ? (isDe ? `Gleise werden auf ${cleanLayerName(selLayer.name)} (${h} mm) gesetzt.` : `Tracks will be set to ${cleanLayerName(selLayer.name)} (${h} mm).`)
      : (isDe ? `Gleise werden auf feste Höhe von ${h} mm gesetzt.` : `Tracks will be set to fixed height of ${h} mm.`);
  };

  panel.querySelectorAll<HTMLInputElement>("input[name='hp-flat-mode']").forEach((radio) => {
    radio.addEventListener("change", () => { flatMode = radio.value as any; updateFlatUI(); });
  });

  flatCustomHeightInput.addEventListener("input", updateFlatUI);

  flatLayerSelect.addEventListener("change", () => {
    if (flatLayerSelect.value === "__create_new__") {
      promptCreateNewLayer((nl) => {
        flatCustomHeightInput.value = String(nl.heightMm ?? 0);
        updateFlatUI();
      });
      flatLayerSelect.value = targetLayerId;
      return;
    }
    const sel = layers.find((l) => l.id === flatLayerSelect.value);
    if (sel) flatCustomHeightInput.value = String(sel.heightMm ?? 0);
    updateFlatUI();
  });

  // =========================================================================
  // TAB 2: INCLINE / RAMP OVER CONNECTED TRACKS
  // =========================================================================
  const rampMetaCount = qs("#hp-ramp-count-badge");
  const rampMetaLength = qs("#hp-ramp-length-badge");
  const rampPickBtnA = qs<HTMLButtonElement>("#hp-ramp-pick-a");
  const rampPickBtnB = qs<HTMLButtonElement>("#hp-ramp-pick-b");
  const rampLabelA = qs("#hp-ramp-label-a");
  const rampLabelB = qs("#hp-ramp-label-b");
  const rampHeightInputA = qs<HTMLInputElement>("#hp-ramp-height-a");
  const rampHeightInputB = qs<HTMLInputElement>("#hp-ramp-height-b");
  const rampSwapBtn = qs<HTMLButtonElement>("#hp-ramp-swap-btn");
  const rampConnStatus = qs("#hp-ramp-conn-status");
  const rampAutoFitChk = qs<HTMLInputElement>("#hp-ramp-auto-fit");
  const rampCustomSteepInput = qs<HTMLInputElement>("#hp-ramp-custom-steep");
  const rampProfileLinear = qs("#hp-ramp-profile-linear");
  const rampProfileEased = qs("#hp-ramp-profile-eased");
  const rampLayerSelect = qs<HTMLSelectElement>("#hp-ramp-layer-select");
  const rampStatusBadge = qs("#hp-ramp-status-badge");
  const rampStatusTitle = qs("#hp-ramp-status-title");
  const rampStatusDesc = qs("#hp-ramp-status-desc");
  const rampBreakdownSummary = qs("#hp-ramp-breakdown-summary");
  const rampBreakdownList = qs("#hp-ramp-breakdown-list");

  renderLayerOptions(rampLayerSelect, targetLayerId);

  let rampPieceA: TrackPiece | null = selectedPieces[0] || null;
  let rampPieceB: TrackPiece | null = selectedPieces.length > 1 ? selectedPieces[selectedPieces.length - 1]! : null;
  let rampProfile: RampProfile = "linear";
  let rampReversed = false;
  let rampTargetSteep = 2.5;

  // Initial heights
  const currentBaseH = currentLayer.heightMm ?? 0;
  rampHeightInputA.value = String(currentBaseH + (rampPieceA?.elevation?.start ?? 0));
  rampHeightInputB.value = String(currentBaseH + (rampPieceB?.elevation?.end ?? (rampPieceA?.elevation?.start ?? 0) + 80));

  const updateRampEndpointsUI = (chain?: Array<{ piece: TrackPiece; entryConnectorIndex: number; exitConnectorIndex: number }>) => {
    rampLabelA.textContent = rampPieceA
      ? `#${rampPieceA.catalogId} (${Math.round(rampPieceA.transform.x)}, ${Math.round(rampPieceA.transform.y)})`
      : (isDe ? "Gleis A wählen..." : "Pick track A...");
    rampLabelB.textContent = rampPieceB
      ? `#${rampPieceB.catalogId} (${Math.round(rampPieceB.transform.x)}, ${Math.round(rampPieceB.transform.y)})`
      : (isDe ? "Gleis B wählen..." : "Pick track B...");

    // Große Start & Stop Pins auf dem Canvas-Overlay (A & B)
    const markers: Array<{ x: number; y: number; label: string; color: number; id?: string }> = [];
    if (chain && chain.length > 0) {
      const first = chain[0]!;
      const startC = first.piece.connectors[first.entryConnectorIndex] || first.piece.transform;
      markers.push({ x: startC.x, y: startC.y, label: "Start", color: 0x10b981, id: "A" });

      const last = chain[chain.length - 1]!;
      const endC = last.piece.connectors[last.exitConnectorIndex] || last.piece.transform;
      markers.push({ x: endC.x, y: endC.y, label: "Stop", color: 0xf97316, id: "B" });
    } else {
      if (rampPieceA) {
        const conn = rampPieceA.connectors[0] || rampPieceA.transform;
        markers.push({ x: conn.x, y: conn.y, label: "Start", color: 0x10b981, id: "A" });
      }
      if (rampPieceB && rampPieceB.id !== rampPieceA?.id) {
        const conn = rampPieceB.connectors[rampPieceB.connectors.length - 1] || rampPieceB.transform;
        markers.push({ x: conn.x, y: conn.y, label: "Stop", color: 0xf97316, id: "B" });
      } else if (rampPieceA && rampPieceA.connectors.length > 1) {
        const conn = rampPieceA.connectors[rampPieceA.connectors.length - 1]!;
        markers.push({ x: conn.x, y: conn.y, label: "Stop", color: 0xf97316, id: "B" });
      }
    }
    setPointMarkers?.(markers);
  };

  const setRampEndpointPiece = (which: "A" | "B", hitPiece: TrackPiece) => {
    const baseH = (layers.find((l) => l.id === hitPiece.layerId)?.heightMm ?? 0);
    if (which === "A") {
      rampPieceA = hitPiece;
      rampHeightInputA.value = String(baseH + (hitPiece.elevation?.start ?? 0));
    } else {
      rampPieceB = hitPiece;
      rampHeightInputB.value = String(baseH + (hitPiece.elevation?.end ?? (hitPiece.elevation?.start ?? 0)));
    }
    if (rampPieceA && rampPieceB) {
      if (rampPieceA.id === rampPieceB.id) selectedPieces = [rampPieceA];
      else {
        const path = findConnectedTrackPath(rampPieceA.id, rampPieceB.id, layout.pieces);
        selectedPieces = (path && path.length > 0) ? path : (selectedPieces.some((p) => p.id === hitPiece.id) ? selectedPieces : [...selectedPieces, hitPiece]);
      }
    } else {
      selectedPieces = [hitPiece];
    }
    selectedPieceIds = selectedPieces.map((p) => p.id);
    stopPicking();
    updateRampEndpointsUI();
    updateRampCalculation();
  };

  const startPickingForRamp = (which: "A" | "B") => {
    if (pickingMode === (which === "A" ? "ramp-a" : "ramp-b")) { stopPicking(); return; }
    stopPicking();
    pickingMode = which === "A" ? "ramp-a" : "ramp-b";
    (which === "A" ? rampPickBtnA : rampPickBtnB).classList.add("picking");
    setPickHandler?.((_wPos, hitPiece) => { if (hitPiece) setRampEndpointPiece(which, hitPiece); });
  };

  rampPickBtnA.addEventListener("click", () => startPickingForRamp("A"));
  rampPickBtnB.addEventListener("click", () => startPickingForRamp("B"));

  rampSwapBtn.addEventListener("click", () => {
    const tempP = rampPieceA; rampPieceA = rampPieceB; rampPieceB = tempP;
    const tempH = rampHeightInputA.value; rampHeightInputA.value = rampHeightInputB.value; rampHeightInputB.value = tempH;
    rampReversed = !rampReversed;
    updateRampEndpointsUI();
    updateRampCalculation();
  });

  // Profile selection
  [rampProfileLinear, rampProfileEased].forEach((el) => {
    el.addEventListener("click", () => {
      rampProfile = el === rampProfileLinear ? "linear" : "eased";
      rampProfileLinear.classList.toggle("active", rampProfile === "linear");
      rampProfileEased.classList.toggle("active", rampProfile === "eased");
      updateRampCalculation();
    });
  });

  // Slope presets
  panel.querySelectorAll<HTMLButtonElement>("#hp-tab-content-ramp .hp-pill-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      panel.querySelectorAll("#hp-tab-content-ramp .hp-pill-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      rampTargetSteep = parseFloat(btn.dataset.steep || "2.5");
      rampCustomSteepInput.value = rampTargetSteep.toFixed(1);
      updateRampCalculation();
    });
  });

  rampCustomSteepInput.addEventListener("input", () => {
    const val = parseFloat(rampCustomSteepInput.value);
    if (!isNaN(val) && val >= 0.1 && val <= 15.0) {
      rampTargetSteep = val;
      panel.querySelectorAll<HTMLButtonElement>("#hp-tab-content-ramp .hp-pill-btn").forEach((b) => {
        b.classList.toggle("active", Math.abs(parseFloat(b.dataset.steep || "0") - val) < 0.05);
      });
      rampTargetWarning = null;
      updateRampCalculation();
    }
  });

  rampAutoFitChk.addEventListener("change", () => { rampTargetWarning = null; updateRampCalculation(); });
  rampHeightInputA.addEventListener("input", () => { rampTargetWarning = null; updateRampCalculation(); });
  rampHeightInputB.addEventListener("input", () => { rampTargetWarning = null; updateRampCalculation(); });

  let rampTargetWarning: { title: string; desc: string } | null = null;

  const applyRampTargetLayer = (targetLayer: Layer) => {
    targetLayerId = targetLayer.id;
    const targetH = targetLayer.heightMm ?? 0;
    const sH = parseFloat(rampHeightInputA.value) || 0;
    const pieces = selectedPieces.length > 0 ? selectedPieces : (rampPieceA ? [rampPieceA] : []);
    const testRes = calculateRamp({
      pieces,
      catalog,
      startHeightMm: sH,
      endHeightMm: targetH,
      layerBaseHeightMm: targetH,
      profile: rampProfile,
      reversed: rampReversed,
      targetMaxGradientPercent: rampTargetSteep,
      startPieceId: rampPieceA?.id,
      endPieceId: rampPieceB?.id,
    });

    if (testRes.totalLengthMm > 0 && testRes.exceedsTargetMax && testRes.recommendedEndHeightMm !== undefined) {
      const maxAchievableH = testRes.recommendedEndHeightMm;
      rampHeightInputB.value = String(maxAchievableH);
      const curLenCm = (testRes.totalLengthMm / 10).toFixed(1);
      const reqLenCm = testRes.requiredLengthMm ? (testRes.requiredLengthMm / 10).toFixed(1) : "?";
      const maxRiseMm = testRes.maxAllowableRiseMm ?? 0;
      const dirWord = targetH < sH ? (isDe ? "Tiefe" : "depth") : (isDe ? "Höhe" : "height");

      rampTargetWarning = {
        title: isDe
          ? `⚠️ Gleisstrecke zu kurz für Zielebene (${targetH} mm)!`
          : `⚠️ Track too short for target layer (${targetH} mm)!`,
        desc: isDe
          ? `Gleislänge (${curLenCm} cm) reicht nicht für ${targetH} mm (benötigt: ${reqLenCm} cm bei max. ${rampTargetSteep}%). Endwert auf max. ${dirWord} (${maxAchievableH} mm, Δ ${maxRiseMm} mm) begrenzt.`
          : `Track length (${curLenCm} cm) cannot reach ${targetH} mm (needed: ${reqLenCm} cm at max ${rampTargetSteep}%). End value capped to max ${dirWord} (${maxAchievableH} mm, Δ ${maxRiseMm} mm).`,
      };
    } else {
      rampTargetWarning = null;
      rampHeightInputB.value = String(targetH);
    }
    updateRampCalculation();
  };

  rampLayerSelect.addEventListener("change", () => {
    if (rampLayerSelect.value === "__create_new__") {
      promptCreateNewLayer((nl) => applyRampTargetLayer(nl));
      rampLayerSelect.value = targetLayerId;
      return;
    }
    const sel = layers.find((l) => l.id === rampLayerSelect.value);
    if (sel) applyRampTargetLayer(sel);
    else updateRampCalculation();
  });

  let latestRampResult: ReturnType<typeof calculateRamp> | null = null;

  const updateRampCalculation = () => {
    if (selectedPieces.length === 0) {
      if (rampPieceA) selectedPieces = [rampPieceA];
    }

    rampMetaCount.textContent = `${selectedPieces.length} ${isDe ? "Gleise" : "tracks"}`;

    const selLayer = layers.find((l) => l.id === rampLayerSelect.value) || currentLayer;
    const baseH = selLayer.heightMm ?? 0;
    let sH = parseFloat(rampHeightInputA.value) || 0;
    let eH = parseFloat(rampHeightInputB.value) || 0;

    let res = calculateRamp({
      pieces: selectedPieces,
      catalog,
      startHeightMm: sH,
      endHeightMm: eH,
      layerBaseHeightMm: baseH,
      profile: rampProfile,
      reversed: rampReversed,
      targetMaxGradientPercent: rampTargetSteep,
      startPieceId: rampPieceA?.id,
      endPieceId: rampPieceB?.id,
    });

    rampMetaLength.textContent = `${(res.totalLengthMm / 10).toFixed(1)} cm`;
    rampConnStatus.textContent = res.isContinuous ? (isDe ? "Verbunden" : "Continuous") : (isDe ? "Getrennt" : "Split");
    rampConnStatus.style.color = res.isContinuous ? "#10b981" : "#f59e0b";

    // Auto-fit if slope exceeds limit
    if (rampAutoFitChk.checked && res.exceedsTargetMax && res.totalLengthMm > 0) {
      const dir: 1 | -1 = eH < sH ? -1 : 1;
      const fittedEndH = Math.round(
        calculateEndHeightFromMaxGradient({
          startHeightMm: sH,
          totalLengthMm: res.totalLengthMm,
          targetMaxGradientPercent: rampTargetSteep,
          profile: rampProfile,
          direction: dir,
        }),
      );
      eH = fittedEndH;
      rampHeightInputB.value = String(fittedEndH);

      res = calculateRamp({
        pieces: selectedPieces,
        catalog,
        startHeightMm: sH,
        endHeightMm: eH,
        layerBaseHeightMm: baseH,
        profile: rampProfile,
        reversed: rampReversed,
        targetMaxGradientPercent: rampTargetSteep,
        startPieceId: rampPieceA?.id,
        endPieceId: rampPieceB?.id,
      });
    }

    latestRampResult = res;

    // Live preview on 2D canvas!
    onPreviewRamp?.(res.pieceElevations);

    // Status badge
    const deltaH = Math.abs(eH - sH);
    const grad = res.averageGradientPercent;
    const maxG = res.maxGradientPercent;
    const evalGrad = Math.max(grad, maxG);

    if (rampTargetWarning) {
      rampStatusBadge.className = "hp-status-box warn";
      rampStatusTitle.textContent = rampTargetWarning.title;
      rampStatusDesc.textContent = rampTargetWarning.desc;
    } else if (evalGrad <= 3.0) {
      rampStatusBadge.className = "hp-status-box ok";
      rampStatusTitle.textContent = `✓ ${isDe ? "Sichere Steigung" : "Safe Slope"}: ${grad.toFixed(1)}% (Δ ${deltaH} mm)`;
      rampStatusDesc.textContent = isDe ? "Optimal befahrbar für alle Zuggarnituren." : "Safe for all train formations.";
    } else if (evalGrad <= 4.5) {
      rampStatusBadge.className = "hp-status-box warn";
      rampStatusTitle.textContent = `⚠️ ${isDe ? "Mäßige Steigung" : "Moderate Slope"}: ${grad.toFixed(1)}% (Δ ${deltaH} mm)`;
      rampStatusDesc.textContent = isDe ? "Für kurze bis mittlere Züge mit Haftreifen geeignet." : "Suitable for short to medium trains.";
    } else {
      rampStatusBadge.className = "hp-status-box danger";
      rampStatusTitle.textContent = `⚠️ ${isDe ? "Steile Rampe" : "Steep Ramp"}: ${grad.toFixed(1)}% (Δ ${deltaH} mm)`;
      rampStatusDesc.textContent = isDe ? "Gefahr von Durchdrehen oder Entkuppeln! Gleislänge vergrößern." : "Risk of slipping or uncoupling! Increase length.";
    }

    // Breakdown list
    rampBreakdownSummary.textContent = `▸ ${isDe ? "Gleis-Details" : "Track Details"} (${selectedPieces.length} ${isDe ? "Gleise" : "tracks"})`;
    rampBreakdownList.innerHTML = res.chain
      .map((info, idx) => {
        const h1 = info.entryHeightMm >= 0 ? `+${info.entryHeightMm}` : `${info.entryHeightMm}`;
        const h2 = info.exitHeightMm >= 0 ? `+${info.exitHeightMm}` : `${info.exitHeightMm}`;
        return `
          <div class="hp-breakdown-row">
            <span>${idx + 1}. #${info.piece.catalogId} (${Math.round(info.lengthMm)} mm)</span>
            <span>${h1} ➔ ${h2} mm (${info.gradientPercent.toFixed(1)}%)</span>
          </div>
        `;
      })
      .join("");

    updateRampEndpointsUI(res.chain);
  };

  updateRampEndpointsUI();

  // =========================================================================
  // TAB 3: AUTO CREATE TRACK RAMP (GENERATOR)
  // =========================================================================
  const scale = layout.scale || catalog.scale || "H0";
  const minClearance = getMinClearanceHeightForScale(scale);

  const genStartSelect = qs<HTMLSelectElement>("#hp-gen-start-select");
  const genPickCanvasBtn = qs<HTMLButtonElement>("#hp-gen-pick-canvas-btn");
  const genStartSubtext = qs("#hp-gen-start-subtext");
  const genTargetLayerSelect = qs<HTMLSelectElement>("#hp-gen-target-layer");
  const genClearanceHint = qs("#hp-gen-clearance-hint");
  const genTargetHeightInput = qs<HTMLInputElement>("#hp-gen-target-height");
  const genBtnSlopeUp = qs<HTMLButtonElement>("#hp-gen-btn-slope-up");
  const genBtnSlopeDown = qs<HTMLButtonElement>("#hp-gen-btn-slope-down");
  const genCurveOptions = qs("#hp-gen-curve-options");
  const genCurveRadiusSelect = qs<HTMLSelectElement>("#hp-gen-curve-radius");
  const genCircumHint = qs("#hp-gen-circum-hint");
  const genDirLeft = qs<HTMLButtonElement>("#hp-gen-dir-left");
  const genDirRight = qs<HTMLButtonElement>("#hp-gen-dir-right");
  const genAngleField = qs("#hp-gen-angle-field");
  const genCustomGradInput = qs<HTMLInputElement>("#hp-gen-custom-grad");
  const genStatusBox = qs("#hp-gen-status-box");
  const genStatusTitle = qs("#hp-gen-status-title");
  const genStatusDesc = qs("#hp-gen-status-desc");

  genClearanceHint.textContent = `(Min. ${minClearance} mm)`;

  // Collect start points (open connectors)
  const startConnectorOptions: StartConnectorOption[] = [];
  const initialAnchor = options.anchorPiece || selectedPieces[0] || layout.pieces[0];

  const pushConnectorOptions = (piece: TrackPiece) => {
    const openIdxs = getOpenConnectors(piece, layout.pieces, 8.0, layers);
    for (const ci of openIdxs) {
      const conn = piece.connectors[ci]!;
      const h = getPieceConnectorHeight(piece, ci, layers);
      const letter = String.fromCharCode(65 + (ci % 26));
      const label = `#${piece.catalogId} · Ende ${letter} · ${Math.round(h)} mm · (${Math.round(conn.x)}, ${Math.round(conn.y)})`;
      startConnectorOptions.push({ piece, connIdx: ci, heightMm: h, label });
    }
  };

  if (initialAnchor) {
    pushConnectorOptions(initialAnchor);
  }
  for (const p of layout.pieces) {
    if (initialAnchor && p.id === initialAnchor.id) continue;
    if (startConnectorOptions.length >= 60) break;
    pushConnectorOptions(p);
  }

  // Populate start options
  if (startConnectorOptions.length > 0) {
    genStartSelect.innerHTML = startConnectorOptions
      .map((o, idx) => `<option value="${idx}">${o.label}</option>`)
      .join("");
  } else {
    genStartSelect.innerHTML = `<option value="-1">${isDe ? "Keine offenen Enden gefunden" : "No open ends found"}</option>`;
  }

  let genStartIdx = 0;
  if (options.anchorConnectorIndex !== undefined && initialAnchor) {
    const foundIdx = startConnectorOptions.findIndex(
      (o) => o.piece.id === initialAnchor.id && o.connIdx === options.anchorConnectorIndex,
    );
    if (foundIdx >= 0) genStartIdx = foundIdx;
  }

  // Available curve radii
  const availableRadii = getAvailableCurveRadii(catalog);
  if (availableRadii.length > 0) {
    genCurveRadiusSelect.innerHTML = availableRadii
      .map((r) => `<option value="${r.catalogId}">${r.label} (${Math.round(r.radiusMm)} mm, ${r.angleDeg}°)</option>`)
      .join("");
  }

  // Build target layer options with + Create Above/Below
  const buildGenLayerOptions = (preselectedId: string): string =>
    layers.map((l) => `<option value="${l.id}" ${l.id === preselectedId ? "selected" : ""}>${cleanLayerName(l.name)} (${l.heightMm ?? 0} mm)</option>`).join("") +
    `<option value="__create_above__" ${preselectedId === "__create_above__" ? "selected" : ""}>+ ${isDe ? "Neue Ebene oben" : "New layer above"} (+${minClearance} mm)</option>
    <option value="__create_below__" ${preselectedId === "__create_below__" ? "selected" : ""}>+ ${isDe ? "Neue Ebene unten" : "New layer below"} (-${minClearance} mm)</option>`;

  genTargetLayerSelect.innerHTML = buildGenLayerOptions(targetLayerId);

  let genShape: RampShapeType = "straight";
  let genDirection: "left" | "right" = "left";
  let genCurveAngle = 180;
  let genGradient = 2.8;
  let genCurveId = availableRadii[0]?.catalogId;

  // Sync slope direction buttons
  const syncGenSlopeButtons = (th: number, curH: number) => {
    genBtnSlopeUp.classList.toggle("active", th >= curH);
    genBtnSlopeDown.classList.toggle("active", th < curH);
  };

  const getActiveStartOption = (): StartConnectorOption | undefined => {
    return startConnectorOptions[genStartIdx];
  };

  // Canvas picking & Drag-and-Drop Handler für Startpunkt
  const setGenStartPiece = (hitPiece: TrackPiece, wPos: { x: number; y: number }) => {
    const openIdxs = getOpenConnectors(hitPiece, layout.pieces, 8.0, layers);
    if (openIdxs.length === 0) return;
    let bestCi = openIdxs[0]!, bestDist = Infinity;
    for (const ci of openIdxs) {
      const conn = hitPiece.connectors[ci]!;
      const d = Math.hypot(wPos.x - conn.x, wPos.y - conn.y);
      if (d < bestDist) { bestDist = d; bestCi = ci; }
    }
    let optIdx = startConnectorOptions.findIndex((o) => o.piece.id === hitPiece.id && o.connIdx === bestCi);
    if (optIdx < 0) {
      const conn = hitPiece.connectors[bestCi]!;
      const h = getPieceConnectorHeight(hitPiece, bestCi, layers);
      const letter = String.fromCharCode(65 + (bestCi % 26));
      const label = `#${hitPiece.catalogId} · Ende ${letter} · ${Math.round(h)} mm · (${Math.round(conn.x)}, ${Math.round(conn.y)})`;
      startConnectorOptions.unshift({ piece: hitPiece, connIdx: bestCi, heightMm: h, label });
      genStartSelect.innerHTML = startConnectorOptions.map((o, idx) => `<option value="${idx}">${o.label}</option>`).join("");
      optIdx = 0;
    }
    genStartIdx = optIdx;
    genStartSelect.value = String(optIdx);
    stopPicking();
    updateGenerateCalculation();
  };

  genPickCanvasBtn.addEventListener("click", () => {
    if (pickingMode === "gen-start") { stopPicking(); return; }
    stopPicking();
    pickingMode = "gen-start";
    genPickCanvasBtn.classList.add("picking");
    setPickHandler?.((wPos, hitPiece) => { if (hitPiece) setGenStartPiece(hitPiece, wPos); });
  });

  // Globaler Pin-Drop Handler: Wird aufgerufen wenn Pin A oder B auf ein Gleis gezogen & losgelassen wird
  setPinDropHandler?.((pinId, wPos, hitPiece) => {
    if (!hitPiece) return;
    if (activeTab === "ramp") {
      const which = (pinId === "B" || pinId === "stop" || pinId.toLowerCase().includes("stop")) ? "B" : "A";
      setRampEndpointPiece(which, hitPiece);
    } else if (activeTab === "generate") {
      setGenStartPiece(hitPiece, wPos);
    }
  });

  genStartSelect.addEventListener("change", () => {
    genStartIdx = parseInt(genStartSelect.value, 10) || 0;
    updateGenerateCalculation();
  });

  // Shape switcher
  panel.querySelectorAll<HTMLElement>("#hp-tab-content-generate .hp-segmented-item").forEach((item) => {
    item.addEventListener("click", () => {
      panel.querySelectorAll("#hp-tab-content-generate .hp-segmented-item").forEach((s) => s.classList.remove("active"));
      item.classList.add("active");
      genShape = (item.dataset.shape as RampShapeType) || "straight";
      genCurveOptions.style.display = genShape === "straight" ? "none" : "flex";
      genAngleField.style.display = genShape === "helix" ? "none" : "flex";
      updateGenerateCalculation();
    });
  });

  // Curve radius change
  genCurveRadiusSelect.addEventListener("change", () => {
    genCurveId = genCurveRadiusSelect.value;
    updateGenerateCalculation();
  });

  // Curve direction
  [genDirLeft, genDirRight].forEach((b) => {
    b.addEventListener("click", () => {
      genDirection = b === genDirLeft ? "left" : "right";
      genDirLeft.classList.toggle("active", genDirection === "left");
      genDirRight.classList.toggle("active", genDirection === "right");
      updateGenerateCalculation();
    });
  });

  // Curve angle buttons
  panel.querySelectorAll<HTMLButtonElement>("#hp-gen-angle-field .hp-pill-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      panel.querySelectorAll("#hp-gen-angle-field .hp-pill-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      genCurveAngle = parseFloat(btn.dataset.angle || "180");
      updateGenerateCalculation();
    });
  });

  // Gradient presets
  panel.querySelectorAll<HTMLButtonElement>("#hp-tab-content-generate .hp-pill-btn[data-grad]").forEach((btn) => {
    btn.addEventListener("click", () => {
      panel.querySelectorAll("#hp-tab-content-generate .hp-pill-btn[data-grad]").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      genGradient = parseFloat(btn.dataset.grad || "2.8");
      genCustomGradInput.value = genGradient.toFixed(1);
      updateGenerateCalculation();
    });
  });

  genCustomGradInput.addEventListener("input", () => {
    const val = parseFloat(genCustomGradInput.value);
    if (!isNaN(val) && val >= 0.5 && val <= 15.0) {
      genGradient = val;
      panel.querySelectorAll<HTMLButtonElement>("#hp-tab-content-generate .hp-pill-btn[data-grad]").forEach((b) => {
        b.classList.toggle("active", Math.abs(parseFloat(b.dataset.grad || "0") - val) < 0.05);
      });
      updateGenerateCalculation();
    }
  });

  // Slope direction toggles
  const setGenSlopeDir = (up: boolean) => {
    const curH = getActiveStartOption()?.heightMm ?? 0;
    const curTH = parseFloat(genTargetHeightInput.value) || (curH + minClearance);
    const absDelta = Math.abs(curTH - curH) || minClearance;
    genTargetHeightInput.value = String(Math.round(up ? curH + absDelta : curH - absDelta));
    updateGenerateCalculation();
  };
  genBtnSlopeUp.addEventListener("click", () => setGenSlopeDir(true));
  genBtnSlopeDown.addEventListener("click", () => setGenSlopeDir(false));

  genTargetLayerSelect.addEventListener("change", () => {
    const val = genTargetLayerSelect.value;
    const curH = getActiveStartOption()?.heightMm ?? 0;
    if (val === "__create_above__") {
      genTargetHeightInput.value = String(Math.round(curH + minClearance));
    } else if (val === "__create_below__") {
      genTargetHeightInput.value = String(Math.round(curH - minClearance));
    } else {
      const lay = layers.find((l) => l.id === val);
      if (lay) genTargetHeightInput.value = String(lay.heightMm ?? 0);
    }
    updateGenerateCalculation();
  });

  genTargetHeightInput.addEventListener("input", () => updateGenerateCalculation());

  let latestGenerateResult: ReturnType<typeof generateRampPieces> | null = null;

  const updateGenerateCalculation = () => {
    const startOpt = getActiveStartOption();
    if (!startOpt) {
      genStatusBox.className = "hp-status-box danger";
      genStatusTitle.textContent = isDe ? "Kein Startpunkt vorhanden" : "No start point available";
      return;
    }

    const curH = startOpt.heightMm;
    const parsedTH = parseFloat(genTargetHeightInput.value);
    const tH = isNaN(parsedTH) ? curH + minClearance : parsedTH;
    syncGenSlopeButtons(tH, curH);

    // Marker on start connector
    const conn = startOpt.piece.connectors[startOpt.connIdx]!;
    const genMarkers: Array<{ x: number; y: number; label: string; color: number; id?: string }> = [
      { x: conn.x, y: conn.y, label: "Start", color: 0x10b981, id: "A" },
    ];
    setPointMarkers?.(genMarkers);

    genStartSubtext.textContent = isDe
      ? `Starthöhe: ${Math.round(curH)} mm · Richtung: ${Math.round(conn.angle)}°`
      : `Start height: ${Math.round(curH)} mm · Heading: ${Math.round(conn.angle)}°`;

    // Circumference hint for helix
    const selRadius = availableRadii.find((r) => r.catalogId === genCurveId);
    if (genShape === "helix" && selRadius) {
      const circum = Math.round(2 * Math.PI * selRadius.radiusMm);
      genCircumHint.textContent = `${isDe ? "Umfang" : "Circumference"}: ${circum} mm`;
    } else {
      genCircumHint.textContent = "";
    }

    const res = generateRampPieces({
      catalog,
      anchorPiece: startOpt.piece,
      anchorConnectorIndex: startOpt.connIdx,
      targetLayerId: genTargetLayerSelect.value,
      targetHeightMm: tH,
      maxGradientPercent: genGradient,
      shape: genShape,
      selectedCurveCatalogId: genCurveId,
      curveDirection: genDirection,
      curveAngleDeg: genCurveAngle,
      layers,
    });

    latestGenerateResult = res;

    if (!res.success) {
      genStatusBox.className = "hp-status-box danger";
      genStatusTitle.textContent = isDe ? "⚠️ Rampe kann nicht berechnet werden" : "⚠️ Cannot compute ramp";
      genStatusDesc.textContent = res.reason === "same_height"
        ? (isDe ? "Start- und Zielhöhe sind identisch." : "Start and target heights are identical.")
        : (isDe ? "Kein offener Anschluss oder ungültige Parameter." : "No open connector or invalid parameters.");
      onPreviewGenerate?.(null);
      applyBtn.disabled = true;
      return;
    }

    applyBtn.disabled = false;
    onPreviewGenerate?.(res.pieces);

    // Marker on end/stop connector of generated ramp
    if (res.pieces.length > 0) {
      const lastPiece = res.pieces[res.pieces.length - 1]!;
      const endConn = lastPiece.connectors[1] || lastPiece.connectors[0] || lastPiece.transform;
      genMarkers.push({ x: endConn.x, y: endConn.y, label: "Stop", color: 0xf97316, id: "B" });
      setPointMarkers?.(genMarkers);
    }

    const count = res.pieces.length;
    const lenCm = (res.totalLengthMm / 10).toFixed(1);
    const grad = Math.abs(res.gradientPercent);
    const deltaH = Math.round(tH - curH);

    let turnsText = "";
    if (genShape === "helix" && selRadius) {
      const piecesPerTurn = Math.max(4, Math.round(360 / Math.max(1, selRadius.angleDeg)));
      const turns = (count / piecesPerTurn).toFixed(1);
      turnsText = `, ${turns} ${isDe ? "Umdr." : "turns"}`;
    }

    const gradLevel = grad <= 3.0 ? "ok" : grad <= 4.5 ? "warn" : "danger";
    const gradPrefix = grad <= 3.0 ? (isDe ? "✓ Sichere Steigung" : "✓ Safe gradient") : grad <= 4.5 ? (isDe ? "⚠️ Mäßige Steigung" : "⚠️ Moderate gradient") : (isDe ? "⚠️ Steile Rampe" : "⚠️ Steep ramp");
    genStatusBox.className = `hp-status-box ${gradLevel}`;
    genStatusTitle.textContent = `${gradPrefix}: ${grad.toFixed(1)}% (${count} ${isDe ? "Gleise" : "tracks"}${turnsText}, ${lenCm} cm)`;
    genStatusDesc.textContent = `${Math.round(curH)} mm ➔ ${Math.round(tH)} mm (Δ ${deltaH > 0 ? "+" : ""}${deltaH} mm)`;
  };

  switchTab(activeTab);

  // =========================================================================
  // APPLY HANDLER
  // =========================================================================
  applyBtn.addEventListener("click", () => {
    if (activeTab === "flat") {
      closePanel();
      onApplyFlat?.(flatLayerSelect.value, flatMode === "custom" ? (parseFloat(flatCustomHeightInput.value) || 0) : undefined);
    } else if (activeTab === "ramp") {
      if (!latestRampResult) return;
      closePanel();
      onApplyRamp?.(rampLayerSelect.value, latestRampResult.pieceElevations);
    } else if (activeTab === "generate") {
      if (!latestGenerateResult || !latestGenerateResult.success || latestGenerateResult.pieces.length === 0) return;
      const startOpt = getActiveStartOption();
      const curH = startOpt?.heightMm ?? 0;
      const parsedTH = parseFloat(genTargetHeightInput.value);
      const tH = isNaN(parsedTH) ? curH + minClearance : parsedTH;
      const layerVal = genTargetLayerSelect.value;

      let targetLayer: Layer;
      let newCreatedLayer: Layer | undefined;

      if (layerVal === "__create_above__" || layerVal === "__create_below__") {
        const sign = tH > 0 ? "+" : "";
        const defaultName = (tH >= curH)
          ? (isDe ? `Bergebene (${sign}${Math.round(tH)} mm)` : `Mountain Level (${sign}${Math.round(tH)} mm)`)
          : (isDe ? `Schattenbahnhof (${Math.round(tH)} mm)` : `Staging Yard (${Math.round(tH)} mm)`);
        newCreatedLayer = { id: `layer-${Date.now()}`, name: defaultName, visible: true, locked: false, heightMm: Math.round(tH) };
        targetLayer = newCreatedLayer;
      } else {
        targetLayer = layers.find((l) => l.id === layerVal) || currentLayer;
      }

      closePanel();
      onApplyGenerate?.(latestGenerateResult.pieces.map((p) => ({ ...p, layerId: targetLayer.id })), newCreatedLayer);
    }
  });

  return panel;
}
