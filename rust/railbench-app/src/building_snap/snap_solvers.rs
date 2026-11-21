use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn snap_model_to_turntable_inner(
    cand: Transform,
    model_h: f64,
    pieces: &[TrackPiece],
    layer_heights: &HashMap<String, f64>,
    catalogs: &[Catalog],
    radial_tol: f64,
    angle_tol: f64,
    model_depth: f64,
    require_same_height: bool,
) -> Option<(Transform, Vec2, bool)> {
    let mut best_same: Option<(f64, Transform, Vec2)> = None;
    let mut best_diff: Option<(f64, Transform, Vec2)> = None;

    for p in pieces {
        let entry = find_entry(catalogs, &p.catalog_id);
        let is_tt = entry.map_or(false, |e| e.kind == "turntable") || p.connectors.len() >= 6;
        if !is_tt {
            continue;
        }
        let p_lh = layer_height_from_map(layer_heights, p.layer_id.as_deref());
        let tt_h = piece_connector_height(p, 0, p_lh);
        let same_height = (model_h - tt_h).abs() <= SNAP_HEIGHT_TOL_MM;
        if require_same_height && !same_height {
            continue;
        }

        let tt_radius = entry
            .and_then(|e| e.radius.or_else(|| e.length.map(|l| l * 0.5)))
            .unwrap_or_else(|| {
                p.connectors
                    .first()
                    .map(|c| (c.x - p.transform.x).hypot(c.y - p.transform.y))
                    .unwrap_or(189.0)
            });
        let target_dist = tt_radius + 6.0 + model_depth * 0.5;

        let dx = cand.x - p.transform.x;
        let dy = cand.y - p.transform.y;
        let current_dist = dx.hypot(dy);
        if (current_dist - target_dist).abs() > radial_tol {
            continue;
        }

        let current_angle_deg = norm_angle_deg(dy.atan2(dx).to_degrees());
        let mut exit_angles = Vec::new();
        if let Some(lcs) = entry.and_then(|e| e.local_connectors.as_ref()).filter(|v| !v.is_empty()) {
            for lc in lcs {
                let base_angle = lc.y.atan2(lc.x).to_degrees();
                exit_angles.push(norm_angle_deg(base_angle + p.transform.rotation));
            }
        } else {
            let step = entry.and_then(|e| e.angle).unwrap_or(12.0).max(5.0);
            let mut a = 0.0;
            while a < 360.0 - 0.1 {
                exit_angles.push(norm_angle_deg(a + p.transform.rotation));
                a += step;
            }
        }

        if exit_angles.is_empty() {
            continue;
        }

        let mut closest_angle = exit_angles[0];
        let mut min_angle_diff = f64::INFINITY;
        for ea in exit_angles {
            let diff = angle_diff_deg(current_angle_deg, ea);
            if diff < min_angle_diff {
                min_angle_diff = diff;
                closest_angle = ea;
            }
        }
        if min_angle_diff > angle_tol {
            continue;
        }

        let rad = deg2rad(closest_angle);
        let snap_x = (p.transform.x + target_dist * rad.cos()).round();
        let snap_y = (p.transform.y + target_dist * rad.sin()).round();
        let snap_rot = norm_angle_deg(closest_angle - 90.0);
        let dist_to_cand = (snap_x - cand.x).hypot(snap_y - cand.y);

        let tr = Transform {
            x: snap_x,
            y: snap_y,
            rotation: snap_rot,
        };
        let edge = Vec2 {
            x: p.transform.x + tt_radius * rad.cos(),
            y: p.transform.y + tt_radius * rad.sin(),
        };
        if same_height {
            if best_same.as_ref().map_or(true, |(bd, _, _)| dist_to_cand < *bd) {
                best_same = Some((dist_to_cand, tr, edge));
            }
        } else if best_diff.as_ref().map_or(true, |(bd, _, _)| dist_to_cand < *bd) {
            best_diff = Some((dist_to_cand, tr, edge));
        }
    }

    best_same
        .map(|(_, tr, edge)| (tr, edge, true))
        .or_else(|| best_diff.map(|(_, tr, edge)| (tr, edge, false)))
}

#[allow(dead_code)]
pub fn snap_engine_shed_to_track(
    cand: Transform,
    pieces: &[TrackPiece],
    catalogs: &[Catalog],
    snap_tol: f64,
    angle_tol: f64,
    shed_len: f64,
) -> Option<(Transform, Vec2)> {
    snap_engine_shed_to_track_inner(
        cand,
        0.0,
        pieces,
        &HashMap::new(),
        catalogs,
        snap_tol,
        angle_tol,
        shed_len,
        true,
    )
    .map(|(tr, pt, _)| (tr, pt))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn snap_engine_shed_to_track_inner(
    cand: Transform,
    model_h: f64,
    pieces: &[TrackPiece],
    layer_heights: &HashMap<String, f64>,
    catalogs: &[Catalog],
    snap_tol: f64,
    angle_tol: f64,
    shed_len: f64,
    require_same_height: bool,
) -> Option<(Transform, Vec2, bool)> {
    let rp = rotate(Vec2 { x: -shed_len * 0.5, y: 0.0 }, cand.rotation);
    let cand_door = Vec2 {
        x: cand.x + rp.x,
        y: cand.y + rp.y,
    };

    let mut best_same: Option<(f64, Transform, Vec2)> = None;
    let mut best_diff: Option<(f64, Transform, Vec2)> = None;
    for p in pieces {
        if is_road_id(catalogs, &p.catalog_id) {
            continue;
        }
        let p_lh = layer_height_from_map(layer_heights, p.layer_id.as_deref());
        for (ci, c) in p.connectors.iter().enumerate() {
            let c_h = piece_connector_height(p, ci, p_lh);
            let same_height = (model_h - c_h).abs() <= SNAP_HEIGHT_TOL_MM;
            if require_same_height && !same_height {
                continue;
            }
            let d = (cand_door.x - c.x).hypot(cand_door.y - c.y);
            if d > snap_tol {
                continue;
            }
            let diff_a = angle_diff_deg(cand.rotation, c.angle);
            let diff_b = angle_diff_deg(cand.rotation, c.angle + 180.0);
            let use_b = diff_b < diff_a;
            let angle_diff = if use_b { diff_b } else { diff_a };
            let target_angle = norm_angle_deg(if use_b { c.angle + 180.0 } else { c.angle });

            let max_allowed = if d <= 25.0 { 180.0 } else { angle_tol };
            if angle_diff > max_allowed {
                continue;
            }

            let score = d + (angle_diff / 180.0) * 20.0;
            let rad = deg2rad(target_angle);
            let snapped_x = ((c.x + (shed_len * 0.5) * rad.cos()) * 10.0).round() / 10.0;
            let snapped_y = ((c.y + (shed_len * 0.5) * rad.sin()) * 10.0).round() / 10.0;
            let tr = Transform {
                x: snapped_x,
                y: snapped_y,
                rotation: target_angle,
            };
            let pt = Vec2 { x: c.x, y: c.y };
            if same_height {
                if best_same.as_ref().map_or(true, |(bs, _, _)| score < *bs) {
                    best_same = Some((score, tr, pt));
                }
            } else if best_diff.as_ref().map_or(true, |(bs, _, _)| score < *bs) {
                best_diff = Some((score, tr, pt));
            }
        }
    }

    best_same
        .map(|(_, tr, pt)| (tr, pt, true))
        .or_else(|| best_diff.map(|(_, tr, pt)| (tr, pt, false)))
}

#[allow(dead_code)]
pub fn snap_trainhall_to_track(
    cand: Transform,
    pieces: &[TrackPiece],
    catalogs: &[Catalog],
    snap_tol: f64,
    angle_tol: f64,
) -> Option<(Transform, Vec2)> {
    snap_trainhall_to_track_inner(
        cand,
        0.0,
        pieces,
        &HashMap::new(),
        catalogs,
        snap_tol,
        angle_tol,
        true,
    )
    .map(|(tr, pt, _)| (tr, pt))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn snap_trainhall_to_track_inner(
    cand: Transform,
    model_h: f64,
    pieces: &[TrackPiece],
    layer_heights: &HashMap<String, f64>,
    catalogs: &[Catalog],
    snap_tol: f64,
    angle_tol: f64,
    require_same_height: bool,
) -> Option<(Transform, Vec2, bool)> {
    let fake_model = Model3D {
        id: "__temp__".into(),
        name: "Ringlokschuppen".into(),
        category: "railway".into(),
        transform: cand,
        elevation: None,
        scale: Some(1.0),
        layer_id: None,
        width: 320.0,
        depth: 310.0,
        height: 115.0,
        model_type: "builtin".into(),
        builtin_id: Some("turntable-trainhall".into()),
        mountain_style: None,
        mountain_path: None,
        ground_terrain_type: None,
        file_data: None,
        file_name: None,
    };

    let doors = get_building_door_connectors(&fake_model);
    let mut best_same: Option<(f64, Transform, Vec2)> = None;
    let mut best_diff: Option<(f64, Transform, Vec2)> = None;

    for door in &doors {
        for p in pieces {
            if is_road_id(catalogs, &p.catalog_id) {
                continue;
            }
            let p_lh = layer_height_from_map(layer_heights, p.layer_id.as_deref());
            for (ci, c) in p.connectors.iter().enumerate() {
                let c_h = piece_connector_height(p, ci, p_lh);
                let same_height = (model_h - c_h).abs() <= SNAP_HEIGHT_TOL_MM;
                if require_same_height && !same_height {
                    continue;
                }
                let d = (door.x - c.x).hypot(door.y - c.y);
                if d > snap_tol {
                    continue;
                }
                let diff_in = angle_diff_deg(door.inward_angle, c.angle);
                let diff_out = angle_diff_deg(door.inward_angle, c.angle + 180.0);
                let use_out = diff_out < diff_in;
                let angle_diff = if use_out { diff_out } else { diff_in };
                let effective_conn_angle = norm_angle_deg(if use_out { c.angle + 180.0 } else { c.angle });

                let max_allowed = if d <= 25.0 { 180.0 } else { angle_tol };
                if angle_diff > max_allowed {
                    continue;
                }

                let score = d + (angle_diff / 180.0) * 20.0;
                let delta_angle = signed_angle_diff_deg(effective_conn_angle, door.inward_angle);
                let snapped_rot = norm_angle_deg(cand.rotation + delta_angle);
                let local_stall = match door.stall_index {
                    0 => Vec2 { x: -40.54, y: -159.26 },
                    1 => Vec2 { x: 0.0, y: -155.0 },
                    _ => Vec2 { x: 40.54, y: -159.26 },
                };
                let rot_off = rotate(local_stall, snapped_rot);
                let snapped_x = ((c.x - rot_off.x) * 10.0).round() / 10.0;
                let snapped_y = ((c.y - rot_off.y) * 10.0).round() / 10.0;
                let tr = Transform {
                    x: snapped_x,
                    y: snapped_y,
                    rotation: snapped_rot,
                };
                let pt = Vec2 { x: c.x, y: c.y };
                if same_height {
                    if best_same.as_ref().map_or(true, |(bs, _, _)| score < *bs) {
                        best_same = Some((score, tr, pt));
                    }
                } else if best_diff.as_ref().map_or(true, |(bs, _, _)| score < *bs) {
                    best_diff = Some((score, tr, pt));
                }
            }
        }
    }

    best_same
        .map(|(_, tr, pt)| (tr, pt, true))
        .or_else(|| best_diff.map(|(_, tr, pt)| (tr, pt, false)))
}

pub fn find_best_track_to_building_snap(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> Option<BuildingTrackSnapCandidate> {
    find_best_track_to_building_snap_internal(layout, catalogs, selected, max_dist, true)
}

pub fn find_best_track_to_building_snap_any_level(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> Option<BuildingTrackSnapCandidate> {
    find_best_track_to_building_snap_internal(layout, catalogs, selected, max_dist, false)
}

pub(super) fn find_best_track_to_building_snap_internal(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
    require_same_level: bool,
) -> Option<BuildingTrackSnapCandidate> {
    let models = layout.models.as_ref()?;
    let layer_heights = build_layer_height_map(layout);
    let mut best_same: Option<(f64, BuildingTrackSnapCandidate)> = None;
    let mut best_diff: Option<(f64, BuildingTrackSnapCandidate)> = None;

    for p in &layout.pieces {
        if !selected.contains(&p.id) || is_road_id(catalogs, &p.catalog_id) {
            continue;
        }
        let p_lh = layer_height_from_map(&layer_heights, p.layer_id.as_deref());
        for (ci, c) in p.connectors.iter().enumerate() {
            let c_h = piece_connector_height(p, ci, p_lh);
            for m in models {
                if !is_layer_visible(layout, m.layer_id.as_deref()) {
                    continue;
                }
                let m_h = layer_height_from_map(&layer_heights, m.layer_id.as_deref())
                    + m.elevation.unwrap_or(0.0);
                let same_level = (c_h - m_h).abs() <= SNAP_HEIGHT_TOL_MM;
                if require_same_level && !same_level {
                    continue;
                }
                for door in get_building_door_connectors(m) {
                    let d = (c.x - door.x).hypot(c.y - door.y);
                    if d > max_dist {
                        continue;
                    }
                    let diff_outside = angle_diff_deg(c.angle, door.outward_angle + 180.0);
                    let diff_inside = angle_diff_deg(c.angle, door.outward_angle);
                    let is_inside = diff_inside < diff_outside;
                    let diff = if is_inside { diff_inside } else { diff_outside };
                    let eff_tol = if d <= 25.0 { 180.0 } else { 60.0 };
                    if diff > eff_tol {
                        continue;
                    }
                    let score = d + (diff / 180.0) * 20.0;
                    let target_angle = if is_inside {
                        norm_angle_deg(door.inward_angle + 180.0)
                    } else {
                        norm_angle_deg(door.outward_angle + 180.0)
                    };
                    let cand = BuildingTrackSnapCandidate {
                        src_x: c.x,
                        src_y: c.y,
                        src_angle: c.angle,
                        dst_x: door.x,
                        dst_y: door.y,
                        target_angle,
                        same_level,
                    };
                    if same_level {
                        if best_same.as_ref().map_or(true, |(bs, _)| score < *bs) {
                            best_same = Some((score, cand));
                        }
                    } else if best_diff.as_ref().map_or(true, |(bs, _)| score < *bs) {
                        best_diff = Some((score, cand));
                    }
                }
            }
        }
    }

    best_same.or(best_diff).map(|(_, cand)| cand)
}

pub fn snap_selected_pieces_to_buildings(
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> bool {
    let Some(cand) = find_best_track_to_building_snap(layout, catalogs, selected, max_dist) else {
        return false;
    };

    let delta_rot = norm_angle_deg(cand.target_angle - cand.src_angle);
    let pivot = Vec2 {
        x: cand.src_x,
        y: cand.src_y,
    };
    let shift_x = cand.dst_x - cand.src_x;
    let shift_y = cand.dst_y - cand.src_y;

    for p in &mut layout.pieces {
        if !selected.contains(&p.id) {
            continue;
        }
        let rel_t = rotate(
            Vec2 {
                x: p.transform.x - pivot.x,
                y: p.transform.y - pivot.y,
            },
            delta_rot,
        );
        p.transform.x = pivot.x + rel_t.x + shift_x;
        p.transform.y = pivot.y + rel_t.y + shift_y;
        p.transform.rotation = norm_angle_deg(p.transform.rotation + delta_rot);

        for c in &mut p.connectors {
            let rel_c = rotate(
                Vec2 {
                    x: c.x - pivot.x,
                    y: c.y - pivot.y,
                },
                delta_rot,
            );
            c.x = pivot.x + rel_c.x + shift_x;
            c.y = pivot.y + rel_c.y + shift_y;
            c.angle = norm_angle_deg(c.angle + delta_rot);
            c.connected_to = None;
        }
    }
    true
}

pub(super) fn find_model_snap_for_model(
    m: &Model3D,
    visible_pieces: &[TrackPiece],
    other_models: &[Model3D],
    layer_heights: &HashMap<String, f64>,
    catalogs: &[Catalog],
    require_same_height: bool,
) -> Option<(Transform, Vec2, bool)> {
    let scale = m.scale.unwrap_or(1.0);
    let model_h = layer_height_from_map(layer_heights, m.layer_id.as_deref()) + m.elevation.unwrap_or(0.0);
    match m.builtin_id.as_deref() {
        Some("turntable-trainhall") => {
            let same = snap_model_to_turntable_inner(
                m.transform,
                model_h,
                visible_pieces,
                layer_heights,
                catalogs,
                75.0,
                15.0,
                m.depth * scale,
                true,
            )
            .or_else(|| {
                snap_trainhall_to_track_inner(
                    m.transform,
                    model_h,
                    visible_pieces,
                    layer_heights,
                    catalogs,
                    50.0,
                    60.0,
                    true,
                )
            });
            if same.is_some() || require_same_height {
                same
            } else {
                snap_model_to_turntable_inner(
                    m.transform,
                    model_h,
                    visible_pieces,
                    layer_heights,
                    catalogs,
                    75.0,
                    15.0,
                    m.depth * scale,
                    false,
                )
                .or_else(|| {
                    snap_trainhall_to_track_inner(
                        m.transform,
                        model_h,
                        visible_pieces,
                        layer_heights,
                        catalogs,
                        50.0,
                        60.0,
                        false,
                    )
                })
            }
        }
        Some("engine-shed") => snap_engine_shed_to_track_inner(
            m.transform,
            model_h,
            visible_pieces,
            layer_heights,
            catalogs,
            50.0,
            75.0,
            m.width * scale,
            require_same_height,
        ),
        Some("station")
        | Some("platform")
        | Some("bahnsteig")
        | Some("coaling-station")
        | Some("water-tower")
        | Some("diesel-fuel-station") => snap_trackside_model_to_track_inner(
            m,
            model_h,
            visible_pieces,
            other_models,
            layer_heights,
            catalogs,
            65.0,
            require_same_height,
        ),
        _ => None,
    }
}

/// Snap a trackside structure (`station`, `platform` / `bahnsteig`, `coaling-station`,
/// `water-tower`, `diesel-fuel-station`) parallel alongside the nearest track piece
/// (or end-to-end with an adjacent platform/station).
#[allow(dead_code)]
pub fn snap_trackside_model_to_track(
    m: &Model3D,
    pieces: &[TrackPiece],
    other_models: &[Model3D],
    catalogs: &[Catalog],
    snap_radius: f64,
) -> Option<(Transform, Vec2)> {
    snap_trackside_model_to_track_inner(
        m,
        m.elevation.unwrap_or(0.0),
        pieces,
        other_models,
        &HashMap::new(),
        catalogs,
        snap_radius,
        true,
    )
    .map(|(tr, pt, _)| (tr, pt))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn snap_trackside_model_to_track_inner(
    m: &Model3D,
    model_h: f64,
    pieces: &[TrackPiece],
    other_models: &[Model3D],
    layer_heights: &HashMap<String, f64>,
    catalogs: &[Catalog],
    snap_radius: f64,
    require_same_height: bool,
) -> Option<(Transform, Vec2, bool)> {
    let bid = m.builtin_id.as_deref()?;
    let scale = m.scale.unwrap_or(1.0);
    let half_w = (m.width * scale) * 0.5;
    let half_d = (m.depth * scale) * 0.5;

    // 1. For `platform` / `bahnsteig` and `station`, check end-to-end chaining with an adjacent platform/station first
    let mut best_diff_chain: Option<(f64, Transform, Vec2)> = None;
    if matches!(bid, "platform" | "bahnsteig" | "station") {
        let mut best_same_chain: Option<(f64, Transform, Vec2)> = None;
        for other in other_models {
            if other.id == m.id {
                continue;
            }
            let obid = other.builtin_id.as_deref().unwrap_or("");
            if !matches!(obid, "platform" | "bahnsteig" | "station") {
                continue;
            }
            let other_h = layer_height_from_map(layer_heights, other.layer_id.as_deref())
                + other.elevation.unwrap_or(0.0);
            let same_height = (model_h - other_h).abs() <= SNAP_HEIGHT_TOL_MM;
            if require_same_height && !same_height {
                continue;
            }
            let oscale = other.scale.unwrap_or(1.0);
            let ohw = (other.width * oscale) * 0.5;
            let orad = other.transform.rotation.to_radians();
            let ux = orad.cos();
            let uy = orad.sin();
            for side in [-1.0_f64, 1.0] {
                let joint_x = other.transform.x + ux * (side * ohw);
                let joint_y = other.transform.y + uy * (side * ohw);
                let target_cx = other.transform.x + ux * (side * (ohw + half_w));
                let target_cy = other.transform.y + uy * (side * (ohw + half_w));
                let dist = (m.transform.x - target_cx).hypot(m.transform.y - target_cy);
                if dist <= snap_radius {
                    // Preserve the user's facing orientation (same or +180° for symmetric platform)
                    let cand_rot = if matches!(bid, "platform" | "bahnsteig")
                        && angle_diff_deg(m.transform.rotation, other.transform.rotation + 180.0)
                            < angle_diff_deg(m.transform.rotation, other.transform.rotation)
                    {
                        norm_angle_deg(other.transform.rotation + 180.0)
                    } else {
                        norm_angle_deg(other.transform.rotation)
                    };
                    let cand_entry = (
                        dist,
                        Transform {
                            x: target_cx,
                            y: target_cy,
                            rotation: cand_rot,
                        },
                        Vec2 {
                            x: joint_x,
                            y: joint_y,
                        },
                    );
                    if same_height {
                        if best_same_chain.as_ref().map_or(true, |b| dist < b.0) {
                            best_same_chain = Some(cand_entry);
                        }
                    } else if best_diff_chain.as_ref().map_or(true, |b| dist < b.0) {
                        best_diff_chain = Some(cand_entry);
                    }
                }
            }
        }
        if let Some((_d, tr, pt)) = best_same_chain {
            return Some((tr, pt, true));
        }
    }

    // 2. Determine lateral clearance offset and local trackside axis for each model type:
    // - `station`: platform & canopy on +Z (+Y in 2D) -> offset = half_d + 18.0 * scale
    // - `platform` / `bahnsteig`: island platform with curbs on BOTH +Z and -Z -> offset = half_d + 17.5 * scale
    // - `coaling-station`: hopper chute & water crane on +Z -> offset = half_d + 17.0 * scale
    // - `diesel-fuel-station`: fueling island & dispensers on +Z -> offset = half_d + 16.5 * scale
    // - `water-tower`: water crane spout extends on +X -> offset = half_w + 17.5 * scale (track runs along local Z, i.e., rot + 90°)
    let (base_offset, allow_both_sides, local_axis_offset_deg) = match bid {
        "station" => (half_d + 18.0 * scale, false, 0.0),
        "platform" | "bahnsteig" => (half_d + 17.5 * scale, true, 0.0),
        "coaling-station" => (half_d + 17.0 * scale, false, 0.0),
        "diesel-fuel-station" => (half_d + 16.5 * scale, false, 0.0),
        "water-tower" => (half_w + 17.5 * scale, false, -90.0),
        _ => return None,
    };

    let mut best_same_track: Option<(f64, Transform, Vec2)> = None;
    let mut best_diff_track: Option<(f64, Transform, Vec2)> = None;

    for piece in pieces {
        if crate::catalog::is_road_id(catalogs, &piece.catalog_id) || piece.connectors.len() < 2 {
            continue;
        }
        let entry = find_entry(catalogs, &piece.catalog_id);
        if entry.map_or(false, |e| e.kind == "turntable") {
            continue;
        }
        let p_lh = layer_height_from_map(layer_heights, piece.layer_id.as_deref());
        let h0 = piece_connector_height(piece, 0, p_lh);
        let gauge_scale = entry.and_then(|e| e.gauge).unwrap_or(16.5) / 16.5;
        let lat_offset = base_offset + (gauge_scale - 1.0) * 9.0;

        // Consider each connector-to-connector chord (main track and any diverging branch)
        let c0 = &piece.connectors[0];
        for (c1_idx, c1) in piece.connectors.iter().enumerate().skip(1) {
            let dx = c1.x - c0.x;
            let dy = c1.y - c0.y;
            let seg_len = dx.hypot(dy);
            if seg_len < 10.0 {
                continue;
            }
            let ux = dx / seg_len;
            let uy = dy / seg_len;
            // Right-hand perpendicular normal `n = (-uy, ux)`
            let nx = -uy;
            let ny = ux;
            let track_angle = dy.atan2(dx).to_degrees();

            // Project model center onto the track segment centerline
            let rel_x = m.transform.x - c0.x;
            let rel_y = m.transform.y - c0.y;
            let along = rel_x * ux + rel_y * uy;
            let margin = half_w.max(35.0);
            if along < -margin || along > seg_len + margin {
                continue;
            }
            let clamped_along = along.clamp(0.0, seg_len);
            // Gently magnetic-snap longitudinal position to track piece midpoint if close
            let mid_along = seg_len * 0.5;
            let snapped_along = if (clamped_along - mid_along).abs() <= 18.0 {
                mid_along
            } else {
                clamped_along
            };

            let foot_x = c0.x + ux * snapped_along;
            let foot_y = c0.y + uy * snapped_along;
            let frac = (snapped_along / seg_len).clamp(0.0, 1.0);
            let h1 = piece_connector_height(piece, c1_idx, p_lh);
            let track_h = h0 + (h1 - h0) * frac;
            let same_height = (model_h - track_h).abs() <= SNAP_HEIGHT_TOL_MM;
            if require_same_height && !same_height {
                continue;
            }

            // Evaluate both sides of the track (`side = +1` along `+n` and `side = -1` along `-n`)
            for side in [1.0_f64, -1.0] {
                let target_x = foot_x + nx * (side * lat_offset);
                let target_y = foot_y + ny * (side * lat_offset);
                let dist = (m.transform.x - target_x).hypot(m.transform.y - target_y);
                if dist > snap_radius {
                    continue;
                }

                let primary_rot = if side < 0.0 {
                    norm_angle_deg(track_angle + local_axis_offset_deg)
                } else {
                    norm_angle_deg(track_angle + 180.0 + local_axis_offset_deg)
                };
                let target_rot = if allow_both_sides {
                    let alt_rot = norm_angle_deg(primary_rot + 180.0);
                    if angle_diff_deg(m.transform.rotation, alt_rot)
                        < angle_diff_deg(m.transform.rotation, primary_rot)
                    {
                        alt_rot
                    } else {
                        primary_rot
                    }
                } else {
                    primary_rot
                };

                let rot_penalty = angle_diff_deg(m.transform.rotation, target_rot) * 0.12;
                let score = dist + rot_penalty;
                let cand_entry = (
                    score,
                    Transform {
                        x: target_x,
                        y: target_y,
                        rotation: target_rot,
                    },
                    Vec2 {
                        x: foot_x,
                        y: foot_y,
                    },
                );
                if same_height {
                    if best_same_track.as_ref().map_or(true, |b| score < b.0) {
                        best_same_track = Some(cand_entry);
                    }
                } else if best_diff_track.as_ref().map_or(true, |b| score < b.0) {
                    best_diff_track = Some(cand_entry);
                }
            }
        }
    }

    best_same_track
        .map(|(_, tr, pt)| (tr, pt, true))
        .or_else(|| best_diff_chain.map(|(_, tr, pt)| (tr, pt, false)))
        .or_else(|| best_diff_track.map(|(_, tr, pt)| (tr, pt, false)))
}

