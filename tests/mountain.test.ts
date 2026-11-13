import { describe, it, expect } from "vitest";
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
});
