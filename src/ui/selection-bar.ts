import type { Layout, TrackPiece, Model3D } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import { isTurnoutEntry } from "@/core/simulation";
import { getTranslations, onLanguageChange } from "@/i18n";

export interface SelectionBarCallbacks {
  onApplyHeight: (targetHeightMm: number) => void;
  onApplyAngle: (targetAngleDeg: number) => void;
  onSelectLayer: (layerId: string) => void;
  onOpenHeightDialog: () => void;
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
      gap: 6px;
      padding: 6px 10px;
      max-width: calc(100% - 24px);
      overflow-x: auto;
      scrollbar-width: none;
    }
    .selection-floating-bar .sel-bar-btn {
      padding: 4px 8px;
      font-size: 12px;
    }
    .selection-floating-bar .sel-bar-divider {
      margin: 0 2px;
    }
    .selection-floating-bar .sel-bar-select {
      width: 84px;
      max-width: 84px;
      padding: 2px 4px;
      font-size: 12px;
      text-overflow: ellipsis;
    }
    .selection-floating-bar .sel-bar-angle-input {
      width: 52px;
    }
    #sel-bar-elev-input {
      width: 84px;
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
  private selBarElevGroup: HTMLElement;
  private selBarElevLabel: HTMLElement | null;
  private selBarElevInput: HTMLInputElement;
  private selBarAngleLabel: HTMLElement | null;
  private selBarAngleInput: HTMLInputElement;
  private selBarBtnHeight: HTMLButtonElement;
  private selBarBtnFeeder: HTMLButtonElement;
  private selBarBtnIsolator: HTMLButtonElement;
  private selBarBtnCatenary: HTMLButtonElement;
  private selBarBtnCatenaryFeed: HTMLButtonElement;
  private selBarBtnCatenaryIso: HTMLButtonElement;
  private selBarBtnDecoder: HTMLButtonElement;
  private selBarBtnLantern: HTMLButtonElement;
  private selBarBtnDelete: HTMLButtonElement;

  constructor(host: HTMLElement, callbacks: SelectionBarCallbacks) {
    this.callbacks = callbacks;
    injectSelectionBarStyles();

    this.container = document.createElement("div");
    this.container.id = "selection-floating-bar";
    this.container.className = "selection-floating-bar hidden";

    const initialSelT = getTranslations().selectionBar;
    this.container.innerHTML = `
      <span class="sel-bar-count" id="sel-bar-count">0 ${initialSelT.piecePlural}</span>
      <div class="sel-bar-layer-group">
        <select id="sel-bar-layer-select" class="sel-bar-select" title="${initialSelT.layerLabel}"></select>
      </div>
      <div class="sel-bar-angle-group" id="sel-bar-elev-group">
        <button id="sel-bar-btn-height" class="sel-bar-btn" title="${initialSelT.heightRampBtn}">${initialSelT.heightLabel}</button>
        <input type="number" id="sel-bar-elev-input" class="sel-bar-angle-input" step="any" placeholder="0" title="Set exact height in mm (Enter to apply, Up/Down for ±1 mm, Shift for ±0.1)" />
        <span class="sel-bar-unit">mm</span>
      </div>
      <div class="sel-bar-angle-group">
        <span class="sel-bar-angle-label" id="sel-bar-angle-lbl">${initialSelT.angleLabel}</span>
        <input type="number" id="sel-bar-angle-input" class="sel-bar-angle-input" step="any" placeholder="0" title="Set exact rotation angle in degrees (Enter to apply, Up/Down for ±7.5°)" />
        <span class="sel-bar-unit">°</span>
      </div>
      <span class="sel-bar-divider"></span>
      <button id="sel-bar-btn-feeder" class="sel-bar-btn" title="Gleis-Einspeisung an/aus (F)">${initialSelT.trackFeederBtn}</button>
      <button id="sel-bar-btn-isolator" class="sel-bar-btn" title="${initialSelT.isolatorInactiveTitle}">${initialSelT.isolatorInactive}</button>
      <span class="sel-bar-divider"></span>
      <button id="sel-bar-btn-catenary" class="sel-bar-btn" title="${initialSelT.catenaryInactiveTitle}">${initialSelT.catenaryBtn}</button>
      <button id="sel-bar-btn-catenary-feed" class="sel-bar-btn" title="${initialSelT.catenaryFeederInactiveTitle}">${initialSelT.catenaryFeederBtn}</button>
      <button id="sel-bar-btn-catenary-iso" class="sel-bar-btn" title="${initialSelT.catenaryIsolatorInactiveTitle}">${initialSelT.catenaryIsolatorBtn}</button>
      <span class="sel-bar-divider"></span>
      <button id="sel-bar-btn-decoder" class="sel-bar-btn" title="${initialSelT.decoderInactiveTitle}">${initialSelT.decoderBtn}</button>
      <button id="sel-bar-btn-lantern" class="sel-bar-btn" title="${initialSelT.lanternInactiveTitle}">${initialSelT.lanternBtn}</button>
      <span class="sel-bar-divider"></span>
      <button id="sel-bar-btn-delete" class="sel-bar-btn sel-bar-btn-danger" title="${initialSelT.deleteBtn} (Del)">${initialSelT.deleteBtn}</button>
    `;

    host.appendChild(this.container);

    this.selBarCount = this.container.querySelector("#sel-bar-count") as HTMLElement;
    this.selBarLayerSelect = this.container.querySelector("#sel-bar-layer-select") as HTMLSelectElement;
    this.selBarElevGroup = this.container.querySelector("#sel-bar-elev-group") as HTMLElement;
    this.selBarElevLabel = this.container.querySelector("#sel-bar-elev-label") as HTMLElement | null;
    this.selBarElevInput = this.container.querySelector("#sel-bar-elev-input") as HTMLInputElement;
    this.selBarAngleLabel = this.container.querySelector("#sel-bar-angle-lbl") as HTMLElement | null;
    this.selBarAngleInput = this.container.querySelector("#sel-bar-angle-input") as HTMLInputElement;
    this.selBarBtnHeight = this.container.querySelector("#sel-bar-btn-height") as HTMLButtonElement;
    this.selBarBtnFeeder = this.container.querySelector("#sel-bar-btn-feeder") as HTMLButtonElement;
    this.selBarBtnIsolator = this.container.querySelector("#sel-bar-btn-isolator") as HTMLButtonElement;
    this.selBarBtnCatenary = this.container.querySelector("#sel-bar-btn-catenary") as HTMLButtonElement;
    this.selBarBtnCatenaryFeed = this.container.querySelector("#sel-bar-btn-catenary-feed") as HTMLButtonElement;
    this.selBarBtnCatenaryIso = this.container.querySelector("#sel-bar-btn-catenary-iso") as HTMLButtonElement;
    this.selBarBtnDecoder = this.container.querySelector("#sel-bar-btn-decoder") as HTMLButtonElement;
    this.selBarBtnLantern = this.container.querySelector("#sel-bar-btn-lantern") as HTMLButtonElement;
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
    if (this.selBarElevLabel) this.selBarElevLabel.textContent = t.heightLabel;
    if (this.selBarBtnHeight) {
      this.selBarBtnHeight.textContent = t.rampPrefix;
      this.selBarBtnHeight.title = t.heightRampBtn;
    }
    if (this.selBarAngleLabel) this.selBarAngleLabel.textContent = t.angleLabel;
    if (this.selBarBtnCatenary) {
      this.selBarBtnCatenary.textContent = t.catenaryBtn;
      this.selBarBtnCatenary.title = t.catenaryInactiveTitle;
    }
    if (this.selBarBtnCatenaryFeed) {
      this.selBarBtnCatenaryFeed.textContent = t.catenaryFeederBtn;
      this.selBarBtnCatenaryFeed.title = t.catenaryFeederInactiveTitle;
    }
    if (this.selBarBtnCatenaryIso) {
      this.selBarBtnCatenaryIso.textContent = t.catenaryIsolatorBtn;
      this.selBarBtnCatenaryIso.title = t.catenaryIsolatorInactiveTitle;
    }
    if (this.selBarBtnLantern) {
      this.selBarBtnLantern.textContent = t.lanternBtn;
      this.selBarBtnLantern.title = t.lanternInactiveTitle;
    }
    if (this.selBarBtnDecoder) {
      this.selBarBtnDecoder.textContent = t.decoderBtn;
      this.selBarBtnDecoder.title = t.decoderInactiveTitle;
    }
    if (this.selBarBtnDelete) {
      this.selBarBtnDelete.textContent = t.deleteBtn;
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

    if (firstModel && selectedPieces.length === 0) {
      this.selBarCount.textContent = `📦 ${firstModel.name} · ${firstModel.width}×${firstModel.depth} mm (H: ${firstModel.height}mm)`;
      this.selBarElevGroup.style.display = "none";
      this.selBarBtnHeight.style.display = "none";
      this.selBarBtnFeeder.style.display = "none";
      this.selBarBtnIsolator.style.display = "none";
      this.selBarBtnCatenary.style.display = "none";
      this.selBarBtnCatenaryFeed.style.display = "none";
      this.selBarBtnCatenaryIso.style.display = "none";
      this.selBarBtnDecoder.style.display = "none";
      this.selBarBtnLantern.style.display = "none";

      const mLayer = layers.find((l) => l.id === firstModel.layerId) || layers[0];
      this.selBarLayerSelect.innerHTML = layers
        .map((l) => `<option value="${l.id}" ${l.id === mLayer?.id ? "selected" : ""}>${l.name}</option>`)
        .join("");
      if (document.activeElement !== this.selBarAngleInput) {
        this.selBarAngleInput.value = (Math.round(firstModel.transform.rotation * 100) / 100).toString();
      }
      return;
    }

    this.selBarElevGroup.style.display = "";
    this.selBarBtnHeight.style.display = "";
    this.selBarBtnFeeder.style.display = "";
    this.selBarBtnIsolator.style.display = "";
    this.selBarBtnCatenary.style.display = "";
    this.selBarBtnCatenaryFeed.style.display = "";
    this.selBarBtnCatenaryIso.style.display = "";
    const tSel = getTranslations().selectionBar;

    // Track Feeder (Einspeisung)
    const allHaveFeeder = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.powerFeed);
    this.selBarBtnFeeder.classList.toggle("active", allHaveFeeder);

    // Track Isolator
    if (firstPiece?.isIsolator) {
      const idx = (firstPiece.isolatedConnectorIndex ?? 0) + 1;
      const total = firstPiece.connectors?.length || 2;
      this.selBarBtnIsolator.classList.add("active");
      this.selBarBtnIsolator.textContent = tSel.isolatorActive.replace("{idx}", String(idx)).replace("{total}", String(total));
      this.selBarBtnIsolator.title = tSel.isolatorActiveTitle.replace("{idx}", String(idx)).replace("{total}", String(total));
    } else {
      this.selBarBtnIsolator.classList.remove("active");
      this.selBarBtnIsolator.textContent = tSel.isolatorInactive;
      this.selBarBtnIsolator.title = tSel.isolatorInactiveTitle;
    }

    // Oberleitung (Catenary)
    const allHaveCatenary = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.hasCatenary);
    if (allHaveCatenary) {
      this.selBarBtnCatenary.classList.add("active");
      this.selBarBtnCatenary.textContent = tSel.catenaryActive;
      this.selBarBtnCatenary.title = tSel.catenaryActiveTitle;
    } else {
      this.selBarBtnCatenary.classList.remove("active");
      this.selBarBtnCatenary.textContent = tSel.catenaryBtn;
      this.selBarBtnCatenary.title = tSel.catenaryInactiveTitle;
    }

    const allHaveCatenaryFeed = selectedPieces.length > 0 && selectedPieces.every((p) => !!p.catenaryPowerFeed);
    if (allHaveCatenaryFeed) {
      this.selBarBtnCatenaryFeed.classList.add("active");
      this.selBarBtnCatenaryFeed.textContent = tSel.catenaryFeederActive;
      this.selBarBtnCatenaryFeed.title = tSel.catenaryFeederActiveTitle;
    } else {
      this.selBarBtnCatenaryFeed.classList.remove("active");
      this.selBarBtnCatenaryFeed.textContent = tSel.catenaryFeederBtn;
      this.selBarBtnCatenaryFeed.title = tSel.catenaryFeederInactiveTitle;
    }

    if (firstPiece?.catenaryIsolator) {
      const idx = (firstPiece.catenaryIsolatedConnectorIndex ?? 0) + 1;
      const total = firstPiece.connectors?.length || 2;
      this.selBarBtnCatenaryIso.classList.add("active");
      this.selBarBtnCatenaryIso.textContent = tSel.catenaryIsolatorActive.replace("{idx}", String(idx)).replace("{total}", String(total));
      this.selBarBtnCatenaryIso.title = tSel.catenaryIsolatorActiveTitle.replace("{idx}", String(idx)).replace("{total}", String(total));
    } else {
      this.selBarBtnCatenaryIso.classList.remove("active");
      this.selBarBtnCatenaryIso.textContent = tSel.catenaryIsolatorBtn;
      this.selBarBtnCatenaryIso.title = tSel.catenaryIsolatorInactiveTitle;
    }

    // Weichen-Optionen (Decoder & Laterne)
    const turnoutPieces = selectedPieces.filter((p) => {
      const entry = this.callbacks.getCatalogEntry(p);
      return isTurnoutEntry(entry);
    });

    if (turnoutPieces.length > 0) {
      this.selBarBtnDecoder.style.display = "";
      this.selBarBtnLantern.style.display = "";

      const allHaveDecoder = turnoutPieces.every((p) => !!p.hasDecoder);
      if (allHaveDecoder) {
        this.selBarBtnDecoder.classList.add("active");
        this.selBarBtnDecoder.textContent = tSel.decoderActive;
        this.selBarBtnDecoder.title = tSel.decoderActiveTitle;
      } else {
        this.selBarBtnDecoder.classList.remove("active");
        this.selBarBtnDecoder.textContent = tSel.decoderBtn;
        this.selBarBtnDecoder.title = tSel.decoderInactiveTitle;
      }

      const allHaveLantern = turnoutPieces.every((p) => !!p.hasLantern);
      if (allHaveLantern) {
        this.selBarBtnLantern.classList.add("active");
        this.selBarBtnLantern.textContent = tSel.lanternActive;
        this.selBarBtnLantern.title = tSel.lanternActiveTitle;
      } else {
        this.selBarBtnLantern.classList.remove("active");
        this.selBarBtnLantern.textContent = tSel.lanternBtn;
        this.selBarBtnLantern.title = tSel.lanternInactiveTitle;
      }
    } else {
      this.selBarBtnDecoder.style.display = "none";
      this.selBarBtnLantern.style.display = "none";
    }

    const pieceWord = selectedIds.length === 1 ? tSel.pieceSingular : tSel.piecePlural;
    this.selBarCount.textContent = `${selectedIds.length} ${pieceWord}`;

    const firstLayerId = firstPiece?.layerId || layout.activeLayerId || layers[0]?.id;
    const allSameLayer = selectedPieces.every((p) => (p.layerId || "layer-1") === (firstPiece?.layerId || "layer-1"));

    this.selBarLayerSelect.innerHTML = layers
      .map((l) => `<option value="${l.id}" ${allSameLayer && l.id === firstLayerId ? "selected" : ""}>${l.name}</option>`)
      .join("");
    if (!allSameLayer) {
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
  }

  public destroy(): void {
    if (this.unsubscribeI18n) {
      this.unsubscribeI18n();
      this.unsubscribeI18n = null;
    }
    this.container.remove();
  }
}
