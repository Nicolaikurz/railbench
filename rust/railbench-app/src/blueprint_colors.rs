use eframe::egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintColor {
    pub hex: u32,
    pub badge_bg: u32,
    pub border: u32,
}

impl BlueprintColor {
    pub const fn color32(self) -> Color32 {
        rgb_u32_to_color32(self.hex)
    }

    pub const fn badge_bg_color32(self) -> Color32 {
        rgb_u32_to_color32(self.badge_bg)
    }

    pub const fn border_color32(self) -> Color32 {
        rgb_u32_to_color32(self.border)
    }

    pub fn rgb_f32(self) -> [f32; 3] {
        [
            ((self.hex >> 16) & 0xff) as f32 / 255.0,
            ((self.hex >> 8) & 0xff) as f32 / 255.0,
            (self.hex & 0xff) as f32 / 255.0,
        ]
    }
}

pub const fn rgb_u32_to_color32(hex: u32) -> Color32 {
    Color32::from_rgb(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

/// High-distinction 18-color palette matching `src/render/blueprint-colors.ts` in the Electron app.
pub const BLUEPRINT_PALETTE: [BlueprintColor; 18] = [
    BlueprintColor { hex: 0x0284c7, badge_bg: 0x0369a1, border: 0x38bdf8 }, // Sky Blue
    BlueprintColor { hex: 0xd97706, badge_bg: 0xb45309, border: 0xfbbf24 }, // Amber
    BlueprintColor { hex: 0x059669, badge_bg: 0x047857, border: 0x34d399 }, // Emerald
    BlueprintColor { hex: 0x7c3aed, badge_bg: 0x6d28d9, border: 0xa78bfa }, // Violet
    BlueprintColor { hex: 0xdb2777, badge_bg: 0xbe185d, border: 0xf472b6 }, // Pink
    BlueprintColor { hex: 0x0891b2, badge_bg: 0x0e7490, border: 0x22d3ee }, // Cyan
    BlueprintColor { hex: 0xea580c, badge_bg: 0xc2410c, border: 0xfb923c }, // Orange
    BlueprintColor { hex: 0x4f46e5, badge_bg: 0x4338ca, border: 0x818cf8 }, // Indigo
    BlueprintColor { hex: 0x65a30d, badge_bg: 0x4d7c0f, border: 0xa3e635 }, // Lime
    BlueprintColor { hex: 0xc026d3, badge_bg: 0xa21caf, border: 0xe879f9 }, // Fuchsia
    BlueprintColor { hex: 0x0d9488, badge_bg: 0x0f766e, border: 0x2dd4bf }, // Teal
    BlueprintColor { hex: 0xdc2626, badge_bg: 0xb91c1c, border: 0xf87171 }, // Red
    BlueprintColor { hex: 0x2563eb, badge_bg: 0x1d4ed8, border: 0x60a5fa }, // Royal Blue
    BlueprintColor { hex: 0xca8a04, badge_bg: 0xa16207, border: 0xfacc15 }, // Yellow
    BlueprintColor { hex: 0x9333ea, badge_bg: 0x7e22ce, border: 0xc084fc }, // Purple
    BlueprintColor { hex: 0x16a34a, badge_bg: 0x15803d, border: 0x4ade80 }, // Green
    BlueprintColor { hex: 0xe11d48, badge_bg: 0xbe123c, border: 0xfb7185 }, // Rose
    BlueprintColor { hex: 0x475569, badge_bg: 0x334155, border: 0x94a3b8 }, // Slate
];

/// Returns the deterministic Electron-compatible blueprint color for a track catalog ID.
pub fn get_track_blueprint_color(catalog_id: &str) -> BlueprintColor {
    let mut hash: i32 = 0;
    for cu in catalog_id.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(cu as i32);
    }
    let idx = (hash.wrapping_abs() as usize) % BLUEPRINT_PALETTE.len();
    BLUEPRINT_PALETTE[idx]
}

/// Darkens RGB channels equally toward black, matching `darkenColor()` in `src/render/track-render-utils.ts`.
pub fn darken_color32(c: Color32, factor: f32) -> Color32 {
    let scale = (1.0 - factor).clamp(0.0, 1.0);
    let r = (c.r() as f32 * scale).round() as u8;
    let g = (c.g() as f32 * scale).round() as u8;
    let b = (c.b() as f32 * scale).round() as u8;
    Color32::from_rgba_unmultiplied(r, g, b, c.a())
}

/// Desaturates & fades inactive-layer colors identically to `desaturateColor()` in `src/render/track-render-utils.ts`.
pub fn desaturate_electron_color32(c: Color32, factor: f32, alpha_scale: f32) -> Color32 {
    let r = c.r() as f32;
    let g = c.g() as f32;
    let b = c.b() as f32;
    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
    let target = luma * 0.85 + 24.0 * 0.15;
    let nr = (r + (target - r) * factor).round().clamp(0.0, 255.0) as u8;
    let ng = (g + (target - g) * factor).round().clamp(0.0, 255.0) as u8;
    let nb = (b + (target - b) * factor).round().clamp(0.0, 255.0) as u8;
    let na = (c.a() as f32 * alpha_scale.clamp(0.0, 1.0)).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(nr, ng, nb, na)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_hash_matches_electron_exact_hex_values() {
        assert_eq!(get_track_blueprint_color("24188").hex, 0xdc2626);
        assert_eq!(get_track_blueprint_color("24172").hex, 0x0d9488);
        assert_eq!(get_track_blueprint_color("24130").hex, 0x0d9488);
        assert_eq!(get_track_blueprint_color("24230").hex, 0x475569);
        assert_eq!(get_track_blueprint_color("24611").hex, 0x059669);
        assert_eq!(get_track_blueprint_color("24612").hex, 0x7c3aed);
        assert_eq!(get_track_blueprint_color("24671").hex, 0x65a30d);
        assert_eq!(get_track_blueprint_color("24977").hex, 0x475569);
    }

    #[test]
    fn darken_and_desaturate_match_electron_formulas() {
        let sky = rgb_u32_to_color32(0x0284c7);
        let dark = darken_color32(sky, 0.30);
        assert_eq!(dark, Color32::from_rgb(1, 92, 139));

        let desat = desaturate_electron_color32(sky, 0.72, 0.58);
        assert_eq!(desat.a(), 148);
        assert!(desat.r() > sky.r() && desat.b() < sky.b());
    }
}
