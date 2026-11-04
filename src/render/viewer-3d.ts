import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import type { Layout, TrackPiece, Layer, BaseboardConfig, Model3D, ColorMode } from "@/core/types";
import { getBaseboardPolygon, getBaseboardBounds, getBaseboards, computeBaseboardLegPositions } from "@/core/layout";
import type { Catalog, CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import { deg2rad, getPieceLocalCenter } from "@/core/geometry";
import { getTrackBlueprintColor } from "@/render/blueprint-colors";
import { createBuiltinModel3DMesh } from "@/render/builtin-models-3d";
import type { ShortCircuitDetectionResult } from "@/core/simulation";
import { getTranslations } from "@/i18n";

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
  private currentShortCircuit?: ShortCircuitDetectionResult;
  private envMode: Viewer3DEnvironment = "hall";
  private showTrainHall = true;
  private showWoodPlate = true;
  private gltfLoader = new GLTFLoader();
  private gltfCache = new Map<string, THREE.Group>();
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
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, powerPreference: "high-performance" });
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

    // Himmelskörper (Sonne, Mond, Korona) initialisieren
    this.setupCelestialBodies();
    this.applyTimeOfDay();

    // 7. Overlay Controls (2D zurück, Reset View, Ebenen-Legende)
    this.setupOverlayUI();

    // Resize listener
    window.addEventListener("resize", () => this.onResize());
  }

  private updateSceneBackground() {
    if (this.envMode === "minimal") {
      const isDark = this.currentTheme === "dark";
      const studioColor = isDark ? 0x0f172a : 0xecf0f5;
      this.scene.background = new THREE.Color(studioColor);
      this.scene.fog = new THREE.FogExp2(studioColor, 0.00008);
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
      const isDark = this.currentTheme === "dark";
      this.sunLight.color.setHex(0xffffff);
      this.sunLight.intensity = isDark ? 0.7 : 1.3;
      this.hemiLight.color.setHex(0xffffff);
      this.hemiLight.groundColor.setHex(0x64748b);
      this.hemiLight.intensity = isDark ? 0.4 : 0.8;
      this.fillLight.color.setHex(0x94a3b8);
      this.fillLight.intensity = 0.35;
      this.sunLight.position.set(1600, 3200, 1400);
    } else {
      this.applyTimeOfDay();
    }
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
    const btnLandscape = this.overlayEl.querySelector("#v3d-env-landscape");
    const btnHall = this.overlayEl.querySelector("#v3d-env-hall");
    const btnMinimal = this.overlayEl.querySelector("#v3d-env-minimal");
    btnLandscape?.classList.toggle("active", this.envMode === "landscape");
    btnHall?.classList.toggle("active", this.envMode === "hall");
    btnMinimal?.classList.toggle("active", this.envMode === "minimal");
  }

  private setupOverlayUI() {
    const overlay = document.createElement("div");
    overlay.className = "viewer-3d-overlay";  
    this.overlayEl = overlay;

    overlay.innerHTML = `
      <div class="viewer-3d-badge">
        <span class="v3d-icon">🧊</span>
        <span class="v3d-title">3D Elevation & Layout View</span>
      </div>
      <div class="viewer-3d-actions">
        <div class="v3d-daynight-bar" title="Tag/Nacht-Zyklus mit wandernder Sonne">
          <button class="v3d-btn ${this.dayNightActive ? "active" : ""}" id="v3d-toggle-cycle" title="Tag/Nacht-Zyklus starten oder anhalten">
            <span id="v3d-cycle-label">14:00</span>
          </button>
          <input type="range" id="v3d-time-slider" class="v3d-time-slider" min="0" max="24" step="0.05" value="${this.timeOfDay.toFixed(2)}" title="Tageszeit einstellen (00:00 - 24:00 Uhr)">
          <button class="v3d-btn" id="v3d-cycle-speed" title="Geschwindigkeit ändern (1x = 20 Min. pro Tag)">
            <span id="v3d-speed-label">${this.cycleSpeed}x</span>
          </button>
        </div>
        <button class="v3d-btn" id="v3d-short-circuit-btn" style="display: none; background: #dc2626; color: #ffffff; border-color: #ef4444; font-weight: 700;" title="Kurzschluss anzeigen">
          <span>⚠️⚡ Kurzschluss</span>
        </button>
        <button class="v3d-btn v3d-btn-back" id="v3d-close" title="Zurück zum 2D-Gleisplan (Esc)">
          <span>← Zurück zum 2D-Plan</span>
        </button>
      </div>
    `;

    this.timeSliderEl = overlay.querySelector("#v3d-time-slider");
    this.cycleBtnEl = overlay.querySelector("#v3d-toggle-cycle");
    this.cycleLabelEl = overlay.querySelector("#v3d-cycle-label");
    this.speedBtnEl = overlay.querySelector("#v3d-cycle-speed");

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

    this.container.appendChild(overlay);
  }

  show(onClose?: () => void) {
    this.isVisible = true;
    this.container.style.display = "block";
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

    // 1. Sonne (leuchtender Himmelskörper)
    const sunGeo = new THREE.SphereGeometry(2400, 24, 24);
    const sunMat = new THREE.MeshBasicMaterial({ color: 0xfffbeb });
    this.sunMesh = new THREE.Mesh(sunGeo, sunMat);
    this.celestialGroup.add(this.sunMesh);

    // Sonnenkorona (leuchtender Kranz mit Transparenz)
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

    // 2. Mond (kraterfarbener Erdtrabant mit kühlem Schimmer)
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

    // Mondschein-Halo
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

    // 1. Interpolation der Himmels- und Lichtfarben über SKY_KEYFRAMES
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

    // 2. Sonnen- und Mondbahn am Himmelsgewölbe berechnen
    // 06:00: Osten (+X), 12:00: Zenit (+Y), 18:00: Westen (-X), 24:00/00:00: Tiefstand (-Y)
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

    // 3. Gerichtete Lichtquelle (Sonne am Tag, Mondschein in der Nacht)
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

    // 4. Laternen & Hallenleuchten bei Dämmerung & Nacht aufleuchten lassen
    for (const mat of this.nightLampMats) {
      mat.emissiveIntensity = curLampGlow;
    }

    // 5. UI-Elemente synchronisieren (Zeitanzeige ohne Icons)
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
    // 24 Stunden in 20 Minuten simuliert (1.200 Sekunden bei 1x Basis-Geschwindigkeit)
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

      this.controls.update();
      this.renderer.render(this.scene, this.camera);
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
    this.currentShortCircuit = options?.shortCircuit;
    this.clearGroup(this.tableGroup);
    this.clearGroup(this.pillarsGroup);
    this.clearGroup(this.woodPlateGroup);
    this.clearGroup(this.trackGroup);
    this.clearGroup(this.modelsGroup);
    this.clearGroup(this.trainHallGroup);
    this.clearGroup(this.landscapeGroup);
    this.clearGroup(this.shortCircuitGroup);

    this.woodPlateGroup.visible = this.showWoodPlate;

    // 0a. Train Hall (Lokhalle / Bahnhofshalle) mit umgebendem Wald
    this.buildTrainHall(layout);

    // 0b. Realistische Natur-Landschaft (Echte Welt - Hügel, Fluss, Straßen, Mischwälder & Berge)
    this.buildRealisticLandscape(layout);

    this.trainHallGroup.visible = this.envMode === "hall";
    this.landscapeGroup.visible = this.envMode === "landscape";
    this.applyTimeOfDay();

    // 1. 3D-Tisch / Grundplatte nach Profilform bauen (Rechteck, L, U, T)
    const baseboards = getBaseboards(layout);
    for (const bb of baseboards) {
      if (bb.enabled) {
        this.buildBaseboard3D(bb);
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
      // Ausgeblendete Ebene?
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

      this.buildTrackPiece3D(piece, entry, startElev, endElev, colorMode, layer?.color);

      // Wood Plate (Trassenbrett)
      const plateGroup = new THREE.Group();
      plateGroup.position.set(piece.transform.x, startElev, piece.transform.y);
      plateGroup.rotation.y = -deg2rad(piece.transform.rotation);
      this.buildWoodPlatePiece3D(plateGroup, piece, entry, endElev - startElev);
      this.woodPlateGroup.add(plateGroup);
    }

    // 4. 3D-Modelle & Scenery (Built-in & Custom .glb/.gltf)
    if (layout.models && layout.models.length > 0) {
      for (const model of layout.models) {
        const layer = model.layerId ? layerMap.get(model.layerId) : undefined;
        if (layer && !layer.visible) continue;

        const baseElevation = layer?.heightMm ?? 0;
        const elev = baseElevation + (model.elevation ?? 0);

        this.buildModel3D(model, elev);
      }
    }

    // 5. 3D-Kurzschluss-Markierungen (2-Leiter-System)
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

        // 3D-Warnleuchte / pulsierendes Warn-Prisma über der Kurzschluss-Stelle
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

        // Vertikaler roter Leitstrahl hinunter zur Schiene
        const beamGeo = new THREE.CylinderGeometry(0.8, 0.8, 28, 6);
        const beamMat = new THREE.MeshBasicMaterial({ color: 0xdc2626 });
        const beam = new THREE.Mesh(beamGeo, beamMat);
        beam.position.set(pt.x, elev + 14, pt.y);
        this.shortCircuitGroup.add(beam);

        // Roter Warnring auf dem Gleisbett
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
    this.controls.target.set(x, y, z);
    this.camera.position.set(x, y + 500, z + 700);
    this.controls.update();
  }

  private buildModel3D(model: Model3D, elevation: number) {
    if (model.modelType === "builtin") {
      const mesh = createBuiltinModel3DMesh(model.builtinId || "station", model);
      mesh.position.set(model.transform.x, elevation, model.transform.y);
      mesh.rotation.y = -deg2rad(model.transform.rotation);
      if (model.scale && model.scale !== 1.0) {
        mesh.scale.setScalar(model.scale);
      }
      this.modelsGroup.add(mesh);
    } else if (model.fileData) {
      // Custom .glb oder .gltf
      const cached = this.gltfCache.get(model.id);
      if (cached) {
        const clone = cached.clone();
        clone.position.set(model.transform.x, elevation, model.transform.y);
        clone.rotation.y = -deg2rad(model.transform.rotation);
        if (model.scale && model.scale !== 1.00) {
          clone.scale.setScalar(model.scale);
        }
        this.modelsGroup.add(clone);
      } else {
        this.loadCustomModel3D(model, elevation);
      }
    }
  }

  private loadCustomModel3D(model: Model3D, elevation: number) {
    if (!model.fileData) return;
    try {
      if (model.modelType === "glb") {
        const arrayBuffer = base64ToArrayBuffer(model.fileData);
        this.gltfLoader.parse(
          arrayBuffer,
          "",
          (gltf) => {
            const root = gltf.scene;
            root.traverse((obj) => {
              if (obj instanceof THREE.Mesh) {
                obj.castShadow = true;
                obj.receiveShadow = true;
              }
            });
            this.gltfCache.set(model.id, root);
            const clone = root.clone();
            clone.position.set(model.transform.x, elevation, model.transform.y);
            clone.rotation.y = -deg2rad(model.transform.rotation);
            if (model.scale && model.scale !== 1.0) {
              clone.scale.setScalar(model.scale);
            }
            this.modelsGroup.add(clone);
          },
          (err) => console.error("Error loading glb model in 3D viewer:", err),
        );
      } else {
        let text = model.fileData;
        if (text.startsWith("data:")) {
          text = atob(text.split(",")[1] || "");
        }
        this.gltfLoader.parse(
          text,
          "",
          (gltf) => {
            const root = gltf.scene;
            root.traverse((obj) => {
              if (obj instanceof THREE.Mesh) {
                obj.castShadow = true;
                obj.receiveShadow = true;
              }
            });
            this.gltfCache.set(model.id, root);
            const clone = root.clone();
            clone.position.set(model.transform.x, elevation, model.transform.y);
            clone.rotation.y = -deg2rad(model.transform.rotation);
            if (model.scale && model.scale !== 1.0) {
              clone.scale.setScalar(model.scale);
            }
            this.modelsGroup.add(clone);
          },
          (err) => console.error("Error loading gltf model in 3D viewer:", err),
        );
      }
    } catch (e) {
      console.error("Failed to parse custom 3D model:", e);
    }
  }

  private buildBaseboard3D(config: BaseboardConfig) {
    const polygon = getBaseboardPolygon(config);
    if (polygon.length < 3) return;

    // Shape im 2D-Raum (X, Y) erstellen
    const shape = new THREE.Shape();
    shape.moveTo(polygon[0]!.x, polygon[0]!.y);
    for (let i = 1; i < polygon.length; i++) {
      shape.lineTo(polygon[i]!.x, polygon[i]!.y);
    }
    shape.closePath();

    // Extrusion (Tischplatte mit 35 mm Stärke und gefaster Kante)
    const bevelThickness = 2.5;
    const extrudeSettings: THREE.ExtrudeGeometryOptions = {
      depth: 35,
      bevelEnabled: true,
      bevelSegments: 3,
      steps: 1,
      bevelSize: 2.5,
      bevelThickness,
    };

    const geometry = new THREE.ExtrudeGeometry(shape, extrudeSettings);
    // Drehung: Im 2D-Layout entspricht Y der Tiefe Z im 3D-Raum
    geometry.rotateX(Math.PI / 2);
    // Tischoberkante liegt exakt auf Y = 0 mm (Ausgleich der BevelThickness von 2.5 mm)
    geometry.translate(0, -bevelThickness, 0);

    const woodColor = this.getStyleColor(config.style);
    const material = new THREE.MeshStandardMaterial({
      color: woodColor,
      roughness: 0.65,
      metalness: 0.05,
    });

    const tableMesh = new THREE.Mesh(geometry, material);
    tableMesh.receiveShadow = true;
    tableMesh.castShadow = true;
    this.tableGroup.add(tableMesh);

    // Tischbeine (Vierkant- oder Rundbeine) an markanten Eckpunkten nach unten
    // Wichtig: Beine sind eingerückt, damit sie niemals über die Tischkanten / Tischecken hinausragen!
    const legRadius = 18;
    const legGeo = new THREE.CylinderGeometry(legRadius, legRadius, 720, 16);
    const legMat = new THREE.MeshStandardMaterial({ color: 0x27272a, roughness: 0.4, metalness: 0.7 });

    const legPositions = computeBaseboardLegPositions(config, legRadius, 45);
    for (const pt of legPositions) {
      const leg = new THREE.Mesh(legGeo, legMat);
      leg.position.set(pt.x, -360 - 35 - bevelThickness, pt.y);
      leg.castShadow = true;
      leg.receiveShadow = true;
      this.tableGroup.add(leg);
    }
  }

  private applyCurveElevation(
    geo: THREE.BufferGeometry,
    r: number,
    rad: number,
    heightDiff: number,
    dir: 1 | -1 = 1,
    leadX = 0,
  ) {
    if (Math.abs(heightDiff) < 0.01) return;
    const pos = geo.attributes.position;
    if (!pos) return;
    const centerZ = dir * r;
    for (let i = 0; i < pos.count; i++) {
      const vx = pos.getX(i) - leadX;
      const vy = pos.getY(i);
      const vz = pos.getZ(i);
      const theta =
        dir === 1
          ? Math.atan2(vz - centerZ, vx) + Math.PI / 2
          : Math.PI / 2 - Math.atan2(vz - centerZ, vx);
      const u = Math.max(0, Math.min(1, theta / rad));
      pos.setY(i, vy + u * heightDiff);
    }
    geo.computeVertexNormals();
  }

  private buildTrackPiece3D(
    piece: TrackPiece,
    entry: CatalogEntry,
    startElev: number,
    endElev: number,
    colorMode: ColorMode,
    layerColor?: string,
  ) {
    const group = new THREE.Group();
    // 2D-Weltkoordinaten: (x, y) -> 3D (x, elevation, z=y)
    // Pivot-Punkt liegt am Start-Konnektor (x=0) auf exakt startElev
    group.position.set(piece.transform.x, startElev, piece.transform.y);
    group.rotation.y = -deg2rad(piece.transform.rotation);

    if (entry.isRoad) {
      this.buildRoadPiece3D(group, piece, entry, endElev - startElev, colorMode, layerColor);
      this.trackGroup.add(group);
      return;
    }

    const bpColor = getTrackBlueprintColor(entry.id);
    let ballastColor = 0x78716c;
    if (colorMode === "types" && bpColor) {
      ballastColor = bpColor.hex;
    } else if (colorMode === "layers") {
      const hexStr = layerColor || "#38bdf8";
      const parsedHex = parseInt(hexStr.replace(/^#/, ""), 16);
      ballastColor = isNaN(parsedHex) ? 0x38bdf8 : parsedHex;
    }

    const kind = entry.kind;
    if (kind === "turntable") {
      this.buildTurntable3D(group, piece, entry, ballastColor);
      this.trackGroup.add(group);
      return;
    }

    const gauge = entry.gauge ?? 16.5;
    const gaugeHalf = gauge / 2;
    const hasRoadbed = entry.hasRoadbed !== false;
    const ballastH = hasRoadbed ? 6 : 1.5;
    const ballastW = hasRoadbed ? (gauge / 16.5) * 36 : (gauge / 16.5) * 26;
    const railH = 2.2;
    const railW = 1.2;
    const railY = ballastH + railH;
    const heightDiff = endElev - startElev;

    // Gemeinsame Materialien
    const ballastMat = new THREE.MeshStandardMaterial({ color: ballastColor, roughness: 0.9 });
    const tieMat = new THREE.MeshStandardMaterial({ color: 0x3e2723, roughness: 0.85 });
    const railMat = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25 });
    const steelMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5 });
    const bufferMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4 });
    const signalRedMat = new THREE.MeshStandardMaterial({ color: 0xdc2626, roughness: 0.4 });

    // --- HELFER: Gerades Gleisstück mit Schotterbett, Schwellen & Schienen ---
    const addStraightSection = (
      targetGroup: THREE.Group,
      len: number,
      startX = 0,
      isAccessory = false,
    ) => {
      // 1. Schotterbett
      const ballastGeo = new THREE.BoxGeometry(len, ballastH, ballastW);
      const ballastMesh = new THREE.Mesh(ballastGeo, ballastMat);
      ballastMesh.position.set(startX + len / 2, ballastH / 2, 0);
      ballastMesh.receiveShadow = true;
      ballastMesh.castShadow = true;
      targetGroup.add(ballastMesh);

      // 2. Schwellen
      const tieGeo = new THREE.BoxGeometry(2.6, 2.2, gauge * 1.55);
      const tieCount = Math.floor(len / 6.5);
      for (let i = 0; i < tieCount; i++) {
        const tie = new THREE.Mesh(tieGeo, tieMat);
        tie.position.set(startX + i * 6.5 + 3.2, ballastH + 1.1, 0);
        tie.castShadow = true;
        targetGroup.add(tie);
      }

      // 3. Schienenstränge (links & rechts)
      const railGeo = new THREE.BoxGeometry(len, railH, railW);
      const rL = new THREE.Mesh(railGeo, railMat);
      rL.position.set(startX + len / 2, railY, -gaugeHalf);
      const rR = new THREE.Mesh(railGeo, railMat);
      rR.position.set(startX + len / 2, railY, gaugeHalf);
      targetGroup.add(rL);
      targetGroup.add(rR);

      // 4. Prellbock-Gleis (Accessory / Prellbock am Gleisende)
      if (isAccessory) {
        const bufferGroup = new THREE.Group();
        const endX = startX + len;

        // Diagonale Stahlträger von Schienen aufwärts
        const strutGeo = new THREE.BoxGeometry(1.6, 1.6, 12);
        const strutL = new THREE.Mesh(strutGeo, steelMat);
        strutL.position.set(endX - 3.5, ballastH + 4.5, -gaugeHalf);
        strutL.rotation.y = Math.PI / 2;
        strutL.rotation.z = Math.PI / 4;
        bufferGroup.add(strutL);

        const strutR = new THREE.Mesh(strutGeo, steelMat);
        strutR.position.set(endX - 3.5, ballastH + 4.5, gaugeHalf);
        strutR.rotation.y = Math.PI / 2;
        strutR.rotation.z = Math.PI / 4;
        bufferGroup.add(strutR);

        // Pufferbohle (Holz/Stahl quer über Gleis)
        const bohleGeo = new THREE.BoxGeometry(4, 7, gauge * 1.6);
        const bohle = new THREE.Mesh(bohleGeo, steelMat);
        bohle.position.set(endX - 3.5, ballastH + 6.5, 0);
        bufferGroup.add(bohle);

        // 2 Pufferteller
        const buffGeo = new THREE.CylinderGeometry(2.4, 2.4, 1.5, 12);
        for (const s of [-1, 1]) {
          const buff = new THREE.Mesh(buffGeo, bufferMat);
          buff.position.set(endX - 0.7, ballastH + 6.5, s * gaugeHalf * 0.75);
          buff.rotation.z = Math.PI / 2;
          bufferGroup.add(buff);
        }

        // Sh0 Halt-Signal (Rotes Signal-Rechteck)
        const sh0Geo = new THREE.BoxGeometry(1.2, 4, 4);
        const sh0 = new THREE.Mesh(sh0Geo, signalRedMat);
        sh0.position.set(endX - 5.6, ballastH + 6.5, 0);
        bufferGroup.add(sh0);

        targetGroup.add(bufferGroup);
      }
    };

    // --- HELFER: Gebogenes Gleisstück mit Schotterbett, Schwellen & Schienen ---
    const addCurvedSection = (
      targetGroup: THREE.Group,
      r: number,
      a: number,
      dir: 1 | -1 = 1,
      leadX = 0,
      customHDiff = heightDiff,
    ) => {
      const rad = deg2rad(a);
      const rIn = r - ballastW / 2;
      const rOut = r + ballastW / 2;

      // 1. Schotterbett
      const curveShape = new THREE.Shape();
      if (dir === 1) {
        curveShape.absarc(0, 0, rOut, -Math.PI / 2, -Math.PI / 2 + rad, false);
        curveShape.absarc(0, 0, rIn, -Math.PI / 2 + rad, -Math.PI / 2, true);
      } else {
        curveShape.absarc(0, 0, rOut, Math.PI / 2, Math.PI / 2 - rad, true);
        curveShape.absarc(0, 0, rIn, Math.PI / 2 - rad, Math.PI / 2, false);
      }
      curveShape.closePath();

      const curveGeo = new THREE.ExtrudeGeometry(curveShape, { depth: ballastH, bevelEnabled: false });
      curveGeo.rotateX(Math.PI / 2);
      curveGeo.translate(leadX, ballastH, dir * r);
      this.applyCurveElevation(curveGeo, r, rad, customHDiff, dir, leadX);

      const ballastMesh = new THREE.Mesh(curveGeo, ballastMat);
      ballastMesh.receiveShadow = true;
      ballastMesh.castShadow = true;
      targetGroup.add(ballastMesh);

      // 2. Schienenstränge im Bogen
      for (const offset of [-gaugeHalf, gaugeHalf]) {
        const railShape = new THREE.Shape();
        const rr = r + offset;
        if (dir === 1) {
          railShape.absarc(0, 0, rr + railW / 2, -Math.PI / 2, -Math.PI / 2 + rad, false);
          railShape.absarc(0, 0, rr - railW / 2, -Math.PI / 2 + rad, -Math.PI / 2, true);
        } else {
          railShape.absarc(0, 0, rr + railW / 2, Math.PI / 2, Math.PI / 2 - rad, true);
          railShape.absarc(0, 0, rr - railW / 2, Math.PI / 2 - rad, Math.PI / 2, false);
        }
        railShape.closePath();

        const bRailGeo = new THREE.ExtrudeGeometry(railShape, { depth: railH, bevelEnabled: false });
        bRailGeo.rotateX(Math.PI / 2);
        bRailGeo.translate(leadX, railY, dir * r);
        this.applyCurveElevation(bRailGeo, r, rad, customHDiff, dir, leadX);

        const bRailMesh = new THREE.Mesh(bRailGeo, railMat);
        targetGroup.add(bRailMesh);
      }

      // 3. Schwellen entlang des Bogens
      const arcLen = r * rad;
      const tieCount = Math.max(3, Math.floor(arcLen / 6.5));
      const tieGeo = new THREE.BoxGeometry(2.6, 2.2, gauge * 1.55);
      for (let i = 0; i < tieCount; i++) {
        const u = (i + 0.5) / tieCount;
        const phi = u * rad;
        const tx = leadX + r * Math.sin(phi);
        const tz = dir * r * (1 - Math.cos(phi));
        const ty = ballastH + 1.1 + u * customHDiff;

        const tieMesh = new THREE.Mesh(tieGeo, tieMat);
        tieMesh.position.set(tx, ty, tz);
        tieMesh.rotation.y = -dir * phi;
        tieMesh.castShadow = true;
        targetGroup.add(tieMesh);
      }
    };

    // Prüfung auf Bogenweiche (auch wenn im Katalog als turnout-left/right deklariert)
    const isCurvedTurnout =
      kind === "curved-turnout-left" ||
      kind === "curved-turnout-right" ||
      ((kind === "turnout-left" || kind === "turnout-right") && !!entry.radius && !entry.length);

    // =========================================================================
    // 1. GERADES GLEIS & ZUBEHÖR (PRELLBOCK)
    // =========================================================================
    if (kind === "straight" || kind === "accessory") {
      const len = entry.length ?? 188.3;
      const straightGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        straightGroup.rotation.z = Math.atan2(heightDiff, len);
      }
      addStraightSection(straightGroup, len, 0, kind === "accessory");
      group.add(straightGroup);
    }
    // =========================================================================
    // 2. GEBOGENES GLEIS (Helix / Kurven-Rampe)
    // =========================================================================
    else if (kind === "curve") {
      const r = entry.radius ?? 437.5;
      const a = entry.angle ?? 30;
      addCurvedSection(group, r, a, 1, 0, heightDiff);
    }
    // =========================================================================
    // 3. BOGENWEICHE (Curved Turnout Left / Right)
    // =========================================================================
    else if (isCurvedTurnout) {
      const dir: 1 | -1 = kind === "curved-turnout-left" || kind === "turnout-left" ? 1 : -1;
      const r1 = entry.radius ?? 360;
      const a1 = entry.angle ?? 30;
      const lead =
        entry.branchLength ??
        (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 =
        entry.branchLength !== undefined
          ? (entry.branchRadius ?? r1)
          : entry.branchRadius && entry.branchRadius > r1
            ? r1
            : (entry.branchRadius ?? r1);
      const a2 = entry.branchAngle ?? 30;

      const cGroup = new THREE.Group();
      // Stammbogen
      addCurvedSection(cGroup, r1, a1, dir, 0, heightDiff);
      // Abzweigbogen mit eventuellem Vorlaufgleis
      if (lead > 0) {
        addStraightSection(cGroup, lead, 0, false);
      }
      addCurvedSection(cGroup, r2, a2, dir, lead, heightDiff);
      group.add(cGroup);
    }
    // =========================================================================
    // 4. STANDARD-WEICHE (Turnout Left / Right mit geradem Stammgleis)
    // =========================================================================
    else if (kind === "turnout-left" || kind === "turnout-right") {
      const len = entry.length ?? 188.3;
      const br = entry.branchRadius ?? 437.5;
      const ba = entry.branchAngle ?? 24.3;
      const dir: 1 | -1 = kind === "turnout-left" ? 1 : -1;

      const turnoutGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        turnoutGroup.rotation.z = Math.atan2(heightDiff, len);
      }

      // Stammgleis (gerade)
      addStraightSection(turnoutGroup, len, 0, false);
      // Abzweigbogen
      addCurvedSection(turnoutGroup, br, ba, dir, 0, heightDiff);
      group.add(turnoutGroup);
    }
    // =========================================================================
    // 5. DREIWEG-WEICHE (Three-Way Turnout)
    // =========================================================================
    else if (kind === "three-way") {
      const len = entry.length ?? 188.3;
      const br = entry.branchRadius ?? 437.5;
      const ba = entry.branchAngle ?? 24.3;

      const threeWayGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        threeWayGroup.rotation.z = Math.atan2(heightDiff, len);
      }

      // Stammgleis (gerade)
      addStraightSection(threeWayGroup, len, 0, false);
      // Abzweig links
      addCurvedSection(threeWayGroup, br, ba, 1, 0, heightDiff);
      // Abzweig rechts
      addCurvedSection(threeWayGroup, br, ba, -1, 0, heightDiff);
      group.add(threeWayGroup);
    }
    // =========================================================================
    // 6. KREUZUNG (Crossing)
    // =========================================================================
    else if (kind === "crossing") {
      const len = entry.length ?? 188.3;
      const a = entry.angle ?? 24.3;
      const rad = deg2rad(a);
      const h = len / 2;

      const crossingGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        crossingGroup.rotation.z = Math.atan2(heightDiff, len);
      }

      // Strang 1 (horizontal von -h bis +h, zentriert um 0,0)
      addStraightSection(crossingGroup, len, -h, false);

      // Strang 2 (gedreht um Kreuzungswinkel rad)
      const track2Group = new THREE.Group();
      track2Group.rotation.y = -rad;
      addStraightSection(track2Group, len, -h, false);
      crossingGroup.add(track2Group);

      group.add(crossingGroup);
    }
    // =========================================================================
    // 7. DOPPELKREUZUNGSWEICHE (DKW / Double Slip Switch)
    // =========================================================================
    else if (kind === "dkw") {
      const len = entry.length ?? 188.3;
      const a = entry.angle ?? 24.3;
      const rad = deg2rad(a);
      const h = len / 2;
      const cx = h;

      const dkwGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        dkwGroup.rotation.z = Math.atan2(heightDiff, len);
      }

      // Strang 1: Gerade von 0 bis len
      addStraightSection(dkwGroup, len, 0, false);

      // Strang 2: Gekreuzter Strang um cx zentriert
      const track2Group = new THREE.Group();
      track2Group.position.set(cx, 0, 0);
      track2Group.rotation.y = -rad;
      addStraightSection(track2Group, len, -h, false);
      dkwGroup.add(track2Group);

      // 4 Verbindungsschienen (Bogenverbindungen des doppelten Gleiswechsels)
      const br = entry.branchRadius ?? 437.5;
      const ba = entry.branchAngle ?? a;
      addCurvedSection(dkwGroup, br, ba, 1, 0, heightDiff);
      addCurvedSection(dkwGroup, br, ba, -1, 0, heightDiff);

      group.add(dkwGroup);
    }
    // =========================================================================
    // 8. FALLBACK FÜR UNBEKANNTE FORMATE
    // =========================================================================
    else {
      const len = entry.length ?? 188.3;
      const fallbackGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        fallbackGroup.rotation.z = Math.atan2(heightDiff, len);
      }
      addStraightSection(fallbackGroup, len, 0, false);
      group.add(fallbackGroup);
    }

    // 4B. OBERLEITUNG IN 3D (Fahrdraht, Tragseil, Hänger & Streckenmasten)
    if (piece.hasCatenary) {
      this.buildCatenary3D(group, entry, startElev, endElev, piece);
    }

    // 4C. WEICHEN-ZUBEHÖR IN 3D (Laterne & Decoder)
    if (piece.hasLantern || piece.hasDecoder) {
      this.buildTurnoutAccessories3D(group, entry, piece);
    }

    this.trackGroup.add(group);
  }

  private buildRoadPiece3D(
    group: THREE.Group,
    _piece: TrackPiece,
    entry: CatalogEntry,
    heightDiff: number,
    colorMode: ColorMode,
    layerColor?: string,
  ) {
    const roadW = entry.roadWidth ?? 60;
    const roadH = 2.4;
    const roadBaseY = 0.08;
    const kind = entry.kind;

    let asphaltHex = 0x334155;
    if (colorMode === "layers" && layerColor) {
      const parsed = parseInt(layerColor.replace(/^#/, ""), 16);
      if (!isNaN(parsed)) asphaltHex = parsed;
    }

    const asphaltMat = new THREE.MeshStandardMaterial({ color: asphaltHex, roughness: 0.92, side: THREE.DoubleSide });
    const lineMat = new THREE.MeshStandardMaterial({ color: 0xf8fafc, roughness: 0.4, side: THREE.DoubleSide });
    const yellowMat = new THREE.MeshStandardMaterial({ color: 0xfbbf24, roughness: 0.4, side: THREE.DoubleSide });
    const plankMat = new THREE.MeshStandardMaterial({ color: 0x272422, roughness: 0.85, side: THREE.DoubleSide });
    const railMat = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25, side: THREE.DoubleSide });

    const addStraightRoad = (target: THREE.Group, len: number, startX = 0, customZ = 0) => {
      const roadGeo = new THREE.BoxGeometry(len, roadH, roadW);
      const roadMesh = new THREE.Mesh(roadGeo, asphaltMat);
      roadMesh.position.set(startX + len / 2, roadBaseY + roadH / 2, customZ);
      roadMesh.receiveShadow = true;
      roadMesh.castShadow = true;
      target.add(roadMesh);

      const edgeLineGeo = new THREE.BoxGeometry(len, 0.12, 1.4);
      const edgeZ1 = customZ - roadW / 2 + 2.5;
      const edgeZ2 = customZ + roadW / 2 - 2.5;

      const edgeL = new THREE.Mesh(edgeLineGeo, lineMat);
      edgeL.position.set(startX + len / 2, roadBaseY + roadH + 0.06, edgeZ1);
      target.add(edgeL);

      const edgeR = new THREE.Mesh(edgeLineGeo, lineMat);
      edgeR.position.set(startX + len / 2, roadBaseY + roadH + 0.06, edgeZ2);
      target.add(edgeR);

      const dashGeo = new THREE.BoxGeometry(8, 0.12, 1.4);
      const period = 14;
      for (let x = startX + 4; x + 8 <= startX + len - 2; x += period) {
        const dash = new THREE.Mesh(dashGeo, lineMat);
        dash.position.set(x + 4, roadBaseY + roadH + 0.06, customZ);
        target.add(dash);
      }
    };

    const addCurvedRoad = (
      target: THREE.Group,
      r: number,
      a: number,
      dir: 1 | -1 = 1,
      customHDiff = heightDiff,
    ) => {
      const rad = deg2rad(a);
      const rIn = r - roadW / 2;
      const rOut = r + roadW / 2;

      const curveShape = new THREE.Shape();
      if (dir === 1) {
        curveShape.absarc(0, 0, rOut, -Math.PI / 2, -Math.PI / 2 + rad, false);
        curveShape.absarc(0, 0, rIn, -Math.PI / 2 + rad, -Math.PI / 2, true);
      } else {
        curveShape.absarc(0, 0, rOut, Math.PI / 2, Math.PI / 2 - rad, true);
        curveShape.absarc(0, 0, rIn, Math.PI / 2 - rad, Math.PI / 2, false);
      }
      curveShape.closePath();

      const curveGeo = new THREE.ExtrudeGeometry(curveShape, { depth: roadH, bevelEnabled: false });
      curveGeo.rotateX(Math.PI / 2);
      curveGeo.translate(0, roadBaseY + roadH, dir * r);
      this.applyCurveElevation(curveGeo, r, rad, customHDiff, dir, 0);

      const roadMesh = new THREE.Mesh(curveGeo, asphaltMat);
      roadMesh.receiveShadow = true;
      roadMesh.castShadow = true;
      target.add(roadMesh);

      for (const off of [-roadW / 2 + 2.5, roadW / 2 - 2.5]) {
        const lineShape = new THREE.Shape();
        const rr = r + off;
        const lineW = 1.4;
        if (dir === 1) {
          lineShape.absarc(0, 0, rr + lineW / 2, -Math.PI / 2, -Math.PI / 2 + rad, false);
          lineShape.absarc(0, 0, rr - lineW / 2, -Math.PI / 2 + rad, -Math.PI / 2, true);
        } else {
          lineShape.absarc(0, 0, rr + lineW / 2, Math.PI / 2, Math.PI / 2 - rad, true);
          lineShape.absarc(0, 0, rr - lineW / 2, -Math.PI / 2, Math.PI / 2, false);
        }
        lineShape.closePath();

        const lineGeo = new THREE.ExtrudeGeometry(lineShape, { depth: 0.12, bevelEnabled: false });
        lineGeo.rotateX(Math.PI / 2);
        lineGeo.translate(0, roadBaseY + roadH + 0.06, dir * r);
        this.applyCurveElevation(lineGeo, r, rad, customHDiff, dir, 0);
        target.add(new THREE.Mesh(lineGeo, lineMat));
      }

      const totalArc = rad * r;
      const numDashes = Math.max(1, Math.floor(totalArc / 14));
      const angStep = rad / numDashes;
      const dashBoxGeo = new THREE.BoxGeometry(8, 0.12, 1.4);

      for (let i = 0; i < numDashes; i++) {
        const phi = (i + 0.5) * angStep;
        const tx = r * Math.sin(phi);
        const tz = dir * r * (1 - Math.cos(phi));
        const u = phi / rad;
        const ty = roadBaseY + roadH + 0.06 + u * customHDiff;

        const dash = new THREE.Mesh(dashBoxGeo, lineMat);
        dash.position.set(tx, ty, tz);
        dash.rotation.y = -dir * phi;
        target.add(dash);
      }
    };

    if (kind === "straight" || kind === "accessory") {
      const len = entry.length ?? 200;
      const straightGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        straightGroup.rotation.z = Math.atan2(heightDiff, len);
      }

      if (entry.id === "ST-LOOP") {
        const neckL = 35;
        const loopCenterX = 55;
        const loopRadius = 42;

        const neckGeo = new THREE.BoxGeometry(neckL, roadH, roadW);
        const neck = new THREE.Mesh(neckGeo, asphaltMat);
        neck.position.set(neckL / 2, roadBaseY + roadH / 2, 0);
        neck.receiveShadow = true;
        straightGroup.add(neck);

        const circleGeo = new THREE.CylinderGeometry(loopRadius, loopRadius, roadH, 32);
        const circleMesh = new THREE.Mesh(circleGeo, asphaltMat);
        circleMesh.position.set(loopCenterX, roadBaseY + roadH / 2, 0);
        circleMesh.receiveShadow = true;
        straightGroup.add(circleMesh);

        const islandGeo = new THREE.CylinderGeometry(14, 14, roadH + 0.4, 24);
        const islandMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.9, side: THREE.DoubleSide });
        const islandMesh = new THREE.Mesh(islandGeo, islandMat);
        islandMesh.position.set(loopCenterX, roadBaseY + (roadH + 0.4) / 2, 0);
        straightGroup.add(islandMesh);

        const edgeLineGeo = new THREE.BoxGeometry(neckL, 0.12, 1.4);
        const eL = new THREE.Mesh(edgeLineGeo, lineMat);
        eL.position.set(neckL / 2, roadBaseY + roadH + 0.06, -roadW / 2 + 2.5);
        straightGroup.add(eL);
        const eR = new THREE.Mesh(edgeLineGeo, lineMat);
        eR.position.set(neckL / 2, roadBaseY + roadH + 0.06, roadW / 2 - 2.5);
        straightGroup.add(eR);

        group.add(straightGroup);
        return;
      }

      if (entry.id === "ST-BUS") {
        addStraightRoad(straightGroup, len);
        const bayGeo = new THREE.BoxGeometry(130, roadH, 15);
        const bayMesh = new THREE.Mesh(bayGeo, asphaltMat);
        bayMesh.position.set(100, roadBaseY + roadH / 2, roadW / 2 + 7.5);
        bayMesh.receiveShadow = true;
        straightGroup.add(bayMesh);

        const yDashGeo = new THREE.BoxGeometry(6, 0.12, 1.4);
        for (let bx = 38; bx + 6 <= 162; bx += 10) {
          const yd = new THREE.Mesh(yDashGeo, yellowMat);
          yd.position.set(bx + 3, roadBaseY + roadH + 0.06, roadW / 2);
          straightGroup.add(yd);
        }
        group.add(straightGroup);
        return;
      }

      if (entry.id === "ST-CROSS") {
        addStraightRoad(straightGroup, len);

        const plankGeo = new THREE.BoxGeometry(36, roadH + 0.3, roadW + 2);
        const plankMesh = new THREE.Mesh(plankGeo, plankMat);
        plankMesh.position.set(100, roadBaseY + (roadH + 0.3) / 2, 0);
        straightGroup.add(plankMesh);

        const railCrossGeo = new THREE.BoxGeometry(1.2, 2.0, roadW + 4);
        const rL = new THREE.Mesh(railCrossGeo, railMat);
        rL.position.set(100 - 8.25, roadBaseY + roadH + 1.0, 0);
        straightGroup.add(rL);
        const rR = new THREE.Mesh(railCrossGeo, railMat);
        rR.position.set(100 + 8.25, roadBaseY + roadH + 1.0, 0);
        straightGroup.add(rR);

        const stopGeo = new THREE.BoxGeometry(2.5, 0.12, roadW - 4);
        const sL = new THREE.Mesh(stopGeo, lineMat);
        sL.position.set(78, roadBaseY + roadH + 0.06, 0);
        straightGroup.add(sL);
        const sR = new THREE.Mesh(stopGeo, lineMat);
        sR.position.set(122, roadBaseY + roadH + 0.06, 0);
        straightGroup.add(sR);

        group.add(straightGroup);
        return;
      }

      addStraightRoad(straightGroup, len);
      group.add(straightGroup);
      return;
    }

    if (kind === "curve") {
      const r = entry.radius ?? 200;
      const a = entry.angle ?? 90;
      addCurvedRoad(group, r, a, 1, heightDiff);
      return;
    }

    if (kind === "turnout-left" || kind === "turnout-right") {
      const len = entry.length ?? 200;
      const turnGroup = new THREE.Group();
      if (Math.abs(heightDiff) > 0.01) {
        turnGroup.rotation.z = Math.atan2(heightDiff, len);
      }
      addStraightRoad(turnGroup, len);

      const isRight = kind === "turnout-right";
      const branchSign = isRight ? -1 : 1;

      if (entry.branchLength !== undefined) {
        const bLen = entry.branchLength;
        const branchGeo = new THREE.BoxGeometry(roadW, roadH, bLen);
        const branchMesh = new THREE.Mesh(branchGeo, asphaltMat);
        branchMesh.position.set(len / 2, roadBaseY + roadH / 2, branchSign * (bLen / 2));
        branchMesh.receiveShadow = true;
        turnGroup.add(branchMesh);

        const edgeZGeo = new THREE.BoxGeometry(1.4, 0.12, bLen);
        const ezL = new THREE.Mesh(edgeZGeo, lineMat);
        ezL.position.set(len / 2 - roadW / 2 + 2.5, roadBaseY + roadH + 0.06, branchSign * (bLen / 2));
        turnGroup.add(ezL);
        const ezR = new THREE.Mesh(edgeZGeo, lineMat);
        ezR.position.set(len / 2 + roadW / 2 - 2.5, roadBaseY + roadH + 0.06, branchSign * (bLen / 2));
        turnGroup.add(ezR);

        const bDashGeo = new THREE.BoxGeometry(1.4, 0.12, 8);
        for (let z = 14; z + 8 <= bLen - 4; z += 14) {
          const bd = new THREE.Mesh(bDashGeo, lineMat);
          bd.position.set(len / 2, roadBaseY + roadH + 0.06, branchSign * (z + 4));
          turnGroup.add(bd);
        }
        group.add(turnGroup);
        return;
      }

      if (entry.branchRadius !== undefined) {
        const br = entry.branchRadius;
        const ba = entry.branchAngle ?? 45;
        addCurvedRoad(turnGroup, br, ba, branchSign, heightDiff);
        group.add(turnGroup);
        return;
      }
      group.add(turnGroup);
      return;
    }

    if (kind === "crossing") {
      const len = entry.length ?? 200;
      const crossGroup = new THREE.Group();
      addStraightRoad(crossGroup, len);

      const crossGeo = new THREE.BoxGeometry(roadW, roadH, len);
      const crossMesh = new THREE.Mesh(crossGeo, asphaltMat);
      crossMesh.position.set(len / 2, roadBaseY + roadH / 2, 0);
      crossMesh.receiveShadow = true;
      crossGroup.add(crossMesh);

      const crossEdgeGeo = new THREE.BoxGeometry(1.4, 0.12, (len - roadW) / 2);
      const ce1 = new THREE.Mesh(crossEdgeGeo, lineMat);
      ce1.position.set(len / 2 - roadW / 2 + 2.5, roadBaseY + roadH + 0.06, -len / 4 - roadW / 4);
      crossGroup.add(ce1);
      const ce2 = new THREE.Mesh(crossEdgeGeo, lineMat);
      ce2.position.set(len / 2 + roadW / 2 - 2.5, roadBaseY + roadH + 0.06, -len / 4 - roadW / 4);
      crossGroup.add(ce2);
      const ce3 = new THREE.Mesh(crossEdgeGeo, lineMat);
      ce3.position.set(len / 2 - roadW / 2 + 2.5, roadBaseY + roadH + 0.06, len / 4 + roadW / 4);
      crossGroup.add(ce3);
      const ce4 = new THREE.Mesh(crossEdgeGeo, lineMat);
      ce4.position.set(len / 2 + roadW / 2 - 2.5, roadBaseY + roadH + 0.06, len / 4 + roadW / 4);
      crossGroup.add(ce4);

      const crossDashGeo = new THREE.BoxGeometry(1.4, 0.12, 8);
      for (let z = -len / 2 + 4; z + 8 <= -roadW / 2 - 2; z += 14) {
        const cd = new THREE.Mesh(crossDashGeo, lineMat);
        cd.position.set(len / 2, roadBaseY + roadH + 0.06, z + 4);
        crossGroup.add(cd);
      }
      for (let z = roadW / 2 + 4; z + 8 <= len / 2 - 2; z += 14) {
        const cd = new THREE.Mesh(crossDashGeo, lineMat);
        cd.position.set(len / 2, roadBaseY + roadH + 0.06, z + 4);
        crossGroup.add(cd);
      }
      group.add(crossGroup);
      return;
    }

    // Fallback: Gerades Standard-Straßenstück
    const fallbackLen = entry.length ?? 200;
    const fallbackGroup = new THREE.Group();
    addStraightRoad(fallbackGroup, fallbackLen);
    group.add(fallbackGroup);
  }

  /**
   * Erzeugt ein Trassenbrett (Wood Plate) unter Gleisen oder Straßen für realistische Modelltisch-Unterbauten.
   */
  private buildWoodPlatePiece3D(
    group: THREE.Group,
    _piece: TrackPiece,
    entry: CatalogEntry,
    heightDiff: number,
  ) {
    const isRoad = entry.isRoad === true || entry.id.startsWith("ST-");
    const roadW = entry.roadWidth ?? 60;
    const plateWidth = isRoad ? roadW + 8 : (entry.gauge ?? 16.5) * 2.4;
    const thickness = 6;
    const woodPlateMat = new THREE.MeshStandardMaterial({
      color: 0xcbb89d,
      roughness: 0.85,
      metalness: 0.05,
    });

    if (entry.kind === "straight" || entry.kind === "accessory") {
      const len = entry.length ?? (isRoad ? 200 : 180);
      const geo = new THREE.BoxGeometry(len, thickness, plateWidth);
      const mesh = new THREE.Mesh(geo, woodPlateMat);
      mesh.position.set(len / 2, -thickness / 2, 0);
      if (Math.abs(heightDiff) > 0.01) {
        mesh.rotation.z = Math.atan2(heightDiff, len);
      }
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      group.add(mesh);
    } else if (entry.kind === "curve") {
      const r = entry.radius ?? 360;
      const deg = entry.angle ?? 30;
      const rad = deg2rad(deg);
      const innerR = Math.max(10, r - plateWidth / 2);
      const outerR = r + plateWidth / 2;
      const segments = Math.max(8, Math.round(deg / 3));

      const dir: 1 | -1 = 1;
      const curveShape = new THREE.Shape();
      curveShape.absarc(0, 0, outerR, -Math.PI / 2, -Math.PI / 2 + rad, false);
      curveShape.absarc(0, 0, innerR, -Math.PI / 2 + rad, -Math.PI / 2, true);
      curveShape.closePath();

      const extrudeSettings = { depth: thickness, bevelEnabled: false, curveSegments: segments };
      const geo = new THREE.ExtrudeGeometry(curveShape, extrudeSettings);
      geo.rotateX(Math.PI / 2);
      geo.translate(0, 0, dir * r);
      if (Math.abs(heightDiff) > 0.01) {
        this.applyCurveElevation(geo, r, rad, heightDiff, dir, 0);
      }
      const mesh = new THREE.Mesh(geo, woodPlateMat);
      mesh.position.set(0, -thickness, 0);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      group.add(mesh);
    } else if (entry.kind === "turntable") {
      const outerR = entry.radius ?? 189;
      const geo = new THREE.CylinderGeometry(outerR, outerR, thickness, 32);
      const mesh = new THREE.Mesh(geo, woodPlateMat);
      mesh.position.set(0, -thickness / 2, 0);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      group.add(mesh);
    } else if (entry.kind?.includes("turnout") || entry.kind === "crossing") {
      const len = entry.length ?? 188.3;
      const mainGeo = new THREE.BoxGeometry(len, thickness, plateWidth);
      const mainMesh = new THREE.Mesh(mainGeo, woodPlateMat);
      mainMesh.position.set(len / 2, -thickness / 2, 0);
      mainMesh.castShadow = true;
      mainMesh.receiveShadow = true;
      group.add(mainMesh);

      if (entry.branchRadius && entry.branchAngle) {
        const brR = entry.branchRadius;
        const brDeg = entry.branchAngle;
        const brRad = deg2rad(brDeg);
        const dir: 1 | -1 = entry.kind.includes("left") ? -1 : 1;
        const innerR = Math.max(10, brR - plateWidth / 2);
        const outerR = brR + plateWidth / 2;
        const shape = new THREE.Shape();
        if (dir === 1) {
          shape.absarc(0, 0, outerR, -Math.PI / 2, -Math.PI / 2 + brRad, false);
          shape.absarc(0, 0, innerR, -Math.PI / 2 + brRad, -Math.PI / 2, true);
        } else {
          shape.absarc(0, 0, outerR, Math.PI / 2, Math.PI / 2 - brRad, true);
          shape.absarc(0, 0, innerR, Math.PI / 2 - brRad, Math.PI / 2, false);
        }
        shape.closePath();
        const geo = new THREE.ExtrudeGeometry(shape, { depth: thickness, bevelEnabled: false, curveSegments: 8 });
        geo.rotateX(Math.PI / 2);
        geo.translate(0, 0, dir * brR);
        const mesh = new THREE.Mesh(geo, woodPlateMat);
        mesh.position.set(0, -thickness, 0);
        mesh.castShadow = true;
        mesh.receiveShadow = true;
        group.add(mesh);
      } else if (entry.branchLength) {
        const bLen = entry.branchLength;
        const bDeg = entry.branchAngle ?? 90;
        const bGeo = new THREE.BoxGeometry(bLen, thickness, plateWidth);
        const bMesh = new THREE.Mesh(bGeo, woodPlateMat);
        bMesh.rotation.y = -deg2rad(bDeg);
        bMesh.position.set(len / 2, -thickness / 2, bLen / 2);
        bMesh.castShadow = true;
        bMesh.receiveShadow = true;
        group.add(bMesh);
      }
    } else {
      const len = entry.length ?? 180;
      const geo = new THREE.BoxGeometry(len, thickness, plateWidth);
      const mesh = new THREE.Mesh(geo, woodPlateMat);
      mesh.position.set(len / 2, -thickness / 2, 0);
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      group.add(mesh);
    }
  }

  private buildBridgePillar(x: number, z: number, height: number) {
    const pillarGeo = new THREE.CylinderGeometry(10, 13, height, 12);
    const pillarMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.75, metalness: 0.1 });
    const pillar = new THREE.Mesh(pillarGeo, pillarMat);
    pillar.position.set(x, height / 2, z);
    pillar.castShadow = true;
    pillar.receiveShadow = true;
    this.pillarsGroup.add(pillar);
  }

  private buildTurnoutAccessories3D(group: THREE.Group, entry: CatalogEntry, piece: TrackPiece) {
    const kind = entry.kind;
    const isLeft = kind === "turnout-left" || kind === "curved-turnout-left";
    const lanternZ = isLeft ? -22 : 22;
    const lanternX = 26;

    const accGroup = new THREE.Group();

    // 1. Stellstange (metallisch)
    const rodGeo = new THREE.CylinderGeometry(0.8, 0.8, Math.abs(lanternZ), 6);
    const metalMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.5, metalness: 0.8 });
    const rod = new THREE.Mesh(rodGeo, metalMat);
    rod.rotation.x = Math.PI / 2;
    rod.position.set(lanternX, 3.5, lanternZ / 2);
    accGroup.add(rod);

    if (piece.hasLantern) {
      // 2. Sockel & Gehäuse
      const baseGeo = new THREE.CylinderGeometry(3.5, 4, 3, 10);
      const baseMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.8 });
      const base = new THREE.Mesh(baseGeo, baseMat);
      base.position.set(lanternX, 2.5, lanternZ);
      base.castShadow = true;
      accGroup.add(base);

      // Laternengehäuse (schwarzer Würfel)
      const housingGeo = new THREE.BoxGeometry(5.5, 6, 5.5);
      const housingMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.7 });
      const housing = new THREE.Mesh(housingGeo, housingMat);
      housing.position.set(lanternX, 6.5, lanternZ);
      housing.castShadow = true;
      accGroup.add(housing);

      // Beleuchtetes Weichensignal / Laternenoptik
      const lensGeo = new THREE.BoxGeometry(4.5, 4.5, 5.8);
      const lensMat = new THREE.MeshStandardMaterial({
        color: 0xfef08a,
        emissive: 0xfef08a,
        emissiveIntensity: 0.8,
        roughness: 0.3,
      });
      const lens = new THREE.Mesh(lensGeo, lensMat);
      lens.position.set(lanternX, 6.5, lanternZ);
      accGroup.add(lens);
    }

    if (piece.hasDecoder) {
      // 3. Digitaldecoder-Kasten
      const decX = piece.hasLantern ? lanternX + 11 : lanternX;
      const decZ = lanternZ;
      const decGeo = new THREE.BoxGeometry(9, 3, 12);
      const decMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.4, metalness: 0.5 });
      const dec = new THREE.Mesh(decGeo, decMat);
      dec.position.set(decX, 2, decZ);
      dec.castShadow = true;
      accGroup.add(dec);

      // Mini-LED Kontrollleuchte auf dem Decoder
      const ledGeo = new THREE.CylinderGeometry(0.8, 0.8, 1, 8);
      const ledMat = new THREE.MeshStandardMaterial({
        color: 0x38bdf8,
        emissive: 0x0284c7,
        emissiveIntensity: 0.9,
      });
      const led = new THREE.Mesh(ledGeo, ledMat);
      led.position.set(decX + 2.5, 3.8, decZ);
      accGroup.add(led);
    }

    group.add(accGroup);
  }

  private buildTurntable3D(
    group: THREE.Group,
    _piece: TrackPiece,
    entry: CatalogEntry,
    ballastColor: number,
  ) {
    const outerR = entry.radius ?? 189;
    const pitR = 139;
    const pitDepth = 12;
    const bridgeL = 263;
    const bridgeW = 34;
    const gauge = entry.gauge ?? 16.5;
    const gaugeHalf = gauge / 2;

    const ballastMat = new THREE.MeshStandardMaterial({ color: ballastColor, roughness: 0.9 });
    const concreteMat = new THREE.MeshStandardMaterial({ color: 0x475569, roughness: 0.85 });
    const pitFloorMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.95 });
    const steelMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.7 });
    const woodMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.85 });
    const railMat = new THREE.MeshStandardMaterial({ color: 0xd1d5db, metalness: 0.85, roughness: 0.25 });
    const cabMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.6 });
    const glassMat = new THREE.MeshStandardMaterial({ color: 0x38bdf8, roughness: 0.2, metalness: 0.9 });

    // 1. Äußerer Schotterkranz (Ring von pitR bis outerR)
    const ringShape = new THREE.Shape();
    ringShape.absarc(0, 0, outerR, 0, Math.PI * 2, false);
    const holePath = new THREE.Path();
    holePath.absarc(0, 0, pitR, 0, Math.PI * 2, true);
    ringShape.holes.push(holePath);

    const ringGeo = new THREE.ExtrudeGeometry(ringShape, { depth: 6, bevelEnabled: false, curveSegments: 36 });
    ringGeo.rotateX(Math.PI / 2);
    const ringMesh = new THREE.Mesh(ringGeo, ballastMat);
    ringMesh.position.set(0, 6, 0);
    ringMesh.receiveShadow = true;
    group.add(ringMesh);

    // 2. Grubenrand-Einfassung
    const rimShape = new THREE.Shape();
    rimShape.absarc(0, 0, pitR + 4, 0, Math.PI * 2, false);
    const rimHole = new THREE.Path();
    rimHole.absarc(0, 0, pitR, 0, Math.PI * 2, true);
    rimShape.holes.push(rimHole);

    const rimGeo = new THREE.ExtrudeGeometry(rimShape, { depth: 7.5, bevelEnabled: false, curveSegments: 36 });
    rimGeo.rotateX(Math.PI / 2);
    const rimMesh = new THREE.Mesh(rimGeo, concreteMat);
    rimMesh.position.set(0, 7.5, 0);
    group.add(rimMesh);

    // 3. Grubenboden
    const floorGeo = new THREE.CylinderGeometry(pitR, pitR, 3, 36);
    const floorMesh = new THREE.Mesh(floorGeo, pitFloorMat);
    floorMesh.position.set(0, -pitDepth, 0);
    floorMesh.receiveShadow = true;
    group.add(floorMesh);

    // 4. Grubenwand
    const wallGeo = new THREE.CylinderGeometry(pitR, pitR, pitDepth + 7.5, 36, 1, true);
    const wallMesh = new THREE.Mesh(wallGeo, concreteMat);
    wallMesh.position.set(0, (-pitDepth + 7.5) / 2, 0);
    group.add(wallMesh);

    // 5. Kreisförmige Laufschiene für die Bühnenräder in der Grube
    const pitRailGeo = new THREE.TorusGeometry(126, 1.2, 8, 36);
    pitRailGeo.rotateX(Math.PI / 2);
    const pitRail = new THREE.Mesh(pitRailGeo, railMat);
    pitRail.position.set(0, -pitDepth + 1.8, 0);
    group.add(pitRail);

    // 6. Mittiger Drehzapfen / Königsstuhl
    const pivotGeo = new THREE.CylinderGeometry(14, 18, 10, 24);
    const pivot = new THREE.Mesh(pivotGeo, steelMat);
    pivot.position.set(0, -pitDepth + 5, 0);
    group.add(pivot);

    // 7. Drehscheibenbühne
    for (const zSide of [-8, 8]) {
      const girderGeo = new THREE.BoxGeometry(bridgeL - 6, 11, 2.5);
      const girder = new THREE.Mesh(girderGeo, steelMat);
      girder.position.set(0, 0, zSide);
      group.add(girder);
    }

    const deckGeo = new THREE.BoxGeometry(bridgeL, 2.4, bridgeW);
    const deck = new THREE.Mesh(deckGeo, woodMat);
    deck.position.set(0, 6.2, 0);
    deck.receiveShadow = true;
    group.add(deck);

    const railGeo = new THREE.BoxGeometry(bridgeL, 2.2, 1.2);
    const rL = new THREE.Mesh(railGeo, railMat);
    rL.position.set(0, 8.5, -gaugeHalf);
    group.add(rL);

    const rR = new THREE.Mesh(railGeo, railMat);
    rR.position.set(0, 8.5, gaugeHalf);
    group.add(rR);

    const tieGeo = new THREE.BoxGeometry(2.6, 1.6, gauge * 1.55);
    const tieCount = Math.floor(bridgeL / 6.5);
    for (let i = 0; i <= tieCount; i++) {
      const tie = new THREE.Mesh(tieGeo, woodMat);
      tie.position.set(-bridgeL / 2 + i * 6.5 + 3.2, 7.6, 0);
      group.add(tie);
    }

    for (const zSide of [-bridgeW / 2 + 1, bridgeW / 2 - 1]) {
      const railBar = new THREE.Mesh(new THREE.BoxGeometry(bridgeL - 10, 1.2, 1.2), steelMat);
      railBar.position.set(0, 15, zSide);
      group.add(railBar);

      for (let x = -bridgeL / 2 + 15; x <= bridgeL / 2 - 15; x += 35) {
        const post = new THREE.Mesh(new THREE.BoxGeometry(1.2, 8, 1.2), steelMat);
        post.position.set(x, 11, zSide);
        group.add(post);
      }
    }

    // Wärterhaus / Bedienstand
    const cabW = 32;
    const cabD = 20;
    const cabH = 26;
    const cab = new THREE.Mesh(new THREE.BoxGeometry(cabW, cabH, cabD), cabMat);
    cab.position.set(40, 17, bridgeW / 2 + cabD / 2 - 2);
    group.add(cab);

    const win = new THREE.Mesh(new THREE.BoxGeometry(cabW - 6, 9, cabD + 0.5), glassMat);
    win.position.set(40, 20, bridgeW / 2 + cabD / 2 - 2);
    group.add(win);

    const cabRoof = new THREE.Mesh(new THREE.BoxGeometry(cabW + 4, 2.5, cabD + 4), steelMat);
    cabRoof.position.set(40, 31, bridgeW / 2 + cabD / 2 - 2);
    group.add(cabRoof);

    // 8. 30 Anschlussstümpfe am Rand
    const stubLen = outerR - pitR;
    for (let a = 0; a < 360; a += 12) {
      const rad = deg2rad(a);
      const stubCenterR = (pitR + outerR) / 2;

      for (const offZ of [-gaugeHalf, gaugeHalf]) {
        const sRail = new THREE.Mesh(new THREE.BoxGeometry(stubLen, 2.2, 1.2), railMat);
        sRail.position.set(stubCenterR * Math.cos(rad) - offZ * Math.sin(rad), 8.5, stubCenterR * Math.sin(rad) + offZ * Math.cos(rad));
        sRail.rotation.y = -rad;
        group.add(sRail);
      }
    }
  }

  private buildCatenary3D(group: THREE.Group, entry: CatalogEntry, startElev: number, endElev: number, piece?: TrackPiece) {
    const catGroup = new THREE.Group();

    const wireMat = new THREE.MeshStandardMaterial({ color: 0xd97706, roughness: 0.3, metalness: 0.85 }); // Kupfer Fahrdraht
    const messengerMat = new THREE.MeshStandardMaterial({ color: 0x94a3b8, roughness: 0.4, metalness: 0.7 }); // Stahl Tragseil
    const mastMat = new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.6 }); // Gittermast
    const baseMat = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.9 }); // Beton-Fundament
    const insMat = new THREE.MeshStandardMaterial({ color: 0x854d0e, roughness: 0.3 }); // Isolator

    const contactH = 72; // 64 mm über Schienenoberkante
    const messengerH = 85; // 13 mm über Fahrdraht

    const buildMast = (x: number, z: number, rotY = 0, side = 1) => {
      const mastGroup = new THREE.Group();
      mastGroup.position.set(x, 0, z);
      mastGroup.rotation.y = rotY;

      const mastDist = 24 * side;

      // Beton-Fundament
      const baseGeo = new THREE.BoxGeometry(6, 8, 6);
      const baseMesh = new THREE.Mesh(baseGeo, baseMat);
      baseMesh.position.set(0, 4, mastDist);
      baseMesh.castShadow = true;
      baseMesh.receiveShadow = true;
      mastGroup.add(baseMesh);

      // Stahl-Gittermast
      const poleGeo = new THREE.BoxGeometry(3, 86, 3);
      const poleMesh = new THREE.Mesh(poleGeo, mastMat);
      poleMesh.position.set(0, 47, mastDist);
      poleMesh.castShadow = true;
      mastGroup.add(poleMesh);

      // Oberer Auslegerarm (horizontal zur Gleismitte)
      const armGeo = new THREE.BoxGeometry(1.6, 1.6, 24);
      const armMesh = new THREE.Mesh(armGeo, mastMat);
      armMesh.position.set(0, messengerH, mastDist / 2);
      mastGroup.add(armMesh);

      // Diagonale Strebe
      const diagLen = Math.hypot(13, 16);
      const diagGeo = new THREE.BoxGeometry(1.2, 1.2, diagLen);
      const diagMesh = new THREE.Mesh(diagGeo, mastMat);
      diagMesh.position.set(0, (contactH + messengerH) / 2, mastDist * 0.6);
      diagMesh.rotation.x = Math.atan2(13, 16) * side;
      mastGroup.add(diagMesh);

      // Isolatoren
      const insGeo = new THREE.CylinderGeometry(1.2, 1.2, 3, 8);
      const ins1 = new THREE.Mesh(insGeo, insMat);
      ins1.position.set(0, messengerH, 3 * side);
      mastGroup.add(ins1);

      const ins2 = new THREE.Mesh(insGeo, insMat);
      ins2.position.set(0, contactH, 1 * side);
      mastGroup.add(ins2);

      catGroup.add(mastGroup);
    };

    // Helfer für gerade Fahrleitungs-Drähte
    const addStraightWire = (
      x1: number,
      z1: number,
      x2: number,
      z2: number,
      withMasts = true,
      mastSide = 1,
    ) => {
      const dx = x2 - x1;
      const dz = z2 - z1;
      const len = Math.hypot(dx, dz);
      if (len < 1) return;
      const angle = Math.atan2(dz, dx);
      const mx = (x1 + x2) / 2;
      const mz = (z1 + z2) / 2;

      // Fahrdraht (Contact Wire)
      const wireGeo = new THREE.BoxGeometry(len, 0.8, 0.8);
      const wireMesh = new THREE.Mesh(wireGeo, wireMat);
      wireMesh.position.set(mx, contactH, mz);
      wireMesh.rotation.y = -angle;
      catGroup.add(wireMesh);

      // Tragseil (Messenger Wire)
      const msgGeo = new THREE.BoxGeometry(len, 0.7, 0.7);
      const msgMesh = new THREE.Mesh(msgGeo, messengerMat);
      msgMesh.position.set(mx, messengerH, mz);
      msgMesh.rotation.y = -angle;
      catGroup.add(msgMesh);

      // Hänger alle 35 mm
      const dropperCount = Math.max(1, Math.floor(len / 35));
      for (let i = 1; i <= dropperCount; i++) {
        const u = i / (dropperCount + 1);
        const hx = x1 + dx * u;
        const hz = z1 + dz * u;
        const dropperGeo = new THREE.CylinderGeometry(0.3, 0.3, 13, 6);
        const dropperMesh = new THREE.Mesh(dropperGeo, messengerMat);
        dropperMesh.position.set(hx, (contactH + messengerH) / 2, hz);
        catGroup.add(dropperMesh);
      }

      if (withMasts) {
        buildMast(x1, z1, -angle, mastSide);
        if (len >= 120) {
          buildMast(x2, z2, -angle, mastSide);
        }
      }
    };

    // Helfer für gebogene Fahrleitungs-Drähte
    const addCurvedWire = (
      r: number,
      rad: number,
      dir: 1 | -1 = 1,
      leadX = 0,
      withMasts = true,
      mastSide = 1,
    ) => {
      const segs = Math.max(4, Math.floor(rad / deg2rad(5)));
      for (let i = 0; i < segs; i++) {
        const th1 = (rad / segs) * i;
        const th2 = (rad / segs) * (i + 1);

        const x1 = leadX + r * Math.sin(th1);
        const z1 = dir * r * (1 - Math.cos(th1));
        const x2 = leadX + r * Math.sin(th2);
        const z2 = dir * r * (1 - Math.cos(th2));

        const segLen = Math.hypot(x2 - x1, z2 - z1);
        const segAngle = Math.atan2(z2 - z1, x2 - x1);
        const mx = (x1 + x2) / 2;
        const mz = (z1 + z2) / 2;

        const wGeo = new THREE.BoxGeometry(segLen, 0.8, 0.8);
        const wMesh = new THREE.Mesh(wGeo, wireMat);
        wMesh.position.set(mx, contactH, mz);
        wMesh.rotation.y = -segAngle;
        catGroup.add(wMesh);

        const mGeo = new THREE.BoxGeometry(segLen, 0.7, 0.7);
        const mMesh = new THREE.Mesh(mGeo, messengerMat);
        mMesh.position.set(mx, messengerH, mz);
        mMesh.rotation.y = -segAngle;
        catGroup.add(mMesh);

        const dropGeo = new THREE.CylinderGeometry(0.3, 0.3, 13, 6);
        const dropMesh = new THREE.Mesh(dropGeo, messengerMat);
        dropMesh.position.set(mx, (contactH + messengerH) / 2, mz);
        catGroup.add(dropMesh);
      }

      if (withMasts) {
        buildMast(leadX, 0, 0, mastSide);
        const endX = leadX + r * Math.sin(rad);
        const endZ = dir * r * (1 - Math.cos(rad));
        buildMast(endX, endZ, -dir * rad, mastSide);
      }
    };

    const kind = entry.kind;
    const isCurvedTurnout =
      kind === "curved-turnout-left" ||
      kind === "curved-turnout-right" ||
      ((kind === "turnout-left" || kind === "turnout-right") && !!entry.radius && !entry.length);

    if (kind === "straight" || kind === "accessory") {
      const len = entry.length ?? 188.3;
      const heightDiff = endElev - startElev;
      if (Math.abs(heightDiff) > 0.01) {
        catGroup.rotation.z = Math.atan2(heightDiff, len);
      }
      addStraightWire(0, 0, len, 0, true, 1);
    } else if (kind === "curve") {
      const r = entry.radius ?? 437.5;
      const a = entry.angle ?? 30;
      addCurvedWire(r, deg2rad(a), 1, 0, true, 1);
    } else if (isCurvedTurnout) {
      const dir: 1 | -1 = kind === "curved-turnout-left" || kind === "turnout-left" ? 1 : -1;
      const r1 = entry.radius ?? 360;
      const a1 = entry.angle ?? 30;
      const lead =
        entry.branchLength ??
        (entry.branchRadius && entry.branchRadius > r1 ? entry.branchRadius - r1 : 0);
      const r2 =
        entry.branchLength !== undefined
          ? (entry.branchRadius ?? r1)
          : entry.branchRadius && entry.branchRadius > r1
            ? r1
            : (entry.branchRadius ?? r1);
      const a2 = entry.branchAngle ?? 30;

      addCurvedWire(r1, deg2rad(a1), dir, 0, true, -dir);
      if (lead > 0) {
        addStraightWire(0, 0, lead, 0, false);
      }
      addCurvedWire(r2, deg2rad(a2), dir, lead, true, dir);
    } else if (kind === "turnout-left" || kind === "turnout-right") {
      const len = entry.length ?? 188.3;
      const br = entry.branchRadius ?? 437.5;
      const ba = entry.branchAngle ?? 24.3;
      const dir: 1 | -1 = kind === "turnout-left" ? 1 : -1;

      addStraightWire(0, 0, len, 0, true, -dir);
      addCurvedWire(br, deg2rad(ba), dir, 0, true, dir);
    } else if (kind === "three-way") {
      const len = entry.length ?? 188.3;
      const br = entry.branchRadius ?? 437.5;
      const ba = entry.branchAngle ?? 24.3;

      addStraightWire(0, 0, len, 0, true, 1);
      addCurvedWire(br, deg2rad(ba), 1, 0, true, 1);
      addCurvedWire(br, deg2rad(ba), -1, 0, true, -1);
    } else if (kind === "crossing") {
      const len = entry.length ?? 188.3;
      const a = entry.angle ?? 24.3;
      const rad = deg2rad(a);
      const h = len / 2;
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);

      addStraightWire(-h, 0, h, 0, true, 1);
      addStraightWire(-h * cos, -h * sin, h * cos, h * sin, true, 1);
    } else if (kind === "dkw") {
      const len = entry.length ?? 188.3;
      const a = entry.angle ?? 24.3;
      const rad = deg2rad(a);
      const h = len / 2;
      const cx = h;
      const cos = Math.cos(rad);
      const sin = Math.sin(rad);

      addStraightWire(0, 0, len, 0, true, 1);
      addStraightWire(cx - h * cos, -h * sin, cx + h * cos, h * sin, true, 1);
    } else {
      const len = entry.length ?? 188.3;
      addStraightWire(0, 0, len, 0, true, 1);
    }

    // 3D-OBERLEITUNGS-EINSPEISUNG (Feeder box, Leitung & Klemmen am Mast)
    if (piece?.catenaryPowerFeed) {
      const feedBoxGeo = new THREE.BoxGeometry(4, 9, 5);
      const feedBoxMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.4 });
      const feedBox = new THREE.Mesh(feedBoxGeo, feedBoxMat);
      feedBox.position.set(0, 5, 27);
      catGroup.add(feedBox);

      // Schutzrohr / Kabel am Gittermast nach oben
      const conduitGeo = new THREE.CylinderGeometry(0.6, 0.6, 85, 6);
      const conduitMat = new THREE.MeshStandardMaterial({ color: 0x0f172a, roughness: 0.5 });
      const conduit = new THREE.Mesh(conduitGeo, conduitMat);
      conduit.position.set(0, 43, 25.5);
      catGroup.add(conduit);

      // Klemmenverbindung vom Mast zum Tragseil & Fahrdraht
      const clampGeo = new THREE.CylinderGeometry(0.5, 0.5, 24, 6);
      const clampMat = new THREE.MeshStandardMaterial({ color: 0x0284c7, roughness: 0.3, metalness: 0.8 });
      const feedWire = new THREE.Mesh(clampGeo, clampMat);
      feedWire.position.set(0, messengerH, 12);
      feedWire.rotation.x = Math.PI / 2;
      catGroup.add(feedWire);

      const dropFeedGeo = new THREE.CylinderGeometry(0.5, 0.5, 13, 6);
      const dropFeed = new THREE.Mesh(dropFeedGeo, clampMat);
      dropFeed.position.set(0, (contactH + messengerH) / 2, 0);
      catGroup.add(dropFeed);
    }

    // 3D-OBERLEITUNGS-TRENNSTELLE (Streckentrenner / Section Insulator mit Keramik-Isolatoren)
    if (piece?.catenaryIsolator) {
      const isoIdx = piece.catenaryIsolatedConnectorIndex ?? 0;
      let isoX = 25;
      let isoZ = 0;
      let isoRotY = 0;

      if (kind === "curve") {
        const r = entry.radius ?? 437.5;
        const a = entry.angle ?? 30;
        const rad = deg2rad(a);
        const frac = isoIdx === 1 ? 0.82 : 0.18;
        const phi = frac * rad;
        isoX = r * Math.sin(phi);
        isoZ = r * (1 - Math.cos(phi));
        isoRotY = -phi;
      } else {
        const len = entry.length ?? 188.3;
        isoX = isoIdx === 1 ? Math.max(20, len - 25) : Math.min(len - 20, 25);
      }

      const isoGroup = new THREE.Group();
      isoGroup.position.set(isoX, 0, isoZ);
      isoGroup.rotation.y = isoRotY;

      // Zwillings-Keramikisolator im Fahrdraht mit Trennspalt
      const secInsGeo = new THREE.CylinderGeometry(1.5, 1.5, 14, 8);
      const secInsMat = new THREE.MeshStandardMaterial({ color: 0x9a3412, roughness: 0.25 }); // Keramik
      const insMesh1 = new THREE.Mesh(secInsGeo, secInsMat);
      insMesh1.position.set(0, contactH, 1.2);
      insMesh1.rotation.z = Math.PI / 2;
      isoGroup.add(insMesh1);

      const insMesh2 = new THREE.Mesh(secInsGeo, secInsMat);
      insMesh2.position.set(0, contactH, -1.2);
      insMesh2.rotation.z = Math.PI / 2;
      isoGroup.add(insMesh2);

      // Keramikisolator im Tragseil
      const topInsMesh = new THREE.Mesh(secInsGeo, secInsMat);
      topInsMesh.position.set(0, messengerH, 0);
      topInsMesh.rotation.z = Math.PI / 2;
      isoGroup.add(topInsMesh);

      // Signalkörper / Schutzabdeckung (Rot)
      const markGeo = new THREE.BoxGeometry(4, 3, 4);
      const markMat = new THREE.MeshStandardMaterial({ color: 0xdc2626, roughness: 0.4 });
      const markMesh = new THREE.Mesh(markGeo, markMat);
      markMesh.position.set(0, contactH + 2.5, 0);
      isoGroup.add(markMesh);

      catGroup.add(isoGroup);
    }

    group.add(catGroup);
  }

  private getStyleColor(style: BaseboardConfig["style"]): number {
    switch (style) {
      case "dark-wood": return 0x4a2e1b;
      case "plywood": return 0xecdcb9;
      case "slate": return 0x27272a;
      case "white": return 0xf8fafc;
      case "wood":
      default: return 0xd4a373;
    }
  }

  resetCamera() {
    this.camera.position.set(0, 1800, 2400);
    this.controls.target.set(0, 50, 0);
    this.controls.update();
  }

  /**
   * Baut die historische Lokhalle / Bahnhofshalle (Train Hall) als dreidimensionalen
   * visuellen Hintergrund, in dessen Mitte die Modellbahn-Anlage auf dem Tisch steht.
   */
  private buildTrainHall(layout: Layout) {
    // 1. Ausdehnung des Tisches / der Anlage ermitteln
    let minX = -1200;
    let maxX = 1200;
    let minZ = -800;
    let maxZ = 800;

    const baseboards = getBaseboards(layout);
    let hasBounds = false;
    for (const bb of baseboards) {
      if (!bb.enabled) continue;
      const b = getBaseboardBounds(bb);
      const bMinX = b.x;
      const bMaxX = b.x + b.width;
      const bMinZ = b.y;
      const bMaxZ = b.y + b.depth;
      if (!hasBounds) {
        minX = bMinX;
        maxX = bMaxX;
        minZ = bMinZ; // 2D Y ist 3D Z
        maxZ = bMaxZ;
        hasBounds = true;
      } else {
        if (bMinX < minX) minX = bMinX;
        if (bMaxX > maxX) maxX = bMaxX;
        if (bMinZ < minZ) minZ = bMinZ;
        if (bMaxZ > maxZ) maxZ = bMaxZ;
      }
    }

    if (!hasBounds && layout.pieces.length > 0) {
      for (const p of layout.pieces) {
        if (!hasBounds) {
          minX = p.transform.x;
          maxX = p.transform.x;
          minZ = p.transform.y;
          maxZ = p.transform.y;
          hasBounds = true;
        } else {
          if (p.transform.x < minX) minX = p.transform.x;
          if (p.transform.x > maxX) maxX = p.transform.x;
          if (p.transform.y < minZ) minZ = p.transform.y;
          if (p.transform.y > maxZ) maxZ = p.transform.y;
        }
      }
      minX -= 250;
      maxX += 250;
      minZ -= 250;
      maxZ += 250;
    }

    const cx = (minX + maxX) / 2;
    const cz = (minZ + maxZ) / 2;
    const wTable = Math.max(1200, maxX - minX);
    const lTable = Math.max(1000, maxZ - minZ);

    // Dimensionen der Lokhalle
    const wHall = Math.max(9000, wTable + 4800);
    const lHall = Math.max(10000, lTable + 5600);
    const floorY = -755; // Niveau unter den 720mm Tischbeinen
    const wallH = floorY + 5200; // Traufhöhe / Stützenhöhe (Y = 1445)
    const apexH = floorY + 6400; // Firsthöhe / Bogenkamm (Y = 3645)

    this.sceneCenter.set(cx, floorY, cz);
    this.nightLampMats = [];

    const isDark = this.currentTheme === "dark";
    const steelColor = isDark ? 0x1e293b : 0x2c3e50;
    const brickColor = isDark ? 0x4a2820 : 0x6e382b;
    const concreteColor = isDark ? 0x334155 : 0x64748b;

    // Gemeinsame Materialien
    const floorMat = new THREE.MeshStandardMaterial({
      color: concreteColor,
      roughness: 0.88,
      metalness: 0.08,
      polygonOffset: true,
      polygonOffsetFactor: -1,
      polygonOffsetUnits: -1,
    });
    const yellowLineMat = new THREE.MeshStandardMaterial({
      color: 0xf59e0b,
      roughness: 0.4,
      metalness: 0.2,
    });
    const steelMat = new THREE.MeshStandardMaterial({
      color: steelColor,
      roughness: 0.45,
      metalness: 0.75,
    });
    const craneYellowMat = new THREE.MeshStandardMaterial({
      color: 0xf59e0b,
      roughness: 0.35,
      metalness: 0.4,
    });
    const brickMat = new THREE.MeshStandardMaterial({
      color: brickColor,
      roughness: 0.9,
      metalness: 0.05,
    });
    const skylightGlassMat = new THREE.MeshStandardMaterial({
      color: 0xcffafe,
      transparent: true,
      opacity: 0.32,
      roughness: 0.1,
      metalness: 0.1,
    });
    const windowGlassMat = new THREE.MeshStandardMaterial({
      color: 0x93c5fd,
      transparent: true,
      opacity: 0.38,
      roughness: 0.15,
      metalness: 0.1,
    });
    const roofMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x1e293b : 0x334155,
      roughness: 0.7,
      metalness: 0.4,
    });
    const sleeperMat = new THREE.MeshStandardMaterial({
      color: 0x27272a,
      roughness: 0.8,
    });
    const railMat = new THREE.MeshStandardMaterial({
      color: 0x94a3b8,
      roughness: 0.25,
      metalness: 0.85,
    });

    // 1. Hallenboden, Sicherheitsmarkierungen & Bodengleise
    this.buildTrainHallFloor(cx, cz, wHall, lHall, floorY, minX, maxX, minZ, maxZ, wTable, floorMat, yellowLineMat, sleeperMat, railMat);

    // 2. Stahl-Fachwerkbogen & Stützen (Hallenbogen)
    this.buildTrainHallArches(cx, cz, wHall, lHall, floorY, wallH, apexH, steelMat);

    // 3. Glas-Oberlicht & Zinkdach
    this.buildTrainHallRoof(cx, cz, wHall, lHall, wallH, apexH, skylightGlassMat, roofMat, steelMat);

    // 4. Giebelwände mit Rundbogenfenstern, Hallentoren & Bahnhofsuhr
    this.buildTrainHallEndWalls(cx, cz, (wHall), (lHall), floorY, wallH, apexH, brickMat, windowGlassMat, steelMat);

    // 5. Seitenwände mit Industriefenstern
    this.buildTrainHallSideWalls(cx, cz, wHall, lHall, floorY, wallH, brickMat, windowGlassMat, steelMat);

    // 6. Decken-Brückenkran (Laufkran mit Haken)
    this.buildTrainHallCrane(cx, cz, wHall, lHall, floorY, craneYellowMat, steelMat);

    // 7. Industrie-Pendelleuchten mit warmem Schein
    this.buildTrainHallLamps(cx, cz, wHall, lHall, floorY, apexH, steelMat);

    // 8. Dichter Wald rund um die Lokhalle (sichtbar durch alle Fenster, Tore und Dachoberlichter)
    this.buildTrainHallForest(cx, cz, wHall, lHall, floorY);

    // 9. Idyllischer kleiner See vor den Hügeln auf der offenen Wiesenseite
    this.buildTrainHallLake(cx, cz, wHall, lHall, floorY);
  }

  private buildTrainHallFloor(
    cx: number,
    cz: number,
    wHall: number,
    lHall: number,
    floorY: number,
    minX: number,
    maxX: number,
    minZ: number,
    maxZ: number,
    wTable: number,
    floorMat: THREE.Material,
    yellowLineMat: THREE.Material,
    sleeperMat: THREE.Material,
    railMat: THREE.Material,
  ) {
    // A. Lokhallen-Betonboden: Leicht angehobenes Fundament (+16 mm über Terrain)
    // mit solider Dicke (50 mm), um Z-Fighting und Artefakte mit dem Waldboden vollständig zu verhindern.
    const floorH = 50;
    const floorElev = 16;
    const floorGeo = new THREE.BoxGeometry(wHall + 80, floorH, lHall + 80);
    const floorMesh = new THREE.Mesh(floorGeo, floorMat);
    floorMesh.position.set(cx, floorY + floorElev - floorH / 2, cz);
    floorMesh.receiveShadow = true;
    this.trainHallGroup.add(floorMesh);

    // B. Gelbe Sicherheitslinien um den Modellbahntisch
    const margin = 220;
    const boxW = (maxX - minX) + margin * 2;
    const boxL = (maxZ - minZ) + margin * 2;
    const lineThick = 35;
    const lineY = floorY + floorElev + 0.8;

    const lineN = new THREE.Mesh(new THREE.BoxGeometry(boxW, 2, lineThick), yellowLineMat);
    lineN.position.set(cx, lineY, minZ - margin);
    this.trainHallGroup.add(lineN);

    const lineS = new THREE.Mesh(new THREE.BoxGeometry(boxW, 2, lineThick), yellowLineMat);
    lineS.position.set(cx, lineY, maxZ + margin);
    this.trainHallGroup.add(lineS);

    const lineW = new THREE.Mesh(new THREE.BoxGeometry(lineThick, 2, boxL), yellowLineMat);
    lineW.position.set(minX - margin, lineY, cz);
    this.trainHallGroup.add(lineW);

    const lineE = new THREE.Mesh(new THREE.BoxGeometry(lineThick, 2, boxL), yellowLineMat);
    lineE.position.set(maxX + margin, lineY, cz);
    this.trainHallGroup.add(lineE);
  }

  private buildTrainHallArches(
    cx: number,
    cz: number,
    wHall: number,
    lHall: number,
    floorY: number,
    wallH: number,
    apexH: number,
    steelMat: THREE.Material,
  ) {
    const numBays = 6;
    const stepZ = lHall / numBays;
    const halfW = wHall / 2;
    const colHeight = wallH - floorY;

    // 1. Stützen & Bogen für jede Hallenachse
    for (let i = 0; i <= numBays; i++) {
      const z = cz - lHall / 2 + i * stepZ;

      // Linke Stütze (Stahl-I-Träger)
      const colL = new THREE.Mesh(new THREE.BoxGeometry(70, colHeight, 70), steelMat);
      colL.position.set(cx - halfW, (floorY + wallH) / 2, z);
      colL.castShadow = true;
      colL.receiveShadow = true;
      this.trainHallGroup.add(colL);

      // Rechte Stütze
      const colR = new THREE.Mesh(new THREE.BoxGeometry(70, colHeight, 70), steelMat);
      colR.position.set(cx + halfW, (floorY + wallH) / 2, z);
      colR.castShadow = true;
      colR.receiveShadow = true;
      this.trainHallGroup.add(colR);

      // Äußerer Bogen-Gurt
      const outerCurve = new THREE.CatmullRomCurve3([
        new THREE.Vector3(-halfW, wallH, 0),
        new THREE.Vector3(-halfW * 0.65, (wallH + apexH) / 2 + 260, 0),
        new THREE.Vector3(0, apexH, 0),
        new THREE.Vector3(halfW * 0.65, (wallH + apexH) / 2 + 260, 0),
        new THREE.Vector3(halfW, wallH, 0),
      ]);
      const outerGeo = new THREE.TubeGeometry(outerCurve, 24, 20, 8, false);
      const outerMesh = new THREE.Mesh(outerGeo, steelMat);
      outerMesh.position.set(cx, 0, z);
      outerMesh.castShadow = true;
      this.trainHallGroup.add(outerMesh);

      // Innerer Bogen-Gurt
      const innerCurve = new THREE.CatmullRomCurve3([
        new THREE.Vector3(-halfW + 160, wallH + 10, 0),
        new THREE.Vector3((-halfW + 160) * 0.65, (wallH + apexH) / 2 + 70, 0),
        new THREE.Vector3(0, apexH - 220, 0),
        new THREE.Vector3((halfW - 160) * 0.65, (wallH + apexH) / 2 + 70, 0),
        new THREE.Vector3(halfW - 160, wallH + 10, 0),
      ]);
      const innerGeo = new THREE.TubeGeometry(innerCurve, 24, 15, 8, false);
      const innerMesh = new THREE.Mesh(innerGeo, steelMat);
      innerMesh.position.set(cx, 0, z);
      this.trainHallGroup.add(innerMesh);

      // Fachwerk-Verbindungsstreben
      const strutSteps = 9;
      for (let s = 1; s < strutSteps; s++) {
        const t = s / strutSteps;
        const ptOut = outerCurve.getPoint(t);
        const ptIn = innerCurve.getPoint(t);
        const vLen = ptOut.distanceTo(ptIn);

        const strutGeo = new THREE.CylinderGeometry(8, 8, vLen, 6);
        const strut = new THREE.Mesh(strutGeo, steelMat);
        strut.position.set(cx + (ptOut.x + ptIn.x) / 2, (ptOut.y + ptIn.y) / 2, z);
        strut.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), ptOut.clone().sub(ptIn).normalize());
        this.trainHallGroup.add(strut);
      }
    }

    // 2. Längs-Pfetten (Stahlträger der Hallendecke entlang Z)
    const purlinPositions = [
      { x: 0, y: apexH }, // First
      { x: -halfW * 0.35, y: (wallH + apexH) / 2 + 250 }, // Schulter links
      { x: halfW * 0.35, y: (wallH + apexH) / 2 + 250 }, // Schulter rechts
      { x: -halfW, y: wallH }, // Traufe links
      { x: halfW, y: wallH }, // Traufe rechts
      { x: -halfW + 80, y: floorY + 1600 }, // Kranschiene links
      { x: halfW - 80, y: floorY + 1600 }, // Kranschiene rechts
    ];

    const purlinGeo = new THREE.BoxGeometry(24, 28, lHall + 100);
    for (const p of purlinPositions) {
      const purlin = new THREE.Mesh(purlinGeo, steelMat);
      purlin.position.set(cx + p.x, p.y, cz);
      this.trainHallGroup.add(purlin);
    }
  }

  private buildTrainHallRoof(
    cx: number,
    cz: number,
    wHall: number,
    lHall: number,
    wallH: number,
    apexH: number,
    skylightGlassMat: THREE.Material,
    roofMat: THREE.Material,
    steelMat: THREE.Material,
  ) {
    const halfW = wHall / 2;
    const skylightW = wHall * 0.34;
    const shoulderY = (wallH + apexH) / 2 + 260;

    // 1. Glas-Oberlicht (zentral entlang des Firstes)
    // Linke Dachglashälfte
    const glassPlaneGeoL = new THREE.BoxGeometry(skylightW / 2 + 10, 8, lHall);
    const glassMeshL = new THREE.Mesh(glassPlaneGeoL, skylightGlassMat);
    const angleL = Math.atan2(apexH - shoulderY, skylightW / 2);
    glassMeshL.position.set(cx - skylightW / 4, (apexH + shoulderY) / 2, cz);
    glassMeshL.rotation.z = angleL;
    this.trainHallGroup.add(glassMeshL);

    // Rechte Dachglashälfte
    const glassMeshR = new THREE.Mesh(glassPlaneGeoL, skylightGlassMat);
    glassMeshR.position.set(cx + skylightW / 4, (apexH + shoulderY) / 2, cz);
    glassMeshR.rotation.z = -angleL;
    this.trainHallGroup.add(glassMeshR);

    // Oberlicht-Sprossen alle 500 mm
    const mullionSteps = Math.floor(lHall / 500);
    const mullionGeo = new THREE.BoxGeometry(10, 14, 12);
    for (let i = 0; i < mullionSteps; i++) {
      const mz = cz - lHall / 2 + i * 500;
      const mRidge = new THREE.Mesh(mullionGeo, steelMat);
      mRidge.position.set(cx, apexH + 12, mz);
      this.trainHallGroup.add(mRidge);
    }

    // 2. Seitliche Zink-Dachflächen
    const sideW = halfW - skylightW / 2;
    const sideSlopeGeo = new THREE.BoxGeometry(sideW + 20, 10, lHall);
    const sideAngle = Math.atan2(shoulderY - wallH, sideW);

    const roofL = new THREE.Mesh(sideSlopeGeo, roofMat);
    roofL.position.set(cx - (skylightW / 2 + sideW / 2), (shoulderY + wallH) / 2, cz);
    roofL.rotation.z = sideAngle;
    this.trainHallGroup.add(roofL);

    const roofR = new THREE.Mesh(sideSlopeGeo, roofMat);
    roofR.position.set(cx + (skylightW / 2 + sideW / 2), (shoulderY + wallH) / 2, cz);
    roofR.rotation.z = -sideAngle;
    this.trainHallGroup.add(roofR);
  }

  private buildTrainHallEndWalls(
    cx: number,
    cz: number,
    wHall: number,
    lHall: number,
    floorY: number,
    wallH: number,
    apexH: number,
    brickMat: THREE.Material,
    windowGlassMat: THREE.Material,
    steelMat: THREE.Material,
  ) {
    const endZPositions = [cz - lHall / 2, cz + lHall / 2];
    const halfW = wHall / 2;

    for (const z of endZPositions) {
      const isSouthWall = z > cz; // Südliche Stirnwand (Vorderseite, zugewandt zur Kamera und zum Weg)
      const lowerWallH = wallH - floorY;

      if (!isSouthWall) {
        // A1. Nordwand: Durchgehende solide Ziegelwand (keine Tore oder Gleise)
        const wallMesh = new THREE.Mesh(new THREE.BoxGeometry(wHall, lowerWallH, 50), brickMat);
        wallMesh.position.set(cx, (floorY + wallH) / 2, z);
        wallMesh.receiveShadow = true;
        this.trainHallGroup.add(wallMesh);
      } else {
        // A2. Südwand: Eingangstür in Personengröße mit offener Holztür
        const doorW = 1250;
        const doorH = 2250;
        const sideWallW = (wHall - doorW) / 2;

        // Linke Ziegelwand
        const wallL = new THREE.Mesh(new THREE.BoxGeometry(sideWallW, lowerWallH, 50), brickMat);
        wallL.position.set(cx - halfW + sideWallW / 2, (floorY + wallH) / 2, z);
        wallL.receiveShadow = true;
        this.trainHallGroup.add(wallL);

        // Rechte Ziegelwand
        const wallR = new THREE.Mesh(new THREE.BoxGeometry(sideWallW, lowerWallH, 50), brickMat);
        wallR.position.set(cx + halfW - sideWallW / 2, (floorY + wallH) / 2, z);
        wallR.receiveShadow = true;
        this.trainHallGroup.add(wallR);

        // Ziegel-Mauersturz über der Tür
        const wallAboveH = lowerWallH - doorH;
        const wallAbove = new THREE.Mesh(new THREE.BoxGeometry(doorW, wallAboveH, 50), brickMat);
        wallAbove.position.set(cx, floorY + doorH + wallAboveH / 2, z);
        wallAbove.receiveShadow = true;
        this.trainHallGroup.add(wallAbove);

        // Massiver Holz-/Balkentürrahmen
        const timberFrameMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.85 });
        const frameL = new THREE.Mesh(new THREE.BoxGeometry(70, doorH + 30, 80), timberFrameMat);
        frameL.position.set(cx - doorW / 2 - 30, floorY + (doorH + 30) / 2, z);
        this.trainHallGroup.add(frameL);

        const frameR = new THREE.Mesh(new THREE.BoxGeometry(70, doorH + 30, 80), timberFrameMat);
        frameR.position.set(cx + doorW / 2 + 30, floorY + (doorH + 30) / 2, z);
        this.trainHallGroup.add(frameR);

        const frameTop = new THREE.Mesh(new THREE.BoxGeometry(doorW + 140, 70, 85), timberFrameMat);
        frameTop.position.set(cx, floorY + doorH + 35, z);
        this.trainHallGroup.add(frameTop);

        // Granit-Eingangsstufe vor der Tür (schöner Übergang auf den angehobenen Boden)
        const stepMat = new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.75 });
        const step = new THREE.Mesh(new THREE.BoxGeometry(doorW + 180, 24, 420), stepMat);
        step.position.set(cx, floorY + 12, z + 180);
        step.receiveShadow = true;
        this.trainHallGroup.add(step);

        // DIE OFFENE HOLZTÜR (nach innen aufgeschwenkt, einladend geöffnet auf dem angehobenen Boden)
        const doorLeafW = doorW - 30;
        const doorLeafH = doorH - 20;
        const doorGroup = new THREE.Group();
        // Drehachse (Scharnier) am linken Türpfosten auf dem neuen Bodenniveau
        doorGroup.position.set(cx - doorW / 2 + 20, floorY + 16, z);
        doorGroup.rotation.y = 1.35; // ca. 77° nach innen geöffnet

        const woodDoorMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });
        const ironMat = new THREE.MeshStandardMaterial({ color: 0x18181b, roughness: 0.5, metalness: 0.85 });

        // Türblatt aus Holzbohlen
        const doorLeaf = new THREE.Mesh(new THREE.BoxGeometry(doorLeafW, doorLeafH, 38), woodDoorMat);
        doorLeaf.position.set(doorLeafW / 2, doorLeafH / 2, 0);
        doorLeaf.castShadow = true;
        doorGroup.add(doorLeaf);

        // Vertikale Bohlenfugen (Rillen)
        for (let bx = 160; bx < doorLeafW; bx += 180) {
          const groove = new THREE.Mesh(new THREE.BoxGeometry(4, doorLeafH - 20, 42), timberFrameMat);
          groove.position.set(bx, doorLeafH / 2, 0);
          doorGroup.add(groove);
        }

        // Horizontale schmiedeeiserne Türbänder (Langbänder mit Nieten)
        const hingeOffsetsY = [doorLeafH * 0.2, doorLeafH * 0.5, doorLeafH * 0.82];
        for (const hy of hingeOffsetsY) {
          const hinge = new THREE.Mesh(new THREE.BoxGeometry(doorLeafW * 0.75, 28, 46), ironMat);
          hinge.position.set(doorLeafW * 0.38, hy, 0);
          doorGroup.add(hinge);
        }

        // Schmiedeeiserner Türgriff / Ringdrücker
        const handlePlate = new THREE.Mesh(new THREE.BoxGeometry(22, 90, 48), ironMat);
        handlePlate.position.set(doorLeafW - 80, doorLeafH * 0.48, 0);
        doorGroup.add(handlePlate);

        const handleRing = new THREE.Mesh(new THREE.TorusGeometry(18, 4, 8, 16), ironMat);
        handleRing.position.set(doorLeafW - 80, doorLeafH * 0.48, 25);
        doorGroup.add(handleRing);

        this.trainHallGroup.add(doorGroup);

        // Eingangslaterne über der Tür
        const lampMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, metalness: 0.85, roughness: 0.35 });
        const lanternBulbMat = new THREE.MeshStandardMaterial({
          color: 0xfef08a,
          emissive: 0xfef08a,
          emissiveIntensity: 1.5,
        });
        this.nightLampMats.push(lanternBulbMat);

        const wallArm = new THREE.Mesh(new THREE.BoxGeometry(10, 10, 75), lampMat);
        wallArm.position.set(cx, floorY + doorH + 180, z + 40);
        this.trainHallGroup.add(wallArm);

        const lanternHead = new THREE.Mesh(new THREE.ConeGeometry(24, 28, 6), lampMat);
        lanternHead.position.set(cx, floorY + doorH + 195, z + 75);
        this.trainHallGroup.add(lanternHead);

        const lanternBulb = new THREE.Mesh(new THREE.SphereGeometry(12, 12, 12), lanternBulbMat);
        lanternBulb.position.set(cx, floorY + doorH + 175, z + 75);
        this.trainHallGroup.add(lanternBulb);
      }

      // B. Obere Giebel-Halbkreisscheibe / Bogenblende
      const gableH = apexH - wallH;
      const gableGeo = new THREE.BoxGeometry(wHall - 40, gableH, 40);
      const gableMesh = new THREE.Mesh(gableGeo, brickMat);
      gableMesh.position.set(cx, wallH + gableH / 2, z);
      gableMesh.receiveShadow = true;
      this.trainHallGroup.add(gableMesh);

      // C. Großes Rundbogen-Kathedralfenster im Giebel
      const winRadius = wHall * 0.22;
      const winCenterY = wallH + gableH * 0.42;

      // Glasfläche Halbkreis
      const winGlassGeo = new THREE.RingGeometry(0, winRadius, 32, 1, 0, Math.PI);
      const winGlass = new THREE.Mesh(winGlassGeo, windowGlassMat);
      winGlass.position.set(cx, winCenterY, z + (z < cz ? 26 : -26));
      this.trainHallGroup.add(winGlass);

      // Bogen-Stahlrahmen
      const winFrameGeo = new THREE.TorusGeometry(winRadius, 16, 8, 32, Math.PI);
      const winFrame = new THREE.Mesh(winFrameGeo, steelMat);
      winFrame.position.set(cx, winCenterY, z + (z < cz ? 28 : -28));
      this.trainHallGroup.add(winFrame);

      // Sprossen (Radiärspeichen alle 30 Grad)
      for (let a = 0; a <= 6; a++) {
        const rad = (a * Math.PI) / 6;
        const spokeGeo = new THREE.BoxGeometry(winRadius, 8, 12);
        const spoke = new THREE.Mesh(spokeGeo, steelMat);
        spoke.position.set(
          cx + (Math.cos(rad) * winRadius) / 2,
          winCenterY + (Math.sin(rad) * winRadius) / 2,
          z + (z < cz ? 28 : -28),
        );
        spoke.rotation.z = rad;
        this.trainHallGroup.add(spoke);
      }
    }

    // E. Historische Schweizer Bahnhofsuhr an der Stirnwand (oben zentriert)
    const clockZ = cz - lHall / 2 + 32;
    const clockY = apexH - 360;

    // Uhrengehäuse
    const clockRim = new THREE.Mesh(
      new THREE.RingGeometry(170, 195, 32),
      new THREE.MeshStandardMaterial({ color: 0x18181b, metalness: 0.8, roughness: 0.3 }),
    );
    clockRim.position.set(cx, clockY, clockZ);
    this.trainHallGroup.add(clockRim);

    // Zifferblatt (weiß)
    const clockFace = new THREE.Mesh(new THREE.CircleGeometry(170, 32), new THREE.MeshBasicMaterial({ color: 0xf8fafc }));
    clockFace.position.set(cx, clockY, clockZ + 1);
    this.trainHallGroup.add(clockFace);

    // 12 Stunden-Striche
    const tickMat = new THREE.MeshBasicMaterial({ color: 0x09090b });
    for (let h = 0; h < 12; h++) {
      const rad = (h * Math.PI) / 6;
      const isQuarter = h % 3 === 0;
      const tick = new THREE.Mesh(new THREE.BoxGeometry(isQuarter ? 8 : 4, isQuarter ? 26 : 16, 2), tickMat);
      tick.position.set(cx + Math.cos(rad) * 145, clockY + Math.sin(rad) * 145, clockZ + 2);
      tick.rotation.z = rad + Math.PI / 2;
      this.trainHallGroup.add(tick);
    }

    // Zeiger (Stunden- und Minutenzeiger)
    const handH = new THREE.Mesh(new THREE.BoxGeometry(7, 85, 2), tickMat);
    handH.position.set(cx + 25, clockY + 25, clockZ + 3);
    handH.rotation.z = Math.PI / 4;
    this.trainHallGroup.add(handH);

    const handM = new THREE.Mesh(new THREE.BoxGeometry(5, 120, 2), tickMat);
    handM.position.set(cx - 35, clockY + 35, clockZ + 4);
    handM.rotation.z = -Math.PI / 6;
    this.trainHallGroup.add(handM);

    // Roter Sekundenzeiger mit Kelle
    const redMat = new THREE.MeshBasicMaterial({ color: 0xef4444 });
    const handS = new THREE.Mesh(new THREE.BoxGeometry(3, 130, 2), redMat);
    handS.position.set(cx, clockY + 45, clockZ + 5);
    handS.rotation.z = -Math.PI / 1.5;
    this.trainHallGroup.add(handS);

    const secondDot = new THREE.Mesh(new THREE.CircleGeometry(12, 16), redMat);
    secondDot.position.set(cx - 45, clockY + 80, clockZ + 6);
    this.trainHallGroup.add(secondDot);
  }

  private buildTrainHallSideWalls(
    cx: number,
    cz: number,
    wHall: number,
    lHall: number,
    floorY: number,
    wallH: number,
    brickMat: THREE.Material,
    windowGlassMat: THREE.Material,
    steelMat: THREE.Material,
  ) {
    const halfW = wHall / 2;
    const sideXPositions = [cx - halfW, cx + halfW];

    for (const x of sideXPositions) {
      // A. Unterer Ziegelsockel
      const plinthH = 500;
      const plinth = new THREE.Mesh(new THREE.BoxGeometry(40, plinthH, lHall), brickMat);
      plinth.position.set(x, floorY + plinthH / 2, cz);
      plinth.receiveShadow = true;
      this.trainHallGroup.add(plinth);

      // B. Große Fabrikfenster zwischen den Stützen
      const winH = wallH - (floorY + plinthH);
      const winY = floorY + plinthH + winH / 2;

      const glassWall = new THREE.Mesh(new THREE.BoxGeometry(10, winH, lHall - 100), windowGlassMat);
      glassWall.position.set(x, winY, cz);
      this.trainHallGroup.add(glassWall);

      // Fenster-Muntins (Sprossen alle 750 mm)
      const count = Math.floor(lHall / 750);
      for (let i = 0; i < count; i++) {
        const wz = cz - lHall / 2 + i * 750;
        const barV = new THREE.Mesh(new THREE.BoxGeometry(16, winH, 16), steelMat);
        barV.position.set(x, winY, wz);
        this.trainHallGroup.add(barV);
      }
    }
  }

  private buildTrainHallCrane(
    cx: number,
    cz: number,
    wHall: number,
    lHall: number,
    floorY: number,
    craneYellowMat: THREE.Material,
    steelMat: THREE.Material,
  ) {
    const craneZ = cz - lHall * 0.22;
    const craneY = floorY + 1600;
    const craneSpan = wHall - 140;

    // 1. Zwei gelbe Doppel-T-Hauptträger des Brückenkrans
    const beamGeo = new THREE.BoxGeometry(craneSpan, 110, 36);

    const beam1 = new THREE.Mesh(beamGeo, craneYellowMat);
    beam1.position.set(cx, craneY, craneZ - 90);
    beam1.castShadow = true;
    this.trainHallGroup.add(beam1);

    const beam2 = new THREE.Mesh(beamGeo, craneYellowMat);
    beam2.position.set(cx, craneY, craneZ + 90);
    beam2.castShadow = true;
    this.trainHallGroup.add(beam2);

    // 2. Kopfträger / Fahrwerke links und rechts
    const truckGeo = new THREE.BoxGeometry(80, 80, 320);
    const truckL = new THREE.Mesh(truckGeo, steelMat);
    truckL.position.set(cx - craneSpan / 2, craneY, craneZ);
    this.trainHallGroup.add(truckL);

    const truckR = new THREE.Mesh(truckGeo, steelMat);
    truckR.position.set(cx + craneSpan / 2, craneY, craneZ);
    this.trainHallGroup.add(truckR);

    // 3. Laufkatze (Hubwerk) mittig/leicht versetzt
    const trolleyX = cx + 500;
    const trolley = new THREE.Mesh(
      new THREE.BoxGeometry(180, 75, 150),
      new THREE.MeshStandardMaterial({ color: 0x1f2937, roughness: 0.5, metalness: 0.8 }),
    );
    trolley.position.set(trolleyX, craneY + 50, craneZ);
    this.trainHallGroup.add(trolley);

    // 4. Stahlseile nach unten
    const cableGeo = new THREE.CylinderGeometry(2, 2, 450, 4);
    const cableMat = new THREE.MeshBasicMaterial({ color: 0x475569 });
    const cable1 = new THREE.Mesh(cableGeo, cableMat);
    cable1.position.set(trolleyX - 25, craneY - 180, craneZ);
    this.trainHallGroup.add(cable1);

    const cable2 = new THREE.Mesh(cableGeo, cableMat);
    cable2.position.set(trolleyX + 25, craneY - 180, craneZ);
    this.trainHallGroup.add(cable2);

    // 5. Flaschenzug & Kranhaken
    const block = new THREE.Mesh(
      new THREE.BoxGeometry(70, 60, 45),
      new THREE.MeshStandardMaterial({ color: 0xf59e0b, roughness: 0.4 }),
    );
    block.position.set(trolleyX, craneY - 420, craneZ);
    this.trainHallGroup.add(block);

    const hookGeo = new THREE.TorusGeometry(32, 9, 8, 20, Math.PI * 1.35);
    const hook = new THREE.Mesh(hookGeo, new THREE.MeshStandardMaterial({ color: 0x111827, metalness: 0.9 }));
    hook.position.set(trolleyX, craneY - 480, craneZ);
    hook.rotation.z = Math.PI * 0.75;
    this.trainHallGroup.add(hook);
  }

  private buildTrainHallLamps(
    cx: number,
    cz: number,
    wHall: number,
    lHall: number,
    floorY: number,
    apexH: number,
    steelMat: THREE.Material,
  ) {
    const lampBays = [cz - lHall * 0.32, cz, cz + lHall * 0.32];
    const lampOffsetsX = [-wHall * 0.22, wHall * 0.22];
    const lampY = floorY + 2400;

    const shadeMat = new THREE.MeshStandardMaterial({ color: 0x1e293b, roughness: 0.4, metalness: 0.6 });
    const bulbMat = new THREE.MeshStandardMaterial({
      color: 0xffedd5,
      emissive: 0xfb923c,
      emissiveIntensity: 0.8,
      roughness: 0.2,
    });
    this.nightLampMats.push(bulbMat);

    for (const z of lampBays) {
      for (const offX of lampOffsetsX) {
        const lx = cx + offX;

        // Pendelseil von der Decke
        const cableH = apexH - lampY;
        const cableGeo = new THREE.CylinderGeometry(2, 2, cableH, 4);
        const cable = new THREE.Mesh(cableGeo, steelMat);
        cable.position.set(lx, lampY + cableH / 2, z);
        this.trainHallGroup.add(cable);

        // Industrielle Tiefstrahler-Glocke (Reflektor)
        const shadeGeo = new THREE.CylinderGeometry(18, 75, 55, 16, 1, true);
        const shade = new THREE.Mesh(shadeGeo, shadeMat);
        shade.position.set(lx, lampY + 20, z);
        this.trainHallGroup.add(shade);

        // Warm leuchtendes Leuchtmittel
        const bulb = new THREE.Mesh(new THREE.SphereGeometry(18, 12, 12), bulbMat);
        bulb.position.set(lx, lampY, z);
        this.trainHallGroup.add(bulb);

        // Sanftes warmes Hallen-Akzentlicht
        const pointLight = new THREE.PointLight(0xffedd5, 0.45, 4500, 1.4);
        pointLight.position.set(lx, lampY - 10, z);
        this.trainHallGroup.add(pointLight);
      }
    }
  }

  /**
   * Baut einen dichten Nadel- und Mischwald rund um die Außenseite der Lokhalle.
   * Die Baumkronen und Stämme ragen an den großen Fabrikfenstern und Toren empor,
   * sodass beim Blick aus den Hallenfenstern eine lebendige Waldlandschaft sichtbar ist.
   */
  private buildTrainHallForest(cx: number, cz: number, wHall: number, lHall: number, floorY: number) {
    const isDark = this.currentTheme === "dark";
    const halfW = wHall / 2;
    const halfL = lHall / 2;

    // A. Weitläufiger Waldboden bis zum Horizont (Radius 160.000 mm)
    const forestFloorGeo = new THREE.CircleGeometry(160000, 64);
    forestFloorGeo.rotateX(-Math.PI / 2);
    const forestFloorMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x132616 : 0x2e5425,
      roughness: 0.95,
      metalness: 0.04,
    });
    const forestFloor = new THREE.Mesh(forestFloorGeo, forestFloorMat);
    forestFloor.position.set(cx, floorY - 3, cz);
    forestFloor.receiveShadow = true;
    this.trainHallGroup.add(forestFloor);

    // Baum- & Natur-Materialien
    const barkMat = new THREE.MeshStandardMaterial({ color: 0x3f2e21, roughness: 0.9 });
    const pineMat1 = new THREE.MeshStandardMaterial({ color: isDark ? 0x052e16 : 0x14532d, roughness: 0.95 });
    const pineMat2 = new THREE.MeshStandardMaterial({ color: isDark ? 0x064e3b : 0x166534, roughness: 0.95 });
    const leafMat = new THREE.MeshStandardMaterial({ color: isDark ? 0x166534 : 0x15803d, roughness: 0.9 });
    const bushMat = new THREE.MeshStandardMaterial({ color: isDark ? 0x052e16 : 0x166534, roughness: 0.95 });
    const hillHorizonMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x0f1e14 : 0x223e1d,
      roughness: 0.95,
    });

    // Shared Geometrien für performantes Rendering vieler Bäume (ausgedehnte Waldlandschaft)
    const pineTrunkGeo = new THREE.CylinderGeometry(18, 28, 220, 8);
    pineTrunkGeo.translate(0, 110, 0);
    const c1Geo = new THREE.ConeGeometry(170, 240, 8);
    c1Geo.translate(0, 230, 0);
    const c2Geo = new THREE.ConeGeometry(140, 220, 8);
    c2Geo.translate(0, 360, 0);
    const c3Geo = new THREE.ConeGeometry(105, 200, 8);
    c3Geo.translate(0, 480, 0);
    const c4Geo = new THREE.ConeGeometry(70, 180, 8);
    c4Geo.translate(0, 590, 0);

    const decTrunkGeo = new THREE.CylinderGeometry(20, 32, 260, 8);
    decTrunkGeo.translate(0, 130, 0);
    const crown1Geo = new THREE.DodecahedronGeometry(190, 1);
    crown1Geo.translate(0, 390, 0);
    const crown2Geo = new THREE.DodecahedronGeometry(140, 1);
    crown2Geo.translate(70, 350, 40);

    // Helfer für einen Baum mit Stamm und Nadelkrone (auf dem grünen Boden stehend)
    const addPineTree = (x: number, z: number, scale: number) => {
      const tree = new THREE.Group();
      tree.position.set(x, floorY, z);
      tree.scale.setScalar(scale);

      const trunk = new THREE.Mesh(pineTrunkGeo, barkMat);
      trunk.castShadow = true;
      tree.add(trunk);

      const c1 = new THREE.Mesh(c1Geo, pineMat1);
      c1.castShadow = true;
      tree.add(c1);

      const c2 = new THREE.Mesh(c2Geo, pineMat2);
      c2.castShadow = true;
      tree.add(c2);

      const c3 = new THREE.Mesh(c3Geo, pineMat1);
      c3.castShadow = true;
      tree.add(c3);

      const c4 = new THREE.Mesh(c4Geo, pineMat2);
      c4.castShadow = true;
      tree.add(c4);

      this.trainHallGroup.add(tree);
    };

    // Helfer für einen Laubbaum (auf dem grünen Boden stehend)
    const addDeciduousTree = (x: number, z: number, scale: number) => {
      const tree = new THREE.Group();
      tree.position.set(x, floorY, z);
      tree.scale.setScalar(scale);

      const trunk = new THREE.Mesh(decTrunkGeo, barkMat);
      trunk.castShadow = true;
      tree.add(trunk);

      const crown1 = new THREE.Mesh(crown1Geo, leafMat);
      crown1.castShadow = true;
      tree.add(crown1);

      const crown2 = new THREE.Mesh(crown2Geo, leafMat);
      crown2.castShadow = true;
      tree.add(crown2);

      this.trainHallGroup.add(tree);
    };

    // 1. WESTSEITE: Riesiges, tiefes und weitläufiges Waldmassiv ("more on one side & spread wider")
    // Reicht von 2.000 mm bis über 55.000 mm weit in die Landschaft und über 70.000 mm entlang der Z-Achse.
    const countZ = Math.floor(lHall / 650);

    // A. Waldsaum nahe der Halle (mit 1.900–2.400 mm Sicherheitsabstand zum Mauerwerk)
    for (let i = -5; i <= countZ + 5; i++) {
      const z = cz - halfL + i * 650 + ((i * 123) % 210);
      const x1 = cx - halfW - 2000 - ((i * 87) % 350);
      const scale1 = 3.8 + ((i * 17) % 8) * 0.2;
      if (i % 3 === 0) {
        addDeciduousTree(x1, z, scale1 * 0.95);
      } else {
        addPineTree(x1, z, scale1);
      }
    }

    // B. Ausgedehnter Mittel- und Tiefwald nach Westen (viele Baumgruppen, weit gestreut)
    // 75 weit verstreute Baumkoordinaten über die gesamte westliche Ebene
    const westForestPoints: [number, number, number, boolean][] = [];
    const seedPoints = 72;
    for (let k = 0; k < seedPoints; k++) {
      // Entfernung von der Halle nach Westen: von 3.600 mm bis 52.000 mm
      const distProgress = (k % 6) / 5; // 0, 0.2, 0.4, 0.6, 0.8, 1.0
      const baseX = 3600 + distProgress * 46000 + ((k * 1317) % 3500);
      const x = cx - halfW - baseX;

      // Z-Ausbreitung von -35.000 mm bis +35.000 mm
      const zRatio = (Math.floor(k / 6) / 12) - 0.5; // -0.5 bis +0.5
      const z = cz + zRatio * 68000 + ((k * 789) % 3200);

      const scale = 4.4 + distProgress * 2.2 + ((k * 37) % 7) * 0.25;
      const isDeciduous = k % 4 === 1;
      westForestPoints.push([x, z, scale, isDeciduous]);
    }

    for (const [fx, fz, fscale, fdec] of westForestPoints) {
      if (fdec) {
        addDeciduousTree(fx, fz, fscale * 0.9);
      } else {
        addPineTree(fx, fz, fscale);
      }
    }

    // C. Dichte Fichten-Horizontkette im fernen Nordwesten (30.000 bis 55.000 mm)
    for (let r = 0; r < 24; r++) {
      const angle = Math.PI * 0.85 + (r / 24) * Math.PI * 0.6;
      const dist = 38000 + ((r * 1543) % 22000);
      const hx = cx + Math.cos(angle) * dist;
      const hz = cz + Math.sin(angle) * dist;
      addPineTree(hx, hz, 6.2 + ((r * 29) % 6) * 0.35);
    }

    // 2. OSTSEITE: Weite, offene Wiesenlandschaft mit nur wenigen, malerischen Baumgruppen
    // (Asymmetrischer Kontrast: Ostseite bleibt frei und sonnig, mit freiem Blick)
    const eastSparseTrees: [number, number, number, boolean][] = [
      // Gruppe 1: kleine Baumgruppe im Südosten (neben der Zufahrtsstraße)
      [cx + halfW + 5800, cz + 4800, 4.2, false],
      [cx + halfW + 6400, cz + 5400, 3.8, true],
      [cx + halfW + 7100, cz + 4400, 4.6, false],

      // Gruppe 2: lockerer Baumbestand im Nordosten
      [cx + halfW + 7200, cz - 3500, 4.8, false],
      [cx + halfW + 8100, cz - 4100, 4.0, true],
      [cx + halfW + 9000, cz - 3200, 5.2, false],
      [cx + halfW + 10200, cz - 4600, 4.5, false],

      // Einzelbäume (Solitärbäume auf der Wiese & am Straßenrand)
      [cx + halfW + 3600, cz - 1400, 4.3, true],   // Schöne alte Solitäreiche am Straßenrand
      [cx + halfW + 5800, cz - 7800, 4.6, false],
      [cx + halfW + 12500, cz + 8500, 5.4, false],
      [cx + halfW + 16000, cz - 12000, 5.8, true],

      // Ferne Baumgruppe am östlichen Horizontrand (35.000–45.000 mm)
      [cx + halfW + 36000, cz - 6000, 6.5, false],
      [cx + halfW + 39000, cz - 4500, 6.8, false],
      [cx + halfW + 43000, cz - 8000, 7.0, true],
      [cx + halfW + 38000, cz + 14000, 6.6, false],
      [cx + halfW + 42000, cz + 16500, 6.9, false],
    ];

    for (const [ex, ez, escale, edec] of eastSparseTrees) {
      if (edec) {
        addDeciduousTree(ex, ez, escale * 0.9);
      } else {
        addPineTree(ex, ez, escale);
      }
    }

    // 3. STIRNSEITEN (Nord & Süd): Wald schmiegt sich an die Westecke, Ostseite bleibt offen
    const endZPositions = [cz - halfL - 600, cz + halfL + 600];
    for (const ez of endZPositions) {
      const signZ = ez < cz ? -1 : 1;

      // Westliche Hälfte der Stirnseiten (starker Waldanschluss)
      for (let j = 0; j < 8; j++) {
        const x = cx - halfW + j * 550 - 1200;
        const z1 = ez + signZ * (2200 + ((j * 113) % 500));
        addPineTree(x, z1, 4.5 + ((j * 17) % 5) * 0.25);

        const z2 = ez + signZ * (5200 + ((j * 179) % 1100));
        addPineTree(x - 300, z2, 5.2 + ((j * 23) % 5) * 0.3);

        const z3 = ez + signZ * (11000 + ((j * 251) % 2200));
        addPineTree(x - 700, z3, 6.0 + ((j * 31) % 5) * 0.35);
      }

      // Östliche Hälfte der Stirnseiten (nur wenige Übergangsbäume)
      const xOpen = cx + halfW * 0.5 + 1200;
      const zOpen = ez + signZ * 3400;
      addPineTree(xOpen, zOpen, 4.4);
      addDeciduousTree(xOpen + 1800, zOpen + signZ * 1200, 4.0);
    }

    // 4. Sanfte Hügelketten am fernen Horizont (Radius 45.000 bis 110.000 mm)
    const horizonHillsCount = 20;
    for (let h = 0; h < horizonHillsCount; h++) {
      const angle = (h / horizonHillsCount) * Math.PI * 2;
      const dist = 55000 + ((h * 1321) % 40000);
      const hx = cx + Math.cos(angle) * dist;
      const hz = cz + Math.sin(angle) * dist;
      const radius = 9000 + ((h * 843) % 7000);
      const height = 2800 + ((h * 617) % 3200);

      const hillGeo = new THREE.CylinderGeometry(0, radius, height, 16, 1, false);
      const hillMesh = new THREE.Mesh(hillGeo, hillHorizonMat);
      hillMesh.position.set(hx, floorY + height / 2 - 100, hz);
      this.trainHallGroup.add(hillMesh);
    }

    // 5. Kleine Zierbüsche am Hallensockel (mit sicherem Abstand zur Wand)
    for (let k = 0; k < 18; k++) {
      const side = k % 2 === 0 ? -1 : 1;
      const bz = cz - halfL + k * (lHall / 16);
      const bx = cx + side * (halfW + 120);
      const bush = new THREE.Mesh(new THREE.DodecahedronGeometry(55, 1), bushMat);
      bush.position.set(bx, floorY + 30, bz);
      this.trainHallGroup.add(bush);
    }
  }



  /**
   * Idyllischer kleiner See / Bergsee vor den Hügeln auf der offenen Wiesenseite (Ostseite):
   * Natürlich geschwungene Wasserfläche mit Himmelsreflexionen, Kiesel- und Sandufer,
   * Uferfelsen / Findlinge, Schilfgürtel, Seerosenblätter und ein rustikaler Holzsteg mit Kahn.
   */
  private buildTrainHallLake(cx: number, cz: number, _wHall: number, _lHall: number, floorY: number) {
    const isDark = this.currentTheme === "dark";

    // See-Position: Auf der Ostseite (wo wenige Bäume stehen) vor den fernen Hügeln
    const lakeX = cx + 25000;
    const lakeZ = cz - 2500;
    const lakeRadiusX = 5200;
    const lakeRadiusZ = 3800;

    // 1. Organische Ufer- und Wasserform
    const lakeShape = new THREE.Shape();
    const pointsCount = 32;
    for (let i = 0; i < pointsCount; i++) {
      const angle = (i / pointsCount) * Math.PI * 2;
      // Sanfte Einbuchtungen und Ausbuchtungen für eine organische Uferlinie
      const noise = 1 + Math.sin(angle * 2) * 0.14 + Math.cos(angle * 3) * 0.08 + Math.sin(angle * 5) * 0.05;
      const px = Math.cos(angle) * lakeRadiusX * noise;
      const pz = Math.sin(angle) * lakeRadiusZ * noise;
      if (i === 0) {
        lakeShape.moveTo(px, pz);
      } else {
        lakeShape.lineTo(px, pz);
      }
    }
    lakeShape.closePath();

    // 2. Kiesel- und Sandstrand um den See (deutlich über Waldboden angehoben gegen Artefakte)
    const beachMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x3f3f46 : 0xa89f81,
      roughness: 0.95,
      metalness: 0.05,
      polygonOffset: true,
      polygonOffsetFactor: -1,
      polygonOffsetUnits: -1,
    });
    const beachGeo = new THREE.ShapeGeometry(lakeShape);
    beachGeo.rotateX(-Math.PI / 2);
    const beachMesh = new THREE.Mesh(beachGeo, beachMat);
    beachMesh.scale.set(1.15, 1, 1.15); // Etwas breiter als der See selbst
    beachMesh.position.set(lakeX, floorY + 14, lakeZ);
    beachMesh.receiveShadow = true;
    this.trainHallGroup.add(beachMesh);

    // 3. Wasserfläche (spiegelndes Bergseewasser, sauber über dem Strand platziert)
    const waterMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x0369a1 : 0x0284c7,
      roughness: 0.08,
      metalness: 0.85,
      transparent: true,
      opacity: 0.88,
      polygonOffset: true,
      polygonOffsetFactor: -2,
      polygonOffsetUnits: -2,
    });
    const waterGeo = new THREE.ShapeGeometry(lakeShape);
    waterGeo.rotateX(-Math.PI / 2);
    const waterMesh = new THREE.Mesh(waterGeo, waterMat);
    waterMesh.position.set(lakeX, floorY + 18, lakeZ);
    waterMesh.receiveShadow = true;
    this.trainHallGroup.add(waterMesh);

    // 4. Ufersteine & Granitblöcke (Findlinge) am Wasser
    const rockMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x334155 : 0x64748b,
      roughness: 0.9,
    });
    const rocksCount = 18;
    for (let r = 0; r < rocksCount; r++) {
      const angle = (r / rocksCount) * Math.PI * 2;
      const noise = 1 + Math.sin(angle * 2) * 0.14 + Math.cos(angle * 3) * 0.08;
      const distOffset = 1.02 + ((r * 17) % 15) * 0.01;
      const rx = lakeX + Math.cos(angle) * (lakeRadiusX * noise * distOffset);
      const rz = lakeZ + Math.sin(angle) * (lakeRadiusZ * noise * distOffset);
      const rRadius = 90 + ((r * 29) % 120);

      const rock = new THREE.Mesh(new THREE.DodecahedronGeometry(rRadius, 1), rockMat);
      rock.scale.set(1.3, 0.75, 1.1);
      rock.position.set(rx, floorY + 16 + rRadius * 0.45, rz);
      rock.rotation.y = (r * 37) % 3;
      rock.castShadow = true;
      rock.receiveShadow = true;
      this.trainHallGroup.add(rock);
    }

    // 5. Schilfgürtel & Binsen in den seichten Uferbuchten
    const reedMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x064e3b : 0x15803d,
      roughness: 0.9,
    });
    const reedTopMat = new THREE.MeshStandardMaterial({
      color: 0x3f2e21,
      roughness: 0.95,
    });

    const reedBeds = [
      { baseAngle: Math.PI * 0.25, count: 20 },
      { baseAngle: Math.PI * 0.85, count: 18 },
      { baseAngle: Math.PI * 1.6, count: 22 },
    ];

    for (const bed of reedBeds) {
      for (let s = 0; s < bed.count; s++) {
        const angle = bed.baseAngle + ((s - bed.count / 2) / bed.count) * 0.35;
        const noise = 1 + Math.sin(angle * 2) * 0.14 + Math.cos(angle * 3) * 0.08;
        const offset = 0.94 + ((s * 13) % 10) * 0.012;
        const sx = lakeX + Math.cos(angle) * (lakeRadiusX * noise * offset);
        const sz = lakeZ + Math.sin(angle) * (lakeRadiusZ * noise * offset);
        const stemH = 160 + ((s * 23) % 80);

        const stem = new THREE.Mesh(new THREE.CylinderGeometry(2, 3.5, stemH, 5), reedMat);
        stem.position.set(sx, floorY + 16 + stemH / 2, sz);
        stem.rotation.z = (((s * 11) % 7) - 3) * 0.04;
        this.trainHallGroup.add(stem);

        // Schilfkolben oben
        if (s % 2 === 0) {
          const cattail = new THREE.Mesh(new THREE.CylinderGeometry(5, 5, 35, 6), reedTopMat);
          cattail.position.set(sx, floorY + 16 + stemH - 15, sz);
          this.trainHallGroup.add(cattail);
        }
      }
    }

    // 6. Kleiner hölzerner Boots- & Badesteg (ragt in den See)
    const pierMat = new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.85 });
    const pierX = lakeX - lakeRadiusX * 0.7;
    const pierZ = lakeZ + lakeRadiusZ * 0.2;
    const pierLen = 1400;
    const pierW = 380;

    // 4 Holzpfähle im Seeboden
    for (const dx of [pierLen * 0.35, pierLen * 0.85]) {
      for (const dz of [-pierW * 0.35, pierW * 0.35]) {
        const piling = new THREE.Mesh(new THREE.CylinderGeometry(10, 12, 120, 6), pierMat);
        piling.position.set(pierX + dx, floorY + 16 + 45, pierZ + dz);
        piling.castShadow = true;
        this.trainHallGroup.add(piling);
      }
    }

    // 2 Längsträger
    for (const dz of [-pierW * 0.32, pierW * 0.32]) {
      const beam = new THREE.Mesh(new THREE.BoxGeometry(pierLen, 14, 18), pierMat);
      beam.position.set(pierX + pierLen / 2, floorY + 18 + 80, pierZ + dz);
      this.trainHallGroup.add(beam);
    }

    // Querbohlen (Holzbretter des Stegs)
    const plankCount = Math.floor(pierLen / 55);
    for (let p = 0; p < plankCount; p++) {
      const plank = new THREE.Mesh(new THREE.BoxGeometry(45, 10, pierW), pierMat);
      plank.position.set(pierX + 30 + p * 55, floorY + 18 + 92, pierZ);
      plank.receiveShadow = true;
      this.trainHallGroup.add(plank);
    }

    // 7. Kleines Holz-Ruderboot (am Steg vertäut)
    const boatGroup = new THREE.Group();
    boatGroup.position.set(pierX + pierLen * 0.85, floorY + 18 + 12, pierZ + pierW * 1.15);
    boatGroup.rotation.y = 0.25;

    const hullMat = new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.8 });
    const hull = new THREE.Mesh(new THREE.BoxGeometry(550, 80, 220), hullMat);
    hull.position.set(0, 30, 0);
    boatGroup.add(hull);

    const bow = new THREE.Mesh(new THREE.ConeGeometry(110, 160, 4), hullMat);
    bow.rotation.z = -Math.PI / 2;
    bow.position.set(340, 30, 0);
    boatGroup.add(bow);

    const bench = new THREE.Mesh(new THREE.BoxGeometry(35, 8, 200), pierMat);
    bench.position.set(0, 55, 0);
    boatGroup.add(bench);

    this.trainHallGroup.add(boatGroup);

    // 8. Seerosenblätter auf der ruhigen Wasseroberfläche
    const lilyMat = new THREE.MeshStandardMaterial({
      color: isDark ? 0x166534 : 0x22c55e,
      roughness: 0.8,
    });
    const lilyFlowerMat = new THREE.MeshBasicMaterial({ color: 0xfdf4ff });

    const lilyClusters = [
      { x: lakeX + lakeRadiusX * 0.35, z: lakeZ - lakeRadiusZ * 0.4 },
      { x: lakeX - lakeRadiusX * 0.25, z: lakeZ - lakeRadiusZ * 0.5 },
      { x: lakeX + lakeRadiusX * 0.5, z: lakeZ + lakeRadiusZ * 0.25 },
    ];

    for (const cl of lilyClusters) {
      for (let l = 0; l < 4; l++) {
        const lx = cl.x + ((l * 47) % 110) - 55;
        const lz = cl.z + ((l * 67) % 110) - 55;
        const pad = new THREE.Mesh(new THREE.CircleGeometry(40 + (l % 3) * 12, 8), lilyMat);
        pad.rotation.x = -Math.PI / 2;
        pad.position.set(lx, floorY + 18 + 1.2, lz);
        this.trainHallGroup.add(pad);

        if (l === 0 || l === 2) {
          const flower = new THREE.Mesh(new THREE.DodecahedronGeometry(12, 0), lilyFlowerMat);
          flower.position.set(lx, floorY + 18 + 8, lz);
          this.trainHallGroup.add(flower);
        }
      }
    }
  }

  /**
   * Erzeugt eine realistische, malerische Natur-Landschaft um die Modellbahnanlage:
   * Sanfte grüne Hügel, Bergketten im Hintergrund, ein Gebirgsfluss mit Steinbogenbrücke,
   * geschwungene Landstraßen, Mischwälder und Wolken.
   */
  private buildRealisticLandscape(layout: Layout) {
    let minX = 0,
      maxX = 0,
      minZ = 0,
      maxZ = 0;
    let hasBounds = false;

    const baseboards = getBaseboards(layout);
    for (const b of baseboards) {
      if (!b.enabled) continue;
      const bounds = getBaseboardBounds(b);
      const bMinX = bounds.x;
      const bMaxX = bounds.x + bounds.width;
      const bMinZ = bounds.y;
      const bMaxZ = bounds.y + bounds.depth;
      if (!hasBounds) {
        minX = bMinX;
        maxX = bMaxX;
        minZ = bMinZ;
        maxZ = bMaxZ;
        hasBounds = true;
      } else {
        if (bMinX < minX) minX = bMinX;
        if (bMaxX > maxX) maxX = bMaxX;
        if (bMinZ < minZ) minZ = bMinZ;
        if (bMaxZ > maxZ) maxZ = bMaxZ;
      }
    }

    if (!hasBounds && layout.pieces.length > 0) {
      for (const p of layout.pieces) {
        if (!hasBounds) {
          minX = p.transform.x;
          maxX = p.transform.x;
          minZ = p.transform.y;
          maxZ = p.transform.y;
          hasBounds = true;
        } else {
          if (p.transform.x < minX) minX = p.transform.x;
          if (p.transform.x > maxX) maxX = p.transform.x;
          if (p.transform.y < minZ) minZ = p.transform.y;
          if (p.transform.y > maxZ) maxZ = p.transform.y;
        }
      }
      minX -= 250;
      maxX += 250;
      minZ -= 250;
      maxZ += 250;
    }

    const cx = (minX + maxX) / 2;
    const cz = (minZ + maxZ) / 2;
    const wTable = Math.max(1400, maxX - minX);
    const lTable = Math.max(1200, maxZ - minZ);
    const floorY = -755; // Niveau unter den 720 mm Tischbeinen

    const isDark = this.currentTheme === "dark";

    // Gemeinsame Landschafts-Materialien
    const meadowColor = isDark ? 0x1e3a1f : 0x4d7c0f;
    const hillColor = isDark ? 0x142817 : 0x3f6212;
    const rockColor = isDark ? 0x334155 : 0x64748b;
    const snowColor = isDark ? 0x94a3b8 : 0xf8fafc;
    const riverColor = isDark ? 0x0369a1 : 0x0284c7;
    const roadColor = isDark ? 0x18181b : 0x27272a;
    const treeBarkColor = 0x3f2e21;
    const pineGreen1 = isDark ? 0x052e16 : 0x14532d;
    const pineGreen2 = isDark ? 0x064e3b : 0x166534;
    const leafGreen1 = isDark ? 0x166534 : 0x15803d;
    const leafGreen2 = isDark ? 0x15803d : 0x65a30d;

    const meadowMat = new THREE.MeshStandardMaterial({ color: meadowColor, roughness: 0.9, metalness: 0.05 });
    const hillMat = new THREE.MeshStandardMaterial({ color: hillColor, roughness: 0.92, metalness: 0.04 });
    const rockMat = new THREE.MeshStandardMaterial({ color: rockColor, roughness: 0.95, metalness: 0.1 });
    const snowMat = new THREE.MeshStandardMaterial({ color: snowColor, roughness: 0.8, metalness: 0.05 });
    const riverMat = new THREE.MeshStandardMaterial({
      color: riverColor,
      roughness: 0.1,
      metalness: 0.8,
      transparent: true,
      opacity: 0.88,
    });
    const riverBankMat = new THREE.MeshStandardMaterial({ color: 0x78716c, roughness: 0.9 });
    const roadMat = new THREE.MeshStandardMaterial({ color: roadColor, roughness: 0.85, metalness: 0.1 });
    const roadMarkMat = new THREE.MeshBasicMaterial({ color: 0xf8fafc });
    const bridgeStoneMat = new THREE.MeshStandardMaterial({ color: 0x71717a, roughness: 0.9 });
    const barkMat = new THREE.MeshStandardMaterial({ color: treeBarkColor, roughness: 0.9 });
    const pineMat1 = new THREE.MeshStandardMaterial({ color: pineGreen1, roughness: 0.95 });
    const pineMat2 = new THREE.MeshStandardMaterial({ color: pineGreen2, roughness: 0.95 });
    const leafMat1 = new THREE.MeshStandardMaterial({ color: leafGreen1, roughness: 0.9 });
    const leafMat2 = new THREE.MeshStandardMaterial({ color: leafGreen2, roughness: 0.9 });

    // 1. Weitläufiges Talboden-Gelände bis zum Horizont (Radius 180.000 mm)
    const worldPlaneGeo = new THREE.CircleGeometry(180000, 64);
    worldPlaneGeo.rotateX(-Math.PI / 2);
    const worldPlane = new THREE.Mesh(worldPlaneGeo, meadowMat);
    worldPlane.position.set(cx, floorY - 2, cz);
    worldPlane.receiveShadow = true;
    this.landscapeGroup.add(worldPlane);

    // 2. Organischer Geländesockel / Böschungsübergang der Anlage (Plateau)
    this.buildLandscapeEmbankment(minX, maxX, minZ, maxZ, floorY, meadowMat, rockMat);

    // 3. Hügelketten & Berge (Rolling Hills & Mountain Range)
    this.buildLandscapeHills(cx, cz, wTable, lTable, floorY, hillMat, rockMat, snowMat);

    // 4. Gebirgsfluss mit Ufer
    this.buildLandscapeRiver(cx, cz, wTable, lTable, floorY, riverMat, riverBankMat);

    // 5. Steinbogenbrücke über den Fluss
    this.buildLandscapeBridge(cx, cz, wTable, lTable, floorY, bridgeStoneMat, roadMat);

    // 6. Geschwungene Landstraße & Allee
    this.buildLandscapeRoad(cx, cz, wTable, lTable, floorY, roadMat, roadMarkMat);

    // 7. Wälder & Baumgruppen (Mischwälder)
    this.buildLandscapeForest(cx, cz, wTable, lTable, floorY, barkMat, pineMat1, pineMat2, leafMat1, leafMat2);

    // 8. Almhütte in den Hügeln
    this.buildLandscapeCottage(cx, cz, wTable, lTable, floorY);

    // 9. 3D-Wolken am Himmel
    this.buildLandscapeClouds(cx, cz);
  }

  /** Natürliche Geländeböschung vom Anlagentisch nach unten ins Tal */
  private buildLandscapeEmbankment(
    minX: number,
    maxX: number,
    minZ: number,
    maxZ: number,
    floorY: number,
    meadowMat: THREE.Material,
    rockMat: THREE.Material,
  ) {
    const slopeOut = 550;
    const topY = -15;
    const botY = floorY;
    const height = topY - botY;

    // 4 Böschungskeile um den Rand des Anlagentisches
    // Nord-Böschung
    const wN = maxX - minX + slopeOut * 2;
    const slopeNGeo = new THREE.BoxGeometry(wN, height, slopeOut);
    const slopeN = new THREE.Mesh(slopeNGeo, meadowMat);
    slopeN.position.set((minX + maxX) / 2, (topY + botY) / 2, minZ - slopeOut / 2);
    slopeN.rotation.x = -0.35;
    slopeN.receiveShadow = true;
    this.landscapeGroup.add(slopeN);

    // Süd-Böschung
    const slopeS = new THREE.Mesh(slopeNGeo, meadowMat);
    slopeS.position.set((minX + maxX) / 2, (topY + botY) / 2, maxZ + slopeOut / 2);
    slopeS.rotation.x = 0.35;
    slopeS.receiveShadow = true;
    this.landscapeGroup.add(slopeS);

    // West-Böschung
    const lW = maxZ - minZ;
    const slopeWGeo = new THREE.BoxGeometry(slopeOut, height, lW);
    const slopeW = new THREE.Mesh(slopeWGeo, meadowMat);
    slopeW.position.set(minX - slopeOut / 2, (topY + botY) / 2, (minZ + maxZ) / 2);
    slopeW.rotation.z = 0.35;
    slopeW.receiveShadow = true;
    this.landscapeGroup.add(slopeW);

    // Ost-Böschung
    const slopeE = new THREE.Mesh(slopeWGeo, meadowMat);
    slopeE.position.set(maxX + slopeOut / 2, (topY + botY) / 2, (minZ + maxZ) / 2);
    slopeE.rotation.z = -0.35;
    slopeE.receiveShadow = true;
    this.landscapeGroup.add(slopeE);

    // Felsvorsprünge / Granitblöcke an den Ecken
    const rockCorners = [
      { x: minX - 100, z: minZ - 100, r: 65 },
      { x: maxX + 100, z: minZ - 100, r: 75 },
      { x: minX - 100, z: maxZ + 100, r: 70 },
      { x: maxX + 100, z: maxZ + 100, r: 80 },
    ];
    for (const rc of rockCorners) {
      const rock = new THREE.Mesh(new THREE.DodecahedronGeometry(rc.r, 1), rockMat);
      rock.scale.set(1.4, 0.8, 1.2);
      rock.position.set(rc.x, botY + 45, rc.z);
      rock.castShadow = true;
      rock.receiveShadow = true;
      this.landscapeGroup.add(rock);
    }
  }

  /** Hügelketten im Mittelgrund und Bergmassiv im Hintergrund */
  private buildLandscapeHills(
    cx: number,
    cz: number,
    wTable: number,
    lTable: number,
    floorY: number,
    hillMat: THREE.Material,
    rockMat: THREE.Material,
    snowMat: THREE.Material,
  ) {
    // A. Sanft geschwungene grüne Hügel im Osten & Westen
    const hills = [
      // Östliche Hügelkette
      { x: cx + wTable * 0.85 + 1800, z: cz - lTable * 0.4, r: 3200, h: 850 },
      { x: cx + wTable * 0.95 + 2600, z: cz + lTable * 0.35, r: 3600, h: 950 },
      { x: cx + wTable * 0.75 + 1200, z: cz + lTable * 0.8, r: 2800, h: 720 },
      // Westliche Hügelkette
      { x: cx - wTable * 0.85 - 1900, z: cz - lTable * 0.3, r: 3400, h: 920 },
      { x: cx - wTable * 0.95 - 2700, z: cz + lTable * 0.4, r: 3800, h: 980 },
      { x: cx - wTable * 0.75 - 1300, z: cz - lTable * 0.75, r: 2900, h: 740 },
    ];

    for (const h of hills) {
      const hillGeo = new THREE.CylinderGeometry(0, h.r, h.h, 24, 1, false);
      const hillMesh = new THREE.Mesh(hillGeo, hillMat);
      hillMesh.position.set(h.x, floorY + h.h / 2, h.z);
      hillMesh.receiveShadow = true;
      hillMesh.castShadow = true;
      this.landscapeGroup.add(hillMesh);
    }

    // B. Majestätisches Alpen- & Mittelgebirgsmassiv im Norden (Hintergrund bis zum Horizont)
    const northZ = cz - lTable * 0.8 - 4800;
    const mountains = [
      { x: cx - 24000, z: northZ - 12000, r: 8500, h: 5400, snow: true },
      { x: cx - 15000, z: northZ - 6000, r: 7200, h: 4800, snow: true },
      { x: cx - 7500, z: northZ - 2200, r: 5600, h: 3800 },
      { x: cx - 3500, z: northZ - 1200, r: 6200, h: 4400, snow: true },
      { x: cx + 500, z: northZ - 1800, r: 6800, h: 5000, snow: true },
      { x: cx + 4500, z: northZ - 1000, r: 5800, h: 4100, snow: true },
      { x: cx + 8500, z: northZ - 2000, r: 5400, h: 3600 },
      { x: cx + 16000, z: northZ - 7000, r: 7600, h: 4900, snow: true },
      { x: cx + 26000, z: northZ - 14000, r: 9000, h: 5600, snow: true },
    ];

    // C. Sanfte Hügelketten am Horizont rundherum
    for (let a = 0; a < 14; a++) {
      const angle = (a / 14) * Math.PI * 2;
      // Nordbereich ist bereits durch das Alpenmassiv bedeckt
      if (angle > Math.PI * 0.8 && angle < Math.PI * 1.2) continue;
      const dist = 50000 + ((a * 2417) % 35000);
      const hx = cx + Math.cos(angle) * dist;
      const hz = cz + Math.sin(angle) * dist;
      const hr = 10000 + ((a * 1523) % 8000);
      const hh = 3200 + ((a * 911) % 3500);

      const hGeo = new THREE.CylinderGeometry(0, hr, hh, 16, 1, false);
      const hMesh = new THREE.Mesh(hGeo, hillMat);
      hMesh.position.set(hx, floorY + hh / 2 - 100, hz);
      this.landscapeGroup.add(hMesh);
    }

    for (const m of mountains) {
      // Felsenkegel
      const mountainGeo = new THREE.ConeGeometry(m.r, m.h, 10);
      const mountainMesh = new THREE.Mesh(mountainGeo, rockMat);
      mountainMesh.position.set(m.x, floorY + m.h / 2, m.z);
      mountainMesh.receiveShadow = true;
      mountainMesh.castShadow = true;
      this.landscapeGroup.add(mountainMesh);

      // Schneekappe auf den höchsten Gipfeln
      if (m.snow) {
        const snowH = m.h * 0.28;
        const snowR = m.r * 0.28;
        const snowGeo = new THREE.ConeGeometry(snowR, snowH, 10);
        const snowMesh = new THREE.Mesh(snowGeo, snowMat);
        snowMesh.position.set(m.x, floorY + m.h - snowH / 2, m.z);
        this.landscapeGroup.add(snowMesh);
      }
    }
  }

  /** Wunderschöner, geschwungener Gebirgsfluss durch das Tal */
  private buildLandscapeRiver(
    cx: number,
    cz: number,
    wTable: number,
    lTable: number,
    floorY: number,
    riverMat: THREE.Material,
    riverBankMat: THREE.Material,
  ) {
    const curvePoints = [
      new THREE.Vector3(cx + wTable * 0.9 + 3000, floorY + 8, cz - lTable * 1.5),
      new THREE.Vector3(cx + wTable * 0.7 + 1800, floorY + 8, cz - lTable * 0.7),
      new THREE.Vector3(cx + wTable * 0.55 + 1100, floorY + 8, cz + lTable * 0.1), // Hier Brücke
      new THREE.Vector3(cx + wTable * 0.35 + 500, floorY + 8, cz + lTable * 0.8),
      new THREE.Vector3(cx - wTable * 0.15, floorY + 8, cz + lTable * 1.45),
      new THREE.Vector3(cx - wTable * 0.7 - 1200, floorY + 8, cz + lTable * 1.9),
    ];

    const riverCurve = new THREE.CatmullRomCurve3(curvePoints);
    const riverGeo = new THREE.TubeGeometry(riverCurve, 40, 160, 8, false);
    riverGeo.scale(1, 0.08, 1); // Flaches Wasserband
    const riverMesh = new THREE.Mesh(riverGeo, riverMat);
    riverMesh.position.set(0, 0, 0);
    this.landscapeGroup.add(riverMesh);

    // Kiesel-/Sand-Ufer auf beiden Seiten
    const bankGeo = new THREE.TubeGeometry(riverCurve, 40, 210, 8, false);
    bankGeo.scale(1, 0.05, 1);
    const bankMesh = new THREE.Mesh(bankGeo, riverBankMat);
    bankMesh.position.set(0, -3, 0);
    this.landscapeGroup.add(bankMesh);
  }

  /** Historische Stein-Bogenbrücke über den Fluss */
  private buildLandscapeBridge(
    cx: number,
    cz: number,
    wTable: number,
    lTable: number,
    floorY: number,
    stoneMat: THREE.Material,
    roadMat: THREE.Material,
  ) {
    const bridgeX = cx + wTable * 0.55 + 1100;
    const bridgeZ = cz + lTable * 0.1;
    const bridgeY = floorY + 85;
    const bridgeW = 160;
    const bridgeLen = 520;
    const rotY = Math.PI / 4;

    const bridgeGroup = new THREE.Group();
    bridgeGroup.position.set(bridgeX, bridgeY, bridgeZ);
    bridgeGroup.rotation.y = rotY;

    // Brückenfahrbahn
    const deckGeo = new THREE.BoxGeometry(bridgeW, 16, bridgeLen);
    const deck = new THREE.Mesh(deckGeo, roadMat);
    bridgeGroup.add(deck);

    // Stein-Wangen / Bögen
    const archMat = stoneMat;
    const sideWallL = new THREE.Mesh(new THREE.BoxGeometry(16, 75, bridgeLen), archMat);
    sideWallL.position.set(-bridgeW / 2 + 8, -25, 0);
    bridgeGroup.add(sideWallL);

    const sideWallR = new THREE.Mesh(new THREE.BoxGeometry(16, 75, bridgeLen), archMat);
    sideWallR.position.set(bridgeW / 2 - 8, -25, 0);
    bridgeGroup.add(sideWallR);

    // 2 Bogen-Gewölbedurchlässe
    const archCut1 = new THREE.Mesh(new THREE.CylinderGeometry(75, 75, bridgeW, 16, 1, false, 0, Math.PI), stoneMat);
    archCut1.rotateZ(Math.PI / 2);
    archCut1.position.set(0, -45, -120);
    bridgeGroup.add(archCut1);

    const archCut2 = new THREE.Mesh(new THREE.CylinderGeometry(75, 75, bridgeW, 16, 1, false, 0, Math.PI), stoneMat);
    archCut2.rotateZ(Math.PI / 2);
    archCut2.position.set(0, -45, 120);
    bridgeGroup.add(archCut2);

    // Brückengeländer oben (Brüstung aus Naturstein)
    const parapetGeo = new THREE.BoxGeometry(14, 28, bridgeLen + 20);
    const parapetL = new THREE.Mesh(parapetGeo, archMat);
    parapetL.position.set(-bridgeW / 2 + 7, 20, 0);
    parapetL.castShadow = true;
    bridgeGroup.add(parapetL);

    const parapetR = new THREE.Mesh(parapetGeo, archMat);
    parapetR.position.set(bridgeW / 2 - 7, 20, 0);
    parapetR.castShadow = true;
    bridgeGroup.add(parapetR);

    this.landscapeGroup.add(bridgeGroup);
  }

  /** Malerische Landstraße durch das Tal & Allee mit Birken/Bäumen */
  private buildLandscapeRoad(
    cx: number,
    cz: number,
    wTable: number,
    lTable: number,
    floorY: number,
    roadMat: THREE.Material,
    markMat: THREE.Material,
  ) {
    const roadPoints = [
      new THREE.Vector3(cx - wTable * 0.9 - 2200, floorY + 4, cz + lTable * 0.8),
      new THREE.Vector3(cx - wTable * 0.4, floorY + 4, cz + lTable * 0.65),
      new THREE.Vector3(cx + wTable * 0.15, floorY + 4, cz + lTable * 0.45),
      new THREE.Vector3(cx + wTable * 0.55 + 1100, floorY + 80, cz + lTable * 0.1), // Über die Brücke
      new THREE.Vector3(cx + wTable * 0.85 + 1900, floorY + 4, cz - lTable * 0.25),
      new THREE.Vector3(cx + wTable * 1.1 + 3200, floorY + 4, cz - lTable * 0.8),
    ];

    const roadCurve = new THREE.CatmullRomCurve3(roadPoints);
    const roadGeo = new THREE.TubeGeometry(roadCurve, 45, 75, 4, false);
    roadGeo.scale(1, 0.05, 1);
    const roadMesh = new THREE.Mesh(roadGeo, roadMat);
    roadMesh.receiveShadow = true;
    this.landscapeGroup.add(roadMesh);

    // Weiße Mittellinie (gestrichelt)
    const samples = 35;
    for (let i = 0; i < samples; i++) {
      if (i % 2 === 0) continue;
      const t = i / samples;
      const pt = roadCurve.getPoint(t);
      const mark = new THREE.Mesh(new THREE.BoxGeometry(6, 2, 45), markMat);
      mark.position.set(pt.x, pt.y + 2, pt.z);
      const tangent = roadCurve.getTangent(t);
      mark.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), tangent);
      this.landscapeGroup.add(mark);
    }
  }

  /** Mischwälder (Nadel- und Laubbäume) in der Landschaft verteilt */
  private buildLandscapeForest(
    cx: number,
    cz: number,
    wTable: number,
    lTable: number,
    floorY: number,
    barkMat: THREE.Material,
    pineMat1: THREE.Material,
    pineMat2: THREE.Material,
    leafMat1: THREE.Material,
    leafMat2: THREE.Material,
  ) {
    // A. Nadelwald-Haine an den Hängen
    const pineClusters = [
      // Nordhang (Waldsaum am Gebirgsfuß)
      { cx: cx - 2200, cz: cz - lTable * 0.7 - 2400, count: 18, spread: 2200 },
      { cx: cx + 2400, cz: cz - lTable * 0.7 - 2600, count: 22, spread: 2500 },
      // Osthang
      { cx: cx + wTable * 0.95 + 2400, cz: cz + 300, count: 16, spread: 1800 },
      // Westhang
      { cx: cx - wTable * 0.95 - 2400, cz: cz - 400, count: 16, spread: 1800 },
    ];

    for (const cl of pineClusters) {
      for (let i = 0; i < cl.count; i++) {
        const angle = (i / cl.count) * Math.PI * 2 + (i % 3);
        const dist = 300 + Math.sin(i * 1.7) * (cl.spread * 0.45);
        const px = cl.cx + Math.cos(angle) * dist;
        const pz = cl.cz + Math.sin(angle) * dist;
        const scale = 0.85 + (i % 5) * 0.12;

        const tree = new THREE.Group();
        tree.position.set(px, floorY, pz);
        tree.scale.setScalar(scale);

        // Stamm
        const trunk = new THREE.Mesh(new THREE.CylinderGeometry(4, 7, 45, 8), barkMat);
        trunk.position.set(0, 22.5, 0);
        trunk.castShadow = true;
        tree.add(trunk);

        // 3 gestufte Nadelkronen
        const mat = i % 2 === 0 ? pineMat1 : pineMat2;
        const c1 = new THREE.Mesh(new THREE.ConeGeometry(32, 45, 8), mat);
        c1.position.set(0, 50, 0);
        c1.castShadow = true;
        tree.add(c1);

        const c2 = new THREE.Mesh(new THREE.ConeGeometry(25, 40, 8), mat);
        c2.position.set(0, 75, 0);
        c2.castShadow = true;
        tree.add(c2);

        const c3 = new THREE.Mesh(new THREE.ConeGeometry(17, 35, 8), mat);
        c3.position.set(0, 98, 0);
        c3.castShadow = true;
        tree.add(c3);

        this.landscapeGroup.add(tree);
      }
    }

    // B. Laubbäume entlang des Flusses & der Straße (Auen- & Alleebäume)
    const deciduousPositions = [
      { x: cx + wTable * 0.5 + 850, z: cz - lTable * 0.4, scale: 1.1 },
      { x: cx + wTable * 0.45 + 750, z: cz + lTable * 0.45, scale: 1.25 },
      { x: cx - wTable * 0.35 - 300, z: cz + lTable * 0.95, scale: 1.15 },
      { x: cx - wTable * 0.7 - 700, z: cz + lTable * 1.2, scale: 1.3 },
      { x: cx + wTable * 0.8 + 1400, z: cz - lTable * 0.85, scale: 1.05 },
      { x: cx - wTable * 0.65 - 1100, z: cz - lTable * 0.5, scale: 1.2 },
    ];

    for (let j = 0; j < deciduousPositions.length; j++) {
      const dp = deciduousPositions[j]!;
      const tree = new THREE.Group();
      tree.position.set(dp.x, floorY, dp.z);
      tree.scale.setScalar(dp.scale);

      // Stamm
      const trunk = new THREE.Mesh(new THREE.CylinderGeometry(5, 8, 55, 8), barkMat);
      trunk.position.set(0, 27.5, 0);
      trunk.castShadow = true;
      tree.add(trunk);

      // Krone aus Dodekaeder-Kugeln
      const mat = j % 2 === 0 ? leafMat1 : leafMat2;
      const crownMain = new THREE.Mesh(new THREE.DodecahedronGeometry(42, 1), mat);
      crownMain.position.set(0, 85, 0);
      crownMain.castShadow = true;
      tree.add(crownMain);

      const crownSub = new THREE.Mesh(new THREE.DodecahedronGeometry(28, 1), mat);
      crownSub.position.set(18, 78, 14);
      crownSub.castShadow = true;
      tree.add(crownSub);

      this.landscapeGroup.add(tree);
    }
  }

  /** Malerische Almhütte / Bauernhaus auf dem Osthügel */
  private buildLandscapeCottage(
    cx: number,
    cz: number,
    wTable: number,
    lTable: number,
    floorY: number,
  ) {
    const hutX = cx + wTable * 0.85 + 2100;
    const hutZ = cz - lTable * 0.35;
    const hutY = floorY + 480;

    const cottage = new THREE.Group();
    cottage.position.set(hutX, hutY, hutZ);
    cottage.rotation.y = -Math.PI / 6;

    // Steinfundament
    const stoneBase = new THREE.Mesh(
      new THREE.BoxGeometry(160, 25, 120),
      new THREE.MeshStandardMaterial({ color: 0x64748b, roughness: 0.9 }),
    );
    stoneBase.position.set(0, 12.5, 0);
    cottage.add(stoneBase);

    // Blockhaus-Holzwände
    const woodWalls = new THREE.Mesh(
      new THREE.BoxGeometry(150, 65, 110),
      new THREE.MeshStandardMaterial({ color: 0x78350f, roughness: 0.85 }),
    );
    woodWalls.position.set(0, 57.5, 0);
    woodWalls.castShadow = true;
    cottage.add(woodWalls);

    // Schindeldach (Satteldach)
    const roofShape = new THREE.Shape();
    roofShape.moveTo(-65, 0);
    roofShape.lineTo(0, 48);
    roofShape.lineTo(65, 0);
    roofShape.closePath();
    const roofGeo = new THREE.ExtrudeGeometry(roofShape, { depth: 168, bevelEnabled: false });
    roofGeo.rotateY(Math.PI / 2);
    roofGeo.translate(-84, 90, 0);
    const roof = new THREE.Mesh(
      roofGeo,
      new THREE.MeshStandardMaterial({ color: 0x451a03, roughness: 0.7 }),
    );
    roof.castShadow = true;
    cottage.add(roof);

    // Kamin
    const chimney = new THREE.Mesh(
      new THREE.BoxGeometry(18, 55, 18),
      new THREE.MeshStandardMaterial({ color: 0x52525b, roughness: 0.9 }),
    );
    chimney.position.set(30, 115, 0);
    cottage.add(chimney);

    // Warm leuchtendes Fenster
    const win = new THREE.Mesh(
      new THREE.BoxGeometry(16, 20, 2),
      new THREE.MeshStandardMaterial({ color: 0xfef08a, emissive: 0xfef08a, emissiveIntensity: 0.8 }),
    );
    win.position.set(-25, 62, 56);
    cottage.add(win);

    // Kleiner Holzzaun um die Hütte
    const fenceMat = new THREE.MeshStandardMaterial({ color: 0x92400e, roughness: 0.9 });
    for (let f = -90; f <= 90; f += 25) {
      const post = new THREE.Mesh(new THREE.BoxGeometry(4, 26, 4), fenceMat);
      post.position.set(f, 13, 85);
      cottage.add(post);
    }
    const rail = new THREE.Mesh(new THREE.BoxGeometry(190, 3, 2), fenceMat);
    rail.position.set(0, 18, 85);
    cottage.add(rail);

    this.landscapeGroup.add(cottage);
  }

  /** Sanfte weiße 3D-Schäfchenwolken am Himmel */
  private buildLandscapeClouds(cx: number, cz: number) {
    const cloudMat = new THREE.MeshStandardMaterial({
      color: 0xffffff,
      roughness: 0.95,
      metalness: 0.0,
      flatShading: true,
    });

    const cloudPuffs = [
      { x: cx - 6000, y: 6800, z: cz - 4000, s: 1.2 },
      { x: cx - 1800, y: 7200, z: cz - 7500, s: 1.5 },
      { x: cx + 4500, y: 6500, z: cz - 5000, s: 1.3 },
      { x: cx + 8500, y: 7400, z: cz - 2500, s: 1.1 },
      { x: cx - 7200, y: 6200, z: cz + 3000, s: 1.0 },
      { x: cx + 2500, y: 6900, z: cz + 5500, s: 1.4 },
    ];

    for (const cp of cloudPuffs) {
      const cloud = new THREE.Group();
      cloud.position.set(cp.x, cp.y, cp.z);
      cloud.scale.setScalar(cp.s);

      // Eine Wolke besteht aus 5-6 weichen überlappenden Kugeln
      const subSpheres = [
        { x: 0, y: 0, z: 0, r: 240 },
        { x: 180, y: 40, z: 20, r: 290 },
        { x: -180, y: 30, z: -20, r: 260 },
        { x: 340, y: -20, z: 10, r: 200 },
        { x: -320, y: -15, z: 15, r: 210 },
        { x: 80, y: 90, z: -10, r: 220 },
      ];

      for (const sp of subSpheres) {
        const puff = new THREE.Mesh(new THREE.DodecahedronGeometry(sp.r, 1), cloudMat);
        puff.position.set(sp.x, sp.y, sp.z);
        cloud.add(puff);
      }

      this.landscapeGroup.add(cloud);
    }
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

function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const clean = base64.replace(/^data:.*?;base64,/, "");
  const binaryString = atob(clean);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes.buffer;
}
