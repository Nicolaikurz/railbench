export interface BlueprintColor {
  hex: number;
  css: string;
  badgeBg: string;
  badgeText: string;
  border: string;
}

/**
 * High-distinction palette for color-coding identical track types in Blueprint mode.
 */
const BLUEPRINT_PALETTE: BlueprintColor[] = [
  { hex: 0x0284c7, css: "#0284c7", badgeBg: "#0369a1", badgeText: "#ffffff", border: "#38bdf8" }, // Sky Blue
  { hex: 0xd97706, css: "#d97706", badgeBg: "#b45309", badgeText: "#ffffff", border: "#fbbf24" }, // Amber
  { hex: 0x059669, css: "#059669", badgeBg: "#047857", badgeText: "#ffffff", border: "#34d399" }, // Emerald
  { hex: 0x7c3aed, css: "#7c3aed", badgeBg: "#6d28d9", badgeText: "#ffffff", border: "#a78bfa" }, // Violet
  { hex: 0xdb2777, css: "#db2777", badgeBg: "#be185d", badgeText: "#ffffff", border: "#f472b6" }, // Pink
  { hex: 0x0891b2, css: "#0891b2", badgeBg: "#0e7490", badgeText: "#ffffff", border: "#22d3ee" }, // Cyan
  { hex: 0xea580c, css: "#ea580c", badgeBg: "#c2410c", badgeText: "#ffffff", border: "#fb923c" }, // Orange
  { hex: 0x4f46e5, css: "#4f46e5", badgeBg: "#4338ca", badgeText: "#ffffff", border: "#818cf8" }, // Indigo
  { hex: 0x65a30d, css: "#65a30d", badgeBg: "#4d7c0f", badgeText: "#ffffff", border: "#a3e635" }, // Lime
  { hex: 0xc026d3, css: "#c026d3", badgeBg: "#a21caf", badgeText: "#ffffff", border: "#e879f9" }, // Fuchsia
  { hex: 0x0d9488, css: "#0d9488", badgeBg: "#0f766e", badgeText: "#ffffff", border: "#2dd4bf" }, // Teal
  { hex: 0xdc2626, css: "#dc2626", badgeBg: "#b91c1c", badgeText: "#ffffff", border: "#f87171" }, // Red
  { hex: 0x2563eb, css: "#2563eb", badgeBg: "#1d4ed8", badgeText: "#ffffff", border: "#60a5fa" }, // Royal Blue
  { hex: 0xca8a04, css: "#ca8a04", badgeBg: "#a16207", badgeText: "#ffffff", border: "#facc15" }, // Yellow
  { hex: 0x9333ea, css: "#9333ea", badgeBg: "#7e22ce", badgeText: "#ffffff", border: "#c084fc" }, // Purple
  { hex: 0x16a34a, css: "#16a34a", badgeBg: "#15803d", badgeText: "#ffffff", border: "#4ade80" }, // Green
  { hex: 0xe11d48, css: "#e11d48", badgeBg: "#be123c", badgeText: "#ffffff", border: "#fb7185" }, // Rose
  { hex: 0x475569, css: "#475569", badgeBg: "#334155", badgeText: "#ffffff", border: "#94a3b8" }, // Slate
];

/**
 * Returns a consistent, deterministic color for any track article / catalog ID.
 * Identical track pieces will always share the exact same color.
 */
export function getTrackBlueprintColor(catalogId: string): BlueprintColor {
  let hash = 0;
  for (let i = 0; i < catalogId.length; i++) {
    hash = ((hash << 5) - hash + catalogId.charCodeAt(i)) | 0;
  }
  const idx = Math.abs(hash) % BLUEPRINT_PALETTE.length;
  return BLUEPRINT_PALETTE[idx]!;
}

