use std::collections::{HashMap, HashSet};

#[allow(unused_imports)]
use eframe::egui::{self, Color32, CornerRadius, FontId, Frame, Margin, Rect, Response, RichText, Stroke};
#[allow(unused_imports)]
use railbench_core::geometry::{norm_angle_deg, rotate};
#[allow(unused_imports)]
use railbench_core::snapping::{piece_connector_height, update_layout_connections};
#[allow(unused_imports)]
use railbench_core::types::{Catalog, CatalogEntry, Elevation, Layout, Model3D, TrackPiece, Transform, Vec2};

#[allow(unused_imports)]
use crate::catalog::{create_piece, disconnect_cross_type_connections, find_entry, is_road_id, local_connectors};
#[allow(unused_imports)]
use crate::export::sample_track_branches_world;
#[allow(unused_imports)]
use crate::i18n::Lang;

#[allow(unused_imports)]
use super::{Camera3D, Viewer3DEnv};

#[derive(Debug, Clone, Default)]
pub struct Edit3DState {
    pub enabled: bool,
    pub dragging_selection: bool,
    pub dragging_vertical: bool,
    pub dragging_rotation: bool,
    pub undo_pushed_for_drag: bool,
    pub drag_last_xz: Option<[f64; 2]>,
    pub drag_plane_y: f32,
    pub rot_pivot_xz: [f64; 2],
    pub rot_start_angle_deg: f64,
    pub rot_applied_delta_deg: f64,
    pub rot_base_angle_deg: f64,
    pub rot_initial_piece_deg: Option<f64>,
    pub hovered_id: Option<String>,
    pub snap_preview_world: Option<[f32; 3]>,
    pub preview_piece: Option<TrackPiece>,
    pub preview_elev_mm: f32,
}

pub struct Edit3DParams<'a> {
    pub state: &'a mut Edit3DState,
    pub layout: &'a mut Layout,
    pub catalogs: &'a [Catalog],
    pub selected: &'a mut HashSet<String>,
    pub dragging_catalog: Option<&'a CatalogEntry>,
    pub dragging_model: Option<&'a Model3D>,
    pub terrain_brush: &'a mut crate::terrain_tools::TerrainBrushState,
    pub snapping_enabled: bool,
    pub layer_protection: bool,
    pub rotation_step: f64,
    pub terrain_only_selection: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Edit3DOutput {
    pub layout_changed: bool,
    pub push_undo: bool,
    pub rotate_delta_deg: Option<f64>,
    pub quick_add_at: Option<(String, usize)>,
    pub drop_catalog_at: Option<(CatalogEntry, Vec2, f64)>,
    pub drop_model_at: Option<(Model3D, Vec2)>,
}

/// Computes a realistic 3D ghost `TrackPiece` for dragging `entry` onto `hit_world_xz` at `active_lh`,
/// magnetically snapping to the closest compatible open track connector or building doorway within
/// `snap_radius`. Returns `(ghost_piece, optional_snap_target_xyz, preview_elev_mm)`.
pub fn compute_3d_catalog_drop_preview(
    entry: &CatalogEntry,
    hit_world_xz: Vec2,
    active_lh: f64,
    layout: &Layout,
    catalogs: &[Catalog],
    snapping_enabled: bool,
    snap_radius: f64,
) -> (TrackPiece, Option<[f32; 3]>, f32) {
    let locals = local_connectors(entry);
    let (cx, cy) = if locals.is_empty() {
        (0.0, 0.0)
    } else {
        let sx: f64 = locals.iter().map(|c| c.x).sum();
        let sy: f64 = locals.iter().map(|c| c.y).sum();
        (sx / locals.len() as f64, sy / locals.len() as f64)
    };
    let tr = Transform {
        x: hit_world_xz.x - cx,
        y: hit_world_xz.y - cy,
        rotation: 0.0,
    };
    let mut ghost = create_piece(
        "__3d_ghost__".into(),
        entry,
        tr,
        layout.active_layer_id.clone(),
    );
    if !snapping_enabled || ghost.connectors.is_empty() {
        return (ghost, None, active_lh as f32);
    }

    let layers = layer_info_map(layout);
    let ghost_is_road = entry.is_road == Some(true) || is_road_id(catalogs, &entry.id);
    let max_d2 = snap_radius * snap_radius;

    // Candidate target connectors (either other piece's open connectors of matching type, or building doors)
    let mut best: Option<(f64, usize, f64, f64, f64, f64)> = None; // (d2, c1_idx, tx, ty, tang, te_mm)

    for (c1_i, c1) in ghost.connectors.iter().enumerate() {
        for other in &layout.pieces {
            let olid = other.layer_id.as_deref().unwrap_or("layer-1");
            let (olh, vis, _) = layers.get(olid).copied().unwrap_or((0.0, true, false));
            if !vis {
                continue;
            }
            if is_road_id(catalogs, &other.catalog_id) != ghost_is_road {
                continue;
            }
            for (c2_i, c2) in other.connectors.iter().enumerate() {
                if c2.connected_to.is_some() {
                    continue;
                }
                let d2 = (c1.x - c2.x).powi(2) + (c1.y - c2.y).powi(2);
                if d2 <= max_d2 && best.map_or(true, |b| d2 < b.0) {
                    let h2 = piece_connector_height(other, c2_i, olh);
                    best = Some((d2, c1_i, c2.x, c2.y, c2.angle, h2));
                }
            }
        }
        if !ghost_is_road {
            if let Some(models) = &layout.models {
                for m in models {
                    let mlid = m.layer_id.as_deref().unwrap_or("layer-1");
                    let (mlh, vis, _) = layers.get(mlid).copied().unwrap_or((0.0, true, false));
                    if !vis {
                        continue;
                    }
                    let mh = mlh + m.elevation.unwrap_or(0.0);
                    for door in crate::building_snap::get_building_door_connectors(m) {
                        let d2 = (c1.x - door.x).powi(2) + (c1.y - door.y).powi(2);
                        if d2 <= max_d2 && best.map_or(true, |b| d2 < b.0) {
                            best = Some((d2, c1_i, door.x, door.y, door.outward_angle, mh));
                        }
                    }
                }
            }
        }
    }

    if let Some((_, c1_i, tx, ty, tang, te_mm)) = best {
        let loc = locals.get(c1_i).unwrap_or(&locals[0]);
        let rot = norm_angle_deg(tang + 180.0 - loc.angle);
        let ro = rotate(Vec2 { x: loc.x, y: loc.y }, rot);
        let snap_tr = Transform {
            x: tx - ro.x,
            y: ty - ro.y,
            rotation: rot,
        };
        ghost = create_piece(
            "__3d_ghost__".into(),
            entry,
            snap_tr,
            layout.active_layer_id.clone(),
        );
        return (
            ghost,
            Some([tx as f32, te_mm as f32, ty as f32]),
            te_mm as f32,
        );
    }

    (ghost, None, active_lh as f32)
}

pub(super) fn collect_3d_plus_connectors(
    layout: &Layout,
    selected: &HashSet<String>,
    hovered_id: Option<&str>,
    rect: Rect,
    camera: &Camera3D,
    can_build: bool,
) -> Vec<(String, usize, eframe::egui::Pos2)> {
    if !can_build || layout.pieces.is_empty() {
        return Vec::new();
    }
    let layers = layer_info_map(layout);
    let mut focus_ids: HashSet<&str> = HashSet::new();
    for id in selected {
        focus_ids.insert(id.as_str());
    }
    if let Some(hid) = hovered_id {
        focus_ids.insert(hid);
    }
    if focus_ids.is_empty() {
        for p in layout.pieces.iter().rev().take(2) {
            focus_ids.insert(p.id.as_str());
        }
    }

    let mut out = Vec::new();
    for p in &layout.pieces {
        if !focus_ids.contains(p.id.as_str()) {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let (lh, vis, _) = layers.get(lid).copied().unwrap_or((0.0, true, false));
        if !vis {
            continue;
        }
        for (ci, c) in p.connectors.iter().enumerate() {
            if c.connected_to.is_some() {
                continue;
            }
            let rad = c.angle.to_radians();
            let px = c.x + rad.cos() * 18.0;
            let pz = c.y + rad.sin() * 18.0;
            let py = piece_connector_height(p, ci, lh) + 12.0;
            if let Some(sp) = camera.world_to_screen(rect, [px as f32, py as f32, pz as f32]) {
                if rect.contains(sp) {
                    out.push((p.id.clone(), ci, sp));
                }
            }
        }
    }
    out
}

pub fn ray_intersect_y_plane(origin: [f32; 3], dir: [f32; 3], plane_y: f32) -> Option<[f32; 3]> {
    if dir[1].abs() < 1e-4 {
        return None;
    }
    let t = (plane_y - origin[1]) / dir[1];
    if t <= 0.5 || t > 100_000.0 {
        return None;
    }
    Some([
        origin[0] + dir[0] * t,
        plane_y,
        origin[2] + dir[2] * t,
    ])
}

/// Computes minimum `(distance, ray_t)` between ray `origin + s*dir (s>=0)` and 3D segment `A..B`.
pub(super) fn ray_segment_distance_3d(
    origin: [f32; 3],
    dir: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
) -> (f32, f32) {
    let u = dir;
    let v = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let w = [origin[0] - a[0], origin[1] - a[1], origin[2] - a[2]];

    let a_len = u[0] * u[0] + u[1] * u[1] + u[2] * u[2];
    let b_dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let c_len = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let d_dot = u[0] * w[0] + u[1] * w[1] + u[2] * w[2];
    let e_dot = v[0] * w[0] + v[1] * w[1] + v[2] * w[2];
    let denom = a_len * c_len - b_dot * b_dot;

    let mut sc = if denom.abs() < 1e-6 {
        0.0
    } else {
        (b_dot * e_dot - c_len * d_dot) / denom
    };
    let mut tc = if c_len.abs() < 1e-6 {
        0.0
    } else if denom.abs() < 1e-6 {
        e_dot / c_len
    } else {
        (a_len * e_dot - b_dot * d_dot) / denom
    };

    if tc < 0.0 {
        tc = 0.0;
        sc = (-d_dot / a_len.max(1e-6)).max(0.0);
    } else if tc > 1.0 {
        tc = 1.0;
        sc = ((-d_dot + b_dot) / a_len.max(1e-6)).max(0.0);
    } else if sc < 0.0 {
        sc = 0.0;
        tc = (e_dot / c_len.max(1e-6)).clamp(0.0, 1.0);
    } else {
        sc = sc.max(0.0);
    }

    let px = origin[0] + sc * u[0];
    let py = origin[1] + sc * u[1];
    let pz = origin[2] + sc * u[2];
    let qx = a[0] + tc * v[0];
    let qy = a[1] + tc * v[1];
    let qz = a[2] + tc * v[2];
    let dist = ((px - qx).powi(2) + (py - qy).powi(2) + (pz - qz).powi(2)).sqrt();
    (dist, sc)
}

pub(super) fn layer_info_map(layout: &Layout) -> HashMap<&str, (f64, bool, bool)> {
    let mut map = HashMap::new();
    if let Some(layers) = &layout.layers {
        for l in layers {
            map.insert(
                l.id.as_str(),
                (l.height_mm.unwrap_or(0.0), l.visible, l.locked),
            );
        }
    }
    map
}

pub(super) fn is_obj_editable(
    layout: &Layout,
    layers: &HashMap<&str, (f64, bool, bool)>,
    layer_id: Option<&str>,
    layer_protection: bool,
) -> bool {
    let lid = layer_id.unwrap_or("layer-1");
    let (_, vis, locked) = layers.get(lid).copied().unwrap_or((0.0, true, false));
    if !vis || locked {
        return false;
    }
    if layer_protection {
        let active = layout.active_layer_id.as_deref().unwrap_or("layer-1");
        return lid == active;
    }
    true
}

/// Ray-picks the closest editable track piece or 3D building/model along `origin + t*dir`.
pub fn pick_3d_object(
    origin: [f32; 3],
    dir: [f32; 3],
    layout: &Layout,
    catalogs: &[Catalog],
    layer_protection: bool,
) -> Option<String> {
    let layers = layer_info_map(layout);
    let mut best: Option<(f32, String)> = None;

    for p in &layout.pieces {
        if !is_obj_editable(layout, &layers, p.layer_id.as_deref(), layer_protection) {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0) as f32;
        let e0 = lh + p.elevation.map(|e| e.start).unwrap_or(0.0) as f32 + 5.5;
        let e1 = lh + p.elevation.map(|e| e.end).unwrap_or((e0 - lh - 5.5) as f64) as f32 + 5.5;
        let entry = find_entry(catalogs, &p.catalog_id);
        let gauge = entry.and_then(|e| e.gauge).unwrap_or(16.5) as f32;
        let is_road = is_road_id(catalogs, &p.catalog_id);
        let hit_r = if is_road {
            entry.and_then(|e| e.road_width).unwrap_or(60.0) as f32 * 0.58
        } else {
            (gauge * 1.45).max(24.0)
        };

        for br in sample_track_branches_world(p, entry) {
            if br.len() < 2 {
                continue;
            }
            let n = (br.len() - 1).max(1) as f32;
            for (k, w) in br.windows(2).enumerate() {
                let t0 = k as f32 / n;
                let t1 = (k + 1) as f32 / n;
                let a = [w[0].x as f32, e0 + (e1 - e0) * t0, w[0].y as f32];
                let b = [w[1].x as f32, e0 + (e1 - e0) * t1, w[1].y as f32];
                let (dist, ray_t) = ray_segment_distance_3d(origin, dir, a, b);
                if dist <= hit_r && best.as_ref().map_or(true, |(bt, _)| ray_t < *bt) {
                    best = Some((ray_t, p.id.clone()));
                }
            }
        }
    }

    if let Some(models) = &layout.models {
        for m in models {
            if !is_obj_editable(layout, &layers, m.layer_id.as_deref(), layer_protection) {
                continue;
            }
            if matches!(m.builtin_id.as_deref(), Some("ground-terrain") | Some("mountain")) {
                continue;
            }
            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
            let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0) as f32;
            let s = m.scale.unwrap_or(1.0) as f32;
            let my = lh + m.elevation.unwrap_or(0.0) as f32;
            let mh = (m.height as f32 * s).max(16.0);
            let rad = ((m.width as f32).hypot(m.depth as f32) * 0.5 * s).max(22.0);
            let a = [m.transform.x as f32, my, m.transform.y as f32];
            let b = [m.transform.x as f32, my + mh, m.transform.y as f32];
            let (dist, ray_t) = ray_segment_distance_3d(origin, dir, a, b);
            if dist <= rad && best.as_ref().map_or(true, |(bt, _)| ray_t < *bt) {
                best = Some((ray_t, m.id.clone()));
            }
        }
    }

    best.map(|(_, id)| id)
}

pub(super) fn active_selection_pivot_world(layout: &Layout, selected: &HashSet<String>) -> [f32; 3] {
    let layers = layer_info_map(layout);
    let mut sx = 0.0_f64;
    let mut sy = 0.0_f64;
    let mut sz = 0.0_f64;
    let mut count = 0usize;

    for p in &layout.pieces {
        if !selected.contains(&p.id) {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0);
        let elev = p.elevation.map(|e| (e.start + e.end) * 0.5).unwrap_or(0.0);
        if p.connectors.is_empty() {
            sx += p.transform.x;
            sz += p.transform.y;
        } else {
            let cx: f64 = p.connectors.iter().map(|c| c.x).sum::<f64>() / p.connectors.len() as f64;
            let cy: f64 = p.connectors.iter().map(|c| c.y).sum::<f64>() / p.connectors.len() as f64;
            sx += cx;
            sz += cy;
        }
        sy += lh + elev;
        count += 1;
    }

    if let Some(models) = &layout.models {
        for m in models {
            if !selected.contains(&m.id) {
                continue;
            }
            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
            let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0);
            sx += m.transform.x;
            sy += lh + m.elevation.unwrap_or(0.0);
            sz += m.transform.y;
            count += 1;
        }
    }

    if count == 0 {
        [0.0, 0.0, 0.0]
    } else {
        let n = count as f64;
        [(sx / n) as f32, (sy / n) as f32, (sz / n) as f32]
    }
}

/// Computes `(pivot_3d, handle_3d, base_angle_deg, single_initial_rot)` for an interactive
/// 3D rotation handle point protruding in the XZ plane from the active selection pivot.
pub fn compute_3d_rotation_handle_world(
    layout: &Layout,
    selected: &HashSet<String>,
    layer_protection: bool,
    camera_dist: f32,
    edit: &Edit3DState,
) -> Option<([f32; 3], [f32; 3], f64, Option<f64>)> {
    if selected.is_empty() {
        return None;
    }
    let layers = layer_info_map(layout);
    let editable_pieces: Vec<&TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| selected.contains(&p.id) && is_obj_editable(layout, &layers, p.layer_id.as_deref(), layer_protection))
        .collect();
    let editable_models: Vec<&Model3D> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| {
                    selected.contains(&m.id)
                        && is_obj_editable(layout, &layers, m.layer_id.as_deref(), layer_protection)
                        && !matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain"))
                })
                .collect()
        })
        .unwrap_or_default();
    if editable_pieces.is_empty() && editable_models.is_empty() {
        return None;
    }

    let mut pivot = active_selection_pivot_world(layout, selected);
    if edit.dragging_rotation {
        pivot[0] = edit.rot_pivot_xz[0] as f32;
        pivot[2] = edit.rot_pivot_xz[1] as f32;
    }
    let r = (camera_dist * 0.085).clamp(48.0, 140.0);

    if editable_pieces.len() == 1 && editable_models.is_empty() {
        let rot = editable_pieces[0].transform.rotation;
        let base_ang = norm_angle_deg(rot - 90.0);
        let rad = rot.to_radians();
        let handle_3d = [
            pivot[0] + (rad.sin() as f32) * r,
            pivot[1] + 8.0,
            pivot[2] - (rad.cos() as f32) * r,
        ];
        return Some((pivot, handle_3d, base_ang, Some(rot)));
    }

    let ang = if edit.dragging_rotation {
        edit.rot_base_angle_deg + edit.rot_applied_delta_deg
    } else {
        -90.0
    };
    let rad = ang.to_radians();
    let handle_3d = [
        pivot[0] + (rad.cos() as f32) * r,
        pivot[1] + 8.0,
        pivot[2] + (rad.sin() as f32) * r,
    ];
    Some((pivot, handle_3d, -90.0, None))
}

mod interaction;
mod terrain_sculpt;

#[allow(unused_imports)]
pub use interaction::*;
pub use terrain_sculpt::*;
