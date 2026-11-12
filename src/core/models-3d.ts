import type { Model3D, ModelCategory, Transform } from "@/core/types";
import { deg2rad } from "@/core/geometry";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";

export interface BuiltinModelDef {
  id: string;
  name: string;
  category: ModelCategory;
  icon: string;
  description: string;
  width: number;  // mm (Footprint X)
  depth: number;  // mm (Footprint Y)
  height: number; // mm (Vertical Z)
  badgeBg: string;
  badgeColor: string;
}

export const BUILTIN_MODELS: BuiltinModelDef[] = [
  {
    id: "station",
    name: "Bahnhof Neustadt",
    category: "buildings",
    icon: "🏢",
    description: "Classic country passenger station with gabled tile roof, clock tower, and platform canopy.",
    width: 320,
    depth: 140,
    height: 120,
    badgeBg: "#b45309",
    badgeColor: "#ffffff",
  },
  {
    id: "signal-box",
    name: "Stellwerk W1",
    category: "railway",
    icon: "🏭",
    description: "Two-storey brick signal tower with panoramic upper control room windows and stairway.",
    width: 120,
    depth: 75,
    height: 110,
    badgeBg: "#0369a1",
    badgeColor: "#ffffff",
  },
  {
    id: "engine-shed",
    name: "Lokschuppen",
    category: "railway",
    icon: "🚂",
    description: "Single-stall red brick locomotive shed with arched doorway and exhaust vent.",
    width: 240,
    depth: 72,
    height: 105,
    badgeBg: "#475569",
    badgeColor: "#ffffff",
  },
  {
    id: "town-house",
    name: "Fachwerkhaus",
    category: "buildings",
    icon: "🏠",
    description: "Traditional half-timbered residential house with steep pitched roof and chimney.",
    width: 110,
    depth: 90,
    height: 125,
    badgeBg: "#b45309",
    badgeColor: "#ffffff",
  },
  {
    id: "water-tower",
    name: "Wasserturm",
    category: "railway",
    icon: "💧",
    description: "Round stone tower supporting an elevated spherical water reservoir and pivoting crane.",
    width: 85,
    depth: 85,
    height: 170,
    badgeBg: "#0284c7",
    badgeColor: "#ffffff",
  },
  {
    id: "light-signal",
    name: "Lichtsignal (Blocksignal)",
    category: "railway",
    icon: "🚦",
    description: "Standard railway color-light block signal with steel mast and visor optics.",
    width: 25,
    depth: 25,
    height: 85,
    badgeBg: "#15803d",
    badgeColor: "#ffffff",
  },
  {
    id: "buffer-stop",
    name: "Prellbock",
    category: "railway",
    icon: "🛑",
    description: "End of track buffer stop with steel rail A-frame and wooden bumper beam.",
    width: 25,
    depth: 35,
    height: 25,
    badgeBg: "#dc2626",
    badgeColor: "#ffffff",
  },
  {
    id: "pine-tree",
    name: "Fichte / Tanne",
    category: "nature",
    icon: "🌲",
    description: "Layered evergreen coniferous pine tree with natural foliage shading.",
    width: 55,
    depth: 55,
    height: 135,
    badgeBg: "#166534",
    badgeColor: "#ffffff",
  },
  {
    id: "deciduous-tree",
    name: "Laubbaum / Eiche",
    category: "nature",
    icon: "🌳",
    description: "Broadleaf deciduous oak tree with organic branching canopy.",
    width: 80,
    depth: 80,
    height: 120,
    badgeBg: "#15803d",
    badgeColor: "#ffffff",
  },
  {
    id: "platform-lamp",
    name: "Bahnsteigleuchte",
    category: "railway",
    icon: "💡",
    description: "Curved swan-neck platform lamppost with lantern casing.",
    width: 20,
    depth: 20,
    height: 70,
    badgeBg: "#eab308",
    badgeColor: "#000000",
  },
  {
    id: "turntable-trainhall",
    name: "Ringlokschuppen (Drehscheibe)",
    category: "railway",
    icon: "🏛️",
    description: "3-ständiger Ringlokschuppen im 12°-Raster für Drehscheiben, mit Rauchabzügen und Loktoren.",
    width: 320,
    depth: 310,
    height: 115,
    badgeBg: "#0f766e",
    badgeColor: "#ffffff",
  },
  {
    id: "skyscraper",
    name: "Wolkenkratzer / Hochhaus",
    category: "buildings",
    icon: "🏙️",
    description: "Modernes Hochhaus mit Glasfassade, gestufter Architektur, Antennenmast und Dachterrasse.",
    width: 130,
    depth: 110,
    height: 280,
    badgeBg: "#0284c7",
    badgeColor: "#ffffff",
  },
  {
    id: "industry-building",
    name: "Industriehalle / Fabrik",
    category: "buildings",
    icon: "🏭",
    description: "Große Industrie- und Fertigungshalle mit Sheddach (Sägezahndach), Laderampe, Stahlsilo und Werktor.",
    width: 260,
    depth: 160,
    height: 110,
    badgeBg: "#475569",
    badgeColor: "#ffffff",
  },
  {
    id: "mountain",
    name: "Berg / Gebirgsmassiv",
    category: "nature",
    icon: "⛰️",
    description: "Massiver Gebirgszug mit Felsformationen, Schneegipfel und automatischen Tunnelausschnitten für kreuzende Gleise und Straßen.",
    width: 600,
    depth: 450,
    height: 220,
    badgeBg: "#44403c",
    badgeColor: "#ffffff",
  },
];

export function findBuiltinModelDef(id: string): BuiltinModelDef | undefined {
  return BUILTIN_MODELS.find((m) => m.id === id);
}

export function createBuiltinModel(
  defId: string,
  transform: Transform = { x: 0, y: 0, rotation: 0 },
  layerId?: string,
): Model3D {
  const def = findBuiltinModelDef(defId) || BUILTIN_MODELS[0]!;
  return {
    id: `model-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    name: def.name,
    category: def.category,
    transform: { ...transform },
    elevation: 0,
    scale: 1.0,
    layerId,
    width: def.width,
    depth: def.depth,
    height: def.height,
    modelType: "builtin",
    builtinId: def.id,
  };
}

/** Hit-test: prüft ob ein 2D-Weltpunkt innerhalb des 2D-Footprints des 3D-Modells liegt */
export function isPointInsideModel(pt: { x: number; y: number }, model: Model3D): boolean {
  const dx = pt.x - model.transform.x;
  const dy = pt.y - model.transform.y;
  const rad = -deg2rad(model.transform.rotation);

  // In lokale Model-Koordinaten drehen
  const lx = dx * Math.cos(rad) - dy * Math.sin(rad);
  const ly = dx * Math.sin(rad) + dy * Math.cos(rad);

  const halfW = (model.width * (model.scale ?? 1.0)) / 2;
  const halfD = (model.depth * (model.scale ?? 1.0)) / 2;

  return Math.abs(lx) <= halfW && Math.abs(ly) <= halfD;
}

/** Berechnet die Weltkoordinaten des Drehgriffs für ein 3D-Modell in 2D */
export function getModelRotationHandleWorld(model: Model3D): { x: number; y: number } {
  const rad = deg2rad(model.transform.rotation);
  const handleDistance = (model.depth * (model.scale ?? 1.0)) / 2 + 18;

  // Griff befindet sich oben (-Y in lokalen Koordinaten)
  return {
    x: model.transform.x + handleDistance * Math.sin(rad),
    y: model.transform.y - handleDistance * Math.cos(rad),
  };
}

/** Liest eine vom Benutzer ausgewählte .glb oder .gltf Datei ein und berechnet die Abmessungen */
export async function parseGlbOrGltfFile(file: File): Promise<Model3D> {
  const isGlb = file.name.toLowerCase().endsWith(".glb");
  const isGltf = file.name.toLowerCase().endsWith(".gltf");

  if (!isGlb && !isGltf) {
    throw new Error("Unsupported format. Please select a .glb or .gltf 3D model.");
  }

  const loader = new GLTFLoader();

  if (isGlb) {
    const arrayBuffer = await file.arrayBuffer();
    const base64 = await fileToBase64(file);

    return new Promise((resolve, reject) => {
      loader.parse(
        arrayBuffer,
        "",
        (gltf) => {
          const box = new THREE.Box3().setFromObject(gltf.scene);
          const size = box.getSize(new THREE.Vector3());

          // Heuristik für Maßeinheit:
          // Falls die Box kleiner als 10 Einheiten ist, ist das Modell vermutlich in Metern gebaut -> auf H0 mm skalieren (1:87)
          let autoScale = 1.0;
          if (size.x < 15 && size.z < 15) {
            autoScale = 1000 / 87; // Realweltmeter zu H0 mm
          }

          const width = Math.max(15, Math.round((size.x || 1) * autoScale));
          const depth = Math.max(15, Math.round((size.z || 1) * autoScale));
          const height = Math.max(10, Math.round((size.y || 1) * autoScale));

          resolve({
            id: `model-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
            name: file.name.replace(/\.(glb|gltf)$/i, ""),
            category: "custom",
            transform: { x: 0, y: 0, rotation: 0 },
            elevation: 0,
            scale: autoScale !== 1.0 ? autoScale : 1.0,
            width,
            depth,
            height,
            modelType: "glb",
            fileData: base64,
            fileName: file.name,
          });
        },
        (error) => {
          reject(new Error(`Failed to parse .glb model: ${error.message || error}`));
        },
      );
    });
  } else {
    // .gltf (JSON)
    const text = await file.text();
    const base64 = "data:model/gltf+json;base64," + btoa(unescape(encodeURIComponent(text)));

    return new Promise((resolve, reject) => {
      loader.parse(
        text,
        "",
        (gltf) => {
          const box = new THREE.Box3().setFromObject(gltf.scene);
          const size = box.getSize(new THREE.Vector3());

          let autoScale = 1.0;
          if (size.x < 15 && size.z < 15) {
            autoScale = 1000 / 87;
          }

          const width = Math.max(15, Math.round((size.x || 1) * autoScale));
          const depth = Math.max(15, Math.round((size.z || 1) * autoScale));
          const height = Math.max(10, Math.round((size.y || 1) * autoScale));

          resolve({
            id: `model-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
            name: file.name.replace(/\.(glb|gltf)$/i, ""),
            category: "custom",
            transform: { x: 0, y: 0, rotation: 0 },
            elevation: 0,
            scale: autoScale !== 1.0 ? autoScale : 1.0,
            width,
            depth,
            height,
            modelType: "gltf",
            fileData: base64,
            fileName: file.name,
          });
        },
        (error) => {
          reject(new Error(`Failed to parse .gltf model: ${error.message || error}`));
        },
      );
    });
  }
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

