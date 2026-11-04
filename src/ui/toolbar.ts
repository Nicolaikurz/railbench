import type { ColorMode } from "@/core/types";
import { getTranslations, getLanguage, setLanguage, toggleLanguage } from "@/i18n";

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

export function createToolbar(host: HTMLElement, events: ToolbarEvents) {
  host.innerHTML = "";
  const t = getTranslations().toolbar;

  const container = document.createElement("div");
  container.className = "toolbar-container";

  // 1. Brand
  const brand = document.createElement("div");
  brand.className = "toolbar-brand";
  brand.innerHTML = `
    <span class="brand-icon"></span>
    <span class="brand-title">RAILbench</span>
  `;
  container.appendChild(brand);

  container.appendChild(createDivider());

  // 2. Dropdown Menus: File, Edit, View, Help, Language
  const menusGroup = document.createElement("div");
  menusGroup.className = "toolbar-group";

  // 2A. Layout / Gleisplan Menu Dropdown (Option B)
  const fileMenu = createDropdownMenu(t.menus.plan, [
    { label: t.fileMenu.newPlan, shortcut: "Ctrl+N", onClick: events.onNewPlan },
    { label: t.fileMenu.openPlan, shortcut: "Ctrl+O", onClick: events.onOpenPlan },
    { label: t.fileMenu.savePlan, shortcut: "Ctrl+S", onClick: events.onSavePlan },
    { label: t.fileMenu.saveAs, shortcut: "Ctrl+Shift+S", onClick: events.onSavePlanAs },
    { isDivider: true },
    { label: t.fileMenu.import3d, onClick: events.onImportModel },
    { isDivider: true },
    { label: t.fileMenu.exportXml, onClick: events.onSaveXML },
    { label: t.fileMenu.exportCsv, shortcut: "Ctrl+E", onClick: events.onExportCSV },
    { isDivider: true },
    { label: t.fileMenu.printPlan, shortcut: "Ctrl+P", onClick: () => events.onPrintPlan?.() },
    { isDivider: true },
    { label: t.fileMenu.collaborate, onClick: () => events.onOpenCollab?.() },
  ]);
  menusGroup.appendChild(fileMenu);

  // 2B. Edit / Bearbeiten Menu Dropdown
  const editMenu = createDropdownMenu(t.menus.edit, [
    { label: t.editMenu.undo, shortcut: "Ctrl+Z", onClick: events.onUndo, id: "item-undo" },
    { label: t.editMenu.redo, shortcut: "Ctrl+Y", onClick: events.onRedo, id: "item-redo" },
    { isDivider: true },
    { label: t.editMenu.copy, shortcut: "Ctrl+C", onClick: () => events.onCopy?.(), id: "item-copy" },
    { label: t.editMenu.paste, shortcut: "Ctrl+V", onClick: () => events.onPaste?.(), id: "item-paste" },
    { label: t.editMenu.duplicate, shortcut: "Ctrl+D", onClick: () => events.onDuplicate?.(), id: "item-duplicate" },
    { label: t.editMenu.selectAll, shortcut: "Ctrl+A", onClick: () => events.onSelectAll?.(), id: "item-select-all" },
    { label: t.editMenu.deselectAll, shortcut: "Esc", onClick: () => events.onDeselectAll?.(), id: "item-deselect-all" },
    { isDivider: true },
    { label: t.editMenu.rotateCw, shortcut: "R", onClick: () => events.onRotateSelected(7.5) },
    { label: t.editMenu.rotateCcw, shortcut: "Shift+R", onClick: () => events.onRotateSelected(-7.5) },
    { label: t.editMenu.deleteSelected, shortcut: "Del", onClick: events.onDeleteSelected, isDanger: true },
    { isDivider: true },
    { label: t.editMenu.changeHeight, shortcut: "M", onClick: () => events.onMoveSelectedToLayerPrompt?.() },
  ]);
  menusGroup.appendChild(editMenu);

  // 2C. View / Ansicht Menu Dropdown
  const viewMenu = createDropdownMenu(t.menus.view, [
    { label: t.viewMenu.resetView, shortcut: "Ctrl+0", onClick: events.onResetView },
    { isDivider: true },
    { label: t.viewMenu.toggleRulers, shortcut: "Shift+L", onClick: events.onToggleRulers, id: "item-rulers" },
    { label: t.viewMenu.measureDistance, shortcut: "M", onClick: () => events.onToggleMeasureTool?.(), id: "item-measure" },
    { label: t.viewMenu.woodPlate, onClick: () => events.onToggleWoodPlate?.(), id: "item-woodplate" },
    { isDivider: true },
    { label: t.viewMenu.themeSystem, onClick: () => events.onSetTheme?.("system"), id: "item-theme-system" },
    { label: t.viewMenu.themeDark, onClick: () => events.onSetTheme?.("dark"), id: "item-theme-dark" },
    { label: t.viewMenu.themeLight, onClick: () => events.onSetTheme?.("light"), id: "item-theme-light" },
    { isDivider: true },
    { label: t.viewMenu.togglePerfMonitor, shortcut: "F3", onClick: events.onTogglePerfMonitor, id: "item-perf-monitor" },
  ]);
  menusGroup.appendChild(viewMenu);

  // 2D. Help / Hilfe Menu Dropdown
  const helpMenu = createDropdownMenu(t.menus.help, [
    { label: t.helpMenu.helpGuide, shortcut: "F1", onClick: () => events.onOpenHelp?.("overview") },
    { label: t.helpMenu.shortcuts, shortcut: "?", onClick: () => events.onOpenHelp?.("shortcuts") },
    { isDivider: true },
    { label: t.helpMenu.layersGuide, onClick: () => events.onOpenHelp?.("layers") },
    { label: t.helpMenu.voltageGuide, onClick: () => events.onOpenHelp?.("power") },
    { label: t.helpMenu.printGuide, onClick: () => events.onOpenHelp?.("print") },
    { isDivider: true },
    { label: t.helpMenu.about, onClick: () => events.onOpenHelp?.("about") },
  ]);
  menusGroup.appendChild(helpMenu);

  container.appendChild(menusGroup);

  container.appendChild(createDivider());

  // 3. Quick Actions: Undo, Redo, Rotate, Delete (1-click direct access)
  const quickGroup = document.createElement("div");
  quickGroup.className = "toolbar-group";

  const btnUndo = createIconButton("↶", t.quick.undoTitle, events.onUndo, "tb-undo");
  const btnRedo = createIconButton("↷", t.quick.redoTitle, events.onRedo, "tb-redo");
  const btnRotate = createIconButton("↻", t.quick.rotateTitle, () => events.onRotateSelected(7.5), "tb-rotate");
  const btnDelete = createIconButton("🗑", t.quick.deleteTitle, events.onDeleteSelected, "tb-delete", "btn-danger");

  quickGroup.appendChild(btnUndo);
  quickGroup.appendChild(btnRedo);
  quickGroup.appendChild(btnRotate);
  quickGroup.appendChild(btnDelete);
  container.appendChild(quickGroup);

  container.appendChild(createDivider());

  // 4. Feature Toggles: Snapping, Color Coding, Track Numbers, Voltage Drop
  const featureGroup = document.createElement("div");
  featureGroup.className = "toolbar-group";

  const btnSnap = createToggleBtn("🧲", t.toggles.snapLabel, t.toggles.snapLabel, true, events.onToggleSnapping, "tb-snap");
  btnSnap.title = t.toggles.snapTitle;

  const btnMeasure = createToggleBtn("📏", t.toggles.measureLabel, t.toggles.measureLabel, false, () => events.onToggleMeasureTool?.(), "tb-measure");
  btnMeasure.title = t.toggles.measureTitle;

  // Split-Button für Colors: Modus umschalten per Klick oder Direktwahl im Menü
  const colorsWrapper = document.createElement("div");
  colorsWrapper.className = "toolbar-split-group";

  const btnColors = createToggleBtn("", t.toggles.colorsLabel, t.toggles.colorsLabel, true, events.onToggleColorCode, "tb-colors");
  btnColors.title = t.toggles.colorsTitle;
  colorsWrapper.appendChild(btnColors);

  const colorsMenu = createDropdownMenu("", [
    { label: t.colorsMenu.types, onClick: () => events.onSetColorMode?.("types"), id: "split-color-types" },
    { label: t.colorsMenu.layers, onClick: () => events.onSetColorMode?.("layers"), id: "split-color-layers" },
    { label: t.colorsMenu.off, onClick: () => events.onSetColorMode?.("off"), id: "split-color-off" },
  ]);
  colorsMenu.classList.add("colors-split-dropdown");
  colorsWrapper.appendChild(colorsMenu);

  const btnLabels = createToggleBtn("🏷", t.toggles.labelsLabel, t.toggles.labelsLabel, true, events.onToggleTrackLabels, "tb-labels");
  btnLabels.title = t.toggles.labelsTitle;

  // Split-Button für Voltage Drop + Versorgungsspannung Kontext-Menü
  const powerWrapper = document.createElement("div");
  powerWrapper.className = "toolbar-split-group";

  const btnPower = createToggleBtn("⚡", t.toggles.voltageLabel, t.toggles.voltageLabel, false, events.onTogglePowerSim, "tb-power");
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

  const btn3D = createToggleBtn("🧊", "3D (Alpha)", "3D", false, () => events.onToggle3D?.(), "tb-3d");
  btn3D.title = t.toggles.toggle3dTitle;

  featureGroup.appendChild(btnSnap);
  featureGroup.appendChild(btnMeasure);
  featureGroup.appendChild(colorsWrapper);
  featureGroup.appendChild(btnLabels);
  featureGroup.appendChild(powerWrapper);

  const btnShortCircuit = document.createElement("button");
  btnShortCircuit.id = "tb-short-circuit";
  btnShortCircuit.className = "toolbar-btn tb-short-circuit-btn";
  btnShortCircuit.style.display = "none";
  btnShortCircuit.title = t.status.shortCircuitTooltip;
  btnShortCircuit.innerHTML = `
    <span class="tb-short-circuit-icon">⚠️⚡</span>
    <span class="tb-short-circuit-label">${t.status.shortCircuitButton}</span>
  `;
  btnShortCircuit.addEventListener("click", () => {
    events.onFocusShortCircuit?.();
  });
  featureGroup.appendChild(btnShortCircuit);

  featureGroup.appendChild(btn3D);
  container.appendChild(featureGroup);

  // 5. Right Area: File Info & Count, Language Switcher & Right Sidebar Toggle
  const rightGroup = document.createElement("div");
  rightGroup.className = "toolbar-group toolbar-right";

  const fileLabel = document.createElement("span");
  fileLabel.id = "tb-file-name";
  fileLabel.className = "brand-file";
  fileLabel.title = "Current layout plan";
  fileLabel.textContent = "Model Railway.gleisplan";

  const countBadge = document.createElement("span");
  countBadge.id = "tb-piece-count";
  countBadge.className = "brand-badge";
  countBadge.textContent = `0 ${t.status.tracksUnitPlural}`;

  // 1-Click Language Switch Button
  const currentLang = getLanguage();
  const btnLang = document.createElement("button");
  btnLang.id = "tb-lang-toggle";
  btnLang.className = "toolbar-btn tb-lang-btn";
  btnLang.title = t.status.langSwitchTitle;
  btnLang.innerHTML = `<span style="font-weight: 700; font-size: 11px;">${currentLang.toUpperCase()}</span>`;
  btnLang.addEventListener("click", () => {
    toggleLanguage();
  });

  // 1-Click Theme Switcher Button (Auto / Dark / Light)
  const btnTheme = document.createElement("button");
  btnTheme.id = "tb-theme-toggle";
  btnTheme.className = "toolbar-btn tb-theme-btn";
  btnTheme.title = t.viewMenu.themeTooltip;
  btnTheme.innerHTML = `<span class="tb-theme-label" style="font-size: 11px; font-weight: 600;"></span>`;
  btnTheme.addEventListener("click", () => {
    events.onToggleTheme();
  });

  // Collaboration Button
  const btnCollab = document.createElement("button");
  btnCollab.id = "tb-collab-btn";
  btnCollab.className = "toolbar-btn tb-collab-btn";
  btnCollab.title = t.status.collabTooltip;
  btnCollab.innerHTML = `<span class="tb-collab-label" style="font-size: 11px; font-weight: 600;">${t.status.collabButton}</span>`;
  btnCollab.addEventListener("click", () => {
    events.onOpenCollab?.();
  });

  const btnRightSidebar = createIconButton("🏗️", t.status.toggleRightSidebar, () => events.onToggleRightSidebar?.(), "tb-right-sidebar-toggle", "toolbar-toggle active");

  rightGroup.appendChild(fileLabel);
  rightGroup.appendChild(countBadge);
  rightGroup.appendChild(btnCollab);
  rightGroup.appendChild(btnTheme);
  rightGroup.appendChild(btnLang);
  rightGroup.appendChild(btnRightSidebar);
  container.appendChild(rightGroup);

  host.appendChild(container);

  // Close dropdowns on outside click or escape
  setupDropdownGlobalDismiss();
}

export function updateToolbarState(state: ToolbarState) {
  const t = getTranslations().toolbar;

  const btnSnap = document.getElementById("tb-snap");
  if (btnSnap) {
    setBtnLabel(btnSnap, "", t.toggles.snapLabel, t.toggles.snapLabel);
    if (state.snappingEnabled) {
      btnSnap.classList.add("active");
    } else {
      btnSnap.classList.remove("active");
    }
  }

  const btnMeasure = document.getElementById("tb-measure");
  if (btnMeasure) {
    setBtnLabel(btnMeasure, "📏", t.toggles.measureLabel, t.toggles.measureLabel);
    if (state.measureToolActive) {
      btnMeasure.classList.add("active");
    } else {
      btnMeasure.classList.remove("active");
    }
  }

  const btnColors = document.getElementById("tb-colors");
  if (btnColors) {
    const mode = state.colorMode ?? (state.colorCodeEnabled ? "types" : "off");
    setBtnLabel(btnColors, "", t.toggles.colorsLabel, t.toggles.colorsLabel);
    if (mode !== "off") {
      btnColors.classList.add("active");
    } else {
      btnColors.classList.remove("active");
    }
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

    updateCheck("view-colors-types", mode === "types", t.viewMenu.colorsTypes);
    updateCheck("view-colors-layers", mode === "layers", t.viewMenu.colorsLayers);
    updateCheck("view-colors-off", mode === "off", t.viewMenu.colorsOff);
  }

  const btnLabels = document.getElementById("tb-labels");
  if (btnLabels) {
    setBtnLabel(btnLabels, "🏷", t.toggles.labelsLabel, t.toggles.labelsLabel);
    if (state.trackLabelsEnabled) {
      btnLabels.classList.add("active");
    } else {
      btnLabels.classList.remove("active");
    }
  }

  const btnPower = document.getElementById("tb-power");
  if (btnPower) {
    setBtnLabel(btnPower, "⚡", t.toggles.voltageLabel, t.toggles.voltageLabel);
    if (state.powerSimEnabled) {
      btnPower.classList.add("active-warning");
    } else {
      btnPower.classList.remove("active-warning");
    }
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

  const btnRotate = document.getElementById("tb-rotate") as HTMLButtonElement | null;
  if (btnRotate) btnRotate.disabled = !state.hasSelection;

  const btnDelete = document.getElementById("tb-delete") as HTMLButtonElement | null;
  if (btnDelete) btnDelete.disabled = !state.hasSelection;

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

  const btnTheme = document.getElementById("tb-theme-toggle");
  if (btnTheme) {
    const pref = state.themePref || "system";
    let label = t.viewMenu.themeStatusSystem;
    if (pref === "dark") {
      label = t.viewMenu.themeStatusDark;
    } else if (pref === "light") {
      label = t.viewMenu.themeStatusLight;
    } else {
      label = t.viewMenu.themeStatusSystem;
    }
    btnTheme.innerHTML = `<span class="tb-theme-label" style="font-size: 11px; font-weight: 600;">${label}</span>`;
    const resolvedLabel = state.resolvedTheme === "dark" ? t.viewMenu.themeStatusDark : t.viewMenu.themeStatusLight;
    btnTheme.title = pref === "system"
      ? `${t.viewMenu.themeTooltip} (${t.viewMenu.themeStatusSystem}: ${resolvedLabel})`
      : `${t.viewMenu.themeTooltip} (${label})`;
  }

  const itemThemeSys = document.getElementById("item-theme-system");
  if (itemThemeSys && state.themePref !== undefined) {
    const isSys = state.themePref === "system";
    const check = isSys ? "✓ " : "";
    const leftSpan = itemThemeSys.querySelector(".dropdown-item-left span:last-child");
    if (leftSpan) leftSpan.textContent = `${check}${t.viewMenu.themeSystem}`;
  }

  const itemThemeDark = document.getElementById("item-theme-dark");
  if (itemThemeDark && state.themePref !== undefined) {
    const isDark = state.themePref === "dark";
    const check = isDark ? "✓ " : "";
    const leftSpan = itemThemeDark.querySelector(".dropdown-item-left span:last-child");
    if (leftSpan) leftSpan.textContent = `${check}${t.viewMenu.themeDark}`;
  }

  const itemThemeLight = document.getElementById("item-theme-light");
  if (itemThemeLight && state.themePref !== undefined) {
    const isLight = state.themePref === "light";
    const check = isLight ? "✓ " : "";
    const leftSpan = itemThemeLight.querySelector(".dropdown-item-left span:last-child");
    if (leftSpan) leftSpan.textContent = `${check}${t.viewMenu.themeLight}`;
  }

  const btnCollab = document.getElementById("tb-collab-btn");
  if (btnCollab && state.collabStatus !== undefined) {
    if (state.collabStatus === "connected") {
      const count = (state.collabUsersCount || 0) + 1;
      btnCollab.innerHTML = `<span style="font-size: 10px; color: #10b981;">🟢</span> <span class="tb-collab-label" style="font-size: 11px; font-weight: 700; color: #10b981;">${t.status.collabOnline.replace("{count}", String(count))}</span>`;
      btnCollab.classList.add("active");
    } else if (state.collabStatus === "connecting") {
      btnCollab.innerHTML = `<span style="font-size: 10px; color: #f59e0b;">🟡</span> <span class="tb-collab-label" style="font-size: 11px; font-weight: 600;">${t.status.collabConnecting}</span>`;
      btnCollab.classList.remove("active");
    } else {
      btnCollab.innerHTML = `<span class="tb-collab-label" style="font-size: 11px; font-weight: 600;">${t.status.collabButton}</span>`;
      btnCollab.classList.remove("active");
    }
  }

  const fileLabel = document.getElementById("tb-file-name");
  if (fileLabel && state.activeFileName) {
    const dirty = state.isDirty ? " ●" : "";
    fileLabel.textContent = `${state.activeFileName}${dirty}`;
    if (state.isDirty) {
      fileLabel.classList.add("dirty");
    } else {
      fileLabel.classList.remove("dirty");
    }
  }

  const btnRightSidebar = document.getElementById("tb-right-sidebar-toggle");
  if (btnRightSidebar && state.rightSidebarCollapsed !== undefined) {
    if (state.rightSidebarCollapsed) {
      btnRightSidebar.classList.remove("active");
      btnRightSidebar.title = "Show Floating Layers Menu (Shift+B)";
    } else {
      btnRightSidebar.classList.add("active");
      btnRightSidebar.title = "Hide Floating Layers Menu (Shift+B)";
    }
  }

  const itemRulers = document.getElementById("item-rulers");
  if (itemRulers && state.rulersEnabled !== undefined) {
    const check = state.rulersEnabled ? "✓ " : "";
    const leftSpan = itemRulers.querySelector(".dropdown-item-left span:last-child");
    if (leftSpan) leftSpan.textContent = `${check}Toggle Rulers`;
  }

  const itemBaseboard = document.getElementById("item-baseboard");
  if (itemBaseboard && state.baseboardEnabled !== undefined) {
    const check = state.baseboardEnabled ? "✓ " : "";
    const leftSpan = itemBaseboard.querySelector(".dropdown-item-left span:last-child");
    if (leftSpan) leftSpan.textContent = `${check}Toggle Baseboard Table`;
  }

  const btn3D = document.getElementById("tb-3d");
  if (btn3D && state.is3DActive !== undefined) {
    if (state.is3DActive) {
      btn3D.classList.add("active");
      setBtnLabel(btn3D, "📐", "2D Plan", "2D");
      btn3D.title = "3 / V: Return to 2D track planning mode";
    } else {
      btn3D.classList.remove("active");
      setBtnLabel(btn3D, "🧊", "3D (Alpha)", "3D");
      btn3D.title = "3 / V: Switch to realistic 3D perspective with layers & bridges (Alpha)";
    }
  }

  const item3D = document.getElementById("item-3d");
  if (item3D && state.is3DActive !== undefined) {
    const leftSpan = item3D.querySelector(".dropdown-item-left span:last-child");
    if (leftSpan) leftSpan.textContent = state.is3DActive ? "Switch to 2D Plan" : "Toggle 3D View (Alpha)";
  }
}

interface DropdownItemConfig {
  label?: string;
  shortcut?: string;
  icon?: string;
  onClick?: () => void;
  id?: string;
  isDivider?: boolean;
  isDanger?: boolean;
}

function createDropdownMenu(triggerLabel: string, items: DropdownItemConfig[]): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = "toolbar-dropdown";

  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "toolbar-btn dropdown-trigger";
  trigger.innerHTML = `${triggerLabel} <span class="dropdown-arrow">▾</span>`;

  trigger.addEventListener("pointerdown", (e) => {
    e.stopPropagation();
  });

  trigger.addEventListener("click", (e) => {
    e.stopPropagation();
    const isOpen = wrapper.classList.contains("open");
    // Close any other open dropdowns first
    document.querySelectorAll(".toolbar-dropdown.open").forEach((d) => d.classList.remove("open"));
    if (!isOpen) {
      wrapper.classList.add("open");
    }
  });

  trigger.addEventListener("mouseenter", () => {
    // Wenn bereits ein anderes Menü geöffnet ist, wechsle flüssig zu diesem
    const anyOpen = document.querySelector(".toolbar-dropdown.open");
    if (anyOpen && anyOpen !== wrapper) {
      anyOpen.classList.remove("open");
      wrapper.classList.add("open");
    }
  });

  wrapper.appendChild(trigger);

  const menu = document.createElement("div");
  menu.className = "dropdown-menu";

  for (const item of items) {
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
      wrapper.classList.remove("open");
      item.onClick?.();
    });

    menu.appendChild(btn);
  }

  wrapper.appendChild(menu);
  return wrapper;
}

function setBtnLabel(btn: HTMLElement, _icon: string, fullLabel: string, compactLabel?: string) {
  const compact = compactLabel !== undefined ? compactLabel : fullLabel;
  btn.innerHTML = `<span class="tb-text-full">${fullLabel}</span><span class="tb-text-compact">${compact}</span>`;
}

function createIconButton(icon: string, title: string, onClick: () => void, id?: string, extraClass = ""): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.className = `toolbar-btn btn-icon-only ${extraClass}`.trim();
  btn.title = title;
  if (id) btn.id = id;
  btn.innerHTML = `<span class="tb-icon">${icon}</span>`;
  btn.addEventListener("click", onClick);
  return btn;
}

function createToggleBtn(
  icon: string,
  fullLabel: string,
  compactLabel: string,
  initialActive: boolean,
  onClick: () => void,
  id: string,
): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.className = `toolbar-btn toolbar-toggle ${initialActive ? "active" : ""}`;
  btn.id = id;
  setBtnLabel(btn, icon, fullLabel, compactLabel);
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
