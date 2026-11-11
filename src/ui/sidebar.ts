import {
  catalogRegistry,
  type Catalog,
  type CatalogEntry,
} from "@/catalog/catalog";
import type { Layout, Model3D } from "@/core/types";
import type { SimulationSummary } from "@/core/simulation";
import { ensureDefaultLayers } from "@/core/layout";
import {
  getTranslations,
  onLanguageChange,
  getLanguage,
} from "@/i18n";
import { getPieceListSummary } from "@/io/export";
import { BUILTIN_MODELS } from "@/core/models-3d";
import type { ModelScale } from "@/core/model-scale";
import { findRoadCatalogForScale } from "./sidebar/shared";
import { renderCatalogTab } from "./sidebar/catalog-tab";
import { renderRoadsTab } from "./sidebar/roads-tab";
import { renderModelsTab } from "./sidebar/models-tab";
import { renderLayersTab } from "./sidebar/layers-tab";
import { renderBoardTab } from "./sidebar/board-tab";
import { renderBomTab } from "./sidebar/bom-tab";

export * from "./sidebar/shared";
export {
  renderCatalogTab,
  renderRoadsTab,
  renderModelsTab,
  renderLayersTab,
  renderBoardTab,
  renderBomTab,
};

export type { ModelScale };
export type SidebarTab = "catalog" | "roads" | "models" | "board" | "bom" | "layers";

export function setupDoubleAction(
  element: { addEventListener: (event: string, handler: (e: any) => void) => void },
  onTrigger: () => void,
  tapThresholdMs = 350
): void {
  let lastTap = 0;
  element.addEventListener("pointerdown", (e: any) => {
    if (e.pointerType === "touch") {
      const now = Date.now();
      if (now - lastTap < tapThresholdMs) {
        e.preventDefault?.();
        onTrigger();
        lastTap = 0;
        return;
      }
      lastTap = now;
    }
  });
  element.addEventListener("dblclick", (e: any) => {
    e.preventDefault?.();
    onTrigger();
  });
}

export interface SidebarCallbacks {
  onAddPiece?: (entry: CatalogEntry) => void;
  onAddModel?: (model: Model3D) => void;
  onExportCSV: () => void;
  onImportCatalog?: () => void;
  onSelectSystem?: (system: string) => void;
  onSelectRoadScale?: (scale: ModelScale) => void;
  onLayerChange?: () => void;
  onMoveSelectedToLayer?: (layerId: string) => void;
  onToggleSelectionLock?: () => void;
  isSelectionLockDisabled?: () => boolean;
  onToggleWoodPlate?: () => void;
  isWoodPlateActive?: () => boolean;
  selectedPieceIds?: () => string[];
  onBaseboardChange?: () => void;
  getZoom?: () => number;
  onTabChange?: (tab: SidebarTab) => void;
}

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
  --popover-header-text: var(--text-muted, rgba(0, 0, 0, 0.6));
  --popover-hover-bg: var(--cl-glass-hover, rgba(0, 0, 0, 0.05));
  --popover-shadow: 0 10px 25px rgba(0, 0, 0, 0.15);
}

.catalog-system-box {
  margin-bottom: 8px;
}

.catalog-dropdown-trigger {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-surface);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  color: var(--text-main);
  cursor: pointer;
  text-align: left;
  transition: all 0.15s ease;
}

.catalog-dropdown-trigger:hover {
  background: var(--bg-hover);
  border-color: var(--theme-accent, #38bdf8);
}

.catalog-trigger-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow: hidden;
}

.catalog-trigger-title {
  font-weight: 600;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.catalog-trigger-sub {
  font-size: 11px;
  color: var(--text-muted);
}

.catalog-dropdown-arrow {
  font-size: 10px;
  color: var(--text-muted);
  margin-left: 8px;
}

.two-step-popover {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 100;
  background: var(--popover-bg);
  border: 1px solid var(--popover-border);
  border-radius: 8px;
  box-shadow: var(--popover-shadow);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  backdrop-filter: blur(12px);
}

.popover-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--popover-header-bg);
  border-bottom: 1px solid var(--popover-header-border);
  font-size: 11px;
  font-weight: 600;
  color: var(--popover-header-text);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.popover-back-btn {
  background: transparent;
  border: none;
  color: var(--text-main);
  cursor: pointer;
  padding: 0 4px;
  font-size: 14px;
  line-height: 1;
  border-radius: 4px;
}

.popover-back-btn:hover {
  background: var(--popover-hover-bg);
}

.popover-list {
  max-height: 280px;
  overflow-y: auto;
  padding: 4px;
}

.popover-item {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-main);
  font-size: 12px;
  cursor: pointer;
  text-align: left;
  transition: background 0.12s ease;
}

.popover-item:hover {
  background: var(--popover-hover-bg);
}

.popover-item.active {
  background: var(--theme-accent, #0284c7);
  color: #ffffff;
  font-weight: 600;
}

.popover-item-sub {
  font-size: 10px;
  color: var(--text-muted);
  margin-top: 1px;
}

.popover-item.active .popover-item-sub {
  color: rgba(255, 255, 255, 0.8);
}

.favorite-track-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  font-size: 14px;
  padding: 2px 4px;
  line-height: 1;
  color: var(--text-muted, #94a3b8);
  transition: transform 0.15s ease, color 0.15s ease;
  user-select: none;
}

.favorite-track-btn:hover {
  transform: scale(1.2);
  color: #f59e0b;
}

.favorite-track-btn.active {
  color: #f59e0b !important;
  text-shadow: 0 0 6px rgba(245, 158, 11, 0.5);
}

.track-feeder-toggle-btn,
.track-isolator-toggle-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  font-size: 13px;
  padding: 2px 4px;
  line-height: 1;
  transition: transform 0.15s ease, filter 0.15s ease, opacity 0.15s ease;
  user-select: none;
}

.track-feeder-toggle-btn.disabled,
.track-isolator-toggle-btn.disabled {
  filter: grayscale(100%);
  opacity: 0.35;
}

.track-feeder-toggle-btn.active {
  filter: none;
  opacity: 1;
  color: #eab308 !important;
  text-shadow: 0 0 6px rgba(234, 179, 8, 0.5);
}

.track-isolator-toggle-btn.active {
  filter: none;
  opacity: 1;
  color: #ef4444 !important;
  text-shadow: 0 0 6px rgba(239, 68, 68, 0.4);
}

.track-feeder-toggle-btn:hover,
.track-isolator-toggle-btn:hover {
  transform: scale(1.18);
  opacity: 1 !important;
}
`;

function ensureSidebarDrilldownStyles() {
  if (document.getElementById(SIDEBAR_DRILLDOWN_STYLES_ID)) return;
  const styleEl = document.createElement("style");
  styleEl.id = SIDEBAR_DRILLDOWN_STYLES_ID;
  styleEl.textContent = SIDEBAR_DRILLDOWN_STYLES;
  document.head.appendChild(styleEl);
}

export class Sidebar {
  activeTab: SidebarTab = "catalog";
  searchQuery = "";
  selectedCategory = "All";
  roadSearchQuery = "";
  selectedRoadCategory = "All";
  roadCatalog: Catalog;
  currentRoadScale: ModelScale = "H0";
  modelsSearchQuery = "";
  selectedModelCategory = "All";
  currentModelScale: ModelScale = "H0";
  customModelsLibrary: Model3D[] = [];
  activeBoardId?: string;
  openPlateLayers = new Set<string>();
  surfaceOpen = false;
  activePartsCategory: "all" | "tracks" | "roads" | "buildings" = "all";
  host: HTMLElement;
  catalog: Catalog;
  layout: Layout;
  simulation: SimulationSummary | null = null;
  callbacks: SidebarCallbacks;
  favoriteItems: Array<{ catalogId?: string; trackId: string }> = [];
  entryPowerEnabled: Map<string, boolean> = new Map();
  entryIsolatingEnabled: Map<string, boolean> = new Map();
  windowCleanups: Array<() => void> = [];

  constructor(
    host: HTMLElement,
    catalog: Catalog,
    layout: Layout,
    callbacks: SidebarCallbacks,
  ) {
    this.host = host;
    ensureSidebarDrilldownStyles();
    this.loadFavorites();

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

  /** Loads favorite tracks from localStorage. */
  loadFavorites() {
    this.favoriteItems = [];
    if (typeof localStorage === "undefined") return;
    try {
      const raw = localStorage.getItem("railbench_favorite_tracks");
      if (!raw) return;
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        for (const item of parsed) {
          if (typeof item === "string") {
            const colonIdx = item.indexOf(":");
            if (colonIdx > 0) {
              this.favoriteItems.push({
                catalogId: item.substring(0, colonIdx),
                trackId: item.substring(colonIdx + 1),
              });
            } else {
              this.favoriteItems.push({ trackId: item });
            }
          } else if (item && typeof item === "object" && typeof item.trackId === "string") {
            this.favoriteItems.push({
              catalogId: typeof item.catalogId === "string" ? item.catalogId : undefined,
              trackId: item.trackId,
            });
          }
        }
      }
    } catch (err) {
      console.warn("Could not load favorite tracks from localStorage:", err);
    }
  }

  /** Saves favorite tracks to localStorage. */
  saveFavorites() {
    if (typeof localStorage === "undefined") return;
    try {
      localStorage.setItem("railbench_favorite_tracks", JSON.stringify(this.favoriteItems));
    } catch (err) {
      console.warn("Could not save favorite tracks to localStorage:", err);
    }
  }

  /** Checks whether a track is currently favorited. */
  isFavorite(trackId: string, catalogId?: string): boolean {
    return this.favoriteItems.some((item) => {
      if (item.trackId !== trackId) return false;
      if (!catalogId || !item.catalogId) return true;
      return item.catalogId === catalogId;
    });
  }

  /** Toggles favorite status for a track. Returns true if now favorited, false if removed. */
  toggleFavorite(trackId: string, catalogId?: string): boolean {
    const existingIndex = this.favoriteItems.findIndex((item) => {
      if (item.trackId !== trackId) return false;
      if (!catalogId || !item.catalogId) return true;
      return item.catalogId === catalogId;
    });

    let isNowFavorite = false;
    if (existingIndex >= 0) {
      this.favoriteItems.splice(existingIndex, 1);
      isNowFavorite = false;
    } else {
      this.favoriteItems.push({
        catalogId: catalogId && catalogId !== "favorites" ? catalogId : undefined,
        trackId,
      });
      isNowFavorite = true;
    }
    this.saveFavorites();
    return isNowFavorite;
  }

  /** Returns all stored favorite items. */
  getFavoriteItems(): Array<{ catalogId?: string; trackId: string }> {
    return [...this.favoriteItems];
  }

  /** Number of favorites in the current catalog (or total if in favorites catalog). */
  getFavoritesCount(): number {
    if (this.catalog.id === "favorites") {
      return this.catalog.entries.length;
    }
    return this.catalog.entries.filter((e) => this.isFavorite(e.id, this.catalog.id)).length;
  }

  /** Total number of favorites across all catalogs. */
  getTotalFavoritesCount(): number {
    return this.favoriteItems.length;
  }

  /** Constructs a virtual Catalog containing all saved favorites across track systems. */
  getFavoritesCatalog(): Catalog {
    const isDe = getLanguage() === "de";
    const allCatalogs = catalogRegistry.getAll().filter((c) => !c.isRoadSystem);
    const entries: CatalogEntry[] = [];
    const seen = new Set<string>();

    for (const item of this.favoriteItems) {
      let matchedEntry: CatalogEntry | undefined;
      let originCat: Catalog | undefined;

      if (item.catalogId) {
        originCat = allCatalogs.find((c) => c.id === item.catalogId);
        matchedEntry = originCat?.entries.find((e) => e.id === item.trackId);
      }
      if (!matchedEntry) {
        for (const cat of allCatalogs) {
          matchedEntry = cat.entries.find((e) => e.id === item.trackId);
          if (matchedEntry) {
            originCat = cat;
            break;
          }
        }
      }

      if (matchedEntry && originCat) {
        const uniqueKey = `${originCat.id}:${matchedEntry.id}`;
        if (!seen.has(uniqueKey)) {
          seen.add(uniqueKey);
          entries.push({
            ...matchedEntry,
            gauge: matchedEntry.gauge ?? originCat.gauge,
            description: matchedEntry.description || `${originCat.manufacturer} ${originCat.system}`,
            _originCatalogId: originCat.id,
            _originCatalogName: `${originCat.manufacturer} ${originCat.system}`,
          } as CatalogEntry);
        }
      }
    }

    return {
      id: "favorites",
      name: isDe ? "Meine Favoriten" : "My Favorites",
      manufacturer: isDe ? "Favoriten" : "Favorites",
      system: isDe ? "Gespeicherte Gleise" : "Saved Tracks",
      scale: this.layout.scale || "H0",
      gauge: 16.5,
      entries,
    };
  }

  /** Checks whether power feed is currently enabled for a feeder track. Defaults to false (grey) until toggled. */
  isEntryPowerEnabled(entry: CatalogEntry): boolean {
    if (this.entryPowerEnabled.has(entry.id)) {
      return this.entryPowerEnabled.get(entry.id)!;
    }
    return false;
  }

  /** Toggles power feed for a feeder track. */
  toggleEntryPower(entry: CatalogEntry): boolean {
    const current = this.isEntryPowerEnabled(entry);
    this.entryPowerEnabled.set(entry.id, !current);
    return !current;
  }

  /** Checks whether track isolation is currently enabled for an isolating track. Defaults to false (grey) until toggled. */
  isEntryIsolatingEnabled(entry: CatalogEntry): boolean {
    if (this.entryIsolatingEnabled.has(entry.id)) {
      return this.entryIsolatingEnabled.get(entry.id)!;
    }
    return false;
  }

  /** Toggles track isolation for an isolating track. */
  toggleEntryIsolating(entry: CatalogEntry): boolean {
    const current = this.isEntryIsolatingEnabled(entry);
    this.entryIsolatingEnabled.set(entry.id, !current);
    return !current;
  }

  /** Registers a window click listener that is automatically removed on the next render(). */
  addWindowClickListener(handler: (e: MouseEvent) => void) {
    window.addEventListener("click", handler);
    this.windowCleanups.push(() => window.removeEventListener("click", handler));
  }

  render() {
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
      renderCatalogTab(this, content);
    } else if (this.activeTab === "roads") {
      renderRoadsTab(this, content);
    } else if (this.activeTab === "models") {
      renderModelsTab(this, content);
    } else if (this.activeTab === "board") {
      renderBoardTab(this, content);
    } else if (this.activeTab === "bom") {
      renderBomTab(this, content, bomSummary);
    } else if (this.activeTab === "layers") {
      renderLayersTab(this, content);
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
    const b = document.createElement("button");
    b.className = `tab-btn ${this.activeTab === tab ? "active" : ""}`;
    b.title = tooltip;
    b.innerHTML = `<span class="tab-label">${label}</span>`;
    b.addEventListener("click", () => {
      this.activeTab = tab;
      this.callbacks.onTabChange?.(tab);
      this.render();
    });
    return b;
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
