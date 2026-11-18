import { Container, Graphics, Text, TextStyle } from "pixi.js";
import type { Vec2 } from "@/core/types";
import { getTranslations } from "@/i18n";
import { terrainBrush } from "@/editor/terrain-brush-controller";
import { boardSketcher } from "@/editor/board-sketcher-controller";
import type { RenderStateOptions } from "./renderer";

const _segmentBadgeStyleCache = new Map<number, TextStyle>();
function getSegmentBadgeStyle(color: number): TextStyle {
  let style = _segmentBadgeStyleCache.get(color);
  if (!style) {
    style = new TextStyle({
      fontSize: 10.5,
      fill: color,
      fontWeight: "bold",
      stroke: { color: 0x0f172a, width: 3 },
    });
    _segmentBadgeStyleCache.set(color, style);
  }
  return style;
}

const MEASURE_POINT_TEXT_STYLE = new TextStyle({
  fontSize: 9,
  fill: 0xffffff,
  fontWeight: "bold",
  stroke: { color: 0x0f172a, width: 2 },
});

const MEASURE_DIST_TEXT_STYLE = new TextStyle({
  fontSize: 11,
  fill: 0xffffff,
  fontWeight: "bold",
  align: "center",
  stroke: { color: 0x0f172a, width: 3 },
});

const SHORT_CIRCUIT_TEXT_STYLE = new TextStyle({
  fontSize: 9,
  fill: 0xffffff,
  fontWeight: "bold",
});

const POINT_MARKER_TEXT_STYLE = new TextStyle({
  fontSize: 12.5,
  fill: 0xffffff,
  fontWeight: "bold",
  stroke: { color: 0x0f172a, width: 3.5 },
});

export function drawDashedLine(
  g: Graphics,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  dash = 4,
  gap = 4,
): void {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.hypot(dx, dy);
  if (len === 0) return;
  const cos = dx / len;
  const sin = dy / len;
  let dist = 0;
  while (dist < len) {
    const curDash = Math.min(dash, len - dist);
    g.moveTo(x1 + cos * dist, y1 + sin * dist);
    g.lineTo(x1 + cos * (dist + curDash), y1 + sin * (dist + curDash));
    dist += dash + gap;
  }
}

function addSegmentLengthBadge(
  overlayLayer: Container,
  p1: Vec2,
  p2: Vec2,
  color = 0x38bdf8,
): void {
  const dx = p2.x - p1.x;
  const dy = p2.y - p1.y;
  const len = Math.hypot(dx, dy);
  if (len < 60) return;

  const midX = (p1.x + p2.x) / 2;
  const midY = (p1.y + p2.y) / 2;
  const txt = new Text({
    text: `${Math.round(len)} mm`,
    style: getSegmentBadgeStyle(color),
  });
  txt.anchor.set(0.5, 0.5);
  txt.position.set(midX, midY - 10);
  overlayLayer.addChild(txt);
}

/**
 * Renders the interactive CAD Sketcher overlay for tables and layer plates.
 */
export function renderBoardSketcherOverlay(overlayLayer: Container, ov: Graphics): void {
  if (!boardSketcher.isActive) return;

  // 1. Orthogonal / alignment guide lines
  for (const guide of boardSketcher.activeGuides) {
    drawDashedLine(ov, guide.x1, guide.y1, guide.x2, guide.y2, 6, 4);
    ov.stroke({ width: 1.5, color: 0x10b981, alpha: 0.85 });
  }

  // 2. Interactive Vertex & Edge Editor ("edit" mode)
  if (boardSketcher.mode === "edit" && boardSketcher.editPoints.length >= 3) {
    const pts = boardSketcher.editPoints;
    const n = pts.length;

    // Highlighted CAD contour
    ov.poly(pts);
    ov.fill({ color: 0x0ea5e9, alpha: 0.1 });
    ov.stroke({ width: 2.5, color: 0x38bdf8, alpha: 0.95 });

    // Edge length labels + Midpoint [+] handles
    for (let i = 0; i < n; i++) {
      const p1 = pts[i]!;
      const p2 = pts[(i + 1) % n]!;
      addSegmentLengthBadge(overlayLayer, p1, p2, 0xe0f2fe);

      const mx = (p1.x + p2.x) / 2;
      const my = (p1.y + p2.y) / 2;
      const isHoveredMid = boardSketcher.hoveredMidpointIndex === i;
      const rMid = isHoveredMid ? 8 : 5.5;
      ov.circle(mx, my, rMid);
      ov.fill({ color: isHoveredMid ? 0x22c55e : 0x0f172a, alpha: 0.92 });
      ov.stroke({ width: 1.6, color: 0x22c55e });
      // '+' cross inside midpoint handle
      ov.moveTo(mx - 3, my).lineTo(mx + 3, my);
      ov.moveTo(mx, my - 3).lineTo(mx, my + 3);
      ov.stroke({ width: 1.4, color: 0xffffff });
    }

    // Corner vertex handles
    for (let i = 0; i < n; i++) {
      const p = pts[i]!;
      const isActive =
        boardSketcher.draggingVertexIndex === i || boardSketcher.hoveredVertexIndex === i;
      const sz = isActive ? 14 : 10;
      ov.rect(p.x - sz / 2, p.y - sz / 2, sz, sz);
      ov.fill({ color: isActive ? 0xf59e0b : 0xffffff });
      ov.stroke({ width: 2.2, color: 0x0284c7 });
    }
    return;
  }

  // 3. Rectangle Sketcher ("rect" mode)
  if (boardSketcher.mode === "rect") {
    const cur = boardSketcher.snappedCursor;
    if (boardSketcher.rectStart && cur) {
      const p1 = boardSketcher.rectStart;
      const minX = Math.min(p1.x, cur.x);
      const maxX = Math.max(p1.x, cur.x);
      const minY = Math.min(p1.y, cur.y);
      const maxY = Math.max(p1.y, cur.y);
      const w = maxX - minX;
      const d = maxY - minY;

      ov.rect(minX, minY, w, d);
      ov.fill({ color: 0x0284c7, alpha: 0.18 });
      ov.stroke({ width: 2.5, color: 0x38bdf8, alpha: 0.95 });

      addSegmentLengthBadge(
        overlayLayer,
        { x: minX, y: minY },
        { x: maxX, y: minY },
        0x38bdf8,
      );
      addSegmentLengthBadge(
        overlayLayer,
        { x: minX, y: minY },
        { x: minX, y: maxY },
        0x38bdf8,
      );
    }
    if (cur) {
      ov.circle(cur.x, cur.y, 5);
      ov.fill({ color: 0x38bdf8 });
      ov.stroke({ width: 1.5, color: 0xffffff });
    }
    return;
  }

  // 4. Polygon Sketcher ("poly" mode)
  const pts = boardSketcher.points;
  const cur = boardSketcher.snappedCursor;

  if (pts.length >= 2) {
    const previewPoly = cur ? [...pts, cur] : pts;
    if (previewPoly.length >= 3) {
      ov.poly(previewPoly);
      ov.fill({ color: 0x0284c7, alpha: 0.16 });
    }
  }

  // Placed segments
  for (let i = 0; i < pts.length - 1; i++) {
    const p1 = pts[i]!;
    const p2 = pts[i + 1]!;
    ov.moveTo(p1.x, p1.y).lineTo(p2.x, p2.y);
    ov.stroke({ width: 2.5, color: 0x38bdf8, alpha: 0.95 });
    addSegmentLengthBadge(overlayLayer, p1, p2, 0xe0f2fe);
  }

  // Live rubber-band line from last vertex to cursor + closing preview line to start vertex
  if (pts.length >= 1 && cur) {
    const last = pts[pts.length - 1]!;
    ov.moveTo(last.x, last.y).lineTo(cur.x, cur.y);
    ov.stroke({ width: 2.2, color: 0x22c55e, alpha: 0.95 });
    addSegmentLengthBadge(overlayLayer, last, cur, 0x4ade80);

    if (pts.length >= 2) {
      const first = pts[0]!;
      drawDashedLine(ov, cur.x, cur.y, first.x, first.y, 6, 5);
      ov.stroke({ width: 1.5, color: 0x94a3b8, alpha: 0.7 });
    }
  }

  // Placed vertices
  for (let i = 0; i < pts.length; i++) {
    const p = pts[i]!;
    const isStart = i === 0;
    if (isStart && boardSketcher.isNearStartPoint) {
      ov.circle(p.x, p.y, 13);
      ov.fill({ color: 0x22c55e, alpha: 0.3 });
      ov.stroke({ width: 2.5, color: 0x22c55e });
    }
    ov.circle(p.x, p.y, isStart ? 6.5 : 5);
    ov.fill({ color: isStart ? 0x22c55e : 0xffffff });
    ov.stroke({ width: 2, color: 0x0284c7 });
  }

  // Cursor crosshair dot
  if (cur) {
    ov.circle(cur.x, cur.y, 5);
    ov.fill({ color: 0x38bdf8 });
    ov.stroke({ width: 1.5, color: 0xffffff });
  }
}

/**
 * Renders interactive tool overlays (terrain brush, board sketcher, marquee box,
 * measure tool, short-circuit badges, and A/B route pins) onto the 2D overlayLayer.
 */
export function renderSceneOverlays(
  overlayLayer: Container,
  ov: Graphics,
  options: RenderStateOptions,
): void {
  // 1. Gelände-Pinsel / Stift Vorschau (Cursor-Radius & aktiver Pfad)
  if (terrainBrush.isActive) {
    const r = terrainBrush.brushRadius;
    const modeColorMap = {
      paint: 0x22c55e,
      extend: 0x0ea5e9,
      stamp: 0xf59e0b,
      erase: 0xef4444,
    } as const;
    const strokeColor = modeColorMap[terrainBrush.mode] ?? 0x22c55e;
    if (terrainBrush.strokePoints.length >= 2) {
      ov.moveTo(terrainBrush.strokePoints[0]!.x, terrainBrush.strokePoints[0]!.y);
      for (let i = 1; i < terrainBrush.strokePoints.length; i++) {
        ov.lineTo(terrainBrush.strokePoints[i]!.x, terrainBrush.strokePoints[i]!.y);
      }
      ov.stroke({ width: r * 2, color: 0x44403c, alpha: 0.48, cap: "round", join: "round" });
      ov.moveTo(terrainBrush.strokePoints[0]!.x, terrainBrush.strokePoints[0]!.y);
      for (let i = 1; i < terrainBrush.strokePoints.length; i++) {
        ov.lineTo(terrainBrush.strokePoints[i]!.x, terrainBrush.strokePoints[i]!.y);
      }
      ov.stroke({ width: r * 0.7, color: 0xf8fafc, alpha: 0.65, cap: "round", join: "round" });
      for (const pt of terrainBrush.strokePoints) {
        ov.circle(pt.x, pt.y, 6);
        ov.fill({ color: strokeColor, alpha: 0.9 });
      }
    }
    if (terrainBrush.cursorWorld) {
      const cx = terrainBrush.cursorWorld.x;
      const cy = terrainBrush.cursorWorld.y;
      ov.circle(cx, cy, r);
      ov.fill({ color: strokeColor, alpha: 0.14 });
      ov.stroke({ width: 2.2, color: strokeColor, alpha: 0.9 });
      if (terrainBrush.mode !== "erase") {
        ov.circle(cx, cy, r * 0.35);
        ov.stroke({ width: 1.4, color: 0xffffff, alpha: 0.55 });
      }
    }
  }

  // 1B. Tisch- & Holzplatten-Sketcher Vorschau
  renderBoardSketcherOverlay(overlayLayer, ov);

  // 2. Auswahlrahmen (Marquee Box)
  if (options.selectionBox) {
    const box = options.selectionBox;
    const minX = Math.min(box.start.x, box.end.x);
    const maxX = Math.max(box.start.x, box.end.x);
    const minY = Math.min(box.start.y, box.end.y);
    const maxY = Math.max(box.start.y, box.end.y);
    const w = maxX - minX;
    const h = maxY - minY;

    if (w > 2 || h > 2) {
      ov.rect(minX, minY, w, h);
      ov.fill({ color: 0x0284c7, alpha: 0.15 });
      ov.stroke({ width: 1.5, color: 0x38bdf8, alpha: 0.95 });

      const cornerSize = Math.min(8, w / 4, h / 4);
      if (cornerSize > 1) {
        ov.moveTo(minX, minY + cornerSize).lineTo(minX, minY).lineTo(minX + cornerSize, minY);
        ov.moveTo(maxX - cornerSize, minY).lineTo(maxX, minY).lineTo(maxX, minY + cornerSize);
        ov.moveTo(minX, maxY - cornerSize).lineTo(minX, maxY).lineTo(minX + cornerSize, maxY);
        ov.moveTo(maxX - cornerSize, maxY).lineTo(maxX, maxY).lineTo(maxX, maxY - cornerSize);
        ov.stroke({ width: 2.5, color: 0x0284c7, alpha: 1 });
      }
    }
  }

  // 3. Messwerkzeug (Measure Tool)
  if (options.measureTool) {
    const { start: p1, current: p2, locked, scaleRatio } = options.measureTool;

    if (p1) {
      ov.circle(p1.x, p1.y, 8).fill({ color: 0x0284c7, alpha: 0.25 });
      ov.circle(p1.x, p1.y, 5).fill({ color: 0x0284c7 }).stroke({ width: 1.5, color: 0xffffff });

      const txtA = new Text({
        text: "A",
        style: MEASURE_POINT_TEXT_STYLE,
      });
      txtA.anchor.set(0.5, 1);
      txtA.position.set(p1.x, p1.y - 7);
      overlayLayer.addChild(txtA);
    }

    if (p1 && p2) {
      const dx = p2.x - p1.x;
      const dy = p2.y - p1.y;
      const dist = Math.hypot(dx, dy);

      if (dist > 15 && (Math.abs(dx) > 10 || Math.abs(dy) > 10)) {
        drawDashedLine(ov, p1.x, p1.y, p2.x, p1.y, 4, 3);
        ov.stroke({ width: 1, color: 0x94a3b8, alpha: 0.5 });

        drawDashedLine(ov, p2.x, p1.y, p2.x, p2.y, 4, 3);
        ov.stroke({ width: 1, color: 0x94a3b8, alpha: 0.5 });
      }

      if (dist > 0.5) {
        const nx = -dy / dist;
        const ny = dx / dist;
        const tickLen = 7;

        ov.moveTo(p1.x, p1.y).lineTo(p2.x, p2.y);
        ov.stroke({ width: 3.5, color: 0x0f172a, alpha: 0.7 });

        const lineColor = locked ? 0x10b981 : 0x0284c7;
        ov.moveTo(p1.x, p1.y).lineTo(p2.x, p2.y);
        ov.stroke({ width: 2, color: lineColor, alpha: 0.95 });

        ov.moveTo(p1.x + nx * tickLen, p1.y + ny * tickLen).lineTo(p1.x - nx * tickLen, p1.y - ny * tickLen);
        ov.moveTo(p2.x + nx * tickLen, p2.y + ny * tickLen).lineTo(p2.x - nx * tickLen, p2.y - ny * tickLen);
        ov.stroke({ width: 2, color: lineColor, alpha: 0.95 });

        ov.circle(p2.x, p2.y, 8).fill({ color: lineColor, alpha: 0.25 });
        ov.circle(p2.x, p2.y, 5).fill({ color: lineColor }).stroke({ width: 1.5, color: 0xffffff });

        const txtB = new Text({
          text: "B",
          style: MEASURE_POINT_TEXT_STYLE,
        });
        txtB.anchor.set(0.5, 1);
        txtB.position.set(p2.x, p2.y - 7);
        overlayLayer.addChild(txtB);

        const midX = (p1.x + p2.x) / 2;
        const midY = (p1.y + p2.y) / 2;

        let distStr =
          dist >= 1000
            ? `${dist.toFixed(1)} mm (${(dist / 1000).toFixed(2)} m)`
            : `${dist.toFixed(1)} mm`;

        if (scaleRatio && scaleRatio > 1) {
          const protoM = (dist * scaleRatio) / 1000;
          distStr += `\nVorbild: ${protoM.toFixed(2)} m`;
        }

        const txt = new Text({
          text: distStr,
          style: MEASURE_DIST_TEXT_STYLE,
        });
        txt.anchor.set(0.5, 0.5);
        txt.position.set(midX + nx * 16, midY + ny * 16);
        overlayLayer.addChild(txt);
      }
    }

    if (options.measureTool.snapPoint) {
      const sp = options.measureTool.snapPoint;
      ov.circle(sp.x, sp.y, 7).stroke({ width: 2, color: 0x10b981, alpha: 0.9 });
      ov.circle(sp.x, sp.y, 3).fill({ color: 0x10b981 });
    }
  }

  // 4. Kurzschluss-Indikator
  if (options.shortCircuit?.hasShortCircuit) {
    const t = getTranslations().toolbar.status;
    const overlayText = t.shortCircuitOverlayLabel || "KURZSCHLUSS";

    for (const pt of options.shortCircuit.shortCircuitPoints) {
      ov.circle(pt.x, pt.y, 18);
      ov.fill({ color: 0xef4444, alpha: 0.28 });
      ov.circle(pt.x, pt.y, 12);
      ov.stroke({ width: 2.5, color: 0xdc2626, alpha: 0.95 });

      ov.circle(pt.x, pt.y, 4.5);
      ov.fill({ color: 0xffffff });
      ov.stroke({ width: 2, color: 0xb91c1c });

      const badgeCont = new Container();
      badgeCont.position.set(pt.x, pt.y - 20);

      const pillBg = new Graphics();
      pillBg.roundRect(-48, -10, 96, 20, 5);
      pillBg.fill({ color: 0x7f1d1d, alpha: 0.95 });
      pillBg.stroke({ width: 1.5, color: 0xef4444 });
      badgeCont.addChild(pillBg);

      const lbl = new Text({
        text: `⚡ ${overlayText}`,
        style: SHORT_CIRCUIT_TEXT_STYLE,
      });
      lbl.anchor.set(0.5, 0.5);
      badgeCont.addChild(lbl);

      overlayLayer.addChild(badgeCont);
    }
  }

  // 5. Point Markers (Start / Stop Pins)
  if (options.pointMarkers && options.pointMarkers.length > 0) {
    for (const pm of options.pointMarkers) {
      ov.circle(pm.x, pm.y, 16);
      ov.fill({ color: pm.color, alpha: 0.25 });
      ov.circle(pm.x, pm.y, 8);
      ov.stroke({ width: 2.5, color: pm.color, alpha: 0.95 });
      ov.circle(pm.x, pm.y, 3.5);
      ov.fill({ color: 0xffffff, alpha: 1.0 });

      const pinCont = new Container();
      pinCont.position.set(pm.x, pm.y);

      const badgeH = 26;
      const badgeW = Math.max(36, pm.label.length * 10.5 + 20);
      const badgeY = -30;

      const pinGfx = new Graphics();
      pinGfx.poly([-7, -14, 7, -14, 0, 0]);
      pinGfx.roundRect(-badgeW / 2 + 1, badgeY - badgeH / 2 + 2, badgeW, badgeH, badgeH / 2);
      pinGfx.fill({ color: 0x000000, alpha: 0.32 });

      pinGfx.roundRect(-badgeW / 2 - 3, badgeY - badgeH / 2 - 3, badgeW + 6, badgeH + 6, (badgeH + 6) / 2);
      pinGfx.fill({ color: pm.color, alpha: 0.35 });

      pinGfx.poly([-8, badgeY + badgeH / 2 - 2, 8, badgeY + badgeH / 2 - 2, 0, -2]);
      pinGfx.fill({ color: pm.color, alpha: 0.98 });
      pinGfx.stroke({ width: 2.5, color: 0xffffff });

      pinGfx.roundRect(-badgeW / 2, badgeY - badgeH / 2, badgeW, badgeH, badgeH / 2);
      pinGfx.fill({ color: pm.color, alpha: 0.98 });
      pinGfx.stroke({ width: 2.5, color: 0xffffff });

      pinGfx.roundRect(-badgeW / 2, badgeY - badgeH / 2, badgeW, badgeH, badgeH / 2);
      pinGfx.stroke({ width: 1, color: 0x0f172a, alpha: 0.45 });

      pinCont.addChild(pinGfx);

      const txt = new Text({
        text: pm.label,
        style: POINT_MARKER_TEXT_STYLE,
      });
      txt.anchor.set(0.5, 0.5);
      txt.position.set(0, badgeY);
      pinCont.addChild(txt);

      overlayLayer.addChild(pinCont);
    }
  }
}
