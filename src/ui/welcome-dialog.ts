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
    modal.className = "welcome-dialog-backdrop";
    modal.innerHTML = `
      <style>
        .welcome-dialog-backdrop {
          position: fixed;
          inset: 0;
          background: rgba(10, 15, 29, 0.78);
          backdrop-filter: blur(8px);
          -webkit-backdrop-filter: blur(8px);
          z-index: 10000;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 20px;
          opacity: 1;
          transition: opacity 0.18s ease;
        }
        .welcome-dialog-backdrop.fade-out {
          opacity: 0;
          pointer-events: none;
        }
        .welcome-card {
          position: relative;
          background: var(--bg-card, #1e293b);
          color: var(--text-main, #f8fafc);
          border: 1px solid var(--border-main, #334155);
          border-radius: 16px;
          box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.55), 0 0 0 1px rgba(255, 255, 255, 0.06);
          width: 100%;
          max-width: 520px;
          padding: 28px;
          box-sizing: border-box;
          animation: welcomeCardPop 0.22s cubic-bezier(0.16, 1, 0.3, 1);
        }
        @keyframes welcomeCardPop {
          from { opacity: 0; transform: scale(0.95) translateY(8px); }
          to { opacity: 1; transform: scale(1) translateY(0); }
        }
        .welcome-close-btn {
          position: absolute;
          top: 14px;
          right: 14px;
          width: 32px;
          height: 32px;
          border-radius: 8px;
          border: 1px solid transparent;
          background: transparent;
          color: var(--text-muted, #94a3b8);
          font-size: 20px;
          line-height: 1;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: pointer;
          transition: all 0.15s ease;
        }
        .welcome-close-btn:hover {
          background: rgba(255, 255, 255, 0.08);
          color: var(--text-main, #f8fafc);
        }
        .welcome-header {
          text-align: center;
          margin-bottom: 22px;
        }
        .welcome-logo-badge {
          font-size: 38px;
          display: inline-block;
          margin-bottom: 8px;
          filter: drop-shadow(0 4px 6px rgba(0, 0, 0, 0.3));
        }
        .welcome-header h2 {
          margin: 0 0 6px 0;
          font-size: 22px;
          font-weight: 700;
          color: var(--text-main, #f8fafc);
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 6px;
        }
        .welcome-subtitle {
          margin: 0;
          font-size: 13.5px;
          color: var(--text-muted, #94a3b8);
        }
        .welcome-actions {
          display: flex;
          flex-direction: column;
          gap: 10px;
          margin-bottom: 20px;
        }
        .welcome-btn {
          display: flex;
          align-items: center;
          gap: 14px;
          padding: 12px 16px;
          background: var(--bg-surface, #0f172a);
          border: 1px solid var(--border-main, #334155);
          border-radius: 10px;
          color: var(--text-main, #f8fafc);
          cursor: pointer;
          text-align: left;
          transition: all 0.15s ease;
          font-family: inherit;
          width: 100%;
          box-sizing: border-box;
        }
        .welcome-btn:hover {
          background: var(--bg-hover, #24324a);
          border-color: #0284c7;
          transform: translateY(-1px);
          box-shadow: 0 4px 12px rgba(2, 132, 199, 0.15);
        }
        .welcome-btn.primary {
          border-color: #0284c7;
          background: linear-gradient(135deg, rgba(2, 132, 199, 0.18) 0%, rgba(15, 23, 42, 0.85) 100%);
        }
        .welcome-btn.primary:hover {
          background: linear-gradient(135deg, rgba(2, 132, 199, 0.28) 0%, rgba(20, 30, 55, 0.95) 100%);
          border-color: #38bdf8;
        }
        .welcome-btn-icon {
          font-size: 22px;
          width: 36px;
          height: 36px;
          display: flex;
          align-items: center;
          justify-content: center;
          background: rgba(255, 255, 255, 0.05);
          border-radius: 8px;
          flex-shrink: 0;
        }
        .welcome-btn-text {
          display: flex;
          flex-direction: column;
          gap: 2px;
          overflow: hidden;
        }
        .welcome-btn-text strong {
          font-size: 13.5px;
          font-weight: 600;
        }
        .welcome-btn-text small {
          font-size: 12px;
          color: var(--text-muted, #94a3b8);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        .welcome-footer {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding-top: 14px;
          border-top: 1px solid var(--border-main, rgba(255, 255, 255, 0.08));
          font-size: 12px;
          color: var(--text-muted, #94a3b8);
        }
        .welcome-remember-label {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          cursor: pointer;
          user-select: none;
        }
        .welcome-remember-label input[type="checkbox"] {
          accent-color: #0284c7;
          cursor: pointer;
          width: 14px;
          height: 14px;
        }
      </style>

      <div class="welcome-card" role="dialog" aria-modal="true" aria-labelledby="welcome-title">
        <button type="button" class="welcome-close-btn" id="welcome-close-btn" title="${isDe ? "Schließen (Esc)" : "Close (Esc)"}" aria-label="Close">×</button>

        <div class="welcome-header">
          <span class="welcome-logo-badge">🚆</span>
          <h2 id="welcome-title">Railbench <span style="font-size:12px;opacity:0.6;font-weight:normal;vertical-align:middle;margin-left:4px;">${getAppVersionTag()}</span></h2>
          <p class="welcome-subtitle">
            ${isDe ? "Wähle, wie du heute starten möchtest:" : "Choose how you want to get started:"}
          </p>
        </div>
        
        <div class="welcome-actions">
          ${
            hasRecent
              ? `
            <button type="button" class="welcome-btn primary" id="welcome-btn-recent" autofocus>
              <span class="welcome-btn-icon">📂</span>
              <div class="welcome-btn-text">
                <strong>${isDe ? "Letzten Plan fortsetzen" : "Continue Last Plan"}</strong>
                <small>${recentName || (isDe ? "Zuletzt bearbeitetes Layout" : "Last edited layout")}</small>
              </div>
            </button>
          `
              : ""
          }
          
          <button type="button" class="welcome-btn ${!hasRecent ? "primary" : ""}" id="welcome-btn-new" ${!hasRecent ? "autofocus" : ""}>
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
              <small>${isDe ? "Musterstrecke zum Testen & Planen (Alpenland H0)" : "Sample layout for testing & planning (Alpenland H0)"}</small>
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

    let resolved = false;
    const cleanup = (action: StartupChoice["action"]) => {
      if (resolved) return;
      resolved = true;
      document.removeEventListener("keydown", onKeyDown);
      const dontAskAgain = checkbox?.checked ?? false;
      modal.classList.add("fade-out");
      setTimeout(() => {
        modal.remove();
        resolve({ action, dontAskAgain });
      }, 150);
    };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        cleanup(hasRecent ? "recent" : "demo");
      }
    };
    document.addEventListener("keydown", onKeyDown);

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
