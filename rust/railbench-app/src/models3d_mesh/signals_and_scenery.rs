use super::*;

// ----------------------------------------------------------------------------
// 9. LIGHT SIGNAL (DB Lichthauptsignal — plinth, cabinet, tapered mast, ladder, hooded optics)
// ----------------------------------------------------------------------------
pub(crate) fn build_light_signal(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [25.0, 85.0, 25.0]);
    let concrete = Mat::matte(0x64748b, 0.90);
    let mast_m = Mat::metal(0x334155, 0.42, 0.72);
    let cab_m = Mat::metal(0x94a3b8, 0.55, 0.45);
    let shield_m = Mat::matte(0x0f172a, 0.65);
    let white_m = Mat::matte(0xf8fafc, 0.50);
    let red_m = Mat::matte(0xdc2626, 0.50);
    let green_glow = Mat::emissive(0x22c55e, 1.15);
    let red_glow = Mat::emissive(0xef4444, 0.95);
    let yellow_glow = Mat::emissive(0xf59e0b, 0.65);

    // 1. Stepped concrete foundation & steel base flange
    b.rbox([0.0, 2.2, 0.0], [18.0, 4.4, 18.0], concrete);
    b.rbox([0.0, 5.1, 0.0], [9.0, 1.4, 9.0], mast_m);

    // 2. Smooth tapered steel tubular mast & trackside relay cabinet
    b.cyl_y([0.0, 41.0, 0.0], 1.65, 2.15, 72.0, 16, mast_m);
    b.rbox([0.0, 13.0, -1.5], [9.5, 14.0, 7.5], cab_m);
    b.rbox([0.0, 20.5, -1.5], [10.5, 1.2, 8.5], mast_m);

    // 3. White-red-white DB mast plate (+Z) + Indusi / PZB track magnet box at base
    b.rbox([0.0, 38.0, 2.3], [4.2, 22.0, 0.8], white_m);
    b.rbox([0.0, 38.0, 2.85], [2.2, 21.5, 0.5], red_m);

    // 4. Rear maintenance ladder (-Z) & maintenance platform basket with safety handrail
    for lx in [-1.8_f32, 1.8] {
        b.cyl_y([lx, 47.1, -3.5], 0.35, 0.35, 52.0, 10, mast_m);
    }
    for ry in (22..=72).step_by(5) {
        b.rbox([0.0, ry as f32, -3.5], [3.8, 0.5, 0.5], mast_m);
    }
    b.rbox([0.0, 53.0, -2.5], [11.0, 1.0, 7.0], mast_m);
    // Safety cage hoop around maintenance platform
    b.rbox([0.0, 58.0, -5.8], [11.0, 0.6, 0.6], mast_m);
    for sx in [-5.2_f32, 5.2] {
        b.rbox([sx, 55.5, -5.8], [0.6, 5.2, 0.6], mast_m);
        b.rbox([sx, 58.0, -2.8], [0.6, 0.6, 6.0], mast_m);
    }

    // 5. Chamfered DB Signal Head Shield (octagonal-clipped appearance with white border frame)
    b.rbox([0.0, 69.0, 2.8], [13.2, 26.8, 1.4], white_m);
    b.rbox([0.0, 69.0, 3.4], [11.8, 25.4, 1.8], shield_m);

    // 6. Three main hooded signal optics (Hp1 green top, Hp0 red middle, Hp2 yellow bottom)
    //    + two small white Sh1 / Ra12 shunting indicator optics
    for (ay, lm) in [(76.0_f32, green_glow), (69.0, red_glow), (62.0, yellow_glow)] {
        b.cyl_z([0.0, ay, 4.35], 1.9, 1.9, 1.6, 18, lm);
        // Curved/sloped sun hood visor (Schute) over each lens
        b.rbox([0.0, ay + 2.1, 5.6], [4.6, 0.65, 4.2], shield_m);
        for sx in [-2.0_f32, 2.0] {
            b.rbox([sx, ay + 1.0, 5.3], [0.55, 2.3, 3.6], shield_m);
        }
    }
    let sh1_glow = Mat::emissive(0xfef9c3, 0.85);
    for &(sx, sy) in &[(-2.8_f32, 65.2_f32), (2.4, 67.5)] {
        b.cyl_z([sx, sy, 4.32], 0.85, 0.85, 1.4, 12, sh1_glow);
    }
}

/// Builds a realistic 3D DB Turnout Lantern (Weichenlaterne Wn 1 / Wn 2) & optional digital decoder box.
/// - Placed at `origin` (`[lx, ly, lz]`) rotated by `yaw` (track piece rotation in radians).
/// - `track_rod_start` (`[tx, ly, tz]`) is the point on the switch tie where the steel actuator rod connects.
/// - `has_lantern_housing` renders the mechanical lantern & position sign for every turnout so switch state is visible.
/// - `lantern_lit` switches the aspect between matte painted enamel (unlit, 0 mA) and warm glowing milk-glass (lit, +mA).
/// - `active_branch` selects straight (0) vs. diverging (1, 2) Wn 1 / Wn 2 aspect symbols.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_turnout_accessories_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    track_rod_start: [f32; 3],
    yaw: f32,
    kind_str: &str,
    has_lantern_housing: bool,
    lantern_lit: bool,
    active_branch: usize,
    has_decoder: bool,
) {
    let is_left = matches!(kind_str, "turnout-left" | "curved-turnout-left");
    let is_right = matches!(kind_str, "turnout-right" | "curved-turnout-right");
    let side_sign = if is_left { -1.0_f32 } else { 1.0_f32 };

    let mut b = ModelBuilder::new(verts, idxs, origin, yaw, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]);
    let rod_steel = Mat::metal(0x64748b, 0.38, 0.82);
    let base_cast = Mat::metal(0x1e293b, 0.65, 0.45);
    let housing_black = Mat::matte(0x0b1120, 0.55);
    let frame_trim = Mat::metal(0x334155, 0.48, 0.60);
    let aspect_mat = if lantern_lit {
        Mat::emissive(0xfef9c3, 1.15)
    } else {
        Mat::matte(0xdbe3ee, 0.52)
    };
    let dec_body = Mat::metal(0x0f172a, 0.42, 0.50);
    let dec_rib = Mat::metal(0x1e293b, 0.50, 0.45);
    let led_blue = Mat::emissive(0x38bdf8, 1.20);

    // 1. Cylindrical steel switch actuator rod (Stellstange) from track blades to lantern drive box
    let dx = track_rod_start[0] - origin[0];
    let dz = track_rod_start[2] - origin[2];
    let (local_tx, local_tz) = rot_xz(dx, dz, -yaw);
    b.cyl_between([local_tx, 3.2, local_tz], [0.0, 3.2, 0.0], 0.65, 0.65, 12, rod_steel);
    // Point-machine drive plate (Weichenantriebskasten) beside the sleepers
    b.rbox([0.0, 1.4, -side_sign * 3.2], [5.8, 2.4, 7.2], base_cast);

    if has_lantern_housing {
        // 2. Stepped circular cast-iron pedestal & vertical spindle shaft
        b.cyl_y([0.0, 1.5, 0.0], 3.4, 4.0, 3.0, 18, base_cast);
        b.cyl_y([0.0, 3.5, 0.0], 1.2, 1.5, 1.6, 14, frame_trim);

        // 3. Black DB Weichenlaterne cube housing (centered at y = 6.8, size 5.5 x 6.0 x 5.5)
        let hc_y = 6.8_f32;
        b.rbox([0.0, hc_y, 0.0], [5.5, 6.0, 5.5], housing_black);
        // Upper and lower metallic frame collars on housing
        b.rbox([0.0, hc_y - 2.85, 0.0], [5.8, 0.45, 5.8], frame_trim);
        if lantern_lit {
            b.rbox([0.0, hc_y + 2.55, 0.0], [5.65, 0.35, 5.65], Mat::emissive(0xfde68a, 0.55));
        }
        b.rbox([0.0, hc_y + 2.85, 0.0], [5.8, 0.45, 5.8], frame_trim);

        // 4. Authentic DB Wn 1 / Wn 2 symbols on all 4 faces reflecting active turnout branch:
        //    Face +X (heel / Ferse), Face -X (toe / Spitze), Face +Z (right side), Face -Z (left side)
        //    0 = Wn 1 vertical bar (Geradeaus), 1 = Wn 2b round disc (Fersenansicht),
        //    2 = Wn 2a diagonal bar sloping left (\), 3 = Wn 2a diagonal bar sloping right (/)
        let (sym_pos_x, sym_neg_x, sym_pos_z, sym_neg_z, dir_yaw) = if kind_str == "double-slip" {
            if active_branch % 2 == 0 {
                (0u8, 0u8, 3u8, 2u8, 0.0_f32)
            } else {
                (1u8, 3u8, 0u8, 0u8, -std::f32::consts::FRAC_PI_4)
            }
        } else if is_left {
            if active_branch == 0 {
                (0u8, 0u8, 1u8, 2u8, 0.0_f32)
            } else {
                (1u8, 2u8, 0u8, 0u8, std::f32::consts::FRAC_PI_4)
            }
        } else if is_right {
            if active_branch == 0 {
                (0u8, 0u8, 1u8, 3u8, 0.0_f32)
            } else {
                (1u8, 3u8, 0u8, 0u8, -std::f32::consts::FRAC_PI_4)
            }
        } else if kind_str == "three-way" {
            match active_branch {
                0 => (0u8, 0u8, 3u8, 2u8, 0.0_f32),
                1 => (1u8, 2u8, 0u8, 0u8, std::f32::consts::FRAC_PI_4),
                _ => (1u8, 3u8, 0u8, 0u8, -std::f32::consts::FRAC_PI_4),
            }
        } else if active_branch == 0 {
            (0u8, 0u8, 3u8, 2u8, 0.0_f32)
        } else {
            (2u8, 3u8, 0u8, 0u8, -std::f32::consts::FRAC_PI_4)
        };

        // Helper for X-facing lantern faces (+X at x = +2.82, -X at x = -2.82)
        for &(x_pos, sym, sign_x) in &[(2.82_f32, sym_pos_x, 1.0_f32), (-2.82, sym_neg_x, -1.0)] {
            // Recessed dark glass backing window
            b.rbox([x_pos - sign_x * 0.03, hc_y, 0.0], [0.10, 4.7, 4.7], frame_trim);
            match sym {
                0 => {
                    // Wn 1: Vertical bar
                    b.rbox([x_pos, hc_y, 0.0], [0.16, 4.1, 1.25], aspect_mat);
                }
                1 => {
                    // Wn 2b: Circular white disc (Fersenansicht)
                    b.cyl_x([x_pos, hc_y, 0.0], 1.65, 1.65, 0.18, 18, aspect_mat);
                }
                2 => {
                    // Wn 2a: Diagonal bar sloping to the left
                    let tilt = 0.72_f32 * sign_x;
                    b.obox([x_pos, hc_y, 0.0], [0.16, 4.2, 1.18], [tilt, 0.0, 0.0], aspect_mat);
                }
                _ => {
                    // Wn 2a: Diagonal bar sloping to the right
                    let tilt = -0.72_f32 * sign_x;
                    b.obox([x_pos, hc_y, 0.0], [0.16, 4.2, 1.18], [tilt, 0.0, 0.0], aspect_mat);
                }
            }
        }

        // Helper for Z-facing lantern faces (+Z at z = +2.82, -Z at z = -2.82)
        for &(z_pos, sym, sign_z) in &[(2.82_f32, sym_pos_z, 1.0_f32), (-2.82, sym_neg_z, -1.0)] {
            b.rbox([0.0, hc_y, z_pos - sign_z * 0.03], [4.7, 4.7, 0.10], frame_trim);
            match sym {
                0 => {
                    b.rbox([0.0, hc_y, z_pos], [1.25, 4.1, 0.16], aspect_mat);
                }
                1 => {
                    b.cyl_z([0.0, hc_y, z_pos], 1.65, 1.65, 0.18, 18, aspect_mat);
                }
                2 => {
                    let tilt = -0.72_f32 * sign_z;
                    b.obox([0.0, hc_y, z_pos], [1.18, 4.2, 0.16], [0.0, 0.0, tilt], aspect_mat);
                }
                _ => {
                    let tilt = 0.72_f32 * sign_z;
                    b.obox([0.0, hc_y, z_pos], [1.18, 4.2, 0.16], [0.0, 0.0, tilt], aspect_mat);
                }
            }
        }

        // 5. Pyramidal 4-sided lantern roof cap, top vent finial knob & overhead branch indicator bar
        b.cone_y([0.0, 10.6, 0.0], 4.4, 2.0, 4, std::f32::consts::FRAC_PI_4, housing_black);
        b.cyl_y([0.0, 11.7, 0.0], 0.55, 0.75, 0.7, 12, frame_trim);
        b.obox([0.0, 11.45, 0.0], [5.2, 0.55, 1.3], [0.0, dir_yaw, 0.0], aspect_mat);
    }

    if has_decoder {
        // 6. Trackside digital turnout decoder housing with cooling ribs, cable conduit & glowing blue status LED
        let dec_x = if has_lantern_housing { 11.0_f32 } else { 0.0 };
        b.cyl_between([0.0, 1.4, -side_sign * 2.0], [dec_x, 1.4, 0.0], 0.45, 0.45, 8, frame_trim);
        b.rbox([dec_x, 2.0, 0.0], [9.2, 3.2, 11.6], dec_body);
        for rz in [-3.6_f32, -1.2, 1.2, 3.6] {
            b.rbox([dec_x, 3.7, rz], [8.2, 0.35, 0.7], dec_rib);
        }
        b.cyl_y([dec_x + 2.4, 3.9, 0.0], 0.85, 0.85, 0.9, 14, led_blue);
    }
}

// ----------------------------------------------------------------------------
// 10. BUFFER STOP (Prellbock — open steel A-frame, rail clamps, oak buffer beam, twin round buffers, Sh2)
// ----------------------------------------------------------------------------
pub(crate) fn build_buffer_stop_with_signal(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot: f32,
    size: [f32; 3],
    illuminated: bool,
) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [25.0, 25.0, 35.0]);
    let steel = Mat::metal(0x334155, 0.52, 0.65);
    let dark_steel = Mat::metal(0x1e293b, 0.44, 0.76);
    let rust = Mat::matte(0x4b3832, 0.80);
    let wood = Mat::matte(0x6b3e1e, 0.88);
    let white_m = Mat::matte(0xf8fafc, 0.45);
    let sh2_red = if illuminated {
        Mat::emissive(0xef4444, 1.05)
    } else {
        Mat::emissive(0xdc2626, 0.35)
    };

    let rail_half = 8.25_f32;
    let beam_y = 12.6_f32;
    let beam_z = -8.0_f32;
    for side in [-1.0_f32, 1.0] {
        let rx = side * rail_half;
        b.rbox([rx, 0.9, 1.2], [3.2, 1.8, 25.5], dark_steel);
        for cz in [-10.0_f32, 2.0, 11.8] {
            b.rbox([rx, 1.1, cz], [4.2, 2.2, 2.4], rust);
        }
        b.strut_between([rx, 1.4, -10.5], [rx, beam_y + 1.2, beam_z + 0.5], 2.1, 2.3, steel);
        b.strut_between([rx, beam_y + 0.8, beam_z + 1.2], [rx, 1.4, 12.5], 2.1, 2.3, steel);
        b.strut_between([rx, 6.2, -8.8], [rx, 6.2, 4.2], 1.5, 1.6, dark_steel);
    }
    b.rbox([0.0, 2.2, 10.5], [rail_half * 2.0 + 2.4, 2.0, 2.2], dark_steel);
    b.rbox([0.0, beam_y, beam_z + 2.4], [rail_half * 2.0 + 4.0, 3.2, 2.4], steel);
    b.rbox([0.0, beam_y, beam_z - 0.5], [27.5, 4.8, 3.8], wood);
    for side in [-1.0_f32, 1.0] {
        b.rbox([side * 13.4, beam_y, beam_z - 0.5], [1.4, 5.2, 4.2], dark_steel);
        let px = side * 9.8;
        b.rbox([px, beam_y, beam_z - 2.6], [4.2, 4.2, 0.8], dark_steel);
        b.cyl_z([px, beam_y, beam_z - 4.2], 1.45, 1.85, 2.8, 10, steel);
        b.cyl_z([px, beam_y, beam_z - 5.8], 2.5, 2.5, 0.8, 14, dark_steel);
    }
    b.rbox([0.0, beam_y + 4.8, beam_z - 0.2], [1.3, 5.6, 1.3], dark_steel);
    b.rbox([0.0, beam_y + 8.6, beam_z - 0.3], [8.4, 6.2, 1.4], dark_steel);
    b.rbox([0.0, beam_y + 8.6, beam_z - 1.05], [7.8, 5.6, 0.35], white_m);
    b.rbox([0.0, beam_y + 8.6, beam_z - 1.25], [6.0, 3.8, 0.35], sh2_red);
}

pub(super) fn build_buffer_stop(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    build_buffer_stop_with_signal(verts, idxs, origin, rot, size, false);
}

// ----------------------------------------------------------------------------
// 11. PINE TREE (Fichte / Tanne — root flare, tapered trunk & 7 organic drooping whorls)
// ----------------------------------------------------------------------------
pub(super) fn build_pine_tree(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [55.0, 135.0, 55.0]);
    let bark = Mat::matte(0x3f2e21, 0.94);
    b.cyl_y([0.0, 5.0, 0.0], 5.5, 8.8, 10.0, 9, bark);
    b.cyl_y([0.0, 36.0, 0.0], 2.4, 5.6, 56.0, 9, bark);

    let tiers = [
        (22.0_f32, 28.5_f32, 34.0_f32, 0.0_f32, Mat::matte(0x134e2a, 0.95)),
        (39.0, 25.5, 32.0, 0.55, Mat::matte(0x14532d, 0.94)),
        (56.0, 22.0, 30.0, 1.15, Mat::matte(0x166534, 0.94)),
        (72.0, 18.5, 28.0, 0.30, Mat::matte(0x15803d, 0.93)),
        (88.0, 14.5, 25.0, 0.92, Mat::matte(0x166534, 0.92)),
        (103.0, 10.5, 22.0, 1.48, Mat::matte(0x16a34a, 0.91)),
        (117.0, 6.4, 21.0, 0.72, Mat::matte(0x22c55e, 0.90)),
    ];
    for (idx, (y, r, th, yaw, mat)) in tiers.into_iter().enumerate() {
        b.pine_whorl([0.0, y, 0.0], r, th, 11, yaw, 100 + idx as u32 * 53, mat);
    }
}

// ----------------------------------------------------------------------------
// 12. DECIDUOUS TREE (Laubbaum / Eiche — roots, forked limbs & 9 sculpted foliage clusters)
// ----------------------------------------------------------------------------
pub(super) fn build_deciduous_tree(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [80.0, 120.0, 80.0]);
    let bark = Mat::matte(0x4b3621, 0.93);
    let leaf_dark = Mat::matte(0x14532d, 0.92);
    let leaf_mid = Mat::matte(0x15803d, 0.90);
    let leaf_light = Mat::matte(0x22c55e, 0.88);
    let leaf_sun = Mat::matte(0x4ade80, 0.86);

    b.cyl_y([0.0, 5.5, 0.0], 7.2, 12.0, 11.0, 10, bark);
    b.cyl_y([0.0, 31.0, 0.0], 4.4, 7.4, 46.0, 10, bark);
    for &(tx, ty, tz) in &[
        (18.0_f32, 72.0_f32, 11.0_f32),
        (-18.0, 74.0, -10.0),
        (5.0, 76.0, -17.0),
        (-6.0, 78.0, 16.0),
    ] {
        b.cyl_between([0.0, 49.0, 0.0], [tx, ty, tz], 3.4, 1.8, 7, bark);
    }

    let clusters = [
        (0.0_f32, 82.0_f32, 0.0_f32, 28.0_f32, 23.0_f32, 27.0_f32, 11_u32, leaf_mid),
        (18.0, 73.0, 12.0, 22.0, 19.0, 22.0, 23, leaf_light),
        (-18.0, 75.0, -12.0, 22.0, 20.0, 21.0, 37, leaf_dark),
        (-15.0, 86.0, 14.0, 20.0, 18.0, 20.0, 49, leaf_light),
        (16.0, 85.0, -15.0, 19.0, 17.0, 19.0, 61, leaf_mid),
        (2.0, 99.0, 2.0, 20.0, 17.0, 19.0, 73, leaf_sun),
        (-22.0, 68.0, 6.0, 17.0, 15.0, 17.0, 83, leaf_dark),
        (21.0, 69.0, -9.0, 17.0, 15.0, 16.0, 97, leaf_mid),
        (0.0, 72.0, 19.0, 17.0, 15.0, 16.0, 109, leaf_light),
    ];
    for (cx, cy, cz, rx, ry, rz, seed, cm) in clusters {
        b.organic_foliage_cluster([cx, cy, cz], rx, ry, rz, seed, cm);
    }
}

// ----------------------------------------------------------------------------
// 13. PLATFORM LAMP (Bahnsteigleuchte — cast-iron plinth, tapered post, 180-deg swan-neck, shade & bulb)
// ----------------------------------------------------------------------------
pub(super) fn build_platform_lamp(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [20.0, 70.0, 20.0]);
    let iron = Mat::metal(0x1e293b, 0.36, 0.78);
    let shade = Mat::metal(0x334155, 0.44, 0.45);
    let reflector = Mat::matte(0xf8fafc, 0.40);
    let glow = Mat::emissive(0xfef08a, 1.35);

    b.rbox([0.0, 1.1, 0.0], [7.5, 2.2, 7.5], iron);
    b.cyl_y([0.0, 7.5, 0.0], 2.4, 3.6, 11.0, 10, iron);
    b.cyl_y([0.0, 13.5, 0.0], 3.0, 3.0, 1.6, 10, iron);
    b.cyl_y([0.0, 38.0, 0.0], 1.1, 1.8, 48.0, 10, iron);
    b.cyl_y([0.0, 61.5, 0.0], 1.9, 1.9, 1.4, 10, iron);

    let mut neck = Vec::with_capacity(17);
    for i in 0..=16 {
        let a = std::f32::consts::PI * (1.0 - i as f32 / 16.0);
        neck.push([7.8 + a.cos() * 7.8, 62.0 + a.sin() * 7.8, 0.0]);
    }
    b.curved_pipe(&neck, 0.95, 8, iron);
    b.cone_y([15.6, 59.6, 0.0], 5.6, 3.2, 16, 0.0, shade);
    b.cyl_y([15.6, 59.3, 0.0], 5.1, 5.1, 0.6, 16, reflector);
    b.sphere([15.6, 58.0, 0.0], 2.2, 8, 12, glow);
}

// ----------------------------------------------------------------------------
// 14. PLATFORM (Bahnsteig — sloped ramps, tapered curbs, open stairs, butterfly canopy, clock, ZZA)
// ----------------------------------------------------------------------------
pub(super) fn build_platform(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [320.0, 55.0, 54.0]);
    let deck = Mat::matte(0x94a3b8, 0.88);
    let curb = Mat::matte(0x475569, 0.90);
    let safety_w = Mat::matte(0xf8fafc, 0.50);
    let warn_y = Mat::matte(0xeab308, 0.60);
    let steel = Mat::metal(0x1e293b, 0.44, 0.68);
    let sheet = Mat::metal(0x334155, 0.54, 0.42);
    let glass = Mat::glass(0x7dd3fc);
    let rail_glass = Mat::glass(0x38bdf8);
    let chrome = Mat::metal(0x94a3b8, 0.25, 0.85);
    let wood = Mat::matte(0xb45309, 0.76);
    let db_blue = Mat::metal(0x0369a1, 0.48, 0.18);
    let zza_glow = Mat::emissive(0x38bdf8, 0.85);
    let sos_red = Mat::matte(0xdc2626, 0.50);
    let lamp_glow = Mat::emissive(0xfef08a, 1.10);
    let dark_pit = Mat::matte(0x0f172a, 0.95);

    // 1. Concrete platform deck split around the central 34x18 stairwell + sloped end ramps
    b.rbox([-78.5, 5.0, 0.0], [123.0, 10.0, 52.0], deck);
    b.rbox([78.5, 5.0, 0.0], [123.0, 10.0, 52.0], deck);
    b.rbox([0.0, 5.0, -17.5], [34.0, 10.0, 17.0], deck);
    b.rbox([0.0, 5.0, 17.5], [34.0, 10.0, 17.0], deck);

    // Left ramp wedge
    for (z0, z1, nz) in [(-26.0_f32, -26.0, -1.0_f32), (26.0, 26.0, 1.0)] {
        b.quad_oriented([-160.0, 0.0, z0], [-140.0, 0.0, z0], [-140.0, 10.0, z1], [-160.0, 1.2, z1], [0.0, 0.0, nz], deck);
    }
    b.quad_oriented([-160.0, 1.2, -26.0], [-140.0, 10.0, -26.0], [-140.0, 10.0, 26.0], [-160.0, 1.2, 26.0], [-0.4, 0.9, 0.0], deck);
    // Right ramp wedge
    for (z0, z1, nz) in [(-26.0_f32, -26.0, -1.0_f32), (26.0, 26.0, 1.0)] {
        b.quad_oriented([140.0, 0.0, z0], [160.0, 0.0, z0], [160.0, 1.2, z1], [140.0, 10.0, z1], [0.0, 0.0, nz], deck);
    }
    b.quad_oriented([140.0, 10.0, -26.0], [160.0, 1.2, -26.0], [160.0, 1.2, 26.0], [140.0, 10.0, 26.0], [0.4, 0.9, 0.0], deck);

    for rx in [-139.5_f32, 139.5] {
        b.rbox([rx, 10.2, 0.0], [2.0, 0.4, 52.0], warn_y);
    }
    // Flat deck curbs (-140..+140) + tapered curb wedges along both sloped ramps
    for z_side in [-26.2_f32, 26.2] {
        b.rbox([0.0, 5.15, z_side], [280.0, 10.3, 2.0], curb);
        let z0 = z_side - 1.0;
        let z1 = z_side + 1.0;
        // Left tapered curb
        b.quad_oriented([-160.0, 0.0, z0], [-140.0, 0.0, z0], [-140.0, 10.3, z0], [-160.0, 1.4, z0], [0.0, 0.0, -1.0], curb);
        b.quad_oriented([-160.0, 0.0, z1], [-140.0, 0.0, z1], [-140.0, 10.3, z1], [-160.0, 1.4, z1], [0.0, 0.0, 1.0], curb);
        b.quad_oriented([-160.0, 1.4, z0], [-140.0, 10.3, z0], [-140.0, 10.3, z1], [-160.0, 1.4, z1], [-0.4, 0.9, 0.0], curb);
        b.quad_oriented([-160.0, 0.0, z0], [-160.0, 1.4, z0], [-160.0, 1.4, z1], [-160.0, 0.0, z1], [-1.0, 0.0, 0.0], curb);
        // Right tapered curb
        b.quad_oriented([140.0, 0.0, z0], [160.0, 0.0, z0], [160.0, 1.4, z0], [140.0, 10.3, z0], [0.0, 0.0, -1.0], curb);
        b.quad_oriented([140.0, 0.0, z1], [160.0, 0.0, z1], [160.0, 1.4, z1], [140.0, 10.3, z1], [0.0, 0.0, 1.0], curb);
        b.quad_oriented([140.0, 10.3, z0], [160.0, 1.4, z0], [160.0, 1.4, z1], [140.0, 10.3, z1], [0.4, 0.9, 0.0], curb);
        b.quad_oriented([160.0, 0.0, z0], [160.0, 1.4, z0], [160.0, 1.4, z1], [160.0, 0.0, z1], [1.0, 0.0, 0.0], curb);
    }
    // White tactile safety lines strictly on flat deck (-138..+138)
    for z_side in [-22.5_f32, 22.5] {
        b.rbox([0.0, 10.22, z_side], [276.0, 0.4, 1.8], safety_w);
    }

    // 2. Open underpass stairwell + solid descending steps + chrome railings & glass balustrades
    b.rbox([0.0, 0.5, 0.0], [34.0, 1.0, 18.0], dark_pit);
    b.rbox([-16.75, 5.1, 0.0], [0.5, 9.6, 18.0], dark_pit);
    for z_wall in [-8.75_f32, 8.75] {
        b.rbox([0.0, 5.1, z_wall], [34.0, 9.6, 0.5], dark_pit);
    }
    for s in 0..7 {
        let sx = 13.0 - s as f32 * 3.8;
        let step_top = 9.8 - s as f32 * 1.15;
        let step_h = step_top - 1.0;
        b.rbox([sx, 1.0 + step_h * 0.5, 0.0], [3.8, step_h, 16.4], curb);
    }
    for &(px, pz) in &[
        (-17.0_f32, -9.0_f32), (-6.0, -9.0), (6.0, -9.0), (17.0, -9.0),
        (-17.0, 9.0), (-6.0, 9.0), (6.0, 9.0), (17.0, 9.0), (-17.0, 0.0),
    ] {
        b.cyl_y([px, 15.0, pz], 0.5, 0.5, 10.0, 8, chrome);
    }
    for pz in [-9.0_f32, 9.0] {
        b.rbox([0.0, 20.0, pz], [34.0, 0.8, 0.8], chrome);
        b.rbox([0.0, 14.5, pz], [32.0, 8.0, 0.4], rail_glass);
    }
    b.rbox([-17.0, 20.0, 0.0], [0.8, 0.8, 18.0], chrome);
    b.rbox([-17.0, 14.5, 0.0], [0.4, 8.0, 16.0], rail_glass);

    // 3. Steel-and-glass V butterfly canopy (Y-arms + cylindrical struts)
    let (can_len, roof_y, slope_ang) = (210.0_f32, 40.0_f32, 0.12_f32);
    let slope_tan = slope_ang.tan();
    let y_ridge = roof_y + 0.6;
    for px in [-80.0_f32, -32.0, 32.0, 80.0] {
        b.rbox([px, 10.75, 0.0], [5.0, 1.5, 5.0], steel);
        b.rbox([px, 25.0, 0.0], [3.5, 30.0, 3.5], steel);
        for z_dir in [-1.0_f32, 1.0] {
            let arm_len = 24.0 / slope_ang.cos();
            let arm_y = (roof_y - 0.8) + 12.0 * slope_tan;
            b.obox([px, arm_y, z_dir * 12.0], [2.5, 2.2, arm_len], [-z_dir * slope_ang, 0.0, 0.0], steel);
            b.cyl_between([px, 31.0, 0.0], [px, roof_y - 0.8 + 11.0 * slope_tan, z_dir * 11.0], 0.85, 0.85, 8, steel);
        }
    }
    b.rbox([0.0, roof_y, 0.0], [can_len, 3.0, 3.2], steel);
    for z_side in [-24.2_f32, 24.2] {
        b.rbox([0.0, y_ridge + 24.2 * slope_tan - 0.6, z_side], [can_len, 2.6, 2.0], steel);
    }
    for z_dir in [-1.0_f32, 1.0] {
        b.obox([0.0, y_ridge + 5.0 * slope_tan, z_dir * 5.0], [can_len - 4.0, 1.2, 7.4], [-z_dir * slope_ang, 0.0, 0.0], glass);
        b.obox([0.0, y_ridge + 16.5 * slope_tan, z_dir * 16.5], [can_len, 1.5, 16.6], [-z_dir * slope_ang, 0.0, 0.0], sheet);
    }
    for x_end in [-can_len * 0.5, can_len * 0.5] {
        for z_dir in [-1.0_f32, 1.0] {
            b.obox([x_end, y_ridge + 12.25 * slope_tan - 0.3, z_dir * 12.25], [1.5, 2.8, 25.0], [-z_dir * slope_ang, 0.0, 0.0], steel);
        }
    }

    // 4. Hanging ZZA boards, station sign, clear double-sided clock at X=22.5, benches, SOS pillar & end lamps
    for zx in [-45.0_f32, 45.0] {
        b.rbox([zx, roof_y - 5.0, 0.0], [18.0, 6.5, 7.0], db_blue);
        for hx in [-6.0_f32, 6.0] {
            b.cyl_y([zx + hx, roof_y - 1.0, 0.0], 0.4, 0.4, 4.0, 6, steel);
        }
        for z_dir in [-1.0_f32, 1.0] {
            b.rbox([zx, roof_y - 5.0, z_dir * 3.6], [16.0, 4.5, 0.5], zza_glow);
        }
    }
    b.rbox([0.0, roof_y - 5.5, 0.0], [32.0, 5.5, 1.6], db_blue);
    b.rbox([0.0, roof_y - 5.5, 0.0], [28.0, 3.2, 2.0], safety_w);
    b.cyl_y([-11.0, roof_y - 1.5, 0.0], 0.4, 0.4, 4.5, 6, steel);
    b.cyl_y([11.0, roof_y - 1.5, 0.0], 0.4, 0.4, 4.5, 6, steel);
    b.cyl_y([22.5, roof_y - 1.0, 0.0], 0.4, 0.4, 3.5, 6, steel);
    for side in [-1.0_f32, 1.0] {
        b.clock_face_x([22.5 + side * 0.2, roof_y - 4.5, 0.0], side, 3.5, steel, safety_w, steel);
    }
    for bx in [-58.0_f32, 58.0] {
        for fx in [-7.0_f32, 7.0] {
            b.rbox([bx + fx, 12.25, 0.0], [1.2, 4.5, 7.5], steel);
        }
        b.rbox([bx, 14.5, 0.0], [18.0, 0.8, 7.5], wood);
        b.rbox([bx, 17.0, 0.0], [18.0, 3.5, 0.8], wood);
    }
    b.rbox([-32.0, 16.0, 0.0], [1.2, 12.0, 14.0], steel);
    b.rbox([-32.0, 16.0, 0.0], [1.6, 9.5, 11.0], warn_y);
    b.rbox([32.0, 17.0, 4.0], [2.5, 14.0, 2.5], sos_red);
    b.rbox([32.0, 23.5, 4.0], [3.2, 2.5, 3.2], safety_w);
    for px in [-80.0_f32, 80.0] {
        b.cyl_y([px, 14.5, 3.5], 1.5, 1.3, 4.0, 10, chrome);
    }
    for lx in [-132.0_f32, 132.0] {
        b.cyl_y([lx, 25.0, 0.0], 1.0, 1.4, 30.0, 8, steel);
        b.rbox([lx, 40.5, 0.0], [8.0, 1.2, 4.0], steel);
        b.rbox([lx, 39.8, 0.0], [6.5, 0.7, 2.8], lamp_glow);
    }
}

// ----------------------------------------------------------------------------
// 15. SKYSCRAPER (Wolkenkratzer — colonnade lobby, glazed curtain wall + mullions, 4-sided terrace, spire)
// ----------------------------------------------------------------------------
pub(super) fn build_skyscraper(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [130.0, 280.0, 110.0]);
    let plaza = Mat::matte(0x334155, 0.82);
    let curb = Mat::matte(0x64748b, 0.74);
    let bush = Mat::matte(0x15803d, 0.90);
    let steel = Mat::metal(0x1e293b, 0.38, 0.72);
    let alum = Mat::metal(0x94a3b8, 0.32, 0.70);
    let glass = Mat::warm_window(0xfde68a, 0.42);
    let warm_office = Mat::warm_window(0xfde68a, 0.95);
    let dark_glass = Mat::warm_window(0xbae6fd, 0.36);
    let roof = Mat::matte(0x1e293b, 0.75);
    let beacon = Mat::emissive(0xef4444, 1.30);
    let sign_glow = Mat::emissive(0xfef08a, 1.00);

    // 1. Plaza, non-coplanar stepped entrance & ornamental planter bushes
    b.rbox([0.0, 1.8, 0.0], [130.0, 3.6, 110.0], plaza);
    b.rbox([0.0, 3.9, 50.0], [60.0, 0.6, 10.0], curb);
    b.rbox([0.0, 4.1, 44.5], [54.0, 1.0, 7.0], curb);
    for px in [-42.0_f32, 42.0] {
        b.rbox([px, 5.0, 45.0], [14.0, 5.5, 14.0], curb);
        b.organic_foliage_cluster([px, 11.5, 45.0], 6.4, 5.8, 6.4, (px as i32 + 99) as u32, bush);
    }

    // 2. Lobby podium with open central portal colonnade & lit entrance awning
    let lobby_h = 24.4_f32;
    b.rbox([0.0, 3.6 + lobby_h * 0.5, 0.0], [102.0, lobby_h, 82.0], warm_office);
    b.rbox([0.0, 22.0, 47.0], [50.0, 2.4, 18.0], steel);
    b.rbox([0.0, 24.8, 48.0], [36.0, 3.6, 1.5], sign_glow);
    for cx in [-49.0_f32, -30.0, -14.0, 14.0, 30.0, 49.0] {
        b.rbox([cx, 3.6 + lobby_h * 0.5, 41.0], [3.6, lobby_h, 3.6], steel);
    }
    b.rbox([0.0, 3.6 + lobby_h + 1.5, 0.0], [106.0, 3.0, 86.0], steel);

    // 3. Main curtain-wall shaft + 10 spandrel bands + perimeter mullions + lit office window accents
    let t1_h = 136.0_f32;
    let t1_base = 31.0_f32;
    let t1_y = t1_base + t1_h * 0.5;
    b.rbox([0.0, t1_y, 0.0], [98.0, t1_h, 78.0], glass);
    for f in 1..=10 {
        let fy = t1_base + (t1_h * f as f32) / 11.0;
        b.rbox([0.0, fy, 0.0], [99.4, 2.1, 79.4], alum);
        if f % 2 == 1 {
            b.rbox([((f as f32 - 5.5) * 6.0).clamp(-32.0, 32.0), fy - 5.5, 39.4], [24.0, 7.2, 0.8], warm_office);
            b.rbox([((5.5 - f as f32) * 5.0).clamp(-30.0, 30.0), fy - 5.5, -39.4], [22.0, 7.2, 0.8], warm_office);
        }
    }
    for mx in (-44..=44).step_by(14) {
        b.rbox([mx as f32, t1_y, 39.4], [1.5, t1_h, 1.8], steel);
        b.rbox([mx as f32, t1_y, -39.4], [1.5, t1_h, 1.8], steel);
    }
    for mz in (-32..=32).step_by(16) {
        b.rbox([-49.4, t1_y, mz as f32], [1.8, t1_h, 1.5], steel);
        b.rbox([49.4, t1_y, mz as f32], [1.8, t1_h, 1.5], steel);
    }

    // 4. Setback upper tower with observation terrace & 4-sided perimeter safety rail
    let setback_y = t1_base + t1_h;
    b.rbox([0.0, setback_y + 1.5, 0.0], [102.0, 3.0, 82.0], steel);
    for sx in [-49.0_f32, 49.0] {
        b.rbox([sx, setback_y + 5.0, 0.0], [1.6, 5.2, 80.0], alum);
    }
    for sz in [-39.5_f32, 39.5] {
        b.rbox([0.0, setback_y + 5.0, sz], [99.6, 5.2, 1.6], alum);
    }
    let t2_h = 62.0_f32;
    let t2_y = setback_y + 3.0 + t2_h * 0.5;
    b.rbox([0.0, t2_y, 0.0], [76.0, t2_h, 60.0], dark_glass);
    for f in 1..=5 {
        b.rbox([0.0, setback_y + 3.0 + (t2_h * f as f32) / 6.0, 0.0], [77.2, 1.9, 61.2], alum);
    }
    for mx in [-30.0_f32, -10.0, 10.0, 30.0] {
        b.rbox([mx, t2_y, 30.3], [1.4, t2_h, 1.5], steel);
        b.rbox([mx, t2_y, -30.3], [1.4, t2_h, 1.5], steel);
    }

    // 5. Roof mechanical penthouse, HVAC units, tapered cylindrical antenna spire & aviation beacon
    let roof_y = setback_y + 3.0 + t2_h;
    b.rbox([0.0, roof_y + 1.5, 0.0], [78.0, 3.0, 62.0], steel);
    b.rbox([-7.0, roof_y + 9.5, 0.0], [34.0, 14.0, 28.0], roof);
    for acz in [-9.0_f32, 9.0] {
        b.rbox([20.0, roof_y + 5.8, acz], [14.0, 7.2, 10.0], curb);
    }
    b.cyl_y([-7.0, roof_y + 31.0, 0.0], 1.1, 2.3, 30.0, 10, steel);
    b.cyl_y([-7.0, roof_y + 52.0, 0.0], 0.4, 1.0, 13.0, 8, alum);
    b.sphere([-7.0, roof_y + 59.0, 0.0], 2.0, 8, 10, beacon);
}

// ----------------------------------------------------------------------------
// 16. INDUSTRY BUILDING (Industriehalle — sawtooth roofs, dock & ramp, roll doors, cylindrical silo & stack)
// ----------------------------------------------------------------------------
pub(super) fn build_industry_building(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [260.0, 110.0, 160.0]);
    let ground = Mat::matte(0x475569, 0.90);
    let ramp = Mat::matte(0x64748b, 0.85);
    let brick = Mat::matte(0x78350f, 0.87);
    let metal_wall = Mat::metal(0x52525b, 0.56, 0.38);
    let trim = Mat::metal(0x334155, 0.48, 0.55);
    let glass = Mat::glass(0x38bdf8);
    let win_warm = Mat::warm_window(0xfde68a, 0.72);
    let gate_blue = Mat::metal(0x0369a1, 0.40, 0.50);
    let silo_m = Mat::metal(0x94a3b8, 0.28, 0.82);
    let pipe_m = Mat::metal(0x1e293b, 0.36, 0.76);
    let hazard = Mat::matte(0xeab308, 0.52);
    let chim_m = Mat::matte(0x28211e, 0.76);

    // 1. Concrete yard apron, flush loading dock, sloped forklift wedge & dock bumpers
    b.rbox([0.0, 2.0, 0.0], [260.0, 4.0, 160.0], ground);
    b.rbox([-35.0, 9.0, 66.0], [126.0, 10.0, 22.0], ramp);
    for (z0, nz) in [(55.0_f32, -1.0_f32), (77.0, 1.0)] {
        b.tri_oriented([28.0, 4.0, z0], [50.0, 4.0, z0], [28.0, 14.0, z0], [0.0, 0.0, nz], ramp);
    }
    b.quad_oriented([28.0, 14.0, 55.0], [50.0, 4.0, 55.0], [50.0, 4.0, 77.0], [28.0, 14.0, 77.0], [0.4, 0.9, 0.0], ramp);
    for bx in [-82.0_f32, -59.0, -36.0, -13.0, 10.0] {
        b.rbox([bx, 10.0, 78.5], [5.5, 5.5, 3.0], pipe_m);
    }

    // 2. Two-tone hall body (brick lower + steel panel upper + pilasters + loading canopy & sectional doors)
    let (hall_w, hall_d, wall_h, hall_cx) = (182.0_f32, 110.0_f32, 52.0_f32, -20.0_f32);
    b.rbox([hall_cx, 12.0, 0.0], [hall_w, 16.0, hall_d], brick);
    b.rbox([hall_cx, 20.0 + (wall_h - 16.0) * 0.5, 0.0], [hall_w, wall_h - 16.0, hall_d], metal_wall);
    for k in 0..=5 {
        let cx = hall_cx - hall_w * 0.5 + (k as f32) * (hall_w / 5.0);
        b.rbox([cx, 4.0 + wall_h * 0.5, hall_d * 0.5 + 0.3], [3.0, wall_h, 3.4], trim);
    }
    b.rbox([-35.0, 40.0, 66.2], [134.0, 2.4, 21.6], trim);
    b.rbox([-35.0, 39.4, 77.3], [134.0, 1.2, 0.6], hazard);
    for gx in [-62.0_f32, -10.0] {
        b.rbox([gx, 26.5, 55.9], [30.0, 25.0, 1.6], trim);
        b.rbox([gx, 26.0, 56.9], [26.0, 23.0, 0.8], gate_blue);
    }
    b.rbox([hall_cx, 20.0, -55.7], [26.0, 32.0, 1.2], gate_blue);
    for fz in [-32.0_f32, 0.0, 32.0] {
        b.rbox([hall_cx - hall_w * 0.5 - 0.7, 35.0, fz], [1.2, 13.0, 19.0], win_warm);
    }

    // 3. True asymmetrical sawtooth (shed) roofs with north-lit skylight glass bays
    let z_step = hall_d / 3.0;
    for i in 0..3 {
        let z0 = -hall_d * 0.5 + i as f32 * z_step;
        let z1 = z0 + z_step;
        b.sawtooth_roof_bay(
            hall_cx - hall_w * 0.5,
            hall_cx + hall_w * 0.5,
            z0,
            z1,
            4.0 + wall_h,
            21.0,
            trim,
            glass,
            pipe_m,
        );
    }

    // 4. Cylindrical brushed-steel silo (+X = 98) with hopper, cone roof, ladder & overhead pipe
    let (sx, sz, sr, sh) = (98.0_f32, 4.0_f32, 17.5_f32, 76.0_f32);
    b.cyl_y([sx, 16.0 + sh * 0.5, sz], sr, sr, sh, 32, silo_m);
    b.cone_y([sx, 16.0 + sh, sz], sr + 1.5, 11.0, 32, 0.0, silo_m);
    b.cyl_y([sx, 10.0, sz], sr, 6.0, 12.0, 24, trim);
    for k in 0..4 {
        let a = (k as f32) * std::f32::consts::FRAC_PI_2;
        b.rbox([sx + (sr - 2.0) * a.cos(), 13.0, sz + (sr - 2.0) * a.sin()], [2.8, 18.0, 2.8], pipe_m);
    }
    b.rbox([sx, 48.0, sz + sr + 1.2], [3.6, 76.0, 1.5], pipe_m);
    b.cyl_x([sx - 20.0, 48.0, sz], 2.4, 2.4, 28.0, 16, pipe_m);

    // 5. Tall tapered brick/masonry factory chimney (+X = 82, -Z = 42)
    b.cyl_y([82.0, 56.0, -42.0], 4.8, 7.2, 100.0, 24, chim_m);
    for by in [36.0_f32, 62.0, 88.0] {
        let r = 7.2 - ((by - 6.0) / 100.0) * 2.4;
        b.cyl_y([82.0, by, -42.0], r + 0.35, r + 0.35, 1.8, 24, pipe_m);
    }
    b.cone_y([82.0, 106.0, -42.0], 9.2, 5.2, 24, 0.0, pipe_m);
}

// ----------------------------------------------------------------------------
// GENERIC FALLBACK BUILDING (architectural plinth, plaster, framed windows, overhang roof)
// ----------------------------------------------------------------------------
pub(super) fn build_generic_building(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [80.0, 60.0, 60.0]);
    let stone = Mat::matte(0x52525b, 0.88);
    let wall = Mat::matte(0xdcd3c4, 0.80);
    let roof = Mat::matte(0x7c2d12, 0.70);
    let cornice = Mat::matte(0xcbd5e1, 0.75);
    let frame = Mat::matte(0xf8fafc, 0.65);
    let win = Mat::warm_window(0xfde68a, 0.72);
    let door = Mat::matte(0x5c2e14, 0.78);

    b.rbox([0.0, 2.5, 0.0], [78.0, 5.0, 58.0], stone);
    b.rbox([0.0, 23.0, 0.0], [74.0, 36.0, 54.0], wall);
    b.rbox([0.0, 41.5, 0.0], [76.0, 1.8, 56.0], cornice);
    b.gable_roof_x([0.0, 42.4, 0.0], 74.0, 54.0, 16.0, 2.4, 1.8, roof, wall, roof);
    b.rbox([0.0, 15.0, 27.2], [11.0, 20.0, 1.2], door);
    for side in [-1.0_f32, 1.0] {
        for dx in [-22.0_f32, 22.0] {
            b.window_z(dx, 26.0, side * 27.0, side, 9.0, 12.0, win, frame, cornice, stone);
        }
    }
    let _ = MAT_DEFAULT;
}

