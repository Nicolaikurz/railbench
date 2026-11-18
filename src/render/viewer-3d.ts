import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import type { Layout, TrackPiece, Layer, Model3D, ColorMode, LayerWoodPlateMode, Vec2 } from "@/core/types";
import { getBaseboardBounds, getBaseboards } from "@/core/layout";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import { parseModelScale, type ModelScale } from "@/core/model-scale";
import type { ShortCircuitDetectionResult } from "@/core/simulation";
import { getTranslations, getLanguage, onLanguageChange } from "@/i18n";
import { getEffectiveFlexEntry } from "@/core/flextrack";
import { buildRealisticLandscape } from "./viewer-3d-landscape";
import { buildTrainHall } from "./viewer-3d-trainhall";
import { buildBaseboard3D, buildCustomLayerPlate3D, buildLayerDeckPlate3D } from "./viewer-3d-baseboard";
import { buildAllWoodPlates3D, type WoodPlateBuildItem } from "./viewer-3d-roads";
import { buildAllModels3D, buildBuildingWoodPlate3D } from "./viewer-3d-models";
import { buildAllTracks3D, freezeStaticTransforms, type TrackBuildItem } from "./viewer-3d-track";
import { buildCatenary3D, buildTurntableCatenary3D, buildTransferTableCatenary3D } from "./viewer-3d-catenary";
import { buildTrackSignal3D, isPillarColliding, buildTrackBridge3D } from "./viewer-3d-bridges";
import {
  SKY_KEYFRAMES,
  applyCameraViewPreset,
  buildCelestialBodies,
  createViewCubeWidget,
  orbitCameraByDegrees,
  updateViewCubeOrientation,
  type CameraViewPreset,
} from "./viewer-3d-viewcube";

export type { CameraViewPreset };
export type Viewer3DEnvironment = "landscape" | "hall" | "minimal";

export class Viewer3D {
  private container: HTMLElement;
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private renderer: THREE.WebGLRenderer;
  private controls: OrbitControls;
  private animId: number | null = null;
  private trackGroup = new THREE.Group();
  private tableGroup = new THREE.Group();
  private woodPlateGroup = new THREE.Group();
  private modelsGroup = new THREE.Group();
  private trainHallGroup = new THREE.Group();
  private landscapeGroup = new THREE.Group();
  private shortCircuitGroup = new THREE.Group();
  private currentLayout?: Layout;
  private lastCatalog?: Catalog;
  private lastOptions?: { showColorCode?: boolean; colorMode?: ColorMode; shortCircuit?: ShortCircuitDetectionResult };
  private currentShortCircuit?: ShortCircuitDetectionResult;
  private envMode: Viewer3DEnvironment = "hall";
  private showWoodPlate = true;
  private gltfLoader = new GLTFLoader();
  private gltfCache = new Map<string, THREE.Group>();
  /** Maßstab des aktuellen Layouts (Fallback für eingebaute Modelle ohne bekannte Basismaße). */
  private layoutScale: ModelScale = "H0";
  private isVisible = false;
  private currentTheme: "light" | "dark" = "light";
  private sunLight: THREE.DirectionalLight | null = null;
  private fillLight: THREE.DirectionalLight | null = null;
  private hemiLight: THREE.HemisphereLight | null = null;
  private overlayEl: HTMLElement | null = null;

  // Tag/Nacht-Zyklus & wandernde Sonne (standardmäßig pausiert auf 14:00 Uhr für 0% Idle-GPU-Last)
  private dayNightActive = false;
  private timeOfDay = 14.0; // 14:00 Uhr
  private cycleSpeed = 1.0;
  private readonly cycleDayDurationSec = 1200; // 20 Minuten für einen vollen 24h-Tag (Bereich 15 - 30 Min.)
  private lastFrameTime = performance.now();
  private lastDayNightRenderTime = 0;
  private lastLayoutSignature = "";
  private sceneCenter = new THREE.Vector3(0, 0, 0);
  private celestialGroup = new THREE.Group();
  private sunMesh: THREE.Mesh | null = null;
  private sunCorona: THREE.Mesh | null = null;
  private moonMesh: THREE.Mesh | null = null;
  private moonGlow: THREE.Mesh | null = null;
  private nightLampMats: THREE.MeshStandardMaterial[] = [];
  private timeSliderEl: HTMLInputElement | null = null;
  private cycleBtnEl: HTMLButtonElement | null = null;
  private cycleLabelEl: HTMLElement | null = null;
  private speedBtnEl: HTMLButtonElement | null = null;
  private viewCubeEl: HTMLElement | null = null;
  private refreshViewCubeLabels: (() => void) | null = null;
  private lastTimeStr = "";
  private builtHallKey = "";
  private builtLandscapeKey = "";
  private needsRender = true;
  private lastShadowUpdateTime = 0;
  private resizeHandler: (() => void) | null = null;
  private readonly _c1 = new THREE.Color();
  private readonly _curSky = new THREE.Color();
  private readonly _curFog = new THREE.Color();
  private readonly _curSun = new THREE.Color();
  private readonly _curHemiSky = new THREE.Color();
  private readonly _curHemiGnd = new THREE.Color();

  constructor(host: HTMLElement) {
    this.container = document.createElement("div");
    this.container.className = "viewer-3d-container";
    this.container.style.position = "absolute";
    this.container.style.top = "0";
    this.container.style.left = "0";
    this.container.style.width = "100%";
    this.container.style.height = "100%";
    this.container.style.display = "none";
    this.container.style.zIndex = "20";
    host.appendChild(this.container);

    const isDark = document.documentElement.getAttribute("data-theme") === "dark";
    this.currentTheme = isDark ? "dark" : "light";

    // 3D-Umgebungsmodus ("hall" = Lokhalle mit Wald außen, "landscape" = Landschaft/Echte Welt, "minimal" = Nur Tisch)
    const savedEnv = typeof localStorage !== "undefined" ? (localStorage.getItem("railbench_3d_env") as Viewer3DEnvironment | null) : null;
    if (savedEnv === "landscape" || savedEnv === "hall" || savedEnv === "minimal") {
      this.envMode = savedEnv;
    } else {
      this.envMode = "hall";
    }

    this.trainHallGroup.visible = this.envMode === "hall";
    this.landscapeGroup.visible = this.envMode === "landscape";

    this.showWoodPlate = true;
    this.woodPlateGroup.visible = this.showWoodPlate;

    // 1. Scene
    this.scene = new THREE.Scene();
    this.updateSceneBackground();

    // 2. Camera
    const width = host.clientWidth || 800;
    const height = host.clientHeight || 600;
    this.camera = new THREE.PerspectiveCamera(45, width / height, 8, 110000);
    this.camera.position.set(0, 1600, 2200);

    // 3. WebGL Renderer (bevorzugt dedizierte GPU für maximale 3D-Beschleunigung)
    try {
      this.renderer = new THREE.WebGLRenderer({
        antialias: true,
        alpha: false,
        stencil: false,
        powerPreference: "high-performance",
      });
      this.renderer.setSize(width, height);
      // Begrenzung auf max. 1.25 DPR spart auf Retina/4K-Displays ~300-500 MB Framebuffer-VRAM bei gestochen scharfem MSAA
      this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.25));
      this.renderer.shadowMap.enabled = true;
      this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
      this.renderer.shadowMap.autoUpdate = false;
      this.renderer.shadowMap.needsUpdate = true;
      this.container.appendChild(this.renderer.domElement);

      // 4. Orbit Controls (Volle 360°/180° Perspektivenfreiheit inkl. Ansicht von unten)
      this.controls = new OrbitControls(this.camera, this.renderer.domElement);
      this.controls.enableDamping = true;
      this.controls.dampingFactor = 0.08;
      this.controls.minPolarAngle = 0;
      this.controls.maxPolarAngle = Math.PI;
      this.controls.minAzimuthAngle = -Infinity;
      this.controls.maxAzimuthAngle = Infinity;
      this.controls.minDistance = 30;
      this.controls.maxDistance = 65000;
      this.controls.target.set(0, 0, 0);
      this.controls.addEventListener("change", () => {
        this.needsRender = true;
      });
    } catch (err) {
      console.warn("[Viewer3D] WebGL context initialization failed:", err);
      this.renderer = null as any;
      this.controls = null as any;
    }

    // 5. Lighting
    this.setupLighting();

    // 6. Scene Groups
    this.scene.add(this.celestialGroup);
    this.scene.add(this.landscapeGroup);
    this.scene.add(this.trainHallGroup);
    this.scene.add(this.tableGroup);
    this.scene.add(this.woodPlateGroup);
    this.scene.add(this.trackGroup);
    this.scene.add(this.modelsGroup);
    this.scene.add(this.shortCircuitGroup);

    // Himmelskörper initialisieren
    this.setupCelestialBodies();
    this.applyTimeOfDay();

    // 7. Overlay Controls
    this.setupOverlayUI();

    // Resize listener
    if (typeof window !== "undefined") {
      this.resizeHandler = () => this.onResize();
      window.addEventListener("resize", this.resizeHandler);
    }
  }

  private getIsDark(): boolean {
    if (typeof document !== "undefined" && document.documentElement) {
      const attr = document.documentElement.getAttribute("data-theme");
      if (attr === "dark") return true;
      if (attr === "light") return false;
    }
    return this.currentTheme === "dark";
  }

  private updateSceneBackground() {
    if (this.envMode === "minimal") {
      const isDark = this.getIsDark();
      const cadColor = isDark ? 0x131b2e : 0xe4dfd5;
      this.scene.background = new THREE.Color(cadColor);
      this.scene.fog = new THREE.FogExp2(cadColor, 0.00002);
      this.celestialGroup.visible = false;
    } else {
      this.celestialGroup.visible = true;
      this.applyTimeOfDay();
    }
  }

  requestRender() {
    this.needsRender = true;
  }

  requestShadowUpdate() {
    if (this.renderer) {
      this.renderer.shadowMap.needsUpdate = true;
    }
    this.needsRender = true;
  }

  setTheme(theme: "light" | "dark") {
    this.currentTheme = theme;
    this.updateSceneBackground();
    this.updateLighting();
    this.updateOverlayUIActiveStates();
    this.requestShadowUpdate();
  }

  private setupLighting() {
    this.hemiLight = new THREE.HemisphereLight(0xffffff, 0x444444, 0.75);
    this.hemiLight.position.set(0, 3000, 0);
    this.scene.add(this.hemiLight);

    this.sunLight = new THREE.DirectionalLight(0xfff8ed, 1.4);
    this.sunLight.position.set(1600, 3200, 1400);
    this.sunLight.castShadow = true;
    this.sunLight.shadow.mapSize.width = 1024;
    this.sunLight.shadow.mapSize.height = 1024;
    this.sunLight.shadow.camera.near = 100;
    this.sunLight.shadow.camera.far = 18000;
    const d = 3200;
    this.sunLight.shadow.camera.left = -d;
    this.sunLight.shadow.camera.right = d;
    this.sunLight.shadow.camera.top = d;
    this.sunLight.shadow.camera.bottom = -d;
    this.sunLight.shadow.bias = 0.00005;
    this.sunLight.shadow.normalBias = 0.04;
    this.scene.add(this.sunLight);
    this.scene.add(this.sunLight.target);

    this.fillLight = new THREE.DirectionalLight(0x93c5fd, 0.45);
    this.fillLight.position.set(-1800, 1800, -1800);
    this.scene.add(this.fillLight);

    this.updateLighting();
  }

  private updateLighting() {
    if (!this.sunLight || !this.hemiLight || !this.fillLight) return;
    if (this.envMode === "minimal") {
      const isDark = this.getIsDark();
      this.sunLight.color.setHex(0xffffff);
      this.sunLight.intensity = isDark ? 0.9 : 1.25;
      this.hemiLight.color.setHex(0xffffff);
      this.hemiLight.groundColor.setHex(isDark ? 0x1e293b : 0xc8c2b7);
      this.hemiLight.intensity = isDark ? 0.5 : 0.8;
      this.fillLight.color.setHex(isDark ? 0x475569 : 0xd1ccc0);
      this.fillLight.intensity = isDark ? 0.3 : 0.35;
      this.sunLight.position.set(1600, 3200, 1400);
    } else {
      this.applyTimeOfDay();
    }
  }

  cycleEnvironment(): Viewer3DEnvironment {
    const modes: Viewer3DEnvironment[] = ["hall", "minimal"];
    const curIdx = modes.indexOf(this.envMode);
    const nextMode = modes[(curIdx + 1) % modes.length]!;
    this.setEnvironment(nextMode);
    return nextMode;
  }

  setEnvironment(mode: Viewer3DEnvironment) {
    this.envMode = mode;
    // Inaktive Umgebungen sofort aus dem RAM freigeben
    if (mode === "hall") {
      this.clearGroup(this.landscapeGroup);
      this.builtLandscapeKey = "";
    } else if (mode === "landscape") {
      this.clearGroup(this.trainHallGroup);
      this.builtHallKey = "";
      this.nightLampMats = [];
    } else if (mode === "minimal") {
      // Im Minimalmodus Tag/Nacht-Zyklus stoppen und Lokhalle komplett entladen
      this.setDayNightActive(false);
      this.clearGroup(this.trainHallGroup);
      this.clearGroup(this.landscapeGroup);
      this.nightLampMats = [];
      this.builtHallKey = "";
      this.builtLandscapeKey = "";
      if (this.currentLayout) {
        const { cx, cz } = this.computeTableCenter(this.currentLayout);
        this.sceneCenter.set(cx, 0, cz);
        if (this.controls) {
          this.controls.target.set(cx, 0, cz);
        }
      } else {
        this.sceneCenter.set(0, 0, 0);
        if (this.controls) {
          this.controls.target.set(0, 0, 0);
        }
      }
    }
    this.ensureActiveEnvironmentBuilt();
    this.landscapeGroup.visible = mode === "landscape";
    this.trainHallGroup.visible = mode === "hall";
    this.celestialGroup.visible = mode !== "minimal";
    this.updateSceneBackground();
    this.updateLighting();

    try {
      localStorage.setItem("railbench_3d_env", mode);
      localStorage.setItem("railbench_3d_trainhall", mode === "hall" ? "1" : "0");
    } catch {}

    this.updateOverlayUIActiveStates();
    this.requestShadowUpdate();
  }

  private updateOverlayUIActiveStates() {
    if (!this.overlayEl) return;
    const isDe = getLanguage() === "de";
    const iconEl = this.overlayEl.querySelector("#v3d-env-icon");
    const labelEl = this.overlayEl.querySelector("#v3d-env-label");
    const toggleBtn = this.overlayEl.querySelector("#v3d-toggle-env") as HTMLElement | null;

    if (iconEl && labelEl) {
      if (this.envMode === "minimal") {
        iconEl.textContent = "📐";
        labelEl.textContent = isDe ? "CAD / Minimal" : "CAD / Minimal";
      } else {
        iconEl.textContent = "🏛️";
        labelEl.textContent = isDe ? "Halle" : "Hall";
      }
    }

    if (toggleBtn) {
      const nextMode = this.envMode === "hall" 
        ? (isDe ? "CAD / Minimal" : "CAD / Minimal") 
        : (isDe ? "Halle" : "Hall");
      toggleBtn.title = isDe
        ? `3D-Hintergrund umschalten (Klicken für: ${nextMode})`
        : `Toggle 3D background (Click for: ${nextMode})`;
    }

    const dayNightBar = this.overlayEl.querySelector(".v3d-daynight-bar") as HTMLElement | null;
    if (dayNightBar) {
      dayNightBar.style.display = this.envMode === "minimal" ? "none" : "inline-flex";
      dayNightBar.title = isDe ? "Tag/Nacht-Zyklus mit wandernder Sonne" : "Day/Night cycle with moving sun";
    }
    if (this.cycleBtnEl) {
      this.cycleBtnEl.title = isDe ? "Tag/Nacht-Zyklus starten oder anhalten" : "Start or pause Day/Night cycle";
    }
    if (this.timeSliderEl) {
      this.timeSliderEl.title = isDe ? "Tageszeit einstellen (00:00 - 24:00 Uhr)" : "Set time of day (00:00 - 24:00)";
    }
    if (this.speedBtnEl) {
      this.speedBtnEl.title = isDe ? "Geschwindigkeit ändern (1x = 20 Min. pro Tag)" : "Change cycle speed (1x = 20 min per day)";
    }

    const btnHall = this.overlayEl.querySelector("#v3d-env-hall");
    const btnMinimal = this.overlayEl.querySelector("#v3d-env-minimal");
    btnHall?.classList.toggle("active", this.envMode === "hall");
    btnMinimal?.classList.toggle("active", this.envMode === "minimal");
    this.refreshViewCubeLabels?.();
  }

  setCameraViewPreset(preset: CameraViewPreset) {
    applyCameraViewPreset(this.camera, this.controls, preset, this.sceneCenter);
    updateViewCubeOrientation(this.viewCubeEl, this.camera, this.controls);
  }

  orbitCameraBy(deltaAzimuthDeg: number, deltaPolarDeg: number) {
    orbitCameraByDegrees(this.camera, this.controls, deltaAzimuthDeg, deltaPolarDeg);
    updateViewCubeOrientation(this.viewCubeEl, this.camera, this.controls);
  }

  zoomBy(factor: number) {
    if (!this.controls || !this.camera) return;
    const offset = this.camera.position.clone().sub(this.controls.target);
    offset.multiplyScalar(1 / factor);
    this.camera.position.copy(this.controls.target).add(offset);
    this.controls.update();
    updateViewCubeOrientation(this.viewCubeEl, this.camera, this.controls);
  }

  panBy(dx: number, dy: number) {
    if (!this.controls || !this.camera) return;
    const vRight = new THREE.Vector3(1, 0, 0).applyQuaternion(this.camera.quaternion);
    const vUp = new THREE.Vector3(0, 1, 0).applyQuaternion(this.camera.quaternion);
    const dist = this.camera.position.distanceTo(this.controls.target);
    const scale = Math.max(1, dist * 0.002);
    const panOffset = vRight.multiplyScalar(-dx * scale).add(vUp.multiplyScalar(dy * scale));
    this.camera.position.add(panOffset);
    this.controls.target.add(panOffset);
    this.controls.update();
    updateViewCubeOrientation(this.viewCubeEl, this.camera, this.controls);
  }

  private setupOverlayUI() {
    const overlay = document.createElement("div");
    overlay.className = "viewer-3d-overlay";  
    this.overlayEl = overlay;

    overlay.innerHTML = `
      <div class="viewer-3d-header" style="position: absolute; top: 16px; left: 16px; right: 16px; display: flex; align-items: center; justify-content: space-between; pointer-events: none; z-index: 25;">
        <div class="viewer-3d-left" style="pointer-events: auto; display: flex; align-items: center; gap: 8px;">
          <button class="v3d-btn" id="v3d-toggle-env" title="3D-Hintergrund umschalten (Halle / Minimal)">
            <span id="v3d-env-icon">🏛️</span>
            <span id="v3d-env-label">Halle</span>
          </button>

          <div class="v3d-daynight-bar" title="Tag/Nacht-Zyklus mit wandernder Sonne">
            <button class="v3d-btn ${this.dayNightActive ? "active" : ""}" id="v3d-toggle-cycle" title="Tag/Nacht-Zyklus starten oder anhalten">
              <span id="v3d-cycle-label">14:00</span>
            </button>
            <input type="range" id="v3d-time-slider" class="v3d-time-slider" min="0" max="24" step="0.05" value="${this.timeOfDay.toFixed(2)}" title="Tageszeit einstellen (00:00 - 24:00 Uhr)">
            <button class="v3d-btn" id="v3d-cycle-speed" title="Geschwindigkeit ändern (1x = 20 Min. pro Tag)">
              <span id="v3d-speed-label">${this.cycleSpeed}x</span>
            </button>
          </div>
        </div>

        <div class="viewer-3d-actions" style="pointer-events: auto; display: flex; align-items: center; gap: 8px;">
        </div>
      </div>
    `;

    this.timeSliderEl = overlay.querySelector("#v3d-time-slider");
    this.cycleBtnEl = overlay.querySelector("#v3d-toggle-cycle");
    this.cycleLabelEl = overlay.querySelector("#v3d-cycle-label");
    this.speedBtnEl = overlay.querySelector("#v3d-cycle-speed");

    const toggleEnvBtn = overlay.querySelector("#v3d-toggle-env");
    toggleEnvBtn?.addEventListener("click", () => {
      this.cycleEnvironment();
    });

    this.cycleBtnEl?.addEventListener("click", () => {
      this.setDayNightActive(!this.dayNightActive);
    });

    this.timeSliderEl?.addEventListener("input", (e) => {
      const val = parseFloat((e.target as HTMLInputElement).value);
      this.setTimeOfDay(val);
    });

    this.speedBtnEl?.addEventListener("click", () => {
      const speeds = [1.0, 1.33, 0.75, 2.0];
      const nextIdx = (speeds.indexOf(this.cycleSpeed) + 1) % speeds.length;
      this.setCycleSpeed(speeds[nextIdx]!);
    });

    const shortCircuitBtn = overlay.querySelector("#v3d-short-circuit-btn") as HTMLElement | null;
    shortCircuitBtn?.addEventListener("click", () => {
      if (this.currentShortCircuit?.shortCircuitPoints.length) {
        const pt = this.currentShortCircuit.shortCircuitPoints[0]!;
        this.focusOnPoint(pt.x, pt.y);
      }
    });

    // CAD ViewCube ("Ansichtswürfel") direkt unter dem Hintergrund-Umschalter platzieren
    const vc = createViewCubeWidget(
      (preset) => this.setCameraViewPreset(preset),
      (dAz, dPol) => this.orbitCameraBy(dAz, dPol),
    );
    vc.panelEl.style.position = "absolute";
    vc.panelEl.style.top = "58px";
    vc.panelEl.style.left = "16px";
    vc.panelEl.style.zIndex = "25";
    overlay.appendChild(vc.panelEl);
    this.viewCubeEl = vc.cubeEl;
    this.refreshViewCubeLabels = vc.refreshLabels;
    updateViewCubeOrientation(this.viewCubeEl, this.camera, this.controls);

    this.updateOverlayUIActiveStates();
    onLanguageChange(() => this.updateOverlayUIActiveStates());
    this.container.appendChild(overlay);
  }

  isSupported(): boolean {
    return Boolean(this.renderer && this.controls);
  }

  /** Gibt Details über die aktive GPU und Hardware-Beschleunigung zurück. */
  getGpuInfo(): { renderer: string; vendor: string; isHardwareAccelerated: boolean } {
    if (!this.renderer) {
      return { renderer: "None (WebGL unavailable)", vendor: "Unknown", isHardwareAccelerated: false };
    }
    try {
      const gl = this.renderer.getContext();
      if (!gl) {
        return { renderer: "Unknown", vendor: "Unknown", isHardwareAccelerated: false };
      }
      const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
      const unmaskedRenderer = debugInfo
        ? gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL)
        : gl.getParameter(gl.RENDERER) || "WebGL";
      const unmaskedVendor = debugInfo
        ? gl.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL)
        : gl.getParameter(gl.VENDOR) || "Unknown";

      const lower = String(unmaskedRenderer || "").toLowerCase();
      const isSoftware = lower.includes("swiftshader") || lower.includes("llvmpipe") || lower.includes("software");
      return {
        renderer: String(unmaskedRenderer),
        vendor: String(unmaskedVendor),
        isHardwareAccelerated: !isSoftware,
      };
    } catch {
      return { renderer: "WebGL", vendor: "Unknown", isHardwareAccelerated: true };
    }
  }

  show(onClose?: () => void) {
    if (!this.isSupported()) {
      onClose?.();
      return;
    }
    this.isVisible = true;
    this.container.style.display = "block";
    const isDark = this.getIsDark();
    this.currentTheme = isDark ? "dark" : "light";
    this.ensureActiveEnvironmentBuilt();
    this.updateSceneBackground();
    this.updateLighting();
    this.updateOverlayUIActiveStates();
    const closeBtn = this.container.querySelector("#v3d-close") as HTMLElement | null;
    if (closeBtn && onClose) {
      closeBtn.onclick = onClose;
    }
    if (this.trackGroup.children.length === 0 && this.currentLayout && this.lastCatalog) {
      this.updateLayout(this.currentLayout, this.lastCatalog, this.lastOptions);
    }
    this.onResize();
    this.requestShadowUpdate();
    this.startLoop();
  }

  resetScene() {
    this.clearGroup(this.trainHallGroup);
    this.clearGroup(this.landscapeGroup);
    this.clearGroup(this.tableGroup);
    this.clearGroup(this.woodPlateGroup);
    this.clearGroup(this.trackGroup);
    this.clearGroup(this.modelsGroup);
    this.clearGroup(this.shortCircuitGroup);
    this.builtHallKey = "";
    this.builtLandscapeKey = "";
    this.lastLayoutSignature = "";
    this.currentLayout = undefined;
    this.lastCatalog = undefined;
    this.lastOptions = undefined;
    this.gltfCache.clear();
    (this.renderer as any)?.renderLists?.dispose?.();
  }

  hide() {
    this.isVisible = false;
    this.container.style.display = "none";
    this.stopLoop();
    this.resetScene();
  }

  isOpen(): boolean {
    return this.isVisible;
  }

  setWoodPlate(enabled: boolean) {
    this.showWoodPlate = enabled;
    this.woodPlateGroup.visible = enabled;
    const btn = this.container.querySelector("#v3d-toggle-woodplate") as HTMLElement | null;
    btn?.classList.toggle("active", enabled);
    try {
      localStorage.setItem("railbench_3d_woodplate", enabled ? "1" : "0");
    } catch {}
    this.requestRender();
  }

  isWoodPlateEnabled(): boolean {
    return this.showWoodPlate;
  }

  private setupCelestialBodies() {
    const bodies = buildCelestialBodies(this.celestialGroup);
    this.sunMesh = bodies.sunMesh;
    this.sunCorona = bodies.sunCorona;
    this.moonMesh = bodies.moonMesh;
    this.moonGlow = bodies.moonGlow;
  }

  private applyTimeOfDay() {
    if (!this.scene) return;
    if (this.envMode === "minimal") {
      this.celestialGroup.visible = false;
      return;
    }
    this.celestialGroup.visible = true;

    let kf0 = SKY_KEYFRAMES[0]!;
    let kf1 = SKY_KEYFRAMES[SKY_KEYFRAMES.length - 1]!;
    for (let i = 0; i < SKY_KEYFRAMES.length - 1; i++) {
      if (this.timeOfDay >= SKY_KEYFRAMES[i]!.hour && this.timeOfDay <= SKY_KEYFRAMES[i + 1]!.hour) {
        kf0 = SKY_KEYFRAMES[i]!;
        kf1 = SKY_KEYFRAMES[i + 1]!;
        break;
      }
    }
    const span = kf1.hour - kf0.hour;
    const t = span > 0 ? (this.timeOfDay - kf0.hour) / span : 0;

    const curSky = this._curSky.setHex(kf0.sky).lerp(this._c1.setHex(kf1.sky), t);
    const curFog = this._curFog.setHex(kf0.fog).lerp(this._c1.setHex(kf1.fog), t);
    const curSun = this._curSun.setHex(kf0.sunLight).lerp(this._c1.setHex(kf1.sunLight), t);
    const curSunInt = THREE.MathUtils.lerp(kf0.sunInt, kf1.sunInt, t);
    const curHemiSky = this._curHemiSky.setHex(kf0.hemiSky).lerp(this._c1.setHex(kf1.hemiSky), t);
    const curHemiGnd = this._curHemiGnd.setHex(kf0.hemiGround).lerp(this._c1.setHex(kf1.hemiGround), t);
    const curHemiInt = THREE.MathUtils.lerp(kf0.hemiInt, kf1.hemiInt, t);
    const curLampGlow = THREE.MathUtils.lerp(kf0.lampGlow, kf1.lampGlow, t);

    if (this.scene.background instanceof THREE.Color) {
      this.scene.background.copy(curSky);
    } else {
      this.scene.background = curSky.clone();
    }
    if (this.scene.fog && this.scene.fog instanceof THREE.FogExp2) {
      this.scene.fog.color.copy(curFog);
    } else {
      this.scene.fog = new THREE.FogExp2(curFog.clone(), 0.000013);
    }

    if (this.hemiLight) {
      this.hemiLight.color.copy(curHemiSky);
      this.hemiLight.groundColor.copy(curHemiGnd);
      this.hemiLight.intensity = curHemiInt;
    }

    if (this.fillLight) {
      this.fillLight.color.copy(curHemiSky);
      this.fillLight.intensity = curHemiInt * 0.45;
    }

    const sunAngle = ((this.timeOfDay - 6) / 24) * Math.PI * 2;
    const moonAngle = sunAngle + Math.PI;

    const orbitR = 85000;
    const sunX = this.sceneCenter.x - Math.cos(sunAngle) * orbitR;
    const sunY = this.sceneCenter.y + Math.sin(sunAngle) * (orbitR * 0.88);
    const sunZ = this.sceneCenter.z - Math.cos(sunAngle * 0.5) * (orbitR * 0.28);

    const moonX = this.sceneCenter.x - Math.cos(moonAngle) * orbitR;
    const moonY = this.sceneCenter.y + Math.sin(moonAngle) * (orbitR * 0.88);
    const moonZ = this.sceneCenter.z - Math.cos(moonAngle * 0.5) * (orbitR * 0.28);

    if (this.sunMesh) {
      this.sunMesh.position.set(sunX, sunY, sunZ);
      this.sunMesh.visible = sunY > this.sceneCenter.y - 4500;
    }
    if (this.sunCorona) {
      this.sunCorona.position.set(sunX, sunY, sunZ);
      this.sunCorona.visible = sunY > this.sceneCenter.y - 4500;
    }

    if (this.moonMesh) {
      this.moonMesh.position.set(moonX, moonY, moonZ);
      this.moonMesh.visible = moonY > this.sceneCenter.y - 4500;
    }
    if (this.moonGlow) {
      this.moonGlow.position.set(moonX, moonY, moonZ);
      this.moonGlow.visible = moonY > this.sceneCenter.y - 4500;
    }

    if (this.sunLight) {
      const isDay = Math.sin(sunAngle) > -0.05;
      const lightDist = 5500;
      if (isDay) {
        const effY = Math.max(0.15, Math.sin(sunAngle));
        this.sunLight.position.set(
          this.sceneCenter.x - Math.cos(sunAngle) * lightDist,
          this.sceneCenter.y + effY * lightDist,
          this.sceneCenter.z - Math.cos(sunAngle * 0.5) * (lightDist * 0.35),
        );
      } else {
        const effY = Math.max(0.2, Math.sin(moonAngle));
        this.sunLight.position.set(
          this.sceneCenter.x - Math.cos(moonAngle) * lightDist,
          this.sceneCenter.y + effY * lightDist,
          this.sceneCenter.z - Math.cos(moonAngle * 0.5) * (lightDist * 0.35),
        );
      }
      this.sunLight.color.copy(curSun);
      this.sunLight.intensity = curSunInt;
      this.sunLight.target.position.copy(this.sceneCenter);
      this.sunLight.target.updateMatrixWorld();
    }

    for (const mat of this.nightLampMats) {
      mat.emissiveIntensity = curLampGlow;
    }

    const h = Math.floor(this.timeOfDay);
    const m = Math.floor((this.timeOfDay - h) * 60);
    const timeStr = `${h.toString().padStart(2, "0")}:${m.toString().padStart(2, "0")}`;
    if (this.lastTimeStr !== timeStr) {
      this.lastTimeStr = timeStr;
      if (this.cycleLabelEl) this.cycleLabelEl.textContent = timeStr;
      if (this.timeSliderEl && document.activeElement !== this.timeSliderEl) {
        this.timeSliderEl.value = this.timeOfDay.toFixed(2);
      }
    }
  }

  private updateDayNight(dt: number) {
    if (!this.dayNightActive || this.envMode === "minimal") return;
    const deltaHours = (dt * (24 / this.cycleDayDurationSec)) * this.cycleSpeed;
    this.timeOfDay = (this.timeOfDay + deltaHours) % 24;
    const now = performance.now();
    // Throttle sky/sun interpolation to ~15 FPS and shadow map depth pass to 2 FPS during active cycle
    if (now - this.lastDayNightRenderTime < 66) return;
    this.lastDayNightRenderTime = now;
    this.applyTimeOfDay();
    if (now - this.lastShadowUpdateTime > 500) {
      this.lastShadowUpdateTime = now;
      if (this.renderer) {
        this.renderer.shadowMap.needsUpdate = true;
      }
    }
    this.needsRender = true;
  }

  setTimeOfDay(hour: number) {
    this.timeOfDay = Math.max(0, Math.min(24, hour)) % 24;
    this.applyTimeOfDay();
    this.requestShadowUpdate();
  }

  setDayNightActive(active: boolean) {
    this.dayNightActive = active;
    this.cycleBtnEl?.classList.toggle("active", active);
    this.requestRender();
  }

  setCycleSpeed(speed: number) {
    this.cycleSpeed = speed;
    if (this.speedBtnEl) {
      const lbl = this.speedBtnEl.querySelector("#v3d-speed-label");
      if (lbl) lbl.textContent = `${speed}x`;
    }
  }

  private startLoop() {
    if (this.animId !== null) return;
    this.lastFrameTime = performance.now();
    const animate = () => {
      this.animId = requestAnimationFrame(animate);
      const now = performance.now();
      const dt = Math.min((now - this.lastFrameTime) / 1000, 0.1);
      this.lastFrameTime = now;

      if (this.dayNightActive && this.envMode !== "minimal") {
        this.updateDayNight(dt);
      }

      // OrbitControls with damping: update() returns true if camera target or position changed
      const controlsChanged = this.controls ? this.controls.update() : false;
      if (controlsChanged) {
        this.needsRender = true;
      }

      if (this.needsRender) {
        if (this.envMode !== "minimal") {
          if (this.sunCorona) this.sunCorona.quaternion.copy(this.camera.quaternion);
          if (this.moonGlow) this.moonGlow.quaternion.copy(this.camera.quaternion);
        }

        updateViewCubeOrientation(this.viewCubeEl, this.camera, this.controls);
        if (this.camera) {
          const aboveFloor = this.camera.position.y >= -700;
          this.trainHallGroup.visible = this.envMode === "hall" && aboveFloor;
          this.landscapeGroup.visible = this.envMode === "landscape" && aboveFloor;
        }
        if (this.renderer && this.scene && this.camera) {
          this.renderer.render(this.scene, this.camera);
        }
        this.needsRender = false;
      }
    };
    animate();
  }

  private stopLoop() {
    if (this.animId !== null) {
      cancelAnimationFrame(this.animId);
      this.animId = null;
    }
  }

  private onResize() {
    if (!this.container || !this.renderer || !this.camera) return;
    const width = this.container.clientWidth;
    const height = this.container.clientHeight;
    if (width === 0 || height === 0) return;

    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.25));
    this.renderer.setSize(width, height);
    this.requestRender();
  }

  private computeEnvironmentKey(layout: Layout, isDark: boolean): string {
    let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
    for (const bb of getBaseboards(layout)) {
      if (!bb.enabled) continue;
      const b = getBaseboardBounds(bb);
      minX = Math.min(minX, b.x);
      maxX = Math.max(maxX, b.x + b.width);
      minZ = Math.min(minZ, b.y);
      maxZ = Math.max(maxZ, b.y + b.depth);
    }
    if (!Number.isFinite(minX)) {
      for (const p of layout.pieces) {
        minX = Math.min(minX, p.transform.x);
        maxX = Math.max(maxX, p.transform.x);
        minZ = Math.min(minZ, p.transform.y);
        maxZ = Math.max(maxZ, p.transform.y);
      }
    }
    if (!Number.isFinite(minX)) {
      minX = -1200; maxX = 1200; minZ = -800; maxZ = 800;
    }
    return `${isDark ? 1 : 0}:${Math.round(minX)}:${Math.round(maxX)}:${Math.round(minZ)}:${Math.round(maxZ)}`;
  }

  private computeTableCenter(layout: Layout): { cx: number; cz: number } {
    let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
    for (const bb of getBaseboards(layout)) {
      if (!bb.enabled) continue;
      const b = getBaseboardBounds(bb);
      minX = Math.min(minX, b.x);
      maxX = Math.max(maxX, b.x + b.width);
      minZ = Math.min(minZ, b.y);
      maxZ = Math.max(maxZ, b.y + b.depth);
    }
    if (!Number.isFinite(minX)) {
      for (const p of layout.pieces) {
        minX = Math.min(minX, p.transform.x);
        maxX = Math.max(maxX, p.transform.x);
        minZ = Math.min(minZ, p.transform.y);
        maxZ = Math.max(maxZ, p.transform.y);
      }
    }
    const cx = Number.isFinite(minX) ? (minX + maxX) / 2 : 0;
    const cz = Number.isFinite(minZ) ? (minZ + maxZ) / 2 : 0;
    return { cx, cz };
  }

  private ensureActiveEnvironmentBuilt() {
    if (!this.currentLayout) return;
    const isDark = this.currentTheme === "dark";
    const envKey = this.computeEnvironmentKey(this.currentLayout, isDark);
    if (this.envMode === "hall" && this.builtHallKey !== envKey) {
      this.clearGroup(this.trainHallGroup);
      const hallRes = buildTrainHall(this.trainHallGroup, this.currentLayout, isDark);
      this.sceneCenter.set(hallRes.sceneCenter.cx, hallRes.sceneCenter.floorY, hallRes.sceneCenter.cz);
      this.nightLampMats = hallRes.nightLampMats;
      this.builtHallKey = envKey;
      freezeStaticTransforms(this.trainHallGroup);
    } else if (this.envMode === "landscape" && this.builtLandscapeKey !== envKey) {
      this.clearGroup(this.landscapeGroup);
      buildRealisticLandscape(this.landscapeGroup, this.currentLayout, isDark);
      this.builtLandscapeKey = envKey;
      freezeStaticTransforms(this.landscapeGroup);
    } else if (this.envMode === "minimal") {
      this.clearGroup(this.trainHallGroup);
      this.clearGroup(this.landscapeGroup);
      this.nightLampMats = [];
      this.builtHallKey = "";
      this.builtLandscapeKey = "";
      const { cx, cz } = this.computeTableCenter(this.currentLayout);
      this.sceneCenter.set(cx, 0, cz);
    }
  }

  private computeLayout3DSignature(
    layout: Layout,
    catalog: Catalog,
    options?: { showColorCode?: boolean; colorMode?: ColorMode; shortCircuit?: ShortCircuitDetectionResult },
  ): string {
    const colorMode: ColorMode = options?.colorMode ?? (options?.showColorCode === false ? "off" : "types");
    const scPts = options?.shortCircuit?.hasShortCircuit
      ? options.shortCircuit.shortCircuitPoints.map((p) => `${Math.round(p.x)}:${Math.round(p.y)}:${p.pieceId1}`).join(",")
      : "";
    const bbSig = getBaseboards(layout)
      .map((b) => `${b.id ?? ""}:${b.enabled ? 1 : 0}:${b.width}:${b.depth}:${b.customX ?? 0}:${b.customY ?? 0}:${b.rotation ?? 0}:${b.shape ?? "rect"}:${b.style ?? "wood"}:${b.customPolygon?.length ?? 0}`)
      .join("|");
    const layerSig = (layout.layers ?? [])
      .map((l) => `${l.id}:${l.visible ? 1 : 0}:${l.heightMm ?? 0}:${l.color ?? ""}:${l.woodPlateMode ?? ""}:${l.woodPlateThicknessMm ?? ""}:${l.woodPlateStyle ?? ""}:${l.plates?.map((pl) => `${pl.id}:${pl.width}:${pl.depth}:${pl.x ?? 0}:${pl.y ?? 0}:${pl.style ?? ""}:${pl.shape ?? ""}`).join(",") ?? ""}`)
      .join("|");
    const piecesSig = layout.pieces
      .map((p) => `${p.id}:${p.catalogId}:${Math.round(p.transform.x * 10)}:${Math.round(p.transform.y * 10)}:${Math.round(p.transform.rotation * 10)}:${p.layerId ?? ""}:${p.elevation?.start ?? 0}:${p.elevation?.end ?? 0}:${p.hasBridge ? 1 : 0}:${p.hasTunnel ? 1 : 0}:${p.hasCatenary ? 1 : 0}:${p.hasSignal ? 1 : 0}:${p.signalSide ?? ""}:${p.signalDirection ?? ""}:${p.hasLantern ? 1 : 0}:${p.hasDecoder ? 1 : 0}:${p.customLength ?? ""}:${p.customRadius ?? ""}:${p.customAngle ?? ""}:${p.customOffsets?.join(",") ?? ""}`)
      .join("|");
    const modelsSig = (layout.models ?? [])
      .map((m) => `${m.id}:${m.builtinId ?? m.fileName ?? ""}:${Math.round(m.transform.x * 10)}:${Math.round(m.transform.y * 10)}:${Math.round(m.transform.rotation * 10)}:${m.layerId ?? ""}:${m.elevation ?? 0}:${m.scale ?? 1}:${m.width}:${m.depth}:${m.height}:${m.mountainStyle ?? ""}:${m.mountainPath?.length ?? 0}`)
      .join("|");
    return `${catalog.system}:${layout.scale ?? ""}:${colorMode}:${scPts}#${bbSig}#${layerSig}#${piecesSig}#${modelsSig}`;
  }

  /** Baut die gesamte 3D-Szene auf: Tischform, Pfeiler, Schotterbett, Schienen und Schwellen nach Ebenen. */
  updateLayout(
    layout: Layout,
    catalog: Catalog,
    options?: { showColorCode?: boolean; colorMode?: ColorMode; shortCircuit?: ShortCircuitDetectionResult },
  ) {
    this.currentLayout = layout;
    this.lastCatalog = catalog;
    this.lastOptions = options;
    this.currentShortCircuit = options?.shortCircuit;
    this.layoutScale = parseModelScale(layout.scale);

    const nextSig = this.computeLayout3DSignature(layout, catalog, options);
    if (this.lastLayoutSignature === nextSig && (this.trackGroup.children.length > 0 || layout.pieces.length === 0)) {
      this.ensureActiveEnvironmentBuilt();
      this.requestRender();
      return;
    }
    this.lastLayoutSignature = nextSig;

    this.clearGroup(this.tableGroup);
    this.clearGroup(this.woodPlateGroup);
    this.clearGroup(this.trackGroup);
    this.clearGroup(this.modelsGroup);
    this.clearGroup(this.shortCircuitGroup);

    this.woodPlateGroup.visible = this.showWoodPlate;

    // 0. Aktive 3D-Umgebung (Lokhalle oder Landschaft) nur bei geänderter Plattengröße/Theme neu aufbauen
    this.ensureActiveEnvironmentBuilt();
    this.trainHallGroup.visible = this.envMode === "hall";
    this.landscapeGroup.visible = this.envMode === "landscape";
    this.applyTimeOfDay();

    // 1. 3D-Tisch / Grundplatte nach Profilform bauen
    const baseboards = getBaseboards(layout);
    const tableLayer = layout.layers?.find((l) => (l.heightMm ?? 0) === 0);
    const tableVisible = tableLayer ? tableLayer.visible : true;
    const tablePlateMode: LayerWoodPlateMode =
      tableLayer?.woodPlateMode && tableLayer.woodPlateMode !== "none"
        ? tableLayer.woodPlateMode
        : "table";
    const tableThickness: number | undefined = tableLayer?.woodPlateThicknessMm;

    if (tableVisible) {
      for (const bb of baseboards) {
        if (bb.enabled) {
          const matchingPlate = tableLayer?.plates?.find(
            (p) => (p.baseboardId || baseboards[0]?.id) === bb.id,
          );
          const effectiveStyle = matchingPlate?.style ?? tableLayer?.woodPlateStyle ?? bb.style ?? "wood";
          const bbConfig = bb.style === effectiveStyle ? bb : { ...bb, style: effectiveStyle };
          buildBaseboard3D(this.tableGroup, bbConfig, layout, catalog, tablePlateMode, tableThickness);
        }
      }
    }

    // 2. Ebenen vorbereiten
    const layerMap = new Map<string, Layer>();
    if (layout.layers) {
      for (const lay of layout.layers) layerMap.set(lay.id, lay);
    }

    const entryMap = new Map<string, CatalogEntry>();
    for (const e of catalog.entries) entryMap.set(e.id, e);

    const colorMode: ColorMode = options?.colorMode ?? (options?.showColorCode === false ? "off" : "types");

    // 3. Gleise mit realer 3D-Höhe (Elevation) nach Ebene erzeugen
    const trackItems: TrackBuildItem[] = [];
    const woodPlateItems: WoodPlateBuildItem[] = [];

    for (const piece of layout.pieces) {
      const layer = piece.layerId ? layerMap.get(piece.layerId) : undefined;
      if (layer && !layer.visible) continue;

      const baseElevation = layer?.heightMm ?? 0;
      const startElev = baseElevation + (piece.elevation?.start ?? 0);
      const endElev = baseElevation + (piece.elevation?.end ?? 0);

      const isRoadPiece = piece.catalogId.startsWith("ST-");
      let entry: CatalogEntry | undefined = entryMap.get(piece.catalogId) ?? catalogRegistry.findEntry(piece.catalogId);
      if (!entry) {
        entry = {
          id: piece.catalogId,
          name: piece.catalogId,
          kind: isRoadPiece && piece.catalogId.includes("-C") ? "curve" : "straight",
          category: isRoadPiece ? "Straßen" : "Geraden",
          description: "",
          length: isRoadPiece ? 200 : 188.3,
          isRoad: isRoadPiece,
          roadWidth: isRoadPiece ? 60 : undefined,
        };
      } else if (isRoadPiece && !entry.isRoad) {
        entry.isRoad = true;
      }

      entry = getEffectiveFlexEntry(entry, piece);

      trackItems.push({
        piece,
        entry,
        startElev,
        endElev,
        colorMode,
        layerColor: layer?.color,
      });

      // Wood Plate (Trassenbrett)
      const plateMode = layer?.woodPlateMode ?? (baseElevation !== 0 || Math.abs(endElev - startElev) > 0.01 ? "tight" : "none");
      if (plateMode === "tight") {
        const plateThickness = layer?.woodPlateThicknessMm ?? 8;
        woodPlateItems.push({
          piece,
          entry,
          startElev,
          endElev,
          thickness: plateThickness,
        });
      }
    }

    if (trackItems.length > 0) {
      buildAllTracks3D(this.trackGroup, trackItems, layout);
    }

    if (woodPlateItems.length > 0) {
      buildAllWoodPlates3D(this.woodPlateGroup, woodPlateItems);
    }

    // 4. 3D-Modelle & Scenery (mit Instanzierung identischer Modelle)
    if (layout.models && layout.models.length > 0) {
      const modelsWithElev: Array<{ model: Model3D; elev: number }> = [];
      for (const model of layout.models) {
        const layer = model.layerId ? layerMap.get(model.layerId) : undefined;
        if (layer && !layer.visible) continue;

        const baseElevation = layer?.heightMm ?? 0;
        const elev = baseElevation + (model.elevation ?? 0);
        modelsWithElev.push({ model, elev });

        const plateMode = layer?.woodPlateMode ?? (baseElevation !== 0 ? "tight" : "none");
        if (plateMode === "tight" && model.builtinId !== "mountain") {
          const plateThickness = layer?.woodPlateThicknessMm ?? 8;
          buildBuildingWoodPlate3D(this.woodPlateGroup, model, elev, plateThickness);
        }
      }

      buildAllModels3D(
        this.modelsGroup,
        this.gltfCache,
        this.gltfLoader,
        modelsWithElev,
        this.layoutScale,
        layout,
        entryMap,
      );
    }

    // 4B. Vollflächige Holzplatten / Tisch-Zwischenebenen
    if (layout.layers) {
      for (const layer of layout.layers) {
        if (!layer.visible) continue;
        if (layer.plates && layer.plates.length > 0) {
          for (const plate of layer.plates) {
            const targetBoard = (plate.baseboardId ? baseboards.find((b) => b.id === plate.baseboardId) : undefined) || baseboards[0];
            const isGroundMatchingBoard =
              (layer.heightMm ?? 0) === 0 &&
              targetBoard &&
              targetBoard.enabled &&
              (plate.baseboardId === targetBoard.id || !plate.baseboardId) &&
              (!plate.x || plate.x === 0) &&
              (!plate.y || plate.y === 0) &&
              plate.width === targetBoard.width &&
              plate.depth === targetBoard.depth &&
              plate.shape !== "custom" &&
              targetBoard.shape !== "custom" &&
              (!plate.shape || plate.shape === (targetBoard.shape ?? "rect"));

            if (isGroundMatchingBoard) {
              continue;
            }
            buildCustomLayerPlate3D(this.woodPlateGroup, layer, plate, layout, catalog);
          }
        } else if (layer.woodPlateMode === "table" && (layer.heightMm ?? 0) !== 0) {
          for (const bb of baseboards) {
            if (bb.enabled) {
              buildLayerDeckPlate3D(this.woodPlateGroup, layer, bb, layout, catalog);
            }
          }
        }
      }
    }

    // 5. 3D-Kurzschluss-Markierungen
    const scBtn = this.overlayEl?.querySelector("#v3d-short-circuit-btn") as HTMLElement | null;
    if (options?.shortCircuit?.hasShortCircuit) {
      if (scBtn) {
        scBtn.style.display = "inline-flex";
        const t = getTranslations().toolbar.status;
        const count = options.shortCircuit.shortCircuitPoints.length;
        scBtn.textContent = count > 1 ? `⚠️⚡ ${t.shortCircuitBadge3D} (${count})` : `⚠️⚡ ${t.shortCircuitBadge3D}`;
      }

      for (const pt of options.shortCircuit.shortCircuitPoints) {
        const p1 = layout.pieces.find((p) => p.id === pt.pieceId1);
        const layer1 = p1?.layerId ? layerMap.get(p1.layerId) : undefined;
        const baseElevation = layer1?.heightMm ?? 0;
        const elev = baseElevation + (p1?.elevation?.start ?? 0);

        const beaconGeo = new THREE.OctahedronGeometry(14, 0);
        const beaconMat = new THREE.MeshStandardMaterial({
          color: 0xef4444,
          emissive: 0xef4444,
          emissiveIntensity: 1.2,
          roughness: 0.1,
          metalness: 0.2,
        });
        const beacon = new THREE.Mesh(beaconGeo, beaconMat);
        beacon.position.set(pt.x, elev + 28, pt.y);
        this.shortCircuitGroup.add(beacon);

        const beamGeo = new THREE.CylinderGeometry(0.8, 0.8, 28, 6);
        const beamMat = new THREE.MeshBasicMaterial({ color: 0xdc2626 });
        const beam = new THREE.Mesh(beamGeo, beamMat);
        beam.position.set(pt.x, elev + 14, pt.y);
        this.shortCircuitGroup.add(beam);

        const ringGeo = new THREE.RingGeometry(8, 14, 16);
        ringGeo.rotateX(-Math.PI / 2);
        const ringMat = new THREE.MeshBasicMaterial({ color: 0xef4444, side: THREE.DoubleSide });
        const ring = new THREE.Mesh(ringGeo, ringMat);
        ring.position.set(pt.x, elev + 2.5, pt.y);
        this.shortCircuitGroup.add(ring);
      }
    } else {
      if (scBtn) {
        scBtn.style.display = "none";
      }
    }

    // Statische Objekt-Transformationen einfrieren (spart 1000e Matrix-Multiplikationen pro Frame)
    freezeStaticTransforms(this.tableGroup);
    freezeStaticTransforms(this.woodPlateGroup);
    freezeStaticTransforms(this.trackGroup);
    freezeStaticTransforms(this.modelsGroup);

    this.requestShadowUpdate();
  }

  /** Fokussiert die 3D-Kamera auf bestimmte Weltkoordinaten (z.B. Kurzschluss-Stelle). */
  focusOnPoint(x: number, z: number, y = 50) {
    if (this.controls) this.controls.target.set(x, y, z);
    if (this.camera) this.camera.position.set(x, y + 500, z + 700);
    if (this.controls) this.controls.update();
    this.requestRender();
  }

  resetCamera() {
    if (this.camera) this.camera.position.set(0, 1800, 2400);
    if (this.controls) {
      this.controls.target.set(0, 50, 0);
      this.controls.update();
    }
    this.requestRender();
  }

  buildCatenary3D(group: THREE.Group, entry: CatalogEntry, startElev: number, endElev: number, piece?: TrackPiece) {
    buildCatenary3D(group, entry, startElev, endElev, piece);
  }

  buildTurntableCatenary3D(group: THREE.Group, piece: TrackPiece, entry: CatalogEntry) {
    buildTurntableCatenary3D(group, piece, entry, this.currentLayout);
  }

  buildTransferTableCatenary3D(group: THREE.Group, piece: TrackPiece, entry: CatalogEntry) {
    buildTransferTableCatenary3D(group, piece, entry, this.currentLayout);
  }

  isPillarColliding(worldPos: Vec2, pillarTopElev: number, currentPieceId: string): boolean {
    return isPillarColliding(worldPos, pillarTopElev, currentPieceId, this.currentLayout);
  }

  buildTrackSignal3D(group: THREE.Group, entry: CatalogEntry, piece: TrackPiece) {
    buildTrackSignal3D(group, entry, piece);
  }

  buildTrackBridge3D(group: THREE.Group, entry: CatalogEntry, piece: TrackPiece, startElev: number, endElev: number) {
    buildTrackBridge3D(group, entry, piece, startElev, endElev, this.currentLayout);
  }

  private clearGroup(group: THREE.Group) {
    while (group.children.length > 0) {
      const obj = group.children[0]!;
      group.remove(obj);
      obj.traverse((child) => {
        if (child instanceof THREE.InstancedMesh) {
          child.instanceMatrix?.dispose?.();
          (child as any).instanceColor?.dispose?.();
        }
        if (child instanceof THREE.Mesh || child instanceof THREE.Line || child instanceof THREE.Points) {
          if (child.geometry && !child.geometry.userData?.shared) {
            child.geometry.dispose();
          }
          const mats = Array.isArray(child.material) ? child.material : [child.material];
          for (const m of mats) {
            if (m && !m.userData?.shared) {
              if (m.map) m.map.dispose();
              if (m.alphaMap) m.alphaMap.dispose();
              if (m.normalMap) m.normalMap.dispose();
              if (m.roughnessMap) m.roughnessMap.dispose();
              if (m.metalnessMap) m.metalnessMap.dispose();
              m.dispose();
            }
          }
        }
      });
      (obj as any).clear?.();
    }
    (this.renderer as any)?.renderLists?.dispose?.();
  }

  dispose() {
    this.stopLoop();
    if (this.resizeHandler && typeof window !== "undefined") {
      window.removeEventListener("resize", this.resizeHandler);
      this.resizeHandler = null;
    }
    if (this.controls) {
      this.controls.dispose();
      this.controls = null as any;
    }
    this.clearGroup(this.celestialGroup);
    this.clearGroup(this.landscapeGroup);
    this.clearGroup(this.trainHallGroup);
    this.clearGroup(this.tableGroup);
    this.clearGroup(this.woodPlateGroup);
    this.clearGroup(this.trackGroup);
    this.clearGroup(this.modelsGroup);
    this.clearGroup(this.shortCircuitGroup);

    if (this.renderer) {
      this.renderer.dispose();
      this.renderer.forceContextLoss?.();
      if (this.renderer.domElement && this.renderer.domElement.parentElement) {
        this.renderer.domElement.parentElement.removeChild(this.renderer.domElement);
      }
      this.renderer = null as any;
    }
    if (this.container && this.container.parentElement) {
      this.container.parentElement.removeChild(this.container);
    }
  }
}
