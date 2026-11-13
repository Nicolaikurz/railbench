import heightDialogHtml from "./templates/height-dialog.html?raw";
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

  const layerOptions = layers
    .map((l) => {
      const cleanName = l.name.replace(/\\s*\\([0-9]+\\s*mm\\)$/, "");
      const isSel = l.id === targetLayerId ? "selected" : "";
      return `<option value="${l.id}" ${isSel}>${cleanName} (${l.heightMm ?? 0} mm)</option>`;
    })
    .join("");

  modal.innerHTML = heightDialogHtml
    .replace("{{TITLE}}", t.title)
    .replace("{{SELECTED_COUNT}}", String(selectedPieces.length))
    .replace("{{CURRENT_LAYER_NAME}}", currentLayer.name.replace(/\\s*\\([0-9]+\\s*mm\\)$/, ""))
    .replace("{{TARGET_LAYER_LABEL}}", t.targetLayer)
    .replace("{{LAYER_OPTIONS}}", layerOptions)
    .replace("{{CREATE_NEW_OPTION}}", t.createNewOption)
    .replace("{{TARGET_BASE_HEIGHT}}", String(targetBaseHeight))
    .replace("{{RAMP_START_LABEL}}", t.rampStart)
    .replace("{{DEFAULT_START_H}}", String(defaultStartH))
    .replace("{{RAMP_END_LABEL}}", t.rampEnd)
    .replace("{{DEFAULT_END_H}}", String(defaultEndH))
    .replace("{{SELECTED_COUNT}}", String(selectedPieces.length))
    .replace("{{CANCEL_BTN}}", t.cancelBtn)
    .replace("{{APPLY_BTN}}", t.applyBtn);

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