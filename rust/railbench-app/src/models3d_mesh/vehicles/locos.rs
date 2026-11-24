use super::{
    dir_lamps, half_gauge_local, push_buffers_and_coupler, push_curved_roof, push_detailed_bogie,
    push_pantograph, push_spoked_steam_wheelset,
};
use crate::models3d_mesh::{Mat, ModelBuilder};
use crate::viewer3d::Vertex3D;

// ============================================================================
//  1. DB BR 01 PACIFIC STEAM LOCOMOTIVE (2'C1' h2 + Einheitstender 2'2' T34)
// ============================================================================
#[allow(clippy::too_many_arguments)]
pub(super) fn build_loco_steam(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    gauge_mm: f32,
    headlights_on: bool,
    driving_backward: bool,
) {
    const NOM: [f32; 3] = [258.0, 50.0, 35.0];
    let hg = half_gauge_local(size, NOM[2], gauge_mm);
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, NOM).with_pitch(pitch_rad);

    let boiler_m = Mat::metal(0x14181e, 0.38, 0.52);
    let smokebox_m = Mat::metal(0x1f252e, 0.44, 0.48);
    let chassis_m = Mat::matte(0x991b1b, 0.46);
    let wheel_red = Mat::metal(0xb91c1c, 0.35, 0.42);
    let backing_dark = Mat::matte(0x0b0e12, 0.85);
    let steel_tire = Mat::metal(0xdbe3ec, 0.20, 0.90);
    let rod_steel = Mat::metal(0xcbd5e1, 0.22, 0.88);
    let brass_m = Mat::metal(0xd4a017, 0.26, 0.88);
    let cab_roof = Mat::matte(0x272b33, 0.62);
    let coal_m = Mat::matte(0x0a0d11, 0.96);
    let plate_m = Mat::matte(0x0f172a, 0.50);
    let glass = Mat::glass(0x93c5fd);
    let (front_l, rear_l) = dir_lamps(headlights_on, driving_backward);

    // 1) Bar frame (`Barrenrahmen`), ashpan, rail guards & UIC buffer beams
    b.rbox([34.0, 12.2, 0.0], [174.0, 6.8, (hg * 1.75).max(14.5)], chassis_m);
    b.rbox([-14.0, 8.2, 0.0], [20.0, 5.2, 13.0], smokebox_m); // firebox ashpan
    push_buffers_and_coupler(&mut b, 125.0, 10.6, 1.0, chassis_m, boiler_m, steel_tire);
    push_buffers_and_coupler(&mut b, -125.0, 10.6, -1.0, chassis_m, boiler_m, steel_tire);
    // Front sloped apron + rail guards (`Schienenraeumer`)
    b.obox([121.5, 14.5, 0.0], [9.5, 2.2, 29.0], [0.0, 0.0, -0.45], boiler_m);
    for &side in &[-1.0_f32, 1.0] {
        b.rbox([123.0, 4.5, side * hg], [2.4, 6.2, 1.2], chassis_m);
        b.rbox([120.0, 12.0, side * 12.8], [4.5, 1.2, 4.5], boiler_m); // front steps
    }

    // 2) Running boards (`Umlaufbleche`) with red valence & Knorr air compressor
    for &side in &[-1.0_f32, 1.0] {
        let uz = side * (hg + 5.8).max(14.0);
        b.rbox([38.0, 20.6, uz], [148.0, 1.3, 4.6], boiler_m);
        b.rbox([38.0, 19.7, uz + side * 2.0], [148.0, 1.1, 0.65], chassis_m);
    }
    // Knorr compound air pump on right running board + feedwater pump on left
    let pump_z = (hg + 5.6).max(13.8);
    b.cyl_y([26.0, 24.8, pump_z], 2.2, 2.2, 7.6, 12, smokebox_m);
    b.cyl_y([26.0, 25.0, -pump_z], 1.8, 1.8, 6.6, 12, brass_m);

    // 3) Authentic 4-6-2 wheel arrangement at exact `±hg`
    // Leading 2-axle bogie (`Vorlaufdrehgestell`, spoked red wheels r = 5.2)
    b.rbox([98.5, 6.5, 0.0], [34.0, 3.4, (hg * 1.55).max(13.0)], chassis_m);
    for &wx in &[87.0_f32, 110.0] {
        push_spoked_steam_wheelset(&mut b, wx, 5.2, hg, wheel_red, backing_dark, steel_tire, rod_steel, false);
    }
    // Three large 2,000 mm prototype coupled driving wheels (`r = 10.2`) + brake shoes
    for &wx in &[16.0_f32, 41.0, 66.0] {
        push_spoked_steam_wheelset(&mut b, wx, 10.2, hg, wheel_red, backing_dark, steel_tire, rod_steel, true);
        for &side in &[-1.0_f32, 1.0] {
            b.obox([wx - 9.8, 9.8, side * hg], [1.8, 5.5, 1.7], [0.0, 0.0, 0.2], chassis_m);
        }
    }
    // Rear trailing wheel (`Nachlaufachse` r = 6.2)
    push_spoked_steam_wheelset(&mut b, -14.0, 6.2, hg, wheel_red, backing_dark, steel_tire, rod_steel, false);

    // 4) Detailed outside cylinders & Walschaerts (`Heusinger`) valve gear
    let cyl_z = hg + 5.1;
    let rod_z = hg + 2.55;
    for &side in &[-1.0_f32, 1.0] {
        let cz = side * cyl_z;
        let rz = side * rod_z;
        // Main steam cylinder + upper piston-valve chest + silver front/rear covers
        b.cyl_x([95.0, 10.4, cz], 4.8, 4.8, 16.5, 16, smokebox_m);
        b.cyl_x([95.0, 15.8, cz - side * 0.8], 2.5, 2.5, 14.5, 12, smokebox_m);
        b.cyl_x([103.5, 10.4, cz], 4.2, 4.2, 0.8, 14, rod_steel);
        b.cyl_x([86.5, 10.4, cz], 4.2, 4.2, 0.8, 14, rod_steel);
        // Twin horizontal slidebars (`Gleitbahnen`) & crosshead block (`Kreuzkopf`)
        b.rbox([77.0, 11.4, rz], [18.0, 0.75, 0.9], rod_steel);
        b.rbox([77.0, 9.0, rz], [18.0, 0.75, 0.9], rod_steel);
        b.rbox([77.5, 10.2, rz], [4.2, 3.2, 1.3], smokebox_m);
        b.cyl_x([83.0, 10.2, rz], 0.6, 0.6, 8.0, 8, rod_steel); // piston rod
        // Main 3-axle coupling rod (`Kuppelstange`) with red-painted flute inset
        b.rbox([41.0, 6.0, rz], [53.0, 1.6, 0.95], rod_steel);
        b.rbox([41.0, 6.0, rz + side * 0.22], [49.0, 0.7, 0.55], wheel_red);
        // Connecting rod (`Treibstange`) from crosshead to center driver crankpin + valve gear
        b.cyl_between([76.5, 10.2, rz + side * 0.5], [41.0, 5.7, rz + side * 0.6], 0.72, 0.88, 8, rod_steel);
        b.cyl_between([85.0, 15.6, rz], [62.0, 14.2, rz], 0.45, 0.45, 8, rod_steel);
        b.rbox([62.0, 14.0, rz], [2.0, 5.2, 1.0], smokebox_m); // expansion link bracket
        b.cyl_between([41.0, 5.7, rz + side * 0.7], [62.0, 12.2, rz + side * 0.3], 0.38, 0.38, 6, rod_steel);
    }

    // 5) Stepped boiler (`Langkessel`), brass bands, recessed surface preheater & fittings
    b.cyl_x([41.0, 31.0, 0.0], 11.8, 11.8, 128.0, 22, boiler_m);
    b.cyl_x([99.0, 31.0, 0.0], 12.1, 12.1, 30.0, 22, smokebox_m);
    b.cyl_x([114.6, 31.0, 0.0], 9.2, 11.9, 3.8, 20, smokebox_m); // domed smokebox door
    b.cyl_x([117.0, 31.0, 0.0], 2.0, 2.0, 1.1, 12, brass_m); // central locking wheel
    b.rbox([116.8, 26.8, 0.0], [0.8, 2.4, 8.2], plate_m); // front DB 01 number plate
    for &bx in &[-10.0_f32, 16.0, 42.0, 68.0] {
        b.cyl_x([bx, 31.0, 0.0], 12.05, 12.05, 1.1, 22, brass_m);
    }
    // Recessed horizontal surface feedwater heater (`Oberflaechenvorwaermer`) ahead of chimney
    b.cyl_z([112.0, 41.2, 0.0], 2.5, 2.5, 11.8, 12, smokebox_m);
    // Flared chimney (`Schornstein`), tall steam dome & sand dome with sand pipes
    b.cyl_y([101.0, 44.8, 0.0], 3.8, 3.1, 9.8, 16, smokebox_m);
    b.cyl_y([101.0, 49.4, 0.0], 4.5, 3.8, 1.4, 16, smokebox_m);
    b.cyl_y([54.0, 43.8, 0.0], 5.4, 6.2, 6.8, 14, boiler_m);
    b.cyl_y([26.0, 43.2, 0.0], 4.9, 5.4, 5.8, 14, boiler_m);
    for &side in &[-1.0_f32, 1.0] {
        b.cyl_between([26.0, 40.5, side * 5.2], [41.0, 21.0, side * (hg + 1.0)], 0.35, 0.35, 6, boiler_m);
        // Boiler-side brass handrails & stanchions
        b.cyl_x([41.0, 35.0, side * 12.6], 0.36, 0.36, 122.0, 8, brass_m);
    }
    for &sz in &[-2.1_f32, 2.1] {
        b.cyl_y([4.0, 43.8, sz], 0.85, 1.0, 4.2, 10, brass_m);
    }
    // Iconic Witte smoke deflectors (`Windleitbleche`) mounted on stanchions
    for &side in &[-1.0_f32, 1.0] {
        let wz = side * (hg + 6.4).max(14.6);
        b.rbox([100.0, 34.4, wz], [27.0, 15.0, 0.85], boiler_m);
        b.rbox([100.0, 34.4, wz - side * 1.4], [2.2, 1.2, 2.8], smokebox_m);
    }

    // 6) Einheitsfuehrerhaus cab with curved roof, raised ventilator hatch, rain awnings & entry ladders
    b.rbox([-31.0, 30.8, 0.0], [34.0, 25.5, 32.0], boiler_m);
    push_curved_roof(&mut b, -31.0, 43.5, 37.5, 16.8, 4.8, 12, cab_roof);
    b.rbox([-31.0, 48.6, 0.0], [12.0, 1.5, 13.0], cab_roof);
    for &side in &[-1.0_f32, 1.0] {
        let wz = side * 16.2;
        b.rbox([-36.5, 35.0, wz], [8.8, 9.2, 0.7], glass);
        b.rbox([-25.0, 35.0, wz], [8.8, 9.2, 0.7], glass);
        b.rbox([-30.5, 25.5, wz], [8.2, 2.6, 0.6], chassis_m); // BR 01 red cab plate
        b.cyl_y([-47.0, 23.0, side * 15.6], 0.34, 0.34, 15.0, 8, rod_steel);
    }

    // 7) Riveted Einheitstender `2'2' T34` with 2 gauge-matched bogies, heaped coal & rear ladder
    b.rbox([-86.0, 11.2, 0.0], [64.0, 4.6, 28.0], chassis_m);
    push_detailed_bogie(&mut b, -68.0, &[-7.8, 7.8], 4.8, hg, boiler_m, wheel_red, steel_tire, chassis_m, false);
    push_detailed_bogie(&mut b, -104.0, &[-7.8, 7.8], 4.8, hg, boiler_m, wheel_red, steel_tire, chassis_m, false);
    b.rbox([-86.0, 25.2, 0.0], [64.0, 23.4, 30.6], boiler_m);
    b.rbox([-86.0, 37.4, 0.0], [65.4, 1.9, 31.8], smokebox_m);
    // Multi-tier sculpted black coal mound + rear water fill hatches & rear climbing ladder
    b.rbox([-79.0, 38.8, 0.0], [42.0, 4.4, 22.8], coal_m);
    b.rbox([-78.0, 41.8, 0.0], [28.0, 3.2, 16.0], coal_m);
    b.rbox([-77.0, 44.0, 0.0], [16.0, 2.2, 10.0], coal_m);
    b.cyl_y([-110.0, 38.8, -6.0], 2.8, 2.8, 1.4, 10, smokebox_m);
    b.cyl_y([-110.0, 38.8, 6.0], 2.8, 2.8, 1.4, 10, smokebox_m);
    for &lz in &[-4.2_f32, -1.6] {
        b.cyl_y([-118.4, 24.5, lz], 0.35, 0.35, 25.0, 8, rod_steel);
    }
    b.rbox([-51.5, 21.0, 0.0], [8.5, 7.0, 21.0], smokebox_m); // loco-tender gangway bridge

    // 8) Black-housed DB Reflexglas 3-point headlights (+X) & tender rear lamps (-X)
    for &sz in &[-10.05_f32, 10.05] {
        b.cyl_x([124.2, 15.8, sz], 2.35, 2.35, 2.8, 12, smokebox_m);
        b.cyl_x([125.5, 15.8, sz], 1.75, 1.75, 0.8, 12, front_l);
        b.cyl_x([-124.0, 15.8, sz], 2.2, 2.2, 2.6, 12, smokebox_m);
        b.cyl_x([-125.2, 15.8, sz], 1.6, 1.6, 0.8, 12, rear_l);
    }
    b.cyl_x([115.6, 39.2, 0.0], 2.1, 2.1, 2.4, 12, smokebox_m);
    b.cyl_x([116.7, 39.2, 0.0], 1.55, 1.55, 0.8, 12, front_l);
    b.cyl_x([-118.6, 36.0, 0.0], 2.0, 2.0, 2.2, 12, smokebox_m);
    b.cyl_x([-119.6, 36.0, 0.0], 1.45, 1.45, 0.8, 12, rear_l);
}

// ============================================================================
//  2. DB BR 103 TEE EXPRESS ELECTRIC LOCOMOTIVE (Co'Co')
// ============================================================================
#[allow(clippy::too_many_arguments)]
pub(super) fn build_loco_electric(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    gauge_mm: f32,
    livery: u32,
    headlights_on: bool,
    driving_backward: bool,
) {
    const NOM: [f32; 3] = [224.0, 50.0, 35.0];
    let hg = half_gauge_local(size, NOM[2], gauge_mm);
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, NOM).with_pitch(pitch_rad);

    let red_m = Mat::matte(livery, 0.32); // RAL 3004 Purpurrot
    let cream_m = Mat::matte(0xfef3c7, 0.34); // RAL 1001 Elfenbein
    let skirt_m = Mat::metal(0x1e293b, 0.48, 0.42);
    let roof_m = Mat::metal(0xa3b1c2, 0.36, 0.56); // Weissaluminium
    let bogie_m = Mat::metal(0x1f2937, 0.54, 0.48);
    let steel_tire = Mat::metal(0xdbe3ec, 0.20, 0.90);
    let chrome_m = Mat::metal(0xe2e8f0, 0.18, 0.92);
    let panto_red = Mat::metal(0xdc2626, 0.36, 0.65);
    let insul_m = Mat::matte(0x9a3412, 0.42);
    let grille_m = Mat::metal(0x64748b, 0.46, 0.62);
    let gasket_m = Mat::matte(0x111827, 0.78);
    let glass = Mat::glass(0x38bdf8);
    let (front_l, tail_l) = dir_lamps(headlights_on, driving_backward);

    // 1) Underframe skirt, central transformer tank & 2 gauge-matched 3-axle Henschel Co'Co' bogies
    b.rbox([0.0, 10.6, 0.0], [186.0, 4.8, 30.2], skirt_m);
    b.rbox([0.0, 7.6, 0.0], [44.0, 5.8, 24.5], bogie_m);
    push_detailed_bogie(&mut b, -54.0, &[-15.5, 0.0, 15.5], 5.6, hg, bogie_m, skirt_m, steel_tire, grille_m, true);
    push_detailed_bogie(&mut b, 54.0, &[-15.5, 0.0, 15.5], 5.6, hg, bogie_m, skirt_m, steel_tire, grille_m, true);
    push_buffers_and_coupler(&mut b, 108.0, 10.8, 1.0, skirt_m, bogie_m, steel_tire);
    push_buffers_and_coupler(&mut b, -108.0, 10.8, -1.0, skirt_m, bogie_m, steel_tire);

    // 2) Main carbody: Purpurrot lower band, Elfenbein upper wall, chrome waist belt & double silver louver bands
    b.rbox([0.0, 16.8, 0.0], [160.0, 9.6, 33.4], red_m);
    b.rbox([0.0, 27.6, 0.0], [160.0, 12.4, 33.4], cream_m);
    b.rbox([0.0, 21.7, 0.0], [162.0, 1.05, 33.9], chrome_m);
    for &sz in &[-16.85_f32, 16.85] {
        // Twin horizontal rows of fine silver louver ventilation grilles (signature BR 103 look!)
        for k in 0..8 {
            let gx = -56.0 + k as f32 * 16.0;
            b.rbox([gx, 30.2, sz], [13.2, 2.5, 0.75], grille_m);
            b.rbox([gx, 26.8, sz], [13.2, 2.5, 0.75], grille_m);
        }
    }

    // 3) Sculpted double-curved aerodynamic BR 103 noses (`Ei-Form`) at both ends (+X & -X)
    for &dir in &[-1.0_f32, 1.0] {
        // Tapered red nose belly & lower apron with front snowplow
        b.rbox([dir * 86.0, 16.8, 0.0], [13.0, 9.4, 31.8], red_m);
        b.obox([dir * 96.5, 16.2, 0.0], [12.0, 8.8, 28.6], [0.0, 0.0, -dir * 0.10], red_m);
        b.obox([dir * 103.5, 15.6, 0.0], [6.8, 7.2, 23.4], [0.0, 0.0, -dir * 0.14], red_m);
        // Dark aerodynamic lower valance skirt & snowplow blade
        b.rbox([dir * 101.0, 9.6, 0.0], [12.0, 4.2, 26.0], skirt_m);
        b.obox([dir * 106.5, 6.4, 0.0], [4.5, 3.6, 22.0], [0.0, 0.0, -dir * 0.35], skirt_m);
        // Winged chrome DB front badge on Purpurrot nose center
        b.rbox([dir * 107.1, 18.8, 0.0], [0.9, 2.8, 6.2], chrome_m);
        b.rbox([dir * 107.3, 18.8, 0.0], [1.0, 1.7, 3.2], red_m);

        // Swept Elfenbein upper cab nose curving inward cleanly to curved brow
        b.obox([dir * 86.0, 27.4, 0.0], [13.0, 11.8, 31.6], [0.0, 0.0, -dir * 0.10], cream_m);
        b.obox([dir * 95.8, 27.2, 0.0], [11.5, 10.6, 27.8], [0.0, 0.0, -dir * 0.24], cream_m);
        b.obox([dir * 102.2, 25.0, 0.0], [5.8, 6.4, 23.4], [0.0, 0.0, -dir * 0.32], cream_m);
        // Rounded tapered roof dome over cab nose
        b.obox([dir * 87.5, 35.6, 0.0], [16.0, 5.8, 27.4], [0.0, 0.0, -dir * 0.16], roof_m);
        b.obox([dir * 96.5, 34.0, 0.0], [9.0, 4.2, 21.8], [0.0, 0.0, -dir * 0.28], roof_m);

        // Twin wrap-around panoramic windshields with dark gaskets & windshield wipers
        for &sz in &[-5.6_f32, 5.6] {
            b.obox([dir * 98.2, 28.6, sz], [2.4, 7.6, 10.2], [0.0, sz.signum() * dir * 0.14, -dir * 0.28], gasket_m);
            b.obox([dir * 98.6, 28.6, sz], [2.2, 6.4, 8.8], [0.0, sz.signum() * dir * 0.14, -dir * 0.28], glass);
            b.obox([dir * 99.6, 26.8, sz], [0.5, 3.4, 0.45], [0.35, 0.0, -dir * 0.28], gasket_m);
        }
        // Side cab windows, recessed cab doors, footsteps & chrome grab rails
        for &side in &[-1.0_f32, 1.0] {
            let sz = side * 16.4;
            b.rbox([dir * 83.0, 28.2, sz], [11.2, 7.6, 0.7], gasket_m);
            b.rbox([dir * 83.0, 28.2, sz + side * 0.12], [9.8, 6.4, 0.6], glass);
            b.rbox([dir * 71.5, 22.8, sz - side * 0.2], [8.8, 19.0, 0.5], red_m);
            b.cyl_y([dir * 66.8, 22.5, sz + side * 0.35], 0.32, 0.32, 14.0, 8, chrome_m);
            b.rbox([dir * 71.5, 11.2, sz - side * 0.4], [7.2, 1.2, 1.8], grille_m);
        }
    }

    // 4) Smooth arched Weissaluminium roof + HV busbar, circuit breaker & twin crimson single-arm pantographs
    push_curved_roof(&mut b, 0.0, 33.8, 162.0, 16.2, 5.8, 14, roof_m);
    b.cyl_x([0.0, 41.2, 5.2], 0.40, 0.40, 96.0, 8, panto_red);
    for &sx in &[-32.0_f32, 0.0, 32.0] {
        b.cyl_y([sx, 40.0, 5.2], 0.85, 1.0, 2.2, 8, insul_m);
    }
    b.rbox([0.0, 41.4, -3.5], [14.0, 2.8, 6.5], grille_m); // air-blast main circuit breaker
    for &fx in &[-20.0_f32, 0.0, 20.0] {
        b.cyl_y([fx, 39.8, 0.0], 5.6, 5.6, 0.95, 14, grille_m);
    }
    push_pantograph(&mut b, -48.0, 39.4, 1.0, bogie_m, panto_red, chrome_m, insul_m);
    push_pantograph(&mut b, 48.0, 39.4, -1.0, bogie_m, panto_red, chrome_m, insul_m);

    // 5) Twin oval-bezel double headlights + upper 3rd Spitzensignal at both noses
    for &sz in &[-8.8_f32, 8.8] {
        b.rbox([105.6, 16.5, sz], [1.8, 3.0, 4.4], chrome_m);
        b.cyl_x([106.4, 16.5, sz], 1.45, 1.45, 0.9, 12, front_l);
        b.rbox([-105.6, 16.5, sz], [1.8, 3.0, 4.4], chrome_m);
        b.cyl_x([-106.4, 16.5, sz], 1.45, 1.45, 0.9, 12, tail_l);
    }
    b.cyl_x([95.2, 34.4, 0.0], 1.55, 1.55, 1.6, 12, front_l);
    b.cyl_x([-95.2, 34.4, 0.0], 1.55, 1.55, 1.6, 12, tail_l);
}

// ============================================================================
//  3. DB BR 218 DIESEL-HYDRAULIC LOCOMOTIVE (B'B' V160 Knickfront)
// ============================================================================
#[allow(clippy::too_many_arguments)]
pub(super) fn build_loco_diesel(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    rot_rad: f32,
    pitch_rad: f32,
    size: [f32; 3],
    gauge_mm: f32,
    livery: u32,
    headlights_on: bool,
    driving_backward: bool,
) {
    const NOM: [f32; 3] = [189.0, 48.0, 34.5];
    let hg = half_gauge_local(size, NOM[2], gauge_mm);
    let mut b = ModelBuilder::new(verts, idxs, origin, rot_rad, size, NOM).with_pitch(pitch_rad);

    let body_m = Mat::matte(livery, 0.36);
    let bib_m = Mat::matte(0xf1f5f9, 0.42);
    let roof_m = Mat::metal(0x475569, 0.50, 0.40);
    let bogie_m = Mat::metal(0x1e293b, 0.54, 0.46);
    let steel_tire = Mat::metal(0xdbe3ec, 0.20, 0.90);
    let grille_m = Mat::metal(0x334155, 0.48, 0.58);
    let chrome_m = Mat::metal(0xe2e8f0, 0.22, 0.88);
    let gasket_m = Mat::matte(0x111827, 0.78);
    let glass = Mat::glass(0x7dd3fc);
    let (front_l, tail_l) = dir_lamps(headlights_on, driving_backward);

    // 1) Heavy frame, central fuel tank, twin air reservoirs & 2 gauge-matched B'B' bogies
    b.rbox([0.0, 10.6, 0.0], [174.0, 4.8, 30.2], bogie_m);
    b.rbox([0.0, 7.2, 0.0], [48.0, 6.2, 23.0], bogie_m);
    for &sz in &[-10.8_f32, 10.8] {
        b.cyl_x([0.0, 7.6, sz], 2.2, 2.2, 32.0, 12, grille_m);
    }
    push_detailed_bogie(&mut b, -48.0, &[-11.8, 11.8], 5.4, hg, bogie_m, bogie_m, steel_tire, grille_m, true);
    push_detailed_bogie(&mut b, 48.0, &[-11.8, 11.8], 5.4, hg, bogie_m, bogie_m, steel_tire, grille_m, true);
    push_buffers_and_coupler(&mut b, 89.5, 10.6, 1.0, bogie_m, bogie_m, steel_tire);
    push_buffers_and_coupler(&mut b, -89.5, 10.6, -1.0, bogie_m, bogie_m, steel_tire);

    // 2) Main carbody with DB solebar white pinstripe & sloped roof cant
    b.rbox([0.0, 24.4, 0.0], [154.0, 23.2, 32.4], body_m);
    b.rbox([0.0, 13.4, 0.0], [155.0, 1.4, 32.8], bib_m);

    // Side engine-room cooling louver grilles & porthole windows
    for &side in &[-1.0_f32, 1.0] {
        let sz = side * 16.3;
        for &gx in &[-32.0_f32, 0.0, 32.0] {
            b.rbox([gx, 28.4, sz], [17.0, 7.6, 0.7], grille_m);
        }
        for &px in &[-46.0_f32, -16.0, 16.0, 46.0] {
            b.rbox([px, 28.6, sz], [5.2, 4.6, 0.65], gasket_m);
            b.rbox([px, 28.6, sz + side * 0.12], [4.2, 3.6, 0.55], glass);
        }
    }

    // 3) Authentic V160 / BR 218 `Knickfront` cab ends (+X & -X)
    for &dir in &[-1.0_f32, 1.0] {
        // Lower inward-tapered apron + horizontal crease (`Knick`) + white safety bib (`Latz`)
        b.obox([dir * 82.0, 17.6, 0.0], [11.0, 10.6, 31.2], [0.0, 0.0, dir * 0.10], body_m);
        b.rbox([dir * 86.2, 17.2, 0.0], [3.6, 7.2, 22.8], bib_m);
        b.cyl_z([dir * 89.2, 14.2, 0.0], 0.42, 0.42, 24.0, 8, chrome_m); // front grab rail
        b.obox([dir * 88.2, 7.2, 0.0], [4.2, 3.8, 24.0], [0.0, 0.0, -dir * 0.30], bogie_m); // snowplow
        // Upper backward-sloped windshield wall
        b.obox([dir * 81.5, 29.2, 0.0], [11.5, 13.2, 31.2], [0.0, 0.0, -dir * 0.22], body_m);
        for &sz in &[-6.6_f32, 6.6] {
            b.obox([dir * 86.2, 29.4, sz], [2.2, 7.4, 10.6], [0.0, 0.0, -dir * 0.22], gasket_m);
            b.obox([dir * 86.6, 29.4, sz], [2.0, 6.2, 9.2], [0.0, 0.0, -dir * 0.22], glass);
            b.obox([dir * 87.5, 27.6, sz], [0.45, 3.2, 0.4], [0.0, 0.0, -dir * 0.22], gasket_m);
        }
        // Side cab windows, recessed cab doors & vertical chrome grab rails
        for &side in &[-1.0_f32, 1.0] {
            let sz = side * 16.2;
            b.rbox([dir * 71.0, 29.2, sz], [10.4, 7.2, 0.7], gasket_m);
            b.rbox([dir * 71.0, 29.2, sz + side * 0.12], [9.2, 6.0, 0.6], glass);
            b.rbox([dir * 59.5, 23.6, sz - side * 0.2], [7.8, 19.5, 0.5], body_m);
            b.cyl_y([dir * 55.2, 22.8, sz + side * 0.35], 0.32, 0.32, 14.0, 8, chrome_m);
        }
    }

    // 4) Smooth arched Umbragrau roof + twin angled BR 218 exhaust deflectors (`Abgashutzen`) + 3 radiator fans
    push_curved_roof(&mut b, 0.0, 35.8, 162.0, 15.8, 5.2, 12, roof_m);
    for &ex in &[-16.0_f32, 16.0] {
        b.obox([ex, 42.2, -2.5], [7.2, 3.2, 5.0], [0.22, 0.0, 0.16], bogie_m);
        b.obox([ex, 42.2, 2.5], [7.2, 3.2, 5.0], [-0.22, 0.0, 0.16], bogie_m);
    }
    for &fx in &[-38.0_f32, 0.0, 38.0] {
        b.cyl_y([fx, 41.2, 0.0], 6.4, 6.4, 1.0, 14, grille_m);
    }

    // 5) 3-point silver-bezel DB headlights (+X) & taillights (-X)
    for &sz in &[-9.2_f32, 9.2] {
        b.cyl_x([87.6, 17.0, sz], 2.0, 2.0, 1.6, 12, chrome_m);
        b.cyl_x([88.4, 17.0, sz], 1.5, 1.5, 0.9, 12, front_l);
        b.cyl_x([-87.6, 17.0, sz], 2.0, 2.0, 1.6, 12, chrome_m);
        b.cyl_x([-88.4, 17.0, sz], 1.5, 1.5, 0.9, 12, tail_l);
    }
    b.cyl_x([84.4, 35.6, 0.0], 1.55, 1.55, 1.6, 12, front_l);
    b.cyl_x([-84.4, 35.6, 0.0], 1.55, 1.55, 1.6, 12, tail_l);
}
