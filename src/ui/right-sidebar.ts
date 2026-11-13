import { ICONS } from "./icons";
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
  onToggleSelectionLock?: () => void;
  isSelectionLockDisabled?: () => boolean;
  onClose?: () => void;
  selectedPieceIds: () => string[];
}



export class RightSidebar {
  private host: HTMLElement;
  private layout: Layout;
  private callbacks: RightSidebarCallbacks;

  constructor(host: HTMLElement, layout: Layout, callbacks: RightSidebarCallbacks) {
    this.host = host;
    this.layout = layout;
    this.callbacks = callbacks;
    this.host.classList.add("crystal-panel");
    ensureDefaultLayers(this.layout);
    this.render();
    onLanguageChange(() => this.render());
  }

  update(layout: Layout) {
    this.layout = layout;
    ensureDefaultLayers(this.layout);
    this.render();
  }

  /** Small helper: icon button with tooltip + aria-label. */
  private iconButton(className: string, icon: string, title: string, onClick: (e: MouseEvent) => void) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = className;
    btn.innerHTML = icon;
    btn.title = title;
    btn.setAttribute("aria-label", title);
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      onClick(e);
    });
    return btn;
  }

  private addLayerButton(label: string, tooltip: string, above: boolean) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "cl-add";
    btn.innerHTML = `${ICONS.plus}<span>${label}</span>`;
    btn.title = tooltip;
    btn.addEventListener("click", () => this.promptAddLayer(above));
    return btn;
  }

  private getContentContainer(): HTMLElement {
    let container = this.host.querySelector(".crystal-panel-inner") as HTMLElement;
    if (!container) {
      container = document.createElement("div");
      container.className = "crystal-panel-inner";
      container.style.cssText = "width: 100%; height: 100%; display: flex; flex-direction: column; overflow: hidden;";
      this.host.appendChild(container);
    }
    return container;
  }

  private render() {
    ensureDefaultLayers(this.layout);
    const container = this.getContentContainer();
    container.innerHTML = "";

    const t = getTranslations().rightSidebar;
    const layers = this.layout.layers || [];
    const activeId = this.layout.activeLayerId || layers[0]?.id;

    // Body
    const body = document.createElement("div");
    body.className = "cl-body";
    const list = document.createElement("div");
    list.className = "cl-list";

    const sortedLayers = sortLayers(layers);
    this.layout.layers = sortedLayers;

    for (const layer of sortedLayers) {
      list.appendChild(this.renderLayer(layer, layer.id === activeId, t));
    }
    body.appendChild(list);
    body.appendChild(
      this.addLayerButton(getLanguage() === "de" ? "Ebene hinzufügen" : "Add layer", t.addAboveTooltip, true),
    );

    // Orphaned items alert (e.g. from previously deleted layers)
    const orphanedPieces = this.layout.pieces.filter((p) => !layers.some((l) => l.id === (p.layerId || "layer-1")));
    const orphanedModels = (this.layout.models || []).filter((m) => !layers.some((l) => l.id === (m.layerId || "layer-1")));
    const totalOrphaned = orphanedPieces.length + orphanedModels.length;

    if (totalOrphaned > 0) {
      const alertBox = document.createElement("div");
      alertBox.className = "cl-orphan-alert";
      alertBox.style.cssText =
        "margin: 6px 0; padding: 8px 10px; border-radius: 8px; background: rgba(239, 68, 68, 0.12); border: 1px solid rgba(239, 68, 68, 0.35); font-size: 11px; display: flex; flex-direction: column; gap: 6px;";
      alertBox.innerHTML = `
        <div style="color: #ef4444; font-weight: 600;">⚠️ ${t.orphanedItemsWarning.replace("{count}", String(totalOrphaned))}</div>
        <button type="button" class="cl-pill is-primary" style="align-self: flex-start; cursor: pointer; padding: 3px 8px; font-size: 11px;">
          ${t.reassignOrphaned}
        </button>
      `;
      alertBox.querySelector("button")?.addEventListener("click", () => {
        const targetId = this.layout.activeLayerId || layers[0]?.id || "layer-1";
        for (const p of orphanedPieces) p.layerId = targetId;
        for (const m of orphanedModels) m.layerId = targetId;
        this.callbacks.onLayerChange();
        this.render();
      });
      body.appendChild(alertBox);
    }

    // Action buttons container for Selection Lock & Woodplate
    if (this.callbacks.onToggleSelectionLock || this.callbacks.onToggleWoodPlate) {
      const btnGroup = document.createElement("div");
      btnGroup.className = "cl-btn-group";

      // Disable Selection Lock button
      if (this.callbacks.onToggleSelectionLock) {
        const de = getLanguage() === "de";
        const on = this.callbacks.isSelectionLockDisabled?.() ?? false;
        const lockBtn = document.createElement("button");
        lockBtn.type = "button";
        lockBtn.className = `cl-btn ${on ? "is-active" : ""}`;
        lockBtn.id = "cl-toggle-selection-lock";
        lockBtn.title = de
          ? "Auswahlsperre: Erlaubt oder sperrt das Auswählen von Gleisen und Modellen über inaktive, gesperrte oder gelöschte Ebenen hinweg"
          : "Selection Lock: Allows or restricts selecting items across inactive, locked or deleted layers";
        lockBtn.innerHTML = `
          <span style="display: flex; align-items: center; gap: 6px;">
            <span class="cl-btn-icon">${on ? ICONS.unlock : ICONS.lock}</span>
            <span>${de ? "Auswahlsperre" : "Selection lock"}</span>
          </span>
          <span class="cl-btn-badge">${on ? (de ? "Aufgehoben" : "Unlocked") : (de ? "Aktiv" : "Active")}</span>
        `;
        lockBtn.addEventListener("click", () => {
          this.callbacks.onToggleSelectionLock?.();
          this.render();
        });
        btnGroup.appendChild(lockBtn);
      }

      // Wood plate button
      if (this.callbacks.onToggleWoodPlate) {
        const de = getLanguage() === "de";
        const on = this.callbacks.isWoodPlateActive?.() ?? false;
        const woodBtn = document.createElement("button");
        woodBtn.type = "button";
        woodBtn.className = `cl-btn ${on ? "is-active" : ""}`;
        woodBtn.id = "cl-toggle-woodplate";
        woodBtn.title = de
          ? "Trassenbrett / Woodplatte unter Gleisen und Straßen ein- oder ausblenden"
          : "Show or hide wood roadbed plate under tracks and roads";
        woodBtn.innerHTML = `
          <span style="display: flex; align-items: center; gap: 6px;">
            <span class="cl-btn-icon">${ICONS.wood}</span>
            <span>${de ? "Woodplatte" : "Wood plate"}</span>
          </span>
          <span class="cl-btn-badge">${on ? (de ? "An" : "On") : (de ? "Aus" : "Off")}</span>
        `;
        woodBtn.addEventListener("click", () => {
          this.callbacks.onToggleWoodPlate?.();
          this.render();
        });
        btnGroup.appendChild(woodBtn);
      }

      body.appendChild(btnGroup);
    }

    container.appendChild(body);
  }

  private renderLayer(layer: Layer, isActive: boolean, t: ReturnType<typeof getTranslations>["rightSidebar"]) {
    const layers = this.layout.layers || [];
    const count = this.layout.pieces.filter((p) => (p.layerId || "layer-1") === layer.id).length;
    const hMm = layer.heightMm ?? 0;
    const hSign = hMm > 0 ? `+${hMm}` : `${hMm}`;

    const card = document.createElement("div");
    card.className = `cl-card ${isActive ? "is-active" : ""} ${layer.visible ? "" : "is-hidden"}`;
    card.style.setProperty("--layer-color", layer.color || "#38bdf8");

    card.addEventListener("click", (e) => {
      if ((e.target as HTMLElement).closest("button, input")) return;
      this.layout.activeLayerId = layer.id;
      this.callbacks.onSelectActiveLayer?.(layer.id);
      this.callbacks.onLayerChange();
      this.render();
    });

    // --- Row: name + meta | actions
    const row = document.createElement("div");
    row.className = "cl-row";

    const info = document.createElement("div");
    info.className = "cl-info";

    const nameRow = document.createElement("div");
    nameRow.className = "cl-name-row";

    const nameLabel = document.createElement("span");
    nameLabel.className = "cl-name";
    nameLabel.textContent = layer.name;
    nameLabel.title = t.renameTooltip;

    const startEditingName = () => {
      const input = document.createElement("input");
      input.type = "text";
      input.className = "cl-name-input";
      input.value = layer.name;
      input.setAttribute("aria-label", t.renamePlaceholder);

      let done = false;
      const finish = (save: boolean) => {
        if (done) return;
        done = true;
        const val = input.value.trim();
        if (save && val && val !== layer.name) {
          layer.name = val;
          this.layout.layers = sortLayers(this.layout.layers || []);
          this.callbacks.onLayerChange();
        }
        this.render();
      };

      input.addEventListener("keydown", (ev) => {
        ev.stopPropagation();
        if (ev.key === "Enter") { ev.preventDefault(); finish(true); }
        else if (ev.key === "Escape") { ev.preventDefault(); finish(false); }
      });
      input.addEventListener("blur", () => finish(true));
      input.addEventListener("pointerdown", (ev) => ev.stopPropagation());

      nameRow.innerHTML = "";
      nameRow.appendChild(input);
      input.focus();
      input.select();
    };

    nameLabel.addEventListener("dblclick", (e) => { e.stopPropagation(); startEditingName(); });
    nameRow.appendChild(nameLabel);
    nameRow.appendChild(this.iconButton("cl-icon-btn cl-subtle", ICONS.edit, t.renameTooltip, startEditingName));

    // Meta: colour dot (click = edit), height, track count
    const meta = document.createElement("div");
    meta.className = "cl-meta";

    const editTitle = t.editElevationPrompt.replace('"{name}":', "").replace("{name}", layer.name);
    const dot = document.createElement("button");
    dot.type = "button";
    dot.className = "cl-dot";
    dot.title = editTitle;
    dot.setAttribute("aria-label", editTitle);
    dot.addEventListener("click", (e) => { e.stopPropagation(); this.openEditLayer(layer); });

    const height = document.createElement("button");
    height.type = "button";
    height.className = "cl-height";
    height.textContent = `${hSign} mm`;
    height.title = editTitle;
    height.addEventListener("click", (e) => { e.stopPropagation(); this.openEditLayer(layer); });

    const tracks = document.createElement("span");
    tracks.className = "cl-tracks";
    tracks.textContent = t.tracksCount.replace("{count}", String(count)).replace("{s}", count === 1 ? "" : "s");

    meta.append(dot, height, tracks);
    info.append(nameRow, meta);

    // Actions
    const actions = document.createElement("div");
    actions.className = "cl-actions";

    actions.appendChild(
      this.iconButton(
        `cl-icon-btn ${layer.visible ? "" : "is-off"}`,
        layer.visible ? ICONS.eye : ICONS.eyeOff,
        layer.visible ? t.visTooltipVisible : t.visTooltipHidden,
        () => { layer.visible = !layer.visible; this.callbacks.onLayerChange(); this.render(); },
      ),
    );

    actions.appendChild(
      this.iconButton(
        `cl-icon-btn ${layer.locked ? "is-on" : ""}`,
        layer.locked ? ICONS.lock : ICONS.unlock,
        layer.locked ? t.lockTooltipLocked : t.lockTooltipUnlocked,
        () => { layer.locked = !layer.locked; this.callbacks.onLayerChange(); this.render(); },
      ),
    );

    if (layers.length > 1) {
      actions.appendChild(
        this.iconButton("cl-icon-btn cl-danger", ICONS.trash, t.deleteLayer, () => {
          if (!confirm(t.deleteConfirm.replace("{name}", layer.name))) return;
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
          if (this.layout.activeLayerId === layer.id) this.layout.activeLayerId = fallbackId;
          this.callbacks.onLayerChange();
          this.render();
        }),
      );
    }

    row.append(info, actions);
    card.appendChild(row);

    return card;
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
        if (this.layout.layers) this.layout.layers = sortLayers(this.layout.layers);
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