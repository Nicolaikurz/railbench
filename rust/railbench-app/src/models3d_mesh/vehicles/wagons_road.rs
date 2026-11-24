use super::{
    half_gauge_local, push_buffers_and_coupler, push_curved_roof, push_detailed_bogie,
    push_flanged_wheelset,
};
use crate::models3d_mesh::{Mat, ModelBuilder};
use crate::viewer3d::Vertex3D;

// ============================================================================
//  4. DB UIC-X EXPRESS PASSENGER COACH (26.4m m-Wagen Bm 235)
// ============================================================================
#[allow(clippy::too_many_arguments)]
pub(super) fn build_wagon_passenger(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    gauge_mm: f32,
    livery: u32,
    lights_on: bool,
) {
    const NOM: [f32; 3] = [264.0, 46.5, 32.5];
    let hg = half_gauge_local(size, NOM[2], gauge_mm);
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, NOM).with_pitch(pitch_rad);

    let body_m = Mat::matte(livery, 0.38); // RAL 6020 Chromoxidgruen
    let skirt_m = Mat::matte(0x1f2937, 0.60);
    let trim_m = Mat::matte(0xfef3c7, 0.42);
    let roof_m = Mat::metal(0x64748b, 0.48, 0.40);
    let bogie_m = Mat::metal(0x1e293b, 0.54, 0.46);
    let steel_tire = Mat::metal(0xdbe3ec, 0.20, 0.90);
    let alu_m = Mat::metal(0xcbd5e1, 0.24, 0.82);
    let bellows_m = Mat::matte(0x111827, 0.86);
    let win_m = if lights_on {
        Mat::warm_window(0xfde68a, 0.85)
    } else {
        Mat::glass(0x93c5fd)
    };

    // 1) Underframe solebar, battery/heating/brake equipment boxes & 2 Minden-Deutz 36 bogies
    b.rbox([0.0, 10.0, 0.0], [250.0, 3.6, 28.6], bogie_m);
    b.rbox([-22.0, 7.0, 0.0], [34.0, 4.4, 21.5], bogie_m);
    b.rbox([26.0, 7.0, 0.0], [24.0, 4.2, 19.5], bogie_m);
    push_detailed_bogie(&mut b, -88.0, &[-11.5, 11.5], 5.1, hg, bogie_m, bogie_m, steel_tire, alu_m, false);
    push_detailed_bogie(&mut b, 88.0, &[-11.5, 11.5], 5.1, hg, bogie_m, bogie_m, steel_tire, alu_m, false);
    push_buffers_and_coupler(&mut b, 128.0, 10.2, 1.0, bogie_m, bogie_m, steel_tire);
    push_buffers_and_coupler(&mut b, -128.0, 10.2, -1.0, bogie_m, bogie_m, steel_tire);

    // 2) Main coach body (`Schuerze`) + waist trim + rain drip rail
    b.rbox([0.0, 12.2, 0.0], [220.0, 3.2, 31.4], skirt_m);
    b.rbox([0.0, 24.4, 0.0], [224.0, 23.2, 31.2], body_m);
    b.rbox([0.0, 18.8, 0.0], [226.0, 1.3, 31.6], trim_m);
    b.rbox([0.0, 35.8, 0.0], [252.0, 0.85, 31.6], alu_m);

    // 3) Authentic inward-tapered coach ends (`eingezogene Wagenenden`), folding doors & rubber gangways
    for &dir in &[-1.0_f32, 1.0] {
        b.obox([dir * 119.5, 24.2, 0.0], [16.0, 22.8, 28.8], [0.0, 0.0, 0.0], body_m);
        // Recessed pleated rubber gangway bellows (`Gummiwulstuebergang`)
        b.rbox([dir * 129.0, 23.6, 0.0], [5.8, 21.0, 16.5], bellows_m);
        b.rbox([dir * 129.5, 23.6, 0.0], [6.2, 18.5, 13.5], bogie_m);
        for &side in &[-1.0_f32, 1.0] {
            let dz = side * 14.6;
            // Recessed folding entry doors (`Drehfalttueren`), twin entry steps & vertical handrails
            b.rbox([dir * 119.5, 24.0, dz], [9.8, 21.2, 0.5], body_m);
            b.rbox([dir * 119.5, 27.6, dz + side * 0.15], [5.4, 8.2, 0.45], win_m);
            b.rbox([dir * 119.5, 10.8, dz + side * 0.4], [9.2, 1.2, 2.2], alu_m);
            b.cyl_y([dir * 125.0, 22.0, dz + side * 0.45], 0.30, 0.30, 14.0, 8, alu_m);
        }
    }

    // 4) 10 aluminum-framed split passenger compartment windows (`Uebersetzfenster`) + destination board
    for &side in &[-1.0_f32, 1.0] {
        let sz = side * 15.7;
        for k in 0..10 {
            let wx = -94.5 + k as f32 * 21.0;
            b.rbox([wx, 27.2, sz], [14.8, 10.6, 0.55], alu_m);
            b.rbox([wx, 27.2, sz + side * 0.16], [13.2, 9.0, 0.45], win_m);
            // Horizontal upper transom sash bar
            b.rbox([wx, 29.2, sz + side * 0.35], [13.6, 0.55, 0.35], alu_m);
        }
        b.rbox([0.0, 16.0, sz + side * 0.15], [14.0, 2.2, 0.4], trim_m); // Zuglaufschild
    }

    // 5) Arched barrel roof with tapered vestibule ends & 8 Kuckuck roof ventilators
    push_curved_roof(&mut b, 0.0, 36.0, 252.0, 15.6, 5.4, 14, roof_m);
    for k in 0..8 {
        let vx = -87.5 + k as f32 * 25.0;
        b.cyl_y([vx, 41.6, 0.0], 1.75, 1.75, 1.4, 10, roof_m);
    }
}

// ============================================================================
//  5. DB Gbs 254 COVERED FREIGHT BOXCAR (2-Axle UIC Boxcar)
// ============================================================================
#[allow(clippy::too_many_arguments)]
pub(super) fn build_wagon_freight(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    gauge_mm: f32,
    livery: u32,
) {
    const NOM: [f32; 3] = [161.0, 46.0, 33.5];
    let hg = half_gauge_local(size, NOM[2], gauge_mm);
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, NOM).with_pitch(pitch_rad);

    let body_m = Mat::matte(livery, 0.72); // RAL 8012 Rotbraun
    let door_m = Mat::matte(0x572617, 0.68);
    let rib_m = Mat::metal(0x3a1b10, 0.60, 0.35);
    let roof_m = Mat::metal(0x475569, 0.66, 0.32);
    let bogie_m = Mat::metal(0x1e293b, 0.58, 0.45);
    let steel_tire = Mat::metal(0xdbe3ec, 0.20, 0.90);
    let vent_m = Mat::metal(0x94a3b8, 0.46, 0.55);
    let shunter_yel = Mat::matte(0xeab308, 0.45);

    // 1) Heavy UIC steel underframe, brake cylinder & 2 gauge-matched wheelsets with leaf springs
    b.rbox([0.0, 9.8, 0.0], [150.0, 3.8, 28.8], bogie_m);
    b.cyl_x([0.0, 7.0, -4.8], 2.2, 2.2, 24.0, 10, bogie_m);
    let guard_z = hg + 2.3;
    for &wx in &[-46.0_f32, 46.0] {
        push_flanged_wheelset(&mut b, wx, 5.2, hg, steel_tire, bogie_m, bogie_m);
        for &side in &[-1.0_f32, 1.0] {
            // Brake shoes
            b.rbox([wx - 4.4, 5.0, side * hg], [1.4, 3.2, 1.6], bogie_m);
            // W-iron axlebox guard + laminated leaf spring (`Blattfeder`)
            b.rbox([wx, 6.2, side * guard_z], [5.2, 5.4, 1.7], bogie_m);
            b.rbox([wx, 9.0, side * (guard_z + side * 0.2)], [16.0, 1.5, 1.5], bogie_m);
        }
    }
    push_buffers_and_coupler(&mut b, 77.0, 10.2, 1.0, bogie_m, bogie_m, steel_tire);
    push_buffers_and_coupler(&mut b, -77.0, 10.2, -1.0, bogie_m, bogie_m, steel_tire);

    // 4 corner shunter footsteps (`Rangiertritte`) + yellow grab handles (`Rangierergriffe`)
    for &ex in &[-71.5_f32, 71.5] {
        for &side in &[-1.0_f32, 1.0] {
            b.rbox([ex, 6.8, side * 14.8], [7.5, 1.0, 2.6], bogie_m);
            b.cyl_y([ex, 11.8, side * 15.6], 0.34, 0.34, 8.0, 8, shunter_yel);
        }
    }

    // 2) Planked Gbs 254 boxcar body, steel stanchions, corrugated sliding doors & alu louvers
    b.rbox([0.0, 24.2, 0.0], [142.0, 25.0, 31.2], body_m);
    for &side in &[-1.0_f32, 1.0] {
        let sz = side * 15.8;
        for &rx in &[-68.0_f32, -48.0, -24.0, 24.0, 48.0, 68.0] {
            b.rbox([rx, 24.2, sz], [1.7, 24.8, 0.9], rib_m);
        }
        // Central corrugated sliding cargo door + upper/lower roller tracks
        b.rbox([0.0, 23.6, sz + side * 0.3], [32.0, 23.0, 1.1], door_m);
        for &cx in &[-10.0_f32, -5.0, 0.0, 5.0, 10.0] {
            b.rbox([cx, 23.6, sz + side * 0.9], [1.1, 21.5, 0.45], rib_m);
        }
        b.rbox([0.0, 35.6, sz + side * 0.55], [70.0, 1.1, 0.85], bogie_m);
        b.rbox([0.0, 11.8, sz + side * 0.55], [70.0, 1.1, 0.85], bogie_m);
        // Twin corner aluminum ventilation hatches (`Lueftungsklappen`)
        for &vx in &[-58.0_f32, 58.0] {
            b.rbox([vx, 30.2, sz + side * 0.2], [11.5, 5.8, 0.75], vent_m);
        }
    }
    // End wall steel stanchions & handbrake wheel at -X end
    for &ex in &[-71.4_f32, 71.4] {
        b.rbox([ex, 24.2, -7.8], [0.95, 24.0, 1.6], rib_m);
        b.rbox([ex, 24.2, 7.8], [0.95, 24.0, 1.6], rib_m);
    }
    b.cyl_x([-72.0, 16.5, -10.5], 2.2, 2.2, 0.6, 12, shunter_yel);

    // 3) Smooth curved overhanging freight roof
    push_curved_roof(&mut b, 0.0, 36.6, 146.0, 16.4, 5.0, 12, roof_m);
}

// ============================================================================
//  ROAD VEHICLES (Sedan, Delivery Truck, City Bus)
// ============================================================================
#[allow(clippy::too_many_arguments)]
pub(super) fn build_car_sedan(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    livery: u32,
    headlights_on: bool,
) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, [52.0, 17.5, 22.0]).with_pitch(pitch_rad);
    let body_m = Mat::metal(livery, 0.25, 0.35);
    let tire_m = Mat::matte(0x111827, 0.88);
    let rim_m = Mat::metal(0xcbd5e1, 0.25, 0.85);
    let glass = Mat::glass(0x7dd3fc);
    let front_l = if headlights_on { Mat::emissive(0xfef9c3, 0.95) } else { Mat::metal(0xe2e8f0, 0.3, 0.5) };
    let tail_l = if headlights_on { Mat::emissive(0xef4444, 0.9) } else { Mat::matte(0x991b1b, 0.5) };

    for &wx in &[-15.5_f32, 15.5] {
        for &wz in &[-10.2_f32, 10.2] {
            b.cyl_z([wx, 4.2, wz], 4.2, 4.2, 3.0, 12, tire_m);
            b.cyl_z([wx, 4.2, wz + wz.signum() * 0.4], 2.5, 2.5, 2.6, 12, rim_m);
        }
    }
    b.rbox([0.0, 7.2, 0.0], [50.0, 7.2, 20.5], body_m);
    b.rbox([-2.0, 13.4, 0.0], [26.0, 6.2, 17.8], body_m);
    b.rbox([10.5, 13.2, 0.0], [3.2, 5.2, 16.4], glass);
    b.rbox([-14.5, 13.2, 0.0], [3.0, 5.0, 16.2], glass);
    b.rbox([-2.0, 13.4, 9.1], [21.0, 4.4, 0.6], glass);
    b.rbox([-2.0, 13.4, -9.1], [21.0, 4.4, 0.6], glass);
    for &sz in &[-7.0_f32, 7.0] {
        b.rbox([25.2, 7.8, sz], [1.0, 2.2, 3.4], front_l);
        b.rbox([-25.2, 7.8, sz], [1.0, 2.2, 3.4], tail_l);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_car_truck(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    livery: u32,
    headlights_on: bool,
) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, [96.0, 34.0, 27.0]).with_pitch(pitch_rad);
    let cab_m = Mat::metal(livery, 0.30, 0.35);
    let box_m = Mat::matte(0xf1f5f9, 0.55);
    let chassis_m = Mat::matte(0x1e293b, 0.72);
    let tire_m = Mat::matte(0x111827, 0.88);
    let rim_m = Mat::metal(0xcbd5e1, 0.25, 0.85);
    let glass = Mat::glass(0x7dd3fc);
    let front_l = if headlights_on { Mat::emissive(0xfef9c3, 0.95) } else { Mat::metal(0xe2e8f0, 0.3, 0.5) };
    let tail_l = if headlights_on { Mat::emissive(0xef4444, 0.9) } else { Mat::matte(0x991b1b, 0.5) };

    b.rbox([0.0, 8.0, 0.0], [92.0, 4.5, 22.0], chassis_m);
    for &wx in &[-32.0_f32, -18.0, 32.0] {
        for &wz in &[-11.8_f32, 11.8] {
            b.cyl_z([wx, 5.5, wz], 5.5, 5.5, 3.6, 12, tire_m);
            b.cyl_z([wx, 5.5, wz + wz.signum() * 0.5], 3.1, 3.1, 3.0, 12, rim_m);
        }
    }
    b.rbox([34.0, 19.5, 0.0], [22.0, 21.0, 24.5], cab_m);
    b.rbox([45.0, 22.5, 0.0], [1.2, 9.0, 21.0], glass);
    b.rbox([-14.0, 21.5, 0.0], [66.0, 24.5, 26.0], box_m);
    for &sz in &[-8.5_f32, 8.5] {
        b.rbox([45.3, 12.0, sz], [1.0, 2.8, 3.8], front_l);
        b.rbox([-47.2, 11.5, sz], [1.0, 2.5, 3.4], tail_l);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_car_bus(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    livery: u32,
    headlights_on: bool,
) {
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, [118.0, 33.0, 27.0]).with_pitch(pitch_rad);
    let body_m = Mat::matte(livery, 0.36);
    let roof_m = Mat::matte(0xe2e8f0, 0.50);
    let tire_m = Mat::matte(0x111827, 0.88);
    let rim_m = Mat::metal(0xcbd5e1, 0.25, 0.85);
    let glass = Mat::glass(0x7dd3fc);
    let front_l = if headlights_on { Mat::emissive(0xfef9c3, 0.95) } else { Mat::metal(0xe2e8f0, 0.3, 0.5) };
    let tail_l = if headlights_on { Mat::emissive(0xef4444, 0.9) } else { Mat::matte(0x991b1b, 0.5) };

    for &wx in &[-36.0_f32, 38.0] {
        for &wz in &[-12.0_f32, 12.0] {
            b.cyl_z([wx, 5.2, wz], 5.2, 5.2, 3.4, 12, tire_m);
            b.cyl_z([wx, 5.2, wz + wz.signum() * 0.45], 2.9, 2.9, 2.9, 12, rim_m);
        }
    }
    b.rbox([0.0, 18.0, 0.0], [116.0, 25.0, 26.0], body_m);
    b.rbox([0.0, 31.2, 0.0], [112.0, 2.4, 24.5], roof_m);
    b.rbox([-6.0, 33.4, 0.0], [32.0, 2.8, 16.0], roof_m);
    b.rbox([58.1, 21.0, 0.0], [1.0, 12.0, 23.0], glass);
    b.rbox([0.0, 21.5, 13.2], [104.0, 10.5, 0.6], glass);
    b.rbox([0.0, 21.5, -13.2], [104.0, 10.5, 0.6], glass);
    for &sz in &[-9.0_f32, 9.0] {
        b.rbox([58.2, 10.5, sz], [1.0, 2.5, 3.4], front_l);
        b.rbox([-58.2, 10.5, sz], [1.0, 2.5, 3.4], tail_l);
    }
}
