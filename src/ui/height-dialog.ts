import { openHeightPanel } from "./height-panel";
import type { Layout, Layer } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";

export interface ChangeHeightDialogOptions {
  layout: Layout;
  catalog: Catalog;
  selectedPieceIds: string[];
  preselectedLayerId?: string;
  onApply: (
    targetLayerId: string,
    elevation?: { start: number; end: number } | Map<string, { start: number; end: number }>,
  ) => void;
  onCreateLayer?: (name: string, heightMm: number) => Layer;
}

export function openChangeHeightDialog(options: ChangeHeightDialogOptions) {
  const { layout, catalog, selectedPieceIds, preselectedLayerId, onApply } = options;

  if (selectedPieceIds.length === 0) {
    alert("Please select one or more tracks first.");
    return;
  }

  const initialTab = selectedPieceIds.length > 1 ? "ramp" : "flat";

  return openHeightPanel({
    layout,
    catalog,
    initialTab,
    selectedPieceIds,
    preselectedLayerId,
    onApplyFlat: (targetLayerId, customElevationMm) => {
      const elevation = customElevationMm !== undefined
        ? (() => {
            const lay = layout.layers?.find((l) => l.id === targetLayerId);
            const baseH = lay?.heightMm ?? 0;
            const off = customElevationMm - baseH;
            return { start: off, end: off };
          })()
        : { start: 0, end: 0 };
      onApply(targetLayerId, elevation);
    },
    onApplyRamp: (targetLayerId, pieceElevations) => {
      onApply(targetLayerId, pieceElevations);
    },
  });
}
