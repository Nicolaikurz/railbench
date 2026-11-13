import rampGenHtml from "./templates/ramp-generator-dialog.html?raw";
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

  const layerOptions = layers
    .map((l) => {
      const cleanName = l.name.replace(/\\s*\\([0-9]+\\s*mm\\)$/, "");
      const isSel = l.id === preselectedLayerId ? "selected" : "";
      return `<option value="${l.id}" ${isSel}>${cleanName} (${l.heightMm ?? 0} mm)</option>`;
    })
    .join("") + `
    <option value="__create_above__" ${preselectedLayerId === "__create_above__" ? "selected" : ""}>
      + ${isDe ? "Neue Ebene oberhalb erstellen" : "Create new layer above"} (+${minClearance} mm)
    </option>
    <option value="__create_below__">
      + ${isDe ? "Neue Ebene unterhalb erstellen" : "Create new layer below"} (-${minClearance} mm)
    </option>`;

  let curveRadiusSection = "";
  if (availableRadii.length > 0) {
    curveRadiusSection = `
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
    </div>`;
  }

  modal.innerHTML = rampGenHtml
    .replace("{{HEADER_TITLE}}", isDe ? "Gleisrampe automatisch erstellen" : "Auto-Create Track Ramp")
    .replace("{{START_TRACK_LABEL}}", isDe ? "Startgleis" : "Start Track")
    .replace("{{ANCHOR_PIECE_ID}}", anchorPiece.catalogId)
    .replace("{{LAYER_LABEL}}", isDe ? "Ebene" : "Layer")
    .replace("{{CURRENT_LAYER_NAME}}", currentLayer.name.replace(/\\s*\\([0-9]+\\s*mm\\)$/, ""))
    .replace("{{CURRENT_HEIGHT}}", String(currentHeight))
    .replace("{{OPEN_ENDS_LABEL}}", isDe ? "Freie Anschlüsse" : "Open Ends")
    .replace("{{OPEN_CONNS_COUNT}}", String(openConns.length))
    .replace("{{TARGET_LAYER_LABEL}}", isDe ? "Zielebene / Zielhöhe" : "Target Layer / Target Elevation")
    .replace("{{LAYER_OPTIONS}}", layerOptions)
    .replace("{{TARGET_HEIGHT_LABEL}}", isDe ? "Zielhöhe (mm)" : "Target Height (mm)")
    .replace("{{CLEARANCE_HINT}}", isDe ? `Mindesthöhe Spur ${scale}: ${minClearance} mm` : `Min clearance scale ${scale}: ${minClearance} mm`)
    .replace("{{TARGET_HEIGHT_MM}}", String(targetHeightMm))
    .replace("{{RAMP_SHAPE_LABEL}}", isDe ? "Rampenverlauf / Form" : "Ramp Shape / Geometry")
    .replace("{{SHAPE_STRAIGHT}}", isDe ? "Gerade" : "Straight")
    .replace("{{SHAPE_CURVED}}", isDe ? "Bogen / Kehre" : "Curve / Turn")
    .replace("{{SHAPE_HELIX}}", isDe ? "Wendel (Helix)" : "Helix (Spiral)")
    .replace("{{CURVE_RADIUS_SECTION}}", curveRadiusSection)
    .replace("{{CURVE_DIR_LABEL}}", isDe ? "Bogenrichtung" : "Curve Direction")
    .replace("{{DIR_LEFT}}", isDe ? "Links" : "Left")
    .replace("{{DIR_RIGHT}}", isDe ? "Rechts" : "Right")
    .replace("{{TURN_ANGLE_LABEL}}", isDe ? "Drehwinkel" : "Turn Angle")
    .replace("{{TARGET_GRADIENT_LABEL}}", isDe ? "Ziel-Steigung" : "Target Gradient")
    .replace("{{CANCEL_LABEL}}", isDe ? "Abbrechen" : "Cancel")
    .replace("{{CREATE_LABEL}}", isDe ? "Rampe erstellen" : "Create Ramp");

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
