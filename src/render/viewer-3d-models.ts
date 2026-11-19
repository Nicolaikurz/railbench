import * as THREE from "three";
import type { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import type { CatalogEntry } from "@/catalog/catalog";
import { catalogRegistry } from "@/catalog/catalog";
import type { Layout, Model3D } from "@/core/types";
import { deg2rad } from "@/core/geometry";
import { createBuiltinModel3DMesh, findCollidingTracksForMountain } from "@/render/builtin-models-3d";
import { findBuiltinModelDef } from "@/core/models-3d";
import { getScaleFactor, type ModelScale } from "@/core/model-scale";

import { SHARED_WOOD_PLATE_MAT } from "./viewer-3d-roads";

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

const _buildingPlateGeoCache = new Map<string, THREE.BoxGeometry>();

export function buildBuildingWoodPlate3D(
  woodPlateGroup: THREE.Group,
  model: Model3D,
  baseElevation: number,
  thickness: number,
) {
  const scale = model.scale ?? 1.0;
  const width = Math.max(20, Math.round((model.width * scale + 14) * 10) / 10);
  const depth = Math.max(20, Math.round((model.depth * scale + 14) * 10) / 10);
  const key = `${width}:${thickness}:${depth}`;

  let geo = _buildingPlateGeoCache.get(key);
  if (!geo) {
    if (_buildingPlateGeoCache.size >= 128) {
      const oldestKey = _buildingPlateGeoCache.keys().next().value;
      if (oldestKey !== undefined) {
        const oldGeo = _buildingPlateGeoCache.get(oldestKey);
        if (oldGeo) oldGeo.userData.shared = false;
        _buildingPlateGeoCache.delete(oldestKey);
      }
    }
    geo = new THREE.BoxGeometry(width, thickness, depth);
    geo.userData.shared = true;
    _buildingPlateGeoCache.set(key, geo);
  }

  const mesh = new THREE.Mesh(geo, SHARED_WOOD_PLATE_MAT);
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
      extraData = { collisions, allMountains, layout };
    } else if (model.builtinId === "ground-terrain" && layout) {
      extraData = { layout, entryMap };
    }
    const mesh = createBuiltinModel3DMesh(model.builtinId || "station", model, "H0", extraData);
    mesh.position.set(model.transform.x, elevation, model.transform.y);
    mesh.rotation.y = -deg2rad(model.transform.rotation);
    if (model.builtinId === "mountain" || model.builtinId === "ground-terrain") {
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

interface TemplateMeshPart {
  geometry: THREE.BufferGeometry;
  material: THREE.Material | THREE.Material[];
  matrix: THREE.Matrix4;
  castShadow: boolean;
  receiveShadow: boolean;
}

function extractTemplateMeshes(root: THREE.Object3D): TemplateMeshPart[] {
  root.updateMatrixWorld(true);
  const parts: TemplateMeshPart[] = [];
  root.traverse((obj) => {
    if (obj instanceof THREE.Mesh) {
      parts.push({
        geometry: obj.geometry,
        material: obj.material,
        matrix: obj.matrixWorld.clone(),
        castShadow: obj.castShadow,
        receiveShadow: obj.receiveShadow,
      });
    }
  });
  return parts;
}

function createInstancedGroup(
  parts: TemplateMeshPart[],
  list: Array<{ model: Model3D; elev: number }>,
  layoutScale: ModelScale,
  centerOffset: { x: number; y: number; z: number } = { x: 0, y: 0, z: 0 },
): THREE.Group {
  const group = new THREE.Group();
  const count = list.length;
  const modelMatrix = new THREE.Matrix4();
  const quat = new THREE.Quaternion();
  const pos = new THREE.Vector3();
  const scaleVec = new THREE.Vector3();

  for (const part of parts) {
    const instanced = new THREE.InstancedMesh(part.geometry, part.material, count);
    instanced.castShadow = part.castShadow;
    instanced.receiveShadow = part.receiveShadow;

    for (let i = 0; i < count; i++) {
      const { model, elev } = list[i]!;
      const rotY = -deg2rad(model.transform.rotation);
      const s = getBuiltinScaleFactor(model, layoutScale) * (model.scale || 1);

      pos.set(
        model.transform.x - centerOffset.x,
        elev - centerOffset.y,
        model.transform.y - centerOffset.z,
      );
      quat.setFromAxisAngle(new THREE.Vector3(0, 1, 0), rotY);
      scaleVec.set(s, s, s);

      modelMatrix.compose(pos, quat, scaleVec);
      const finalMatrix = new THREE.Matrix4().multiplyMatrices(modelMatrix, part.matrix);
      instanced.setMatrixAt(i, finalMatrix);
    }

    instanced.instanceMatrix.needsUpdate = true;
    group.add(instanced);
  }

  return group;
}

/**
 * Baut alle Modelle der Anlage. Mehrfach vorkommende identische Builtin-Modelle
 * (wie Bäume, Lampen, Häuser) werden automatisch in GPU-InstancedMeshes gebatcht
 * und unterstützen Level of Detail (LOD) bei Kamera-Abstand.
 */
export function buildAllModels3D(
  modelsGroup: THREE.Group,
  gltfCache: Map<string, THREE.Group>,
  gltfLoader: GLTFLoader,
  modelsWithElev: Array<{ model: Model3D; elev: number }>,
  layoutScale: ModelScale,
  layout?: Layout,
  entryMap?: Map<string, CatalogEntry>,
) {
  const builtinGroups = new Map<string, Array<{ model: Model3D; elev: number }>>();
  const individualModels: Array<{ model: Model3D; elev: number }> = [];

  for (const item of modelsWithElev) {
    const { model } = item;
    if (
      model.modelType === "builtin" &&
      model.builtinId &&
      model.builtinId !== "mountain" &&
      model.builtinId !== "ground-terrain"
    ) {
      let list = builtinGroups.get(model.builtinId);
      if (!list) {
        list = [];
        builtinGroups.set(model.builtinId, list);
      }
      list.push(item);
    } else {
      individualModels.push(item);
    }
  }

  // Gruppierte Builtin-Modelle: Wenn N >= 2, per InstancedMesh instanziieren
  for (const [builtinId, list] of builtinGroups.entries()) {
    if (list.length >= 2) {
      const template = createBuiltinModel3DMesh(builtinId, list[0]!.model, "H0");
      template.scale.set(1, 1, 1);
      template.position.set(0, 0, 0);
      template.rotation.set(0, 0, 0);
      template.updateMatrixWorld(true);

      const lodNode = template.children.find((c) => (c as any).isLOD) as THREE.LOD | undefined;

      if (lodNode && lodNode.levels.length > 1) {
        // LOD vorhanden: InstancedMesh für jede Detailstufe anlegen
        let avgX = 0, avgY = 0, avgZ = 0;
        for (const item of list) {
          avgX += item.model.transform.x;
          avgY += item.elev;
          avgZ += item.model.transform.y;
        }
        avgX /= list.length;
        avgY /= list.length;
        avgZ /= list.length;

        const instancedLOD = new THREE.LOD();
        instancedLOD.position.set(avgX, avgY, avgZ);

        for (const level of lodNode.levels) {
          const parts = extractTemplateMeshes(level.object);
          const levelGroup = createInstancedGroup(parts, list, layoutScale, { x: avgX, y: avgY, z: avgZ });
          instancedLOD.addLevel(levelGroup, level.distance);
        }

        modelsGroup.add(instancedLOD);
      } else {
        const parts = extractTemplateMeshes(template);
        const group = createInstancedGroup(parts, list, layoutScale);
        modelsGroup.add(group);
      }
    } else {
      for (const item of list) {
        individualModels.push(item);
      }
    }
  }

  // Einzelne Modelle rendern (Berge, Unikate, benutzerdefinierte GLTF-Dateien)
  for (const item of individualModels) {
    buildModel3D(
      modelsGroup,
      gltfCache,
      gltfLoader,
      item.model,
      item.elev,
      layoutScale,
      layout,
      entryMap,
    );
  }
}
