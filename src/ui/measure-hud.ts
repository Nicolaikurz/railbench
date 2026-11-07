import { getTranslations } from "@/i18n";

export interface MeasureState {
  isActive: boolean;
  start: { x: number; y: number } | null;
  current: { x: number; y: number } | null;
  locked: boolean;
  snapEnabled: boolean;
  scale: string;
}

export interface MeasureHudCallbacks {
  onToggleSnap: (enabled: boolean) => void;
  onReset: () => void;
  onClose: () => void;
  onManualCoordsChange: (coords: {
    ax?: number;
    ay?: number;
    bx?: number;
    by?: number;
  }) => void;
}

export function getScaleRatio(scale: string): number {
  switch (scale) {
    case "H0": return 87;
    case "TT": return 120;
    case "N": return 160;
    case "Z": return 220;
    case "1": return 32;
    case "G": return 22.5;
    default: return 87;
  }
}

export class MeasureHud {
  private element: HTMLDivElement;
  private callbacks: MeasureHudCallbacks;

  private snapBtn: HTMLButtonElement | null;
  private resetBtn: HTMLButtonElement | null;
  private closeBtn: HTMLButtonElement | null;
  private axInput: HTMLInputElement | null;
  private ayInput: HTMLInputElement | null;
  private bxInput: HTMLInputElement | null;
  private byInput: HTMLInputElement | null;
  private statusEl: HTMLElement | null;
  private distEl: HTMLElement | null;
  private protoEl: HTMLElement | null;
  private deltasEl: HTMLElement | null;

  constructor(callbacks: MeasureHudCallbacks) {
    this.callbacks = callbacks;

    this.element = document.createElement("div");
    this.element.id = "measure-hud";
    this.element.className = "measure-hud";
    this.element.style.display = "none";
    this.element.innerHTML = `
      <div class="measure-hud-header">
        <span class="measure-hud-title">Distanzmessung</span>
        <span class="measure-hud-status" id="measure-hud-status"></span>
        <div class="measure-hud-actions">
          <button type="button" class="measure-hud-btn active" id="measure-hud-snap" title="Objektfang an/aus (Alt)">Fang: AN</button>
          <button type="button" class="measure-hud-btn" id="measure-hud-reset" title="Neu messen (Esc)">Neu</button>
          <button type="button" class="measure-hud-close" id="measure-hud-close" title="Messen beenden (Esc)">✕</button>
        </div>
      </div>
      <div class="measure-hud-body">
        <div class="measure-hud-dist-box">
          <span class="measure-hud-dist" id="measure-hud-dist">0.0 mm</span>
          <span class="measure-hud-proto" id="measure-hud-proto"></span>
        </div>
        <div class="measure-hud-coords">
          <div class="measure-coord-group">
            <span class="measure-coord-tag tag-a">A</span>
            <label class="measure-coord-label">X <input type="number" step="0.5" id="measure-ax" class="measure-coord-input" placeholder="0" /></label>
            <label class="measure-coord-label">Y <input type="number" step="0.5" id="measure-ay" class="measure-coord-input" placeholder="0" /></label>
          </div>
          <div class="measure-coord-group">
            <span class="measure-coord-tag tag-b">B</span>
            <label class="measure-coord-label">X <input type="number" step="0.5" id="measure-bx" class="measure-coord-input" placeholder="0" /></label>
            <label class="measure-coord-label">Y <input type="number" step="0.5" id="measure-by" class="measure-coord-input" placeholder="0" /></label>
          </div>
        </div>
        <div class="measure-hud-deltas" id="measure-hud-deltas">
          ΔX: 0.0 mm • ΔY: 0.0 mm • 0.0°
        </div>
      </div>
    `;
    document.body.appendChild(this.element);

    this.snapBtn = this.element.querySelector("#measure-hud-snap");
    this.resetBtn = this.element.querySelector("#measure-hud-reset");
    this.closeBtn = this.element.querySelector("#measure-hud-close");
    this.axInput = this.element.querySelector("#measure-ax");
    this.ayInput = this.element.querySelector("#measure-ay");
    this.bxInput = this.element.querySelector("#measure-bx");
    this.byInput = this.element.querySelector("#measure-by");
    this.statusEl = this.element.querySelector("#measure-hud-status");
    this.distEl = this.element.querySelector("#measure-hud-dist");
    this.protoEl = this.element.querySelector("#measure-hud-proto");
    this.deltasEl = this.element.querySelector("#measure-hud-deltas");

    this.bindEvents();
  }

  private bindEvents(): void {
    this.snapBtn?.addEventListener("click", () => {
      const isNowActive = !this.snapBtn?.classList.contains("active");
      this.callbacks.onToggleSnap(isNowActive);
    });

    this.resetBtn?.addEventListener("click", () => {
      this.callbacks.onReset();
    });

    this.closeBtn?.addEventListener("click", () => {
      this.callbacks.onClose();
    });

    const onManualInput = () => {
      const ax = parseFloat(this.axInput?.value ?? "");
      const ay = parseFloat(this.ayInput?.value ?? "");
      const bx = parseFloat(this.bxInput?.value ?? "");
      const by = parseFloat(this.byInput?.value ?? "");
      this.callbacks.onManualCoordsChange({
        ax: isNaN(ax) ? undefined : ax,
        ay: isNaN(ay) ? undefined : ay,
        bx: isNaN(bx) ? undefined : bx,
        by: isNaN(by) ? undefined : by,
      });
    };

    [this.axInput, this.ayInput, this.bxInput, this.byInput].forEach((inp) => {
      inp?.addEventListener("input", onManualInput);
      inp?.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === "Escape") {
          inp.blur();
        }
      });
    });
  }

  public update(state: MeasureState, updateInputs = true): void {
    if (!state.isActive) {
      this.element.style.display = "none";
      return;
    }
    this.element.style.display = "flex";

    const t = getTranslations().toolbar.toggles;

    if (this.snapBtn) {
      this.snapBtn.textContent = state.snapEnabled ? t.measureSnapOn : t.measureSnapOff;
      this.snapBtn.classList.toggle("active", state.snapEnabled);
    }
    if (this.resetBtn) {
      this.resetBtn.textContent = t.measureReset;
    }

    if (!state.start) {
      if (this.statusEl) this.statusEl.textContent = t.measurePromptA;
      if (this.distEl) this.distEl.textContent = "0.0 mm";
      if (this.protoEl) this.protoEl.textContent = "";
      if (this.deltasEl) this.deltasEl.textContent = "ΔX: 0.0 mm • ΔY: 0.0 mm • 0.0°";
      if (updateInputs) {
        if (this.axInput && document.activeElement !== this.axInput) this.axInput.value = "";
        if (this.ayInput && document.activeElement !== this.ayInput) this.ayInput.value = "";
        if (this.bxInput && document.activeElement !== this.bxInput) this.bxInput.value = "";
        if (this.byInput && document.activeElement !== this.byInput) this.byInput.value = "";
      }
      return;
    }

    const p1 = state.start;
    const p2 = state.current || p1;
    const dx = p2.x - p1.x;
    const dy = p2.y - p1.y;
    const d = Math.hypot(dx, dy);
    const angle = (Math.atan2(dy, dx) * 180) / Math.PI;
    const scaleRatio = getScaleRatio(state.scale);

    if (this.statusEl) {
      this.statusEl.textContent = state.locked ? t.measureMeasured : t.measurePromptB;
    }

    if (this.distEl) {
      this.distEl.textContent = d >= 1000 ? `${d.toFixed(1)} mm (${(d / 1000).toFixed(2)} m)` : `${d.toFixed(1)} mm`;
    }

    if (this.protoEl) {
      if (scaleRatio && scaleRatio > 1 && d > 0.1) {
        const protoM = (d * scaleRatio) / 1000;
        this.protoEl.textContent = `(${t.measurePrototype} ${state.scale} 1:${scaleRatio}: ${protoM.toFixed(2)} m)`;
      } else {
        this.protoEl.textContent = "";
      }
    }

    if (this.deltasEl) {
      this.deltasEl.textContent = `ΔX: ${Math.abs(dx).toFixed(1)} mm • ΔY: ${Math.abs(dy).toFixed(1)} mm • ${Math.abs(angle).toFixed(1)}°`;
    }

    if (updateInputs) {
      if (this.axInput && document.activeElement !== this.axInput) this.axInput.value = p1.x.toFixed(1);
      if (this.ayInput && document.activeElement !== this.ayInput) this.ayInput.value = p1.y.toFixed(1);
      if (state.current) {
        if (this.bxInput && document.activeElement !== this.bxInput) this.bxInput.value = p2.x.toFixed(1);
        if (this.byInput && document.activeElement !== this.byInput) this.byInput.value = p2.y.toFixed(1);
      } else {
        if (this.bxInput && document.activeElement !== this.bxInput) this.bxInput.value = "";
        if (this.byInput && document.activeElement !== this.byInput) this.byInput.value = "";
      }
    }
  }

  public destroy(): void {
    this.element.remove();
  }
}
