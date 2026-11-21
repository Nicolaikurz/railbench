use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn push_railway_piers(
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
    let target_fracs = pier_target_fractions(total_len, sc);
    let clearance_r = girder_off + 24.0 * sc;

    for u0 in target_fracs {
        let Some(sample) = find_clear_pier_sample(
            pts,
            cum_dists,
            total_len,
            u0,
            14.0 * sc,
            clearance_r,
            piece,
            layout,
        ) else {
            continue;
        };

        let yaw = sample.tangent[1].atan2(sample.tangent[0]);
        let seat_y = sample.pos[1] - 13.4 * sc;
        let origin = [sample.pos[0], 0.0, sample.pos[2]];

        // 1. Cast-steel rocker bearings (Brückenlager) + bronze pins under both main girders
        for s in [-1.0_f32, 1.0] {
            let lz = s * girder_off;
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [0.0, seat_y - 0.8 * sc, lz],
                [6.4 * sc, 1.6 * sc, 6.4 * sc],
                IRON_BEARING,
                MAT_CAST_IRON,
            );
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [0.0, seat_y - 2.4 * sc, lz],
                [3.2 * sc, 2.0 * sc, 6.8 * sc],
                BRONZE_PIN,
                MAT_BRONZE,
            );
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [0.0, seat_y - 4.2 * sc, lz],
                [8.2 * sc, 2.0 * sc, 8.2 * sc],
                IRON_BEARING,
                MAT_CAST_IRON,
            );
        }

        // 2. Overhanging chamfered stone pier cap (Kämpfergesims)
        let cap_top = seat_y - 5.2 * sc;
        let cap_bot = (cap_top - 6.0 * sc).max(GROUND_ANCHOR_Y + 4.0);
        let span_lz = girder_off * 2.0;
        push_tapered_prism_y_mat(
            verts,
            idxs,
            origin,
            yaw,
            cap_bot,
            cap_top,
            18.5 * sc,
            span_lz + 10.0 * sc,
            22.0 * sc,
            span_lz + 14.0 * sc,
            STONE_CAP,
            MAT_STONE,
        );

        // 3. Tapered ashlar masonry shaft + pointed cutwaters (Eisbrecher)
        let shaft_h = cap_bot - GROUND_ANCHOR_Y;
        let plinth_top = (GROUND_ANCHOR_Y + shaft_h * 0.16).clamp(GROUND_ANCHOR_Y + 3.0, GROUND_ANCHOR_Y + 12.0 * sc);
        let shaft_bot_lx = 21.0 * sc;
        let shaft_top_lx = 16.5 * sc;
        let shaft_bot_lz = span_lz + 12.0 * sc;
        let shaft_top_lz = span_lz + 6.0 * sc;
        if cap_bot > plinth_top + 0.5 {
            push_tapered_prism_y_mat(
                verts,
                idxs,
                origin,
                yaw,
                plinth_top,
                cap_bot,
                shaft_bot_lx,
                shaft_bot_lz,
                shaft_top_lx,
                shaft_top_lz,
                STONE_SHAFT,
                MAT_STONE,
            );
            // Mid-height decorative stone belt course on tall piers
            if cap_bot - plinth_top > 38.0 * sc {
                let belt_y = 0.5 * (plinth_top + cap_bot);
                push_tapered_prism_y_mat(
                    verts,
                    idxs,
                    origin,
                    yaw,
                    belt_y - 1.4 * sc,
                    belt_y + 1.4 * sc,
                    20.2 * sc,
                    span_lz + 10.6 * sc,
                    20.2 * sc,
                    span_lz + 10.6 * sc,
                    STONE_CAP,
                    MAT_STONE,
                );
            }
            // Pointed cutwater noses along +Z and -Z
            for s in [-1.0_f32, 1.0] {
                push_cutwater_prism_y_mat(
                    verts,
                    idxs,
                    origin,
                    yaw,
                    plinth_top,
                    cap_bot,
                    shaft_bot_lx * 0.48,
                    shaft_top_lx * 0.45,
                    s * (shaft_bot_lz * 0.5),
                    s * (shaft_top_lz * 0.5),
                    s * (shaft_bot_lz * 0.5 + 10.5 * sc),
                    s * (shaft_top_lz * 0.5 + 6.5 * sc),
                    STONE_DARK,
                    MAT_STONE,
                );
            }
        }

        // 4. Stepped masonry foundation plinth & lower cutwater
        let pl_lz = span_lz + 22.0 * sc;
        let pl_lx = 25.0 * sc;
        push_tapered_prism_y_mat(
            verts,
            idxs,
            origin,
            yaw,
            GROUND_ANCHOR_Y,
            plinth_top,
            pl_lx + 2.5 * sc,
            pl_lz + 2.0 * sc,
            pl_lx,
            pl_lz,
            STONE_DARK,
            MAT_STONE,
        );
        for s in [-1.0_f32, 1.0] {
            push_cutwater_prism_y_mat(
                verts,
                idxs,
                origin,
                yaw,
                GROUND_ANCHOR_Y,
                plinth_top,
                pl_lx * 0.48,
                pl_lx * 0.46,
                s * (pl_lz * 0.5),
                s * (pl_lz * 0.5),
                s * (pl_lz * 0.5 + 11.5 * sc),
                s * (pl_lz * 0.5 + 10.0 * sc),
                STONE_DARK,
                MAT_STONE,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_railway_abutments(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    layout: &Layout,
    pts: &[[f32; 3]],
    girder_off: f32,
    sc: f32,
    start_bridged: bool,
    end_bridged: bool,
) {
    let min_elev = 14.0 * sc;
    let clearance_r = girder_off + 20.0 * sc;
    let n = pts.len();
    let ends: [(bool, [f32; 3], [f32; 2]); 2] = [
        (start_bridged, pts[0], seg_unit_dir(pts[1], pts[0])),
        (end_bridged, pts[n - 1], seg_unit_dir(pts[n - 2], pts[n - 1])),
    ];

    for (neighbor_bridged, pos, out_dir) in ends {
        if neighbor_bridged || pos[1] < min_elev {
            continue;
        }
        let check_x = pos[0] + out_dir[0] * 10.0 * sc;
        let check_z = pos[2] + out_dir[1] * 10.0 * sc;
        if !is_support_footprint_clear(check_x, check_z, pos[1], clearance_r, piece, layout) {
            continue;
        }

        // Local +X points outward away from the bridge span; local -X extends under girder end
        let yaw = out_dir[1].atan2(out_dir[0]);
        let origin = [pos[0], 0.0, pos[2]];
        let track_y = pos[1];
        let seat_y = (track_y - 13.4 * sc).max(GROUND_ANCHOR_Y + 7.5);
        let wall_top = (seat_y - 4.8 * sc).max(GROUND_ANCHOR_Y + 4.0);
        let wall_w = girder_off * 2.0 + 13.0 * sc;

        // Bearings on bridge seat shelf
        for s in [-1.0_f32, 1.0] {
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [-0.5 * sc, seat_y - 2.4 * sc, s * girder_off],
                [6.6 * sc, 4.8 * sc, 6.8 * sc],
                IRON_BEARING,
                MAT_CAST_IRON,
            );
        }

        // Bridge seat shelf cap & battered masonry breast wall
        push_tapered_prism_y_mat(
            verts,
            idxs,
            origin,
            yaw,
            GROUND_ANCHOR_Y + 5.5,
            wall_top,
            21.0 * sc,
            wall_w + 3.0 * sc,
            18.5 * sc,
            wall_w,
            STONE_SHAFT,
            MAT_STONE,
        );
        // Shift abutment wall center slightly outward (+4*sc) via oriented boxes for ballast backwall
        // Upper ballast support backwall STRICTLY below deck (y <= track_y - 0.5) so tracks are never blocked!
        let back_top = track_y - 0.6;
        if back_top > wall_top {
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [8.5 * sc, 0.5 * (wall_top + back_top), 0.0],
                [10.0 * sc, back_top - wall_top, wall_w],
                STONE_CAP,
                MAT_STONE,
            );
        }
        // Flanking masonry cheek parapets outside girders/catwalk clearance
        let parapet_h = 13.5 * sc;
        for s in [-1.0_f32, 1.0] {
            let pz = s * (girder_off + 3.5 * sc);
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [8.5 * sc, wall_top + (track_y - wall_top + parapet_h) * 0.5, pz],
                [10.5 * sc, track_y - wall_top + parapet_h, 5.5 * sc],
                STONE_CAP,
                MAT_STONE,
            );
        }
        // Plinth foundation
        push_oriented_box_6_mat(
            verts,
            idxs,
            origin,
            yaw,
            [3.0 * sc, GROUND_ANCHOR_Y + 3.0, 0.0],
            [24.0 * sc, 6.0, wall_w + 8.0 * sc],
            STONE_DARK,
            MAT_STONE,
        );

        // 30-degree splayed masonry wing walls (Flügelmauern) retaining approaching embankment
        push_wing_walls(
            verts,
            idxs,
            origin,
            yaw,
            9.5 * sc,
            girder_off + 5.0 * sc,
            25.0 * sc,
            4.5 * sc,
            track_y + 9.0 * sc,
            GROUND_ANCHOR_Y + 9.0 * sc,
            STONE_SHAFT,
            STONE_CAP,
            MAT_STONE,
        );
    }
}

// ============================================================================
//  Road Bridge Piers & Abutments
// ============================================================================

pub(super) fn push_road_piers(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    layout: &Layout,
    pts: &[[f32; 3]],
    cum_dists: &[f32],
    total_len: f32,
    road_w: f32,
) {
    let sc = (road_w / 60.0).clamp(0.55, 2.2);
    let target_fracs = pier_target_fractions(total_len, sc);
    let half_rw = road_w * 0.5;
    let clearance_r = half_rw + 24.0;

    for u0 in target_fracs {
        let Some(sample) = find_clear_pier_sample(
            pts,
            cum_dists,
            total_len,
            u0,
            15.0,
            clearance_r,
            piece,
            layout,
        ) else {
            continue;
        };

        let yaw = sample.tangent[1].atan2(sample.tangent[0]);
        let origin = [sample.pos[0], 0.0, sample.pos[2]];
        let seat_y = sample.pos[1] - 10.5;

        // Twin elastomeric/steel girder bearings under box-girder spine
        for s in [-1.0_f32, 1.0] {
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [0.0, seat_y - 1.4, s * road_w * 0.24],
                [7.0, 2.8, 7.0],
                IRON_BEARING,
                MAT_CAST_IRON,
            );
        }

        // Overhanging concrete hammerhead pier cap
        let cap_top = seat_y - 2.8;
        let cap_bot = (cap_top - 7.5).max(GROUND_ANCHOR_Y + 5.0);
        push_tapered_prism_y_mat(
            verts,
            idxs,
            origin,
            yaw,
            cap_bot,
            cap_top,
            12.0,
            road_w * 0.62,
            17.0,
            road_w * 0.80,
            CONCRETE_CURB,
            MAT_CONCRETE,
        );

        // Twin chamfered concrete pier columns (or single wide tapered pier for narrow roads)
        let pl_top = (GROUND_ANCHOR_Y + (cap_bot - GROUND_ANCHOR_Y) * 0.14).clamp(GROUND_ANCHOR_Y + 3.0, GROUND_ANCHOR_Y + 10.0);
        if cap_bot > pl_top + 0.5 {
            let col_off = road_w * 0.23;
            for s in [-1.0_f32, 1.0] {
                let c_origin = [
                    origin[0] + sample.normal[0] * s * col_off,
                    0.0,
                    origin[2] + sample.normal[1] * s * col_off,
                ];
                push_tapered_prism_y_mat(
                    verts,
                    idxs,
                    c_origin,
                    yaw,
                    pl_top,
                    cap_bot,
                    12.5,
                    13.5,
                    10.5,
                    11.5,
                    CONCRETE_DECK,
                    MAT_CONCRETE,
                );
            }
        }

        // Continuous foundation plinth footing
        push_oriented_box_6_mat(
            verts,
            idxs,
            origin,
            yaw,
            [0.0, 0.5 * (GROUND_ANCHOR_Y + pl_top), 0.0],
            [18.0, (pl_top - GROUND_ANCHOR_Y).max(3.0), road_w * 0.78],
            CONCRETE_SOFFIT,
            MAT_CONCRETE,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_road_abutments(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    piece: &TrackPiece,
    layout: &Layout,
    pts: &[[f32; 3]],
    road_w: f32,
    start_bridged: bool,
    end_bridged: bool,
) {
    let half_rw = road_w * 0.5;
    let clearance_r = half_rw + 22.0;
    let n = pts.len();
    let ends: [(bool, [f32; 3], [f32; 2]); 2] = [
        (start_bridged, pts[0], seg_unit_dir(pts[1], pts[0])),
        (end_bridged, pts[n - 1], seg_unit_dir(pts[n - 2], pts[n - 1])),
    ];

    for (neighbor_bridged, pos, out_dir) in ends {
        if neighbor_bridged || pos[1] < 15.0 {
            continue;
        }
        let check_x = pos[0] + out_dir[0] * 10.0;
        let check_z = pos[2] + out_dir[1] * 10.0;
        if !is_support_footprint_clear(check_x, check_z, pos[1], clearance_r, piece, layout) {
            continue;
        }

        let yaw = out_dir[1].atan2(out_dir[0]);
        let origin = [pos[0], 0.0, pos[2]];
        let seat_y = (pos[1] - 10.5).max(GROUND_ANCHOR_Y + 6.0);
        let wall_w = road_w + 15.0;

        push_tapered_prism_y_mat(
            verts,
            idxs,
            origin,
            yaw,
            GROUND_ANCHOR_Y + 4.0,
            seat_y,
            20.0,
            wall_w + 3.0,
            16.0,
            wall_w,
            CONCRETE_SOFFIT,
            MAT_CONCRETE,
        );
        // Curb flanking end blocks
        for s in [-1.0_f32, 1.0] {
            push_oriented_box_6_mat(
                verts,
                idxs,
                origin,
                yaw,
                [6.0, seat_y + (pos[1] - seat_y + 11.0) * 0.5, s * (half_rw + 5.5)],
                [10.0, pos[1] - seat_y + 11.0, 5.0],
                CONCRETE_CURB,
                MAT_CONCRETE,
            );
        }
        push_oriented_box_6_mat(
            verts,
            idxs,
            origin,
            yaw,
            [2.0, GROUND_ANCHOR_Y + 2.5, 0.0],
            [22.0, 5.0, wall_w + 6.0],
            CONCRETE_SOFFIT,
            MAT_CONCRETE,
        );

        push_wing_walls(
            verts,
            idxs,
            origin,
            yaw,
            8.0,
            half_rw + 6.5,
            24.0,
            4.2,
            pos[1] + 8.0,
            GROUND_ANCHOR_Y + 8.0,
            CONCRETE_SOFFIT,
            CONCRETE_CURB,
            MAT_CONCRETE,
        );
    }
}

/// Generates symmetric 30°-splayed abutment wing walls with sloped coping stones.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_wing_walls(
    verts: &mut Vec<Vertex3D>,
    idxs: &mut Vec<u32>,
    origin: [f32; 3],
    yaw: f32,
    start_x: f32,
    start_z_abs: f32,
    length: f32,
    thick: f32,
    top_y_near: f32,
    top_y_far: f32,
    wall_col: [f32; 3],
    cap_col: [f32; 3],
    mat: [f32; 3],
) {
    let steps = 4;
    let splay_rad = 30.0_f32.to_radians();
    let dx = splay_rad.cos();
    let dz = splay_rad.sin();

    for s in [-1.0_f32, 1.0] {
        let wall_yaw = yaw + s * splay_rad;
        for step in 0..steps {
            let u0 = step as f32 / steps as f32;
            let u1 = (step + 1) as f32 / steps as f32;
            let u_mid = 0.5 * (u0 + u1);
            let seg_len = length / steps as f32 + 1.0;
            let top_y = top_y_near + (top_y_far - top_y_near) * u_mid;
            let h = (top_y - GROUND_ANCHOR_Y).max(3.0);

            let lx = start_x + dx * (u_mid * length);
            let lz = s * (start_z_abs + dz * (u_mid * length));
            let (wx, wz) = rot_xz(lx, lz, yaw);
            let seg_origin = [origin[0] + wx, 0.0, origin[2] + wz];

            push_oriented_box_6_mat(
                verts,
                idxs,
                seg_origin,
                wall_yaw,
                [0.0, GROUND_ANCHOR_Y + h * 0.5, 0.0],
                [seg_len, h, thick],
                wall_col,
                mat,
            );
            push_oriented_box_6_mat(
                verts,
                idxs,
                seg_origin,
                wall_yaw,
                [0.0, top_y + 0.7, 0.0],
                [seg_len + 0.4, 1.4, thick + 1.2],
                cap_col,
                mat,
            );
        }
    }
}

// ============================================================================
//  Continuity & Layer-Aware Collision Avoidance
// ============================================================================

pub(super) fn pier_target_fractions(total_len: f32, sc: f32) -> Vec<f32> {
    if total_len <= 250.0 * sc {
        vec![0.5]
    } else {
        let count = ((total_len / (165.0 * sc)).floor() as usize).max(2);
        (0..count).map(|i| (i + 1) as f32 / (count + 1) as f32).collect()
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn find_clear_pier_sample(
    pts: &[[f32; 3]],
    cum_dists: &[f32],
    total_len: f32,
    u0: f32,
    min_seat_clearance: f32,
    clearance_r: f32,
    piece: &TrackPiece,
    layout: &Layout,
) -> Option<PathSample> {
    for du in [0.0_f32, -0.10, 0.10, -0.18, 0.18] {
        let u = (u0 + du).clamp(0.14, 0.86);
        let smp = sample_polyline_at_dist(pts, cum_dists, u * total_len);
        if smp.pos[1] - 13.4 < min_seat_clearance {
            continue;
        }
        if is_support_footprint_clear(smp.pos[0], smp.pos[2], smp.pos[1], clearance_r, piece, layout) {
            return Some(smp);
        }
    }
    None
}

pub(super) fn piece_layer_info(piece: &TrackPiece, layout: &Layout) -> (f32, bool) {
    let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
    if let Some(layers) = &layout.layers {
        if let Some(layer) = layers.iter().find(|l| l.id == lid) {
            return (layer.height_mm.unwrap_or(0.0) as f32, layer.visible);
        }
    }
    (0.0, true)
}

pub(super) fn neighbor_piece_at<'a>(piece: &TrackPiece, conn_idx: usize, layout: &'a Layout) -> Option<&'a TrackPiece> {
    let conn = piece.connectors.get(conn_idx)?;
    let r = conn.connected_to.as_ref()?;
    layout.pieces.iter().find(|p| p.id == r.piece_id)
}

pub(super) fn is_bridge_neighbor(piece: &TrackPiece, conn_idx: usize, layout: &Layout) -> bool {
    let Some(nb) = neighbor_piece_at(piece, conn_idx, layout) else {
        return false;
    };
    let (_, visible) = piece_layer_info(nb, layout);
    visible && nb.has_bridge == Some(true)
}

pub(super) fn is_straight_bridge_neighbor(piece: &TrackPiece, conn_idx: usize, layout: &Layout) -> bool {
    let Some(nb) = neighbor_piece_at(piece, conn_idx, layout) else {
        return false;
    };
    let (_, visible) = piece_layer_info(nb, layout);
    if !visible || nb.has_bridge != Some(true) || nb.connectors.len() != 2 {
        return false;
    }
    let a0 = nb.connectors[0].angle.to_radians();
    let a1 = nb.connectors[1].angle.to_radians();
    let dot = a0.cos() * a1.cos() + a0.sin() * a1.sin();
    dot < -0.995
}

pub(super) fn is_straight_truss_piece(
    piece: &TrackPiece,
    entry: Option<&CatalogEntry>,
    pts: &[[f32; 3]],
    total_len: f32,
    sc: f32,
) -> bool {
    if piece.connectors.len() != 2 || total_len < 105.0 * sc || pts.len() < 2 {
        return false;
    }
    let kind = entry.map(|e| e.kind.as_str()).unwrap_or("straight");
    if kind != "straight" {
        return false;
    }
    let a = pts[0];
    let b = pts[pts.len() - 1];
    let mid = pts[pts.len() / 2];
    point_to_segment_dist_xz(mid[0], mid[2], a[0], a[2], b[0], b[2]) < 1.0
}

pub(super) fn is_support_footprint_clear(
    px: f32,
    pz: f32,
    deck_y: f32,
    clearance_r: f32,
    piece: &TrackPiece,
    layout: &Layout,
) -> bool {
    for other in &layout.pieces {
        if other.id == piece.id {
            continue;
        }
        let (lh, visible) = piece_layer_info(other, layout);
        if !visible {
            continue;
        }
        let other_min_e = other
            .elevation
            .map(|e| e.start.min(e.end))
            .unwrap_or(0.0) as f32;
        if lh + other_min_e >= deck_y - 8.0 {
            continue;
        }

        // `other.connectors` are already world coordinates (x, y) -> world (x, z).
        let c = &other.connectors;
        let min_dist = match c.len() {
            0 => (other.transform.x as f32 - px).hypot(other.transform.y as f32 - pz),
            1 => {
                let d_tr = (other.transform.x as f32 - px).hypot(other.transform.y as f32 - pz);
                let d_c0 = (c[0].x as f32 - px).hypot(c[0].y as f32 - pz);
                d_tr.min(d_c0)
            }
            2 => {
                let arc = sample_arc_between_connectors(
                    c[0].x, c[0].y, c[0].angle, c[1].x, c[1].y, c[1].angle, 6,
                );
                arc.windows(2)
                    .map(|w| {
                        point_to_segment_dist_xz(
                            px,
                            pz,
                            w[0].x as f32,
                            w[0].y as f32,
                            w[1].x as f32,
                            w[1].y as f32,
                        )
                    })
                    .fold(f32::INFINITY, f32::min)
            }
            _ => {
                let mut md = f32::INFINITY;
                for ci in &c[1..] {
                    md = md.min(point_to_segment_dist_xz(
                        px,
                        pz,
                        c[0].x as f32,
                        c[0].y as f32,
                        ci.x as f32,
                        ci.y as f32,
                    ));
                }
                if c.len() >= 3 {
                    md = md.min(point_to_segment_dist_xz(
                        px,
                        pz,
                        c[1].x as f32,
                        c[1].y as f32,
                        c[2].x as f32,
                        c[2].y as f32,
                    ));
                }
                md
            }
        };

        if min_dist < clearance_r {
            return false;
        }
    }
    true
}

pub(super) fn point_to_segment_dist_xz(px: f32, pz: f32, ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    let abx = bx - ax;
    let abz = bz - az;
    let ab_len_sq = abx * abx + abz * abz;
    if ab_len_sq <= 1e-6 {
        return (px - ax).hypot(pz - az);
    }
    let t = (((px - ax) * abx + (pz - az) * abz) / ab_len_sq).clamp(0.0, 1.0);
    let qx = ax + t * abx;
    let qz = az + t * abz;
    (px - qx).hypot(pz - qz)
}

// ============================================================================
//  Polyline Sampling & Low-Level 6-Faced 3D Primitives
// ============================================================================

#[derive(Clone)]
pub(super) struct PathSample {
    pub(super) pos: [f32; 3],
    pub(super) tangent: [f32; 2],
    pub(super) normal: [f32; 2],
}

pub(super) fn seg_unit_dir(a: [f32; 3], b: [f32; 3]) -> [f32; 2] {
    let dx = b[0] - a[0];
    let dz = b[2] - a[2];
    let len = dx.hypot(dz).max(1e-5);
    [dx / len, dz / len]
}

pub(super) fn cumulative_xz_dists(pts: &[[f32; 3]]) -> Vec<f32> {
    let mut cum = Vec::with_capacity(pts.len());
    let mut acc = 0.0_f32;
    cum.push(0.0);
    for w in pts.windows(2) {
        acc += (w[1][0] - w[0][0]).hypot(w[1][2] - w[0][2]);
        cum.push(acc);
    }
    cum
}

