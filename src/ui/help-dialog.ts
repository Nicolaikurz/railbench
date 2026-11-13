import helpDialogHtml from "./templates/help-dialog.html?raw";
import { getLanguage } from "@/i18n";
import { getAppVersionTag } from "@/version";
import { renderGermanTab } from "./help-content-de";
import { renderEnglishTab } from "./help-content-en";
import { downloadFile } from "@/io/export";
import { EXAMPLE_H0_CATALOG_JSON } from "@/catalog/example-catalog";

export type HelpTab =
  | "overview"
  | "layers"
  | "print"
  | "power"
  | "baseboard"
  | "view3d"
  | "server"
  | "shortcuts"
  | "catalog"
  | "about";

export function openHelpDialog(initialTab: HelpTab = "overview"): void {
  const existing = document.getElementById("help-modal");
  if (existing) existing.remove();

  const isDe = getLanguage() === "de";

  const modal = document.createElement("div");
  modal.id = "help-modal";
  modal.className = "rb-modal-overlay";

  const modalTitle = isDe
    ? "Railbench – Benutzerhandbuch & Funktionsübersicht"
    : "Railbench – User & Feature Guide";
  const closeTitle = isDe ? "Hilfe schließen (Esc)" : "Close Help (Esc)";
  const closeBtnText = isDe ? "Handbuch schließen" : "Close Guide";
  const footerTip = isDe
    ? "Tipp: Drücken Sie jederzeit <strong>F1</strong>, um diese Hilfe zu öffnen, oder <strong>M</strong> bei ausgewählten Gleisen für Höhen- und Rampenanpassung."
    : "Tip: Press <strong>F1</strong> anywhere to open this guide, or <strong>M</strong> when tracks are selected to change layer elevation.";

  const tabs: Array<{ id: HelpTab; label: string }> = isDe
    ? [
        { id: "overview", label: "Erste Schritte" },
        { id: "layers", label: "Ebenen & Höhen" },
        { id: "print", label: "Ebenen-Druck" },
        { id: "power", label: "Spannungs-Simulation" },
        { id: "baseboard", label: "Tisch & Platte" },
        { id: "view3d", label: "3D-Ansicht" },
        { id: "server", label: "Multi-User & Server" },
        { id: "shortcuts", label: "Tastaturkürzel" },
        { id: "catalog", label: "Gleiskatalog (JSON)" },
        { id: "about", label: "Über Railbench & Gleise" },
      ]
    : [
        { id: "overview", label: "Getting Started" },
        { id: "layers", label: "Layers & Heights" },
        { id: "print", label: "Layer Printing" },
        { id: "power", label: "Voltage Simulation" },
        { id: "baseboard", label: "Table & Baseboard" },
        { id: "view3d", label: "3D View" },
        { id: "server", label: "Multi-User & Server" },
        { id: "shortcuts", label: "Keyboard Shortcuts" },
        { id: "catalog", label: "Track Catalog (JSON)" },
        { id: "about", label: "About & Systems" },
      ];

  const navButtonsHtml = tabs
    .map(
      (tb) => `
    <button class="help-nav-btn ${initialTab === tb.id ? "active" : ""}" data-tab="${tb.id}">
      <span>${tb.label}</span>
    </button>
  `,
    )
    .join("");

  modal.innerHTML = helpDialogHtml
    .replace("{{MODAL_TITLE}}", modalTitle)
    .replace("{{VERSION_TAG}}", getAppVersionTag())
    .replace("{{CLOSE_TITLE}}", closeTitle)
    .replace("{{NAV_BUTTONS}}", navButtonsHtml)
    .replace("{{FOOTER_TIP}}", footerTip)
    .replace("{{CLOSE_BTN_TEXT}}", closeBtnText);

  document.body.appendChild(modal);

  const closeBtn = modal.querySelector("#help-modal-close") as HTMLButtonElement;
  const bottomCloseBtn = modal.querySelector("#help-btn-close") as HTMLButtonElement;
  const contentArea = modal.querySelector("#help-content-area") as HTMLElement;
  const navButtons = modal.querySelectorAll<HTMLButtonElement>(".help-nav-btn");

  const closeModal = () => modal.remove();
  closeBtn.addEventListener("click", closeModal);
  bottomCloseBtn.addEventListener("click", closeModal);

  // ESC schließt Dialog
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closeModal();
      window.removeEventListener("keydown", handleKeyDown);
    }
  };
  window.addEventListener("keydown", handleKeyDown);

  const renderTabContent = (tab: HelpTab) => {
    if (isDe) {
      renderGermanTab(tab, contentArea);
    } else {
      renderEnglishTab(tab, contentArea);
    }
  };

  // Navigations-Tabs schalten
  navButtons.forEach((btn) => {
    btn.addEventListener("click", () => {
      navButtons.forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      const tab = btn.getAttribute("data-tab") as HelpTab;
      renderTabContent(tab);
    });
  });

  // Initialen Tab rendern
  renderTabContent(initialTab);

  // Aktionen im Inhaltsbereich (z. B. JSON kopieren & herunterladen)
  contentArea.addEventListener("click", (e) => {
    const target = (e.target as HTMLElement).closest<HTMLButtonElement>("button[data-action]");
    if (!target) return;
    const action = target.getAttribute("data-action");

    if (action === "copy-catalog-json") {
      const codeEl = contentArea.querySelector("#example-catalog-json");
      const text = codeEl?.textContent || EXAMPLE_H0_CATALOG_JSON;
      const originalText = target.innerHTML;

      const markSuccess = () => {
        target.innerHTML = isDe ? "✓ In Zwischenablage kopiert!" : "✓ Copied to clipboard!";
        target.style.background = "#059669";
        target.style.color = "#ffffff";
        setTimeout(() => {
          target.innerHTML = originalText;
          target.style.background = "";
          target.style.color = "";
        }, 2200);
      };

      if (navigator.clipboard?.writeText) {
        navigator.clipboard.writeText(text).then(markSuccess).catch(() => {
          // Fallback
          const ta = document.createElement("textarea");
          ta.value = text;
          document.body.appendChild(ta);
          ta.select();
          document.execCommand("copy");
          ta.remove();
          markSuccess();
        });
      } else {
        const ta = document.createElement("textarea");
        ta.value = text;
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        ta.remove();
        markSuccess();
      }
    } else if (action === "download-catalog-json") {
      downloadFile("railbench-catalog-h0-example.json", EXAMPLE_H0_CATALOG_JSON, "application/json");
    }
  });
}

