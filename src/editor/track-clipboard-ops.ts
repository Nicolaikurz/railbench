import type { EditorContext } from "./track-operations";
import type { TrackPiece, Model3D } from "@/core/types";
import { computeWorldConnectors } from "@/catalog/catalog";
import { updatePieceConnections } from "@/core/snapping";
import { isFlexTrack } from "@/core/flextrack";
import { AddPieceCommand, AddPiecesCommand } from "@/editor/commands";
import { deg2rad } from "@/core/geometry";
import { getLanguage } from "@/i18n";

interface TrackClipboardData {
  pieces: TrackPiece[];
  models: Model3D[];
}

let trackClipboard: TrackClipboardData | null = null;
let pasteOffsetCount = 1;

export function copySelected(ctx: EditorContext): boolean {
  const layout = ctx.getLayout();
  const selection = ctx.getSelection();
  const selectedIds = selection.all();
  if (selectedIds.length === 0) return false;

  const piecesToCopy: TrackPiece[] = [];
  for (const id of selectedIds) {
    const p = layout.pieces.find((x) => x.id === id);
    if (p) {
      piecesToCopy.push(JSON.parse(JSON.stringify(p)));
    }
  }

  const modelsToCopy: Model3D[] = [];
  if (layout.models) {
    for (const id of selectedIds) {
      const m = layout.models.find((x) => x.id === id);
      if (m) {
        modelsToCopy.push(JSON.parse(JSON.stringify(m)));
      }
    }
  }

  if (piecesToCopy.length > 0 || modelsToCopy.length > 0) {
    trackClipboard = { pieces: piecesToCopy, models: modelsToCopy };
    pasteOffsetCount = 1;

    try {
      const payload = JSON.stringify(trackClipboard);
      localStorage.setItem("railbench_clipboard", payload);
      if (navigator.clipboard?.writeText) {
        navigator.clipboard.writeText(payload).catch(() => {});
      }
    } catch {}

    const total = piecesToCopy.length + modelsToCopy.length;
    const isDe = getLanguage() === "de";
    const itemLabel = isDe
      ? `${total} Element${total === 1 ? "" : "e"}`
      : `${total} item${total === 1 ? "" : "s"}`;
    ctx.showToast(isDe ? `${itemLabel} in Zwischenablage kopiert (Strg+C)` : `${itemLabel} copied to clipboard (Ctrl+C)`, "📋");
    return true;
  }
  return false;
}

export function pasteClipboard(
  ctx: EditorContext,
  customOffset?: { x: number; y: number },
  onSetLastPlacedCatalogId?: (id: string | null) => void,
): boolean {
  const history = ctx.getHistory();
  const selection = ctx.getSelection();

  if (!trackClipboard || (trackClipboard.pieces.length === 0 && trackClipboard.models.length === 0)) {
    try {
      const stored = localStorage.getItem("railbench_clipboard");
      if (stored) {
        trackClipboard = JSON.parse(stored);
      }
    } catch {}
  }

  if (!trackClipboard || (trackClipboard.pieces.length === 0 && trackClipboard.models.length === 0)) {
    const isDe = getLanguage() === "de";
    ctx.showToast(isDe ? "Zwischenablage ist leer" : "Clipboard is empty", "⚠️");
    return false;
  }

  const offset = customOffset ?? {
    x: 40 * pasteOffsetCount,
    y: 40 * pasteOffsetCount,
  };
  if (!customOffset) {
    pasteOffsetCount++;
  }

  const newPieces: TrackPiece[] = [];
  const newModels: Model3D[] = [];
  const newIds: string[] = [];
  const now = Date.now();

  for (let i = 0; i < trackClipboard.pieces.length; i++) {
    const orig = trackClipboard.pieces[i];
    if (!orig) continue;
    const newId = `p-${now}-${i}-${Math.random().toString(36).substring(2, 7)}`;
    const entry = ctx.getCatalogEntry(orig);
    const newTransform = {
      x: orig.transform.x + offset.x,
      y: orig.transform.y + offset.y,
      rotation: orig.transform.rotation,
    };

    const copyPiece: TrackPiece = {
      ...JSON.parse(JSON.stringify(orig)),
      id: newId,
      transform: newTransform,
      connectors: entry ? computeWorldConnectors(entry, newTransform) : orig.connectors,
    };
    newPieces.push(copyPiece);
    newIds.push(newId);
  }

  for (let i = 0; i < trackClipboard.models.length; i++) {
    const orig = trackClipboard.models[i];
    if (!orig) continue;
    const newId = `m-${now}-${i}-${Math.random().toString(36).substring(2, 7)}`;
    const copyModel: Model3D = {
      ...JSON.parse(JSON.stringify(orig)),
      id: newId,
      transform: {
        ...orig.transform,
        x: orig.transform.x + offset.x,
        y: orig.transform.y + offset.y,
      },
    };
    newModels.push(copyModel);
    newIds.push(newId);
  }

  if (newPieces.length > 0 || newModels.length > 0) {
    if (newPieces.length > 0 && onSetLastPlacedCatalogId) {
      onSetLastPlacedCatalogId(newPieces[newPieces.length - 1]!.catalogId);
    }
    history.run(new AddPiecesCommand(newPieces, newModels));
    selection.clear();
    for (const id of newIds) {
      selection.add(id);
    }
    ctx.sync(true);

    const total = newPieces.length + newModels.length;
    const isDe = getLanguage() === "de";
    const itemLabel = isDe
      ? `${total} Element${total === 1 ? "" : "e"}`
      : `${total} item${total === 1 ? "" : "s"}`;
    ctx.showToast(isDe ? `${itemLabel} eingefügt (Strg+V)` : `${itemLabel} pasted (Ctrl+V)`, "📥");
    return true;
  }
  return false;
}

export function duplicateSelected(
  ctx: EditorContext,
  onSetLastPlacedCatalogId?: (id: string | null) => void,
): void {
  const selection = ctx.getSelection();
  const selectedIds = selection.all();
  if (selectedIds.length === 0) {
    const isDe = getLanguage() === "de";
    ctx.showToast(isDe ? "Keine Gleise zum Duplizieren markiert" : "No tracks selected to duplicate", "⚠️");
    return;
  }
  copySelected(ctx);
  pasteClipboard(ctx, { x: 40, y: 40 }, onSetLastPlacedCatalogId);
}

export function selectConnectedTracks(ctx: EditorContext, startPieceId?: string): void {
  const layout = ctx.getLayout();
  const selection = ctx.getSelection();
  const initialIds = startPieceId ? [startPieceId] : selection.all();
  if (initialIds.length === 0) return;

  const visited = new Set<string>();
  const queue = [...initialIds];

  while (queue.length > 0) {
    const curId = queue.shift()!;
    if (visited.has(curId)) continue;
    visited.add(curId);

    const piece = layout.pieces.find((p) => p.id === curId);
    if (!piece) continue;

    for (const conn of piece.connectors) {
      if (conn.connectedTo && !visited.has(conn.connectedTo.pieceId)) {
        queue.push(conn.connectedTo.pieceId);
      }
    }
  }

  selection.clear();
  for (const id of visited) {
    selection.add(id);
  }
  ctx.sync();
  const isDe = getLanguage() === "de";
  ctx.showToast(isDe ? `${visited.size} verbundene Gleise ausgewählt` : `Selected ${visited.size} connected tracks`, "🔗");
}

export function selectLayerTracks(ctx: EditorContext, layerId?: string): void {
  const layout = ctx.getLayout();
  const selection = ctx.getSelection();
  let targetLayerId = layerId;

  if (!targetLayerId) {
    const selectedIds = selection.all();
    const firstPiece = layout.pieces.find((p) => selectedIds.includes(p.id));
    targetLayerId = firstPiece?.layerId || layout.activeLayerId || layout.layers?.[0]?.id;
  }

  if (!targetLayerId) return;

  selection.clear();
  let count = 0;
  for (const p of layout.pieces) {
    if ((p.layerId || "layer-1") === targetLayerId && ctx.isPieceInteractive(p)) {
      selection.add(p.id);
      count++;
    }
  }
  ctx.sync();
  const layer = layout.layers?.find((l) => l.id === targetLayerId);
  const layerName = layer?.name || targetLayerId;
  const isDe = getLanguage() === "de";
  ctx.showToast(isDe ? `${count} Gleise auf Ebene "${layerName}" ausgewählt` : `Selected ${count} tracks on layer "${layerName}"`, "📑");
}

export function toggleGroupSelected(ctx: EditorContext): void {
  const layout = ctx.getLayout();
  const selection = ctx.getSelection();
  const selectedIds = selection.all();
  if (selectedIds.length < 2) {
    const isDe = getLanguage() === "de";
    ctx.showToast(isDe ? "Mindestens 2 Gleise zum Gruppieren markieren" : "Select at least 2 tracks to group", "⚠️");
    return;
  }

  const selectedPieces = layout.pieces.filter((p) => selectedIds.includes(p.id));
  const firstGroupId = selectedPieces[0]?.groupId;
  const allHaveSameGroup = firstGroupId && selectedPieces.every((p) => p.groupId === firstGroupId);

  const isDe = getLanguage() === "de";
  if (allHaveSameGroup) {
    for (const p of selectedPieces) {
      delete p.groupId;
    }
    ctx.showToast(isDe ? "Gruppierung aufgehoben" : "Tracks ungrouped", "🗂");
  } else {
    const newGroupId = `group-${Date.now()}`;
    for (const p of selectedPieces) {
      p.groupId = newGroupId;
    }
    ctx.showToast(isDe ? `${selectedPieces.length} Gleise gruppiert` : `Grouped ${selectedPieces.length} tracks`, "🗂");
  }
  ctx.sync(true);
}

export function splitTrack(
  ctx: EditorContext,
  pieceId: string,
  adjustFlexTrack: (pieceId: string, cutLength?: number) => void,
): boolean {
  const layout = ctx.getLayout();
  const history = ctx.getHistory();
  const piece = layout.pieces.find((p) => p.id === pieceId);
  if (!piece) return false;
  const entry = ctx.getCatalogEntry(piece);
  if (!entry) return false;

  const catalog = ctx.getCatalog();
  const isDe = getLanguage() === "de";

  if (isFlexTrack(entry, piece)) {
    const curLen = piece.customLength ?? entry.length ?? 900;
    const half = Math.round(curLen / 2);
    adjustFlexTrack(pieceId, half);
    ctx.showToast(isDe ? `Flexgleis bei ${half} mm geteilt` : `Flex track split at ${half} mm`, "✂");
    return true;
  }

  const curLen = entry.length ?? 180;
  const halfLen = curLen / 2;
  const halfCandidate = catalog.entries.find(
    (e) => e.kind === "straight" && Math.abs((e.length || 0) - halfLen) <= 6,
  );

  if (halfCandidate) {
    piece.catalogId = halfCandidate.id;
    piece.connectors = computeWorldConnectors(halfCandidate, piece.transform);

    const rad = deg2rad(piece.transform.rotation);
    const halfActual = halfCandidate.length ?? halfLen;
    const secondPos = {
      x: piece.transform.x + halfActual * Math.cos(rad),
      y: piece.transform.y + halfActual * Math.sin(rad),
      rotation: piece.transform.rotation,
    };
    const secondPiece: TrackPiece = {
      id: `p-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      catalogId: halfCandidate.id,
      transform: secondPos,
      connectors: computeWorldConnectors(halfCandidate, secondPos),
      layerId: piece.layerId,
      elevation: piece.elevation ? { ...piece.elevation } : undefined,
    };

    history.run(new AddPieceCommand(secondPiece));
    updatePieceConnections(layout.pieces, layout.layers);
    ctx.sync(true);
    ctx.showToast(
      isDe
        ? `Gleis geteilt in 2× #${halfCandidate.id} (${halfCandidate.length} mm)`
        : `Split track into 2× #${halfCandidate.id} (${halfCandidate.length} mm)`,
      "✂",
    );
    return true;
  } else {
    ctx.showToast(
      isDe ? "Kein passendes Teilgleis im Katalog gefunden" : "No matching half-track found in catalog",
      "⚠️",
    );
    return false;
  }
}

export function replaceTrack(ctx: EditorContext, pieceId: string, newCatalogId: string): boolean {
  const layout = ctx.getLayout();
  const piece = layout.pieces.find((p) => p.id === pieceId);
  if (!piece) return false;
  const newEntry = ctx.getCatalog().entries.find((e) => e.id === newCatalogId);
  if (!newEntry) return false;

  piece.catalogId = newCatalogId;
  piece.connectors = computeWorldConnectors(newEntry, piece.transform);
  updatePieceConnections(layout.pieces, layout.layers);
  ctx.sync(true);
  const isDe = getLanguage() === "de";
  ctx.showToast(isDe ? `Gleis ersetzt durch #${newCatalogId}` : `Replaced track with #${newCatalogId}`, "✂");
  return true;
}
