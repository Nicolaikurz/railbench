import { DEMO_LAYOUTS, type DemoLayoutDef } from "@/editor/demo-tracks";
import { getLanguage, getTranslations } from "@/i18n";
import type { ModelScale } from "@/core/model-scale";

export interface DemoTracksDialogOptions {
  currentLayoutDirty?: boolean;
  onSelect: (demo: DemoLayoutDef) => void;
  onCancel?: () => void;
}

const DIALOG_ID = "railbench-demo-tracks-dialog";

export function showDemoTracksDialog(options: DemoTracksDialogOptions): Promise<DemoLayoutDef | null> {
  return new Promise((resolve) => {
    const existing = document.getElementById(DIALOG_ID);
    if (existing) existing.remove();

    const isDe = getLanguage() === "de";
    const t = getTranslations().demoDialog;

    const overlay = document.createElement("div");
    overlay.id = DIALOG_ID;
    overlay.className = "modal-overlay demo-dialog-overlay";
    overlay.style.cssText = `
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.72);
      backdrop-filter: blur(8px);
      z-index: 99999;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 16px;
      box-sizing: border-box;
      animation: demoFadeIn 0.18s ease-out;
    `;

    let selectedScale: "all" | ModelScale = "all";

    const modal = document.createElement("div");
    modal.className = "modal-container demo-dialog-modal";
    modal.style.cssText = `
      background: var(--bg-surface, #1e293b);
      color: var(--text-main, #f8fafc);
      border: 1px solid var(--border-main, rgba(255, 255, 255, 0.15));
      border-radius: 16px;
      box-shadow: 0 20px 45px rgba(0, 0, 0, 0.5), 0 0 1px rgba(255, 255, 255, 0.2);
      width: 100%;
      max-width: 920px;
      max-height: 90vh;
      display: flex;
      flex-direction: column;
      overflow: hidden;
      font-family: inherit;
    `;

    // 1. Header
    const header = document.createElement("div");
    header.style.cssText = `
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 20px 24px 14px;
      border-bottom: 1px solid var(--border-main, rgba(255, 255, 255, 0.08));
    `;
    header.innerHTML = `
      <div>
        <h2 style="margin: 0; font-size: 19px; font-weight: 700; display: flex; align-items: center; gap: 8px;">
          <span>🚆</span>
          <span>${t.title}</span>
        </h2>
        <p style="margin: 4px 0 0; font-size: 12.5px; color: var(--text-muted, #94a3b8);">
          ${t.subtitle}
        </p>
      </div>
      <button type="button" class="modal-close-btn" aria-label="Close" style="
        background: transparent;
        border: none;
        font-size: 22px;
        line-height: 1;
        cursor: pointer;
        color: var(--text-muted, #94a3b8);
        padding: 4px 8px;
        border-radius: 8px;
        transition: background 0.15s, color 0.15s;
      ">×</button>
    `;

    const closeBtn = header.querySelector(".modal-close-btn") as HTMLButtonElement;

    // 2. Filter Bar
    const filterBar = document.createElement("div");
    filterBar.style.cssText = `
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 12px 24px;
      background: rgba(0, 0, 0, 0.12);
      border-bottom: 1px solid var(--border-main, rgba(255, 255, 255, 0.06));
      overflow-x: auto;
    `;

    const scalesList: Array<{ id: "all" | ModelScale; label: string }> = [
      { id: "all", label: `${t.filterAll} (${DEMO_LAYOUTS.length})` },
      { id: "H0", label: `H0 (1:87)` },
      { id: "N", label: `N (1:160)` },
      { id: "G", label: `G (1:22.5)` },
      { id: "0", label: `0 (1:45)` },
      { id: "Z", label: `Z (1:220)` },
    ];

    // 3. Grid container
    const bodyContainer = document.createElement("div");
    bodyContainer.style.cssText = `
      padding: 20px 24px;
      overflow-y: auto;
      flex: 1;
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(265px, 1fr));
      gap: 16px;
    `;

    // 4. Footer
    const footer = document.createElement("div");
    footer.style.cssText = `
      padding: 14px 24px;
      background: rgba(0, 0, 0, 0.18);
      border-top: 1px solid var(--border-main, rgba(255, 255, 255, 0.08));
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      font-size: 11.5px;
      color: var(--text-muted, #94a3b8);
    `;
    footer.innerHTML = `
      <div style="display: flex; align-items: center; gap: 6px;">
        <span>💡</span>
        <span>${t.autoSwitchHint}</span>
      </div>
      <button type="button" class="btn-secondary demo-close-footer-btn" style="
        background: rgba(255, 255, 255, 0.08);
        border: 1px solid rgba(255, 255, 255, 0.12);
        color: var(--text-main, #f8fafc);
        padding: 6px 14px;
        border-radius: 8px;
        cursor: pointer;
        font-size: 12px;
        font-weight: 500;
      ">${t.closeBtn}</button>
    `;

    const closeFooterBtn = footer.querySelector(".demo-close-footer-btn") as HTMLButtonElement;

    const cleanup = (chosen: DemoLayoutDef | null) => {
      document.removeEventListener("keydown", onKeyDown);
      overlay.remove();
      if (chosen) {
        options.onSelect(chosen);
        resolve(chosen);
      } else {
        options.onCancel?.();
        resolve(null);
      }
    };

    const handleSelectDemo = (demo: DemoLayoutDef) => {
      if (options.currentLayoutDirty) {
        const confirmed = window.confirm(t.unsavedWarning);
        if (!confirmed) return;
      }
      cleanup(demo);
    };

    const renderCards = () => {
      bodyContainer.innerHTML = "";
      const filtered =
        selectedScale === "all"
          ? DEMO_LAYOUTS
          : DEMO_LAYOUTS.filter((d) => d.scale === selectedScale);

      for (const demo of filtered) {
        const card = document.createElement("div");
        card.className = "demo-track-card";
        card.tabIndex = 0;
        card.style.cssText = `
          background: rgba(255, 255, 255, 0.04);
          border: 1px solid rgba(255, 255, 255, 0.1);
          border-radius: 12px;
          padding: 16px;
          display: flex;
          flex-direction: column;
          justify-content: space-between;
          cursor: pointer;
          transition: transform 0.14s ease, border-color 0.14s ease, box-shadow 0.14s ease, background 0.14s ease;
        `;

        const name = isDe ? demo.name : demo.nameEn;
        const desc = isDe ? demo.description : demo.descriptionEn;
        const widthCm = Math.round(demo.dimensionsMm.width / 10);
        const heightCm = Math.round(demo.dimensionsMm.height / 10);

        card.innerHTML = `
          <div>
            <div style="display: flex; align-items: flex-start; justify-content: space-between; gap: 8px; margin-bottom: 8px;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="font-size: 24px; line-height: 1;">${demo.icon}</span>
                <div>
                  <h3 style="margin: 0; font-size: 14px; font-weight: 700; line-height: 1.2;">${name}</h3>
                  <div style="font-size: 11px; color: var(--text-muted, #94a3b8); margin-top: 2px;">
                    ${demo.systemName}
                  </div>
                </div>
              </div>
              <span style="
                background: var(--primary-color, #0284c7);
                color: #ffffff;
                font-size: 10.5px;
                font-weight: 700;
                padding: 2px 7px;
                border-radius: 6px;
                letter-spacing: 0.5px;
              ">${demo.scale}</span>
            </div>

            <p style="margin: 8px 0 12px; font-size: 11.5px; line-height: 1.45; color: rgba(248, 250, 252, 0.85);">
              ${desc}
            </p>

            <div style="display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 12px;">
              <span style="background: rgba(255, 255, 255, 0.07); padding: 2px 6px; border-radius: 4px; font-size: 10.5px;">
                📐 ${widthCm} × ${heightCm} cm
              </span>
              <span style="background: rgba(255, 255, 255, 0.07); padding: 2px 6px; border-radius: 4px; font-size: 10.5px;">
                🛤️ ${demo.pieceCount} ${t.tracksUnit}
              </span>
            </div>
          </div>

          <button type="button" class="btn-primary demo-card-load-btn" style="
            width: 100%;
            background: var(--primary-color, #0284c7);
            color: #ffffff;
            border: none;
            padding: 7px 12px;
            border-radius: 8px;
            font-size: 12px;
            font-weight: 600;
            cursor: pointer;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 6px;
            transition: background 0.12s, transform 0.12s;
          ">
            <span>✓</span>
            <span>${t.loadBtn}</span>
          </button>
        `;

        card.addEventListener("mouseenter", () => {
          card.style.transform = "translateY(-2px)";
          card.style.borderColor = "var(--primary-color, #0284c7)";
          card.style.boxShadow = "0 8px 20px rgba(0, 0, 0, 0.35)";
          card.style.background = "rgba(255, 255, 255, 0.07)";
        });
        card.addEventListener("mouseleave", () => {
          card.style.transform = "none";
          card.style.borderColor = "rgba(255, 255, 255, 0.1)";
          card.style.boxShadow = "none";
          card.style.background = "rgba(255, 255, 255, 0.04)";
        });

        card.addEventListener("click", () => handleSelectDemo(demo));
        card.addEventListener("keydown", (e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleSelectDemo(demo);
          }
        });

        bodyContainer.appendChild(card);
      }
    };

    const renderFilterBar = () => {
      filterBar.innerHTML = "";
      for (const item of scalesList) {
        const pill = document.createElement("button");
        pill.type = "button";
        const isActive = selectedScale === item.id;
        pill.className = `filter-pill ${isActive ? "active" : ""}`;
        pill.style.cssText = `
          background: ${isActive ? "var(--primary-color, #0284c7)" : "transparent"};
          color: ${isActive ? "#ffffff" : "var(--text-muted, #94a3b8)"};
          border: 1px solid ${isActive ? "var(--primary-color, #0284c7)" : "rgba(255, 255, 255, 0.12)"};
          padding: 4px 10px;
          border-radius: 12px;
          font-size: 11px;
          font-weight: 600;
          cursor: pointer;
          white-space: nowrap;
          transition: all 0.12s ease;
        `;
        pill.textContent = item.label;
        pill.addEventListener("click", () => {
          selectedScale = item.id;
          renderFilterBar();
          renderCards();
        });
        filterBar.appendChild(pill);
      }
    };

    renderFilterBar();
    renderCards();

    modal.append(header, filterBar, bodyContainer, footer);
    overlay.appendChild(modal);

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        cleanup(null);
      }
    };
    document.addEventListener("keydown", onKeyDown);

    closeBtn.addEventListener("click", () => cleanup(null));
    closeFooterBtn.addEventListener("click", () => cleanup(null));
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) cleanup(null);
    });

    document.body.appendChild(overlay);
  });
}
