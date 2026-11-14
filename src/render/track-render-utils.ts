import { Graphics } from "pixi.js";

export function desaturateColor(hex: number, factor = 0.72): number {
  const r = (hex >> 16) & 0xff;
  const g = (hex >> 8) & 0xff;
  const b = hex & 0xff;
  const luma = 0.299 * r + 0.587 * g + 0.114 * b;
  const target = luma * 0.85 + 24 * 0.15;
  const nr = Math.round(Math.max(0, Math.min(255, r + (target - r) * factor)));
  const ng = Math.round(Math.max(0, Math.min(255, g + (target - g) * factor)));
  const nb = Math.round(Math.max(0, Math.min(255, b + (target - b) * factor)));
  return (nr << 16) | (ng << 8) | nb;
}

export function drawArcStroke(
  g: Graphics,
  cx: number,
  cy: number,
  radius: number,
  startAngle: number,
  endAngle: number,
  strokeWidth: number,
  color: number,
  counterclockwise = false,
  alpha = 1,
) {
  const sx = cx + radius * Math.cos(startAngle);
  const sy = cy + radius * Math.sin(startAngle);
  g.moveTo(sx, sy);
  g.arc(cx, cy, radius, startAngle, endAngle, counterclockwise);
  g.stroke({ width: strokeWidth, color, alpha, cap: "butt", join: "miter" });
}
