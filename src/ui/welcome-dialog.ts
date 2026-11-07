import { getLanguage } from "@/i18n";
import { getAppVersionTag } from "@/version";

export type StartupAction = "recent" | "new" | "demo" | "open" | "ask";

export interface StartupChoice {
  action: "recent" | "new" | "demo" | "open";
  dontAskAgain: boolean;
}

export function showWelcomeDialog(
  hasRecent: boolean,
  recentName?: string,
): Promise<StartupChoice> {
  return new Promise((resolve) => {
    const isDe = getLanguage() === "de";

    const modal = document.createElement("div");
    modal.className = "modal-backdrop welcome-dialog-backdrop";
    modal.innerHTML = `
      <div class="welcome-card">
        <div class="welcome-header">
          <span class="welcome-logo-badge">🚆</span>
          <h2>Railbench <span style="font-size:12px;opacity:0.6;font-weight:normal;vertical-align:middle;margin-left:4px;">${getAppVersionTag()}</span></h2>
          <p class="welcome-subtitle">
            ${isDe ? "Wähle, wie du heute starten möchtest:" : "Choose how you want to get started:"}
          </p>
        </div>
        
        <div class="welcome-actions">
          ${
            hasRecent
              ? `
            <button type="button" class="welcome-btn primary" id="welcome-btn-recent">
              <span class="welcome-btn-icon">📂</span>
              <div class="welcome-btn-text">
                <strong>${isDe ? "Letzten Plan fortsetzen" : "Continue Last Plan"}</strong>
                <small>${recentName || (isDe ? "Zuletzt bearbeitetes Layout" : "Last edited layout")}</small>
              </div>
            </button>
          `
              : ""
          }
          
          <button type="button" class="welcome-btn" id="welcome-btn-new">
            <span class="welcome-btn-icon">✨</span>
            <div class="welcome-btn-text">
              <strong>${isDe ? "Neuer Plan" : "New Plan"}</strong>
              <small>${isDe ? "Leere Platte mit Standard-Ebenen" : "Empty baseboard with default layers"}</small>
            </div>
          </button>
          
          <button type="button" class="welcome-btn" id="welcome-btn-demo">
            <span class="welcome-btn-icon">🔄</span>
            <div class="welcome-btn-text">
              <strong>${isDe ? "Demo-Layout" : "Demo Layout"}</strong>
              <small>${isDe ? "Musterstrecke zum Testen & Planen" : "Sample oval for testing & planning"}</small>
            </div>
          </button>
          
          <button type="button" class="welcome-btn" id="welcome-btn-open">
            <span class="welcome-btn-icon">📁</span>
            <div class="welcome-btn-text">
              <strong>${isDe ? "Plan von Festplatte öffnen…" : "Open Plan from File…"}</strong>
              <small>${isDe ? ".railbench Datei auswählen" : "Select .railbench file"}</small>
            </div>
          </button>
        </div>

        <div class="welcome-footer">
          <label class="welcome-remember-label">
            <input type="checkbox" id="welcome-remember-check" />
            <span>${isDe ? "Auswahl merken (nicht mehr fragen)" : "Remember choice (don't ask again)"}</span>
          </label>
        </div>
      </div>
    `;

    document.body.appendChild(modal);

    const checkbox = modal.querySelector(
      "#welcome-remember-check",
    ) as HTMLInputElement | null;

    const cleanup = (action: StartupChoice["action"]) => {
      const dontAskAgain = checkbox?.checked ?? false;
      modal.classList.add("fade-out");
      setTimeout(() => {
        modal.remove();
        resolve({ action, dontAskAgain });
      }, 150);
    };

    modal
      .querySelector("#welcome-btn-recent")
      ?.addEventListener("click", () => cleanup("recent"));
    modal
      .querySelector("#welcome-btn-new")
      ?.addEventListener("click", () => cleanup("new"));
    modal
      .querySelector("#welcome-btn-demo")
      ?.addEventListener("click", () => cleanup("demo"));
    modal
      .querySelector("#welcome-btn-open")
      ?.addEventListener("click", () => cleanup("open"));
  });
}