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

    const recentBtnHtml = hasRecent
      ? `
        <button type="button" class="welcome-btn primary" id="welcome-btn-recent" autofocus>
          <span class="welcome-btn-icon">⏱️</span>
          <span class="welcome-btn-text">
            <strong>${isDe ? "Letzten Plan fortsetzen" : "Continue Last Plan"}</strong>
            <small>${recentName || (isDe ? "Zuletzt bearbeitetes Layout" : "Last edited layout")}</small>
          </span>
          <span class="welcome-btn-arrow" aria-hidden="true">→</span>
        </button>
      `
      : "";

    const modal = document.createElement("div");
    modal.className = "welcome-dialog-backdrop";
    modal.innerHTML = `
      <div class="welcome-card" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
        <div class="welcome-header">
          <h2 id="welcome-title">
            <span>🚆</span>
            <span>Railbench</span>
            <span class="welcome-version">${getAppVersionTag()}</span>
          </h2>
          <button type="button" class="welcome-close-btn" id="welcome-close-btn" title="${isDe ? "Schließen (Esc)" : "Close (Esc)"}">×</button>
        </div>

        <div class="welcome-actions">
          ${recentBtnHtml}
          <button type="button" class="welcome-btn ${!hasRecent ? "primary" : ""}" id="welcome-btn-new" ${!hasRecent ? "autofocus" : ""}>
            <span class="welcome-btn-icon">✨</span>
            <span class="welcome-btn-text">
              <strong>${isDe ? "Neuer Plan" : "New Plan"}</strong>
              <small>${isDe ? "Leere Platte mit Standard-Ebenen" : "Empty baseboard with default layers"}</small>
            </span>
            <span class="welcome-btn-arrow" aria-hidden="true">→</span>
          </button>

          <button type="button" class="welcome-btn" id="welcome-btn-demo">
            <span class="welcome-btn-icon">🏔️</span>
            <span class="welcome-btn-text">
              <strong>${isDe ? "Demo-Layout" : "Demo Layout"}</strong>
              <small>${isDe ? "Musterstrecke zum Testen & Planen" : "Sample layout for testing & planning"}</small>
            </span>
            <span class="welcome-btn-arrow" aria-hidden="true">→</span>
          </button>

          <button type="button" class="welcome-btn" id="welcome-btn-open">
            <span class="welcome-btn-icon">📂</span>
            <span class="welcome-btn-text">
              <strong>${isDe ? "Plan öffnen…" : "Open Plan…"}</strong>
              <small>${isDe ? ".railbench Datei von Festplatte laden" : "Load .railbench file from disk"}</small>
            </span>
            <span class="welcome-btn-arrow" aria-hidden="true">→</span>
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

    let resolved = false;
    const cleanup = (action: StartupChoice["action"]) => {
      if (resolved) return;
      resolved = true;
      document.removeEventListener("keydown", onKeyDown);
      const dontAskAgain = checkbox?.checked ?? false;
      modal.remove();
      resolve({ action, dontAskAgain });
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        cleanup(hasRecent ? "recent" : "demo");
      }
    };
    document.addEventListener("keydown", onKeyDown);

    modal.addEventListener?.("click", (e: MouseEvent) => {
      if (e.target === modal) {
        cleanup(hasRecent ? "recent" : "demo");
      }
    });

    modal
      .querySelector("#welcome-close-btn")
      ?.addEventListener("click", () => cleanup(hasRecent ? "recent" : "demo"));
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
