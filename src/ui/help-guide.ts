import type { HelpDialogTranslations, HelpGuideImage } from "@/i18n";
import heightRampPanelUrl from "@/assets/help/height-ramp-panel.png?url";
import layoutUrl from "@/assets/help/guide-layout.png?url";
import toolbarUrl from "@/assets/help/guide-toolbar.png?url";
import selectionUrl from "@/assets/help/guide-selection.png?url";
import contextMenuUrl from "@/assets/help/guide-context-menu.png?url";

type GuideTranslations = HelpDialogTranslations["guide"];

function shot(url: string, alt: string, maxWidth = "100%"): string {
  return `<img src="${url}" alt="${alt}" style="display:block;max-width:${maxWidth};width:100%;margin:0 auto;border-radius:8px;border:1px solid var(--border-main, #334155);" />`;
}

// Real screenshots of the running application (English UI).
const IMAGES: Record<HelpGuideImage, () => string> = {
  layout: () => shot(layoutUrl, "Railbench main window"),
  toolbar: () => shot(toolbarUrl, "Railbench toolbar"),
  selection: () => shot(selectionUrl, "Selection bar", "830px"),
  contextMenu: () => shot(contextMenuUrl, "Track right-click context menu", "290px"),
  heightRamp: () => shot(heightRampPanelUrl, "Height & Ramp panel", "340px"),
};

/** Renders the illustrated step-by-step guide shown in the help window. */
export function renderHelpGuide(guide: GuideTranslations): string {
  return `
    <div class="help-topic-section">
      <h2>${guide.title}</h2>
      <p>${guide.intro}</p>
      ${guide.steps
        .map(
          (step) => `
        <div class="help-card" style="margin: 18px 0;">
          <h3>${step.title}</h3>
          <figure style="margin: 10px 0 12px 0;">
            ${IMAGES[step.image]()}
            <figcaption style="font-size: 11.5px; color: var(--text-muted); text-align: center; margin-top: 6px;">${step.caption}</figcaption>
          </figure>
          <p>${step.body}</p>
        </div>`,
        )
        .join("")}
    </div>`;
}
