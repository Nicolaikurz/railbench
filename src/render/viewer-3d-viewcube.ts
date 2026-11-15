import * as THREE from "three";
import type { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { getLanguage, onLanguageChange } from "@/i18n";

export type CameraViewPreset =
  | "top"
  | "bottom"
  | "front"
  | "back"
  | "left"
  | "right"
  | "iso-se"
  | "iso-sw"
  | "iso-ne"
  | "iso-nw"
  | "home";

export interface SkyKeyframe {
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

export const SKY_KEYFRAMES: SkyKeyframe[] = [
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

export function buildCelestialBodies(celestialGroup: THREE.Group): {
  sunMesh: THREE.Mesh;
  sunCorona: THREE.Mesh;
  moonMesh: THREE.Mesh;
  moonGlow: THREE.Mesh;
} {
  celestialGroup.clear();

  const sunGeo = new THREE.SphereGeometry(2400, 24, 24);
  const sunMat = new THREE.MeshBasicMaterial({ color: 0xfffbeb });
  const sunMesh = new THREE.Mesh(sunGeo, sunMat);
  celestialGroup.add(sunMesh);

  const coronaGeo = new THREE.RingGeometry(2400, 8200, 36);
  const coronaMat = new THREE.MeshBasicMaterial({
    color: 0xfef08a,
    transparent: true,
    opacity: 0.45,
    side: THREE.DoubleSide,
    depthWrite: false,
  });
  const sunCorona = new THREE.Mesh(coronaGeo, coronaMat);
  celestialGroup.add(sunCorona);

  const moonGeo = new THREE.SphereGeometry(1800, 24, 24);
  const moonMat = new THREE.MeshStandardMaterial({
    color: 0xf1f5f9,
    roughness: 0.85,
    metalness: 0.15,
    emissive: 0x93c5fd,
    emissiveIntensity: 0.25,
  });
  const moonMesh = new THREE.Mesh(moonGeo, moonMat);
  celestialGroup.add(moonMesh);

  const moonGlowGeo = new THREE.RingGeometry(1800, 5600, 32);
  const moonGlowMat = new THREE.MeshBasicMaterial({
    color: 0xa5b4fc,
    transparent: true,
    opacity: 0.35,
    side: THREE.DoubleSide,
    depthWrite: false,
  });
  const moonGlow = new THREE.Mesh(moonGlowGeo, moonGlowMat);
  celestialGroup.add(moonGlow);

  return { sunMesh, sunCorona, moonMesh, moonGlow };
}

/**
 * Orbits the camera around controls.target by deltaAzimuthDeg (horizontal) and deltaPolarDeg (vertical).
 */
export function orbitCameraByDegrees(
  camera: THREE.PerspectiveCamera | null,
  controls: OrbitControls | null,
  deltaAzimuthDeg: number,
  deltaPolarDeg: number,
): void {
  if (!camera || !controls) return;
  const offset = camera.position.clone().sub(controls.target);
  const spherical = new THREE.Spherical().setFromVector3(offset);
  spherical.theta += THREE.MathUtils.degToRad(deltaAzimuthDeg);
  spherical.phi = THREE.MathUtils.clamp(
    spherical.phi + THREE.MathUtils.degToRad(deltaPolarDeg),
    0.001,
    Math.PI - 0.001,
  );
  offset.setFromSpherical(spherical);
  camera.position.copy(controls.target).add(offset);
  camera.lookAt(controls.target);
  controls.update();
}

const VIEWCUBE_STYLES_ID = "railbench-v3d-viewcube-styles";

function ensureViewCubeStyles(): void {
  if (typeof document === "undefined") return;
  if (document.getElementById(VIEWCUBE_STYLES_ID)) return;

  const style = document.createElement("style");
  style.id = VIEWCUBE_STYLES_ID;
  style.textContent = `
    .v3d-viewcube-panel {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 6px;
      padding: 8px;
      border-radius: 12px;
      background: rgba(15, 23, 42, 0.78);
      border: 1px solid rgba(148, 163, 184, 0.24);
      backdrop-filter: blur(10px);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
      user-select: none;
      pointer-events: auto;
      width: 112px;
    }
    [data-theme="light"] .v3d-viewcube-panel {
      background: rgba(255, 255, 255, 0.86);
      border-color: rgba(15, 23, 42, 0.15);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    }
    .v3d-viewcube-stage {
      position: relative;
      width: 88px;
      height: 88px;
      display: flex;
      align-items: center;
      justify-content: center;
      perspective: 240px;
    }
    .v3d-viewcube-cube {
      width: 54px;
      height: 54px;
      position: relative;
      transform-style: preserve-3d;
      transition: transform 0.06s linear;
    }
    .v3d-viewcube-face {
      position: absolute;
      width: 54px;
      height: 54px;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 9px;
      font-weight: 800;
      letter-spacing: 0.4px;
      text-transform: uppercase;
      color: #f8fafc;
      background: rgba(30, 41, 59, 0.92);
      border: 1.5px solid rgba(56, 189, 248, 0.55);
      border-radius: 5px;
      cursor: pointer;
      backface-visibility: hidden;
      transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
      box-shadow: inset 0 0 8px rgba(0, 0, 0, 0.35);
      padding: 0;
    }
    [data-theme="light"] .v3d-viewcube-face {
      color: #0f172a;
      background: rgba(241, 245, 249, 0.94);
      border-color: rgba(2, 132, 199, 0.5);
      box-shadow: inset 0 0 6px rgba(0, 0, 0, 0.08);
    }
    .v3d-viewcube-face:hover {
      background: rgba(2, 132, 199, 0.92);
      border-color: #ffffff;
      color: #ffffff;
    }
    .v3d-face-front  { transform: rotateY(0deg) translateZ(27px); border-bottom: 2.5px solid #3b82f6; }
    .v3d-face-back   { transform: rotateY(180deg) translateZ(27px); border-bottom: 2.5px solid #3b82f6; }
    .v3d-face-right  { transform: rotateY(90deg) translateZ(27px); border-bottom: 2.5px solid #ef4444; }
    .v3d-face-left   { transform: rotateY(-90deg) translateZ(27px); border-bottom: 2.5px solid #ef4444; }
    .v3d-face-top    { transform: rotateX(90deg) translateZ(27px); border-bottom: 2.5px solid #22c55e; }
    .v3d-face-bottom { transform: rotateX(-90deg) translateZ(27px); border-top: 2.5px solid #22c55e; }

    .v3d-corner-btn {
      position: absolute;
      width: 19px;
      height: 19px;
      border-radius: 5px;
      border: 1px solid rgba(148, 163, 184, 0.35);
      background: rgba(15, 23, 42, 0.75);
      color: #cbd5e1;
      font-size: 10px;
      line-height: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      padding: 0;
      transition: all 0.12s ease;
      z-index: 2;
    }
    [data-theme="light"] .v3d-corner-btn {
      background: rgba(248, 250, 252, 0.9);
      color: #334155;
      border-color: rgba(15, 23, 42, 0.2);
    }
    .v3d-corner-btn:hover {
      background: #0284c7;
      color: #ffffff;
      border-color: #38bdf8;
      transform: scale(1.08);
    }
    .v3d-corner-nw { top: 0; left: 0; }
    .v3d-corner-ne { top: 0; right: 0; }
    .v3d-corner-sw { bottom: 0; left: 0; }
    .v3d-corner-se { bottom: 0; right: 0; }

    .v3d-viewcube-bar {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 3px;
      width: 100%;
    }
    .v3d-vc-mini-btn {
      border: 1px solid rgba(148, 163, 184, 0.28);
      background: rgba(30, 41, 59, 0.75);
      color: #e2e8f0;
      border-radius: 5px;
      font-size: 9.5px;
      font-weight: 700;
      padding: 3px 2px;
      cursor: pointer;
      text-align: center;
      transition: all 0.12s ease;
    }
    [data-theme="light"] .v3d-vc-mini-btn {
      background: rgba(241, 245, 249, 0.9);
      color: #1e293b;
      border-color: rgba(15, 23, 42, 0.15);
    }
    .v3d-vc-mini-btn:hover {
      background: #0284c7;
      color: #ffffff;
      border-color: #38bdf8;
    }
  `;
  document.head.appendChild(style);
}

/**
 * Moves the 3D camera and OrbitControls to a named CAD view preset
 * around the current scene/controls target.
 */
export function applyCameraViewPreset(
  camera: THREE.PerspectiveCamera | null,
  controls: OrbitControls | null,
  preset: CameraViewPreset,
  sceneCenter?: THREE.Vector3,
): void {
  if (!camera || !controls) return;

  const target = controls.target.clone();
  if (preset === "home" && sceneCenter) {
    target.set(sceneCenter.x, 0, sceneCenter.z);
    controls.target.copy(target);
  }

  const currentDist = camera.position.distanceTo(target);
  const dist = Math.max(600, Math.min(8000, Number.isFinite(currentDist) && currentDist > 120 ? currentDist : 2600));

  switch (preset) {
    case "top":
      camera.position.set(target.x, target.y + dist, target.z + 0.5);
      break;
    case "bottom":
      camera.position.set(target.x, target.y - dist, target.z + 0.5);
      break;
    case "front":
      camera.position.set(target.x, target.y + dist * 0.06, target.z + dist);
      break;
    case "back":
      camera.position.set(target.x, target.y + dist * 0.06, target.z - dist);
      break;
    case "left":
      camera.position.set(target.x - dist, target.y + dist * 0.06, target.z);
      break;
    case "right":
      camera.position.set(target.x + dist, target.y + dist * 0.06, target.z);
      break;
    case "iso-nw":
      camera.position.set(target.x - dist * 0.62, target.y + dist * 0.52, target.z - dist * 0.62);
      break;
    case "iso-ne":
      camera.position.set(target.x + dist * 0.62, target.y + dist * 0.52, target.z - dist * 0.62);
      break;
    case "iso-sw":
      camera.position.set(target.x - dist * 0.62, target.y + dist * 0.52, target.z + dist * 0.62);
      break;
    case "iso-se":
    case "home":
    default:
      camera.position.set(target.x + dist * 0.58, target.y + dist * 0.52, target.z + dist * 0.62);
      break;
  }

  camera.lookAt(target);
  controls.update();
}

let _lastCubeTransform = "";

/**
 * Updates the 3D CSS ViewCube transform to match the live Three.js camera orientation.
 */
export function updateViewCubeOrientation(
  cubeEl: HTMLElement | null,
  camera: THREE.PerspectiveCamera | null,
  controls: OrbitControls | null,
): void {
  if (!cubeEl || !camera || !controls) return;
  const dx = camera.position.x - controls.target.x;
  const dy = camera.position.y - controls.target.y;
  const dz = camera.position.z - controls.target.z;

  const horiz = Math.hypot(dx, dz);
  const azimuthDeg = THREE.MathUtils.radToDeg(Math.atan2(dx, dz));
  const elevDeg = THREE.MathUtils.radToDeg(Math.atan2(dy, Math.max(1e-4, horiz)));

  const newTransform = `rotateX(${(-elevDeg).toFixed(1)}deg) rotateY(${(-azimuthDeg).toFixed(1)}deg)`;
  if (newTransform !== _lastCubeTransform) {
    _lastCubeTransform = newTransform;
    cubeEl.style.transform = newTransform;
  }
}

function createMockElement(tag = "div"): any {
  const children: any[] = [];
  const listeners: Record<string, ((e: any) => void)[]> = {};
  const dataset: Record<string, string> = {};

  const el: any = {
    tagName: tag.toUpperCase(),
    className: "",
    style: {},
    innerHTML: "",
    textContent: "",
    type: "button",
    title: "",
    dataset,
    children,
    parentElement: null,
    appendChild(child: any) {
      children.push(child);
      child.parentElement = el;
      return child;
    },
    addEventListener(type: string, fn: (e: any) => void) {
      if (!listeners[type]) listeners[type] = [];
      listeners[type].push(fn);
    },
    click() {
      const fns = listeners["click"] || [];
      fns.forEach((fn) => fn({ stopPropagation: () => {} }));
    },
    querySelector(selector: string) {
      return el.querySelectorAll(selector)[0] || null;
    },
    querySelectorAll(selector: string) {
      const results: any[] = [];
      function match(node: any) {
        if (!node) return;
        let isMatch = false;
        if (selector.startsWith(".")) {
          const cls = selector.slice(1);
          if (node.className && node.className.split(" ").includes(cls)) isMatch = true;
        } else if (selector.startsWith("[") && selector.endsWith("]")) {
          const [key, val] = selector.slice(1, -1).split("=");
          if (key && val) {
            const cleanVal = val.replace(/['"]/g, "");
            const camelKey = key.replace(/^data-/, "").replace(/-([a-z])/g, (_, g) => g.toUpperCase());
            if (node.dataset && node.dataset[camelKey] === cleanVal) isMatch = true;
          }
        }
        if (isMatch) results.push(node);
        if (node.children) {
          for (const ch of node.children) match(ch);
        }
      }
      match(el);
      return results;
    },
  };
  return el;
}

/**
 * Creates the interactive CAD ViewCube DOM element placed under the background toggle button.
 */
export function createViewCubeWidget(
  onSelectPreset: (preset: CameraViewPreset) => void,
  onOrbitStep: (deltaAzimuthDeg: number, deltaPolarDeg: number) => void,
): { panelEl: HTMLElement; cubeEl: HTMLElement; refreshLabels: () => void } {
  ensureViewCubeStyles();

  const doc = typeof document !== "undefined" ? document : { createElement: (tag: string) => createMockElement(tag) };

  const panelEl = doc.createElement("div");
  panelEl.className = "v3d-viewcube-panel";

  const stageEl = doc.createElement("div");
  stageEl.className = "v3d-viewcube-stage";

  // 4 Isometric corner buttons around the cube
  const corners: Array<{ cls: string; preset: CameraViewPreset; sym: string; de: string; en: string }> = [
    { cls: "v3d-corner-nw", preset: "iso-nw", sym: "↖", de: "Isometrie Hinten-Links (NW)", en: "Isometric Back-Left (NW)" },
    { cls: "v3d-corner-ne", preset: "iso-ne", sym: "↗", de: "Isometrie Hinten-Rechts (NO)", en: "Isometric Back-Right (NE)" },
    { cls: "v3d-corner-sw", preset: "iso-sw", sym: "↙", de: "Isometrie Vorne-Links (SW)", en: "Isometric Front-Left (SW)" },
    { cls: "v3d-corner-se", preset: "iso-se", sym: "↘", de: "Isometrie Vorne-Rechts (SO)", en: "Isometric Front-Right (SE)" },
  ];

  const cornerBtns: Array<{ el: HTMLButtonElement; de: string; en: string }> = [];
  for (const c of corners) {
    const b = doc.createElement("button");
    b.type = "button";
    b.className = `v3d-corner-btn ${c.cls}`;
    b.textContent = c.sym;
    b.addEventListener("click", () => onSelectPreset(c.preset));
    stageEl.appendChild(b);
    cornerBtns.push({ el: b, de: c.de, en: c.en });
  }

  // 3D Cube with 6 clickable faces
  const cubeEl = doc.createElement("div");
  cubeEl.className = "v3d-viewcube-cube";

  const faces: Array<{
    cls: string;
    preset: CameraViewPreset;
    de: string;
    en: string;
    titleDe: string;
    titleEn: string;
  }> = [
    { cls: "v3d-face-front", preset: "front", de: "Vorne", en: "Front", titleDe: "Ansicht von vorne", titleEn: "Front view" },
    { cls: "v3d-face-back", preset: "back", de: "Hinten", en: "Back", titleDe: "Ansicht von hinten", titleEn: "Back view" },
    { cls: "v3d-face-left", preset: "left", de: "Links", en: "Left", titleDe: "Ansicht von links", titleEn: "Left view" },
    { cls: "v3d-face-right", preset: "right", de: "Rechts", en: "Right", titleDe: "Ansicht von rechts", titleEn: "Right view" },
    { cls: "v3d-face-top", preset: "top", de: "Oben", en: "Top", titleDe: "Draufsicht (von oben)", titleEn: "Top view" },
    { cls: "v3d-face-bottom", preset: "bottom", de: "Unten", en: "Bottom", titleDe: "Ansicht von unten (Unterseite)", titleEn: "Bottom view (underside)" },
  ];

  const faceEls: Array<{ el: HTMLButtonElement; de: string; en: string; titleDe: string; titleEn: string }> = [];
  for (const f of faces) {
    const btn = doc.createElement("button");
    btn.type = "button";
    btn.className = `v3d-viewcube-face ${f.cls}`;
    btn.dataset.viewPreset = f.preset;
    btn.addEventListener("click", (e: any) => {
      e.stopPropagation?.();
      onSelectPreset(f.preset);
    });
    cubeEl.appendChild(btn);
    faceEls.push({ el: btn, de: f.de, en: f.en, titleDe: f.titleDe, titleEn: f.titleEn });
  }

  // Support dragging directly on the ViewCube stage to orbit the camera
  let dragStart: { x: number; y: number } | null = null;
  let moved = false;
  stageEl.addEventListener("pointerdown", (e: any) => {
    if (e.button !== 0) return;
    dragStart = { x: e.clientX, y: e.clientY };
    moved = false;
  });
  if (typeof window !== "undefined") {
    window.addEventListener("pointermove", (e: any) => {
      if (!dragStart) return;
      const dx = e.clientX - dragStart.x;
      const dy = e.clientY - dragStart.y;
      if (Math.hypot(dx, dy) > 4) {
        moved = true;
        onOrbitStep(-dx * 0.85, -dy * 0.85);
        dragStart = { x: e.clientX, y: e.clientY };
      }
    });
    window.addEventListener("pointerup", () => {
      dragStart = null;
    });
  }
  stageEl.addEventListener(
    "click",
    (e: any) => {
      if (moved) {
        e.stopPropagation?.();
        moved = false;
      }
    },
    true,
  );

  stageEl.appendChild(cubeEl);
  panelEl.appendChild(stageEl);

  // Quick-action bar below the cube: ⟲ 90° | ⌂ ISO | ⟳ 90° and Oben | Vorne | Unten
  const barEl = doc.createElement("div");
  barEl.className = "v3d-viewcube-bar";

  const quickButtons: Array<{
    id: string;
    de: string;
    en: string;
    titleDe: string;
    titleEn: string;
    onClick: () => void;
  }> = [
    {
      id: "rot-ccw",
      de: "⟲ 45°",
      en: "⟲ 45°",
      titleDe: "Ansicht 45° nach links drehen",
      titleEn: "Rotate view 45° left",
      onClick: () => onOrbitStep(-45, 0),
    },
    {
      id: "home",
      de: "⌂ ISO",
      en: "⌂ ISO",
      titleDe: "Standard-Isometrie (Home)",
      titleEn: "Reset to Home Isometric view",
      onClick: () => onSelectPreset("home"),
    },
    {
      id: "rot-cw",
      de: "⟳ 45°",
      en: "⟳ 45°",
      titleDe: "Ansicht 45° nach rechts drehen",
      titleEn: "Rotate view 45° right",
      onClick: () => onOrbitStep(45, 0),
    },
    {
      id: "top",
      de: "⊤ Oben",
      en: "⊤ Top",
      titleDe: "Direkt von oben blicken",
      titleEn: "Look straight from above",
      onClick: () => onSelectPreset("top"),
    },
    {
      id: "front",
      de: "▭ Vorne",
      en: "▭ Front",
      titleDe: "Direkt von vorne blicken",
      titleEn: "Look straight from front",
      onClick: () => onSelectPreset("front"),
    },
    {
      id: "bottom",
      de: "⊥ Unten",
      en: "⊥ Bottom",
      titleDe: "Direkt von unten unter den Tisch blicken",
      titleEn: "Look straight from underneath the table",
      onClick: () => onSelectPreset("bottom"),
    },
  ];

  const miniEls: Array<{ el: HTMLButtonElement; de: string; en: string; titleDe: string; titleEn: string }> = [];
  for (const qb of quickButtons) {
    const b = doc.createElement("button") as any;
    b.type = "button";
    b.className = "v3d-vc-mini-btn";
    b.dataset.vcAction = qb.id;
    b.addEventListener("click", qb.onClick);
    barEl.appendChild(b);
    miniEls.push({ el: b, de: qb.de, en: qb.en, titleDe: qb.titleDe, titleEn: qb.titleEn });
  }

  panelEl.appendChild(barEl);

  const refreshLabels = () => {
    const isDe = getLanguage() === "de";
    for (const f of faceEls) {
      f.el.textContent = isDe ? f.de : f.en;
      f.el.title = isDe ? f.titleDe : f.titleEn;
    }
    for (const c of cornerBtns) {
      c.el.title = isDe ? c.de : c.en;
    }
    for (const m of miniEls) {
      m.el.textContent = isDe ? m.de : m.en;
      m.el.title = isDe ? m.titleDe : m.titleEn;
    }
  };

  refreshLabels();
  onLanguageChange(() => refreshLabels());
  return { panelEl, cubeEl, refreshLabels };
}
