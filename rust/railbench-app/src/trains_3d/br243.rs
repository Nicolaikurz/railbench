use super::loco_builder::{LocoAnimState, LocoBuilder, Mat, PrimBuilder, TrainPose};
use crate::viewer3d::Vertex3D;

/// Builds a detailed DR BR 243 / DB BR 143 electric locomotive in H0 scale
/// (`191.3 mm` over buffers x `34.2 mm` width x `44.1 mm` roof, pantograph `~64 mm`).
pub fn push_br243_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pose: TrainPose,
    anim: LocoAnimState,
) {
    let mut b = LocoBuilder::new(verts, idxs, pose);

    let bordeaux = Mat::matte(0x7c1d26, 0.52);
    let cream = Mat::matte(0xf1e7d0, 0.56);
    let roof_grey = Mat::matte(0x6b7280, 0.62);
    let chassis = Mat::matte(0x1c2128, 0.78);
    let bogie_dark = Mat::matte(0x232933, 0.72);
    let steel = Mat::metal(0xb8c2cc, 0.26, 0.85);
    let tire_steel = Mat::metal(0xc8d0da, 0.22, 0.88);
    let disc_dark = Mat::metal(0x2d343e, 0.52, 0.45);
    let rubber = Mat::matte(0x111418, 0.88);
    let louver_m = Mat::metal(0x4b5563, 0.45, 0.60);
    let copper = Mat::metal(0xb45309, 0.32, 0.82);
    let insulator_m = Mat::matte(0x8c3b2b, 0.42);
    let panto_red = Mat::metal(0xb91c1c, 0.42, 0.45);
    let cab_glass = Mat::warm_window(0xfde68a, 0.72);
    let glass_dark = Mat::glass(0x1a2c3d);
    let buffer_beam_red = Mat::matte(0x991b1b, 0.58);

    let white_light = if anim.headlights_on {
        Mat::emissive(0xfff7d6, 1.55)
    } else {
        Mat::glass(0x94a3b8)
    };
    let red_tail = if anim.headlights_on {
        Mat::emissive(0xef4444, 1.20)
    } else {
        Mat::matte(0x6b1c1c, 0.50)
    };
    let lamp_off = Mat::glass(0x475569);

    // 1. Underframe deck, buffer beams, snowplows & central transformer/air tanks
    b.rbox([0.0, 14.0, 0.0], [182.0, 4.4, 32.8], chassis);
    b.rbox([0.0, 16.5, 0.0], [181.0, 1.6, 33.4], bordeaux);
    for dir_x in [-1.0_f32, 1.0] {
        b.rbox([dir_x * 91.5, 13.0, 0.0], [2.4, 4.6, 33.6], buffer_beam_red);
        b.push_buffer_beam_details(dir_x * 92.7, 12.6, dir_x, chassis, rubber);
        // Rail guards / snowplow (Bahnraeumer)
        b.obox([dir_x * 89.2, 7.2, 0.0], [3.4, 6.2, 26.0], [0.0, 0.0, dir_x * 0.22], chassis);
        // End-skirt foot steps
        for side_z in [-1.0_f32, 1.0] {
            b.rbox([dir_x * 88.2, 9.0, side_z * 14.8], [3.6, 1.0, 2.8], steel);
        }
    }
    // Underframe main transformer housing, cooling fins & compressed-air reservoirs (X = -20..+20)
    b.rbox([0.0, 9.8, 0.0], [36.0, 7.2, 22.0], chassis);
    for fx in (-14..=14).step_by(4) {
        b.rbox([fx as f32, 9.6, 0.0], [1.0, 6.2, 24.4], bogie_dark);
    }
    for sz in [-1.0_f32, 1.0] {
        b.cyl_x([-18.0, 10.6, sz * 12.6], 2.1, 2.1, 14.0, 10, bogie_dark);
        b.cyl_x([18.0, 10.6, sz * 12.6], 2.1, 2.1, 14.0, 10, bogie_dark);
    }

    // 2. Twin Bo'Bo' Bogies (centered at X = ±48.0 mm, axle spacing ±16.1 mm, wheel r = 7.2 mm)
    for bogie_x in [-48.0_f32, 48.0] {
        for ax in [-16.1_f32, 16.1] {
            b.push_solid_wheelset(bogie_x + ax, 7.2, anim.wheel_angle, tire_steel, disc_dark, chassis);
        }
        // Heavy articulated bogie sideframes, bolster & primary coil springs
        b.rbox([bogie_x, 9.2, 0.0], [38.0, 3.6, 14.0], bogie_dark);
        for sz in [-1.0_f32, 1.0] {
            b.rbox([bogie_x, 9.0, sz * 11.8], [41.5, 4.6, 2.2], bogie_dark);
            b.rbox([bogie_x, 11.4, sz * 12.2], [11.0, 3.0, 2.4], bogie_dark);
            for ax in [-16.1_f32, 16.1] {
                b.rbox([bogie_x + ax, 7.2, sz * 12.2], [4.6, 4.4, 2.6], bogie_dark);
                // Primary suspension springs left & right of each axle box
                for sp in [-3.4_f32, 3.4] {
                    b.cyl_y([bogie_x + ax + sp, 9.6, sz * 12.0], 1.1, 1.1, 2.8, 8, chassis);
                }
            }
            // Sand pipes
            b.strut_between([bogie_x - 20.0, 8.8, sz * 9.5], [bogie_x - 18.0, 2.8, sz * 8.2], 0.4, 0.4, chassis);
            b.strut_between([bogie_x + 20.0, 8.8, sz * 9.5], [bogie_x + 18.0, 2.8, sz * 8.2], 0.4, 0.4, chassis);
        }
    }

    // 3. Superstructure: DR Bordeaux-Red body + Cream/Ivory belly band + Sicken corrugations + Cabs
    // Lower skirt band (17.2..19.2)
    b.rbox([0.0, 18.2, 0.0], [181.0, 2.0, 34.0], bordeaux);
    // Ivory/Cream belly stripe wrapping around front & sides (19.2..22.8)
    b.rbox([0.0, 21.0, 0.0], [181.4, 3.6, 34.2], cream);
    // Main upper machine-room body (22.8..38.8)
    b.rbox([0.0, 30.8, 0.0], [166.0, 16.0, 34.0], bordeaux);
    // Sloped upper cantrail & shallow arched steel-grey roof (38.8..44.15 mm)
    b.rbox([0.0, 39.8, 0.0], [168.0, 2.2, 31.6], bordeaux);
    b.rbox([0.0, 42.1, 0.0], [168.0, 2.4, 22.0], roof_grey);
    b.rbox([0.0, 43.65, 0.0], [168.0, 1.0, 14.0], roof_grey);
    for sz in [-1.0_f32, 1.0] {
        b.obox([0.0, 41.5, sz * 12.8], [168.0, 1.8, 7.6], [-sz * 0.24, 0.0, 0.0], roof_grey);
    }

    // Signature 4 horizontal side corrugations (Sicken) along machine-room flanks
    for sy in [24.2_f32, 26.8, 29.4, 32.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([0.0, sy, sz * 17.15], [122.0, 0.65, 0.45], bordeaux);
        }
    }

    // Side machine-room ventilation louver banks & circular/square inspection windows
    for lx in [-42.0_f32, -22.0, 0.0, 22.0, 42.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([lx, 32.5, sz * 17.08], [13.0, 9.0, 0.65], steel);
            b.rbox([lx, 32.5, sz * 17.22], [11.6, 7.6, 0.5], louver_m);
        }
    }
    for wx in [-32.0_f32, 32.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([wx, 31.5, sz * 17.1], [5.8, 6.2, 0.6], steel);
            b.rbox([wx, 31.5, sz * 17.25], [4.6, 5.0, 0.5], glass_dark);
        }
    }

    // 4. Forward & Rear Beveled Driver's Cabs, Windshields, Doors, Grab Rails & Spitzensignal
    for &dir_x in &[1.0_f32, -1.0] {
        let is_leading = (dir_x > 0.0) == anim.forward_lights;
        let main_lens = if is_leading { white_light } else { lamp_off };
        let tail_lens = if !is_leading { red_tail } else { lamp_off };

        // Lower nose front below windshield (22.8..28.5)
        b.rbox([dir_x * 86.2, 25.6, 0.0], [10.0, 5.8, 33.8], bordeaux);
        // Cream belly stripe nose wrap (reinforced)
        b.rbox([dir_x * 90.8, 21.0, 0.0], [1.2, 3.6, 33.8], cream);

        // Raked upper cab face (Y = 28.5..39.2 sloping back from X = ±90.5 to ±86.8)
        b.obox([dir_x * 86.2, 33.8, 0.0], [7.6, 11.2, 33.6], [0.0, 0.0, dir_x * 0.22], bordeaux);
        // Beveled cab roof nose brow
        b.obox([dir_x * 84.8, 40.4, 0.0], [8.0, 3.4, 30.0], [0.0, 0.0, dir_x * 0.26], roof_grey);

        // Twin angled recessed cab windshields + dark seals + wipers
        for sz in [-1.0_f32, 1.0] {
            b.obox([dir_x * 88.9, 33.6, sz * 7.2], [1.0, 7.6, 11.4], [0.0, 0.0, dir_x * 0.22], rubber);
            b.obox([dir_x * 89.2, 33.6, sz * 7.2], [0.9, 6.4, 10.2], [0.0, 0.0, dir_x * 0.22], cab_glass);
            // Windshield wiper
            b.obox([dir_x * 89.6, 32.8, sz * 7.0], [0.5, 4.4, 0.45], [0.25 * sz, 0.0, dir_x * 0.22], rubber);
            // Corner cab window + cab entry door + vertical steel grab rails
            b.rbox([dir_x * 74.5, 32.8, sz * 17.08], [9.0, 7.2, 0.6], rubber);
            b.rbox([dir_x * 74.5, 32.8, sz * 17.22], [7.6, 5.8, 0.5], cab_glass);
            // Side cab door
            b.rbox([dir_x * 65.5, 28.2, sz * 17.05], [7.8, 17.4, 0.5], bordeaux);
            b.rbox([dir_x * 65.5, 32.6, sz * 17.15], [4.8, 5.4, 0.5], glass_dark);
            b.cyl_y([dir_x * 69.6, 26.8, sz * 17.6], 0.32, 0.32, 12.0, 8, steel);
            b.cyl_y([dir_x * 61.6, 26.8, sz * 17.6], 0.32, 0.32, 12.0, 8, steel);
            // Front waist handrail below windshield
            b.cyl_z([dir_x * 91.5, 27.6, sz * 6.5], 0.32, 0.32, 10.5, 8, steel);
        }

        // Triple A Spitzensignal: upper center headlight + twin dual-lens lower lamp clusters
        b.push_lantern([dir_x * 90.4, 27.5, 0.0], dir_x, 1.45, chassis, main_lens);
        for sz in [-1.0_f32, 1.0] {
            b.rbox([dir_x * 91.0, 18.2, sz * 11.4], [1.4, 2.8, 5.6], chassis);
            b.push_lantern([dir_x * 91.4, 18.2, sz * 12.6], dir_x, 1.15, chassis, main_lens);
            b.push_lantern([dir_x * 91.4, 18.2, sz * 10.1], dir_x, 0.90, chassis, tail_lens);
        }
    }

    // 5. Roof Busbar, Brake Resistor Grids & Dual Single-Arm Pantographs (Einholmstromabnehmer)
    b.rbox([0.0, 44.3, 0.0], [116.0, 1.2, 18.5], bogie_dark);
    for vx in [-18.0_f32, 0.0, 18.0] {
        b.cyl_y([vx, 44.9, 0.0], 5.6, 5.8, 1.2, 14, louver_m);
        b.cyl_y([vx, 45.3, 0.0], 2.0, 2.0, 1.4, 10, chassis);
    }
    // Porcelain insulators & copper high-voltage roof line
    for ix in [-36.0_f32, -10.0, 10.0, 36.0] {
        b.cyl_y([ix, 45.5, 4.5], 0.65, 0.85, 1.8, 8, insulator_m);
    }
    b.cyl_x([0.0, 46.5, 4.5], 0.38, 0.38, 92.0, 8, copper);

    // Trailing pantograph raises when pantograph_up is true (classic DB/DR practice)
    let rear_dir = if anim.forward_lights { -1.0_f32 } else { 1.0_f32 };
    for &px in &[-46.0_f32, 46.0] {
        let is_raised = anim.pantograph_up && (px * rear_dir > 0.0);
        push_single_arm_pantograph(&mut b, px, is_raised, panto_red, insulator_m, steel);
    }
}

fn push_single_arm_pantograph(
    b: &mut LocoBuilder<'_>,
    cx: f32,
    raised: bool,
    frame_m: Mat,
    insul_m: Mat,
    pan_m: Mat,
) {
    let base_y = 44.2_f32;
    // 3 base insulators + red base frame
    for &(dx, dz) in &[(-5.5_f32, -5.5_f32), (-5.5, 5.5), (5.5, 0.0)] {
        b.cyl_y([cx + dx, base_y + 1.0, dz], 0.7, 0.9, 2.0, 8, insul_m);
    }
    b.rbox([cx, base_y + 2.2, 0.0], [12.5, 0.9, 12.5], frame_m);

    // Knee hinge extends outward toward loco center (`dir = -cx.signum()`)
    let dir = if cx >= 0.0 { -1.0_f32 } else { 1.0_f32 };
    let pivot = [cx - dir * 3.5, base_y + 2.6, 0.0];
    let (knee, head_y) = if raised {
        ([cx + dir * 8.0, base_y + 10.6, 0.0], base_y + 18.8)
    } else {
        ([cx + dir * 9.5, base_y + 4.0, 0.0], base_y + 5.2)
    };
    let pan_center = [cx - dir * 1.0, head_y, 0.0];

    // Lower arm + diagonal stabilizer rod
    b.cyl_between([pivot[0], pivot[1], -3.0], [knee[0], knee[1], -2.2], 0.45, 0.45, 8, frame_m);
    b.cyl_between([pivot[0], pivot[1], 3.0], [knee[0], knee[1], 2.2], 0.45, 0.45, 8, frame_m);
    b.cyl_between([knee[0], knee[1], -2.4], [knee[0], knee[1], 2.4], 0.42, 0.42, 8, frame_m);
    // Twin upper V-arm tubes up to collector head
    b.cyl_between([knee[0], knee[1], -2.2], [pan_center[0], pan_center[1], -3.6], 0.36, 0.36, 8, frame_m);
    b.cyl_between([knee[0], knee[1], 2.2], [pan_center[0], pan_center[1], 3.6], 0.36, 0.36, 8, frame_m);
    // Collector pan & twin carbon contact strips (Schleifstuecke) with curved end horns
    b.rbox([pan_center[0], pan_center[1] + 0.5, 0.0], [4.2, 0.65, 16.5], pan_m);
    for sx in [-1.5_f32, 1.5] {
        b.rbox([pan_center[0] + sx, pan_center[1] + 0.9, 0.0], [0.7, 0.45, 18.0], pan_m);
    }
    for sz in [-1.0_f32, 1.0] {
        b.cyl_between(
            [pan_center[0], pan_center[1] + 0.8, sz * 8.6],
            [pan_center[0], pan_center[1] - 0.8, sz * 11.0],
            0.35,
            0.25,
            6,
            pan_m,
        );
    }
}
