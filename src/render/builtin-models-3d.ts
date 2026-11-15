import * as THREE from "three";
import { applyModelScale, type ModelScale } from "@/core/model-scale";
import type { Model3D } from "@/core/types";
import { BUILTIN_MODEL_BUILDERS, BUILTIN_MODEL_LOW_POLY_BUILDERS, buildGenericBuilding3D } from "./models-3d";

export * from "./models-3d";

const _builtinModelTemplateCache = new Map<string, THREE.Group>();

function tagShadowAndShared(target: THREE.Object3D) {
  target.traverse((obj) => {
    if (obj instanceof THREE.Mesh) {
      if (obj.geometry) obj.geometry.userData.shared = true;
      if (Array.isArray(obj.material)) {
        obj.material.forEach((m) => (m.userData.shared = true));
      } else if (obj.material) {
        obj.material.userData.shared = true;
      }
      const isGroundPlane =
        obj.position.y <= 3.5 &&
        obj.geometry instanceof THREE.BoxGeometry &&
        ((obj.geometry as any).parameters?.height ?? 10) <= 3.5;
      obj.castShadow = !isGroundPlane;
      obj.receiveShadow = true;
    }
  });
}

function populateModelWithLOD(
  target: THREE.Group,
  builtinId: string,
  model?: Model3D,
  extraData?: any,
): void {
  const builder = BUILTIN_MODEL_BUILDERS[builtinId] || buildGenericBuilding3D;
  const lowBuilder = BUILTIN_MODEL_LOW_POLY_BUILDERS[builtinId];

  if (!lowBuilder) {
    builder(target, model, extraData);
    tagShadowAndShared(target);
    return;
  }

  // Level of Detail (LOD): High-Detail bei Nahansicht, vereinfachte Geometrie bei Übersicht
  const lod = new THREE.LOD();

  const highGroup = new THREE.Group();
  builder(highGroup, model, extraData);
  tagShadowAndShared(highGroup);

  const lowGroup = new THREE.Group();
  lowBuilder(lowGroup, model, extraData);
  tagShadowAndShared(lowGroup);

  // Stufe 0: Volle Details bei Distanz < 3400 mm
  lod.addLevel(highGroup, 0);
  // Stufe 1: Reduzierte Geometrien bei Distanz >= 3400 mm (Übersicht der Anlage)
  lod.addLevel(lowGroup, 3400);

  target.add(lod);
}

export function createBuiltinModel3DMesh(
  builtinId: string,
  model?: Model3D,
  scale: ModelScale = "H0",
  extraData?: any,
): THREE.Group {
  // Mountains haben pro Instanz eigene Pfad-Geometrien
  const canCache = builtinId !== "mountain" && !extraData;
  const cacheKey = `${builtinId}:${scale}`;

  if (canCache) {
    let template = _builtinModelTemplateCache.get(cacheKey);
    if (!template) {
      template = new THREE.Group();
      populateModelWithLOD(template, builtinId, model, extraData);
      applyModelScale(template, scale);
      _builtinModelTemplateCache.set(cacheKey, template);
    }
    return template.clone();
  }

  const group = new THREE.Group();
  populateModelWithLOD(group, builtinId, model, extraData);
  if (builtinId !== "mountain") {
    applyModelScale(group, scale);
  }
  return group;
}
