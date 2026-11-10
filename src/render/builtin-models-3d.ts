import * as THREE from "three";
import { applyModelScale, type ModelScale } from "@/core/model-scale";
import type { Model3D } from "@/core/types";
import { BUILTIN_MODEL_BUILDERS, buildGenericBuilding3D } from "./models-3d";

export * from "./models-3d";

export function createBuiltinModel3DMesh(
  builtinId: string,
  _model: Model3D,
  scale: ModelScale = "H0",
): THREE.Group {
  const group = new THREE.Group();

  const builder = BUILTIN_MODEL_BUILDERS[builtinId] || buildGenericBuilding3D;
  builder(group);

  // Schatten für alle Mesh-Bestandteile aktivieren
  group.traverse((obj) => {
    if (obj instanceof THREE.Mesh) {
      obj.castShadow = true;
      obj.receiveShadow = true;
    }
  });
  applyModelScale(group, scale);
  return group;
}
