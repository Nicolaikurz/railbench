import * as THREE from "three";
import { applyModelScale, type ModelScale } from "@/core/model-scale";
import type { Model3D } from "@/core/types";
import { BUILTIN_MODEL_BUILDERS, buildGenericBuilding3D } from "./models-3d";

export * from "./models-3d";

export function createBuiltinModel3DMesh(
  builtinId: string,
  model?: Model3D,
  scale: ModelScale = "H0",
  extraData?: any,
): THREE.Group {
  const group = new THREE.Group();

  const builder = BUILTIN_MODEL_BUILDERS[builtinId] || buildGenericBuilding3D;
  builder(group, model, extraData);

  // Schatten für alle Mesh-Bestandteile aktivieren
  group.traverse((obj) => {
    if (obj instanceof THREE.Mesh) {
      obj.castShadow = true;
      obj.receiveShadow = true;
    }
  });
  if (builtinId !== "mountain") {
    applyModelScale(group, scale);
  }
  return group;
}
