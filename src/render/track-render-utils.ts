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

export function darkenColor(hex: number, factor = 0.18): number {
  const scale = Math.max(0, Math.min(1, 1 - factor));
  const r = Math.round(((hex >> 16) & 0xff) * scale);
  const g = Math.round(((hex >> 8) & 0xff) * scale);
  const b = Math.round((hex & 0xff) * scale);
  return (r << 16) | (g << 8) | b;
}

/**
 * Zeichnet einen Kreisbogen mit exakten radialen Endkappen als gefülltes Ringsegment
 * statt eines Pixi-`arc().stroke({ cap: "butt" })`.
 * Verhindert keilförmige Spalten an den Übergängen von Bogengleisen, Bogenweichen
 * und gekrümmten Straßen in der 2D-Ansicht.
 */
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
  let sweep = endAngle - startAngle;
  if (!counterclockwise) {
    while (sweep < 0) sweep += Math.PI * 2;
  } else {
    while (sweep > 0) sweep -= Math.PI * 2;
  }

  const absSweep = Math.abs(sweep);
  if (absSweep < 1e-6 || strokeWidth <= 0 || radius <= 0) return;

  const dir = sweep >= 0 ? 1 : -1;
  // Minimaler Überlapp (0.35 mm bei Schotter/Straße, 0.2 mm bei Schienen),
  // um Sub-Pixel-Anti-Aliasing-Spalten an Gleisübergängen zu schließen.
  const padMm = strokeWidth >= 6 ? 0.35 : 0.2;
  const padAngle = Math.min(padMm / Math.max(radius, 1), absSweep * 0.25);
  const a0 = startAngle - dir * padAngle;
  const a1 = startAngle + sweep + dir * padAngle;

  const halfW = strokeWidth / 2;
  const rInner = Math.max(0.1, radius - halfW);
  const rOuter = radius + halfW;
  const arcLen = Math.abs(a1 - a0) * rOuter;
  const steps = Math.max(12, Math.min(96, Math.ceil(arcLen / 6)));

  for (let i = 0; i <= steps; i++) {
    const ang = a0 + ((a1 - a0) * i) / steps;
    const x = cx + rOuter * Math.cos(ang);
    const y = cy + rOuter * Math.sin(ang);
    if (i === 0) g.moveTo(x, y);
    else g.lineTo(x, y);
  }

  for (let i = steps; i >= 0; i--) {
    const ang = a0 + ((a1 - a0) * i) / steps;
    const x = cx + rInner * Math.cos(ang);
    const y = cy + rInner * Math.sin(ang);
    g.lineTo(x, y);
  }

  g.closePath();
  g.fill({ color, alpha });
}
