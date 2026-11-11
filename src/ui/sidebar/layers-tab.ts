import { sortLayers } from "@/core/layout";
import { getLanguage } from "@/i18n";
import { openLayerDialog } from "@/ui/layer-dialog";
import type { Sidebar } from "../sidebar";

export function renderLayersTab(sidebar: Sidebar, container: HTMLElement) {
  sidebar.layout.layers = sortLayers(sidebar.layout.layers || []);
  const layers = sidebar.layout.layers;
  const selectedPieceIds = sidebar.callbacks.selectedPieceIds?.() ?? [];

  const header = document.createElement("div");
  header.className = "layers-header";
  header.innerHTML = `
    <div class="layers-title">Layout Layers</div>
    <div class="layers-subtitle">Active layer receives new tracks. Hide or lock layers during construction.</div>
  `;

  const isDe = getLanguage() === "de";
  const isUnlocked = sidebar.callbacks.isSelectionLockDisabled?.() ?? false;
  const lockBar = document.createElement("div");
  lockBar.style.cssText = "display: flex; gap: 8px; margin-top: 8px; align-items: center;";
  const selLockBtn = document.createElement("button");
  selLockBtn.className = `btn-pill ${isUnlocked ? "active" : ""}`;
  selLockBtn.style.cssText = isUnlocked
    ? "background: rgba(245, 158, 11, 0.2); border: 1px solid #f59e0b; color: #f59e0b; font-weight: 600; padding: 4px 8px; font-size: 11px; cursor: pointer; border-radius: 6px;"
    : "background: var(--bg-surface); border: 1px solid var(--border-color); color: var(--text-main); padding: 4px 8px; font-size: 11px; cursor: pointer; border-radius: 6px;";
  selLockBtn.innerHTML = isUnlocked
    ? (isDe ? "🔓 Auswahlsperre aufgehoben" : "🔓 Selection lock disabled")
    : (isDe ? "🔒 Auswahlsperre aktiv" : "🔒 Selection lock active");
  selLockBtn.title = isDe
    ? "Klicken, um die Auswahlsperre aufzuheben (erlaubt Auswahl von Gleisen auf gesperrten oder gelöschten Ebenen)"
    : "Click to toggle selection lock (select items across locked or deleted layers)";
  selLockBtn.addEventListener("click", () => {
    sidebar.callbacks.onToggleSelectionLock?.();
    sidebar.render();
  });
  lockBar.appendChild(selLockBtn);

  if (sidebar.callbacks.onToggleWoodPlate) {
    const isWoodOn = sidebar.callbacks.isWoodPlateActive?.() ?? false;
    const woodBtn = document.createElement("button");
    woodBtn.className = `btn-pill ${isWoodOn ? "active" : ""}`;
    woodBtn.style.cssText = isWoodOn
      ? "background: rgba(2, 132, 199, 0.2); border: 1px solid #0284c7; color: #0284c7; font-weight: 600; padding: 4px 8px; font-size: 11px; cursor: pointer; border-radius: 6px;"
      : "background: var(--bg-surface); border: 1px solid var(--border-color); color: var(--text-main); padding: 4px 8px; font-size: 11px; cursor: pointer; border-radius: 6px;";
    woodBtn.innerHTML = isWoodOn
      ? (isDe ? "🪵 Woodplatte an" : "🪵 Wood plate on")
      : (isDe ? "🪵 Woodplatte aus" : "🪵 Wood plate off");
    woodBtn.title = isDe
      ? "Trassenbrett / Woodplatte unter Gleisen und Straßen ein- oder ausblenden"
      : "Show or hide wood roadbed plate under tracks and roads";
    woodBtn.addEventListener("click", () => {
      sidebar.callbacks.onToggleWoodPlate?.();
      sidebar.render();
    });
    lockBar.appendChild(woodBtn);
  }

  header.appendChild(lockBar);

  // Orphaned items alert
  const orphanedPieces = sidebar.layout.pieces.filter((p) => !layers.some((l) => l.id === (p.layerId || "layer-1")));
  const orphanedModels = (sidebar.layout.models || []).filter((m) => !layers.some((l) => l.id === (m.layerId || "layer-1")));
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
      const targetId = sidebar.layout.activeLayerId || layers[0]?.id || "layer-1";
      for (const p of orphanedPieces) p.layerId = targetId;
      for (const m of orphanedModels) m.layerId = targetId;
      sidebar.callbacks.onLayerChange?.();
      sidebar.render();
    });
    header.appendChild(alertBox);
  }

  container.appendChild(header);

  const list = document.createElement("div");
  list.className = "layers-list";

  for (const layer of layers) {
    const isActive = layer.id === sidebar.layout.activeLayerId;
    const count = sidebar.layout.pieces.filter((p) => (p.layerId || "layer-1") === layer.id).length;

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
          sidebar.callbacks.onLayerChange?.();
        }
        sidebar.render();
      };

      const cancel = () => {
        if (committed) return;
        committed = true;
        sidebar.render();
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
        layout: sidebar.layout,
        layer,
        onApply: (updated) => {
          Object.assign(layer, updated);
          sidebar.callbacks.onLayerChange?.();
          sidebar.render();
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
        sidebar.layout.activeLayerId = layer.id;
        sidebar.callbacks.onLayerChange?.();
        sidebar.render();
      });
      actions.appendChild(activateBtn);
    }

    const visBtn = document.createElement("button");
    visBtn.className = `layer-icon-btn ${layer.visible ? "active" : "muted"}`;
    visBtn.innerHTML = layer.visible ? "👁️" : "🕶️";
    visBtn.title = layer.visible ? "Visible (Click to hide)" : "Hidden (Click to show)";
    visBtn.addEventListener("click", () => {
      layer.visible = !layer.visible;
      sidebar.callbacks.onLayerChange?.();
      sidebar.render();
    });
    actions.appendChild(visBtn);

    const lockBtn = document.createElement("button");
    lockBtn.className = `layer-icon-btn ${layer.locked ? "locked active-warning" : ""}`;
    lockBtn.innerHTML = layer.locked ? "🔒" : "🔓";
    lockBtn.title = layer.locked ? "Locked (Click to unlock)" : "Unlocked (Click to lock)";
    lockBtn.addEventListener("click", () => {
      layer.locked = !layer.locked;
      sidebar.callbacks.onLayerChange?.();
      sidebar.render();
    });
    actions.appendChild(lockBtn);

    if (selectedPieceIds.length > 0) {
      const moveBtn = document.createElement("button");
      moveBtn.className = "layer-btn-pill";
      moveBtn.textContent = `Move (${selectedPieceIds.length})`;
      moveBtn.title = `Move selected tracks to ${layer.name}`;
      moveBtn.addEventListener("click", () => {
        sidebar.callbacks.onMoveSelectedToLayer?.(layer.id);
        sidebar.render();
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
          for (const p of sidebar.layout.pieces) {
            if (p.layerId === layer.id) p.layerId = fallbackId;
          }
          if (sidebar.layout.models) {
            for (const m of sidebar.layout.models) {
              if (m.layerId === layer.id) m.layerId = fallbackId;
            }
          }
          sidebar.layout.layers = layers.filter((l) => l.id !== layer.id);
          if (sidebar.layout.activeLayerId === layer.id) {
            sidebar.layout.activeLayerId = fallbackId;
          }
          sidebar.callbacks.onLayerChange?.();
          sidebar.render();
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
      layout: sidebar.layout,
      position: "above",
      onApply: (newLayer, _pos, makeActive) => {
        if (!sidebar.layout.layers) sidebar.layout.layers = [];
        sidebar.layout.layers.push(newLayer);
        sidebar.layout.layers = sortLayers(sidebar.layout.layers);
        if (makeActive) {
          sidebar.layout.activeLayerId = newLayer.id;
        }
        sidebar.callbacks.onLayerChange?.();
        sidebar.render();
      },
    });
  });
  container.appendChild(addLayerBtn);
}
