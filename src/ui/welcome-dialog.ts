import { getLanguage } from "@/i18n";
import { getAppVersionTag } from "@/version";
import welcomeHtml from "./templates/welcome-dialog.html?raw";
import recentBtnHtml from "./templates/welcome-recent-button.html?raw";

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

    let recentButtonHtml = "";
    if (hasRecent) {
      recentButtonHtml = recentBtnHtml
        .replace("{{RECENT_TITLE}}", isDe ? "Letzten Plan fortsetzen" : "Continue Last Plan")
        .replace("{{RECENT_DESC}}", recentName || (isDe ? "Zuletzt bearbeitetes Layout" : "Last edited layout"));
    }

    const filledHtml = welcomeHtml
      .replace("{{CLOSE_TITLE}}", isDe ? "Schließen (Esc)" : "Close (Esc)")
      .replace("{{VERSION}}", getAppVersionTag())
      .replace("{{SUBTITLE}}", isDe ? "Wähle, wie du heute starten möchtest:" : "Choose how you want to get started:")
      .replace("{{RECENT_BUTTON}}", recentButtonHtml)
      .replace("{{NEW_CLASS}}", !hasRecent ? "primary" : "")
      .replace("{{NEW_AUTOFOCUS}}", !hasRecent ? "autofocus" : "")
      .replace("{{NEW_TITLE}}", isDe ? "Neuer Plan" : "New Plan")
      .replace("{{NEW_DESC}}", isDe ? "Leere Platte mit Standard-Ebenen" : "Empty baseboard with default layers")
      .replace("{{DEMO_TITLE}}", isDe ? "Demo-Layout" : "Demo Layout")
      .replace("{{DEMO_DESC}}", isDe ? "Musterstrecke zum Testen & Planen (Alpenland H0)" : "Sample layout for testing & planning (Alpenland H0)")
      .replace("{{OPEN_TITLE}}", isDe ? "Plan von Festplatte öffnen…" : "Open Plan from File…")
      .replace("{{OPEN_DESC}}", isDe ? ".railbench Datei auswählen" : "Select .railbench file")
      .replace("{{REMEMBER_LABEL}}", isDe ? "Auswahl merken (nicht mehr fragen)" : "Remember choice (don't ask again)");

    const modal = document.createElement("div");
    modal.className = "welcome-dialog-backdrop";
    modal.innerHTML = filledHtml;

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
