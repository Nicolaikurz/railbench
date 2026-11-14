import { openHeightPanel } from "./height-panel";
import type { Layout, Layer, TrackPiece } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";

export interface RampGeneratorDialogOptions {
  layout: Layout;
  catalog: Catalog;
  anchorPiece: TrackPiece;
  /** Optional: Index des Anschlusses am Ankergleis, der als Startpunkt vorgewählt wird. */
  anchorConnectorIndex?: number;
  onApply: (generatedPieces: TrackPiece[], newLayer?: Layer) => void;
}

export function openRampGeneratorDialog(options: RampGeneratorDialogOptions): void {
  const { layout, catalog, anchorPiece, anchorConnectorIndex, onApply } = options;

  openHeightPanel({
    layout,
    catalog,
    initialTab: "generate",
    anchorPiece,
    anchorConnectorIndex,
    onApplyGenerate: onApply,
  });
}
