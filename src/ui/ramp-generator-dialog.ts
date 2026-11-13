import rampGenHtml from "./templates/ramp-generator-dialog.html?raw";
import type { Layout, Layer, TrackPiece } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import { getLanguage } from "@/i18n";
import { getPieceConnectorHeight } from "@/core/snapping";
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
  /** Optional: Index des Anschlusses am Ankergleis, der als Startpunkt vorgewählt wird. */
  anchorConnectorIndex?: number;
  onApply: (generatedPieces: TrackPiece[], newLayer?: Layer) => void;
}

interface StartPointOption {
  piece: TrackPiece;
  connIdx: number;
  heightMm: number;
  label: string;
}

const MAX_START_OPTIONS = 60;

export function openRampGeneratorDialog(options: RampGeneratorDialogOptions): void {
  const { layout, catalog, anchorPiece, onApply } = options;
  const isDe = getLanguage() === "de";

  const existing = document.getElementById("ramp-generator-modal");
  if (existing) existing.remove();

  const layers = layout.layers || [
    { id: "layer-1", name: "Hauptebene", visible: true, locked: false, heightMm: 0 },
  ];

  const scale = layout.scale || catalog.scale || "H0";
  const minClearance = getMinClearanceHeightForScale(scale);

  // ---- Startpunkte sammeln (freie Anschlüsse; Ankergleis zuerst, dann nach Entfernung) ----
  const entryName = (p: TrackPiece): string =>
    catalog.entries.find((e) => e.id === p.catalogId)?.name || p.catalogId;
  const layerName = (id?: string): string => layers.find((l) => l.id === id)?.name || "—";
  const endLetter = (i: number): string => String.fromCharCode(65 + (i % 26));

  const startOptions: StartPointOption[] = [];
  const pushOptions = (piece: TrackPiece): void => {
    for (const ci of getOpenConnectors(piece, layout.pieces, 8.0, layers)) {
      const conn = piece.connectors[ci]!;
      const h = getPieceConnectorHeight(piece, ci, layers);
      const label =
        `#${piece.catalogId} ${entryName(piece)} · ${isDe ? "Ende" : "End"} ${endLetter(ci)} ` +
        `· ${Math.round(h)} mm · (${Math.round(conn.x)}, ${Math.round(conn.y)})`;
      startOptions.push({ piece, connIdx: ci, heightMm: h, label });
    }
  };

  pushOptions(anchorPiece);
  const others = layout.pieces
    .filter((p) => p.id !== anchorPiece.id)
    .filter((p) => layers.find((l) => l.id === p.layerId)?.visible !== false)
    .map((p) => ({
      p,
      d: Math.hypot(p.transform.x - anchorPiece.transform.x, p.transform.y - anchorPiece.transform.y),
    }))
    .sort((a, b) => a.d - b.d);
  for (const { p } of others) {
    if (startOptions.length >= MAX_START_OPTIONS) break;
    pushOptions(p);
  }

  if (startOptions.length === 0) {
    alert(
      isDe
        ? "Dieses Gleis hat keine freien Anschlüsse. Bitte ein Gleis mit offenem Ende wählen."
        : "This track has no open connectors. Please select a track with an open end.",
    );
    return;
  }

  // Vorauswahl: gewünschter Anschluss des Ankergleises, sonst erster freier Anschluss
  let startIdx = Math.max(
    0,
    startOptions.findIndex(
      (o) => o.piece.id === anchorPiece.id && o.connIdx === options.anchorConnectorIndex,
    ),
  );

  let startPiece = startOptions[startIdx]!.piece;
  let startConnIdx = startOptions[startIdx]!.connIdx;
  let currentHeight = startOptions[startIdx]!.heightMm;
  let currentLayerId = startPiece.layerId || layout.activeLayerId || layers[0]!.id;
  let currentLayer = layers.find((l) => l.id === currentLayerId) || layers[0]!;

  const availableRadii = getAvailableCurveRadii(catalog);
  let currentCurveId = availableRadii.length > 0 ? availableRadii[0]!.catalogId : undefined;

  let currentShape: RampShapeType = "straight";
  let currentDirection: "left" | "right" = "left";
  let currentCurveAngle = 180;
  let currentGradient = 2.8;

  const modal = document.createElement("div");
  modal.id = "ramp-generator-modal";
  modal.className = "rb-modal-overlay";

  // FIX: auch Vorzeichen (+/-) und Dezimal-Höhen entfernen, z. B. "Schattenbahnhof (-40 mm)" oder "Bergebene (+80 mm)"
  const cleanLayerName = (name: string): string =>
    name.replace(/\s*\(\s*[+-]?[0-9]+(?:[.,][0-9]+)?\s*mm\s*\)$/, "");

  /** Zielebenen-Optionen abhängig von der Starthöhe (höhere Ebene bevorzugt, sonst tiefere). */
  const computeLayerPreselect = (): { id: string; height: number } => {
    const higher = layers.find((l) => (l.heightMm ?? 0) > currentHeight + 0.1);
    if (higher) return { id: higher.id, height: higher.heightMm ?? 0 };

    // Keine höhere Ebene vorhanden: nächst tiefere Ebene anbieten (Abwärtsrampe)
    const lower = layers
      .filter((l) => (l.heightMm ?? 0) < currentHeight - 0.1)
      .sort((a, b) => (b.heightMm ?? 0) - (a.heightMm ?? 0))[0];
    if (lower) return { id: lower.id, height: lower.heightMm ?? 0 };

    return { id: "__create_above__", height: currentHeight + minClearance };
  };

  const buildLayerOptions = (preselectedId: string): string =>
    layers
      .map((l) => {
        const isSel = l.id === preselectedId ? "selected" : "";
        return `<option value="${l.id}" ${isSel}>${cleanLayerName(l.name)} (${l.heightMm ?? 0} mm)</option>`;
      })
      .join("") +
    `
    <option value="__create_above__" ${preselectedId === "__create_above__" ? "selected" : ""}>
      + ${isDe ? "Neue Ebene oberhalb erstellen" : "Create new layer above"} (+${minClearance} mm)
    </option>
    <option value="__create_below__" ${preselectedId === "__create_below__" ? "selected" : ""}>
      + ${isDe ? "Neue Ebene unterhalb erstellen" : "Create new layer below"} (-${minClearance} mm)
    </option>`;

  const initialPreselect = computeLayerPreselect();
  const targetHeightMm = initialPreselect.height;

  const startOptionsHtml = startOptions
    .map((o, i) => `<option value="${i}" ${i === startIdx ? "selected" : ""}>${o.label}</option>`)
    .join("");

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
    .replace("{{START_POINT_LABEL}}", isDe ? "Startpunkt (freier Anschluss)" : "Start Point (open connector)")
    .replace("{{START_OPTIONS}}", startOptionsHtml)
    .replace("{{START_TRACK_LABEL}}", isDe ? "Startgleis" : "Start Track")
    .replace("{{LAYER_LABEL}}", isDe ? "Ebene" : "Layer")
    .replace("{{OPEN_ENDS_LABEL}}", isDe ? "Freie Anschlüsse" : "Open Ends")
    .replace("{{TARGET_LAYER_LABEL}}", isDe ? "Zielebene / Zielhöhe" : "Target Layer / Target Elevation")
    .replace("{{LAYER_OPTIONS}}", buildLayerOptions(initialPreselect.id))
    .replace("{{TARGET_HEIGHT_LABEL}}", isDe ? "Zielhöhe (mm)" : "Target Height (mm)")
    .replace("{{CLEARANCE_HINT}}", isDe ? `Mindesthöhe Spur ${scale}: ${minClearance} mm` : `Min clearance scale ${scale}: ${minClearance} mm`)
    .replace("{{TARGET_HEIGHT_MM}}", String(Math.round(targetHeightMm)))
    .replace("{{SLOPE_UP_LABEL}}", isDe ? "Steigung" : "Up")
    .replace("{{SLOPE_DOWN_LABEL}}", isDe ? "Gefälle" : "Down")
    .replace("{{SLOPE_UP_TITLE}}", isDe ? "Aufwärtsrampe (+)" : "Ascending ramp (+)")
    .replace("{{SLOPE_DOWN_TITLE}}", isDe ? "Abwärtsrampe (-)" : "Descending ramp (-)")
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

  const startSelect = modal.querySelector("#ramp-start-point") as HTMLSelectElement;
  const startCount = modal.querySelector("#ramp-start-count") as HTMLElement;
  const startHint = modal.querySelector("#ramp-start-hint") as HTMLElement;
  const stripTrack = modal.querySelector("#ramp-strip-track") as HTMLElement;
  const stripLayer = modal.querySelector("#ramp-strip-layer") as HTMLElement;
  const stripOpen = modal.querySelector("#ramp-strip-open") as HTMLElement;

  const targetLayerSelect = modal.querySelector("#ramp-target-layer") as HTMLSelectElement;
  const targetHeightInput = modal.querySelector("#ramp-target-height-input") as HTMLInputElement;
  const btnSlopeUp = modal.querySelector("#btn-slope-up") as HTMLButtonElement | null;
  const btnSlopeDown = modal.querySelector("#btn-slope-down") as HTMLButtonElement | null;

  targetHeightInput.removeAttribute("min");

  const curveOptionsRow = modal.querySelector("#curve-options-row") as HTMLElement;
  const curveAngleContainer = modal.querySelector("#curve-angle-container") as HTMLElement;
  const curveRadiusSelect = modal.querySelector("#ramp-curve-radius") as HTMLSelectElement | null;
  const radiusCircumHint = modal.querySelector("#radius-circum-hint") as HTMLElement | null;
  const btnDirLeft = modal.querySelector("#btn-dir-left") as HTMLButtonElement;
  const btnDirRight = modal.querySelector("#btn-dir-right") as HTMLButtonElement;

  const statusCard = modal.querySelector("#ramp-live-status") as HTMLElement;
  const statusTitle = modal.querySelector("#status-title") as HTMLElement;
  const statusDesc = modal.querySelector("#status-desc") as HTMLElement;

  // Esc closes modal
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") closeModal();
  };
  const closeModal = () => {
    window.removeEventListener("keydown", handleKeyDown);
    modal.remove();
  };
  closeBtn.addEventListener("click", closeModal);
  cancelBtn.addEventListener("click", closeModal);
  window.addEventListener("keydown", handleKeyDown);

  /** Gemeinsame Generator-Parameter für den aktuell gewählten Startpunkt. */
  const buildParams = (tH: number, layerId: string, layerList: Layer[]) => ({
    anchorPiece: startPiece,
    connectorIndex: startConnIdx,
    targetHeightMm: tH,
    targetLayerId: layerId,
    rampShape: currentShape,
    curveDirection: currentDirection,
    curveAngleDeg: currentCurveAngle,
    selectedCurveCatalogId: currentCurveId,
    catalog,
    layers: layerList,
    allPieces: layout.pieces,
    targetGradientPercent: currentGradient,
  });

  const syncSlopeButtons = (tH: number): void => {
    const isDown = tH < currentHeight - 0.05;
    if (btnSlopeUp && btnSlopeDown) {
      btnSlopeUp.classList.toggle("active", !isDown);
      btnSlopeDown.classList.toggle("active", isDown);
    }
  };

  const syncLayerSelectFromHeight = (tH: number): void => {
    syncSlopeButtons(tH);
    const matchingLayer = layers.find((l) => Math.abs((l.heightMm ?? 0) - tH) < 0.5 && l.id !== currentLayerId);
    if (matchingLayer) {
      targetLayerSelect.value = matchingLayer.id;
      return;
    }
    if (tH < currentHeight - 0.1) {
      const selLayer = layers.find((l) => l.id === targetLayerSelect.value);
      if (targetLayerSelect.value === "__create_above__" || (selLayer && (selLayer.heightMm ?? 0) >= currentHeight - 0.1)) {
        targetLayerSelect.value = "__create_below__";
      }
    } else if (tH > currentHeight + 0.1) {
      const selLayer = layers.find((l) => l.id === targetLayerSelect.value);
      if (targetLayerSelect.value === "__create_below__" || (selLayer && (selLayer.heightMm ?? 0) <= currentHeight + 0.1)) {
        targetLayerSelect.value = "__create_above__";
      }
    }
  };

  const calcRamp = (tH: number) => generateRampPieces(buildParams(tH, currentLayerId, layers));

  /** Zusammenfassung + Zielebene an den gewählten Startpunkt anpassen. */
  function applyStartPoint(resetTarget: boolean): void {
    const opt = startOptions[startIdx]!;
    startPiece = opt.piece;
    startConnIdx = opt.connIdx;
    currentHeight = opt.heightMm;
    currentLayerId = startPiece.layerId || layout.activeLayerId || layers[0]!.id;
    currentLayer = layers.find((l) => l.id === currentLayerId) || layers[0]!;

    const openCount = getOpenConnectors(startPiece, layout.pieces, 8.0, layers).length;
    stripTrack.textContent = `#${startPiece.catalogId}`;
    stripLayer.textContent = `${cleanLayerName(currentLayer.name)} (${Math.round(currentHeight)} mm)`;
    stripOpen.textContent = String(openCount);
    startCount.textContent = isDe
      ? `${startOptions.length} Startpunkte verfügbar`
      : `${startOptions.length} start points available`;
    startHint.textContent = isDe
      ? `Ebene: ${cleanLayerName(layerName(startPiece.layerId))} · Starthöhe ${Math.round(currentHeight)} mm · ` +
        `Richtung ${Math.round(startPiece.connectors[startConnIdx]!.angle)}°`
      : `Layer: ${cleanLayerName(layerName(startPiece.layerId))} · Start height ${Math.round(currentHeight)} mm · ` +
        `Heading ${Math.round(startPiece.connectors[startConnIdx]!.angle)}°`;

    if (resetTarget) {
      const pre = computeLayerPreselect();
      targetLayerSelect.innerHTML = buildLayerOptions(pre.id);
      targetHeightInput.value = String(Math.round(pre.height));
      syncSlopeButtons(pre.height);
    }
  }

  startSelect.addEventListener("change", () => {
    startIdx = parseInt(startSelect.value, 10) || 0;
    applyStartPoint(true);
    updateLiveCalculation();
  });

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
      const absVal = Math.abs(val);
      if (!isNaN(absVal) && absVal >= 0.5 && absVal <= 15) {
        currentGradient = absVal;
        if (val < 0) {
          const parsedH = parseFloat(targetHeightInput.value);
          const curTH = isNaN(parsedH) ? currentHeight + minClearance : parsedH;
          if (curTH > currentHeight) {
            const absDelta = Math.abs(curTH - currentHeight) || minClearance;
            const newTH = Math.round((currentHeight - absDelta) * 10) / 10;
            targetHeightInput.value = String(newTH);
            syncLayerSelectFromHeight(newTH);
          }
        }
        modal.querySelectorAll<HTMLButtonElement>("button[data-grad]").forEach((b) => {
          if (Math.abs(parseFloat(b.dataset.grad || "0") - absVal) < 0.05) {
            b.classList.add("active");
          } else {
            b.classList.remove("active");
          }
        });
        updateLiveCalculation();
      }
    });
  }

  // Slope direction toggle (↑ Steigung / ↓ Gefälle)
  if (btnSlopeUp && btnSlopeDown) {
    btnSlopeUp.addEventListener("click", () => {
      const parsed = parseFloat(targetHeightInput.value);
      const curTH = isNaN(parsed) ? currentHeight : parsed;
      const absDelta = Math.abs(curTH - currentHeight) || minClearance;
      const newTH = Math.round((currentHeight + absDelta) * 10) / 10;
      targetHeightInput.value = String(newTH);
      syncLayerSelectFromHeight(newTH);
      updateLiveCalculation();
    });
    btnSlopeDown.addEventListener("click", () => {
      const parsed = parseFloat(targetHeightInput.value);
      const curTH = isNaN(parsed) ? currentHeight : parsed;
      const absDelta = Math.abs(curTH - currentHeight) || minClearance;
      const newTH = Math.round((currentHeight - absDelta) * 10) / 10;
      targetHeightInput.value = String(newTH);
      syncLayerSelectFromHeight(newTH);
      updateLiveCalculation();
    });
  }

  // Target layer change
  targetLayerSelect.addEventListener("change", () => {
    const val = targetLayerSelect.value;
    if (val === "__create_above__") {
      targetHeightInput.value = String(Math.round(currentHeight + minClearance));
    } else if (val === "__create_below__") {
      targetHeightInput.value = String(Math.round(currentHeight - minClearance));
    } else {
      const lay = layers.find((l) => l.id === val);
      if (lay) {
        targetHeightInput.value = String(lay.heightMm ?? 0);
      }
    }
    const parsed = parseFloat(targetHeightInput.value);
    syncSlopeButtons(isNaN(parsed) ? 0 : parsed);
    updateLiveCalculation();
  });

  targetHeightInput.addEventListener("input", () => {
    const parsed = parseFloat(targetHeightInput.value);
    if (!isNaN(parsed)) {
      syncLayerSelectFromHeight(parsed);
    }
    updateLiveCalculation();
  });

  let latestResult = calcRamp(targetHeightMm);

  function updateLiveCalculation(): void {
    const parsed = parseFloat(targetHeightInput.value);
    const tH = isNaN(parsed) ? 0 : parsed;
    syncSlopeButtons(tH);
    const signedDelta = Math.round((tH - currentHeight) * 10) / 10;
    const deltaH = Math.abs(signedDelta);

    const selRadiusOpt = availableRadii.find((r) => r.catalogId === currentCurveId);
    if (radiusCircumHint) {
      if (currentShape === "helix" && selRadiusOpt) {
        const circum = Math.round(2 * Math.PI * selRadiusOpt.radiusMm);
        radiusCircumHint.textContent = isDe ? `Umfang: ${circum} mm/Runde` : `Circumference: ${circum} mm/turn`;
      } else {
        radiusCircumHint.textContent = "";
      }
    }

    latestResult = calcRamp(tH);

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
    const grad = Math.abs(latestResult.gradientPercent);
    const dirIcon = signedDelta < 0 ? "↘" : "↗";

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
      statusTitle.textContent = `✓ ${dirIcon} ${isDe ? "Sichere Steigung" : "Safe gradient"}: ${grad.toFixed(1)}% (${count} ${isDe ? "Gleise" : "tracks"}${turnsText}, ${lenCm} cm)`;
    } else if (grad <= 4.5) {
      statusCard.className = "ramp-status-card warn";
      statusTitle.textContent = `⚠️ ${dirIcon} ${isDe ? "Mäßige Steigung" : "Moderate gradient"}: ${grad.toFixed(1)}% (${count} ${isDe ? "Gleise" : "tracks"}${turnsText}, ${lenCm} cm)`;
    } else {
      statusCard.className = "ramp-status-card danger";
      statusTitle.textContent = `⚠️ ${dirIcon} ${isDe ? "Steile Rampe" : "Steep ramp"}: ${grad.toFixed(1)}% (${count} ${isDe ? "Gleise" : "tracks"}${turnsText}, ${lenCm} cm)`;
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

    statusDesc.textContent = `${isDe ? "Verlauf" : "Profile"}: ${Math.round(currentHeight * 10) / 10} mm ➔ ${tH} mm (Δ ${signedDelta > 0 ? "+" : ""}${signedDelta} mm)${warningText}`;
  }

  applyStartPoint(false);
  updateLiveCalculation();

  createBtn.addEventListener("click", () => {
    if (!latestResult.success || latestResult.pieces.length === 0) return;

    const parsed = parseFloat(targetHeightInput.value);
    const tH = isNaN(parsed) ? 0 : parsed;
    syncLayerSelectFromHeight(tH);
    const layerSelectVal = targetLayerSelect.value;
    let targetLayer: Layer;
    let newCreatedLayer: Layer | undefined;

    if (layerSelectVal === "__create_above__" || layerSelectVal === "__create_below__") {
      const isAbove = tH >= currentHeight;
      const newLayerId = `layer-${Date.now()}`;
      const sign = tH > 0 ? "+" : "";
      const defaultName = isAbove
        ? (isDe ? `Bergebene (${sign}${Math.round(tH)} mm)` : `Mountain Level (${sign}${Math.round(tH)} mm)`)
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

    const finalLayers: Layer[] = newCreatedLayer ? [...layers, newCreatedLayer] : layers;
    const finalResult = generateRampPieces(buildParams(tH, targetLayer.id, finalLayers));

    closeModal();
    if (finalResult.success && finalResult.pieces.length > 0) {
      onApply(finalResult.pieces, newCreatedLayer);
    }
  });
}