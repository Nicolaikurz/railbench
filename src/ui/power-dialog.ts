import type { Layout } from "@/core/types";
import {
  getDefaultLocoCurrentMa,
  MIN_LOCO_CURRENT_MA,
  MAX_LOCO_CURRENT_MA,
  isTurnoutEntry,
} from "@/core/simulation";
import type { Catalog } from "@/catalog/catalog";
import { getTranslations } from "@/i18n";

export interface PowerSettingsDialogOptions {
  layout: Layout;
  catalog?: Catalog;
  initialFocus?: "voltage" | "current" | "catenary" | "turnout";
  onApply: (params: {
    feedVoltage: number;
    locoCurrentMa: number;
    locoCount: number;
    catenaryFeedVoltage?: number;
    catenaryCircuitMode?: "separate" | "parallel";
    turnoutLanternCurrentMa?: number;
    turnoutDecoderCurrentMa?: number;
    bulkTurnoutAction?: "equipAll" | "clearAll";
  }) => void;
}

/**
 * Öffnet ein modales Dialogfenster zur Konfiguration von Versorgungsspannung (Custom Voltage),
 * Oberleitungs-Spannung & Betriebsart (Catenary Circuit),
 * Weichen-Zubehör / Digitaldecoder (Turnout Accessories)
 * und Lok-Stromaufnahme / Gesamtzuglast (Traction Current).
 */
export function openPowerSettingsDialog(options: PowerSettingsDialogOptions): void {
  const { layout, catalog, onApply, initialFocus = "voltage" } = options;
  const t = getTranslations().powerDialog;

  const existing = document.getElementById("power-settings-modal");
  if (existing) existing.remove();

  let currentVoltage = layout.feedVoltage ?? 16.0;
  let currentLocoMa = layout.locoCurrentMa ?? getDefaultLocoCurrentMa(layout.scale);
  currentLocoMa = Math.min(MAX_LOCO_CURRENT_MA, Math.max(MIN_LOCO_CURRENT_MA, currentLocoMa));
  let currentLocoCount = layout.locoCount ?? 1;

  let currentCatenaryVoltage = layout.catenaryFeedVoltage ?? currentVoltage;
  let currentCatenaryMode: "separate" | "parallel" = layout.catenaryCircuitMode ?? "separate";

  // Weichen-Zubehör Parameter
  let currentLanternMa = layout.turnoutLanternCurrentMa ?? 20;
  let currentDecoderMa = layout.turnoutDecoderCurrentMa ?? 15;
  let bulkTurnoutAction: "equipAll" | "clearAll" | undefined = undefined;

  const pieces = layout.pieces || [];
  const catenaryTracks = pieces.filter((p) => !!p.hasCatenary);
  const catenaryFeeders = catenaryTracks.filter((p) => !!p.catenaryPowerFeed);
  const catenaryIsolators = catenaryTracks.filter((p) => !!p.catenaryIsolator);

  // Weichen ermitteln
  const entryMap = new Map();
  if (catalog) {
    for (const e of catalog.entries) {
      entryMap.set(e.id, e);
    }
  }

  let totalTurnoutCount = 0;
  let initialLanternCount = 0;
  let initialDecoderCount = 0;

  for (const p of pieces) {
    const entry = entryMap.get(p.catalogId);
    if (isTurnoutEntry(entry)) {
      totalTurnoutCount++;
      if (p.hasLantern) initialLanternCount++;
      if (p.hasDecoder) initialDecoderCount++;
    }
  }

  let activeLanternCount = initialLanternCount;
  let activeDecoderCount = initialDecoderCount;

  const modal = document.createElement("div");
  modal.id = "power-settings-modal";
  modal.className = "rb-modal-overlay";

  modal.innerHTML = `
    <div class="rb-modal-card power-dialog-card">
      <div class="rb-modal-header">
        <div class="rb-modal-title">
          <span></span>
          <span>${t.title}</span>
        </div>
        <button class="rb-modal-close-btn" id="power-modal-close" title="✕">✕</button>
      </div>

      <div class="power-dialog-body">
        <!-- 1. VERSORGUNGSSPANNUNG -->
        <div class="power-dialog-section">
          <div class="power-dialog-section-header">
            <strong>${t.feederHeader}</strong>
            <span class="power-dialog-sub">${t.feederSub}</span>
          </div>

          <div class="power-dialog-presets-grid">
            <button type="button" class="power-dialog-btn-preset ${Math.abs(currentVoltage - 16) < 0.1 ? "active" : ""}" data-voltage="16">
              <span class="preset-title">16 V</span>
              <span class="preset-desc">${t.preset16VDesc}</span>
            </button>
            <button type="button" class="power-dialog-btn-preset ${Math.abs(currentVoltage - 14) < 0.1 ? "active" : ""}" data-voltage="14">
              <span class="preset-title">14 V</span>
              <span class="preset-desc">${t.preset14VDesc}</span>
            </button>
            <button type="button" class="power-dialog-btn-preset ${Math.abs(currentVoltage - 12) < 0.1 ? "active" : ""}" data-voltage="12">
              <span class="preset-title">12 V</span>
              <span class="preset-desc">${t.preset12VDesc}</span>
            </button>
            <button type="button" class="power-dialog-btn-preset ${Math.abs(currentVoltage - 18) < 0.1 ? "active" : ""}" data-voltage="18">
              <span class="preset-title">18 V</span>
              <span class="preset-desc">${t.preset18VDesc}</span>
            </button>
          </div>

          <div class="power-dialog-input-row">
            <label for="power-input-voltage" class="power-dialog-label">${t.customVoltageLabel}</label>
            <div class="power-dialog-unit-input">
              <input type="number" id="power-input-voltage" min="4" max="28" step="0.5" value="${currentVoltage.toFixed(1)}" class="rb-input power-num-field" />
              <span class="unit-addon">${t.voltageUnit}</span>
            </div>
          </div>
        </div>

        <!-- 2. OBERLEITUNG / CATENARY -->
        <div class="power-dialog-section">
          <div class="power-dialog-section-header">
            <strong>${t.catenaryHeader}</strong>
            <span class="power-dialog-sub">${t.catenarySub}</span>
          </div>

          <div class="power-dialog-stats-badge">
            ${
              catenaryTracks.length > 0
                ? t.catenaryStats
                    .replace("{count}", String(catenaryTracks.length))
                    .replace("{feeders}", String(catenaryFeeders.length))
                    .replace("{isolators}", String(catenaryIsolators.length))
                : t.catenaryNoTracks
            }
          </div>

          <div class="power-dialog-input-row">
            <label for="power-input-catenary-voltage" class="power-dialog-label">${t.catenaryVoltageLabel}</label>
            <div class="power-dialog-unit-input">
              <input type="number" id="power-input-catenary-voltage" min="4" max="28" step="0.5" value="${currentCatenaryVoltage.toFixed(1)}" class="rb-input power-num-field" ${currentCatenaryMode === "parallel" ? "disabled" : ""} />
              <span class="unit-addon">${t.voltageUnit}</span>
            </div>
          </div>

          <div class="power-dialog-mode-row">
            <span class="power-dialog-label">${t.catenaryModeLabel}</span>
            <div class="power-dialog-radio-group">
              <label class="power-radio-label ${currentCatenaryMode === "separate" ? "active" : ""}" id="label-cat-mode-separate">
                <input type="radio" name="catenary-mode" value="separate" ${currentCatenaryMode === "separate" ? "checked" : ""} />
                <span>${t.catenaryModeSeparate}</span>
              </label>
              <label class="power-radio-label ${currentCatenaryMode === "parallel" ? "active" : ""}" id="label-cat-mode-parallel">
                <input type="radio" name="catenary-mode" value="parallel" ${currentCatenaryMode === "parallel" ? "checked" : ""} />
                <span>${t.catenaryModeParallel}</span>
              </label>
            </div>
          </div>
        </div>

        <!-- 3. WEICHEN-ZUBEHÖR & DIGITALDECODER -->
        <div class="power-dialog-section">
          <div class="power-dialog-section-header">
            <strong>${t.turnoutHeader}</strong>
            <span class="power-dialog-sub">${t.turnoutSub}</span>
          </div>

          <div class="power-dialog-stats-badge" style="display: flex; justify-content: space-between; align-items: center; background: rgba(255,255,255,0.04); padding: 8px 12px; border-radius: 6px; margin-bottom: 12px; border: 1px solid rgba(255,255,255,0.08);">
            <span id="power-turnout-stats-text" style="font-size: 13px; font-weight: 500;">
              ${t.turnoutStats.replace("{count}", String(totalTurnoutCount)).replace("{lanterns}", String(activeLanternCount)).replace("{decoders}", String(activeDecoderCount))}
            </span>
            <div style="display: flex; gap: 6px;">
              <button type="button" class="rb-btn rb-btn-secondary" id="power-btn-equip-all-turnouts" style="font-size: 11px; padding: 4px 8px; cursor: pointer;" title="${t.turnoutEquipAllBtn}">
                ${t.turnoutEquipAllBtn}
              </button>
              <button type="button" class="rb-btn rb-btn-secondary" id="power-btn-clear-all-turnouts" style="font-size: 11px; padding: 4px 8px; cursor: pointer;" title="${t.turnoutClearAllBtn}">
                ✕ ${t.turnoutClearAllBtn}
              </button>
            </div>
          </div>

          <div class="power-dialog-slider-box" style="margin-bottom: 12px;">
            <div class="power-dialog-slider-header">
              <label for="power-slider-lantern">${t.turnoutLanternLabel}</label>
              <span class="power-highlight-val" id="power-lbl-lantern-ma"><strong>${currentLanternMa}</strong> mA</span>
            </div>
            <div class="power-dialog-slider-row">
              <input type="range" id="power-slider-lantern" min="5" max="80" step="5" value="${currentLanternMa}" class="power-slider" />
              <div class="power-dialog-unit-input">
                <input type="number" id="power-input-lantern-ma" min="5" max="150" step="5" value="${currentLanternMa}" class="rb-input power-num-field" />
                <span class="unit-addon">mA</span>
              </div>
            </div>
          </div>

          <div class="power-dialog-slider-box">
            <div class="power-dialog-slider-header">
              <label for="power-slider-decoder">${t.turnoutDecoderLabel}</label>
              <span class="power-highlight-val" id="power-lbl-decoder-ma"><strong>${currentDecoderMa}</strong> mA</span>
            </div>
            <div class="power-dialog-slider-row">
              <input type="range" id="power-slider-decoder" min="5" max="60" step="5" value="${currentDecoderMa}" class="power-slider" />
              <div class="power-dialog-unit-input">
                <input type="number" id="power-input-decoder-ma" min="5" max="100" step="5" value="${currentDecoderMa}" class="rb-input power-num-field" />
                <span class="unit-addon">mA</span>
              </div>
            </div>
          </div>
        </div>

        <!-- 4. LOK-STROMVERBRAUCH & RICHTWERTE -->
        <div class="power-dialog-section">
          <div class="power-dialog-section-header">
            <strong>${t.locoHeader}</strong>
            <span class="power-dialog-sub">${t.locoSub}</span>
          </div>

          <div class="power-dialog-presets-grid">
           
          </div>

          <div class="power-dialog-slider-box">
            <div class="power-dialog-slider-header">
              <label for="power-slider-loco">${t.locoCurrentLabel}</label>
              <span class="power-highlight-val" id="power-lbl-loco-ma"><strong>${currentLocoMa}</strong> mA (${(currentLocoMa / 1000).toFixed(2)} A)</span>
            </div>
            <div class="power-dialog-slider-row">
              <input type="range" id="power-slider-loco" min="${MIN_LOCO_CURRENT_MA}" max="${MAX_LOCO_CURRENT_MA}" step="10" value="${currentLocoMa}" class="power-slider" />
              <div class="power-dialog-unit-input">
                <input type="number" id="power-input-loco-ma" min="${MIN_LOCO_CURRENT_MA}" max="${MAX_LOCO_CURRENT_MA}" step="10" value="${currentLocoMa}" class="rb-input power-num-field" />
                <span class="unit-addon">mA</span>
              </div>
            </div>
            <div class="power-dialog-hint">
              ${t.locoHint}
            </div>
          </div>
        </div>

        <!-- 5. FAHRBETRIEB & GESAMTLAST -->
        <div class="power-dialog-section">
          <div class="power-dialog-section-header">
            <strong>${t.trafficHeader}</strong>
            <span class="power-dialog-sub">${t.trafficSub}</span>
          </div>

          <div class="power-dialog-counter-row">
            <label for="power-input-loco-count" class="power-dialog-label">${t.activeLocosLabel}</label>
            <div class="power-dialog-stepper">
              <button type="button" class="power-step-btn" id="power-btn-dec-count" ${currentLocoCount <= 1 ? "disabled" : ""}>−</button>
              <input type="number" id="power-input-loco-count" min="1" max="10" value="${currentLocoCount}" class="rb-input power-count-field" />
              <button type="button" class="power-step-btn" id="power-btn-inc-count" ${currentLocoCount >= 10 ? "disabled" : ""}>+</button>
            </div>
          </div>

          <!-- Gesamtzuglast Box -->
          <div class="power-dialog-summary-card">
            <div class="power-summary-left">
              <span class="summary-lbl">${t.totalCombinedCurrentLabel}</span>
              <span class="summary-sub" id="power-summary-details">${currentLocoCount}x ${currentLocoMa} mA</span>
            </div>
            <div class="power-summary-right">
              <span class="summary-total-amps" id="power-summary-total-amps">${((currentLocoMa * currentLocoCount) / 1000).toFixed(2)} A</span>
              <span class="summary-total-ma" id="power-summary-total-ma">(${currentLocoMa * currentLocoCount} mA)</span>
            </div>
          </div>
          <div id="power-dialog-booster-warning" class="power-dialog-warning" style="display: ${currentLocoMa * currentLocoCount > 3000 ? "flex" : "none"}">
            ${t.boosterWarning}
          </div>
        </div>
      </div>

      <div class="rb-modal-footer">
        <button type="button" class="rb-btn rb-btn-secondary" id="power-btn-cancel">${t.cancel}</button>
        <button type="button" class="rb-btn rb-btn-primary" id="power-btn-apply">${t.apply}</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#power-modal-close") as HTMLButtonElement;
  const cancelBtn = modal.querySelector("#power-btn-cancel") as HTMLButtonElement;
  const applyBtn = modal.querySelector("#power-btn-apply") as HTMLButtonElement;

  const voltInput = modal.querySelector("#power-input-voltage") as HTMLInputElement;
  const catenaryVoltInput = modal.querySelector("#power-input-catenary-voltage") as HTMLInputElement;
  const sliderLoco = modal.querySelector("#power-slider-loco") as HTMLInputElement;
  const numLoco = modal.querySelector("#power-input-loco-ma") as HTMLInputElement;
  const countInput = modal.querySelector("#power-input-loco-count") as HTMLInputElement;
  const decBtn = modal.querySelector("#power-btn-dec-count") as HTMLButtonElement;
  const incBtn = modal.querySelector("#power-btn-inc-count") as HTMLButtonElement;

  // Turnout elements
  const turnoutStatsText = modal.querySelector("#power-turnout-stats-text") as HTMLElement;
  const btnEquipAll = modal.querySelector("#power-btn-equip-all-turnouts") as HTMLButtonElement;
  const btnClearAll = modal.querySelector("#power-btn-clear-all-turnouts") as HTMLButtonElement;
  const sliderLantern = modal.querySelector("#power-slider-lantern") as HTMLInputElement;
  const numLantern = modal.querySelector("#power-input-lantern-ma") as HTMLInputElement;
  const lblLanternMa = modal.querySelector("#power-lbl-lantern-ma") as HTMLElement;
  const sliderDecoder = modal.querySelector("#power-slider-decoder") as HTMLInputElement;
  const numDecoder = modal.querySelector("#power-input-decoder-ma") as HTMLInputElement;
  const lblDecoderMa = modal.querySelector("#power-lbl-decoder-ma") as HTMLElement;

  const lblLocoMa = modal.querySelector("#power-lbl-loco-ma") as HTMLElement;
  const summaryDetails = modal.querySelector("#power-summary-details") as HTMLElement;
  const summaryTotalAmps = modal.querySelector("#power-summary-total-amps") as HTMLElement;
  const summaryTotalMa = modal.querySelector("#power-summary-total-ma") as HTMLElement;
  const boosterWarning = modal.querySelector("#power-dialog-booster-warning") as HTMLElement;

  const voltPresetBtns = modal.querySelectorAll<HTMLButtonElement>("[data-voltage]");
  const maPresetBtns = modal.querySelectorAll<HTMLButtonElement>("[data-loco-ma]");
  const modeRadios = modal.querySelectorAll<HTMLInputElement>('input[name="catenary-mode"]');
  const labelCatSeparate = modal.querySelector("#label-cat-mode-separate") as HTMLElement;
  const labelCatParallel = modal.querySelector("#label-cat-mode-parallel") as HTMLElement;

  function updateDisplay() {
    lblLocoMa.innerHTML = `<strong>${currentLocoMa}</strong> mA (${(currentLocoMa / 1000).toFixed(2)} A)`;
    lblLanternMa.innerHTML = `<strong>${currentLanternMa}</strong> mA`;
    lblDecoderMa.innerHTML = `<strong>${currentDecoderMa}</strong> mA`;

    turnoutStatsText.textContent = t.turnoutStats
      .replace("{count}", String(totalTurnoutCount))
      .replace("{lanterns}", String(activeLanternCount))
      .replace("{decoders}", String(activeDecoderCount));

    const tractionMa = currentLocoMa * currentLocoCount;
    const turnoutMa = activeLanternCount * currentLanternMa + activeDecoderCount * currentDecoderMa;
    const totalCombinedMa = tractionMa + turnoutMa;
    const totalCombinedAmps = totalCombinedMa / 1000;

    const detailsStr = turnoutMa > 0
      ? `${currentLocoCount}x ${currentLocoMa} mA Traktion + ${turnoutMa} mA Weichen`
      : `${currentLocoCount}x ${currentLocoMa} mA`;

    summaryDetails.textContent = detailsStr;
    summaryTotalAmps.textContent = `${totalCombinedAmps.toFixed(2)} A`;
    summaryTotalMa.textContent = `(${totalCombinedMa} mA)`;

    boosterWarning.style.display = totalCombinedMa > 3000 ? "flex" : "none";

    decBtn.disabled = currentLocoCount <= 1;
    incBtn.disabled = currentLocoCount >= 10;

    // Presets visual active state
    voltPresetBtns.forEach((btn) => {
      const v = parseFloat(btn.dataset.voltage || "0");
      btn.classList.toggle("active", Math.abs(currentVoltage - v) < 0.1);
    });

    maPresetBtns.forEach((btn) => {
      const ma = parseInt(btn.dataset.locoMa || "0", 10);
      btn.classList.toggle("active", currentLocoMa === ma);
    });

    // Catenary mode UI state
    labelCatSeparate.classList.toggle("active", currentCatenaryMode === "separate");
    labelCatParallel.classList.toggle("active", currentCatenaryMode === "parallel");
    catenaryVoltInput.disabled = currentCatenaryMode === "parallel";
    if (currentCatenaryMode === "parallel") {
      catenaryVoltInput.value = currentVoltage.toFixed(1);
    }
  }

  // Turnout bulk actions
  btnEquipAll.addEventListener("click", () => {
    activeLanternCount = totalTurnoutCount;
    activeDecoderCount = totalTurnoutCount;
    bulkTurnoutAction = "equipAll";
    updateDisplay();
  });

  btnClearAll.addEventListener("click", () => {
    activeLanternCount = 0;
    activeDecoderCount = 0;
    bulkTurnoutAction = "clearAll";
    updateDisplay();
  });

  // Lantern slider & input
  sliderLantern.addEventListener("input", () => {
    currentLanternMa = parseInt(sliderLantern.value, 10);
    numLantern.value = currentLanternMa.toString();
    updateDisplay();
  });

  numLantern.addEventListener("change", () => {
    let val = parseInt(numLantern.value, 10);
    if (isNaN(val) || val < 5) val = 5;
    if (val > 150) val = 150;
    currentLanternMa = val;
    sliderLantern.value = val.toString();
    numLantern.value = val.toString();
    updateDisplay();
  });

  // Decoder slider & input
  sliderDecoder.addEventListener("input", () => {
    currentDecoderMa = parseInt(sliderDecoder.value, 10);
    numDecoder.value = currentDecoderMa.toString();
    updateDisplay();
  });

  numDecoder.addEventListener("change", () => {
    let val = parseInt(numDecoder.value, 10);
    if (isNaN(val) || val < 5) val = 5;
    if (val > 100) val = 100;
    currentDecoderMa = val;
    sliderDecoder.value = val.toString();
    numDecoder.value = val.toString();
    updateDisplay();
  });

  // Voltage presets
  voltPresetBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      currentVoltage = parseFloat(btn.dataset.voltage || "16");
      voltInput.value = currentVoltage.toFixed(1);
      if (currentCatenaryMode === "parallel") {
        currentCatenaryVoltage = currentVoltage;
      }
      updateDisplay();
    });
  });

  voltInput.addEventListener("input", () => {
    const val = parseFloat(voltInput.value);
    if (!isNaN(val) && val > 0) {
      currentVoltage = val;
      if (currentCatenaryMode === "parallel") {
        currentCatenaryVoltage = currentVoltage;
      }
      updateDisplay();
    }
  });

  // Catenary mode radios
  modeRadios.forEach((radio) => {
    radio.addEventListener("change", () => {
      if (radio.checked) {
        currentCatenaryMode = radio.value as "separate" | "parallel";
        if (currentCatenaryMode === "parallel") {
          currentCatenaryVoltage = currentVoltage;
        } else {
          currentCatenaryVoltage = parseFloat(catenaryVoltInput.value) || currentVoltage;
        }
        updateDisplay();
      }
    });
  });

  // Catenary voltage input
  catenaryVoltInput.addEventListener("input", () => {
    const val = parseFloat(catenaryVoltInput.value);
    if (!isNaN(val) && val > 0) {
      currentCatenaryVoltage = val;
    }
  });

  // Current presets
  maPresetBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      currentLocoMa = parseInt(btn.dataset.locoMa || "450", 10);
      sliderLoco.value = currentLocoMa.toString();
      numLoco.value = currentLocoMa.toString();
      updateDisplay();
    });
  });

  sliderLoco.addEventListener("input", () => {
    currentLocoMa = parseInt(sliderLoco.value, 10);
    numLoco.value = currentLocoMa.toString();
    updateDisplay();
  });

  numLoco.addEventListener("change", () => {
    let val = parseInt(numLoco.value, 10);
    if (isNaN(val)) val = 450;
    val = Math.min(MAX_LOCO_CURRENT_MA, Math.max(MIN_LOCO_CURRENT_MA, val));
    currentLocoMa = val;
    sliderLoco.value = val.toString();
    numLoco.value = val.toString();
    updateDisplay();
  });

  // Count stepper
  decBtn.addEventListener("click", () => {
    currentLocoCount = Math.max(1, currentLocoCount - 1);
    countInput.value = currentLocoCount.toString();
    updateDisplay();
  });

  incBtn.addEventListener("click", () => {
    currentLocoCount = Math.min(10, currentLocoCount + 1);
    countInput.value = currentLocoCount.toString();
    updateDisplay();
  });

  countInput.addEventListener("change", () => {
    let val = parseInt(countInput.value, 10);
    if (isNaN(val) || val < 1) val = 1;
    if (val > 10) val = 10;
    currentLocoCount = val;
    countInput.value = val.toString();
    updateDisplay();
  });

  const closeDialog = () => {
    modal.classList.add("closing");
    setTimeout(() => modal.remove(), 120);
  };

  closeBtn.addEventListener("click", closeDialog);
  cancelBtn.addEventListener("click", closeDialog);
  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeDialog();
  });

  applyBtn.addEventListener("click", () => {
    closeDialog();
    onApply({
      feedVoltage: Math.round(currentVoltage * 10) / 10,
      catenaryFeedVoltage: Math.round(currentCatenaryVoltage * 10) / 10,
      catenaryCircuitMode: currentCatenaryMode,
      locoCurrentMa: currentLocoMa,
      locoCount: currentLocoCount,
      turnoutLanternCurrentMa: currentLanternMa,
      turnoutDecoderCurrentMa: currentDecoderMa,
      bulkTurnoutAction,
    });
  });

  updateDisplay();

  // Focus
  if (initialFocus === "current") {
    numLoco.focus();
    numLoco.select();
  } else if (initialFocus === "catenary") {
    catenaryVoltInput.focus();
    catenaryVoltInput.select();
  } else if (initialFocus === "turnout") {
    numLantern.focus();
    numLantern.select();
  } else {
    voltInput.focus();
    voltInput.select();
  }
}
