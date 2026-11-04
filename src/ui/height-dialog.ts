import type { Layout, Layer, TrackPiece } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import { getTranslations } from "@/i18n";
import { calculateRamp, calculateEndHeightFromMaxGradient, type RampProfile } from "@/core/ramp";
import { openLayerDialog } from "./layer-dialog";

export interface ChangeHeightDialogOptions {
  layout: Layout;
  catalog: Catalog;
  selectedPieceIds: string[];
  preselectedLayerId?: string;
  onApply: (
    targetLayerId: string,
    elevation?: { start: number; end: number } | Map<string, { start: number; end: number }>,
  ) => void;
  onCreateLayer?: (name: string, heightMm: number) => Layer;
}

export function openChangeHeightDialog(options: ChangeHeightDialogOptions) {
  const { layout, catalog, selectedPieceIds, onApply } = options;

  if (selectedPieceIds.length === 0) {
    alert("Please select one or more tracks first.");
    return;
  }

  const existingModal = document.getElementById("height-modal");
  if (existingModal) existingModal.remove();

  const layers = layout.layers || [
    { id: "layer-1", name: "Ground Level / Ebene 1", visible: true, locked: false, heightMm: 0 },
  ];

  // Find selected pieces
  const selectedPieces = layout.pieces.filter((p) => selectedPieceIds.includes(p.id));
  const firstPiece = selectedPieces[0];
  const currentLayerId = firstPiece?.layerId || layout.activeLayerId || layers[0]?.id;
  const currentLayer = layers.find((l) => l.id === currentLayerId) || layers[0]!;

  const targetLayerId = options.preselectedLayerId || currentLayer.id;
  const initialTargetLayer = layers.find((l) => l.id === targetLayerId) || currentLayer;
  const currentBaseHeight = currentLayer.heightMm ?? 0;
  let targetBaseHeight = initialTargetLayer.heightMm ?? 0;

  let rampProfile: RampProfile = "linear";
  let rampReversed = false;

  const defaultStartH = currentBaseHeight + (firstPiece?.elevation?.start ?? 0);
  let defaultEndH = targetBaseHeight !== currentBaseHeight ? targetBaseHeight : defaultStartH + 80;

  const modal = document.createElement("div");
  modal.id = "height-modal";
  modal.className = "rb-modal-overlay";
  const t = getTranslations().heightDialog;

  modal.innerHTML = `
    <style>
      #height-modal {
        position: fixed;
        inset: 0;
        background: rgba(10, 15, 26, 0.7);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 9999;
      }
      .rb-dialog-card {
        width: 440px;
        max-height: 90vh;
        display: flex;
        flex-direction: column;
        background: #1e2638;
        color: #e2e8f0;
        border-radius: 12px;
        overflow: hidden;
        box-shadow: 0 20px 45px rgba(0, 0, 0, 0.5);
        font-size: 13px;
        font-family: inherit;
      }
      .rb-dialog-header {
        padding: 12px 18px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        border-bottom: 1px solid #2d3748;
      }
      .rb-dialog-body {
        padding: 16px 18px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 14px;
        scrollbar-width: thin;
      }
      .rb-dialog-body::-webkit-scrollbar,
      #height-modal #ramp-piece-list::-webkit-scrollbar {
        width: 6px;
      }
      .rb-dialog-body::-webkit-scrollbar-thumb,
      #height-modal #ramp-piece-list::-webkit-scrollbar-thumb {
        background: rgba(148, 163, 184, 0.35);
        border-radius: 3px;
      }
      .rb-meta-strip {
        display: flex;
        justify-content: space-between;
        background: #141b2a;
        padding: 8px 12px;
        border-radius: 6px;
        font-size: 12px;
        color: #94a3b8;
      }
      .rb-meta-strip strong { color: #f8fafc; }
      
      /* Segmented 3-Way Mode Switch */
      .rb-segmented-control {
        display: grid;
        grid-template-columns: 1fr 1fr 1fr;
        background: #141b2a;
        border-radius: 8px;
        padding: 3px;
        gap: 3px;
      }
      .rb-segmented-item {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 7px 4px;
        cursor: pointer;
        border-radius: 6px;
        font-size: 12px;
        color: #94a3b8;
        transition: all 0.15s ease;
        user-select: none;
      }
      .rb-segmented-item input { display: none; }
      .rb-segmented-item.active {
        background: #0284c7;
        color: #ffffff;
        font-weight: 600;
      }

      .ramp-field-input, .height-field-select {
        background: #141b2a;
        border: 1px solid #334155;
        color: #f8fafc;
        border-radius: 6px;
        padding: 6px 10px;
        font-size: 13px;
        width: 100%;
        box-sizing: border-box;
      }
      .btn-icon-subtle {
        background: #242f44;
        border: 1px solid #3b4963;
        color: #cbd5e1;
        border-radius: 6px;
        padding: 6px 10px;
        cursor: pointer;
        font-size: 12.5px;
        transition: background 0.15s;
      }
      .btn-icon-subtle:hover { background: #323f5b; color: #fff; }
      .btn-icon-subtle.active {
        background: #0284c7;
        border-color: #0284c7;
        color: #ffffff;
      }

      .ramp-inputs-grid {
        display: grid;
        grid-template-columns: 1fr auto 1fr auto;
        gap: 8px;
        align-items: center;
      }
      .steep-row {
        display: flex;
        align-items: center;
        gap: 8px;
        justify-content: space-between;
      }
      .pill-btn {
        background: #242f44;
        border: 1px solid transparent;
        color: #94a3b8;
        padding: 3px 8px;
        border-radius: 4px;
        font-size: 11px;
        cursor: pointer;
      }
      .pill-btn.active {
        background: #0284c7;
        color: #fff;
      }

      /* Status Badge */
      .badge-status {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 12px;
        border-radius: 6px;
        font-weight: 500;
        font-size: 12px;
      }
      .badge-status.ok { background: rgba(34, 197, 94, 0.15); color: #4ade80; border: 1px solid rgba(34, 197, 94, 0.3); }
      .badge-status.warn { background: rgba(245, 158, 11, 0.15); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.3); }
      .badge-status.danger { background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3); }

      /* Collapsible Details */
      details.ramp-breakdown-details {
        background: #141b2a;
        border-radius: 6px;
        padding: 8px 12px;
        font-size: 12px;
      }
      details.ramp-breakdown-details summary {
        cursor: pointer;
        color: #94a3b8;
        font-weight: 500;
        user-select: none;
      }
      .ramp-piece-list {
        max-height: 140px;
        overflow-y: auto;
        margin-top: 8px;
        display: flex;
        flex-direction: column;
        gap: 4px;
        scrollbar-width: thin;
      }
      .ramp-piece-row {
        display: flex;
        justify-content: space-between;
        font-size: 11.5px;
        color: #cbd5e1;
        padding: 2px 0;
      }

      .rb-dialog-footer {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
        padding: 12px 18px;
        border-top: 1px solid #2d3748;
        background: #171e2e;
      }
    </style>

    <div class="rb-dialog-card">
      <div class="rb-dialog-header">
        <strong>📐 ${t.title}</strong>
        <button class="btn-icon-subtle" id="height-modal-close" style="padding: 2px 7px;">✕</button>
      </div>

      <div class="rb-dialog-body">
        <!-- Selection Summary -->
        <div class="rb-meta-strip">
          <span><strong>${selectedPieces.length}</strong> tracks</span>
          <span id="summary-total-length">...</span>
          <span>${currentLayer.name.replace(/\s*\([0-9]+\s*mm\)$/, "")}</span>
        </div>

        <!-- Target Layer -->
        <div>
          <label style="display:block; font-size:11px; color:#94a3b8; margin-bottom:5px;">${t.targetLayer}</label>
          <select id="height-layer-select" class="height-field-select">
            ${layers
              .map((l) => {
                const cleanName = l.name.replace(/\s*\([0-9]+\s*mm\)$/, "");
                const isSel = l.id === targetLayerId ? "selected" : "";
                return `<option value="${l.id}" ${isSel}>${cleanName} (${l.heightMm ?? 0} mm)</option>`;
              })
              .join("")}
            <option value="__create_new__">${t.createNewOption}</option>
          </select>
        </div>

        <!-- Segmented Mode Control -->
        <div class="rb-segmented-control">
          <label class="rb-segmented-item active" id="card-mode-match">
            <input type="radio" name="height-mode" value="match" checked />
            Flat on Layer
          </label>
          <label class="rb-segmented-item" id="card-mode-custom">
            <input type="radio" name="height-mode" value="custom" />
            Custom
          </label>
          <label class="rb-segmented-item" id="card-mode-ramp">
            <input type="radio" name="height-mode" value="ramp" />
            Incline (Ramp)
          </label>
        </div>

        <!-- Mode Custom Input -->
        <div id="custom-inputs-row" style="display: none;">
          <label style="display:block; font-size:11px; color:#94a3b8; margin-bottom:5px;">Elevation Height (mm)</label>
          <input type="number" id="input-custom-height" value="${targetBaseHeight}" step="1" class="ramp-field-input" />
        </div>

        <!-- Mode Ramp Controls -->
        <div id="ramp-inputs-row" style="display: none; flex-direction: column; gap: 12px;">
          <!-- Start / End Row -->
          <div class="ramp-inputs-grid">
            <div>
              <span style="font-size:11px; color:#94a3b8;">${t.rampStart}</span>
              <input type="number" id="input-ramp-start" value="${defaultStartH}" step="1" class="ramp-field-input" />
            </div>
            <span style="color:#64748b; font-size:14px; margin-top:14px;">➔</span>
            <div>
              <span style="font-size:11px; color:#94a3b8;">${t.rampEnd}</span>
              <input type="number" id="input-ramp-end" value="${defaultEndH}" step="1" class="ramp-field-input" />
            </div>
            <button type="button" class="btn-icon-subtle" id="btn-ramp-reverse" title="Reverse Direction" style="margin-top:14px;">⇄</button>
          </div>

          <!-- Max Gradient and Auto-Fit -->
          <div class="steep-row">
            <div style="display:flex; align-items:center; gap:6px;">
              <span style="color:#94a3b8; font-size:11px;">Max Slope:</span>
              <input type="number" id="input-target-max-steep" value="2.5" step="0.1" style="width:52px; padding:3px 6px; background:#141b2a; border:1px solid #334155; color:#fff; border-radius:4px;" />
              <span style="color:#94a3b8; font-size:11px;">%</span>
            </div>
            <div style="display:flex; gap:4px;">
              <button type="button" class="pill-btn" data-steep="2.0">2.0%</button>
              <button type="button" class="pill-btn active" data-steep="2.5">2.5%</button>
              <button type="button" class="pill-btn" data-steep="3.0">3.0%</button>
            </div>
            <label style="font-size:11px; display:inline-flex; align-items:center; gap:4px; cursor:pointer; color:#94a3b8; user-select:none;">
              <input type="checkbox" id="chk-auto-clamp-steep" checked /> Auto-Fit
            </label>
          </div>

          <!-- Profile Selectors -->
          <div style="display:grid; grid-template-columns:1fr 1fr; gap:6px;">
            <button type="button" class="btn-icon-subtle active" id="btn-profile-linear">Linear</button>
            <button type="button" class="btn-icon-subtle" id="btn-profile-eased">Eased (S-Curve)</button>
          </div>

          <!-- Live Slope Badge -->
          <div class="badge-status ok" id="ramp-gradient-calc">
            <span id="ramp-gradient-text">Slope: 0.0%</span>
            <span id="ramp-gradient-badge" style="font-size:11px;">Gentle</span>
          </div>

          <!-- Collapsible Pieces List -->
          <details class="ramp-breakdown-details">
            <summary id="ramp-chain-title-text">▸ Piece breakdown (${selectedPieces.length} tracks)</summary>
            <div class="ramp-piece-list" id="ramp-piece-list"></div>
          </details>
        </div>
      </div>

      <div class="rb-dialog-footer">
        <button class="btn-icon-subtle" id="height-btn-cancel">${t.cancelBtn}</button>
        <button class="rb-btn rb-btn-primary" id="height-btn-apply" style="background:#0284c7; color:#fff; border:none; padding:7px 16px; border-radius:6px; cursor:pointer; font-weight:600;">
          <span id="apply-btn-label">${t.applyBtn}</span>
        </button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#height-modal-close") as HTMLButtonElement;
  const cancelBtn = modal.querySelector("#height-btn-cancel") as HTMLButtonElement;
  const applyBtn = modal.querySelector("#height-btn-apply") as HTMLButtonElement;
  const layerSelect = modal.querySelector("#height-layer-select") as HTMLSelectElement;
  const applyLabel = modal.querySelector("#apply-btn-label") as HTMLElement;

  const cardMatch = modal.querySelector("#card-mode-match") as HTMLElement;
  const cardCustom = modal.querySelector("#card-mode-custom") as HTMLElement;
  const cardRamp = modal.querySelector("#card-mode-ramp") as HTMLElement;

  const customInputsRow = modal.querySelector("#custom-inputs-row") as HTMLElement;
  const rampInputsRow = modal.querySelector("#ramp-inputs-row") as HTMLElement;
  const summaryTotalLength = modal.querySelector("#summary-total-length") as HTMLElement;

  const rampGradCalc = modal.querySelector("#ramp-gradient-calc") as HTMLElement;
  const rampGradText = modal.querySelector("#ramp-gradient-text") as HTMLElement;
  const rampGradBadge = modal.querySelector("#ramp-gradient-badge") as HTMLElement;

  const btnProfileLinear = modal.querySelector("#btn-profile-linear") as HTMLButtonElement;
  const btnProfileEased = modal.querySelector("#btn-profile-eased") as HTMLButtonElement;
  const btnRampReverse = modal.querySelector("#btn-ramp-reverse") as HTMLButtonElement;
  const rampPieceList = modal.querySelector("#ramp-piece-list") as HTMLElement;
  const rampChainTitleText = modal.querySelector("#ramp-chain-title-text") as HTMLElement;

  const inputTargetSteep = modal.querySelector("#input-target-max-steep") as HTMLInputElement;
  const chkAutoClampSteep = modal.querySelector("#chk-auto-clamp-steep") as HTMLInputElement;

  const inputCustom = modal.querySelector("#input-custom-height") as HTMLInputElement;
  const inputRampStart = modal.querySelector("#input-ramp-start") as HTMLInputElement;
  const inputRampEnd = modal.querySelector("#input-ramp-end") as HTMLInputElement;

  let lastTotalLengthMm = 0;

  const closeModal = () => modal.remove();
  closeBtn.addEventListener("click", closeModal);
  cancelBtn.addEventListener("click", closeModal);

  btnProfileLinear.addEventListener("click", (e) => {
    e.stopPropagation();
    rampProfile = "linear";
    btnProfileLinear.classList.add("active");
    btnProfileEased.classList.remove("active");
    updateModeCards();
  });

  btnProfileEased.addEventListener("click", (e) => {
    e.stopPropagation();
    rampProfile = "eased";
    btnProfileEased.classList.add("active");
    btnProfileLinear.classList.remove("active");
    updateModeCards();
  });

  btnRampReverse.addEventListener("click", (e) => {
    e.stopPropagation();
    rampReversed = !rampReversed;
    const s = inputRampStart.value;
    inputRampStart.value = inputRampEnd.value;
    inputRampEnd.value = s;
    updateModeCards();
  });

  const updateModeCards = () => {
    const mode = modal.querySelector<HTMLInputElement>("input[name='height-mode']:checked")?.value || "match";

    cardMatch.classList.toggle("active", mode === "match");
    cardCustom.classList.toggle("active", mode === "custom");
    cardRamp.classList.toggle("active", mode === "ramp");

    customInputsRow.style.display = mode === "custom" ? "block" : "none";
    rampInputsRow.style.display = mode === "ramp" ? "flex" : "none";

    const selLayerId = layerSelect.value;
    const selLayer = layers.find((l) => l.id === selLayerId) || initialTargetLayer;
    const selHeight = selLayer.heightMm ?? 0;

    let sH = parseFloat(inputRampStart.value) || 0;
    let eH = parseFloat(inputRampEnd.value) || 0;
    const targetSteep = parseFloat(inputTargetSteep.value) || 2.5;

    // Sync quick preset pills
    modal.querySelectorAll<HTMLButtonElement>(".pill-btn").forEach((btn) => {
      const val = parseFloat(btn.dataset.steep || "0");
      btn.classList.toggle("active", Math.abs(val - targetSteep) < 0.05);
    });

    let rampResult = calculateRamp({
      pieces: selectedPieces,
      catalog,
      startHeightMm: sH,
      endHeightMm: eH,
      layerBaseHeightMm: selHeight,
      profile: rampProfile,
      reversed: rampReversed,
      targetMaxGradientPercent: targetSteep,
    });

    lastTotalLengthMm = rampResult.totalLengthMm;
    summaryTotalLength.textContent = `${(rampResult.totalLengthMm / 10).toFixed(1)} cm`;

    // Auto-fit calculation when enabled and maximum threshold exceeded
    if (chkAutoClampSteep?.checked && rampResult.exceedsTargetMax) {
      const dir: 1 | -1 = eH < sH ? -1 : 1;
      const fittedEndH = Math.round(
        calculateEndHeightFromMaxGradient({
          startHeightMm: sH,
          totalLengthMm: rampResult.totalLengthMm,
          targetMaxGradientPercent: targetSteep,
          profile: rampProfile,
          direction: dir,
        }),
      );

      eH = fittedEndH;
      inputRampEnd.value = String(fittedEndH);

      rampResult = calculateRamp({
        pieces: selectedPieces,
        catalog,
        startHeightMm: sH,
        endHeightMm: eH,
        layerBaseHeightMm: selHeight,
        profile: rampProfile,
        reversed: rampReversed,
        targetMaxGradientPercent: targetSteep,
      });
    }

    if (mode === "match") {
      applyLabel.textContent = `Set to ${selLayer.name} (${selHeight} mm)`;
    } else if (mode === "custom") {
      const customH = parseFloat(inputCustom.value) || 0;
      applyLabel.textContent = `Set to ${customH} mm`;
    } else {
      applyLabel.textContent = `Apply Ramp (${sH} → ${eH} mm)`;

      const deltaH = Math.abs(eH - sH);
      const gradient = rampResult.averageGradientPercent;
      const maxGradient = rampResult.maxGradientPercent;

      rampGradText.textContent = `Slope: ${gradient.toFixed(1)}% (Δ ${deltaH} mm)`;

      const evalGrad = Math.max(gradient, maxGradient);
      if (evalGrad <= 3.0) {
        rampGradBadge.textContent = "Safe (< 3.0%)";
        rampGradCalc.className = "badge-status ok";
      } else if (evalGrad <= 4.5) {
        rampGradBadge.textContent = "Moderate (3-4.5%)";
        rampGradCalc.className = "badge-status warn";
      } else {
        rampGradBadge.textContent = "Steep (> 4.5%)";
        rampGradCalc.className = "badge-status danger";
      }

      rampChainTitleText.textContent = `▸ View breakdown (${selectedPieces.length} tracks, ${rampResult.isContinuous ? "connected" : "split"})`;

      rampPieceList.innerHTML = rampResult.chain
        .map((info, idx) => {
          const h1Str = info.entryHeightMm >= 0 ? `+${info.entryHeightMm}` : `${info.entryHeightMm}`;
          const h2Str = info.exitHeightMm >= 0 ? `+${info.exitHeightMm}` : `${info.exitHeightMm}`;
          return `
            <div class="ramp-piece-row">
              <span>${idx + 1}. #${info.piece.catalogId} (${Math.round(info.lengthMm)} mm)</span>
              <span>${h1Str} ➔ ${h2Str} mm (${info.gradientPercent.toFixed(1)}%)</span>
            </div>
          `;
        })
        .join("");
    }
  };

  modal.querySelectorAll("input[name='height-mode']").forEach((radio) => {
    radio.addEventListener("change", updateModeCards);
  });

  inputCustom.addEventListener("input", updateModeCards);
  inputRampStart.addEventListener("input", updateModeCards);
  inputRampEnd.addEventListener("input", updateModeCards);
  inputTargetSteep.addEventListener("input", updateModeCards);
  chkAutoClampSteep.addEventListener("change", updateModeCards);

  modal.querySelectorAll<HTMLButtonElement>(".pill-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const val = parseFloat(btn.dataset.steep || "2.5") || 2.5;
      inputTargetSteep.value = val.toFixed(1);
      updateModeCards();
    });
  });

  layerSelect.addEventListener("change", () => {
    if (layerSelect.value === "__create_new__") {
      openLayerDialog({
        layout,
        targetHeightMm: targetBaseHeight + 80,
        position: "above",
        onApply: (newLayer) => {
          layers.push(newLayer);
          if (!layout.layers) layout.layers = layers;

          layerSelect.innerHTML = `
            ${layers
              .map((l) => `<option value="${l.id}" ${l.id === newLayer.id ? "selected" : ""}>${l.name.replace(/\s*\([0-9]+\s*mm\)$/, "")} (${l.heightMm ?? 0} mm)</option>`)
              .join("")}
            <option value="__create_new__">${t.createNewOption}</option>
          `;
          layerSelect.value = newLayer.id;
          const newH = newLayer.heightMm ?? 0;
          inputCustom.value = `${newH}`;
          inputRampEnd.value = `${newH}`;
          updateModeCards();
        },
      });
      layerSelect.value = targetLayerId;
      return;
    } else {
      const sel = layers.find((l) => l.id === layerSelect.value);
      if (sel) {
        inputCustom.value = `${sel.heightMm ?? 0}`;
        inputRampEnd.value = `${sel.heightMm ?? 0}`;
      }
    }
    updateModeCards();
  });

  applyBtn.addEventListener("click", () => {
    const selLayerId = layerSelect.value;
    const selLayer = layers.find((l) => l.id === selLayerId) || initialTargetLayer;
    const selBaseH = selLayer.heightMm ?? 0;
    const mode = modal.querySelector<HTMLInputElement>("input[name='height-mode']:checked")?.value || "match";

    let elevation: { start: number; end: number } | Map<string, { start: number; end: number }> | undefined = undefined;

    if (mode === "match") {
      elevation = { start: 0, end: 0 };
    } else if (mode === "custom") {
      const customH = parseFloat(inputCustom.value) || 0;
      const offset = customH - selBaseH;
      elevation = { start: offset, end: offset };
    } else if (mode === "ramp") {
      const sH = parseFloat(inputRampStart.value) || 0;
      const eH = parseFloat(inputRampEnd.value) || 0;
      const targetSteep = parseFloat(inputTargetSteep.value) || 2.5;

      const rampResult = calculateRamp({
        pieces: selectedPieces,
        catalog,
        startHeightMm: sH,
        endHeightMm: eH,
        layerBaseHeightMm: selBaseH,
        profile: rampProfile,
        reversed: rampReversed,
        targetMaxGradientPercent: targetSteep,
      });

      elevation = rampResult.pieceElevations;
    }

    closeModal();
    onApply(selLayer.id, elevation);
  });

  updateModeCards();
}