import layerDialogHtml from "./templates/layer-dialog.html?raw";
import type { Layout, Layer, LayerWoodPlateMode } from "@/core/types";
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
  let currentWoodPlateMode: LayerWoodPlateMode = isEdit && layer?.woodPlateMode ? layer.woodPlateMode : "none";
  let currentWoodPlateThickness = isEdit && layer?.woodPlateThicknessMm ? layer.woodPlateThicknessMm : 10;

  const modal = document.createElement("div");
  modal.id = "layer-modal";
  modal.className = "rb-modal-overlay";

  const swatchesHtml = LAYER_COLOR_PALETTE.map(
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
  ).join("");

  modal.innerHTML = layerDialogHtml
    .replace("{{TITLE}}", isEdit ? t.editTitle : t.addTitle)
    .replace("{{NAME_LABEL}}", t.nameLabel)
    .replace("{{CURRENT_NAME}}", escapeHtml(currentName))
    .replace("{{NAME_PLACEHOLDER}}", t.namePlaceholder)
    .replace("{{ELEVATION_LABEL}}", t.elevationLabel)
    .replace("{{CURRENT_HEIGHT}}", String(currentHeight))
    .replace("{{ELEVATION_UNIT}}", t.elevationUnit)
    .replace("{{COLOR_LABEL}}", t.colorLabel)
    .replace("{{SWATCHES}}", swatchesHtml)
    .replace("{{CUSTOM_COLOR_TOOLTIP}}", t.customColorTooltip)
    .replace("{{CURRENT_COLOR}}", currentColor)
    .replace("{{WOOD_PLATE_LABEL}}", t.woodPlateLabel)
    .replace("{{WOOD_PLATE_NONE_ACTIVE}}", currentWoodPlateMode === "none" ? "active" : "")
    .replace("{{WOOD_PLATE_NONE_CHECKED}}", currentWoodPlateMode === "none" ? "checked" : "")
    .replace("{{WOOD_PLATE_NONE_TITLE}}", t.woodPlateNoneTitle)
    .replace("{{WOOD_PLATE_NONE_DESC}}", t.woodPlateNoneDesc)
    .replace("{{WOOD_PLATE_TABLE_ACTIVE}}", currentWoodPlateMode === "table" ? "active" : "")
    .replace("{{WOOD_PLATE_TABLE_CHECKED}}", currentWoodPlateMode === "table" ? "checked" : "")
    .replace("{{WOOD_PLATE_TABLE_TITLE}}", t.woodPlateTableTitle)
    .replace("{{WOOD_PLATE_TABLE_DESC}}", t.woodPlateTableDesc)
    .replace("{{WOOD_PLATE_TIGHT_ACTIVE}}", currentWoodPlateMode === "tight" ? "active" : "")
    .replace("{{WOOD_PLATE_TIGHT_CHECKED}}", currentWoodPlateMode === "tight" ? "checked" : "")
    .replace("{{WOOD_PLATE_TIGHT_TITLE}}", t.woodPlateTightTitle)
    .replace("{{WOOD_PLATE_TIGHT_DESC}}", t.woodPlateTightDesc)
    .replace("{{THICKNESS_ROW_DISPLAY}}", currentWoodPlateMode === "none" ? "none" : "flex")
    .replace("{{WOOD_PLATE_THICKNESS_LABEL}}", t.woodPlateThicknessLabel)
    .replace("{{CURRENT_THICKNESS}}", String(currentWoodPlateThickness))
    .replace("{{CANCEL_BTN}}", t.cancelBtn)
    .replace("{{SUBMIT_BTN}}", isEdit ? t.saveBtn : t.createBtn);

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#layer-modal-close") as HTMLButtonElement;
  const cancelBtn = modal.querySelector("#layer-btn-cancel") as HTMLButtonElement;
  const submitBtn = modal.querySelector("#layer-btn-submit") as HTMLButtonElement;

  const nameInput = modal.querySelector("#layer-input-name") as HTMLInputElement;
  const heightInput = modal.querySelector("#layer-input-height") as HTMLInputElement;
  const customColorInput = modal.querySelector("#layer-custom-color-input") as HTMLInputElement;
  const swatchBtns = modal.querySelectorAll<HTMLButtonElement>(".layer-swatch-btn");

  const woodPlateRadios = modal.querySelectorAll<HTMLInputElement>('input[name="layer-woodplate-mode"]');
  const woodPlateChoices = modal.querySelectorAll<HTMLLabelElement>(".layer-woodplate-choice");
  const thicknessRow = modal.querySelector("#layer-woodplate-thickness-row") as HTMLElement;
  const thicknessInput = modal.querySelector("#layer-input-thickness") as HTMLInputElement;

  woodPlateRadios.forEach((radio) => {
    radio.addEventListener("change", () => {
      currentWoodPlateMode = radio.value as LayerWoodPlateMode;
      woodPlateChoices.forEach((choice) => {
        const r = choice.querySelector("input");
        choice.classList.toggle("active", r?.value === currentWoodPlateMode);
      });
      if (thicknessRow) {
        thicknessRow.style.display = currentWoodPlateMode === "none" ? "none" : "flex";
      }
    });
  });

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
    const parsedThickness = parseFloat(thicknessInput?.value ?? "10");
    const finalThickness = isNaN(parsedThickness) ? 10 : Math.max(2, Math.min(50, parsedThickness));

    const resultLayer: Layer = {
      id: isEdit && layer ? layer.id : `layer-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      name: finalName,
      color: currentColor,
      heightMm: Math.round(finalHeight * 10) / 10,
      visible: isEdit && layer ? layer.visible !== false : true,
      locked: isEdit && layer ? layer.locked === true : false,
      woodPlateMode: currentWoodPlateMode,
      woodPlateThicknessMm: finalThickness,
      woodPlateStyle: isEdit && layer?.woodPlateStyle ? layer.woodPlateStyle : undefined,
      plates: isEdit && layer?.plates ? layer.plates : undefined,
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
