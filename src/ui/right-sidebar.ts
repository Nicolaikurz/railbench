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

const STYLE_ID = "crystal-sidebar-styles";
const STYLES = `
/* Crystal-clear glass sidebar (compact) — scoped to .crystal-panel */

.crystal-panel {
  /* light theme (default) */
  --cl-text: #1b2433;
  --cl-text-dim: rgba(27, 36, 51, 0.6);
  --cl-glass: rgba(0, 0, 0, 0.04);
  --cl-glass-hover: rgba(0, 0, 0, 0.07);
  --cl-glass-active: rgba(0, 0, 0, 0.1);
  --cl-edge: rgba(0, 0, 0, 0.13);
  --cl-accent: #0284c7;
  --cl-accent-soft: rgba(2, 132, 199, 0.16);
  --cl-accent-edge: rgba(2, 132, 199, 0.5);
  --cl-danger: #dc2626;
  --cl-track: rgba(0, 0, 0, 0.2);
  --cl-input-bg: rgba(255, 255, 255, 0.7);
  --cl-text-shadow: 0 1px 2px rgba(255, 255, 255, 0.6);
  --cl-bg: rgba(255, 255, 255, 0.45);
  --cl-bg-glass: linear-gradient(160deg, rgba(255, 255, 255, 0.4), rgba(255, 255, 255, 0.12));
  --cl-panel-shadow: 0 8px 24px rgba(0, 0, 0, 0.12), inset 0 1px 0 rgba(255, 255, 255, 0.6);

  width: 100%;
  box-sizing: border-box;
  min-width: 0;
  color: var(--cl-text);
  text-shadow: var(--cl-text-shadow);
  font: 11px/1.3 system-ui, -apple-system, "Segoe UI", sans-serif;
  background: var(--cl-bg);
  border: 1px solid var(--cl-edge);
  border-radius: 10px;
  box-shadow: var(--cl-panel-shadow);
  overflow: hidden;
}
@supports (backdrop-filter: blur(1px)) {
  .crystal-panel {
    background: var(--cl-bg-glass);
    backdrop-filter: blur(8px) saturate(130%);
    -webkit-backdrop-filter: blur(8px) saturate(130%);
  }
}

@media (prefers-color-scheme: dark) {
  .crystal-panel {
  --cl-text: #f2f6fb;
  --cl-text-dim: rgba(242, 246, 251, 0.62);
  --cl-glass: rgba(255, 255, 255, 0.04);
  --cl-glass-hover: rgba(255, 255, 255, 0.09);
  --cl-glass-active: rgba(255, 255, 255, 0.12);
  --cl-edge: rgba(255, 255, 255, 0.14);
  --cl-accent: #7dd3fc;
  --cl-accent-soft: rgba(125, 211, 252, 0.18);
  --cl-accent-edge: rgba(125, 211, 252, 0.5);
  --cl-danger: #fca5a5;
  --cl-track: rgba(255, 255, 255, 0.18);
  --cl-input-bg: rgba(0, 0, 0, 0.25);
  --cl-text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
  --cl-bg: rgba(20, 28, 40, 0.3);
  --cl-bg-glass: linear-gradient(160deg, rgba(255, 255, 255, 0.07), rgba(255, 255, 255, 0.01));
  --cl-panel-shadow: 0 8px 24px rgba(0, 0, 0, 0.18), inset 0 1px 0 rgba(255, 255, 255, 0.18);
  }
}
[data-theme="dark"] .crystal-panel,
[data-bs-theme="dark"] .crystal-panel,
.dark .crystal-panel,
.dark-mode .crystal-panel,
.theme-dark .crystal-panel {
  --cl-text: #f2f6fb;
  --cl-text-dim: rgba(242, 246, 251, 0.62);
  --cl-glass: rgba(255, 255, 255, 0.04);
  --cl-glass-hover: rgba(255, 255, 255, 0.09);
  --cl-glass-active: rgba(255, 255, 255, 0.12);
  --cl-edge: rgba(255, 255, 255, 0.14);
  --cl-accent: #7dd3fc;
  --cl-accent-soft: rgba(125, 211, 252, 0.18);
  --cl-accent-edge: rgba(125, 211, 252, 0.5);
  --cl-danger: #fca5a5;
  --cl-track: rgba(255, 255, 255, 0.18);
  --cl-input-bg: rgba(0, 0, 0, 0.25);
  --cl-text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
  --cl-bg: rgba(20, 28, 40, 0.3);
  --cl-bg-glass: linear-gradient(160deg, rgba(255, 255, 255, 0.07), rgba(255, 255, 255, 0.01));
  --cl-panel-shadow: 0 8px 24px rgba(0, 0, 0, 0.18), inset 0 1px 0 rgba(255, 255, 255, 0.18);
}
[data-theme="light"] .crystal-panel,
[data-bs-theme="light"] .crystal-panel,
.light .crystal-panel,
.light-mode .crystal-panel,
.theme-light .crystal-panel {
  --cl-text: #1b2433;
  --cl-text-dim: rgba(27, 36, 51, 0.6);
  --cl-glass: rgba(0, 0, 0, 0.04);
  --cl-glass-hover: rgba(0, 0, 0, 0.07);
  --cl-glass-active: rgba(0, 0, 0, 0.1);
  --cl-edge: rgba(0, 0, 0, 0.13);
  --cl-accent: #0284c7;
  --cl-accent-soft: rgba(2, 132, 199, 0.16);
  --cl-accent-edge: rgba(2, 132, 199, 0.5);
  --cl-danger: #dc2626;
  --cl-track: rgba(0, 0, 0, 0.2);
  --cl-input-bg: rgba(255, 255, 255, 0.7);
  --cl-text-shadow: 0 1px 2px rgba(255, 255, 255, 0.6);
  --cl-bg: rgba(255, 255, 255, 0.45);
  --cl-bg-glass: linear-gradient(160deg, rgba(255, 255, 255, 0.4), rgba(255, 255, 255, 0.12));
  --cl-panel-shadow: 0 8px 24px rgba(0, 0, 0, 0.12), inset 0 1px 0 rgba(255, 255, 255, 0.6);
}

.crystal-panel button { font: inherit; color: inherit; cursor: pointer; }
.crystal-panel :focus-visible { outline: 2px solid var(--cl-accent); outline-offset: 1px; }

.cl-body { display: flex; flex-direction: column; gap: 3px; padding: 4px; }
.cl-list { display: flex; flex-direction: column; gap: 2px; }

/* Icon buttons */
.cl-icon-btn {
  display: inline-grid; place-items: center;
  width: 20px; height: 20px; padding: 0;
  background: transparent; border: 0; border-radius: 6px;
  color: var(--cl-text-dim);
  transition: background 0.15s, color 0.15s;
}
.cl-icon-btn:hover { background: var(--cl-glass-hover); color: var(--cl-text); }
.cl-icon-btn.is-on { color: var(--cl-accent); }
.cl-icon-btn.is-off { opacity: 0.5; }
.cl-icon-btn.cl-danger:hover { color: var(--cl-danger); }
.cl-icon-btn svg { width: 13px; height: 13px; }
.cl-subtle { display: none; width: 16px; height: 16px; }
.cl-card:hover .cl-subtle, .cl-card.is-active .cl-subtle { display: inline-grid; }

/* Add layer */
.cl-add {
  display: flex; align-items: center; justify-content: center; gap: 4px;
  padding: 3px 8px;
  background: transparent; border: 1px dashed var(--cl-edge); border-radius: 8px;
  color: var(--cl-text-dim);
  transition: background 0.15s, color 0.15s;
}
.cl-add svg { width: 12px; height: 12px; }
.cl-add:hover { background: var(--cl-glass); color: var(--cl-text); }

/* Layer card */
.cl-card {
  padding: 4px 4px 4px 7px;
  background: var(--cl-glass);
  border: 1px solid transparent;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s;
}
.cl-card:hover { background: var(--cl-glass-hover); }
.cl-card.is-active {
  background: var(--cl-glass-active);
  border-color: var(--cl-edge);
  box-shadow: inset 2px 0 0 var(--layer-color);
}
.cl-card.is-hidden .cl-info { opacity: 0.5; }

.cl-row { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
.cl-info { min-width: 0; flex: 1; }
.cl-name-row { display: flex; align-items: center; gap: 2px; }
.cl-name { font-weight: 600; white-space: nowrap; }
.cl-name-input {
  width: 100px; padding: 0 5px; font: inherit; font-weight: 600; color: var(--cl-text);
  background: var(--cl-input-bg); border: 1px solid var(--cl-accent); border-radius: 5px;
}

.cl-meta { display: flex; align-items: center; gap: 5px; color: var(--cl-text-dim); font-size: 10px; white-space: nowrap; }
.cl-dot {
  width: 7px; height: 7px; padding: 0; flex: none;
  background: var(--layer-color); border: 0; border-radius: 50%;
}
.cl-height { padding: 0; background: none; border: 0; color: var(--cl-text); font-variant-numeric: tabular-nums; }
.cl-height:hover { text-decoration: underline; }

.cl-actions { display: flex; flex: none; }

/* Selection actions */
.cl-selection { display: flex; flex-wrap: wrap; gap: 3px; margin-top: 4px; }
.cl-pill {
  padding: 1px 7px; font-size: 10px;
  background: var(--cl-glass); border: 1px solid var(--cl-edge); border-radius: 999px;
}
.cl-pill:hover { background: var(--cl-glass-hover); }
.cl-pill.is-primary { background: var(--cl-accent-soft); border-color: var(--cl-accent-edge); }

/* Action buttons (Auswahlsperre, Woodplatte) */
.cl-btn {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 4px 8px;
  font-size: 11px;
  font-weight: 500;
  text-align: left;
  border-radius: 6px;
  border: 1px solid var(--cl-edge);
  background: var(--cl-glass);
  color: var(--cl-text);
  cursor: pointer;
  user-select: none;
  transition: all 0.15s ease;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
}
.cl-btn:hover {
  background: var(--cl-glass-hover);
  border-color: var(--cl-accent-edge);
  color: var(--cl-text);
}
.cl-btn:active {
  background: var(--cl-glass-active);
  transform: scale(0.98);
}
.cl-btn.is-active,
.cl-btn.is-on {
  background: var(--cl-accent-soft);
  border-color: var(--cl-accent-edge);
  color: var(--cl-accent);
  font-weight: 600;
}
.cl-btn-badge {
  font-size: 9.5px;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--cl-glass);
  border: 1px solid var(--cl-edge);
  color: var(--cl-text-dim);
  font-weight: 600;
  flex: none;
  transition: all 0.15s ease;
}
.cl-btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex: none;
}
.cl-btn-icon svg {
  width: 13px;
  height: 13px;
}
.cl-btn.is-active .cl-btn-badge,
.cl-btn.is-on .cl-btn-badge {
  background: var(--cl-accent);
  border-color: var(--cl-accent);
  color: #ffffff;
}
.cl-btn-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 4px;
  border-top: 1px solid var(--cl-edge);
  padding-top: 4px;
}

@media (prefers-reduced-motion: reduce) {
  .crystal-panel * { transition: none !important; }
}
`;

function ensureStyles() {
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement("style");
  el.id = STYLE_ID;
  el.textContent = STYLES;
  document.head.appendChild(el);
}

const svg = (d: string) =>
  `<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${d}</svg>`;

const ICONS = {
  close: svg('<path d="M6 6l12 12M18 6L6 18"/>'),
  eye: svg('<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>'),
  eyeOff: svg('<path d="M3 3l18 18"/><path d="M10.6 6.1A10 10 0 0 1 12 6c6.5 0 10 6 10 6a17 17 0 0 1-3.2 3.9M6.5 7.6C3.9 9.3 2 12 2 12s3.5 7 10 7c1.6 0 3-.4 4.3-1"/>'),
  lock: svg('<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>'),
  unlock: svg('<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 7.5-2"/>'),
  trash: svg('<path d="M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3"/>'),
  edit: svg('<path d="M4 20h4L19 9l-4-4L4 16v4z"/>'),
  plus: svg('<path d="M12 5v14M5 12h14"/>'),
  wood: svg('<path d="M4 6h16M4 12h16M4 18h16"/><path d="M7 6v12M17 6v12"/>'),
};

export class RightSidebar {
  private host: HTMLElement;
  private layout: Layout;
  private callbacks: RightSidebarCallbacks;

  constructor(host: HTMLElement, layout: Layout, callbacks: RightSidebarCallbacks) {
    this.host = host;
    this.layout = layout;
    this.callbacks = callbacks;
    ensureStyles();
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