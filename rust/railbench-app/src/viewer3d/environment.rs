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
    let w_hall = (w_table + 4600.0).max(8600.0);
    let l_hall = (l_table + 5200.0).max(9400.0);
    let hw_hall = w_hall * 0.5;
    let hl_hall = l_hall * 0.5;
    let hall_r = hw_hall.hypot(hl_hall);
    let floor_y = -755.0_f32;
    let wall_h = 3200.0_f32;
    let arch_rise = 1350.0_f32;

    let concrete = if dark_mode { [0.18, 0.21, 0.27] } else { [0.36, 0.40, 0.46] };
    let steel = if dark_mode { [0.12, 0.16, 0.22] } else { [0.20, 0.25, 0.33] };
    let brick = if dark_mode { [0.28, 0.16, 0.13] } else { [0.43, 0.22, 0.17] };
    let yellow = [0.88, 0.62, 0.08];
    let sandstone = [0.68, 0.62, 0.52];

    // 1. Industrial Exhibition Hall (floor slab, exterior apron, brick lower walls, gable portals, arches, crane)
    push_box(verts, idxs, [cx, floor_y - 12.0, cz], [w_hall, 24.0, l_hall], concrete);
    push_box(verts, idxs, [cx, floor_y - 18.0, cz], [w_hall + 640.0, 14.0, l_hall + 640.0], concrete);
    for i in -3..=3 {
        let sx = cx + (i as f32) * (w_hall / 7.0);
        push_box(verts, idxs, [sx, floor_y + 0.6, cz], [8.0, 1.2, l_hall - 80.0], [0.12, 0.14, 0.18]);
    }
    let sw = w_table + 720.0;
    let sl = l_table + 720.0;
    push_box(verts, idxs, [cx, floor_y + 1.5, cz - sl * 0.5], [sw, 3.0, 26.0], yellow);
    push_box(verts, idxs, [cx, floor_y + 1.5, cz + sl * 0.5], [sw, 3.0, 26.0], yellow);
    push_box(verts, idxs, [cx - sw * 0.5, floor_y + 1.5, cz], [26.0, 3.0, sl], yellow);
    push_box(verts, idxs, [cx + sw * 0.5, floor_y + 1.5, cz], [26.0, 3.0, sl], yellow);

    // Lower brick wainscoting walls + sandstone coping trim + longitudinal steel eaves beams
    push_box(verts, idxs, [cx, floor_y + 520.0, cz - hl_hall], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [cx, floor_y + 520.0, cz + hl_hall], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [cx - hw_hall, floor_y + 520.0, cz], [120.0, 1040.0, l_hall], brick);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 520.0, cz], [120.0, 1040.0, l_hall], brick);
    push_box(verts, idxs, [cx - hw_hall, floor_y + 1052.0, cz], [140.0, 24.0, l_hall + 20.0], sandstone);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 1052.0, cz], [140.0, 24.0, l_hall + 20.0], sandstone);
    for sx in [-1.0_f32, 1.0] {
        push_box(verts, idxs, [cx + sx * hw_hall, floor_y + wall_h, cz], [95.0, 95.0, l_hall], steel);
    }

    // Gable end portals: 2-track locomotive rolling doors + night-illuminated station clock at north/south ends
    for sz in [-1.0_f32, 1.0] {
        let gz = cz + sz * (hl_hall - 40.0);
        for dx in [-950.0_f32, 950.0] {
            push_box(verts, idxs, [cx + dx, floor_y + 820.0, gz], [960.0, 1640.0, 70.0], [0.18, 0.24, 0.20]);
            push_box(verts, idxs, [cx + dx, floor_y + 1650.0, gz], [1020.0, 44.0, 96.0], sandstone);
        }
        let clk_y = floor_y + wall_h + arch_rise * 0.48;
        push_box(verts, idxs, [cx, clk_y, gz], [380.0, 380.0, 56.0], steel);
        push_rotated_box_mat(
            verts,
            idxs,
            [cx, clk_y, gz - sz * 32.0],
            0.0,
            [0.0, 0.0, 0.0],
            [310.0, 310.0, 14.0],
            [0.96, 0.95, 0.88],
            [0.30, 0.05, -0.85],
        );
        push_box(verts, idxs, [cx, clk_y + 52.0, gz - sz * 42.0], [18.0, 120.0, 12.0], [0.10, 0.12, 0.15]);
        push_box(verts, idxs, [cx + 46.0, clk_y, gz - sz * 42.0], [105.0, 16.0, 12.0], [0.10, 0.12, 0.15]);
    }

    let bays = 4;
    let arch_segs = 10;
    for i in 0..=bays {
        let pz = (cz - hl_hall) + (i as f32 / bays as f32) * l_hall;
        for sx in [-1.0_f32, 1.0] {
            push_box(verts, idxs, [cx + sx * hw_hall, floor_y + wall_h * 0.5, pz], [160.0, wall_h, 160.0], steel);
        }
        push_box(verts, idxs, [cx, floor_y + wall_h, pz], [w_hall + 160.0, 95.0, 95.0], steel);
        for s in 0..arch_segs {
            let u0 = s as f32 / arch_segs as f32;
            let u1 = (s + 1) as f32 / arch_segs as f32;
            let x0 = cx + (u0 - 0.5) * w_hall;
            let x1 = cx + (u1 - 0.5) * w_hall;
            let y0 = floor_y + wall_h + (u0 * std::f32::consts::PI).sin() * arch_rise;
            let y1 = floor_y + wall_h + (u1 * std::f32::consts::PI).sin() * arch_rise;
            push_box_segment(verts, idxs, [x0, y0, pz], [x1, y1, pz], 90.0, 115.0, steel);
            if s > 0 {
                push_box_segment(verts, idxs, [x0, floor_y + wall_h, pz], [x0, y0, pz], 42.0, 42.0, steel);
            }
        }
        // Suspended industrial pendant lamps (emissive at night via mat.z < 0)
        for lx_off in [-w_hall * 0.24, 0.0, w_hall * 0.24] {
            let ly = floor_y + wall_h - 140.0;
            push_box(verts, idxs, [cx + lx_off, ly + 70.0, pz], [12.0, 140.0, 12.0], steel);
            push_box(verts, idxs, [cx + lx_off, ly, pz], [150.0, 42.0, 150.0], [0.22, 0.26, 0.32]);
            push_rotated_box_mat(
                verts,
                idxs,
                [cx + lx_off, ly - 22.0, pz],
                0.0,
                [0.0, 0.0, 0.0],
                [115.0, 16.0, 115.0],
                [1.0, 0.93, 0.72],
                [0.20, 0.05, -1.15],
            );
        }
    }

    // Open industrial clerestory window frames & skylight crown purlins (transparent view to alpine landscape outside)
    for i in 0..bays {
        let z_mid = (cz - hl_hall) + ((i as f32 + 0.5) / bays as f32) * l_hall;
        let bay_l = (l_hall / bays as f32) - 180.0;
        for sx in [-1.0_f32, 1.0] {
            let wx = cx + sx * hw_hall;
            push_box(verts, idxs, [wx, floor_y + 2050.0, z_mid], [52.0, 42.0, bay_l], steel);
            for m in -1..=1 {
                let mz = z_mid + (m as f32) * (bay_l * 0.32);
                push_box(verts, idxs, [wx, floor_y + 2130.0, mz], [48.0, 2140.0, 32.0], steel);
            }
        }
        let crown_y = floor_y + wall_h + arch_rise + 80.0;
        for px_off in [-560.0_f32, 560.0] {
            push_box(verts, idxs, [cx + px_off, crown_y, z_mid], [48.0, 48.0, bay_l], steel);
        }
    }

    // Longitudinal crane runway rails + yellow double-girder bridge crane
    let crane_y = floor_y + wall_h - 340.0;
    let rail_span = w_hall - 280.0;
    for sx in [-1.0_f32, 1.0] {
        push_box(verts, idxs, [cx + sx * rail_span * 0.5, crane_y, cz], [75.0, 95.0, l_hall - 160.0], steel);
    }
    let crane_z = cz - l_hall * 0.18;
    push_box(verts, idxs, [cx, crane_y + 110.0, crane_z - 150.0], [rail_span, 145.0, 90.0], yellow);
    push_box(verts, idxs, [cx, crane_y + 110.0, crane_z + 150.0], [rail_span, 145.0, 90.0], yellow);
    let trolley_x = cx + w_table * 0.15;
    push_box(verts, idxs, [trolley_x, crane_y + 210.0, crane_z], [380.0, 110.0, 380.0], [0.20, 0.25, 0.32]);
    push_box(verts, idxs, [trolley_x, crane_y - 210.0, crane_z], [14.0, 740.0, 14.0], [0.15, 0.17, 0.20]);
    push_box(verts, idxs, [trolley_x, crane_y - 600.0, crane_z], [95.0, 110.0, 65.0], yellow);

    // 2. Surrounding Alpine Valley Landscape (strictly outside hall footprint, visible through open windows)
    let meadow = if dark_mode { [0.10, 0.21, 0.11] } else { [0.28, 0.45, 0.14] };
    let hill = if dark_mode { [0.09, 0.17, 0.10] } else { [0.22, 0.36, 0.12] };
    let rock = if dark_mode { [0.20, 0.25, 0.32] } else { [0.38, 0.44, 0.50] };
    let snow = [0.92, 0.94, 0.98];
    let river_col = if dark_mode { [0.10, 0.28, 0.44] } else { [0.18, 0.48, 0.72] };
    let bank_col = if dark_mode { [0.36, 0.33, 0.24] } else { [0.68, 0.62, 0.46] };

    // Broad valley ground plane sitting just below hall concrete floor & exterior apron
    let ground_span = (hall_r * 2.0 + 24000.0).max(34000.0);
    push_box(verts, idxs, [cx, floor_y - 34.0, cz], [ground_span, 24.0, ground_span], meadow);

    // Scenic outdoor lake + sandy shore outside east/northeast hall windows
    let lake_cx = cx + hw_hall + 2800.0;
    let lake_cz = cz - hl_hall * 0.45;
    push_box(verts, idxs, [lake_cx, floor_y - 20.0, lake_cz], [4450.0, 8.0, 3050.0], bank_col);
    push_rotated_box_mat(
        verts,
        idxs,
        [lake_cx, floor_y - 17.0, lake_cz],
        0.0,
        [0.0, 0.0, 0.0],
        [4200.0, 10.0, 2800.0],
        river_col,
        MAT_WATER,
    );

    // Winding valley river + sandy banks strictly south of the hall
    let river_z_base = cz + hl_hall + 2300.0;
    let segs = 18;
    for i in 0..segs {
        let u0 = (i as f32 / segs as f32) - 0.5;
        let u1 = ((i + 1) as f32 / segs as f32) - 0.5;
        let x0 = cx + u0 * 22000.0;
        let x1 = cx + u1 * 22000.0;
        let z0 = river_z_base + (u0 * 6.0).sin() * 680.0;
        let z1 = river_z_base + (u1 * 6.0).sin() * 680.0;
        push_box_segment(verts, idxs, [x0, floor_y - 20.0, z0], [x1, floor_y - 20.0, z1], 920.0, 4.0, bank_col);
        push_box_segment_mat(verts, idxs, [x0, floor_y - 17.0, z0], [x1, floor_y - 17.0, z1], 680.0, 6.0, river_col, MAT_WATER);
    }

    // Country road & stone arch bridge strictly west of the hall
    let br_x = cx - hw_hall - 1500.0;
    let br_z = river_z_base + (((br_x - cx) / 22000.0) * 6.0_f32).sin() * 680.0;
    push_box(verts, idxs, [br_x, floor_y - 18.0, cz], [340.0, 8.0, 20000.0], [0.28, 0.30, 0.34]);
    push_box(verts, idxs, [br_x, floor_y + 90.0, br_z], [440.0, 210.0, 960.0], [0.48, 0.46, 0.44]);
    push_box(verts, idxs, [br_x - 200.0, floor_y + 220.0, br_z], [36.0, 65.0, 960.0], [0.42, 0.40, 0.38]);
    push_box(verts, idxs, [br_x + 200.0, floor_y + 220.0, br_z], [36.0, 65.0, 960.0], [0.42, 0.40, 0.38]);

    // Trackside alpine cottage with pitched roof & night-lit windows outside east hall windows
    let hx = cx + hw_hall + 1550.0;
    let hz = cz + 650.0;
    push_box(verts, idxs, [hx, floor_y + 220.0, hz], [640.0, 480.0, 520.0], [0.88, 0.85, 0.78]);
    push_box(verts, idxs, [hx, floor_y + 520.0, hz], [720.0, 140.0, 600.0], [0.58, 0.22, 0.16]);
    push_rotated_box_mat(
        verts,
        idxs,
        [hx - 322.0, floor_y + 240.0, hz],
        0.0,
        [0.0, 0.0, 0.0],
        [12.0, 110.0, 140.0],
        [1.0, 0.88, 0.52],
        [0.25, 0.05, -0.95],
    );

    // Soft volumetric low-poly clouds floating above hall roof crown
    let cloud_col = if dark_mode { [0.34, 0.40, 0.52] } else { [0.96, 0.98, 1.0] };
    let cloud_r = (hall_r + 2400.0).max(8000.0);
    for c in 0..8 {
        let ang = (c as f32 / 8.0) * std::f32::consts::TAU + 0.4;
        let clx = cx + ang.cos() * cloud_r;
        let clz = cz + ang.sin() * (cloud_r * 0.92);
        let cly = 4900.0 + ((c % 3) as f32) * 480.0;
        push_box(verts, idxs, [clx, cly, clz], [1400.0, 220.0, 820.0], cloud_col);
        push_box(verts, idxs, [clx + 360.0, cly + 90.0, clz - 120.0], [920.0, 200.0, 640.0], cloud_col);
    }

    // Distant alpine ridge peaks encircling the hall & valley
    for k in 0..12 {
        let ang = (k as f32 / 12.0) * std::f32::consts::TAU;
        let dist = hall_r + 4200.0 + ((k % 3) as f32) * 1500.0;
        let px = cx + ang.cos() * dist;
        let pz = cz + ang.sin() * dist;
        let mh = 2200.0 + ((k * 7) % 5) as f32 * 450.0;
        push_box(verts, idxs, [px, floor_y + mh * 0.25, pz], [3200.0, mh * 0.5, 2800.0], hill);
        push_box(verts, idxs, [px, floor_y + mh * 0.62, pz], [1800.0, mh * 0.32, 1600.0], rock);
        push_box(verts, idxs, [px, floor_y + mh * 0.84, pz], [820.0, mh * 0.18, 740.0], snow);
    }

    // Staggered pine forest rings surrounding the hall (skipping road, river, lake & cottage)
    for ring in 0..2 {
        let ring_off = (ring as f32) * 1350.0;
        for k in 0..14 {
            let ang = (k as f32 / 14.0) * std::f32::consts::TAU + (ring as f32) * 0.22;
            let rx = cx + ang.cos() * (hw_hall + 1050.0 + ring_off);
            let rz = cz + ang.sin() * (hl_hall + 1050.0 + ring_off);
            let in_road = (rx - br_x).abs() < 420.0;
            let in_river = (rz - river_z_base).abs() < 1150.0;
            let in_lake = (rx - lake_cx).abs() < 2350.0 && (rz - lake_cz).abs() < 1600.0;
            let in_cottage = (rx - hx).abs() < 600.0 && (rz - hz).abs() < 550.0;
            if in_road || in_river || in_lake || in_cottage {
                continue;
            }
            push_env_pine(verts, idxs, rx, floor_y - 22.0, rz, 1150.0 + (((k + ring) % 4) as f32) * 280.0);
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
    let dist = 16500.0_f32;
    let sun_pos = [sx / slen * dist, sy / slen * dist, sz / slen * dist];
    if sun_pos[1] > -1200.0 {
        push_rotated_box_mat(verts, idxs, sun_pos, 0.0, [0.0, 0.0, 0.0], [720.0, 720.0, 720.0], [1.0, 0.95, 0.68], [0.10, 0.0, 1.35]);
        push_rotated_box_mat(verts, idxs, sun_pos, 0.78, [0.0, 0.0, 0.0], [680.0, 680.0, 680.0], [1.0, 0.90, 0.52], [0.10, 0.0, 1.25]);
    }
    let moon_pos = [-sun_pos[0], -sun_pos[1], -sun_pos[2]];
    if moon_pos[1] > -1200.0 {
        push_rotated_box_mat(verts, idxs, moon_pos, 0.35, [0.0, 0.0, 0.0], [520.0, 520.0, 520.0], [0.82, 0.90, 1.0], [0.15, 0.0, 1.15]);
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

