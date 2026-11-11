import type { Layout, Layer, TrackPiece } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import { getLanguage } from "@/i18n";
import {
  generateRampPieces,
  getMinClearanceHeightForScale,
  getOpenConnectors,
  getAvailableCurveRadii,
  type RampShapeType,
} from "@/core/ramp-generator";

export interface RampGeneratorDialogOptions {
  layout: Layout;
  catalog: Catalog;
  anchorPiece: TrackPiece;
  onApply: (generatedPieces: TrackPiece[], newLayer?: Layer) => void;
}

export function openRampGeneratorDialog(options: RampGeneratorDialogOptions): void {
  const { layout, catalog, anchorPiece, onApply } = options;
  const isDe = getLanguage() === "de";

  const existing = document.getElementById("ramp-generator-modal");
  if (existing) existing.remove();

  const layers = layout.layers || [
    { id: "layer-1", name: "Hauptebene", visible: true, locked: false, heightMm: 0 },
  ];

  const currentLayerId = anchorPiece.layerId || layout.activeLayerId || layers[0]!.id;
  const currentLayer = layers.find((l) => l.id === currentLayerId) || layers[0]!;
  const currentHeight = currentLayer.heightMm ?? 0;

  const scale = layout.scale || catalog.scale || "H0";
  const minClearance = getMinClearanceHeightForScale(scale);

  // Check open connectors
  const openConns = getOpenConnectors(anchorPiece, layout.pieces);
  if (openConns.length === 0) {
    alert(
      isDe
        ? "Dieses Gleis hat keine freien Anschlüsse. Bitte ein Gleis mit offenem Ende wählen."
        : "This track has no open connectors. Please select a track with an open end.",
    );
    return;
  }

  // Pre-select higher layer if available, otherwise suggest new layer above
  const higherLayer = layers.find((l) => (l.heightMm ?? 0) > currentHeight);
  let preselectedLayerId = higherLayer ? higherLayer.id : "__create_above__";
  let targetHeightMm = higherLayer ? (higherLayer.heightMm ?? 0) : currentHeight + minClearance;

  const availableRadii = getAvailableCurveRadii(catalog);
  let currentCurveId = availableRadii.length > 0 ? availableRadii[0]!.catalogId : undefined;

  let currentShape: RampShapeType = "straight";
  let currentDirection: "left" | "right" = "left";
  let currentCurveAngle = 180;
  let currentGradient = 2.8;

  const modal = document.createElement("div");
  modal.id = "ramp-generator-modal";
  modal.className = "rb-modal-overlay";

  modal.innerHTML = `
    <style>
      #ramp-generator-modal {
        position: fixed;
        inset: 0;
        background: rgba(10, 15, 26, 0.72);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 10000;
        font-family: inherit;
        font-size: 13px;
      }
      .ramp-gen-card {
        width: 460px;
        max-height: 90vh;
        display: flex;
        flex-direction: column;
        background: #1e2638;
        color: #e2e8f0;
        border-radius: 12px;
        overflow: hidden;
        box-shadow: 0 20px 50px rgba(0, 0, 0, 0.55);
      }
      .ramp-gen-header {
        padding: 12px 18px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        border-bottom: 1px solid #2d3748;
        font-weight: 700;
        font-size: 14px;
      }
      .ramp-gen-body {
        padding: 16px 18px;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 14px;
        scrollbar-width: thin;
      }
      .ramp-gen-body::-webkit-scrollbar { width: 6px; }
      .ramp-gen-body::-webkit-scrollbar-thumb {
        background: rgba(148, 163, 184, 0.35);
        border-radius: 3px;
      }
      .ramp-gen-strip {
        display: flex;
        justify-content: space-between;
        background: #141b2a;
        padding: 8px 12px;
        border-radius: 6px;
        font-size: 12px;
        color: #94a3b8;
      }
      .ramp-gen-strip strong { color: #f8fafc; }
      .ramp-gen-field {
        display: flex;
        flex-direction: column;
        gap: 5px;
      }
      .ramp-gen-field label {
        font-size: 11.5px;
        color: #94a3b8;
        font-weight: 600;
      }
      .ramp-gen-select, .ramp-gen-input {
        background: #141b2a;
        border: 1px solid #334155;
        color: #f8fafc;
        border-radius: 6px;
        padding: 7px 10px;
        font-size: 13px;
        width: 100%;
        box-sizing: border-box;
      }
      .ramp-gen-segmented {
        display: grid;
        grid-template-columns: 1fr 1fr 1fr;
        background: #141b2a;
        border-radius: 8px;
        padding: 3px;
        gap: 3px;
      }
      .ramp-gen-seg-item {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 7px 4px;
        cursor: pointer;
        border-radius: 6px;
        font-size: 12px;
        color: #94a3b8;
        user-select: none;
        transition: all 0.15s ease;
      }
      .ramp-gen-seg-item.active {
        background: #0284c7;
        color: #ffffff;
        font-weight: 600;
      }
      .ramp-btn-pill {
        background: #242f44;
        border: 1px solid transparent;
        color: #94a3b8;
        padding: 4px 9px;
        border-radius: 5px;
        font-size: 11.5px;
        cursor: pointer;
        transition: all 0.12s;
      }
      .ramp-btn-pill:hover { background: #334155; color: #fff; }
      .ramp-btn-pill.active {
        background: #0284c7;
        color: #fff;
        font-weight: 600;
      }
      .ramp-status-card {
        padding: 10px 14px;
        border-radius: 8px;
        display: flex;
        flex-direction: column;
        gap: 4px;
        font-size: 12px;
      }
      .ramp-status-card.ok { background: rgba(34, 197, 94, 0.12); border: 1px solid rgba(34, 197, 94, 0.3); color: #4ade80; }
      .ramp-status-card.warn { background: rgba(245, 158, 11, 0.12); border: 1px solid rgba(245, 158, 11, 0.3); color: #fbbf24; }
      .ramp-status-card.danger { background: rgba(239, 68, 68, 0.12); border: 1px solid rgba(239, 68, 68, 0.3); color: #f87171; }
      .ramp-gen-footer {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
        padding: 12px 18px;
        border-top: 1px solid #2d3748;
        background: #171e2e;
      }
    </style>

    <div class="ramp-gen-card">
      <div class="ramp-gen-header">
        <span>📐 ${isDe ? "Gleisrampe automatisch erstellen" : "Auto-Create Track Ramp"}</span>
        <button id="ramp-gen-close" style="background:transparent; border:none; color:#94a3b8; cursor:pointer; font-size:16px;">✕</button>
      </div>

      <div class="ramp-gen-body">
        <!-- Start Piece Summary -->
        <div class="ramp-gen-strip">
          <span>${isDe ? "Startgleis" : "Start Track"}: <strong>#${anchorPiece.catalogId}</strong></span>
          <span>${isDe ? "Ebene" : "Layer"}: <strong>${currentLayer.name.replace(/\s*\([0-9]+\s*mm\)$/, "")} (${currentHeight} mm)</strong></span>
          <span>${isDe ? "Freie Anschlüsse" : "Open Ends"}: <strong>${openConns.length}</strong></span>
        </div>

        <!-- Target Layer & Height -->
        <div class="ramp-gen-field">
          <label>${isDe ? "Zielebene / Zielhöhe" : "Target Layer / Target Elevation"}</label>
          <select id="ramp-target-layer" class="ramp-gen-select">
            ${layers
              .map((l) => {
                const cleanName = l.name.replace(/\s*\([0-9]+\s*mm\)$/, "");
                const isSel = l.id === preselectedLayerId ? "selected" : "";
                return `<option value="${l.id}" ${isSel}>${cleanName} (${l.heightMm ?? 0} mm)</option>`;
              })
              .join("")}
            <option value="__create_above__" ${preselectedLayerId === "__create_above__" ? "selected" : ""}>
              + ${isDe ? "Neue Ebene oberhalb erstellen" : "Create new layer above"} (+${minClearance} mm)
            </option>
            <option value="__create_below__">
              + ${isDe ? "Neue Ebene unterhalb erstellen" : "Create new layer below"} (-${minClearance} mm)
            </option>
          </select>
        </div>

        <div class="ramp-gen-field" id="custom-height-container">
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <label>${isDe ? "Zielhöhe (mm)" : "Target Height (mm)"}</label>
            <span style="font-size: 11px; color: #94a3b8;" id="clearance-hint">
              ${isDe ? `Mindesthöhe Spur ${scale}: ${minClearance} mm` : `Min clearance scale ${scale}: ${minClearance} mm`}
            </span>
          </div>
          <input type="number" id="ramp-target-height-input" value="${targetHeightMm}" step="1" class="ramp-gen-input" />
        </div>

        <!-- Shape Selector (Straight / Curved / Helix) -->
        <div class="ramp-gen-field">
          <label>${isDe ? "Rampenverlauf / Form" : "Ramp Shape / Geometry"}</label>
          <div class="ramp-gen-segmented">
            <div class="ramp-gen-seg-item active" data-shape="straight">${isDe ? "Gerade" : "Straight"}</div>
            <div class="ramp-gen-seg-item" data-shape="curved">${isDe ? "Bogen / Kehre" : "Curve / Turn"}</div>
            <div class="ramp-gen-seg-item" data-shape="helix">${isDe ? "Wendel (Helix)" : "Helix (Spiral)"}</div>
          </div>
        </div>

        <!-- Curve / Helix Options -->
        <div id="curve-options-row" style="display: none; flex-direction: column; gap: 10px;">
          ${
            availableRadii.length > 0
              ? `
          <div class="ramp-gen-field">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <label for="ramp-curve-radius">${isDe ? "Gleisradius / Bogen-Typ" : "Curve Radius / Track Type"}</label>
              <span style="font-size: 11px; color: #94a3b8;" id="radius-circum-hint"></span>
            </div>
            <select id="ramp-curve-radius" class="ramp-gen-select">
              ${availableRadii
                .map(
                  (r) =>
                    `<option value="${r.catalogId}" ${r.catalogId === currentCurveId ? "selected" : ""}>${r.label}</option>`,
                )
                .join("")}
            </select>
          </div>
          `
              : ""
          }

          <div style="display: flex; justify-content: space-between; align-items: center;">
            <span style="font-size: 11.5px; color: #94a3b8; font-weight: 600;">${isDe ? "Bogenrichtung" : "Curve Direction"}</span>
            <div style="display: flex; gap: 6px;">
              <button type="button" class="ramp-btn-pill active" id="btn-dir-left">↰ ${isDe ? "Links" : "Left"}</button>
              <button type="button" class="ramp-btn-pill" id="btn-dir-right">↱ ${isDe ? "Rechts" : "Right"}</button>
            </div>
          </div>

          <div id="curve-angle-container" style="display: flex; justify-content: space-between; align-items: center;">
            <span style="font-size: 11.5px; color: #94a3b8; font-weight: 600;">${isDe ? "Drehwinkel" : "Turn Angle"}</span>
            <div style="display: flex; gap: 4px;">
              <button type="button" class="ramp-btn-pill" data-angle="90">90°</button>
              <button type="button" class="ramp-btn-pill active" data-angle="180">180° (U)</button>
              <button type="button" class="ramp-btn-pill" data-angle="270">270°</button>
              <button type="button" class="ramp-btn-pill" data-angle="360">360°</button>
            </div>
          </div>
        </div>

        <!-- Target Max Gradient Selector -->
        <div class="ramp-gen-field">
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <label>${isDe ? "Ziel-Steigung" : "Target Gradient"}</label>
            <div style="display: flex; gap: 4px; align-items: center;">
              <button type="button" class="ramp-btn-pill" data-grad="2.0">2.0%</button>
              <button type="button" class="ramp-btn-pill active" data-grad="2.8">2.8%</button>
              <button type="button" class="ramp-btn-pill" data-grad="3.5">3.5%</button>
              <button type="button" class="ramp-btn-pill" data-grad="4.0">4.0%</button>
              <input type="number" id="ramp-custom-grad-input" value="2.8" min="0.5" max="10.0" step="0.1" style="width: 48px; background: #141b2a; border: 1px solid #334155; color: #f8fafc; border-radius: 6px; padding: 4px 6px; font-size: 12px; text-align: center;" />
              <span style="font-size: 11px; color: #94a3b8;">%</span>
            </div>
          </div>
        </div>

        <!-- Live Preview Status -->
        <div class="ramp-status-card ok" id="ramp-live-status">
          <strong id="status-title">...</strong>
          <span id="status-desc" style="font-size: 11.5px;">...</span>
        </div>
      </div>

      <div class="ramp-gen-footer">
        <button class="ramp-btn-pill" id="ramp-btn-cancel" style="padding: 7px 14px;">${isDe ? "Abbrechen" : "Cancel"}</button>
        <button id="ramp-btn-create" style="background:#0284c7; color:#fff; border:none; padding:7px 18px; border-radius:6px; cursor:pointer; font-weight:700;">
          📐 ${isDe ? "Rampe erstellen" : "Create Ramp"}
        </button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#ramp-gen-close") as HTMLButtonElement;
  const cancelBtn = modal.querySelector("#ramp-btn-cancel") as HTMLButtonElement;
  const createBtn = modal.querySelector("#ramp-btn-create") as HTMLButtonElement;

  const targetLayerSelect = modal.querySelector("#ramp-target-layer") as HTMLSelectElement;
  const targetHeightInput = modal.querySelector("#ramp-target-height-input") as HTMLInputElement;

  const curveOptionsRow = modal.querySelector("#curve-options-row") as HTMLElement;
  const curveAngleContainer = modal.querySelector("#curve-angle-container") as HTMLElement;
  const curveRadiusSelect = modal.querySelector("#ramp-curve-radius") as HTMLSelectElement | null;
  const radiusCircumHint = modal.querySelector("#radius-circum-hint") as HTMLElement | null;
  const btnDirLeft = modal.querySelector("#btn-dir-left") as HTMLButtonElement;
  const btnDirRight = modal.querySelector("#btn-dir-right") as HTMLButtonElement;

  const statusCard = modal.querySelector("#ramp-live-status") as HTMLElement;
  const statusTitle = modal.querySelector("#status-title") as HTMLElement;
  const statusDesc = modal.querySelector("#status-desc") as HTMLElement;

  const closeModal = () => modal.remove();
  closeBtn.addEventListener("click", closeModal);
  cancelBtn.addEventListener("click", closeModal);

  // Esc closes modal
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closeModal();
      window.removeEventListener("keydown", handleKeyDown);
    }
  };
  window.addEventListener("keydown", handleKeyDown);

  // Shape switcher
  modal.querySelectorAll<HTMLElement>(".ramp-gen-seg-item").forEach((item) => {
    item.addEventListener("click", () => {
      modal.querySelectorAll(".ramp-gen-seg-item").forEach((s) => s.classList.remove("active"));
      item.classList.add("active");
      currentShape = (item.dataset.shape as RampShapeType) || "straight";

      curveOptionsRow.style.display = currentShape === "straight" ? "none" : "flex";
      curveAngleContainer.style.display = currentShape === "helix" ? "none" : "flex";
      updateLiveCalculation();
    });
  });

  // Curve radius selector
  if (curveRadiusSelect) {
    curveRadiusSelect.addEventListener("change", () => {
      currentCurveId = curveRadiusSelect.value;
      updateLiveCalculation();
    });
  }

  // Curve direction
  btnDirLeft.addEventListener("click", () => {
    currentDirection = "left";
    btnDirLeft.classList.add("active");
    btnDirRight.classList.remove("active");
    updateLiveCalculation();
  });
  btnDirRight.addEventListener("click", () => {
    currentDirection = "right";
    btnDirRight.classList.add("active");
    btnDirLeft.classList.remove("active");
    updateLiveCalculation();
  });

  // Curve angle buttons
  modal.querySelectorAll<HTMLButtonElement>("#curve-angle-container .ramp-btn-pill").forEach((btn) => {
    btn.addEventListener("click", () => {
      modal.querySelectorAll("#curve-angle-container .ramp-btn-pill").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      currentCurveAngle = parseFloat(btn.dataset.angle || "180");
      updateLiveCalculation();
    });
  });

  // Gradient preset pills & input
  const gradInput = modal.querySelector("#ramp-custom-grad-input") as HTMLInputElement | null;

  modal.querySelectorAll<HTMLButtonElement>("button[data-grad]").forEach((btn) => {
    btn.addEventListener("click", () => {
      modal.querySelectorAll("button[data-grad]").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      currentGradient = parseFloat(btn.dataset.grad || "2.8");
      if (gradInput) gradInput.value = currentGradient.toFixed(1);
      updateLiveCalculation();
    });
  });

  if (gradInput) {
    gradInput.addEventListener("input", () => {
      const val = parseFloat(gradInput.value);
      if (!isNaN(val) && val >= 0.5 && val <= 15) {
        currentGradient = val;
        modal.querySelectorAll<HTMLButtonElement>("button[data-grad]").forEach((b) => {
          if (Math.abs(parseFloat(b.dataset.grad || "0") - val) < 0.05) {
            b.classList.add("active");
          } else {
            b.classList.remove("active");
          }
        });
        updateLiveCalculation();
      }
    });
  }

  // Target layer change
  targetLayerSelect.addEventListener("change", () => {
    const val = targetLayerSelect.value;
    if (val === "__create_above__") {
      targetHeightInput.value = String(currentHeight + minClearance);
    } else if (val === "__create_below__") {
      targetHeightInput.value = String(currentHeight - minClearance);
    } else {
      const lay = layers.find((l) => l.id === val);
      if (lay) {
        targetHeightInput.value = String(lay.heightMm ?? 0);
      }
    }
    updateLiveCalculation();
  });

  targetHeightInput.addEventListener("input", updateLiveCalculation);

  let latestResult = generateRampPieces({
    anchorPiece,
    targetHeightMm,
    targetLayerId: currentLayerId,
    rampShape: currentShape,
    curveDirection: currentDirection,
    curveAngleDeg: currentCurveAngle,
    selectedCurveCatalogId: currentCurveId,
    catalog,
    layers,
    allPieces: layout.pieces,
    targetGradientPercent: currentGradient,
  });

  function updateLiveCalculation(): void {
    const tH = parseFloat(targetHeightInput.value) || 0;
    const deltaH = Math.abs(tH - currentHeight);

    const selRadiusOpt = availableRadii.find((r) => r.catalogId === currentCurveId);
    if (radiusCircumHint) {
      if (currentShape === "helix" && selRadiusOpt) {
        const circum = Math.round(2 * Math.PI * selRadiusOpt.radiusMm);
        radiusCircumHint.textContent = isDe ? `Umfang: ${circum} mm/Runde` : `Circumference: ${circum} mm/turn`;
      } else {
        radiusCircumHint.textContent = "";
      }
    }

    latestResult = generateRampPieces({
      anchorPiece,
      targetHeightMm: tH,
      targetLayerId: currentLayerId,
      rampShape: currentShape,
      curveDirection: currentDirection,
      curveAngleDeg: currentCurveAngle,
      selectedCurveCatalogId: currentCurveId,
      catalog,
      layers,
      allPieces: layout.pieces,
      targetGradientPercent: currentGradient,
    });

    if (!latestResult.success) {
      statusCard.className = "ramp-status-card danger";
      statusTitle.textContent = isDe ? "⚠️ Rampe kann nicht erstellt werden" : "⚠️ Cannot generate ramp";
      if (latestResult.reason === "same_height") {
        statusDesc.textContent = isDe
          ? "Start- und Zielhöhe sind identisch. Bitte eine Höhendifferenz eingeben."
          : "Start and target heights are identical. Please specify a height difference.";
      } else {
        statusDesc.textContent = isDe ? "Kein offener Anschluss gefunden." : "No open track connector found.";
      }
      createBtn.disabled = true;
      return;
    }

    createBtn.disabled = false;
    const count = latestResult.pieces.length;
    const lenCm = (latestResult.totalLengthMm / 10).toFixed(1);
    const grad = latestResult.gradientPercent;

    let turnsText = "";
    if (currentShape === "helix") {
      const selCurve = catalog.entries.find((e) => e.id === currentCurveId);
      const cAngle = selCurve?.angle ?? 30;
      const piecesPerTurn = Math.max(4, Math.round(360 / Math.max(1, cAngle)));
      const turns = (count / piecesPerTurn).toFixed(1);
      turnsText = `, ${turns} ${isDe ? "Umdr." : "turns"}`;
    }

    if (grad <= 3.0) {
      statusCard.className = "ramp-status-card ok";
      statusTitle.textContent = `✓ ${isDe ? "Sichere Steigung" : "Safe gradient"}: ${grad.toFixed(1)}% (${count} ${isDe ? "Gleise" : "tracks"}${turnsText}, ${lenCm} cm)`;
    } else if (grad <= 4.5) {
      statusCard.className = "ramp-status-card warn";
      statusTitle.textContent = `⚠️ ${isDe ? "Mäßige Steigung" : "Moderate gradient"}: ${grad.toFixed(1)}% (${count} ${isDe ? "Gleise" : "tracks"}${turnsText}, ${lenCm} cm)`;
    } else {
      statusCard.className = "ramp-status-card danger";
      statusTitle.textContent = `⚠️ ${isDe ? "Steile Rampe" : "Steep ramp"}: ${grad.toFixed(1)}% (${count} ${isDe ? "Gleise" : "tracks"}${turnsText}, ${lenCm} cm)`;
    }

    let warningText = "";
    if (deltaH > 0 && deltaH < minClearance && currentShape !== "helix") {
      warningText = isDe
        ? ` (Hinweis: Höhendifferenz ${deltaH} mm < empfohlene Mindesthöhe ${minClearance} mm für Spur ${scale})`
        : ` (Note: Height difference ${deltaH} mm < minimum clearance ${minClearance} mm for scale ${scale})`;
    } else if (currentShape === "helix") {
      const selCurve = catalog.entries.find((e) => e.id === currentCurveId);
      const cAngle = selCurve?.angle ?? 30;
      const piecesPerTurn = Math.max(4, Math.round(360 / Math.max(1, cAngle)));
      const turns = count / piecesPerTurn;
      if (turns > 1.05) {
        const risePerTurn = Math.round((deltaH / turns) * 10) / 10;
        if (risePerTurn < minClearance) {
          warningText = isDe
            ? ` (Achtung: Durchfahrtshöhe ${risePerTurn} mm/Runde < Mindesthöhe ${minClearance} mm)`
            : ` (Warning: Clearance ${risePerTurn} mm/turn < min clearance ${minClearance} mm)`;
        }
      }
    }

    statusDesc.textContent = `${isDe ? "Verlauf" : "Profile"}: ${currentHeight} mm ➔ ${tH} mm (Δ ${deltaH} mm)${warningText}`;
  }

  updateLiveCalculation();

  createBtn.addEventListener("click", () => {
    if (!latestResult.success || latestResult.pieces.length === 0) return;

    const tH = parseFloat(targetHeightInput.value) || 0;
    const layerSelectVal = targetLayerSelect.value;
    let targetLayer: Layer;
    let newCreatedLayer: Layer | undefined;

    if (layerSelectVal === "__create_above__" || layerSelectVal === "__create_below__") {
      const isAbove = layerSelectVal === "__create_above__";
      const newLayerId = `layer-${Date.now()}`;
      const defaultName = isAbove
        ? (isDe ? `Bergebene (+${Math.round(tH)} mm)` : `Mountain Level (+${Math.round(tH)} mm)`)
        : (isDe ? `Schattenbahnhof (${Math.round(tH)} mm)` : `Staging Yard (${Math.round(tH)} mm)`);

      newCreatedLayer = {
        id: newLayerId,
        name: defaultName,
        visible: true,
        locked: false,
        heightMm: Math.round(tH),
      };
      targetLayer = newCreatedLayer;
    } else {
      targetLayer = layers.find((l) => l.id === layerSelectVal) || currentLayer;
    }

    // Re-run with confirmed targetLayerId
    const finalResult = generateRampPieces({
      anchorPiece,
      targetHeightMm: tH,
      targetLayerId: targetLayer.id,
      rampShape: currentShape,
      curveDirection: currentDirection,
      curveAngleDeg: currentCurveAngle,
      selectedCurveCatalogId: currentCurveId,
      catalog,
      layers: newCreatedLayer ? [...layers, newCreatedLayer] : layers,
      allPieces: layout.pieces,
      targetGradientPercent: currentGradient,
    });

    closeModal();
    if (finalResult.success && finalResult.pieces.length > 0) {
      onApply(finalResult.pieces, newCreatedLayer);
    }
  });
}
