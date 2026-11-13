export interface LayoutResizerOptions {
  appEl?: HTMLElement | null;
  toolbarEl?: HTMLElement | null;
  sidebarEl?: HTMLElement | null;
  rightSidebarEl?: HTMLElement | null;
}

export const DEFAULT_SIDEBAR_WIDTH = 280;
export const DEFAULT_TOOLBAR_HEIGHT = 68;
export const DEFAULT_TOOLBAR_HEIGHT_MAC = 42;
export const DEFAULT_RIGHT_SIDEBAR_WIDTH = 290;

export const MIN_SIDEBAR_WIDTH = 180;
export const MAX_SIDEBAR_WIDTH = 700;

export const MIN_TOOLBAR_HEIGHT = 34;
export const MAX_TOOLBAR_HEIGHT = 110;

export const MIN_RIGHT_SIDEBAR_WIDTH = 200;
export const MAX_RIGHT_SIDEBAR_WIDTH = 600;

export const MIN_RIGHT_SIDEBAR_HEIGHT = 160;

export class LayoutResizer {
  private appEl: HTMLElement | null = null;
  private toolbarEl: HTMLElement | null = null;
  private sidebarEl: HTMLElement | null = null;
  private rightSidebarEl: HTMLElement | null = null;

  private sidebarResizer: HTMLElement | null = null;
  private toolbarResizer: HTMLElement | null = null;
  private rightResizerLeft: HTMLElement | null = null;
  private rightResizerBottom: HTMLElement | null = null;
  private rightResizerCorner: HTMLElement | null = null;

  private locked = true;

  private activeDrag: string | null = null;
  private dragStartX = 0;
  private dragStartY = 0;
  private dragStartValW = 0;
  private dragStartValH = 0;

  constructor(options?: LayoutResizerOptions) {
    if (typeof document === "undefined") return;

    this.appEl = options?.appEl ?? document.getElementById("app");
    this.toolbarEl = options?.toolbarEl ?? document.getElementById("toolbar");
    this.sidebarEl = options?.sidebarEl ?? document.getElementById("sidebar");
    this.rightSidebarEl = options?.rightSidebarEl ?? document.getElementById("right-sidebar");

    this.restoreSavedSizes();
    this.attachResizers();

    if (typeof window !== "undefined") {
      window.addEventListener("resize", this.onWindowResize);
    }
  }

  private restoreSavedSizes(): void {
    if (typeof localStorage === "undefined") return;

    const savedSidebarWidth = parseFloat(localStorage.getItem("railbench_sidebar_width") ?? "");
    if (!isNaN(savedSidebarWidth) && savedSidebarWidth >= MIN_SIDEBAR_WIDTH) {
      this.setSidebarWidth(savedSidebarWidth, false);
    }

    const isMac = typeof document !== "undefined" && (
      document.body?.classList?.contains("has-system-menus") ||
      this.appEl?.classList?.contains("has-system-menus")
    );
    const savedToolbarHeight = parseFloat(localStorage.getItem("railbench_toolbar_height") ?? "");
    if (!isMac && savedToolbarHeight === 42) {
      this.setToolbarHeight(DEFAULT_TOOLBAR_HEIGHT, false);
    } else if (!isNaN(savedToolbarHeight) && savedToolbarHeight >= MIN_TOOLBAR_HEIGHT) {
      this.setToolbarHeight(savedToolbarHeight, false);
    }

    const savedRightWidth = parseFloat(localStorage.getItem("railbench_right_sidebar_width") ?? "");
    if (!isNaN(savedRightWidth) && savedRightWidth >= MIN_RIGHT_SIDEBAR_WIDTH) {
      this.setRightSidebarWidth(savedRightWidth, false);
    }

    const savedRightHeight = parseFloat(localStorage.getItem("railbench_right_sidebar_height") ?? "");
    if (!isNaN(savedRightHeight) && savedRightHeight >= MIN_RIGHT_SIDEBAR_HEIGHT) {
      this.setRightSidebarHeight(savedRightHeight, false);
    }

    const savedLocked = localStorage.getItem("railbench_bars_locked");
    // Normal locked by default (if null or true)
    this.locked = savedLocked === null ? true : savedLocked === "true";
  }

  private attachResizers(): void {
    if (!this.appEl) return;

    // 1. Sidebar vertical resizer (attached to #app so it cannot be wiped by sidebar.render())
    if (this.appEl && !this.appEl.querySelector("#sidebar-resizer")) {
      const resizer = document.createElement("div");
      resizer.id = "sidebar-resizer";
      resizer.className = "ui-resizer-v";
      resizer.title = "Breite anpassen (Doppelklick zum Zurücksetzen) / Drag to resize (Double-click to reset)";
      resizer.addEventListener("pointerdown", (e) => this.onSidebarPointerDown(e, resizer));
      resizer.addEventListener("dblclick", () => this.resetSidebarWidth());
      this.appEl.appendChild(resizer);
      this.sidebarResizer = resizer;
    }

    // 2. Toolbar horizontal resizer (attached to #app so it cannot be wiped by toolbar rendering)
    if (this.appEl && !this.appEl.querySelector("#toolbar-resizer")) {
      const resizer = document.createElement("div");
      resizer.id = "toolbar-resizer";
      resizer.className = "ui-resizer-h";
      resizer.title = "Höhe anpassen (Doppelklick zum Zurücksetzen) / Drag to resize (Double-click to reset)";
      resizer.addEventListener("pointerdown", (e) => this.onToolbarPointerDown(e, resizer));
      resizer.addEventListener("dblclick", () => this.resetToolbarHeight());
      this.appEl.appendChild(resizer);
      this.toolbarResizer = resizer;
    }

    // 3. Right Sidebar resizers
    if (this.rightSidebarEl) {
      if (!this.rightSidebarEl.querySelector("#right-sidebar-resizer-left")) {
        const leftHandle = document.createElement("div");
        leftHandle.id = "right-sidebar-resizer-left";
        leftHandle.className = "ui-resizer-right-left";
        leftHandle.title = "Breite anpassen (Doppelklick zum Zurücksetzen) / Drag to resize (Double-click to reset)";
        leftHandle.addEventListener("pointerdown", (e) => this.onRightSidebarPointerDown(e, leftHandle, "left"));
        leftHandle.addEventListener("dblclick", () => this.resetRightSidebarWidth());
        this.rightSidebarEl.appendChild(leftHandle);
        this.rightResizerLeft = leftHandle;
      }

      if (!this.rightSidebarEl.querySelector("#right-sidebar-resizer-bottom")) {
        const bottomHandle = document.createElement("div");
        bottomHandle.id = "right-sidebar-resizer-bottom";
        bottomHandle.className = "ui-resizer-right-bottom";
        bottomHandle.title = "Höhe anpassen (Doppelklick zum Zurücksetzen) / Drag to resize (Double-click to reset)";
        bottomHandle.addEventListener("pointerdown", (e) => this.onRightSidebarPointerDown(e, bottomHandle, "bottom"));
        bottomHandle.addEventListener("dblclick", () => this.resetRightSidebarHeight());
        this.rightSidebarEl.appendChild(bottomHandle);
        this.rightResizerBottom = bottomHandle;
      }

      if (!this.rightSidebarEl.querySelector("#right-sidebar-resizer-corner")) {
        const cornerHandle = document.createElement("div");
        cornerHandle.id = "right-sidebar-resizer-corner";
        cornerHandle.className = "ui-resizer-right-corner";
        cornerHandle.title = "Größe anpassen (Doppelklick zum Zurücksetzen) / Drag to resize (Double-click to reset)";
        cornerHandle.addEventListener("pointerdown", (e) => this.onRightSidebarPointerDown(e, cornerHandle, "corner"));
        cornerHandle.addEventListener("dblclick", () => this.resetRightSidebarAll());
        this.rightSidebarEl.appendChild(cornerHandle);
        this.rightResizerCorner = cornerHandle;
      }
    }

    this.applyLockedState();
  }

  isLocked(): boolean {
    return this.locked;
  }

  setLocked(locked: boolean): void {
    this.locked = locked;
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_bars_locked", String(locked));
      } catch {}
    }
    this.applyLockedState();
  }

  toggleLocked(): boolean {
    this.setLocked(!this.locked);
    return this.locked;
  }

  private applyLockedState(): void {
    if (this.appEl) {
      this.appEl.classList.toggle("bars-locked", this.locked);
    }
    const displayVal = this.locked ? "none" : "";
    if (this.sidebarResizer) this.sidebarResizer.style.display = displayVal;
    if (this.toolbarResizer) this.toolbarResizer.style.display = displayVal;
    if (this.rightResizerLeft) this.rightResizerLeft.style.display = displayVal;
    if (this.rightResizerBottom) this.rightResizerBottom.style.display = displayVal;
    if (this.rightResizerCorner) this.rightResizerCorner.style.display = displayVal;
  }

  // --- Handlers: Left Sidebar ---
  private onSidebarPointerDown(e: PointerEvent, handle: HTMLElement): void {
    if (this.locked || e.button !== 0) return;
    e.preventDefault();

    this.activeDrag = "sidebar";
    this.dragStartX = e.clientX;
    const currentW = this.getSidebarWidth();
    this.dragStartValW = currentW;

    handle.classList.add("dragging");
    if (document.body) {
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    }

    try {
      handle.setPointerCapture(e.pointerId);
    } catch {}

    window.addEventListener("pointermove", this.onGlobalPointerMove);
    window.addEventListener("pointerup", this.onGlobalPointerUp);
    window.addEventListener("pointercancel", this.onGlobalPointerUp);
  }

  // --- Handlers: Top Toolbar ---
  private onToolbarPointerDown(e: PointerEvent, handle: HTMLElement): void {
    if (this.locked || e.button !== 0) return;
    e.preventDefault();

    this.activeDrag = "toolbar";
    this.dragStartY = e.clientY;
    const currentH = this.getToolbarHeight();
    this.dragStartValH = currentH;

    handle.classList.add("dragging");
    if (document.body) {
      document.body.style.cursor = "row-resize";
      document.body.style.userSelect = "none";
    }

    try {
      handle.setPointerCapture(e.pointerId);
    } catch {}

    window.addEventListener("pointermove", this.onGlobalPointerMove);
    window.addEventListener("pointerup", this.onGlobalPointerUp);
    window.addEventListener("pointercancel", this.onGlobalPointerUp);
  }

  // --- Handlers: Right Sidebar ---
  private onRightSidebarPointerDown(e: PointerEvent, handle: HTMLElement, mode: "left" | "bottom" | "corner"): void {
    if (this.locked || e.button !== 0) return;
    e.preventDefault();

    this.activeDrag = `right-${mode}`;
    this.dragStartX = e.clientX;
    this.dragStartY = e.clientY;
    this.dragStartValW = this.getRightSidebarWidth();
    this.dragStartValH = this.getRightSidebarHeight();

    handle.classList.add("dragging");
    if (document.body) {
      document.body.style.cursor = mode === "left" ? "ew-resize" : mode === "bottom" ? "ns-resize" : "nesw-resize";
      document.body.style.userSelect = "none";
    }

    try {
      handle.setPointerCapture(e.pointerId);
    } catch {}

    window.addEventListener("pointermove", this.onGlobalPointerMove);
    window.addEventListener("pointerup", this.onGlobalPointerUp);
    window.addEventListener("pointercancel", this.onGlobalPointerUp);
  }

  private onGlobalPointerMove = (e: PointerEvent): void => {
    if (!this.activeDrag) return;

    if (this.activeDrag === "sidebar") {
      const dx = e.clientX - this.dragStartX;
      const targetW = this.dragStartValW + dx;
      this.setSidebarWidth(targetW, true);
    } else if (this.activeDrag === "toolbar") {
      const dy = e.clientY - this.dragStartY;
      const targetH = this.dragStartValH + dy;
      this.setToolbarHeight(targetH, true);
    } else if (this.activeDrag === "right-left") {
      const dx = e.clientX - this.dragStartX;
      // Moving left increases width
      const targetW = this.dragStartValW - dx;
      this.setRightSidebarWidth(targetW, true);
    } else if (this.activeDrag === "right-bottom") {
      const dy = e.clientY - this.dragStartY;
      const targetH = this.dragStartValH + dy;
      this.setRightSidebarHeight(targetH, true);
    } else if (this.activeDrag === "right-corner") {
      const dx = e.clientX - this.dragStartX;
      const dy = e.clientY - this.dragStartY;
      this.setRightSidebarWidth(this.dragStartValW - dx, true);
      this.setRightSidebarHeight(this.dragStartValH + dy, true);
    }
  };

  private onGlobalPointerUp = (e?: PointerEvent): void => {
    if (!this.activeDrag) return;

    const drag = this.activeDrag;
    this.activeDrag = null;

    if (document.body) {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }

    this.sidebarResizer?.classList.remove("dragging");
    this.toolbarResizer?.classList.remove("dragging");
    this.rightResizerLeft?.classList.remove("dragging");
    this.rightResizerBottom?.classList.remove("dragging");
    this.rightResizerCorner?.classList.remove("dragging");

    if (e) {
      try {
        (e.target as HTMLElement)?.releasePointerCapture?.(e.pointerId);
      } catch {}
    }

    window.removeEventListener("pointermove", this.onGlobalPointerMove);
    window.removeEventListener("pointerup", this.onGlobalPointerUp);
    window.removeEventListener("pointercancel", this.onGlobalPointerUp);

    // Save final values to localStorage
    if (typeof localStorage !== "undefined") {
      try {
        if (drag === "sidebar") {
          localStorage.setItem("railbench_sidebar_width", String(this.getSidebarWidth()));
        } else if (drag === "toolbar") {
          localStorage.setItem("railbench_toolbar_height", String(this.getToolbarHeight()));
        } else if (drag.startsWith("right")) {
          localStorage.setItem("railbench_right_sidebar_width", String(this.getRightSidebarWidth()));
          const h = this.getRightSidebarHeight();
          if (h) {
            localStorage.setItem("railbench_right_sidebar_height", String(h));
          }
        }
      } catch {}
    }

    this.notifyResize();
  };

  private onWindowResize = (): void => {
    // Clamp if exceeding bounds
    this.clampAllSizes();
  };

  private clampAllSizes(): void {
    if (typeof window === "undefined") return;

    const maxSidebar = Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, window.innerWidth - 300));
    const currentSidebar = this.getSidebarWidth();
    if (currentSidebar > maxSidebar) {
      this.setSidebarWidth(maxSidebar, false);
    }

    const maxRight = Math.min(MAX_RIGHT_SIDEBAR_WIDTH, Math.max(MIN_RIGHT_SIDEBAR_WIDTH, window.innerWidth - 320));
    const currentRight = this.getRightSidebarWidth();
    if (currentRight > maxRight) {
      this.setRightSidebarWidth(maxRight, false);
    }
  }

  // --- Dimension getters and setters ---

  getSidebarWidth(): number {
    if (this.appEl) {
      const customProp = parseFloat(getComputedStyle(this.appEl).getPropertyValue("--sidebar-width"));
      if (!isNaN(customProp) && customProp > 0) return customProp;
    }
    return this.sidebarEl?.offsetWidth || DEFAULT_SIDEBAR_WIDTH;
  }

  setSidebarWidth(width: number, saveToStorage = false): void {
    const max = typeof window !== "undefined" ? Math.min(MAX_SIDEBAR_WIDTH, window.innerWidth - 300) : MAX_SIDEBAR_WIDTH;
    const clamped = Math.round(Math.max(MIN_SIDEBAR_WIDTH, Math.min(width, max)));

    if (this.appEl) {
      this.appEl.style.setProperty("--sidebar-width", `${clamped}px`);
    }
    if (saveToStorage && typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_sidebar_width", String(clamped));
      } catch {}
    }
    this.notifyResize();
  }

  resetSidebarWidth(): void {
    if (this.appEl) {
      this.appEl.style.setProperty("--sidebar-width", `${DEFAULT_SIDEBAR_WIDTH}px`);
    }
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem("railbench_sidebar_width");
      } catch {}
    }
    this.notifyResize();
  }

  getToolbarHeight(): number {
    if (this.appEl) {
      const customProp = parseFloat(getComputedStyle(this.appEl).getPropertyValue("--toolbar-height"));
      if (!isNaN(customProp) && customProp > 0) return customProp;
    }
    return this.toolbarEl?.offsetHeight || DEFAULT_TOOLBAR_HEIGHT;
  }

  setToolbarHeight(height: number, saveToStorage = false): void {
    const clamped = Math.round(Math.max(MIN_TOOLBAR_HEIGHT, Math.min(height, MAX_TOOLBAR_HEIGHT)));

    if (this.appEl) {
      this.appEl.style.setProperty("--toolbar-height", `${clamped}px`);
    }
    if (saveToStorage && typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_toolbar_height", String(clamped));
      } catch {}
    }
    this.notifyResize();
  }

  resetToolbarHeight(): void {
    const isMac = typeof document !== "undefined" && (
      document.body?.classList?.contains("has-system-menus") ||
      this.appEl?.classList?.contains("has-system-menus")
    );
    const defaultH = isMac ? DEFAULT_TOOLBAR_HEIGHT_MAC : DEFAULT_TOOLBAR_HEIGHT;
    if (this.appEl) {
      this.appEl.style.setProperty("--toolbar-height", `${defaultH}px`);
    }
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem("railbench_toolbar_height");
      } catch {}
    }
    this.notifyResize();
  }

  getRightSidebarWidth(): number {
    if (this.rightSidebarEl) {
      const explicit = parseFloat(this.rightSidebarEl.style.width);
      if (!isNaN(explicit) && explicit > 0) return explicit;
      const customProp = parseFloat(getComputedStyle(this.rightSidebarEl).getPropertyValue("--right-sidebar-width"));
      if (!isNaN(customProp) && customProp > 0) return customProp;
      if (this.rightSidebarEl.offsetWidth > 0) return this.rightSidebarEl.offsetWidth;
    }
    return DEFAULT_RIGHT_SIDEBAR_WIDTH;
  }

  setRightSidebarWidth(width: number, saveToStorage = false): void {
    const max = typeof window !== "undefined" ? Math.min(MAX_RIGHT_SIDEBAR_WIDTH, window.innerWidth - 320) : MAX_RIGHT_SIDEBAR_WIDTH;
    const clamped = Math.round(Math.max(MIN_RIGHT_SIDEBAR_WIDTH, Math.min(width, max)));

    if (this.rightSidebarEl) {
      this.rightSidebarEl.style.width = `${clamped}px`;
      this.rightSidebarEl.style.setProperty("--right-sidebar-width", `${clamped}px`);
    }
    if (saveToStorage && typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_right_sidebar_width", String(clamped));
      } catch {}
    }
  }

  resetRightSidebarWidth(): void {
    if (this.rightSidebarEl) {
      this.rightSidebarEl.style.width = `${DEFAULT_RIGHT_SIDEBAR_WIDTH}px`;
      this.rightSidebarEl.style.setProperty("--right-sidebar-width", `${DEFAULT_RIGHT_SIDEBAR_WIDTH}px`);
    }
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem("railbench_right_sidebar_width");
      } catch {}
    }
  }

  getRightSidebarHeight(): number {
    if (this.rightSidebarEl) {
      const explicit = parseFloat(this.rightSidebarEl.style.height);
      if (!isNaN(explicit) && explicit > 0) return explicit;
      const customProp = parseFloat(getComputedStyle(this.rightSidebarEl).getPropertyValue("--right-sidebar-height"));
      if (!isNaN(customProp) && customProp > 0) return customProp;
      if (this.rightSidebarEl.offsetHeight > 0) return this.rightSidebarEl.offsetHeight;
      const rectH = this.rightSidebarEl.getBoundingClientRect?.().height ?? 0;
      if (rectH > 0) return rectH;
    }
    return MIN_RIGHT_SIDEBAR_HEIGHT;
  }

  setRightSidebarHeight(height: number, saveToStorage = false): void {
    const toolbarH = this.getToolbarHeight();
    const maxAllowed = typeof window !== "undefined" ? Math.max(MIN_RIGHT_SIDEBAR_HEIGHT, window.innerHeight - toolbarH - 24) : 900;
    const clamped = Math.round(Math.max(MIN_RIGHT_SIDEBAR_HEIGHT, Math.min(height, maxAllowed)));

    if (this.rightSidebarEl) {
      this.rightSidebarEl.style.height = `${clamped}px`;
      this.rightSidebarEl.style.maxHeight = `${clamped}px`;
      this.rightSidebarEl.style.setProperty("--right-sidebar-height", `${clamped}px`);
    }
    if (saveToStorage && typeof localStorage !== "undefined") {
      try {
        localStorage.setItem("railbench_right_sidebar_height", String(clamped));
      } catch {}
    }
  }

  resetRightSidebarHeight(): void {
    if (this.rightSidebarEl) {
      this.rightSidebarEl.style.height = "";
      this.rightSidebarEl.style.maxHeight = "";
      this.rightSidebarEl.style.setProperty("--right-sidebar-height", "");
    }
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.removeItem("railbench_right_sidebar_height");
      } catch {}
    }
  }

  resetRightSidebarAll(): void {
    this.resetRightSidebarWidth();
    this.resetRightSidebarHeight();
  }

  resetAll(): void {
    this.resetSidebarWidth();
    this.resetToolbarHeight();
    this.resetRightSidebarAll();
  }

  private notifyResize(): void {
    if (typeof window !== "undefined") {
      window.dispatchEvent(new Event("resize"));
    }
  }

  destroy(): void {
    if (typeof window !== "undefined") {
      window.removeEventListener("resize", this.onWindowResize);
      window.removeEventListener("pointermove", this.onGlobalPointerMove);
      window.removeEventListener("pointerup", this.onGlobalPointerUp);
      window.removeEventListener("pointercancel", this.onGlobalPointerUp);
    }
    this.sidebarResizer?.remove();
    this.toolbarResizer?.remove();
    this.rightResizerLeft?.remove();
    this.rightResizerBottom?.remove();
    this.rightResizerCorner?.remove();
  }
}

let activeInstance: LayoutResizer | null = null;

export function initLayoutResizers(options?: LayoutResizerOptions): LayoutResizer {
  if (activeInstance) {
    activeInstance.destroy();
  }
  activeInstance = new LayoutResizer(options);
  return activeInstance;
}

export function getLayoutResizer(): LayoutResizer | null {
  return activeInstance;
}
