import fs from "node:fs";
import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { buildTrainHall } from "@/render/viewer-3d-trainhall";
import { buildRealisticLandscape } from "@/render/viewer-3d-landscape";
import { buildTrackPiece3D } from "@/render/viewer-3d-track";
import { buildWoodPlatePiece3D, buildAllWoodPlates3D } from "@/render/viewer-3d-roads";
import { buildAllModels3D, buildBuildingWoodPlate3D, buildModel3D } from "@/render/viewer-3d-models";
import { catalogRegistry } from "@/catalog/catalog";

function countSceneObjects(root: THREE.Object3D) {
  let meshCount = 0;
  let instancedMeshCount = 0;
  let totalInstances = 0;
  let groupCount = 0;
  const geometries = new Set<THREE.BufferGeometry>();
  const materials = new Set<THREE.Material>();
  let vertexCount = 0;
  let triangleCount = 0;

  root.traverse((obj) => {
    if (obj instanceof THREE.InstancedMesh) {
      instancedMeshCount++;
      totalInstances += obj.count;
    } else if (obj instanceof THREE.Mesh) {
      meshCount++;
    } else if (obj instanceof THREE.Group) {
      groupCount++;
    }
    if ((obj as THREE.Mesh).geometry) {
      geometries.add((obj as THREE.Mesh).geometry);
      const pos = (obj as THREE.Mesh).geometry.attributes.position;
      if (pos) {
        const count = (obj as any).count ? pos.count * (obj as any).count : pos.count;
        vertexCount += count;
        if ((obj as THREE.Mesh).geometry.index) {
          triangleCount += (((obj as THREE.Mesh).geometry.index?.count ?? 0) / 3) * ((obj as any).count || 1);
        } else {
          triangleCount += (pos.count / 3) * ((obj as any).count || 1);
        }
      }
    }
    if ((obj as THREE.Mesh).material) {
      const mat = (obj as THREE.Mesh).material;
      if (Array.isArray(mat)) {
        mat.forEach((m) => materials.add(m));
      } else {
        materials.add(mat);
      }
    }
  });

  return {
    meshCount,
    instancedMeshCount,
    totalInstances,
    groupCount,
    uniqueGeometries: geometries.size,
    uniqueMaterials: materials.size,
    vertexCount,
    triangleCount: Math.round(triangleCount),
  };
}

describe("3D Viewer Benchmark", () => {
  it("profiles 3d scene statistics", () => {
    const layoutData = JSON.parse(fs.readFileSync("demotracks/alpenland-h0.railbench", "utf8"));
    const cat = catalogRegistry.get("maerklin-c-h0")!;

    console.log("\n=================== 3D VIEWER PROFILE ===================");

    // 1. TrainHall
    const hallGroup = new THREE.Group();
    buildTrainHall(hallGroup, layoutData, false);
    console.log("Train Hall Scene:", countSceneObjects(hallGroup));

    // 2. Realistic Landscape
    const landGroup = new THREE.Group();
    buildRealisticLandscape(landGroup, layoutData, false);
    console.log("Realistic Landscape Scene:", countSceneObjects(landGroup));

    // 3. Tracks & Models & Woodplates
    const trackGroup = new THREE.Group();
    const woodPlateGroup = new THREE.Group();
    const modelsGroup = new THREE.Group();

    const entryMap = new Map();
    if (cat?.entries) {
      for (const e of cat.entries) entryMap.set(e.id, e);
    }

    const woodPlateItems: any[] = [];
    for (const piece of layoutData.pieces) {
      let entry = entryMap.get(piece.catalogId) || catalogRegistry.findEntry(piece.catalogId);
      if (!entry) {
        entry = { id: piece.catalogId, name: piece.catalogId, kind: "straight", length: 188.3 };
      }
      buildTrackPiece3D(trackGroup, piece, entry, 0, 0, "types", undefined, layoutData);
      woodPlateItems.push({ piece, entry, startElev: 0, endElev: 0, thickness: 8 });
    }
    buildAllWoodPlates3D(woodPlateGroup, woodPlateItems);

    const modelsWithElev = (layoutData.models || []).map((m: any) => ({ model: m, elev: 0 }));
    buildAllModels3D(modelsGroup, new Map(), {} as any, modelsWithElev, "H0", layoutData, entryMap);
    for (const model of layoutData.models || []) {
      buildBuildingWoodPlate3D(woodPlateGroup, model, 0, 8);
    }

    console.log("Track Group:", countSceneObjects(trackGroup));
    console.log("WoodPlate Group:", countSceneObjects(woodPlateGroup));
    console.log("Models Group:", countSceneObjects(modelsGroup));

    // Overall scene
    const scene = new THREE.Group();
    scene.add(hallGroup);
    scene.add(trackGroup);
    scene.add(woodPlateGroup);
    scene.add(modelsGroup);

    const hallStats = countSceneObjects(hallGroup);
    const landStats = countSceneObjects(landGroup);
    const trackStats = countSceneObjects(trackGroup);
    const woodPlateStats = countSceneObjects(woodPlateGroup);
    const sceneStats = countSceneObjects(scene);

    console.log("TOTAL COMBINED SCENE:", sceneStats);
    console.log("=========================================================\n");

    // Verify memory reduction & instancing assertions
    expect(woodPlateStats.uniqueMaterials).toBeLessThanOrEqual(2);
    expect(trackStats.uniqueGeometries).toBeLessThan(350);
    expect(trackStats.uniqueMaterials).toBeLessThan(100);
    expect(hallStats.instancedMeshCount).toBeGreaterThanOrEqual(8);
    expect(landStats.instancedMeshCount).toBeGreaterThanOrEqual(4);
    expect(sceneStats.uniqueGeometries).toBeLessThan(2000);
    expect(sceneStats.uniqueMaterials).toBeLessThan(350);
  });

  it("verifies Level of Detail (LOD) dynamically switches geometry based on camera distance", () => {
    const layoutData = JSON.parse(fs.readFileSync("demotracks/alpenland-h0.railbench", "utf8"));
    const modelsGroup = new THREE.Group();
    const modelsWithElev = (layoutData.models || []).map((m: any) => ({ model: m, elev: 0 }));
    buildAllModels3D(modelsGroup, new Map(), {} as any, modelsWithElev, "H0", layoutData);

    // Verify LOD nodes exist
    const lodNodes: THREE.LOD[] = [];
    modelsGroup.traverse((obj) => {
      if ((obj as any).isLOD) {
        lodNodes.push(obj as THREE.LOD);
      }
    });
    expect(lodNodes.length).toBeGreaterThan(0);

    const camera = new THREE.PerspectiveCamera(45, 1, 1, 100000);

    // 1. Camera close up (< 3400 mm)
    camera.position.set(0, 500, 0);
    camera.updateMatrixWorld(true);
    for (const lod of lodNodes) {
      lod.update(camera);
    }

    const countVisible = (root: THREE.Object3D) => {
      let visibleMeshes = 0;
      let triangles = 0;
      root.traverse((obj) => {
        let cur: THREE.Object3D | null = obj;
        let isVis = true;
        while (cur) {
          if (!cur.visible) {
            isVis = false;
            break;
          }
          cur = cur.parent;
        }
        if (!isVis) return;
        if (obj instanceof THREE.Mesh) {
          visibleMeshes++;
          const pos = obj.geometry?.attributes?.position;
          if (pos) {
            const mul = (obj as any).count || 1;
            if (obj.geometry.index) {
              triangles += (obj.geometry.index.count / 3) * mul;
            } else {
              triangles += (pos.count / 3) * mul;
            }
          }
        }
      });
      return { visibleMeshes, triangles: Math.round(triangles) };
    };

    const nearStats = countVisible(modelsGroup);

    // 2. Camera zoomed out (> 3400 mm, e.g. 8000 mm overview)
    camera.position.set(0, 8000, 0);
    camera.updateMatrixWorld(true);
    for (const lod of lodNodes) {
      lod.update(camera);
    }

    const farStats = countVisible(modelsGroup);

    console.log("LOD Near Stats (Close inspection):", nearStats);
    console.log("LOD Far Stats (Layout overview):", farStats);

    // In far overview, triangles and visible meshes must be reduced significantly
    expect(farStats.triangles).toBeLessThan(nearStats.triangles);
    expect(farStats.visibleMeshes).toBeLessThan(nearStats.visibleMeshes);
  });
});
