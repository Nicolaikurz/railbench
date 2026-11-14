import type { EditorContext } from "./track-operations";
import type { SignalSide, SignalDirection, Transform } from "@/core/types";
import { computeWorldConnectors } from "@/catalog/catalog";
import {
  ToggleFeederCommand,
  ToggleIsolatorCommand,
  ToggleCatenaryCommand,
  ToggleCatenaryFeederCommand,
  ToggleCatenaryIsolatorCommand,
  ToggleTurnoutLanternCommand,
  ToggleTurnoutDecoderCommand,
  ToggleSignalCommand,
  SetSignalConfigCommand,
  ToggleBridgeCommand,
  MovePiecesToLayerCommand,
  ApplyRampElevationsCommand,
  MovePieceCommand,
  MoveModelCommand,
} from "@/editor/commands";
import { normAngle } from "@/core/geometry";
import { calculateAutoRamp } from "@/core/ramp";
import { getLanguage } from "@/i18n";

export function toggleSelectedFeeder(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const layout = ctx.getLayout();
  const selected = selection.all();
  if (selected.length === 0) return;

  if (selected.length === 1) {
    history.run(new ToggleFeederCommand(selected[0]!));
  } else {
    const selectedPieces = layout.pieces.filter((p) => selected.includes(p.id));
    const allHave = selectedPieces.length > 0 && selectedPieces.every((p) => !p.powerFeed);
    for (const id of selected) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece) {
        if (allHave && piece.powerFeed) history.run(new ToggleFeederCommand(id));
        else if (!allHave && !piece.powerFeed) history.run(new ToggleFeederCommand(id));
      }
    }
  }
  ctx.sync(true);
}

export function toggleSelectedIsolator(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const layout = ctx.getLayout();
  const selected = selection.all();
  if (selected.length === 0) return;

  if (selected.length === 1) {
    history.run(new ToggleIsolatorCommand(selected[0]!));
  } else {
    const selectedPieces = layout.pieces.filter((p) => selected.includes(p.id));
    const allHave = selectedPieces.length > 0 && selectedPieces.every((p) => !p.isIsolator);
    for (const id of selected) {
      const piece = layout.pieces.find((p) => p.id === id);
      if (piece) {
        if (allHave && piece.isIsolator) history.run(new ToggleIsolatorCommand(id));
        else if (!allHave && !piece.isIsolator) history.run(new ToggleIsolatorCommand(id));
      }
    }
  }
  ctx.sync(true);
}

export function toggleSelectedCatenary(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length > 0) {
    history.run(new ToggleCatenaryCommand(selected));
    ctx.sync(true);
  }
}

export function toggleSelectedCatenaryFeeder(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length > 0) {
    history.run(new ToggleCatenaryFeederCommand(selected));
    ctx.sync(true);
  }
}

export function toggleSelectedCatenaryIsolator(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length > 0) {
    history.run(new ToggleCatenaryIsolatorCommand(selected));
    ctx.sync(true);
  }
}

export function toggleSelectedTurnoutLantern(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length > 0) {
    history.run(new ToggleTurnoutLanternCommand(selected));
    ctx.sync(true);
  }
}

export function toggleSelectedTurnoutDecoder(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length > 0) {
    history.run(new ToggleTurnoutDecoderCommand(selected));
    ctx.sync(true);
  }
}

export function toggleSelectedSignal(ctx: EditorContext, targetState?: boolean): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length > 0) {
    history.run(new ToggleSignalCommand(selected, targetState));
    ctx.sync(true);
  }
}

export function cycleSelectedSignalSide(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const layout = ctx.getLayout();
  const selected = selection.all();
  if (selected.length === 0) return;

  const pieces = layout.pieces.filter((p) => selected.includes(p.id));
  const first = pieces[0];
  const currentSide: SignalSide = first?.signalSide ?? "right";
  const nextSide: SignalSide = currentSide === "right" ? "left" : currentSide === "left" ? "both" : "right";

  history.run(new SetSignalConfigCommand(selected, { signalSide: nextSide, hasSignal: true }));
  ctx.sync(true);
}

export function cycleSelectedSignalDirection(ctx: EditorContext): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const layout = ctx.getLayout();
  const selected = selection.all();
  if (selected.length === 0) return;

  const pieces = layout.pieces.filter((p) => selected.includes(p.id));
  const first = pieces[0];
  const currentDir: SignalDirection = first?.signalDirection ?? "forward";
  const nextDir: SignalDirection = currentDir === "forward" ? "backward" : currentDir === "backward" ? "both" : "forward";

  history.run(new SetSignalConfigCommand(selected, { signalDirection: nextDir, hasSignal: true }));
  ctx.sync(true);
}

export function setSignalConfig(
  ctx: EditorContext,
  pieceIds: string[],
  config: { hasSignal?: boolean; signalSide?: SignalSide; signalDirection?: SignalDirection },
): void {
  const history = ctx.getHistory();
  history.run(new SetSignalConfigCommand(pieceIds, config));
  ctx.sync(true);
}

export function toggleSelectedBridge(ctx: EditorContext, targetState?: boolean): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length > 0) {
    history.run(new ToggleBridgeCommand(selected, targetState));
    ctx.sync(true);
  }
}

export function moveSelectedToLayer(
  ctx: EditorContext,
  layerId: string,
  elevation?: { start: number; end: number } | Map<string, { start: number; end: number }>,
): void {
  const selection = ctx.getSelection();
  const history = ctx.getHistory();
  const selected = selection.all();
  if (selected.length === 0) return;
  history.run(new MovePiecesToLayerCommand(selected, layerId, elevation));
  ctx.sync(true);
}

export function applyHeightInput(ctx: EditorContext, target: number): void {
  if (isNaN(target)) return;
  const layout = ctx.getLayout();
  const history = ctx.getHistory();
  const selection = ctx.getSelection();
  const layers = layout.layers || [];

  const byLayer = new Map<string, string[]>();
  for (const id of selection.all()) {
    const p = layout.pieces.find((x) => x.id === id);
    if (!p || !ctx.isPieceInteractive(p)) continue;
    const lid = p.layerId || "layer-1";
    const base = layers.find((l) => l.id === lid)?.heightMm ?? 0;
    const elev = target - base;
    if (Math.abs((p.elevation?.start ?? 0) - elev) < 0.05 && Math.abs((p.elevation?.end ?? 0) - elev) < 0.05) continue;
    if (!byLayer.has(lid)) byLayer.set(lid, []);
    byLayer.get(lid)!.push(id);
  }
  if (byLayer.size === 0) return;

  for (const [lid, ids] of byLayer) {
    const base = layers.find((l) => l.id === lid)?.heightMm ?? 0;
    const elev = target - base;
    history.run(new MovePiecesToLayerCommand(ids, lid, { start: elev, end: elev }));
  }
  ctx.sync(true);
}

export function applyAutoRamp(ctx: EditorContext, onFallbackToDialog?: () => void): boolean {
  const layout = ctx.getLayout();
  const selection = ctx.getSelection();
  const selectedIds = selection.all();
  const isDe = getLanguage() === "de";

  if (selectedIds.length === 0) {
    ctx.showToast(isDe ? "Bitte zuerst Gleise für die Rampe auswählen" : "Please select tracks for the ramp first", "⚠️");
    return false;
  }

  const selectedPieces = layout.pieces.filter((p) => selectedIds.includes(p.id));
  if (selectedPieces.length === 0) {
    ctx.showToast(isDe ? "Keine Gleise ausgewählt" : "No tracks selected", "⚠️");
    return false;
  }

  const result = calculateAutoRamp({
    pieces: selectedPieces,
    allPieces: layout.pieces,
    layers: layout.layers || [],
    catalog: ctx.getCatalog(),
    profile: "linear",
  });

  if (!result.success) {
    if (result.reason === "not_continuous") {
      ctx.showToast(
        isDe
          ? "Gleise müssen eine zusammenhängende Strecke ohne Verzweigungen bilden"
          : "Selected tracks must form a continuous chain without branches",
        "⚠️",
      );
    } else if (result.reason === "same_height") {
      ctx.showToast(
        isDe
          ? `Start und Ende haben dieselbe Höhe (${result.startHeightMm ?? 0} mm). Bitte Höhen-Dialog (⤉) nutzen.`
          : `Start and end tracks have the same height (${result.startHeightMm ?? 0} mm). Please use Height Dialog (⤉).`,
        "⚠️",
      );
      if (onFallbackToDialog) {
        onFallbackToDialog();
      }
    } else {
      ctx.showToast(isDe ? "Rampe konnte nicht berechnet werden" : "Could not calculate ramp", "⚠️");
    }
    return false;
  }

  if (result.pieceElevations && result.pieceElevations.size > 0) {
    ctx.getHistory().run(new ApplyRampElevationsCommand(result.pieceElevations));
    ctx.sync(true);

    const sH = result.startHeightMm ?? 0;
    const eH = result.endHeightMm ?? 0;
    const lenCm = Math.round((result.totalLengthMm ?? 0) / 10);
    const grad = result.gradientPercent ?? 0;
    const warnIcon = grad > 4.5 ? " ⚠️" : "";

    const msg = isDe
      ? `Rampe berechnet: ${sH} mm ➔ ${eH} mm (${lenCm} cm, ${grad.toFixed(1)}% Steigung${warnIcon})`
      : `Ramp applied: ${sH} mm ➔ ${eH} mm (${lenCm} cm, slope ${grad.toFixed(1)}%${warnIcon})`;

    ctx.showToast(msg, "📐");
    return true;
  }
  return false;
}

export function applyAngleInput(ctx: EditorContext, rawVal: number): void {
  if (isNaN(rawVal)) return;
  const layout = ctx.getLayout();
  const history = ctx.getHistory();
  const selection = ctx.getSelection();
  const targetAngle = normAngle(rawVal);
  const selectedIds = selection.all();
  if (selectedIds.length === 0) return;

  const firstPiece = layout.pieces.find((p) => p.id === selectedIds[0]);
  const firstModel = layout.models?.find((m) => m.id === selectedIds[0]);
  const currentBaseAngle = firstPiece ? firstPiece.transform.rotation : (firstModel ? firstModel.transform.rotation : 0);
  const delta = targetAngle - currentBaseAngle;

  for (const id of selectedIds) {
    const piece = layout.pieces.find((p) => p.id === id);
    if (piece && ctx.isPieceInteractive(piece)) {
      const oldT = { ...piece.transform };
      const newRot = selectedIds.length === 1 ? targetAngle : normAngle(piece.transform.rotation + delta);
      const newT: Transform = { ...piece.transform, rotation: newRot };
      const entry = ctx.getCatalogEntry(piece);
      if (entry) {
        piece.transform = newT;
        piece.connectors = computeWorldConnectors(entry, newT);
        history.run(new MovePieceCommand(piece.id, oldT, newT));
      }
    }
    const model = layout.models?.find((m) => m.id === id);
    if (model) {
      const oldT = { ...model.transform };
      const newRot = selectedIds.length === 1 ? targetAngle : normAngle(model.transform.rotation + delta);
      const newT: Transform = { ...oldT, rotation: newRot };
      model.transform = newT;
      history.run(new MoveModelCommand(model.id, oldT, newT));
    }
  }
  ctx.sync(true);
}
