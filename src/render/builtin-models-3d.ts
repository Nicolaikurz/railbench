import * as THREE from "three";
import * as BufferGeometryUtils from "three/examples/jsm/utils/BufferGeometryUtils.js";
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

function normalizeGeometryForMerge(
  sourceGeo: THREE.BufferGeometry,
  matrixWorld: THREE.Matrix4,
  vertexColor?: THREE.Color,
  preserveUv = false,
): THREE.BufferGeometry | null {
  if (!sourceGeo.getAttribute("position")) return null;
  const g = sourceGeo.index ? sourceGeo.toNonIndexed() : sourceGeo.clone();
  g.applyMatrix4(matrixWorld);

  // Bei gespiegelter Transformation (negative Determinante) Dreiecks-Winding umdrehen
  if (matrixWorld.determinant() < 0) {
    const pos = g.getAttribute("position");
    for (let i = 0; i < pos.count; i += 3) {
      const x1 = pos.getX(i + 1), y1 = pos.getY(i + 1), z1 = pos.getZ(i + 1);
      pos.setXYZ(i + 1, pos.getX(i + 2), pos.getY(i + 2), pos.getZ(i + 2));
      pos.setXYZ(i + 2, x1, y1, z1);
    }
    pos.needsUpdate = true;
    g.deleteAttribute("normal");
  }

  if (!g.getAttribute("normal")) {
    g.computeVertexNormals();
  }

  for (const attrName of Object.keys(g.attributes)) {
    if (attrName === "position" || attrName === "normal") continue;
    if (preserveUv && attrName === "uv") continue;
    g.deleteAttribute(attrName);
  }

  if (vertexColor) {
    const count = g.getAttribute("position").count;
    const colors = new Float32Array(count * 3);
    const r = vertexColor.r;
    const gr = vertexColor.g;
    const b = vertexColor.b;
    for (let i = 0; i < count; i++) {
      colors[i * 3] = r;
      colors[i * 3 + 1] = gr;
      colors[i * 3 + 2] = b;
    }
    g.setAttribute("color", new THREE.BufferAttribute(colors, 3));
  }

  return g;
}

/**
 * Fasst alle statischen Teil-Meshes einer Builtin-Modellgruppe pro Material-Klasse
 * mittels Vertex-Colors & BufferGeometryUtils.mergeGeometries zusammen.
 * Reduziert typische Gebäude von 25–60 Draw-Calls auf 1–4 Draw-Calls.
 */
export function mergeStaticModelGroup(group: THREE.Group): void {
  group.updateMatrixWorld(true);

  const sourceMeshes: THREE.Mesh[] = [];
  group.traverse((obj) => {
    if (obj instanceof THREE.Mesh && obj.geometry && obj.material && !Array.isArray(obj.material)) {
      sourceMeshes.push(obj);
    }
  });

  if (sourceMeshes.length <= 1) return;

  interface MergeBucket {
    geos: THREE.BufferGeometry[];
    material: THREE.Material;
    castShadow: boolean;
    receiveShadow: boolean;
  }

  const buckets = new Map<string, MergeBucket>();
  const keptNonMeshChildren: THREE.Object3D[] = [];
  for (const child of group.children) {
    if (!(child instanceof THREE.Mesh) && !(child instanceof THREE.Group)) {
      keptNonMeshChildren.push(child);
    }
  }

  for (const mesh of sourceMeshes) {
    const mat = mesh.material as THREE.Material;
    const isStd = mat instanceof THREE.MeshStandardMaterial;
    const hasTexture = isStd && Boolean(mat.map || mat.normalMap || mat.roughnessMap || mat.metalnessMap || mat.alphaMap);
    const hasEmissive = isStd && mat.emissiveIntensity > 0 && (mat.emissive.r > 0 || mat.emissive.g > 0 || mat.emissive.b > 0);
    const canVertexColorMerge = isStd && !mat.transparent && !hasTexture && !hasEmissive;

    let key: string;
    let bucketMat: THREE.Material;
    let normalized: THREE.BufferGeometry | null;

    if (canVertexColorMerge) {
      const roughTier = mat.roughness >= 0.7 ? 2 : mat.roughness >= 0.35 ? 1 : 0;
      const metalTier = mat.metalness >= 0.6 ? 2 : mat.metalness >= 0.25 ? 1 : 0;
      key = `vc:${roughTier}:${metalTier}:${mat.flatShading ? 1 : 0}:${mat.side}:${mesh.castShadow ? 1 : 0}:${mesh.receiveShadow ? 1 : 0}`;
      const existing = buckets.get(key);
      if (existing) {
        bucketMat = existing.material;
      } else {
        const roughness = roughTier === 2 ? 0.88 : roughTier === 1 ? 0.52 : 0.2;
        const metalness = metalTier === 2 ? 0.72 : metalTier === 1 ? 0.42 : 0.05;
        bucketMat = new THREE.MeshStandardMaterial({
          color: 0xffffff,
          vertexColors: true,
          roughness,
          metalness,
          flatShading: mat.flatShading,
          side: mat.side,
        });
        bucketMat.userData.shared = true;
      }
      normalized = normalizeGeometryForMerge(mesh.geometry, mesh.matrixWorld, mat.color, false);
    } else {
      const stdSig = isStd
        ? `${mat.color.getHexString()}:${Math.round(mat.roughness * 100)}:${Math.round(mat.metalness * 100)}:${mat.emissive.getHexString()}:${Math.round(mat.emissiveIntensity * 100)}:${mat.transparent ? 1 : 0}:${Math.round(mat.opacity * 100)}:${mat.side}:${mat.flatShading ? 1 : 0}`
        : mat.uuid;
      key = `mat:${stdSig}:${mesh.castShadow ? 1 : 0}:${mesh.receiveShadow ? 1 : 0}`;
      const existing = buckets.get(key);
      bucketMat = existing ? existing.material : mat;
      bucketMat.userData.shared = true;
      normalized = normalizeGeometryForMerge(mesh.geometry, mesh.matrixWorld, undefined, hasTexture);
    }

    if (!normalized) continue;

    let bucket = buckets.get(key);
    if (!bucket) {
      bucket = {
        geos: [],
        material: bucketMat,
        castShadow: mesh.castShadow,
        receiveShadow: mesh.receiveShadow,
      };
      buckets.set(key, bucket);
    }
    bucket.geos.push(normalized);
  }

  if (buckets.size === 0) return;

  const mergedMeshes: THREE.Mesh[] = [];
  for (const bucket of buckets.values()) {
    const mergedGeo =
      bucket.geos.length === 1
        ? bucket.geos[0]!
        : BufferGeometryUtils.mergeGeometries(bucket.geos, false);

    if (bucket.geos.length > 1) {
      for (const g of bucket.geos) g.dispose();
    }
    if (!mergedGeo) continue;

    mergedGeo.userData.shared = true;
    const mergedMesh = new THREE.Mesh(mergedGeo, bucket.material);
    mergedMesh.castShadow = bucket.castShadow;
    mergedMesh.receiveShadow = bucket.receiveShadow;
    mergedMeshes.push(mergedMesh);
  }

  if (mergedMeshes.length === 0) return;

  group.clear();
  for (const kept of keptNonMeshChildren) {
    group.add(kept);
  }
  for (const m of mergedMeshes) {
    group.add(m);
  }
}

function populateModelWithLOD(
  target: THREE.Group,
  builtinId: string,
  model?: Model3D,
  extraData?: any,
): void {
  const builder = BUILTIN_MODEL_BUILDERS[builtinId] || buildGenericBuilding3D;
  const lowBuilder = BUILTIN_MODEL_LOW_POLY_BUILDERS[builtinId];
  const shouldMerge = builtinId !== "mountain" && builtinId !== "ground-terrain";

  if (!lowBuilder) {
    builder(target, model, extraData);
    tagShadowAndShared(target);
    if (shouldMerge) mergeStaticModelGroup(target);
    return;
  }

  // Level of Detail (LOD): High-Detail bei Nahansicht, vereinfachte Geometrie bei Übersicht
  const lod = new THREE.LOD();

  const highGroup = new THREE.Group();
  builder(highGroup, model, extraData);
  tagShadowAndShared(highGroup);
  if (shouldMerge) mergeStaticModelGroup(highGroup);

  const lowGroup = new THREE.Group();
  lowBuilder(lowGroup, model, extraData);
  tagShadowAndShared(lowGroup);
  if (shouldMerge) mergeStaticModelGroup(lowGroup);

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
  // Mountains & Ground-Terrains haben pro Instanz eigene adaptive Geometrien
  const isDynamicTerrain = builtinId === "mountain" || builtinId === "ground-terrain";
  const canCache = !isDynamicTerrain && !extraData;
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
  if (!isDynamicTerrain) {
    applyModelScale(group, scale);
  }
  return group;
}
