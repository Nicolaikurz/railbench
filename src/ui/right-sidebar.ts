import type { Layout, Layer } from "@/core/types";
import { ensureDefaultLayers, sortLayers } from "@/core/layout";
import { getTranslations, getLanguage, onLanguageChange } from "@/i18n";
import { openLayerDialog } from "./layer-dialog";

export interface RightSidebarCallbacks {
  onLayerChange: () => void;
  onMoveSelectedToLayer?: (layerId: string, elevation?: { start: number; end: number }) => void;
  onOpenHeightDialog?: (preselectedLayerId?: string) => void;
  onSelectActiveLayer?: (layerId: string) => void;
  onToggleWoodPlate?: () => void;
  isWoodPlateActive?: () => boolean;
  onClose?: () => void;
  selectedPieceIds: () => string[];
}

export class RightSidebar {
  private host: HTMLElement;
  private layout: Layout;
  private callbacks: RightSidebarCallbacks;

  constructor(
    host: HTMLElement,
    layout: Layout,
    callbacks: RightSidebarCallbacks,
  ) {
    this.host = host;
    this.layout = layout;
    this.callbacks = callbacks;
    ensureDefaultLayers(this.layout);
    this.render();
    onLanguageChange(() => this.render());
  }

  update(layout: Layout) {
    this.layout = layout;
    ensureDefaultLayers(this.layout);
    this.render();
  }

  private render() {
    this.host.innerHTML = "";
    ensureDefaultLayers(this.layout);

    const t = getTranslations().rightSidebar;
    const layers = this.layout.layers || [];
    const activeId = this.layout.activeLayerId || layers[0]?.id;
    const selectedIds = this.callbacks.selectedPieceIds();

    // 1. Floating Header (Titel & Schließen-Button)
    const header = document.createElement("div");
    header.className = "floating-header";
    header.innerHTML = `
      <div class="floating-title">
        <span></span>
        <span>${t.title}</span>
      </div>
      <button class="floating-close-btn" title="${t.closeTooltip}">✕</button>
    `;
    header.querySelector(".floating-close-btn")?.addEventListener("click", () => {
      this.callbacks.onClose?.();
    });
    this.host.appendChild(header);

    // 2. Floating Body
    const body = document.createElement("div");
    body.className = "floating-body";

    // OBERER PLUS-BUTTON (+ Ebene oben hinzufügen)
    const topAddBtn = document.createElement("button");
    topAddBtn.className = "layer-btn-add top-add-btn";
    topAddBtn.innerHTML = `<strong>${t.addLayerAbove}</strong>`;
    topAddBtn.title = t.addAboveTooltip;
    topAddBtn.addEventListener("click", () => this.promptAddLayer(true));
    body.appendChild(topAddBtn);

    // Ebenenliste sortiert nach Höhe (höchste Ebene oben) und sekundär nach Name
    const list = document.createElement("div");
    list.className = "layers-list right-layers-list";

    const sortedLayers = sortLayers(layers);
    this.layout.layers = sortedLayers;

    for (const layer of sortedLayers) {
      const isAct = layer.id === activeId;
      const count = this.layout.pieces.filter((p) => (p.layerId || "layer-1") === layer.id).length;
      const hMm = layer.heightMm ?? 0;
      const hSign = hMm > 0 ? `+${hMm}` : `${hMm}`;

      const card = document.createElement("div");
      card.className = `layer-card right-layer-card ${isAct ? "active" : ""}`;

      // Klick auf Karte setzt die Ebene als aktiv
      card.addEventListener("click", (e) => {
        const target = e.target as HTMLElement;
        if (
          target.closest(".layer-action-btn") ||
          target.closest(".layer-move-pill") ||
          target.closest(".layer-rename-btn") ||
          target.closest(".layer-name-edit-input") ||
          target.closest(".layer-elev-clickable") ||
          target.closest(".layer-custom-height-btn") ||
          target.closest(".layer-change-height-pill")
        ) return;
        this.layout.activeLayerId = layer.id;
        this.callbacks.onSelectActiveLayer?.(layer.id);
        this.callbacks.onLayerChange();
        this.render();
      });

      // Zeile 1: Symbol/Farbe, Name, Höhe und Buttons für Ausblenden, Locken, Löschen
      const mainRow = document.createElement("div");
      mainRow.className = "layer-row-top";

      // Links: Farbpunkt & Symbol & Name & Höhe
      const leftCol = document.createElement("div");
      leftCol.className = "layer-left-col";

      const colorDot = document.createElement("span");
      colorDot.className = "layer-color-dot";
      colorDot.style.background = layer.color || "#38bdf8";
      colorDot.style.cursor = "pointer";
      colorDot.title = t.editElevationPrompt.replace('"{name}":', "").replace("{name}", layer.name);
      colorDot.addEventListener("click", (e) => {
        e.stopPropagation();
        this.openEditLayer(layer);
      });

      const textGroup = document.createElement("div");
      textGroup.className = "layer-text-group";

      const titleRow = document.createElement("div");
      titleRow.className = "layer-name-title";

      const nameLabel = document.createElement("span");
      nameLabel.className = "layer-name-label";
      nameLabel.textContent = layer.name;
      nameLabel.title = t.renameTooltip;

      const renameBtn = document.createElement("button");
      renameBtn.type = "button";
      renameBtn.className = "layer-rename-btn";
      renameBtn.innerHTML = "✏️";
      renameBtn.title = t.renameTooltip;

      const startEditingName = () => {
        const input = document.createElement("input");
        input.type = "text";
        input.className = "layer-name-edit-input";
        input.value = layer.name;
        input.setAttribute("aria-label", t.renamePlaceholder);

        let committed = false;
        const commit = () => {
          if (committed) return;
          committed = true;
          const val = input.value.trim();
          if (val && val !== layer.name) {
            layer.name = val;
            this.layout.layers = sortLayers(this.layout.layers || []);
            this.callbacks.onLayerChange();
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

        titleRow.innerHTML = "";
        titleRow.appendChild(input);
        input.focus();
        input.select();
      };

      renameBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        startEditingName();
      });

      nameLabel.addEventListener("dblclick", (e) => {
        e.stopPropagation();
        startEditingName();
      });

      titleRow.appendChild(nameLabel);
      titleRow.appendChild(renameBtn);

      const metaRow = document.createElement("div");
      metaRow.className = "layer-meta-tag";

      const elevBadge = document.createElement("span");
      elevBadge.className = "layer-elev-badge layer-elev-clickable";
      elevBadge.textContent = `${hSign} mm`;
      elevBadge.title = t.editElevationPrompt.replace('"{name}":', "");
      elevBadge.addEventListener("click", (e) => {
        e.stopPropagation();
        this.openEditLayer(layer);
      });

      const countBadge = document.createElement("span");
      countBadge.className = "layer-count-badge";
      const sSuffix = count === 1 ? "" : "s";
      countBadge.textContent = t.tracksCount.replace("{count}", String(count)).replace("{s}", sSuffix);

      metaRow.appendChild(elevBadge);
      metaRow.appendChild(countBadge);

      textGroup.appendChild(titleRow);
      textGroup.appendChild(metaRow);

      leftCol.appendChild(colorDot);
      leftCol.appendChild(textGroup);
      mainRow.appendChild(leftCol);

      // Rechts: Icons für Ausblenden, Locken und Löschen
      const actions = document.createElement("div");
      actions.className = "layer-actions-group";

      // 1. Ausblenden / Einblenden Button
      const visBtn = document.createElement("button");
      visBtn.className = `layer-action-btn ${layer.visible ? "btn-vis-on" : "btn-vis-off"}`;
      visBtn.title = layer.visible ? t.visTooltipVisible : t.visTooltipHidden;
      visBtn.innerHTML = layer.visible ? "👁️" : "🕶️";
      visBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        layer.visible = !layer.visible;
        this.callbacks.onLayerChange();
        this.render();
      });
      actions.appendChild(visBtn);

      // 2. Locken / Entsperren Button
      const lockBtn = document.createElement("button");
      lockBtn.className = `layer-action-btn ${layer.locked ? "btn-lock-on" : "btn-lock-off"}`;
      lockBtn.title = layer.locked ? t.lockTooltipLocked : t.lockTooltipUnlocked;
      lockBtn.innerHTML = layer.locked ? "🔒" : "🔓";
      lockBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        layer.locked = !layer.locked;
        this.callbacks.onLayerChange();
        this.render();
      });
      actions.appendChild(lockBtn);

      // 3. Löschen Button
      if (layers.length > 1) {
        const delBtn = document.createElement("button");
        delBtn.className = "layer-action-btn btn-danger";
        delBtn.title = t.deleteLayer;
        delBtn.innerHTML = "🗑";
        delBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          if (confirm(t.deleteConfirm.replace("{name}", layer.name))) {
            const fallbackId = layers.find((l) => l.id !== layer.id)!.id;
            for (const p of this.layout.pieces) {
              if (p.layerId === layer.id) p.layerId = fallbackId;
            }
            this.layout.layers = layers.filter((l) => l.id !== layer.id);
            if (this.layout.activeLayerId === layer.id) {
              this.layout.activeLayerId = fallbackId;
            }
            this.callbacks.onLayerChange();
            this.render();
          }
        });
        actions.appendChild(delBtn);
      }

      mainRow.appendChild(actions);
      card.appendChild(mainRow);

      // Falls Gleise markiert sind: Aktionen zum Ändern der Ebene und Höhe
      if (selectedIds.length > 0) {
        const moveRow = document.createElement("div");
        moveRow.className = "layer-selection-actions";

        if (!isAct) {
          // 1. Direkt mit Höhenanpassung auf Ziel-Ebene verschieben
          const moveBtn = document.createElement("button");
          moveBtn.className = "layer-move-pill";
          moveBtn.title = t.moveAndSetHeight.replace("{height}", hSign);
          moveBtn.innerHTML = t.moveAndSetHeight.replace("{height}", hSign);
          moveBtn.addEventListener("click", (e) => {
            e.stopPropagation();
            this.callbacks.onMoveSelectedToLayer?.(layer.id, { start: 0, end: 0 });
            this.render();
          });
          moveRow.appendChild(moveBtn);

          // 2. Dialog für Rampen & benutzerdefinierte Höhen öffnen
          const customBtn = document.createElement("button");
          customBtn.className = "layer-custom-height-btn";
          customBtn.title = t.rampConfig;
          customBtn.innerHTML = t.rampConfig;
          customBtn.addEventListener("click", (e) => {
            e.stopPropagation();
            this.callbacks.onOpenHeightDialog?.(layer.id);
          });
          moveRow.appendChild(customBtn);
        } else {
          // Wenn bereits auf dieser Ebene: Höhe/Rampe der selektierten Stücke anpassen
          const heightBtn = document.createElement("button");
          heightBtn.className = "layer-change-height-pill";
          heightBtn.title = t.changeHeightOrRamp;
          heightBtn.innerHTML = t.changeHeightOrRamp;
          heightBtn.addEventListener("click", (e) => {
            e.stopPropagation();
            this.callbacks.onOpenHeightDialog?.(layer.id);
          });
          moveRow.appendChild(heightBtn);
        }

        card.appendChild(moveRow);
      }

      list.appendChild(card);
    }
    body.appendChild(list);

    // UNTERER PLUS-BUTTON (+ Ebene unten hinzufügen)
    const bottomAddBtn = document.createElement("button");
    bottomAddBtn.className = "layer-btn-add bottom-add-btn";
    bottomAddBtn.innerHTML = `<strong>${t.addLayerBelow}</strong>`;
    bottomAddBtn.title = t.addBelowTooltip;
    bottomAddBtn.addEventListener("click", () => this.promptAddLayer(false));
    body.appendChild(bottomAddBtn);

    if (this.callbacks.onToggleWoodPlate) {
      const isPlateActive = this.callbacks.isWoodPlateActive?.() ?? false;
      const plateBtn = document.createElement("button");
      plateBtn.className = `layer-btn-add woodplate-btn ${isPlateActive ? "active" : ""}`;
      plateBtn.style.marginTop = "6px";
      plateBtn.style.background = isPlateActive ? "rgba(2, 132, 199, 0.15)" : "var(--bg-card)";
      plateBtn.style.borderColor = isPlateActive ? "var(--primary-color)" : "var(--border-main)";
      plateBtn.innerHTML = `<strong> ${isPlateActive ? (getLanguage() === "de" ? "Woodplatte aktiv" : "Wood Plate Active") : (getLanguage() === "de" ? "Woodplatte unter Gleisen/Straßen" : "Wood Plate under Tracks/Roads")}</strong>`;
      plateBtn.title = getLanguage() === "de" ? "Trassenbrett / Woodplatte unter Gleisen und Straßen platzieren" : "Place wood roadbed plate under tracks and roads";
      plateBtn.addEventListener("click", () => {
        this.callbacks.onToggleWoodPlate?.();
        this.render();
      });
      body.appendChild(plateBtn);
    }

    this.host.appendChild(body);
  }

  private promptAddLayer(isAbove: boolean) {
    openLayerDialog({
      layout: this.layout,
      position: isAbove ? "above" : "below",
      onApply: (newLayer, _pos, makeActive) => {
        if (!this.layout.layers) this.layout.layers = [];
        this.layout.layers.push(newLayer);
        this.layout.layers = sortLayers(this.layout.layers);
        if (makeActive ?? true) {
          this.layout.activeLayerId = newLayer.id;
          this.callbacks.onSelectActiveLayer?.(newLayer.id);
        }
        this.callbacks.onLayerChange();
        this.render();
      },
    });
  }

  private openEditLayer(layer: Layer) {
    openLayerDialog({
      layout: this.layout,
      layer,
      onApply: (updatedLayer, _pos, makeActive) => {
        Object.assign(layer, updatedLayer);
        if (this.layout.layers) {
          this.layout.layers = sortLayers(this.layout.layers);
        }
        if (makeActive) {
          this.layout.activeLayerId = layer.id;
          this.callbacks.onSelectActiveLayer?.(layer.id);
        }
        this.callbacks.onLayerChange();
        this.render();
      },
    });
  }
}
