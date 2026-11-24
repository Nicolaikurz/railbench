mod locos;
mod wagons_road;

use super::*;

use locos::{build_loco_diesel, build_loco_electric, build_loco_steam};
use wagons_road::{
    build_car_bus, build_car_sedan, build_car_truck, build_wagon_freight, build_wagon_passenger,
};

pub(super) fn rgb_hex(rgb: [f32; 3]) -> u32 {
    let r = (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u32;
    let g = (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u32;
    let b = (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u32;
    (r << 16) | (g << 8) | b
}

/// Computes local-Z half-gauge so wheel treads align with exact track gauge `±(gauge_mm * 0.5)` in world space.
pub(super) fn half_gauge_local(size: [f32; 3], nominal_w: f32, gauge_mm: f32) -> f32 {
    let sz = (size[2] / nominal_w.max(1.0)).max(0.05);
    if gauge_mm > 0.5 {
        (gauge_mm * 0.5) / sz
    } else {
        8.25
    }
}

/// Builds a 3D vehicle mesh aligned along local +X (front = +X, rear = -X),
/// resting with wheel treads / tires flush at `y = 0` (`origin[1]`), rotated by `heading_rad` and pitched by `pitch_rad`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_vehicle_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    kind_id: &str,
    origin: [f32; 3],
    heading_rad: f32,
    pitch_rad: f32,
    size: [f32; 3], // [length_x, height_y, width_z]
    gauge_mm: f32,
    livery_rgb: [f32; 3],
    headlights_on: bool,
    driving_backward: bool,
) {
    let livery = rgb_hex(livery_rgb);
    match kind_id {
        "loco-steam" => build_loco_steam(
            verts,
            idxs,
            origin,
            heading_rad,
            pitch_rad,
            size,
            gauge_mm,
            headlights_on,
            driving_backward,
        ),
        "loco-electric" => build_loco_electric(
            verts,
            idxs,
            origin,
            heading_rad,
            pitch_rad,
            size,
            gauge_mm,
            livery,
            headlights_on,
            driving_backward,
        ),
        "loco-diesel" => build_loco_diesel(
            verts,
            idxs,
            origin,
            heading_rad,
            pitch_rad,
            size,
            gauge_mm,
            livery,
            headlights_on,
            driving_backward,
        ),
        "wagon-passenger" => build_wagon_passenger(
            verts,
            idxs,
            origin,
            heading_rad,
            pitch_rad,
            size,
            gauge_mm,
            livery,
            headlights_on,
        ),
        "wagon-freight" => build_wagon_freight(
            verts,
            idxs,
            origin,
            heading_rad,
            pitch_rad,
            size,
            gauge_mm,
            livery,
        ),
        "car-truck" => build_car_truck(verts, idxs, origin, heading_rad, pitch_rad, size, livery, headlights_on),
        "car-bus" => build_car_bus(verts, idxs, origin, heading_rad, pitch_rad, size, livery, headlights_on),
        _ => build_car_sedan(verts, idxs, origin, heading_rad, pitch_rad, size, livery, headlights_on),
    }
}

pub(super) fn dir_lamps(headlights_on: bool, driving_backward: bool) -> (Mat, Mat) {
    let white = if headlights_on {
        Mat::emissive(0xfef08a, 1.08)
    } else {
        Mat::metal(0xe2e8f0, 0.24, 0.60)
    };
    let red = if headlights_on {
        Mat::emissive(0xef4444, 0.98)
    } else {
        Mat::matte(0x7f1d1d, 0.48)
    };
    if driving_backward { (red, white) } else { (white, red) }
}

/// Gauge-accurate machined steel wheelset with inner flange (`Spurkranz`), recessed wheel disc & hub boss.
/// Wheel tread sits flush on `y = 0` at lateral position `±half_g`.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_flanged_wheelset(
    b: &mut ModelBuilder<'_>,
    wx: f32,
    wheel_r: f32,
    half_g: f32,
    tire_m: Mat,
    center_m: Mat,
    axle_m: Mat,
) {
    for &side in &[-1.0_f32, 1.0] {
        let wz = side * half_g;
        // Machined steel running tread directly above rail head centerline
        b.cyl_z([wx, wheel_r, wz], wheel_r, wheel_r, 1.85, 16, tire_m);
        // Inner wheel flange (`Spurkranz`) hugging inside edge of rail head
        b.cyl_z(
            [wx, wheel_r, wz - side * 1.12],
            wheel_r + 0.72,
            wheel_r + 0.72,
            0.52,
            16,
            tire_m,
        );
        // Dark painted recessed wheel center + raised hub boss
        b.cyl_z([wx, wheel_r, wz], wheel_r * 0.76, wheel_r * 0.76, 2.02, 14, center_m);
        b.cyl_z(
            [wx, wheel_r, wz + side * 0.95],
            wheel_r * 0.28,
            wheel_r * 0.28,
            0.55,
            10,
            tire_m,
        );
    }
    b.cyl_z([wx, wheel_r, 0.0], (wheel_r * 0.22).max(0.85), (wheel_r * 0.22).max(0.85), half_g * 2.02, 8, axle_m);
}

/// Authentic red-spoked DB steam locomotive wheelset with machined steel tire, inner flange,
/// 12 radial spokes, crescent counterweight & outside crankpin boss.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_spoked_steam_wheelset(
    b: &mut ModelBuilder<'_>,
    wx: f32,
    wheel_r: f32,
    half_g: f32,
    red_m: Mat,
    backing_m: Mat,
    tire_m: Mat,
    steel_m: Mat,
    with_counterweight: bool,
) {
    let spoke_count = 12usize;
    let spoke_len = wheel_r * 0.64;
    let spoke_r = wheel_r * 0.50;
    for &side in &[-1.0_f32, 1.0] {
        let wz = side * half_g;
        // Machined steel tire + inner flange
        b.cyl_z([wx, wheel_r, wz], wheel_r, wheel_r, 1.90, 20, tire_m);
        b.cyl_z(
            [wx, wheel_r, wz - side * 1.12],
            wheel_r + 0.72,
            wheel_r + 0.72,
            0.52,
            20,
            tire_m,
        );
        // Red inner tire band + dark shadow recess so 12 radial spokes pop cleanly
        b.cyl_z([wx, wheel_r, wz], wheel_r * 0.87, wheel_r * 0.87, 1.98, 18, red_m);
        b.cyl_z([wx, wheel_r, wz], wheel_r * 0.79, wheel_r * 0.79, 1.60, 18, backing_m);

        for i in 0..spoke_count {
            let ang = (i as f32 / spoke_count as f32) * std::f32::consts::TAU;
            let sx = wx + ang.cos() * spoke_r;
            let sy = wheel_r + ang.sin() * spoke_r;
            b.obox(
                [sx, sy, wz + side * 0.15],
                [spoke_len * 0.88, (wheel_r * 0.11).max(0.55), 1.65],
                [0.0, 0.0, ang],
                red_m,
            );
        }
        // Central hub (`Nabe`)
        b.cyl_z([wx, wheel_r, wz + side * 0.22], wheel_r * 0.27, wheel_r * 0.27, 2.05, 12, red_m);

        if with_counterweight {
            // Crescent counterweight opposite bottom dead-center crankpin
            for &da in &[-0.38_f32, 0.0, 0.38] {
                let ca = std::f32::consts::FRAC_PI_2 + da;
                b.obox(
                    [wx + ca.cos() * (wheel_r * 0.58), wheel_r + ca.sin() * (wheel_r * 0.58), wz + side * 0.18],
                    [wheel_r * 0.42, wheel_r * 0.30, 1.85],
                    [0.0, 0.0, ca + std::f32::consts::FRAC_PI_2],
                    red_m,
                );
            }
            // Crankpin boss & silver crankpin
            let cpy = wheel_r - wheel_r * 0.44;
            b.cyl_z([wx, cpy, wz + side * 0.70], wheel_r * 0.19, wheel_r * 0.19, 1.35, 10, red_m);
            b.cyl_z([wx, cpy, wz + side * 1.35], wheel_r * 0.11, wheel_r * 0.11, 1.15, 10, steel_m);
        }
    }
    b.cyl_z([wx, wheel_r, 0.0], (wheel_r * 0.20).max(1.0), (wheel_r * 0.20).max(1.0), half_g * 2.02, 10, red_m);
}

/// Shared UIC buffer beam with twin flanged buffers (`Puffer` at `±10.05 mm`), screw coupler & hanging air hoses.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_buffers_and_coupler(
    b: &mut ModelBuilder<'_>,
    x_beam: f32,
    y_beam: f32,
    dir_x: f32,
    beam_m: Mat,
    steel_m: Mat,
    plate_m: Mat,
) {
    b.rbox([x_beam, y_beam, 0.0], [3.0, 4.8, 29.8], beam_m);
    for &sz in &[-10.05_f32, 10.05] {
        // Buffer base mounting flange + outer sleeve + plunger + large buffer head plate
        b.rbox([x_beam + dir_x * 1.4, y_beam, sz], [0.8, 2.8, 2.8], steel_m);
        b.cyl_x([x_beam + dir_x * 2.6, y_beam, sz], 1.25, 1.15, 2.2, 10, steel_m);
        b.cyl_x([x_beam + dir_x * 4.0, y_beam, sz], 0.85, 0.85, 1.8, 10, plate_m);
        b.cyl_x([x_beam + dir_x * 5.0, y_beam, sz], 2.35, 2.35, 0.72, 14, plate_m);
    }
    // Center screw coupler hook, tension link & hanging air brake hoses (red/yellow)
    b.rbox([x_beam + dir_x * 2.7, y_beam - 0.1, 0.0], [3.2, 1.1, 1.5], steel_m);
    b.rbox([x_beam + dir_x * 3.8, y_beam - 2.0, 0.0], [1.1, 2.8, 1.1], steel_m);
    let hose_red = Mat::matte(0xdc2626, 0.55);
    let hose_yel = Mat::matte(0xeab308, 0.55);
    for &(hz, hm) in &[(-4.2_f32, hose_red), (4.2_f32, hose_yel)] {
        b.cyl_between(
            [x_beam + dir_x * 1.6, y_beam - 1.2, hz],
            [x_beam + dir_x * 2.5, y_beam - 4.4, hz * 0.82],
            0.42,
            0.42,
            6,
            hm,
        );
        b.rbox([x_beam + dir_x * 1.8, y_beam - 0.8, hz], [1.1, 1.1, 1.1], hm);
    }
}

/// Shared smooth arched/barrel railway roof with end caps.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_curved_roof(
    b: &mut ModelBuilder<'_>,
    cx: f32,
    y_eave: f32,
    len_x: f32,
    half_z: f32,
    crown_h: f32,
    segs: usize,
    roof_m: Mat,
) {
    let x0 = cx - len_x * 0.5;
    let x1 = cx + len_x * 0.5;
    let mut ring: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let ang = (t - 0.5) * std::f32::consts::PI;
        let z = half_z * ang.sin();
        let y = y_eave + crown_h * ang.cos();
        let nz = ang.sin() / half_z.max(1.0);
        let ny = ang.cos() / crown_h.max(1.0);
        let nlen = (ny * ny + nz * nz).sqrt().max(1e-5);
        ring.push((y, z, ny / nlen, nz / nlen));
    }
    for i in 0..segs {
        let (y0, z0, ny0, nz0) = ring[i];
        let (y1, z1, ny1, nz1) = ring[i + 1];
        b.push_smooth_quad(
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z1],
            [x0, y1, z1],
            [0.0, ny0, nz0],
            [0.0, ny0, nz0],
            [0.0, ny1, nz1],
            [0.0, ny1, nz1],
            roof_m,
        );
        b.tri_oriented([x1, y_eave, 0.0], [x1, y0, z0], [x1, y1, z1], [1.0, 0.0, 0.0], roof_m);
        b.tri_oriented([x0, y_eave, 0.0], [x0, y1, z1], [x0, y0, z0], [-1.0, 0.0, 0.0], roof_m);
    }
}

/// Shared gauge-matched multi-axle bogie with outside sideframes, axleboxes,
/// primary/bolster coil springs, brake shoes flanking each wheel, and optional Indusi magnet.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_detailed_bogie(
    b: &mut ModelBuilder<'_>,
    cx: f32,
    rel_wheel_xs: &[f32],
    wheel_r: f32,
    half_g: f32,
    frame_m: Mat,
    wheel_center_m: Mat,
    tire_m: Mat,
    spring_m: Mat,
    has_indusi: bool,
) {
    let min_x = rel_wheel_xs.iter().copied().fold(f32::INFINITY, f32::min);
    let max_x = rel_wheel_xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let frame_len = (max_x - min_x) + wheel_r * 2.25;
    let side_z = half_g + 2.35;

    for &rx in rel_wheel_xs {
        let wx = cx + rx;
        push_flanged_wheelset(b, wx, wheel_r, half_g, tire_m, wheel_center_m, frame_m);
        // Brake shoes gripping front & rear of each wheel tread
        for &sz in &[-half_g, half_g] {
            for &dx in &[-(wheel_r * 0.88), wheel_r * 0.88] {
                b.obox(
                    [wx + dx, wheel_r * 0.92, sz],
                    [1.6, wheel_r * 0.65, 1.7],
                    [0.0, 0.0, -dx.signum() * 0.25],
                    frame_m,
                );
            }
        }
    }

    // Left & right outside bogie sideframes, axleboxes, springs & yaw dampers
    for &side in &[-1.0_f32, 1.0] {
        let sz = side * side_z;
        b.rbox([cx, wheel_r + 1.5, sz], [frame_len, 2.8, 1.75], frame_m);
        // Secondary bolster coil springs at bogie center
        b.cyl_y([cx - 2.8, wheel_r + 2.2, sz], 1.45, 1.45, 3.8, 10, spring_m);
        b.cyl_y([cx + 2.8, wheel_r + 2.2, sz], 1.45, 1.45, 3.8, 10, spring_m);
        // Brake cylinder on sideframe
        b.cyl_x([cx, wheel_r + 2.4, sz - side * 1.1], 1.1, 1.1, 5.2, 10, spring_m);
        for &rx in rel_wheel_xs {
            b.rbox([cx + rx, wheel_r, sz + side * 0.55], [4.0, 4.0, 1.9], frame_m);
            b.cyl_y([cx + rx - 2.4, wheel_r + 1.1, sz], 0.8, 0.8, 2.2, 8, spring_m);
            b.cyl_y([cx + rx + 2.4, wheel_r + 1.1, sz], 0.8, 0.8, 2.2, 8, spring_m);
        }
    }
    // Central bogie bolster cross-beam
    b.rbox([cx, wheel_r + 2.9, 0.0], [9.5, 2.4, side_z * 1.95], frame_m);

    // Trackside Indusi / PZB vehicle magnet mounted below right leading axlebox
    if has_indusi {
        let ind_m = Mat::metal(0x64748b, 0.42, 0.65);
        b.rbox([cx + max_x * 0.55, 1.9, side_z + 0.55], [5.8, 1.7, 1.5], ind_m);
    }
}

/// Shared DB single-arm pantograph (`DBS 54` / `Einholm-Stromabnehmer`) with porcelain insulators & twin collector strips.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_pantograph(
    b: &mut ModelBuilder<'_>,
    cx: f32,
    y_base: f32,
    dir_x: f32,
    base_m: Mat,
    arm_m: Mat,
    strip_m: Mat,
    insul_m: Mat,
) {
    for &ix in &[-6.8_f32, 6.8] {
        for &iz in &[-5.8_f32, 5.8] {
            b.cyl_y([cx + ix, y_base + 1.0, iz], 1.05, 1.35, 2.1, 10, insul_m);
            b.cyl_y([cx + ix, y_base + 1.4, iz], 1.25, 1.25, 0.45, 10, insul_m);
        }
    }
    b.rbox([cx, y_base + 2.35, 0.0], [16.2, 1.1, 14.0], base_m);
    let p_root = [cx - dir_x * 5.8, y_base + 3.0, 0.0];
    let p_knee = [cx + dir_x * 8.2, y_base + 7.4, 0.0];
    let p_head = [cx - dir_x * 1.4, y_base + 12.0, 0.0];
    b.cyl_between(p_root, p_knee, 0.68, 0.60, 8, arm_m);
    // Parallel control link rod beneath lower arm
    b.cyl_between(
        [cx - dir_x * 3.8, y_base + 2.8, 1.8],
        [p_knee[0] - dir_x * 0.6, p_knee[1] - 0.5, 1.4],
        0.32,
        0.32,
        6,
        arm_m,
    );
    for &sz in &[-3.2_f32, 3.2] {
        b.cyl_between([p_knee[0], p_knee[1], sz * 0.5], [p_head[0], p_head[1], sz], 0.44, 0.40, 8, arm_m);
    }
    // Twin graphite collector strips (`Schleifstuecke`) & curved end horns
    b.rbox([p_head[0] - 1.3, p_head[1] + 0.5, 0.0], [1.05, 0.8, 21.6], strip_m);
    b.rbox([p_head[0] + 1.3, p_head[1] + 0.5, 0.0], [1.05, 0.8, 21.6], strip_m);
    for &sz in &[-11.6_f32, 11.6] {
        b.obox([p_head[0], p_head[1] - 0.35, sz], [4.0, 0.65, 2.7], [sz.signum() * 0.45, 0.0, 0.0], arm_m);
    }
}
