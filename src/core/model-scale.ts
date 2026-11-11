import * as THREE from "three";

export type ModelScale = "H0" | "TT" | "N" | "Z" | "1" | "G" | "0";

/** Maßstab-Nenner (1:x). */
export const MODEL_SCALE_RATIOS: Record<ModelScale, number> = {
  "0": 45,
  "1": 32,
  G: 22.5,
  H0: 87,
  TT: 120,
  N: 160,
  Z: 220,
};

/** Maßstab, in dem alle eingebauten Modelle gezeichnet sind (Geometrie in mm). */
export const BUILTIN_BASE_SCALE: ModelScale = "H0";

/** Normalisiert Strings wie "h0", "1", "G", "0" oder undefined zu einem gültigen ModelScale. */
export function parseModelScale(value?: string | null): ModelScale {
  const s = (value ?? "").trim().toUpperCase();
  if (s in MODEL_SCALE_RATIOS) return s as ModelScale;
  if (s === "SPUR 0" || s === "SPUR0" || s === "GAUGE 0" || s === "GAUGE0" || s === "SCALE 0" || s === "O") return "0";
  if (s === "SPUR 1" || s === "SPUR1" || s === "GAUGE 1" || s === "GAUGE1" || s === "SCALE 1") return "1";
  if (s === "SPUR G" || s === "SPURG" || s === "GAUGE G" || s === "GAUGEG" || s === "SCALE G" || s === "LGB") return "G";
  return BUILTIN_BASE_SCALE;
}

/** Faktor, um etwas von einem Maßstab in einen anderen umzurechnen (H0 -> N = 87/160 ≈ 0.544). */
export function getScaleFactor(to: ModelScale, from: ModelScale = BUILTIN_BASE_SCALE): number {
  return MODEL_SCALE_RATIOS[from] / MODEL_SCALE_RATIOS[to];
}

/**
 * Skaliert eine Three.js-Gruppe gleichmäßig auf den Zielmaßstab.
 * Die Modelle haben ihren Ursprung am Boden (y = 0), daher bleibt die Grundfläche beim Skalieren an Ort und Stelle.
 * Idempotent: mehrfaches Aufrufen setzt den Faktor immer absolut, nicht relativ.
 */
export function applyModelScale(
  group: THREE.Object3D,
  to: ModelScale,
  from: ModelScale = BUILTIN_BASE_SCALE,
): THREE.Object3D {
  const f = getScaleFactor(to, from);
  group.scale.setScalar(f);
  group.userData.modelScale = to;
  group.updateMatrixWorld(true);
  return group;
}

/** Skaliert Maße (mm) eines Modells, auf 0,1 mm gerundet. */
export function scaleDimensions<T extends { width: number; depth: number; height: number }>(
  dims: T,
  to: ModelScale,
  from: ModelScale = BUILTIN_BASE_SCALE,
): T {
  const f = getScaleFactor(to, from);
  const r = (v: number) => Math.round(v * f * 10) / 10;
  return { ...dims, width: r(dims.width), depth: r(dims.depth), height: r(dims.height) };
}
