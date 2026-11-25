use super::loco_builder::{LocoAnimState, LocoBuilder, Mat, TrainPose};
use crate::viewer3d::Vertex3D;

/// Builds a detailed DB V 60 (BR 260 / 360) C / 0-6-0 jackshaft rod-drive diesel shunter
/// in H0 scale (`120.1 mm` over buffers x `35.6 mm` width x `48.3 mm` cab roof).
pub fn push_v60_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pose: TrainPose,
    anim: LocoAnimState,
) {
    let mut b = LocoBuilder::new(verts, idxs, pose);

    let altrot = Mat::matte(0x9e1b24, 0.50);
    let cream = Mat::matte(0xf5ebd6, 0.54);
    let frame_black = Mat::matte(0x1b1f24, 0.78);
    let wheel_red = Mat::matte(0xb91c1c, 0.52);
    let tire_steel = Mat::metal(0xcbd5e1, 0.22, 0.88);
    let rod_steel = Mat::metal(0xdce3ec, 0.24, 0.88);
    let roof_grey = Mat::matte(0x5c6470, 0.62);
    let grill_dark = Mat::matte(0x121519, 0.82);
    let rubber = Mat::matte(0x111418, 0.88);
    let warn_yellow = Mat::matte(0xfacc15, 0.55);
    let cab_glass = Mat::warm_window(0xfde68a, 0.76);
    let glass_dark = Mat::glass(0x1e3244);

    let fwd_main = if anim.headlights_on && anim.forward_lights {
        Mat::emissive(0xfff5cc, 1.55)
    } else if anim.headlights_on && !anim.forward_lights {
        Mat::emissive(0xef4444, 1.15)
    } else {
        Mat::glass(0x94a3b8)
    };
    let rear_main = if anim.headlights_on && !anim.forward_lights {
        Mat::emissive(0xfff5cc, 1.55)
    } else if anim.headlights_on && anim.forward_lights {
        Mat::emissive(0xef4444, 1.15)
    } else {
        Mat::glass(0x94a3b8)
    };

    // =========================================================================
    // 1. HEAVY BAR CHASSIS, 3 SPOKED AXLES + JACKSHAFT (BLINDWELLE) + COUPLING RODS
    // =========================================================================
    let r_wheel = 7.2_f32;
    let r_crank = r_wheel * 0.54;
    let axles = [-32.0_f32, -4.0, 24.0];
    let jackshaft_x = 10.0_f32;
    let (sa, ca) = anim.wheel_angle.sin_cos();

    for &ax in &axles {
        b.push_spoked_wheelset(ax, r_wheel, anim.wheel_angle, 12, true, tire_steel, wheel_red, frame_black);
    }

    // Heavy bar frame + underframe fuel/air tanks
    for sz in [-1.0_f32, 1.0] {
        b.rbox([0.0, 11.4, sz * 6.0], [108.0, 7.8, 1.8], frame_black);
    }
    b.rbox([-14.0, 8.6, 0.0], [14.0, 6.2, 15.0], frame_black);
    for sz in [-1.0_f32, 1.0] {
        b.cyl_x([-18.0, 10.2, sz * 11.6], 2.0, 2.0, 12.0, 10, frame_black);
    }

    // Signature DB V 60 Jackshaft (Blindwelle) counterweight crank + full side coupling rods
    for sz in [-1.0_f32, 1.0] {
        let jz = sz * 8.6;
        // Jackshaft axle boss
        b.cyl_z([jackshaft_x, r_wheel, 0.0], 1.4, 1.4, 16.5, 10, frame_black);
        // Rotating crimson jackshaft crank arm + counterweight lobe opposite the crankpin
        b.strut_between(
            [jackshaft_x - ca * 4.4, r_wheel - sa * 4.4, jz],
            [jackshaft_x + ca * r_crank, r_wheel + sa * r_crank, jz],
            3.6,
            1.7,
            wheel_red,
        );
        b.cyl_z([jackshaft_x + ca * r_crank, r_wheel + sa * r_crank, jz + sz * 0.65], 0.9, 0.9, 1.3, 10, tire_steel);

        // Continuous steel coupling rods connecting Axle 1 (-32) <-> Axle 2 (-4) <-> Jackshaft (+10) <-> Axle 3 (+24)
        let cy_pin = r_wheel + sa * r_crank;
        let rod_z = sz * 9.5;
        b.strut_between(
            [-32.0 + ca * r_crank, cy_pin, rod_z],
            [24.0 + ca * r_crank, cy_pin, rod_z],
            1.35,
            0.80,
            rod_steel,
        );
    }

    // =========================================================================
    // 2. WALKWAYS, BUFFER BEAMS, CORNER SHUNTER STEPS & PERIMETER HANDRAILS
    // =========================================================================
    b.rbox([0.0, 16.6, 0.0], [112.0, 1.8, 34.8], frame_black);
    for dir_x in [-1.0_f32, 1.0] {
        b.rbox([dir_x * 55.6, 13.0, 0.0], [2.4, 5.2, 34.4], wheel_red);
        b.push_buffer_beam_details(dir_x * 56.8, 12.6, dir_x, frame_black, rubber);
        b.rbox([dir_x * 53.0, 6.2, 0.0], [1.8, 4.6, 22.0], frame_black);
        // End platform deck handrail
        b.cyl_z([dir_x * 54.0, 27.2, 0.0], 0.34, 0.34, 30.0, 8, rod_steel);
        b.cyl_z([dir_x * 54.0, 22.4, 0.0], 0.32, 0.32, 30.0, 8, rod_steel);
        for sz in [-1.0_f32, 1.0] {
            // Corner shunter steps (Rangierertritte) with yellow tread + grab pole
            b.rbox([dir_x * 51.0, 9.2, sz * 15.2], [7.2, 1.1, 3.4], warn_yellow);
            b.rbox([dir_x * 51.0, 12.6, sz * 15.2], [7.2, 1.1, 3.4], frame_black);
            b.cyl_y([dir_x * 54.0, 22.2, sz * 15.0], 0.34, 0.34, 11.4, 8, rod_steel);
            b.cyl_y([dir_x * 46.8, 22.2, sz * 15.0], 0.34, 0.34, 11.4, 8, rod_steel);
        }
    }
    // Side walkway safety railings along front & rear hoods
    for sz in [-1.0_f32, 1.0] {
        let rz = sz * 15.6;
        // Front hood side railing (+5 .. +48)
        b.cyl_x([27.0, 27.2, rz], 0.32, 0.32, 46.0, 8, rod_steel);
        for sx in [8.0_f32, 26.0, 44.0] {
            b.cyl_y([sx, 22.2, rz], 0.32, 0.32, 10.6, 8, rod_steel);
        }
        // Rear hood side railing (-48 .. -20)
        b.cyl_x([-34.0, 27.2, rz], 0.32, 0.32, 30.0, 8, rod_steel);
        for sx in [-45.0_f32, -33.0, -21.0] {
            b.cyl_y([sx, 22.2, rz], 0.32, 0.32, 10.6, 8, rod_steel);
        }
    }

    // =========================================================================
    // 3. ASYMMETRIC LONG FRONT HOOD (+5..+44) & SHORT REAR HOOD (-44..-20)
    // =========================================================================
    // Long front diesel engine hood
    b.rbox([24.5, 25.8, 0.0], [39.0, 16.6, 21.4], altrot);
    b.rbox([24.5, 34.8, 0.0], [38.5, 2.0, 18.6], altrot);
    // Crisp Cream/Ivory waist stripe + front V-chevron emblem
    b.rbox([24.5, 23.0, 0.0], [39.3, 2.2, 21.7], cream);
    b.obox([44.1, 21.8, -3.5], [0.5, 1.8, 7.6], [-0.45, 0.0, 0.0], cream);
    b.obox([44.1, 21.8, 3.5], [0.5, 1.8, 7.6], [0.45, 0.0, 0.0], cream);
    // Front cooling radiator grille + side louver grilles
    b.rbox([44.15, 29.0, 0.0], [0.55, 7.8, 15.0], grill_dark);
    for gz in (-5..=5).step_by(2) {
        b.rbox([44.3, 29.0, gz as f32 * 1.2], [0.4, 7.2, 0.45], rod_steel);
    }
    for lx in [15.0_f32, 26.0, 36.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox([lx, 29.2, sz * 10.8], [7.2, 6.2, 0.45], grill_dark);
        }
    }
    // Exhaust stack ahead of cab
    b.cyl_y([8.5, 38.6, 0.0], 1.25, 1.45, 6.2, 10, frame_black);

    // Short rear auxiliary hood
    b.rbox([-32.0, 25.0, 0.0], [24.0, 15.0, 21.4], altrot);
    b.rbox([-32.0, 33.2, 0.0], [23.5, 1.8, 18.6], altrot);
    b.rbox([-32.0, 23.0, 0.0], [24.3, 2.2, 21.7], cream);
    b.obox([-44.1, 21.8, -3.5], [0.5, 1.8, 7.6], [-0.45, 0.0, 0.0], cream);
    b.obox([-44.1, 21.8, 3.5], [0.5, 1.8, 7.6], [0.45, 0.0, 0.0], cream);
    b.rbox([-44.15, 28.4, 0.0], [0.55, 6.8, 14.0], grill_dark);
    for sz in [-1.0_f32, 1.0] {
        b.rbox([-32.0, 28.4, sz * 10.8], [13.0, 5.2, 0.45], grill_dark);
    }

    // =========================================================================
    // 4. ELEVATED PANORAMIC DRIVER CAB (-20..+5 mm, Y = 17.5..48.3 mm)
    // =========================================================================
    // Lower cab waist (Altrot + Cream belt + cab doors)
    b.rbox([-7.5, 25.2, 0.0], [25.0, 15.6, 30.8], altrot);
    b.rbox([-7.5, 23.0, 0.0], [25.3, 2.2, 31.1], cream);
    // Upper windowed cab section (Y = 33..45.8)
    b.rbox([-7.5, 39.4, 0.0], [25.0, 12.8, 30.8], altrot);
    // Cab sun-visor overhanging shallow-arched slate-grey roof (top 48.2 mm, antenna 51.8 mm)
    b.rbox([-7.5, 46.5, 0.0], [29.0, 1.5, 32.8], roof_grey);
    b.rbox([-7.5, 47.6, 0.0], [28.6, 1.2, 22.0], roof_grey);
    for sz in [-1.0_f32, 1.0] {
        b.obox([-7.5, 47.0, sz * 12.8], [28.6, 1.1, 6.8], [-sz * 0.18, 0.0, 0.0], roof_grey);
    }
    // Roof air horn & shunter radio antenna
    b.cyl_x([-2.0, 48.2, 4.5], 0.55, 1.0, 3.6, 8, rod_steel);
    b.cyl_y([-12.5, 49.6, -4.5], 0.25, 0.45, 4.5, 6, frame_black);

    // Panoramic silver-framed cab windows on front, rear & both sides + interior control stand
    b.rbox([-7.5, 34.5, 0.0], [9.0, 5.0, 18.0], frame_black);
    for dir_x in [-1.0_f32, 1.0] {
        let wx = -7.5 + dir_x * 12.6;
        for sz in [-1.0_f32, 1.0] {
            b.rbox([wx, 39.8, sz * 7.2], [0.55, 7.6, 11.2], rod_steel);
            b.rbox([wx + dir_x * 0.12, 39.8, sz * 7.2], [0.55, 6.4, 10.0], cab_glass);
        }
    }
    for sz in [-1.0_f32, 1.0] {
        let wz = sz * 15.5;
        b.rbox([-7.5, 39.8, wz], [18.8, 7.6, 0.55], rod_steel);
        b.rbox([-7.5, 39.8, wz + sz * 0.12], [17.4, 6.4, 0.55], cab_glass);
        // Side cab doors + vertical steel handrails
        b.rbox([-7.5, 25.0, wz], [9.2, 14.0, 0.5], glass_dark);
        b.rbox([-7.5, 25.0, wz], [9.0, 13.8, 0.55], altrot);
        b.cyl_y([-12.4, 24.5, sz * 16.0], 0.34, 0.34, 13.0, 8, rod_steel);
        b.cyl_y([-2.6, 24.5, sz * 16.0], 0.34, 0.34, 13.0, 8, rod_steel);
    }

    // =========================================================================
    // 5. DIRECTIONAL DB TRIPLE A-HEADLIGHTS (Zg 1) FRONT & REAR
    // =========================================================================
    // Forward end (+X)
    b.push_lantern([44.4, 33.0, 0.0], 1.0, 1.35, frame_black, fwd_main);
    for sz in [-1.0_f32, 1.0] {
        b.push_lantern([49.0, 19.6, sz * 11.4], 1.0, 1.35, frame_black, fwd_main);
    }
    // Rear end (-X)
    b.push_lantern([-44.4, 31.8, 0.0], -1.0, 1.35, frame_black, rear_main);
    for sz in [-1.0_f32, 1.0] {
        b.push_lantern([-49.0, 19.6, sz * 11.4], -1.0, 1.35, frame_black, rear_main);
    }
}
