import type { Layout, Layer } from "@/core/types";
import { getTranslations } from "@/i18n";

export interface LayerDialogOptions {
  layout: Layout;
  layer?: Layer; // Falls übergeben: Bearbeiten einer bestehenden Ebene, sonst Neuanlage
  position?: "above" | "below" | "auto";
  targetHeightMm?: number;
  onApply: (layer: Layer, position: "above" | "below" | "auto", makeActive: boolean) => void;
}

export const LAYER_COLOR_PALETTE = [
  "#38bdf8", // Himmelblau (Sky Blue)
  "#34d399", // Smaragdgrün (Emerald Green)
  "#fbbf24", // Bernsteingold (Amber Gold)
  "#f472b6", // Rosenpink (Rose Pink)
  "#a78bfa", // Lavendelviolett (Purple)
  "#f97316", // Warmorange (Orange)
  "#2dd4bf", // Türkis (Teal Cyan)
  "#ef4444", // Korallenrot (Coral Red)
  "#60a5fa", // Kobaltblau (Cobalt Blue)
  "#94a3b8", // Schiefergrau (Slate Gray)
];

/**
 * Öffnet das fokussierte Ebenen-Fenster für Ebenenname, Ebenenhöhe und Ebenenfarbe.
 */
export function openLayerDialog(options: LayerDialogOptions): void {
  const { layout, layer, targetHeightMm } = options;
  const isEdit = !!layer;
  const t = getTranslations().layerDialog;

  const existing = document.getElementById("layer-modal");
  if (existing) existing.remove();

  const layers = layout.layers || [];
  const nextNum = layers.length + 1;

  // Höhen der bestehenden Ebenen ermitteln
  const existingHeights = layers.map((l) => l.heightMm ?? 0);
  const maxH = layers.length > 0 ? Math.max(...existingHeights) : 0;
  const minH = layers.length > 0 ? Math.min(...existingHeights) : 0;

  let initialHeight = 0;
  if (isEdit && layer) {
    initialHeight = layer.heightMm ?? 0;
  } else if (targetHeightMm !== undefined) {
    initialHeight = targetHeightMm;
  } else if (options.position === "below") {
    initialHeight = minH - 80;
  } else {
    initialHeight = layers.length > 0 ? maxH + 80 : 0;
  }

  let currentName = isEdit && layer ? layer.name : `Layer ${nextNum}`;
  let currentHeight = initialHeight;
  let currentColor = isEdit && layer
    ? (layer.color || LAYER_COLOR_PALETTE[0]!)
    : LAYER_COLOR_PALETTE[(nextNum - 1) % LAYER_COLOR_PALETTE.length]!;

  const modal = document.createElement("div");
  modal.id = "layer-modal";
  modal.className = "rb-modal-overlay";

  modal.innerHTML = `
    <div class="rb-modal-card layer-modal-card" style="width: 440px;">
      <div class="rb-modal-header">
        <div class="rb-modal-title">
          <span>${isEdit ? t.editTitle : t.addTitle}</span>
        </div>
        <button class="rb-modal-close-btn" id="layer-modal-close" title="✕">✕</button>
      </div>

      <div class="layer-modal-body" style="padding: 18px 20px; display: flex; flex-direction: column; gap: 14px;">
        <!-- 1. Layer Name -->
        <div class="layer-dialog-section">
          <div class="layer-input-row">
            <label for="layer-input-name" class="layer-dialog-label">${t.nameLabel}</label>
            <input
              type="text"
              id="layer-input-name"
              class="rb-input layer-text-input"
              value="${escapeHtml(currentName)}"
              placeholder="${t.namePlaceholder}"
              maxlength="50"
            />
          </div>
        </div>

        <!-- 2. Layer Height -->
        <div class="layer-dialog-section">
          <div class="layer-input-row">
            <label for="layer-input-height" class="layer-dialog-label">${t.elevationLabel}</label>
            <div class="layer-unit-input-group" style="width: 100%;">
              <input
                type="number"
                id="layer-input-height"
                class="rb-input layer-height-field"
                value="${currentHeight}"
                step="1"
              />
              <span class="unit-addon" style="position: absolute; right: 12px; font-size: 13px; color: var(--text-muted); font-weight: 600;">${t.elevationUnit}</span>
            </div>
          </div>
        </div>

        <!-- 3. Layer Color -->
        <div class="layer-dialog-section">
          <div class="layer-color-row">
            <span class="layer-dialog-label">${t.colorLabel}</span>
            <div class="layer-color-swatches" id="layer-color-swatches">
              ${LAYER_COLOR_PALETTE.map(
                (hex) => `
                <button
                  type="button"
                  class="layer-swatch-btn ${hex.toLowerCase() === currentColor.toLowerCase() ? "active" : ""}"
                  data-color="${hex}"
                  style="background-color: ${hex};"
                  title="${hex}"
                >
                  <span class="swatch-check">✓</span>
                </button>
              `,
              ).join("")}
              <label class="layer-custom-color-picker" title="${t.customColorTooltip}">
                <input type="color" id="layer-custom-color-input" value="${currentColor}" />
                <span class="picker-icon">🎨</span>
              </label>
            </div>
          </div>
        </div>
      </div>

      <div class="rb-modal-footer">
        <button type="button" class="rb-btn rb-btn-secondary" id="layer-btn-cancel">${t.cancelBtn}</button>
        <button type="button" class="rb-btn rb-btn-primary" id="layer-btn-submit">${isEdit ? t.saveBtn : t.createBtn}</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#layer-modal-close") as HTMLButtonElement;
  const cancelBtn = modal.querySelector("#layer-btn-cancel") as HTMLButtonElement;
  const submitBtn = modal.querySelector("#layer-btn-submit") as HTMLButtonElement;

  const nameInput = modal.querySelector("#layer-input-name") as HTMLInputElement;
  const heightInput = modal.querySelector("#layer-input-height") as HTMLInputElement;
  const customColorInput = modal.querySelector("#layer-custom-color-input") as HTMLInputElement;
  const swatchBtns = modal.querySelectorAll<HTMLButtonElement>(".layer-swatch-btn");

  function updateColorSwatches() {
    swatchBtns.forEach((btn) => {
      const color = btn.dataset.color || "";
      btn.classList.toggle("active", color.toLowerCase() === currentColor.toLowerCase());
    });
    customColorInput.value = currentColor;
  }

  // Farb-Swatches
  swatchBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      currentColor = btn.dataset.color || "#38bdf8";
      updateColorSwatches();
    });
  });

  customColorInput.addEventListener("input", () => {
    currentColor = customColorInput.value;
    updateColorSwatches();
  });

  const closeDialog = () => {
    modal.classList.add("closing");
    setTimeout(() => modal.remove(), 120);
  };

  const doSubmit = () => {
    const finalName = nameInput.value.trim() || (isEdit && layer ? layer.name : `Layer ${nextNum}`);
    const parsedHeight = parseFloat(heightInput.value);
    const finalHeight = isNaN(parsedHeight) ? 0 : parsedHeight;

    const resultLayer: Layer = {
      id: isEdit && layer ? layer.id : `layer-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      name: finalName,
      color: currentColor,
      heightMm: Math.round(finalHeight * 10) / 10,
      visible: isEdit && layer ? layer.visible !== false : true,
      locked: isEdit && layer ? layer.locked === true : false,
    };

    closeDialog();
    options.onApply(resultLayer, "auto", !isEdit);
  };

  closeBtn.addEventListener("click", closeDialog);
  cancelBtn.addEventListener("click", closeDialog);
  submitBtn.addEventListener("click", doSubmit);

  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeDialog();
  });

  const handleKeydown = (ev: KeyboardEvent) => {
    if (ev.key === "Escape") {
      ev.preventDefault();
      closeDialog();
    } else if (ev.key === "Enter") {
      ev.preventDefault();
      doSubmit();
    }
  };

  nameInput.addEventListener("keydown", handleKeydown);
  heightInput.addEventListener("keydown", handleKeydown);

  nameInput.focus();
  nameInput.select();
}

function escapeHtml(str: string): string {
  return str.replace(/[&<>"']/g, (m) => {
    switch (m) {
      case "&": return "&amp;";
      case "<": return "&lt;";
      case ">": return "&gt;";
      case '"': return "&quot;";
      case "'": return "&#039;";
      default: return m;
    }
  });
}
