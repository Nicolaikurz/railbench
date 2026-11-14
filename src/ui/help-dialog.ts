import helpDialogHtml from "./templates/help-dialog.html?raw";
import {
  getTranslations,
  onLanguageChange,
  type HelpDialogTranslations,
  type HelpStandardSection,
  type HelpTab,
} from "@/i18n";
import { getAppVersionTag } from "@/version";
import { downloadFile } from "@/io/export";
import { EXAMPLE_H0_CATALOG_JSON } from "@/catalog/example-catalog";

export type { HelpTab } from "@/i18n";

export const HELP_TABS: readonly HelpTab[] = [
  "overview",
  "layers",
  "print",
  "power",
  "baseboard",
  "view3d",
  "server",
  "shortcuts",
  "catalog",
  "about",
];

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function renderCardGrid(cards: HelpStandardSection["cards"]): string {
  return `
    <div class="help-feature-grid">
      ${cards
        .map(
          (card) => `
        <div class="help-card">
          <h3>${card.title}</h3>
          <p>${card.body}</p>
        </div>
      `,
        )
        .join("")}
    </div>
  `;
}

function renderStandardSection(sec: HelpStandardSection): string {
  return `
    <div class="help-topic-section">
      <h2>${sec.title}</h2>
      <p>${sec.intro}</p>
      ${renderCardGrid(sec.cards)}
      ${sec.callout ? `<div class="help-callout">${sec.callout}</div>` : ""}
    </div>
  `;
}

function renderTable(
  headers: [string, string, string],
  rows: Array<[string, string, string]>,
  widths: [string, string, string],
): string {
  return `
    <table class="help-shortcuts-table">
      <thead>
        <tr>
          <th style="width: ${widths[0]};">${headers[0]}</th>
          <th style="width: ${widths[1]};">${headers[1]}</th>
          <th style="width: ${widths[2]};">${headers[2]}</th>
        </tr>
      </thead>
      <tbody>
        ${rows
          .map(
            ([c0, c1, c2]) => `
          <tr><td>${c0}</td><td>${c1}</td><td>${c2}</td></tr>
        `,
          )
          .join("")}
      </tbody>
    </table>
  `;
}

export function renderHelpTab(
  tab: HelpTab,
  contentArea: HTMLElement,
  t: HelpDialogTranslations = getTranslations().helpDialog,
): void {
  switch (tab) {
    case "overview":
    case "layers":
    case "print":
    case "power":
    case "baseboard":
    case "view3d":
      contentArea.innerHTML = renderStandardSection(t[tab]);
      break;

    case "server": {
      const srv = t.server;
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>${srv.title}</h2>
          <p>${srv.intro}</p>
          ${renderCardGrid(srv.cards)}
          <div class="help-topic-section" style="margin-top: 20px;">
            <h3>${srv.cliTitle}</h3>
            <p>${srv.cliIntro}</p>
            ${renderTable(srv.cliHeaders, srv.cliRows, ["35%", "25%", "40%"])}
            <h4 style="margin-top: 14px; margin-bottom: 6px;">${srv.envTitle}</h4>
            ${renderTable(srv.envHeaders, srv.envRows, ["35%", "25%", "40%"])}
          </div>
          ${srv.callout ? `<div class="help-callout" style="margin-top: 16px;">${srv.callout}</div>` : ""}
        </div>
      `;
      break;
    }

    case "shortcuts": {
      const sc = t.shortcuts;
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>${sc.title}</h2>
          <p>${sc.intro}</p>
          ${renderTable(sc.headers, sc.rows, ["25%", "40%", "35%"])}
        </div>
      `;
      break;
    }

    case "catalog": {
      const cat = t.catalog;
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>${cat.title}</h2>
          <p>${cat.intro}</p>
          ${renderCardGrid(cat.cards)}
          <div style="display: flex; gap: 10px; align-items: center; margin: 16px 0 10px 0;">
            <button class="rb-btn rb-btn-primary" data-action="copy-catalog-json" style="padding: 6px 14px; font-size: 12.5px; cursor: pointer; display: inline-flex; align-items: center; gap: 6px;">
              ${cat.copyBtn}
            </button>
            <button class="rb-btn rb-btn-secondary" data-action="download-catalog-json" style="padding: 6px 14px; font-size: 12.5px; cursor: pointer; display: inline-flex; align-items: center; gap: 6px;">
              ${cat.downloadBtn}
            </button>
            <span style="font-size: 12px; color: var(--text-muted);">${cat.metaInfo}</span>
          </div>
          <div style="position: relative; background: #0f172a; border: 1px solid #334155; border-radius: 8px; overflow: hidden;">
            <pre style="margin: 0; padding: 14px 16px; max-height: 440px; overflow-y: auto; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 11.5px; line-height: 1.5; color: #e2e8f0; scrollbar-width: thin;"><code id="example-catalog-json">${escapeHtml(EXAMPLE_H0_CATALOG_JSON)}</code></pre>
          </div>
        </div>
      `;
      break;
    }

    case "about": {
      const ab = t.about;
      const intro = ab.intro.replace("{version}", getAppVersionTag());
      contentArea.innerHTML = `
        <div class="help-topic-section">
          <h2>${ab.title}</h2>
          <p>${intro}</p>
          <h3>${ab.systemsHeading}</h3>
          <ul class="help-systems-list">
            ${ab.systems.map((item) => `<li>${item}</li>`).join("")}
          </ul>
          <div class="help-callout">${ab.callout}</div>
          <p style="font-size: 11px; color: var(--text-muted); margin-top: 20px;">
            ${ab.licenseNote}
          </p>
        </div>
      `;
      break;
    }
  }
}

export function openHelpDialog(initialTab: HelpTab = "overview"): void {
  document.getElementById("help-modal")?.remove();

  let activeTab: HelpTab = initialTab;

  const modal = document.createElement("div");
  modal.id = "help-modal";
  modal.className = "rb-modal-overlay";

  const renderModalShell = () => {
    const t = getTranslations().helpDialog;
    const navButtonsHtml = HELP_TABS.map(
      (id) => `
      <button class="help-nav-btn ${activeTab === id ? "active" : ""}" data-tab="${id}">
        <span>${t.tabs[id]}</span>
      </button>
    `,
    ).join("");

    modal.innerHTML = helpDialogHtml
      .replace("{{MODAL_TITLE}}", t.modalTitle)
      .replace("{{VERSION_TAG}}", getAppVersionTag())
      .replace("{{CLOSE_TITLE}}", t.closeTitle)
      .replace("{{NAV_BUTTONS}}", navButtonsHtml)
      .replace("{{FOOTER_TIP}}", t.footerTip)
      .replace("{{CLOSE_BTN_TEXT}}", t.closeBtnText);

    const closeBtn = modal.querySelector("#help-modal-close") as HTMLButtonElement | null;
    const bottomCloseBtn = modal.querySelector("#help-btn-close") as HTMLButtonElement | null;
    const contentArea = modal.querySelector("#help-content-area") as HTMLElement | null;
    const navButtons = modal.querySelectorAll<HTMLButtonElement>(".help-nav-btn");

    closeBtn?.addEventListener("click", closeModal);
    bottomCloseBtn?.addEventListener("click", closeModal);

    navButtons.forEach((btn) => {
      btn.addEventListener("click", () => {
        navButtons.forEach((b) => b.classList.remove("active"));
        btn.classList.add("active");
        activeTab = (btn.getAttribute("data-tab") as HelpTab) || "overview";
        if (contentArea) {
          renderHelpTab(activeTab, contentArea, getTranslations().helpDialog);
        }
      });
    });

    if (contentArea) {
      renderHelpTab(activeTab, contentArea, t);
      contentArea.addEventListener("click", (e) => {
        const target = (e.target as HTMLElement).closest<HTMLButtonElement>("button[data-action]");
        if (!target) return;
        const action = target.getAttribute("data-action");

        if (action === "copy-catalog-json") {
          const codeEl = contentArea.querySelector("#example-catalog-json");
          const text = codeEl?.textContent || EXAMPLE_H0_CATALOG_JSON;
          const originalText = target.innerHTML;
          const copiedMsg = getTranslations().helpDialog.copiedText;

          const markSuccess = () => {
            target.innerHTML = copiedMsg;
            target.style.background = "#059669";
            target.style.color = "#ffffff";
            setTimeout(() => {
              target.innerHTML = originalText;
              target.style.background = "";
              target.style.color = "";
            }, 2200);
          };

          const copyFallback = () => {
            const ta = document.createElement("textarea");
            ta.value = text;
            document.body.appendChild(ta);
            ta.select();
            document.execCommand("copy");
            ta.remove();
            markSuccess();
          };

          if (navigator.clipboard?.writeText) {
            navigator.clipboard.writeText(text).then(markSuccess).catch(copyFallback);
          } else {
            copyFallback();
          }
        } else if (action === "download-catalog-json") {
          downloadFile("railbench-catalog-h0-example.json", EXAMPLE_H0_CATALOG_JSON, "application/json");
        }
      });
    }
  };

  const unsubscribeLang = onLanguageChange(() => {
    renderModalShell();
  });

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      closeModal();
    }
  };

  function closeModal() {
    unsubscribeLang();
    window.removeEventListener("keydown", handleKeyDown);
    modal.remove();
  }

  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeModal();
  });
  window.addEventListener("keydown", handleKeyDown);

  renderModalShell();
  document.body.appendChild(modal);
}
