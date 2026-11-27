use super::loco_builder::{LocoAnimState, LocoBuilder, Mat, PrimBuilder, TrainPose};
use crate::viewer3d::Vertex3D;

/// Builds a detailed DRG BR 96 (Bavarian Gt 2x4/4) 0-8-8-0T articulated Mallet steam locomotive
/// in H0 scale (`203.4 mm` over buffers x `35.6 mm` width x `51.5 mm` height).
pub fn push_br96_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pose: TrainPose,
    anim: LocoAnimState,
) {
    let mut b = LocoBuilder::new(verts, idxs, pose);

    let black = Mat::matte(0x17191d, 0.72);
    let smokebox_black = Mat::matte(0x1f2329, 0.58);
    let red = Mat::matte(0xb91c1c, 0.54);
    let tire_steel = Mat::metal(0xcbd5e1, 0.24, 0.88);
    let rod_steel = Mat::metal(0xdce3ec, 0.26, 0.88);
    let brass = Mat::metal(0xd4a024, 0.30, 0.86);
    let dark_iron = Mat::metal(0x282d35, 0.55, 0.50);
    let coal = Mat::matte(0x141518, 0.65);
    let rubber = Mat::matte(0x111418, 0.88);
    let firebox_glass = Mat::warm_window(0xfb923c, 0.95);

    let fwd_lamp = if anim.headlights_on && anim.forward_lights {
        Mat::emissive(0xfff4c2, 1.55)
    } else if anim.headlights_on && !anim.forward_lights {
        Mat::emissive(0xef4444, 1.15)
    } else {
        Mat::glass(0x94a3b8)
    };
    let rear_lamp = if anim.headlights_on && !anim.forward_lights {
        Mat::emissive(0xfff4c2, 1.55)
    } else if anim.headlights_on && anim.forward_lights {
        Mat::emissive(0xef4444, 1.15)
    } else {
        Mat::glass(0x94a3b8)
    };

    // =========================================================================
    // 1. REAR HIGH-PRESSURE RIGID ENGINE UNIT (4 axles: -57, -41, -25, -9 mm)
    // =========================================================================
    let rear_axles = [-57.0_f32, -41.0, -25.0, -9.0];
    let r_wheel = 7.0_f32;
    let r_crank = r_wheel * 0.54;
    let (sa, ca) = anim.wheel_angle.sin_cos();

    for &ax in &rear_axles {
        b.push_spoked_wheelset(ax, r_wheel, anim.wheel_angle, 14, true, tire_steel, red, dark_iron);
    }
    // Red bar chassis sideframes + rear buffer beam
    for sz in [-1.0_f32, 1.0] {
        b.rbox([-37.0, 11.2, sz * 5.8], [96.0, 7.6, 1.8], red);
    }
    b.rbox([-96.5, 13.0, 0.0], [3.2, 5.2, 34.0], red);
    b.push_buffer_beam_details(-98.0, 12.6, -1.0, dark_iron, rubber);
    b.push_lantern([-96.2, 18.2, -10.5], -1.0, 1.35, dark_iron, rear_lamp);
    b.push_lantern([-96.2, 18.2, 10.5], -1.0, 1.35, dark_iron, rear_lamp);
    b.push_lantern([-95.4, 38.5, 0.0], -1.0, 1.35, dark_iron, rear_lamp);

    // High-pressure cylinders (at X = -1.0 mm, compact bore) + valve chests + Walschaerts drive to X = -25 mm
    for sz in [-1.0_f32, 1.0] {
        let cy_x = 0.0_f32;
        let cy_y = 8.4_f32;
        let cy_z = sz * 12.6_f32;
        b.cyl_x([cy_x, cy_y, cy_z], 4.2, 4.2, 11.5, 14, black);
        b.cyl_x([cy_x, cy_y + 4.2, cy_z * 0.94], 2.4, 2.4, 10.5, 10, dark_iron);
        // Slidebars & crosshead reciprocating along X
        let ch_x = -10.5 + ca * r_crank * 0.85;
        b.rbox([-10.5, cy_y + 0.9, cy_z], [9.5, 0.65, 0.8], rod_steel);
        b.rbox([-10.5, cy_y - 0.9, cy_z], [9.5, 0.65, 0.8], rod_steel);
        b.rbox([ch_x, cy_y, cy_z], [2.4, 2.1, 1.5], dark_iron);
        // Coupling rod connecting all 4 rear axles (-57 .. -9)
        let cy_pin = r_wheel + sa * r_crank;
        b.strut_between(
            [-57.0 + ca * r_crank, cy_pin, sz * 9.35],
            [-9.0 + ca * r_crank, cy_pin, sz * 9.35],
            1.15,
            0.75,
            rod_steel,
        );
        // Main connecting rod (crosshead -> axle 3 at -25 mm)
        b.strut_between(
            [ch_x, cy_y, cy_z],
            [-25.0 + ca * r_crank, cy_pin, sz * 10.2],
            1.25,
            0.80,
            rod_steel,
        );
        // Eccentric return crank & valve rod
        b.strut_between(
            [-25.0 + ca * r_crank, cy_pin, sz * 10.8],
            [-12.0, cy_y + 4.2, sz * 11.6],
            0.65,
            0.55,
            rod_steel,
        );
    }

    // =========================================================================
    // 2. FRONT ARTICULATED LOW-PRESSURE MALLET UNIT (pivots at X = +6.0 mm)
    // =========================================================================
    let front_axles = [17.0_f32, 33.0, 49.0, 65.0];
    b.with_sub_pivot(6.0, anim.mallet_articulation_rad, |f| {
        for &ax in &front_axles {
            f.push_spoked_wheelset(ax, r_wheel, anim.wheel_angle, 14, true, tire_steel, red, dark_iron);
        }
        // Front articulated red chassis frame + front buffer beam + rail guards
        for sz in [-1.0_f32, 1.0] {
            f.rbox([51.0, 11.2, sz * 5.8], [90.0, 7.6, 1.8], red);
        }
        f.rbox([96.5, 13.0, 0.0], [3.2, 5.2, 34.0], red);
        f.rbox([88.0, 16.6, 0.0], [18.0, 1.6, 33.6], red);
        f.push_buffer_beam_details(98.0, 12.6, 1.0, dark_iron, rubber);
        for sz in [-1.0_f32, 1.0] {
            f.obox([93.5, 5.6, sz * 8.2], [2.4, 5.6, 1.4], [0.0, 0.0, 0.25], red);
        }
        f.push_lantern([96.2, 18.2, -10.5], 1.0, 1.4, dark_iron, fwd_lamp);
        f.push_lantern([96.2, 18.2, 10.5], 1.0, 1.4, dark_iron, fwd_lamp);

        // Massive forward Low-Pressure steam cylinders (X = +79 mm) & rear-facing connecting rods to X = +33 mm
        for sz in [-1.0_f32, 1.0] {
            let cy_x = 79.5_f32;
            let cy_y = 8.8_f32;
            let cy_z = sz * 13.5_f32;
            f.cyl_x([cy_x, cy_y, cy_z], 5.3, 5.3, 12.5, 16, black);
            f.cyl_x([cy_x, cy_y, cy_z], 5.42, 5.42, 0.9, 16, brass);
            f.cyl_x([cy_x, cy_y + 4.8, cy_z * 0.94], 2.7, 2.7, 11.0, 10, dark_iron);

            // Crosshead & slidebars extending aft from front cylinders
            let ch_x = 67.5 + ca * r_crank * 0.85;
            f.rbox([68.0, cy_y + 0.95, cy_z], [10.0, 0.65, 0.8], rod_steel);
            f.rbox([68.0, cy_y - 0.95, cy_z], [10.0, 0.65, 0.8], rod_steel);
            f.rbox([ch_x, cy_y, cy_z], [2.6, 2.2, 1.6], dark_iron);

            // Front 4-axle coupling rod (+17 .. +65)
            let cy_pin = r_wheel + sa * r_crank;
            f.strut_between(
                [17.0 + ca * r_crank, cy_pin, sz * 9.35],
                [65.0 + ca * r_crank, cy_pin, sz * 9.35],
                1.15,
                0.75,
                rod_steel,
            );
            // Main connecting rod (crosshead -> axle +33 mm)
            f.strut_between(
                [ch_x, cy_y, cy_z],
                [33.0 + ca * r_crank, cy_pin, sz * 10.3],
                1.30,
                0.85,
                rod_steel,
            );
            // Valve gear link
            f.strut_between(
                [33.0 + ca * r_crank, cy_pin, sz * 10.9],
                [68.0, cy_y + 4.8, sz * 11.8],
                0.65,
                0.55,
                rod_steel,
            );
        }
    });

    // =========================================================================
    // 3. RUNNING BOARDS, LONG BOILER, SMOKEBOX, DOMES, CHIMNEY & SIDE TANKS
    // =========================================================================
    // Main continuous running board + red edge trim
    b.rbox([-6.0, 17.8, 0.0], [176.0, 1.2, 34.4], black);
    for sz in [-1.0_f32, 1.0] {
        b.rbox([-6.0, 17.8, sz * 17.25], [176.0, 1.0, 0.55], red);
    }

    // Horizontal Boiler Barrel (-48..+60) + Brass Bands + Smokebox (+60..+82)
    let boiler_y = 30.8_f32;
    let boiler_r = 10.6_f32;
    b.cyl_x([6.0, boiler_y, 0.0], boiler_r, boiler_r, 108.0, 22, black);
    for bx in [-36.0_f32, -16.0, 4.0, 24.0, 44.0] {
        b.cyl_x([bx, boiler_y, 0.0], boiler_r + 0.25, boiler_r + 0.25, 0.85, 22, brass);
    }
    b.cyl_x([71.0, boiler_y, 0.0], boiler_r + 0.45, boiler_r + 0.45, 22.0, 22, smokebox_black);
    b.rbox([71.0, 21.5, 0.0], [14.0, 8.0, 14.0], smokebox_black);
    // Conical Smokebox Door + central locking dart + upper front smokebox lantern
    b.hemi_cap_x([82.0, boiler_y, 0.0], boiler_r - 0.5, 2.8, 1.0, 4, 16, smokebox_black);
    b.rbox([85.0, boiler_y, 0.0], [0.8, 3.2, 0.8], steel_or_brass(brass));
    b.push_lantern([82.6, 40.4, 0.0], 1.0, 1.45, dark_iron, fwd_lamp);

    // Flared Chimney (+70 mm), Sandboxes, Tall Steam Dome (+24 mm), Brass Safety Valves & Whistle
    b.cyl_y([70.0, 45.2, 0.0], 2.75, 2.15, 9.6, 14, smokebox_black);
    b.cyl_y([70.0, 50.6, 0.0], 3.35, 2.75, 1.6, 14, brass);
    b.dome_y([24.0, 40.0, 0.0], 4.2, 5.4, 3.0, 16, black); // Tall main steam dome
    b.dome_y([45.0, 40.0, 0.0], 3.4, 3.6, 2.2, 12, black); // Sandbox 1
    b.dome_y([0.0, 40.0, 0.0], 3.4, 3.6, 2.2, 12, black); // Sandbox 2
    for vz in [-1.1_f32, 1.1] {
        b.cyl_y([-14.0, 42.8, vz], 0.85, 1.0, 4.0, 10, brass);
    }
    b.cyl_y([-28.0, 43.2, 1.5], 0.5, 0.6, 3.6, 8, brass); // Steam whistle

    // Boiler longitudinal handrails & feedwater pipes on both sides
    for sz in [-1.0_f32, 1.0] {
        b.cyl_x([12.0, 36.2, sz * 11.4], 0.35, 0.35, 118.0, 8, brass);
        b.cyl_x([14.0, 21.8, sz * 15.8], 0.6, 0.6, 116.0, 8, dark_iron);
    }

    // Stepped Massive Side Water Tanks (-48..+42) with filler hatches & footsteps
    for sz in [-1.0_f32, 1.0] {
        // Main rear tank block (-48..+24)
        b.rbox([-12.0, 27.6, sz * 13.0], [72.0, 18.4, 8.2], black);
        // Forward stepped extension (+24..+44) for crew sightlines
        b.rbox([34.0, 24.4, sz * 12.6], [20.0, 12.0, 7.6], black);
        // Water tank filler hatch & red frame pinstripe
        b.rbox([10.0, 37.2, sz * 13.4], [9.5, 1.2, 5.2], dark_iron);
        b.rbox([-12.0, 27.6, sz * 17.2], [70.0, 1.0, 0.35], red);
    }

    // =========================================================================
    // 4. ENCLOSED DRG DRIVER CAB, NUMBERPLATES & REAR COAL BUNKER
    // =========================================================================
    b.rbox([-62.0, 32.2, 0.0], [28.0, 27.6, 33.8], black);
    b.rbox([-62.0, 46.8, 0.0], [31.5, 2.4, 35.6], black);
    b.cyl_x([-62.0, 44.2, 0.0], 17.2, 17.2, 31.2, 14, black);
    b.rbox([-62.0, 48.2, 0.0], [11.0, 1.6, 14.0], dark_iron); // Cab roof hatch

    for sz in [-1.0_f32, 1.0] {
        // Twin framed side cab windows + subtle warm firebox glow + brass builder/number plates
        for wx in [-68.0_f32, -57.0] {
            b.rbox([wx, 37.2, sz * 17.05], [7.2, 9.2, 0.55], brass);
            b.rbox([wx, 37.2, sz * 17.22], [5.8, 7.8, 0.5], firebox_glass);
        }
        b.rbox([-62.5, 27.5, sz * 17.05], [14.0, 4.0, 0.5], brass); // Number plate "96 022"
        b.cyl_y([-48.5, 26.5, sz * 17.6], 0.36, 0.36, 17.0, 8, brass);
    }
    // Forward brass cab spectacle windows facing boiler top
    for sz in [-1.0_f32, 1.0] {
        b.cyl_x([-47.8, 42.6, sz * 9.5], 2.2, 2.2, 0.9, 12, brass);
        b.cyl_x([-47.4, 42.6, sz * 9.5], 1.6, 1.6, 0.8, 12, firebox_glass);
    }

    // Rear Coal Bunker (-95..-76) heaped with sculpted coal chunks + rear cab ladder
    b.rbox([-85.5, 27.8, 0.0], [19.0, 18.8, 32.6], black);
    b.rbox([-85.5, 37.4, 0.0], [16.5, 3.0, 29.5], coal);
    for (cx, cy, cz, sx, sy, sz) in [
        (-86.0_f32, 39.2_f32, 0.0_f32, 10.5_f32, 4.4_f32, 18.0_f32),
        (-81.5, 38.8, -6.5, 6.2, 3.8, 9.0),
        (-89.5, 38.6, 5.5, 6.5, 3.4, 10.0),
    ] {
        b.obox([cx, cy, cz], [sx, sy, sz], [0.22, 0.35, 0.14], coal);
    }
    for sz in [-1.0_f32, 1.0] {
        b.cyl_y([-95.4, 24.0, sz * 5.2], 0.35, 0.35, 20.0, 6, dark_iron);
    }
    for ry in (18..=34).step_by(4) {
        b.rbox([-95.4, ry as f32, 0.0], [0.6, 0.6, 10.4], dark_iron);
    }

    // Dynamic chimney steam puffs
    if anim.steam_on {
        b.push_steam_puffs([70.0, 51.6, 0.0], anim.steam_time, anim.speed_norm);
    }
}

#[inline]
const fn steel_or_brass(m: Mat) -> Mat {
    m
}
