use super::*;

// ----------------------------------------------------------------------------
// 1. STATION (Bahnhof Neustadt — 3-wing Gründerzeit station + clock tower + glazed canopy)
// ----------------------------------------------------------------------------
pub(super) fn build_station(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [320.0, 120.0, 140.0]);
    let stone = Mat::matte(0x64748b, 0.88);
    let platform = Mat::matte(0x94a3b8, 0.85);
    let forecourt = Mat::matte(0xcbd5e1, 0.84);
    let tactile = Mat::matte(0xfef08a, 0.72);
    let wall = Mat::matte(0xe5d6be, 0.78);
    let brick = Mat::matte(0x9a3412, 0.78);
    let cornice = Mat::matte(0xd6d3d1, 0.74);
    let roof = Mat::matte(0x991b1b, 0.68);
    let ridge = Mat::matte(0x7c2d12, 0.72);
    let copper = Mat::metal(0x2d6a56, 0.42, 0.35);
    let iron = Mat::metal(0x1e293b, 0.40, 0.72);
    let canopy_m = Mat::metal(0x334155, 0.50, 0.45);
    let skylight = Mat::glass(0x93c5fd);
    let win_dark = Mat::warm_window(0xfde68a, 0.38);
    let win_warm = Mat::warm_window(0xfde68a, 0.92);
    let frame = Mat::matte(0xf8fafc, 0.62);
    let door_w = Mat::matte(0x5c2e14, 0.74);
    let bench_w = Mat::matte(0x92400e, 0.78);
    let sign_blue = Mat::metal(0x1d4ed8, 0.45, 0.15);
    let sign_white = Mat::matte(0xf8fafc, 0.55);
    let brass = Mat::metal(0xd97706, 0.28, 0.82);
    let lamp_glow = Mat::emissive(0xfef08a, 0.95);
    let shrub_m = Mat::matte(0x15803d, 0.90);

    // 1. Plinth, house platform (+Z) & forecourt (-Z) — non-overlapping Y slabs prevent depth z-fighting
    b.rbox([0.0, 2.0, 0.0], [320.0, 4.0, 140.0], stone);
    b.rbox([0.0, 5.1, 48.0], [316.0, 2.2, 42.0], platform);
    b.rbox([0.0, 5.25, 68.5], [316.0, 2.5, 3.0], cornice);
    b.rbox([0.0, 6.45, 63.5], [312.0, 0.5, 2.2], tactile);
    b.rbox([0.0, 4.6, -60.0], [316.0, 1.2, 20.0], forecourt);

    // 2. Central main hall
    b.rbox([0.0, 7.0, -12.0], [142.0, 6.0, 88.0], stone);
    b.rbox([0.0, 41.0, -12.0], [140.0, 62.0, 86.0], wall);
    b.rbox([0.0, 40.0, -12.0], [143.0, 2.5, 89.0], cornice);
    b.rbox([0.0, 72.0, -12.0], [145.0, 3.0, 91.0], cornice);
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([sx * 68.5, 41.0, -12.0 + sz * 41.5], [4.5, 62.0, 4.5], brick);
        }
    }
    b.gable_roof_x([0.0, 73.5, -12.0], 144.0, 88.0, 26.0, 3.5, 2.0, roof, wall, ridge);

    // 4 roof dormers + brick chimney
    for dx in [-42.0_f32, 42.0] {
        for side in [-1.0_f32, 1.0] {
            let dz = -12.0 + side * 24.0;
            b.rbox([dx, 82.0, dz], [15.0, 10.0, 14.0], wall);
            b.rbox([dx, 87.8, dz], [17.5, 2.4, 16.5], roof);
            b.rbox([dx, 82.5, dz + side * 7.2], [7.5, 6.2, 0.9], win_warm);
        }
    }
    b.rbox([52.0, 94.0, -24.0], [9.5, 22.0, 9.5], brick);
    b.rbox([52.0, 105.6, -24.0], [12.0, 2.2, 12.0], cornice);

    // 3. Symmetrical left & right wings
    for side_x in [-1.0_f32, 1.0] {
        let wx = side_x * 112.0;
        b.rbox([wx, 6.5, -12.0], [88.0, 5.0, 76.0], stone);
        b.rbox([wx, 31.0, -12.0], [86.0, 44.0, 74.0], wall);
        b.rbox([wx, 53.0, -12.0], [90.0, 2.5, 78.0], cornice);
        for sz in [-1.0_f32, 1.0] {
            b.rbox([wx + side_x * 41.5, 31.0, -12.0 + sz * 35.5], [4.0, 44.0, 4.0], brick);
        }
        b.gable_roof_x([wx, 54.2, -12.0], 88.0, 76.0, 20.0, 3.0, 1.8, roof, wall, ridge);
        b.window_x(wx + side_x * 43.1, 33.0, -12.0, side_x, 11.0, 14.5, win_warm, frame, cornice, brick);
    }

    // 4. Central Clock Tower (starts above hall roof base y=72.0 so inner walls never coplanar-fight) + 4 clock dials + spire
    b.rbox([0.0, 90.0, -12.0], [34.0, 36.0, 34.0], wall);
    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([sx * 16.0, 90.0, -12.0 + sz * 16.0], [3.0, 38.0, 3.0], brick);
        }
    }
    b.rbox([0.0, 86.0, -12.0], [37.0, 2.4, 37.0], cornice);
    b.rbox([0.0, 108.5, -12.0], [39.0, 2.8, 39.0], cornice);
    for side in [-1.0_f32, 1.0] {
        b.clock_face_z([0.0, 98.0, -12.0 + side * 17.2], side, 8.2, iron, sign_white, iron);
        b.clock_face_x([side * 17.2, 98.0, -12.0], side, 8.2, iron, sign_white, iron);
    }
    b.pyramid_y([0.0, 110.0, -12.0], 20.5, 20.5, 30.0, copper);
    b.sphere([0.0, 141.0, -12.0], 2.0, 6, 8, brass);
    b.cyl_y([0.0, 147.0, -12.0], 0.4, 0.9, 12.0, 8, brass);

    // 5. Windows, grand portals, DB station boards & planters
    for side in [-1.0_f32, 1.0] {
        let wz = -12.0 + side * 43.0;
        for dx in [-46.0_f32, -26.0, 26.0, 46.0] {
            let wm = if dx.abs() < 30.0 { win_warm } else { win_dark };
            b.window_z(dx, 24.0, wz, side, 10.5, 16.0, wm, frame, cornice, brick);
        }
        for dx in [-46.0_f32, -23.0, 0.0, 23.0, 46.0] {
            let wm = if dx == 0.0 || dx.abs() > 40.0 { win_warm } else { win_dark };
            b.window_z(dx, 55.0, wz, side, 10.5, 15.5, wm, frame, cornice, brick);
        }
        // Main entrance portal
        for px in [-12.0_f32, 12.0] {
            b.rbox([px, 20.5, wz + side * 1.8], [3.8, 28.0, 4.5], cornice);
        }
        b.rbox([0.0, 35.5, wz + side * 2.2], [29.0, 4.2, 5.5], cornice);
        b.rbox([0.0, 20.0, wz + side * 0.5], [20.0, 27.5, 1.2], iron);
        for dx in [-4.5_f32, 4.5] {
            b.rbox([dx, 16.5, wz + side * 1.1], [8.0, 21.0, 1.1], door_w);
        }
        b.rbox([0.0, 30.2, wz + side * 1.1], [16.8, 4.6, 1.1], win_warm);
        b.rbox([0.0, 5.2, wz + side * 3.4], [28.0, 2.0, 6.8], cornice);

        // Wing windows
        let wing_wz = -12.0 + side * 37.0;
        for side_x in [-1.0_f32, 1.0] {
            let wx = side_x * 112.0;
            for ox in [-24.0_f32, 0.0, 24.0] {
                let wm = if ox == 0.0 { win_warm } else { win_dark };
                b.window_z(wx + ox, 27.0, wing_wz, side, 10.0, 15.0, wm, frame, cornice, brick);
            }
        }
    }
    b.rbox([0.0, 43.0, -56.4], [54.0, 7.0, 1.2], sign_blue);
    b.rbox([0.0, 43.0, -56.7], [42.0, 2.4, 1.2], sign_white);
    for px in [-24.0_f32, 24.0] {
        b.rbox([px, 7.6, -60.0], [10.0, 5.4, 10.0], stone);
        b.organic_foliage_cluster([px, 14.5, -60.0], 5.5, 5.2, 5.5, (px as i32 + 50) as u32, shrub_m);
    }

    // 6. Trackside glazed canopy (+Z) on cast-iron columns, benches, hanging signs & lamps
    for side_x in [-1.0_f32, 1.0] {
        b.obox([side_x * 109.0, 41.6, 27.8], [80.0, 2.0, 6.4], [0.08, 0.0, 0.0], canopy_m);
    }
    b.obox([0.0, 40.8, 36.0], [298.0, 2.0, 13.0], [0.08, 0.0, 0.0], canopy_m);
    b.obox([0.0, 39.9, 47.5], [294.0, 1.2, 11.0], [0.08, 0.0, 0.0], skylight);
    b.obox([0.0, 38.9, 59.0], [298.0, 2.0, 13.0], [0.08, 0.0, 0.0], canopy_m);
    b.rbox([0.0, 37.8, 65.5], [298.0, 3.2, 1.5], iron);

    for i in 0..=5 {
        let x = -130.0 + (i as f32) * 52.0;
        b.rbox([x, 8.5, 58.0], [5.8, 5.0, 5.8], stone);
        b.cyl_y([x, 24.5, 58.0], 1.6, 2.0, 30.0, 10, iron);
        b.rbox([x, 38.0, 58.0], [4.8, 2.0, 4.8], iron);
        if x.abs() > 70.0 {
            b.obox([x, 38.6, 43.0], [2.0, 2.2, 41.0], [0.08, 0.0, 0.0], iron);
        } else {
            b.obox([x, 38.4, 46.0], [2.0, 2.2, 35.0], [0.08, 0.0, 0.0], iron);
        }
    }
    for sx in [-78.0_f32, 0.0, 78.0] {
        b.rbox([sx, 33.8, 56.0], [28.0, 5.0, 1.2], sign_blue);
        b.rbox([sx, 33.8, 56.0], [22.0, 1.8, 1.5], sign_white);
    }
    for lx in [-104.0_f32, -52.0, 52.0, 104.0] {
        b.cyl_y([lx, 36.4, 49.0], 2.6, 3.6, 2.0, 10, iron);
        b.sphere([lx, 35.0, 49.0], 1.7, 6, 8, lamp_glow);
    }
    for bx in [-95.0_f32, -40.0, 40.0, 95.0] {
        b.rbox([bx, 10.2, 42.0], [22.0, 1.2, 5.5], bench_w);
        b.rbox([bx, 13.0, 39.5], [22.0, 4.2, 1.0], bench_w);
        for lx in [-8.5_f32, 8.5] {
            b.rbox([bx + lx, 8.4, 42.0], [1.2, 4.4, 5.0], iron);
        }
    }
    b.rbox([22.0, 18.0, 32.0], [13.5, 15.0, 2.2], iron);
    b.rbox([22.0, 18.0, 32.35], [11.0, 12.2, 1.8], sign_white);
}

// ----------------------------------------------------------------------------
// 2. SIGNAL BOX (Stellwerk W1 — brick lower, cantilevered glazed upper, hip roof & steel stairs)
// ----------------------------------------------------------------------------
pub(super) fn build_signal_box(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [120.0, 110.0, 75.0]);
    let stone = Mat::matte(0x64748b, 0.88);
    let cornice = Mat::matte(0xd6d3d1, 0.75);
    let brick = Mat::matte(0x9a3412, 0.82);
    let upper_wall = Mat::matte(0xe5d6be, 0.76);
    let steel = Mat::metal(0x1e293b, 0.42, 0.72);
    let roof = Mat::matte(0x334155, 0.62);
    let ridge = Mat::matte(0x1e293b, 0.65);
    let win_dark = Mat::warm_window(0xfde68a, 0.35);
    let win_warm = Mat::warm_window(0xfde68a, 0.90);
    let frame = Mat::matte(0xf8fafc, 0.65);
    let door_w = Mat::matte(0x5c2e14, 0.78);
    let sign_blue = Mat::metal(0x1d4ed8, 0.45, 0.15);
    let sign_white = Mat::matte(0xf8fafc, 0.55);
    let desk = Mat::matte(0x475569, 0.60);
    let lever_red = Mat::metal(0xdc2626, 0.40, 0.40);
    let lever_blue = Mat::metal(0x2563eb, 0.40, 0.40);

    // 1. Concrete plinth & brick ground floor
    b.rbox([0.0, 2.5, 0.0], [118.0, 5.0, 73.0], stone);
    b.rbox([-4.0, 26.0, 0.0], [92.0, 42.0, 58.0], brick);
    b.rbox([-4.0, 48.0, 0.0], [108.0, 3.2, 70.0], cornice);
    for side in [-1.0_f32, 1.0] {
        for dx in [-32.0_f32, -8.0, 16.0] {
            let wm = if side > 0.0 && dx == -8.0 { win_warm } else { win_dark };
            b.window_z(dx, 27.0, side * 29.0, side, 8.6, 11.6, wm, frame, cornice, stone);
        }
    }
    b.rbox([42.1, 16.0, 0.0], [1.6, 22.0, 13.0], cornice);
    b.rbox([42.4, 15.5, 0.0], [1.6, 20.0, 10.5], door_w);

    // 2. Cantilevered interlocking room with open glazed observation bay & visible lever bank
    b.rbox([-4.0, 55.0, 0.0], [106.0, 11.0, 68.0], upper_wall);
    b.rbox([-4.0, 81.0, 0.0], [106.0, 6.0, 68.0], upper_wall);
    b.rbox([-4.0, 60.5, 0.0], [100.0, 1.0, 62.0], stone);
    b.rbox([-4.0, 69.5, -24.0], [96.0, 17.0, 1.5], win_warm);
    b.rbox([-4.0, 77.6, 4.0], [78.0, 0.8, 14.0], win_warm);
    b.rbox([-4.0, 60.8, 0.0], [106.4, 1.0, 68.4], steel);
    b.rbox([-4.0, 78.2, 0.0], [106.4, 1.0, 68.4], steel);
    b.rbox([-4.0, 64.0, 12.0], [66.0, 8.5, 9.0], desk);
    for lx in (-28..=20).step_by(8) {
        let lm = if lx % 16 == 0 { lever_red } else { lever_blue };
        b.obox([lx as f32, 70.0, 12.0], [1.2, 7.0, 1.2], [0.25, 0.0, 0.0], lm);
    }
    for side in [-1.0_f32, 1.0] {
        for mx in (-56..=48).step_by(13) {
            b.rbox([mx as f32, 69.5, side * 33.4], [2.2, 19.0, 2.2], steel);
        }
    }
    for side_x in [-56.0_f32, 48.0] {
        for k in 0..4 {
            let mz = -22.0 + (k as f32) * 14.5;
            b.rbox([side_x, 69.5, mz], [2.2, 19.0, 2.2], steel);
        }
    }
    for side in [-1.0_f32, 1.0] {
        b.rbox([-4.0, 55.0, side * 34.4], [34.0, 6.4, 1.2], sign_blue);
        b.rbox([-4.0, 55.0, side * 34.6], [26.0, 2.2, 1.2], sign_white);
    }

    // 3. Overhanging hip roof + vent + radio antenna
    b.rbox([-4.0, 85.5, 0.0], [116.0, 3.2, 78.0], roof);
    b.hip_roof([-4.0, 87.0, 0.0], 116.0, 78.0, 56.0, 17.5, roof, ridge);
    b.rbox([22.0, 99.0, -12.0], [9.5, 6.0, 9.5], stone);
    b.cyl_y([-36.0, 102.0, -14.0], 0.45, 0.85, 18.0, 8, steel);

    // 4. Exterior steel staircase on +X (inner stringer, intermediate posts & upper landing railings)
    b.rbox([54.0, 49.0, -18.0], [13.0, 2.0, 18.0], steel);
    for i in 0..11 {
        let fi = i as f32;
        b.rbox([54.0, 7.0 + fi * 4.1, 24.0 - fi * 3.8], [11.0, 1.4, 4.0], steel);
    }
    b.strut_between([59.5, 7.0, 25.0], [59.5, 49.0, -16.0], 1.3, 2.4, steel);
    b.strut_between([48.5, 7.0, 25.0], [48.5, 49.0, -16.0], 1.3, 2.4, steel);
    b.strut_between([59.5, 17.0, 25.0], [59.5, 59.0, -16.0], 1.0, 1.2, steel);
    b.strut_between([59.5, 7.0, 25.0], [59.5, 17.0, 25.0], 1.0, 1.0, steel);
    b.strut_between([59.5, 21.3, 11.0], [59.5, 31.3, 11.0], 1.0, 1.0, steel);
    b.strut_between([59.5, 35.7, -3.0], [59.5, 45.7, -3.0], 1.0, 1.0, steel);
    b.strut_between([59.5, 49.0, -16.0], [59.5, 59.0, -16.0], 1.0, 1.0, steel);
    b.strut_between([59.8, 49.0, -26.5], [59.8, 59.0, -26.5], 1.0, 1.0, steel);
    b.strut_between([48.0, 49.0, -26.5], [48.0, 59.0, -26.5], 1.0, 1.0, steel);
    b.strut_between([59.5, 59.0, -16.0], [59.8, 59.0, -26.5], 1.0, 1.0, steel);
    b.strut_between([59.8, 59.0, -26.5], [48.0, 59.0, -26.5], 1.0, 1.0, steel);
}

// ----------------------------------------------------------------------------
// 3. ENGINE SHED (Lokschuppen — open -X portal for snapped track, arched lintel, open doors, pit, vents)
// ----------------------------------------------------------------------------
pub(super) fn build_engine_shed(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [240.0, 105.0, 72.0]);
    let brick = Mat::matte(0x852e16, 0.85);
    let trim = Mat::matte(0xc4b5a5, 0.80);
    let concrete = Mat::matte(0x52525b, 0.90);
    let pit = Mat::matte(0x18181b, 0.95);
    let rail = Mat::metal(0xb0bcc9, 0.24, 0.85);
    let roof = Mat::matte(0x334155, 0.65);
    let skylight = Mat::glass(0x93c5fd);
    let win = Mat::warm_window(0xfde68a, 0.65);
    let frame = Mat::metal(0x1e293b, 0.55, 0.35);
    let door_w = Mat::matte(0x78350f, 0.85);
    let iron = Mat::metal(0x0f172a, 0.55, 0.55);
    let buf_red = Mat::matte(0xb91c1c, 0.70);
    let lamp_glow = Mat::emissive(0xfef08a, 0.95);

    // 1. Concrete foundation + inspection pit + raised interior H0 rails (top 9.6 mm) + rear buffer stop
    b.rbox([0.0, 1.5, 0.0], [242.0, 3.0, 72.0], concrete);
    b.rbox([10.0, 3.2, 0.0], [190.0, 0.8, 12.2], pit);
    b.rbox([-10.0, 5.3, -8.25], [220.0, 4.6, 4.2], concrete);
    b.rbox([-10.0, 5.3, 8.25], [220.0, 4.6, 4.2], concrete);
    b.rbox([-10.0, 8.6, -8.25], [220.0, 2.0, 1.5], rail);
    b.rbox([-10.0, 8.6, 8.25], [220.0, 2.0, 1.5], rail);
    b.rbox([95.0, 14.5, 0.0], [4.0, 5.5, 22.0], buf_red);
    b.rbox([95.0, 9.0, -8.0], [3.0, 12.0, 3.0], iron);
    b.rbox([95.0, 9.0, 8.0], [3.0, 12.0, 3.0], iron);

    // 2. Brick side & rear walls (clear opening at -X from Z=-22..+22, Y=3..87 for full catenary clearance)
    let wall_h = 94.0_f32;
    let wall_y = 3.0 + wall_h * 0.5;
    b.rbox([0.0, wall_y, -34.0], [240.0, wall_h, 3.0], brick);
    b.rbox([0.0, wall_y, 34.0], [240.0, wall_h, 3.0], brick);
    b.rbox([118.5, wall_y, 0.0], [3.0, wall_h, 70.0], brick);
    // Masonry pilasters along sides
    for px in [-96.0_f32, -48.0, 0.0, 48.0, 96.0] {
        b.rbox([px, wall_y, -35.8], [4.2, wall_h, 1.4], brick);
        b.rbox([px, wall_y, 35.8], [4.2, wall_h, 1.4], brick);
    }
    // 4 industrial workshop windows per side
    for sx in [-72.0_f32, -24.0, 24.0, 72.0] {
        for z_side in [-1.0_f32, 1.0] {
            b.window_z(sx, 48.0, z_side * 34.8, z_side, 22.0, 42.0, win, frame, trim, trim);
        }
    }

    // 3. Front -X portal piers, sandstone arch lintel, lantern & wide-open outward double doors
    let portal_w = 44.0_f32;
    let portal_h = 84.0_f32;
    let pillar_w = (70.0 - portal_w) * 0.5;
    b.rbox([-118.5, wall_y, -35.0 + pillar_w * 0.5], [3.8, wall_h, pillar_w], brick);
    b.rbox([-118.5, wall_y, 35.0 - pillar_w * 0.5], [3.8, wall_h, pillar_w], brick);
    b.rbox([-118.5, 3.0 + portal_h + (wall_h - portal_h) * 0.5, 0.0], [4.4, wall_h - portal_h, 70.0], trim);
    b.rbox([-120.5, 3.0 + portal_h + 1.5, 0.0], [2.4, 3.2, 5.0], trim); // Keystone
    b.rbox([-121.0, 3.0 + portal_h + 4.5, 0.0], [4.0, 1.8, 1.8], iron);
    b.sphere([-122.6, 3.0 + portal_h + 3.6, 0.0], 2.1, 6, 8, lamp_glow);

    let leaf_w = portal_w * 0.5;
    let leaf_h = portal_h - 1.5;
    let leaf_cy = 3.0 + leaf_h * 0.5;
    for &(side, yaw) in &[(-1.0_f32, 1.78_f32), (1.0_f32, -1.78_f32)] {
        let hinge_z = side * (portal_w * 0.5);
        let dir_z = -side;
        let (dx, dz) = rot_xz(0.0, dir_z * leaf_w * 0.5, yaw);
        b.obox([-120.0 + dx, leaf_cy, hinge_z + dz], [2.0, leaf_h, leaf_w], [0.0, -yaw, 0.0], door_w);
        b.obox([-120.0 + dx, leaf_cy + leaf_h * 0.28, hinge_z + dz], [2.6, 13.0, 8.0], [0.0, -yaw, 0.0], win);
        for hy in [-leaf_h * 0.36, 0.0, leaf_h * 0.36] {
            b.obox([-120.0 + dx * 0.85, leaf_cy + hy, hinge_z + dz * 0.85], [2.5, 2.2, leaf_w * 0.85], [0.0, -yaw, 0.0], iron);
        }
    }

    // 4. Pitched roof, split glazed skylight bays & 3 locomotive smoke jacks
    b.gable_roof_x([0.0, 97.0, 0.0], 242.0, 72.0, 18.0, 2.5, 2.0, roof, brick, trim);
    for sx in [-37.0_f32, 37.0] {
        b.rbox([sx, 116.5, 0.0], [50.0, 4.5, 11.5], skylight);
        b.rbox([sx, 119.0, 0.0], [52.0, 1.6, 2.0], iron);
    }
    for cx in [-75.0_f32, 0.0, 75.0] {
        b.cyl_y([cx, 121.0, 0.0], 4.2, 5.2, 11.0, 12, iron);
        b.cone_y([cx, 126.5, 0.0], 6.8, 4.0, 12, 0.0, iron);
    }
}

// ----------------------------------------------------------------------------
// 4. TURNTABLE TRAINHALL (Ringlokschuppen — true radial 3-stall fan around -Z turntable)
// ----------------------------------------------------------------------------
pub(super) fn build_turntable_trainhall(verts: &mut Vec<Vertex3D>, idxs: &mut Vec<u32>, origin: [f32; 3], rot: f32, size: [f32; 3]) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot, size, [320.0, 115.0, 310.0]);
    const TT_Z: f32 = -350.0;
    const R_FRONT: f32 = 195.0;
    const R_REAR: f32 = 505.0;
    let deg2rad = std::f32::consts::PI / 180.0;
    let stall_centers = [-12.0 * deg2rad, 0.0, 12.0 * deg2rad];
    let part_angles = [-18.0 * deg2rad, -6.0 * deg2rad, 6.0 * deg2rad, 18.0 * deg2rad];

    let y_ground = 0.0_f32;
    let y_floor = 3.5_f32;
    let y_eave = 96.0_f32;
    let y_ridge = 114.0_f32;

    let radial_pt = |r: f32, ang: f32, y: f32| -> [f32; 3] { [r * ang.sin(), y, r * ang.cos() + TT_Z] };

    let ground_m = Mat::matte(0x475569, 0.90);
    let brick_m = Mat::matte(0x8b3a2b, 0.83);
    let trim_m = Mat::matte(0xd4b996, 0.76);
    let roof_m = Mat::matte(0x1e293b, 0.68);
    let glass_m = Mat::warm_window(0xfde68a, 0.55);
    let skylight_m = Mat::glass(0x38bdf8);
    let door_m = Mat::matte(0x1e3a5f, 0.80);
    let steel_m = Mat::metal(0x334155, 0.42, 0.70);
    let rail_m = Mat::metal(0xb0bcc9, 0.24, 0.85);
    let chim_m = Mat::metal(0x1c1917, 0.58, 0.35);
    let lamp_m = Mat::emissive(0xfef08a, 0.95);
    let pit_m = Mat::matte(0x1f2937, 0.95);
    let stop_m = Mat::matte(0x78350f, 0.82);

    // 1. Curved radial concrete floor slab (16 segments + closed ±18 deg flank edges)
    let n_steps = 16usize;
    for j in 0..n_steps {
        let a1 = -18.0 * deg2rad + (36.0 * deg2rad * j as f32) / n_steps as f32;
        let a2 = -18.0 * deg2rad + (36.0 * deg2rad * (j + 1) as f32) / n_steps as f32;
        let am = 0.5 * (a1 + a2);
        let t1 = radial_pt(R_FRONT, a1, y_floor);
        let t2 = radial_pt(R_FRONT, a2, y_floor);
        let t3 = radial_pt(R_REAR, a2, y_floor);
        let t4 = radial_pt(R_REAR, a1, y_floor);
        b.quad_oriented(t1, t2, t3, t4, [0.0, 1.0, 0.0], ground_m);
        let b1 = radial_pt(R_FRONT, a1, y_ground);
        let b2 = radial_pt(R_FRONT, a2, y_ground);
        b.quad_oriented(b1, t1, t2, b2, [-am.sin(), 0.0, -am.cos()], ground_m);
        let b4 = radial_pt(R_REAR, a1, y_ground);
        let b3 = radial_pt(R_REAR, a2, y_ground);
        b.quad_oriented(b3, t3, t4, b4, [am.sin(), 0.0, am.cos()], ground_m);
    }
    let a_l = -18.0 * deg2rad;
    b.quad_oriented(
        radial_pt(R_REAR, a_l, y_ground),
        radial_pt(R_REAR, a_l, y_floor),
        radial_pt(R_FRONT, a_l, y_floor),
        radial_pt(R_FRONT, a_l, y_ground),
        [-a_l.cos(), 0.0, a_l.sin()],
        ground_m,
    );
    let a_r = 18.0 * deg2rad;
    b.quad_oriented(
        radial_pt(R_FRONT, a_r, y_ground),
        radial_pt(R_FRONT, a_r, y_floor),
        radial_pt(R_REAR, a_r, y_floor),
        radial_pt(R_REAR, a_r, y_ground),
        [a_r.cos(), 0.0, -a_r.sin()],
        ground_m,
    );

    // 2. Interior stall tracks (top 9.6 mm matching turntable exit), pits & buffer stops
    let track_len = R_REAR - R_FRONT - 25.0;
    let wall_h = y_eave - y_floor;
    let flank_len = R_REAR - R_FRONT;

    for &ang in &stall_centers {
        let rc = R_FRONT + track_len * 0.5;
        let p_c = radial_pt(rc, ang, 0.0);
        let (dx_l, dz_l) = (ang.cos() * (-8.25), -ang.sin() * (-8.25));
        let (dx_r, dz_r) = (ang.cos() * 8.25, -ang.sin() * 8.25);
        b.obox([p_c[0] + dx_l, 5.55, p_c[2] + dz_l], [4.0, 4.1, track_len], [0.0, ang, 0.0], ground_m);
        b.obox([p_c[0] + dx_r, 5.55, p_c[2] + dz_r], [4.0, 4.1, track_len], [0.0, ang, 0.0], ground_m);
        b.obox([p_c[0] + dx_l, 8.6, p_c[2] + dz_l], [1.6, 2.0, track_len], [0.0, ang, 0.0], rail_m);
        b.obox([p_c[0] + dx_r, 8.6, p_c[2] + dz_r], [1.6, 2.0, track_len], [0.0, ang, 0.0], rail_m);
        b.obox([p_c[0], y_floor + 0.3, p_c[2]], [11.0, 0.6, track_len - 28.0], [0.0, ang, 0.0], pit_m);
        let bs = radial_pt(R_FRONT + track_len - 6.0, ang, 14.5);
        b.obox(bs, [22.0, 6.5, 4.0], [0.0, ang, 0.0], stop_m);
        for s in [-1.0_f32, 1.0] {
            let off_x = ang.cos() * (s * 8.0);
            let off_z = -ang.sin() * (s * 8.0);
            b.obox([bs[0] + off_x, 9.0, bs[2] + off_z], [3.0, 11.0, 3.0], [0.0, ang, 0.0], steel_m);
        }
    }

    // 3. Side flank walls (-18 deg and +18 deg) with workshop windows
    for &side in &[-1.0_f32, 1.0] {
        let ang = side * 18.0 * deg2rad;
        let mid = radial_pt(R_FRONT + flank_len * 0.5, ang, y_floor + wall_h * 0.5);
        b.obox(mid, [4.0, wall_h, flank_len], [0.0, ang, 0.0], brick_m);
        for k in 0..4 {
            let fz = R_FRONT + 45.0 + k as f32 * 58.0;
            let wp = radial_pt(fz, ang, y_floor + 44.0);
            b.obox(wp, [4.6, 36.0, 28.0], [0.0, ang, 0.0], glass_m);
            b.obox([wp[0], y_floor + 24.5, wp[2]], [5.4, 2.4, 32.0], [0.0, ang, 0.0], trim_m);
        }
    }

    // 4. Segmented curved rear brick wall with windows & sandstone pilasters
    let rear_seg_w = R_REAR * 12.0 * deg2rad;
    for &ang in &stall_centers {
        let rw = radial_pt(R_REAR - 2.0, ang, y_floor + wall_h * 0.5);
        b.obox(rw, [rear_seg_w + 2.0, wall_h, 4.0], [0.0, ang, 0.0], brick_m);
        let rwin = radial_pt(R_REAR - 2.0, ang, y_floor + 46.0);
        b.obox(rwin, [rear_seg_w * 0.64, 42.0, 4.6], [0.0, ang, 0.0], glass_m);
    }
    for &pang in &part_angles {
        let pp = radial_pt(R_REAR - 1.0, pang, y_floor + wall_h * 0.5 + 0.5);
        b.obox(pp, [5.0, wall_h + 1.5, 6.0], [0.0, pang, 0.0], trim_m);
    }

    // 5. Front locomotive portals (clear opening 34 x 84 mm at R_FRONT facing turntable -Z) with open doors
    let portal_w = 34.0_f32;
    let portal_h = 84.0_f32;
    let front_seg_w = R_FRONT * 12.0 * deg2rad + 2.0;
    let lintel_h = wall_h - portal_h;
    let side_pw = (front_seg_w - portal_w) * 0.5;
    let leaf_w = portal_w * 0.5 - 0.5;
    let door_h = portal_h - 1.0;

    for &ang in &stall_centers {
        let dir_x = ang.sin();
        let dir_z = ang.cos();
        let right_x = ang.cos();
        let right_z = -ang.sin();

        let fc = [dir_x * (R_FRONT + 2.0), y_floor + portal_h + lintel_h * 0.5, TT_Z + dir_z * (R_FRONT + 2.0)];
        b.obox(fc, [front_seg_w, lintel_h, 4.0], [0.0, ang, 0.0], brick_m);

        for s in [-1.0_f32, 1.0] {
            let off = s * (front_seg_w * 0.5 - side_pw * 0.5);
            b.obox(
                [dir_x * (R_FRONT + 2.0) + right_x * off, y_floor + wall_h * 0.5, TT_Z + dir_z * (R_FRONT + 2.0) + right_z * off],
                [side_pw + 1.2, wall_h, 4.6],
                [0.0, ang, 0.0],
                trim_m,
            );
        }
        b.obox(
            [dir_x * (R_FRONT + 2.0), y_floor + portal_h + 1.6, TT_Z + dir_z * (R_FRONT + 2.0)],
            [portal_w + 4.0, 3.2, 5.0],
            [0.0, ang, 0.0],
            trim_m,
        );

        // Hinged open stall doors swinging 90 deg outward toward turntable
        for s in [-1.0_f32, 1.0] {
            let hx = dir_x * (R_FRONT + 2.0) + right_x * (s * portal_w * 0.5);
            let hz = TT_Z + dir_z * (R_FRONT + 2.0) + right_z * (s * portal_w * 0.5);
            let swing = -s * 1.57_f32;
            let leaf_ang = ang + swing;
            let local_cx = -s * leaf_w * 0.5;
            let ox = local_cx * leaf_ang.cos();
            let oz = -local_cx * leaf_ang.sin();
            b.obox([hx + ox, y_floor + door_h * 0.5, hz + oz], [leaf_w, door_h, 1.8], [0.0, leaf_ang, 0.0], door_m);
            b.obox([hx + ox, y_floor + door_h * 0.72, hz + oz], [8.5, 18.0, 2.3], [0.0, leaf_ang, 0.0], glass_m);
        }

        // Portal lantern facing turntable
        let lamp_p = radial_pt(R_FRONT - 1.0, ang, y_floor + portal_h + 6.0);
        b.sphere(lamp_p, 2.1, 6, 8, lamp_m);
    }

    // 6. True radial fan 3-gable pitched slate roof, brick gables, split skylights & 3 smoke stacks
    let r_roof_f = R_FRONT - 3.0;
    let r_roof_r = R_REAR + 5.0;
    for s in 0..3 {
        let a_l = part_angles[s];
        let a_c = stall_centers[s];
        let a_r = part_angles[s + 1];

        let fl = radial_pt(r_roof_f, a_l, y_eave);
        let fc = radial_pt(r_roof_f, a_c, y_ridge);
        let fr = radial_pt(r_roof_f, a_r, y_eave);
        let rl = radial_pt(r_roof_r, a_l, y_eave);
        let rc = radial_pt(r_roof_r, a_c, y_ridge);
        let rr = radial_pt(r_roof_r, a_r, y_eave);

        b.quad_oriented(fl, rl, rc, fc, [-a_c.cos(), 1.0, a_c.sin()], roof_m);
        b.quad_oriented(fc, rc, rr, fr, [a_c.cos(), 1.0, -a_c.sin()], roof_m);

        let flw = radial_pt(R_FRONT + 2.0, a_l, y_eave);
        let fcw = radial_pt(R_FRONT + 2.0, a_c, y_ridge);
        let frw = radial_pt(R_FRONT + 2.0, a_r, y_eave);
        b.tri_oriented(flw, fcw, frw, [-a_c.sin(), 0.0, -a_c.cos()], brick_m);

        let rlw = radial_pt(R_REAR - 2.0, a_l, y_eave);
        let rcw = radial_pt(R_REAR - 2.0, a_c, y_ridge);
        let rrw = radial_pt(R_REAR - 2.0, a_r, y_eave);
        b.tri_oriented(rrw, rcw, rlw, [a_c.sin(), 0.0, a_c.cos()], brick_m);

        // Split ridge skylights (front & rear bays leave locomotive chimney at R=335 clear)
        for &(sky_mid_r, sky_len) in &[(266.0_f32, 92.0_f32), (419.0, 122.0)] {
            b.obox(radial_pt(sky_mid_r, a_c, y_ridge + 2.4), [9.5, 4.8, sky_len], [0.0, a_c, 0.0], skylight_m);
            b.obox(radial_pt(sky_mid_r, a_c, y_ridge + 5.0), [10.8, 1.2, sky_len + 2.0], [0.0, a_c, 0.0], steel_m);
        }

        let ch_p = radial_pt(R_FRONT + 140.0, a_c, y_ridge + 9.0);
        b.cyl_y(ch_p, 4.8, 5.8, 18.0, 12, chim_m);
        b.cone_y([ch_p[0], ch_p[1] + 9.0, ch_p[2]], 7.8, 4.8, 12, 0.0, chim_m);
    }

    // Interior bay support columns at ±6 deg
    for &pang in &[-6.0 * deg2rad, 6.0 * deg2rad] {
        for pr in [R_FRONT + 75.0, R_FRONT + 155.0, R_FRONT + 235.0] {
            b.obox(radial_pt(pr, pang, y_floor + wall_h * 0.5), [3.5, wall_h, 3.5], [0.0, pang, 0.0], steel_m);
        }
    }
}

