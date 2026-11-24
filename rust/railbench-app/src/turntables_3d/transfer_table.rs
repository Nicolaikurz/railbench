use super::*;

pub(crate) fn push_transfer_table_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    layout: &Layout,
    e0: f64,
    bridge_z_override: Option<f64>,
) {
    let l = entry.and_then(|e| e.length).unwrap_or(360.0) as f32;
    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
    let hg = gauge * 0.5;
    let track_zs = extract_transfer_track_zs(entry);
    let min_tz = track_zs[0];
    let max_tz = *track_zs.last().unwrap_or(&309.6);
    let center_z = 0.5 * (min_tz + max_tz);
    let top_z = min_tz - 55.2;
    let bot_z = max_tz + 55.2;
    let plate_d = bot_z - top_z;

    let pit_left = 36.0_f32;
    let pit_right = l - 36.0;
    let pit_w = pit_right - pit_left;
    let pit_top = top_z + 6.0;
    let pit_bot = bot_z - 6.0;
    let pit_d = pit_bot - pit_top;
    let pit_depth = 12.0_f32;
    let bridge_l = pit_w - 2.0;
    let bridge_cx = l * 0.5;
    let bridge_z = bridge_z_override
        .map(|v| v as f32)
        .unwrap_or(track_zs[track_zs.len() / 2]);
    let rail_xs = [pit_left + 44.0, l * 0.5, pit_right - 44.0];
    let girder_z_off = (hg + 4.2).max(12.6);

    let p = SharedPalette::new(piece.has_tunnel == Some(true));
    let mut b = ModelBuilder::new(
        verts,
        idxs,
        [piece.transform.x as f32, e0 as f32, piece.transform.y as f32],
        (piece.transform.rotation as f32).to_radians(),
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    );

    // 1. Left & right gravel approach aprons + pit-edge concrete coping + top/bottom headwalls (z-fight free)
    let apron_w = (pit_left - 3.5).max(8.0);
    b.rbox([apron_w * 0.5, 2.75, center_z], [apron_w, 5.48, plate_d], p.ballast);
    b.rbox([l - apron_w * 0.5, 2.75, center_z], [apron_w, 5.48, plate_d], p.ballast);

    b.rbox([pit_left - 1.75, 3.1, center_z], [3.5, 6.2, pit_d + 2.0], p.concrete);
    b.rbox([pit_right + 1.75, 3.1, center_z], [3.5, 6.2, pit_d + 2.0], p.concrete);
    b.rbox([bridge_cx, 3.2, top_z + 3.0], [pit_w + 7.0, 6.4, 6.0], p.concrete);
    b.rbox([bridge_cx, 3.2, bot_z - 3.0], [pit_w + 7.0, 6.4, 6.0], p.concrete);

    // 2. Recessed concrete transfer pit: floor slab, 4 solid walls, pilasters, ladder, busbar & sump
    b.rbox([bridge_cx, -pit_depth - 1.25, center_z], [pit_w + 6.0, 2.5, pit_d + 6.0], p.pit_floor);
    let wall_h = pit_depth + 5.8;
    let wall_cy = -pit_depth + wall_h * 0.5;
    b.rbox([pit_left - 1.5, wall_cy, center_z], [3.0, wall_h, pit_d + 4.0], p.pit_wall);
    b.rbox([pit_right + 1.5, wall_cy, center_z], [3.0, wall_h, pit_d + 4.0], p.pit_wall);
    b.rbox([bridge_cx, wall_cy, pit_top - 1.5], [pit_w + 4.0, wall_h, 3.0], p.pit_wall);
    b.rbox([bridge_cx, wall_cy, pit_bot + 1.5], [pit_w + 4.0, wall_h, 3.0], p.pit_wall);

    // Interior concrete wall pilasters between approach bays
    for w in track_zs.windows(2) {
        let pz = 0.5 * (w[0] + w[1]);
        b.rbox([pit_left + 1.0, wall_cy - 0.2, pz], [2.2, wall_h - 0.6, 5.0], p.concrete_dark);
        b.rbox([pit_right - 1.0, wall_cy - 0.2, pz], [2.2, wall_h - 0.6, 5.0], p.concrete_dark);
    }
    // Copper collector busbar along left pit wall + maintenance ladder on top headwall
    b.rbox([pit_left + 1.2, -3.8, center_z], [0.9, 1.2, pit_d - 14.0], p.bronze);
    let lad_x = bridge_cx + 34.0;
    for lx in [lad_x - 2.0, lad_x + 2.0] {
        b.rbox([lx, wall_cy, pit_top + 1.1], [0.7, wall_h - 1.0, 0.7], p.steel_dark);
    }
    for rung in 0..5 {
        let ry = -pit_depth + 2.2 + rung as f32 * (wall_h - 4.0) / 4.0;
        b.rbox([lad_x, ry, pit_top + 1.1], [4.4, 0.6, 0.6], p.steel_dark);
    }

    // Dual longitudinal drainage channels + grates flanking center traverse rail
    for dx in [-14.0_f32, 14.0] {
        b.rbox([bridge_cx + dx, -pit_depth + 0.18, center_z], [3.8, 0.36, pit_d - 12.0], p.sump);
        for k in 0..8 {
            let gz = pit_top + 26.0 + (pit_d - 52.0) * (k as f32 / 7.0);
            b.rbox([bridge_cx + dx, -pit_depth + 0.42, gz], [3.4, 0.25, 6.0], p.grate);
        }
    }

    // 3. Three raised concrete traverse-rail beams, metallic traverse rails & 6 end wheel-stop bumpers
    let rail_top_y = -pit_depth + 4.2; // -7.8 mm
    let curb_len = pit_d - 6.0;
    for &rx in &rail_xs {
        b.rbox([rx, -pit_depth + 1.1, center_z], [7.2, 2.2, curb_len], p.concrete);
        b.rbox([rx, -pit_depth + 3.2, center_z], [1.8, 2.0, curb_len - 4.0], p.rail);
        for ez in [pit_top + 6.5, pit_bot - 6.5] {
            b.rbox([rx, -pit_depth + 4.6, ez], [6.4, 5.0, 5.2], p.steel_dark);
            b.rbox([rx, -pit_depth + 6.4, ez], [6.8, 1.5, 5.6], p.hazard_yellow);
        }
    }

    // 4. 10 Approach track stubs (sleepers strictly on aprons + flush 9.6 mm rails + locking shoes)
    let tie_w_std = 22.0 * (gauge / 16.5);
    for &tz in &track_zs {
        for ti in 0..4 {
            let u = ti as f32 / 3.0;
            let xl = 5.5 + (pit_left - 11.0) * u;
            let xr = pit_right + 5.5 + (l - pit_right - 11.0) * u;
            b.rbox([xl, 6.3, tz], [2.6, 1.6, tie_w_std], p.tie);
            b.rbox([xr, 6.3, tz], [2.6, 1.6, tie_w_std], p.tie);
        }
        for side in [-1.0_f32, 1.0] {
            let rz = tz + side * hg;
            b.rbox([pit_left * 0.5, 8.35, rz], [pit_left, 2.5, 1.4], p.rail);
            b.rbox([pit_right + (l - pit_right) * 0.5, 8.35, rz], [l - pit_right, 2.5, 1.4], p.rail);
            b.rbox([pit_left - 1.5, 6.65, rz], [3.0, 0.9, 2.8], p.steel_dark);
            b.rbox([pit_right + 1.5, 6.65, rz], [3.0, 0.9, 2.8], p.steel_dark);
        }
        b.rbox([pit_left - 1.4, 6.55, tz], [2.4, 0.8, gauge * 0.50], p.bronze);
        b.rbox([pit_right + 1.4, 6.55, tz], [2.4, 0.8, gauge * 0.50], p.bronze);
    }

    // 5. Six flanged steel traverse wheels (axles along X!) + drive shaft + motor/gearbox
    let wheel_r = 3.2_f32;
    let flange_r = 3.85_f32;
    let wheel_y = rail_top_y + wheel_r; // -4.6 mm -> rests directly on top of -7.8 mm traverse rail
    for &rx in &rail_xs {
        for sz in [-1.0_f32, 1.0] {
            let wz = bridge_z + sz * girder_z_off;
            b.cyl_x([rx, wheel_y, wz], wheel_r, wheel_r, 2.0, 14, p.wheel_steel);
            b.cyl_x([rx - 1.15, wheel_y, wz], flange_r, flange_r, 0.5, 14, p.steel_dark);
            b.cyl_x([rx + 1.15, wheel_y, wz], flange_r, flange_r, 0.5, 14, p.steel_dark);
            b.rbox([rx, wheel_y + 2.4, wz], [5.4, 4.4, 4.4], p.steel_dark);
        }
    }
    // Full-length X drive shaft connecting the +Z bogie wheels + electric drive motor & bevel gearbox
    let shaft_z = bridge_z + girder_z_off;
    b.cyl_x([bridge_cx, wheel_y, shaft_z], 0.95, 0.95, rail_xs[2] - rail_xs[0] + 10.0, 10, p.wheel_steel);
    let motor_x = bridge_cx - 28.0;
    b.rbox([motor_x + 7.0, wheel_y + 0.5, shaft_z + 2.2], [6.5, 5.2, 6.2], p.steel_dark);
    b.cyl_x([motor_x - 2.0, wheel_y + 0.6, shaft_z + 4.4], 3.4, 3.4, 10.5, 12, p.steel_girder);
    b.cyl_x([motor_x - 7.8, wheel_y + 0.6, shaft_z + 4.4], 2.6, 3.4, 1.4, 10, p.bronze);

    // 6. Heavy flanged bridge plate girders, head-beams, deck & symmetrically centered ties/rails
    let girder_l = bridge_l - 2.0;
    for sz in [-1.0_f32, 1.0] {
        let gz = bridge_z + sz * girder_z_off;
        b.rbox([bridge_cx, 3.2, gz], [girder_l, 10.8, 2.0], p.steel_girder);
        b.rbox([bridge_cx, 8.9, gz], [girder_l + 1.0, 1.1, 3.4], p.steel_dark);
        b.rbox([bridge_cx, -2.3, gz], [girder_l + 1.0, 1.2, 3.6], p.steel_dark);
        for st in 0..=12 {
            let sx = (pit_left + 4.0) + (bridge_l - 8.0) * (st as f32 / 12.0);
            b.rbox([sx, 3.2, gz], [1.5, 10.8, 3.2], p.steel_dark);
        }
    }
    for cx_beam in 0..=10 {
        let bx = (pit_left + 3.0) + (bridge_l - 6.0) * (cx_beam as f32 / 10.0);
        b.rbox([bx, 3.4, bridge_z], [2.2, 2.4, girder_z_off * 2.0], p.steel_dark);
    }
    for ex in [pit_left + 2.2, pit_right - 2.2] {
        b.rbox([ex, 5.5, bridge_z], [2.4, 2.4, (girder_z_off + 8.0) * 2.0], p.steel_dark);
        b.rbox([ex, 6.3, bridge_z], [1.8, 1.0, gauge * 0.48], p.bronze);
    }
    b.rbox([bridge_cx, 5.15, bridge_z], [bridge_l - 1.0, 0.7, girder_z_off * 2.0 - 1.2], p.steel_deck);

    let tie_w_bridge = 21.0 * (gauge / 16.5);
    let tie_span = bridge_l - 10.0;
    let n_ties = ((tie_span / 8.0).round() as usize).max(4);
    for i in 0..=n_ties {
        let tx = bridge_cx - tie_span * 0.5 + tie_span * (i as f32 / n_ties as f32);
        b.rbox([tx, 6.3, bridge_z], [2.6, 1.6, tie_w_bridge], p.tie);
    }
    for side in [-1.0_f32, 1.0] {
        b.rbox([bridge_cx, 8.35, bridge_z + side * hg], [bridge_l, 2.5, 1.4], p.rail);
    }

    // 7. Side walkways, 2-bar safety handrails, hazard end plates & Gleissperrsignale
    let walk_w = 7.4_f32;
    let walk_z_off = girder_z_off + 4.4;
    let rail_z_off = girder_z_off + 7.7;
    let cab_cx = bridge_cx - 34.0;
    let cab_w = 36.0_f32;
    let cab_d = 21.0_f32;
    let cab_cz = bridge_z + rail_z_off + cab_d * 0.5;

    for sz in [-1.0_f32, 1.0] {
        let wz = bridge_z + sz * walk_z_off;
        b.rbox([bridge_cx, 5.15, wz], [bridge_l - 4.0, 0.7, walk_w], p.steel_deck);
        b.rbox([bridge_cx, 6.2, bridge_z + sz * (walk_z_off + walk_w * 0.5 - 0.35)], [bridge_l - 4.0, 1.5, 0.7], p.steel_dark);
        for bx in 0..=8 {
            let x = (pit_left + 12.0) + (bridge_l - 24.0) * (bx as f32 / 8.0);
            b.strut_between([x, 4.9, bridge_z + sz * (rail_z_off - 0.4)], [x, 1.0, bridge_z + sz * girder_z_off], 1.2, 1.2, p.steel_dark);
        }
        // Yellow/black end hazard plates
        for ex in [pit_left + 2.0, pit_right - 2.0] {
            b.rbox([ex, 5.6, wz], [2.2, 2.0, walk_w], p.hazard_yellow);
        }
    }
    build_safety_handrail_x(&mut b, pit_left + 5.0, pit_right - 5.0, 5.5, bridge_z - rail_z_off, None, 1.0, p.steel_dark);
    build_safety_handrail_x(
        &mut b,
        pit_left + 5.0,
        pit_right - 5.0,
        5.5,
        bridge_z + rail_z_off,
        Some((cab_cx - cab_w * 0.5, cab_cx + cab_w * 0.5)),
        1.0,
        p.steel_dark,
    );
    build_gleissperrsignal(&mut b, pit_right - 7.5, 5.5, bridge_z - (girder_z_off + 5.2), 1.0, &p);
    build_gleissperrsignal(&mut b, pit_left + 7.5, 5.5, bridge_z + (girder_z_off + 5.2), 1.0, &p);

    // 8. Cantilevered steel-braced Wärterhaus operator cabin
    build_operator_cabin(&mut b, cab_cx, cab_cz, cab_w, cab_d, bridge_z + girder_z_off, 5.5, -2.2, 1.0, &p);

    // 9. Signature overhead steel power-collector portal arch at bridge center (top crossbeam at y = 85.0,
    // clearing both H0 rolling stock and y = 68.0 / 79.0 catenary wires!)
    let arch_h = 80.5_f32;
    let arch_top_y = 5.5 + arch_h; // 86.0 mm
    for sz in [-1.0_f32, 1.0] {
        let pz = bridge_z + sz * rail_z_off;
        b.rbox([bridge_cx, 5.5 + arch_h * 0.5, pz], [3.6, arch_h, 3.2], p.steel_girder);
        b.rbox([bridge_cx, 7.2, pz], [5.8, 3.4, 5.0], p.steel_dark);
        b.strut_between(
            [bridge_cx, arch_top_y - 12.0, pz],
            [bridge_cx, arch_top_y - 2.0, bridge_z + sz * (rail_z_off - 10.0)],
            1.6,
            1.6,
            p.steel_dark,
        );
    }
    let span_total = rail_z_off * 2.0 + 4.0;
    b.rbox([bridge_cx, arch_top_y - 1.2, bridge_z], [3.8, 2.8, span_total], p.steel_girder);
    b.rbox([bridge_cx, arch_top_y - 4.2, bridge_z], [2.4, 1.4, span_total - 4.0], p.steel_dark);
    for gz_off in [-rail_z_off * 0.65, 0.0, rail_z_off * 0.65] {
        b.strut_between([bridge_cx, arch_top_y - 4.2, bridge_z + gz_off], [bridge_cx, arch_top_y - 1.2, bridge_z + gz_off + 6.0], 1.1, 1.1, p.steel_dark);
        b.strut_between([bridge_cx, arch_top_y - 4.2, bridge_z + gz_off], [bridge_cx, arch_top_y - 1.2, bridge_z + gz_off - 6.0], 1.1, 1.1, p.steel_dark);
    }
    // Roof ceramic insulators + collector pantograph/busbar pick-up shoe
    for sz in [-1.0_f32, 1.0] {
        b.cyl_y([bridge_cx, arch_top_y + 1.5, bridge_z + sz * 8.0], 1.4, 1.8, 2.8, 8, p.insulator);
    }
    b.rbox([bridge_cx, arch_top_y + 3.4, bridge_z], [2.2, 1.0, 20.0], p.steel_dark);
    b.rbox([bridge_cx, arch_top_y + 5.6, bridge_z], [1.6, 0.9, 14.0], p.bronze);
    b.strut_between([bridge_cx, arch_top_y + 3.4, bridge_z - 7.0], [bridge_cx, arch_top_y + 5.6, bridge_z - 4.0], 0.7, 0.7, p.wheel_steel);
    b.strut_between([bridge_cx, arch_top_y + 3.4, bridge_z + 7.0], [bridge_cx, arch_top_y + 5.6, bridge_z + 4.0], 0.7, 0.7, p.wheel_steel);

    // 10. Optional Catenary on bridge + connected approach stubs
    if piece.has_catenary == Some(true) {
        let contact_h = 72.0_f32;
        let messenger_h = 85.0_f32;
        push_catenary_span_x(&mut b, pit_left + 1.0, pit_right - 1.0, bridge_z, contact_h, messenger_h, &p);
        build_catenary_portal(&mut b, pit_left + 12.0, 5.5, bridge_z, 0.0, 15.5, contact_h, messenger_h, &p);
        build_catenary_portal(&mut b, pit_right - 12.0, 5.5, bridge_z, 0.0, 15.5, contact_h, messenger_h, &p);

        let local_conns = entry.and_then(|e| e.local_connectors.as_ref());
        for (idx, conn) in piece.connectors.iter().enumerate() {
            let Some(ref cref) = conn.connected_to else {
                continue;
            };
            let other_cat = layout
                .pieces
                .iter()
                .find(|op| op.id == cref.piece_id)
                .map_or(false, |op| op.has_catenary == Some(true));
            if !other_cat {
                continue;
            }
            let lc = local_conns.and_then(|v| v.get(idx));
            let is_left = lc.map_or(idx < track_zs.len(), |c| c.x < bridge_cx as f64);
            let z = lc.map_or_else(
                || track_zs[idx % track_zs.len()],
                |c| c.y as f32,
            );
            if is_left {
                push_catenary_span_x(&mut b, 0.0, pit_left, z, contact_h, messenger_h, &p);
                build_catenary_portal(&mut b, pit_left - 2.5, 5.5, z, 0.0, 14.5, contact_h, messenger_h, &p);
                b.rbox([0.0, 44.0, z + 24.0], [2.8, 88.0, 2.8], p.cat_green);
                b.strut_between([0.0, messenger_h, z + 24.0], [0.0, messenger_h, z], 1.3, 1.3, p.cat_green);
                b.strut_between([0.0, contact_h - 3.0, z + 24.0], [0.0, messenger_h, z + 4.0], 1.1, 1.1, p.cat_green);
                b.strut_between([0.0, contact_h + 5.0, z + 14.0], [0.0, contact_h, z], 0.95, 0.95, p.cat_green);
            } else {
                push_catenary_span_x(&mut b, pit_right, l, z, contact_h, messenger_h, &p);
                build_catenary_portal(&mut b, pit_right + 2.5, 5.5, z, 0.0, 14.5, contact_h, messenger_h, &p);
                b.rbox([l, 44.0, z + 24.0], [2.8, 88.0, 2.8], p.cat_green);
                b.strut_between([l, messenger_h, z + 24.0], [l, messenger_h, z], 1.3, 1.3, p.cat_green);
                b.strut_between([l, contact_h - 3.0, z + 24.0], [l, messenger_h, z + 4.0], 1.1, 1.1, p.cat_green);
                b.strut_between([l, contact_h + 5.0, z + 14.0], [l, contact_h, z], 0.95, 0.95, p.cat_green);
            }
        }
    }
}

/// World-space 2D center of a transfer-table's bounding rectangle.
pub(crate) fn transfer_table_world_rect_corners(
    piece: &TrackPiece,
    entry: &CatalogEntry,
    margin_x: f64,
    margin_z: f64,
) -> [Vec2; 4] {
    let l = entry.length.unwrap_or(360.0);
    let zs = extract_transfer_track_zs(Some(entry));
    let min_tz = zs[0] as f64;
    let max_tz = *zs.last().unwrap_or(&309.6) as f64;
    let p_left = 36.0 - margin_x;
    let p_right = (l - 36.0) + margin_x;
    let p_top = (min_tz - 55.2) + 6.0 - margin_z;
    let p_bot = (max_tz + 55.2) - 6.0 + margin_z;
    let to_w = |lx: f64, ly: f64| -> Vec2 {
        let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
        Vec2 {
            x: piece.transform.x + rp.x,
            y: piece.transform.y + rp.y,
        }
    };
    [
        to_w(p_left, p_top),
        to_w(p_right, p_top),
        to_w(p_right, p_bot),
        to_w(p_left, p_bot),
    ]
}

pub(crate) fn turntable_outer_radius(piece: &TrackPiece, entry: Option<&CatalogEntry>) -> f64 {
    entry.and_then(|e| e.radius).unwrap_or_else(|| {
        piece
            .connectors
            .first()
            .map(|c| (c.x - piece.transform.x).hypot(c.y - piece.transform.y))
            .filter(|&r| r > 20.0)
            .unwrap_or(189.0)
    })
}

pub(crate) fn transfer_table_local_bounds(entry: Option<&CatalogEntry>) -> (f64, f64, f64, f64) {
    let l = entry.and_then(|e| e.length).unwrap_or(360.0);
    let zs = extract_transfer_track_zs(entry);
    let min_tz = zs[0] as f64;
    let max_tz = *zs.last().unwrap_or(&309.6) as f64;
    (0.0, l, min_tz - 55.2, max_tz + 55.2)
}

pub(crate) fn transfer_table_outer_world_corners(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
) -> [Vec2; 4] {
    let (min_x, max_x, top_z, bot_z) = transfer_table_local_bounds(entry);
    let to_w = |lx: f64, ly: f64| -> Vec2 {
        let rp = rotate(Vec2 { x: lx, y: ly }, piece.transform.rotation);
        Vec2 {
            x: piece.transform.x + rp.x,
            y: piece.transform.y + rp.y,
        }
    };
    [
        to_w(min_x, top_z),
        to_w(max_x, top_z),
        to_w(max_x, bot_z),
        to_w(min_x, bot_z),
    ]
}
