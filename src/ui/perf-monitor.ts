import type { Renderer } from "@/render/renderer";
import type { Layout } from "@/core/types";

export interface PerfMonitorOptions {
  host: HTMLElement;
  renderer: Renderer;
  getLayout: () => Layout;
  onGenerateStressTracks?: (count: number) => void;
}

export class PerfMonitor {
  private container: HTMLElement;
  private isVisible = false;
  private animFrameId: number | null = null;
  private lastTime = performance.now();
  private frames = 0;
  private fps = 60;
  private frameTimeMs = 0;
  private renderer: Renderer;
  private getLayout: () => Layout;
  private onGenerateStressTracks?: (count: number) => void;

  private gpuInfo = { vendor: "Unknown", renderer: "WebGL" };

  constructor(options: PerfMonitorOptions) {
    this.renderer = options.renderer;
    this.getLayout = options.getLayout;
    this.onGenerateStressTracks = options.onGenerateStressTracks;

    this.container = document.createElement("div");
    this.container.id = "perf-monitor-overlay";
    this.container.className = "perf-monitor-overlay";
    this.container.style.position = "absolute";
    this.container.style.top = "10px";
    this.container.style.right = "10px";
    this.container.style.backgroundColor = "rgba(15, 23, 42, 0.88)";
    this.container.style.color = "#f8fafc";
    this.container.style.border = "1px solid rgba(51, 65, 85, 0.8)";
    this.container.style.borderRadius = "8px";
    this.container.style.padding = "10px 12px";
    this.container.style.fontSize = "11.5px";
    this.container.style.fontFamily = "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace";
    this.container.style.lineHeight = "1.5";
    this.container.style.backdropFilter = "blur(8px)";
    this.container.style.boxShadow = "0 8px 24px rgba(0, 0, 0, 0.4)";
    this.container.style.zIndex = "95";
    this.container.style.pointerEvents = "auto";
    this.container.style.display = "none";
    this.container.style.minWidth = "250px";

    options.host.appendChild(this.container);

    this.detectGpu();

    const savedPref = localStorage.getItem("railbench_show_perf_monitor") === "true";
    if (savedPref) {
      this.show();
    }
  }

  private detectGpu() {
    try {
      const gl =
        this.renderer.app.canvas.getContext("webgl2") ||
        this.renderer.app.canvas.getContext("webgl");
      if (gl) {
        const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
        if (debugInfo) {
          const vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) as string;
          const renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) as string;
          this.gpuInfo = {
            vendor: vendor || "GPU",
            renderer: renderer || "WebGL Hardware Accelerated",
          };
        }
      }
    } catch {}
  }

  show() {
    this.isVisible = true;
    this.container.style.display = "block";
    localStorage.setItem("railbench_show_perf_monitor", "true");
    this.lastTime = performance.now();
    this.frames = 0;
    this.loop();
  }

  hide() {
    this.isVisible = false;
    this.container.style.display = "none";
    localStorage.setItem("railbench_show_perf_monitor", "false");
    if (this.animFrameId !== null) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
  }

  toggle(): boolean {
    if (this.isVisible) {
      this.hide();
      return false;
    } else {
      this.show();
      return true;
    }
  }

  isOpen(): boolean {
    return this.isVisible;
  }

  private loop = () => {
    if (!this.isVisible) return;

    const now = performance.now();
    this.frames++;
    const delta = now - this.lastTime;

    if (delta >= 500) {
      this.fps = Math.round((this.frames * 1000) / delta);
      this.frameTimeMs = Math.round((delta / this.frames) * 10) / 10;
      this.frames = 0;
      this.lastTime = now;
      this.renderOverlay();
    }

    this.animFrameId = requestAnimationFrame(this.loop);
  };

  private renderOverlay() {
    const layout = this.getLayout();
    const pieceCount = layout.pieces.length;
    const modelCount = layout.models?.length || 0;

    // RAM calculation (Chrome/Edge/Electron have performance.memory)
    const perfMem = (performance as unknown as { memory?: { usedJSHeapSize: number; totalJSHeapSize: number } })
      .memory;
    let ramStr = "N/A (Browser Protected)";
    if (perfMem) {
      const usedMb = (perfMem.usedJSHeapSize / (1024 * 1024)).toFixed(1);
      const totalMb = (perfMem.totalJSHeapSize / (1024 * 1024)).toFixed(1);
      ramStr = `${usedMb} MB / ${totalMb} MB`;
    } else {
      // Estimated track memory footprint
      const estimatedMb = ((pieceCount * 1200 + modelCount * 50000) / (1024 * 1024)).toFixed(1);
      ramStr = `~${estimatedMb} MB (Layout Heap)`;
    }

    const fpsColor = this.fps >= 55 ? "#10b981" : this.fps >= 30 ? "#f59e0b" : "#ef4444";

    this.container.innerHTML = `
      <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid rgba(51, 65, 85, 0.8); padding-bottom: 6px; margin-bottom: 8px;">
        <span style="font-weight: 700; color: #38bdf8; display: flex; align-items: center; gap: 4px;">
          <span>⚡</span>
          <span>HARDWARE MONITOR</span>
        </span>
        <button id="perf-close-btn" style="background: none; border: none; color: #94a3b8; cursor: pointer; font-size: 13px;">✕</button>
      </div>

      <div style="display: flex; flex-direction: column; gap: 4px;">
        <div style="display: flex; justify-content: space-between;">
          <span style="color: #94a3b8;">FPS:</span>
          <span style="color: ${fpsColor}; font-weight: 700;">${this.fps} FPS <span style="font-size: 10px; color: #94a3b8;">(${this.frameTimeMs} ms)</span></span>
        </div>

        <div style="display: flex; justify-content: space-between;">
          <span style="color: #94a3b8;">CPU Frame Time:</span>
          <span style="color: #f8fafc; font-weight: 600;">${this.frameTimeMs} ms</span>
        </div>

        <div style="display: flex; justify-content: space-between;">
          <span style="color: #94a3b8;">RAM (JS Heap):</span>
          <span style="color: #f8fafc; font-weight: 600;">${ramStr}</span>
        </div>

        <div style="display: flex; justify-content: space-between;">
          <span style="color: #94a3b8;">Gleise & Objekte:</span>
          <span style="color: #f8fafc; font-weight: 600;">${pieceCount} Gleise, ${modelCount} 3D</span>
        </div>

        <div style="border-top: 1px solid rgba(51, 65, 85, 0.6); margin-top: 4px; padding-top: 4px;">
          <div style="color: #94a3b8; font-size: 10px; margin-bottom: 2px;">GPU VENDOR & ACCELERATION:</div>
          <div style="color: #38bdf8; font-size: 10.5px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;" title="${this.gpuInfo.renderer}">
            ${this.gpuInfo.renderer}
          </div>
        </div>

        ${
          this.onGenerateStressTracks
            ? `
          <div style="border-top: 1px solid rgba(51, 65, 85, 0.6); margin-top: 6px; padding-top: 6px;">
            <div style="color: #94a3b8; font-size: 10.5px; margin-bottom: 4px;">Stresstest (Auslastung prüfen):</div>
            <div style="display: flex; gap: 4px;">
              <button class="perf-stress-btn" data-count="500" style="flex: 1; background: #0284c7; color: white; border: none; border-radius: 4px; padding: 4px 6px; font-size: 10px; cursor: pointer; font-weight: 600;">+500</button>
              <button class="perf-stress-btn" data-count="1000" style="flex: 1; background: #0369a1; color: white; border: none; border-radius: 4px; padding: 4px 6px; font-size: 10px; cursor: pointer; font-weight: 600;">+1000</button>
            </div>
          </div>
        `
            : ""
        }
      </div>
    `;

    // Close button event
    const closeBtn = this.container.querySelector("#perf-close-btn");
    closeBtn?.addEventListener("click", () => this.hide());

    // Stress test buttons event
    const stressBtns = this.container.querySelectorAll<HTMLButtonElement>(".perf-stress-btn");
    stressBtns.forEach((btn) => {
      btn.addEventListener("click", () => {
        const count = parseInt(btn.dataset.count || "500", 10);
        this.onGenerateStressTracks?.(count);
      });
    });
  }
}

