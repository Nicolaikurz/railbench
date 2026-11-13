import { ICONS } from "./icons";
import type { ColorMode } from "@/core/types";
import { getTranslations, getLanguage, toggleLanguage } from "@/i18n";
import { getAppVersionTag, getFullAppTitle } from "@/version";

export interface ToolbarEvents {
  onToggleRightSidebar?: () => void;
  onNewPlan: () => void;
  onOpenPlan: () => void;
  onOpenDemoTracks?: () => void;
  onSavePlan: () => void;
  onSavePlanAs: () => void;
  onSaveXML: () => void;
  onExportCSV: () => void;
  onPrintPlan?: () => void;
  onMoveSelectedToLayerPrompt?: () => void;
  onOpenHelp?: (tab?: string) => void;
  onOpenWelcome?: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  onDuplicate?: () => void;
  onSelectAll?: () => void;
  onDeselectAll?: () => void;
  onRotateSelected: (step?: number) => void;
  onDeleteSelected: () => void;
  onToggleFeeder: () => void;
  onToggleIsolator?: () => void;
  onToggleCatenary?: () => void;
  onToggleCatenaryFeeder?: () => void;
  onToggleCatenaryIsolator?: () => void;
  onToggleTurnoutLantern?: () => void;
  onToggleTurnoutDecoder?: () => void;
  onResetView: () => void;
  onToggleRulers?: () => void;
  onToggleBaseboard?: () => void;
  onConfigureBaseboard?: () => void;
  onToggle3D?: () => void;
  onImportModel?: () => void;
  onImportCatalog?: () => void;
  onToggleTheme: () => void;
  onSetTheme?: (theme: "system" | "light" | "dark") => void;
  onToggleMeasureTool?: () => void;
  onToggleWoodPlate?: () => void;
  onToggleLockBars?: () => void;
  onResetUiSizes?: () => void;
  onToggleSnapping: () => void;
  onToggleSelectionLock?: () => void;
  onToggleColorCode: () => void;
  onSetColorMode?: (mode: ColorMode) => void;
  onToggleTrackLabels: () => void;
  onTogglePowerSim: () => void;
  onSetFeedVoltage?: (v: number) => void;
  onSetLocoCurrent?: (ma: number) => void;
  onOpenPowerSettings?: (initialFocus?: "voltage" | "current") => void;
  onFocusShortCircuit?: () => void;
  onClearLayout?: () => void;
  onOpenCollab?: () => void;
  onTogglePerfMonitor?: () => void;
  onAutoSelectLanguage?: () => void;
  onResetPersonalConfig?: () => void;
}

export interface ToolbarState {
  canUndo?: boolean;
  canRedo?: boolean;
  hasSelection?: boolean;
  snappingEnabled?: boolean;
  selectionLockDisabled?: boolean;
  measureToolActive?: boolean;
  colorCodeEnabled?: boolean;
  colorMode?: ColorMode;
  trackLabelsEnabled?: boolean;
  powerSimEnabled?: boolean;
  hasShortCircuit?: boolean;
  shortCircuitCount?: number;
  rulersEnabled?: boolean;
  baseboardEnabled?: boolean;
  is3DActive?: boolean;
  selectedHasFeeder?: boolean;
  selectedHasIsolator?: boolean;
  totalPieces?: number;
  activeFileName?: string;
  isDirty?: boolean;
  themePref?: "system" | "light" | "dark";
  resolvedTheme?: "light" | "dark";
  rightSidebarCollapsed?: boolean;
  barsLocked?: boolean;
  collabStatus?: "disconnected" | "connecting" | "connected";
  collabUsersCount?: number;
}

interface DropdownItemConfig {
  label?: string;
  shortcut?: string;
  icon?: string;
  onClick?: () => void;
  id?: string;
  isDivider?: boolean;
  isDanger?: boolean;
  sectionHeader?: string;
}



/**
 * Attaches hover behavior with a timeout buffer and coordination across dropdowns
 * so the menu does not vanish when moving the mouse between the trigger and the menu.
 */
function attachDropdownHoverBridge(
  wrapper: HTMLElement,
  onOpen?: () => void,
  onClose?: () => void,
  closeDelayMs = 220
) {
  let timer: ReturnType<typeof setTimeout> | null = null;

  wrapper.addEventListener("mouseenter", () => {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    document.querySelectorAll(".toolbar-dropdown.open").forEach((d) => {
      if (d !== wrapper) d.classList.remove("open");
    });
    wrapper.classList.add("open");
    onOpen?.();
  });

  wrapper.addEventListener("mouseleave", () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      wrapper.classList.remove("open");
      onClose?.();
      timer = null;
    }, closeDelayMs);
  });
}

export function isMacOS(): boolean {
  if (typeof window !== "undefined") {
    if ((window as any).__FORCE_MAC_MENU__ !== undefined) return Boolean((window as any).__FORCE_MAC_MENU__);
    if (window.electronAPI?.platform === "darwin") return true;
    if (typeof navigator !== "undefined") {
      const p = (navigator.platform || "").toUpperCase();
      const ua = (navigator.userAgent || "").toUpperCase();
      return p.includes("MAC") || ua.includes("MACINTOSH");
    }
  }
  return false;
}

export function createToolbar(host: HTMLElement, events: ToolbarEvents) {
  host.innerHTML = "";
  const t = getTranslations().toolbar;
  const isMac = isMacOS();

  if (typeof document !== "undefined") {
    if (isMac) {
      document.body?.classList?.add?.("has-system-menus");
      const appEl = document.getElementById("app");
      appEl?.classList?.add?.("has-system-menus");
    } else {
      document.body?.classList?.remove?.("has-system-menus");
      const appEl = document.getElementById("app");
      appEl?.classList?.remove?.("has-system-menus");
    }
  }

  const container = document.createElement("div");
  container.className = "toolbar-container";

  // --- LINE 1: Menubar Row (Layout, Edit, View, Settings, Help) ---
  const menubarRow = document.createElement("div");
  menubarRow.className = "toolbar-row toolbar-menubar-row";
  if (isMac) {
    menubarRow.style.display = "none";
  }

  const menubarLeft = document.createElement("div");
  menubarLeft.className = "toolbar-group toolbar-menubar-left";

  const brandBadge = createAppBrandBadge(events);
  const layoutMenu = createLayoutMenu(t, events);
  const editMenu = createEditMenu(t, events);
  const viewMenu = createViewMenu(t, events);
  const settingsMenu = createSettingsMenu(t, events);
  const helpMenu = createHelpMenu(t, events);

  menubarLeft.appendChild(brandBadge);
  menubarLeft.appendChild(layoutMenu);
  menubarLeft.appendChild(editMenu);
  menubarLeft.appendChild(viewMenu);
  menubarLeft.appendChild(settingsMenu);
  menubarLeft.appendChild(helpMenu);
  menubarRow.appendChild(menubarLeft);

  const menubarRight = document.createElement("div");
  menubarRight.className = "toolbar-group toolbar-menubar-right";

  const fileLabel = document.createElement("span");
  fileLabel.id = "tb-file-name";
  fileLabel.className = "brand-file";
  fileLabel.title = "Current layout plan";
  fileLabel.textContent = "Model Railway.railbench";
  fileLabel.addEventListener("click", () => {
    if (fileLabel.classList.contains("dirty")) {
      events.onSavePlan();
    }
  });

  const countBadge = document.createElement("span");
  countBadge.id = "tb-piece-count";
  countBadge.className = "brand-badge";
  countBadge.textContent = `0 ${t.status.tracksUnitPlural}`;

  if (!isMac) {
    menubarRight.appendChild(fileLabel);
    menubarRight.appendChild(countBadge);
    menubarRow.appendChild(menubarRight);
  }

  container.appendChild(menubarRow);

  // --- LINE 2: Actions Row (Quick buttons & Tools) ---
  const actionsRow = document.createElement("div");
  actionsRow.className = "toolbar-row toolbar-actions-row";

  // Register tabs container (mounted on the left of Line 2, at the exact same height and width as the sidebar)
  const sidebarTabsContainer = document.createElement("div");
  sidebarTabsContainer.id = "toolbar-sidebar-tabs";
  sidebarTabsContainer.className = "toolbar-sidebar-tabs";
  actionsRow.appendChild(sidebarTabsContainer);
  actionsRow.appendChild(createDivider());

  // Quick file actions
  const fileActionsGroup = document.createElement("div");
  fileActionsGroup.className = "toolbar-group toolbar-file-quick";

  const btnNew = createIconButton(ICONS.filePlus, `${t.fileMenu.newPlan} (Ctrl+N)`, events.onNewPlan, "tb-new");
  const btnOpen = createIconButton(ICONS.folderOpen, `${t.fileMenu.openPlan} (Ctrl+O)`, events.onOpenPlan, "tb-open");
  const btnSave = createIconButton(ICONS.save, `${t.fileMenu.savePlan} (Ctrl+S)`, events.onSavePlan, "tb-save");

  fileActionsGroup.appendChild(btnNew);
  fileActionsGroup.appendChild(btnOpen);
  fileActionsGroup.appendChild(btnSave);

  actionsRow.appendChild(fileActionsGroup);
  actionsRow.appendChild(createDivider());

  const featureGroup = document.createElement("div");
  featureGroup.className = "toolbar-group toolbar-features";

  const btnSnap = createToggleBtn(ICONS.magnet, t.toggles.snapLabel, "Snap", true, events.onToggleSnapping, "tb-snap");
  btnSnap.title = t.toggles.snapTitle;

  const btnSelectionLock = createToggleBtn(
    ICONS.lock,
    t.toggles.selectionLockLabel,
    "Lock",
    false,
    () => events.onToggleSelectionLock?.(),
    "tb-selection-lock",
  );
  btnSelectionLock.title = t.toggles.selectionLockTitle;

  const btnMeasure = createToggleBtn(ICONS.ruler, t.toggles.measureLabel, "Dist", false, () => events.onToggleMeasureTool?.(), "tb-measure");
  btnMeasure.title = t.toggles.measureTitle;

  const colorsWrapper = document.createElement("div");
  colorsWrapper.className = "toolbar-split-group active";

  const btnColors = createToggleBtn(ICONS.palette, t.toggles.colorsLabel, "Col", true, events.onToggleColorCode, "tb-colors");
  btnColors.title = t.toggles.colorsTitle;
  colorsWrapper.appendChild(btnColors);

  const colorsMenu = createDropdownMenu("", [
    { label: t.colorsMenu.types, onClick: () => events.onSetColorMode?.("types"), id: "split-color-types" },
    { label: t.colorsMenu.layers, onClick: () => events.onSetColorMode?.("layers"), id: "split-color-layers" },
    { label: t.colorsMenu.off, onClick: () => events.onSetColorMode?.("off"), id: "split-color-off" },
  ]);
  colorsMenu.classList.add("colors-split-dropdown");
  colorsWrapper.appendChild(colorsMenu);

  const btnLabels = createToggleBtn(ICONS.tag, t.toggles.labelsLabel, "Labels", true, events.onToggleTrackLabels, "tb-labels");
  btnLabels.title = t.toggles.labelsTitle;

  const powerWrapper = document.createElement("div");
  powerWrapper.className = "toolbar-split-group";

  const btnPower = createToggleBtn(ICONS.zap, t.toggles.voltageLabel, "Volt", false, events.onTogglePowerSim, "tb-power");
  btnPower.title = t.toggles.voltageTitle;
  powerWrapper.appendChild(btnPower);

  const powerMenu = createDropdownMenu("", [
    { label: t.powerMenu.h0Standard, onClick: () => events.onSetFeedVoltage?.(16) },
    { label: t.powerMenu.nScale, onClick: () => events.onSetFeedVoltage?.(12) },
    { label: t.powerMenu.gauge1, onClick: () => events.onSetFeedVoltage?.(18) },
    { isDivider: true },
    { label: t.powerMenu.h0Current, onClick: () => events.onSetLocoCurrent?.(450) },
    { label: t.powerMenu.nzCurrent, onClick: () => events.onSetLocoCurrent?.(100) },
    { isDivider: true },
    { label: t.powerMenu.configPower, onClick: () => events.onOpenPowerSettings?.("voltage") },
    { isDivider: true },
    { label: t.powerMenu.toggleFeeder, shortcut: "F", onClick: events.onToggleFeeder },
    { label: t.powerMenu.toggleIsolator, shortcut: "I", onClick: events.onToggleIsolator },
    { isDivider: true },
    { label: t.powerMenu.toggleCatenary, shortcut: "O", onClick: events.onToggleCatenary },
    { label: t.powerMenu.toggleCatenaryFeeder, shortcut: "Shift+F", onClick: events.onToggleCatenaryFeeder },
    { label: t.powerMenu.toggleCatenaryIsolator, shortcut: "Shift+I", onClick: events.onToggleCatenaryIsolator },
    { isDivider: true },
    { label: t.powerMenu.toggleTurnoutLantern, shortcut: "Shift+L", onClick: events.onToggleTurnoutLantern },
    { label: t.powerMenu.toggleTurnoutDecoder, shortcut: "Shift+D", onClick: events.onToggleTurnoutDecoder },
  ]);
  powerMenu.classList.add("power-split-dropdown");
  powerWrapper.appendChild(powerMenu);

  const btnShortCircuit = document.createElement("button");
  btnShortCircuit.id = "tb-short-circuit";
  btnShortCircuit.className = "toolbar-btn tb-short-circuit-btn";
  btnShortCircuit.style.display = "none";
  btnShortCircuit.title = t.status.shortCircuitTooltip;
  btnShortCircuit.innerHTML = `
    <span class="tb-short-circuit-icon">${ICONS.alertTriangle}</span>
    <span class="tb-short-circuit-label">${t.status.shortCircuitButton}</span>
  `;
  btnShortCircuit.addEventListener("click", () => {
    events.onFocusShortCircuit?.();
  });

  const btn3D = createToggleBtn(ICONS.box3d, "3D", "3D", false, () => events.onToggle3D?.(), "tb-3d");
  btn3D.title = t.toggles.toggle3dTitle;

  featureGroup.appendChild(btnSnap);
  featureGroup.appendChild(btnSelectionLock);
  featureGroup.appendChild(btnMeasure);
  featureGroup.appendChild(colorsWrapper);
  featureGroup.appendChild(btnLabels);
  featureGroup.appendChild(powerWrapper);
  featureGroup.appendChild(btnShortCircuit);
  featureGroup.appendChild(btn3D);
  actionsRow.appendChild(featureGroup);

  const rightGroup = document.createElement("div");
  rightGroup.className = "toolbar-group toolbar-right";

  if (isMac) {
    rightGroup.appendChild(fileLabel);
    rightGroup.appendChild(countBadge);
  }

  const currentLang = getLanguage();
  const btnLang = document.createElement("button");
  btnLang.id = "tb-lang-toggle";
  btnLang.className = "toolbar-btn tb-lang-btn";
  btnLang.title = t.status.langSwitchTitle;
  btnLang.innerHTML = `<span style="font-weight: 700; font-size: 11px;">${currentLang.toUpperCase()}</span>`;
  btnLang.addEventListener("click", () => {
    toggleLanguage();
  });

  const btnCollab = createCollabButton(t, events);
  const btnTheme = createThemeButton(t, events);
  const btnRightSidebar = createIconButton(ICONS.layers, t.status.toggleRightSidebar, () => events.onToggleRightSidebar?.(), "tb-right-sidebar-toggle", "toolbar-toggle active");

  rightGroup.appendChild(btnCollab);
  rightGroup.appendChild(btnTheme);
  rightGroup.appendChild(btnLang);
  rightGroup.appendChild(btnRightSidebar);
  actionsRow.appendChild(rightGroup);

  container.appendChild(actionsRow);

  host.appendChild(container);
  setupDropdownGlobalDismiss();
}

function createAppBrandBadge(events: ToolbarEvents): HTMLElement {
  const brand = document.createElement("div");
  brand.id = "tb-brand-trigger";
  brand.className = "toolbar-brand-badge";
  brand.style.display = "inline-flex";
  brand.style.alignItems = "center";
  brand.style.gap = "5px";
  brand.style.padding = "0 6px";
  brand.style.cursor = "pointer";
  brand.title = getFullAppTitle();
  brand.innerHTML = `
    <span class="brand-icon" style="display:inline-flex; align-items:center; color:#38bdf8;">${ICONS.train}</span>
    <span class="brand-title" style="color:#ffffff; font-weight:700; font-size:12px; letter-spacing:-0.02em;">RAILbench</span>
    <span class="brand-version" style="color:rgba(255,255,255,0.5); font-size:10px; font-variant-numeric: tabular-nums;">${getAppVersionTag()}</span>
  `;
  brand.addEventListener("click", () => {
    events.onOpenWelcome?.();
  });
  return brand;
}

function createTextDropdownMenu(label: string, items: DropdownItemConfig[], id?: string): HTMLElement {
  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "toolbar-btn toolbar-menu-btn dropdown-trigger";
  trigger.title = label;
  if (id) trigger.id = id;
  trigger.innerHTML = `<span class="tb-menu-label">${label}</span>`;

  const wrapper = document.createElement("div");
  wrapper.className = "toolbar-dropdown";
  wrapper.style.display = "inline-flex";
  wrapper.style.position = "relative";
  wrapper.style.alignItems = "center";

  attachDropdownHoverBridge(wrapper);

  trigger.addEventListener("pointerdown", (e) => e.stopPropagation());
  trigger.addEventListener("click", (e) => {
    e.stopPropagation();
    const isOpen = wrapper.classList.contains("open");
    document.querySelectorAll(".toolbar-dropdown.open").forEach((d) => d.classList.remove("open"));
    if (!isOpen) wrapper.classList.add("open");
  });

  wrapper.appendChild(trigger);
  wrapper.appendChild(buildDropdownItems(items, () => wrapper.classList.remove("open")));
  return wrapper;
}

function createLayoutMenu(t: any, events: ToolbarEvents): HTMLElement {
  return createTextDropdownMenu(
    t.menus.plan,
    [
      { label: t.fileMenu.newPlan, shortcut: "Ctrl+N", onClick: events.onNewPlan },
      { label: t.fileMenu.openPlan, shortcut: "Ctrl+O", onClick: events.onOpenPlan },
      { label: t.fileMenu.demoTracks, onClick: () => events.onOpenDemoTracks?.() },
      { isDivider: true },
      { label: t.fileMenu.savePlan, shortcut: "Ctrl+S", onClick: events.onSavePlan },
      { label: t.fileMenu.saveAs, shortcut: "Ctrl+Shift+S", onClick: events.onSavePlanAs },
      { isDivider: true },
      { label: t.fileMenu.importCatalog, onClick: () => events.onImportCatalog?.() },
      { label: t.fileMenu.import3d, onClick: events.onImportModel },
      { label: t.fileMenu.exportXml, onClick: events.onSaveXML },
      { label: t.fileMenu.exportCsv, shortcut: "Ctrl+E", onClick: events.onExportCSV },
      { label: t.fileMenu.printPlan, shortcut: "Ctrl+P", onClick: () => events.onPrintPlan?.() },
      { isDivider: true },
      { label: t.fileMenu.collaborate, onClick: () => events.onOpenCollab?.() },
    ],
    "tb-layout",
  );
}

function createEditMenu(t: any, events: ToolbarEvents): HTMLElement {
  return createTextDropdownMenu(
    t.menus.edit,
    [
      { label: t.editMenu.undo, shortcut: "Ctrl+Z", onClick: events.onUndo, id: "item-undo" },
      { label: t.editMenu.redo, shortcut: "Ctrl+Y", onClick: events.onRedo, id: "item-redo" },
      { isDivider: true },
      { label: t.editMenu.copy, shortcut: "Ctrl+C", onClick: () => events.onCopy?.(), id: "item-copy" },
      { label: t.editMenu.paste, shortcut: "Ctrl+V", onClick: () => events.onPaste?.(), id: "item-paste" },
      { label: t.editMenu.duplicate, shortcut: "Ctrl+D", onClick: () => events.onDuplicate?.(), id: "item-duplicate" },
      { isDivider: true },
      { label: t.editMenu.selectAll, shortcut: "Ctrl+A", onClick: () => events.onSelectAll?.(), id: "item-select-all" },
      { label: t.editMenu.deselectAll, shortcut: "Esc", onClick: () => events.onDeselectAll?.(), id: "item-deselect-all" },
      { isDivider: true },
      { label: t.editMenu.rotateCw, shortcut: "R", onClick: () => events.onRotateSelected(7.5) },
      { label: t.editMenu.rotateCcw, shortcut: "Shift+R", onClick: () => events.onRotateSelected(-7.5) },
      { label: t.editMenu.changeHeight, shortcut: "M", onClick: () => events.onMoveSelectedToLayerPrompt?.() },
      { label: t.powerMenu.toggleFeeder, shortcut: "F", onClick: events.onToggleFeeder },
      { isDivider: true },
      { label: t.editMenu.deleteSelected, shortcut: "Del", onClick: events.onDeleteSelected, isDanger: true },
    ],
    "tb-edit",
  );
}

function createViewMenu(t: any, events: ToolbarEvents): HTMLElement {
  const isDe = getLanguage() === "de";
  return createTextDropdownMenu(
    t.menus.view,
    [
      { label: t.viewMenu.resetView, shortcut: "Ctrl+0", onClick: events.onResetView },
      { label: isDe ? "3D-Ansicht umschalten" : "Toggle 3D View", shortcut: "3", onClick: () => events.onToggle3D?.() },
      { isDivider: true },
      { label: t.viewMenu.toggleRulers, shortcut: "Shift+L", onClick: events.onToggleRulers, id: "item-rulers" },
      { label: t.viewMenu.woodPlate, onClick: () => events.onToggleWoodPlate?.(), id: "item-woodplate" },
      { isDivider: true },
      { label: t.viewMenu.themeSystem, onClick: () => events.onSetTheme?.("system"), id: "item-theme-system" },
      { label: t.viewMenu.themeDark, onClick: () => events.onSetTheme?.("dark"), id: "item-theme-dark" },
      { label: t.viewMenu.themeLight, onClick: () => events.onSetTheme?.("light"), id: "item-theme-light" },
      { isDivider: true },
      { label: t.viewMenu.togglePerfMonitor, shortcut: "F3", onClick: events.onTogglePerfMonitor, id: "item-perf-monitor" },
      { label: t.viewMenu.lockBars, onClick: () => events.onToggleLockBars?.(), id: "item-lock-bars" },
      { label: t.viewMenu.resetUiSizes, onClick: () => events.onResetUiSizes?.(), id: "item-reset-ui-sizes" },
    ],
    "tb-view",
  );
}

function createSettingsMenu(t: any, events: ToolbarEvents): HTMLElement {
  return createTextDropdownMenu(
    t.menus.settings,
    [
      { label: t.powerMenu.configPower, onClick: () => events.onOpenPowerSettings?.("voltage") },
      { isDivider: true },
      { label: t.toggles.snapLabel + " (" + t.toggles.snapTitle + ")", shortcut: "S", onClick: events.onToggleSnapping },
      { label: t.toggles.selectionLockLabel + " (" + t.toggles.selectionLockTitle + ")", onClick: () => events.onToggleSelectionLock?.() },
      { label: t.toggles.labelsLabel + " (" + t.toggles.labelsTitle + ")", shortcut: "N", onClick: events.onToggleTrackLabels },
      { label: t.toggles.voltageLabel + " (" + t.toggles.voltageTitle + ")", shortcut: "P", onClick: events.onTogglePowerSim },
      { isDivider: true },
      { label: t.colorsMenu.types, onClick: () => events.onSetColorMode?.("types") },
      { label: t.colorsMenu.layers, onClick: () => events.onSetColorMode?.("layers") },
      { label: t.colorsMenu.off, onClick: () => events.onSetColorMode?.("off") },
      { isDivider: true },
      {
        label: getLanguage() === "de" ? "Sprache automatisch wählen (System)" : "Auto-select language (System)",
        onClick: () => events.onAutoSelectLanguage?.(),
      },
      { isDivider: true },
      {
        label: t.menus.resetPersonalConfig,
        id: "item-reset-personal-config",
        isDanger: true,
        onClick: () => events.onResetPersonalConfig?.(),
      },
    ],
    "tb-settings",
  );
}

function createHelpMenu(t: any, events: ToolbarEvents): HTMLElement {
  const isDe = getLanguage() === "de";
  return createTextDropdownMenu(
    t.menus.help,
    [
      { label: t.helpMenu.helpGuide, shortcut: "F1", onClick: () => events.onOpenHelp?.("overview") },
      { label: t.helpMenu.shortcuts, shortcut: "?", onClick: () => events.onOpenHelp?.("shortcuts") },
      { isDivider: true },
      { label: isDe ? "Willkommens-Bildschirm…" : "Welcome Screen…", onClick: () => events.onOpenWelcome?.() },
      { isDivider: true },
      { label: t.helpMenu.layersGuide, onClick: () => events.onOpenHelp?.("layers") },
      { label: t.helpMenu.voltageGuide, onClick: () => events.onOpenHelp?.("power") },
      { label: t.helpMenu.printGuide, onClick: () => events.onOpenHelp?.("print") },
      { isDivider: true },
      { label: t.helpMenu.about, onClick: () => events.onOpenHelp?.("about") },
    ],
    "tb-help",
  );
}

function createCollabButton(t: any, events: ToolbarEvents): HTMLButtonElement {
  const btnCollab = document.createElement("button");
  btnCollab.id = "tb-collab-btn";
  btnCollab.className = "toolbar-btn btn-icon-only tb-collab-btn";
  btnCollab.title = t.status.collabTooltip;
  btnCollab.innerHTML = `<span class="tb-icon">${ICONS.users}</span>`;
  btnCollab.addEventListener("click", () => {
    events.onOpenCollab?.();
  });
  return btnCollab;
}

function createThemeButton(t: any, events: ToolbarEvents): HTMLButtonElement {
  const btnTheme = document.createElement("button");
  btnTheme.id = "tb-theme-toggle";
  btnTheme.className = "toolbar-btn btn-icon-only tb-theme-btn";
  btnTheme.title = t.viewMenu.themeTooltip;
  btnTheme.innerHTML = `<span class="tb-icon">${ICONS.sunMoon}</span>`;
  btnTheme.addEventListener("click", () => {
    events.onToggleTheme();
  });
  return btnTheme;
}

function updateCollabAndThemeState(state: ToolbarState, t: any) {
  const btnTheme = document.getElementById("tb-theme-toggle");
  if (btnTheme) {
    const pref = state.themePref || "system";
    let themeIcon = ICONS.sunMoon;
    let label = t.viewMenu.themeStatusSystem;

    if (pref === "dark") {
      themeIcon = ICONS.moon;
      label = t.viewMenu.themeStatusDark;
    } else if (pref === "light") {
      themeIcon = ICONS.sun;
      label = t.viewMenu.themeStatusLight;
    }

    btnTheme.innerHTML = `<span class="tb-icon">${themeIcon}</span>`;
    const resolvedLabel = state.resolvedTheme === "dark" ? t.viewMenu.themeStatusDark : t.viewMenu.themeStatusLight;
    btnTheme.title = pref === "system"
      ? `${t.viewMenu.themeTooltip} (${t.viewMenu.themeStatusSystem}: ${resolvedLabel})`
      : `${t.viewMenu.themeTooltip} (${label})`;
  }

  const btnCollab = document.getElementById("tb-collab-btn");
  if (btnCollab && state.collabStatus !== undefined) {
    btnCollab.innerHTML = `<span class="tb-icon">${ICONS.users}</span>`;
    if (state.collabStatus === "connected") {
      const count = (state.collabUsersCount || 0) + 1;
      btnCollab.classList.add("active");
      btnCollab.title = `${t.status.collabButton}: ${t.status.collabOnline.replace("{count}", String(count))}`;
    } else if (state.collabStatus === "connecting") {
      btnCollab.classList.remove("active");
      btnCollab.title = `${t.status.collabButton}: ${t.status.collabConnecting}`;
    } else {
      btnCollab.classList.remove("active");
      btnCollab.title = t.status.collabTooltip;
    }
  }
}

export function updateToolbarState(state: ToolbarState) {
  const t = getTranslations().toolbar;

  const btnUndo = document.getElementById("tb-undo") as HTMLButtonElement | null;
  if (btnUndo && state.canUndo !== undefined) btnUndo.disabled = !state.canUndo;

  const btnRedo = document.getElementById("tb-redo") as HTMLButtonElement | null;
  if (btnRedo && state.canRedo !== undefined) btnRedo.disabled = !state.canRedo;

  const itemUndo = document.getElementById("item-undo") as HTMLButtonElement | null;
  if (itemUndo && state.canUndo !== undefined) {
    itemUndo.disabled = !state.canUndo;
    itemUndo.style.opacity = state.canUndo ? "1" : "0.5";
    itemUndo.style.pointerEvents = state.canUndo ? "auto" : "none";
  }

  const itemRedo = document.getElementById("item-redo") as HTMLButtonElement | null;
  if (itemRedo && state.canRedo !== undefined) {
    itemRedo.disabled = !state.canRedo;
    itemRedo.style.opacity = state.canRedo ? "1" : "0.5";
    itemRedo.style.pointerEvents = state.canRedo ? "auto" : "none";
  }

  const btnRotate = document.getElementById("tb-rotate") as HTMLButtonElement | null;
  if (btnRotate && state.hasSelection !== undefined) btnRotate.disabled = !state.hasSelection;

  const btnDelete = document.getElementById("tb-delete") as HTMLButtonElement | null;
  if (btnDelete && state.hasSelection !== undefined) btnDelete.disabled = !state.hasSelection;

  const btnSnap = document.getElementById("tb-snap");
  if (btnSnap) {
    setBtnLabel(btnSnap, ICONS.magnet, t.toggles.snapLabel, "Snap");
    if (state.snappingEnabled !== undefined) {
      btnSnap.classList.toggle("active", !!state.snappingEnabled);
    }
  }

  const btnSelectionLock = document.getElementById("tb-selection-lock");
  if (btnSelectionLock && state.selectionLockDisabled !== undefined) {
    const isUnlocked = !!state.selectionLockDisabled;
    btnSelectionLock.classList.toggle("active", isUnlocked);
    btnSelectionLock.classList.toggle("tb-unlocked-alert", isUnlocked);
    setBtnLabel(
      btnSelectionLock,
      isUnlocked ? ICONS.unlock : ICONS.lock,
      isUnlocked ? t.toggles.selectionLockDisabledLabel : t.toggles.selectionLockLabel,
      isUnlocked ? "Unlocked" : "Lock",
    );
    btnSelectionLock.title = t.toggles.selectionLockTitle;
  }

  const btnMeasure = document.getElementById("tb-measure");
  if (btnMeasure) {
    setBtnLabel(btnMeasure, ICONS.ruler, t.toggles.measureLabel, "Dist");
    if (state.measureToolActive !== undefined) {
      btnMeasure.classList.toggle("active", !!state.measureToolActive);
    }
  }

  const btnColors = document.getElementById("tb-colors");
  if (btnColors) {
    setBtnLabel(btnColors, ICONS.palette, t.toggles.colorsLabel, "Col");
    if (state.colorMode !== undefined || state.colorCodeEnabled !== undefined) {
      const mode = state.colorMode ?? (state.colorCodeEnabled ? "types" : "off");
      const isActive = mode !== "off";
      const isLayers = mode === "layers";
      btnColors.classList.toggle("active", isActive);
      btnColors.classList.toggle("mode-layers", isLayers);
      const colorsGroup = (btnColors.closest?.(".toolbar-split-group") as HTMLElement | null) || btnColors.parentElement;
      if (colorsGroup) {
        colorsGroup.classList.toggle("active", isActive);
        colorsGroup.classList.toggle("mode-layers", isLayers);
      }
      btnColors.title = mode === "layers"
        ? `${t.toggles.colorsTitle} (${t.colorsMenu.layers})`
        : (mode === "types" ? `${t.toggles.colorsTitle} (${t.colorsMenu.types})` : t.toggles.colorsTitle);

      const updateCheck = (id: string, isSelected: boolean, text: string) => {
        const el = document.getElementById(id);
        if (!el) return;
        const span = el.querySelector(".dropdown-item-left span:last-child");
        if (span) span.textContent = (isSelected ? "✓ " : "") + text;
      };

      updateCheck("split-color-types", mode === "types", t.colorsMenu.types);
      updateCheck("split-color-layers", mode === "layers", t.colorsMenu.layers);
      updateCheck("split-color-off", mode === "off", t.colorsMenu.off);
    }
  }

  const btnLabels = document.getElementById("tb-labels");
  if (btnLabels) {
    setBtnLabel(btnLabels, ICONS.tag, t.toggles.labelsLabel, "Labels");
    if (state.trackLabelsEnabled !== undefined) {
      btnLabels.classList.toggle("active", !!state.trackLabelsEnabled);
    }
  }

  const btnPower = document.getElementById("tb-power");
  if (btnPower) {
    setBtnLabel(btnPower, ICONS.zap, t.toggles.voltageLabel, "Volt");
    if (state.powerSimEnabled !== undefined) {
      const isPowerActive = !!state.powerSimEnabled;
      btnPower.classList.toggle("active-warning", isPowerActive);
      const powerGroup = (btnPower.closest?.(".toolbar-split-group") as HTMLElement | null) || btnPower.parentElement;
      if (powerGroup) {
        powerGroup.classList.toggle("active-warning", isPowerActive);
      }
    }
  }

  const btnShortCircuit = document.getElementById("tb-short-circuit");
  if (btnShortCircuit && state.hasShortCircuit !== undefined) {
    if (state.hasShortCircuit) {
      btnShortCircuit.style.display = "inline-flex";
      btnShortCircuit.title = t.status.shortCircuitTooltip;
      const lbl = btnShortCircuit.querySelector(".tb-short-circuit-label");
      if (lbl) {
        lbl.textContent =
          state.shortCircuitCount && state.shortCircuitCount > 1
            ? `${t.status.shortCircuitButton} (${state.shortCircuitCount})`
            : t.status.shortCircuitButton;
      }
    } else {
      btnShortCircuit.style.display = "none";
    }
  }

  const countBadge = document.getElementById("tb-piece-count");
  if (countBadge && typeof state.totalPieces === "number") {
    const unit = state.totalPieces === 1 ? t.status.tracksUnit : t.status.tracksUnitPlural;
    countBadge.textContent = `${state.totalPieces} ${unit}`;
  }

  const btnLang = document.getElementById("tb-lang-toggle");
  if (btnLang) {
    btnLang.innerHTML = `<span style="font-weight: 700; font-size: 11px;">${getLanguage().toUpperCase()}</span>`;
    btnLang.title = t.status.langSwitchTitle;
  }

  const fileLabel = document.getElementById("tb-file-name");
  if (fileLabel && state.activeFileName) {
    const isDe = getLanguage() === "de";
    const dirty = state.isDirty ? " ●" : "";
    fileLabel.textContent = `${state.activeFileName}${dirty}`;
    fileLabel.classList.toggle("dirty", !!state.isDirty);
    fileLabel.title = state.isDirty
      ? (isDe ? "Ungespeicherte Änderungen (Klicken zum Speichern, Strg+S)" : "Unsaved changes (Click to save, Ctrl+S)")
      : (isDe ? "Alle Änderungen gespeichert" : "All changes saved");
  }

  const btnSave = document.getElementById("tb-save");
  if (btnSave && state.isDirty !== undefined) {
    const isDe = getLanguage() === "de";
    btnSave.classList.toggle("has-unsaved", !!state.isDirty);
    btnSave.title = state.isDirty
      ? `${t.fileMenu.savePlan} ● (${isDe ? "ungespeichert" : "unsaved"}, Ctrl+S)`
      : `${t.fileMenu.savePlan} (Ctrl+S)`;
  }

  const btnRightSidebar = document.getElementById("tb-right-sidebar-toggle");
  if (btnRightSidebar && state.rightSidebarCollapsed !== undefined) {
    btnRightSidebar.classList.toggle("active", !state.rightSidebarCollapsed);
    btnRightSidebar.title = state.rightSidebarCollapsed
      ? "Show Floating Layers Menu (Shift+B)"
      : "Hide Floating Layers Menu (Shift+B)";
  }

  const btn3D = document.getElementById("tb-3d");
  if (btn3D && state.is3DActive !== undefined) {
    if (state.is3DActive) {
      btn3D.classList.add("active");
      setBtnLabel(btn3D, ICONS.plan2d, "2D Plan", "2D");
      btn3D.title = "3 / V: Return to 2D track planning mode";
    } else {
      btn3D.classList.remove("active");
      setBtnLabel(btn3D, ICONS.box3d, "3D", "3D");
      btn3D.title = t.toggles.toggle3dTitle;
    }
  }

  const elLockBars = document.getElementById("item-lock-bars");
  if (elLockBars && state.barsLocked !== undefined) {
    const span = elLockBars.querySelector(".dropdown-item-left span:last-child");
    if (span) {
      span.textContent = `${state.barsLocked ? "✓ " : ""}${t.viewMenu.lockBars}`;
    }
  }

  updateCollabAndThemeState(state, t);
}

function buildDropdownItems(items: DropdownItemConfig[], onClose: () => void): HTMLElement {
  const menu = document.createElement("div");
  menu.className = "dropdown-menu";

  for (const item of items) {
    if (item.sectionHeader) {
      const header = document.createElement("div");
      header.className = "dropdown-section-header";
      header.textContent = item.sectionHeader;
      menu.appendChild(header);
      continue;
    }

    if (item.isDivider) {
      const divider = document.createElement("div");
      divider.className = "dropdown-divider";
      menu.appendChild(divider);
      continue;
    }

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `dropdown-item ${item.isDanger ? "item-danger" : ""}`;
    if (item.id) btn.id = item.id;

    btn.innerHTML = `
      <span class="dropdown-item-left">
        <span>${item.label}</span>
      </span>
      ${item.shortcut ? `<span class="dropdown-shortcut">${item.shortcut}</span>` : ""}
    `;

    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      onClose();
      item.onClick?.();
    });

    menu.appendChild(btn);
  }

  return menu;
}

function createDropdownMenu(triggerLabel: string, items: DropdownItemConfig[]): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = "toolbar-dropdown";

  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "toolbar-btn dropdown-trigger";
  trigger.innerHTML = `${triggerLabel ? `<span>${triggerLabel}</span>` : ""} <span class="dropdown-arrow">${ICONS.chevronDown}</span>`;

  // Attach safe-hover bridge
  attachDropdownHoverBridge(wrapper);

  trigger.addEventListener("pointerdown", (e) => e.stopPropagation());
  trigger.addEventListener("click", (e) => {
    e.stopPropagation();
    const isOpen = wrapper.classList.contains("open");
    document.querySelectorAll(".toolbar-dropdown.open").forEach((d) => d.classList.remove("open"));
    if (!isOpen) wrapper.classList.add("open");
  });

  wrapper.appendChild(trigger);
  wrapper.appendChild(buildDropdownItems(items, () => wrapper.classList.remove("open")));
  return wrapper;
}

function setBtnLabel(btn: HTMLElement, iconSvg: string, fullLabel: string, compactLabel?: string) {
  const compact = compactLabel !== undefined ? compactLabel : fullLabel;
  const iconMarkup = iconSvg ? `<span class="tb-btn-icon">${iconSvg}</span>` : "";
  btn.innerHTML = `${iconMarkup}<span class="tb-text-full">${fullLabel}</span><span class="tb-text-compact">${compact}</span>`;
}

function createIconButton(iconSvg: string, title: string, onClick: () => void, id?: string, extraClass = ""): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.className = `toolbar-btn btn-icon-only ${extraClass}`.trim();
  btn.title = title;
  if (id) btn.id = id;
  btn.innerHTML = `<span class="tb-icon">${iconSvg}</span>`;
  btn.addEventListener("click", onClick);
  return btn;
}

function createToggleBtn(
  iconSvg: string,
  fullLabel: string,
  compactLabel: string,
  initialActive: boolean,
  onClick: () => void,
  id: string,
): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.className = `toolbar-btn toolbar-toggle ${initialActive ? "active" : ""}`;
  btn.id = id;
  setBtnLabel(btn, iconSvg, fullLabel, compactLabel);
  btn.addEventListener("click", onClick);
  return btn;
}

function createDivider(): HTMLDivElement {
  const d = document.createElement("div");
  d.className = "toolbar-divider";
  return d;
}

let dropdownDismissSetup = false;
function setupDropdownGlobalDismiss() {
  if (typeof window === "undefined") return;
  if (dropdownDismissSetup) return;
  dropdownDismissSetup = true;

  const dismiss = (e: Event) => {
    const target = e.target as HTMLElement;
    if (!target.closest(".toolbar-dropdown")) {
      document.querySelectorAll(".toolbar-dropdown.open").forEach((d) => d.classList.remove("open"));
    }
  };

  window.addEventListener("pointerdown", dismiss);
  window.addEventListener("click", dismiss);
  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      document.querySelectorAll(".toolbar-dropdown.open").forEach((d) => d.classList.remove("open"));
    }
  });
}