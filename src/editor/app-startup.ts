import type { Layout } from "@/core/types";
import { ensureDefaultLayers, createLayout } from "@/core/layout";
import { catalogRegistry, type Catalog } from "@/catalog/catalog";
import { History } from "@/editor/commands";
import { Selection } from "@/editor/selection";
import { PlanManager } from "@/io/plan-manager";
import { getLanguage } from "@/i18n";
import { showToast } from "@/ui/toast";
import { showDemoTracksDialog } from "@/ui/demo-tracks-dialog";
import { showWelcomeDialog } from "@/ui/welcome-dialog";
import { initDemoTracks, type DemoLayoutDef } from "@/editor/demo-tracks";
import { saveAppSettings } from "@/settings/settings-manager";

export interface AppStartupContext {
  getLayout: () => Layout;
  setLayout: (l: Layout) => void;
  getCatalog: () => Catalog;
  setCatalog: (c: Catalog) => void;
  getSelection: () => Selection;
  getHistory: () => History;
  setHistory: (h: History) => void;
  planManager: PlanManager;
  saveSettingsState: () => void;
  sync: (dirty?: boolean) => void;
  onCatalogChange?: (c: Catalog) => void;
  triggerOpenPlan?: () => void;
}

export function loadDemoLayout(ctx: AppStartupContext, demo: DemoLayoutDef) {
  const chosen = demo.createLayout();
  ensureDefaultLayers(chosen);
  ctx.setLayout(chosen);

  const matchedCat = catalogRegistry.get(demo.catalogId);
  if (matchedCat) {
    ctx.setCatalog(matchedCat);
    ctx.onCatalogChange?.(matchedCat);
  }

  ctx.planManager.setPlanInfo(`${chosen.name}.railbench`, null, "railbench");
  ctx.planManager.markDirty(false);
  ctx.setHistory(new History(chosen));
  ctx.getSelection().clear();
  ctx.sync(false);
  ctx.saveSettingsState();
  showToast(
    getLanguage() === "de"
      ? `Demo-Layout "${demo.name}" geladen!`
      : `Demo layout "${demo.nameEn}" loaded!`,
    ""
  );
}

export function triggerDemoTracksDialog(ctx: AppStartupContext) {
  showDemoTracksDialog({
    currentLayoutDirty: ctx.planManager.getState().isDirty,
    onSelect: (demo) => {
      loadDemoLayout(ctx, demo);
    },
  });
}

export function applyStartupAction(
  ctx: AppStartupContext,
  action: "recent" | "new" | "demo" | "open",
  cached?: Layout | null,
) {
  let layout = ctx.getLayout();
  const catalog = ctx.getCatalog();

  if (action === "recent" && cached && cached.pieces?.length) {
    layout = cached;
    ctx.setLayout(layout);
    ensureDefaultLayers(layout);
    ctx.planManager.setPlanInfo(layout.name + ".railbench", null, "railbench");
  } else if (action === "new") {
    const newName = getLanguage() === "de" ? "Neuer Plan" : "New Layout";
    const fresh = createLayout(newName);
    fresh.scale = layout.scale || "H0";
    ctx.setLayout(fresh);
    ctx.planManager.setPlanInfo(getLanguage() === "de" ? "Neuer Plan.railbench" : "New Layout.railbench", null, "railbench");
    ctx.planManager.markDirty(false);
    layout = fresh;
  } else if (action === "demo") {
    initDemoTracks(layout, catalog);
    ctx.planManager.setPlanInfo(`${layout.name}.railbench`, null, "railbench");
    triggerDemoTracksDialog(ctx);
  } else {
    initDemoTracks(layout, catalog);
    ctx.planManager.setPlanInfo(`${layout.name}.railbench`, null, "railbench");
    if (action === "open") setTimeout(() => ctx.triggerOpenPlan?.(), 50);
  }
  ctx.setHistory(new History(layout));
  ctx.getSelection().clear();
  ctx.sync(false);
}

export async function triggerWelcomeDialog(ctx: AppStartupContext) {
  const cached = ctx.planManager.loadFromLocalStorage();
  const hasRec = Boolean(cached && cached.pieces?.length);
  const recName = cached?.name ? `${cached.name}.railbench` : undefined;
  const choice = await showWelcomeDialog(hasRec, recName);
  if (choice.dontAskAgain) {
    saveAppSettings({ startupAction: choice.action });
  } else {
    saveAppSettings({ startupAction: "ask" });
  }
  applyStartupAction(ctx, choice.action, cached);
}
