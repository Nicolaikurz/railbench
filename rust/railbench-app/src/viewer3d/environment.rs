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

fn push_env_deciduous_tree(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, x: f32, y0: f32, z: f32, h: f32, seed: usize) {
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
fn push_alpine_chalet(
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
fn push_alpine_chapel(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, cx: f32, y0: f32, cz: f32) {
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
fn env_river_center_z(x: f32, cx: f32, river_z_base: f32) -> f32 {
    let u = (x - cx) / 110000.0;
    river_z_base + (u * 4.5).sin() * 6500.0 + (u * 9.5).cos() * 1800.0
}

#[allow(clippy::too_many_arguments)]
fn smoothstep32(a: f32, b: f32, x: f32) -> f32 {
    smoothstep(a as f64, b as f64, x as f64) as f32
}

#[allow(clippy::too_many_arguments)]
fn eval_env_terrain_y(
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

pub(super) fn push_minimal_cad_grid_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    boards: &[BaseboardConfig],
    dark_mode: bool,
) {
    let (mut min_x, mut max_x, mut min_z, mut max_z) = (-1200.0_f64, 1200.0_f64, -800.0_f64, 800.0_f64);
    let mut has = false;
    for b in boards {
        if !b.enabled {
            continue;
        }
        for pt in board_tools::get_baseboard_polygon(b) {
            if !has {
                min_x = pt.x; max_x = pt.x; min_z = pt.y; max_z = pt.y; has = true;
            } else {
                min_x = min_x.min(pt.x); max_x = max_x.max(pt.x);
                min_z = min_z.min(pt.y); max_z = max_z.max(pt.y);
            }
        }
    }
    let gx0 = ((min_x - 1600.0) / 500.0).floor() * 500.0;
    let gx1 = ((max_x + 1600.0) / 500.0).ceil() * 500.0;
    let gz0 = ((min_z - 1600.0) / 500.0).floor() * 500.0;
    let gz1 = ((max_z + 1600.0) / 500.0).ceil() * 500.0;
    let y = -22.5_f32;
    let minor_col = if dark_mode { [0.16, 0.22, 0.34] } else { [0.68, 0.74, 0.82] };
    let major_col = if dark_mode { [0.24, 0.36, 0.56] } else { [0.46, 0.56, 0.68] };

    let mut gx = gx0;
    while gx <= gx1 + 1.0 {
        let is_major = (gx / 500.0 - (gx / 500.0).round()).abs() < 1e-4;
        let col = if is_major { major_col } else { minor_col };
        let w = if is_major { 3.2 } else { 1.4 };
        push_box_segment(verts, idxs, [gx as f32, y, gz0 as f32], [gx as f32, y, gz1 as f32], w, 1.0, col);
        gx += 100.0;
    }
    let mut gz = gz0;
    while gz <= gz1 + 1.0 {
        let is_major = (gz / 500.0 - (gz / 500.0).round()).abs() < 1e-4;
        let col = if is_major { major_col } else { minor_col };
        let w = if is_major { 3.2 } else { 1.4 };
        push_box_segment(verts, idxs, [gx0 as f32, y, gz as f32], [gx1 as f32, y, gz as f32], w, 1.0, col);
        gz += 100.0;
    }
}

pub(super) fn push_hall_landscape_env_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    boards: &[BaseboardConfig],
    dark_mode: bool,
) {
    let (mut min_x, mut max_x, mut min_z, mut max_z) = (-1200.0_f64, 1200.0_f64, -800.0_f64, 800.0_f64);
    let mut has = false;
    for b in boards {
        if !b.enabled {
            continue;
        }
        for pt in board_tools::get_baseboard_polygon(b) {
            if !has {
                min_x = pt.x; max_x = pt.x; min_z = pt.y; max_z = pt.y; has = true;
            } else {
                min_x = min_x.min(pt.x); max_x = max_x.max(pt.x);
                min_z = min_z.min(pt.y); max_z = max_z.max(pt.y);
            }
        }
    }
    let cx = ((min_x + max_x) * 0.5) as f32;
    let cz = ((min_z + max_z) * 0.5) as f32;
    let w_table = (max_x - min_x).max(1200.0) as f32;
    let l_table = (max_z - min_z).max(1000.0) as f32;
    let w_hall = (w_table + 4800.0).max(9000.0);
    let l_hall = (l_table + 5400.0).max(9800.0);
    let hw_hall = w_hall * 0.5;
    let hl_hall = l_hall * 0.5;
    let hall_r = hw_hall.hypot(hl_hall);
    let floor_y = -755.0_f32;
    let wall_h = 3350.0_f32;
    let arch_rise = 1420.0_f32;

    let concrete = if dark_mode { [0.18, 0.21, 0.27] } else { [0.38, 0.42, 0.48] };
    let apron_col = if dark_mode { [0.22, 0.24, 0.28] } else { [0.46, 0.48, 0.52] };
    let steel = if dark_mode { [0.11, 0.15, 0.21] } else { [0.19, 0.24, 0.32] };
    let steel_dark = [0.08, 0.11, 0.16];
    let brick = if dark_mode { [0.30, 0.16, 0.12] } else { [0.46, 0.23, 0.16] };
    let brick_pilaster = [brick[0] * 0.84, brick[1] * 0.84, brick[2] * 0.84];
    let yellow = [0.89, 0.64, 0.08];
    let sandstone = [0.70, 0.64, 0.53];
    let rail_col = [0.15, 0.20, 0.28];

    // =========================================================================
    //  1. Historic Steel-and-Brick Exhibition Hall
    // =========================================================================
    // Concrete floor slab + exterior stone walkway apron + floor grid seams
    push_box(verts, idxs, [cx, floor_y - 12.0, cz], [w_hall, 24.0, l_hall], concrete);
    push_box(verts, idxs, [cx, floor_y - 18.0, cz], [w_hall + 960.0, 14.0, l_hall + 960.0], apron_col);
    for i in -4..=4 {
        let sx = cx + (i as f32) * (w_hall / 9.0);
        push_box(verts, idxs, [sx, floor_y + 0.6, cz], [8.0, 1.2, l_hall - 80.0], steel_dark);
    }
    for j in -4..=4 {
        let sz = cz + (j as f32) * (l_hall / 9.0);
        push_box(verts, idxs, [cx, floor_y + 0.6, sz], [w_hall - 80.0, 1.2, 8.0], steel_dark);
    }

    // Yellow safety line & steel visitor viewing guardrail framing the layout table
    let sw = w_table + 820.0;
    let sl = l_table + 820.0;
    push_box(verts, idxs, [cx, floor_y + 1.5, cz - sl * 0.5], [sw, 3.0, 26.0], yellow);
    push_box(verts, idxs, [cx, floor_y + 1.5, cz + sl * 0.5], [sw, 3.0, 26.0], yellow);
    push_box(verts, idxs, [cx - sw * 0.5, floor_y + 1.5, cz], [26.0, 3.0, sl], yellow);
    push_box(verts, idxs, [cx + sw * 0.5, floor_y + 1.5, cz], [26.0, 3.0, sl], yellow);

    let rw = sw + 130.0;
    let rl = sl + 130.0;
    for hy in [420.0_f32, 740.0] {
        push_box(verts, idxs, [cx, floor_y + hy, cz - rl * 0.5], [rw, 16.0, 16.0], rail_col);
        push_box(verts, idxs, [cx, floor_y + hy, cz + rl * 0.5], [rw, 16.0, 16.0], rail_col);
        push_box(verts, idxs, [cx - rw * 0.5, floor_y + hy, cz], [16.0, 16.0, rl], rail_col);
        push_box(verts, idxs, [cx + rw * 0.5, floor_y + hy, cz], [16.0, 16.0, rl], rail_col);
    }
    let posts_x = 6usize;
    let posts_z = 5usize;
    for k in 0..=posts_x {
        let px = cx - rw * 0.5 + (k as f32 / posts_x as f32) * rw;
        push_box(verts, idxs, [px, floor_y + 390.0, cz - rl * 0.5], [22.0, 780.0, 22.0], rail_col);
        push_box(verts, idxs, [px, floor_y + 390.0, cz + rl * 0.5], [22.0, 780.0, 22.0], rail_col);
    }
    for k in 1..posts_z {
        let pz = cz - rl * 0.5 + (k as f32 / posts_z as f32) * rl;
        push_box(verts, idxs, [cx - rw * 0.5, floor_y + 390.0, pz], [22.0, 780.0, 22.0], rail_col);
        push_box(verts, idxs, [cx + rw * 0.5, floor_y + 390.0, pz], [22.0, 780.0, 22.0], rail_col);
    }
    // Safety corner bollards
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            let bx = cx + sx * (rw * 0.5 + 45.0);
            let bz = cz + sz * (rl * 0.5 + 45.0);
            push_box(verts, idxs, [bx, floor_y + 220.0, bz], [40.0, 440.0, 40.0], yellow);
            push_box(verts, idxs, [bx, floor_y + 390.0, bz], [42.0, 60.0, 42.0], steel_dark);
        }
    }

    // Brick wainscoting walls + stone plinth + carved sandstone sills + eaves beams
    push_box(verts, idxs, [cx, floor_y + 90.0, cz - hl_hall], [w_hall + 40.0, 180.0, 150.0], sandstone);
    push_box(verts, idxs, [cx, floor_y + 90.0, cz + hl_hall], [w_hall + 40.0, 180.0, 150.0], sandstone);
    push_box(verts, idxs, [cx - hw_hall, floor_y + 90.0, cz], [150.0, 180.0, l_hall + 40.0], sandstone);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 90.0, cz], [150.0, 180.0, l_hall + 40.0], sandstone);

    push_box(verts, idxs, [cx, floor_y + 520.0, cz - hl_hall], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [cx, floor_y + 520.0, cz + hl_hall], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [cx - hw_hall, floor_y + 520.0, cz], [120.0, 1040.0, l_hall], brick);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 520.0, cz], [120.0, 1040.0, l_hall], brick);
    push_box(verts, idxs, [cx - hw_hall, floor_y + 1054.0, cz], [148.0, 28.0, l_hall + 40.0], sandstone);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 1054.0, cz], [148.0, 28.0, l_hall + 40.0], sandstone);
    for sx in [-1.0_f32, 1.0] {
        push_box(verts, idxs, [cx + sx * hw_hall, floor_y + wall_h, cz], [105.0, 105.0, l_hall + 40.0], steel);
    }

    // Full-height brick walls with glazed clerestory windows (side walls) and glazed arched gables.
    // Wall panels sit between the sandstone/brick wainscot (1.04 m) and the steel eaves beam.
    let glass_col = if dark_mode { [0.55, 0.62, 0.70] } else { [0.98, 0.88, 0.62] };
    let glass_mat = [0.14, 0.18, -0.55];
    let wall_t = 120.0_f32;
    let wain_h = 1040.0_f32;
    let sill_y = 1780.0_f32;
    let head_y = 2520.0_f32;
    let wall_bays = 5usize;
    for sx in [-1.0_f32, 1.0] {
        let wx = cx + sx * hw_hall;
        push_box(verts, idxs, [wx, floor_y + (wain_h + sill_y) * 0.5, cz], [wall_t, sill_y - wain_h, l_hall], brick);
        push_box(verts, idxs, [wx, floor_y + (head_y + wall_h) * 0.5, cz], [wall_t, wall_h - head_y, l_hall], brick);
        for i in 0..=wall_bays {
            let pz = (cz - hl_hall) + (i as f32 / wall_bays as f32) * l_hall;
            push_box(verts, idxs, [wx, floor_y + (sill_y + head_y) * 0.5, pz], [wall_t, head_y - sill_y, 200.0], brick);
        }
        for i in 0..wall_bays {
            let z_mid = (cz - hl_hall) + ((i as f32 + 0.5) / wall_bays as f32) * l_hall;
            let bay_l = l_hall / wall_bays as f32 - 200.0;
            push_rotated_box_mat(
                verts,
                idxs,
                [wx, floor_y + (sill_y + head_y) * 0.5, z_mid],
                0.0,
                [0.0, 0.0, 0.0],
                [24.0, head_y - sill_y, bay_l],
                glass_col,
                glass_mat,
            );
        }
    }
    for sz in [-1.0_f32, 1.0] {
        let gz = cz + sz * hl_hall;
        let band_top = 1740.0_f32;
        let edges = [-hw_hall, -1540.0, -560.0, 560.0, 1540.0, hw_hall];
        for (a, b) in [(edges[0], edges[1]), (edges[2], edges[3]), (edges[4], edges[5])] {
            push_box(verts, idxs, [cx + (a + b) * 0.5, floor_y + (wain_h + band_top) * 0.5, gz], [b - a, band_top - wain_h, wall_t], brick);
        }
        let strips = 24usize;
        let ws = w_hall / strips as f32;
        for s in 0..strips {
            let u = (s as f32 + 0.5) / strips as f32;
            let xc = cx + (u - 0.5) * w_hall;
            let ytop = wall_h + (u * std::f32::consts::PI).sin() * arch_rise - 70.0;
            let h = ytop - band_top;
            if h < 50.0 {
                continue;
            }
            if (u - 0.5).abs() < 0.33 {
                push_rotated_box_mat(verts, idxs, [xc, floor_y + band_top + h * 0.5, gz], 0.0, [0.0, 0.0, 0.0], [ws, h, 24.0], glass_col, glass_mat);
            } else {
                push_box(verts, idxs, [xc, floor_y + band_top + h * 0.5, gz], [ws, h, wall_t], brick);
            }
        }
    }

    // Gable ends: paneled twin locomotive rolling doors, arched transom mullions & detailed heritage station clocks
    for sz in [-1.0_f32, 1.0] {
        let gz = cz + sz * (hl_hall - 40.0);
        for dx in [-1050.0_f32, 1050.0] {
            push_box(verts, idxs, [cx + dx, floor_y + 840.0, gz], [980.0, 1680.0, 74.0], [0.16, 0.22, 0.19]);
            for pane in -2..=2 {
                push_box(verts, idxs, [cx + dx + (pane as f32) * 185.0, floor_y + 840.0, gz - sz * 22.0], [16.0, 1600.0, 36.0], steel_dark);
            }
            push_box(verts, idxs, [cx + dx, floor_y + 1700.0, gz], [1080.0, 52.0, 104.0], sandstone);
        }
        // Overhead rolling-door rail beam + glazed arch mullions in gable
        push_box(verts, idxs, [cx, floor_y + 1860.0, gz - sz * 30.0], [w_hall * 0.72, 34.0, 44.0], steel);
        for mx in [-0.30_f32, -0.15, 0.15, 0.30] {
            let gx_m = cx + mx * w_hall;
            let u = mx + 0.5;
            let arch_y = wall_h + (u * std::f32::consts::PI).sin() * arch_rise;
            let col_h = (arch_y - 1050.0).max(400.0);
            push_box(verts, idxs, [gx_m, floor_y + 1050.0 + col_h * 0.5, gz], [38.0, col_h, 38.0], steel);
        }
        // Heritage station clock with bezel, glowing dial, 4 cardinal hour ticks, & hands
        let clk_y = floor_y + wall_h + arch_rise * 0.50;
        push_box(verts, idxs, [cx, clk_y, gz], [410.0, 410.0, 58.0], steel);
        push_rotated_box_mat(verts, idxs, [cx, clk_y, gz - sz * 34.0], 0.0, [0.0, 0.0, 0.0], [335.0, 335.0, 14.0], [0.97, 0.96, 0.90], [0.28, 0.05, -0.88]);
        for &(ox, oy, sx, sy) in &[
            (0.0_f32, 128.0_f32, 12.0_f32, 34.0_f32),
            (0.0, -128.0, 12.0, 34.0),
            (-128.0, 0.0, 34.0, 12.0),
            (128.0, 0.0, 34.0, 12.0),
        ] {
            push_box(verts, idxs, [cx + ox, clk_y + oy, gz - sz * 43.0], [sx, sy, 10.0], steel_dark);
        }
        push_box(verts, idxs, [cx, clk_y + 56.0, gz - sz * 46.0], [18.0, 128.0, 12.0], steel_dark);
        push_box(verts, idxs, [cx + 48.0, clk_y, gz - sz * 46.0], [108.0, 16.0, 12.0], steel_dark);
    }

    // 5 Hall Structural Bays: Brick pilasters, twin-chord lattice arch trusses, skyline lantern & pendant lamps
    let bays = 5usize;
    let arch_segs = 12usize;
    let truss_depth = 175.0_f32;
    for i in 0..=bays {
        let pz = (cz - hl_hall) + (i as f32 / bays as f32) * l_hall;
        for sx in [-1.0_f32, 1.0] {
            push_box(verts, idxs, [cx + sx * hw_hall, floor_y + wall_h * 0.5, pz], [170.0, wall_h, 170.0], steel);
            push_box(verts, idxs, [cx + sx * (hw_hall + 18.0), floor_y + 520.0, pz], [190.0, 1040.0, 210.0], brick_pilaster);
        }
        push_box(verts, idxs, [cx, floor_y + wall_h, pz], [w_hall + 170.0, 100.0, 100.0], steel);
        // Dual-chord steel lattice arch truss (upper + lower chord + web struts)
        for s in 0..arch_segs {
            let u0 = s as f32 / arch_segs as f32;
            let u1 = (s + 1) as f32 / arch_segs as f32;
            let x0 = cx + (u0 - 0.5) * w_hall;
            let x1 = cx + (u1 - 0.5) * w_hall;
            let yu0 = floor_y + wall_h + (u0 * std::f32::consts::PI).sin() * arch_rise;
            let yu1 = floor_y + wall_h + (u1 * std::f32::consts::PI).sin() * arch_rise;
            let yl0 = yu0 - truss_depth * (u0 * std::f32::consts::PI).sin().max(0.25);
            let yl1 = yu1 - truss_depth * (u1 * std::f32::consts::PI).sin().max(0.25);
            push_box_segment(verts, idxs, [x0, yu0, pz], [x1, yu1, pz], 85.0, 88.0, steel);
            push_box_segment(verts, idxs, [x0, yl0, pz], [x1, yl1, pz], 64.0, 64.0, steel);
            push_box_segment(verts, idxs, [x0, yl0, pz], [x0, yu0, pz], 34.0, 34.0, steel);
            push_box_segment(verts, idxs, [x0, yl0, pz], [x1, yu1, pz], 28.0, 28.0, steel);
            if s > 0 && s < arch_segs {
                push_box_segment(verts, idxs, [x0, floor_y + wall_h, pz], [x0, yl0, pz], 32.0, 32.0, steel);
            }
        }
        // Suspended industrial enamel pendant lamps (flared shade + warm bulb)
        if i < bays {
            let pz_l = (cz - hl_hall) + ((i as f32 + 0.5) / bays as f32) * l_hall;
            for lx_off in [-w_hall * 0.24, 0.0, w_hall * 0.24] {
                let ly = floor_y + wall_h - 140.0;
                push_box(verts, idxs, [cx + lx_off, ly + 85.0, pz_l], [14.0, 170.0, 14.0], steel);
                push_box(verts, idxs, [cx + lx_off, ly + 22.0, pz_l], [110.0, 32.0, 110.0], [0.22, 0.27, 0.34]);
                push_box(verts, idxs, [cx + lx_off, ly, pz_l], [175.0, 24.0, 175.0], [0.18, 0.23, 0.29]);
                push_rotated_box_mat(verts, idxs, [cx + lx_off, ly - 18.0, pz_l], 0.0, [0.0, 0.0, 0.0], [110.0, 20.0, 110.0], [1.0, 0.92, 0.70], [0.18, 0.05, -1.25]);
            }
        }
    }

    // Panoramic open industrial mullioned clerestory windows + central ridge skylight lantern
    for i in 0..bays {
        let z_mid = (cz - hl_hall) + ((i as f32 + 0.5) / bays as f32) * l_hall;
        let bay_l = (l_hall / bays as f32) - 180.0;
        for sx in [-1.0_f32, 1.0] {
            let wx = cx + sx * hw_hall;
            push_box(verts, idxs, [wx, floor_y + 1780.0, z_mid], [44.0, 34.0, bay_l], steel);
            push_box(verts, idxs, [wx, floor_y + 2520.0, z_mid], [44.0, 34.0, bay_l], steel);
            for m in -1..=1 {
                let mz = z_mid + (m as f32) * (bay_l * 0.32);
                push_box(verts, idxs, [wx, floor_y + 2190.0, mz], [42.0, 2240.0, 28.0], steel);
            }
        }
        // Ridge lantern along crown
        let crown_y = floor_y + wall_h + arch_rise;
        push_box(verts, idxs, [cx - 640.0, crown_y + 140.0, z_mid], [48.0, 280.0, bay_l], steel);
        push_box(verts, idxs, [cx + 640.0, crown_y + 140.0, z_mid], [48.0, 280.0, bay_l], steel);
        push_rotated_box_mat(verts, idxs, [cx, crown_y + 280.0, z_mid], 0.0, [0.0, 0.0, 0.0], [1340.0, 22.0, bay_l], [0.70, 0.85, 0.96], [0.85, 0.35, 0.0]);
    }

    // Detailed Overhead Double-Girder Yellow Bridge Crane (rails, end trucks, girders, catwalk, trolley, drums, ropes, hook)
    let crane_y = floor_y + wall_h - 360.0;
    let rail_span = w_hall - 280.0;
    for sx in [-1.0_f32, 1.0] {
        push_box(verts, idxs, [cx + sx * rail_span * 0.5, crane_y, cz], [82.0, 105.0, l_hall - 160.0], steel);
    }
    let crane_z = cz - l_hall * 0.18;
    for dx in [-1.0_f32, 1.0] {
        push_box(verts, idxs, [cx + dx * (rail_span * 0.5 - 35.0), crane_y + 95.0, crane_z], [130.0, 120.0, 520.0], yellow);
    }
    push_box(verts, idxs, [cx, crane_y + 115.0, crane_z - 160.0], [rail_span, 155.0, 96.0], yellow);
    push_box(verts, idxs, [cx, crane_y + 115.0, crane_z + 160.0], [rail_span, 155.0, 96.0], yellow);
    push_box(verts, idxs, [cx, crane_y + 245.0, crane_z - 220.0], [rail_span - 240.0, 14.0, 14.0], rail_col);
    let trolley_x = cx + w_table * 0.16;
    push_box(verts, idxs, [trolley_x, crane_y + 215.0, crane_z], [420.0, 115.0, 410.0], [0.20, 0.25, 0.32]);
    push_box(verts, idxs, [trolley_x - 160.0, crane_y - 35.0, crane_z + 250.0], [180.0, 210.0, 170.0], yellow);
    push_box(verts, idxs, [trolley_x - 160.0, crane_y - 25.0, crane_z + 338.0], [120.0, 110.0, 10.0], [0.65, 0.80, 0.92]);
    push_box(verts, idxs, [trolley_x, crane_y + 290.0, crane_z], [160.0, 95.0, 120.0], [0.35, 0.38, 0.42]);
    for rx in [-36.0_f32, 36.0] {
        push_box(verts, idxs, [trolley_x + rx, crane_y - 190.0, crane_z], [12.0, 720.0, 12.0], [0.16, 0.18, 0.22]);
    }
    push_box(verts, idxs, [trolley_x, crane_y - 575.0, crane_z], [115.0, 125.0, 76.0], yellow);

    // Interior perimeter hall props strictly near brick walls (display boards, benches, workbench, switchgear, green EXIT signs)
    for dx in [-w_hall * 0.24, 0.0, w_hall * 0.24] {
        push_box(verts, idxs, [cx + dx, floor_y + 220.0, cz + hl_hall - 260.0], [420.0, 115.0, 130.0], [0.45, 0.30, 0.18]);
    }
    push_box(verts, idxs, [cx - hw_hall + 180.0, floor_y + 185.0, cz - hl_hall * 0.28], [160.0, 370.0, 480.0], [0.32, 0.36, 0.42]);
    push_box(verts, idxs, [cx - hw_hall + 150.0, floor_y + 250.0, cz + hl_hall * 0.25], [150.0, 210.0, 520.0], [0.24, 0.35, 0.48]);
    for bz in [-hl_hall * 0.30, hl_hall * 0.30] {
        push_box(verts, idxs, [cx + hw_hall - 75.0, floor_y + 620.0, cz + bz], [24.0, 560.0, 880.0], [0.18, 0.24, 0.32]);
        push_box(verts, idxs, [cx + hw_hall - 60.0, floor_y + 620.0, cz + bz], [14.0, 470.0, 780.0], [0.85, 0.87, 0.89]);
    }
    for sz in [-1.0_f32, 1.0] {
        push_rotated_box_mat(verts, idxs, [cx, floor_y + 1780.0, cz + sz * (hl_hall - 95.0)], 0.0, [0.0, 0.0, 0.0], [220.0, 72.0, 16.0], [0.18, 0.92, 0.42], [0.20, 0.05, 0.85]);
    }

    // =========================================================================
    //  2. Horizon-Spanning Continuous Alpine Valley & Mountain Heightfield
    // =========================================================================
    let g = floor_y - 24.0;
    let valley_r = hw_hall.hypot(hl_hall) + 100000.0;
    let river_z_base = cz + hl_hall + 70000.0;
    let lake_cx = cx + hw_hall + 44000.0;
    let lake_cz = cz - hl_hall - 22000.0;

    let mut rings: Vec<f32> = Vec::with_capacity(50);
    rings.push(0.0);
    for i in 1..=4 {
        rings.push(valley_r * 0.25 * i as f32);
    }
    for i in 1..=44 {
        rings.push(valley_r + (i as f32 / 44.0) * (322000.0 - valley_r));
    }
    let sectors = 144usize;
    let nr = rings.len();
    let mut grid_pos: Vec<[f32; 3]> = Vec::with_capacity(nr * sectors);
    let mut grid_col: Vec<[f32; 3]> = Vec::with_capacity(nr * sectors);

    for &rad in &rings {
        for s in 0..sectors {
            let ang = (s as f32 / sectors as f32) * std::f32::consts::TAU;
            let px = cx + ang.cos() * rad;
            let pz = cz + ang.sin() * rad;
            let (py, is_water) = eval_env_terrain_y(
                px, pz, cx, cz, hw_hall, hl_hall, river_z_base, lake_cx, lake_cz, floor_y,
            );
            let h_above = py - g;
            let meadow_c = if dark_mode { [0.11, 0.23, 0.11] } else { [0.27, 0.45, 0.15] };
            let spruce_c = if dark_mode { [0.08, 0.16, 0.09] } else { [0.18, 0.32, 0.13] };
            let rock_c = if dark_mode { [0.23, 0.26, 0.31] } else { [0.44, 0.46, 0.50] };
            let sand_c = if dark_mode { [0.35, 0.32, 0.23] } else { [0.66, 0.60, 0.45] };
            let snow_c = if dark_mode { [0.76, 0.80, 0.88] } else { [0.94, 0.96, 0.99] };
            let vcol = if is_water {
                sand_c
            } else if h_above > 38000.0 {
                let st = smoothstep32(38000.0, 62000.0, h_above);
                [
                    rock_c[0] * (1.0 - st) + snow_c[0] * st,
                    rock_c[1] * (1.0 - st) + snow_c[1] * st,
                    rock_c[2] * (1.0 - st) + snow_c[2] * st,
                ]
            } else if h_above > 14000.0 {
                let st = smoothstep32(14000.0, 32000.0, h_above);
                [
                    spruce_c[0] * (1.0 - st) + rock_c[0] * st,
                    spruce_c[1] * (1.0 - st) + rock_c[1] * st,
                    spruce_c[2] * (1.0 - st) + rock_c[2] * st,
                ]
            } else {
                let st = smoothstep32(1500.0, 11000.0, h_above);
                [
                    meadow_c[0] * (1.0 - st) + spruce_c[0] * st,
                    meadow_c[1] * (1.0 - st) + spruce_c[1] * st,
                    meadow_c[2] * (1.0 - st) + spruce_c[2] * st,
                ]
            };
            grid_pos.push([px, py, pz]);
            grid_col.push(vcol);
        }
    }

    let base_v = verts.len() as u32;
    let mut accum_norm: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; grid_pos.len()];
    let mut tri_idx: Vec<[u32; 3]> = Vec::with_capacity((nr - 1) * sectors * 2);
    for r_i in 0..(nr - 1) {
        for s in 0..sectors {
            let s_next = (s + 1) % sectors;
            let i00 = (r_i * sectors + s) as u32;
            let i10 = ((r_i + 1) * sectors + s) as u32;
            let i11 = ((r_i + 1) * sectors + s_next) as u32;
            let i01 = (r_i * sectors + s_next) as u32;
            let p00 = grid_pos[i00 as usize];
            let p10 = grid_pos[i10 as usize];
            let p11 = grid_pos[i11 as usize];
            let p01 = grid_pos[i01 as usize];
            for tri in [[i00, i10, i11], [i00, i11, i01]] {
                let pa = p00;
                let pb = if tri == [i00, i10, i11] { p10 } else { p11 };
                let pc = if tri == [i00, i10, i11] { p11 } else { p01 };
                let n = normalize3(cross3(
                    [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]],
                    [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]],
                ));
                let ny_fix = if n[1] < 0.0 { -1.0 } else { 1.0 };
                for &vi in &tri {
                    accum_norm[vi as usize][0] += n[0] * ny_fix;
                    accum_norm[vi as usize][1] += n[1] * ny_fix;
                    accum_norm[vi as usize][2] += n[2] * ny_fix;
                }
                tri_idx.push(tri);
            }
        }
    }
    for (i, pos) in grid_pos.iter().enumerate() {
        let n = if accum_norm[i][1].abs() < 1e-4 {
            [0.0, 1.0, 0.0]
        } else {
            normalize3(accum_norm[i])
        };
        verts.push(Vertex3D::new(*pos, n, grid_col[i], MAT_TERRAIN));
    }
    for tri in tri_idx {
        idxs.extend_from_slice(&[base_v + tri[0], base_v + tri[1], base_v + tri[2]]);
    }

    // =========================================================================
    //  3. 1:1 Scenic Landmarks (lake, river, country road, viaduct, village, farm, forest, clouds)
    // =========================================================================
    let river_col = if dark_mode { [0.09, 0.27, 0.43] } else { [0.16, 0.48, 0.74] };
    let road_col = [0.25, 0.27, 0.31];
    let shoulder_col = [0.46, 0.43, 0.37];
    let line_col = [0.92, 0.90, 0.82];
    let stone_col = [0.52, 0.48, 0.44];
    let water_top = g + 120.0;
    let water_y = water_top - 30.0;

    // Alpine lake (55 x 37 m) + timber pier (11 m) on pilings + boathouse
    push_rotated_box_mat(
        verts,
        idxs,
        [lake_cx, water_y, lake_cz],
        0.15,
        [0.0, 0.0, 0.0],
        [55000.0, 60.0, 37000.0],
        river_col,
        MAT_WATER,
    );
    let shore_x = lake_cx - 27500.0;
    let pier_z = lake_cz + 2500.0;
    let wood = [0.46, 0.31, 0.18];
    push_box(verts, idxs, [shore_x + 3000.0, water_top + 620.0, pier_z], [11000.0, 260.0, 3000.0], wood);
    for k in 0..5 {
        for pz_off in [-1300.0_f32, 1300.0] {
            push_box(
                verts,
                idxs,
                [shore_x - 2000.0 + k as f32 * 2500.0, water_top - 410.0, pier_z + pz_off],
                [320.0, 1800.0, 320.0],
                [0.32, 0.21, 0.12],
            );
        }
    }
    let bh_x = shore_x - 7500.0;
    push_box(verts, idxs, [bh_x, g + 2100.0, pier_z], [7600.0, 4200.0, 6400.0], [0.54, 0.35, 0.20]);
    push_pitched_roof_3d(verts, idxs, bh_x, g + 4200.0, pier_z, 8400.0, 7000.0, 2400.0, 0.0, [0.48, 0.20, 0.15], [0.46, 0.30, 0.18]);

    // Valley river, 22 m wide
    let river_segs = 48usize;
    for i in 0..river_segs {
        let x0 = cx - 150000.0 + (i as f32 / river_segs as f32) * 300000.0;
        let x1 = cx - 150000.0 + ((i + 1) as f32 / river_segs as f32) * 300000.0;
        let z0 = env_river_center_z(x0, cx, river_z_base);
        let z1 = env_river_center_z(x1, cx, river_z_base);
        push_box_segment_mat(verts, idxs, [x0, water_top - 60.0, z0], [x1, water_top - 60.0, z1], 22000.0, 60.0, river_col, MAT_WATER);
    }

    // Country road: 7 m two-lane asphalt, 1.6 m gravel shoulders, edge lines and dashed centre line,
    // climbing over a 36 m stone viaduct across the river.
    let road_x = cx - hw_hall - 16000.0;
    let br_z = env_river_center_z(road_x, cx, river_z_base);
    let bridge_lift = 4500.0_f32;
    let road_lift = |z: f32| -> f32 {
        let d = (z - br_z).abs();
        bridge_lift * ((63000.0 - d) / 45000.0).clamp(0.0, 1.0)
    };
    let step = 3000.0_f32;
    let z_start = cz - 99000.0;
    for i in 0..66 {
        let za = z_start + i as f32 * step;
        let zb = za + step;
        let ya = g + 200.0 + road_lift(za);
        let yb = g + 200.0 + road_lift(zb);
        let bridged = (za + step * 0.5 - br_z).abs() < 18000.0;
        if !bridged {
            let h = ((ya + yb) * 0.5 - 80.0 - g).max(120.0);
            push_box_segment(
                verts,
                idxs,
                [road_x, ya - 80.0 - h, za],
                [road_x, yb - 80.0 - h, zb],
                8600.0,
                h,
                shoulder_col,
            );
        }
        push_box_segment(verts, idxs, [road_x, ya - 80.0, za], [road_x, yb - 80.0, zb], 7000.0, 80.0, road_col);
        for sx in [-3250.0_f32, 3250.0] {
            push_box_segment(verts, idxs, [road_x + sx, ya - 2.0, za], [road_x + sx, yb - 2.0, zb], 150.0, 17.0, line_col);
        }
        if i % 3 == 0 {
            push_box_segment(verts, idxs, [road_x, ya - 2.0, za], [road_x, yb - 2.0, zb], 150.0, 17.0, line_col);
        }
    }
    // Viaduct deck, parapets and piers
    let deck_top = g + 200.0 + bridge_lift;
    let sandstone_col = [0.70, 0.64, 0.53];
    push_box(verts, idxs, [road_x, deck_top - 80.0 - 325.0, br_z], [8800.0, 650.0, 36000.0], sandstone_col);
    for sx in [-4050.0_f32, 4050.0] {
        push_box(verts, idxs, [road_x + sx, deck_top - 80.0 + 550.0, br_z], [450.0, 1100.0, 36000.0], [0.54, 0.50, 0.45]);
    }
    let pier_h = deck_top - 80.0 - 650.0 - g;
    for k in 0..6 {
        let pz = br_z - 15000.0 + k as f32 * 6000.0;
        push_box(verts, idxs, [road_x, g + pier_h * 0.5, pz], [8000.0, pier_h, 1900.0], stone_col);
    }
    // Hall driveway: 6.5 m wide apron from the country road to the west hall door
    let drive_x0 = road_x + 4300.0;
    let drive_x1 = cx - hw_hall - 480.0;
    push_box(
        verts,
        idxs,
        [(drive_x0 + drive_x1) * 0.5, g + 100.0, cz],
        [(drive_x1 - drive_x0).abs(), 200.0, 6500.0],
        apron_col,
    );

    // Alpine village (1:1 chalets 10-13 m wide) + chapel
    let chalets = [
        (cx + hw_hall + 16000.0, cz + 12000.0, 0.12_f32, 10400.0_f32, 8600.0_f32, 5800.0_f32, [0.58, 0.22, 0.16]),
        (cx + hw_hall + 28000.0, cz + 22000.0, -0.35, 11200.0, 9000.0, 6200.0, [0.36, 0.34, 0.32]),
        (cx + hw_hall + 14000.0, cz + 30000.0, 0.55, 10800.0, 8800.0, 6000.0, [0.60, 0.25, 0.18]),
        (cx + hw_hall + 34000.0, cz + 6000.0, -0.18, 12000.0, 9600.0, 6600.0, [0.42, 0.32, 0.24]),
        (cx + hw_hall + 44000.0, cz + 18000.0, 0.25, 13200.0, 10400.0, 6800.0, [0.54, 0.24, 0.17]),
    ];
    let mut excl: Vec<(f32, f32, f32)> = Vec::new();
    for &(vx, vz, vrot, vw, vd, vh, roof_c) in &chalets {
        let (vy, is_w) = eval_env_terrain_y(vx, vz, cx, cz, hw_hall, hl_hall, river_z_base, lake_cx, lake_cz, floor_y);
        if !is_w {
            push_alpine_chalet(verts, idxs, vx, vy, vz, vrot, vw, vd, vh, roof_c);
        }
        excl.push((vx, vz, 11500.0));
    }
    let ch_x = cx + hw_hall + 30000.0;
    let ch_z = cz + 38000.0;
    let (ch_y, ch_w) = eval_env_terrain_y(ch_x, ch_z, cx, cz, hw_hall, hl_hall, river_z_base, lake_cx, lake_cz, floor_y);
    if !ch_w {
        push_alpine_chapel(verts, idxs, ch_x, ch_y, ch_z);
    }
    excl.push((ch_x, ch_z, 16000.0));

    // Red alpine barn (16.5 x 11.5 m, 7.4 m walls) with a silo west of the country road
    let bx = road_x - 24000.0;
    let bz = cz - 14000.0;
    let (by, bw) = eval_env_terrain_y(bx, bz, cx, cz, hw_hall, hl_hall, river_z_base, lake_cx, lake_cz, floor_y);
    if !bw {
        push_box(verts, idxs, [bx, by + 3700.0, bz], [16500.0, 7400.0, 11500.0], [0.62, 0.18, 0.14]);
        push_pitched_roof_3d(verts, idxs, bx, by + 7400.0, bz, 17500.0, 12500.0, 4500.0, 0.0, [0.36, 0.32, 0.28], [0.62, 0.18, 0.14]);
        push_box(verts, idxs, [bx + 11000.0, by + 7250.0, bz - 3000.0], [4200.0, 14500.0, 4200.0], [0.82, 0.80, 0.76]);
        push_box(verts, idxs, [bx + 11000.0, by + 15300.0, bz - 3000.0], [4800.0, 1600.0, 4800.0], [0.42, 0.44, 0.48]);
    }
    excl.push((bx + 4000.0, bz, 17000.0));
    excl.push((bh_x, pier_z, 10000.0));

    // Multi-lobe volumetric clouds high above the valley
    let cloud_col = if dark_mode { [0.32, 0.38, 0.50] } else { [0.97, 0.98, 1.0] };
    for c in 0..14 {
        let ang = (c as f32 / 14.0) * std::f32::consts::TAU + 0.35;
        let cl_rad = (hall_r + 38000.0) + ((c * 5) % 4) as f32 * 38000.0;
        let clx = cx + ang.cos() * cl_rad;
        let clz = cz + ang.sin() * cl_rad;
        let cly = 65000.0 + ((c * 3) % 5) as f32 * 7500.0;
        push_rotated_box(verts, idxs, [clx, cly, clz], ang, [0.0, 0.0, 0.0], [26000.0, 3800.0, 14500.0], cloud_col);
        push_rotated_box(verts, idxs, [clx + 6000.0, cly + 1700.0, clz - 2400.0], ang + 0.3, [0.0, 0.0, 0.0], [18000.0, 3400.0, 11500.0], cloud_col);
        push_rotated_box(verts, idxs, [clx - 6700.0, cly + 1200.0, clz + 2600.0], ang - 0.2, [0.0, 0.0, 0.0], [15000.0, 3100.0, 10000.0], cloud_col);
    }

    // Mixed 1:1 spruce (12.5-22.5 m) & broadleaf (9.5-17 m) forest rings on the flat valley floor
    for ring in 0..5 {
        let ring_r = hall_r + 9000.0 + (ring as f32) * 16000.0;
        let count = 70 + ring * 30;
        for k in 0..count {
            let ang = (k as f32 / count as f32) * std::f32::consts::TAU + (ring as f32) * 0.27;
            let jitter = (((k * 13 + ring * 7) % 11) as f32 - 5.0) * 1400.0;
            let rx = cx + ang.cos() * (ring_r + jitter);
            let rz = cz + ang.sin() * (ring_r + jitter);
            if (rx - cx).abs() < hw_hall + 10000.0 && (rz - cz).abs() < hl_hall + 10000.0 {
                continue;
            }
            if (rx - road_x).abs() < 7500.0 && (rz - cz).abs() < 102000.0 {
                continue;
            }
            if rx < cx - hw_hall && rx > road_x && (rz - cz).abs() < 6500.0 {
                continue;
            }
            if (((rx - lake_cx) / 31000.0).powi(2) + ((rz - lake_cz) / 22000.0).powi(2)).sqrt() < 1.0 {
                continue;
            }
            if (rz - env_river_center_z(rx, cx, river_z_base)).abs() < 15500.0 {
                continue;
            }
            if excl.iter().any(|&(ex, ez, er)| (rx - ex).hypot(rz - ez) < er) {
                continue;
            }
            let (ty, is_w) = eval_env_terrain_y(rx, rz, cx, cz, hw_hall, hl_hall, river_z_base, lake_cx, lake_cz, floor_y);
            if is_w || (ty - g) > 24000.0 {
                continue;
            }
            let tree_h = 12500.0 + (((k + ring * 3) % 5) as f32) * 2500.0;
            if (k + ring) % 4 == 0 {
                push_env_deciduous_tree(verts, idxs, rx, ty, rz, tree_h * 0.76, k + ring * 19);
            } else {
                push_env_pine(verts, idxs, rx, ty, rz, tree_h);
            }
        }
    }
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

