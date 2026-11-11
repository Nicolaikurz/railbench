import type { LayerWoodPlateMode, Layout } from "@/core/types";
import { getTranslations } from "@/i18n";

export interface WoodPlateDialogOptions {
  layout: Layout;
  currentMode?: LayerWoodPlateMode;
  currentThickness?: number;
  onApply: (mode: LayerWoodPlateMode, thickness: number) => void;
}

/**
 * Öffnet einen modalen Dialog zur Festlegung der Holzplatten-Bauweise für alle Ebenen:
 * - "table": Vollflächige Platten passend zur Tischform für jede Ebene (0 mm = Tischplatte)
 * - "tight": Nur Trassenbretter unter Gleisen & Fundamente unter Gebäuden (0 mm = offene Rahmenbauweise)
 * - "none": Keine Holzplatten
 */
export function openWoodPlateBatchDialog(options: WoodPlateDialogOptions): void {
  const existing = document.getElementById("woodplate-batch-modal");
  if (existing) existing.remove();

  const t = getTranslations().rightSidebar;
  const tDlg = getTranslations().layerDialog;

  // Initialer Modus: Vom ersten Layer ableiten oder Standard "table"
  const firstLayer = options.layout.layers?.[0];
  let selectedMode: LayerWoodPlateMode =
    options.currentMode ?? firstLayer?.woodPlateMode ?? "table";
  let thickness =
    options.currentThickness ?? firstLayer?.woodPlateThicknessMm ?? 12;

  const modal = document.createElement("div");
  modal.id = "woodplate-batch-modal";
  modal.className = "rb-modal-overlay";

  modal.innerHTML = `
    <div class="rb-modal-card" style="width: 480px; max-width: 95vw; background: var(--panel-bg, #ffffff); border-radius: 12px; box-shadow: 0 20px 40px rgba(0,0,0,0.3); border: 1px solid var(--border-color, #cbd5e1); overflow: hidden; display: flex; flex-direction: column;">
      <div class="rb-modal-header" style="padding: 16px 20px; border-bottom: 1px solid var(--border-color, #cbd5e1); display: flex; align-items: center; justify-content: space-between;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 20px;">🪵</span>
          <span style="font-weight: 700; font-size: 16px; color: var(--text-color, #0f172a);">${t.woodPlateBatchDialogTitle}</span>
        </div>
        <button type="button" id="wp-btn-close" style="background: none; border: none; font-size: 18px; cursor: pointer; color: var(--text-muted, #64748b);">✕</button>
      </div>

      <div style="padding: 16px 20px; display: flex; flex-direction: column; gap: 14px;">
        <p style="margin: 0; font-size: 12px; color: var(--text-muted, #64748b); line-height: 1.45;">
          ${t.woodPlateBatchDialogDesc}
        </p>

        <div style="display: flex; flex-direction: column; gap: 8px;">
          <!-- 1. Passend zum Tisch -->
          <label class="wp-choice-card ${selectedMode === "table" ? "active" : ""}" data-mode="table" style="display: flex; align-items: flex-start; gap: 12px; padding: 10px 14px; border-radius: 8px; border: 1.5px solid ${selectedMode === "table" ? "#0284c7" : "var(--border-color, #cbd5e1)"}; background: ${selectedMode === "table" ? "rgba(2, 132, 199, 0.06)" : "transparent"}; cursor: pointer; transition: all 0.15s ease;">
            <input type="radio" name="wp-mode" value="table" ${selectedMode === "table" ? "checked" : ""} style="margin-top: 3px;" />
            <div style="display: flex; flex-direction: column; gap: 2px;">
              <span style="font-weight: 600; font-size: 13px; color: var(--text-color, #0f172a);">${tDlg.woodPlateTableTitle}</span>
              <span style="font-size: 11px; color: var(--text-muted, #64748b); line-height: 1.4;">${tDlg.woodPlateTableDesc}</span>
            </div>
          </label>

          <!-- 2. Nur Gleise & Gebäude -->
          <label class="wp-choice-card ${selectedMode === "tight" ? "active" : ""}" data-mode="tight" style="display: flex; align-items: flex-start; gap: 12px; padding: 10px 14px; border-radius: 8px; border: 1.5px solid ${selectedMode === "tight" ? "#0284c7" : "var(--border-color, #cbd5e1)"}; background: ${selectedMode === "tight" ? "rgba(2, 132, 199, 0.06)" : "transparent"}; cursor: pointer; transition: all 0.15s ease;">
            <input type="radio" name="wp-mode" value="tight" ${selectedMode === "tight" ? "checked" : ""} style="margin-top: 3px;" />
            <div style="display: flex; flex-direction: column; gap: 2px;">
              <span style="font-weight: 600; font-size: 13px; color: var(--text-color, #0f172a);">${tDlg.woodPlateTightTitle}</span>
              <span style="font-size: 11px; color: var(--text-muted, #64748b); line-height: 1.4;">${tDlg.woodPlateTightDesc}</span>
            </div>
          </label>

          <!-- 3. Keine Holzplatten -->
          <label class="wp-choice-card ${selectedMode === "none" ? "active" : ""}" data-mode="none" style="display: flex; align-items: flex-start; gap: 12px; padding: 10px 14px; border-radius: 8px; border: 1.5px solid ${selectedMode === "none" ? "#0284c7" : "var(--border-color, #cbd5e1)"}; background: ${selectedMode === "none" ? "rgba(2, 132, 199, 0.06)" : "transparent"}; cursor: pointer; transition: all 0.15s ease;">
            <input type="radio" name="wp-mode" value="none" ${selectedMode === "none" ? "checked" : ""} style="margin-top: 3px;" />
            <div style="display: flex; flex-direction: column; gap: 2px;">
              <span style="font-weight: 600; font-size: 13px; color: var(--text-color, #0f172a);">${tDlg.woodPlateNoneTitle}</span>
              <span style="font-size: 11px; color: var(--text-muted, #64748b); line-height: 1.4;">${tDlg.woodPlateNoneDesc}</span>
            </div>
          </label>
        </div>

        <!-- Plattenstärke-Eingabe -->
        <div id="wp-thickness-row" style="display: ${selectedMode === "none" ? "none" : "flex"}; align-items: center; justify-content: space-between; padding: 8px 12px; background: rgba(0,0,0,0.03); border-radius: 6px;">
          <label for="wp-thickness-input" style="font-size: 12px; font-weight: 600; color: var(--text-color, #0f172a);">${tDlg.woodPlateThicknessLabel}</label>
          <div style="position: relative; width: 110px;">
            <input
              type="number"
              id="wp-thickness-input"
              class="rb-input"
              style="width: 100%; padding-right: 32px; font-weight: 600;"
              value="${thickness}"
              min="2"
              max="50"
              step="1"
            />
            <span style="position: absolute; right: 10px; top: 50%; transform: translateY(-50%); font-size: 12px; color: var(--text-muted, #64748b);">mm</span>
          </div>
        </div>
      </div>

      <div style="padding: 12px 20px; background: rgba(0,0,0,0.02); border-top: 1px solid var(--border-color, #cbd5e1); display: flex; justify-content: flex-end; gap: 8px;">
        <button type="button" id="wp-btn-cancel" class="cl-btn" style="padding: 6px 14px; font-size: 13px;">${tDlg.cancelBtn}</button>
        <button type="button" id="wp-btn-apply" class="cl-btn is-primary" style="padding: 6px 18px; font-size: 13px; font-weight: 600;">✓ ${tDlg.saveBtn}</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  const closeDialog = () => {
    modal.remove();
  };

  modal.querySelector("#wp-btn-close")?.addEventListener("click", closeDialog);
  modal.querySelector("#wp-btn-cancel")?.addEventListener("click", closeDialog);
  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeDialog();
  });

  const cards = modal.querySelectorAll<HTMLElement>(".wp-choice-card");
  const thicknessRow = modal.querySelector<HTMLElement>("#wp-thickness-row");

  cards.forEach((card) => {
    card.addEventListener("click", () => {
      const mode = card.dataset.mode as LayerWoodPlateMode;
      selectedMode = mode;
      cards.forEach((c) => {
        const isThis = c.dataset.mode === mode;
        c.style.borderColor = isThis ? "#0284c7" : "var(--border-color, #cbd5e1)";
        c.style.background = isThis ? "rgba(2, 132, 199, 0.06)" : "transparent";
        const radio = c.querySelector<HTMLInputElement>("input[type='radio']");
        if (radio) radio.checked = isThis;
      });
      if (thicknessRow) {
        thicknessRow.style.display = mode === "none" ? "none" : "flex";
      }
    });
  });

  modal.querySelector("#wp-btn-apply")?.addEventListener("click", () => {
    const inputEl = modal.querySelector<HTMLInputElement>("#wp-thickness-input");
    const parsed = parseFloat(inputEl?.value ?? "12");
    const finalThickness = isNaN(parsed) ? 12 : Math.max(2, Math.min(50, parsed));
    closeDialog();
    options.onApply(selectedMode, finalThickness);
  });
}
