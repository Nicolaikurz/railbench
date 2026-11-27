use super::*;

pub(in crate::viewer3d) fn push_hall_landscape_env_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    glass_idxs: &mut Vec<u32>,
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
    // (West wall leaves a 1.6 m central doorway opening aligned with the street driveway)
    let door_w = 1600.0_f32;
    let door_half = door_w * 0.5;
    let door_h = 2400.0_f32;
    let west_x = cx - hw_hall;
    let z_min_p = cz - hl_hall - 20.0;
    let z_max_p = cz + hl_hall + 20.0;
    let z_min_b = cz - hl_hall;
    let z_max_b = cz + hl_hall;

    push_box(verts, idxs, [cx, floor_y + 90.0, cz - hl_hall], [w_hall + 40.0, 180.0, 150.0], sandstone);
    push_box(verts, idxs, [cx, floor_y + 90.0, cz + hl_hall], [w_hall + 40.0, 180.0, 150.0], sandstone);
    push_box(verts, idxs, [west_x, floor_y + 90.0, 0.5 * (z_min_p + (cz - door_half))], [150.0, 180.0, (cz - door_half) - z_min_p], sandstone);
    push_box(verts, idxs, [west_x, floor_y + 90.0, 0.5 * ((cz + door_half) + z_max_p)], [150.0, 180.0, z_max_p - (cz + door_half)], sandstone);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 90.0, cz], [150.0, 180.0, l_hall + 40.0], sandstone);

    push_box(verts, idxs, [cx, floor_y + 520.0, cz - hl_hall], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [cx, floor_y + 520.0, cz + hl_hall], [w_hall, 1040.0, 120.0], brick);
    push_box(verts, idxs, [west_x, floor_y + 520.0, 0.5 * (z_min_b + (cz - door_half))], [120.0, 1040.0, (cz - door_half) - z_min_b], brick);
    push_box(verts, idxs, [west_x, floor_y + 520.0, 0.5 * ((cz + door_half) + z_max_b)], [120.0, 1040.0, z_max_b - (cz + door_half)], brick);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 520.0, cz], [120.0, 1040.0, l_hall], brick);
    push_box(verts, idxs, [west_x, floor_y + 1054.0, 0.5 * (z_min_p + (cz - door_half))], [148.0, 28.0, (cz - door_half) - z_min_p], sandstone);
    push_box(verts, idxs, [west_x, floor_y + 1054.0, 0.5 * ((cz + door_half) + z_max_p)], [148.0, 28.0, z_max_p - (cz + door_half)], sandstone);
    push_box(verts, idxs, [cx + hw_hall, floor_y + 1054.0, cz], [148.0, 28.0, l_hall + 40.0], sandstone);
    for sx in [-1.0_f32, 1.0] {
        push_box(verts, idxs, [cx + sx * hw_hall, floor_y + wall_h, cz], [105.0, 105.0, l_hall + 40.0], steel);
    }

    // Full-height brick walls with translucent glazed clerestory windows (side walls) and arched gables.
    // Wall panels sit between the sandstone/brick wainscot (1.04 m) and the steel eaves beam.
    let win_glass_col = if dark_mode { [0.26, 0.38, 0.54] } else { [0.58, 0.78, 0.92] };
    let win_glass_alpha = 0.28_f32;
    let roof_glass_col = if dark_mode { [0.24, 0.36, 0.52] } else { [0.62, 0.80, 0.94] };
    let roof_glass_alpha = 0.22_f32;
    let glass_mat = [0.06, 0.16, 0.0];
    let wall_t = 120.0_f32;
    let wain_h = 1040.0_f32;
    let sill_y = 1780.0_f32;
    let head_y = 2520.0_f32;
    let wall_bays = 5usize;
    for sx in [-1.0_f32, 1.0] {
        let wx = cx + sx * hw_hall;
        if sx < 0.0 {
            push_box(verts, idxs, [wx, floor_y + (wain_h + sill_y) * 0.5, 0.5 * (z_min_b + (cz - door_half))], [wall_t, sill_y - wain_h, (cz - door_half) - z_min_b], brick);
            push_box(verts, idxs, [wx, floor_y + (wain_h + sill_y) * 0.5, 0.5 * ((cz + door_half) + z_max_b)], [wall_t, sill_y - wain_h, z_max_b - (cz + door_half)], brick);
        } else {
            push_box(verts, idxs, [wx, floor_y + (wain_h + sill_y) * 0.5, cz], [wall_t, sill_y - wain_h, l_hall], brick);
        }
        push_box(verts, idxs, [wx, floor_y + (head_y + wall_h) * 0.5, cz], [wall_t, wall_h - head_y, l_hall], brick);
        for i in 0..=wall_bays {
            let pz = (cz - hl_hall) + (i as f32 / wall_bays as f32) * l_hall;
            push_box(verts, idxs, [wx, floor_y + (sill_y + head_y) * 0.5, pz], [wall_t, head_y - sill_y, 200.0], brick);
        }
        for i in 0..wall_bays {
            if sx < 0.0 && i == 2 {
                continue;
            }
            let z0 = (cz - hl_hall) + (i as f32 / wall_bays as f32) * l_hall + 100.0;
            let z1 = (cz - hl_hall) + ((i + 1) as f32 / wall_bays as f32) * l_hall - 100.0;
            push_glass_quad(
                verts,
                glass_idxs,
                [wx, floor_y + sill_y, z0],
                [wx, floor_y + sill_y, z1],
                [wx, floor_y + head_y, z1],
                [wx, floor_y + head_y, z0],
                [sx, 0.0, 0.0],
                win_glass_col,
                win_glass_alpha,
                glass_mat,
            );
        }
    }

    // Street-facing main entrance double door on west wall (aligned with driveway at z = cz)
    {
        let door_col = [0.16, 0.22, 0.19];
        let handle_col = [0.76, 0.80, 0.85];
        // Stone threshold + sandstone surround jamb pilasters & lintel
        push_box(verts, idxs, [west_x - 10.0, floor_y + 10.0, cz], [190.0, 20.0, door_w + 180.0], [0.34, 0.36, 0.40]);
        for s_z in [-1.0_f32, 1.0] {
            push_box(verts, idxs, [west_x - 12.0, floor_y + door_h * 0.5, cz + s_z * (door_half + 70.0)], [156.0, door_h, 140.0], sandstone);
        }
        push_box(verts, idxs, [west_x - 16.0, floor_y + door_h + 75.0, cz], [172.0, 150.0, door_w + 340.0], sandstone);

        // Steel door frame, center meeting stile, bottom kickplates, lock rail & top transom bar
        for s_z in [-1.0_f32, 1.0] {
            push_box(verts, idxs, [west_x, floor_y + door_h * 0.5, cz + s_z * (door_half - 24.0)], [68.0, door_h, 48.0], door_col);
        }
        push_box(verts, idxs, [west_x, floor_y + 2120.0, cz], [68.0, 60.0, door_w - 40.0], door_col);
        push_box(verts, idxs, [west_x, floor_y + 1060.0, cz], [72.0, 2120.0, 56.0], door_col);
        push_box(verts, idxs, [west_x, floor_y + 280.0, cz], [64.0, 520.0, door_w - 80.0], door_col);
        push_box(verts, idxs, [west_x, floor_y + 960.0, cz], [66.0, 76.0, door_w - 80.0], door_col);
        // Exterior & interior brushed-steel vertical pull handles
        for sx_off in [-48.0_f32, 48.0] {
            for sz_off in [-85.0_f32, 85.0] {
                push_box(verts, idxs, [west_x + sx_off, floor_y + 1040.0, cz + sz_off], [26.0, 360.0, 22.0], handle_col);
            }
        }
        // Translucent door vision panes & upper transom light
        for (za, zb) in [(cz - door_half + 48.0, cz - 28.0), (cz + 28.0, cz + door_half - 48.0)] {
            push_glass_quad(
                verts,
                glass_idxs,
                [west_x, floor_y + 540.0, za],
                [west_x, floor_y + 540.0, zb],
                [west_x, floor_y + 2090.0, zb],
                [west_x, floor_y + 2090.0, za],
                [-1.0, 0.0, 0.0],
                win_glass_col,
                win_glass_alpha,
                glass_mat,
            );
        }
        push_glass_quad(
            verts,
            glass_idxs,
            [west_x, floor_y + 2150.0, cz - door_half + 48.0],
            [west_x, floor_y + 2150.0, cz + door_half - 48.0],
            [west_x, floor_y + 2380.0, cz + door_half - 48.0],
            [west_x, floor_y + 2380.0, cz - door_half + 48.0],
            [-1.0, 0.0, 0.0],
            win_glass_col,
            win_glass_alpha,
            glass_mat,
        );

        // Cantilevered steel-and-glass street canopy, tie rods, flanking sconces & interior EXIT sign
        let canopy_y = floor_y + 2600.0;
        let canopy_half_z = door_half + 340.0;
        let canopy_proj = 1150.0_f32;
        push_box(verts, idxs, [west_x - canopy_proj - 30.0, canopy_y, cz], [56.0, 74.0, canopy_half_z * 2.0 + 48.0], steel);
        for s_z in [-1.0_f32, 1.0] {
            let cz_rail = cz + s_z * canopy_half_z;
            push_box(verts, idxs, [west_x - 60.0 - canopy_proj * 0.5, canopy_y, cz_rail], [canopy_proj, 66.0, 48.0], steel);
            push_box_segment(verts, idxs, [west_x - canopy_proj + 120.0, canopy_y + 30.0, cz_rail], [west_x - 60.0, floor_y + 3140.0, cz_rail], 26.0, 26.0, steel);
            // Exterior brass/steel wall sconce flanking entrance
            let sconce_z = cz + s_z * (door_half + 240.0);
            push_box(verts, idxs, [west_x - 75.0, floor_y + 2150.0, sconce_z], [70.0, 120.0, 46.0], steel_dark);
            push_rotated_box_mat(verts, idxs, [west_x - 115.0, floor_y + 2150.0, sconce_z], 0.0, [0.0, 0.0, 0.0], [58.0, 110.0, 68.0], [1.0, 0.90, 0.65], [0.20, 0.05, -1.15]);
        }
        push_glass_quad(
            verts,
            glass_idxs,
            [west_x - 60.0, canopy_y + 34.0, cz - canopy_half_z],
            [west_x - canopy_proj - 10.0, canopy_y + 18.0, cz - canopy_half_z],
            [west_x - canopy_proj - 10.0, canopy_y + 18.0, cz + canopy_half_z],
            [west_x - 60.0, canopy_y + 34.0, cz + canopy_half_z],
            normalize3([-16.0, canopy_proj, 0.0]),
            roof_glass_col,
            roof_glass_alpha,
            glass_mat,
        );
        // Interior glowing EXIT sign above street entrance
        push_rotated_box_mat(verts, idxs, [west_x + 72.0, floor_y + 2560.0, cz], 0.0, [0.0, 0.0, 0.0], [16.0, 72.0, 220.0], [0.18, 0.92, 0.42], [0.20, 0.05, 0.85]);
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
                let x0 = xc - ws * 0.5;
                let x1 = xc + ws * 0.5;
                push_glass_quad(
                    verts,
                    glass_idxs,
                    [x0, floor_y + band_top, gz],
                    [x1, floor_y + band_top, gz],
                    [x1, floor_y + ytop, gz],
                    [x0, floor_y + ytop, gz],
                    [0.0, 0.0, sz],
                    win_glass_col,
                    win_glass_alpha,
                    glass_mat,
                );
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

    // Panoramic industrial mullioned clerestory frames, curved translucent glass roof & glazed ridge lantern
    for i in 0..bays {
        let z0 = (cz - hl_hall) + (i as f32 / bays as f32) * l_hall;
        let z1 = (cz - hl_hall) + ((i + 1) as f32 / bays as f32) * l_hall;
        let z_mid = 0.5 * (z0 + z1);
        let bay_l = (l_hall / bays as f32) - 180.0;
        for sx in [-1.0_f32, 1.0] {
            if sx < 0.0 && i == 2 {
                continue;
            }
            let wx = cx + sx * hw_hall;
            push_box(verts, idxs, [wx, floor_y + 1780.0, z_mid], [44.0, 34.0, bay_l], steel);
            push_box(verts, idxs, [wx, floor_y + 2520.0, z_mid], [44.0, 34.0, bay_l], steel);
            for m in -1..=1 {
                let mz = z_mid + (m as f32) * (bay_l * 0.32);
                push_box(verts, idxs, [wx, floor_y + 2190.0, mz], [42.0, 2240.0, 28.0], steel);
            }
        }

        // Intermediate steel glazing rafters dividing each bay into 3 roof glass panes along Z
        for frac in [1.0_f32 / 3.0, 2.0 / 3.0] {
            let pz_sub = z0 + (z1 - z0) * frac;
            for s in 0..arch_segs {
                let u0 = s as f32 / arch_segs as f32;
                let u1 = (s + 1) as f32 / arch_segs as f32;
                let x0 = cx + (u0 - 0.5) * w_hall;
                let x1 = cx + (u1 - 0.5) * w_hall;
                let yu0 = floor_y + wall_h + (u0 * std::f32::consts::PI).sin() * arch_rise + 88.0;
                let yu1 = floor_y + wall_h + (u1 * std::f32::consts::PI).sin() * arch_rise + 88.0;
                push_box_segment(verts, idxs, [x0, yu0, pz_sub], [x1, yu1, pz_sub], 24.0, 30.0, steel);
            }
        }

        // Curved semi-transparent barrel-vault glass roof panels
        for s in 0..arch_segs {
            let u0 = s as f32 / arch_segs as f32;
            let u1 = (s + 1) as f32 / arch_segs as f32;
            let x0 = cx + (u0 - 0.5) * w_hall;
            let x1 = cx + (u1 - 0.5) * w_hall;
            let y0 = floor_y + wall_h + (u0 * std::f32::consts::PI).sin() * arch_rise + 114.0;
            let y1 = floor_y + wall_h + (u1 * std::f32::consts::PI).sin() * arch_rise + 114.0;
            let n = normalize3([-(y1 - y0), x1 - x0, 0.0]);
            push_glass_quad(
                verts,
                glass_idxs,
                [x0, y0, z0],
                [x1, y1, z0],
                [x1, y1, z1],
                [x0, y0, z1],
                n,
                roof_glass_col,
                roof_glass_alpha,
                glass_mat,
            );
        }

        // Glazed central ridge skylight lantern above crown
        let crown_y = floor_y + wall_h + arch_rise;
        for sx in [-1.0_f32, 1.0] {
            let lx = cx + sx * 640.0;
            push_box(verts, idxs, [lx, crown_y + 208.0, z_mid], [48.0, 184.0, 48.0], steel);
            push_box(verts, idxs, [lx, crown_y + 300.0, z_mid], [44.0, 36.0, z1 - z0], steel);
            push_glass_quad(
                verts,
                glass_idxs,
                [lx, crown_y + 116.0, z0],
                [lx, crown_y + 116.0, z1],
                [lx, crown_y + 300.0, z1],
                [lx, crown_y + 300.0, z0],
                [sx, 0.0, 0.0],
                roof_glass_col,
                0.24,
                glass_mat,
            );
        }
        // Pitched glazed skylight roof slopes
        let nl = normalize3([-140.0, 680.0, 0.0]);
        let nr = normalize3([140.0, 680.0, 0.0]);
        push_glass_quad(
            verts,
            glass_idxs,
            [cx - 680.0, crown_y + 300.0, z0],
            [cx, crown_y + 440.0, z0],
            [cx, crown_y + 440.0, z1],
            [cx - 680.0, crown_y + 300.0, z1],
            nl,
            roof_glass_col,
            0.24,
            glass_mat,
        );
        push_glass_quad(
            verts,
            glass_idxs,
            [cx, crown_y + 440.0, z0],
            [cx + 680.0, crown_y + 300.0, z0],
            [cx + 680.0, crown_y + 300.0, z1],
            [cx, crown_y + 440.0, z1],
            nr,
            roof_glass_col,
            0.24,
            glass_mat,
        );
    }

    // Continuous longitudinal steel roof purlins & lantern ridge beam along the full hall length
    for s in 0..=arch_segs {
        let u = s as f32 / arch_segs as f32;
        let px = cx + (u - 0.5) * w_hall;
        let py = floor_y + wall_h + (u * std::f32::consts::PI).sin() * arch_rise + 92.0;
        push_box(verts, idxs, [px, py, cz], [42.0, 46.0, l_hall + 80.0], steel);
    }
    let ridge_crown_y = floor_y + wall_h + arch_rise + 440.0;
    push_box(verts, idxs, [cx, ridge_crown_y, cz], [54.0, 54.0, l_hall + 80.0], steel);

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

