/**
 * RAILbench 2D Drag-and-Drop Preview Generator
 *
 * Generates high-fidelity offscreen 2D Canvas previews for tracks, roads,
 * and 3D model footprints, and attaches them synchronously to DataTransfer.
 */

import type { CatalogEntry, TrackKind } from "@/catalog/types";
import type { BuiltinModelDef } from "@/core/models-3d";
import type { Model3D } from "@/core/types";

export interface DragPreviewOptions {
  isRoad?: boolean;
  roadWidthMm?: number;
  scaleGauge?: number;
  scaleFactor?: number;
  zoom?: number;
}

/**
 * Creates a crisp 2D canvas depicting the exact track or road piece 1:1 with the trackplan.
 */
export function createPieceDragCanvas(
  entry: CatalogEntry,
  isRoad: boolean = false,
  roadWidthMm: number = 60,
  scaleGauge: number = 16.5,
  zoom: number = 1.0,
  hideBadge: boolean = false,
): HTMLCanvasElement {
  if (typeof document === "undefined") {
    return {} as HTMLCanvasElement;
  }

  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) return canvas;

  const effectiveIsRoad = isRoad || !!entry.isRoad;
  // 1:1 scale with trackplan
  const pxPerMm = Math.max(0.2, Math.min(3.0, zoom || 1.0));

  if (effectiveIsRoad) {
    renderRoadPiece(ctx, canvas, entry, roadWidthMm, pxPerMm, hideBadge);
  } else {
    renderTrackPiece(ctx, canvas, entry, scaleGauge, pxPerMm, hideBadge);
  }

  return canvas;
}

const thumbnailCache = new Map<string, HTMLCanvasElement>();

/**
 * Creates a crisp, proportional 2D thumbnail preview of a catalog entry for sidebar cards.
 */
export function createCatalogThumbnail(
  entry: CatalogEntry,
  isRoad: boolean = false,
  roadWidthMm: number = 60,
  scaleGauge: number = 16.5,
  targetW: number = 60,
  targetH: number = 40,
): HTMLCanvasElement {
  if (typeof document === "undefined") {
    return {} as HTMLCanvasElement;
  }

  const cacheKey = `${entry.id}_${isRoad ? "r" : "t"}_${roadWidthMm}_${scaleGauge}_${targetW}_${targetH}`;
  const cached = thumbnailCache.get(cacheKey);
  if (cached) {
    const clone = document.createElement("canvas");
    clone.width = cached.width;
    clone.height = cached.height;
    clone.style.width = cached.style.width;
    clone.style.height = cached.style.height;
    const ctx = clone.getContext("2d");
    if (ctx) ctx.drawImage(cached, 0, 0);
    return clone;
  }

  const dpr = typeof window !== "undefined" ? Math.min(2, window.devicePixelRatio || 1) : 1;
  const thumbCanvas = document.createElement("canvas");
  thumbCanvas.width = targetW * dpr;
  thumbCanvas.height = targetH * dpr;
  thumbCanvas.style.width = `${targetW}px`;
  thumbCanvas.style.height = `${targetH}px`;

  const ctx = thumbCanvas.getContext("2d");
  if (!ctx) return thumbCanvas;

  const rawCanvas = createPieceDragCanvas(entry, isRoad, roadWidthMm, scaleGauge, 0.6, true);
  if (rawCanvas && rawCanvas.width && rawCanvas.height) {
    const pad = 3 * dpr;
    const availW = thumbCanvas.width - pad * 2;
    const availH = thumbCanvas.height - pad * 2;
    const scale = Math.min(availW / rawCanvas.width, availH / rawCanvas.height);
    const drawW = rawCanvas.width * scale;
    const drawH = rawCanvas.height * scale;
    const drawX = (thumbCanvas.width - drawW) / 2;
    const drawY = (thumbCanvas.height - drawH) / 2;

    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(rawCanvas, 0, 0, rawCanvas.width, rawCanvas.height, drawX, drawY, drawW, drawH);
  }

  thumbnailCache.set(cacheKey, thumbCanvas);

  const result = document.createElement("canvas");
  result.width = thumbCanvas.width;
  result.height = thumbCanvas.height;
  result.style.width = thumbCanvas.style.width;
  result.style.height = thumbCanvas.style.height;
  const rCtx = result.getContext("2d");
  if (rCtx) rCtx.drawImage(thumbCanvas, 0, 0);
  return result;
}

/**
 * Renders a track piece (straights, curves, turnouts, crossings, buffers, etc.) onto canvas.
 */
function renderTrackPiece(
  ctx: CanvasRenderingContext2D,
  canvas: HTMLCanvasElement,
  entry: CatalogEntry,
  gaugeMm: number,
  pxPerMm: number,
  hideBadge: boolean = false,
): void {
  const gauge = Math.max(6, gaugeMm * pxPerMm);
  const ballastWidth = gauge + 9;
  const ballastColor = "#475569";
  const sleeperColor = "#334155";
  const railColor = "#cbd5e1";
  const railShadowColor = "#1e293b";
  const accentColor = "#0284c7";

  const kind: TrackKind = entry.kind || "straight";

  if (kind === "curve" && entry.radius && entry.angle) {
    // Curved Track
    const r = entry.radius * pxPerMm;
    const rad = (Math.abs(entry.angle) * Math.PI) / 180;
    const pad = 16;
    const arcW = Math.max(40, r * Math.sin(rad));
    const arcH = Math.max(40, r * (1 - Math.cos(rad)));

    canvas.width = Math.ceil(arcW + pad * 2);
    canvas.height = Math.ceil(arcH + pad * 2);

    ctx.save();
    ctx.translate(pad, pad + r);

    // Ballast arc
    ctx.beginPath();
    ctx.arc(0, 0, r, -Math.PI / 2, -Math.PI / 2 + rad, false);
    ctx.lineWidth = ballastWidth;
    ctx.strokeStyle = ballastColor;
    ctx.lineCap = "round";
    ctx.stroke();

    // Sleepers (radial)
    const sleeperStep = 6;
    const totalLength = r * rad;
    const count = Math.max(3, Math.floor(totalLength / sleeperStep));
    ctx.strokeStyle = sleeperColor;
    ctx.lineWidth = 2;
    for (let i = 0; i <= count; i++) {
      const a = -Math.PI / 2 + (rad * i) / count;
      const cosA = Math.cos(a);
      const sinA = Math.sin(a);
      const rInner = r - gauge / 2 - 2;
      const rOuter = r + gauge / 2 + 2;
      ctx.beginPath();
      ctx.moveTo(rInner * cosA, rInner * sinA);
      ctx.lineTo(rOuter * cosA, rOuter * sinA);
      ctx.stroke();
    }

    // Dual concentric rails
    for (const offset of [-gauge / 2, gauge / 2]) {
      ctx.beginPath();
      ctx.arc(0, 0, r + offset, -Math.PI / 2, -Math.PI / 2 + rad, false);
      ctx.lineWidth = 2.5;
      ctx.strokeStyle = railShadowColor;
      ctx.stroke();

      ctx.beginPath();
      ctx.arc(0, 0, r + offset, -Math.PI / 2, -Math.PI / 2 + rad, false);
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = railColor;
      ctx.stroke();
    }

    // Connector indicators
    drawConnectorDot(ctx, 0, -r, accentColor);
    drawConnectorDot(ctx, r * Math.sin(rad), -r * Math.cos(rad), accentColor);

    ctx.restore();
    if (!hideBadge) drawPieceIdBadge(ctx, entry.id, canvas.width, canvas.height);
    return;
  }

  if (kind === "turnout-left" || kind === "turnout-right" || kind === "three-way") {
    // Turnout Track
    const len = (entry.length || 188) * pxPerMm;
    const branchAngleDeg = Math.abs(entry.branchAngle || entry.angle || 15);
    const branchRad = (branchAngleDeg * Math.PI) / 180;
    const isLeft = kind === "turnout-left" || kind === "three-way";
    const isRight = kind === "turnout-right" || kind === "three-way";

    const branchR = (entry.branchRadius || entry.radius || 437.5) * pxPerMm;
    const branchH = branchR * (1 - Math.cos(branchRad));
    const pad = 16;

    const extraH = kind === "three-way" ? branchH * 2 : branchH;
    canvas.width = Math.ceil(len + pad * 2);
    canvas.height = Math.ceil(ballastWidth + extraH + pad * 2);

    ctx.save();
    const mainY = kind === "three-way" ? canvas.height / 2 : isLeft ? canvas.height - pad - ballastWidth / 2 : pad + ballastWidth / 2;
    ctx.translate(pad, mainY);

    // Ballast main track
    drawStraightBallast(ctx, 0, 0, len, ballastWidth, ballastColor);

    // Diverging ballast branches
    if (isLeft) {
      drawTurnoutBranch(ctx, branchR, branchRad, true, ballastWidth, gauge, ballastColor, sleeperColor, railColor, railShadowColor);
    }
    if (isRight) {
      drawTurnoutBranch(ctx, branchR, branchRad, false, ballastWidth, gauge, ballastColor, sleeperColor, railColor, railShadowColor);
    }

    // Sleepers main track
    drawStraightSleepers(ctx, 0, 0, len, gauge, sleeperColor);

    // Rails main track
    drawStraightRails(ctx, 0, 0, len, gauge, railColor, railShadowColor);

    // Connectors
    drawConnectorDot(ctx, 0, 0, accentColor);
    drawConnectorDot(ctx, len, 0, accentColor);
    if (isLeft) {
      drawConnectorDot(ctx, branchR * Math.sin(branchRad), -branchR * (1 - Math.cos(branchRad)), accentColor);
    }
    if (isRight) {
      drawConnectorDot(ctx, branchR * Math.sin(branchRad), branchR * (1 - Math.cos(branchRad)), accentColor);
    }

    ctx.restore();
    if (!hideBadge) drawPieceIdBadge(ctx, entry.id, canvas.width, canvas.height);
    return;
  }

  if (kind === "curved-turnout-left" || kind === "curved-turnout-right") {
    // Curved Turnout
    const rMain = (entry.radius || 360) * pxPerMm;
    const rBranch = (entry.branchRadius || rMain * 0.8) * pxPerMm;
    const rad = (Math.abs(entry.angle || 30) * Math.PI) / 180;
    const pad = 16;
    const arcW = Math.max(50, rMain * Math.sin(rad));
    const arcH = Math.max(50, rMain * (1 - Math.cos(rad)) + 15);

    canvas.width = Math.ceil(arcW + pad * 2);
    canvas.height = Math.ceil(arcH + pad * 2);

    ctx.save();
    ctx.translate(pad, pad + rMain);

    // Both ballast arcs
    for (const r of [rMain, rBranch]) {
      ctx.beginPath();
      ctx.arc(0, 0, r, -Math.PI / 2, -Math.PI / 2 + rad, false);
      ctx.lineWidth = ballastWidth;
      ctx.strokeStyle = ballastColor;
      ctx.lineCap = "round";
      ctx.stroke();
    }

    // Rails for both arcs
    for (const r of [rMain, rBranch]) {
      for (const offset of [-gauge / 2, gauge / 2]) {
        ctx.beginPath();
        ctx.arc(0, 0, r + offset, -Math.PI / 2, -Math.PI / 2 + rad, false);
        ctx.lineWidth = 1.8;
        ctx.strokeStyle = railColor;
        ctx.stroke();
      }
    }

    drawConnectorDot(ctx, 0, -rMain, accentColor);
    drawConnectorDot(ctx, rMain * Math.sin(rad), -rMain * Math.cos(rad), accentColor);
    drawConnectorDot(ctx, rBranch * Math.sin(rad), -rBranch * Math.cos(rad), accentColor);

    ctx.restore();
    if (!hideBadge) drawPieceIdBadge(ctx, entry.id, canvas.width, canvas.height);
    return;
  }

  if (kind === "crossing" || kind === "double-slip") {
    // Crossing & Double Slip
    const len = (entry.length || 188) * pxPerMm;
    const crossAngleDeg = entry.angle || 15;
    const crossRad = (crossAngleDeg * Math.PI) / 180;
    const pad = 16;
    const crossH = Math.max(30, len * Math.sin(crossRad));

    canvas.width = Math.ceil(len + pad * 2);
    canvas.height = Math.ceil(crossH + pad * 2);

    ctx.save();
    const cx = canvas.width / 2;
    const cy = canvas.height / 2;
    ctx.translate(cx, cy);

    // Track 1 (Horizontal)
    drawStraightBallast(ctx, -len / 2, 0, len, ballastWidth, ballastColor);
    drawStraightSleepers(ctx, -len / 2, 0, len, gauge, sleeperColor);
    drawStraightRails(ctx, -len / 2, 0, len, gauge, railColor, railShadowColor);

    // Track 2 (Angled)
    ctx.save();
    ctx.rotate(crossRad);
    drawStraightBallast(ctx, -len / 2, 0, len, ballastWidth, ballastColor);
    drawStraightSleepers(ctx, -len / 2, 0, len, gauge, sleeperColor);
    drawStraightRails(ctx, -len / 2, 0, len, gauge, railColor, railShadowColor);
    ctx.restore();

    // DKW center slip curve hints
    if (kind === "double-slip") {
      ctx.beginPath();
      ctx.ellipse(0, 0, len * 0.25, crossH * 0.35, 0, 0, Math.PI * 2);
      ctx.strokeStyle = "rgba(56, 189, 248, 0.7)";
      ctx.lineWidth = 1.5;
      ctx.stroke();
    }

    ctx.restore();
    if (!hideBadge) drawPieceIdBadge(ctx, entry.id, canvas.width, canvas.height);
    return;
  }

  // Default: Straight piece (including uncoupler, buffer-stop, feeder, etc.)
  const len = (entry.length || 180) * pxPerMm;
  const pad = 14;
  canvas.width = Math.ceil(len + pad * 2);
  canvas.height = Math.ceil(ballastWidth + pad * 2);

  ctx.save();
  const cy = canvas.height / 2;
  ctx.translate(pad, cy);

  drawStraightBallast(ctx, 0, 0, len, ballastWidth, ballastColor);
  drawStraightSleepers(ctx, 0, 0, len, gauge, sleeperColor);
  drawStraightRails(ctx, 0, 0, len, gauge, railColor, railShadowColor);

  // Special accessories on straight tracks
  if (kind === "buffer-stop") {
    // Buffer beam (Prellbock)
    ctx.fillStyle = "#ef4444";
    ctx.fillRect(len - 4, -ballastWidth / 2, 5, ballastWidth);
    ctx.fillStyle = "#ffffff";
    ctx.beginPath();
    ctx.arc(len - 2, -gauge / 2, 2.5, 0, Math.PI * 2);
    ctx.arc(len - 2, gauge / 2, 2.5, 0, Math.PI * 2);
    ctx.fill();
  } else if (kind === "uncoupler") {
    // Uncoupler bar in center
    ctx.fillStyle = "#f59e0b";
    ctx.fillRect(len / 2 - 10, -gauge / 2 + 1, 20, gauge - 2);
  } else if (kind === "feeder") {
    // Feeder symbol
    drawConnectorDot(ctx, len / 2, 0, "#10b981", 4);
  } else if (kind === "isolating") {
    // Isolation gaps
    ctx.fillStyle = "#ef4444";
    ctx.fillRect(len * 0.4 - 1, -gauge / 2 - 3, 2, 6);
    ctx.fillRect(len * 0.6 - 1, gauge / 2 - 3, 2, 6);
  }

  drawConnectorDot(ctx, 0, 0, accentColor);
  if (kind !== "buffer-stop") {
    drawConnectorDot(ctx, len, 0, accentColor);
  }

  ctx.restore();
  if (!hideBadge) drawPieceIdBadge(ctx, entry.id, canvas.width, canvas.height);
}

/**
 * Renders a road piece onto canvas.
 */
function renderRoadPiece(
  ctx: CanvasRenderingContext2D,
  canvas: HTMLCanvasElement,
  entry: CatalogEntry,
  roadWidthMm: number,
  pxPerMm: number,
  hideBadge: boolean = false,
): void {
  const roadW = Math.max(20, (roadWidthMm || entry.roadWidth || 60) * pxPerMm);
  const roadColor = "#1e293b";
  const shoulderColor = "#64748b";
  const dashedColor = "#facc15";

  if (entry.radius && entry.angle) {
    const r = entry.radius * pxPerMm;
    const rad = (Math.abs(entry.angle) * Math.PI) / 180;
    const pad = 16;
    const arcW = Math.max(50, r * Math.sin(rad));
    const arcH = Math.max(50, r * (1 - Math.cos(rad)));

    canvas.width = Math.ceil(arcW + pad * 2);
    canvas.height = Math.ceil(arcH + pad * 2);

    ctx.save();
    ctx.translate(pad, pad + r);

    // Road asphalt surface
    ctx.beginPath();
    ctx.arc(0, 0, r, -Math.PI / 2, -Math.PI / 2 + rad, false);
    ctx.lineWidth = roadW;
    ctx.strokeStyle = roadColor;
    ctx.lineCap = "butt";
    ctx.stroke();

    // Road outer shoulders
    for (const offset of [-roadW / 2 + 1, roadW / 2 - 1]) {
      ctx.beginPath();
      ctx.arc(0, 0, r + offset, -Math.PI / 2, -Math.PI / 2 + rad, false);
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = shoulderColor;
      ctx.stroke();
    }

    // Center dashed line
    ctx.beginPath();
    ctx.arc(0, 0, r, -Math.PI / 2, -Math.PI / 2 + rad, false);
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = dashedColor;
    ctx.setLineDash([6, 4]);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.restore();
    if (!hideBadge) drawPieceIdBadge(ctx, entry.id, canvas.width, canvas.height);
    return;
  }

  // Straight road
  const len = (entry.length || 180) * pxPerMm;
  const pad = 12;
  canvas.width = Math.ceil(len + pad * 2);
  canvas.height = Math.ceil(roadW + pad * 2);

  ctx.save();
  const cy = canvas.height / 2;
  ctx.translate(pad, cy);

  // Asphalt
  ctx.fillStyle = roadColor;
  ctx.fillRect(0, -roadW / 2, len, roadW);

  // White shoulder lines
  ctx.fillStyle = shoulderColor;
  ctx.fillRect(0, -roadW / 2, len, 1.5);
  ctx.fillRect(0, roadW / 2 - 1.5, len, 1.5);

  // Center dashed line
  ctx.strokeStyle = dashedColor;
  ctx.lineWidth = 1.5;
  ctx.setLineDash([7, 5]);
  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(len, 0);
  ctx.stroke();
  ctx.setLineDash([]);

  ctx.restore();
  if (!hideBadge) drawPieceIdBadge(ctx, entry.id, canvas.width, canvas.height);
}

function getModelCategoryTheme(category?: string): {
  fillColor: string;
  strokeColor: string;
  defaultIcon: string;
} {
  switch (category) {
    case "nature":
      return { fillColor: "rgba(22, 163, 74, 0.40)", strokeColor: "#15803d", defaultIcon: "🌲" };
    case "railway":
      return { fillColor: "rgba(2, 132, 199, 0.40)", strokeColor: "#0369a1", defaultIcon: "🚂" };
    case "buildings":
      return { fillColor: "rgba(217, 119, 6, 0.40)", strokeColor: "#b45309", defaultIcon: "🏢" };
    case "custom":
    default:
      return { fillColor: "rgba(99, 102, 241, 0.40)", strokeColor: "#4f46e5", defaultIcon: "🧊" };
  }
}

/**
 * Creates a top-down 2D footprint canvas preview for a builtin 3D model,
 * matching the 2D track plan 1:1 in geometry, category styling and scale.
 */
export function createModelDragCanvas(
  def: BuiltinModelDef,
  scaleFactor: number = 1.0,
  zoom: number = 1.0,
): HTMLCanvasElement {
  if (typeof document === "undefined") {
    return {} as HTMLCanvasElement;
  }

  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) return canvas;

  // 1:1 scale with the trackplan (1 mm in layout = 1 screen pixel * zoom)
  const pxPerMm = Math.max(0.2, Math.min(3.0, zoom || 1.0));
  const widthMm = Math.round(def.width * scaleFactor);
  const depthMm = Math.round(def.depth * scaleFactor);
  const w = Math.max(12, Math.round(widthMm * pxPerMm));
  const d = Math.max(12, Math.round(depthMm * pxPerMm));
  const pad = 6;

  canvas.width = w + pad * 2;
  canvas.height = d + pad * 2;

  const theme = getModelCategoryTheme(def.category);
  const icon = def.icon || theme.defaultIcon;

  renderModelFootprint1to1(
    ctx,
    pad,
    pad,
    w,
    d,
    def.name,
    icon,
    theme,
    widthMm,
    depthMm,
  );

  return canvas;
}

/**
 * Creates a top-down 2D footprint canvas preview for a custom imported 3D model,
 * matching the 2D track plan 1:1.
 */
export function createCustomModelDragCanvas(
  model: Model3D,
  zoom: number = 1.0,
): HTMLCanvasElement {
  if (typeof document === "undefined") {
    return {} as HTMLCanvasElement;
  }

  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) return canvas;

  // 1:1 scale with the trackplan
  const pxPerMm = Math.max(0.2, Math.min(3.0, zoom || 1.0));
  const scale = model.scale ?? 1.0;
  const widthMm = Math.round((model.width || 120) * scale);
  const depthMm = Math.round((model.depth || 80) * scale);
  const w = Math.max(12, Math.round(widthMm * pxPerMm));
  const d = Math.max(12, Math.round(depthMm * pxPerMm));
  const pad = 6;

  canvas.width = w + pad * 2;
  canvas.height = d + pad * 2;

  const theme = getModelCategoryTheme(model.category);
  const icon = "📁";

  renderModelFootprint1to1(
    ctx,
    pad,
    pad,
    w,
    d,
    model.name,
    icon,
    theme,
    widthMm,
    depthMm,
  );

  return canvas;
}

/**
 * Renders the 2D model footprint identically to createModel2DGraphics on the track plan.
 */
function renderModelFootprint1to1(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  d: number,
  name: string,
  icon: string,
  theme: { fillColor: string; strokeColor: string },
  widthMm: number,
  depthMm: number,
): void {
  ctx.save();

  const corner = Math.min(6, Math.min(w, d) / 4);

  // 1. Semi-transparent category fill (identical to trackplan model-sprite.ts)
  ctx.fillStyle = theme.fillColor;
  roundRect(ctx, x, y, w, d, corner);
  ctx.fill();

  // 2. High-contrast border (matching trackplan stroke)
  ctx.strokeStyle = theme.strokeColor;
  ctx.lineWidth = 2.0;
  roundRect(ctx, x, y, w, d, corner);
  ctx.stroke();

  // 3. Diagonal architectural construction cross (identical to trackplan model-sprite.ts)
  if (w >= 24 && d >= 24) {
    ctx.strokeStyle = theme.strokeColor;
    ctx.lineWidth = 1.0;
    ctx.globalAlpha = 0.35;
    ctx.beginPath();
    ctx.moveTo(x + 4, y + 4);
    ctx.lineTo(x + w - 4, y + d - 4);
    ctx.moveTo(x + w - 4, y + 4);
    ctx.lineTo(x + 4, y + d - 4);
    ctx.stroke();
    ctx.globalAlpha = 1.0;
  }

  // 4. Centered text: Icon + Name + Dimensions (with drop shadow for contrast over any background)
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillStyle = "#ffffff";
  ctx.shadowColor = "rgba(0, 0, 0, 0.9)";
  ctx.shadowBlur = 4;
  ctx.shadowOffsetX = 1;
  ctx.shadowOffsetY = 1;

  const fontSize = Math.max(9, Math.min(13, Math.floor(Math.min(w / 10, d / 3.5))));

  if (d >= 44 && w >= 40) {
    // Two lines: Icon + Name, and dimension tag below
    ctx.font = `bold ${fontSize}px system-ui, -apple-system, sans-serif`;
    const maxChars = Math.max(10, Math.floor(w / 7));
    const displayName = name.length > maxChars ? `${name.slice(0, maxChars - 1)}…` : name;
    ctx.fillText(`${icon} ${displayName}`, x + w / 2, y + d * 0.42);

    ctx.font = `${Math.max(8, fontSize - 2)}px monospace`;
    ctx.fillStyle = "#f1f5f9";
    ctx.fillText(`${widthMm}×${depthMm} mm`, x + w / 2, y + d * 0.65);
  } else if (d >= 20 && w >= 30) {
    // Single line: Icon + Name
    ctx.font = `bold ${fontSize}px system-ui, -apple-system, sans-serif`;
    const maxChars = Math.max(6, Math.floor(w / 8));
    const displayName = name.length > maxChars ? `${name.slice(0, maxChars - 1)}…` : name;
    ctx.fillText(`${icon} ${displayName}`, x + w / 2, y + d / 2);
  } else {
    // Compact: just the icon
    ctx.font = `${Math.min(16, Math.max(10, d - 4))}px sans-serif`;
    ctx.fillText(icon, x + w / 2, y + d / 2);
  }

  ctx.restore();
}

/**
 * Attaches the canvas as the drag image onto DataTransfer directly and cleans up.
 */
export function attachDragImage(dataTransfer: DataTransfer, canvas: HTMLCanvasElement): void {
  if (typeof document === "undefined" || !canvas || !dataTransfer?.setDragImage) return;

  canvas.style.position = "fixed";
  canvas.style.left = "-9999px";
  canvas.style.top = "-9999px";
  canvas.style.pointerEvents = "none";
  canvas.style.zIndex = "-9999";
  document.body.appendChild(canvas);

  const cx = Math.round(canvas.width / 2) || 0;
  const cy = Math.round(canvas.height / 2) || 0;
  try {
    dataTransfer.setDragImage(canvas, cx, cy);
  } catch {
    // Fallback if browser environment restricts setDragImage
  }

  // Schedule removal on next event loop tick
  setTimeout(() => {
    canvas.remove?.();
  }, 0);
}

// Helper drawing utilities

function drawStraightBallast(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  len: number,
  width: number,
  color: string,
): void {
  ctx.fillStyle = color;
  roundRect(ctx, x, y - width / 2, len, width, 3);
  ctx.fill();
}

function drawStraightSleepers(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  len: number,
  gauge: number,
  color: string,
): void {
  ctx.fillStyle = color;
  const step = 6;
  const sleeperH = gauge + 4;
  for (let px = x + 4; px < x + len - 4; px += step) {
    ctx.fillRect(px - 1, y - sleeperH / 2, 2, sleeperH);
  }
}

function drawStraightRails(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  len: number,
  gauge: number,
  railColor: string,
  shadowColor: string,
): void {
  for (const offset of [-gauge / 2, gauge / 2]) {
    ctx.fillStyle = shadowColor;
    ctx.fillRect(x, y + offset - 1, len, 2.5);
    ctx.fillStyle = railColor;
    ctx.fillRect(x, y + offset - 0.75, len, 1.5);
  }
}

function drawTurnoutBranch(
  ctx: CanvasRenderingContext2D,
  r: number,
  rad: number,
  toLeft: boolean,
  ballastWidth: number,
  gauge: number,
  ballastColor: string,
  sleeperColor: string,
  railColor: string,
  shadowColor: string,
): void {
  ctx.save();
  if (toLeft) {
    ctx.scale(1, -1);
  }
  ctx.translate(0, r);

  // Ballast arc
  ctx.beginPath();
  ctx.arc(0, 0, r, -Math.PI / 2, -Math.PI / 2 + rad, false);
  ctx.lineWidth = ballastWidth;
  ctx.strokeStyle = ballastColor;
  ctx.stroke();

  // Rails
  for (const offset of [-gauge / 2, gauge / 2]) {
    ctx.beginPath();
    ctx.arc(0, 0, r + offset, -Math.PI / 2, -Math.PI / 2 + rad, false);
    ctx.lineWidth = 2.5;
    ctx.strokeStyle = shadowColor;
    ctx.stroke();

    ctx.beginPath();
    ctx.arc(0, 0, r + offset, -Math.PI / 2, -Math.PI / 2 + rad, false);
    ctx.lineWidth = 1.5;
    ctx.strokeStyle = railColor;
    ctx.stroke();
  }

  ctx.restore();
}

function drawConnectorDot(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  color: string,
  radius: number = 2.5,
): void {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.arc(x, y, radius, 0, Math.PI * 2);
  ctx.fill();
}

function drawPieceIdBadge(
  ctx: CanvasRenderingContext2D,
  id: string,
  w: number,
  h: number,
): void {
  ctx.save();
  ctx.font = "bold 9px monospace";
  ctx.fillStyle = "rgba(15, 23, 42, 0.75)";
  const metrics = ctx.measureText(id);
  const badgeW = metrics.width + 6;
  const badgeH = 12;
  const bx = w - badgeW - 3;
  const by = h - badgeH - 3;
  ctx.fillRect(bx, by, badgeW, badgeH);

  ctx.fillStyle = "#38bdf8";
  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  ctx.fillText(id, bx + 3, by + badgeH / 2);
  ctx.restore();
}

function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.arcTo(x + w, y, x + w, y + r, r);
  ctx.lineTo(x + w, y + h - r);
  ctx.arcTo(x + w, y + h, x + w - r, y + h, r);
  ctx.lineTo(x + r, y + h);
  ctx.arcTo(x, y + h, x, y + h - r, r);
  ctx.lineTo(x, y + r);
  ctx.arcTo(x, y, x + r, y, r);
  ctx.closePath();
}
