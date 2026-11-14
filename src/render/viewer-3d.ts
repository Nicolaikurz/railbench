import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import type { Layout, TrackPiece, Layer, BaseboardConfig, Model3D, ColorMode, LayerWoodPlateMode, Vec2 } from "@/core/types";
import { getBaseboards } from "@/core/layout";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import { deg2rad } from "@/core/geometry";
import { parseModelScale, type ModelScale } from "@/core/model-scale";
import type { ShortCircuitDetectionResult } from "@/core/simulation";
import { getTranslations, getLanguage } from "@/i18n";
import { getEffectiveFlexEntry } from "@/core/flextrack";
import { buildRealisticLandscape } from "./viewer-3d-landscape";
import { buildTrainHall } from "./viewer-3d-trainhall";
import { buildBaseboard3D, buildCustomLayerPlate3D, buildLayerDeckPlate3D } from "./viewer-3d-baseboard";
import { buildWoodPlatePiece3D } from "./viewer-3d-roads";
import { buildBuildingWoodPlate3D, buildModel3D } from "./viewer-3d-models";
import { buildTrackPiece3D } from "./viewer-3d-track";
import { buildCatenary3D, buildTurntableCatenary3D, buildTransferTableCatenary3D } from "./viewer-3d-catenary";
import { buildTrackSignal3D, isPillarColliding, buildTrackBridge3D } from "./viewer-3d-bridges";

export type Viewer3DEnvironment = "landscape" | "hall" | "minimal";

interface SkyKeyframe {
  hour: number;
  sky: number;
  fog: number;
  sunLight: number;
  sunInt: number;
  hemiSky: number;
  hemiGround: number;
  hemiInt: number;
  lampGlow: number;
  icon: string;
}

const SKY_KEYFRAMES: SkyKeyframe[] = [
  { hour: 0.0, sky: 0x030712, fog: 0x081120, sunLight: 0x60a5fa, sunInt: 0.25, hemiSky: 0x0c192c, hemiGround: 0x040812, hemiInt: 0.25, lampGlow: 1.8, icon: "🌙" },
  { hour: 4.8, sky: 0x0c1b33, fog: 0x14203d, sunLight: 0x818cf8, sunInt: 0.32, hemiSky: 0x1e293b, hemiGround: 0x0f172a, hemiInt: 0.32, lampGlow: 1.6, icon: "🌌" },
  { hour: 6.0, sky: 0xf97316, fog: 0xfdba74, sunLight: 0xfb923c, sunInt: 0.9, hemiSky: 0xfdba74, hemiGround: 0x78350f, hemiInt: 0.65, lampGlow: 0.9, icon: "🌅" },
  { hour: 8.0, sky: 0x60a5fa, fog: 0xbae6fd, sunLight: 0xfef08a, sunInt: 1.35, hemiSky: 0xbde0fe, hemiGround: 0x27272a, hemiInt: 0.8, lampGlow: 0.2, icon: "🌤️" },
  { hour: 12.0, sky: 0x38bdf8, fog: 0xb5daf0, sunLight: 0xfffaed, sunInt: 1.6, hemiSky: 0xe0f2fe, hemiGround: 0x334155, hemiInt: 0.95, lampGlow: 0.0, icon: "☀️" },
  { hour: 17.5, sky: 0xf59e0b, fog: 0xfde047, sunLight: 0xfb923c, sunInt: 1.45, hemiSky: 0xfef08a, hemiGround: 0x451a03, hemiInt: 0.82, lampGlow: 0.3, icon: "🌇" },
  { hour: 19.2, sky: 0xd946ef, fog: 0xf472b6, sunLight: 0xf43f5e, sunInt: 0.75, hemiSky: 0xa855f7, hemiGround: 0x3b0764, hemiInt: 0.55, lampGlow: 1.3, icon: "🌆" },
  { hour: 21.0, sky: 0x0f172a, fog: 0x1e1b4b, sunLight: 0x93c5fd, sunInt: 0.32, hemiSky: 0x1e293b, hemiGround: 0x050d1a, hemiInt: 0.3, lampGlow: 1.8, icon: "🌙" },
  { hour: 24.0, sky: 0x030712, fog: 0x081120, sunLight: 0x60a5fa, sunInt: 0.25, hemiSky: 0x0c192c, hemiGround: 0x040812, hemiInt: 0.25, lampGlow: 1.8, icon: "🌙" },
];

export class Viewer3D {
  private container: HTMLElement;
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private renderer: THREE.WebGLRenderer;
  private controls: OrbitControls;
  private animId: number | null = null;
  private trackGroup = new THREE.Group();
  private tableGroup = new THREE.Group();
  private pillarsGroup = new THREE.Group();
  private woodPlateGroup = new THREE.Group();
  private modelsGroup = new THREE.Group();
  private trainHallGroup = new THREE.Group();
  private landscapeGroup = new THREE.Group();
  private shortCircuitGroup = new THREE.Group();
  private currentLayout?: Layout;
  private currentShortCircuit?: ShortCircuitDetectionResult;
  private envMode: Viewer3DEnvironment = "hall";
  private showTrainHall = true;
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

  // Tag/Nacht-Zyklus & wandernde Sonne
  private dayNightActive = true;
  private timeOfDay = 14.0; // 14:00 Uhr
  private cycleSpeed = 1.0;
  private readonly cycleDayDurationSec = 1200; // 20 Minuten für einen vollen 24h-Tag (Bereich 15 - 30 Min.)
  private lastFrameTime = performance.now();
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
    this.showTrainHall = this.envMode === "hall";

    const savedWoodPlate = typeof localStorage !== "undefined" ? localStorage.getItem("railbench_3d_woodplate") : null;
    this.showWoodPlate = savedWoodPlate === "1";
    this.woodPlateGroup.visible = this.showWoodPlate;

    // 1. Scene
    this.scene = new THREE.Scene();
    this.updateSceneBackground();

    // 2. Camera
    const width = host.clientWidth || 800;
    const height = host.clientHeight || 600;
    this.camera = new THREE.PerspectiveCamera(45, width / height, 5, 220000);
    this.camera.position.set(0, 1600, 2200);

    // 3. WebGL Renderer
    try {
      this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, powerPreference: "default" });
      this.renderer.setSize(width, height);
      this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      this.renderer.shadowMap.enabled = true;
      this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
      this.container.appendChild(this.renderer.domElement);

      // 4. Orbit Controls
      this.controls = new OrbitControls(this.camera, this.renderer.domElement);
      this.controls.enableDamping = true;
      this.controls.dampingFactor = 0.08;
      this.controls.maxPolarAngle = Math.PI / 2 - 0.02; // Nicht unter den Tisch blicken
      this.controls.minDistance = 80;
      this.controls.maxDistance = 65000;
      this.controls.target.set(0, 0, 0);
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
    this.scene.add(this.pillarsGroup);
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
    window.addEventListener("resize", () => this.onResize());
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

  setTheme(theme: "light" | "dark") {
    this.currentTheme = theme;
    this.updateSceneBackground();
    this.updateLighting();
    this.updateOverlayUIActiveStates();
  }

  private setupLighting() {
    this.hemiLight = new THREE.HemisphereLight(0xffffff, 0x444444, 0.75);
    this.hemiLight.position.set(0, 3000, 0);
    this.scene.add(this.hemiLight);

    this.sunLight = new THREE.DirectionalLight(0xfff8ed, 1.4);
    this.sunLight.position.set(1600, 3200, 1400);
    this.sunLight.castShadow = true;
    this.sunLight.shadow.mapSize.width = 2048;
    this.sunLight.shadow.mapSize.height = 2048;
    this.sunLight.shadow.camera.near = 100;
    this.sunLight.shadow.camera.far = 18000;
    const d = 3200;
    this.sunLight.shadow.camera.left = -d;
    this.sunLight.shadow.camera.right = d;
    this.sunLight.shadow.camera.top = d;
    this.sunLight.shadow.camera.bottom = -d;
    this.sunLight.shadow.bias = -0.0005;
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
    this.landscapeGroup.visible = mode === "landscape";
    this.trainHallGroup.visible = mode === "hall";
    this.celestialGroup.visible = mode !== "minimal";
    this.showTrainHall = mode === "hall";
    this.updateSceneBackground();
    this.updateLighting();

    try {
      localStorage.setItem("railbench_3d_env", mode);
      localStorage.setItem("railbench_3d_trainhall", mode === "hall" ? "1" : "0");
    } catch {}

    this.updateOverlayUIActiveStates();
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
    }

    const btnHall = this.overlayEl.querySelector("#v3d-env-hall");
    const btnMinimal = this.overlayEl.querySelector("#v3d-env-minimal");
    btnHall?.classList.toggle("active", this.envMode === "hall");
    btnMinimal?.classList.toggle("active", this.envMode === "minimal");
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

    this.updateOverlayUIActiveStates();
    this.container.appendChild(overlay);
  }

  isSupported(): boolean {
    return Boolean(this.renderer && this.controls);
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
    this.updateSceneBackground();
    this.updateLighting();
    this.updateOverlayUIActiveStates();
    const closeBtn = this.container.querySelector("#v3d-close") as HTMLElement | null;
    if (closeBtn && onClose) {
      closeBtn.onclick = onClose;
    }
    this.onResize();
    this.startLoop();
  }

  hide() {
    this.isVisible = false;
    this.container.style.display = "none";
    this.stopLoop();
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
  }

  isWoodPlateEnabled(): boolean {
    return this.showWoodPlate;
  }

  private setupCelestialBodies() {
    this.celestialGroup.clear();

    const sunGeo = new THREE.SphereGeometry(2400, 24, 24);
    const sunMat = new THREE.MeshBasicMaterial({ color: 0xfffbeb });
    this.sunMesh = new THREE.Mesh(sunGeo, sunMat);
    this.celestialGroup.add(this.sunMesh);

    const coronaGeo = new THREE.RingGeometry(2400, 8200, 36);
    const coronaMat = new THREE.MeshBasicMaterial({
      color: 0xfef08a,
      transparent: true,
      opacity: 0.45,
      side: THREE.DoubleSide,
      depthWrite: false,
    });
    this.sunCorona = new THREE.Mesh(coronaGeo, coronaMat);
    this.celestialGroup.add(this.sunCorona);

    const moonGeo = new THREE.SphereGeometry(1800, 24, 24);
    const moonMat = new THREE.MeshStandardMaterial({
      color: 0xf1f5f9,
      roughness: 0.85,
      metalness: 0.15,
      emissive: 0x93c5fd,
      emissiveIntensity: 0.25,
    });
    this.moonMesh = new THREE.Mesh(moonGeo, moonMat);
    this.celestialGroup.add(this.moonMesh);

    const moonGlowGeo = new THREE.RingGeometry(1800, 5600, 32);
    const moonGlowMat = new THREE.MeshBasicMaterial({
      color: 0xa5b4fc,
      transparent: true,
      opacity: 0.35,
      side: THREE.DoubleSide,
      depthWrite: false,
    });
    this.moonGlow = new THREE.Mesh(moonGlowGeo, moonGlowMat);
    this.celestialGroup.add(this.moonGlow);
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

    const curSky = new THREE.Color(kf0.sky).lerp(new THREE.Color(kf1.sky), t);
    const curFog = new THREE.Color(kf0.fog).lerp(new THREE.Color(kf1.fog), t);
    const curSun = new THREE.Color(kf0.sunLight).lerp(new THREE.Color(kf1.sunLight), t);
    const curSunInt = THREE.MathUtils.lerp(kf0.sunInt, kf1.sunInt, t);
    const curHemiSky = new THREE.Color(kf0.hemiSky).lerp(new THREE.Color(kf1.hemiSky), t);
    const curHemiGnd = new THREE.Color(kf0.hemiGround).lerp(new THREE.Color(kf1.hemiGround), t);
    const curHemiInt = THREE.MathUtils.lerp(kf0.hemiInt, kf1.hemiInt, t);
    const curLampGlow = THREE.MathUtils.lerp(kf0.lampGlow, kf1.lampGlow, t);

    if (this.scene.background instanceof THREE.Color) {
      this.scene.background.copy(curSky);
    } else {
      this.scene.background = curSky;
    }
    if (this.scene.fog && this.scene.fog instanceof THREE.FogExp2) {
      this.scene.fog.color.copy(curFog);
    } else {
      this.scene.fog = new THREE.FogExp2(curFog, 0.000013);
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
    if (this.cycleLabelEl) this.cycleLabelEl.textContent = timeStr;
    if (this.timeSliderEl && document.activeElement !== this.timeSliderEl) {
      this.timeSliderEl.value = this.timeOfDay.toFixed(2);
    }
  }

  private updateDayNight(dt: number) {
    if (!this.dayNightActive) return;
    const deltaHours = (dt * (24 / this.cycleDayDurationSec)) * this.cycleSpeed;
    this.timeOfDay = (this.timeOfDay + deltaHours) % 24;
    this.applyTimeOfDay();
  }

  setTimeOfDay(hour: number) {
    this.timeOfDay = Math.max(0, Math.min(24, hour)) % 24;
    this.applyTimeOfDay();
  }

  setDayNightActive(active: boolean) {
    this.dayNightActive = active;
    this.cycleBtnEl?.classList.toggle("active", active);
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

      if (this.dayNightActive) {
        this.updateDayNight(dt);
      }

      if (this.sunCorona) this.sunCorona.quaternion.copy(this.camera.quaternion);
      if (this.moonGlow) this.moonGlow.quaternion.copy(this.camera.quaternion);

      if (this.controls) this.controls.update();
      if (this.renderer && this.scene && this.camera) {
        this.renderer.render(this.scene, this.camera);
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
    this.renderer.setSize(width, height);
  }

  /** Baut die gesamte 3D-Szene auf: Tischform, Pfeiler, Schotterbett, Schienen und Schwellen nach Ebenen. */
  updateLayout(
    layout: Layout,
    catalog: Catalog,
    options?: { showColorCode?: boolean; colorMode?: ColorMode; shortCircuit?: ShortCircuitDetectionResult },
  ) {
    this.currentLayout = layout;
    this.currentShortCircuit = options?.shortCircuit;
    this.layoutScale = parseModelScale(layout.scale);
    this.clearGroup(this.tableGroup);
    this.clearGroup(this.pillarsGroup);
    this.clearGroup(this.woodPlateGroup);
    this.clearGroup(this.trackGroup);
    this.clearGroup(this.modelsGroup);
    this.clearGroup(this.trainHallGroup);
    this.clearGroup(this.landscapeGroup);
    this.clearGroup(this.shortCircuitGroup);

    this.woodPlateGroup.visible = this.showWoodPlate;

    // 0a. Train Hall mit umgebendem Wald
    const isDark = this.currentTheme === "dark";
    const hallRes = buildTrainHall(this.trainHallGroup, layout, isDark);
    this.sceneCenter.set(hallRes.sceneCenter.cx, hallRes.sceneCenter.floorY, hallRes.sceneCenter.cz);
    this.nightLampMats = hallRes.nightLampMats;

    // 0b. Realistische Natur-Landschaft
    buildRealisticLandscape(this.landscapeGroup, layout, isDark);

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

      buildTrackPiece3D(this.trackGroup, piece, entry, startElev, endElev, colorMode, layer?.color, layout);

      // Wood Plate (Trassenbrett)
      const plateMode = layer?.woodPlateMode ?? (baseElevation !== 0 || Math.abs(endElev - startElev) > 0.01 ? "tight" : "none");
      if (plateMode === "tight") {
        const plateThickness = layer?.woodPlateThicknessMm ?? 8;
        const plateGroup = new THREE.Group();
        plateGroup.position.set(piece.transform.x, startElev, piece.transform.y);
        plateGroup.rotation.y = -deg2rad(piece.transform.rotation);
        buildWoodPlatePiece3D(plateGroup, piece, entry, endElev - startElev, plateThickness);
        this.woodPlateGroup.add(plateGroup);
      }
    }

    // 4. 3D-Modelle & Scenery
    if (layout.models && layout.models.length > 0) {
      for (const model of layout.models) {
        const layer = model.layerId ? layerMap.get(model.layerId) : undefined;
        if (layer && !layer.visible) continue;

        const baseElevation = layer?.heightMm ?? 0;
        const elev = baseElevation + (model.elevation ?? 0);

        buildModel3D(this.modelsGroup, this.gltfCache, this.gltfLoader, model, elev, this.layoutScale, layout, entryMap);

        const plateMode = layer?.woodPlateMode ?? (baseElevation !== 0 ? "tight" : "none");
        if (plateMode === "tight" && model.builtinId !== "mountain") {
          const plateThickness = layer?.woodPlateThicknessMm ?? 8;
          buildBuildingWoodPlate3D(this.woodPlateGroup, model, elev, plateThickness);
        }
      }
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
  }

  /** Fokussiert die 3D-Kamera auf bestimmte Weltkoordinaten (z.B. Kurzschluss-Stelle). */
  focusOnPoint(x: number, z: number, y = 50) {
    if (this.controls) this.controls.target.set(x, y, z);
    if (this.camera) this.camera.position.set(x, y + 500, z + 700);
    if (this.controls) this.controls.update();
  }

  resetCamera() {
    if (this.camera) this.camera.position.set(0, 1800, 2400);
    if (this.controls) {
      this.controls.target.set(0, 50, 0);
      this.controls.update();
    }
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
        if (child instanceof THREE.Mesh) {
          child.geometry?.dispose();
          if (Array.isArray(child.material)) {
            child.material.forEach((m) => m.dispose());
          } else if (child.material) {
            child.material.dispose();
          }
        }
      });
    }
  }
}
