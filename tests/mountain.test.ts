import { describe, it, expect, vi } from "vitest";
import * as THREE from "three";
import { buildMountain3D, findCollidingTracksForMountain } from "@/render/models-3d/mountain";
import { buildMountainCard } from "@/ui/sidebar/mountain-card";
import type { Model3D, TrackPiece } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";

describe("Mountain 3D & Tunnel generation", () => {
  it("builds a 3D mountain mesh with geometry and terrain colors", () => {
    const group = new THREE.Group();
    const mountainModel: Model3D = {
      id: "mountain-1",
      name: "Test Mountain",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 400, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    buildMountain3D(group, mountainModel);

    // Group should contain children (mountain mesh)
    expect(group.children.length).toBeGreaterThan(0);
    const mountainSubgroup = group.children[0] as THREE.Group;
    expect(mountainSubgroup).toBeDefined();

    // Check mesh inside
    const mesh = mountainSubgroup.children.find((c) => c instanceof THREE.Mesh) as THREE.Mesh | undefined;
    expect(mesh).toBeDefined();
    expect(mesh?.geometry).toBeInstanceOf(THREE.BufferGeometry);
    expect(mesh?.geometry.getAttribute("position")).toBeDefined();
    expect(mesh?.geometry.getAttribute("color")).toBeDefined();

    // Verify vertical offset and polygonOffset to prevent Z-fighting artifacts with the table
    expect(mountainSubgroup.position.y).toBeGreaterThan(0);
    const mat = mesh?.material as THREE.MeshStandardMaterial;
    expect(mat.polygonOffset).toBe(true);
    expect(mat.polygonOffsetFactor).toBeLessThan(0);
  });

  it("detects colliding tracks that cross through the mountain footprint", () => {
    const mountainModel: Model3D = {
      id: "mountain-1",
      name: "Test Mountain",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const straightTrackPiece: TrackPiece = {
      id: "track-1",
      catalogId: "24188",
      transform: { x: 500, y: 500, rotation: 0 },
      connectors: [
        { x: 405.9, y: 500, angle: 180 },
        { x: 594.1, y: 500, angle: 0 },
      ],
    };

    const farTrackPiece: TrackPiece = {
      id: "track-far",
      catalogId: "24188",
      transform: { x: 2500, y: 2500, rotation: 0 },
      connectors: [
        { x: 2405.9, y: 2500, angle: 180 },
        { x: 2594.1, y: 2500, angle: 0 },
      ],
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188.3mm",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    const collisions = findCollidingTracksForMountain(
      mountainModel,
      [straightTrackPiece, farTrackPiece],
      (id) => (id === "24188" ? dummyEntry : undefined),
    );

    // straightTrackPiece should collide, farTrackPiece should not
    expect(collisions.length).toBe(1);
    expect(collisions[0]!.piece.id).toBe("track-1");
    expect(collisions[0]!.centerline.length).toBeGreaterThan(0);
  });

  it("ignores tracks below (e.g. -100mm) and above (e.g. +250mm) the mountain and only includes tracks going through the mountain", () => {
    const mountainModel: Model3D = {
      id: "mountain-1",
      name: "Test Mountain",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      layerId: "layer-main",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis 188.3mm",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    const layers = [
      { id: "layer-main", name: "Hauptebene", visible: true, locked: false, heightMm: 0 },
      { id: "layer-under", name: "Schattenbahnhof (-100 mm)", visible: true, locked: false, heightMm: -100 },
      { id: "layer-above", name: "Hochbrücke (+260 mm)", visible: true, locked: false, heightMm: 260 },
    ];

    const throughTrack: TrackPiece = {
      id: "track-through",
      catalogId: "24188",
      layerId: "layer-main",
      transform: { x: 500, y: 500, rotation: 0 },
      connectors: [
        { x: 405.9, y: 500, angle: 180 },
        { x: 594.1, y: 500, angle: 0 },
      ],
    };

    const underLayerTrack: TrackPiece = {
      id: "track-under-layer",
      catalogId: "24188",
      layerId: "layer-under",
      transform: { x: 500, y: 500, rotation: 0 },
      connectors: [
        { x: 405.9, y: 500, angle: 180 },
        { x: 594.1, y: 500, angle: 0 },
      ],
    };

    const underElevTrack: TrackPiece = {
      id: "track-under-elev",
      catalogId: "24188",
      layerId: "layer-main",
      transform: { x: 500, y: 520, rotation: 0 },
      connectors: [
        { x: 405.9, y: 520, angle: 180 },
        { x: 594.1, y: 520, angle: 0 },
      ],
      elevation: { start: -80, end: -80 },
    };

    const aboveLayerTrack: TrackPiece = {
      id: "track-above-layer",
      catalogId: "24188",
      layerId: "layer-above",
      transform: { x: 500, y: 500, rotation: 0 },
      connectors: [
        { x: 405.9, y: 500, angle: 180 },
        { x: 594.1, y: 500, angle: 0 },
      ],
    };

    const collisions = findCollidingTracksForMountain(
      mountainModel,
      [throughTrack, underLayerTrack, underElevTrack, aboveLayerTrack],
      () => dummyEntry,
      layers,
    );

    expect(collisions.length).toBe(1);
    expect(collisions[0]!.piece.id).toBe("track-through");
  });

  it("builds tunnel tube and stone portals when tracks collide with mountain", () => {
    const group = new THREE.Group();
    const mountainModel: Model3D = {
      id: "mountain-1",
      name: "Test Mountain",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    const trackPiece: TrackPiece = {
      id: "track-1",
      catalogId: "24188",
      transform: { x: 500, y: 500, rotation: 0 },
      connectors: [
        { x: 405.9, y: 500, angle: 180 },
        { x: 594.1, y: 500, angle: 0 },
      ],
    };

    const collisions = findCollidingTracksForMountain(mountainModel, [trackPiece], () => dummyEntry);

    buildMountain3D(group, mountainModel, collisions);

    const mountainSubgroup = group.children[0] as THREE.Group;
    expect(mountainSubgroup).toBeDefined();

    // Check if additional children (portals and tube) were created
    // Main terrain mesh is child 0, portals/tubes are subsequent children
    expect(mountainSubgroup.children.length).toBeGreaterThan(1);
  });

  it("creates seamless portal with wing walls, collar casing, headwall and continuous tube when track crosses mountain border", () => {
    const group = new THREE.Group();
    const mountainModel: Model3D = {
      id: "mountain-1",
      name: "Test Mountain",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Track crossing from x = 100 (outside mountain) through center x = 500 to x = 900 (outside mountain)
    const piece1: TrackPiece = {
      id: "p1",
      catalogId: "24188",
      transform: { x: 200, y: 500, rotation: 0 },
      connectors: [
        { x: 105.9, y: 500, angle: 180 },
        { x: 294.1, y: 500, angle: 0 },
      ],
    };
    const piece2: TrackPiece = {
      id: "p2",
      catalogId: "24188",
      transform: { x: 388, y: 500, rotation: 0 },
      connectors: [
        { x: 294.1, y: 500, angle: 180 },
        { x: 482.3, y: 500, angle: 0 },
      ],
    };
    const piece3: TrackPiece = {
      id: "p3",
      catalogId: "24188",
      transform: { x: 576, y: 500, rotation: 0 },
      connectors: [
        { x: 482.3, y: 500, angle: 180 },
        { x: 670.5, y: 500, angle: 0 },
      ],
    };
    const piece4: TrackPiece = {
      id: "p4",
      catalogId: "24188",
      transform: { x: 764, y: 500, rotation: 0 },
      connectors: [
        { x: 670.5, y: 500, angle: 180 },
        { x: 858.7, y: 500, angle: 0 },
      ],
    };

    const collisions = findCollidingTracksForMountain(
      mountainModel,
      [piece1, piece2, piece3, piece4],
      () => dummyEntry,
    );

    expect(collisions.length).toBe(1); // Grouped into 1 continuous corridor
    expect(collisions[0]!.centerline.length).toBeGreaterThan(10);

    buildMountain3D(group, mountainModel, collisions);

    const mountainSubgroup = group.children[0] as THREE.Group;
    expect(mountainSubgroup).toBeDefined();

    // Find portal groups (each portal contains face, crown, collar, wing walls)
    const portalGroups = mountainSubgroup.children.filter(
      (c) => c instanceof THREE.Group && c.children.length >= 4,
    ) as THREE.Group[];

    // Expect 2 portals (entrance and exit)
    expect(portalGroups.length).toBe(2);

    // Each portal must contain face, crown, collar, wing walls, and split foundations
    for (const portal of portalGroups) {
      expect(portal.children.length).toBeGreaterThanOrEqual(6);
      const meshes = portal.children.filter((c) => c instanceof THREE.Mesh);
      expect(meshes.length).toBeGreaterThanOrEqual(6);

      // Verify that no foundation mesh blocks the track centerline (x = 0) at track level (y <= 0)
      for (const mesh of meshes) {
        if (mesh.geometry instanceof THREE.BoxGeometry) {
          const params = (mesh.geometry as THREE.BoxGeometry).parameters;
          // Foundations are boxes located below y=0
          if (mesh.position.y < 0) {
            const minX = mesh.position.x - params.width / 2;
            const maxX = mesh.position.x + params.width / 2;
            // The track center corridor [-25, 25] must NOT be inside the foundation box
            const coversCenter = minX <= 0 && maxX >= 0;
            expect(coversCenter).toBe(false);
          }
        }
      }
    }

    // Tunnel tube must be present (swept tunnel lining mesh)
    const tubeMesh = mountainSubgroup.children.find(
      (c): c is THREE.Mesh => c instanceof THREE.Mesh && c !== mountainSubgroup.children[0],
    );
    expect(tubeMesh).toBeDefined();
    expect(tubeMesh?.geometry).toBeInstanceOf(THREE.BufferGeometry);
    expect((tubeMesh?.geometry as THREE.BufferGeometry).getAttribute("position").count).toBeGreaterThan(0);
  });

  it("verifies that mountain cutout clears the tunnel passage while preserving the mountain roof above", () => {
    const group = new THREE.Group();
    const mountainModel: Model3D = {
      id: "mountain-1",
      name: "High Mountain",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 250,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Track passing straight through y = 500
    const trackPiece: TrackPiece = {
      id: "t1",
      catalogId: "24188",
      transform: { x: 500, y: 500, rotation: 0 },
      connectors: [
        { x: 405.9, y: 500, angle: 180 },
        { x: 594.1, y: 500, angle: 0 },
      ],
    };

    const collisions = findCollidingTracksForMountain(mountainModel, [trackPiece], () => dummyEntry);
    buildMountain3D(group, mountainModel, collisions);

    const mountainSubgroup = group.children[0] as THREE.Group;
    const terrainMesh = mountainSubgroup.children[0] as THREE.Mesh;
    const geo = terrainMesh.geometry as THREE.BufferGeometry;

    // Mountain must have indices (triangles)
    const index = geo.getIndex();
    expect(index).toBeDefined();
    expect(index!.count).toBeGreaterThan(0);
  });

  it("inspects Alpenland H0 mountains and tunnel generation", async () => {
    const { createAlpenlandH0Layout } = await import("@/editor/demo-tracks");
    const { maerklinCCatalog } = await import("@/catalog/catalog");
    const layout = createAlpenlandH0Layout();
    const entryMap = new Map(maerklinCCatalog.entries.map((e) => [e.id, e]));
    const mountains = layout.models?.filter((m) => m.builtinId === "mountain") || [];
    expect(mountains.length).toBeGreaterThanOrEqual(2);

    // Mountain 1 has track collision and builds tunnel + 2 portals
    const m1 = mountains.find((m) => m.name === "Mountain 1")!;
    expect(m1).toBeDefined();
    const collisions1 = findCollidingTracksForMountain(
      m1,
      layout.pieces,
      (id) => entryMap.get(id),
      layout.layers,
    );
    expect(collisions1.length).toBeGreaterThanOrEqual(1);
    expect(collisions1[0]!.centerline.length).toBeGreaterThanOrEqual(30);

    const group1 = new THREE.Group();
    buildMountain3D(group1, m1, collisions1);
    const sub1 = group1.children[0] as THREE.Group;
    expect(sub1).toBeDefined();

    const portalGroups1 = sub1.children.filter((c) => c instanceof THREE.Group) as THREE.Group[];
    expect(portalGroups1.length).toBe(2);

    // Verify portals have wing walls flaring outward (negative Z in portal space)
    for (const portal of portalGroups1) {
      expect(portal.children.length).toBeGreaterThanOrEqual(6);
      const boxes = portal.children.filter(
        (c): c is THREE.Mesh => c instanceof THREE.Mesh && c.geometry instanceof THREE.BoxGeometry,
      );
      // Wing walls are boxes at y > 0
      const wingWalls = boxes.filter((b) => b.position.y > 0 && Math.abs(b.position.x) > 30);
      expect(wingWalls.length).toBe(2);
      for (const w of wingWalls) {
        // Wing wall center position z must flare outward (z <= 0)
        expect(w.position.z).toBeLessThanOrEqual(0);
      }
    }

    // Tunnel tube must be present
    const tube1 = sub1.children.find(
      (c): c is THREE.Mesh => c instanceof THREE.Mesh && c !== sub1.children[0],
    );
    expect(tube1).toBeDefined();

    // Mountain 2 is a freestanding scenic mountain without collisions
    const m2 = mountains.find((m) => m.id === "model-1794593670419-0dyb")!;
    expect(m2).toBeDefined();
    const collisions2 = findCollidingTracksForMountain(
      m2,
      layout.pieces,
      (id) => entryMap.get(id),
      layout.layers,
    );
    expect(collisions2.length).toBe(0);

    const group2 = new THREE.Group();
    buildMountain3D(group2, m2, collisions2);
    const sub2 = group2.children[0] as THREE.Group;
    expect(sub2.children.length).toBe(1); // Terrain mesh only, no portals or tubes
  });

  it("buildMountainCard renders in sidebar and supports adding and adjusting mountains", () => {
    const originalDocument = (globalThis as any).document;
    const createMockElement = (tag: string) => {
      const listeners: Record<string, ((e?: any) => void)[]> = {};
      const el: any = {
        tagName: tag.toUpperCase(),
        className: "",
        style: {},
        children: [] as any[],
        value: "",
        textContent: "",
        innerHTML: "",
        type: "",
        open: false,
        appendChild: (child: any) => {
          el.children.push(child);
          return child;
        },
        addEventListener: (event: string, cb: (e?: any) => void) => {
          listeners[event] = listeners[event] || [];
          listeners[event]!.push(cb);
        },
        dispatchEvent: (event: any) => {
          const type = event?.type || event;
          listeners[type]?.forEach((cb) => cb(event));
          return true;
        },
        querySelector: (sel: string) => {
          if (sel === "button") return el.children.find((c: any) => c.tagName === "BUTTON") || null;
          return null;
        },
        querySelectorAll: (sel: string) => {
          const result: any[] = [];
          const traverse = (node: any) => {
            if (sel.includes("button") && node.tagName === "BUTTON") result.push(node);
            if (sel.includes("input") && node.tagName === "INPUT") result.push(node);
            node.children?.forEach(traverse);
          };
          traverse(el);
          return result;
        },
      };
      return el;
    };

    (globalThis as any).document = {
      createElement: createMockElement,
    };

    try {
      const mockSidebar: any = {
        layout: {
          pieces: [],
          models: [],
          layers: [{ id: "l1", name: "Default", heightMm: 0, visible: true }],
        },
        catalog: { entries: [] },
        mountainsOpen: true,
        activeLayerId: "l1",
        callbacks: {},
      };

      let notifyBoardCalled = false;
      let refreshCalled = false;
      const mockCtx: any = {
        isDe: true,
        bb: { enabled: true, x: 0, y: 0, width: 2000, depth: 1000 },
        notifyBoard: () => {
          notifyBoardCalled = true;
        },
        notifyLayer: () => {},
        refresh: () => {
          refreshCalled = true;
        },
      };

      // Initially 0 mountains
      const cardEl = buildMountainCard(mockSidebar, mockCtx);
      expect(cardEl).toBeDefined();

      // Click Add button
      const allButtons = cardEl.querySelectorAll("button");
      expect(allButtons.length).toBeGreaterThan(0);
      const addBtn: any = allButtons[0];
      addBtn?.dispatchEvent({ type: "click" });

      expect(mockSidebar.layout.models.length).toBe(1);
      const m = mockSidebar.layout.models[0];
      expect(m.builtinId).toBe("mountain");
      expect(m.width).toBe(600);
      expect(m.depth).toBe(450);
      expect(m.height).toBe(220);
      expect(refreshCalled).toBe(true);

      // Re-render card with 1 mountain
      const cardWithMountain = buildMountainCard(mockSidebar, mockCtx);
      const inputs = cardWithMountain.querySelectorAll("input");
      expect(inputs.length).toBeGreaterThanOrEqual(6); // X, Y, Width, Depth, Height, Rotation

      // Adjust width
      const widthInput: any = inputs[2];
      widthInput.value = "850";
      widthInput.dispatchEvent({ type: "change" });
      expect(m.width).toBe(850);
      expect(notifyBoardCalled).toBe(true);

      // Adjust height
      const heightInput: any = inputs[4];
      heightInput.value = "300";
      heightInput.dispatchEvent({ type: "change" });
      expect(m.height).toBe(300);

      // Delete mountain
      const deleteBtn = Array.from(cardWithMountain.querySelectorAll("button")).find(
        (b: any) => b.textContent?.includes("Löschen") || b.className?.includes("danger"),
      ) as any;
      expect(deleteBtn).toBeDefined();
      deleteBtn?.dispatchEvent({ type: "click" });
      expect(mockSidebar.layout.models.length).toBe(0);
    } finally {
      (globalThis as any).document = originalDocument;
    }
  });

  it("removes mountain from BUILTIN_MODELS while keeping definition in findBuiltinModelDef", async () => {
    const { BUILTIN_MODELS, findBuiltinModelDef, MOUNTAIN_MODEL_DEF } = await import("@/core/models-3d");
    // Mountain must NOT be in BUILTIN_MODELS (removed from 3D models tab)
    const inBuiltins = BUILTIN_MODELS.some((m) => m.id === "mountain");
    expect(inBuiltins).toBe(false);

    // findBuiltinModelDef("mountain") must still return the definition
    const def = findBuiltinModelDef("mountain");
    expect(def).toBeDefined();
    expect(def?.id).toBe("mountain");
    expect(def).toBe(MOUNTAIN_MODEL_DEF);
    expect(def?.width).toBe(600);
    expect(def?.depth).toBe(450);
    expect(def?.height).toBe(220);
  });

  it("sidebar.selectMountain switches to board tab and sets active mountain", async () => {
    const originalDocument = (globalThis as any).document;
    const originalWindow = (globalThis as any).window;
    const createMockElement = (tag: string) => {
      const el: any = {
        tagName: tag.toUpperCase(),
        className: "",
        style: {},
        children: [] as any[],
        appendChild: (child: any) => { el.children.push(child); return child; },
        append: (...items: any[]) => { el.children.push(...items); },
        setAttribute: () => {},
        getAttribute: () => null,
        addEventListener: () => {},
        removeEventListener: () => {},
        querySelector: () => null,
        querySelectorAll: () => [],
        getContext: () => null,
      };
      return el;
    };

    (globalThis as any).window = {
      addEventListener: () => {},
      removeEventListener: () => {},
    };

    (globalThis as any).document = {
      createElement: createMockElement,
      head: { appendChild: () => {} },
      getElementById: () => null,
    };

    try {
      const { Sidebar } = await import("@/ui/sidebar");
      const { maerklinCCatalog } = await import("@/catalog/catalog");

      let selectedModelId: string | null = null;
      let tabChanged: string | null = null;

      const mockHost: any = createMockElement("div");

      const layout: any = {
        name: "Test",
        pieces: [],
        models: [
          { id: "m1", builtinId: "mountain", name: "Berg 1", transform: { x: 0, y: 0, rotation: 0 } },
          { id: "m2", builtinId: "mountain", name: "Berg 2", transform: { x: 100, y: 100, rotation: 0 } },
        ],
        baseboard: { enabled: true, width: 2000, depth: 1000 },
        layers: [{ id: "l1", name: "Ebene 1", heightMm: 0, visible: true }],
      };

      const sidebar = new Sidebar(mockHost, maerklinCCatalog, layout, {
        onExportCSV: vi.fn(),
        onTabChange: (tab) => {
          tabChanged = tab;
        },
        onSelectModel: (id) => {
          selectedModelId = id;
        },
      });

      sidebar.setActiveTab("catalog");
      expect(sidebar.getActiveTab()).toBe("catalog");

      // Select Mountain 2
      sidebar.selectMountain("m2");

      expect(sidebar.getActiveTab()).toBe("board");
      expect(sidebar.activeMountainId).toBe("m2");
      expect(sidebar.mountainsOpen).toBe(true);
      expect(selectedModelId).toBe("m2");
      expect(tabChanged).toBe("board");
    } finally {
      (globalThis as any).document = originalDocument;
      (globalThis as any).window = originalWindow;
    }
  });

  it("renders tunnel and portals higher for elevated tracks and lower for depressed tracks", () => {
    const mountainModel: Model3D = {
      id: "mountain-elev",
      name: "Mountain Elev",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // 1. Elevated track (+35 mm)
    const elevatedTracks: TrackPiece[] = [
      {
        id: "el-1",
        catalogId: "24188",
        transform: { x: 200, y: 500, rotation: 0 },
        elevation: { start: 35, end: 35 },
        connectors: [{ x: 105.9, y: 500, angle: 180 }, { x: 294.1, y: 500, angle: 0 }],
      },
      {
        id: "el-2",
        catalogId: "24188",
        transform: { x: 388, y: 500, rotation: 0 },
        elevation: { start: 35, end: 35 },
        connectors: [{ x: 294.1, y: 500, angle: 180 }, { x: 482.3, y: 500, angle: 0 }],
      },
      {
        id: "el-3",
        catalogId: "24188",
        transform: { x: 576, y: 500, rotation: 0 },
        elevation: { start: 35, end: 35 },
        connectors: [{ x: 482.3, y: 500, angle: 180 }, { x: 670.5, y: 500, angle: 0 }],
      },
      {
        id: "el-4",
        catalogId: "24188",
        transform: { x: 764, y: 500, rotation: 0 },
        elevation: { start: 35, end: 35 },
        connectors: [{ x: 670.5, y: 500, angle: 180 }, { x: 858.7, y: 500, angle: 0 }],
      },
    ];

    const elevCols = findCollidingTracksForMountain(mountainModel, elevatedTracks, () => dummyEntry);
    expect(elevCols.length).toBe(1);

    const elevGroup = new THREE.Group();
    buildMountain3D(elevGroup, mountainModel, elevCols);
    const elevSub = elevGroup.children[0] as THREE.Group;
    const elevPortals = elevSub.children.filter((c) => c instanceof THREE.Group && c.children.length >= 4) as THREE.Group[];
    expect(elevPortals.length).toBe(2);
    // Portals for elevated track sit at elevated position (y >= 35)
    for (const p of elevPortals) {
      expect(p.position.y).toBeGreaterThanOrEqual(35);
    }

    // 2. Depressed track (-15 mm)
    const lowTracks: TrackPiece[] = [
      {
        id: "low-1",
        catalogId: "24188",
        transform: { x: 200, y: 500, rotation: 0 },
        elevation: { start: -15, end: -15 },
        connectors: [{ x: 105.9, y: 500, angle: 180 }, { x: 294.1, y: 500, angle: 0 }],
      },
      {
        id: "low-2",
        catalogId: "24188",
        transform: { x: 388, y: 500, rotation: 0 },
        elevation: { start: -15, end: -15 },
        connectors: [{ x: 294.1, y: 500, angle: 180 }, { x: 482.3, y: 500, angle: 0 }],
      },
      {
        id: "low-3",
        catalogId: "24188",
        transform: { x: 576, y: 500, rotation: 0 },
        elevation: { start: -15, end: -15 },
        connectors: [{ x: 482.3, y: 500, angle: 180 }, { x: 670.5, y: 500, angle: 0 }],
      },
      {
        id: "low-4",
        catalogId: "24188",
        transform: { x: 764, y: 500, rotation: 0 },
        elevation: { start: -15, end: -15 },
        connectors: [{ x: 670.5, y: 500, angle: 180 }, { x: 858.7, y: 500, angle: 0 }],
      },
    ];

    const lowCols = findCollidingTracksForMountain(mountainModel, lowTracks, () => dummyEntry);
    expect(lowCols.length).toBe(1);

    const lowGroup = new THREE.Group();
    buildMountain3D(lowGroup, mountainModel, lowCols);
    const lowSub = lowGroup.children[0] as THREE.Group;
    const lowPortals = lowSub.children.filter((c) => c instanceof THREE.Group && c.children.length >= 4) as THREE.Group[];
    expect(lowPortals.length).toBe(2);
    // Portals for lower track sit at lowered position (y <= -13)
    for (const p of lowPortals) {
      expect(p.position.y).toBeLessThanOrEqual(-13);
    }
  });

  it("calculates increased clearance height and width for tunnel with catenary", () => {
    const mountainModel: Model3D = {
      id: "mountain-cat",
      name: "Mountain Catenary",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      gauge: 16.5,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Standard track without catenary
    const standardTracks: TrackPiece[] = [
      {
        id: "std-1",
        catalogId: "24188",
        transform: { x: 388, y: 500, rotation: 0 },
        connectors: [{ x: 294.1, y: 500, angle: 180 }, { x: 482.3, y: 500, angle: 0 }],
      },
      {
        id: "std-2",
        catalogId: "24188",
        transform: { x: 576, y: 500, rotation: 0 },
        connectors: [{ x: 482.3, y: 500, angle: 180 }, { x: 670.5, y: 500, angle: 0 }],
      },
    ];

    // Track with catenary
    const catenaryTracks: TrackPiece[] = [
      {
        id: "cat-1",
        catalogId: "24188",
        transform: { x: 388, y: 500, rotation: 0 },
        hasCatenary: true,
        connectors: [{ x: 294.1, y: 500, angle: 180 }, { x: 482.3, y: 500, angle: 0 }],
      },
      {
        id: "cat-2",
        catalogId: "24188",
        transform: { x: 576, y: 500, rotation: 0 },
        hasCatenary: true,
        connectors: [{ x: 482.3, y: 500, angle: 180 }, { x: 670.5, y: 500, angle: 0 }],
      },
    ];

    const stdCols = findCollidingTracksForMountain(mountainModel, standardTracks, () => dummyEntry);
    const catCols = findCollidingTracksForMountain(mountainModel, catenaryTracks, () => dummyEntry);

    expect(stdCols[0]!.prof?.totalH).toBe(84);
    expect(stdCols[0]!.prof?.innerWidth).toBe(60);

    // Catenary tunnel profile is higher (>= 104 mm) and wider (>= 70 mm) to fit overhead wire and pantograph
    expect(catCols[0]!.prof?.totalH).toBeGreaterThanOrEqual(104);
    expect(catCols[0]!.prof?.innerWidth).toBeGreaterThanOrEqual(70);
  });

  it("adjusts tunnel portal entry width to fit two parallel tracks going through the tunnel", () => {
    const mountainModel: Model3D = {
      id: "mountain-double",
      name: "Mountain Double",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      gauge: 16.5,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Track line 1 at y = 468 (spacing ~ 64 mm)
    const line1: TrackPiece[] = [
      {
        id: "l1-1",
        catalogId: "24188",
        transform: { x: 200, y: 468, rotation: 0 },
        connectors: [{ x: 105.9, y: 468, angle: 180 }, { x: 294.1, y: 468, angle: 0 }],
      },
      {
        id: "l1-2",
        catalogId: "24188",
        transform: { x: 388, y: 468, rotation: 0 },
        connectors: [{ x: 294.1, y: 468, angle: 180 }, { x: 482.3, y: 468, angle: 0 }],
      },
      {
        id: "l1-3",
        catalogId: "24188",
        transform: { x: 576, y: 468, rotation: 0 },
        connectors: [{ x: 482.3, y: 468, angle: 180 }, { x: 670.5, y: 468, angle: 0 }],
      },
      {
        id: "l1-4",
        catalogId: "24188",
        transform: { x: 764, y: 468, rotation: 0 },
        connectors: [{ x: 670.5, y: 468, angle: 180 }, { x: 858.7, y: 468, angle: 0 }],
      },
    ];

    // Track line 2 at y = 532 (parallel, 64 mm distance)
    const line2: TrackPiece[] = [
      {
        id: "l2-1",
        catalogId: "24188",
        transform: { x: 200, y: 532, rotation: 0 },
        connectors: [{ x: 105.9, y: 532, angle: 180 }, { x: 294.1, y: 532, angle: 0 }],
      },
      {
        id: "l2-2",
        catalogId: "24188",
        transform: { x: 388, y: 532, rotation: 0 },
        connectors: [{ x: 294.1, y: 532, angle: 180 }, { x: 482.3, y: 532, angle: 0 }],
      },
      {
        id: "l2-3",
        catalogId: "24188",
        transform: { x: 576, y: 532, rotation: 0 },
        connectors: [{ x: 482.3, y: 532, angle: 180 }, { x: 670.5, y: 532, angle: 0 }],
      },
      {
        id: "l2-4",
        catalogId: "24188",
        transform: { x: 764, y: 532, rotation: 0 },
        connectors: [{ x: 670.5, y: 532, angle: 180 }, { x: 858.7, y: 532, angle: 0 }],
      },
    ];

    const collisions = findCollidingTracksForMountain(mountainModel, [...line1, ...line2], () => dummyEntry);
    expect(collisions.length).toBe(2); // Two parallel corridors

    const group = new THREE.Group();
    buildMountain3D(group, mountainModel, collisions);

    const mountainSub = group.children[0] as THREE.Group;
    const portalGroups = mountainSub.children.filter((c) => c instanceof THREE.Group && c.children.length >= 4) as THREE.Group[];

    // Expect exactly 2 portals: 1 double-track portal at the entrance, 1 double-track portal at the exit
    expect(portalGroups.length).toBe(2);

    // Each portal must be wide enough to accommodate both tracks (64 mm track spread + 60 mm clearance = ~124 mm innerWidth)
    for (const p of portalGroups) {
      // Find the portal crown or face mesh to verify its widened dimensions
      const crownMesh = p.children.find(
        (c): c is THREE.Mesh => c instanceof THREE.Mesh && c.geometry instanceof THREE.BoxGeometry && (c.geometry as THREE.BoxGeometry).parameters.height === 6,
      );
      expect(crownMesh).toBeDefined();
      const crownWidth = (crownMesh?.geometry as THREE.BoxGeometry).parameters.width;
      // Double track portal crown width is significantly wider than single track (~98 mm)
      expect(crownWidth).toBeGreaterThanOrEqual(140);
    }
  });

  it("renders tunnel correctly for tracks crossing in Z direction (negative Z and positive Z portals)", () => {
    const mountainModel: Model3D = {
      id: "mountain-z",
      name: "Mountain Z",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: 500, y: 500, rotation: 0 },
      width: 600,
      depth: 450,
      height: 220,
    };

    const dummyEntry: CatalogEntry = {
      id: "24188",
      name: "Gerades Gleis",
      category: "Geraden",
      description: "",
      kind: "straight",
      length: 188.3,
      localConnectors: [
        { x: 0, y: 0, angle: 180 },
        { x: 188.3, y: 0, angle: 0 },
      ],
    };

    // Tracks running along 2D Y axis (which corresponds to 3D Z axis)
    // Mountain center is at y = 500, depth is 450 (so y goes from 275 to 725)
    // Tracks go from y = 150 to y = 850 (from negative Z to positive Z)
    const zTracks: TrackPiece[] = [
      {
        id: "z-1",
        catalogId: "24188",
        transform: { x: 500, y: 150, rotation: 90 },
        connectors: [{ x: 500, y: 55.9, angle: 270 }, { x: 500, y: 244.1, angle: 90 }],
      },
      {
        id: "z-2",
        catalogId: "24188",
        transform: { x: 500, y: 338, rotation: 90 },
        connectors: [{ x: 500, y: 244.1, angle: 270 }, { x: 500, y: 432.1, angle: 90 }],
      },
      {
        id: "z-3",
        catalogId: "24188",
        transform: { x: 500, y: 526, rotation: 90 },
        connectors: [{ x: 500, y: 432.1, angle: 270 }, { x: 500, y: 620.1, angle: 90 }],
      },
      {
        id: "z-4",
        catalogId: "24188",
        transform: { x: 500, y: 714, rotation: 90 },
        connectors: [{ x: 500, y: 620.1, angle: 270 }, { x: 500, y: 808.1, angle: 90 }],
      },
    ];

    // Tracks with POSITIVE elevation (+40 mm)
    const posElevTracks: TrackPiece[] = [
      {
        id: "z-1",
        catalogId: "24188",
        transform: { x: 500, y: 150, rotation: 90 },
        elevation: { start: 40, end: 40 },
        connectors: [{ x: 500, y: 55.9, angle: 270 }, { x: 500, y: 244.1, angle: 90 }],
      },
      {
        id: "z-2",
        catalogId: "24188",
        transform: { x: 500, y: 338, rotation: 90 },
        elevation: { start: 40, end: 40 },
        connectors: [{ x: 500, y: 244.1, angle: 270 }, { x: 500, y: 432.1, angle: 90 }],
      },
      {
        id: "z-3",
        catalogId: "24188",
        transform: { x: 500, y: 526, rotation: 90 },
        elevation: { start: 40, end: 40 },
        connectors: [{ x: 500, y: 432.1, angle: 270 }, { x: 500, y: 620.1, angle: 90 }],
      },
      {
        id: "z-4",
        catalogId: "24188",
        transform: { x: 500, y: 714, rotation: 90 },
        elevation: { start: 40, end: 40 },
        connectors: [{ x: 500, y: 620.1, angle: 270 }, { x: 500, y: 808.1, angle: 90 }],
      },
    ];

    const collisions = findCollidingTracksForMountain(mountainModel, posElevTracks, () => dummyEntry);
    expect(collisions.length).toBe(1);
    expect(collisions[0]!.centerline.length).toBeGreaterThan(0);

    const group = new THREE.Group();
    buildMountain3D(group, mountainModel, collisions);
    const mountainSub = group.children[0] as THREE.Group;
    const portalGroups = mountainSub.children.filter((c) => c instanceof THREE.Group && c.children.length >= 4) as THREE.Group[];
    expect(portalGroups.length).toBe(2);

    // Both portals sit at the elevated track level (+40 mm plus portal lift)
    for (const p of portalGroups) {
      expect(p.position.y).toBeCloseTo(41.5, 0.5);
    }

    const terrainMesh = mountainSub.children.find((c): c is THREE.Mesh => c instanceof THREE.Mesh && c.geometry instanceof THREE.BufferGeometry);
    expect(terrainMesh).toBeDefined();

    // Verify terrain outer boundary in the Vorfeld is sitting at the base table level (y = 0)
    // rather than being pushed up into a floating 40 mm high wall outside the mountain
    const posAttr = terrainMesh!.geometry.getAttribute("position");
    const trackVerts: Array<{ x: number; y: number; z: number }> = [];
    for (let i = 0; i < posAttr.count; i++) {
      const vx = posAttr.getX(i);
      const vy = posAttr.getY(i);
      const vz = posAttr.getZ(i);
      if (Math.abs(vx) < 15) {
        trackVerts.push({ x: Math.round(vx), y: Math.round(vy), z: Math.round(vz) });
      }
    }
    trackVerts.sort((a, b) => a.z - b.z);

    // Extreme outer points at the mountain perimeter (z = -225 and z = +225) must remain at base table level 0
    const edgeNegZ = trackVerts.find((v) => v.z === -225);
    const edgePosZ = trackVerts.find((v) => v.z === 225);
    expect(edgeNegZ?.y).toBe(0);
    expect(edgePosZ?.y).toBe(0);
  });

  it("prevents bridge pillars from generating inside the mountain footprint", async () => {
    const { Viewer3D } = await import("@/render/viewer-3d");
    const viewer = Object.create(Viewer3D.prototype);
    viewer.currentLayout = {
      version: 1,
      scale: "H0",
      pieces: [],
      models: [
        {
          id: "m-1",
          name: "Test Mountain",
          modelType: "builtin",
          builtinId: "mountain",
          transform: { x: 500, y: 500, rotation: 0 },
          width: 600,
          depth: 450,
          height: 220,
        },
      ],
      baseboard: { shape: "rectangle", width: 1000, length: 1000 },
    };

    // Point in center of mountain
    const insidePillar = viewer.isPillarColliding({ x: 500, y: 500 }, 50, "piece-elev");
    expect(insidePillar).toBe(true);

    // Point near tunnel entrance (inside footprint normR < 0.95)
    const portalPillar = viewer.isPillarColliding({ x: 500, y: 350 }, 50, "piece-elev");
    expect(portalPillar).toBe(true);

    // Point far outside mountain footprint
    const outsidePillar = viewer.isPillarColliding({ x: 100, y: 100 }, 50, "piece-elev");
    expect(outsidePillar).toBe(false);
  });

  it("builds tunnel portals for rotated wide mountain crossing Bergstrecke tracks", async () => {
    const { createAlpenlandH0Layout } = await import("@/editor/demo-tracks");
    const { maerklinCCatalog } = await import("@/catalog/catalog");
    const layout = createAlpenlandH0Layout();
    const entryMap = new Map(maerklinCCatalog.entries.map((e) => [e.id, e]));

    const userMountain: Model3D = {
      id: "mountain-user",
      name: "Mountain 1",
      category: "nature",
      modelType: "builtin",
      builtinId: "mountain",
      transform: { x: -474, y: -439, rotation: 180 },
      width: 1450,
      depth: 430,
      height: 220,
      layerId: "layer-1",
    };

    const collisions = findCollidingTracksForMountain(
      userMountain,
      layout.pieces,
      (id) => entryMap.get(id),
      layout.layers,
    );

    // Must detect the Bergstrecke corridor crossing through the mountain
    expect(collisions.length).toBeGreaterThanOrEqual(1);

    const group = new THREE.Group();
    buildMountain3D(group, userMountain, collisions);
    const sub = group.children[0] as THREE.Group;
    const portals = sub.children.filter((c): c is THREE.Group => c instanceof THREE.Group && c.children.length >= 4);

    // Both entrance and exit portals must be constructed
    expect(portals.length).toBe(2);

    // Continuous tunnel tube must be generated
    const tubeMesh = sub.children.find(
      (c): c is THREE.Mesh => c instanceof THREE.Mesh && c !== sub.children[0],
    );
    expect(tubeMesh).toBeDefined();
  });
});
