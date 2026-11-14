import powerDialogHtml from "./templates/power-dialog.html?raw";
import type { Layout } from "@/core/types";
import {
  getDefaultLocoCurrentMa,
  MIN_LOCO_CURRENT_MA,
  MAX_LOCO_CURRENT_MA,
  isTurnoutEntry,
  detectShortCircuit,
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
 * Öffnet ein kompaktes modales Dialogfenster zur Konfiguration von:
 * 1. Gleisspannung (Track Voltage)
 * 2. Lok-Fahrstrom (Locomotive Traction Current)
 * 3. Oberleitung (Catenary Circuit: Getrennt / Parallel)
 * 4. Fahrbetrieb & Gesamtlast (inkl. Weichen-Zubehör)
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

  let currentCatenaryMode: "separate" | "parallel" = layout.catenaryCircuitMode ?? "separate";
  let currentCatenaryVoltage = currentCatenaryMode === "parallel"
    ? currentVoltage
    : (layout.catenaryFeedVoltage ?? currentVoltage);

  // Weichen-Zubehör Parameter
  let currentLanternMa = layout.turnoutLanternCurrentMa ?? 20;
  let currentDecoderMa = layout.turnoutDecoderCurrentMa ?? 15;
  let bulkTurnoutAction: "equipAll" | "clearAll" | undefined = undefined;

  const pieces = layout.pieces || [];
  const catenaryTracks = pieces.filter((p) => !!p.hasCatenary);
  const catenaryFeeders = catenaryTracks.filter((p) => !!p.catenaryPowerFeed);
  const catenaryIsolators = catenaryTracks.filter((p) => !!p.catenaryIsolator);
  const hasCatenary = catenaryTracks.length > 0;

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

  const shortCircuitRes = catalog
    ? detectShortCircuit(layout, catalog)
    : { hasShortCircuit: false, shortCircuitPieces: new Set<string>(), shortCircuitPoints: [] };

  const modal = document.createElement("div");
  modal.id = "power-settings-modal";
  modal.className = "rb-modal-overlay";

  let scBannerHtml = "";
  if (shortCircuitRes.hasShortCircuit) {
    scBannerHtml = `
      <div class="power-dialog-short-circuit-banner" style="padding: 6px 10px; margin-bottom: 2px;">
        <div class="sc-banner-icon">⚠️</div>
        <div class="sc-banner-body">
          <strong>${t.shortCircuitHeader}</strong>
          <p style="font-size: 11px; margin: 2px 0 0 0;">${t.shortCircuitSub}</p>
        </div>
      </div>`;
  }

  let turnoutSectionHtml = "";
  if (totalTurnoutCount > 0) {
    turnoutSectionHtml = `
      <details id="power-turnout-details" style="margin-top: 4px; font-size: 11px; border-top: 1px solid var(--border-main); padding-top: 4px;">
        <summary style="cursor: pointer; color: var(--text-muted); font-weight: 600; user-select: none;">
          ⚙️ ${t.turnoutHeader} (${totalTurnoutCount})
        </summary>
        <div style="margin-top: 6px; display: flex; flex-direction: column; gap: 6px;">
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <span id="power-turnout-stats-text">${t.turnoutStats.replace("{count}", String(totalTurnoutCount)).replace("{lanterns}", String(activeLanternCount)).replace("{decoders}", String(activeDecoderCount))}</span>
            <div style="display: flex; gap: 4px;">
              <button type="button" class="rb-btn rb-btn-secondary" id="power-btn-equip-all-turnouts" style="font-size: 10px; padding: 2px 6px;">${t.turnoutEquipAllBtn}</button>
              <button type="button" class="rb-btn rb-btn-secondary" id="power-btn-clear-all-turnouts" style="font-size: 10px; padding: 2px 6px;">✕ ${t.turnoutClearAllBtn}</button>
            </div>
          </div>
          <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
            <span style="font-size: 11px;">${t.turnoutLanternLabel} <span id="power-lbl-lantern-ma" class="power-highlight-val"><strong>${currentLanternMa}</strong> mA</span></span>
            <input type="range" id="power-slider-lantern" min="5" max="80" step="5" value="${currentLanternMa}" class="power-slider" style="flex: 1; max-width: 140px;" />
            <input type="number" id="power-input-lantern-ma" min="5" max="150" step="5" value="${currentLanternMa}" class="rb-input power-num-field" style="width: 52px; padding: 2px 4px;" />
          </div>
          <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
            <span style="font-size: 11px;">${t.turnoutDecoderLabel} <span id="power-lbl-decoder-ma" class="power-highlight-val"><strong>${currentDecoderMa}</strong> mA</span></span>
            <input type="range" id="power-slider-decoder" min="5" max="60" step="5" value="${currentDecoderMa}" class="power-slider" style="flex: 1; max-width: 140px;" />
            <input type="number" id="power-input-decoder-ma" min="5" max="100" step="5" value="${currentDecoderMa}" class="rb-input power-num-field" style="width: 52px; padding: 2px 4px;" />
          </div>
        </div>
      </details>`;
  } else {
    turnoutSectionHtml = `
      <div style="display:none;">
        <span id="power-turnout-stats-text"></span>
        <button id="power-btn-equip-all-turnouts"></button>
        <button id="power-btn-clear-all-turnouts"></button>
        <input type="range" id="power-slider-lantern" value="${currentLanternMa}" />
        <input type="number" id="power-input-lantern-ma" value="${currentLanternMa}" />
        <span id="power-lbl-lantern-ma"></span>
        <input type="range" id="power-slider-decoder" value="${currentDecoderMa}" />
        <input type="number" id="power-input-decoder-ma" value="${currentDecoderMa}" />
        <span id="power-lbl-decoder-ma"></span>
      </div>`;
  }

  modal.innerHTML = powerDialogHtml
    .replace("{{TITLE}}", t.title)
    .replace("{{SHORT_CIRCUIT_BANNER}}", scBannerHtml)
    .replace("{{FEEDER_HEADER}}", t.feederHeader)
    .replace(/\{\{CURRENT_VOLTAGE\}\}/g, currentVoltage.toFixed(1))
    .replace(/\{\{VOLTAGE_UNIT\}\}/g, t.voltageUnit)
    .replace("{{TICK_12V_ACTIVE}}", Math.abs(currentVoltage - 12) < 0.25 ? "active" : "")
    .replace("{{TICK_14V_ACTIVE}}", Math.abs(currentVoltage - 14) < 0.25 ? "active" : "")
    .replace("{{TICK_16V_ACTIVE}}", Math.abs(currentVoltage - 16) < 0.25 ? "active" : "")
    .replace("{{TICK_18V_ACTIVE}}", Math.abs(currentVoltage - 18) < 0.25 ? "active" : "")
    .replace(/\{\{PRESET_12V_DESC\}\}/g, t.preset12VDesc)
    .replace(/\{\{PRESET_14V_DESC\}\}/g, t.preset14VDesc)
    .replace(/\{\{PRESET_16V_DESC\}\}/g, t.preset16VDesc)
    .replace(/\{\{PRESET_18V_DESC\}\}/g, t.preset18VDesc)
    .replace("{{CATENARY_DISABLED_CLASS}}", !hasCatenary ? "power-catenary-disabled" : "")
    .replace("{{CATENARY_HEADER}}", t.catenaryHeader)
    .replace("{{CATENARY_TRACK_COUNT}}", hasCatenary ? `(${catenaryTracks.length})` : `(${t.catenaryNoTracks})`)
    .replace("{{CAT_MODE_SEPARATE_ACTIVE}}", currentCatenaryMode === "separate" ? "active" : "")
    .replace("{{CAT_MODE_PARALLEL_ACTIVE}}", currentCatenaryMode === "parallel" ? "active" : "")
    .replace(/\{\{CATENARY_DISABLED_ATTR\}\}/g, !hasCatenary ? "disabled" : "")
    .replace("{{CATENARY_MODE_SEPARATE}}", t.catenaryModeSeparate)
    .replace("{{CATENARY_MODE_PARALLEL}}", t.catenaryModeParallel)
    .replace("{{CATENARY_VOLTAGE_CONTAINER_STYLE}}", !hasCatenary || currentCatenaryMode === "parallel" ? "display: none;" : "")
    .replace("{{CATENARY_VOLTAGE_LABEL}}", t.catenaryVoltageLabel)
    .replace(/\{\{CURRENT_CATENARY_VOLTAGE\}\}/g, currentCatenaryVoltage.toFixed(1))
    .replace("{{CAT_TICK_12V_ACTIVE}}", Math.abs(currentCatenaryVoltage - 12) < 0.25 ? "active" : "")
    .replace("{{CAT_TICK_14V_ACTIVE}}", Math.abs(currentCatenaryVoltage - 14) < 0.25 ? "active" : "")
    .replace("{{CAT_TICK_16V_ACTIVE}}", Math.abs(currentCatenaryVoltage - 16) < 0.25 ? "active" : "")
    .replace("{{CAT_TICK_18V_ACTIVE}}", Math.abs(currentCatenaryVoltage - 18) < 0.25 ? "active" : "")
    .replace("{{LOCO_HEADER}}", t.locoHeader)
    .replace("{{CURRENT_LOCO_AMPS}}", (currentLocoMa / 1000).toFixed(2))
    .replace(/\{\{CURRENT_LOCO_MA\}\}/g, String(currentLocoMa))
    .replace(/\{\{MIN_LOCO_CURRENT_MA\}\}/g, String(MIN_LOCO_CURRENT_MA))
    .replace(/\{\{MAX_LOCO_CURRENT_MA\}\}/g, String(MAX_LOCO_CURRENT_MA))
    .replace("{{LOCO_TICK_150_ACTIVE}}", Math.abs(currentLocoMa - 150) < 25 ? "active" : "")
    .replace("{{LOCO_TICK_300_ACTIVE}}", Math.abs(currentLocoMa - 300) < 25 ? "active" : "")
    .replace("{{LOCO_TICK_450_ACTIVE}}", Math.abs(currentLocoMa - 450) < 25 ? "active" : "")
    .replace("{{LOCO_TICK_750_ACTIVE}}", Math.abs(currentLocoMa - 750) < 25 ? "active" : "")
    .replace("{{LOCO_TICK_1000_ACTIVE}}", Math.abs(currentLocoMa - 1000) < 25 ? "active" : "")
    .replace("{{PRESET_150_DESC}}", t.preset150Desc)
    .replace("{{PRESET_300_DESC}}", t.preset300Desc)
    .replace("{{PRESET_450_DESC}}", t.preset450Desc)
    .replace("{{PRESET_750_DESC}}", t.preset750Desc)
    .replace("{{PRESET_1000_DESC}}", t.preset1000Desc)
    .replace("{{TRAFFIC_HEADER}}", t.trafficHeader)
    .replace("{{ACTIVE_LOCOS_LABEL}}", t.activeLocosLabel)
    .replace("{{DEC_COUNT_DISABLED}}", currentLocoCount <= 1 ? "disabled" : "")
    .replace("{{INC_COUNT_DISABLED}}", currentLocoCount >= 10 ? "disabled" : "")
    .replace(/\{\{CURRENT_LOCO_COUNT\}\}/g, String(currentLocoCount))
    .replace("{{TOTAL_COMBINED_CURRENT_LABEL}}", t.totalCombinedCurrentLabel)
    .replace("{{TOTAL_AMPS}}", ((currentLocoMa * currentLocoCount) / 1000).toFixed(2))
    .replace("{{TOTAL_MA}}", String(currentLocoMa * currentLocoCount))
    .replace("{{TURNOUT_DETAILS_SECTION}}", turnoutSectionHtml)
    .replace("{{BOOSTER_WARNING_DISPLAY}}", currentLocoMa * currentLocoCount > 3000 ? "flex" : "none")
    .replace("{{BOOSTER_WARNING}}", t.boosterWarning)
    .replace("{{CANCEL_BTN}}", t.cancel)
    .replace("{{APPLY_BTN}}", t.apply);

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#power-modal-close") as HTMLButtonElement;
  const cancelBtn = modal.querySelector("#power-btn-cancel") as HTMLButtonElement;
  const applyBtn = modal.querySelector("#power-btn-apply") as HTMLButtonElement;

  // Track Voltage elements
  const sliderVolt = modal.querySelector("#power-slider-voltage") as HTMLInputElement;
  const inputVolt = modal.querySelector("#power-input-voltage") as HTMLInputElement;
  const lblVolt = modal.querySelector("#power-lbl-voltage") as HTMLElement;
  const voltTicks = modal.querySelectorAll<HTMLElement>("#power-ticks-voltage .power-tick-item");

  // Loco Current elements
  const sliderLoco = modal.querySelector("#power-slider-loco") as HTMLInputElement;
  const inputLoco = modal.querySelector("#power-input-loco-ma") as HTMLInputElement;
  const lblLocoMa = modal.querySelector("#power-lbl-loco-ma") as HTMLElement;
  const locoTicks = modal.querySelectorAll<HTMLElement>("#power-ticks-loco .power-tick-item");

  // Catenary elements
  const catToggleBtns = modal.querySelectorAll<HTMLButtonElement>("#power-catenary-toggle-group .power-toggle-btn");
  const catVoltContainer = modal.querySelector("#power-catenary-voltage-container") as HTMLElement;
  const sliderCatVolt = modal.querySelector("#power-slider-catenary-voltage") as HTMLInputElement | null;
  const inputCatVolt = modal.querySelector("#power-input-catenary-voltage") as HTMLInputElement | null;
  const lblCatVolt = modal.querySelector("#power-lbl-catenary-voltage") as HTMLElement | null;
  const catVoltTicks = modal.querySelectorAll<HTMLElement>("#power-ticks-catenary-voltage .power-tick-item");

  // Loco Count elements
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

  // Summary elements
  const summaryDetails = modal.querySelector("#power-summary-details") as HTMLElement;
  const summaryTotalAmps = modal.querySelector("#power-summary-total-amps") as HTMLElement;
  const summaryTotalMa = modal.querySelector("#power-summary-total-ma") as HTMLElement;
  const boosterWarning = modal.querySelector("#power-dialog-booster-warning") as HTMLElement;

  function updateDisplay() {
    // Voltage displays
    lblVolt.innerHTML = `<strong>${currentVoltage.toFixed(1)}</strong> V`;
    voltTicks.forEach((tick) => {
      const v = parseFloat(tick.dataset.voltage || "0");
      tick.classList.toggle("active", Math.abs(currentVoltage - v) < 0.25);
    });

    // Loco Current displays
    lblLocoMa.innerHTML = `<strong>${(currentLocoMa / 1000).toFixed(2)} A</strong> (${currentLocoMa} mA)`;
    locoTicks.forEach((tick) => {
      const ma = parseInt(tick.dataset.locoMa || "0", 10);
      tick.classList.toggle("active", Math.abs(currentLocoMa - ma) < 25);
    });

    // Catenary mode & voltage displays
    catToggleBtns.forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.catMode === currentCatenaryMode);
    });
    if (catVoltContainer) {
      catVoltContainer.style.display = (!hasCatenary || currentCatenaryMode === "parallel") ? "none" : "";
    }
    if (currentCatenaryMode === "parallel") {
      currentCatenaryVoltage = currentVoltage;
    }
    if (lblCatVolt) {
      lblCatVolt.innerHTML = `<strong>${currentCatenaryVoltage.toFixed(1)}</strong> V`;
    }
    if (sliderCatVolt) {
      sliderCatVolt.value = currentCatenaryVoltage.toFixed(1);
    }
    if (inputCatVolt) {
      inputCatVolt.value = currentCatenaryVoltage.toFixed(1);
    }
    catVoltTicks.forEach((tick) => {
      const v = parseFloat(tick.dataset.catenaryVoltage || "0");
      tick.classList.toggle("active", Math.abs(currentCatenaryVoltage - v) < 0.25);
    });

    // Turnout displays
    if (lblLanternMa) lblLanternMa.innerHTML = `<strong>${currentLanternMa}</strong> mA`;
    if (lblDecoderMa) lblDecoderMa.innerHTML = `<strong>${currentDecoderMa}</strong> mA`;
    if (turnoutStatsText) {
      turnoutStatsText.textContent = t.turnoutStats
        .replace("{count}", String(totalTurnoutCount))
        .replace("{lanterns}", String(activeLanternCount))
        .replace("{decoders}", String(activeDecoderCount));
    }

    // Summary calculation
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
  }

  // --- Track Voltage Event Listeners ---
  sliderVolt.addEventListener("input", () => {
    currentVoltage = parseFloat(sliderVolt.value);
    inputVolt.value = currentVoltage.toFixed(1);
    updateDisplay();
  });

  inputVolt.addEventListener("input", () => {
    const val = parseFloat(inputVolt.value);
    if (!isNaN(val) && val > 0) {
      currentVoltage = val;
      sliderVolt.value = Math.min(24, Math.max(8, val)).toString();
      updateDisplay();
    }
  });

  voltTicks.forEach((tick) => {
    tick.addEventListener("click", () => {
      const v = parseFloat(tick.dataset.voltage || "16");
      currentVoltage = v;
      sliderVolt.value = v.toFixed(1);
      inputVolt.value = v.toFixed(1);
      updateDisplay();
    });
  });

  // --- Loco Current Event Listeners ---
  sliderLoco.addEventListener("input", () => {
    currentLocoMa = parseInt(sliderLoco.value, 10);
    inputLoco.value = currentLocoMa.toString();
    updateDisplay();
  });

  inputLoco.addEventListener("change", () => {
    let val = parseInt(inputLoco.value, 10);
    if (isNaN(val)) val = 450;
    val = Math.min(MAX_LOCO_CURRENT_MA, Math.max(MIN_LOCO_CURRENT_MA, val));
    currentLocoMa = val;
    sliderLoco.value = val.toString();
    inputLoco.value = val.toString();
    updateDisplay();
  });

  locoTicks.forEach((tick) => {
    tick.addEventListener("click", () => {
      const ma = parseInt(tick.dataset.locoMa || "450", 10);
      currentLocoMa = ma;
      sliderLoco.value = ma.toString();
      inputLoco.value = ma.toString();
      updateDisplay();
    });
  });

  // --- Catenary Mode Toggle ---
  catToggleBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      if (!hasCatenary) return;
      const mode = (btn.dataset.catMode as "separate" | "parallel") || "separate";
      currentCatenaryMode = mode;
      updateDisplay();
    });
  });

  // --- Catenary Voltage Event Listeners ---
  if (sliderCatVolt && inputCatVolt) {
    sliderCatVolt.addEventListener("input", () => {
      currentCatenaryVoltage = parseFloat(sliderCatVolt.value);
      inputCatVolt.value = currentCatenaryVoltage.toFixed(1);
      updateDisplay();
    });

    inputCatVolt.addEventListener("input", () => {
      const val = parseFloat(inputCatVolt.value);
      if (!isNaN(val) && val > 0) {
        currentCatenaryVoltage = val;
        sliderCatVolt.value = Math.min(24, Math.max(8, val)).toString();
        updateDisplay();
      }
    });

    catVoltTicks.forEach((tick) => {
      tick.addEventListener("click", () => {
        const v = parseFloat(tick.dataset.catenaryVoltage || "16");
        currentCatenaryVoltage = v;
        sliderCatVolt.value = v.toFixed(1);
        inputCatVolt.value = v.toFixed(1);
        updateDisplay();
      });
    });
  }

  // --- Turnout bulk actions & sliders ---
  if (btnEquipAll) {
    btnEquipAll.addEventListener("click", () => {
      activeLanternCount = totalTurnoutCount;
      activeDecoderCount = totalTurnoutCount;
      bulkTurnoutAction = "equipAll";
      updateDisplay();
    });
  }

  if (btnClearAll) {
    btnClearAll.addEventListener("click", () => {
      activeLanternCount = 0;
      activeDecoderCount = 0;
      bulkTurnoutAction = "clearAll";
      updateDisplay();
    });
  }

  if (sliderLantern && numLantern) {
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
  }

  if (sliderDecoder && numDecoder) {
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
  }

  // --- Count stepper ---
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

  // --- Close & Apply actions ---
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
    inputLoco.focus();
    inputLoco.select();
  } else if (initialFocus === "catenary" && hasCatenary && inputCatVolt && currentCatenaryMode === "separate") {
    inputCatVolt.focus();
    inputCatVolt.select();
  } else if (initialFocus === "turnout" && numLantern) {
    numLantern.focus();
    numLantern.select();
  } else {
    inputVolt.focus();
    inputVolt.select();
  }
}
