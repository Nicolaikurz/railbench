import type { Layout, TrackPiece, Model3D } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import { isTurnoutEntry } from "@/core/simulation";
import { getTranslations, onLanguageChange, getLanguage } from "@/i18n";
import { isFlexTrack, getFlexTrackRampInfo } from "@/core/flextrack";

export interface SelectionBarCallbacks {
  onApplyHeight: (targetHeightMm: number) => void;
  onApplyAngle: (targetAngleDeg: number) => void;
  onSelectLayer: (layerId: string) => void;
  onOpenHeightDialog: () => void;
  onAdjustFlexTrack?: (pieceId: string, cutLength?: number, radius?: number, angle?: number) => void;
  onToggleFeeder: () => void;
  onToggleIsolator: () => void;
  onToggleCatenary: () => void;
  onToggleCatenaryFeeder: () => void;
  onToggleCatenaryIsolator: () => void;
  onToggleTurnoutDecoder: () => void;
  onToggleTurnoutLantern: () => void;
  onDelete: () => void;
  getSelectedPieceIds: () => string[];
  getLayout: () => Layout;
  getCatalogEntry: (piece: TrackPiece) => CatalogEntry | undefined;
}

const STYLE_ID = "selection-bar-overrides";

function injectSelectionBarStyles(): void {
  if (document.getElementById(STYLE_ID)) return;

  const style = document.createElement("style");
  style.id = STYLE_ID;
  style.textContent = `
    .selection-floating-bar {
      gap: 4px;
      padding: 4px 8px;
      max-width: calc(100% - 24px);
      overflow-x: auto;
      scrollbar-width: none;
      border-radius: 20px;
      box-shadow: 0 6px 20px rgba(0, 0, 0, 0.22);
      backdrop-filter: blur(8px);
    }
    .selection-floating-bar .sel-bar-btn {
      width: auto !important;
      min-width: 28px !important;
      max-width: none !important;
      height: 26px !important;
      padding: 2px 7px !important;
      font-size: 11.5px !important;
      font-weight: 600;
      border-radius: 10px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      text-align: center;
      box-sizing: border-box;
      user-select: none;
      cursor: pointer;
      background: var(--bg-surface);
      color: var(--text-main);
      border: 1px solid var(--border-main);
      transition: all 0.12s;
    }
    .selection-floating-bar .sel-bar-btn:hover {
      background: var(--bg-card-hover);
      border-color: var(--primary-color);
      color: var(--primary-color);
    }
    .selection-floating-bar .sel-bar-btn:active {
      transform: scale(0.96);
    }
    .selection-floating-bar .sel-bar-btn.active {
      background: var(--primary-color) !important;
      color: #ffffff !important;
      border-color: var(--primary-color) !important;
      box-shadow: 0 0 8px rgba(2, 132, 199, 0.45);
      font-weight: 700;
    }
    .selection-floating-bar .sel-bar-btn:not(.active):not(.sel-bar-btn-danger) {
      color: #94a3b8;
    }
    .selection-floating-bar .sel-bar-btn:not(.active):not(.sel-bar-btn-danger):hover {
      color: var(--text-main);
    }
    .selection-floating-bar #sel-bar-btn-feeder.active {
      background: #f59e0b !important;
      color: #ffffff !important;
      border-color: #f59e0b !important;
      box-shadow: 0 0 8px rgba(245, 158, 11, 0.45);
    }
    .selection-floating-bar #sel-bar-btn-isolator.active {
      background: #dc2626 !important;
      color: #ffffff !important;
      border-color: #dc2626 !important;
      box-shadow: 0 0 8px rgba(220, 38, 38, 0.45);
    }
    .selection-floating-bar .sel-bar-btn-danger:hover {
      background: #ef4444 !important;
      color: #ffffff !important;
      border-color: #ef4444 !important;
      box-shadow: 0 0 8px rgba(239, 68, 68, 0.4);
    }
    .selection-floating-bar .sel-bar-divider {
      width: 1px;
      height: 16px;
      background: var(--border-main);
      margin: 0 2px;
      flex-shrink: 0;
    }
    .selection-floating-bar .sel-bar-count {
      font-size: 11px;
      font-weight: 700;
      color: var(--text-sub);
      background: var(--bg-surface);
      border: 1px solid var(--border-main);
      padding: 3px 8px;
      border-radius: 12px;
      white-space: nowrap;
      max-width: 200px;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .selection-floating-bar .sel-bar-select {
      height: 26px;
      width: auto !important;
      max-width: 86px !important;
      padding: 2px 6px;
      font-size: 11px;
      font-weight: 600;
      border-radius: 10px;
      background: var(--bg-surface);
      color: var(--text-main);
      border: 1px solid var(--border-main);
      text-overflow: ellipsis;
      cursor: pointer;
    }
    .selection-floating-bar .sel-bar-angle-group {
      display: inline-flex;
      align-items: center;
      gap: 3px;
    }
    .selection-floating-bar .sel-bar-angle-label {
      font-size: 11px;
      font-weight: 700;
      color: var(--text-muted);
      cursor: default;
      user-select: none;
    }
    .selection-floating-bar .sel-bar-angle-input {
      width: 52px !important;
      height: 24px;
      padding: 1px 5px;
      font-size: 11px;
      font-weight: 700;
      border-radius: 8px;
      background: var(--bg-surface);
      color: var(--text-main);
      border: 1px solid var(--border-main);
      text-align: right;
    }
    #sel-bar-elev-input {
      width: 52px !important;
    }
    .selection-floating-bar .sel-bar-unit {
      font-size: 10px;
      font-weight: 600;
      color: var(--text-muted);
      user-select: none;
    }
  `;
  document.head.appendChild(style);
}

export class SelectionBar {
  private container: HTMLDivElement;
  private callbacks: SelectionBarCallbacks;
  private unsubscribeI18n: (() => void) | null = null;

  // DOM Elements
  private selBarCount: HTMLElement;
  private selBarLayerSelect: HTMLSelectElement;
  private selBarDivElev: HTMLElement | null = null;
  private selBarElevGroup: HTMLElement;
  private selBarElevLabel: HTMLElement | null;
  private selBarElevInput: HTMLInputElement;
  private selBarAngleLabel: HTMLElement | null;
  private selBarAngleInput: HTMLInputElement;
  private selBarDivFlex: HTMLElement | null = null;
  private selBarFlexGroup: HTMLElement | null = null;
  private selBarFlexLenInput: HTMLInputElement | null = null;
  private selBarFlexRadiusInput: HTMLInputElement | null = null;
  private selBarFlexAngleInput: HTMLInputElement | null = null;
  private selBarBtnHeight: HTMLButtonElement;
  private selBarDivPower: HTMLElement | null = null;
  private selBarBtnFeeder: HTMLButtonElement;
  private selBarBtnIsolator: HTMLButtonElement;
  private selBarDivCatenary: HTMLElement | null = null;
  private selBarBtnCatenary: HTMLButtonElement;
  private selBarBtnCatenaryFeed: HTMLButtonElement;
  private selBarBtnCatenaryIso: HTMLButtonElement;
  private selBarDivTurnout: HTMLElement | null = null;
  private selBarBtnDecoder: HTMLButtonElement;
  private selBarBtnLantern: HTMLButtonElement;
  private selBarDivDel: HTMLElement | null = null;
  private selBarBtnDelete: HTMLButtonElement;

  constructor(host: HTMLElement, callbacks: SelectionBarCallbacks) {
    this.callbacks = callbacks;
    injectSelectionBarStyles();

    this.container = document.createElement("div");
    this.container.id = "selection-floating-bar";
    this.container.className = "selection-floating-bar hidden";

    const initialSelT = getTranslations().selectionBar;
    const isDe = getLanguage() === "de";

    this.container.innerHTML = `
      <span class="sel-bar-count" id="sel-bar-count">0 ${initialSelT.piecePlural}</span>
      <div class="sel-bar-layer-group">
        <select id="sel-bar-layer-select" class="sel-bar-select" title="${initialSelT.layerLabel}"></select>
      </div>
      <span class="sel-bar-divider" id="sel-bar-div-elev"></span>
      <div class="sel-bar-angle-group" id="sel-bar-elev-group">
        <button id="sel-bar-btn-height" class="sel-bar-btn" title="${isDe ? "Gleishöhe & Steigung anpassen (Dialog)" : "Adjust track height & ramp (dialog)"}">⤉</button>
        <input type="number" id="sel-bar-elev-input" class="sel-bar-angle-input" step="any" placeholder="0" title="${isDe ? "Gleishöhe in mm (Enter zum Anwenden, ↑/↓ für ±1 mm, Shift für ±0.1)" : "Set exact height in mm (Enter to apply, Up/Down for ±1 mm, Shift for ±0.1)"}" />
        <span class="sel-bar-unit">mm</span>
      </div>
      <div class="sel-bar-angle-group">
        <span class="sel-bar-angle-label" id="sel-bar-angle-lbl" title="${initialSelT.angleLabel}">↻</span>
        <input type="number" id="sel-bar-angle-input" class="sel-bar-angle-input" step="any" placeholder="0" title="${isDe ? "Drehwinkel in Grad (Enter zum Anwenden, ↑/↓ für ±7.5°)" : "Set exact rotation angle in degrees (Enter to apply, Up/Down for ±7.5°)"}" />
        <span class="sel-bar-unit">°</span>
      </div>
      <span class="sel-bar-divider" id="sel-bar-div-flex" style="display:none"></span>
      <div class="sel-bar-angle-group" id="sel-bar-flex-group" style="display:none">
        <span class="sel-bar-angle-label" title="${isDe ? "Flexgleis Länge in mm zuschneiden" : "Cut flex track length in mm"}">✂ L:</span>
        <input type="number" id="sel-bar-flex-len" class="sel-bar-angle-input" step="any" placeholder="900" style="width:58px !important" title="${isDe ? "Länge in mm zuschneiden" : "Cut length in mm"}" />
        <span class="sel-bar-unit">mm</span>
        <span class="sel-bar-angle-label" title="${isDe ? "Bogenradius in mm (leer = gerade)" : "Curve radius in mm (empty = straight)"}">R:</span>
        <input type="number" id="sel-bar-flex-radius" class="sel-bar-angle-input" step="any" placeholder="-" style="width:52px !important" title="${isDe ? "Bogenradius in mm" : "Radius in mm"}" />
        <span class="sel-bar-unit">mm</span>
        <span class="sel-bar-angle-label" title="${isDe ? "Bogenwinkel in Grad (+ links, - rechts)" : "Arc angle (+ left, - right)"}">W:</span>
        <input type="number" id="sel-bar-flex-angle" class="sel-bar-angle-input" step="any" placeholder="0" style="width:48px !important" title="${isDe ? "Bogenwinkel in Grad" : "Angle in degrees"}" />
        <span class="sel-bar-unit">°</span>
      </div>
      <span class="sel-bar-divider" id="sel-bar-div-power"></span>
      <button id="sel-bar-btn-feeder" class="sel-bar-btn" title="Gleis-Einspeisung an/aus (F)">⚡</button>
      <button id="sel-bar-btn-isolator" class="sel-bar-btn" title="${initialSelT.isolatorInactiveTitle}">⊘</button>
      <span class="sel-bar-divider" id="sel-bar-div-catenary"></span>
      <button id="sel-bar-btn-catenary" class="sel-bar-btn" title="${initialSelT.catenaryInactiveTitle}">⌁</button>
      <button id="sel-bar-btn-catenary-feed" class="sel-bar-btn" title="${initialSelT.catenaryFeederInactiveTitle}">⌁+</button>
      <button id="sel-bar-btn-catenary-iso" class="sel-bar-btn" title="${initialSelT.catenaryIsolatorInactiveTitle}">⌁⊘</button>
      <span class="sel-bar-divider" id="sel-bar-div-turnout" style="display:none"></span>
      <button id="sel-bar-btn-decoder" class="sel-bar-btn" title="${initialSelT.decoderInactiveTitle}" style="display:none">📟</button>
      <button id="sel-bar-btn-lantern" class="sel-bar-btn" title="${initialSelT.lanternInactiveTitle}" style="display:none">💡</button>
      <span class="sel-bar-divider" id="sel-bar-div-del"></span>
      <button id="sel-bar-btn-delete" class="sel-bar-btn sel-bar-btn-danger" title="${initialSelT.deleteBtn} (Del)">🗑</button>
    `;

    host.appendChild(this.container);

    this.selBarCount = this.container.querySelector("#sel-bar-count") as HTMLElement;
    this.selBarLayerSelect = this.container.querySelector("#sel-bar-layer-select") as HTMLSelectElement;
    this.selBarDivElev = this.container.querySelector("#sel-bar-div-elev") as HTMLElement | null;
    this.selBarElevGroup = this.container.querySelector("#sel-bar-elev-group") as HTMLElement;
    this.selBarElevLabel = this.container.querySelector("#sel-bar-elev-label") as HTMLElement | null;
    this.selBarElevInput = this.container.querySelector("#sel-bar-elev-input") as HTMLInputElement;
    this.selBarAngleLabel = this.container.querySelector("#sel-bar-angle-lbl") as HTMLElement | null;
    this.selBarAngleInput = this.container.querySelector("#sel-bar-angle-input") as HTMLInputElement;
    this.selBarDivFlex = this.container.querySelector("#sel-bar-div-flex") as HTMLElement | null;
    this.selBarFlexGroup = this.container.querySelector("#sel-bar-flex-group") as HTMLElement | null;
    this.selBarFlexLenInput = this.container.querySelector("#sel-bar-flex-len") as HTMLInputElement | null;
    this.selBarFlexRadiusInput = this.container.querySelector("#sel-bar-flex-radius") as HTMLInputElement | null;
    this.selBarFlexAngleInput = this.container.querySelector("#sel-bar-flex-angle") as HTMLInputElement | null;
    this.selBarBtnHeight = this.container.querySelector("#sel-bar-btn-height") as HTMLButtonElement;
    this.selBarDivPower = this.container.querySelector("#sel-bar-div-power") as HTMLElement | null;
    this.selBarBtnFeeder = this.container.querySelector("#sel-bar-btn-feeder") as HTMLButtonElement;
    this.selBarBtnIsolator = this.container.querySelector("#sel-bar-btn-isolator") as HTMLButtonElement;
    this.selBarDivCatenary = this.container.querySelector("#sel-bar-div-catenary") as HTMLElement | null;
    this.selBarBtnCatenary = this.container.querySelector("#sel-bar-btn-catenary") as HTMLButtonElement;
    this.selBarBtnCatenaryFeed = this.container.querySelector("#sel-bar-btn-catenary-feed") as HTMLButtonElement;
    this.selBarBtnCatenaryIso = this.container.querySelector("#sel-bar-btn-catenary-iso") as HTMLButtonElement;
    this.selBarDivTurnout = this.container.querySelector("#sel-bar-div-turnout") as HTMLElement | null;
    this.selBarBtnDecoder = this.container.querySelector("#sel-bar-btn-decoder") as HTMLButtonElement;
    this.selBarBtnLantern = this.container.querySelector("#sel-bar-btn-lantern") as HTMLButtonElement;
    this.selBarDivDel = this.container.querySelector("#sel-bar-div-del") as HTMLElement | null;
    this.selBarBtnDelete = this.container.querySelector("#sel-bar-btn-delete") as HTMLButtonElement;

    this.bindEvents();

    this.unsubscribeI18n = onLanguageChange(() => {
      this.updateStaticTexts();
    });
  }

  private bindEvents(): void {
    const handleApplyHeight = () => {
      const target = parseFloat(this.selBarElevInput.value);
      if (!isNaN(target)) {
        this.callbacks.onApplyHeight(target);
      }
    };

    this.selBarElevInput.addEventListener("change", handleApplyHeight);
    this.selBarElevInput.addEventListener("keydown", (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        handleApplyHeight();
        this.selBarElevInput.blur();
      } else if (e.key === "Escape") {
        this.selBarElevInput.blur();
        this.update();
      } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
        e.preventDefault();
        const step = (e.shiftKey ? 0.1 : 1) * (e.key === "ArrowUp" ? 1 : -1);
        const cur = parseFloat(this.selBarElevInput.value) || 0;
        this.selBarElevInput.value = (Math.round((cur + step) * 10) / 10).toString();
        handleApplyHeight();
      }
    });

    const handleApplyAngle = () => {
      const rawVal = parseFloat(this.selBarAngleInput.value);
      if (!isNaN(rawVal)) {
        this.callbacks.onApplyAngle(rawVal);
      }
    };

    this.selBarAngleInput.addEventListener("change", handleApplyAngle);
    this.selBarAngleInput.addEventListener("keydown", (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        handleApplyAngle();
        this.selBarAngleInput.blur();
      } else if (e.key === "Escape") {
        this.update();
        this.selBarAngleInput.blur();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        const step = e.shiftKey ? 1 : 7.5;
        const cur = parseFloat(this.selBarAngleInput.value) || 0;
        this.selBarAngleInput.value = (Math.round((cur + step) * 10) / 10).toString();
        handleApplyAngle();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        const step = e.shiftKey ? 1 : 7.5;
        const cur = parseFloat(this.selBarAngleInput.value) || 0;
        this.selBarAngleInput.value = (Math.round((cur - step) * 10) / 10).toString();
        handleApplyAngle();
      }
    });

    const handleApplyFlex = () => {
      const selectedIds = this.callbacks.getSelectedPieceIds();
      if (selectedIds.length !== 1) return;
      const pieceId = selectedIds[0]!;
      const piece = this.callbacks.getLayout().pieces.find((p) => p.id === pieceId);
      const entry = piece ? this.callbacks.getCatalogEntry(piece) : undefined;
      const maxLen = entry?.length ?? 900;

      let len = this.selBarFlexLenInput ? parseFloat(this.selBarFlexLenInput.value) : undefined;
      if (len !== undefined && !isNaN(len)) {
        if (len > maxLen) {
          len = maxLen;
          if (this.selBarFlexLenInput) this.selBarFlexLenInput.value = String(maxLen);
        } else if (len < 15) {
          len = 15;
          if (this.selBarFlexLenInput) this.selBarFlexLenInput.value = "15";
        }
      }
      const r = this.selBarFlexRadiusInput && this.selBarFlexRadiusInput.value ? parseFloat(this.selBarFlexRadiusInput.value) : undefined;
      const a = this.selBarFlexAngleInput && this.selBarFlexAngleInput.value ? parseFloat(this.selBarFlexAngleInput.value) : undefined;
      this.callbacks.onAdjustFlexTrack?.(pieceId, isNaN(len as any) ? undefined : len, isNaN(r as any) ? undefined : r, isNaN(a as any) ? undefined : a);
    };

    [this.selBarFlexLenInput, this.selBarFlexRadiusInput, this.selBarFlexAngleInput].forEach((inp) => {
      inp?.addEventListener("change", handleApplyFlex);
      inp?.addEventListener("keydown", (e: KeyboardEvent) => {
        if (e.key === "Enter") {
          handleApplyFlex();
          inp.blur();
        } else if (e.key === "Escape") {
          this.update();
          inp.blur();
        }
      });
    });

    this.selBarLayerSelect.addEventListener("change", () => {
      if (this.selBarLayerSelect.value) {
        this.callbacks.onSelectLayer(this.selBarLayerSelect.value);
      }
    });

    this.selBarBtnHeight.addEventListener("click", () => {
      this.callbacks.onOpenHeightDialog();
    });

    this.selBarBtnFeeder.addEventListener("click", () => {
      this.callbacks.onToggleFeeder();
    });

    this.selBarBtnIsolator.addEventListener("click", () => {
      this.callbacks.onToggleIsolator();
    });

    this.selBarBtnCatenary.addEventListener("click", () => {
      this.callbacks.onToggleCatenary();
    });

    this.selBarBtnCatenaryFeed.addEventListener("click", () => {
      this.callbacks.onToggleCatenaryFeeder();
    });

    this.selBarBtnCatenaryIso.addEventListener("click", () => {
      this.callbacks.onToggleCatenaryIsolator();
    });

    this.selBarBtnDecoder.addEventListener("click", () => {
      this.callbacks.onToggleTurnoutDecoder();
    });

    this.selBarBtnLantern.addEventListener("click", () => {
      this.callbacks.onToggleTurnoutLantern();
    });

    this.selBarBtnDelete.addEventListener("click", () => {
      this.callbacks.onDelete();
    });
  }

  public updateStaticTexts(): void {
    const t = getTranslations().selectionBar;
    const isDe = getLanguage() === "de";
    if (this.selBarElevLabel) this.selBarElevLabel.textContent = isDe ? "Höhe:" : "Height:";
    if (this.selBarBtnHeight) {
      this.selBarBtnHeight.textContent = "⤉";
      this.selBarBtnHeight.title = isDe ? "Gleishöhe & Steigung anpassen (Dialog)" : "Adjust track height & ramp (dialog)";
    }
    if (this.selBarAngleLabel) {
      this.selBarAngleLabel.textContent = "↻";
      this.selBarAngleLabel.title = t.angleLabel;
    }
    if (this.selBarBtnCatenary) {
      this.selBarBtnCatenary.textContent = "⌁";
      this.selBarBtnCatenary.title = t.catenaryInactiveTitle;
    }
    if (this.selBarBtnCatenaryFeed) {
      this.selBarBtnCatenaryFeed.textContent = "⌁+";
      this.selBarBtnCatenaryFeed.title = t.catenaryFeederInactiveTitle;
    }
    if (this.selBarBtnCatenaryIso) {
      this.selBarBtnCatenaryIso.textContent = "⌁⊘";
      this.selBarBtnCatenaryIso.title = t.catenaryIsolatorInactiveTitle;
    }
    if (this.selBarBtnLantern) {
      this.selBarBtnLantern.textContent = "💡";
      this.selBarBtnLantern.title = t.lanternInactiveTitle;
    }
    if (this.selBarBtnDecoder) {
      this.selBarBtnDecoder.textContent = "📟";
      this.selBarBtnDecoder.title = t.decoderInactiveTitle;
    }
    if (this.selBarBtnDelete) {
      this.selBarBtnDelete.textContent = "🗑";
      this.selBarBtnDelete.title = `${t.deleteBtn} (Del)`;
    }
    this.update();
  }

  public update(): void {
    const selectedIds = this.callbacks.getSelectedPieceIds();
    if (selectedIds.length === 0) {
      this.container.classList.add("hidden");
      return;
    }
    this.container.classList.remove("hidden");

    const layout = this.callbacks.getLayout();
    const layers = layout.layers || [];
    const selectedPieces = layout.pieces.filter((p) => selectedIds.includes(p.id));
    const firstPiece = selectedPieces[0];
    const firstModel = layout.models?.find((m) => m.id === selectedIds[0]);
    const isDe = getLanguage() === "de";

    if (firstModel && selectedPieces.length === 0) {
      this.selBarCount.textContent = `📦 ${firstModel.name} · ${firstModel.width}×${firstModel.depth} mm (H: ${firstModel.height}mm)`;
      this.selBarElevGroup.style.display = "none";
      if (this.selBarDivElev) this.selBarDivElev.style.display = "none";
      this.selBarBtnHeight.style.display = "none";
      if (this.selBarDivPower) this.selBarDivPower.style.display = "none";
      this.selBarBtnFeeder.style.display = "none";
      this.selBarBtnIsolator.style.display = "none";
      if (this.selBarDivCatenary) this.selBarDivCatenary.style.display = "none";
      this.selBarBtnCatenary.style.display = "none";
      this.selBarBtnCatenaryFeed.style.display = "none";
      this.selBarBtnCatenaryIso.style.display = "none";
      if (this.selBarDivTurnout) this.selBarDivTurnout.style.display = "none";
      this.selBarBtnDecoder.style.display = "none";
      this.selBarBtnLantern.style.display = "none";
      if (this.selBarDivFlex) this.selBarDivFlex.style.display = "none";
      if (this.selBarFlexGroup) this.selBarFlexGroup.style.display = "none";

      const mLayer = layers.find((l) => l.id === firstModel.layerId) || layers[0];
      const modelLayerExists = layers.some((l) => l.id === firstModel.layerId);
      this.selBarLayerSelect.innerHTML = layers
        .map((l) => `<option value="${l.id}" ${modelLayerExists && l.id === mLayer?.id ? "selected" : ""}>${l.name}</option>`)
        .join("");
      if (!modelLayerExists) {
        this.selBarLayerSelect.insertAdjacentHTML(
          "afterbegin",
          `<option value="" selected disabled>⚠️ ${isDe ? "Gelöschte Ebene (Zuweisen)" : "Deleted layer (Assign)"}</option>`,
        );
      }
      if (document.activeElement !== this.selBarAngleInput) {
        this.selBarAngleInput.value = (Math.round(firstModel.transform.rotation * 100) / 100).toString();
      }
      return;
    }

    this.selBarElevGroup.style.display = "";
    if (this.selBarDivElev) this.selBarDivElev.style.display = "";
    this.selBarBtnHeight.style.display = "";
    if (this.selBarDivPower) this.selBarDivPower.style.display = "";
    this.selBarBtnFeeder.style.display = "";
    this.selBarBtnIsolator.style.display = "";
    if (this.selBarDivCatenary) this.selBarDivCatenary.style.display = "";
    this.selBarBtnCatenary.style.display = "";
    this.selBarBtnCatenaryFeed.style.display = "";
    this.selBarBtnCatenaryIso.style.display = "";
    const tSel = getTranslations().selectionBar;

    this.selBarBtnHeight.textContent = "⤉";
    this.selBarBtnHeight.title = isDe ? "Gleishöhe & Steigung anpassen (Dialog)" : "Adjust track height & ramp (dialog)";

    // Track Feeder (Einspeisung)
    const allHaveFeeder = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.powerFeed);
    this.selBarBtnFeeder.classList.toggle("active", allHaveFeeder);
    this.selBarBtnFeeder.textContent = "⚡";
    this.selBarBtnFeeder.title = allHaveFeeder
      ? (isDe ? "Gleis-Einspeisung entfernen (F)" : "Remove track power feeder (F)")
      : (isDe ? "Gleis-Einspeisung hinzufügen (F)" : "Add track power feeder (F)");

    // Track Isolator
    if (firstPiece?.isIsolator) {
      const idx = (firstPiece.isolatedConnectorIndex ?? 0) + 1;
      const total = firstPiece.connectors?.length || 2;
      this.selBarBtnIsolator.classList.add("active");
      this.selBarBtnIsolator.textContent = `⊘ ${idx}/${total}`;
      this.selBarBtnIsolator.title = tSel.isolatorActiveTitle.replace("{idx}", String(idx)).replace("{total}", String(total));
    } else {
      this.selBarBtnIsolator.classList.remove("active");
      this.selBarBtnIsolator.textContent = "⊘";
      this.selBarBtnIsolator.title = tSel.isolatorInactiveTitle;
    }

    // Oberleitung (Catenary)
    const allHaveCatenary = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.hasCatenary);
    this.selBarBtnCatenary.classList.toggle("active", allHaveCatenary);
    this.selBarBtnCatenary.textContent = "⌁";
    this.selBarBtnCatenary.title = allHaveCatenary ? tSel.catenaryActiveTitle : tSel.catenaryInactiveTitle;

    const allHaveCatenaryFeed = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.catenaryPowerFeed);
    this.selBarBtnCatenaryFeed.classList.toggle("active", allHaveCatenaryFeed);
    this.selBarBtnCatenaryFeed.textContent = "⌁+";
    this.selBarBtnCatenaryFeed.title = allHaveCatenaryFeed ? tSel.catenaryFeederActiveTitle : tSel.catenaryFeederInactiveTitle;

    if (firstPiece?.catenaryIsolator) {
      const idx = (firstPiece.catenaryIsolatedConnectorIndex ?? 0) + 1;
      const total = firstPiece.connectors?.length || 2;
      this.selBarBtnCatenaryIso.classList.add("active");
      this.selBarBtnCatenaryIso.textContent = `⌁⊘ ${idx}/${total}`;
      this.selBarBtnCatenaryIso.title = tSel.catenaryIsolatorActiveTitle.replace("{idx}", String(idx)).replace("{total}", String(total));
    } else {
      this.selBarBtnCatenaryIso.classList.remove("active");
      this.selBarBtnCatenaryIso.textContent = "⌁⊘";
      this.selBarBtnCatenaryIso.title = tSel.catenaryIsolatorInactiveTitle;
    }

    // Weichen-Optionen (Decoder & Laterne)
    const turnoutPieces = selectedPieces.filter((p) => {
      const entry = this.callbacks.getCatalogEntry(p);
      return isTurnoutEntry(entry);
    });

    if (turnoutPieces.length > 0) {
      if (this.selBarDivTurnout) this.selBarDivTurnout.style.display = "";
      this.selBarBtnDecoder.style.display = "";
      this.selBarBtnLantern.style.display = "";

      const allHaveDecoder = turnoutPieces.every((p) => !!p.hasDecoder);
      this.selBarBtnDecoder.classList.toggle("active", allHaveDecoder);
      this.selBarBtnDecoder.textContent = "📟";
      this.selBarBtnDecoder.title = allHaveDecoder ? tSel.decoderActiveTitle : tSel.decoderInactiveTitle;

      const allHaveLantern = turnoutPieces.every((p) => !!p.hasLantern);
      this.selBarBtnLantern.classList.toggle("active", allHaveLantern);
      this.selBarBtnLantern.textContent = "💡";
      this.selBarBtnLantern.title = allHaveLantern ? tSel.lanternActiveTitle : tSel.lanternInactiveTitle;
    } else {
      if (this.selBarDivTurnout) this.selBarDivTurnout.style.display = "none";
      this.selBarBtnDecoder.style.display = "none";
      this.selBarBtnLantern.style.display = "none";
    }

    this.selBarBtnDelete.textContent = "🗑";
    this.selBarBtnDelete.title = `${tSel.deleteBtn} (Del)`;

    const pieceWord = selectedIds.length === 1 ? tSel.pieceSingular : tSel.piecePlural;
    this.selBarCount.textContent = `${selectedIds.length} ${pieceWord}`;

    const firstLayerId = firstPiece?.layerId || layout.activeLayerId || layers[0]?.id;
    const layerExists = layers.some((l) => l.id === (firstPiece?.layerId || "layer-1"));
    const allSameLayer = selectedPieces.every((p) => (p.layerId || "layer-1") === (firstPiece?.layerId || "layer-1"));

    this.selBarLayerSelect.innerHTML = layers
      .map((l) => `<option value="${l.id}" ${allSameLayer && layerExists && l.id === firstLayerId ? "selected" : ""}>${l.name}</option>`)
      .join("");
    if (!layerExists) {
      this.selBarLayerSelect.insertAdjacentHTML(
        "afterbegin",
        `<option value="" selected disabled>⚠️ ${getLanguage() === "de" ? "Gelöschte Ebene (Zuweisen)" : "Deleted layer (Assign)"}</option>`,
      );
    } else if (!allSameLayer) {
      this.selBarLayerSelect.insertAdjacentHTML("afterbegin", `<option value="" selected disabled>${tSel.multipleLayers}</option>`);
    }

    const curLayer = layers.find((l) => l.id === firstLayerId) || layers[0];
    const baseH = curLayer?.heightMm ?? 0;

    let minH = Infinity;
    let maxH = -Infinity;
    let anyRamp = false;
    for (const p of selectedPieces) {
      const pLayer = layers.find((l) => l.id === (p.layerId || "layer-1")) || curLayer;
      const bH = pLayer?.heightMm ?? 0;
      const s = bH + (p.elevation?.start ?? 0);
      const e = bH + (p.elevation?.end ?? 0);
      if (Math.abs(s - e) > 0.1) anyRamp = true;
      if (s < minH) minH = s;
      if (e < minH) minH = e;
      if (s > maxH) maxH = s;
      if (e > maxH) maxH = e;
    }
    if (minH === Infinity) { minH = baseH; maxH = baseH; }

    const roundH = (v: number): number => (Math.abs(v) < 0.1 ? 0 : Math.round(v * 10) / 10);
    const minHStr = roundH(minH) > 0 ? `+${roundH(minH)}` : `${roundH(minH)}`;
    const maxHStr = roundH(maxH) > 0 ? `+${roundH(maxH)}` : `${roundH(maxH)}`;

    if (document.activeElement !== this.selBarElevInput) {
      if (anyRamp || Math.abs(minH - maxH) > 0.1) {
        this.selBarElevInput.value = "";
        this.selBarElevInput.placeholder = `${minHStr}..${maxHStr}`;
      } else {
        this.selBarElevInput.value = roundH(minH).toString();
        this.selBarElevInput.placeholder = "0";
      }
    }

    const activeAngle = firstPiece ? firstPiece.transform.rotation : 0;
    if (document.activeElement !== this.selBarAngleInput) {
      this.selBarAngleInput.value = (Math.round(activeAngle * 100) / 100).toString();
    }

    const firstPieceEntry = firstPiece ? this.callbacks.getCatalogEntry(firstPiece) : undefined;
    const isSingleFlex = selectedPieces.length === 1 && firstPiece && isFlexTrack(firstPieceEntry, firstPiece);

    if (isSingleFlex && firstPiece) {
      if (this.selBarDivFlex) this.selBarDivFlex.style.display = "";
      if (this.selBarFlexGroup) this.selBarFlexGroup.style.display = "";
      const maxLen = firstPieceEntry?.length ?? 900;
      if (this.selBarFlexLenInput) {
        this.selBarFlexLenInput.max = String(maxLen);
        this.selBarFlexLenInput.min = "15";
        if (document.activeElement !== this.selBarFlexLenInput) {
          this.selBarFlexLenInput.value = String(firstPiece.customLength ?? maxLen);
        }
      }
      if (document.activeElement !== this.selBarFlexRadiusInput && this.selBarFlexRadiusInput) {
        this.selBarFlexRadiusInput.value = firstPiece.customRadius !== undefined ? String(firstPiece.customRadius) : "";
      }
      if (document.activeElement !== this.selBarFlexAngleInput && this.selBarFlexAngleInput) {
        this.selBarFlexAngleInput.value = firstPiece.customAngle !== undefined ? String(firstPiece.customAngle) : "";
      }
    } else {
      if (this.selBarDivFlex) this.selBarDivFlex.style.display = "none";
      if (this.selBarFlexGroup) this.selBarFlexGroup.style.display = "none";
    }
  }

  public destroy(): void {
    if (this.unsubscribeI18n) {
      this.unsubscribeI18n();
      this.unsubscribeI18n = null;
    }
    this.container.remove();
  }
}
