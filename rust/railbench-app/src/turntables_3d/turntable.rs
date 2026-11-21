use super::*;

pub(crate) fn push_turntable_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    e0: f64,
) {
    let outer_r = entry.and_then(|e| e.radius).unwrap_or(189.0) as f32;
    let scale_f = (outer_r / 189.0).clamp(0.4, 2.5);
    let pit_r = 139.0 * scale_f;
    let pit_depth = 15.0 * scale_f;
    let ring_rail_r = 124.0 * scale_f;
    let bridge_l = pit_r * 2.0 - 2.0; // 1 mm clearance at each pit edge
    let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
    let hg = gauge * 0.5;
    let girder_z = (hg + 4.2).max(12.6);

    let p = SharedPalette::new(piece.has_tunnel == Some(true));
    let mut b = ModelBuilder::new(
        verts,
        idxs,
        [piece.transform.x as f32, e0 as f32, piece.transform.y as f32],
        (piece.transform.rotation as f32).to_radians(),
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
    );

    // 1. Outer gravel ballast collar + concrete pit coping ring (no z-fighting overlap)
    b.annular_ring_y([0.0, 0.0, 0.0], pit_r + 5.6 * scale_f, outer_r, outer_r - 3.0 * scale_f, 0.02, 5.5, 48, p.ballast);
    b.annular_ring_y([0.0, 0.0, 0.0], pit_r - 0.5 * scale_f, pit_r + 6.5 * scale_f, pit_r + 6.0 * scale_f, 0.0, 6.2, 48, p.concrete);

    // 2. Concrete pit walls, 24 wall pilasters, pit maintenance ladder, stepped pit floor & sump
    b.annular_ring_y([0.0, 0.0, 0.0], pit_r, pit_r + 4.0 * scale_f, pit_r + 4.0 * scale_f, -pit_depth, 5.5, 48, p.pit_wall);
    b.cyl_y([0.0, -pit_depth - 1.5, 0.0], pit_r + 4.0 * scale_f, pit_r + 4.0 * scale_f, 3.0, 48, p.pit_floor);

    // Depressed center drainage ring & cast-iron grates
    b.annular_ring_y([0.0, 0.0, 0.0], 25.0 * scale_f, 35.0 * scale_f, 35.0 * scale_f, -pit_depth, -pit_depth + 0.35, 24, p.sump);
    for g in 0..8 {
        let ga = (g as f32 / 8.0) * std::f32::consts::TAU;
        b.obox(
            [30.0 * scale_f * ga.cos(), -pit_depth + 0.55, 30.0 * scale_f * ga.sin()],
            [8.0 * scale_f, 0.4, 3.0 * scale_f],
            [0.0, -ga, 0.0],
            p.grate,
        );
    }

    // 24 interior concrete wall buttress pilasters (skip ladder bay at ~250 deg)
    let wall_h = pit_depth + 5.4;
    for i in 0..24 {
        let deg = i as f32 * 15.0;
        if (deg - 250.0).abs() < 8.0 {
            continue;
        }
        let rad = deg.to_radians();
        b.obox(
            [(pit_r - 0.8 * scale_f) * rad.cos(), -pit_depth + wall_h * 0.5, (pit_r - 0.8 * scale_f) * rad.sin()],
            [2.0 * scale_f, wall_h, 3.2 * scale_f],
            [0.0, -rad, 0.0],
            p.concrete_dark,
        );
    }

    // Pit maintenance ladder on inner wall (at 250 deg)
    let lad_a = 250.0_f32.to_radians();
    let (lc, ls) = (lad_a.cos(), lad_a.sin());
    let lad_r = pit_r - 1.6 * scale_f;
    let lad_h = pit_depth + 5.0;
    for sz in [-2.0 * scale_f, 2.0 * scale_f] {
        b.rbox([lad_r * lc - sz * ls, -pit_depth + lad_h * 0.5, lad_r * ls + sz * lc], [0.7 * scale_f, lad_h, 0.7 * scale_f], p.steel_dark);
    }
    for rung in 0..6 {
        let ry = -pit_depth + 2.2 * scale_f + rung as f32 * (lad_h - 3.0 * scale_f) / 5.0;
        b.obox([lad_r * lc, ry, lad_r * ls], [0.6 * scale_f, 0.6 * scale_f, 4.4 * scale_f], [0.0, -lad_a, 0.0], p.steel_dark);
    }

    // 3. Raised concrete running-rail curb + metallic circular running rail + rail clip plates
    let curb_h = 2.4 * scale_f;
    b.annular_ring_y([0.0, 0.0, 0.0], ring_rail_r - 4.0 * scale_f, ring_rail_r + 4.4 * scale_f, ring_rail_r + 3.4 * scale_f, -pit_depth, -pit_depth + curb_h, 48, p.concrete);
    let rail_minor_r = 1.15 * scale_f;
    let pit_rail_cy = -pit_depth + curb_h + rail_minor_r;
    let pit_rail_top = pit_rail_cy + rail_minor_r;
    b.torus_y([0.0, pit_rail_cy, 0.0], ring_rail_r, rail_minor_r, 48, 8, p.rail);
    for i in 0..24 {
        let a = (i as f32 / 24.0) * std::f32::consts::TAU;
        b.obox(
            [ring_rail_r * a.cos(), -pit_depth + curb_h + 0.15, ring_rail_r * a.sin()],
            [3.2 * scale_f, 0.4, 2.4 * scale_f],
            [0.0, -a, 0.0],
            p.steel_dark,
        );
    }

    // 4. Multi-tier Königsstuhl center pivot assembly
    b.cyl_y([0.0, -pit_depth + 1.8 * scale_f, 0.0], 21.0 * scale_f, 24.0 * scale_f, 3.6 * scale_f, 12, p.concrete_dark);
    b.cyl_y([0.0, -pit_depth + 4.8 * scale_f, 0.0], 15.0 * scale_f, 18.0 * scale_f, 3.0 * scale_f, 16, p.steel_dark);
    for k in 0..12 {
        let ka = (k as f32 / 12.0) * std::f32::consts::TAU;
        b.rbox([16.2 * scale_f * ka.cos(), -pit_depth + 3.8 * scale_f, 16.2 * scale_f * ka.sin()], [1.2 * scale_f, 1.0 * scale_f, 1.2 * scale_f], p.wheel_steel);
    }
    b.cyl_y([0.0, -pit_depth + 7.1 * scale_f, 0.0], 11.5 * scale_f, 12.5 * scale_f, 1.6 * scale_f, 16, p.bronze);
    b.cyl_y([0.0, -pit_depth + 9.2 * scale_f, 0.0], 10.2 * scale_f, 10.2 * scale_f, 3.0 * scale_f, 16, p.steel_girder);

    // 5. Exit track stubs around circumference (sleepers + flush rails + locking chairs)
    let slot_angles: Vec<f32> = entry
        .and_then(|e| e.local_connectors.as_ref())
        .filter(|lc| !lc.is_empty())
        .map(|lc| lc.iter().map(|c| c.angle as f32).collect())
        .unwrap_or_else(|| (0..30).map(|i| i as f32 * 12.0).collect());

    let stub_len = outer_r - pit_r;
    let stub_center_r = 0.5 * (pit_r + outer_r);
    let r_tie_start = pit_r + 8.2 * scale_f;
    let r_tie_end = outer_r - 3.8 * scale_f;
    let n_stub_ties = 5usize;
    let tie_w_std = 22.0 * (gauge / 16.5);

    for &deg in &slot_angles {
        let rad = deg.to_radians();
        let (ca, sa) = (rad.cos(), rad.sin());
        // Wooden sleepers cleanly outside coping ring
        for ti in 0..n_stub_ties {
            let r_t = r_tie_start + (r_tie_end - r_tie_start) * (ti as f32 / (n_stub_ties - 1) as f32);
            b.obox([r_t * ca, 6.3, r_t * sa], [2.6 * scale_f, 1.6, tie_w_std], [0.0, -rad, 0.0], p.tie);
        }
        // Twin flush rails (top = 9.6 mm) + coping locking chairs
        for side in [-1.0_f32, 1.0] {
            let oz = side * hg;
            b.obox(
                [stub_center_r * ca - oz * sa, 8.35, stub_center_r * sa + oz * ca],
                [stub_len, 2.5, 1.4],
                [0.0, -rad, 0.0],
                p.rail,
            );
            let chair_r = pit_r + 2.6 * scale_f;
            b.obox(
                [chair_r * ca - oz * sa, 6.65, chair_r * sa + oz * ca],
                [3.4 * scale_f, 0.9, 2.8],
                [0.0, -rad, 0.0],
                p.steel_dark,
            );
        }
        let lock_r = pit_r + 1.8 * scale_f;
        b.obox([lock_r * ca, 6.55, lock_r * sa], [2.4 * scale_f, 0.8, gauge * 0.52], [0.0, -rad, 0.0], p.bronze);
    }

    // 6. DB 23 m Fish-Belly Tapered Plate-Girder Bridge (along X axis)
    let half_b = bridge_l * 0.5;
    let girder_x_end = half_b - 1.2 * scale_f;
    let bell_profile: [(f32, f32); 6] = [
        (-girder_x_end, -1.8 * scale_f),
        (-ring_rail_r, -3.6 * scale_f),
        (-44.0 * scale_f, -8.6 * scale_f),
        (44.0 * scale_f, -8.6 * scale_f),
        (ring_rail_r, -3.6 * scale_f),
        (girder_x_end, -1.8 * scale_f),
    ];
    let y_web_top = 8.6_f32;
    let hw = 1.0_f32;

    for sz in [-1.0_f32, 1.0] {
        let gz = sz * girder_z;
        for w in bell_profile.windows(2) {
            let (xa, yba) = w[0];
            let (xb, ybb) = w[1];
            // Front (+Z) & Back (-Z) tapered web faces
            b.quad_oriented([xa, yba, gz + hw], [xb, ybb, gz + hw], [xb, y_web_top, gz + hw], [xa, y_web_top, gz + hw], [0.0, 0.0, 1.0], p.steel_girder);
            b.quad_oriented([xb, ybb, gz - hw], [xa, yba, gz - hw], [xa, y_web_top, gz - hw], [xb, y_web_top, gz - hw], [0.0, 0.0, -1.0], p.steel_girder);
            // Continuous angled lower chord flange + top closure
            b.strut_between([xa, yba, gz], [xb, ybb, gz], 3.4, 1.5, p.steel_dark);
        }
        // Top heavy flange beam + end vertical posts
        b.rbox([0.0, 8.9, gz], [girder_x_end * 2.0, 1.0, 3.4], p.steel_dark);
        for sx in [-1.0_f32, 1.0] {
            let ex = sx * girder_x_end;
            let eb = -1.8 * scale_f;
            b.rbox([ex, 0.5 * (eb + 9.2), gz], [1.6, 9.2 - eb, 3.2], p.steel_dark);
        }
        // Vertical web stiffeners
        for &vx in &[
            -102.0 * scale_f,
            -74.0 * scale_f,
            -44.0 * scale_f,
            -22.0 * scale_f,
            0.0,
            22.0 * scale_f,
            44.0 * scale_f,
            74.0 * scale_f,
            102.0 * scale_f,
        ] {
            let ax = vx.abs();
            let yb = if ax <= 44.0 * scale_f {
                -8.6 * scale_f
            } else {
                let u = ((ax - 44.0 * scale_f) / (ring_rail_r - 44.0 * scale_f).max(1.0)).clamp(0.0, 1.0);
                (-8.6 + 5.0 * u) * scale_f
            };
            b.rbox([vx, 0.5 * (yb + y_web_top), gz], [1.5, y_web_top - yb, 3.2], p.steel_dark);
        }
    }

    // Under-deck cross-beams & end head-beams (kept inside circular pit clearance at outer corners)
    let girder_l = girder_x_end * 2.0 - 1.6 * scale_f;
    for i in 0..=12 {
        let cx = -girder_x_end + 0.8 * scale_f + girder_l * (i as f32 / 12.0);
        b.rbox([cx, 3.6, 0.0], [2.0, 2.4, girder_z * 2.0], p.steel_dark);
    }
    for sx in [-1.0_f32, 1.0] {
        b.rbox([sx * (half_b - 1.2), 5.6, 0.0], [2.4, 2.2, (girder_z + 1.5) * 2.0], p.steel_dark);
        b.rbox([sx * (half_b - 0.3), 6.2, 0.0], [1.6, 1.1, gauge * 0.50], p.bronze);
    }

    // 7. 4 Flanged Bogie Wheels riding tangentially on the circular running rail
    let wheel_r = 3.4 * scale_f;
    let flange_r = 4.05 * scale_f;
    let wheel_y = pit_rail_top + wheel_r;
    let x_w = (ring_rail_r * ring_rail_r - girder_z * girder_z).max(100.0).sqrt();
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            let wx = sx * x_w;
            let wz = sz * girder_z;
            let r_len = wx.hypot(wz).max(1.0);
            let dir = [wx / r_len, 0.0, wz / r_len];
            let axle_ang = wz.atan2(wx);
            // Steel tread riding directly on top of the circular pit rail
            let t0 = [wx - dir[0] * 1.1, wheel_y, wz - dir[2] * 1.1];
            let t1 = [wx + dir[0] * 1.1, wheel_y, wz + dir[2] * 1.1];
            b.cyl_between(t0, t1, wheel_r, wheel_r, 14, p.wheel_steel);
            // Inner wheel flange
            let f0 = [wx - dir[0] * 1.45, wheel_y, wz - dir[2] * 1.45];
            let f1 = [wx - dir[0] * 0.95, wheel_y, wz - dir[2] * 0.95];
            b.cyl_between(f0, f1, flange_r, flange_r, 14, p.steel_dark);
            // Outer hub cap & bogie journal housing attached to girder underside
            let h0 = [wx + dir[0] * 1.1, wheel_y, wz + dir[2] * 1.1];
            let h1 = [wx + dir[0] * 1.85, wheel_y, wz + dir[2] * 1.85];
            b.cyl_between(h0, h1, 1.5 * scale_f, 1.2 * scale_f, 10, p.bronze);
            b.obox([wx, wheel_y + 2.8 * scale_f, wz], [4.2 * scale_f, 5.2 * scale_f, 6.0 * scale_f], [0.0, -axle_ang, 0.0], p.steel_dark);
        }
    }

    // 8. Center deck plate, symmetrically spaced ties & flush bridge rails (top = 9.6 mm)
    b.rbox([0.0, 5.15, 0.0], [bridge_l - 1.0, 0.7, girder_z * 2.0 - 1.2], p.steel_deck);
    let tie_w_bridge = 21.0 * (gauge / 16.5);
    let tie_span = bridge_l - 10.0;
    let n_ties = ((tie_span / 8.0).round() as usize).max(4);
    for i in 0..=n_ties {
        let tx = -tie_span * 0.5 + tie_span * (i as f32 / n_ties as f32);
        b.rbox([tx, 6.3, 0.0], [2.6, 1.6, tie_w_bridge], p.tie);
    }
    for side in [-1.0_f32, 1.0] {
        b.rbox([0.0, 8.35, side * hg], [bridge_l, 2.5, 1.4], p.rail);
    }

    // 9. Steel-grated catwalks, kickplates, 2-bar handrails & opposite-corner Gleissperrsignale
    let walk_w = 7.4_f32;
    let walk_z = girder_z + 4.4;
    let rail_z = girder_z + 7.7;
    let cab_cx = -16.0 * scale_f;
    let cab_w = 34.0 * scale_f;
    let cab_d = 20.5 * scale_f;
    let cab_cz = rail_z + cab_d * 0.5;

    let walk_l = bridge_l - 9.0 * scale_f;
    for sz in [-1.0_f32, 1.0] {
        b.rbox([0.0, 5.15, sz * walk_z], [walk_l, 0.7, walk_w], p.steel_deck);
        b.rbox([0.0, 6.2, sz * (walk_z + walk_w * 0.5 - 0.35)], [walk_l, 1.5, 0.7], p.steel_dark);
        for bx in (-112..=112).step_by(28) {
            let x = bx as f32 * scale_f;
            b.strut_between([x, 4.9, sz * (rail_z - 0.4)], [x, 1.0, sz * girder_z], 1.2, 1.2, p.steel_dark);
        }
    }
    build_safety_handrail_x(&mut b, -walk_l * 0.5 + 1.5, walk_l * 0.5 - 1.5, 5.5, -rail_z, None, scale_f, p.steel_dark);
    build_safety_handrail_x(
        &mut b,
        -walk_l * 0.5 + 1.5,
        walk_l * 0.5 - 1.5,
        5.5,
        rail_z,
        Some((cab_cx - cab_w * 0.5, cab_cx + cab_w * 0.5)),
        scale_f,
        p.steel_dark,
    );
    build_gleissperrsignal(&mut b, half_b - 10.0 * scale_f, 5.5, -(girder_z + 5.2), scale_f, &p);
    build_gleissperrsignal(&mut b, -(half_b - 10.0 * scale_f), 5.5, girder_z + 5.2, scale_f, &p);

    // 10. Cantilevered steel-braced Wärterhaus operator cabin
    build_operator_cabin(&mut b, cab_cx, cab_cz, cab_w, cab_d, girder_z, 5.5, -8.2 * scale_f, scale_f, &p);

    // 11. Drehscheiben-Spinne overhead catenary spider (when has_catenary is true)
    if piece.has_catenary == Some(true) {
        let contact_h = 72.0_f32;
        let upper_h = 85.0_f32;
        let spider_r = 22.0 * scale_f;
        let outer_holder_r = outer_r - 4.0 * scale_f;

        b.torus_y([0.0, contact_h, 0.0], spider_r, 0.55, 28, 6, p.wire_copper);
        b.torus_y([0.0, upper_h, 0.0], spider_r, 0.75, 28, 6, p.cat_green);
        for k in 0..8 {
            let a = (k as f32 / 8.0) * std::f32::consts::TAU;
            b.strut_between([spider_r * a.cos(), contact_h, spider_r * a.sin()], [spider_r * a.cos(), upper_h, spider_r * a.sin()], 0.45, 0.45, p.wire_dark);
        }
        // Central radial catenary cross-wires over turntable bridge
        b.strut_between([-spider_r, contact_h, 0.0], [spider_r, contact_h, 0.0], 0.55, 0.55, p.wire_copper);

        for &deg in &slot_angles {
            let rad = deg.to_radians();
            let (ca, sa) = (rad.cos(), rad.sin());
            let ring_pt = [spider_r * ca, contact_h, spider_r * sa];
            let out_pt = [outer_r * ca, contact_h, outer_r * sa];
            let ring_top = [spider_r * ca, upper_h + 2.0, spider_r * sa];
            let pit_top = [pit_r * ca, upper_h, pit_r * sa];
            let out_top = [outer_r * ca, upper_h, outer_r * sa];

            b.strut_between(ring_pt, out_pt, 0.55, 0.55, p.wire_copper);
            b.strut_between(ring_top, pit_top, 0.45, 0.45, p.wire_dark);
            b.strut_between(pit_top, out_top, 0.45, 0.45, p.wire_dark);
            for u in [0.35_f32, 0.70] {
                let rx = (spider_r + (pit_r - spider_r) * u) * ca;
                let rz = (spider_r + (pit_r - spider_r) * u) * sa;
                let top_y = (upper_h + 2.0) + (upper_h - (upper_h + 2.0)) * u;
                b.strut_between([rx, contact_h, rz], [rx, top_y, rz], 0.35, 0.35, p.wire_dark);
            }
            build_catenary_portal(
                &mut b,
                outer_holder_r * ca,
                5.5,
                outer_holder_r * sa,
                -rad,
                12.5,
                contact_h,
                upper_h,
                &p,
            );
        }
    }
}

// ============================================================================
//  2. TRANSFER TABLE (Schiebebühne — Märklin 72941)
// ============================================================================

pub(crate) fn extract_transfer_track_zs(entry: Option<&CatalogEntry>) -> Vec<f32> {
    let mut zs: Vec<f32> = entry
        .and_then(|e| e.local_connectors.as_ref())
        .map(|lcs| lcs.iter().map(|c| c.y as f32).collect())
        .unwrap_or_default();
    zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut uniq: Vec<f32> = Vec::new();
    for z in zs {
        if uniq.last().map_or(true, |&prev| (z - prev).abs() > 1.0) {
            uniq.push(z);
        }
    }
    if uniq.is_empty() {
        vec![0.0, 77.4, 154.8, 232.2, 309.6]
    } else {
        uniq
    }
}

pub(super) fn push_catenary_span_x(
    b: &mut ModelBuilder<'_>,
    x0: f32,
    x1: f32,
    z: f32,
    contact_h: f32,
    messenger_h: f32,
    p: &SharedPalette,
) {
    b.strut_between([x0, contact_h, z], [x1, contact_h, z], 0.65, 0.65, p.wire_copper);
    let mid_x = 0.5 * (x0 + x1);
    let mid_y = messenger_h - 2.4;
    b.strut_between([x0, messenger_h, z], [mid_x, mid_y, z], 0.45, 0.45, p.wire_dark);
    b.strut_between([mid_x, mid_y, z], [x1, messenger_h, z], 0.45, 0.45, p.wire_dark);
    for u in [0.22_f32, 0.50, 0.78] {
        let dx = x0 + (x1 - x0) * u;
        let sag = 1.0 - (2.0 * u - 1.0).abs();
        let top_y = messenger_h - 2.4 * sag;
        b.strut_between([dx, contact_h, z], [dx, top_y, z], 0.35, 0.35, p.wire_dark);
    }
}

