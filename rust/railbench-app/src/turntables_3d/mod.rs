use railbench_core::geometry::rotate;
use railbench_core::types::{CatalogEntry, Layout, TrackPiece, Vec2};

use crate::models3d_mesh::{Mat, ModelBuilder};
use crate::viewer3d::Vertex3D;

struct SharedPalette {
    ballast: Mat,
    tie: Mat,
    rail: Mat,
    concrete: Mat,
    concrete_dark: Mat,
    pit_floor: Mat,
    pit_wall: Mat,
    sump: Mat,
    grate: Mat,
    steel_girder: Mat,
    steel_dark: Mat,
    steel_deck: Mat,
    wheel_steel: Mat,
    bronze: Mat,
    cab_wall: Mat,
    cab_trim: Mat,
    cab_roof: Mat,
    win_pane: Mat,
    signal_box: Mat,
    lamp_warm: Mat,
    signal_red: Mat,
    hazard_yellow: Mat,
    wire_copper: Mat,
    wire_dark: Mat,
    cat_green: Mat,
    insulator: Mat,
}

impl SharedPalette {
    fn new(is_tunnel: bool) -> Self {
        let ballast_col = if is_tunnel { [0.24, 0.26, 0.30] } else { [0.42, 0.44, 0.48] };
        Self {
            ballast: Mat { col: ballast_col, rough: 0.90, metal: 0.02, emissive: 0.0 },
            tie: Mat { col: [0.24, 0.17, 0.12], rough: 0.88, metal: 0.02, emissive: 0.0 },
            rail: Mat { col: [0.84, 0.87, 0.90], rough: 0.24, metal: 0.84, emissive: 0.0 },
            concrete: Mat::matte(0x64748b, 0.88),
            concrete_dark: Mat::matte(0x475569, 0.90),
            pit_floor: Mat::matte(0x334155, 0.92),
            pit_wall: Mat::matte(0x526071, 0.88),
            sump: Mat::matte(0x0f172a, 0.94),
            grate: Mat::metal(0x334155, 0.48, 0.72),
            steel_girder: Mat::metal(0x364a46, 0.46, 0.68),
            steel_dark: Mat::metal(0x1f2937, 0.42, 0.76),
            steel_deck: Mat::metal(0x2d3748, 0.56, 0.58),
            wheel_steel: Mat::metal(0x94a3b8, 0.28, 0.88),
            bronze: Mat::metal(0xb45309, 0.34, 0.82),
            cab_wall: Mat::matte(0x2f453f, 0.72),
            cab_trim: Mat::metal(0x1e293b, 0.48, 0.62),
            cab_roof: Mat::metal(0x3f4c59, 0.52, 0.48),
            win_pane: Mat::warm_window(0xfde68a, 0.88),
            signal_box: Mat::matte(0x111827, 0.65),
            lamp_warm: Mat::emissive(0xfef9c3, 1.05),
            signal_red: Mat::emissive(0xef4444, 0.92),
            hazard_yellow: Mat::matte(0xeab308, 0.58),
            wire_copper: Mat::metal(0xb45309, 0.30, 0.85),
            wire_dark: Mat::metal(0x334155, 0.45, 0.65),
            cat_green: Mat::metal(0x285943, 0.48, 0.45),
            insulator: Mat::matte(0x7c2d12, 0.55),
        }
    }
}

fn build_safety_handrail_x(
    b: &mut ModelBuilder<'_>,
    x0: f32,
    x1: f32,
    y_base: f32,
    z: f32,
    gap: Option<(f32, f32)>,
    scale_f: f32,
    steel: Mat,
) {
    let post_h = 12.2 * scale_f;
    let top_y = y_base + post_h;
    let mid_y = y_base + post_h * 0.52;
    let post_w = 1.0 * scale_f;
    let rail_t = 0.85 * scale_f;

    let emit_span = |b: &mut ModelBuilder<'_>, sa: f32, sb: f32| {
        if sb - sa < 4.0 * scale_f {
            return;
        }
        b.strut_between([sa, top_y, z], [sb, top_y, z], rail_t, rail_t, steel);
        b.strut_between([sa, mid_y, z], [sb, mid_y, z], rail_t * 0.8, rail_t * 0.8, steel);
        let steps = (((sb - sa) / (24.0 * scale_f)).round() as usize).max(1);
        for i in 0..=steps {
            let px = sa + (sb - sa) * (i as f32 / steps as f32);
            b.rbox([px, y_base + post_h * 0.5, z], [post_w, post_h, post_w], steel);
        }
    };

    match gap {
        Some((ga, gb)) => {
            emit_span(b, x0, ga.min(x1));
            emit_span(b, gb.max(x0), x1);
        }
        None => emit_span(b, x0, x1),
    }
}

fn build_gleissperrsignal(
    b: &mut ModelBuilder<'_>,
    x: f32,
    y_base: f32,
    z: f32,
    scale_f: f32,
    p: &SharedPalette,
) {
    let post_h = 5.6 * scale_f;
    b.rbox([x, y_base + post_h * 0.5, z], [1.6 * scale_f, post_h, 1.6 * scale_f], p.steel_dark);
    let box_y = y_base + post_h + 2.8 * scale_f;
    b.rbox([x, box_y, z], [5.2 * scale_f, 5.6 * scale_f, 4.2 * scale_f], p.signal_box);
    b.rbox([x, box_y + 3.2 * scale_f, z], [5.8 * scale_f, 0.8 * scale_f, 4.8 * scale_f], p.steel_dark);
    // Illuminated white shunting signal lenses (+/-X faces) + red top stop indicator
    for sx in [-1.0_f32, 1.0] {
        b.rbox(
            [x + sx * 2.65 * scale_f, box_y - 0.5 * scale_f, z],
            [0.6 * scale_f, 2.2 * scale_f, 2.2 * scale_f],
            p.lamp_warm,
        );
        b.rbox(
            [x + sx * 2.65 * scale_f, box_y + 1.4 * scale_f, z],
            [0.6 * scale_f, 1.1 * scale_f, 2.4 * scale_f],
            p.signal_red,
        );
    }
}

fn build_operator_cabin(
    b: &mut ModelBuilder<'_>,
    cx: f32,
    cz: f32,
    cab_w: f32,
    cab_d: f32,
    girder_z: f32,
    y_deck: f32,
    y_girder_bot: f32,
    scale_f: f32,
    p: &SharedPalette,
) {
    let cab_h = 28.5 * scale_f;
    let waist_h = 11.2 * scale_f;
    let fascia_h = 3.2 * scale_f;
    let win_y0 = y_deck + waist_h;
    let win_y1 = y_deck + cab_h - fascia_h;
    let win_h = win_y1 - win_y0;
    let win_cy = 0.5 * (win_y0 + win_y1);
    let wall_t = 1.4 * scale_f;

    // 1. Steel cantilever platform + 3 diagonal knee-braces anchored down to the +Z main girder
    let inner_z = cz - cab_d * 0.5;
    let outer_z = cz + cab_d * 0.5;
    let span_z = outer_z - girder_z + 1.0 * scale_f;
    b.rbox(
        [cx, y_deck - 0.65 * scale_f, 0.5 * (girder_z + outer_z)],
        [cab_w + 3.0 * scale_f, 1.3 * scale_f, span_z],
        p.steel_dark,
    );
    for &kx in &[cx - cab_w * 0.40, cx, cx + cab_w * 0.40] {
        b.rbox(
            [kx, y_deck - 1.4 * scale_f, 0.5 * (girder_z + outer_z)],
            [1.6 * scale_f, 1.8 * scale_f, span_z],
            p.steel_dark,
        );
        b.strut_between(
            [kx, y_deck - 1.5 * scale_f, outer_z - 1.5 * scale_f],
            [kx, y_girder_bot + 1.0 * scale_f, girder_z + 0.5 * scale_f],
            1.7 * scale_f,
            1.7 * scale_f,
            p.steel_dark,
        );
    }

    // 2. Lower wainscoting steel cabin walls & corner pilasters
    b.rbox([cx, y_deck + waist_h * 0.5, cz], [cab_w, waist_h, cab_d], p.cab_wall);
    b.rbox([cx, y_deck + cab_h - fascia_h * 0.5, cz], [cab_w + 0.8 * scale_f, fascia_h, cab_d + 0.8 * scale_f], p.cab_wall);
    b.rbox([cx, y_deck + waist_h, cz], [cab_w + 1.2 * scale_f, 1.1 * scale_f, cab_d + 1.2 * scale_f], p.cab_trim);

    for sx in [-1.0_f32, 1.0] {
        for sz in [-1.0_f32, 1.0] {
            b.rbox(
                [cx + sx * (cab_w * 0.5 - 0.8 * scale_f), y_deck + cab_h * 0.5, cz + sz * (cab_d * 0.5 - 0.8 * scale_f)],
                [2.2 * scale_f, cab_h, 2.2 * scale_f],
                p.cab_trim,
            );
        }
    }

    // 3. Recessed interior floor & control desk (visible through panoramic glass)
    b.rbox([cx, win_y0 + 1.8 * scale_f, cz - 1.5 * scale_f], [cab_w * 0.52, 4.2 * scale_f, cab_d * 0.35], p.steel_dark);
    b.cyl_y([cx + cab_w * 0.15, win_y0 + 5.2 * scale_f, cz - 1.5 * scale_f], 2.2 * scale_f, 2.2 * scale_f, 0.8 * scale_f, 10, p.bronze);

    // 4. Outer (+Z) panoramic 3-bay window + Left/Right (-X/+X) windows
    b.rbox([cx, win_cy, outer_z - wall_t * 0.5], [cab_w - 4.0 * scale_f, win_h, 0.9 * scale_f], p.win_pane);
    for mx in [-cab_w * 0.16, cab_w * 0.16] {
        b.rbox([cx + mx, win_cy, outer_z - wall_t * 0.3], [1.1 * scale_f, win_h + 0.4 * scale_f, 1.3 * scale_f], p.cab_trim);
    }
    for sx in [-1.0_f32, 1.0] {
        let wx = cx + sx * (cab_w * 0.5 - wall_t * 0.5);
        b.rbox([wx, win_cy, cz], [0.9 * scale_f, win_h, cab_d - 4.4 * scale_f], p.win_pane);
        b.rbox([wx + sx * 0.2 * scale_f, win_cy, cz], [1.2 * scale_f, win_h + 0.4 * scale_f, 1.1 * scale_f], p.cab_trim);
    }

    // 5. Bridge-facing (-Z) wall: observation window on -X side, walkway access door + entry lamp on +X side
    let obs_x = cx - cab_w * 0.16;
    let obs_w = cab_w * 0.46;
    b.rbox([obs_x, win_cy, inner_z + wall_t * 0.5], [obs_w, win_h, 0.9 * scale_f], p.win_pane);
    b.rbox([obs_x, win_cy, inner_z + wall_t * 0.3], [1.1 * scale_f, win_h + 0.4 * scale_f, 1.3 * scale_f], p.cab_trim);

    let door_x = cx + cab_w * 0.25;
    let door_w = cab_w * 0.26;
    let door_h = cab_h - 4.2 * scale_f;
    b.rbox([cx + cab_w * 0.25, y_deck + cab_h * 0.5, inner_z + wall_t * 0.5], [door_w + 3.0 * scale_f, cab_h, wall_t], p.cab_wall);
    b.rbox([door_x, y_deck + door_h * 0.5 + 0.4 * scale_f, inner_z + 0.15 * scale_f], [door_w, door_h, 1.0 * scale_f], p.cab_trim);
    b.rbox([door_x, y_deck + door_h * 0.68, inner_z - 0.2 * scale_f], [door_w * 0.55, 5.5 * scale_f, 0.6 * scale_f], p.win_pane);
    b.rbox([door_x, y_deck + door_h + 1.8 * scale_f, inner_z - 1.1 * scale_f], [2.2 * scale_f, 1.6 * scale_f, 1.6 * scale_f], p.lamp_warm);

    // 6. Pitched overhanging metal roof + ventilator chimney
    let roof_y = y_deck + cab_h;
    let ridge_h = 6.2 * scale_f;
    b.gable_roof_x([cx, roof_y, cz], cab_w, cab_d, ridge_h, 2.4 * scale_f, 1.1 * scale_f, p.cab_roof, p.cab_wall, p.cab_trim);
    b.cyl_y([cx + cab_w * 0.26, roof_y + ridge_h + 1.2 * scale_f, cz + cab_d * 0.12], 1.5 * scale_f, 1.7 * scale_f, 4.8 * scale_f, 8, p.steel_dark);
    b.cyl_y([cx + cab_w * 0.26, roof_y + ridge_h + 3.8 * scale_f, cz + cab_d * 0.12], 2.5 * scale_f, 2.8 * scale_f, 0.9 * scale_f, 8, p.cab_trim);
}

fn build_catenary_portal(
    b: &mut ModelBuilder<'_>,
    cx: f32,
    y_base: f32,
    cz: f32,
    yaw_rad: f32,
    half_span: f32,
    wire_h: f32,
    top_h: f32,
    p: &SharedPalette,
) {
    let post_h = top_h - y_base;
    let cy_post = y_base + post_h * 0.5;
    for sz in [-1.0_f32, 1.0] {
        let (dx, dz) = (yaw_rad.sin() * sz * half_span, yaw_rad.cos() * sz * half_span);
        b.obox([cx + dx, cy_post, cz + dz], [2.0, post_h, 2.0], [0.0, yaw_rad, 0.0], p.cat_green);
        b.obox([cx + dx, y_base + 1.6, cz + dz], [3.6, 3.2, 3.6], [0.0, yaw_rad, 0.0], p.concrete);
    }
    // Cross-beam at top_h + ceramic insulator drop to contact wire height
    b.obox([cx, top_h - 0.8, cz], [2.2, 2.0, half_span * 2.0 + 2.6], [0.0, yaw_rad, 0.0], p.cat_green);
    b.obox([cx, wire_h + 2.4, cz], [1.4, (top_h - 1.2 - wire_h).max(2.0), 1.4], [0.0, yaw_rad, 0.0], p.insulator);
    b.obox([cx, wire_h, cz], [1.2, 0.9, 5.0], [0.0, yaw_rad, 0.0], p.wire_copper);
}

// ============================================================================
//  1. TURNTABLE (DB 23 m Einheitsdrehscheibe — Märklin 74862 / Trix 66862)
// ============================================================================

mod turntable;
#[allow(unused_imports)]
use turntable::*;
pub(crate) use turntable::{push_turntable_3d, extract_transfer_track_zs};
mod transfer_table;
#[allow(unused_imports)]
use transfer_table::*;
pub(crate) use transfer_table::{push_transfer_table_3d, transfer_table_world_rect_corners, turntable_outer_radius, transfer_table_local_bounds, transfer_table_outer_world_corners};
