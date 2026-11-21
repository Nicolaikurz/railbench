use super::*;

// ----------------------------------------------------------------------------
// 5. TOWN HOUSE (Fachwerkhaus — half-timbered beams & spandrel/gable braces, shutters, dormer, chimney)
// ----------------------------------------------------------------------------
pub(super) fn build_town_house(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [110.0, 125.0, 90.0]);
    let stone = Mat::matte(0x64748b, 0.90);
    let plaster = Mat::matte(0xf3ead8, 0.82);
    let beam = Mat::matte(0x451a03, 0.88);
    let roof = Mat::matte(0x991b1b, 0.68);
    let ridge = Mat::matte(0x7c2d12, 0.72);
    let brick = Mat::matte(0x9a3412, 0.82);
    let door_m = Mat::matte(0x5c2e14, 0.78);
    let frame = Mat::matte(0xf8fafc, 0.65);
    let shutter = Mat::matte(0x166534, 0.78);
    let win_dark = Mat::warm_window(0xfde68a, 0.35);
    let win_warm = Mat::warm_window(0xfde68a, 0.92);
    let fbox = Mat::matte(0x78350f, 0.85);
    let flowers = Mat::matte(0xdc2626, 0.70);

    // 1. Stone plinth + ground floor + jettied upper floor
    b.rbox([0.0, 4.5, 0.0], [108.0, 9.0, 86.0], stone);
    b.rbox([0.0, 25.5, 0.0], [102.0, 33.0, 78.0], plaster);
    b.rbox([0.0, 59.0, 0.0], [106.0, 35.0, 82.0], plaster);

    // 2. Half-timbering horizontal rails, posts (clear of window & shutter axes) & diagonal braces
    for by in [42.5_f32, 49.5, 68.5, 75.2] {
        b.rbox([0.0, by, 0.0], [107.6, 2.5, 83.6], beam);
    }
    for sz in [-1.0_f32, 1.0] {
        for bx in [-52.2_f32, -18.0, 18.0, 52.2] {
            b.rbox([bx, 59.0, sz * 40.5], [2.5, 35.0, 2.5], beam);
        }
        for bx in [-50.2_f32, -18.0, 18.0, 50.2] {
            b.rbox([bx, 25.5, sz * 38.5], [2.5, 33.0, 2.5], beam);
        }
        for sx in [-1.0_f32, 1.0] {
            b.strut_between([sx * 51.0, 43.2, sz * 40.9], [sx * 36.0, 48.8, sz * 40.9], 2.0, 1.8, beam);
            b.strut_between([sx * 17.5, 43.2, sz * 40.9], [sx * 4.0, 48.8, sz * 40.9], 2.0, 1.8, beam);
        }
    }
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            b.strut_between([sx * 52.4, 43.2, sz * 39.5], [sx * 52.4, 74.5, sz * 22.0], 2.0, 1.8, beam);
        }
    }

    // 3. Steep gable roof + dormer + brick chimney
    b.gable_roof_x([0.0, 76.5, 0.0], 106.0, 82.0, 44.0, 3.5, 2.0, roof, plaster, ridge);
    b.rbox([0.0, 91.0, 22.0], [20.0, 14.0, 18.0], plaster);
    b.rbox([0.0, 99.0, 22.0], [23.0, 3.0, 21.0], roof);
    b.rbox([0.0, 91.5, 31.2], [10.0, 8.5, 1.2], win_warm);
    b.rbox([28.0, 112.0, -14.0], [10.0, 24.0, 10.0], brick);
    b.rbox([28.0, 124.8, -14.0], [12.0, 2.2, 12.0], stone);

    // 4. Mullioned windows with green shutters & flower boxes
    let mut add_win = |wx: f32, wy: f32, wall_z: f32, side: f32, warm: bool, with_flowers: bool| {
        let wm = if warm { win_warm } else { win_dark };
        b.window_z(wx, wy, wall_z, side, 10.0, 13.0, wm, frame, fbox, beam);
        for sx in [-1.0_f32, 1.0] {
            b.rbox([wx + sx * 8.4, wy, wall_z + side * 0.7], [4.0, 14.8, 1.1], shutter);
        }
        if with_flowers {
            b.rbox([wx, wy - 8.8, wall_z + side * 2.2], [12.8, 2.6, 3.0], fbox);
            b.rbox([wx, wy - 7.0, wall_z + side * 2.2], [11.4, 2.2, 2.2], flowers);
        }
    };
    for side in [1.0_f32, -1.0] {
        for dx in [-36.0_f32, 0.0, 36.0] {
            add_win(dx, 59.0, side * 41.0, side, dx == 0.0 || (side > 0.0 && dx < 0.0), side > 0.0);
        }
        for dx in [-36.0_f32, 36.0] {
            add_win(dx, 26.0, side * 39.0, side, side > 0.0 && dx > 0.0, false);
        }
    }
    for sx in [-1.0_f32, 1.0] {
        b.window_x(sx * 53.0, 90.0, 0.0, sx, 8.8, 10.5, win_warm, frame, stone, beam);
    }

    // 5. Front entrance door & stone stoop (+Z)
    b.rbox([0.0, 21.5, 39.2], [16.0, 25.0, 1.6], beam);
    b.rbox([0.0, 20.5, 39.6], [13.0, 22.5, 1.6], door_m);
    b.rbox([0.0, 7.6, 42.0], [20.0, 3.2, 6.0], stone);
}

// ----------------------------------------------------------------------------
// 6. WATER TOWER (Wasserturm — tapered cylindrical shaft, corbel brackets, tank, conical roof, cupola)
// ----------------------------------------------------------------------------
pub(super) fn build_water_tower(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [85.0, 170.0, 85.0]);
    let stone = Mat::matte(0x64748b, 0.90);
    let cornice = Mat::matte(0xcbd5e1, 0.78);
    let brick = Mat::matte(0x9a3412, 0.82);
    let tank_wall = Mat::metal(0x475569, 0.56, 0.35);
    let tank_band = Mat::metal(0x1e293b, 0.40, 0.72);
    let roof = Mat::matte(0x334155, 0.55);
    let door_m = Mat::matte(0x5c2e14, 0.80);
    let win_warm = Mat::warm_window(0xfde68a, 0.88);
    let win_dark = Mat::warm_window(0xfde68a, 0.32);
    let brass = Mat::metal(0xd97706, 0.28, 0.82);
    let gauge_bg = Mat::matte(0xf8fafc, 0.50);
    let gauge_red = Mat::matte(0xdc2626, 0.45);

    // 1. Stepped cylindrical stone plinth & tapered brick shaft (32 radial segments for ultra-smooth silhouette)
    b.cyl_y([0.0, 3.0, 0.0], 39.0, 41.0, 6.0, 32, stone);
    b.cyl_y([0.0, 13.0, 0.0], 35.0, 37.0, 14.0, 32, stone);
    b.cyl_y([0.0, 21.2, 0.0], 36.5, 36.5, 2.4, 32, cornice);
    b.cyl_y([0.0, 52.0, 0.0], 28.0, 34.0, 60.0, 32, brick);

    // Entry portal (+Z) & 8 radially-aligned shaft windows
    b.rbox([0.0, 16.0, 34.5], [14.0, 22.0, 4.0], cornice);
    b.rbox([0.0, 15.0, 35.0], [10.4, 19.0, 4.2], door_m);
    for i in 0..4 {
        let ang = (i as f32) * std::f32::consts::FRAC_PI_2;
        for &(wy, r, warm) in &[(44.0_f32, 31.5_f32, false), (66.0, 29.4, i % 2 == 0)] {
            let cx = ang.sin() * r;
            let cz = ang.cos() * r;
            b.obox([cx, wy, cz], [6.4, 10.5, 2.2], [0.0, ang, 0.0], cornice);
            b.obox([cx + ang.sin() * 0.4, wy, cz + ang.cos() * 0.4], [4.4, 8.5, 2.2], [0.0, ang, 0.0], if warm { win_warm } else { win_dark });
        }
    }

    // 2. Flared corbel ring & 12 radially-aligned stone brackets supporting overhanging reservoir tank
    b.cyl_y([0.0, 86.5, 0.0], 38.5, 29.0, 9.0, 32, cornice);
    for i in 0..12 {
        let ang = (i as f32 / 12.0) * std::f32::consts::TAU;
        b.obox([ang.sin() * 32.5, 84.5, ang.cos() * 32.5], [3.2, 9.5, 8.5], [0.0, ang, 0.0], stone);
    }

    // 3. Elevated cylindrical reservoir with steel bands, flush control windows & level gauge
    b.cyl_y([0.0, 111.0, 0.0], 39.5, 39.0, 40.0, 36, tank_wall);
    for by in [95.0_f32, 107.0, 119.0, 129.0] {
        b.cyl_y([0.0, by, 0.0], 40.2, 40.2, 1.8, 36, tank_band);
    }
    for i in 0..8 {
        let ang = (i as f32 / 8.0) * std::f32::consts::TAU + std::f32::consts::FRAC_PI_8;
        let cx = ang.sin() * 39.7;
        let cz = ang.cos() * 39.7;
        b.obox([cx, 113.0, cz], [6.8, 7.6, 1.5], [0.0, ang, 0.0], if i % 3 == 0 { win_warm } else { win_dark });
    }
    b.rbox([0.0, 111.0, 40.2], [4.5, 31.0, 1.6], gauge_bg);
    b.rbox([0.0, 118.0, 40.8], [5.2, 3.0, 1.8], gauge_red);

    // 4. Conical slate roof, exposed lit cupola lantern with mullions & brass finial
    b.cone_y([0.0, 131.0, 0.0], 43.0, 26.0, 36, 0.0, roof);
    b.cyl_y([0.0, 154.8, 0.0], 9.6, 10.0, 3.2, 20, tank_band);
    b.cyl_y([0.0, 159.0, 0.0], 8.6, 8.6, 5.4, 20, win_warm);
    for i in 0..6 {
        let a = (i as f32 / 6.0) * std::f32::consts::TAU;
        b.cyl_between([a.sin() * 9.0, 156.2, a.cos() * 9.0], [a.sin() * 9.0, 162.0, a.cos() * 9.0], 0.45, 0.45, 8, tank_band);
    }
    b.cyl_y([0.0, 162.0, 0.0], 9.6, 9.6, 1.0, 20, tank_band);
    b.cone_y([0.0, 162.4, 0.0], 11.5, 9.0, 20, 0.0, roof);
    b.cyl_y([0.0, 174.0, 0.0], 0.5, 1.0, 10.0, 12, brass);

    // 5. Rear climbing ladder outside tank radius (-Z = -41.2) + standoff brackets & water-crane (+X)
    for lx in [-2.4_f32, 2.4] {
        b.cyl_y([lx, 71.0, -41.2], 0.6, 0.6, 118.0, 6, tank_band);
    }
    for ry in (18..=126).step_by(8) {
        b.rbox([0.0, ry as f32, -41.2], [5.2, 0.8, 0.8], tank_band);
    }
    for sy in [26.0_f32, 54.0, 82.0, 110.0] {
        let wall_z: f32 = if sy < 85.0 { -31.5 } else { -39.0 };
        let z_mid = 0.5 * (-41.2 + wall_z);
        let z_span = (-41.2 - wall_z).abs();
        for lx in [-2.4_f32, 2.4] {
            b.rbox([lx, sy, z_mid], [0.8, 0.8, z_span], tank_band);
        }
    }
    b.cyl_y([35.5, 43.0, 0.0], 3.0, 3.6, 80.0, 10, tank_band);
    b.cyl_x([47.0, 80.0, 0.0], 2.6, 2.4, 24.0, 10, tank_band);
    b.cyl_y([58.0, 72.0, 0.0], 2.6, 1.8, 16.0, 10, tank_band);
}

// ----------------------------------------------------------------------------
// 7. COALING STATION (Bekohlungsanlage — bunker, coal mounds, hopper, slewing crane & water crane)
// ----------------------------------------------------------------------------
pub(super) fn build_coaling_station(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [180.0, 125.0, 75.0]);
    let concrete = Mat::matte(0x64748b, 0.90);
    let dark_conc = Mat::matte(0x475569, 0.92);
    let coal = Mat::matte(0x16171b, 0.68);
    let steel = Mat::metal(0x334155, 0.44, 0.68);
    let dark_steel = Mat::metal(0x1e293b, 0.40, 0.76);
    let crane_cab = Mat::metal(0x1e3a5f, 0.48, 0.40);
    let rust = Mat::matte(0x7c2d12, 0.84);
    let warning = Mat::matte(0xea580c, 0.50);
    let cab_glass = Mat::warm_window(0xfef08a, 0.72);

    // 1. Concrete foundation plate
    b.rbox([0.0, 2.0, 0.0], [180.0, 4.0, 75.0], dark_conc);

    // 2. Concrete coal bunker with 10 buttresses & jagged coal mound heap
    let (bx, bz, bw, bd, bh, wt) = (-46.0_f32, -9.0_f32, 76.0_f32, 46.0_f32, 22.0_f32, 3.5_f32);
    b.rbox([bx, 4.0 + bh * 0.5, bz - bd * 0.5 + wt * 0.5], [bw, bh, wt], concrete);
    b.rbox([bx, 4.0 + bh * 0.41, bz + bd * 0.5 - wt * 0.5], [bw, bh * 0.82, wt], concrete);
    b.rbox([-84.0 + wt * 0.5, 4.0 + bh * 0.5, bz], [wt, bh, bd], concrete);
    b.rbox([-8.0 - wt * 0.5, 4.0 + bh * 0.5, bz], [wt, bh, bd], concrete);
    for i in 0..=4 {
        let px = -80.0 + (i as f32) * ((bw - 8.0) / 4.0);
        b.rbox([px, 4.0 + bh * 0.5, bz - bd * 0.5 - 1.0], [2.8, bh + 1.0, 3.5], dark_conc);
        b.rbox([px, 4.0 + bh * 0.41, bz + bd * 0.5 + 1.0], [2.8, bh * 0.82 + 1.0, 3.5], dark_conc);
    }
    b.rbox([bx, 10.5, bz], [bw - 8.0, 13.0, bd - 8.0], coal);
    for (idx, (mx, mz, mr, mh)) in [
        (-64.0_f32, -12.0_f32, 14.0_f32, 15.0_f32),
        (-46.0, -9.0, 16.0, 19.0),
        (-28.0, -11.0, 14.0, 16.0),
        (-56.0, -2.0, 11.0, 12.0),
        (-34.0, -2.0, 11.0, 13.0),
    ].into_iter().enumerate() {
        b.cone_y([mx, 15.0, mz], mr, mh, 7, (idx as f32) * 0.42, coal);
    }

    // 3. Elevated steel weigh hopper & chute (+X = 54)
    let (tx, tz, span_x, span_z) = (54.0_f32, -4.0_f32, 34.0_f32, 32.0_f32);
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            let lx = tx + sx * span_x * 0.5;
            let lz = tz + sz * span_z * 0.5;
            b.rbox([lx, 7.0, lz], [6.0, 6.0, 6.0], concrete);
            b.rbox([lx, 38.0, lz], [3.2, 56.0, 3.2], steel);
        }
    }
    // Cross-bracing struts
    for sz in [-1.0_f32, 1.0] {
        let lz = tz + sz * span_z * 0.5;
        b.strut_between([tx - 17.0, 14.0, lz], [tx + 17.0, 44.0, lz], 1.6, 1.6, steel);
        b.strut_between([tx + 17.0, 14.0, lz], [tx - 17.0, 44.0, lz], 1.6, 1.6, steel);
    }
    for y_lvl in [28.0_f32, 48.0, 64.0] {
        b.rbox([tx, y_lvl, tz - span_z * 0.5], [span_x + 4.0, 2.5, 2.5], steel);
        b.rbox([tx, y_lvl, tz + span_z * 0.5], [span_x + 4.0, 2.5, 2.5], steel);
        b.rbox([tx - span_x * 0.5, y_lvl, tz], [2.5, 2.5, span_z + 4.0], steel);
        b.rbox([tx + span_x * 0.5, y_lvl, tz], [2.5, 2.5, span_z + 4.0], steel);
    }
    b.frustum_rect_y([tx, 56.0, tz], 34.0, 32.0, 11.0, 11.0, 20.0, steel);
    b.rbox([tx, 67.0, tz], [36.0, 4.0, 34.0], dark_steel);
    b.rbox([tx, 76.0, tz - 15.9], [36.0, 22.0, 2.2], steel);
    b.rbox([tx, 76.0, tz + 15.9], [36.0, 22.0, 2.2], steel);
    b.rbox([tx - 16.9, 76.0, tz], [2.2, 22.0, 29.6], steel);
    b.rbox([tx + 16.9, 76.0, tz], [2.2, 22.0, 29.6], steel);
    b.rbox([tx, 74.0, tz], [30.8, 12.0, 28.8], coal);
    b.cone_y([tx, 80.0, tz], 12.0, 6.0, 7, 0.2, coal);
    b.obox([tx, 42.5, tz + 15.0], [12.0, 2.0, 26.0], [0.52, 0.0, 0.0], dark_steel);
    b.obox([tx - 5.5, 44.3, tz + 15.0], [1.5, 4.8, 26.0], [0.52, 0.0, 0.0], steel);
    b.obox([tx + 5.5, 44.3, tz + 15.0], [1.5, 4.8, 26.0], [0.52, 0.0, 0.0], steel);
    b.rbox([tx, 65.0, tz], [42.0, 2.0, 40.0], dark_steel);

    // 4. Slewing coaling crane cab, lattice boom, cable & clamshell grab (pointing toward -X coal heap)
    let (crx, crz) = (12.0_f32, -10.0_f32);
    b.cyl_y([crx, 25.0, crz], 8.5, 10.0, 42.0, 12, concrete);
    b.cyl_y([crx, 47.0, crz], 9.5, 9.5, 3.0, 14, dark_steel);
    b.rbox([crx + 1.0, 56.5, crz], [18.0, 16.0, 14.0], crane_cab);
    b.rbox([crx + 1.0, 65.5, crz], [20.0, 2.0, 16.0], dark_steel);
    b.rbox([crx - 7.5, 58.5, crz], [2.0, 7.0, 10.0], cab_glass);
    b.rbox([crx + 12.0, 54.5, crz], [6.0, 10.0, 12.0], dark_conc);

    let boom_start = [crx - 5.0, 59.5, crz];
    let boom_tip = [crx - 46.0, 100.0, crz];
    for bz in [-2.6_f32, 2.6] {
        b.cyl_between([boom_start[0], boom_start[1], crz + bz], [boom_tip[0], boom_tip[1], crz + bz], 1.2, 0.9, 6, steel);
    }
    for k in 1..6 {
        let t = k as f32 / 6.0;
        let mx = boom_start[0] + (boom_tip[0] - boom_start[0]) * t;
        let my = boom_start[1] + (boom_tip[1] - boom_start[1]) * t;
        b.rbox([mx, my, crz], [1.4, 2.6, 5.8], steel);
    }
    b.cyl_z(boom_tip, 2.8, 2.8, 2.4, 12, warning);
    b.cyl_y([boom_tip[0], boom_tip[1] - 18.0, crz], 0.45, 0.45, 36.0, 6, dark_steel);
    let grab_y = boom_tip[1] - 36.0;
    b.rbox([boom_tip[0], grab_y, crz], [4.5, 5.2, 4.5], dark_steel);
    b.obox([boom_tip[0] - 2.6, grab_y - 4.2, crz], [5.2, 6.2, 7.2], [0.0, 0.0, 0.25], rust);
    b.obox([boom_tip[0] + 2.6, grab_y - 4.2, crz], [5.2, 6.2, 7.2], [0.0, 0.0, -0.25], rust);
    b.organic_foliage_cluster([boom_tip[0], grab_y - 3.6, crz], 3.8, 3.4, 3.8, 99, coal);

    // 5. Trackside articulated water crane (X = -72, Z = +24)
    let (wcx, wcz) = (-72.0_f32, 24.0_f32);
    b.rbox([wcx + 18.0, 4.5, wcz], [14.0, 1.0, 14.0], dark_steel);
    b.cyl_y([wcx, 8.0, wcz], 3.8, 5.0, 8.0, 10, dark_steel);
    b.cyl_y([wcx, 31.0, wcz], 2.4, 2.8, 46.0, 10, dark_steel);
    b.cyl_x([wcx + 8.0, 53.0, wcz], 1.8, 1.8, 28.0, 8, dark_steel);
    b.cyl_y([wcx + 20.0, 48.0, wcz], 1.6, 2.0, 12.0, 8, dark_steel);
    b.sphere([wcx - 6.0, 53.5, wcz], 3.6, 8, 10, warning);
}

// ----------------------------------------------------------------------------
// 8. DIESEL FUEL STATION (Dieseltankstelle — cylindrical tank + dished caps, catwalk, pump house, dispensers)
// ----------------------------------------------------------------------------
pub(super) fn build_diesel_fuel_station(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [170.0, 68.0, 65.0]);
    let concrete = Mat::matte(0x94a3b8, 0.88);
    let dark_conc = Mat::matte(0x64748b, 0.90);
    let grate = Mat::metal(0x334155, 0.55, 0.50);
    let tank_m = Mat::metal(0xdbeafe, 0.36, 0.38);
    let red_band = Mat::matte(0xdc2626, 0.45);
    let dark_steel = Mat::metal(0x334155, 0.45, 0.55);
    let safety = Mat::matte(0xfacc15, 0.50);
    let pump_wall = Mat::matte(0xe2e8f0, 0.85);
    let brick = Mat::matte(0x991b1b, 0.86);
    let canopy_m = Mat::metal(0x475569, 0.48, 0.42);
    let disp_blue = Mat::metal(0x0284c7, 0.38, 0.25);
    let pipe_m = Mat::metal(0x64748b, 0.32, 0.70);
    let hose_m = Mat::matte(0x111827, 0.85);
    let light_m = Mat::emissive(0xfef08a, 0.90);

    // 1. Apron, recessed spill pan, drainage grates & yellow curb
    b.rbox([0.0, 1.25, 0.0], [168.0, 2.5, 63.0], concrete);
    b.rbox([32.0, 2.7, 17.0], [84.0, 0.8, 18.0], dark_conc);
    for gx in [4.0_f32, 22.0, 40.0, 58.0] {
        b.rbox([gx, 3.15, 17.0], [14.0, 0.5, 12.0], grate);
    }
    b.rbox([32.0, 3.1, 26.5], [86.0, 1.4, 1.4], safety);

    // 2. Main horizontal cylindrical diesel tank with dished end-caps on 3 saddle piers
    let (tx, tz, ty, tr, tl) = (-40.0_f32, -8.0_f32, 23.5_f32, 14.5_f32, 68.0_f32);
    for sx in [-24.0_f32, 0.0, 24.0] {
        b.rbox([tx + sx, 7.0, tz], [4.5, 9.0, 24.0], dark_conc);
    }
    b.cyl_x([tx, ty, tz], tr, tr, tl, 24, tank_m);
    b.hemi_cap_x([tx - tl * 0.5, ty, tz], tr, 5.2, -1.0, 6, 18, tank_m);
    b.hemi_cap_x([tx + tl * 0.5, ty, tz], tr, 5.2, 1.0, 6, 18, tank_m);
    for &(bx, bw, bm) in &[(-18.0_f32, 2.2_f32, dark_steel), (0.0, 5.5, red_band), (18.0, 2.2, dark_steel)] {
        b.cyl_x([tx + bx, ty, tz], tr + 0.3, tr + 0.3, bw, 24, bm);
    }
    for dx in [-16.0_f32, 16.0] {
        b.cyl_y([tx + dx, ty + tr + 1.2, tz], 3.1, 3.5, 3.5, 12, dark_steel);
    }
    b.rbox([tx, ty + tr + 2.8, tz], [58.0, 0.8, 6.0], grate);
    b.rbox([tx, ty + tr + 9.5, tz - 2.8], [58.0, 0.6, 0.6], safety);
    for rx in [-28.0_f32, -14.0, 0.0, 14.0, 28.0] {
        b.rbox([tx + rx, ty + tr + 6.2, tz - 2.8], [0.6, 7.0, 0.6], safety);
    }
    let lad_bot = 2.5_f32;
    let lad_top = ty + tr + 9.5;
    let lad_h = lad_top - lad_bot;
    for lx in [-1.8_f32, 1.8] {
        b.rbox([tx + lx, lad_bot + lad_h * 0.5, tz + tr + 1.5], [0.6, lad_h, 0.6], safety);
    }
    let mut ry = 6.5_f32;
    while ry <= ty + tr + 2.8 {
        b.rbox([tx, ry, tz + tr + 1.5], [3.6, 0.5, 0.5], safety);
        ry += 4.5;
    }
    b.rbox([tx, ty + tr + 2.8, tz + 9.5], [5.6, 0.8, 13.4], grate);
    for lx in [-2.4_f32, 2.4] {
        b.rbox([tx + lx, ty + tr + 9.5, tz + 9.5], [0.6, 0.6, 13.4], safety);
        b.rbox([tx + lx, ty + tr + 6.2, tz + 9.5], [0.6, 7.0, 0.6], safety);
    }

    // 3. Brick/plaster pump house + pipe bridge (door proud of plinth)
    let (px, pz) = (18.0_f32, -12.0_f32);
    b.rbox([px, 5.0, pz], [28.0, 5.0, 22.0], brick);
    b.rbox([px, 18.0, pz], [27.0, 21.0, 21.0], pump_wall);
    b.rbox([px, 29.6, pz], [30.0, 2.2, 24.0], canopy_m);
    b.rbox([px - 5.0, 3.0, pz + 11.5], [8.0, 1.0, 1.6], dark_steel);
    b.rbox([px - 5.0, 12.2, pz + 11.1], [7.0, 18.6, 1.0], dark_steel);
    b.rbox([px + 6.0, 17.5, pz + 10.6], [8.0, 7.0, 0.8], light_m);
    for py in [12.5_f32, 16.5] {
        b.cyl_x([px - 20.0, py, pz + 2.0], 1.1, 1.1, 26.0, 10, pipe_m);
    }

    // 4. Fueling island, 2 locomotive dispensers with swivel booms/hoses & cantilevered canopy
    let (ix, iz) = (32.0_f32, 15.0_f32);
    b.rbox([ix, 3.6, iz], [72.0, 2.2, 8.0], concrete);
    let can_y = 54.5_f32;
    for cx in [-22.0_f32, 22.0] {
        b.rbox([ix + cx, 2.5 + 26.0, iz - 1.0], [3.2, 52.0, 3.2], dark_steel);
        b.rbox([ix + cx, can_y - 1.5, iz + 1.0], [2.8, 3.0, 24.0], dark_steel);
    }
    b.rbox([ix, can_y + 1.2, iz + 1.0], [78.0, 2.4, 26.0], canopy_m);
    b.rbox([ix, can_y + 1.3, iz + 1.0], [78.6, 1.8, 26.6], disp_blue);
    for lx in [-18.0_f32, 18.0] {
        b.rbox([ix + lx, can_y - 0.4, iz + 2.0], [16.0, 0.8, 3.0], light_m);
    }
    for dx in [-14.0_f32, 14.0] {
        b.rbox([ix + dx, 14.2, iz], [7.5, 19.0, 5.0], disp_blue);
        b.rbox([ix + dx, 17.7, iz], [5.2, 5.5, 5.4], pump_wall);
        b.cyl_y([ix + dx + 2.2, 28.7, iz], 0.9, 0.9, 18.0, 8, pipe_m);
        b.cyl_between([ix + dx + 2.2, 37.0, iz], [ix + dx + 2.2, 35.0, iz + 11.0], 0.75, 0.7, 8, safety);
        b.cyl_y([ix + dx + 2.2, 26.5, iz + 11.0], 0.45, 0.45, 17.0, 8, hose_m);
    }
    for bx in [-33.0_f32, 33.0] {
        for bz in [-2.5_f32, 2.5] {
            b.cyl_y([ix + bx, 7.5, iz + bz], 1.1, 1.1, 10.0, 10, safety);
        }
    }
}

