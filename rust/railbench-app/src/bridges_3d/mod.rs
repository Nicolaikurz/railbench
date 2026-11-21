use railbench_core::geometry::sample_arc_between_connectors;
use railbench_core::types::{CatalogEntry, Layout, TrackPiece};

use crate::viewer3d::{
    cross3, dot3, normalize3, polyline_miter_normal, push_box_polyline_mat_3d, push_quad_mat,
    rot_xz, Vertex3D, MAT_METAL_RAIL,
};

// Material descriptors (roughness strictly <= 0.92 so the procedural WGSL gravel shader never
// triggers on painted bridge steel, dressed masonry stone, or precast concrete).
const MAT_BRIDGE_STEEL: [f32; 3] = [0.46, 0.56, 0.0];
const MAT_GALV_STEEL: [f32; 3] = [0.34, 0.72, 0.0];
const MAT_CAST_IRON: [f32; 3] = [0.40, 0.78, 0.0];
const MAT_BRONZE: [f32; 3] = [0.32, 0.85, 0.0];
const MAT_STONE: [f32; 3] = [0.88, 0.02, 0.0];
const MAT_CONCRETE: [f32; 3] = [0.84, 0.02, 0.0];

// Structural bridge steel & masonry color palette
const STEEL_WEB: [f32; 3] = [0.28, 0.34, 0.40];
const STEEL_FLANGE: [f32; 3] = [0.35, 0.41, 0.48];
const STEEL_DARK: [f32; 3] = [0.22, 0.27, 0.33];
const STEEL_BEAM: [f32; 3] = [0.25, 0.30, 0.36];
const GALV_RAIL: [f32; 3] = [0.60, 0.65, 0.70];
const GALV_GRATE: [f32; 3] = [0.46, 0.50, 0.55];
const IRON_BEARING: [f32; 3] = [0.18, 0.20, 0.23];
const BRONZE_PIN: [f32; 3] = [0.68, 0.52, 0.24];
const STONE_CAP: [f32; 3] = [0.62, 0.60, 0.56];
const STONE_SHAFT: [f32; 3] = [0.52, 0.50, 0.47];
const STONE_DARK: [f32; 3] = [0.42, 0.41, 0.39];
const CONCRETE_DECK: [f32; 3] = [0.64, 0.65, 0.66];
const CONCRETE_SOFFIT: [f32; 3] = [0.54, 0.56, 0.58];
const CONCRETE_CURB: [f32; 3] = [0.72, 0.73, 0.74];

const GROUND_ANCHOR_Y: f32 = -2.0;

// ============================================================================
//  Public Entry Points
// ============================================================================

/// Appends detailed 3D steel bridge geometry (*Blechträgerbrücke* on curves/turnouts,
/// *Fachwerkbrücke* Warren truss on straight spans), inspection catwalks, handrails,
/// inner guard rails (*Zwangsschienen*), stone masonry piers (*Brückenpfeiler*),
/// and masonry abutments (*Widerlager*) for an elevated/bridged railway piece.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_railway_bridge_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    layout: &Layout,
    pts: &[[f32; 3]],
    br_idx: usize,
    gauge: f32,
    ballast_bot_w: f32,
    ballast_h: f32,
    tie_h: f32,
) {
    if pts.len() < 2 {
        return;
    }
    let cum_dists = cumulative_xz_dists(pts);
    let total_len = *cum_dists.last().unwrap_or(&0.0);
    if total_len < 1.0 {
        return;
    }

    let sc = (gauge / 16.5).clamp(0.45, 2.5);
    let girder_off = ballast_bot_w * 0.5 + 5.0 * sc; // 26.0 mm H0 -> ~50 mm clear inner width
    let deck_w = ballast_bot_w + 10.0 * sc;

    // 1. Continuous steel trough deck under ballast (all branches)
    push_vertical_offset_polyline_mat_3d(
        verts,
        idxs,
        pts,
        -3.6 * sc,
        deck_w,
        3.6 * sc,
        STEEL_WEB,
        MAT_BRIDGE_STEEL,
    );

    // 2. Twin longitudinal I-beam rail stringers directly under the running rails
    let hg = gauge * 0.5;
    for s in [-1.0_f32, 1.0] {
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * hg,
            -11.0 * sc,
            1.8 * sc,
            7.4 * sc,
            STEEL_DARK,
            MAT_BRIDGE_STEEL,
        );
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * hg,
            -11.8 * sc,
            3.8 * sc,
            0.9 * sc,
            STEEL_BEAM,
            MAT_BRIDGE_STEEL,
        );
    }

    // 3. Inner steel guard rails (Zwangsschienen) on sleeper tops between running rails
    let start_bridged = is_bridge_neighbor(piece, 0, layout);
    let end_bridged = is_bridge_neighbor(piece, 1, layout);
    let guard_off = (hg - 2.6 * sc).max(2.0 * sc);
    let guard_y = ballast_h + tie_h;
    push_guard_rails(
        verts,
        idxs,
        pts,
        &cum_dists,
        total_len,
        guard_off,
        guard_y,
        sc,
        br_idx == 0 && !start_bridged,
        br_idx == 0 && !end_bridged,
    );

    // 4. Transverse under-deck floor cross-beams (Querträger) & lower wind X-bracing
    let n_bays = ((total_len / (22.0 * sc)).round() as usize).max(2);
    let cross_span = if br_idx == 0 { girder_off * 2.0 } else { ballast_bot_w };
    let mut bay_samples = Vec::with_capacity(n_bays + 1);
    for k in 0..=n_bays {
        let d = (k as f32 / n_bays as f32) * total_len;
        let sample = sample_polyline_at_dist(pts, &cum_dists, d);
        let [tx, tz] = sample.tangent;
        let yaw = tz.atan2(tx);
        push_oriented_box_6_mat(
            verts,
            idxs,
            sample.pos,
            yaw,
            [0.0, -7.4 * sc, 0.0],
            [2.2 * sc, 7.2 * sc, cross_span],
            STEEL_BEAM,
            MAT_BRIDGE_STEEL,
        );
        bay_samples.push(sample);
    }

    if br_idx > 0 {
        // Keep diverging turnout legs tucked cleanly without intersecting main girders/piers.
        return;
    }

    let brace_lat = girder_off - 2.0 * sc;
    for w in bay_samples.windows(2) {
        let a = &w[0];
        let b = &w[1];
        let al = [
            a.pos[0] + a.normal[0] * brace_lat,
            a.pos[1] - 10.0 * sc,
            a.pos[2] + a.normal[1] * brace_lat,
        ];
        let ar = [
            a.pos[0] - a.normal[0] * brace_lat,
            a.pos[1] - 10.0 * sc,
            a.pos[2] - a.normal[1] * brace_lat,
        ];
        let bl = [
            b.pos[0] + b.normal[0] * brace_lat,
            b.pos[1] - 10.0 * sc,
            b.pos[2] + b.normal[1] * brace_lat,
        ];
        let br_pt = [
            b.pos[0] - b.normal[0] * brace_lat,
            b.pos[1] - 10.0 * sc,
            b.pos[2] - b.normal[1] * brace_lat,
        ];
        push_strut_3d_mat(verts, idxs, al, br_pt, 1.2 * sc, 1.2 * sc, STEEL_DARK, MAT_BRIDGE_STEEL);
        push_strut_3d_mat(verts, idxs, ar, bl, 1.2 * sc, 1.2 * sc, STEEL_DARK, MAT_BRIDGE_STEEL);
    }

    // 5. Continuous mitered main steel plate girders (Blechträger) + vertical stiffeners
    for s in [-1.0_f32, 1.0] {
        let lat = s * girder_off;
        // Web
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            lat,
            -12.0 * sc,
            2.4 * sc,
            26.5 * sc,
            STEEL_WEB,
            MAT_BRIDGE_STEEL,
        );
        // Top riveted flange
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            lat,
            14.3 * sc,
            5.4 * sc,
            1.6 * sc,
            STEEL_FLANGE,
            MAT_BRIDGE_STEEL,
        );
        // Bottom riveted flange
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            lat,
            -13.4 * sc,
            5.6 * sc,
            1.6 * sc,
            STEEL_BEAM,
            MAT_BRIDGE_STEEL,
        );
    }

    for sample in &bay_samples {
        let yaw = sample.tangent[1].atan2(sample.tangent[0]);
        for s in [-1.0_f32, 1.0] {
            push_oriented_box_6_mat(
                verts,
                idxs,
                sample.pos,
                yaw,
                [0.0, 1.1 * sc, s * girder_off],
                [1.4 * sc, 26.2 * sc, 4.6 * sc],
                STEEL_FLANGE,
                MAT_BRIDGE_STEEL,
            );
        }
    }

    // 6. Cantilevered steel-grated inspection catwalks (Dienststege) & 2-bar safety handrails
    let catwalk_lat = girder_off + 4.8 * sc;
    let kick_lat = girder_off + 7.7 * sc;
    let rail_lat = girder_off + 7.9 * sc;
    for s in [-1.0_f32, 1.0] {
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * catwalk_lat,
            0.5 * sc,
            6.6 * sc,
            1.1 * sc,
            GALV_GRATE,
            MAT_GALV_STEEL,
        );
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * kick_lat,
            1.5 * sc,
            0.8 * sc,
            2.2 * sc,
            STEEL_FLANGE,
            MAT_BRIDGE_STEEL,
        );
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * rail_lat,
            15.0 * sc,
            1.1 * sc,
            1.1 * sc,
            GALV_RAIL,
            MAT_GALV_STEEL,
        );
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * rail_lat,
            8.5 * sc,
            0.85 * sc,
            0.85 * sc,
            GALV_GRATE,
            MAT_GALV_STEEL,
        );
    }

    for sample in &bay_samples {
        for s in [-1.0_f32, 1.0] {
            let [nx, nz] = [sample.normal[0] * s, sample.normal[1] * s];
            let post_bot = [
                sample.pos[0] + nx * rail_lat,
                sample.pos[1] + 1.0 * sc,
                sample.pos[2] + nz * rail_lat,
            ];
            let post_top = [
                sample.pos[0] + nx * rail_lat,
                sample.pos[1] + 15.6 * sc,
                sample.pos[2] + nz * rail_lat,
            ];
            push_strut_3d_mat(verts, idxs, post_bot, post_top, 1.1 * sc, 1.1 * sc, GALV_RAIL, MAT_GALV_STEEL);

            let br_in = [
                sample.pos[0] + nx * (girder_off + 1.0 * sc),
                sample.pos[1] - 9.0 * sc,
                sample.pos[2] + nz * (girder_off + 1.0 * sc),
            ];
            let br_out = [
                sample.pos[0] + nx * (girder_off + 7.2 * sc),
                sample.pos[1] + 0.6 * sc,
                sample.pos[2] + nz * (girder_off + 7.2 * sc),
            ];
            push_strut_3d_mat(verts, idxs, br_in, br_out, 1.2 * sc, 1.2 * sc, STEEL_DARK, MAT_BRIDGE_STEEL);
        }
    }

    // 7. Steel Warren Truss superstructure (Fachwerkbrücke) on straight railway spans
    if is_straight_truss_piece(piece, entry, pts, total_len, sc) {
        push_warren_truss_superstructure(
            verts,
            idxs,
            piece,
            layout,
            pts,
            &cum_dists,
            total_len,
            girder_off,
            sc,
        );
    }

    // 8. Track-aligned multi-tier stone masonry piers (Brückenpfeiler)
    push_railway_piers(verts, idxs, piece, layout, pts, &cum_dists, total_len, girder_off, sc);

    // 9. Masonry bridge abutments (Widerlager) with angled wing walls on outer ends
    push_railway_abutments(
        verts,
        idxs,
        piece,
        layout,
        pts,
        girder_off,
        sc,
        start_bridged,
        end_bridged,
    );
}

/// Appends detailed 3D precast concrete box-girder road bridge geometry with raised curbs,
/// galvanized two-rail guardrails (*Leitplanken*), concrete hammerhead piers, and abutments.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_road_bridge_3d(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    layout: &Layout,
    pts: &[[f32; 3]],
    br_idx: usize,
    road_w: f32,
) {
    if pts.len() < 2 {
        return;
    }
    let cum_dists = cumulative_xz_dists(pts);
    let total_len = *cum_dists.last().unwrap_or(&0.0);
    if total_len < 1.0 {
        return;
    }

    let half_rw = road_w * 0.5;

    // 1. Precast concrete deck slab & tapered underside box-girder spine
    push_vertical_offset_polyline_mat_3d(
        verts,
        idxs,
        pts,
        -3.6,
        road_w + 14.0,
        3.6,
        CONCRETE_DECK,
        MAT_CONCRETE,
    );
    push_soffit_polyline_mat_3d(
        verts,
        idxs,
        pts,
        -10.5,
        road_w * 0.78,
        road_w * 0.62,
        6.9,
        CONCRETE_SOFFIT,
        MAT_CONCRETE,
    );

    if br_idx > 0 {
        return;
    }

    // 2. Raised concrete curbs / sidewalks & two-rail galvanized guardrails (Leitplanken)
    let curb_lat = half_rw + 4.2;
    let rail_lat = half_rw + 6.4;
    for s in [-1.0_f32, 1.0] {
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * curb_lat,
            0.0,
            5.6,
            3.8,
            CONCRETE_CURB,
            MAT_CONCRETE,
        );
        // W-beam crash guardrail
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * rail_lat,
            6.2,
            1.3,
            2.4,
            GALV_RAIL,
            MAT_GALV_STEEL,
        );
        // Upper pedestrian railing
        push_offset_box_polyline_mat_3d(
            verts,
            idxs,
            pts,
            s * rail_lat,
            14.2,
            1.1,
            1.1,
            GALV_RAIL,
            MAT_GALV_STEEL,
        );
    }

    let n_posts = ((total_len / 24.0).round() as usize).max(2);
    for k in 0..=n_posts {
        let d = (k as f32 / n_posts as f32) * total_len;
        let sample = sample_polyline_at_dist(pts, &cum_dists, d);
        for s in [-1.0_f32, 1.0] {
            let [nx, nz] = [sample.normal[0] * s, sample.normal[1] * s];
            let bot = [sample.pos[0] + nx * rail_lat, sample.pos[1] + 3.2, sample.pos[2] + nz * rail_lat];
            let top = [sample.pos[0] + nx * rail_lat, sample.pos[1] + 14.8, sample.pos[2] + nz * rail_lat];
            push_strut_3d_mat(verts, idxs, bot, top, 1.2, 1.2, GALV_GRATE, MAT_GALV_STEEL);
        }
    }

    // 3. Tapered concrete hammerhead piers & abutments
    push_road_piers(verts, idxs, piece, layout, pts, &cum_dists, total_len, road_w);

    let start_bridged = is_bridge_neighbor(piece, 0, layout);
    let end_bridged = is_bridge_neighbor(piece, 1, layout);
    push_road_abutments(verts, idxs, piece, layout, pts, road_w, start_bridged, end_bridged);
}

// ============================================================================
//  Railway Bridge Components (Guard Rails, Warren Truss, Masonry Piers, Abutments)
// ============================================================================

#[allow(clippy::too_many_arguments)]
fn push_guard_rails(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    pts: &[[f32; 3]],
    cum_dists: &[f32],
    total_len: f32,
    guard_off: f32,
    guard_y: f32,
    sc: f32,
    taper_start: bool,
    taper_end: bool,
) {
    let taper_len = (10.0 * sc).min(total_len * 0.25);
    let tip_off = (1.3 * sc).min(guard_off * 0.45);
    let can_taper = total_len > 36.0 * sc;

    for s in [-1.0_f32, 1.0] {
        let mut gpts = Vec::with_capacity(pts.len());
        for (i, &p) in pts.iter().enumerate() {
            let d = cum_dists[i];
            let mut lat = guard_off;
            if can_taper && taper_start && d < taper_len {
                let t = (d / taper_len).clamp(0.0, 1.0);
                lat = tip_off + (guard_off - tip_off) * t;
            } else if can_taper && taper_end && (total_len - d) < taper_len {
                let t = ((total_len - d) / taper_len).clamp(0.0, 1.0);
                lat = tip_off + (guard_off - tip_off) * t;
            }
            let (nx, nz, mscale) = polyline_miter_normal(pts, i);
            gpts.push([
                p[0] + nx * s * lat * mscale,
                p[1] + guard_y,
                p[2] + nz * s * lat * mscale,
            ]);
        }
        push_box_polyline_mat_3d(
            verts,
            idxs,
            &gpts,
            1.05 * sc,
            2.1 * sc,
            [0.68, 0.72, 0.76],
            MAT_METAL_RAIL,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_warren_truss_superstructure(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    layout: &Layout,
    pts: &[[f32; 3]],
    cum_dists: &[f32],
    total_len: f32,
    girder_off: f32,
    sc: f32,
) {
    let start_cont = is_straight_bridge_neighbor(piece, 0, layout);
    let end_cont = is_straight_bridge_neighbor(piece, 1, layout);

    // Height clears H0 rolling stock AND catenary contact/messenger wires (72 / 85 mm)
    let truss_h = 88.0 * sc;
    let bot_y = 15.0 * sc;
    let n_truss = ((total_len / (42.0 * sc)).round() as usize).clamp(3, 12);
    let nf = n_truss as f32;

    let sample_at_u = |u: f32| -> PathSample {
        sample_polyline_at_dist(pts, cum_dists, u.clamp(0.0, 1.0) * total_len)
    };

    let mut bot_samples = Vec::with_capacity(n_truss + 1);
    for i in 0..=n_truss {
        bot_samples.push(sample_at_u(i as f32 / nf));
    }
    let mut top_samples = Vec::with_capacity(n_truss);
    for i in 0..n_truss {
        top_samples.push(sample_at_u((i as f32 + 0.5) / nf));
    }

    let side_pt = |smp: &PathSample, side: f32, y_off: f32| -> [f32; 3] {
        [
            smp.pos[0] + smp.normal[0] * side * girder_off,
            smp.pos[1] + y_off,
            smp.pos[2] + smp.normal[1] * side * girder_off,
        ]
    };

    // Build top-chord polyline stations (extending to u=0 / u=1 when joined to adjacent straight bridge)
    let mut chord_stations: Vec<PathSample> = Vec::with_capacity(n_truss + 2);
    if start_cont {
        chord_stations.push(bot_samples[0].clone());
    }
    chord_stations.extend(top_samples.iter().cloned());
    if end_cont {
        chord_stations.push(bot_samples[n_truss].clone());
    }

    for s in [-1.0_f32, 1.0] {
        // Top chord box beams + riveted top cap
        for w in chord_stations.windows(2) {
            let p0 = side_pt(&w[0], s, truss_h);
            let p1 = side_pt(&w[1], s, truss_h);
            push_strut_3d_mat(verts, idxs, p0, p1, 4.2 * sc, 3.8 * sc, STEEL_FLANGE, MAT_BRIDGE_STEEL);
            let c0 = [p0[0], p0[1] + 2.0 * sc, p0[2]];
            let c1 = [p1[0], p1[1] + 2.0 * sc, p1[2]];
            push_strut_3d_mat(verts, idxs, c0, c1, 5.0 * sc, 1.0 * sc, STEEL_WEB, MAT_BRIDGE_STEEL);
        }

        // Warren W-diagonals, vertical hanger posts & gusset plates
        for i in 0..n_truss {
            let b0 = side_pt(&bot_samples[i], s, bot_y);
            let tp = side_pt(&top_samples[i], s, truss_h);
            let b1 = side_pt(&bot_samples[i + 1], s, bot_y);

            let rising_w = if i == 0 && !start_cont { 3.5 * sc } else { 2.6 * sc };
            let falling_w = if i + 1 == n_truss && !end_cont { 3.5 * sc } else { 2.6 * sc };
            push_strut_3d_mat(verts, idxs, b0, tp, rising_w, rising_w, STEEL_FLANGE, MAT_BRIDGE_STEEL);
            push_strut_3d_mat(verts, idxs, tp, b1, falling_w, falling_w, STEEL_FLANGE, MAT_BRIDGE_STEEL);

            // Vertical tension hanger from top node to lower flange
            let v_bot = side_pt(&top_samples[i], s, bot_y);
            push_strut_3d_mat(verts, idxs, v_bot, tp, 1.8 * sc, 1.8 * sc, STEEL_WEB, MAT_BRIDGE_STEEL);

            // Gusset plate at top node
            let yaw_t = top_samples[i].tangent[1].atan2(top_samples[i].tangent[0]);
            push_oriented_box_6_mat(
                verts,
                idxs,
                top_samples[i].pos,
                yaw_t,
                [0.0, truss_h - 1.2 * sc, s * girder_off],
                [7.6 * sc, 5.8 * sc, 3.4 * sc],
                STEEL_BEAM,
                MAT_BRIDGE_STEEL,
            );
        }

        for bs in &bot_samples {
            let yaw_b = bs.tangent[1].atan2(bs.tangent[0]);
            push_oriented_box_6_mat(
                verts,
                idxs,
                bs.pos,
                yaw_b,
                [0.0, bot_y + 1.8 * sc, s * girder_off],
                [7.6 * sc, 5.4 * sc, 3.4 * sc],
                STEEL_BEAM,
                MAT_BRIDGE_STEEL,
            );
        }

        // Vertical end portal posts when joining directly onto a neighboring straight truss bridge
        if start_cont {
            push_strut_3d_mat(
                verts,
                idxs,
                side_pt(&bot_samples[0], s, bot_y),
                side_pt(&bot_samples[0], s, truss_h),
                3.4 * sc,
                3.4 * sc,
                STEEL_FLANGE,
                MAT_BRIDGE_STEEL,
            );
        }
        if end_cont {
            push_strut_3d_mat(
                verts,
                idxs,
                side_pt(&bot_samples[n_truss], s, bot_y),
                side_pt(&bot_samples[n_truss], s, truss_h),
                3.4 * sc,
                3.4 * sc,
                STEEL_FLANGE,
                MAT_BRIDGE_STEEL,
            );
        }
    }

    // Overhead wind portal cross-beams & knee braces (keeping >= 32 mm center clear envelope)
    let knee_in_lat = (girder_off - 9.5 * sc).max(11.0 * sc);
    for (idx, smp) in chord_stations.iter().enumerate() {
        let left_top = side_pt(smp, 1.0, truss_h);
        let right_top = side_pt(smp, -1.0, truss_h);
        push_strut_3d_mat(verts, idxs, left_top, right_top, 3.0 * sc, 3.2 * sc, STEEL_FLANGE, MAT_BRIDGE_STEEL);

        let is_portal = idx == 0 || idx + 1 == chord_stations.len();
        let drop = if is_portal { 12.0 * sc } else { 7.5 * sc };
        for s in [-1.0_f32, 1.0] {
            let outer = side_pt(smp, s, truss_h - drop);
            let inner = [
                smp.pos[0] + smp.normal[0] * s * knee_in_lat,
                smp.pos[1] + truss_h - 0.8 * sc,
                smp.pos[2] + smp.normal[1] * s * knee_in_lat,
            ];
            push_strut_3d_mat(verts, idxs, outer, inner, 1.7 * sc, 1.7 * sc, STEEL_DARK, MAT_BRIDGE_STEEL);
        }
    }

    // Overhead horizontal wind X-bracing between adjacent top portal frames
    let x_lat = girder_off - 1.5 * sc;
    for w in chord_stations.windows(2) {
        let a = &w[0];
        let b = &w[1];
        let al = [a.pos[0] + a.normal[0] * x_lat, a.pos[1] + truss_h, a.pos[2] + a.normal[1] * x_lat];
        let ar = [a.pos[0] - a.normal[0] * x_lat, a.pos[1] + truss_h, a.pos[2] - a.normal[1] * x_lat];
        let bl = [b.pos[0] + b.normal[0] * x_lat, b.pos[1] + truss_h, b.pos[2] + b.normal[1] * x_lat];
        let br_pt = [b.pos[0] - b.normal[0] * x_lat, b.pos[1] + truss_h, b.pos[2] - b.normal[1] * x_lat];
        push_strut_3d_mat(verts, idxs, al, br_pt, 1.5 * sc, 1.5 * sc, STEEL_BEAM, MAT_BRIDGE_STEEL);
        push_strut_3d_mat(verts, idxs, ar, bl, 1.5 * sc, 1.5 * sc, STEEL_BEAM, MAT_BRIDGE_STEEL);
    }
}

mod supports;
#[allow(unused_imports)]
use supports::*;
mod geometry;
#[allow(unused_imports)]
use geometry::*;
#[cfg(test)]
mod tests;
