use std::collections::{HashMap, HashSet};

use railbench_core::geometry::{deg2rad, norm_angle_deg, rotate};
use railbench_core::snapping::piece_connector_height;
use railbench_core::types::{Catalog, Layout, Model3D, TrackPiece, Transform, Vec2};

use crate::catalog::{find_entry, is_road_id};

pub const SNAP_HEIGHT_TOL_MM: f64 = 1.0;

pub fn layer_height_mm(layout: &Layout, layer_id: Option<&str>) -> f64 {
    let lid = layer_id.unwrap_or("layer-1");
    layout
        .layers
        .as_ref()
        .and_then(|ls| ls.iter().find(|l| l.id == lid))
        .and_then(|l| l.height_mm)
        .unwrap_or(0.0)
}

pub fn model_world_height(layout: &Layout, model: &Model3D) -> f64 {
    layer_height_mm(layout, model.layer_id.as_deref()) + model.elevation.unwrap_or(0.0)
}

pub fn piece_connector_world_height(
    layout: &Layout,
    piece: &TrackPiece,
    connector_index: usize,
) -> f64 {
    let lh = layer_height_mm(layout, piece.layer_id.as_deref());
    piece_connector_height(piece, connector_index, lh)
}

fn layer_height_from_map(layer_heights: &HashMap<String, f64>, layer_id: Option<&str>) -> f64 {
    let lid = layer_id.unwrap_or("layer-1");
    layer_heights.get(lid).copied().unwrap_or(0.0)
}

fn build_layer_height_map(layout: &Layout) -> HashMap<String, f64> {
    let mut map = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            map.insert(l.id.clone(), l.height_mm.unwrap_or(0.0));
        }
    }
    map
}

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
    pub same_level: bool,
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
                (-40.54_f64, -159.26_f64, 102.0_f64, 282.0_f64),
                (0.0_f64, -155.0_f64, 90.0_f64, 270.0_f64),
                (40.54_f64, -159.26_f64, 78.0_f64, 258.0_f64),
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
        Some("station") => {
            let half_w = (model.width * scale) * 0.5;
            let half_d = (model.depth * scale) * 0.5;
            let track_y = half_d + 18.0 * scale;
            let p0 = rotate(Vec2 { x: -half_w, y: track_y }, model.transform.rotation);
            let p1 = rotate(Vec2 { x: half_w, y: track_y }, model.transform.rotation);
            let fwd = norm_angle_deg(model.transform.rotation);
            let rev = norm_angle_deg(model.transform.rotation + 180.0);
            vec![
                BuildingDoorConnector {
                    x: ((model.transform.x + p0.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p0.y) * 10.0).round() / 10.0,
                    inward_angle: fwd,
                    outward_angle: rev,
                    stall_index: 0,
                    model_id: model.id.clone(),
                },
                BuildingDoorConnector {
                    x: ((model.transform.x + p1.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p1.y) * 10.0).round() / 10.0,
                    inward_angle: rev,
                    outward_angle: fwd,
                    stall_index: 1,
                    model_id: model.id.clone(),
                },
            ]
        }
        Some("platform") | Some("bahnsteig") => {
            let half_w = (model.width * scale) * 0.5;
            let half_d = (model.depth * scale) * 0.5;
            let off = half_d + 17.5 * scale;
            let fwd = norm_angle_deg(model.transform.rotation);
            let rev = norm_angle_deg(model.transform.rotation + 180.0);
            let mut out = Vec::with_capacity(4);
            for (si, sy) in [off, -off].into_iter().enumerate() {
                let p0 = rotate(Vec2 { x: -half_w, y: sy }, model.transform.rotation);
                let p1 = rotate(Vec2 { x: half_w, y: sy }, model.transform.rotation);
                out.push(BuildingDoorConnector {
                    x: ((model.transform.x + p0.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p0.y) * 10.0).round() / 10.0,
                    inward_angle: fwd,
                    outward_angle: rev,
                    stall_index: si * 2,
                    model_id: model.id.clone(),
                });
                out.push(BuildingDoorConnector {
                    x: ((model.transform.x + p1.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p1.y) * 10.0).round() / 10.0,
                    inward_angle: rev,
                    outward_angle: fwd,
                    stall_index: si * 2 + 1,
                    model_id: model.id.clone(),
                });
            }
            out
        }
        Some("coaling-station") | Some("diesel-fuel-station") => {
            let half_w = (model.width * scale) * 0.5;
            let half_d = (model.depth * scale) * 0.5;
            let track_y = half_d + 17.0 * scale;
            let p0 = rotate(Vec2 { x: -half_w, y: track_y }, model.transform.rotation);
            let p1 = rotate(Vec2 { x: half_w, y: track_y }, model.transform.rotation);
            let fwd = norm_angle_deg(model.transform.rotation);
            let rev = norm_angle_deg(model.transform.rotation + 180.0);
            vec![
                BuildingDoorConnector {
                    x: ((model.transform.x + p0.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p0.y) * 10.0).round() / 10.0,
                    inward_angle: fwd,
                    outward_angle: rev,
                    stall_index: 0,
                    model_id: model.id.clone(),
                },
                BuildingDoorConnector {
                    x: ((model.transform.x + p1.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p1.y) * 10.0).round() / 10.0,
                    inward_angle: rev,
                    outward_angle: fwd,
                    stall_index: 1,
                    model_id: model.id.clone(),
                },
            ]
        }
        Some("water-tower") => {
            let half_w = (model.width * scale) * 0.5;
            let half_d = (model.depth * scale) * 0.5;
            let track_x = half_w + 17.5 * scale;
            let p0 = rotate(Vec2 { x: track_x, y: -half_d }, model.transform.rotation);
            let p1 = rotate(Vec2 { x: track_x, y: half_d }, model.transform.rotation);
            let fwd = norm_angle_deg(model.transform.rotation + 90.0);
            let rev = norm_angle_deg(model.transform.rotation + 270.0);
            vec![
                BuildingDoorConnector {
                    x: ((model.transform.x + p0.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p0.y) * 10.0).round() / 10.0,
                    inward_angle: fwd,
                    outward_angle: rev,
                    stall_index: 0,
                    model_id: model.id.clone(),
                },
                BuildingDoorConnector {
                    x: ((model.transform.x + p1.x) * 10.0).round() / 10.0,
                    y: ((model.transform.y + p1.y) * 10.0).round() / 10.0,
                    inward_angle: rev,
                    outward_angle: fwd,
                    stall_index: 1,
                    model_id: model.id.clone(),
                },
            ]
        }
        _ => Vec::new(),
    }
}

pub fn collect_model_door_positions(
    layout: &Layout,
    model: &Model3D,
    out: &mut Vec<(f64, f64, f64)>,
) {
    let scale = model.scale.unwrap_or(1.0);
    let dh = model_world_height(layout, model);
    match model.builtin_id.as_deref() {
        Some("engine-shed") => {
            let length = model.width * scale;
            let rp = rotate(Vec2 { x: -length * 0.5, y: 0.0 }, model.transform.rotation);
            let wx = ((model.transform.x + rp.x) * 10.0).round() / 10.0;
            let wy = ((model.transform.y + rp.y) * 10.0).round() / 10.0;
            out.push((wx, wy, dh));
        }
        Some("turntable-trainhall") => {
            const STALLS: [(f64, f64); 3] = [
                (-40.54, -159.26),
                (0.0, -155.0),
                (40.54, -159.26),
            ];
            for &(sx, sy) in &STALLS {
                let rp = rotate(
                    Vec2 {
                        x: sx * scale,
                        y: sy * scale,
                    },
                    model.transform.rotation,
                );
                let wx = ((model.transform.x + rp.x) * 10.0).round() / 10.0;
                let wy = ((model.transform.y + rp.y) * 10.0).round() / 10.0;
                out.push((wx, wy, dh));
            }
        }
        _ => {}
    }
}

#[inline]
pub fn is_pos_near_any_door(
    x: f64,
    y: f64,
    h: f64,
    doors: &[(f64, f64, f64)],
    tolerance: f64,
) -> bool {
    let tol_sq = tolerance * tolerance;
    doors.iter().any(|&(dx, dy, dh)| {
        let ddx = x - dx;
        let ddy = y - dy;
        ddx * ddx + ddy * ddy <= tol_sq && (h - dh).abs() <= SNAP_HEIGHT_TOL_MM
    })
}

#[allow(dead_code)]
pub fn snap_model_to_turntable(
    cand: Transform,
    pieces: &[TrackPiece],
    catalogs: &[Catalog],
    radial_tol: f64,
    angle_tol: f64,
    model_depth: f64,
) -> Option<(Transform, Vec2)> {
    snap_model_to_turntable_inner(
        cand,
        0.0,
        pieces,
        &HashMap::new(),
        catalogs,
        radial_tol,
        angle_tol,
        model_depth,
        true,
    )
    .map(|(tr, edge, _)| (tr, edge))
}

pub fn preview_selected_model_snap(
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
) -> Option<(Vec2, Vec2, bool)> {
    let models = layout.models.as_ref()?;
    let layer_heights = build_layer_height_map(layout);
    let visible_pieces: Vec<TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| is_layer_visible(layout, p.layer_id.as_deref()))
        .cloned()
        .collect();
    let visible_models: Vec<Model3D> = models
        .iter()
        .filter(|m| is_layer_visible(layout, m.layer_id.as_deref()))
        .cloned()
        .collect();

    let mut fallback_diff: Option<(Vec2, Vec2, bool)> = None;
    for m in models {
        if !selected.contains(&m.id) {
            continue;
        }
        if let Some((_snapped_tr, target_pt, same_level)) = find_model_snap_for_model(
            m,
            &visible_pieces,
            &visible_models,
            &layer_heights,
            catalogs,
            false,
        ) {
            let door_src = match m.builtin_id.as_deref() {
                Some("engine-shed") | Some("turntable-trainhall") => get_building_door_connectors(m)
                    .first()
                    .map(|d| Vec2 { x: d.x, y: d.y })
                    .unwrap_or(Vec2 {
                        x: m.transform.x,
                        y: m.transform.y,
                    }),
                _ => Vec2 {
                    x: m.transform.x,
                    y: m.transform.y,
                },
            };
            if same_level {
                return Some((door_src, target_pt, true));
            } else if fallback_diff.is_none() {
                fallback_diff = Some((door_src, target_pt, false));
            }
        }
    }
    fallback_diff
}

pub fn snap_selected_models(layout: &mut Layout, catalogs: &[Catalog], selected: &HashSet<String>) -> bool {
    let layer_heights = build_layer_height_map(layout);
    let visible_pieces: Vec<TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| is_layer_visible(layout, p.layer_id.as_deref()))
        .cloned()
        .collect();
    let visible_models: Vec<Model3D> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| is_layer_visible(layout, m.layer_id.as_deref()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let Some(models) = &mut layout.models else {
        return false;
    };

    let mut snapped_any = false;
    for m in models.iter_mut() {
        if !selected.contains(&m.id) {
            continue;
        }
        if let Some((snapped_tr, _, true)) = find_model_snap_for_model(
            m,
            &visible_pieces,
            &visible_models,
            &layer_heights,
            catalogs,
            true,
        ) {
            m.transform = snapped_tr;
            snapped_any = true;
        }
    }
    snapped_any
}

mod snap_solvers;
#[allow(unused_imports)]
use snap_solvers::*;
pub use snap_solvers::{find_best_track_to_building_snap, find_best_track_to_building_snap_any_level, snap_selected_pieces_to_buildings};
#[cfg(test)]
mod tests;
