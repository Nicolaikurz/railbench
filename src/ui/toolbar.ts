import type { ColorMode } from "@/core/types";
import { getTranslations, getLanguage, toggleLanguage } from "@/i18n";

export interface ToolbarEvents {
  onToggleRightSidebar?: () => void;
  onNewPlan: () => void;
  onOpenPlan: () => void;
  onSavePlan: () => void;
  onSavePlanAs: () => void;
  onSaveXML: () => void;
  onExportCSV: () => void;
  onPrintPlan?: () => void;
  onMoveSelectedToLayerPrompt?: () => void;
  onOpenHelp?: (tab?: string) => void;
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
  onToggleTheme: () => void;
  onSetTheme?: (theme: "system" | "light" | "dark") => void;
  onToggleMeasureTool?: () => void;
  onToggleWoodPlate?: () => void;
  onToggleSnapping: () => void;
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
}

export interface ToolbarState {
  canUndo?: boolean;
  canRedo?: boolean;
  hasSelection?: boolean;
  snappingEnabled?: boolean;
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

const ICONS = {
  eye: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>`,
  helpCircle: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`,
  sun: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`,
  moon: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`,
  sunMoon: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/><path d="M12 8a4 4 0 0 1 4 4 4 4 0 0 1-4 4z" fill="currentColor"/></svg>`,
  users: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>`,
  train: `<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="3" width="16" height="16" rx="2"/><path d="M4 11h16"/><path d="M12 3v8"/><path d="m8 19-2 3"/><path d="m18 22-2-3"/><circle cx="8" cy="15" r="1"/><circle cx="16" cy="15" r="1"/></svg>`,
  chevronDown: `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>`,
  filePlus: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="15" y2="15"/></svg>`,
  folderOpen: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 14 1.45-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5c0-1.1.9-2 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2"/></svg>`,
  save: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/><polyline points="17 21 17 13 7 13 7 21"/><polyline points="7 3 7 8 15 8"/></svg>`,
  undo: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7v6h6"/><path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"/></svg>`,
  redo: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 7v6h-6"/><path d="M3 17a9 9 0 0 1 9-9 9 9 0 0 1 6 2.3l3 2.7"/></svg>`,
  rotateCw: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8"/><polyline points="21 3 21 8 16 8"/></svg>`,
  trash: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/></svg>`,
  magnet: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 15-4-4 6.7-6.7a7.78 7.78 0 0 1 11 11L13 22l-4-4 4.7-4.7a2.2 2.2 0 0 0 0-3.1 2.2 2.2 0 0 0-3.1 0L6 15z"/><path d="m5 8 4 4"/><path d="m12 15 4 4"/></svg>`,
  ruler: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.3 8.7 8.7 21.3c-1 1-2.5 1-3.4 0l-2.6-2.6c-1-1-1-2.5 0-3.4L15.3 2.7c1-1 2.5-1 3.4 0l2.6 2.6c1 1 1 2.5 0 3.4Z"/><path d="m14.5 3.5 2 2"/><path d="m11.5 6.5 2 2"/><path d="m8.5 9.5 2 2"/><path d="m5.5 12.5 2 2"/></svg>`,
  palette: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.563-2.512 5.563-5.563C22 6.5 17.5 2 12 2Z"/></svg>`,
  tag: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2H2v10l9.29 9.29c.94.94 2.48.94 3.42 0l6.58-6.58c.94-.94.94-2.48 0-3.42L12 2Z"/><circle cx="7" cy="7" r=".5" fill="currentColor"/></svg>`,
  zap: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>`,
  alertTriangle: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`,
  box3d: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21 8-9-4-9 4v8l9 4 9-4V8Z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>`,
  plan2d: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="9" y1="21" x2="9" y2="9"/></svg>`,
  layers: `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>`,
};

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

export function createToolbar(host: HTMLElement, events: ToolbarEvents) {
  host.innerHTML = "";
  const t = getTranslations().toolbar;

  const container = document.createElement("div");
  container.className = "toolbar-container";

  const mainMenu = createBrandMainMenu(t, events);
  container.appendChild(mainMenu);
  container.appendChild(createDivider());

  const fileActionsGroup = document.createElement("div");
  fileActionsGroup.className = "toolbar-group toolbar-file-quick";

  const btnNew = createIconButton(ICONS.filePlus, `${t.fileMenu.newPlan} (Ctrl+N)`, events.onNewPlan, "tb-new");
  const btnOpen = createIconButton(ICONS.folderOpen, `${t.fileMenu.openPlan} (Ctrl+O)`, events.onOpenPlan, "tb-open");
  const btnSave = createIconButton(ICONS.save, `${t.fileMenu.savePlan} (Ctrl+S)`, events.onSavePlan, "tb-save");
  const viewMenu = createViewMenu(t, events);
  const helpMenu = createHelpMenu(t, events);

  fileActionsGroup.appendChild(btnNew);
  fileActionsGroup.appendChild(btnOpen);
  fileActionsGroup.appendChild(btnSave);
  fileActionsGroup.appendChild(viewMenu);
  fileActionsGroup.appendChild(helpMenu);

  container.appendChild(fileActionsGroup);
  container.appendChild(createDivider());

  const editQuickGroup = document.createElement("div");
  editQuickGroup.className = "toolbar-group toolbar-edit-quick";

  const btnUndo = createIconButton(ICONS.undo, `${t.quick.undoTitle} (Ctrl+Z)`, events.onUndo, "tb-undo");
  const btnRedo = createIconButton(ICONS.redo, `${t.quick.redoTitle} (Ctrl+Y)`, events.onRedo, "tb-redo");
  const btnRotate = createIconButton(ICONS.rotateCw, `${t.quick.rotateTitle} (R)`, () => events.onRotateSelected(7.5), "tb-rotate");
  const btnDelete = createIconButton(ICONS.trash, `${t.quick.deleteTitle} (Del)`, events.onDeleteSelected, "tb-delete", "btn-danger");

  editQuickGroup.appendChild(btnUndo);
  editQuickGroup.appendChild(btnRedo);
  editQuickGroup.appendChild(btnRotate);
  editQuickGroup.appendChild(btnDelete);
  container.appendChild(editQuickGroup);
  container.appendChild(createDivider());

  const featureGroup = document.createElement("div");
  featureGroup.className = "toolbar-group toolbar-features";

  const btnSnap = createToggleBtn(ICONS.magnet, t.toggles.snapLabel, "Snap", true, events.onToggleSnapping, "tb-snap");
  btnSnap.title = t.toggles.snapTitle;

  const btnMeasure = createToggleBtn(ICONS.ruler, t.toggles.measureLabel, "Dist", false, () => events.onToggleMeasureTool?.(), "tb-measure");
  btnMeasure.title = t.toggles.measureTitle;

  const colorsWrapper = document.createElement("div");
  colorsWrapper.className = "toolbar-split-group";

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

  const btn3D = createToggleBtn(ICONS.box3d, "3D (Alpha)", "3D", false, () => events.onToggle3D?.(), "tb-3d");
  btn3D.title = t.toggles.toggle3dTitle;

  featureGroup.appendChild(btnSnap);
  featureGroup.appendChild(btnMeasure);
  featureGroup.appendChild(colorsWrapper);
  featureGroup.appendChild(btnLabels);
  featureGroup.appendChild(powerWrapper);
  featureGroup.appendChild(btnShortCircuit);
  featureGroup.appendChild(btn3D);
  container.appendChild(featureGroup);

  const rightGroup = document.createElement("div");
  rightGroup.className = "toolbar-group toolbar-right";

  const fileLabel = document.createElement("span");
  fileLabel.id = "tb-file-name";
  fileLabel.className = "brand-file";
  fileLabel.title = "Current layout plan";
  fileLabel.textContent = "Model Railway.railbench";

  const countBadge = document.createElement("span");
  countBadge.id = "tb-piece-count";
  countBadge.className = "brand-badge";
  countBadge.textContent = `0 ${t.status.tracksUnitPlural}`;

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

  rightGroup.appendChild(fileLabel);
  rightGroup.appendChild(countBadge);
  rightGroup.appendChild(btnCollab);
  rightGroup.appendChild(btnTheme);
  rightGroup.appendChild(btnLang);
  rightGroup.appendChild(btnRightSidebar);
  container.appendChild(rightGroup);

  host.appendChild(container);
  setupDropdownGlobalDismiss();
}

function createBrandMainMenu(t: any, events: ToolbarEvents): HTMLElement {
  const brandMenuWrapper = document.createElement("div");
  brandMenuWrapper.className = "toolbar-dropdown brand-main-menu";

  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.id = "tb-brand-trigger";
  trigger.className = "toolbar-btn toolbar-brand-btn dropdown-trigger";
  
  trigger.style.display = "inline-flex";
  trigger.style.alignItems = "center";
  trigger.style.gap = "6px";
  trigger.style.background = "rgba(255, 255, 255, 0.08)";
  trigger.style.border = "1px solid rgba(255, 255, 255, 0.12)";
  trigger.style.color = "#ffffff";
  trigger.style.padding = "0 8px";
  trigger.style.borderRadius = "4px";
  trigger.style.height = "28px";
  trigger.style.cursor = "pointer";

  trigger.innerHTML = `
    <span class="brand-icon" style="display:inline-flex; align-items:center; color:#ffffff;">${ICONS.train}</span>
    <span class="brand-title" style="color:#ffffff; font-weight:600; font-size:12px;">RAILbench</span>
    <span class="dropdown-arrow" style="display:inline-flex; align-items:center; color:#ffffff;">${ICONS.chevronDown}</span>
  `;

  // Attach safe-hover bridge with delayed close
  attachDropdownHoverBridge(
    brandMenuWrapper,
    () => { trigger.style.background = "rgba(255, 255, 255, 0.15)"; },
    () => { trigger.style.background = "rgba(255, 255, 255, 0.08)"; }
  );

  // Keep click behavior for mobile/touch or explicit clicks
  trigger.addEventListener("pointerdown", (e) => e.stopPropagation());
  trigger.addEventListener("click", (e) => {
    e.stopPropagation();
    const isOpen = brandMenuWrapper.classList.contains("open");
    document.querySelectorAll(".toolbar-dropdown.open").forEach((d) => d.classList.remove("open"));
    if (!isOpen) {
      brandMenuWrapper.classList.add("open");
      trigger.style.background = "rgba(255, 255, 255, 0.15)";
    } else {
      trigger.style.background = "rgba(255, 255, 255, 0.08)";
    }
  });

  brandMenuWrapper.appendChild(trigger);

  const menuItems: DropdownItemConfig[] = [
    { sectionHeader: t.menus.plan },
    { label: t.fileMenu.newPlan, shortcut: "Ctrl+N", onClick: events.onNewPlan },
    { label: t.fileMenu.openPlan, shortcut: "Ctrl+O", onClick: events.onOpenPlan },
    { label: t.fileMenu.savePlan, shortcut: "Ctrl+S", onClick: events.onSavePlan },
    { label: t.fileMenu.saveAs, shortcut: "Ctrl+Shift+S", onClick: events.onSavePlanAs },
    { label: t.fileMenu.import3d, onClick: events.onImportModel },
    { label: t.fileMenu.exportXml, onClick: events.onSaveXML },
    { label: t.fileMenu.exportCsv, shortcut: "Ctrl+E", onClick: events.onExportCSV },
    { label: t.fileMenu.printPlan, shortcut: "Ctrl+P", onClick: () => events.onPrintPlan?.() },
    { label: t.fileMenu.collaborate, onClick: () => events.onOpenCollab?.() },

    { isDivider: true },
    { sectionHeader: t.menus.edit },
    { label: t.editMenu.undo, shortcut: "Ctrl+Z", onClick: events.onUndo, id: "item-undo" },
    { label: t.editMenu.redo, shortcut: "Ctrl+Y", onClick: events.onRedo, id: "item-redo" },
    { label: t.editMenu.copy, shortcut: "Ctrl+C", onClick: () => events.onCopy?.(), id: "item-copy" },
    { label: t.editMenu.paste, shortcut: "Ctrl+V", onClick: () => events.onPaste?.(), id: "item-paste" },
    { label: t.editMenu.duplicate, shortcut: "Ctrl+D", onClick: () => events.onDuplicate?.(), id: "item-duplicate" },
    { label: t.editMenu.selectAll, shortcut: "Ctrl+A", onClick: () => events.onSelectAll?.(), id: "item-select-all" },
    { label: t.editMenu.deselectAll, shortcut: "Esc", onClick: () => events.onDeselectAll?.(), id: "item-deselect-all" },
    { label: t.editMenu.rotateCw, shortcut: "R", onClick: () => events.onRotateSelected(7.5) },
    { label: t.editMenu.rotateCcw, shortcut: "Shift+R", onClick: () => events.onRotateSelected(-7.5) },
    { label: t.editMenu.deleteSelected, shortcut: "Del", onClick: events.onDeleteSelected, isDanger: true },
    { label: t.editMenu.changeHeight, shortcut: "M", onClick: () => events.onMoveSelectedToLayerPrompt?.() },
  ];

  const menu = buildDropdownItems(menuItems, () => brandMenuWrapper.classList.remove("open"));
  brandMenuWrapper.appendChild(menu);
  return brandMenuWrapper;
}

function createIconDropdownMenu(iconSvg: string, tooltip: string, items: DropdownItemConfig[], id?: string): HTMLElement {
  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "toolbar-btn btn-icon-only";
  trigger.title = tooltip;
  trigger.style.display = "inline-flex";
  trigger.style.alignItems = "center";
  trigger.style.justifyContent = "center";
  trigger.style.boxSizing = "border-box";
  if (id) trigger.id = id;
  trigger.innerHTML = `<span class="tb-icon">${iconSvg}</span>`;

  const wrapper = document.createElement("div");
  wrapper.className = "toolbar-dropdown";
  wrapper.style.display = "inline-flex";
  wrapper.style.position = "relative";
  wrapper.style.alignItems = "center";

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

function createViewMenu(t: any, events: ToolbarEvents): HTMLElement {
  return createIconDropdownMenu(
    ICONS.eye,
    t.menus.view,
    [
      { label: t.viewMenu.resetView, shortcut: "Ctrl+0", onClick: events.onResetView },
      { isDivider: true },
      { label: t.viewMenu.toggleRulers, shortcut: "Shift+L", onClick: events.onToggleRulers, id: "item-rulers" },
      { label: t.viewMenu.woodPlate, onClick: () => events.onToggleWoodPlate?.(), id: "item-woodplate" },
      { isDivider: true },
      { label: t.viewMenu.themeSystem, onClick: () => events.onSetTheme?.("system"), id: "item-theme-system" },
      { label: t.viewMenu.themeDark, onClick: () => events.onSetTheme?.("dark"), id: "item-theme-dark" },
      { label: t.viewMenu.themeLight, onClick: () => events.onSetTheme?.("light"), id: "item-theme-light" },
      { isDivider: true },
      { label: t.viewMenu.togglePerfMonitor, shortcut: "F3", onClick: events.onTogglePerfMonitor, id: "item-perf-monitor" },
    ],
    "tb-view",
  );
}

function createHelpMenu(t: any, events: ToolbarEvents): HTMLElement {
  return createIconDropdownMenu(
    ICONS.helpCircle,
    t.menus.help,
    [
      { label: t.helpMenu.helpGuide, shortcut: "F1", onClick: () => events.onOpenHelp?.("overview") },
      { label: t.helpMenu.shortcuts, shortcut: "?", onClick: () => events.onOpenHelp?.("shortcuts") },
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

  const btnRotate = document.getElementById("tb-rotate") as HTMLButtonElement | null;
  if (btnRotate) btnRotate.disabled = !state.hasSelection;

  const btnDelete = document.getElementById("tb-delete") as HTMLButtonElement | null;
  if (btnDelete) btnDelete.disabled = !state.hasSelection;

  const btnSnap = document.getElementById("tb-snap");
  if (btnSnap) {
    setBtnLabel(btnSnap, ICONS.magnet, t.toggles.snapLabel, "Snap");
    btnSnap.classList.toggle("active", !!state.snappingEnabled);
  }

  const btnMeasure = document.getElementById("tb-measure");
  if (btnMeasure) {
    setBtnLabel(btnMeasure, ICONS.ruler, t.toggles.measureLabel, "Dist");
    btnMeasure.classList.toggle("active", !!state.measureToolActive);
  }

  const btnColors = document.getElementById("tb-colors");
  if (btnColors) {
    const mode = state.colorMode ?? (state.colorCodeEnabled ? "types" : "off");
    setBtnLabel(btnColors, ICONS.palette, t.toggles.colorsLabel, "Col");
    btnColors.classList.toggle("active", mode !== "off");
    btnColors.classList.toggle("mode-layers", mode === "layers");
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

  const btnLabels = document.getElementById("tb-labels");
  if (btnLabels) {
    setBtnLabel(btnLabels, ICONS.tag, t.toggles.labelsLabel, "Labels");
    btnLabels.classList.toggle("active", !!state.trackLabelsEnabled);
  }

  const btnPower = document.getElementById("tb-power");
  if (btnPower) {
    setBtnLabel(btnPower, ICONS.zap, t.toggles.voltageLabel, "Volt");
    btnPower.classList.toggle("active-warning", !!state.powerSimEnabled);
  }

  const btnShortCircuit = document.getElementById("tb-short-circuit");
  if (btnShortCircuit) {
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
    const dirty = state.isDirty ? " ●" : "";
    fileLabel.textContent = `${state.activeFileName}${dirty}`;
    fileLabel.classList.toggle("dirty", !!state.isDirty);
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
      setBtnLabel(btn3D, ICONS.box3d, "3D (Alpha)", "3D");
      btn3D.title = "3 / V: Switch to realistic 3D perspective with layers & bridges (Alpha)";
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