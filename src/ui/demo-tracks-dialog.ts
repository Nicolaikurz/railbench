import { DEMO_LAYOUTS, type DemoLayoutDef } from "@/editor/demo-tracks";
import { getLanguage, getTranslations } from "@/i18n";

export interface DemoTracksDialogOptions {
  currentLayoutDirty?: boolean;
  onSelect: (demo: DemoLayoutDef) => void;
  onCancel?: () => void;
}

const DIALOG_ID = "railbench-demo-tracks-dialog";

export function showDemoTracksDialog(options: DemoTracksDialogOptions): Promise<DemoLayoutDef | null> {
  return new Promise((resolve) => {
    document.getElementById(DIALOG_ID)?.remove();

    const isDe = getLanguage() === "de";
    const t = getTranslations().demoDialog;

    const overlay = document.createElement("div");
    overlay.id = DIALOG_ID;
    overlay.className = "modal-overlay demo-dialog-overlay";

    const modal = document.createElement("div");
    modal.className = "modal-container demo-dialog-modal";
    modal.setAttribute("role", "dialog");
    modal.setAttribute("aria-modal", "true");

    const header = document.createElement("div");
    header.className = "demo-dialog-header";
    header.innerHTML = `
      <h2 class="demo-dialog-title">
        <span>🚆</span>
        <span>${t.title}</span>
      </h2>
      <button type="button" class="demo-dialog-close" aria-label="${t.closeBtn}">×</button>
    `;

    const list = document.createElement("div");
    list.className = "demo-dialog-list";

    let resolved = false;
    const cleanup = (chosen: DemoLayoutDef | null) => {
      if (resolved) return;
      resolved = true;
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

    const handleSelect = (demo: DemoLayoutDef) => {
      if (options.currentLayoutDirty && !window.confirm(t.unsavedWarning)) {
        return;
      }
      cleanup(demo);
    };

    for (const demo of DEMO_LAYOUTS) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "demo-track-item";
      btn.dataset.demoId = demo.id;

      const name = isDe ? demo.name : demo.nameEn;
      const widthCm = Math.round(demo.dimensionsMm.width / 10);
      const heightCm = Math.round(demo.dimensionsMm.height / 10);

      btn.innerHTML = `
        <span class="demo-track-icon">${demo.icon}</span>
        <span class="demo-track-info">
          <span class="demo-track-top">
            <span class="demo-track-name">${name}</span>
            <span class="demo-track-scale">${demo.scale}</span>
          </span>
          <span class="demo-track-meta">${demo.systemName} · ${widthCm} × ${heightCm} cm · ${demo.pieceCount} ${t.tracksUnit}</span>
        </span>
        <span class="demo-track-arrow" aria-hidden="true">→</span>
      `;

      btn.addEventListener("click", () => handleSelect(demo));
      list.appendChild(btn);
    }

    modal.append(header, list);
    overlay.appendChild(modal);

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        cleanup(null);
      }
    };
    document.addEventListener("keydown", onKeyDown);

    header.querySelector(".demo-dialog-close")?.addEventListener("click", () => cleanup(null));
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) cleanup(null);
    });

    document.body.appendChild(overlay);
  });
}
