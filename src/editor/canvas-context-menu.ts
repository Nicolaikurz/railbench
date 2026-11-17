import { TrackPiece } from "@/core/types";
import { showTrackContextMenu } from "@/ui/track-context-menu";
import type { CanvasControllerDeps } from "./canvas-controller-types";

export function openTrackContextMenu(
  deps: CanvasControllerDeps,
  piece: TrackPiece,
  screenX: number,
  screenY: number,
): void {
  const layout = deps.getLayout();
  const entry = deps.getCatalogEntryForPiece(piece);
  const ops = deps.getOperations();

  showTrackContextMenu({
    piece,
    screenX,
    screenY,
    catalogEntry: entry,
    layout,
    onOpenRampGenerator: () => {
      if (deps.openRampGeneratorForPiece) {
        deps.openRampGeneratorForPiece(piece);
      }
    },
    onAutoRamp: () => {
      if (deps.openRampGeneratorForPiece) {
        deps.openRampGeneratorForPiece(piece);
      } else {
        deps.openHeightDialogForSelection();
      }
    },
    onOpenHeightDialog: () => {
      deps.openHeightDialogForSelection();
    },
    onRotateCw: () => {
      ops.rotateSelected(7.5);
    },
    onRotateCcw: () => {
      ops.rotateSelected(-7.5);
    },
    onMirrorFlip: () => {
      ops.flipSelected();
    },
    onCopy: () => {
      ops.copySelected();
    },
    onPaste: () => {
      ops.pasteClipboard();
    },
    onDuplicate: () => {
      ops.duplicateSelected();
    },
    onDelete: () => {
      ops.deleteSelected();
    },
    onSelectConnected: () => {
      ops.selectConnectedTracks(piece.id);
    },
    onSelectLayer: () => {
      ops.selectLayerTracks(piece.layerId);
    },
    onToggleGroup: () => {
      ops.toggleGroupSelected();
    },
    onSplitTrack: () => {
      ops.splitTrack(piece.id);
    },
    onReplaceTrack: (newCatalogId: string) => {
      ops.replaceTrack(piece.id, newCatalogId);
    },
    onToggleTurnoutDecoder: () => {
      ops.toggleSelectedTurnoutDecoder();
    },
    onToggleTurnoutLantern: () => {
      ops.toggleSelectedTurnoutLantern();
    },
    onToggleSignal: () => {
      ops.toggleSelectedSignal();
    },
    onCycleSignalSide: () => {
      ops.cycleSelectedSignalSide();
    },
    onCycleSignalDirection: () => {
      ops.cycleSelectedSignalDirection();
    },
    onToggleBridge: () => {
      ops.toggleSelectedBridge();
    },
    onToggleTunnel: () => {
      ops.toggleSelectedTunnel();
    },
    onResetFlex: () => {
      ops.resetFlexTrack(piece.id);
    },
    onToggleFeeder: () => {
      ops.toggleSelectedFeeder();
    },
    onToggleIsolator: () => {
      ops.toggleSelectedIsolator();
    },
    onToggleCatenary: () => {
      ops.toggleSelectedCatenary();
    },
    onRotate: () => {
      ops.rotateSelected(7.5);
    },
  });
}
