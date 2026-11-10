import { catalogRegistry, type Catalog, type CatalogEntry } from "@/catalog/catalog";
import type { Layout, Layer, BaseboardConfig, Model3D, ModelCategory } from "@/core/types";
import { openLayerDialog } from "./layer-dialog";
import type { SimulationSummary } from "@/core/simulation";
import {
  ensureDefaultLayers,
  sortLayers,
  ensureBaseboard,
  getBaseboards,
  addBaseboard,
  removeBaseboard,
  duplicateBaseboard,
  getBaseboardBounds,
} from "@/core/layout";
import {
  getTranslations,
  onLanguageChange,
  getLanguage,
  getTrackDisplayName,
  getTrackDisplayCategory,
  getTrackDisplayDescription,
} from "@/i18n";
import { getPieceListSummary } from "@/io/export";
import { getTrackBlueprintColor } from "@/render/blueprint-colors";
import {
  BUILTIN_MODELS,
  createBuiltinModel,
  parseGlbOrGltfFile,
  type BuiltinModelDef,
} from "@/core/models-3d";
import { MODEL_SCALE_RATIOS, getScaleFactor, scaleDimensions, type ModelScale } from "@/core/model-scale";
import {
  createPieceDragCanvas,
  createModelDragCanvas,
  createCustomModelDragCanvas,
  attachDragImage,
} from "./drag-preview";

export type { ModelScale };
export type SidebarTab = "catalog" | "roads" | "models" | "board" | "bom" | "layers";

export interface SidebarCallbacks {
  onAddPiece: (entry: CatalogEntry) => void;
  onAddModel?: (model: Model3D) => void;
  onExportCSV: () => void;
  onImportCatalog?: () => void;
  onSelectSystem?: (system: string) => void;
  onSelectRoadScale?: (scale: ModelScale) => void;
  onLayerChange?: () => void;
  onMoveSelectedToLayer?: (layerId: string) => void;
  onToggleSelectionLock?: () => void;
  isSelectionLockDisabled?: () => boolean;
  selectedPieceIds?: () => string[];
  onBaseboardChange?: () => void;
  getZoom?: () => number;
  onTabChange?: (tab: SidebarTab) => void;
}

const CATEGORY_MAP: Record<string, string[]> = {
  All: ["All", "Alle"],
  Straights: ["Straights", "Geraden"],
  Curves: ["Curves", "Bögen"],
  Turnouts: ["Turnouts", "Weichen"],
  Crossings: ["Crossings", "Kreuzungen"],
  Accessories: ["Accessories", "Zubehör"],
};

const AVAILABLE_SCALES: ModelScale[] = ["H0", "TT", "N", "Z"];

const SCALE_DESCRIPTIONS: Record<ModelScale, { de: string; en: string }> = {
  H0: { de: "1:87 (Fahrbahn ~60 mm)", en: "1:87 (Roadway ~60 mm)" },
  TT: { de: "1:120 (Fahrbahn ~45 mm)", en: "1:120 (Roadway ~45 mm)" },
  N: { de: "1:160 (Fahrbahn ~35 mm)", en: "1:160 (Roadway ~35 mm)" },
  Z: { de: "1:220 (Fahrbahn ~25 mm)", en: "1:220 (Roadway ~25 mm)" },
};

/** Roadway width per scale in mm (full 2-lane width). */
const ROAD_WIDTH_MM: Record<ModelScale, number> = { H0: 60, TT: 45, N: 35, Z: 25 };

function getRoadWidthMm(scale?: string): number {
  return ROAD_WIDTH_MM[((scale || "H0").toUpperCase() as ModelScale)] ?? 60;
}

const MODEL_CATEGORIES = ["All", "Buildings", "Railway", "Nature", "Custom"];

const MODEL_CATEGORY_LABELS_DE: Record<string, string> = {
  All: "Alle",
  Buildings: "Gebäude",
  Railway: "Bahn",
  Nature: "Natur",
  Custom: "Eigene",
};

const SIDEBAR_DRILLDOWN_STYLES_ID = "sidebar-drilldown-styles";
const SIDEBAR_DRILLDOWN_STYLES = `
:root {
  --popover-bg: var(--cl-bg-glass, rgba(30, 41, 59, 0.96));
  --popover-border: var(--cl-edge, rgba(255, 255, 255, 0.15));
  --popover-header-bg: var(--cl-header-bg, rgba(0, 0, 0, 0.15));
  --popover-header-border: var(--cl-edge, rgba(255, 255, 255, 0.1));
  --popover-header-text: var(--text-muted, rgba(255, 255, 255, 0.7));
  --popover-hover-bg: var(--cl-glass-hover, rgba(255, 255, 255, 0.08));
  --popover-shadow: 0 10px 25px rgba(0, 0, 0, 0.35);
}

:root[data-theme="light"],
[data-theme="light"] {
  --popover-bg: var(--cl-bg-glass, rgba(255, 255, 255, 0.96));
  --popover-border: var(--cl-edge, rgba(0, 0, 0, 0.12));
  --popover-header-bg: var(--cl-header-bg, rgba(0, 0, 0, 0.04));
  --popover-header-border: var(--cl-edge, rgba(0, 0, 0, 0.08));
  --popover-header-text: var(--text-muted, #64748b);
  --popover-hover-bg: var(--cl-glass-hover, rgba(0, 0, 0, 0.05));
  --popover-shadow: 0 10px 25px rgba(0, 0, 0, 0.12);
}

@media (prefers-color-scheme: light) {
  :root:not([data-theme="dark"]) {
    --popover-bg: var(--cl-bg-glass, rgba(255, 255, 255, 0.96));
    --popover-border: var(--cl-edge, rgba(0, 0, 0, 0.12));
    --popover-header-bg: var(--cl-header-bg, rgba(0, 0, 0, 0.04));
    --popover-header-border: var(--cl-edge, rgba(0, 0, 0, 0.08));
    --popover-header-text: var(--text-muted, #64748b);
    --popover-hover-bg: var(--cl-glass-hover, rgba(0, 0, 0, 0.05));
    --popover-shadow: 0 10px 25px rgba(0, 0, 0, 0.12);
  }
}

.two-step-popover {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  margin-top: 4px;
  background: var(--popover-bg);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid var(--popover-border);
  border-radius: 8px;
  box-shadow: var(--popover-shadow);
  color: var(--text-main, inherit);
  z-index: 100;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.popover-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--popover-header-text);
  border-bottom: 1px solid var(--popover-header-border);
  background: var(--popover-header-bg);
}

.popover-back-btn {
  background: transparent;
  border: 0;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--cl-accent, #0284c7);
  font-size: 14px;
  line-height: 1;
  font-weight: bold;
}
.popover-back-btn:hover {
  background: var(--popover-hover-bg);
}

.popover-list {
  display: flex;
  flex-direction: column;
  max-height: 400px;
  overflow-y: auto;
  padding: 4px;
  gap: 2px;
}

.popover-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  border-radius: 6px;
  background: transparent;
  border: 0;
  color: var(--text-main, inherit);
  font: inherit;
  font-size: 11px;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}

.popover-item:hover {
  background: var(--popover-hover-bg);
}

.popover-item.active {
  background: var(--cl-accent-soft, rgba(2, 132, 199, 0.15));
  color: var(--cl-accent, #0284c7);
  font-weight: 600;
}

.popover-item-sub {
  font-size: 9.5px;
  color: var(--text-muted, rgba(148, 163, 184, 0.85));
}

.models-dropzone {
  cursor: pointer;
}
.models-dropzone:hover,
.models-dropzone:focus-visible {
  border-color: var(--cl-accent, #0284c7);
  outline: none;
}
`;

/**
 * Finds the road catalog for a scale. Matches the catalog's `scale` field (case-insensitive),
 * and falls back to the id convention `generic-streets-<scale>`.
 */
function findRoadCatalogForScale(scale: string): Catalog | undefined {
  const sc = scale.toUpperCase();
  const roads = catalogRegistry.getAll().filter((c) => c.isRoadSystem);
  return (
    roads.find((c) => c.id.toLowerCase() === `generic-streets-${sc.toLowerCase()}`) ??
    roads.find((c) => (c.scale || "").toUpperCase() === sc) ??
    roads.find((c) => c.id.toLowerCase().endsWith(`-${sc.toLowerCase()}`))
  );
}

function ensureSidebarDrilldownStyles() {
  if (document.getElementById(SIDEBAR_DRILLDOWN_STYLES_ID)) return;
  const styleEl = document.createElement("style");
  styleEl.id = SIDEBAR_DRILLDOWN_STYLES_ID;
  styleEl.textContent = SIDEBAR_DRILLDOWN_STYLES;
  document.head.appendChild(styleEl);
}

function formatCatalogTitle(cat: Catalog): string {
  if (cat.isRoadSystem) {
    return cat.name || `${cat.manufacturer} ${cat.system}`;
  }
  return `${cat.manufacturer} ${cat.system} ${cat.scale}`;
}

function formatCatalogSubtitle(cat: Catalog, isDe: boolean, t: any): string {
  if (cat.isRoadSystem) {
    const width = getRoadWidthMm(cat.scale);
    return isDe ? `Fahrbahnbreite ${width} mm, 2-spurig` : `Road width ${width} mm, 2-lane`;
  }
  const gauge = cat.gauge ?? 16.5;
  const roadbedText =
    cat.hasRoadbed === false
      ? isDe
        ? "ohne Bettung"
        : "without roadbed"
      : isDe
      ? "mit Bettung"
      : "roadbed";

  let conductorsText = "";
  if (cat.conductors === 3) {
    conductorsText = t.catalog.threeConductors;
  } else if (cat.conductors === 2) {
    conductorsText = t.catalog.twoConductors;
  } else if (cat.conductors) {
    conductorsText = isDe ? `${cat.conductors} Leiter` : `${cat.conductors}-conductor`;
  }

  return conductorsText ? `${gauge} mm, ${roadbedText}, ${conductorsText}` : `${gauge} mm, ${roadbedText}`;
}



export class Sidebar {
  private activeTab: SidebarTab = "catalog";
  private searchQuery = "";
  private selectedCategory = "All";
  private roadSearchQuery = "";
  private selectedRoadCategory = "All";
  private roadCatalog: Catalog;
  private currentRoadScale: ModelScale = "H0";
  private modelsSearchQuery = "";
  private selectedModelCategory = "All";
  private currentModelScale: ModelScale = "H0";
  private customModelsLibrary: Model3D[] = [];
  private activeBoardId?: string;
  private activePartsCategory: "all" | "tracks" | "roads" | "buildings" = "all";
  private host: HTMLElement;
  private catalog: Catalog;
  private layout: Layout;
  private simulation: SimulationSummary | null = null;
  private callbacks: SidebarCallbacks;
  /** Window listeners registered by the current render pass; removed at the start of the next one. */
  private windowCleanups: Array<() => void> = [];

  constructor(
    host: HTMLElement,
    catalog: Catalog,
    layout: Layout,
    callbacks: SidebarCallbacks,
  ) {
    this.host = host;
    ensureSidebarDrilldownStyles();

    this.catalog = catalog.isRoadSystem
      ? catalogRegistry.getAll().find((c) => !c.isRoadSystem) ?? catalogRegistry.getDefault()
      : catalog;

    this.roadCatalog = catalog.isRoadSystem
      ? catalog
      : findRoadCatalogForScale(layout.scale || "H0") ??
        findRoadCatalogForScale("H0") ??
        catalogRegistry.getAll().find((c) => c.isRoadSystem) ??
        catalog;

    this.currentRoadScale = (layout.scale as ModelScale) || "H0";
    this.currentModelScale = (layout.scale as ModelScale) || "H0";
    this.layout = layout;
    this.callbacks = callbacks;

    ensureDefaultLayers(this.layout);
    onLanguageChange(() => this.render());
    this.render();
  }

  update(layout: Layout, simulation?: SimulationSummary) {
    this.layout = layout;
    ensureDefaultLayers(this.layout);
    if (simulation !== undefined) {
      this.simulation = simulation;
    }
    this.render();
  }

  setCatalog(catalog: Catalog) {
    if (catalog.isRoadSystem) {
      this.roadCatalog = catalog;
      this.currentRoadScale = ((catalog.scale || "H0").toUpperCase() as ModelScale);
      this.activeTab = "roads";
    } else {
      this.catalog = catalog;
      this.activeTab = "catalog";
    }
    this.render();
  }

  setRoadCatalog(catalog: Catalog) {
    this.roadCatalog = catalog;
    this.currentRoadScale = ((catalog.scale || "H0").toUpperCase() as ModelScale);
    this.render();
  }

  /** Registers a window click listener that is automatically removed on the next render(). */
  private addWindowClickListener(handler: (e: MouseEvent) => void) {
    window.addEventListener("click", handler);
    this.windowCleanups.push(() => window.removeEventListener("click", handler));
  }

  private render() {
    for (const cleanup of this.windowCleanups) cleanup();
    this.windowCleanups = [];

    this.host.innerHTML = "";
    ensureDefaultLayers(this.layout);

    // 1. Tab Navigation
    const tabHeader = document.createElement("div");
    tabHeader.className = "sidebar-tabs";

    if (this.activeTab === "layers") {
      this.activeTab = "catalog";
    }

    const bomSummary = getPieceListSummary(this.layout, this.catalog);
    const t = getTranslations().sidebar;

    tabHeader.appendChild(
      this.createTabButton("catalog", t.tabs.tracks, `${t.tabs.tracks} (${this.catalog.entries.length})`),
    );
    tabHeader.appendChild(
      this.createTabButton("roads", t.tabs.roads, `${t.tabs.roads} (${this.roadCatalog.entries.length})`),
    );
    tabHeader.appendChild(
      this.createTabButton(
        "models",
        t.tabs.models,
        `${t.tabs.models} (${BUILTIN_MODELS.length + this.customModelsLibrary.length})`,
      ),
    );
    tabHeader.appendChild(this.createTabButton("board", t.tabs.board, t.board.title));
    tabHeader.appendChild(this.createTabButton("bom", t.tabs.bom, `${t.bom.title} (${bomSummary.totalObjects})`));
    this.host.appendChild(tabHeader);

    // 2. Tab Content Container
    const content = document.createElement("div");
    content.className = "sidebar-content";

    if (this.activeTab === "catalog") {
      this.renderCatalogTab(content);
    } else if (this.activeTab === "roads") {
      this.renderRoadsTab(content);
    } else if (this.activeTab === "models") {
      this.renderModelsTab(content);
    } else if (this.activeTab === "board") {
      this.renderBoardTab(content);
    } else if (this.activeTab === "bom") {
      this.renderBomTab(content, bomSummary);
    } else if (this.activeTab === "layers") {
      this.renderLayersTab(content);
    }

    this.host.appendChild(content);
  }

  setActiveTab(tab: SidebarTab) {
    this.activeTab = tab;
    this.callbacks.onTabChange?.(tab);
    this.render();
  }

  getActiveTab(): SidebarTab {
    return this.activeTab;
  }

  getCatalog(): Catalog {
    return this.catalog;
  }

  getActiveBoardId(): string | undefined {
    return this.activeBoardId;
  }

  setActiveBoardId(id?: string) {
    this.activeBoardId = id;
    this.render();
  }

  isBaseboardTabActive(): boolean {
    return this.activeTab === "board";
  }

  private createTabButton(tab: SidebarTab, label: string, tooltip: string): HTMLButtonElement {
    const btn = document.createElement("button");
    btn.className = `tab-btn ${this.activeTab === tab ? "active" : ""}`;
    btn.title = tooltip;
    btn.innerHTML = `<span class="tab-label">${label}</span>`;
    btn.addEventListener("click", () => {
      this.activeTab = tab;
      this.callbacks.onTabChange?.(tab);
      this.render();
    });
    return btn;
  }

  // =========================================================================
  // TAB 1: TRACK CATALOG (2-STEP MENU: MANUFACTURER -> TRACK SYSTEM)
  // =========================================================================
  private renderCatalogTab(container: HTMLElement) {
    const t = getTranslations().sidebar;
    const isDe = getLanguage() === "de";

    const allCatalogs = catalogRegistry.getAll().filter((c) => !c.isRoadSystem);
    if (this.catalog.isRoadSystem) {
      this.catalog = allCatalogs[0] ?? catalogRegistry.getDefault();
    }

    let menuStep: "closed" | "manufacturers" | "systems" = "closed";
    let activeMfg = this.catalog.manufacturer;

    const selectorBox = document.createElement("div");
    selectorBox.className = "catalog-system-box";
    selectorBox.style.position = "relative";

    const trigger = document.createElement("button");
    trigger.type = "button";
    trigger.className = "catalog-dropdown-trigger";
    trigger.innerHTML = `
      <div class="catalog-trigger-text">
        <span class="catalog-trigger-title">${this.catalog.manufacturer} — ${this.catalog.system}</span>
        <span class="catalog-trigger-sub">${formatCatalogSubtitle(this.catalog, isDe, t)}</span>
      </div>
      <span class="catalog-dropdown-arrow">▾</span>
    `;

    const popover = document.createElement("div");
    popover.className = "two-step-popover";
    popover.style.display = "none";

    const updatePopover = () => {
      if (menuStep === "closed") {
        popover.style.display = "none";
        return;
      }
      popover.style.display = "flex";
      popover.innerHTML = "";

      if (menuStep === "manufacturers") {
        const header = document.createElement("div");
        header.className = "popover-header";
        header.textContent = isDe ? "1. Hersteller wählen" : "1. Select Manufacturer";
        popover.appendChild(header);

        const list = document.createElement("div");
        list.className = "popover-list";

        const manufacturers = Array.from(new Set(allCatalogs.map((c) => c.manufacturer)));
        for (const mfg of manufacturers) {
          const item = document.createElement("button");
          item.type = "button";
          item.className = `popover-item ${mfg === this.catalog.manufacturer ? "active" : ""}`;
          item.innerHTML = `<span>${mfg}</span><span>›</span>`;
          item.addEventListener("click", (e) => {
            e.stopPropagation();
            activeMfg = mfg;
            menuStep = "systems";
            updatePopover();
          });
          list.appendChild(item);
        }

        const importItem = document.createElement("button");
        importItem.type = "button";
        importItem.className = "popover-item";
        importItem.style.color = "var(--theme-accent, #38bdf8)";
        importItem.style.fontWeight = "600";
        importItem.style.borderTop = "1px solid rgba(255, 255, 255, 0.08)";
        importItem.innerHTML = `<span>+ ${isDe ? "Katalog importieren (JSON)..." : "Import Catalog (JSON)..."}</span>`;
        importItem.addEventListener("click", (e) => {
          e.stopPropagation();
          menuStep = "closed";
          updatePopover();
          this.callbacks.onImportCatalog?.();
        });
        list.appendChild(importItem);

        popover.appendChild(list);
      } else if (menuStep === "systems") {
        const header = document.createElement("div");
        header.className = "popover-header";

        const backBtn = document.createElement("button");
        backBtn.type = "button";
        backBtn.className = "popover-back-btn";
        backBtn.textContent = "‹";
        backBtn.title = isDe ? "Zurück zu Herstellern" : "Back to Manufacturers";
        backBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          menuStep = "manufacturers";
          updatePopover();
        });

        const title = document.createElement("span");
        title.textContent = `${activeMfg} Systems`;

        header.append(backBtn, title);
        popover.appendChild(header);

        const list = document.createElement("div");
        list.className = "popover-list";

        const systems = allCatalogs.filter((c) => c.manufacturer === activeMfg);
        for (const cat of systems) {
          const item = document.createElement("button");
          item.type = "button";
          const isSelected = cat.id === this.catalog.id;
          item.className = `popover-item ${isSelected ? "active" : ""}`;
          item.innerHTML = `
            <div>
              <div style="font-weight: 600;">${cat.system}</div>
              <div class="popover-item-sub">${cat.scale} • ${cat.gauge ?? 16.5} mm</div>
            </div>
            ${isSelected ? `<span>✓</span>` : ""}
          `;
          item.addEventListener("click", (e) => {
            e.stopPropagation();
            this.catalog = cat;
            this.callbacks.onSelectSystem?.(cat.id);
            menuStep = "closed";
            this.render();
          });
          list.appendChild(item);
        }

        const importItem = document.createElement("button");
        importItem.type = "button";
        importItem.className = "popover-item";
        importItem.style.color = "var(--theme-accent, #38bdf8)";
        importItem.style.fontWeight = "600";
        importItem.style.borderTop = "1px solid rgba(255, 255, 255, 0.08)";
        importItem.innerHTML = `<span>+ ${isDe ? "Katalog importieren (JSON)..." : "Import Catalog (JSON)..."}</span>`;
        importItem.addEventListener("click", (e) => {
          e.stopPropagation();
          menuStep = "closed";
          updatePopover();
          this.callbacks.onImportCatalog?.();
        });
        list.appendChild(importItem);

        popover.appendChild(list);
      }
    };

    trigger.addEventListener("click", (e) => {
      e.stopPropagation();
      menuStep = menuStep === "closed" ? "manufacturers" : "closed";
      updatePopover();
    });

    this.addWindowClickListener((e) => {
      if (!selectorBox.contains(e.target as Node) && menuStep !== "closed") {
        menuStep = "closed";
        updatePopover();
      }
    });

    selectorBox.append(trigger, popover);
    container.appendChild(selectorBox);

    // Search Box
    const searchBox = document.createElement("div");
    searchBox.className = "search-box";
    const input = document.createElement("input");
    input.type = "text";
    input.placeholder = t.catalog.searchPlaceholder;
    input.value = this.searchQuery;
    input.addEventListener("input", (e) => {
      this.searchQuery = (e.target as HTMLInputElement).value;
      this.renderTrackList(listContainer);
    });
    searchBox.appendChild(input);
    container.appendChild(searchBox);

    // Category Filter Bar
    const categories: Array<{ id: string; label: string }> = [
      { id: "All", label: t.catalog.categories.all },
      { id: "Straights", label: t.catalog.categories.straights },
      { id: "Curves", label: t.catalog.categories.curves },
      { id: "Turnouts", label: t.catalog.categories.turnouts },
      { id: "Crossings", label: t.catalog.categories.crossings },
      { id: "Accessories", label: t.catalog.categories.accessories },
    ];
    const filterBar = document.createElement("div");
    filterBar.className = "category-filter-bar";
    for (const cat of categories) {
      const pill = document.createElement("button");
      pill.className = `filter-pill ${this.selectedCategory === cat.id ? "active" : ""}`;
      pill.textContent = cat.label;
      pill.addEventListener("click", () => {
        this.selectedCategory = cat.id;
        this.render();
      });
      filterBar.appendChild(pill);
    }
    container.appendChild(filterBar);

    // Drag & Drop Hint
    const hint = document.createElement("div");
    hint.className = "drag-hint";
    hint.innerHTML = isDe
      ? " <em>Gleis auf die Arbeitsfläche ziehen oder auf <strong>+</strong> klicken</em>"
      : " <em>Drag track onto canvas or click <strong>+</strong></em>";
    container.appendChild(hint);

    // Track Cards List
    const listContainer = document.createElement("div");
    listContainer.className = "track-cards-list";
    this.renderTrackList(listContainer);
    container.appendChild(listContainer);
  }

  private renderTrackList(container: HTMLElement) {
    container.innerHTML = "";

    const currentLang = getLanguage();
    const isDe = currentLang === "de";
    const query = this.searchQuery.toLowerCase().trim();
    const allowedCats = CATEGORY_MAP[this.selectedCategory] ?? [this.selectedCategory];

    const filtered = this.catalog.entries.filter((e) => {
      const matchCat =
        this.selectedCategory === "All" || allowedCats.includes(e.category) || e.category === this.selectedCategory;
      const matchQuery =
        !query ||
        e.id.toLowerCase().includes(query) ||
        e.name.toLowerCase().includes(query) ||
        getTrackDisplayName(e, "en").toLowerCase().includes(query) ||
        getTrackDisplayName(e, "de").toLowerCase().includes(query) ||
        e.description.toLowerCase().includes(query);
      return matchCat && matchQuery;
    });

    if (filtered.length === 0) {
      container.innerHTML = `<div class="empty-state">${
        isDe ? "Keine passenden Gleise gefunden." : "No matching tracks found."
      }</div>`;
      return;
    }

    const t = getTranslations().sidebar;

    for (const entry of filtered) {
      const card = document.createElement("div");
      card.className = "track-card";
      card.draggable = true;

      const bp = getTrackBlueprintColor(entry.id);

      // Dragstart: Bind JSON payload + directly render 2D Track Preview
      card.addEventListener("dragstart", (e) => {
        card.classList.add("dragging");
        e.dataTransfer?.setData("application/json", JSON.stringify(entry));
        e.dataTransfer?.setData("text/plain", entry.id);

        if (e.dataTransfer) {
          e.dataTransfer.effectAllowed = "copy";
          const zoom = this.callbacks.getZoom?.() ?? 1.0;
          const canvas = createPieceDragCanvas(entry, false, undefined, this.catalog.gauge, zoom);
          attachDragImage(e.dataTransfer, canvas);
        }
      });

      card.addEventListener("dragend", () => {
        card.classList.remove("dragging");
      });

      let spec = "";
      if (entry.length) spec += `L: ${entry.length} mm `;
      if (entry.radius) spec += `R: ${entry.radius} mm `;
      if (entry.angle) spec += `/ ${entry.angle}° `;
      if (entry.branchAngle) spec += `Branch: ${entry.branchAngle}°`;

      const displayName = getTrackDisplayName(entry, currentLang);
      const displayCategory = getTrackDisplayCategory(entry.category, currentLang);
      const displayDesc = getTrackDisplayDescription(entry, currentLang);

      card.innerHTML = `
        <div class="card-top">
          <div style="display: flex; align-items: center; gap: 4px;">
            <span class="card-color-dot" style="background: ${bp.css};" title="Blueprint Color"></span>
            <span class="card-id">${entry.id}</span>
          </div>
          <span class="card-category-badge">${displayCategory}</span>
        </div>
        <div class="card-name" title="${displayDesc}">${displayName}</div>
        <div class="card-spec">${spec}</div>
      `;

      const addBtn = document.createElement("button");
      addBtn.className = "card-add-btn";
      addBtn.innerHTML = `+ ${t.catalog.addToPlan}`;
      addBtn.title = isDe ? "Gleis in die Mitte des Plans einfügen" : "Place track in center of layout";
      addBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        this.callbacks.onAddPiece(entry);
      });

      card.appendChild(addBtn);
      container.appendChild(card);
    }
  }

  // =========================================================================
  // TAB 1B: ROADS CATALOG (1-STEP MENU: SCALE -> AUTO-LOAD)
  // =========================================================================
  private renderRoadsTab(container: HTMLElement) {
    const t = getTranslations().sidebar;
    const isDe = getLanguage() === "de";

    let isMenuOpen = false;

    const selectorBox = document.createElement("div");
    selectorBox.className = "catalog-system-box";
    selectorBox.style.position = "relative";

    const trigger = document.createElement("button");
    trigger.type = "button";
    trigger.className = "catalog-dropdown-trigger";
    trigger.innerHTML = `
      <div class="catalog-trigger-text">
        <span class="catalog-trigger-title">${formatCatalogTitle(this.roadCatalog)}</span>
        <span class="catalog-trigger-sub">${formatCatalogSubtitle(this.roadCatalog, isDe, t)}</span>
      </div>
      <span class="catalog-dropdown-arrow">▾</span>
    `;

    const popover = document.createElement("div");
    popover.className = "two-step-popover";
    popover.style.display = "none";

    const updateRoadPopover = () => {
      if (!isMenuOpen) {
        popover.style.display = "none";
        return;
      }
      popover.style.display = "flex";
      popover.innerHTML = "";

      const header = document.createElement("div");
      header.className = "popover-header";
      header.textContent = isDe ? "Maßstab wählen" : "Select Scale";
      popover.appendChild(header);

      const list = document.createElement("div");
      list.className = "popover-list";

      for (const sc of AVAILABLE_SCALES) {
        const cat = findRoadCatalogForScale(sc);
        const item = document.createElement("button");
        item.type = "button";
        item.disabled = !cat;
        item.className = `popover-item ${sc === this.currentRoadScale ? "active" : ""}`;
        item.style.opacity = cat ? "1" : "0.45";
        item.style.cursor = cat ? "pointer" : "not-allowed";
        item.innerHTML = `
          <div>
            <div style="font-weight: 600;">Spur ${sc}</div>
            <div class="popover-item-sub">${
              cat
                ? isDe
                  ? SCALE_DESCRIPTIONS[sc].de
                  : SCALE_DESCRIPTIONS[sc].en
                : isDe
                ? "Kein Straßenkatalog gefunden"
                : "No road catalog found"
            }</div>
          </div>
          ${sc === this.currentRoadScale ? "<span>✓</span>" : ""}
        `;
        item.addEventListener("click", (e) => {
          e.stopPropagation();
          if (!cat) return;
          this.currentRoadScale = sc;
          this.roadCatalog = cat;
          this.callbacks.onSelectRoadScale?.(sc);
          this.callbacks.onSelectSystem?.(cat.id);
          isMenuOpen = false;
          this.render();
        });
        list.appendChild(item);
      }
      popover.appendChild(list);
    };

    trigger.addEventListener("click", (e) => {
      e.stopPropagation();
      isMenuOpen = !isMenuOpen;
      updateRoadPopover();
    });

    this.addWindowClickListener((e) => {
      if (!selectorBox.contains(e.target as Node) && isMenuOpen) {
        isMenuOpen = false;
        updateRoadPopover();
      }
    });

    selectorBox.append(trigger, popover);
    container.appendChild(selectorBox);

    // Search Box
    const searchBox = document.createElement("div");
    searchBox.className = "search-box";
    const input = document.createElement("input");
    input.type = "text";
    input.placeholder = t.catalog.searchRoadsPlaceholder;
    input.value = this.roadSearchQuery;
    input.addEventListener("input", (e) => {
      this.roadSearchQuery = (e.target as HTMLInputElement).value;
      this.renderRoadsList(listContainer);
    });
    searchBox.appendChild(input);
    container.appendChild(searchBox);

    // Category Filter Bar
    const categories: Array<{ id: string; label: string }> = [
      { id: "All", label: t.catalog.categories.all },
      { id: "Straights", label: t.catalog.categories.straights },
      { id: "Curves", label: t.catalog.categories.curves },
      { id: "Crossings", label: t.catalog.categories.crossings },
      { id: "Accessories", label: t.catalog.categories.accessories },
    ];
    const filterBar = document.createElement("div");
    filterBar.className = "category-filter-bar";
    for (const cat of categories) {
      const pill = document.createElement("button");
      pill.className = `filter-pill ${this.selectedRoadCategory === cat.id ? "active" : ""}`;
      pill.textContent = cat.label;
      pill.addEventListener("click", () => {
        this.selectedRoadCategory = cat.id;
        this.render();
      });
      filterBar.appendChild(pill);
    }
    container.appendChild(filterBar);

    // Drag & Drop Hint
    const hint = document.createElement("div");
    hint.className = "drag-hint";
    hint.innerHTML = isDe
      ? " <em>Straße auf die Arbeitsfläche ziehen oder auf <strong>+</strong> klicken</em>"
      : " <em>Drag road onto canvas or click <strong>+</strong></em>";
    container.appendChild(hint);

    // Road Cards List
    const listContainer = document.createElement("div");
    listContainer.className = "track-cards-list";
    this.renderRoadsList(listContainer);
    container.appendChild(listContainer);
  }

  private renderRoadsList(container: HTMLElement) {
    container.innerHTML = "";

    const currentLang = getLanguage();
    const isDe = currentLang === "de";
    const query = this.roadSearchQuery.toLowerCase().trim();
    const allowedCats = CATEGORY_MAP[this.selectedRoadCategory] ?? [this.selectedRoadCategory];

    const filtered = this.roadCatalog.entries.filter((e) => {
      const matchCat =
        this.selectedRoadCategory === "All" ||
        allowedCats.includes(e.category) ||
        e.category === this.selectedRoadCategory;
      const matchQuery =
        !query ||
        e.id.toLowerCase().includes(query) ||
        e.name.toLowerCase().includes(query) ||
        getTrackDisplayName(e, "en").toLowerCase().includes(query) ||
        getTrackDisplayName(e, "de").toLowerCase().includes(query) ||
        e.description.toLowerCase().includes(query);
      return matchCat && matchQuery;
    });

    if (filtered.length === 0) {
      container.innerHTML = `<div class="empty-state">${
        isDe ? "Keine passenden Straßen gefunden." : "No matching roads found."
      }</div>`;
      return;
    }

    const t = getTranslations().sidebar;

    for (const entry of filtered) {
      const card = document.createElement("div");
      card.className = "track-card";
      card.draggable = true;

      // Dragstart: Bind JSON payload + directly render 2D Road Preview
      card.addEventListener("dragstart", (e) => {
        card.classList.add("dragging");
        e.dataTransfer?.setData("application/json", JSON.stringify(entry));
        e.dataTransfer?.setData("text/plain", entry.id);

        if (e.dataTransfer) {
          e.dataTransfer.effectAllowed = "copy";
          const zoom = this.callbacks.getZoom?.() ?? 1.0;
          const canvas = createPieceDragCanvas(entry, true, getRoadWidthMm(this.roadCatalog.scale), undefined, zoom);
          attachDragImage(e.dataTransfer, canvas);
        }
      });

      card.addEventListener("dragend", () => {
        card.classList.remove("dragging");
      });

      let spec = "";
      if (entry.length) spec += `L: ${entry.length} mm `;
      if (entry.radius) spec += `R: ${entry.radius} mm `;
      if (entry.angle) spec += `/ ${entry.angle}° `;
      if (entry.branchAngle) spec += `Branch: ${entry.branchAngle}°`;

      const displayName = getTrackDisplayName(entry, currentLang);
      const displayCategory = getTrackDisplayCategory(entry.category, currentLang);
      const displayDesc = getTrackDisplayDescription(entry, currentLang);

      card.innerHTML = `
        <div class="card-top">
          <div style="display: flex; align-items: center; gap: 4px;">
            <span class="card-color-dot" style="background: #475569;" title="Road"></span>
            <span class="card-id">${entry.id}</span>
          </div>
          <span class="card-category-badge">${displayCategory}</span>
        </div>
        <div class="card-name" title="${displayDesc}">${displayName}</div>
        <div class="card-spec">${spec}</div>
      `;

      const addBtn = document.createElement("button");
      addBtn.className = "card-add-btn";
      addBtn.innerHTML = `+ ${t.catalog.addToPlan}`;
      addBtn.title = isDe ? "Straße in die Mitte des Plans einfügen" : "Place road in center of layout";
      addBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        this.callbacks.onAddPiece(entry);
      });

      card.appendChild(addBtn);
      container.appendChild(card);
    }
  }

  // =========================================================================
  // TAB 1C: 3D MODELS & SCENERY (2-STEP MENU: SCALE -> CATEGORY)
  // =========================================================================

  /** Built-in models are defined in H0, so scale their dimensions to the selected scale. */
  private getModelScaleFactor(): number {
    return getScaleFactor(this.currentModelScale);
  }

  private createScaledBuiltin(id: string): Model3D {
    return scaleDimensions(createBuiltinModel(id), this.currentModelScale);
  }

  private renderModelsTab(container: HTMLElement) {
    const isDe = getLanguage() === "de";

    // Single import field: click to open file dialog OR drop files
    const importArea = document.createElement("div");
    importArea.className = "models-import-area";

    const fileInput = document.createElement("input");
    fileInput.type = "file";
    fileInput.accept = ".glb,.gltf";
    fileInput.multiple = true;
    fileInput.style.display = "none";
    fileInput.addEventListener("change", async () => {
      const files = Array.from(fileInput.files ?? []);
      fileInput.value = "";
      for (const f of files) await this.handleImportModel(f);
    });
    importArea.appendChild(fileInput);

    const dropzone = document.createElement("div");
    dropzone.className = "models-dropzone";
    dropzone.tabIndex = 0;
    dropzone.setAttribute("role", "button");
    dropzone.title = isDe ? "3D-Modell (.glb / .gltf) importieren" : "Import a 3D model (.glb / .gltf)";
    dropzone.innerHTML = `
      <div class="dropzone-icon">📂</div>
      <div class="dropzone-text">${
        isDe
          ? "<code>.glb</code> / <code>.gltf</code> hierher ziehen oder <strong>klicken</strong>"
          : "Drop <code>.glb</code> / <code>.gltf</code> here or <strong>click to import</strong>"
      }</div>
    `;

    dropzone.addEventListener("click", () => fileInput.click());
    dropzone.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        fileInput.click();
      }
    });
    dropzone.addEventListener("dragover", (e) => {
      e.preventDefault();
      dropzone.classList.add("dragover");
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    });
    dropzone.addEventListener("dragleave", () => dropzone.classList.remove("dragover"));
    dropzone.addEventListener("drop", async (e) => {
      e.preventDefault();
      dropzone.classList.remove("dragover");
      const files = Array.from(e.dataTransfer?.files ?? []);
      for (const f of files) await this.handleImportModel(f);
    });

    importArea.appendChild(dropzone);
    container.appendChild(importArea);

    // 1-step menu: SCALE SELECTOR (identical to streets tab)
    let isMenuOpen = false;

    const selectorBox = document.createElement("div");
    selectorBox.className = "catalog-system-box";
    selectorBox.style.position = "relative";

    const trigger = document.createElement("button");
    trigger.type = "button";
    trigger.className = "catalog-dropdown-trigger";
    trigger.innerHTML = `
      <div class="catalog-trigger-text">
        <span class="catalog-trigger-title">${isDe ? "Spur" : "Scale"} ${this.currentModelScale} (1:${MODEL_SCALE_RATIOS[this.currentModelScale]})</span>
        <span class="catalog-trigger-sub">${
          isDe ? SCALE_DESCRIPTIONS[this.currentModelScale].de : SCALE_DESCRIPTIONS[this.currentModelScale].en
        }</span>
      </div>
      <span class="catalog-dropdown-arrow">▾</span>
    `;

    const popover = document.createElement("div");
    popover.className = "two-step-popover";
    popover.style.display = "none";

    const updateModelPopover = () => {
      if (!isMenuOpen) {
        popover.style.display = "none";
        return;
      }
      popover.style.display = "flex";
      popover.innerHTML = "";

      const header = document.createElement("div");
      header.className = "popover-header";
      header.textContent = isDe ? "Maßstab wählen" : "Select Scale";
      popover.appendChild(header);

      const list = document.createElement("div");
      list.className = "popover-list";

      for (const sc of AVAILABLE_SCALES) {
        const item = document.createElement("button");
        item.type = "button";
        item.className = `popover-item ${sc === this.currentModelScale ? "active" : ""}`;
        item.innerHTML = `
          <div>
            <div style="font-weight: 600;">Spur ${sc}</div>
            <div class="popover-item-sub">1:${MODEL_SCALE_RATIOS[sc]} · ${
              isDe ? SCALE_DESCRIPTIONS[sc].de : SCALE_DESCRIPTIONS[sc].en
            }</div>
          </div>
          ${sc === this.currentModelScale ? "<span>✓</span>" : ""}
        `;
        item.addEventListener("click", (e) => {
          e.stopPropagation();
          this.currentModelScale = sc;
          isMenuOpen = false;
          this.render();
        });
        list.appendChild(item);
      }
      popover.appendChild(list);
    };

    trigger.addEventListener("click", (e) => {
      e.stopPropagation();
      isMenuOpen = !isMenuOpen;
      updateModelPopover();
    });

    this.addWindowClickListener((e) => {
      if (!selectorBox.contains(e.target as Node) && isMenuOpen) {
        isMenuOpen = false;
        updateModelPopover();
      }
    });

    selectorBox.append(trigger, popover);
    container.appendChild(selectorBox);

    // Search Box
    const searchBox = document.createElement("div");
    searchBox.className = "search-box";
    const input = document.createElement("input");
    input.type = "text";
    input.placeholder = isDe ? "🔍 3D-Modelle suchen..." : "🔍 Search 3D models...";
    input.value = this.modelsSearchQuery;
    input.addEventListener("input", (e) => {
      this.modelsSearchQuery = (e.target as HTMLInputElement).value;
      this.renderModelsList(listContainer);
    });
    searchBox.appendChild(input);
    container.appendChild(searchBox);

    // Category Filter Bar (Pills like in streets & tracks)
    const categories: Array<{ id: string; label: string }> = [
      { id: "All", label: isDe ? MODEL_CATEGORY_LABELS_DE.All ?? "Alle" : "All" },
      { id: "Buildings", label: isDe ? MODEL_CATEGORY_LABELS_DE.Buildings ?? "Gebäude" : "Buildings" },
      { id: "Railway", label: isDe ? MODEL_CATEGORY_LABELS_DE.Railway ?? "Bahn" : "Railway" },
      { id: "Nature", label: isDe ? MODEL_CATEGORY_LABELS_DE.Nature ?? "Natur" : "Nature" },
      { id: "Custom", label: isDe ? MODEL_CATEGORY_LABELS_DE.Custom ?? "Eigene" : "Custom" },
    ];
    const filterBar = document.createElement("div");
    filterBar.className = "category-filter-bar";
    for (const cat of categories) {
      const pill = document.createElement("button");
      pill.className = `filter-pill ${
        this.selectedModelCategory.toLowerCase() === cat.id.toLowerCase() ? "active" : ""
      }`;
      pill.textContent = cat.label;
      pill.addEventListener("click", () => {
        this.selectedModelCategory = cat.id;
        this.render();
      });
      filterBar.appendChild(pill);
    }
    container.appendChild(filterBar);

    const hint = document.createElement("div");
    hint.className = "drag-hint";
    hint.innerHTML = isDe
      ? " <em>Modell auf die Arbeitsfläche ziehen oder auf <strong>+</strong> klicken</em>"
      : " <em>Drag model onto canvas or click <strong>+ Add to Plan</strong></em>";
    container.appendChild(hint);

    const listContainer = document.createElement("div");
    listContainer.className = "track-cards-list";
    this.renderModelsList(listContainer);
    container.appendChild(listContainer);
  }

  private async handleImportModel(file: File) {
    try {
      const model = await parseGlbOrGltfFile(file);
      if (!this.customModelsLibrary.some((m) => m.name === model.name)) {
        this.customModelsLibrary.push(model);
      }
      this.callbacks.onAddModel?.(model);
      this.render();
    } catch (err: any) {
      alert(`Error importing 3D model: ${err.message || err}`);
    }
  }

  private renderModelsList(container: HTMLElement) {
    container.innerHTML = "";
    const t = getTranslations().sidebar;
    const isDe = getLanguage() === "de";

    const query = this.modelsSearchQuery.toLowerCase().trim();
    const selCat = this.selectedModelCategory.toLowerCase();

    const matchingBuiltins = BUILTIN_MODELS.filter((def) => {
      if (selCat !== "all" && selCat !== "custom" && def.category.toLowerCase() !== selCat) {
        return false;
      }
      if (selCat === "custom") return false;
      if (!query) return true;
      return def.name.toLowerCase().includes(query) || def.description.toLowerCase().includes(query);
    });

    const matchingCustoms = this.customModelsLibrary.filter((m) => {
      if (selCat !== "all" && selCat !== "custom") return false;
      if (!query) return true;
      return m.name.toLowerCase().includes(query);
    });

    if (matchingBuiltins.length === 0 && matchingCustoms.length === 0) {
      const empty = document.createElement("div");
      empty.className = "catalog-empty";
      empty.textContent = isDe ? "Keine 3D-Modelle gefunden." : "No 3D models found matching your search.";
      container.appendChild(empty);
      return;
    }

    for (const def of matchingBuiltins) {
      const f = this.getModelScaleFactor();
      const fmt = (v: number) => Math.round(v * f * 10) / 10;

      const card = document.createElement("div");
      card.className = "track-card model-card";
      card.draggable = true;

      card.addEventListener("dragstart", (e) => {
        card.classList.add("dragging");
        const newModel = this.createScaledBuiltin(def.id);
        e.dataTransfer?.setData("application/railbench-model", JSON.stringify(newModel));
        e.dataTransfer?.setData("text/plain", def.name);
        if (e.dataTransfer) {
          e.dataTransfer.effectAllowed = "copy";
          const zoom = this.callbacks.getZoom?.() ?? 1.0;
          const canvas = createModelDragCanvas(def, f, zoom);
          attachDragImage(e.dataTransfer, canvas);
        }
      });

      card.addEventListener("dragend", () => {
        card.classList.remove("dragging");
      });

      card.innerHTML = `
        <div class="card-top">
          <div style="display: flex; align-items: center; gap: 6px;">
            <span style="font-size: 16px;">${def.icon}</span>
            <span class="card-id" style="font-weight: 700;">${def.name}</span>
          </div>
          <span class="card-category-badge" style="background: ${def.badgeBg}; color: ${def.badgeColor};">${def.category}</span>
        </div>
        <div class="card-spec" style="margin-top: 4px; font-weight: 600; color: var(--text-main);">
          Size: ${fmt(def.width)} × ${fmt(def.depth)} × ${fmt(def.height)} mm (${this.currentModelScale})
        </div>
        <div class="card-desc" style="font-size: 10px; color: var(--text-muted); line-height: 1.3; margin-top: 2px;">
          ${def.description}
        </div>
      `;

      const addBtn = document.createElement("button");
      addBtn.className = "card-add-btn";
      addBtn.innerHTML = `+ ${t.catalog.addToPlan}`;
      addBtn.title = isDe ? `${def.name} in den Plan einfügen` : `Place ${def.name} in center of layout`;
      addBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        this.callbacks.onAddModel?.(this.createScaledBuiltin(def.id));
      });

      card.appendChild(addBtn);
      container.appendChild(card);
    }

    for (const m of matchingCustoms) {
      const card = document.createElement("div");
      card.className = "track-card model-card custom-model-card";
      card.draggable = true;

      card.addEventListener("dragstart", (e) => {
        card.classList.add("dragging");
        const fresh = { ...m, id: `model-${Date.now()}-${Math.random().toString(36).slice(2, 6)}` };
        e.dataTransfer?.setData("application/railbench-model", JSON.stringify(fresh));
        if (e.dataTransfer) {
          e.dataTransfer.effectAllowed = "copy";
          const zoom = this.callbacks.getZoom?.() ?? 1.0;
          const canvas = createCustomModelDragCanvas(m, zoom);
          attachDragImage(e.dataTransfer, canvas);
        }
      });

      card.addEventListener("dragend", () => {
        card.classList.remove("dragging");
      });

      card.innerHTML = `
        <div class="card-top">
          <div style="display: flex; align-items: center; gap: 6px;">
            <span style="font-size: 16px;">📁</span>
            <span class="card-id" style="font-weight: 700;">${m.name}</span>
          </div>
          <span class="card-category-badge" style="background: #6366f1; color: #ffffff;">.${m.modelType}</span>
        </div>
        <div class="card-spec" style="margin-top: 4px; font-weight: 600; color: var(--text-main);">
          Size: ${m.width} × ${m.depth} × ${m.height} mm
        </div>
      `;

      const actions = document.createElement("div");
      actions.style.display = "flex";
      actions.style.gap = "4px";
      actions.style.marginTop = "6px";

      const addBtn = document.createElement("button");
      addBtn.className = "card-add-btn";
      addBtn.style.flex = "1";
      addBtn.innerHTML = `+ ${t.catalog.addToPlan}`;
      addBtn.title = isDe ? `${m.name} in den Plan einfügen` : `Place ${m.name} in center of layout`;
      addBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        const fresh = { ...m, id: `model-${Date.now()}-${Math.random().toString(36).slice(2, 6)}` };
        this.callbacks.onAddModel?.(fresh);
      });
      actions.appendChild(addBtn);

      const delBtn = document.createElement("button");
      delBtn.className = "layer-action-btn btn-danger";
      delBtn.innerHTML = "🗑";
      delBtn.title = isDe ? "Aus eigener Bibliothek entfernen" : "Remove from custom library";
      delBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        this.customModelsLibrary = this.customModelsLibrary.filter((x) => x.name !== m.name);
        this.renderModelsList(container);
      });
      actions.appendChild(delBtn);

      card.appendChild(actions);
      container.appendChild(card);
    }
  }

  // =========================================================================
  // TAB 2: LAYERS MANAGEMENT
  // =========================================================================
  private renderLayersTab(container: HTMLElement) {
    this.layout.layers = sortLayers(this.layout.layers || []);
    const layers = this.layout.layers;
    const selectedPieceIds = this.callbacks.selectedPieceIds?.() ?? [];

    const header = document.createElement("div");
    header.className = "layers-header";
    header.innerHTML = `
      <div class="layers-title">Layout Layers</div>
      <div class="layers-subtitle">Active layer receives new tracks. Hide or lock layers during construction.</div>
    `;

    const isDe = getLanguage() === "de";
    const isUnlocked = this.callbacks.isSelectionLockDisabled?.() ?? false;
    const lockBar = document.createElement("div");
    lockBar.style.cssText = "display: flex; gap: 8px; margin-top: 8px; align-items: center;";
    const lockBtn = document.createElement("button");
    lockBtn.className = `btn-pill ${isUnlocked ? "active" : ""}`;
    lockBtn.style.cssText = isUnlocked
      ? "background: rgba(245, 158, 11, 0.2); border: 1px solid #f59e0b; color: #f59e0b; font-weight: 600; padding: 4px 8px; font-size: 11px; cursor: pointer; border-radius: 6px;"
      : "background: var(--bg-surface); border: 1px solid var(--border-color); color: var(--text-main); padding: 4px 8px; font-size: 11px; cursor: pointer; border-radius: 6px;";
    lockBtn.innerHTML = isUnlocked
      ? (isDe ? "🔓 Auswahlsperre aufgehoben" : "🔓 Selection lock disabled")
      : (isDe ? "🔒 Auswahlsperre aktiv" : "🔒 Selection lock active");
    lockBtn.title = isDe
      ? "Klicken, um die Auswahlsperre aufzuheben (erlaubt Auswahl von Gleisen auf gesperrten oder gelöschten Ebenen)"
      : "Click to toggle selection lock (select items across locked or deleted layers)";
    lockBtn.addEventListener("click", () => {
      this.callbacks.onToggleSelectionLock?.();
      this.render();
    });
    lockBar.appendChild(lockBtn);
    header.appendChild(lockBar);

    // Orphaned items alert
    const orphanedPieces = this.layout.pieces.filter((p) => !layers.some((l) => l.id === (p.layerId || "layer-1")));
    const orphanedModels = (this.layout.models || []).filter((m) => !layers.some((l) => l.id === (m.layerId || "layer-1")));
    const totalOrphaned = orphanedPieces.length + orphanedModels.length;

    if (totalOrphaned > 0) {
      const alertBox = document.createElement("div");
      alertBox.style.cssText =
        "margin-top: 8px; padding: 8px; border-radius: 6px; background: rgba(239, 68, 68, 0.15); border: 1px solid rgba(239, 68, 68, 0.4); font-size: 11px; display: flex; flex-direction: column; gap: 6px;";
      alertBox.innerHTML = `
        <div style="color: #ef4444; font-weight: 600;">⚠️ ${totalOrphaned} ${isDe ? "Element(e) auf gelöschter Ebene" : "item(s) on deleted layer"}</div>
        <button type="button" style="align-self: flex-start; cursor: pointer; padding: 3px 8px; font-size: 11px; border-radius: 4px; border: 1px solid #0284c7; background: #0284c7; color: #fff;">
          ${isDe ? "Auf aktive Ebene verschieben" : "Move to active layer"}
        </button>
      `;
      alertBox.querySelector("button")?.addEventListener("click", () => {
        const targetId = this.layout.activeLayerId || layers[0]?.id || "layer-1";
        for (const p of orphanedPieces) p.layerId = targetId;
        for (const m of orphanedModels) m.layerId = targetId;
        this.callbacks.onLayerChange?.();
        this.render();
      });
      header.appendChild(alertBox);
    }

    container.appendChild(header);

    const list = document.createElement("div");
    list.className = "layers-list";

    for (const layer of layers) {
      const isActive = layer.id === this.layout.activeLayerId;
      const count = this.layout.pieces.filter((p) => (p.layerId || "layer-1") === layer.id).length;

      const card = document.createElement("div");
      card.className = `layer-card ${isActive ? "active" : ""}`;
      card.style.borderLeft = `4px solid ${layer.color || "#38bdf8"}`;

      const cardMain = document.createElement("div");
      cardMain.className = "layer-card-main";

      const nameRow = document.createElement("div");
      nameRow.className = "layer-name-row";

      const nameSpan = document.createElement("span");
      nameSpan.className = "layer-name layer-name-label";
      nameSpan.textContent = layer.name;
      nameSpan.title = "Double-click or click ✏️ to rename layer";

      const renameBtn = document.createElement("button");
      renameBtn.type = "button";
      renameBtn.className = "layer-rename-btn";
      renameBtn.innerHTML = "✏️";
      renameBtn.title = "Rename layer (Ebene umbenennen)";

      const startEditingName = () => {
        const input = document.createElement("input");
        input.type = "text";
        input.className = "layer-name-edit-input";
        input.value = layer.name;

        let committed = false;
        const commit = () => {
          if (committed) return;
          committed = true;
          const val = input.value.trim();
          if (val && val !== layer.name) {
            layer.name = val;
            this.callbacks.onLayerChange?.();
          }
          this.render();
        };

        const cancel = () => {
          if (committed) return;
          committed = true;
          this.render();
        };

        input.addEventListener("keydown", (ev) => {
          ev.stopPropagation();
          if (ev.key === "Enter") {
            ev.preventDefault();
            commit();
          } else if (ev.key === "Escape") {
            ev.preventDefault();
            cancel();
          }
        });
        input.addEventListener("blur", commit);
        input.addEventListener("pointerdown", (ev) => ev.stopPropagation());
        input.addEventListener("click", (ev) => ev.stopPropagation());

        nameRow.innerHTML = "";
        nameRow.appendChild(input);
        input.focus();
        input.select();
      };

      renameBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        startEditingName();
      });

      nameSpan.addEventListener("dblclick", (e) => {
        e.stopPropagation();
        startEditingName();
      });

      nameRow.appendChild(nameSpan);
      nameRow.appendChild(renameBtn);

      const metaDiv = document.createElement("div");
      metaDiv.className = "layer-meta";
      metaDiv.innerHTML = `
        <span>${count} track${count === 1 ? "" : "s"}</span>
        <span>•</span>
        <span class="layer-elev-clickable" title="Click to edit elevation (mm)">Elevation: ${layer.heightMm ?? 0} mm</span>
      `;
      metaDiv.querySelector(".layer-elev-clickable")?.addEventListener("click", (e) => {
        e.stopPropagation();
        openLayerDialog({
          layout: this.layout,
          layer,
          onApply: (updated) => {
            Object.assign(layer, updated);
            this.callbacks.onLayerChange?.();
            this.render();
          },
        });
      });

      cardMain.appendChild(nameRow);
      cardMain.appendChild(metaDiv);
      card.appendChild(cardMain);

      const actions = document.createElement("div");
      actions.className = "layer-actions";

      if (!isActive) {
        const activateBtn = document.createElement("button");
        activateBtn.className = "layer-btn-pill";
        activateBtn.textContent = "Select";
        activateBtn.title = "Set as active layer";
        activateBtn.addEventListener("click", () => {
          this.layout.activeLayerId = layer.id;
          this.callbacks.onLayerChange?.();
          this.render();
        });
        actions.appendChild(activateBtn);
      }

      const visBtn = document.createElement("button");
      visBtn.className = `layer-icon-btn ${layer.visible ? "active" : "muted"}`;
      visBtn.innerHTML = layer.visible ? "👁️" : "🕶️";
      visBtn.title = layer.visible ? "Visible (Click to hide)" : "Hidden (Click to show)";
      visBtn.addEventListener("click", () => {
        layer.visible = !layer.visible;
        this.callbacks.onLayerChange?.();
        this.render();
      });
      actions.appendChild(visBtn);

      const lockBtn = document.createElement("button");
      lockBtn.className = `layer-icon-btn ${layer.locked ? "locked active-warning" : ""}`;
      lockBtn.innerHTML = layer.locked ? "🔒" : "🔓";
      lockBtn.title = layer.locked ? "Locked (Click to unlock)" : "Unlocked (Click to lock)";
      lockBtn.addEventListener("click", () => {
        layer.locked = !layer.locked;
        this.callbacks.onLayerChange?.();
        this.render();
      });
      actions.appendChild(lockBtn);

      if (selectedPieceIds.length > 0) {
        const moveBtn = document.createElement("button");
        moveBtn.className = "layer-btn-pill";
        moveBtn.textContent = `Move (${selectedPieceIds.length})`;
        moveBtn.title = `Move selected tracks to ${layer.name}`;
        moveBtn.addEventListener("click", () => {
          this.callbacks.onMoveSelectedToLayer?.(layer.id);
          this.render();
        });
        actions.appendChild(moveBtn);
      }

      if (layers.length > 1) {
        const delBtn = document.createElement("button");
        delBtn.className = "layer-icon-btn btn-danger";
        delBtn.innerHTML = "🗑";
        delBtn.title = "Delete layer (tracks move to main layer)";
        delBtn.addEventListener("click", () => {
          if (confirm(`Delete layer "${layer.name}"? Tracks on this layer will be moved to the primary layer.`)) {
            const fallbackId = layers.find((l) => l.id !== layer.id)!.id;
            for (const p of this.layout.pieces) {
              if (p.layerId === layer.id) p.layerId = fallbackId;
            }
            if (this.layout.models) {
              for (const m of this.layout.models) {
                if (m.layerId === layer.id) m.layerId = fallbackId;
              }
            }
            this.layout.layers = layers.filter((l) => l.id !== layer.id);
            if (this.layout.activeLayerId === layer.id) {
              this.layout.activeLayerId = fallbackId;
            }
            this.callbacks.onLayerChange?.();
            this.render();
          }
        });
        actions.appendChild(delBtn);
      }

      card.appendChild(actions);
      list.appendChild(card);
    }

    container.appendChild(list);

    const addLayerBtn = document.createElement("button");
    addLayerBtn.className = "btn-export-csv";
    addLayerBtn.style.marginTop = "6px";
    addLayerBtn.innerHTML = "+ Add New Layer";
    addLayerBtn.addEventListener("click", () => {
      openLayerDialog({
        layout: this.layout,
        position: "above",
        onApply: (newLayer, _pos, makeActive) => {
          if (!this.layout.layers) this.layout.layers = [];
          this.layout.layers.push(newLayer);
          this.layout.layers = sortLayers(this.layout.layers);
          if (makeActive) {
            this.layout.activeLayerId = newLayer.id;
          }
          this.callbacks.onLayerChange?.();
          this.render();
        },
      });
    });
    container.appendChild(addLayerBtn);
  }

  // =========================================================================
  // TAB 2B: TABLE / BASEBOARD SETTINGS
  // =========================================================================
  private renderBoardTab(container: HTMLElement) {
    const baseboards = getBaseboards(this.layout);
    if (!this.activeBoardId || !baseboards.some((b) => b.id === this.activeBoardId)) {
      this.activeBoardId = baseboards[0]?.id;
    }
    const bb = baseboards.find((b) => b.id === this.activeBoardId) || baseboards[0]!;

    const tBoard = getTranslations().sidebar.board;
    const isDe = getLanguage() === "de";

    const header = document.createElement("div");
    container.appendChild(header);

    const anyEnabled = baseboards.some((b) => b.enabled);
    const topCard = document.createElement("div");
    topCard.className = "board-card";
    topCard.style.display = "flex";
    topCard.style.flexDirection = "row";
    topCard.style.justifyContent = "space-between";
    topCard.style.alignItems = "center";
    topCard.innerHTML = `
      <label class="board-toggle-label">
        <input type="checkbox" id="bb-enabled" ${anyEnabled ? "checked" : ""}>
        <span>${tBoard.enableToggle}</span>
      </label>
      <button class="rb-btn rb-btn-secondary board-add-table-btn" id="bb-add-btn">
        + ${isDe ? "Tisch hinzufügen" : "Add Table"}
      </button>
    `;

    const checkInput = topCard.querySelector("#bb-enabled") as HTMLInputElement;
    checkInput.addEventListener("change", () => {
      const state = checkInput.checked;
      for (const b of baseboards) {
        b.enabled = state;
      }
      this.callbacks.onBaseboardChange?.();
      this.render();
    });

    const addBtn = topCard.querySelector("#bb-add-btn") as HTMLButtonElement;
    addBtn.addEventListener("click", () => {
      const newBoard = addBaseboard(this.layout);
      this.activeBoardId = newBoard.id;
      this.callbacks.onBaseboardChange?.();
      this.render();
    });
    container.appendChild(topCard);

    if (!anyEnabled) {
      const note = document.createElement("div");
      note.className = "empty-state";
      note.innerHTML = `<span class="empty-text">${
        isDe ? "Tisch ist ausgeblendet" : "Table is hidden"
      }</span><span class="empty-sub">${
        isDe ? "Aktivieren Sie die Option oben, um den Tisch anzuzeigen." : "Check the option above to show table benchwork."
      }</span>`;
      container.appendChild(note);
      return;
    }

    const tableCard = document.createElement("div");
    tableCard.className = "board-card";

    if (baseboards.length > 1) {
      tableCard.innerHTML = `
        <div class="board-section-title">${isDe ? "Tisch auswählen" : "Select Table"} (${baseboards.length})</div>
        <div style="display: flex; gap: 6px; align-items: center;">
          <select id="bb-table-select" class="board-select" style="flex: 1;">
            ${baseboards
              .map(
                (b, i) =>
                  `<option value="${b.id}" ${b.id === bb.id ? "selected" : ""}>${
                    b.name ?? (isDe ? `Tisch ${i + 1}` : `Table ${i + 1}`)
                  } (${b.width} × ${b.depth} mm)</option>`,
              )
              .join("")}
          </select>
          <button class="rb-btn rb-btn-secondary board-action-btn" id="bb-vis-btn" title="${
            isDe ? "Sichtbarkeit umschalten" : "Toggle visibility"
          }">${bb.enabled ? "👁️" : "🚫"}</button>
          <button class="rb-btn rb-btn-secondary board-action-btn" id="bb-dup-btn" title="${tBoard.duplicateTable}">📑</button>
          <button class="rb-btn rb-btn-secondary board-action-btn" id="bb-del-btn" title="${tBoard.deleteTable}" style="color: #ef4444;">🗑️</button>
        </div>
        <div class="board-field" style="margin-top: 6px;">
          <label>${tBoard.tableName}</label>
          <input type="text" id="bb-table-name" value="${bb.name ?? "Tisch"}" class="board-text-input">
        </div>
      `;

      const tableSelect = tableCard.querySelector("#bb-table-select") as HTMLSelectElement;
      tableSelect.addEventListener("change", () => {
        this.activeBoardId = tableSelect.value;
        this.render();
      });

      const visBtn = tableCard.querySelector("#bb-vis-btn") as HTMLButtonElement;
      visBtn.addEventListener("click", () => {
        bb.enabled = !bb.enabled;
        this.callbacks.onBaseboardChange?.();
        this.render();
      });

      const dupBtn = tableCard.querySelector("#bb-dup-btn") as HTMLButtonElement;
      dupBtn.addEventListener("click", () => {
        const clone = duplicateBaseboard(this.layout, bb.id!);
        if (clone) {
          this.activeBoardId = clone.id;
          this.callbacks.onBaseboardChange?.();
          this.render();
        }
      });

      const delBtn = tableCard.querySelector("#bb-del-btn") as HTMLButtonElement;
      delBtn.addEventListener("click", () => {
        if (baseboards.length <= 1) return;
        removeBaseboard(this.layout, bb.id!);
        this.activeBoardId = this.layout.baseboards?.[0]?.id;
        this.callbacks.onBaseboardChange?.();
        this.render();
      });
    } else {
      tableCard.innerHTML = `
        <div class="board-field">
          <label>${tBoard.tableName}</label>
          <input type="text" id="bb-table-name" value="${bb.name ?? "Tisch"}" class="board-text-input">
        </div>
      `;
    }

    const nameInput = tableCard.querySelector("#bb-table-name") as HTMLInputElement;
    nameInput.addEventListener("input", () => {
      bb.name = nameInput.value;
      this.callbacks.onBaseboardChange?.();
    });
    container.appendChild(tableCard);

    const currentShape = bb.shape ?? "rect";
    const dimsCard = document.createElement("div");
    dimsCard.className = "board-card";

    let shapeSpecificHTML = "";
    if (currentShape === "l-shape") {
      shapeSpecificHTML = `
        <div class="board-inputs-row" style="margin-top: 4px;">
          <div class="board-input-field">
            <label>${isDe ? "Flügelbreite" : "Wing Width"}</label>
            <div class="board-input-with-unit">
              <input type="number" id="bb-l-wing-w" value="${bb.lWingWidth ?? 800}" step="50" min="300" max="10000">
              <span class="unit-tag">mm</span>
            </div>
          </div>
          <div class="board-input-field">
            <label>${isDe ? "Haupttiefe" : "Main Depth"}</label>
            <div class="board-input-with-unit">
              <input type="number" id="bb-l-main-d" value="${bb.lWingDepth ?? 800}" step="50" min="300" max="10000">
              <span class="unit-tag">mm</span>
            </div>
          </div>
        </div>
        <div class="board-field" style="margin-top: 4px;">
          <label>${isDe ? "Flügelposition" : "Wing Position"}</label>
          <select id="bb-l-side" class="board-select">
            <option value="left" ${(bb.lSide ?? "left") === "left" ? "selected" : ""}>${isDe ? "Links" : "Left"}</option>
            <option value="right" ${bb.lSide === "right" ? "selected" : ""}>${isDe ? "Rechts" : "Right"}</option>
          </select>
        </div>
      `;
    } else if (currentShape === "u-shape") {
      shapeSpecificHTML = `
        <div class="board-inputs-row" style="margin-top: 4px;">
          <div class="board-input-field">
            <label>${isDe ? "Schenkel links" : "Left Wing"}</label>
            <div class="board-input-with-unit">
              <input type="number" id="bb-u-left-w" value="${bb.uLeftWingWidth ?? 700}" step="50" min="300" max="5000">
              <span class="unit-tag">mm</span>
            </div>
          </div>
          <div class="board-input-field">
            <label>${isDe ? "Schenkel rechts" : "Right Wing"}</label>
            <div class="board-input-with-unit">
              <input type="number" id="bb-u-right-w" value="${bb.uRightWingWidth ?? 700}" step="50" min="300" max="5000">
              <span class="unit-tag">mm</span>
            </div>
          </div>
        </div>
        <div class="board-field" style="margin-top: 4px;">
          <label>${isDe ? "Quertiefe" : "Bridge Depth"}</label>
          <div class="board-input-with-unit">
            <input type="number" id="bb-u-back-d" value="${bb.uBackDepth ?? 600}" step="50" min="300" max="5000">
            <span class="unit-tag">mm</span>
          </div>
        </div>
      `;
    } else if (currentShape === "t-shape") {
      shapeSpecificHTML = `
        <div class="board-inputs-row" style="margin-top: 4px;">
          <div class="board-input-field">
            <label>${isDe ? "Kopfteil Tiefe" : "Top Bar Depth"}</label>
            <div class="board-input-with-unit">
              <input type="number" id="bb-t-bar-d" value="${bb.tBarDepth ?? 600}" step="50" min="300" max="5000">
              <span class="unit-tag">mm</span>
            </div>
          </div>
          <div class="board-input-field">
            <label>${isDe ? "Zungenbreite" : "Stem Width"}</label>
            <div class="board-input-with-unit">
              <input type="number" id="bb-t-stem-w" value="${bb.tStemWidth ?? 800}" step="50" min="300" max="5000">
              <span class="unit-tag">mm</span>
            </div>
          </div>
        </div>
      `;
    }

    dimsCard.innerHTML = `
      <div class="board-section-title">${tBoard.shape} & ${isDe ? "Maße" : "Dimensions"}</div>
      
      <div class="board-inputs-row">
        <div class="board-input-field">
          <label>${tBoard.shape}</label>
          <select id="bb-shape" class="board-select">
            <option value="rect" ${currentShape === "rect" ? "selected" : ""}>${tBoard.shapes.rect}</option>
            <option value="l-shape" ${currentShape === "l-shape" ? "selected" : ""}>${tBoard.shapes.lShape}</option>
            <option value="u-shape" ${currentShape === "u-shape" ? "selected" : ""}>${tBoard.shapes.uShape}</option>
            <option value="t-shape" ${currentShape === "t-shape" ? "selected" : ""}>${tBoard.shapes.tShape}</option>
          </select>
        </div>
        <div class="board-input-field">
          <label>${isDe ? "Standard-Vorlagen" : "Presets"}</label>
          <select id="bb-presets-select" class="board-select">
            <option value="">${isDe ? "Vorlage wählen..." : "Choose preset..."}</option>
            <option value="2400x1200:rect">2400 × 1200 mm</option>
            <option value="2000x1000:rect">2000 × 1000 mm</option>
            <option value="1200x800:rect">1200 × 800 mm</option>
            <option value="2600x2000:l-shape">2600 × 2000 mm (L)</option>
            <option value="3200x2400:u-shape">3200 × 2400 mm (U)</option>
            <option value="2500x2000:t-shape">2500 × 2000 mm (T)</option>
          </select>
        </div>
      </div>

      <div class="board-inputs-row" style="margin-top: 4px;">
        <div class="board-input-field">
          <label>${isDe ? "Breite (X)" : "Width (X)"}</label>
          <div class="board-input-with-unit">
            <input type="number" id="bb-width" value="${bb.width}" step="50" min="200" max="20000">
            <span class="unit-tag">mm</span>
          </div>
        </div>
        <div class="board-input-field">
          <label>${isDe ? "Tiefe (Y)" : "Depth (Y)"}</label>
          <div class="board-input-with-unit">
            <input type="number" id="bb-depth" value="${bb.depth}" step="50" min="200" max="20000">
            <span class="unit-tag">mm</span>
          </div>
        </div>
      </div>

      ${shapeSpecificHTML}

      <button class="board-btn-fit" id="bb-fit-tracks" title="${
        isDe ? "Tisch automatisch an alle platzierten Gleise anpassen" : "Auto-calculate table size from placed tracks"
      }" style="margin-top: 6px;">
        ${tBoard.fitTracks}
      </button>
    `;

    const shapeSelect = dimsCard.querySelector("#bb-shape") as HTMLSelectElement;
    shapeSelect.addEventListener("change", () => {
      bb.shape = shapeSelect.value as any;
      this.callbacks.onBaseboardChange?.();
      this.render();
    });

    const presetSelect = dimsCard.querySelector("#bb-presets-select") as HTMLSelectElement;
    presetSelect.addEventListener("change", () => {
      const val = presetSelect.value;
      if (!val) return;
      const [dimStr, shape] = val.split(":");
      const [wStr, dStr] = dimStr!.split("x");
      bb.width = parseInt(wStr!, 10);
      bb.depth = parseInt(dStr!, 10);
      bb.shape = shape as any;
      this.callbacks.onBaseboardChange?.();
      this.render();
    });

    const inputW = dimsCard.querySelector("#bb-width") as HTMLInputElement;
    const inputD = dimsCard.querySelector("#bb-depth") as HTMLInputElement;
    inputW.addEventListener("change", () => {
      bb.width = Math.max(200, parseFloat(inputW.value) || 2400);
      this.callbacks.onBaseboardChange?.();
    });
    inputD.addEventListener("change", () => {
      bb.depth = Math.max(200, parseFloat(inputD.value) || 1200);
      this.callbacks.onBaseboardChange?.();
    });

    const lWingW = dimsCard.querySelector("#bb-l-wing-w") as HTMLInputElement | null;
    const lMainD = dimsCard.querySelector("#bb-l-main-d") as HTMLInputElement | null;
    const lSideSel = dimsCard.querySelector("#bb-l-side") as HTMLSelectElement | null;
    if (lWingW) lWingW.addEventListener("change", () => { bb.lWingWidth = parseFloat(lWingW.value) || 800; this.callbacks.onBaseboardChange?.(); });
    if (lMainD) lMainD.addEventListener("change", () => { bb.lWingDepth = parseFloat(lMainD.value) || 800; this.callbacks.onBaseboardChange?.(); });
    if (lSideSel) lSideSel.addEventListener("change", () => { bb.lSide = lSideSel.value as any; this.callbacks.onBaseboardChange?.(); });

    const uLeftW = dimsCard.querySelector("#bb-u-left-w") as HTMLInputElement | null;
    const uRightW = dimsCard.querySelector("#bb-u-right-w") as HTMLInputElement | null;
    const uBackD = dimsCard.querySelector("#bb-u-back-d") as HTMLInputElement | null;
    if (uLeftW) uLeftW.addEventListener("change", () => { bb.uLeftWingWidth = parseFloat(uLeftW.value) || 700; this.callbacks.onBaseboardChange?.(); });
    if (uRightW) uRightW.addEventListener("change", () => { bb.uRightWingWidth = parseFloat(uRightW.value) || 700; this.callbacks.onBaseboardChange?.(); });
    if (uBackD) uBackD.addEventListener("change", () => { bb.uBackDepth = parseFloat(uBackD.value) || 600; this.callbacks.onBaseboardChange?.(); });

    const tBarD = dimsCard.querySelector("#bb-t-bar-d") as HTMLInputElement | null;
    const tStemW = dimsCard.querySelector("#bb-t-stem-w") as HTMLInputElement | null;
    if (tBarD) tBarD.addEventListener("change", () => { bb.tBarDepth = parseFloat(tBarD.value) || 600; this.callbacks.onBaseboardChange?.(); });
    if (tStemW) tStemW.addEventListener("change", () => { bb.tStemWidth = parseFloat(tStemW.value) || 800; this.callbacks.onBaseboardChange?.(); });

    const fitBtn = dimsCard.querySelector("#bb-fit-tracks") as HTMLButtonElement;
    fitBtn.addEventListener("click", () => {
      if (this.layout.pieces.length === 0) {
        alert(isDe ? "Noch keine Gleise im Plan vorhanden." : "No tracks on the layout yet!");
        return;
      }
      let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
      for (const piece of this.layout.pieces) {
        for (const conn of piece.connectors) {
          if (conn.x < minX) minX = conn.x;
          if (conn.x > maxX) maxX = conn.x;
          if (conn.y < minY) minY = conn.y;
          if (conn.y > maxY) maxY = conn.y;
        }
      }
      const padding = 100;
      const fitW = Math.max(400, Math.ceil((maxX - minX + padding * 2) / 50) * 50);
      const fitD = Math.max(400, Math.ceil((maxY - minY + padding * 2) / 50) * 50);
      bb.width = fitW;
      bb.depth = fitD;
      bb.positionMode = "custom";
      bb.customX = Math.round(minX - padding);
      bb.customY = Math.round(minY - padding);
      this.callbacks.onBaseboardChange?.();
      this.render();
    });

    container.appendChild(dimsCard);

    // Position & Surface Style Card
    const styleCard = document.createElement("div");
    styleCard.className = "board-card";
    const hasOtherTables = baseboards.length > 1;

    styleCard.innerHTML = `
      <div class="board-section-title">${isDe ? "Position & Oberfläche" : "Position & Surface"}</div>
      
      <div class="board-inputs-row">
        <div class="board-input-field">
          <label>${tBoard.positioning}</label>
          <select id="bb-pos-mode" class="board-select">
            <option value="center" ${bb.positionMode === "center" ? "selected" : ""}>${isDe ? "Zentriert (0, 0)" : "Centered (0, 0)"}</option>
            <option value="top-left" ${bb.positionMode === "top-left" ? "selected" : ""}>${isDe ? "Oben-Links (0, 0)" : "Top-Left (0, 0)"}</option>
            <option value="custom" ${bb.positionMode === "custom" ? "selected" : ""}>${isDe ? "Benutzerdefiniert" : "Custom Offset"}</option>
          </select>
        </div>
        <div class="board-input-field">
          <label>${tBoard.style}</label>
          <select id="bb-style" class="board-select">
            <option value="wood" ${bb.style === "wood" ? "selected" : ""}>${isDe ? "Sperrholz hell" : "Birch Plywood"}</option>
            <option value="dark-wood" ${bb.style === "dark-wood" ? "selected" : ""}>${isDe ? "Eiche dunkel" : "Dark Walnut"}</option>
            <option value="plywood" ${bb.style === "plywood" ? "selected" : ""}>${isDe ? "Kiefer" : "Multiplex Pine"}</option>
            <option value="slate" ${bb.style === "slate" ? "selected" : ""}>${isDe ? "Schiefergrau" : "Slate Anthracite"}</option>
            <option value="white" ${bb.style === "white" ? "selected" : ""}>${isDe ? "Weiß" : "Blueprint White"}</option>
          </select>
        </div>
      </div>

      <div class="board-inputs-row" id="bb-custom-coords" style="display: ${bb.positionMode === "custom" ? "flex" : "none"}; margin-top: 4px;">
        <div class="board-input-field">
          <label>Offset X (mm)</label>
          <input type="number" id="bb-custom-x" value="${bb.customX ?? 0}" step="20" class="board-text-input">
        </div>
        <div class="board-input-field">
          <label>Offset Y (mm)</label>
          <input type="number" id="bb-custom-y" value="${bb.customY ?? 0}" step="20" class="board-text-input">
        </div>
      </div>

      ${
        hasOtherTables
          ? `
        <button class="board-btn-fit" id="bb-snap-right" style="margin-top: 4px;" title="${tBoard.snapToRight}">
          ${tBoard.snapToRight}
        </button>
      `
          : ""
      }

      <div class="board-inputs-row" style="margin-top: 4px;">
        <div class="board-input-field">
          <label>${isDe ? "Eckenrundung" : "Corner Radius"}</label>
          <div class="board-input-with-unit">
            <input type="number" id="bb-radius" value="${bb.cornerRadius ?? 12}" min="0" max="100">
            <span class="unit-tag">mm</span>
          </div>
        </div>
        <div class="board-input-field" style="display: flex; justify-content: flex-end; padding-bottom: 5px;">
          <label class="board-toggle-label" style="font-size: 11px;">
            <input type="checkbox" id="bb-show-dims" ${bb.showDimensions !== false ? "checked" : ""}>
            <span>${isDe ? "Maße anzeigen" : "Show Dimensions"}</span>
          </label>
        </div>
      </div>
    `;

    const posSelect = styleCard.querySelector("#bb-pos-mode") as HTMLSelectElement;
    const customCoords = styleCard.querySelector("#bb-custom-coords") as HTMLElement;
    const customXInput = styleCard.querySelector("#bb-custom-x") as HTMLInputElement;
    const customYInput = styleCard.querySelector("#bb-custom-y") as HTMLInputElement;
    const snapRightBtn = styleCard.querySelector("#bb-snap-right") as HTMLButtonElement | null;
    const styleSelect = styleCard.querySelector("#bb-style") as HTMLSelectElement;
    const radiusInput = styleCard.querySelector("#bb-radius") as HTMLInputElement;
    const showDimsInput = styleCard.querySelector("#bb-show-dims") as HTMLInputElement;

    posSelect.addEventListener("change", () => {
      bb.positionMode = posSelect.value as any;
      customCoords.style.display = bb.positionMode === "custom" ? "flex" : "none";
      this.callbacks.onBaseboardChange?.();
    });

    customXInput.addEventListener("change", () => {
      bb.customX = parseFloat(customXInput.value) || 0;
      this.callbacks.onBaseboardChange?.();
    });

    customYInput.addEventListener("change", () => {
      bb.customY = parseFloat(customYInput.value) || 0;
      this.callbacks.onBaseboardChange?.();
    });

    if (snapRightBtn) {
      snapRightBtn.addEventListener("click", () => {
        const prev = baseboards.find((x) => x.id !== bb.id);
        if (prev) {
          const pB = getBaseboardBounds(prev);
          bb.positionMode = "custom";
          bb.customX = Math.round(pB.x + pB.width + 20);
          bb.customY = Math.round(pB.y);
          this.callbacks.onBaseboardChange?.();
          this.render();
        }
      });
    }

    styleSelect.addEventListener("change", () => {
      bb.style = styleSelect.value as any;
      this.callbacks.onBaseboardChange?.();
    });

    radiusInput.addEventListener("change", () => {
      bb.cornerRadius = Math.max(0, parseFloat(radiusInput.value) || 0);
      this.callbacks.onBaseboardChange?.();
    });

    showDimsInput.addEventListener("change", () => {
      bb.showDimensions = showDimsInput.checked;
      this.callbacks.onBaseboardChange?.();
    });

    container.appendChild(styleCard);
  }

  // =========================================================================
  // TAB 3: BILL OF MATERIALS (BOM)
  // =========================================================================
  private renderBomTab(container: HTMLElement, summary: ReturnType<typeof getPieceListSummary>) {
    const tBom = getTranslations().sidebar.bom;

    const filterBar = document.createElement("div");
    filterBar.className = "bom-filter-bar";
    filterBar.innerHTML = `
      <button type="button" class="bom-filter-btn ${this.activePartsCategory === "all" ? "active" : ""}" data-cat="all">
        ${tBom.categories.all} (${summary.totalObjects})
      </button>
      <button type="button" class="bom-filter-btn ${this.activePartsCategory === "tracks" ? "active" : ""}" data-cat="tracks">
        ${tBom.categories.tracks} (${summary.tracks.totalPieces})
      </button>
      <button type="button" class="bom-filter-btn ${this.activePartsCategory === "roads" ? "active" : ""}" data-cat="roads">
        ${tBom.categories.roads} (${summary.roads.totalPieces})
      </button>
      <button type="button" class="bom-filter-btn ${this.activePartsCategory === "buildings" ? "active" : ""}" data-cat="buildings">
        ${tBom.categories.buildings} (${summary.buildings.totalPieces})
      </button>
    `;

    filterBar.querySelectorAll(".bom-filter-btn").forEach((btn) => {
      btn.addEventListener("click", () => {
        const cat = btn.getAttribute("data-cat") as typeof this.activePartsCategory;
        if (cat) {
          this.activePartsCategory = cat;
          this.render();
        }
      });
    });
    container.appendChild(filterBar);

    const summaryBox = document.createElement("div");
    summaryBox.className = "bom-summary-box";

    if (this.activePartsCategory === "all") {
      summaryBox.innerHTML = `
        <div class="summary-metric">
          <span class="metric-val">${summary.totalObjects}</span>
          <span class="metric-lbl">${tBom.categories.all}</span>
        </div>
        <div class="summary-metric">
          <span class="metric-val">${summary.tracks.totalPieces}</span>
          <span class="metric-lbl">${summary.tracks.totalLengthMeters.toFixed(2)} m ${tBom.categories.tracks}</span>
        </div>
        <div class="summary-metric">
          <span class="metric-val">${summary.roads.totalPieces}</span>
          <span class="metric-lbl">${summary.roads.totalLengthMeters.toFixed(2)} m ${tBom.categories.roads}</span>
        </div>
      `;
    } else if (this.activePartsCategory === "tracks") {
      summaryBox.innerHTML = `
        <div class="summary-metric">
          <span class="metric-val">${summary.tracks.totalPieces}</span>
          <span class="metric-lbl">${getTranslations().toolbar.status.tracksUnitPlural}</span>
        </div>
        <div class="summary-metric">
          <span class="metric-val">${summary.tracks.totalLengthMeters.toFixed(2)} m</span>
          <span class="metric-lbl">${tBom.colLength}</span>
        </div>
      `;
    } else if (this.activePartsCategory === "roads") {
      summaryBox.innerHTML = `
        <div class="summary-metric">
          <span class="metric-val">${summary.roads.totalPieces}</span>
          <span class="metric-lbl">${tBom.categories.roads}</span>
        </div>
        <div class="summary-metric">
          <span class="metric-val">${summary.roads.totalLengthMeters.toFixed(2)} m</span>
          <span class="metric-lbl">${tBom.colLength}</span>
        </div>
      `;
    } else {
      summaryBox.innerHTML = `
        <div class="summary-metric">
          <span class="metric-val">${summary.buildings.totalPieces}</span>
          <span class="metric-lbl">${tBom.categories.buildings}</span>
        </div>
      `;
    }
    container.appendChild(summaryBox);

    const exportBtn = document.createElement("button");
    exportBtn.className = "btn-export-csv";
    exportBtn.innerHTML = tBom.exportCsvBtn;
    exportBtn.disabled = summary.totalObjects === 0;
    exportBtn.addEventListener("click", () => this.callbacks.onExportCSV());
    container.appendChild(exportBtn);

    if (summary.totalObjects === 0) {
      const empty = document.createElement("div");
      empty.className = "empty-state";
      empty.innerHTML = `<p>${tBom.emptyState}</p>`;
      container.appendChild(empty);
      return;
    }

    const showTracks = this.activePartsCategory === "all" || this.activePartsCategory === "tracks";
    const showRoads = this.activePartsCategory === "all" || this.activePartsCategory === "roads";
    const showBuildings = this.activePartsCategory === "all" || this.activePartsCategory === "buildings";

    if (showTracks) {
      if (summary.tracks.items.length > 0) {
        const secHeader = document.createElement("div");
        secHeader.className = "bom-section-header";
        secHeader.innerHTML = `<span>${tBom.tracksHeader}</span><span style="font-weight: 600; font-size: 10px; color: var(--text-muted);">${summary.tracks.totalPieces} Stk. • ${summary.tracks.totalLengthMeters.toFixed(2)} m</span>`;
        container.appendChild(secHeader);

        const table = document.createElement("table");
        table.className = "bom-table";
        table.innerHTML = `
          <thead>
            <tr>
              <th>${tBom.colId}</th>
              <th>${tBom.colName}</th>
              <th style="text-align: right;">${tBom.colCount}</th>
            </tr>
          </thead>
          <tbody>
            ${summary.tracks.items
              .map((it) => {
                const bp = getTrackBlueprintColor(it.catalogId);
                return `
              <tr>
                <td>
                  <span class="bom-id-badge" style="background: ${bp.badgeBg}; color: #ffffff; border: 1px solid ${bp.border};">
                    ${it.catalogId}
                  </span>
                </td>
                <td>
                  <div class="bom-name">${it.name}</div>
                  <div class="bom-sub">${it.specification}</div>
                </td>
                <td style="text-align: right;"><span class="badge-count">${it.count}x</span></td>
              </tr>
            `;
              })
              .join("")}
          </tbody>
        `;
        container.appendChild(table);
      } else if (this.activePartsCategory === "tracks") {
        const empty = document.createElement("div");
        empty.className = "empty-state";
        empty.innerHTML = `<p>${tBom.emptyCategory}</p>`;
        container.appendChild(empty);
      }
    }

    if (showRoads) {
      if (summary.roads.items.length > 0) {
        const secHeader = document.createElement("div");
        secHeader.className = "bom-section-header";
        secHeader.innerHTML = `<span>${tBom.roadsHeader}</span><span style="font-weight: 600; font-size: 10px; color: var(--text-muted);">${summary.roads.totalPieces} Stk. • ${summary.roads.totalLengthMeters.toFixed(2)} m</span>`;
        container.appendChild(secHeader);

        const table = document.createElement("table");
        table.className = "bom-table";
        table.innerHTML = `
          <thead>
            <tr>
              <th>${tBom.colId}</th>
              <th>${tBom.colName}</th>
              <th style="text-align: right;">${tBom.colCount}</th>
            </tr>
          </thead>
          <tbody>
            ${summary.roads.items
              .map(
                (it) => `
              <tr>
                <td>
                  <span class="bom-id-badge" style="background: #334155; color: #f8fafc; border: 1px solid #64748b;">
                    ${it.catalogId}
                  </span>
                </td>
                <td>
                  <div class="bom-name">${it.name}</div>
                  <div class="bom-sub">${it.specification}</div>
                </td>
                <td style="text-align: right;"><span class="badge-count">${it.count}x</span></td>
              </tr>
            `,
              )
              .join("")}
          </tbody>
        `;
        container.appendChild(table);
      } else if (this.activePartsCategory === "roads") {
        const empty = document.createElement("div");
        empty.className = "empty-state";
        empty.innerHTML = `<p>${tBom.emptyCategory}</p>`;
        container.appendChild(empty);
      }
    }

    if (showBuildings) {
      if (summary.buildings.items.length > 0) {
        const secHeader = document.createElement("div");
        secHeader.className = "bom-section-header";
        secHeader.innerHTML = `<span>${tBom.buildingsHeader}</span><span style="font-weight: 600; font-size: 10px; color: var(--text-muted);">${summary.buildings.totalPieces} Stk.</span>`;
        container.appendChild(secHeader);

        const table = document.createElement("table");
        table.className = "bom-table";
        table.innerHTML = `
          <thead>
            <tr>
              <th>${tBom.colName}</th>
              <th>${tBom.colDimensions}</th>
              <th style="text-align: right;">${tBom.colCount}</th>
            </tr>
          </thead>
          <tbody>
            ${summary.buildings.items
              .map(
                (it) => `
              <tr>
                <td>
                  <div class="bom-name" style="font-weight: 600;">${it.name}</div>
                  <div class="bom-sub">${it.category} • ${it.type}</div>
                </td>
                <td>
                  <span style="font-size: 10px; color: var(--text-muted); font-family: monospace;">${it.dimensions}</span>
                </td>
                <td style="text-align: right;"><span class="badge-count">${it.count}x</span></td>
              </tr>
            `,
              )
              .join("")}
          </tbody>
        `;
        container.appendChild(table);
      } else if (this.activePartsCategory === "buildings") {
        const empty = document.createElement("div");
        empty.className = "empty-state";
        empty.innerHTML = `<p>${tBom.emptyCategory}</p>`;
        container.appendChild(empty);
      }
    }
  }
}

/** Legacy helper */
export function renderCatalog(host: HTMLElement, catalog: Catalog) {
  new Sidebar(
    host,
    catalog,
    { name: "New Layout", scale: "H0", pieces: [] },
    {
      onAddPiece: () => {},
      onExportCSV: () => {},
    },
  );
}