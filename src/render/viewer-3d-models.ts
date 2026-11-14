import * as THREE from "three";
import type { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import type { CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import type { Layout, Model3D } from "@/core/types";
import { deg2rad } from "@/core/geometry";
import { createBuiltinModel3DMesh, findCollidingTracksForMountain } from "@/render/builtin-models-3d";
import { findBuiltinModelDef } from "@/core/models-3d";
import { getScaleFactor, type ModelScale } from "@/core/model-scale";

export function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const clean = base64.replace(/^data:.*?;base64,/, "");
  const binaryString = atob(clean);
  const len = binaryString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  return bytes.buffer;
}

export function getBuiltinScaleFactor(model: Model3D, layoutScale: ModelScale): number {
  const def = model.builtinId ? findBuiltinModelDef(model.builtinId) : undefined;
  if (def && def.width > 0 && model.width > 0) return model.width / def.width;
  return getScaleFactor(layoutScale);
}

export function buildBuildingWoodPlate3D(
  woodPlateGroup: THREE.Group,
  model: Model3D,
  baseElevation: number,
  thickness: number,
) {
  const scale = model.scale ?? 1.0;
  const width = Math.max(20, model.width * scale + 14);
  const depth = Math.max(20, model.depth * scale + 14);

  const geo = new THREE.BoxGeometry(width, thickness, depth);
  const woodPlateMat = new THREE.MeshStandardMaterial({
    color: 0xcbb89d,
    roughness: 0.85,
    metalness: 0.05,
  });
  const mesh = new THREE.Mesh(geo, woodPlateMat);
  mesh.position.set(model.transform.x, baseElevation - thickness / 2, model.transform.y);
  mesh.rotation.y = -deg2rad(model.transform.rotation);
  mesh.castShadow = true;
  mesh.receiveShadow = true;
  woodPlateGroup.add(mesh);
}

export function loadCustomModel3D(
  modelsGroup: THREE.Group,
  gltfCache: Map<string, THREE.Group>,
  gltfLoader: GLTFLoader,
  model: Model3D,
  elevation: number,
) {
  if (!model.fileData) return;
  try {
    if (model.modelType === "glb") {
      const arrayBuffer = base64ToArrayBuffer(model.fileData);
      gltfLoader.parse(
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
          gltfCache.set(model.id, root);
          const clone = root.clone();
          clone.position.set(model.transform.x, elevation, model.transform.y);
          clone.rotation.y = -deg2rad(model.transform.rotation);
          if (model.scale && model.scale !== 1.0) {
            clone.scale.setScalar(model.scale);
          }
          modelsGroup.add(clone);
        },
        (err) => console.error("Error loading glb model in 3D viewer:", err),
      );
    } else {
      let text = model.fileData;
      if (text.startsWith("data:")) {
        text = atob(text.split(",")[1] || "");
      }
      gltfLoader.parse(
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
          gltfCache.set(model.id, root);
          const clone = root.clone();
          clone.position.set(model.transform.x, elevation, model.transform.y);
          clone.rotation.y = -deg2rad(model.transform.rotation);
          if (model.scale && model.scale !== 1.0) {
            clone.scale.setScalar(model.scale);
          }
          modelsGroup.add(clone);
        },
        (err) => console.error("Error loading gltf model in 3D viewer:", err),
      );
    }
  } catch (e) {
    console.error("Failed to parse custom 3D model:", e);
  }
}

export function buildModel3D(
  modelsGroup: THREE.Group,
  gltfCache: Map<string, THREE.Group>,
  gltfLoader: GLTFLoader,
  model: Model3D,
  elevation: number,
  layoutScale: ModelScale,
  layout?: Layout,
  entryMap?: Map<string, CatalogEntry>,
) {
  if (model.modelType === "builtin") {
    let extraData: any = undefined;
    if (model.builtinId === "mountain" && layout) {
      const collisions = findCollidingTracksForMountain(
        model,
        layout.pieces,
        (id) => entryMap?.get(id) ?? catalogRegistry.findEntry(id),
        layout.layers,
      );
      const allMountains = (layout.models ?? []).filter((m) => m.builtinId === "mountain");
      extraData = { collisions, allMountains };
    }
    const mesh = createBuiltinModel3DMesh(model.builtinId || "station", model, "H0", extraData);
    mesh.position.set(model.transform.x, elevation, model.transform.y);
    mesh.rotation.y = -deg2rad(model.transform.rotation);
    if (model.builtinId === "mountain") {
      mesh.scale.set(model.scale || 1, model.scale || 1, model.scale || 1);
    } else {
      mesh.scale.setScalar(getBuiltinScaleFactor(model, layoutScale) * (model.scale || 1));
    }
    modelsGroup.add(mesh);
  } else if (model.fileData) {
    const cached = gltfCache.get(model.id);
    if (cached) {
      const clone = cached.clone();
      clone.position.set(model.transform.x, elevation, model.transform.y);
      clone.rotation.y = -deg2rad(model.transform.rotation);
      if (model.scale && model.scale !== 1.0) {
        clone.scale.setScalar(model.scale);
      }
      modelsGroup.add(clone);
    } else {
      loadCustomModel3D(modelsGroup, gltfCache, gltfLoader, model, elevation);
    }
  }
}
