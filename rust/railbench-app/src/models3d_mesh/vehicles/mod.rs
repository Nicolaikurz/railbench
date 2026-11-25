mod wagons_road;

use super::*;

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
    let loco_spec = match kind_id {
        "loco-br243" => Some((crate::trains_3d::TrainLocoKind::Br243, 191.3_f32)),
        "loco-br96" => Some((crate::trains_3d::TrainLocoKind::Br96Mallet, 203.4_f32)),
        "loco-v60" => Some((crate::trains_3d::TrainLocoKind::V60Diesel, 120.1_f32)),
        _ => None,
    };
    if let Some((loco_kind, nom_len)) = loco_spec {
        let scale = if gauge_mm > 0.5 {
            gauge_mm / 16.5
        } else {
            (size[0] / nom_len).max(0.25)
        };
        crate::trains_3d::push_locomotive_3d(
            verts,
            idxs,
            loco_kind,
            crate::trains_3d::TrainPose {
                pos: origin,
                yaw_rad: heading_rad,
                pitch_rad,
                scale,
            },
            crate::trains_3d::LocoAnimState {
                wheel_angle: 0.0,
                forward_lights: !driving_backward,
                headlights_on,
                pantograph_up: true,
                steam_on: false,
                steam_time: 0.0,
                speed_norm: 0.0,
                mallet_articulation_rad: 0.0,
            },
        );
        return;
    }
    match kind_id {
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
