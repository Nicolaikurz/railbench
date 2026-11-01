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
  private showTrainHall = true;
  private showWoodPlate = true;
  private gltfLoader = new GLTFLoader();
  private gltfCache = new Map<string, THREE.Group>();
  private isVisible = false;
  private currentTheme: "light" | "dark" = "light";

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

    const savedHall = typeof localStorage !== "undefined" ? localStorage.getItem("railbench_3d_trainhall") : null;
    this.showTrainHall = savedHall === null ? true : savedHall === "1";
    this.trainHallGroup.visible = this.showTrainHall;

    const savedWoodPlate = typeof localStorage !== "undefined" ? localStorage.getItem("railbench_3d_woodplate") : null;
    this.showWoodPlate = savedWoodPlate === "1";
    this.woodPlateGroup.visible = this.showWoodPlate;

    // 1. Scene
    this.scene = new THREE.Scene();
    this.updateSceneBackground();

    // 2. Camera
    const width = host.clientWidth || 800;
    const height = host.clientHeight || 600;
    this.camera = new THREE.PerspectiveCamera(45, width / height, 5, 80000);
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
    this.controls.maxDistance = 25000;
    this.controls.target.set(0, 0, 0);

    // 5. Lighting
    this.setupLighting();

    // 6. Scene Groups
    this.scene.add(this.trainHallGroup);
    this.scene.add(this.tableGroup);
    this.scene.add(this.pillarsGroup);
    this.scene.add(this.woodPlateGroup);
    this.scene.add(this.trackGroup);
    this.scene.add(this.modelsGroup);

    // 7. Overlay Controls (2D zurück, Reset View, Ebenen-Legende)
    this.setupOverlayUI();

    // Resize listener
    window.addEventListener("resize", () => this.onResize());
  }

  private updateSceneBackground() {
    const isDark = this.currentTheme === "dark";
    if (this.showTrainHall) {
      this.scene.background = new THREE.Color(isDark ? 0x0a0f1d : 0xdde7f0);
      this.scene.fog = new THREE.FogExp2(isDark ? 0x0a0f1d : 0xdde7f0, 0.000035);
    } else {
      this.scene.background = new THREE.Color(isDark ? 0x0f172a : 0xecf0f5);
      this.scene.fog = new THREE.FogExp2(isDark ? 0x0f172a : 0xecf0f5, 0.00008);
    }
  }

  setTheme(theme: "light" | "dark") {
    this.currentTheme = theme;
    this.updateSceneBackground();
  }

  private setupLighting() {
    // Weiches Umgebungslicht
    const hemi = new THREE.HemisphereLight(0xffffff, 0x444444, 0.75);
    hemi.position.set(0, 2000, 0);
    this.scene.add(hemi);

    // Haupt-Sonnenlicht mit Schatten
    const sun = new THREE.DirectionalLight(0xfff8ed, 1.4);
    sun.position.set(1200, 2500, 1200);
    sun.castShadow = true;
    sun.shadow.mapSize.width = 2048;
    sun.shadow.mapSize.height = 2048;
    sun.shadow.camera.near = 100;
    sun.shadow.camera.far = 10000;
    const d = 2500;
    sun.shadow.camera.left = -d;
    sun.shadow.camera.right = d;
    sun.shadow.camera.top = d;
    sun.shadow.camera.bottom = -d;
    sun.shadow.bias = -0.0005;
    this.scene.add(sun);

    // Sanftes Fülllicht von der Gegenseite
    const fillLight = new THREE.DirectionalLight(0x93c5fd, 0.45);
    fillLight.position.set(-1500, 1500, -1500);
    this.scene.add(fillLight);
  }

  private setupOverlayUI() {
    const overlay = document.createElement("div");
    overlay.className = "viewer-3d-overlay";  

    overlay.querySelector("#v3d-reset-cam")?.addEventListener("click", () => this.resetCamera());

    const toggleWoodPlateBtn = overlay.querySelector("#v3d-toggle-woodplate") as HTMLButtonElement | null;
    toggleWoodPlateBtn?.addEventListener("click", () => {
      this.showWoodPlate = !this.showWoodPlate;
      this.woodPlateGroup.visible = this.showWoodPlate;
      toggleWoodPlateBtn.classList.toggle("active", this.showWoodPlate);
      try {
        localStorage.setItem("railbench_3d_woodplate", this.showWoodPlate ? "1" : "0");
      } catch {}
    });

    const toggleHallBtn = overlay.querySelector("#v3d-toggle-hall") as HTMLButtonElement | null;
    toggleHallBtn?.addEventListener("click", () => {
      this.showTrainHall = !this.showTrainHall;
      this.trainHallGroup.visible = this.showTrainHall;
      toggleHallBtn.classList.toggle("active", this.showTrainHall);
      this.updateSceneBackground();
      try {
        localStorage.setItem("railbench_3d_trainhall", this.showTrainHall ? "1" : "0");
      } catch {}
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

  private startLoop() {
    if (this.animId !== null) return;
    const animate = () => {
      this.animId = requestAnimationFrame(animate);
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
  updateLayout(layout: Layout, catalog: Catalog, options?: { showColorCode?: boolean; colorMode?: ColorMode }) {
    this.clearGroup(this.tableGroup);
    this.clearGroup(this.pillarsGroup);
    this.clearGroup(this.woodPlateGroup);
    this.clearGroup(this.trackGroup);
    this.clearGroup(this.modelsGroup);
    this.clearGroup(this.trainHallGroup);

    this.woodPlateGroup.visible = this.showWoodPlate;

    // 0. Train Hall (Lokhalle / Bahnhofshalle) als visueller 3D-Hintergrund um den Tisch
    this.buildTrainHall(layout);

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

    const isDark = this.currentTheme === "dark";
    const steelColor = isDark ? 0x1e293b : 0x2c3e50;
    const brickColor = isDark ? 0x4a2820 : 0x6e382b;
    const concreteColor = isDark ? 0x334155 : 0x64748b;

    // Gemeinsame Materialien
    const floorMat = new THREE.MeshStandardMaterial({
      color: concreteColor,
      roughness: 0.88,
      metalness: 0.08,
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
    // A. Großer Beton-Hallenboden
    const floorGeo = new THREE.BoxGeometry(wHall + 6000, 24, lHall + 6000);
    const floorMesh = new THREE.Mesh(floorGeo, floorMat);
    floorMesh.position.set(cx, floorY - 12, cz);
    floorMesh.receiveShadow = true;
    this.trainHallGroup.add(floorMesh);

    // B. Gelbe Sicherheitslinien um den Modellbahntisch
    const margin = 220;
    const boxW = (maxX - minX) + margin * 2;
    const boxL = (maxZ - minZ) + margin * 2;
    const lineThick = 35;
    const lineY = floorY + 1;

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

    // C. Bodengleise (Laufgleise links und rechts neben dem Tisch)
    const trackOffsets = [-(wTable / 2 + 1200), (wTable / 2 + 1200)];
    const trackLen = lHall + 2000;
    const gauge = 160;

    for (const offX of trackOffsets) {
      const trackX = cx + offX;

      // Schienen
      const railGeo = new THREE.BoxGeometry(10, 20, trackLen);
      const railL = new THREE.Mesh(railGeo, railMat);
      railL.position.set(trackX - gauge / 2, floorY + 10, cz);
      railL.receiveShadow = true;
      this.trainHallGroup.add(railL);

      const railR = new THREE.Mesh(railGeo, railMat);
      railR.position.set(trackX + gauge / 2, floorY + 10, cz);
      railR.receiveShadow = true;
      this.trainHallGroup.add(railR);

      // Schwellen alle 300 mm
      const sleeperCount = Math.floor(trackLen / 300);
      const sleeperGeo = new THREE.BoxGeometry(gauge + 100, 10, 42);
      for (let i = 0; i < sleeperCount; i++) {
        const sz = cz - trackLen / 2 + i * 300;
        const sleeper = new THREE.Mesh(sleeperGeo, sleeperMat);
        sleeper.position.set(trackX, floorY + 5, sz);
        sleeper.receiveShadow = true;
        this.trainHallGroup.add(sleeper);
      }
    }

    // D. Wartungsgrube (Untersuchungsgrube im rechten Bodengleis)
    const pitTrackX = cx + trackOffsets[1]!;
    const pitLen = 4200;
    const pitGeo = new THREE.BoxGeometry(gauge - 20, 110, pitLen);
    const pitMat = new THREE.MeshStandardMaterial({ color: 0x18181b, roughness: 0.95 });
    const pitMesh = new THREE.Mesh(pitGeo, pitMat);
    pitMesh.position.set(pitTrackX, floorY - 55, cz);
    this.trainHallGroup.add(pitMesh);

    // Gelbe Gruben-Sicherheitskante
    const pitEdgeL = new THREE.Mesh(new THREE.BoxGeometry(14, 2, pitLen), yellowLineMat);
    pitEdgeL.position.set(pitTrackX - gauge / 2 + 10, floorY + 1, cz);
    this.trainHallGroup.add(pitEdgeL);

    const pitEdgeR = new THREE.Mesh(new THREE.BoxGeometry(14, 2, pitLen), yellowLineMat);
    pitEdgeR.position.set(pitTrackX + gauge / 2 - 10, floorY + 1, cz);
    this.trainHallGroup.add(pitEdgeR);
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
      // A. Untere Ziegelwand
      const lowerWallH = wallH - floorY;
      const wallMesh = new THREE.Mesh(new THREE.BoxGeometry(wHall, lowerWallH, 50), brickMat);
      wallMesh.position.set(cx, (floorY + wallH) / 2, z);
      wallMesh.receiveShadow = true;
      this.trainHallGroup.add(wallMesh);

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

      // D. Hallentore (Riesige Rolltore für einfahrende Züge)
      const portalOffsets = [-wHall * 0.28, wHall * 0.28];
      const portalW = 650;
      const portalH = 950;
      for (const pOff of portalOffsets) {
        // Dunkle Toröffnung
        const portalOpening = new THREE.Mesh(
          new THREE.BoxGeometry(portalW, portalH, 55),
          new THREE.MeshStandardMaterial({ color: 0x09090b, roughness: 0.95 }),
        );
        portalOpening.position.set(cx + pOff, floorY + portalH / 2, z);
        this.trainHallGroup.add(portalOpening);

        // Stahl-Rolltor oben gerollt
        const shutter = new THREE.Mesh(
          new THREE.BoxGeometry(portalW - 20, 240, 60),
          new THREE.MeshStandardMaterial({ color: 0x334155, roughness: 0.5, metalness: 0.7 }),
        );
        shutter.position.set(cx + pOff, floorY + portalH - 120, z + (z < cz ? 28 : -28));
        this.trainHallGroup.add(shutter);
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
    const bulbMat = new THREE.MeshBasicMaterial({ color: 0xffedd5 });

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
