use std::collections::HashSet;

use railbench_core::geometry::{deg2rad, norm_angle_deg, rotate};
use railbench_core::types::{Catalog, Layout, Model3D, TrackPiece, Transform, Vec2};

use crate::catalog::{find_entry, is_road_id};

#[derive(Debug, Clone)]
pub struct BuildingDoorConnector {
    pub x: f64,
    pub y: f64,
    pub inward_angle: f64,
    pub outward_angle: f64,
    #[allow(dead_code)]
    pub stall_index: usize,
    #[allow(dead_code)]
    pub model_id: String,
}

#[derive(Debug, Clone)]
pub struct BuildingTrackSnapCandidate {
    pub src_x: f64,
    pub src_y: f64,
    pub src_angle: f64,
    pub dst_x: f64,
    pub dst_y: f64,
    pub target_angle: f64,
}

fn angle_diff_deg(a: f64, b: f64) -> f64 {
    let mut d = norm_angle_deg(a - b).abs();
    if d > 180.0 {
        d = 360.0 - d;
    }
    d
}

fn signed_angle_diff_deg(a: f64, b: f64) -> f64 {
    let mut d = norm_angle_deg(a - b);
    if d > 180.0 {
        d -= 360.0;
    }
    d
}

fn is_layer_visible(layout: &Layout, layer_id: Option<&str>) -> bool {
    let lid = layer_id.unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .map(|l| l.visible)
        .unwrap_or(true)
}

pub fn get_building_door_connectors(model: &Model3D) -> Vec<BuildingDoorConnector> {
    let scale = model.scale.unwrap_or(1.0);
    match model.builtin_id.as_deref() {
        Some("engine-shed") => {
            let length = model.width * scale;
            let rp = rotate(Vec2 { x: -length * 0.5, y: 0.0 }, model.transform.rotation);
            let wx = ((model.transform.x + rp.x) * 10.0).round() / 10.0;
            let wy = ((model.transform.y + rp.y) * 10.0).round() / 10.0;
            let inward = norm_angle_deg(model.transform.rotation);
            let outward = norm_angle_deg(model.transform.rotation + 180.0);
            vec![BuildingDoorConnector {
                x: wx,
                y: wy,
                inward_angle: inward,
                outward_angle: outward,
                stall_index: 0,
                model_id: model.id.clone(),
            }]
        }
        Some("turntable-trainhall") => {
            let stalls = [
                (-40.54_f64, -159.35_f64, 78.0_f64, 258.0_f64),
                (0.0_f64, -155.0_f64, 90.0_f64, 270.0_f64),
                (40.54_f64, -159.35_f64, 102.0_f64, 282.0_f64),
            ];
            stalls
                .iter()
                .enumerate()
                .map(|(idx, &(sx, sy, in_a, out_a))| {
                    let rp = rotate(
                        Vec2 {
                            x: sx * scale,
                            y: sy * scale,
                        },
                        model.transform.rotation,
                    );
                    BuildingDoorConnector {
                        x: ((model.transform.x + rp.x) * 10.0).round() / 10.0,
                        y: ((model.transform.y + rp.y) * 10.0).round() / 10.0,
                        inward_angle: norm_angle_deg(in_a + model.transform.rotation),
                        outward_angle: norm_angle_deg(out_a + model.transform.rotation),
                        stall_index: idx,
                        model_id: model.id.clone(),
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

pub fn is_connector_connected_to_building(x: f64, y: f64, models: &[Model3D], tolerance: f64) -> bool {
    for m in models {
        for d in get_building_door_connectors(m) {
            if (x - d.x).hypot(y - d.y) <= tolerance {
                return true;
            }
        }
    }
    false
}

pub fn snap_model_to_turntable(
    cand: Transform,
    pieces: &[TrackPiece],
    catalogs: &[Catalog],
    radial_tol: f64,
    angle_tol: f64,
    model_depth: f64,
) -> Option<(Transform, Vec2)> {
    let mut best: Option<(f64, Transform, Vec2)> = None;

    for p in pieces {
        let entry = find_entry(catalogs, &p.catalog_id);
        let is_tt = entry.map_or(false, |e| e.kind == "turntable") || p.connectors.len() >= 6;
        if !is_tt {
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

        if best.as_ref().map_or(true, |(bd, _, _)| dist_to_cand < *bd) {
            best = Some((
                dist_to_cand,
                Transform {
                    x: snap_x,
                    y: snap_y,
                    rotation: snap_rot,
                },
                Vec2 {
                    x: p.transform.x + tt_radius * rad.cos(),
                    y: p.transform.y + tt_radius * rad.sin(),
                },
            ));
        }
    }

    best.map(|(_, tr, edge)| (tr, edge))
}

pub fn snap_engine_shed_to_track(
    cand: Transform,
    pieces: &[TrackPiece],
    catalogs: &[Catalog],
    snap_tol: f64,
    angle_tol: f64,
    shed_len: f64,
) -> Option<(Transform, Vec2)> {
    let rp = rotate(Vec2 { x: -shed_len * 0.5, y: 0.0 }, cand.rotation);
    let cand_door = Vec2 {
        x: cand.x + rp.x,
        y: cand.y + rp.y,
    };

    let mut best: Option<(f64, Transform, Vec2)> = None;
    for p in pieces {
        if is_road_id(catalogs, &p.catalog_id) {
            continue;
        }
        for c in &p.connectors {
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
            if best.as_ref().map_or(true, |(bs, _, _)| score < *bs) {
                let rad = deg2rad(target_angle);
                let snapped_x = ((c.x + (shed_len * 0.5) * rad.cos()) * 10.0).round() / 10.0;
                let snapped_y = ((c.y + (shed_len * 0.5) * rad.sin()) * 10.0).round() / 10.0;
                best = Some((
                    score,
                    Transform {
                        x: snapped_x,
                        y: snapped_y,
                        rotation: target_angle,
                    },
                    Vec2 { x: c.x, y: c.y },
                ));
            }
        }
    }

    best.map(|(_, tr, pt)| (tr, pt))
}

pub fn snap_trainhall_to_track(
    cand: Transform,
    pieces: &[TrackPiece],
    catalogs: &[Catalog],
    snap_tol: f64,
    angle_tol: f64,
) -> Option<(Transform, Vec2)> {
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
    let mut best: Option<(f64, Transform, Vec2)> = None;

    for door in &doors {
        for p in pieces {
            if is_road_id(catalogs, &p.catalog_id) {
                continue;
            }
            for c in &p.connectors {
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
                if best.as_ref().map_or(true, |(bs, _, _)| score < *bs) {
                    let delta_angle = signed_angle_diff_deg(effective_conn_angle, door.inward_angle);
                    let snapped_rot = norm_angle_deg(cand.rotation + delta_angle);
                    let local_stall = match door.stall_index {
                        0 => Vec2 { x: -40.54, y: -159.35 },
                        1 => Vec2 { x: 0.0, y: -155.0 },
                        _ => Vec2 { x: 40.54, y: -159.35 },
                    };
                    let rot_off = rotate(local_stall, snapped_rot);
                    let snapped_x = ((c.x - rot_off.x) * 10.0).round() / 10.0;
                    let snapped_y = ((c.y - rot_off.y) * 10.0).round() / 10.0;
                    best = Some((
                        score,
                        Transform {
                            x: snapped_x,
                            y: snapped_y,
                            rotation: snapped_rot,
                        },
                        Vec2 { x: c.x, y: c.y },
                    ));
                }
            }
        }
    }

    best.map(|(_, tr, pt)| (tr, pt))
}

pub fn find_best_track_to_building_snap(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    max_dist: f64,
) -> Option<BuildingTrackSnapCandidate> {
    let models = layout.models.as_ref()?;
    let mut best: Option<(f64, BuildingTrackSnapCandidate)> = None;

    for p in &layout.pieces {
        if !selected.contains(&p.id) || is_road_id(catalogs, &p.catalog_id) {
            continue;
        }
        for c in &p.connectors {
            for m in models {
                if !is_layer_visible(layout, m.layer_id.as_deref()) {
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
                    if best.as_ref().map_or(true, |(bs, _)| score < *bs) {
                        let target_angle = if is_inside {
                            norm_angle_deg(door.inward_angle + 180.0)
                        } else {
                            norm_angle_deg(door.outward_angle + 180.0)
                        };
                        best = Some((
                            score,
                            BuildingTrackSnapCandidate {
                                src_x: c.x,
                                src_y: c.y,
                                src_angle: c.angle,
                                dst_x: door.x,
                                dst_y: door.y,
                                target_angle,
                            },
                        ));
                    }
                }
            }
        }
    }

    best.map(|(_, cand)| cand)
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

fn find_model_snap_for_model(
    m: &Model3D,
    visible_pieces: &[TrackPiece],
    catalogs: &[Catalog],
) -> Option<(Transform, Vec2)> {
    let scale = m.scale.unwrap_or(1.0);
    match m.builtin_id.as_deref() {
        Some("turntable-trainhall") => {
            snap_model_to_turntable(m.transform, visible_pieces, catalogs, 75.0, 15.0, m.depth * scale)
                .or_else(|| snap_trainhall_to_track(m.transform, visible_pieces, catalogs, 50.0, 60.0))
        }
        Some("engine-shed") => snap_engine_shed_to_track(
            m.transform,
            visible_pieces,
            catalogs,
            50.0,
            75.0,
            m.width * scale,
        ),
        _ => None,
    }
}

pub fn preview_selected_model_snap(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
) -> Option<(Vec2, Vec2)> {
    let models = layout.models.as_ref()?;
    let visible_pieces: Vec<TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| is_layer_visible(layout, p.layer_id.as_deref()))
        .cloned()
        .collect();

    for m in models {
        if !selected.contains(&m.id) {
            continue;
        }
        if let Some((_snapped_tr, target_pt)) = find_model_snap_for_model(m, &visible_pieces, catalogs) {
            let door_src = get_building_door_connectors(m)
                .first()
                .map(|d| Vec2 { x: d.x, y: d.y })
                .unwrap_or(Vec2 {
                    x: m.transform.x,
                    y: m.transform.y,
                });
            return Some((door_src, target_pt));
        }
    }
    None
}

pub fn snap_selected_models(layout: &mut Layout, catalogs: &[Catalog], selected: &HashSet<String>) -> bool {
    let visible_pieces: Vec<TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| is_layer_visible(layout, p.layer_id.as_deref()))
        .cloned()
        .collect();
    let Some(models) = &mut layout.models else {
        return false;
    };

    let mut snapped_any = false;
    for m in models.iter_mut() {
        if !selected.contains(&m.id) {
            continue;
        }
        if let Some((snapped_tr, _)) = find_model_snap_for_model(m, &visible_pieces, catalogs) {
            m.transform = snapped_tr;
            snapped_any = true;
        }
    }
    snapped_any
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};

    fn make_test_shed(x: f64, y: f64, rotation: f64) -> Model3D {
        Model3D {
            id: "shed-1".into(),
            name: "Lokschuppen".into(),
            category: "railway".into(),
            transform: Transform { x, y, rotation },
            elevation: Some(0.0),
            scale: Some(1.0),
            layer_id: Some("layer-1".into()),
            width: 240.0,
            depth: 110.0,
            height: 95.0,
            model_type: "builtin".into(),
            builtin_id: Some("engine-shed".into()),
            mountain_style: None,
            mountain_path: None,
            ground_terrain_type: None,
            file_data: None,
            file_name: None,
        }
    }

    #[test]
    fn engine_shed_snaps_to_open_track_connector() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24188").unwrap();
        let piece = create_piece("p1".into(), entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        // Connector 1 is at (188.3, 0.0) facing 0 deg; shed door is at -120 -> center ~308.3
        let cand = Transform {
            x: 320.0,
            y: 12.0,
            rotation: 8.0,
        };
        let res = snap_engine_shed_to_track(cand, &[piece], &catalogs, 50.0, 75.0, 240.0);
        let (tr, pt) = res.expect("engine shed should snap to track connector");
        assert!((pt.x - 188.3).abs() < 0.2);
        assert!((tr.x - 308.3).abs() < 0.3);
        assert!((tr.y - 0.0).abs() < 0.3);
        assert!(tr.rotation.abs() < 0.5 || (tr.rotation - 360.0).abs() < 0.5);
    }

    #[test]
    fn roundhouse_snaps_to_turntable_exit() {
        let catalogs = builtin_catalogs();
        let tt_entry = catalogs
            .iter()
            .flat_map(|c| c.entries.iter())
            .find(|e| e.kind == "turntable")
            .expect("catalog should contain a turntable");
        let tt_r = tt_entry
            .radius
            .or_else(|| tt_entry.length.map(|l| l * 0.5))
            .unwrap_or(189.0);
        let tt = create_piece("tt-1".into(), tt_entry, Transform { x: 0.0, y: 0.0, rotation: 0.0 }, None);
        let target_dist = tt_r + 6.0 + 310.0 * 0.5;
        let cand = Transform {
            x: target_dist + 12.0,
            y: 8.0,
            rotation: 0.0,
        };
        let (tr, _edge) = snap_model_to_turntable(cand, &[tt], &catalogs, 75.0, 15.0, 310.0)
            .expect("roundhouse should snap radially to turntable");
        let d = tr.x.hypot(tr.y);
        assert!((d - target_dist).abs() < 3.0);
    }

    #[test]
    fn track_piece_snaps_to_engine_shed_door() {
        let catalogs = builtin_catalogs();
        let entry = find_entry(&catalogs, "24188").unwrap();
        let shed = make_test_shed(308.3, 0.0, 0.0);
        // Shed door is at (188.3, 0.0); place straight track near it
        let p = create_piece("p1".into(), entry, Transform { x: -10.0, y: 12.0, rotation: 4.0 }, None);
        let mut selected = HashSet::new();
        selected.insert("p1".to_string());
        let mut layout = Layout {
            name: "T".into(),
            scale: "H0".into(),
            pieces: vec![p],
            models: Some(vec![shed]),
            feed_voltage: None,
            catenary_feed_voltage: None,
            catenary_circuit_mode: None,
            loco_current_ma: None,
            loco_count: None,
            train_current_amps: None,
            turnout_lantern_current_ma: None,
            turnout_decoder_current_ma: None,
            layers: None,
            active_layer_id: None,
            baseboard: None,
            baseboards: None,
        };
        assert!(snap_selected_pieces_to_buildings(&mut layout, &catalogs, &selected, 45.0));
        let door_c = &layout.pieces[0].connectors[1];
        assert!((door_c.x - 188.3).abs() < 0.3);
        assert!(door_c.y.abs() < 0.3);
    }
}
