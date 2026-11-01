import type { Container } from "pixi.js";

export interface RulerOptions {
  host: HTMLElement;
  world: Container;
  onCenterOrigin?: () => void;
  visible?: boolean;
}

export class Ruler {
  private host: HTMLElement;
  private world: Container;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private cornerBtn: HTMLElement;
  private isRulerVisible = true;
  private currentTheme: "light" | "dark" = "light";
  private mouseX = -1;
  private mouseY = -1;
  private hasMouse = false;
  private animFrameId: number | null = null;
  private onCenterOrigin?: () => void;

  readonly rulerSize = 24; // Pixel-Breite/Höhe des Lineals

  constructor(options: RulerOptions) {
    this.host = options.host;
    this.world = options.world;
    this.onCenterOrigin = options.onCenterOrigin;
    this.isRulerVisible = options.visible ?? true;

    const isDark = document.documentElement.getAttribute("data-theme") === "dark";
    this.currentTheme = isDark ? "dark" : "light";

    // 1. Lineal-Canvas
    this.canvas = document.createElement("canvas");
    this.canvas.className = "ruler-canvas";
    this.canvas.style.position = "absolute";
    this.canvas.style.top = "0";
    this.canvas.style.left = "0";
    this.canvas.style.width = "100%";
    this.canvas.style.height = "100%";
    this.canvas.style.pointerEvents = "none";
    this.canvas.style.zIndex = "10";
    this.host.appendChild(this.canvas);

    const ctx = this.canvas.getContext("2d");
    if (!ctx) throw new Error("Could not get 2D context for ruler");
    this.ctx = ctx;

    // 2. Klickbare Nullpunkt-Ecke oben links
    this.cornerBtn = document.createElement("button");
    this.cornerBtn.className = "ruler-corner-btn";
    this.cornerBtn.title = "Nullpunkt (0, 0) – Klicken zum Zentrieren";
    this.cornerBtn.innerHTML = `<span>mm</span>`;
    this.cornerBtn.style.position = "absolute";
    this.cornerBtn.style.top = "0";
    this.cornerBtn.style.left = "0";
    this.cornerBtn.style.width = `${this.rulerSize}px`;
    this.cornerBtn.style.height = `${this.rulerSize}px`;
    this.cornerBtn.style.zIndex = "15";
    this.cornerBtn.style.cursor = "pointer";
    this.cornerBtn.addEventListener("click", () => {
      this.onCenterOrigin?.();
      this.update();
    });
    this.host.appendChild(this.cornerBtn);

    // 3. Maus-Tracking für Fadenkreuz-Markierungen
    this.host.addEventListener("pointermove", (e) => {
      const rect = this.host.getBoundingClientRect();
      this.mouseX = e.clientX - rect.left;
      this.mouseY = e.clientY - rect.top;
      this.hasMouse = true;
      this.requestUpdate();
    });

    this.host.addEventListener("pointerleave", () => {
      this.hasMouse = false;
      this.requestUpdate();
    });

    window.addEventListener("resize", () => {
      this.update();
    });

    this.setVisible(this.isRulerVisible);
    this.update();
  }

  setVisible(visible: boolean) {
    this.isRulerVisible = visible;
    this.canvas.style.display = visible ? "block" : "none";
    this.cornerBtn.style.display = visible ? "flex" : "none";
    if (visible) {
      this.update();
    }
  }

  isVisible(): boolean {
    return this.isRulerVisible;
  }

  toggle(): boolean {
    this.setVisible(!this.isRulerVisible);
    return this.isRulerVisible;
  }

  setTheme(theme: "light" | "dark") {
    this.currentTheme = theme;
    this.update();
  }

  requestUpdate() {
    if (this.animFrameId !== null) return;
    this.animFrameId = requestAnimationFrame(() => {
      this.animFrameId = null;
      this.update();
    });
  }

  update() {
    if (!this.isRulerVisible) return;

    const w = this.host.clientWidth;
    const h = this.host.clientHeight;
    if (w <= 0 || h <= 0) return;

    const dpr = window.devicePixelRatio || 1;
    if (this.canvas.width !== Math.round(w * dpr) || this.canvas.height !== Math.round(h * dpr)) {
      this.canvas.width = Math.round(w * dpr);
      this.canvas.height = Math.round(h * dpr);
    }

    const ctx = this.ctx;
    ctx.save();
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const isDark = this.currentTheme === "dark";
    const bg = isDark ? "#0f172a" : "#f1f5f9";
    const border = isDark ? "#334155" : "#cbd5e1";
    const textCol = isDark ? "#94a3b8" : "#475569";
    const tickColMajor = isDark ? "#64748b" : "#64748b";
    const tickColMinor = isDark ? "#334155" : "#cbd5e1";
    const originCol = isDark ? "#38bdf8" : "#0284c7";
    const hairLineCol = isDark ? "#f43f5e" : "#e11d48";

    const R = this.rulerSize;
    const scale = this.world.scale.x;
    const wx0 = this.world.x;
    const wy0 = this.world.y;

    // 1. Hintergrundflächen für Horizontales und Vertikales Lineal
    ctx.fillStyle = bg;
    // Oben
    ctx.fillRect(R, 0, w - R, R);
    // Links
    ctx.fillRect(0, R, R, h - R);

    // 2. Randlinien
    ctx.strokeStyle = border;
    ctx.lineWidth = 1;
    // Horizontale Trennlinie
    ctx.beginPath();
    ctx.moveTo(0, R - 0.5);
    ctx.lineTo(w, R - 0.5);
    ctx.stroke();

    // Vertikale Trennlinie
    ctx.beginPath();
    ctx.moveTo(R - 0.5, 0);
    ctx.lineTo(R - 0.5, h);
    ctx.stroke();

    // 3. Schrittweite berechnen (Ziel: ca. 60-80 Pixel pro Hauptstrich)
    const step = this.calculateStep(scale);
    const subDivs = this.calculateSubdivisions(step);
    const subStep = step / subDivs;

    ctx.font = '9px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    ctx.textBaseline = "middle";

    // --- 4. TOP RULER (X-ACHSE) ---
    const minWorldX = (R - wx0) / scale;
    const maxWorldX = (w - wx0) / scale;
    const startX = Math.floor(minWorldX / step) * step;

    for (let wx = startX; wx <= maxWorldX + step; wx += step) {
      // Unterteilungsschritte (Minor Ticks)
      for (let s = 1; s < subDivs; s++) {
        const subW = wx + s * subStep;
        const subScreen = subW * scale + wx0;
        if (subScreen >= R && subScreen <= w) {
          const isMid = subDivs % 2 === 0 && s === subDivs / 2;
          const tickH = isMid ? 6 : 4;
          ctx.strokeStyle = tickColMinor;
          ctx.beginPath();
          ctx.moveTo(Math.round(subScreen) + 0.5, R - tickH);
          ctx.lineTo(Math.round(subScreen) + 0.5, R);
          ctx.stroke();
        }
      }

      // Hauptstrich (Major Tick)
      const sx = wx * scale + wx0;
      if (sx >= R && sx <= w) {
        const isOrigin = Math.abs(wx) < 0.001;
        ctx.strokeStyle = isOrigin ? originCol : tickColMajor;
        ctx.lineWidth = isOrigin ? 1.5 : 1;

        ctx.beginPath();
        ctx.moveTo(Math.round(sx) + 0.5, isOrigin ? 2 : R - 10);
        ctx.lineTo(Math.round(sx) + 0.5, R);
        ctx.stroke();

        // Label
        const label = this.formatCoord(wx);
        ctx.fillStyle = isOrigin ? originCol : textCol;
        ctx.textAlign = "left";
        ctx.fillText(label, sx + 3, 8);
      }
    }

    // --- 5. LEFT RULER (Y-ACHSE) ---
    const minWorldY = (R - wy0) / scale;
    const maxWorldY = (h - wy0) / scale;
    const startY = Math.floor(minWorldY / step) * step;

    for (let wy = startY; wy <= maxWorldY + step; wy += step) {
      // Unterteilungsschritte (Minor Ticks)
      for (let s = 1; s < subDivs; s++) {
        const subW = wy + s * subStep;
        const subScreen = subW * scale + wy0;
        if (subScreen >= R && subScreen <= h) {
          const isMid = subDivs % 2 === 0 && s === subDivs / 2;
          const tickW = isMid ? 6 : 4;
          ctx.strokeStyle = tickColMinor;
          ctx.beginPath();
          ctx.moveTo(R - tickW, Math.round(subScreen) + 0.5);
          ctx.lineTo(R, Math.round(subScreen) + 0.5);
          ctx.stroke();
        }
      }

      // Hauptstrich (Major Tick)
      const sy = wy * scale + wy0;
      if (sy >= R && sy <= h) {
        const isOrigin = Math.abs(wy) < 0.001;
        ctx.strokeStyle = isOrigin ? originCol : tickColMajor;
        ctx.lineWidth = isOrigin ? 1.5 : 1;

        ctx.beginPath();
        ctx.moveTo(isOrigin ? 2 : R - 10, Math.round(sy) + 0.5);
        ctx.lineTo(R, Math.round(sy) + 0.5);
        ctx.stroke();

        // Label vertikal / gedreht oder kompakt
        const label = this.formatCoord(wy);
        ctx.save();
        ctx.translate(8, sy - 2);
        ctx.rotate(-Math.PI / 2);
        ctx.textAlign = "right";
        ctx.fillStyle = isOrigin ? originCol : textCol;
        ctx.fillText(label, 0, 0);
        ctx.restore();
      }
    }

    // --- 6. MAUSPOSITION-INDIKATOR (HAIRLINES) ---
    if (this.hasMouse) {
      ctx.fillStyle = hairLineCol;
      ctx.strokeStyle = hairLineCol;
      ctx.lineWidth = 1;

      // X-Indikator
      if (this.mouseX >= R && this.mouseX <= w) {
        const mx = Math.round(this.mouseX) + 0.5;
        ctx.beginPath();
        ctx.moveTo(mx, 0);
        ctx.lineTo(mx, R);
        ctx.stroke();

        // Kleiner Pfeil nach unten
        ctx.beginPath();
        ctx.moveTo(mx - 3, 0);
        ctx.lineTo(mx + 3, 0);
        ctx.lineTo(mx, 4);
        ctx.closePath();
        ctx.fill();
      }

      // Y-Indikator
      if (this.mouseY >= R && this.mouseY <= h) {
        const my = Math.round(this.mouseY) + 0.5;
        ctx.beginPath();
        ctx.moveTo(0, my);
        ctx.lineTo(R, my);
        ctx.stroke();

        // Kleiner Pfeil nach rechts
        ctx.beginPath();
        ctx.moveTo(0, my - 3);
        ctx.lineTo(0, my + 3);
        ctx.lineTo(4, my);
        ctx.closePath();
        ctx.fill();
      }
    }

    ctx.restore();
  }

  private calculateStep(scale: number): number {
    const minPixelDist = 65;
    const candidateSteps = [10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000];
    for (const step of candidateSteps) {
      if (step * scale >= minPixelDist) {
        return step;
      }
    }
    // Wenn sehr weit herausgezoomt:
    let step = 10000;
    while (step * scale < minPixelDist) {
      step *= 2;
    }
    return step;
  }

  private calculateSubdivisions(step: number): number {
    const s = step.toString();
    if (s.startsWith("1")) return 10;
    if (s.startsWith("2")) return 4;
    if (s.startsWith("5")) return 5;
    return 5;
  }

  private formatCoord(val: number): string {
    const rounded = Math.round(val);
    if (Math.abs(rounded) < 0.001) return "0";
    if (Math.abs(rounded) >= 1000) {
      const m = rounded / 1000;
      return `${m % 1 === 0 ? m.toFixed(0) : m.toFixed(1)}m`;
    }
    return `${rounded}`;
  }
}

