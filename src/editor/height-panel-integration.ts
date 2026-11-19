import type { Layout, TrackPiece } from "@/core/types";
import type { Catalog } from "@/catalog/catalog";
import { Selection } from "@/editor/selection";
import { History, AddPiecesCommand } from "@/editor/commands";
import { TrackOperations } from "@/editor/track-operations";
import { openHeightPanel } from "@/ui/height-panel";
import { getLanguage } from "@/i18n";
import { showToast } from "@/ui/toast";
import { ICONS } from "@/ui/icons";

export interface HeightPanelDeps {
  getLayout: () => Layout;
  getCatalog: () => Catalog;
  getSelection: () => Selection;
  getHistory: () => History;
  getOperations: () => TrackOperations;
  sync: (dirty?: boolean) => void;
  setPreviewRampElevations: (map: Map<string, { start: number; end: number }> | null) => void;
  setPreviewGeneratedPieces: (pieces: TrackPiece[] | null) => void;
  setActivePointMarkers: (markers: any) => void;
  setActivePickHandler: (handler: any) => void;
  setActivePinDropHandler: (handler: any) => void;
}

export function openHeightPanelUnified(
  deps: HeightPanelDeps,
  initialTab: "flat" | "ramp" | "generate",
  anchorPiece?: TrackPiece,
  preselectedLayerId?: string,
) {
  const layout = deps.getLayout();
  const catalog = deps.getCatalog();
  const selection = deps.getSelection();
  const history = deps.getHistory();
  const operations = deps.getOperations();

  const selected = selection.all();
  const effectivePieces = selected.length > 0
    ? layout.pieces.filter((p) => selected.includes(p.id))
    : (anchorPiece ? [anchorPiece] : []);

  openHeightPanel({
    layout,
    catalog,
    initialTab,
    selectedPieceIds: effectivePieces.map((p) => p.id),
    anchorPiece: anchorPiece || effectivePieces[0],
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
      operations.moveSelectedToLayer(targetLayerId, elevation);
      deps.setPreviewRampElevations(null);
      deps.setPreviewGeneratedPieces(null);
      deps.setActivePointMarkers(null);
      deps.sync(true);
    },
    onApplyRamp: (targetLayerId, pieceElevations) => {
      operations.moveSelectedToLayer(targetLayerId, pieceElevations);
      deps.setPreviewRampElevations(null);
      deps.setPreviewGeneratedPieces(null);
      deps.setActivePointMarkers(null);
      deps.sync(true);
    },
    onApplyGenerate: (generatedPieces, newLayer) => {
      if (newLayer) {
        if (!layout.layers) layout.layers = [];
        if (!layout.layers.some((l) => l.id === newLayer.id)) {
          layout.layers.push(newLayer);
        }
        layout.activeLayerId = newLayer.id;
      }
      history.run(new AddPiecesCommand(generatedPieces));
      selection.setMany(generatedPieces.map((p) => p.id));
      deps.setPreviewRampElevations(null);
      deps.setPreviewGeneratedPieces(null);
      deps.setActivePointMarkers(null);
      deps.sync(true);
      const isDe = getLanguage() === "de";
      showToast(
        isDe ? `${generatedPieces.length} Rampengleise erstellt` : `Created ${generatedPieces.length} ramp tracks`,
        ICONS.slope,
      );
    },
    onPreviewRamp: (elevations) => {
      deps.setPreviewRampElevations(elevations);
      deps.sync(false);
    },
    onPreviewGenerate: (pieces) => {
      deps.setPreviewGeneratedPieces(pieces);
      deps.sync(false);
    },
    setPointMarkers: (markers) => {
      deps.setActivePointMarkers(markers);
      deps.sync(false);
    },
    setPickHandler: (handler) => {
      deps.setActivePickHandler(handler);
    },
    setPinDropHandler: (handler) => {
      deps.setActivePinDropHandler(handler);
    },
    onClose: () => {
      deps.setPreviewRampElevations(null);
      deps.setPreviewGeneratedPieces(null);
      deps.setActivePointMarkers(null);
      deps.setActivePickHandler(null);
      deps.setActivePinDropHandler(null);
      deps.sync(false);
    },
  });
}

export function openHeightDialogForSelection(
  deps: HeightPanelDeps,
  preselectedLayerId?: string,
) {
  const selected = deps.getSelection().all();
  const tab: "flat" | "ramp" = selected.length > 1 ? "ramp" : "flat";
  openHeightPanelUnified(deps, tab, undefined, preselectedLayerId);
}

export function openRampGeneratorForPiece(
  deps: HeightPanelDeps,
  anchorPiece: TrackPiece,
) {
  openHeightPanelUnified(deps, "generate", anchorPiece);
}
