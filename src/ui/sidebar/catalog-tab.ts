import type { CatalogEntry } from "@/catalog/catalog";
import {
  compareCatalogEntriesByNumber,
  isFeederEntry,
  isIsolatingEntry,
} from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/registry";
import {
  getLanguage,
  getTranslations,
  getTrackDisplayName,
  getTrackDisplayCategory,
  getTrackDisplayDescription,
} from "@/i18n";
import { getTrackBlueprintColor } from "@/render/blueprint-colors";
import {
  createCatalogThumbnail,
  attachDragImage,
  createPieceDragCanvas,
} from "../drag-preview";
import { CATEGORY_MAP, formatCatalogSubtitle } from "./shared";
import { setupDoubleAction, type Sidebar } from "../sidebar";

export function renderCatalogTab(sidebar: Sidebar, container: HTMLElement) {
  const t = getTranslations().sidebar;
  const isDe = getLanguage() === "de";

  const allCatalogs = catalogRegistry.getAll().filter((c) => !c.isRoadSystem);
  if (sidebar.catalog.isRoadSystem) {
    sidebar.catalog = allCatalogs[0] ?? catalogRegistry.getDefault();
  } else if (sidebar.catalog.id === "favorites") {
    sidebar.catalog = sidebar.getFavoritesCatalog();
  }

  let menuStep: "closed" | "manufacturers" | "systems" = "closed";
  let activeMfg =
    sidebar.catalog.id === "favorites" ? (allCatalogs[0]?.manufacturer ?? "Märklin") : sidebar.catalog.manufacturer;

  const selectorBox = document.createElement("div");
  selectorBox.className = "catalog-system-box";
  selectorBox.style.position = "relative";

  const catTitle =
    sidebar.catalog.id === "favorites"
      ? `★ ${sidebar.catalog.name}`
      : `${sidebar.catalog.manufacturer} — ${sidebar.catalog.system}`;

  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "catalog-dropdown-trigger";
  trigger.innerHTML = `
    <div class="catalog-trigger-text">
      <span class="catalog-trigger-title">${catTitle}</span>
      <span class="catalog-trigger-sub">${formatCatalogSubtitle(sidebar.catalog, isDe, t)}</span>
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

      // Top entry: Meine Favoriten
      const favCount = sidebar.getTotalFavoritesCount();
      const favItem = document.createElement("button");
      favItem.type = "button";
      const isFavActive = sidebar.catalog.id === "favorites";
      favItem.className = `popover-item ${isFavActive ? "active" : ""}`;
      favItem.style.color = "#f59e0b";
      favItem.style.fontWeight = "600";
      favItem.style.borderBottom = "1px solid rgba(255, 255, 255, 0.08)";
      favItem.style.marginBottom = "4px";
      favItem.innerHTML = `
        <div style="display: flex; align-items: center; gap: 6px;">
          <span>★</span>
          <span>${isDe ? "Meine Favoriten" : "My Favorites"}</span>
        </div>
        <span class="popover-item-sub" style="color: inherit; font-weight: bold;">${favCount}</span>
      `;
      favItem.addEventListener("click", (e) => {
        e.stopPropagation();
        menuStep = "closed";
        sidebar.catalog = sidebar.getFavoritesCatalog();
        sidebar.selectedCategory = "All";
        sidebar.render();
      });
      list.appendChild(favItem);

      const manufacturers = Array.from(new Set(allCatalogs.map((c) => c.manufacturer)));
      for (const mfg of manufacturers) {
        const item = document.createElement("button");
        item.type = "button";
        item.className = `popover-item ${mfg === sidebar.catalog.manufacturer ? "active" : ""}`;
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
        sidebar.callbacks.onImportCatalog?.();
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
        const isSelected = cat.id === sidebar.catalog.id;
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
          sidebar.catalog = cat;
          sidebar.callbacks.onSelectSystem?.(cat.id);
          menuStep = "closed";
          sidebar.render();
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
        sidebar.callbacks.onImportCatalog?.();
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

  sidebar.addWindowClickListener((e) => {
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
  input.value = sidebar.searchQuery;
  input.addEventListener("input", (e) => {
    sidebar.searchQuery = (e.target as HTMLInputElement).value;
    renderTrackList(sidebar, listContainer);
  });
  searchBox.appendChild(input);
  container.appendChild(searchBox);

  // Category Filter Bar
  const favCount = sidebar.getFavoritesCount();
  const categories: Array<{ id: string; label: string }> = [
    { id: "Favorites", label: `${t.catalog.categories.favorites ?? "★ Favoriten"} (${favCount})` },
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
    pill.className = `filter-pill ${sidebar.selectedCategory === cat.id ? "active" : ""}`;
    pill.textContent = cat.label;
    pill.addEventListener("click", () => {
      sidebar.selectedCategory = cat.id;
      sidebar.render();
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
  renderTrackList(sidebar, listContainer);
  container.appendChild(listContainer);
}

export function renderTrackList(sidebar: Sidebar, container: HTMLElement) {
  container.innerHTML = "";

  const currentLang = getLanguage();
  const isDe = currentLang === "de";
  const query = sidebar.searchQuery.toLowerCase().trim();
  const allowedCats = CATEGORY_MAP[sidebar.selectedCategory] ?? [sidebar.selectedCategory];

  const filtered = sidebar.catalog.entries.filter((e) => {
    const matchCat =
      sidebar.selectedCategory === "All"
        ? true
        : sidebar.selectedCategory === "Favorites"
        ? sidebar.isFavorite(e.id, (e as any)._originCatalogId || (sidebar.catalog.id !== "favorites" ? sidebar.catalog.id : undefined))
        : allowedCats.includes(e.category) || e.category === sidebar.selectedCategory;
    const matchQuery =
      !query ||
      e.id.toLowerCase().includes(query) ||
      e.name.toLowerCase().includes(query) ||
      getTrackDisplayName(e, "en").toLowerCase().includes(query) ||
      getTrackDisplayName(e, "de").toLowerCase().includes(query) ||
      e.description.toLowerCase().includes(query);
    return matchCat && matchQuery;
  });

  filtered.sort(compareCatalogEntriesByNumber);

  if (filtered.length === 0) {
    if (sidebar.selectedCategory === "Favorites" || sidebar.catalog.id === "favorites") {
      const totalFavs = sidebar.getTotalFavoritesCount();
      const emptyBox = document.createElement("div");
      emptyBox.className = "empty-state";
      emptyBox.style.padding = "16px 8px";
      emptyBox.style.textAlign = "center";
      emptyBox.innerHTML = `
        <div style="font-size: 24px; margin-bottom: 8px; color: #f59e0b;">★</div>
        <div style="font-weight: 600; margin-bottom: 4px;">
          ${isDe ? "Keine Favoriten gefunden" : "No favorites found"}
        </div>
        <div style="font-size: 11px; opacity: 0.8; margin-bottom: 12px;">
          ${isDe
            ? "Klicke auf den Stern (★) bei einem Gleis, um es als Favorit zu speichern."
            : "Click the star (★) on any track card to save it as a favorite."}
        </div>
      `;
      if (totalFavs > 0 && sidebar.catalog.id !== "favorites") {
        const viewAllBtn = document.createElement("button");
        viewAllBtn.type = "button";
        viewAllBtn.className = "btn-secondary";
        viewAllBtn.style.padding = "4px 10px";
        viewAllBtn.style.fontSize = "11px";
        viewAllBtn.textContent = isDe
          ? `Alle Favoriten anzeigen (${totalFavs})`
          : `Show all favorites (${totalFavs})`;
        viewAllBtn.addEventListener("click", () => {
          sidebar.catalog = sidebar.getFavoritesCatalog();
          sidebar.selectedCategory = "All";
          sidebar.render();
        });
        emptyBox.appendChild(viewAllBtn);
      }
      container.appendChild(emptyBox);
      return;
    }

    container.innerHTML = `<div class="empty-state">${
      isDe ? "Keine passenden Gleise gefunden." : "No matching tracks found."
    }</div>`;
    return;
  }

  for (const entry of filtered) {
    const card = document.createElement("div");
    card.className = "track-card";
    card.draggable = true;

    const bp = getTrackBlueprintColor(entry.id);
    const entryGauge = entry.gauge ?? sidebar.catalog.gauge;

    const isFeeder = isFeederEntry(entry);
    const isIso = isIsolatingEntry(entry);
    const powerEnabled = isFeeder ? sidebar.isEntryPowerEnabled(entry) : false;
    const isoEnabled = isIso ? sidebar.isEntryIsolatingEnabled(entry) : false;

    const powerTitle = powerEnabled
      ? isDe
        ? "Gleis-Einspeisung aktiv (Klicken zum Ausschalten)"
        : "Track power feed active (click to disable)"
      : isDe
      ? "Gleis-Einspeisung hinzufügen (Klicken zum Einschalten)"
      : "Add power feed to track (click to enable)";

    const isoTitle = isoEnabled
      ? isDe
        ? "Trennstrecke aktiv (Klicken zum Ausschalten)"
        : "Track isolator active (click to disable)"
      : isDe
      ? "Trennstrecke isolieren (Klicken zum Einschalten)"
      : "Isolate track (click to enable)";

    const entryPayload: CatalogEntry = {
      ...entry,
      _initialPowerFeed: isFeeder ? powerEnabled : undefined,
      _initialIsolator: isIso ? isoEnabled : undefined,
    } as any;

    // Dragstart: Bind JSON payload + directly render 2D Track Preview
    card.addEventListener("dragstart", (e) => {
      if ((e.target as HTMLElement).closest(".favorite-track-btn, .track-feeder-toggle-btn, .track-isolator-toggle-btn")) {
        e.preventDefault();
        return;
      }

      card.classList.add("dragging");
      e.dataTransfer?.setData("application/json", JSON.stringify(entryPayload));
      e.dataTransfer?.setData("text/plain", entry.id);

      if (e.dataTransfer) {
        e.dataTransfer.effectAllowed = "copy";
        const zoom = sidebar.callbacks.getZoom?.() ?? 1.0;
        const canvas = createPieceDragCanvas(entryPayload, false, undefined, entryGauge, zoom);
        attachDragImage(e.dataTransfer, canvas);
      }
    });

    card.addEventListener("dragend", () => {
      card.classList.remove("dragging");
    });

    card.title = isDe
      ? "Doppelklick: An freies Gleisende anfügen / Ziehen: Frei platzieren"
      : "Double-click: Attach to open snap point / Drag: Place freely";

    setupDoubleAction(card, () => sidebar.callbacks.onAddPiece?.(entryPayload));

    let spec = "";
    if (entry.length) spec += `L: ${entry.length} mm `;
    if (entry.radius) spec += `R: ${entry.radius} mm `;
    if (entry.angle) spec += `/ ${entry.angle}° `;
    if (entry.branchAngle) spec += `Branch: ${entry.branchAngle}°`;

    const displayName = getTrackDisplayName(entry, currentLang);
    const displayCategory = getTrackDisplayCategory(entry.category, currentLang);
    const displayDesc = getTrackDisplayDescription(entry, currentLang);
    const thumb = createCatalogThumbnail(entry, false, undefined, entryGauge, 60, 40);

    const isFav = sidebar.isFavorite(
      entry.id,
      (entry as any)._originCatalogId || (sidebar.catalog.id !== "favorites" ? sidebar.catalog.id : undefined)
    );
    const favTitle = isFav
      ? isDe
        ? "Aus Favoriten entfernen"
        : "Remove from favorites"
      : isDe
      ? "Zu Favoriten hinzufügen"
      : "Add to favorites";

    const categoryBadge =
      sidebar.catalog.id === "favorites" && (entry as any)._originCatalogName
        ? (entry as any)._originCatalogName
        : displayCategory;

    card.innerHTML = `
      <div class="card-top">
        <div style="display: flex; align-items: center; gap: 4px;">
          <button type="button" class="favorite-track-btn ${isFav ? "active" : ""}" title="${favTitle}" aria-label="${favTitle}">
            ${isFav ? "★" : "☆"}
          </button>
          ${
            isFeeder
              ? `<button type="button" class="track-feeder-toggle-btn ${powerEnabled ? "active" : "disabled"}" title="${powerTitle}" aria-label="${powerTitle}">⚡</button>`
              : ""
          }
          ${
            isIso
              ? `<button type="button" class="track-isolator-toggle-btn ${isoEnabled ? "active" : "disabled"}" title="${isoTitle}" aria-label="${isoTitle}">⊘</button>`
              : ""
          }
          <span class="card-color-dot" style="background: ${bp.css};" title="Blueprint Color"></span>
          <span class="card-id">${entry.id}</span>
        </div>
        <span class="card-category-badge">${categoryBadge}</span>
      </div>
      <div class="card-body">
        <div class="card-info">
          <div class="card-name" title="${displayDesc}">${displayName}</div>
          <div class="card-spec">${spec}</div>
        </div>
        <div class="card-preview" title="${displayName}"></div>
      </div>
    `;

    const favBtn = card.querySelector<HTMLButtonElement>(".favorite-track-btn");
    if (favBtn) {
      favBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        e.preventDefault();
        sidebar.toggleFavorite(
          entry.id,
          (entry as any)._originCatalogId || (sidebar.catalog.id !== "favorites" ? sidebar.catalog.id : undefined)
        );
        sidebar.render();
      });
    }

    const feederBtn = card.querySelector<HTMLButtonElement>(".track-feeder-toggle-btn");
    if (feederBtn) {
      feederBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        e.preventDefault();
        sidebar.toggleEntryPower(entry);
        sidebar.render();
      });
    }

    const isoBtn = card.querySelector<HTMLButtonElement>(".track-isolator-toggle-btn");
    if (isoBtn) {
      isoBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        e.preventDefault();
        sidebar.toggleEntryIsolating(entry);
        sidebar.render();
      });
    }

    const previewContainer = card.querySelector(".card-preview");
    if (previewContainer && thumb && typeof thumb.getContext === "function") {
      previewContainer.appendChild(thumb);
    }

    container.appendChild(card);
  }
}
