use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn push_extruded_polygon_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    outer: &[Vec2],
    holes: &[Vec<Vec2>],
    y_top: f32,
    thickness: f32,
    top_col: [f32; 3],
    side_col: [f32; 3],
) {
    let tris = board_tools::triangulate_polygon_with_holes(outer, holes);
    let y_bot = y_top - thickness;
    for tri in &tris {
        let base_t = verts.len() as u32;
        for pt in tri {
            verts.push(Vertex3D::new(
                [pt.x as f32, y_top, pt.y as f32],
                [0.0, 1.0, 0.0],
                top_col,
                MAT_DEFAULT,
            ));
        }
        idxs.extend_from_slice(&[base_t, base_t + 1, base_t + 2]);

        let base_b = verts.len() as u32;
        for pt in tri.iter().rev() {
            verts.push(Vertex3D::new(
                [pt.x as f32, y_bot, pt.y as f32],
                [0.0, -1.0, 0.0],
                side_col,
                MAT_DEFAULT,
            ));
        }
        idxs.extend_from_slice(&[base_b, base_b + 1, base_b + 2]);
    }

    let push_wall_ring = |verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, ring: &[Vec2], flip_norm: bool| {
        if ring.len() < 3 {
            return;
        }
        let area_sign = if board_tools::signed_polygon_area(ring) >= 0.0 { 1.0_f32 } else { -1.0 };
        let sign = if flip_norm { -area_sign } else { area_sign };
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let dx = (b.x - a.x) as f32;
            let dz = (b.y - a.y) as f32;
            let len = dx.hypot(dz);
            if len < 1e-4 {
                continue;
            }
            let n = normalize3([sign * (dz / len), 0.0, sign * (-dx / len)]);
            push_quad(
                verts,
                idxs,
                [a.x as f32, y_bot, a.y as f32],
                [b.x as f32, y_bot, b.y as f32],
                [b.x as f32, y_top, b.y as f32],
                [a.x as f32, y_top, a.y as f32],
                n,
                side_col,
            );
        }
    };

    push_wall_ring(verts, idxs, outer, false);
    for hole in holes {
        push_wall_ring(verts, idxs, hole, true);
    }
}

pub(super) fn collect_turntable_holes(
    layout: &Layout,
    catalogs: &[Catalog],
    outer: &[Vec2],
    layer_elev: f64,
    layer_info: &std::collections::HashMap<&str, (f64, bool)>,
) -> Vec<Vec<Vec2>> {
    let mut holes = Vec::new();
    for p in &layout.pieces {
        let entry = find_entry(catalogs, &p.catalog_id);
        let kind = entry.map(|e| e.kind.as_str());
        let is_tt = kind == Some("turntable");
        let is_tf = kind == Some("transfer-table");
        if !is_tt && !is_tf && !(entry.is_none() && p.connectors.len() >= 6) {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let (lh, vis) = layer_info.get(lid).copied().unwrap_or((0.0, true));
        if !vis {
            continue;
        }
        let piece_elev = lh + p.elevation.map(|e| e.start).unwrap_or(0.0);
        if (piece_elev - layer_elev).abs() > 18.0 {
            continue;
        }

        if is_tf {
            if let Some(e) = entry {
                let corners = crate::turntables_3d::transfer_table_world_rect_corners(p, e, 1.5, 1.5);
                let center = Vec2 {
                    x: 0.25 * (corners[0].x + corners[1].x + corners[2].x + corners[3].x),
                    y: 0.25 * (corners[0].y + corners[1].y + corners[2].y + corners[3].y),
                };
                if board_tools::is_point_in_polygon(center, outer) {
                    holes.push(corners.to_vec());
                }
            }
            continue;
        }

        // Circular turntable pit cutout: sized strictly inside coping & ballast collar (`142 mm` for `189 mm` outer radius)
        let c = Vec2 { x: p.transform.x, y: p.transform.y };
        if !board_tools::is_point_in_polygon(c, outer) {
            continue;
        }
        let outer_r = entry
            .and_then(|e| e.radius.or_else(|| e.length.map(|l| l * 0.5)))
            .unwrap_or(189.0);
        let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
        let r = 142.0 * scale_f;
        let segs = 36usize;
        let mut ring = Vec::with_capacity(segs);
        for k in 0..segs {
            let a = (k as f64 / segs as f64) * std::f64::consts::TAU;
            ring.push(Vec2 { x: c.x + r * a.cos(), y: c.y + r * a.sin() });
        }
        holes.push(ring);
    }
    holes
}

pub(super) fn push_env_pine(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, x: f32, floor_y: f32, z: f32, h: f32) {
    crate::models3d_mesh::push_env_pine_3d(verts, idxs, x, floor_y, z, h);
}

pub(super) fn push_env_deciduous_tree(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, x: f32, y0: f32, z: f32, h: f32, seed: usize) {
    // 1:1 broadleaf tree: slender trunk (~0.5-0.8 m) and a wide, rounded crown.
    let trunk_h = h * 0.44;
    let trunk_w = (h * 0.048).max(280.0);
    let bark = [0.34, 0.24, 0.16];
    push_box(verts, idxs, [x, y0 + trunk_h * 0.5, z], [trunk_w, trunk_h, trunk_w], bark);

    let tint = ((seed % 5) as f32 - 2.0) * 0.025;
    let c_main = [
        (0.16 + tint).clamp(0.08, 0.35),
        (0.38 + tint * 1.4).clamp(0.22, 0.52),
        (0.11 + tint * 0.4).clamp(0.06, 0.22),
    ];
    let c_hi = [c_main[0] + 0.05, c_main[1] + 0.08, c_main[2] + 0.03];
    let cw = h * 0.56;
    let ch = h * 0.38;
    let cy = y0 + trunk_h + ch * 0.35;
    push_rotated_box(verts, idxs, [x, cy, z], (seed as f32) * 0.65, [0.0, 0.0, 0.0], [cw, ch, cw * 0.92], c_main);
    push_rotated_box(verts, idxs, [x + cw * 0.18, cy + ch * 0.38, z - cw * 0.14], (seed as f32) * 0.4 + 0.5, [0.0, 0.0, 0.0], [cw * 0.68, ch * 0.72, cw * 0.66], c_hi);
    push_rotated_box(verts, idxs, [x - cw * 0.20, cy + ch * 0.18, z + cw * 0.16], (seed as f32) * 0.3 + 1.1, [0.0, 0.0, 0.0], [cw * 0.58, ch * 0.64, cw * 0.58], c_main);
}

#[allow(clippy::too_many_arguments)]
fn push_pitched_roof_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    cx: f32,
    eaves_y: f32,
    cz: f32,
    width_x: f32,
    length_z: f32,
    ridge_h: f32,
    rot_rad: f32,
    roof_col: [f32; 3],
    gable_col: [f32; 3],
) {
    let hw = width_x * 0.5;
    let hl = length_z * 0.5;
    let xf = |lx: f32, ly: f32, lz: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(lx, lz, rot_rad);
        [cx + rx, eaves_y + ly, cz + rz]
    };
    let rn = |nx: f32, ny: f32, nz: f32| -> [f32; 3] {
        let (rx, rz) = rot_xz(nx, nz, rot_rad);
        normalize3([rx, ny, rz])
    };
    let slope_len = hw.hypot(ridge_h).max(1.0);
    let n_l = rn(-ridge_h / slope_len, hw / slope_len, 0.0);
    let n_r = rn(ridge_h / slope_len, hw / slope_len, 0.0);
    push_quad(verts, idxs, xf(-hw, 0.0, -hl), xf(0.0, ridge_h, -hl), xf(0.0, ridge_h, hl), xf(-hw, 0.0, hl), n_l, roof_col);
    push_quad(verts, idxs, xf(0.0, ridge_h, -hl), xf(hw, 0.0, -hl), xf(hw, 0.0, hl), xf(0.0, ridge_h, hl), n_r, roof_col);
    push_tri(verts, idxs, xf(-hw, 0.0, -hl), xf(hw, 0.0, -hl), xf(0.0, ridge_h, -hl), rn(0.0, 0.0, -1.0), gable_col);
    push_tri(verts, idxs, xf(hw, 0.0, hl), xf(-hw, 0.0, hl), xf(0.0, ridge_h, hl), rn(0.0, 0.0, 1.0), gable_col);
}

/// 1:1 alpine chalet (typical footprint 10-13 m x 8-10 m, wall height ~6 m, local +Z = front gable).
pub(super) fn push_alpine_chalet(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    cx: f32,
    y0: f32,
    cz: f32,
    rot_rad: f32,
    w: f32,
    d: f32,
    wall_h: f32,
    roof_col: [f32; 3],
) {
    let plinth_h = wall_h * 0.16;
    let body_h = wall_h - plinth_h;
    let o = [cx, y0, cz];
    let front_z = d * 0.5 - 120.0;
    push_rotated_box(verts, idxs, o, rot_rad, [0.0, plinth_h * 0.5, 0.0], [w, plinth_h, d], [0.46, 0.44, 0.41]);
    push_rotated_box(verts, idxs, o, rot_rad, [0.0, plinth_h + body_h * 0.5, 0.0], [w - 240.0, body_h, d - 240.0], [0.90, 0.87, 0.80]);

    // Upper-storey timber balcony on the front gable (1.35 m deep, 1.05 m railing)
    let bal_y = plinth_h + body_h * 0.55;
    let bal_w = w * 0.84;
    push_rotated_box(verts, idxs, o, rot_rad, [0.0, bal_y, front_z + 675.0], [bal_w, 220.0, 1350.0], [0.42, 0.27, 0.15]);
    push_rotated_box(verts, idxs, o, rot_rad, [0.0, bal_y + 110.0 + 525.0, front_z + 1280.0], [bal_w, 1050.0, 140.0], [0.36, 0.22, 0.12]);
    for sx in [-1.0_f32, 1.0] {
        push_rotated_box(verts, idxs, o, rot_rad, [sx * bal_w * 0.5, bal_y + 110.0 + 525.0, front_z + 700.0], [140.0, 1050.0, 1200.0], [0.36, 0.22, 0.12]);
    }

    // Ground-floor entrance door (1.1 x 2.2 m)
    push_rotated_box(verts, idxs, o, rot_rad, [0.0, plinth_h + 1100.0, front_z + 30.0], [1100.0, 2200.0, 120.0], [0.30, 0.18, 0.09]);

    // Night-lit windows (~1.3 x 1.5 m), two storeys on both sides + front
    let win_col = [1.0, 0.86, 0.50];
    let win_mat = [0.22, 0.05, -0.95];
    for sx in [-1.0_f32, 1.0] {
        for dz in [-d * 0.22, d * 0.22] {
            for fy in [0.30_f32, 0.72] {
                push_rotated_box_mat(verts, idxs, o, rot_rad, [sx * (w * 0.5 - 120.0 + 40.0), plinth_h + body_h * fy, dz], [140.0, 1500.0, 1300.0], win_col, win_mat);
            }
        }
    }
    for sx in [-1.0_f32, 1.0] {
        push_rotated_box_mat(verts, idxs, o, rot_rad, [sx * w * 0.28, plinth_h + body_h * 0.34, front_z + 30.0], [1300.0, 1500.0, 140.0], win_col, win_mat);
        push_rotated_box_mat(verts, idxs, o, rot_rad, [sx * w * 0.28, plinth_h + body_h * 0.78, front_z + 30.0], [1300.0, 1500.0, 140.0], win_col, win_mat);
    }

    // Overhanging alpine pitched roof + stone chimney
    let ridge_h = w * 0.34;
    push_pitched_roof_3d(verts, idxs, cx, y0 + wall_h, cz, w + 1400.0, d + 1600.0, ridge_h, rot_rad, roof_col, [0.46, 0.30, 0.18]);
    push_rotated_box(verts, idxs, o, rot_rad, [w * 0.22, wall_h + ridge_h * 0.65, -d * 0.22], [900.0, ridge_h * 0.9, 900.0], [0.48, 0.44, 0.42]);
}

/// 1:1 village chapel: 9 x 16 m nave, 12 m ridge, 18 m bell tower with a 28 m steeple.
pub(super) fn push_alpine_chapel(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, cx: f32, y0: f32, cz: f32) {
    let stucco = [0.93, 0.91, 0.86];
    let roof = [0.54, 0.21, 0.15];
    let spire_col = [0.22, 0.29, 0.26];
    // Nave
    push_box(verts, idxs, [cx, y0 + 3800.0, cz], [9000.0, 7600.0, 16000.0], stucco);
    push_pitched_roof_3d(verts, idxs, cx, y0 + 7600.0, cz, 10200.0, 17400.0, 4400.0, 0.0, roof, stucco);
    for sx in [-1.0_f32, 1.0] {
        for dz in [-4200.0_f32, 0.0, 4200.0] {
            push_rotated_box_mat(verts, idxs, [cx + sx * 4540.0, y0 + 3600.0, cz + dz], 0.0, [0.0, 0.0, 0.0], [140.0, 3400.0, 1250.0], [1.0, 0.85, 0.48], [0.22, 0.05, -0.95]);
        }
    }
    // Bell tower + steeple
    let tz = cz + 8800.0;
    push_box(verts, idxs, [cx, y0 + 9000.0, tz], [4200.0, 18000.0, 4200.0], stucco);
    push_rotated_box_mat(verts, idxs, [cx, y0 + 14500.0, tz + 2160.0], 0.0, [0.0, 0.0, 0.0], [1850.0, 1850.0, 140.0], [0.98, 0.96, 0.86], [0.25, 0.05, -0.85]);
    let base_y = y0 + 18000.0;
    let tip = [cx, base_y + 10000.0, tz];
    let hs = 2300.0_f32;
    push_tri(verts, idxs, [cx - hs, base_y, tz - hs], [cx + hs, base_y, tz - hs], tip, normalize3([0.0, 0.45, -1.0]), spire_col);
    push_tri(verts, idxs, [cx + hs, base_y, tz + hs], [cx - hs, base_y, tz + hs], tip, normalize3([0.0, 0.45, 1.0]), spire_col);
    push_tri(verts, idxs, [cx + hs, base_y, tz - hs], [cx + hs, base_y, tz + hs], tip, normalize3([1.0, 0.45, 0.0]), spire_col);
    push_tri(verts, idxs, [cx - hs, base_y, tz + hs], [cx - hs, base_y, tz - hs], tip, normalize3([-1.0, 0.45, 0.0]), spire_col);
}

#[inline]
pub(super) fn env_river_center_z(x: f32, cx: f32, river_z_base: f32) -> f32 {
    let u = (x - cx) / 110000.0;
    river_z_base + (u * 4.5).sin() * 6500.0 + (u * 9.5).cos() * 1800.0
}

#[allow(clippy::too_many_arguments)]
pub(super) fn smoothstep32(a: f32, b: f32, x: f32) -> f32 {
    smoothstep(a as f64, b as f64, x as f64) as f32
}

#[allow(clippy::too_many_arguments)]
pub(super) fn eval_env_terrain_y(
    x: f32,
    z: f32,
    cx: f32,
    cz: f32,
    hw_hall: f32,
    hl_hall: f32,
    river_z_base: f32,
    lake_cx: f32,
    lake_cz: f32,
    floor_y: f32,
) -> (f32, bool) {
    // 1:1 world: the valley floor (hall, village, road, river, lake, forest) is perfectly flat so that
    // 10 m buildings and 15 m trees sit exactly on the ground; hills and mountains start ~100 m out.
    let base_y = floor_y - 24.0;
    let r = (x - cx).hypot(z - cz);
    let rz = env_river_center_z(x, cx, river_z_base);
    let river_d = (z - rz).abs();
    let is_river = river_d < 11500.0 && (x - cx).abs() < 150000.0;
    let lake_norm = (((x - lake_cx) / 27500.0).powi(2) + ((z - lake_cz) / 18500.0).powi(2)).sqrt();
    let is_water_basin = is_river || lake_norm < 1.0;

    let valley_r = hw_hall.hypot(hl_hall) + 100000.0;
    let past_valley = (r - valley_r).max(0.0);
    let foothill_w = smoothstep32(0.0, 90000.0, past_valley);
    let mtn_w = smoothstep32(45000.0, 115000.0, past_valley) * (1.0 - smoothstep32(235000.0, 315000.0, r));
    let far_ridge_w = smoothstep32(140000.0, 250000.0, r) * (1.0 - smoothstep32(275000.0, 325000.0, r));
    let river_clear = smoothstep32(14000.0, 30000.0, river_d);

    let ang = (z - cz).atan2(x - cx);
    let n1 = mountain_noise(x as f64 * 0.000042, z as f64 * 0.000042) as f32;
    let n2 = mountain_noise(x as f64 * 0.00011 + 31.7, z as f64 * 0.00011 - 14.2) as f32;
    let n3 = mountain_noise(x as f64 * 0.00026 - 9.1, z as f64 * 0.00026 + 22.4) as f32;
    let n1_pos = (n1 * 0.4 + 0.5).clamp(0.0, 1.0);
    let n2_pos = (n2 * 0.4 + 0.5).clamp(0.0, 1.0);
    let ridge_wave = ((ang * 5.0).cos() * 0.5 + 0.5) * 0.55 + ((ang * 9.0 + 1.1).sin() * 0.5 + 0.5) * 0.45;
    let ridge = (1.0 - n1.abs().clamp(0.0, 1.0)).powf(1.45);

    let foothills = (n1_pos * 7000.0 + n2_pos * 3000.0) * foothill_w;
    let peaks = (16000.0 + ridge * 42000.0 + n2_pos * 14000.0 + n3 * 3000.0).max(0.0)
        * (0.45 + 0.72 * ridge_wave)
        * mtn_w;
    let horizon_hills = (9000.0 + (n1_pos * 0.6 + n2_pos * 0.4) * 15000.0) * far_ridge_w;

    let y = base_y + (foothills + peaks + horizon_hills) * river_clear;
    (y, is_water_basin)
}


pub(super) fn push_celestial_bodies_3d(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, time_of_day: f32, env_mode: Viewer3DEnv) {
    if env_mode == Viewer3DEnv::Minimal {
        return;
    }
    let h = time_of_day.rem_euclid(24.0);
    let sun_phase = ((h - 6.0) / 12.0) * std::f32::consts::PI;
    let sx = -sun_phase.cos() * 0.85;
    let sy = sun_phase.sin();
    let sz = -0.42_f32;
    let slen = (sx * sx + sy * sy + sz * sz).sqrt().max(1e-4);
    let dist = 240000.0_f32;
    let sun_pos = [sx / slen * dist, sy / slen * dist, sz / slen * dist];
    if sun_pos[1] > -16000.0 {
        push_rotated_box_mat(verts, idxs, sun_pos, 0.0, [0.0, 0.0, 0.0], [9500.0, 9500.0, 9500.0], [1.0, 0.95, 0.68], [0.10, 0.0, 1.35]);
        push_rotated_box_mat(verts, idxs, sun_pos, 0.78, [0.0, 0.0, 0.0], [9000.0, 9000.0, 9000.0], [1.0, 0.90, 0.52], [0.10, 0.0, 1.25]);
    }
    let moon_pos = [-sun_pos[0], -sun_pos[1], -sun_pos[2]];
    if moon_pos[1] > -16000.0 {
        push_rotated_box_mat(verts, idxs, moon_pos, 0.35, [0.0, 0.0, 0.0], [7000.0, 7000.0, 7000.0], [0.82, 0.90, 1.0], [0.15, 0.0, 1.15]);
    }
}

pub(super) fn parse_hex_color_f32(hex: &str) -> Option<[f32; 3]> {
    let s = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&s[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&s[4..6], 16).ok()? as f32 / 255.0;
    Some([r, g, b])
}

#[derive(Default)]
pub(super) struct CatenaryMast3DContext {
    placed_masts: Vec<[f32; 3]>,
    mast_count: usize,
}

impl CatenaryMast3DContext {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_place_mast(
        &mut self,
        verts: &mut Vec<Vertex3D>,
        idxs: &mut Vec<u32>,
        anchor: [f32; 3],
        nx: f32,
        nz: f32,
        preferred_side: f32,
        mast_off: f32,
        contact_h: f32,
        messenger_h: f32,
        piece_id: &str,
        layout: &Layout,
    ) {
        // Deduplicate by track centerline anchor so connected track joints share one mast
        if self.placed_masts.iter().any(|pm| {
            (pm[0] - anchor[0]).hypot(pm[2] - anchor[2]) < 34.0 && (pm[1] - anchor[1]).abs() < 25.0
        }) {
            return;
        }

        let mut chosen_pos: Option<[f32; 3]> = None;
        for side_sign in [preferred_side, -preferred_side] {
            let cand_x = anchor[0] + nx * mast_off * side_sign;
            let cand_z = anchor[2] + nz * mast_off * side_sign;
            let hits_other_track = layout.pieces.iter().any(|other| {
                if other.id == piece_id {
                    return false;
                }
                (other.transform.x as f32 - cand_x).hypot(other.transform.y as f32 - cand_z) < 20.0
            });
            if !hits_other_track {
                chosen_pos = Some([cand_x, anchor[1], cand_z]);
                break;
            }
        }

        let Some([mx, my, mz]) = chosen_pos else {
            return;
        };
        self.placed_masts.push(anchor);
        self.mast_count += 1;

        let mast_col = [0.20, 0.25, 0.33];
        let insulator_brown = [0.52, 0.30, 0.06];
        let pole_h = messenger_h + 3.0;
        // Concrete foundation pad + steel lattice mast pole
        push_box(verts, idxs, [mx, my + 4.0, mz], [6.5, 8.0, 6.5], [0.46, 0.48, 0.52]);
        push_box(verts, idxs, [mx, my + 4.0 + pole_h * 0.5, mz], [3.0, pole_h, 3.0], mast_col);

        // 1. Upper horizontal Spitzenausleger arm at messenger_h (from mast straight to messenger wire anchor)
        push_box_segment(
            verts,
            idxs,
            [mx, my + messenger_h, mz],
            [anchor[0], my + messenger_h, anchor[2]],
            1.5,
            1.5,
            mast_col,
        );
        // Messenger wire clamp directly at (anchor[0], my + messenger_h, anchor[2])
        push_box(
            verts,
            idxs,
            [anchor[0], my + messenger_h + 0.6, anchor[2]],
            [2.0, 2.0, 2.0],
            mast_col,
        );

        // 2. Diagonal Auslegerrohr from mast (y = contact_h - 3.0) up to upper arm (at 16% out from track)
        let top_join_x = anchor[0] * 0.84 + mx * 0.16;
        let top_join_z = anchor[2] * 0.84 + mz * 0.16;
        let mast_diag_y = my + contact_h - 3.0;
        push_box_segment(
            verts,
            idxs,
            [mx, mast_diag_y, mz],
            [top_join_x, my + messenger_h, top_join_z],
            1.2,
            1.2,
            mast_col,
        );

        // 3. Lower registration arm (Seitenhalter) from mid-diagonal (58% toward mast, y = contact_h + 5.0)
        //    directly to the contact wire at (anchor[0], my + contact_h, anchor[2])
        let mid_diag_x = anchor[0] * 0.42 + mx * 0.58;
        let mid_diag_z = anchor[2] * 0.42 + mz * 0.58;
        let mid_diag_y = my + contact_h + 5.0;
        push_box_segment(
            verts,
            idxs,
            [mid_diag_x, mid_diag_y, mid_diag_z],
            [anchor[0], my + contact_h, anchor[2]],
            1.0,
            1.0,
            mast_col,
        );
        // Contact wire clamp directly at (anchor[0], my + contact_h, anchor[2])
        push_box(
            verts,
            idxs,
            [anchor[0], my + contact_h + 0.5, anchor[2]],
            [2.2, 1.6, 1.6],
            mast_col,
        );

        // 4. Three ceramic insulators positioned strictly ON the three arms
        let ins_upper_x = anchor[0] * 0.15 + mx * 0.85;
        let ins_upper_z = anchor[2] * 0.15 + mz * 0.85;
        push_box(verts, idxs, [ins_upper_x, my + messenger_h + 0.75, ins_upper_z], [2.8, 2.8, 2.8], insulator_brown);

        let ins_diag_u = 0.82_f32;
        let ins_diag_x = top_join_x * (1.0 - ins_diag_u) + mx * ins_diag_u;
        let ins_diag_z = top_join_z * (1.0 - ins_diag_u) + mz * ins_diag_u;
        let ins_diag_y = (my + messenger_h) * (1.0 - ins_diag_u) + mast_diag_y * ins_diag_u;
        push_box(verts, idxs, [ins_diag_x, ins_diag_y + 0.6, ins_diag_z], [2.8, 2.8, 2.8], insulator_brown);

        let ins_reg_x = anchor[0] * 0.35 + mid_diag_x * 0.65;
        let ins_reg_z = anchor[2] * 0.35 + mid_diag_z * 0.65;
        let ins_reg_y = (my + contact_h) * 0.35 + mid_diag_y * 0.65;
        push_box(verts, idxs, [ins_reg_x, ins_reg_y + 0.5, ins_reg_z], [2.4, 2.4, 2.4], insulator_brown);

        // Every 6th mast: Radspannwerk (tension wheel + hanging concrete counterweight stack)
        if self.mast_count % 6 == 0 {
            let wx = mx + (mx - anchor[0]) * 0.16;
            let wz = mz + (mz - anchor[2]) * 0.16;
            push_box(verts, idxs, [wx, my + contact_h + 8.0, wz], [4.6, 4.6, 1.8], [0.25, 0.28, 0.32]);
            push_box(verts, idxs, [wx, my + 28.0, wz], [4.0, 22.0, 4.0], [0.42, 0.44, 0.46]);
        }
    }
}

pub(super) fn fnv1a_u64(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub(super) fn hash_f64(h: u64, v: f64) -> u64 {
    fnv1a_u64(h, &v.to_bits().to_le_bytes())
}


mod hall_landscape;
pub(super) use hall_landscape::push_hall_landscape_env_3d;
