import type { Model3D, TrackPiece } from "@/core/types";
import type { CatalogEntry } from "@/catalog/catalog";
import { parseGlbOrGltfFile } from "@/core/models-3d";
import type { Renderer } from "@/render/renderer";
import type { TrackOperations } from "./track-operations";

export async function handleCanvasDrop(
  e: DragEvent,
  renderer: Renderer,
  operations: TrackOperations,
  sync: (dirty?: boolean) => void,
): Promise<void> {
  e.preventDefault();
  const rawTrack = e.dataTransfer?.getData("application/json");
  const rawModel = e.dataTransfer?.getData("application/railbench-model");
  const worldPos = renderer.screenToWorld(e.clientX, e.clientY);

  if (rawModel) {
    try {
      const model = JSON.parse(rawModel) as Model3D;
      operations.addModelToLayout(model, worldPos);
      return;
    } catch (err) {
      console.error("Drop Model Fehler:", err);
    }
  }

  if (rawTrack) {
    try {
      const entry = JSON.parse(rawTrack) as CatalogEntry;
      operations.addPieceToLayout(entry, worldPos);
      sync(true);
      return;
    } catch (err) {
      console.error("Drop Track Fehler:", err);
    }
  }

  if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0]) {
    const file = e.dataTransfer.files[0];
    if (file.name.toLowerCase().endsWith(".glb") || file.name.toLowerCase().endsWith(".gltf")) {
      try {
        const model = await parseGlbOrGltfFile(file);
        operations.addModelToLayout(model, worldPos);
      } catch (err: any) {
        alert(`Error loading dropped 3D model: ${err.message || err}`);
      }
    }
  }
}
