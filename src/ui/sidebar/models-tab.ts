import type { Model3D } from "@/core/types";
import {
  BUILTIN_MODELS,
  createBuiltinModel,
  parseGlbOrGltfFile,
} from "@/core/models-3d";
import {
  MODEL_SCALE_RATIOS,
  getScaleFactor,
  scaleDimensions,
} from "@/core/model-scale";
import { getLanguage, getTranslations } from "@/i18n";
import {
  createModelDragCanvas,
  createCustomModelDragCanvas,
  attachDragImage,
} from "@/ui/drag-preview";
import {
  AVAILABLE_SCALES,
  SCALE_DESCRIPTIONS,
  MODEL_CATEGORIES,
  MODEL_CATEGORY_LABELS_DE,
} from "./shared";
import type { Sidebar } from "../sidebar";

export function getModelScaleFactor(sidebar: Sidebar): number {
  return getScaleFactor(sidebar.currentModelScale);
}

export function createScaledBuiltin(sidebar: Sidebar, id: string): Model3D {
  return scaleDimensions(createBuiltinModel(id), sidebar.currentModelScale);
}

export async function handleImportModel(sidebar: Sidebar, file: File) {
  try {
    const model = await parseGlbOrGltfFile(file);
    if (!sidebar.customModelsLibrary.some((m) => m.name === model.name)) {
      sidebar.customModelsLibrary.push(model);
    }
    sidebar.callbacks.onAddModel?.(model);
    sidebar.render();
  } catch (err: any) {
    alert(`Error importing 3D model: ${err.message || err}`);
  }
}

export function renderModelsTab(sidebar: Sidebar, container: HTMLElement) {
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
    for (const f of files) await handleImportModel(sidebar, f);
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
    for (const f of files) await handleImportModel(sidebar, f);
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
      <span class="catalog-trigger-title">${isDe ? "Spur" : "Scale"} ${sidebar.currentModelScale} (1:${MODEL_SCALE_RATIOS[sidebar.currentModelScale]})</span>
      <span class="catalog-trigger-sub">${
        isDe ? SCALE_DESCRIPTIONS[sidebar.currentModelScale].de : SCALE_DESCRIPTIONS[sidebar.currentModelScale].en
      }</span>
    </div>
    <span class="catalog-dropdown-arrow">▾</span>
  `;

  const popover = document.createElement("div");
  popover.className = "two-step-popover";
  popover.style.display = "none";

  const updateScalePopover = () => {
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
      item.className = `popover-item ${sc === sidebar.currentModelScale ? "active" : ""}`;
      item.innerHTML = `
        <div>
          <div style="font-weight: 600;">Spur ${sc}</div>
          <div class="popover-item-sub">1:${MODEL_SCALE_RATIOS[sc]} · ${
            isDe ? SCALE_DESCRIPTIONS[sc].de : SCALE_DESCRIPTIONS[sc].en
          }</div>
        </div>
        ${sc === sidebar.currentModelScale ? "<span>✓</span>" : ""}
      `;
      item.addEventListener("click", (e) => {
        e.stopPropagation();
        sidebar.currentModelScale = sc;
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
    updateScalePopover();
  });

  sidebar.addWindowClickListener((e) => {
    if (!selectorBox.contains(e.target as Node) && isMenuOpen) {
      isMenuOpen = false;
      updateScalePopover();
    }
  });

  selectorBox.append(trigger, popover);
  container.appendChild(selectorBox);

  // Search Box
  const searchBox = document.createElement("div");
  searchBox.className = "search-box";
  const input = document.createElement("input");
  input.type = "text";
  input.placeholder = isDe ? "3D-Modelle durchsuchen…" : "Search 3D models…";
  input.value = sidebar.modelsSearchQuery;
  input.addEventListener("input", (e) => {
    sidebar.modelsSearchQuery = (e.target as HTMLInputElement).value;
    renderModelsList(sidebar, listContainer);
  });
  searchBox.appendChild(input);
  container.appendChild(searchBox);

  // Category Filter Bar
  const filterBar = document.createElement("div");
  filterBar.className = "category-filter-bar";
  for (const cat of MODEL_CATEGORIES) {
    const pill = document.createElement("button");
    pill.className = `filter-pill ${sidebar.selectedModelCategory === cat ? "active" : ""}`;
    pill.textContent = isDe ? MODEL_CATEGORY_LABELS_DE[cat] || cat : cat;
    pill.addEventListener("click", () => {
      sidebar.selectedModelCategory = cat;
      sidebar.render();
    });
    filterBar.appendChild(pill);
  }
  container.appendChild(filterBar);

  // Drag & Drop Hint
  const hint = document.createElement("div");
  hint.className = "drag-hint";
  hint.innerHTML = isDe
    ? " <em>Modell auf die Arbeitsfläche ziehen oder auf <strong>+</strong> klicken</em>"
    : " <em>Drag model onto canvas or click <strong>+</strong></em>";
  container.appendChild(hint);

  const listContainer = document.createElement("div");
  listContainer.className = "track-cards-list";
  renderModelsList(sidebar, listContainer);
  container.appendChild(listContainer);
}

export function renderModelsList(sidebar: Sidebar, container: HTMLElement) {
  container.innerHTML = "";
  const t = getTranslations().sidebar;
  const isDe = getLanguage() === "de";

  const query = sidebar.modelsSearchQuery.toLowerCase().trim();
  const selCat = sidebar.selectedModelCategory.toLowerCase();

  const matchingBuiltins = BUILTIN_MODELS.filter((def) => {
    if (selCat !== "all" && selCat !== "custom" && def.category.toLowerCase() !== selCat) {
      return false;
    }
    if (selCat === "custom") return false;
    if (!query) return true;
    return def.name.toLowerCase().includes(query) || def.description.toLowerCase().includes(query);
  });

  const matchingCustoms = sidebar.customModelsLibrary.filter((m) => {
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
    const f = getModelScaleFactor(sidebar);
    const fmt = (v: number) => Math.round(v * f * 10) / 10;

    const card = document.createElement("div");
    card.className = "track-card model-card";
    card.draggable = true;

    card.addEventListener("dragstart", (e) => {
      card.classList.add("dragging");
      const newModel = createScaledBuiltin(sidebar, def.id);
      e.dataTransfer?.setData("application/railbench-model", JSON.stringify(newModel));
      e.dataTransfer?.setData("text/plain", def.name);
      if (e.dataTransfer) {
        e.dataTransfer.effectAllowed = "copy";
        const zoom = sidebar.callbacks.getZoom?.() ?? 1.0;
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
        Size: ${fmt(def.width)} × ${fmt(def.depth)} × ${fmt(def.height)} mm (${sidebar.currentModelScale})
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
      sidebar.callbacks.onAddModel?.(createScaledBuiltin(sidebar, def.id));
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
        const zoom = sidebar.callbacks.getZoom?.() ?? 1.0;
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
      sidebar.callbacks.onAddModel?.(fresh);
    });
    actions.appendChild(addBtn);

    const delBtn = document.createElement("button");
    delBtn.className = "layer-action-btn btn-danger";
    delBtn.innerHTML = "🗑";
    delBtn.title = isDe ? "Aus eigener Bibliothek entfernen" : "Remove from custom library";
    delBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      sidebar.customModelsLibrary = sidebar.customModelsLibrary.filter((x) => x.name !== m.name);
      renderModelsList(sidebar, container);
    });
    actions.appendChild(delBtn);

    card.appendChild(actions);
    container.appendChild(card);
  }
}
