import { compareCatalogEntriesByNumber } from "@/catalog/catalog";
import {
  getLanguage,
  getTranslations,
  getTrackDisplayName,
  getTrackDisplayCategory,
  getTrackDisplayDescription,
} from "@/i18n";
import {
  createCatalogThumbnail,
  attachDragImage,
  createPieceDragCanvas,
} from "../drag-preview";
import {
  AVAILABLE_SCALES,
  CATEGORY_MAP,
  SCALE_DESCRIPTIONS,
  findRoadCatalogForScale,
  formatCatalogSubtitle,
  formatCatalogTitle,
  getRoadWidthMm,
} from "./shared";
import { setupDoubleAction, type Sidebar } from "../sidebar";

export function renderRoadsTab(sidebar: Sidebar, container: HTMLElement) {
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
      <span class="catalog-trigger-title">${formatCatalogTitle(sidebar.roadCatalog)}</span>
      <span class="catalog-trigger-sub">${formatCatalogSubtitle(sidebar.roadCatalog, isDe, t)}</span>
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
      item.className = `popover-item ${sc === sidebar.currentRoadScale ? "active" : ""}`;
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
        ${sc === sidebar.currentRoadScale ? "<span>✓</span>" : ""}
      `;
      item.addEventListener("click", (e) => {
        e.stopPropagation();
        if (!cat) return;
        sidebar.currentRoadScale = sc;
        sidebar.roadCatalog = cat;
        sidebar.callbacks.onSelectRoadScale?.(sc);
        sidebar.callbacks.onSelectSystem?.(cat.id);
        isMenuOpen = false;
        sidebar.render();
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

  sidebar.addWindowClickListener((e) => {
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
  input.value = sidebar.roadSearchQuery;
  input.addEventListener("input", (e) => {
    sidebar.roadSearchQuery = (e.target as HTMLInputElement).value;
    renderRoadsList(sidebar, listContainer);
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
    pill.className = `filter-pill ${sidebar.selectedRoadCategory === cat.id ? "active" : ""}`;
    pill.textContent = cat.label;
    pill.addEventListener("click", () => {
      sidebar.selectedRoadCategory = cat.id;
      sidebar.render();
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
  renderRoadsList(sidebar, listContainer);
  container.appendChild(listContainer);
}

export function renderRoadsList(sidebar: Sidebar, container: HTMLElement) {
  container.innerHTML = "";

  const currentLang = getLanguage();
  const isDe = currentLang === "de";
  const query = sidebar.roadSearchQuery.toLowerCase().trim();
  const allowedCats = CATEGORY_MAP[sidebar.selectedRoadCategory] ?? [sidebar.selectedRoadCategory];

  const filtered = sidebar.roadCatalog.entries.filter((e) => {
    const matchCat =
      sidebar.selectedRoadCategory === "All" ||
      allowedCats.includes(e.category) ||
      e.category === sidebar.selectedRoadCategory;
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
    container.innerHTML = `<div class="empty-state">${
      isDe ? "Keine passenden Straßen gefunden." : "No matching roads found."
    }</div>`;
    return;
  }

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
        const zoom = sidebar.callbacks.getZoom?.() ?? 1.0;
        const canvas = createPieceDragCanvas(entry, true, getRoadWidthMm(sidebar.roadCatalog.scale), undefined, zoom);
        attachDragImage(e.dataTransfer, canvas);
      }
    });

    card.addEventListener("dragend", () => {
      card.classList.remove("dragging");
    });

    card.title = isDe
      ? "Doppelklick: An freies Straßenende anfügen / Ziehen: Frei platzieren"
      : "Double-click: Attach to open road snap point / Drag: Place freely";

    setupDoubleAction(card, () => sidebar.callbacks.onAddPiece?.(entry));

    let spec = "";
    if (entry.length) spec += `L: ${entry.length} mm `;
    if (entry.radius) spec += `R: ${entry.radius} mm `;
    if (entry.angle) spec += `/ ${entry.angle}° `;
    if (entry.branchAngle) spec += `Branch: ${entry.branchAngle}°`;

    const displayName = getTrackDisplayName(entry, currentLang);
    const displayCategory = getTrackDisplayCategory(entry.category, currentLang);
    const displayDesc = getTrackDisplayDescription(entry, currentLang);
    const thumb = createCatalogThumbnail(entry, true, getRoadWidthMm(sidebar.roadCatalog.scale), undefined, 60, 40);

    card.innerHTML = `
      <div class="card-top">
        <div style="display: flex; align-items: center; gap: 4px;">
          <span class="card-color-dot" style="background: #475569;" title="Road"></span>
          <span class="card-id">${entry.id}</span>
        </div>
        <span class="card-category-badge">${displayCategory}</span>
      </div>
      <div class="card-body">
        <div class="card-info">
          <div class="card-name" title="${displayDesc}">${displayName}</div>
          <div class="card-spec">${spec}</div>
        </div>
        <div class="card-preview" title="${displayName}"></div>
      </div>
    `;

    const previewContainer = card.querySelector(".card-preview");
    if (previewContainer && thumb && typeof thumb.getContext === "function") {
      previewContainer.appendChild(thumb);
    }

    container.appendChild(card);
  }
}
