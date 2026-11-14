import type { Renderer } from "@/render/renderer";
import type { Layout } from "@/core/types";

export interface PerfMonitorOptions {
  host: HTMLElement;
  renderer: Renderer;
  getLayout: () => Layout;
  onGenerateStressTracks?: (count: number) => void;
}

export class PerfMonitor {
  private host: HTMLElement;
  private container: HTMLElement;
  private headerEl!: HTMLElement;
  private closeBtnEl!: HTMLButtonElement;
  private bodyEl!: HTMLElement;
  private fpsValEl!: HTMLElement;
  private cpuValEl!: HTMLElement;
  private ramValEl!: HTMLElement;
  private tracksValEl!: HTMLElement;
  private gpuValEl!: HTMLElement;

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

  private electronAppRamMb: number | null = null;
  private electronSystemUsedMb: number | null = null;
  private electronSystemTotalMb: number | null = null;
  private uaSpecificMemoryBytes: number | null = null;
  private isPollingMemory = false;

  private isDragging = false;
  private dragStartX = 0;
  private dragStartY = 0;
  private initialLeft = 0;
  private initialTop = 0;

  constructor(options: PerfMonitorOptions) {
    this.host = options.host;
    this.renderer = options.renderer;
    this.getLayout = options.getLayout;
    this.onGenerateStressTracks = options.onGenerateStressTracks;

    this.container = document.createElement("div");
    this.container.id = "perf-monitor-overlay";
    this.container.className = "perf-monitor-overlay";
    this.container.style.position = "absolute";
    this.container.style.bottom = "10px";
    this.container.style.right = "10px";
    this.container.style.top = "auto";
    this.container.style.left = "auto";
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

    this.buildDom();
    this.initDrag();
    this.initSavedPosition();

    this.host.appendChild(this.container);

    this.detectGpu();

    if (typeof window !== "undefined") {
      window.addEventListener("resize", () => {
        if (this.isVisible) {
          this.clampPosition();
        }
      });
    }

    const savedPref = typeof localStorage !== "undefined" && localStorage.getItem("railbench_show_perf_monitor") === "true";
    if (savedPref) {
      this.show();
    }
  }

  private buildDom(): void {
    this.headerEl = document.createElement("div");
    this.headerEl.className = "perf-monitor-header";
    this.headerEl.style.display = "flex";
    this.headerEl.style.alignItems = "center";
    this.headerEl.style.justifyContent = "space-between";
    this.headerEl.style.borderBottom = "1px solid rgba(51, 65, 85, 0.8)";
    this.headerEl.style.paddingBottom = "6px";
    this.headerEl.style.marginBottom = "8px";
    this.headerEl.style.cursor = "grab";
    this.headerEl.style.userSelect = "none";
    this.headerEl.style.touchAction = "none";
    this.headerEl.title = "Ziehen zum Verschieben (Doppelklick zum Zurücksetzen) / Drag to move (Double-click to reset)";

    this.headerEl.innerHTML = `
      <span style="font-weight: 700; color: #38bdf8; display: flex; align-items: center; gap: 6px; pointer-events: none;">
        <span style="color: #64748b; font-size: 13px; line-height: 1;">⠿</span>
        <span>⚡</span>
        <span>HARDWARE MONITOR</span>
      </span>
      <button id="perf-close-btn" style="background: none; border: none; color: #94a3b8; cursor: pointer; font-size: 13px; padding: 2px 5px; border-radius: 4px; line-height: 1; transition: all 0.15s;" title="Schließen (F3)">✕</button>
    `;

    this.closeBtnEl = this.headerEl.querySelector("#perf-close-btn") as HTMLButtonElement;
    this.closeBtnEl.addEventListener("click", (e) => {
      e.stopPropagation();
      this.hide();
    });
    this.closeBtnEl.addEventListener("mouseenter", () => {
      this.closeBtnEl.style.color = "#f8fafc";
      this.closeBtnEl.style.backgroundColor = "rgba(255, 255, 255, 0.1)";
    });
    this.closeBtnEl.addEventListener("mouseleave", () => {
      this.closeBtnEl.style.color = "#94a3b8";
      this.closeBtnEl.style.backgroundColor = "transparent";
    });

    this.bodyEl = document.createElement("div");
    this.bodyEl.className = "perf-monitor-body";
    this.bodyEl.style.display = "flex";
    this.bodyEl.style.flexDirection = "column";
    this.bodyEl.style.gap = "4px";

    this.bodyEl.innerHTML = `
      <div style="display: flex; justify-content: space-between;">
        <span style="color: #94a3b8;">FPS:</span>
        <span id="perf-fps-val" style="color: #10b981; font-weight: 700;">60 FPS <span style="font-size: 10px; color: #94a3b8;">(16.6 ms)</span></span>
      </div>

      <div style="display: flex; justify-content: space-between;">
        <span style="color: #94a3b8;">CPU Frame Time:</span>
        <span id="perf-cpu-val" style="color: #f8fafc; font-weight: 600;">0.0 ms</span>
      </div>

      <div style="display: flex; justify-content: space-between;">
        <span style="color: #94a3b8;">RAM Auslastung:</span>
        <span id="perf-ram-val" style="color: #f8fafc; font-weight: 600;">--</span>
      </div>

      <div style="display: flex; justify-content: space-between;">
        <span style="color: #94a3b8;">Gleise & Objekte:</span>
        <span id="perf-tracks-val" style="color: #f8fafc; font-weight: 600;">0 Gleise, 0 3D</span>
      </div>

      <div style="border-top: 1px solid rgba(51, 65, 85, 0.6); margin-top: 4px; padding-top: 4px;">
        <div style="color: #94a3b8; font-size: 10px; margin-bottom: 2px;">GPU VENDOR & ACCELERATION:</div>
        <div id="perf-gpu-val" style="color: #38bdf8; font-size: 10.5px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
          WebGL Hardware Accelerated
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
    `;

    this.fpsValEl = this.bodyEl.querySelector("#perf-fps-val") as HTMLElement;
    this.cpuValEl = this.bodyEl.querySelector("#perf-cpu-val") as HTMLElement;
    this.ramValEl = this.bodyEl.querySelector("#perf-ram-val") as HTMLElement;
    this.tracksValEl = this.bodyEl.querySelector("#perf-tracks-val") as HTMLElement;
    this.gpuValEl = this.bodyEl.querySelector("#perf-gpu-val") as HTMLElement;

    if (this.onGenerateStressTracks) {
      const stressBtns = this.bodyEl.querySelectorAll<HTMLButtonElement>(".perf-stress-btn");
      stressBtns.forEach((btn) => {
        btn.addEventListener("click", () => {
          const count = parseInt(btn.dataset.count || "500", 10);
          this.onGenerateStressTracks?.(count);
        });
      });
    }

    this.container.appendChild(this.headerEl);
    this.container.appendChild(this.bodyEl);
  }

  private initDrag(): void {
    this.headerEl.addEventListener("pointerdown", this.onDragStart);
    this.headerEl.addEventListener("dblclick", () => {
      this.resetPosition();
    });
  }

  private onDragStart = (e: PointerEvent): void => {
    if ((e.target as HTMLElement)?.closest?.("#perf-close-btn")) {
      return;
    }
    if (e.button !== 0) return;

    e.preventDefault();
    this.isDragging = true;
    this.dragStartX = e.clientX;
    this.dragStartY = e.clientY;

    const hostRect = typeof this.host.getBoundingClientRect === "function"
      ? this.host.getBoundingClientRect()
      : { left: 0, top: 0, width: 0, height: 0 };
    const containerRect = typeof this.container.getBoundingClientRect === "function"
      ? this.container.getBoundingClientRect()
      : { left: 0, top: 0, width: 0, height: 0 };

    if (containerRect.width && hostRect.width) {
      this.initialLeft = containerRect.left - hostRect.left;
      this.initialTop = containerRect.top - hostRect.top;
    } else {
      const hostW = this.host?.clientWidth || 800;
      const hostH = this.host?.clientHeight || 600;
      const overlayW = this.container.offsetWidth || 250;
      const overlayH = this.container.offsetHeight || 180;
      this.initialLeft =
        parseFloat(this.container.style.left) ||
        this.container.offsetLeft ||
        (this.container.style.right ? hostW - overlayW - (parseFloat(this.container.style.right) || 10) : 0);
      this.initialTop =
        parseFloat(this.container.style.top) ||
        this.container.offsetTop ||
        (this.container.style.bottom ? hostH - overlayH - (parseFloat(this.container.style.bottom) || 10) : 10);
    }

    this.container.style.right = "auto";
    this.container.style.bottom = "auto";
    this.container.style.left = `${Math.round(this.initialLeft)}px`;
    this.container.style.top = `${Math.round(this.initialTop)}px`;

    this.headerEl.style.cursor = "grabbing";
    if (typeof document !== "undefined" && document.body) {
      document.body.style.userSelect = "none";
    }

    if (typeof this.headerEl.setPointerCapture === "function") {
      try {
        this.headerEl.setPointerCapture(e.pointerId);
      } catch {}
    }

    if (typeof window !== "undefined") {
      window.addEventListener("pointermove", this.onDragMove);
      window.addEventListener("pointerup", this.onDragEnd);
      window.addEventListener("pointercancel", this.onDragEnd);
    }
  };

  private onDragMove = (e: PointerEvent): void => {
    if (!this.isDragging) return;

    const dx = e.clientX - this.dragStartX;
    const dy = e.clientY - this.dragStartY;

    let newLeft = this.initialLeft + dx;
    let newTop = this.initialTop + dy;

    const hostWidth = this.host?.clientWidth ?? 0;
    const hostHeight = this.host?.clientHeight ?? 0;
    const overlayWidth = this.container.offsetWidth || 250;
    const overlayHeight = this.container.offsetHeight || 180;

    if (hostWidth > 0 && hostWidth > overlayWidth) {
      const maxLeft = hostWidth - overlayWidth;
      newLeft = Math.max(0, Math.min(newLeft, maxLeft));
    } else {
      newLeft = Math.max(0, newLeft);
    }

    if (hostHeight > 0 && hostHeight > overlayHeight) {
      const maxTop = hostHeight - overlayHeight;
      newTop = Math.max(0, Math.min(newTop, maxTop));
    } else {
      newTop = Math.max(0, newTop);
    }

    this.container.style.left = `${Math.round(newLeft)}px`;
    this.container.style.top = `${Math.round(newTop)}px`;
  };

  private onDragEnd = (e?: PointerEvent): void => {
    if (!this.isDragging) return;
    this.isDragging = false;
    this.headerEl.style.cursor = "grab";
    if (typeof document !== "undefined" && document.body) {
      document.body.style.userSelect = "";
    }

    if (e && typeof this.headerEl.hasPointerCapture === "function") {
      try {
        if (this.headerEl.hasPointerCapture(e.pointerId)) {
          this.headerEl.releasePointerCapture(e.pointerId);
        }
      } catch {}
    }

    if (typeof window !== "undefined") {
      window.removeEventListener("pointermove", this.onDragMove);
      window.removeEventListener("pointerup", this.onDragEnd);
      window.removeEventListener("pointercancel", this.onDragEnd);
    }

    const currentLeft = parseFloat(this.container.style.left);
    const currentTop = parseFloat(this.container.style.top);
    if (!isNaN(currentLeft) && !isNaN(currentTop) && typeof localStorage !== "undefined") {
      try {
        localStorage.setItem(
          "railbench_perf_monitor_pos",
          JSON.stringify({ left: currentLeft, top: currentTop })
        );
      } catch {}
    }
  };

  private initSavedPosition(): void {
    if (typeof localStorage === "undefined") return;
    try {
      const savedPosStr = localStorage.getItem("railbench_perf_monitor_pos");
      if (savedPosStr) {
        const savedPos = JSON.parse(savedPosStr);
        if (typeof savedPos?.left === "number" && typeof savedPos?.top === "number") {
          this.setPosition(savedPos.left, savedPos.top);
        }
      }
    } catch {}
  }

  setPosition(left: number, top: number): void {
    let clampedLeft = Math.max(0, left);
    let clampedTop = Math.max(0, top);

    const hostWidth = this.host?.clientWidth ?? 0;
    const hostHeight = this.host?.clientHeight ?? 0;
    const overlayWidth = this.container.offsetWidth || 250;
    const overlayHeight = this.container.offsetHeight || 180;

    if (hostWidth > 0 && hostWidth > overlayWidth) {
      clampedLeft = Math.min(clampedLeft, hostWidth - overlayWidth);
    }
    if (hostHeight > 0 && hostHeight > overlayHeight) {
      clampedTop = Math.min(clampedTop, hostHeight - overlayHeight);
    }

    this.container.style.right = "auto";
    this.container.style.bottom = "auto";
    this.container.style.left = `${Math.round(clampedLeft)}px`;
    this.container.style.top = `${Math.round(clampedTop)}px`;
  }

  resetPosition(): void {
    this.container.style.left = "auto";
    this.container.style.top = "auto";
    this.container.style.bottom = "10px";
    this.container.style.right = "10px";
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem("railbench_perf_monitor_pos");
      } catch {}
    }
  }

  getPosition(): { left: number; top: number } | null {
    const left = parseFloat(this.container.style.left);
    const top = parseFloat(this.container.style.top);
    if (!isNaN(left) && !isNaN(top)) {
      return { left, top };
    }
    return null;
  }

  clampPosition(): void {
    if (!this.host) return;
    const currentLeft = parseFloat(this.container.style.left);
    const currentTop = parseFloat(this.container.style.top);
    if (isNaN(currentLeft) || isNaN(currentTop)) return;

    this.setPosition(currentLeft, currentTop);
  }

  private detectGpu() {
    try {
      const gl =
        this.renderer.app?.canvas?.getContext?.("webgl2") ||
        this.renderer.app?.canvas?.getContext?.("webgl");
      if (gl) {
        const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
        if (debugInfo) {
          const vendor = gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL) as string;
          const renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) as string;
          this.gpuInfo = {
            vendor: vendor || "GPU",
            renderer: renderer || "WebGL Hardware Accelerated",
          };
          if (this.gpuValEl) {
            this.gpuValEl.textContent = this.gpuInfo.renderer;
            this.gpuValEl.title = this.gpuInfo.renderer;
          }
        }
      }
    } catch {}
  }

  show() {
    this.isVisible = true;
    this.container.style.display = "block";
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_show_perf_monitor", "true");
      } catch {}
    }
    this.clampPosition();
    this.pollMemoryInfo();
    this.renderOverlay();
    this.lastTime = performance.now();
    this.frames = 0;
    this.loop();
  }

  hide() {
    this.isVisible = false;
    this.container.style.display = "none";
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_show_perf_monitor", "false");
      } catch {}
    }
    if (this.animFrameId !== null && typeof cancelAnimationFrame !== "undefined") {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = null;
    }
    if (this.isDragging) {
      this.onDragEnd();
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

  private pollMemoryInfo(): void {
    if (this.isPollingMemory) return;

    const electronApi = typeof window !== "undefined" ? window.electronAPI : undefined;
    if (electronApi?.getMemoryUsage) {
      this.isPollingMemory = true;
      electronApi
        .getMemoryUsage()
        .then((res) => {
          if (res && typeof res.appRamMb === "number") {
            this.electronAppRamMb = res.appRamMb;
            this.electronSystemUsedMb = res.systemUsedMb;
            this.electronSystemTotalMb = res.systemTotalMb;
            if (this.isVisible) {
              this.renderOverlay();
            }
          }
        })
        .catch(() => {})
        .finally(() => {
          this.isPollingMemory = false;
        });
      return;
    }

    const perfAny = typeof performance !== "undefined" ? (performance as any) : null;
    if (perfAny && typeof perfAny.measureUserAgentSpecificMemory === "function") {
      this.isPollingMemory = true;
      perfAny
        .measureUserAgentSpecificMemory()
        .then((res: { bytes?: number }) => {
          if (res && typeof res.bytes === "number" && res.bytes > 0) {
            this.uaSpecificMemoryBytes = res.bytes;
            if (this.isVisible) {
              this.renderOverlay();
            }
          }
        })
        .catch(() => {})
        .finally(() => {
          this.isPollingMemory = false;
        });
    }
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
      this.pollMemoryInfo();
      this.renderOverlay();
    }

    if (typeof requestAnimationFrame !== "undefined") {
      this.animFrameId = requestAnimationFrame(this.loop);
    }
  };

  private renderOverlay() {
    const layout = this.getLayout();
    const pieceCount = layout?.pieces?.length || 0;
    const modelCount = layout?.models?.length || 0;
    const boardCount = layout?.baseboards?.length || 0;

    // 1. Check Electron native process & system memory first
    const electronApi = typeof window !== "undefined" ? window.electronAPI : undefined;
    const procMem = electronApi?.getProcessMemoryInfo?.() ?? null;
    const perfMem = (
      typeof performance !== "undefined"
        ? (performance as unknown as {
            memory?: { usedJSHeapSize: number; totalJSHeapSize: number; jsHeapSizeLimit?: number };
          })?.memory
        : undefined
    );

    // WebGL framebuffer + PixiJS sprite batches + 3D geometry/texture buffers
    const gpuSceneBufferMb =
      28.0 + (pieceCount * 48000 + modelCount * 380000 + boardCount * 260000) / (1024 * 1024);

    let ramStr = "--";
    let ramTooltip = "";

    if (this.electronAppRamMb !== null || procMem) {
      const appMb =
        this.electronAppRamMb !== null && procMem
          ? Math.max(this.electronAppRamMb, procMem.rssMb)
          : (this.electronAppRamMb ?? procMem!.rssMb);
      const heapMb =
        procMem?.heapUsedMb ?? (perfMem ? perfMem.usedJSHeapSize / (1024 * 1024) : null);
      const sysUsedGb = (this.electronSystemUsedMb ?? procMem?.systemUsedMb ?? 0) / 1024;
      const sysTotalGb = (this.electronSystemTotalMb ?? procMem?.systemTotalMb ?? 0) / 1024;

      ramStr =
        heapMb !== null
          ? `${appMb.toFixed(1)} MB (Heap: ${heapMb.toFixed(1)} MB)`
          : `${appMb.toFixed(1)} MB`;
      if (sysTotalGb > 0) {
        ramTooltip = `App RAM (RSS): ${appMb.toFixed(1)} MB | JS Heap: ${(heapMb ?? 0).toFixed(1)} MB | System RAM: ${sysUsedGb.toFixed(1)} / ${sysTotalGb.toFixed(1)} GB`;
      }
    } else if (this.uaSpecificMemoryBytes !== null && this.uaSpecificMemoryBytes > 0) {
      const totalMb = this.uaSpecificMemoryBytes / (1024 * 1024);
      const heapMb = perfMem ? perfMem.usedJSHeapSize / (1024 * 1024) : null;
      ramStr =
        heapMb !== null
          ? `${totalMb.toFixed(1)} MB (Heap: ${heapMb.toFixed(1)} MB)`
          : `${totalMb.toFixed(1)} MB`;
    } else if (perfMem && perfMem.usedJSHeapSize > 0) {
      const usedHeapMb = perfMem.usedJSHeapSize / (1024 * 1024);
      const totalHeapMb = perfMem.totalJSHeapSize / (1024 * 1024);
      const totalAppMb = Math.max(totalHeapMb, usedHeapMb + gpuSceneBufferMb);
      ramStr = `${totalAppMb.toFixed(1)} MB (Heap: ${usedHeapMb.toFixed(1)} MB)`;
      ramTooltip = `Gesamt (inkl. WebGL/Canvas): ${totalAppMb.toFixed(1)} MB | JS Heap belegt: ${usedHeapMb.toFixed(1)} MB | JS Heap reserviert: ${totalHeapMb.toFixed(1)} MB`;
    } else {
      // Browser without performance.memory (Firefox / Safari / WebKit)
      const baseRuntimeMb = 42.0;
      const estimatedTotalMb = (baseRuntimeMb + gpuSceneBufferMb).toFixed(1);
      ramStr = `~${estimatedTotalMb} MB (App + WebGL)`;
    }

    const fpsColor = this.fps >= 55 ? "#10b981" : this.fps >= 30 ? "#f59e0b" : "#ef4444";

    if (this.fpsValEl) {
      this.fpsValEl.innerHTML = `${this.fps} FPS <span style="font-size: 10px; color: #94a3b8;">(${this.frameTimeMs} ms)</span>`;
      this.fpsValEl.style.color = fpsColor;
    }

    if (this.cpuValEl) {
      this.cpuValEl.textContent = `${this.frameTimeMs} ms`;
    }

    if (this.ramValEl) {
      this.ramValEl.textContent = ramStr;
      if (ramTooltip) {
        this.ramValEl.title = ramTooltip;
      }
    }

    if (this.tracksValEl) {
      this.tracksValEl.textContent = `${pieceCount} Gleise, ${modelCount} 3D`;
    }

    if (this.gpuValEl && this.gpuInfo.renderer) {
      this.gpuValEl.textContent = this.gpuInfo.renderer;
      this.gpuValEl.title = this.gpuInfo.renderer;
    }
  }

  destroy(): void {
    this.hide();
    if (typeof window !== "undefined") {
      window.removeEventListener("pointermove", this.onDragMove);
      window.removeEventListener("pointerup", this.onDragEnd);
      window.removeEventListener("pointercancel", this.onDragEnd);
    }
    this.container.remove();
  }
}
