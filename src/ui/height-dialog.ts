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

  // Finde die selektierten Stücke
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
    <div class="rb-modal-card height-modal-card">
      <div class="rb-modal-header">
        <div class="rb-modal-title">
          <span>📐</span>
          <span>${t.title}</span>
        </div>
        <button class="rb-modal-close-btn" id="height-modal-close">✕</button>
      </div>

      <div class="height-modal-body">
        <!-- Zusammenfassung der Selektion -->
        <div class="height-summary-box">
          <div class="height-summary-item">
            <span class="height-summary-val">${selectedPieces.length}</span>
            <span class="height-summary-lbl">${t.selectedCount}</span>
          </div>
          <div class="height-summary-item">
            <span class="height-summary-val" id="summary-total-length">...</span>
            <span class="height-summary-lbl">${t.trackLength}</span>
          </div>
          <div class="height-summary-item">
            <span class="height-summary-val">${currentLayer.name}</span>
            <span class="height-summary-lbl">${t.currentLayer} (${currentBaseHeight} mm)</span>
          </div>
        </div>

        <!-- 1. Ziel-Ebene auswählen -->
        <div class="height-field-group">
          <label class="height-field-label">${t.targetLayer}</label>
          <div class="height-layer-select-row">
            <select id="height-layer-select" class="height-input-select">
              ${layers
                .map((l) => {
                  const h = l.heightMm ?? 0;
                  const hStr = h > 0 ? `+${h} mm` : `${h} mm`;
                  const isSel = l.id === targetLayerId ? "selected" : "";
                  return `<option value="${l.id}" ${isSel}>${l.name} (${hStr})</option>`;
                })
                .join("")}
              <option value="__create_new__">${t.createNewOption}</option>
            </select>
          </div>
        </div>

        <!-- 2. Höhenmodus wählen -->
        <div class="height-field-group">
          <label class="height-field-label">${t.adjustmentLabel}</label>
          
          <div class="height-mode-options">
            <!-- Modus A: Ziel-Ebenen-Höhe übernehmen (Flat) -->
            <label class="height-mode-card active" id="card-mode-match">
              <input type="radio" name="height-mode" value="match" checked />
              <div class="height-mode-content">
                <strong>${t.modeMatch}</strong>
                <span class="height-mode-desc" id="desc-match-height">${t.modeMatchDesc.replace("{height}", String(targetBaseHeight))}</span>
              </div>
            </label>

            <!-- Modus B: Eigene absolute Höhe -->
            <label class="height-mode-card" id="card-mode-custom">
              <input type="radio" name="height-mode" value="custom" />
              <div class="height-mode-content">
                <strong>${t.modeCustom}</strong>
                <span class="height-mode-desc">${t.modeCustomDesc}</span>
                <div class="height-inline-inputs" id="custom-inputs-row" style="display: none;">
                  <input type="number" id="input-custom-height" value="${targetBaseHeight}" step="1" class="height-num-input" />
                  <span class="unit-tag">mm</span>
                </div>
              </div>
            </label>

            <!-- Modus C: Steigung / Rampe (Incline mit Dreisatz & Ausrundung) -->
            <label class="height-mode-card" id="card-mode-ramp">
              <input type="radio" name="height-mode" value="ramp" />
              <div class="height-mode-content">
                <strong>${t.modeRamp}</strong>
                <span class="height-mode-desc">${t.modeRampDesc}</span>
                
                <div class="height-ramp-inputs" id="ramp-inputs-row" style="display: none;">
                  <div class="ramp-col">
                    <label>${t.rampStart}</label>
                    <div class="ramp-input-wrap">
                      <input type="number" id="input-ramp-start" value="${defaultStartH}" step="1" class="height-num-input" />
                      <span class="unit-tag">mm</span>
                    </div>
                  </div>
                  <span class="ramp-arrow">➔</span>
                  <div class="ramp-col">
                    <label>${t.rampEnd}</label>
                    <div class="ramp-input-wrap">
                      <input type="number" id="input-ramp-end" value="${defaultEndH}" step="1" class="height-num-input" />
                      <span class="unit-tag">mm</span>
                    </div>
                  </div>
                </div>

                <!-- Rampen-Optionen: Linear vs. Ausrundung & Richtung -->
                <div class="ramp-options-row" id="ramp-options-row" style="display: none;">
                  <div class="ramp-profile-selector">
                    <button type="button" class="ramp-profile-btn active" id="btn-profile-linear" title="${t.rampProfileLinearDesc}">
                      📏 <span>${t.rampProfileLinear}</span>
                    </button>
                    <button type="button" class="ramp-profile-btn" id="btn-profile-eased" title="${t.rampProfileEasedDesc}">
                      〰️ <span>${t.rampProfileEased}</span>
                    </button>
                  </div>
                  <button type="button" class="ramp-reverse-btn" id="btn-ramp-reverse" title="${t.reverseRampDirection}">
                    <span>${t.reverseRampDirection}</span>
                  </button>
                </div>

                <!-- Ziel-Maximalsteigung -->
                <div class="ramp-target-steep-box" id="ramp-target-steep-box" style="display: none;">
                  <div class="ramp-target-steep-header">
                    <span>🎯 ${t.targetMaxSteep}</span>
                    <span style="font-size: 9.5px; opacity: 0.8;">${t.targetMaxSteepDesc}</span>
                  </div>
                  <div class="ramp-target-steep-controls">
                    <div class="steep-input-wrap">
                      <input type="number" id="input-target-max-steep" value="2.5" step="0.1" min="0.5" max="15.0" />
                      <span class="steep-unit">%</span>
                    </div>
                    <div class="steep-presets">
                      <button type="button" class="steep-preset-btn" data-steep="2.0">2.0%</button>
                      <button type="button" class="steep-preset-btn active" data-steep="2.5">2.5%</button>
                      <button type="button" class="steep-preset-btn" data-steep="3.0">3.0%</button>
                      <button type="button" class="steep-preset-btn" data-steep="3.5">3.5%</button>
                    </div>
                    <button type="button" class="ramp-apply-steep-btn" id="btn-apply-max-steep" title="${t.targetMaxSteepDesc}">
                      <span>✨</span>
                      <span>${t.applyTargetSteep}</span>
                    </button>
                  </div>
                  <div class="ramp-steep-warning-box" id="ramp-steep-warning-box" style="display: none;">
                    <div class="steep-warning-header">
                      <span id="steep-warning-text"></span>
                      <button type="button" class="steep-clamp-btn" id="btn-clamp-steep">${t.clampToTarget}</button>
                    </div>
                    <div class="steep-req-length" id="steep-req-length"></div>
                  </div>
                </div>

                <!-- Steigungs-Indikator -->
                <div class="ramp-gradient-calc" id="ramp-gradient-calc" style="display: none;">
                  <span id="ramp-gradient-text">${t.rampSlope} 0.0%</span>
                  <span id="ramp-gradient-badge" class="gradient-badge ok">Gentle Incline</span>
                </div>

                <!-- Stück-Vorschau (Dreisatz / Kette) -->
                <div class="ramp-chain-box" id="ramp-chain-box" style="display: none;">
                  <div class="ramp-chain-title">
                    <span id="ramp-chain-title-text">${t.rampPieceBreakdown}</span>
                    <span id="ramp-continuity-badge"></span>
                  </div>
                  <div class="ramp-piece-list" id="ramp-piece-list"></div>
                </div>
              </div>
            </label>
          </div>
        </div>
      </div>

      <div class="rb-modal-footer">
        <button class="rb-btn rb-btn-secondary" id="height-btn-cancel">${t.cancelBtn}</button>
        <button class="rb-btn rb-btn-primary" id="height-btn-apply">
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
  const rampOptionsRow = modal.querySelector("#ramp-options-row") as HTMLElement;
  const rampCalcRow = modal.querySelector("#ramp-gradient-calc") as HTMLElement;
  const rampGradText = modal.querySelector("#ramp-gradient-text") as HTMLElement;
  const rampGradBadge = modal.querySelector("#ramp-gradient-badge") as HTMLElement;
  const descMatchHeight = modal.querySelector("#desc-match-height") as HTMLElement;
  const summaryTotalLength = modal.querySelector("#summary-total-length") as HTMLElement;

  const btnProfileLinear = modal.querySelector("#btn-profile-linear") as HTMLButtonElement;
  const btnProfileEased = modal.querySelector("#btn-profile-eased") as HTMLButtonElement;
  const btnRampReverse = modal.querySelector("#btn-ramp-reverse") as HTMLButtonElement;
  const rampChainBox = modal.querySelector("#ramp-chain-box") as HTMLElement;
  const rampPieceList = modal.querySelector("#ramp-piece-list") as HTMLElement;
  const rampContinuityBadge = modal.querySelector("#ramp-continuity-badge") as HTMLElement;

  const rampTargetSteepBox = modal.querySelector("#ramp-target-steep-box") as HTMLElement;
  const inputTargetSteep = modal.querySelector("#input-target-max-steep") as HTMLInputElement;
  const btnApplyMaxSteep = modal.querySelector("#btn-apply-max-steep") as HTMLButtonElement;
  const rampSteepWarningBox = modal.querySelector("#ramp-steep-warning-box") as HTMLElement;
  const steepWarningText = modal.querySelector("#steep-warning-text") as HTMLElement;
  const btnClampSteep = modal.querySelector("#btn-clamp-steep") as HTMLButtonElement;
  const steepReqLength = modal.querySelector("#steep-req-length") as HTMLElement;

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
    updateModeCards();
  });

  // Modus-Umschaltung
  const updateModeCards = () => {
    const mode = modal.querySelector<HTMLInputElement>("input[name='height-mode']:checked")?.value || "match";

    cardMatch.classList.toggle("active", mode === "match");
    cardCustom.classList.toggle("active", mode === "custom");
    cardRamp.classList.toggle("active", mode === "ramp");

    customInputsRow.style.display = mode === "custom" ? "flex" : "none";
    rampInputsRow.style.display = mode === "ramp" ? "flex" : "none";
    rampOptionsRow.style.display = mode === "ramp" ? "flex" : "none";
    rampTargetSteepBox.style.display = mode === "ramp" ? "flex" : "none";
    rampCalcRow.style.display = mode === "ramp" ? "flex" : "none";
    rampChainBox.style.display = mode === "ramp" ? "block" : "none";

    const selLayerId = layerSelect.value;
    const selLayer = layers.find((l) => l.id === selLayerId) || initialTargetLayer;
    const selHeight = selLayer.heightMm ?? 0;

    descMatchHeight.textContent = t.modeMatchDesc.replace("{height}", String(selHeight));

    // Ramp calculation
    const sH = parseFloat(inputRampStart.value) || 0;
    const eH = parseFloat(inputRampEnd.value) || 0;
    const targetSteep = parseFloat(inputTargetSteep.value) || 2.5;

    // Update preset pills active state
    modal.querySelectorAll<HTMLButtonElement>(".steep-preset-btn").forEach((btn) => {
      const val = parseFloat(btn.dataset.steep || "0");
      btn.classList.toggle("active", Math.abs(val - targetSteep) < 0.05);
    });

    const rampResult = calculateRamp({
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

    if (mode === "match") {
      applyLabel.textContent = t.applyMoveTo.replace("{layer}", selLayer.name).replace("{height}", String(selHeight));
    } else if (mode === "custom") {
      const customH = parseFloat(inputCustom.value) || 0;
      applyLabel.textContent = t.applyCustomHeight.replace("{height}", String(customH));
    } else {
      applyLabel.textContent = t.applyRampLabel.replace("{start}", String(sH)).replace("{end}", String(eH));

      // Steigungsberechnung
      const deltaH = Math.abs(eH - sH);
      const gradient = rampResult.averageGradientPercent;
      const maxGradient = rampResult.maxGradientPercent;

      let gradSummary = t.gradientCalcText
        .replace("{gradient}", gradient.toFixed(1))
        .replace("{delta}", String(deltaH))
        .replace("{total}", String(Math.round(rampResult.totalLengthMm)));

      if (rampProfile === "eased" && Math.abs(maxGradient - gradient) >= 0.2) {
        gradSummary += ` [Max: ${maxGradient.toFixed(1)}%]`;
      }

      rampGradText.textContent = gradSummary;

      const evalGrad = Math.max(gradient, maxGradient);
      if (evalGrad <= 3.0) {
        rampGradBadge.textContent = t.badgeRecommended;
        rampGradBadge.className = "gradient-badge ok";
      } else if (evalGrad <= 4.5) {
        rampGradBadge.textContent = t.badgeModerate;
        rampGradBadge.className = "gradient-badge warn";
      } else {
        rampGradBadge.textContent = t.badgeSteep;
        rampGradBadge.className = "gradient-badge danger";
      }

      // Ziel-Maximalsteigung Warnung & Anpassen
      if (rampResult.exceedsTargetMax) {
        rampSteepWarningBox.style.display = "flex";
        steepWarningText.textContent = t.steepExceededWarning
          .replace("{current}", rampResult.maxGradientPercent.toFixed(1))
          .replace("{target}", targetSteep.toFixed(1));
        btnClampSteep.textContent = t.clampToTarget.replace("{target}", targetSteep.toFixed(1));

        const avgPieceLen = rampResult.totalLengthMm / (selectedPieces.length || 1);
        const approxPieces = Math.ceil((rampResult.requiredLengthMm || 0) / (avgPieceLen > 0 ? avgPieceLen : 188.3));
        steepReqLength.textContent = t.requiredLengthTip
          .replace("{delta}", String(deltaH))
          .replace("{target}", targetSteep.toFixed(1))
          .replace("{length}", ((rampResult.requiredLengthMm || 0) / 10).toFixed(1))
          .replace("{tracks}", String(approxPieces));
      } else {
        rampSteepWarningBox.style.display = "none";
      }

      // Chain Breakdown Rendering
      if (rampResult.isContinuous) {
        rampContinuityBadge.textContent = "✔ Kette verbunden";
        rampContinuityBadge.style.color = "#16a34a";
      } else {
        rampContinuityBadge.textContent = "ℹ Teilweise unverbunden";
        rampContinuityBadge.style.color = "#f59e0b";
      }

      rampPieceList.innerHTML = rampResult.chain
        .map((info, idx) => {
          let gradClass = "ok";
          if (info.gradientPercent > 4.5) gradClass = "danger";
          else if (info.gradientPercent > 3.0) gradClass = "warn";

          const h1Str = info.entryHeightMm >= 0 ? `+${info.entryHeightMm}` : `${info.entryHeightMm}`;
          const h2Str = info.exitHeightMm >= 0 ? `+${info.exitHeightMm}` : `${info.exitHeightMm}`;

          return `
            <div class="ramp-piece-row">
              <span>${idx + 1}. #${info.piece.catalogId} (${Math.round(info.lengthMm)} mm)</span>
              <span>${h1Str} ➔ ${h2Str} mm <span class="grad ${gradClass}">(${info.gradientPercent.toFixed(1)}%)</span></span>
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

  modal.querySelectorAll<HTMLButtonElement>(".steep-preset-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const val = parseFloat(btn.dataset.steep || "2.5") || 2.5;
      inputTargetSteep.value = val.toFixed(1);
      updateModeCards();
    });
  });

  const applyMaxSteepHandler = (e: Event) => {
    e.stopPropagation();
    const sH = parseFloat(inputRampStart.value) || 0;
    const eH = parseFloat(inputRampEnd.value) || 0;
    const targetSteep = parseFloat(inputTargetSteep.value) || 2.5;
    const dir: 1 | -1 = eH < sH ? -1 : 1;
    const newEndH = calculateEndHeightFromMaxGradient({
      startHeightMm: sH,
      totalLengthMm: lastTotalLengthMm,
      targetMaxGradientPercent: targetSteep,
      profile: rampProfile,
      direction: dir,
    });
    inputRampEnd.value = String(newEndH);
    updateModeCards();
  };

  btnApplyMaxSteep.addEventListener("click", applyMaxSteepHandler);
  btnClampSteep.addEventListener("click", applyMaxSteepHandler);

  // Layer Select Change
  layerSelect.addEventListener("change", () => {
    if (layerSelect.value === "__create_new__") {
      openLayerDialog({
        layout,
        targetHeightMm: targetBaseHeight + 80,
        position: "above",
        onApply: (newLayer) => {
          layers.push(newLayer);
          if (!layout.layers) layout.layers = layers;

          // Dropdown aktualisieren
          layerSelect.innerHTML = `
            ${layers
              .map((l) => `<option value="${l.id}" ${l.id === newLayer.id ? "selected" : ""}>${l.name} (${l.heightMm ?? 0} mm)</option>`)
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
      // Fallback falls der Dialog abgebrochen wird
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

  // Apply Action
  applyBtn.addEventListener("click", () => {
    const selLayerId = layerSelect.value;
    const selLayer = layers.find((l) => l.id === selLayerId) || initialTargetLayer;
    const selBaseH = selLayer.heightMm ?? 0;
    const mode = modal.querySelector<HTMLInputElement>("input[name='height-mode']:checked")?.value || "match";

    let elevation: { start: number; end: number } | Map<string, { start: number; end: number }> | undefined = undefined;

    if (mode === "match") {
      // Flach auf Ebene: Relatives Offset = 0
      elevation = { start: 0, end: 0 };
    } else if (mode === "custom") {
      // Eigene absolute Höhe: Relatives Offset = CustomH - BaseH
      const customH = parseFloat(inputCustom.value) || 0;
      const offset = customH - selBaseH;
      elevation = { start: offset, end: offset };
    } else if (mode === "ramp") {
      // Berechne proportionale Höhenverteilung per Dreisatz / Kette für alle Stücke
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
