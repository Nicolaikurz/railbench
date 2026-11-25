use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, CornerRadius, FontId, Frame, Margin, Rect, Response, RichText, Stroke};
use railbench_core::geometry::{norm_angle_deg, rotate};
use railbench_core::snapping::{piece_connector_height, update_layout_connections};
use railbench_core::types::{Catalog, CatalogEntry, Elevation, Layout, Model3D, TrackPiece, Transform, Vec2};

use crate::catalog::{create_piece, disconnect_cross_type_connections, find_entry, is_road_id, local_connectors};
use crate::export::sample_track_branches_world;
use crate::i18n::Lang;

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
    pub last_placed_catalog_id: Option<&'a str>,
    pub terrain_brush: &'a mut crate::terrain_tools::TerrainBrushState,
    pub snapping_enabled: &'a mut bool,
    pub layer_protection: bool,
    pub rotation_step: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Edit3DOutput {
    pub layout_changed: bool,
    pub push_undo: bool,
    pub rotate_delta_deg: Option<f64>,
    pub elevation_delta_mm: Option<f64>,
    pub delete_requested: bool,
    pub close_gap_requested: bool,
    pub parallel_requested: bool,
    pub add_last_track_requested: bool,
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

fn collect_3d_plus_connectors(
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
fn ray_segment_distance_3d(
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

fn layer_info_map(layout: &Layout) -> HashMap<&str, (f64, bool, bool)> {
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

fn is_obj_editable(
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

fn active_selection_pivot_world(layout: &Layout, selected: &HashSet<String>) -> [f32; 3] {
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

/// Processes 3D selection, XZ dragging with snap, Alt+Drag vertical elevation, catalog drops,
/// and keyboard shortcuts when `Edit 3D` is enabled.
/// Returns `(consumed_primary_pointer, Edit3DOutput)`.
#[allow(clippy::too_many_arguments)]
pub fn handle_3d_edit_interaction(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &Response,
    camera: &Camera3D,
    edit: &mut Edit3DState,
    layout: &mut Layout,
    catalogs: &[Catalog],
    selected: &mut HashSet<String>,
    dragging_catalog: Option<&CatalogEntry>,
    dragging_model: Option<&Model3D>,
    snapping_enabled: bool,
    layer_protection: bool,
    rotation_step: f64,
    env_mode: Viewer3DEnv,
) -> (bool, Edit3DOutput) {
    let mut out = Edit3DOutput::default();
    let can_build = edit.enabled || env_mode == Viewer3DEnv::Minimal;
    if !can_build {
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.dragging_rotation = false;
        edit.undo_pushed_for_drag = false;
        edit.hovered_id = None;
        edit.snap_preview_world = None;
        edit.preview_piece = None;
        return (false, out);
    }

    let hover_pos = ui.input(|i| i.pointer.hover_pos()).or_else(|| {
        if dragging_catalog.is_some() || dragging_model.is_some() {
            ui.input(|i| i.pointer.latest_pos())
        } else {
            None
        }
    });
    let layers = layer_info_map(layout);
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1").to_string();
    let active_lh = layers.get(active_lid.as_str()).map(|v| v.0).unwrap_or(0.0);

    // 1. Handle Catalog Entry Drag-and-Drop directly onto the 3D plane with magnetic snap preview!
    if let Some(entry) = dragging_catalog {
        if let Some(hp) = hover_pos {
            if rect.contains(hp) {
                let (ray_o, ray_d) = camera.screen_to_ray(rect, hp);
                if let Some(hit) = ray_intersect_y_plane(ray_o, ray_d, active_lh as f32) {
                    let dyn_snap = (camera.distance as f64 * 0.045).clamp(45.0, 125.0);
                    let raw_xz = Vec2 {
                        x: hit[0] as f64,
                        y: hit[2] as f64,
                    };
                    let (ghost, snap_pt, prev_elev) = compute_3d_catalog_drop_preview(
                        entry,
                        raw_xz,
                        active_lh,
                        layout,
                        catalogs,
                        snapping_enabled,
                        dyn_snap,
                    );
                    edit.snap_preview_world = snap_pt.or(Some(hit));
                    edit.preview_piece = Some(ghost);
                    edit.preview_elev_mm = prev_elev;
                    if ui.input(|i| i.pointer.any_released()) {
                        out.drop_catalog_at = Some((entry.clone(), raw_xz, dyn_snap));
                        edit.snap_preview_world = None;
                        edit.preview_piece = None;
                    }
                    return (true, out);
                }
            }
        }
        edit.snap_preview_world = None;
        edit.preview_piece = None;
        return (false, out);
    }
    edit.preview_piece = None;

    // 1b. Handle 3D Model Drag-and-Drop directly onto the 3D plane
    if let Some(model) = dragging_model {
        if let Some(hp) = hover_pos {
            if rect.contains(hp) {
                let (ray_o, ray_d) = camera.screen_to_ray(rect, hp);
                if let Some(hit) = ray_intersect_y_plane(ray_o, ray_d, active_lh as f32) {
                    edit.snap_preview_world = Some(hit);
                    edit.preview_elev_mm = active_lh as f32;
                    if ui.input(|i| i.pointer.any_released()) {
                        out.drop_model_at = Some((
                            model.clone(),
                            Vec2 {
                                x: hit[0] as f64,
                                y: hit[2] as f64,
                            },
                        ));
                        edit.snap_preview_world = None;
                    }
                    return (true, out);
                }
            }
        }
        edit.snap_preview_world = None;
        return (false, out);
    }
    edit.snap_preview_world = None;

    // 2. Update 3D hover target when not dragging
    edit.hovered_id = if !edit.dragging_selection {
        hover_pos
            .filter(|hp| rect.contains(*hp))
            .and_then(|hp| {
                let (ro, rd) = camera.screen_to_ray(rect, hp);
                pick_3d_object(ro, rd, layout, catalogs, layer_protection)
            })
    } else {
        None
    };

    // 3. Keyboard PageUp/PageDown elevation & Shift+Wheel rotation while Edit 3D is active
    if !ui.ctx().wants_keyboard_input() && !selected.is_empty() {
        let (rot_dir, elev_step) = ui.input(|i| {
            let mut r = 0.0_f64;
            if i.modifiers.shift && i.smooth_scroll_delta.y.abs() > 0.5 {
                r += if i.smooth_scroll_delta.y > 0.0 { -5.0 } else { 5.0 };
            }
            let mut e = 0.0_f64;
            if i.key_pressed(egui::Key::PageUp) {
                e += 5.0;
            }
            if i.key_pressed(egui::Key::PageDown) {
                e -= 5.0;
            }
            (r, e)
        });
        if rot_dir.abs() > 0.01 {
            out.rotate_delta_deg = Some(rot_dir);
        }
        if elev_step.abs() > 0.01 {
            out.push_undo = true;
            nudge_selection_elevation(layout, selected, layer_protection, elev_step);
            out.layout_changed = true;
        }
    }

    // 4. Primary Click: Check clickable '+' Open Connector Badges FIRST, then 3D Object Picking
    let shift_held = ui.input(|i| i.modifiers.shift);
    let alt_held = ui.input(|i| i.modifiers.alt);
    let rot_handle_proj = if !selected.is_empty() {
        compute_3d_rotation_handle_world(layout, selected, layer_protection, camera.distance, edit)
            .and_then(|(p3, h3, base_ang, single_rot)| {
                camera
                    .world_to_screen(rect, h3)
                    .map(|hs| (p3, hs, base_ang, single_rot))
            })
    } else {
        None
    };

    if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(hp) = hover_pos {
            if rot_handle_proj
                .as_ref()
                .is_some_and(|(_, hs, _, _)| hp.distance(*hs) <= 14.0)
            {
                return (true, out);
            }
            let plus_badges = collect_3d_plus_connectors(
                layout,
                selected,
                edit.hovered_id.as_deref(),
                rect,
                camera,
                can_build,
            );
            if let Some((pid, ci, _)) = plus_badges.into_iter().find(|(_, _, sp)| hp.distance(*sp) <= 13.0) {
                out.quick_add_at = Some((pid, ci));
                return (true, out);
            }
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some(hit_id) = pick_3d_object(ro, rd, layout, catalogs, layer_protection) {
                if shift_held {
                    if !selected.insert(hit_id.clone()) {
                        selected.remove(&hit_id);
                    }
                } else {
                    selected.clear();
                    selected.insert(hit_id);
                }
            } else if !shift_held {
                selected.clear();
            }
        }
        return (false, out);
    }

    // 5. Primary Drag of Selected 3D Rotation Handle OR 3D Object(s) (or empty-space camera Orbit)
    let mut consumed_drag = false;
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(hp) = hover_pos {
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some((p3, _hs, base_ang, single_rot)) =
                rot_handle_proj.filter(|(_, hs, _, _)| hp.distance(*hs) <= 14.5) {
                if let Some(hit) = ray_intersect_y_plane(ro, rd, p3[1]) {
                    let pivot_xz = [p3[0] as f64, p3[2] as f64];
                    edit.dragging_rotation = true;
                    edit.dragging_selection = false;
                    edit.undo_pushed_for_drag = false;
                    edit.drag_plane_y = p3[1];
                    edit.rot_pivot_xz = pivot_xz;
                    edit.rot_start_angle_deg =
                        (hit[2] as f64 - pivot_xz[1]).atan2(hit[0] as f64 - pivot_xz[0]).to_degrees();
                    edit.rot_applied_delta_deg = 0.0;
                    edit.rot_base_angle_deg = base_ang;
                    edit.rot_initial_piece_deg = single_rot;
                }
            } else {
                let picked = pick_3d_object(ro, rd, layout, catalogs, layer_protection);
                if let Some(hit_id) = picked {
                    if !selected.contains(&hit_id) {
                        if !shift_held {
                            selected.clear();
                        }
                        selected.insert(hit_id);
                    }
                    let pivot = active_selection_pivot_world(layout, selected);
                    edit.dragging_selection = true;
                    edit.dragging_vertical = alt_held;
                    edit.undo_pushed_for_drag = false;
                    edit.drag_plane_y = pivot[1];
                    edit.drag_last_xz = ray_intersect_y_plane(ro, rd, pivot[1]).map(|p| [p[0] as f64, p[2] as f64]);
                }
            }
        }
    }

    if edit.dragging_rotation && response.dragged_by(egui::PointerButton::Primary) {
        consumed_drag = true;
        if let Some(hp) = hover_pos {
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some(hit) = ray_intersect_y_plane(ro, rd, edit.drag_plane_y) {
                let cur_ang = (hit[2] as f64 - edit.rot_pivot_xz[1])
                    .atan2(hit[0] as f64 - edit.rot_pivot_xz[0])
                    .to_degrees();
                let raw_delta = crate::canvas2d::unwrap_drag_angle_delta(
                    cur_ang,
                    edit.rot_start_angle_deg,
                    edit.rot_applied_delta_deg,
                );
                let desired_delta =
                    crate::canvas2d::snap_drag_rotation_delta(raw_delta, rotation_step, shift_held);
                let step_delta = desired_delta - edit.rot_applied_delta_deg;
                if step_delta.abs() > 1e-6 {
                    if !edit.undo_pushed_for_drag {
                        out.push_undo = true;
                        edit.undo_pushed_for_drag = true;
                    }
                    if crate::canvas2d::rotate_selection_around_pivot(
                        layout,
                        selected,
                        layer_protection,
                        Vec2 {
                            x: edit.rot_pivot_xz[0],
                            y: edit.rot_pivot_xz[1],
                        },
                        step_delta,
                    ) {
                        edit.rot_applied_delta_deg = desired_delta;
                        out.layout_changed = true;
                    }
                }
            }
        }
    } else if edit.dragging_rotation && !response.dragged_by(egui::PointerButton::Primary) {
        update_layout_connections(layout, 4.0, 35.0, 1.0);
        disconnect_cross_type_connections(layout, catalogs);
        edit.dragging_rotation = false;
        edit.undo_pushed_for_drag = false;
        out.layout_changed = true;
    } else if edit.dragging_selection && response.dragged_by(egui::PointerButton::Primary) {
        consumed_drag = true;
        if edit.dragging_vertical || alt_held {
            let dy_px = response.drag_delta().y;
            if dy_px.abs() > 0.01 {
                if !edit.undo_pushed_for_drag {
                    out.push_undo = true;
                    edit.undo_pushed_for_drag = true;
                }
                let delta_mm = -(dy_px as f64) * (camera.distance as f64 * 0.0014);
                nudge_selection_elevation(layout, selected, layer_protection, delta_mm);
                out.layout_changed = true;
            }
        } else if let Some(hp) = hover_pos {
            let (ro, rd) = camera.screen_to_ray(rect, hp);
            if let Some(hit) = ray_intersect_y_plane(ro, rd, edit.drag_plane_y) {
                let cur_xz = [hit[0] as f64, hit[2] as f64];
                if let Some(prev) = edit.drag_last_xz {
                    let dx = cur_xz[0] - prev[0];
                    let dz = cur_xz[1] - prev[1];
                    if dx.hypot(dz) > 0.05 {
                        if !edit.undo_pushed_for_drag {
                            out.push_undo = true;
                            edit.undo_pushed_for_drag = true;
                        }
                        translate_selection_xz(layout, selected, layer_protection, dx, dz);
                        out.layout_changed = true;
                    }
                }
                edit.drag_last_xz = Some(cur_xz);
            }
        }
    }

    if edit.dragging_selection && !response.dragged_by(egui::PointerButton::Primary) {
        if edit.undo_pushed_for_drag && !edit.dragging_vertical && snapping_enabled {
            let _ = crate::canvas2d::snap_selected_pieces(layout, catalogs, selected, 22.0);
        }
        update_layout_connections(layout, 4.0, 35.0, 1.0);
        disconnect_cross_type_connections(layout, catalogs);
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.undo_pushed_for_drag = false;
        edit.drag_last_xz = None;
        out.layout_changed = true;
    }

    (consumed_drag, out)
}

fn translate_selection_xz(
    layout: &mut Layout,
    selected: &HashSet<String>,
    layer_protection: bool,
    dx: f64,
    dz: f64,
) {
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1").to_string();
    let layers = layer_info_map(layout);
    let can_edit = |lid: Option<&str>| -> bool {
        let l = lid.unwrap_or("layer-1");
        let (_, vis, locked) = layers.get(l).copied().unwrap_or((0.0, true, false));
        vis && !locked && (!layer_protection || l == active_lid.as_str())
    };
    let editable_ids: Vec<String> = selected
        .iter()
        .filter(|id| {
            layout.pieces.iter().any(|p| &p.id == *id && can_edit(p.layer_id.as_deref()))
                || layout
                    .models
                    .as_ref()
                    .map_or(false, |ms| ms.iter().any(|m| &m.id == *id && can_edit(m.layer_id.as_deref())))
        })
        .cloned()
        .collect();
    drop(layers);

    for p in &mut layout.pieces {
        if editable_ids.contains(&p.id) {
            p.transform.x += dx;
            p.transform.y += dz;
            for c in &mut p.connectors {
                c.x += dx;
                c.y += dz;
                c.connected_to = None;
            }
        }
    }
    if let Some(models) = &mut layout.models {
        for m in models {
            if editable_ids.contains(&m.id) {
                m.transform.x += dx;
                m.transform.y += dz;
            }
        }
    }
}

pub(crate) fn nudge_selection_elevation(
    layout: &mut Layout,
    selected: &HashSet<String>,
    layer_protection: bool,
    delta_mm: f64,
) {
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1").to_string();
    let layers = layer_info_map(layout);
    let can_edit = |lid: Option<&str>| -> bool {
        let l = lid.unwrap_or("layer-1");
        let (_, vis, locked) = layers.get(l).copied().unwrap_or((0.0, true, false));
        vis && !locked && (!layer_protection || l == active_lid.as_str())
    };
    let editable_ids: Vec<String> = selected
        .iter()
        .filter(|id| {
            layout.pieces.iter().any(|p| &p.id == *id && can_edit(p.layer_id.as_deref()))
                || layout
                    .models
                    .as_ref()
                    .map_or(false, |ms| ms.iter().any(|m| &m.id == *id && can_edit(m.layer_id.as_deref())))
        })
        .cloned()
        .collect();
    drop(layers);

    for p in &mut layout.pieces {
        if editable_ids.contains(&p.id) {
            let e = p.elevation.get_or_insert(Elevation {
                start: 0.0,
                end: 0.0,
            });
            e.start = (e.start + delta_mm).clamp(-300.0, 1200.0);
            e.end = (e.end + delta_mm).clamp(-300.0, 1200.0);
        }
    }
    if let Some(models) = &mut layout.models {
        for m in models {
            if editable_ids.contains(&m.id) {
                let h = m.elevation.get_or_insert(0.0);
                *h = (*h + delta_mm).clamp(-300.0, 1200.0);
            }
        }
    }
}

/// Renders 3D projected selection wireframes, hover feedback, catalog/model drop ghost,
/// open-connector `+` add badges, XYZ gizmo triad, and the bottom-center floating 3D HUD bar.
#[allow(clippy::too_many_arguments)]
pub fn draw_3d_edit_overlays(
    ui: &mut egui::Ui,
    rect: Rect,
    camera: &Camera3D,
    edit: &Edit3DState,
    layout: &Layout,
    catalogs: &[Catalog],
    selected: &HashSet<String>,
    dragging_catalog: Option<&CatalogEntry>,
    dragging_model: Option<&Model3D>,
    last_placed_catalog_id: Option<&str>,
    env_mode: Viewer3DEnv,
    snapping_enabled: &mut bool,
    rotation_step: f64,
    dark_mode: bool,
    lang: Lang,
    out: &mut Edit3DOutput,
) -> bool {
    let mut exit_requested = false;
    let can_build = edit.enabled || env_mode == Viewer3DEnv::Minimal;
    if !can_build {
        return false;
    }

    let painter = ui.painter_at(rect);
    let layers = layer_info_map(layout);
    let is_de = lang == Lang::De;

    // 1. Projected 3D Selection & Hover Wireframes
    for p in &layout.pieces {
        let is_sel = selected.contains(&p.id);
        let is_hov = edit.hovered_id.as_deref() == Some(p.id.as_str());
        if !is_sel && !is_hov {
            continue;
        }
        let lid = p.layer_id.as_deref().unwrap_or("layer-1");
        let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0) as f32;
        let e0 = lh + p.elevation.map(|e| e.start).unwrap_or(0.0) as f32 + 9.0;
        let e1 = lh + p.elevation.map(|e| e.end).unwrap_or((e0 - lh - 9.0) as f64) as f32 + 9.0;
        let entry = find_entry(catalogs, &p.catalog_id);
        let col = if is_sel {
            Color32::from_rgb(56, 189, 248)
        } else {
            Color32::from_rgba_unmultiplied(250, 204, 21, 185)
        };
        let stroke = Stroke::new(if is_sel { 2.6_f32 } else { 1.6_f32 }, col);
        for br in sample_track_branches_world(p, entry) {
            let n = (br.len().saturating_sub(1)).max(1) as f32;
            let mut spts = Vec::with_capacity(br.len());
            for (k, pt) in br.iter().enumerate() {
                let y = e0 + (e1 - e0) * (k as f32 / n);
                if let Some(sp) = camera.world_to_screen(rect, [pt.x as f32, y, pt.y as f32]) {
                    spts.push(sp);
                }
            }
            if spts.len() >= 2 {
                painter.add(egui::Shape::line(spts, stroke));
            }
        }
        if is_sel {
            for (ci, c) in p.connectors.iter().enumerate() {
                let ch = piece_connector_height(p, ci, lh as f64) as f32 + 9.0;
                if let Some(sp) = camera.world_to_screen(rect, [c.x as f32, ch, c.y as f32]) {
                    let cc = if c.connected_to.is_some() {
                        Color32::from_rgb(34, 197, 94)
                    } else {
                        Color32::from_rgb(250, 204, 21)
                    };
                    painter.circle_filled(sp, 4.5, cc);
                    painter.circle_stroke(sp, 5.2, Stroke::new(1.2_f32, Color32::WHITE));
                }
            }
        }
    }

    if let Some(models) = &layout.models {
        for m in models {
            let is_sel = selected.contains(&m.id);
            let is_hov = edit.hovered_id.as_deref() == Some(m.id.as_str());
            if !is_sel && !is_hov {
                continue;
            }
            let lid = m.layer_id.as_deref().unwrap_or("layer-1");
            let lh = layers.get(lid).map(|v| v.0).unwrap_or(0.0) as f32;
            let y0 = lh + m.elevation.unwrap_or(0.0) as f32;
            let s = m.scale.unwrap_or(1.0);
            let hw = m.width * 0.5 * s;
            let hd = m.depth * 0.5 * s;
            let h = (m.height * s) as f32;
            let col = if is_sel {
                Color32::from_rgb(56, 189, 248)
            } else {
                Color32::from_rgb(250, 204, 21)
            };
            let corners_l = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
            let mut bot = Vec::with_capacity(4);
            let mut top = Vec::with_capacity(4);
            for &(lx, ly) in &corners_l {
                let r = rotate(Vec2 { x: lx, y: ly }, m.transform.rotation);
                let wx = (m.transform.x + r.x) as f32;
                let wz = (m.transform.y + r.y) as f32;
                if let (Some(sb), Some(st)) = (
                    camera.world_to_screen(rect, [wx, y0, wz]),
                    camera.world_to_screen(rect, [wx, y0 + h, wz]),
                ) {
                    bot.push(sb);
                    top.push(st);
                }
            }
            if bot.len() == 4 && top.len() == 4 {
                let st = Stroke::new(if is_sel { 2.0_f32 } else { 1.3_f32 }, col);
                for k in 0..4 {
                    let nk = (k + 1) % 4;
                    painter.line_segment([bot[k], bot[nk]], st);
                    painter.line_segment([top[k], top[nk]], st);
                    painter.line_segment([bot[k], top[k]], st);
                }
            }
        }
    }

    // 2. Catalog & Model Drag Ghost Preview in 3D + Open Connector `+` Badges
    if let (Some(entry), Some(ghost)) = (dragging_catalog, edit.preview_piece.as_ref()) {
        let y_preview = edit.preview_elev_mm + 9.0;
        let ghost_stroke = Stroke::new(3.2_f32, Color32::from_rgb(56, 189, 248));
        for br in sample_track_branches_world(ghost, Some(entry)) {
            let mut spts = Vec::with_capacity(br.len());
            for pt in br {
                if let Some(sp) = camera.world_to_screen(rect, [pt.x as f32, y_preview, pt.y as f32]) {
                    spts.push(sp);
                }
            }
            if spts.len() >= 2 {
                painter.add(egui::Shape::line(spts, ghost_stroke));
            }
        }
        for c in &ghost.connectors {
            if let Some(sp) = camera.world_to_screen(rect, [c.x as f32, y_preview, c.y as f32]) {
                painter.circle_filled(sp, 4.5, Color32::from_rgb(250, 204, 21));
                painter.circle_stroke(sp, 5.2, Stroke::new(1.1_f32, Color32::WHITE));
            }
        }
        if let Some(snap_pt) = edit.snap_preview_world {
            if let Some(sp) = camera.world_to_screen(rect, [snap_pt[0], snap_pt[1] + 9.0, snap_pt[2]]) {
                painter.circle_stroke(sp, 10.0, Stroke::new(2.0_f32, Color32::from_rgb(34, 197, 94)));
            }
        }
    } else if let (Some(model), Some(gp)) = (dragging_model, edit.snap_preview_world) {
        let s = model.scale.unwrap_or(1.0);
        let hw = (model.width * 0.5 * s) as f32;
        let hd = (model.depth * 0.5 * s) as f32;
        let h = (model.height * s) as f32;
        let corners_l = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
        let mut bot = Vec::with_capacity(4);
        let mut top = Vec::with_capacity(4);
        for (lx, lz) in corners_l {
            let wx = gp[0] + lx;
            let wz = gp[2] + lz;
            if let (Some(sb), Some(st)) = (
                camera.world_to_screen(rect, [wx, gp[1], wz]),
                camera.world_to_screen(rect, [wx, gp[1] + h, wz]),
            ) {
                bot.push(sb);
                top.push(st);
            }
        }
        if bot.len() == 4 && top.len() == 4 {
            let st = Stroke::new(2.0_f32, Color32::from_rgb(56, 189, 248));
            for k in 0..4 {
                let nk = (k + 1) % 4;
                painter.line_segment([bot[k], bot[nk]], st);
                painter.line_segment([top[k], top[nk]], st);
                painter.line_segment([bot[k], top[k]], st);
            }
        }
    }

    if dragging_catalog.is_none() && dragging_model.is_none() {
        let hover_pos = ui.ctx().pointer_hover_pos();
        let badges = collect_3d_plus_connectors(
            layout,
            selected,
            edit.hovered_id.as_deref(),
            rect,
            camera,
            can_build,
        );
        for (_pid, _ci, sp) in badges {
            let is_hov = hover_pos.is_some_and(|hp| hp.distance(sp) <= 12.0);
            let fill = if is_hov {
                Color32::from_rgb(34, 197, 94)
            } else {
                Color32::from_rgb(14, 165, 233)
            };
            painter.circle_filled(sp, 9.5, fill);
            painter.circle_stroke(sp, 9.8, Stroke::new(1.3_f32, Color32::WHITE));
            painter.line_segment(
                [sp - egui::vec2(4.2, 0.0), sp + egui::vec2(4.2, 0.0)],
                Stroke::new(2.0_f32, Color32::WHITE),
            );
            painter.line_segment(
                [sp - egui::vec2(0.0, 4.2), sp + egui::vec2(0.0, 4.2)],
                Stroke::new(2.0_f32, Color32::WHITE),
            );
        }
    }

    // 3. XYZ RGB Axis Triad at Active Selection Pivot
    if !selected.is_empty() {
        let pivot = active_selection_pivot_world(layout, selected);
        let arm = (camera.distance * 0.065).clamp(30.0, 95.0);
        if let Some(sp0) = camera.world_to_screen(rect, pivot) {
            for (axis, col, lbl) in [
                ([arm, 0.0, 0.0], Color32::from_rgb(239, 68, 68), "X"),
                ([0.0, arm, 0.0], Color32::from_rgb(34, 197, 94), "Y"),
                ([0.0, 0.0, arm], Color32::from_rgb(59, 130, 246), "Z"),
            ] {
                if let Some(sp1) = camera.world_to_screen(
                    rect,
                    [pivot[0] + axis[0], pivot[1] + axis[1], pivot[2] + axis[2]],
                ) {
                    painter.line_segment([sp0, sp1], Stroke::new(2.2_f32, col));
                    painter.circle_filled(sp1, 3.5, col);
                    painter.text(
                        sp1 + egui::vec2(6.0, -6.0),
                        egui::Align2::LEFT_BOTTOM,
                        lbl,
                        FontId::proportional(10.0),
                        col,
                    );
                }
            }
            painter.circle_filled(sp0, 4.0, Color32::WHITE);
        }

        // 3B. Interactive 3D Track/Selection Rotation Handle Point
        if let Some((rot_piv3, rot_hdl3, _, _)) =
            compute_3d_rotation_handle_world(layout, selected, false, camera.distance, edit)
        {
            if let (Some(ps), Some(hs)) = (
                camera.world_to_screen(rect, rot_piv3),
                camera.world_to_screen(rect, rot_hdl3),
            ) {
                let is_hov = ui
                    .ctx()
                    .pointer_hover_pos()
                    .is_some_and(|hp| hp.distance(hs) <= 14.0);
                let active_or_hov = edit.dragging_rotation || is_hov;
                painter.line_segment(
                    [ps, hs],
                    Stroke::new(1.8_f32, Color32::from_rgb(56, 189, 248)),
                );
                let knob_r = if active_or_hov { 9.0_f32 } else { 8.0_f32 };
                painter.circle_filled(hs, knob_r, Color32::from_rgb(245, 158, 11));
                painter.circle_stroke(hs, knob_r + 1.0, Stroke::new(1.6_f32, Color32::WHITE));
                if edit.dragging_rotation {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    let shift_held = ui.input(|i| i.modifiers.shift);
                    let readout = if let Some(init) = edit.rot_initial_piece_deg {
                        let abs_deg = norm_angle_deg(init + edit.rot_applied_delta_deg);
                        crate::canvas2d::format_rotation_readout(abs_deg, false, shift_held)
                    } else {
                        crate::canvas2d::format_rotation_readout(
                            edit.rot_applied_delta_deg,
                            true,
                            shift_held,
                        )
                    };
                    let pill = Rect::from_min_size(
                        hs + egui::vec2(14.0, -24.0),
                        egui::vec2(68.0, 22.0),
                    );
                    painter.rect_filled(
                        pill,
                        6.0,
                        Color32::from_rgba_unmultiplied(15, 23, 42, 230),
                    );
                    painter.rect_stroke(
                        pill,
                        6.0,
                        Stroke::new(1.2_f32, Color32::from_rgb(245, 158, 11)),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        pill.center(),
                        egui::Align2::CENTER_CENTER,
                        readout,
                        FontId::proportional(11.5),
                        Color32::WHITE,
                    );
                } else if is_hov {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
        }
    }

    // 4. Floating Bottom-Center 3D Edit HUD Bar
    {
        let bg = if dark_mode {
            Color32::from_rgba_unmultiplied(15, 23, 42, 232)
        } else {
            Color32::from_rgba_unmultiplied(248, 250, 252, 242)
        };
        let border = if dark_mode {
            Color32::from_rgba_unmultiplied(56, 189, 248, 120)
        } else {
            Color32::from_rgba_unmultiplied(2, 132, 199, 150)
        };
        egui::Area::new(egui::Id::new("hud_3d_edit_bar"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
            .constrain_to(rect.shrink(6.0))
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(bg)
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(12, 7))
                    .stroke(Stroke::new(1.2_f32, border))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mode_title = if edit.enabled {
                                if is_de { "3D BEARBEITEN" } else { "3D EDIT MODE" }
                            } else {
                                "MINIMAL CAD 3D"
                            };
                            ui.label(
                                RichText::new(mode_title)
                                    .strong()
                                    .size(11.5)
                                    .color(Color32::from_rgb(56, 189, 248)),
                            );
                            let sel_cnt = selected.len();
                            ui.weak(if is_de {
                                format!("{sel_cnt} ausgewaehlt")
                            } else {
                                format!("{sel_cnt} selected")
                            });
                            ui.separator();

                            let add_label = match last_placed_catalog_id {
                                Some(cid) => format!("+ {cid}"),
                                None => if is_de { "+ Gleis".into() } else { "+ Track".into() },
                            };
                            if ui
                                .small_button(add_label)
                                .on_hover_text(if is_de {
                                    "Letztes Gleis hinzufuegen oder an ausgewaehltem Anschluss verlaengern"
                                } else {
                                    "Add track piece or extend selected open connector"
                                })
                                .clicked()
                            {
                                out.add_last_track_requested = true;
                            }
                            if ui.selectable_label(*snapping_enabled, "Snap").clicked() {
                                *snapping_enabled = !*snapping_enabled;
                            }
                            if ui
                                .add_enabled(sel_cnt > 0, egui::Button::new("-Rot").small())
                                .on_hover_text(if is_de { "Links drehen (Shift+R)" } else { "Rotate CCW (Shift+R)" })
                                .clicked()
                            {
                                out.rotate_delta_deg = Some(-rotation_step);
                            }
                            if ui
                                .add_enabled(sel_cnt > 0, egui::Button::new("+Rot").small())
                                .on_hover_text(if is_de { "Rechts drehen (R)" } else { "Rotate CW (R)" })
                                .clicked()
                            {
                                out.rotate_delta_deg = Some(rotation_step);
                            }
                            if ui
                                .add_enabled(sel_cnt > 0, egui::Button::new("-5mm").small())
                                .on_hover_text(if is_de { "Hoehe -5 mm (Bild ab)" } else { "Lower 5 mm (PageDown)" })
                                .clicked()
                            {
                                out.elevation_delta_mm = Some(-5.0);
                            }
                            if ui
                                .add_enabled(sel_cnt > 0, egui::Button::new("+5mm").small())
                                .on_hover_text(if is_de { "Hoehe +5 mm (Bild auf)" } else { "Raise 5 mm (PageUp)" })
                                .clicked()
                            {
                                out.elevation_delta_mm = Some(5.0);
                            }
                            if ui
                                .small_button(if is_de { "Luecke" } else { "Close Gap" })
                                .clicked()
                            {
                                out.close_gap_requested = true;
                            }
                            if ui
                                .add_enabled(
                                    sel_cnt > 0,
                                    egui::Button::new(if is_de { "Parallel" } else { "Parallel" }).small(),
                                )
                                .clicked()
                            {
                                out.parallel_requested = true;
                            }
                            if ui
                                .add_enabled(
                                    sel_cnt > 0,
                                    egui::Button::new(RichText::new(if is_de { "Loeschen" } else { "Delete" }).color(Color32::from_rgb(248, 113, 113))).small(),
                                )
                                .clicked()
                            {
                                out.delete_requested = true;
                            }
                            if edit.enabled {
                                ui.separator();
                                if ui.small_button(if is_de { "Fertig" } else { "Done" }).clicked() {
                                    exit_requested = true;
                                }
                            }
                        });
                    });
            });
    }

    let _ = norm_angle_deg(0.0);
    exit_requested
}

/// Processes direct 3D terrain sculpting when `terrain_brush.active && terrain_brush.is_heightfield_mode()`.
/// Returns `(consumed_primary_drag, consumed_scroll, Edit3DOutput)`.
pub fn handle_3d_terrain_sculpt_interaction(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &Response,
    camera: &Camera3D,
    brush: &mut crate::terrain_tools::TerrainBrushState,
    layout: &mut Layout,
) -> (bool, bool, Edit3DOutput) {
    let mut out = Edit3DOutput::default();
    if !brush.active || !brush.is_heightfield_mode() {
        return (false, false, out);
    }

    let (active_lid, active_y0, editable, _) = crate::terrain_tools::active_layer_info(layout);
    let bounds = crate::terrain_tools::active_layer_bounds(layout);
    let hover_pos = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
    let shift_held = ui.input(|i| i.modifiers.shift);
    let alt_held = ui.input(|i| i.modifiers.alt);
    let ctrl_held = ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd);

    let mut consumed_scroll = false;
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            if shift_held {
                brush.adjust_radius(if scroll > 0.0 { 15.0 } else { -15.0 });
                consumed_scroll = true;
            } else if ctrl_held {
                brush.adjust_strength(if scroll > 0.0 { 0.08 } else { -0.08 });
                consumed_scroll = true;
            }
        }
    }

    if !editable {
        brush.cursor_world = None;
        brush.is_drawing = false;
        return (false, consumed_scroll, out);
    }

    let boards: Vec<railbench_core::types::BaseboardConfig> =
        crate::board_tools::get_baseboards_slice(layout)
            .into_iter()
            .filter(|b| b.enabled)
            .collect();
    let support_polys = crate::terrain_tools::collect_terrain_support_polygons(
        layout,
        active_lid.as_deref(),
        &boards,
    );
    let (poly_offsets, poly_coords) = crate::terrain_tools::pack_support_polygons(&support_polys);
    let active_hf = crate::terrain_tools::active_layer_heightfield(layout);

    let hit_pt = hover_pos.and_then(|hp| {
        let (ro, rd) = camera.screen_to_ray(rect, hp);
        let ro_local = [ro[0], ro[1] - active_y0 as f32, ro[2]];
        let hit = railbench_core::terrain::raycast_heightfield_3d(
            ro_local,
            rd,
            active_hf,
            bounds,
        )?;
        if poly_offsets.len() >= 2
            && !railbench_core::geometry::is_point_in_any_polygon(
                hit[0] as f64,
                hit[2] as f64,
                &poly_offsets,
                &poly_coords,
            )
        {
            return None;
        }
        Some(hit)
    });
    brush.cursor_world = hit_pt.map(|p| Vec2 {
        x: p[0] as f64,
        y: p[2] as f64,
    });
    if hover_pos.is_some() && hit_pt.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }

    if response.clicked_by(egui::PointerButton::Secondary) {
        brush.set_active(false, None, None);
        return (false, consumed_scroll, out);
    }

    let mut consumed_drag = false;
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(wp) = brush.cursor_world {
            brush.is_drawing = true;
            brush.flatten_target_mm = None;
            brush.last_sculpt_world = None;
            out.push_undo = true;
            if brush.apply_live_sculpt_step(wp, layout, shift_held, alt_held) {
                out.layout_changed = true;
            }
            consumed_drag = true;
        }
    }

    if brush.is_drawing && response.dragged_by(egui::PointerButton::Primary) {
        consumed_drag = true;
        if let Some(wp) = brush.cursor_world {
            if brush.apply_live_sculpt_step(wp, layout, shift_held, alt_held) {
                out.layout_changed = true;
            }
        }
    } else if brush.is_drawing && !response.dragged_by(egui::PointerButton::Primary) {
        brush.is_drawing = false;
        brush.flatten_target_mm = None;
        brush.last_sculpt_world = None;
        if brush.auto_tunnel && crate::terrain_tools::apply_auto_tunnel_for_heightfield(layout) > 0 {
            out.layout_changed = true;
        }
        out.layout_changed = true;
    } else if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(wp) = brush.cursor_world {
            brush.flatten_target_mm = None;
            brush.last_sculpt_world = None;
            out.push_undo = true;
            if brush.apply_live_sculpt_step(wp, layout, shift_held, alt_held) {
                out.layout_changed = true;
            }
            brush.flatten_target_mm = None;
            brush.last_sculpt_world = None;
            if brush.auto_tunnel && crate::terrain_tools::apply_auto_tunnel_for_heightfield(layout) > 0 {
                out.layout_changed = true;
            }
        }
    }

    (consumed_drag, consumed_scroll, out)
}

/// Projects the terrain-conforming 3D brush reticle and draws the floating bottom-center
/// 3D Terrain Sculpt toolbar when unified heightfield sculpting is active.
#[allow(clippy::too_many_arguments)]
pub fn draw_3d_terrain_brush_overlay(
    ui: &mut egui::Ui,
    rect: Rect,
    camera: &Camera3D,
    brush: &mut crate::terrain_tools::TerrainBrushState,
    layout: &Layout,
    dark_mode: bool,
    lang: Lang,
) {
    if !brush.active || !brush.is_heightfield_mode() {
        return;
    }
    let is_de = lang == Lang::De;
    let painter = ui.painter_at(rect);
    let shift_held = ui.input(|i| i.modifiers.shift);
    let alt_held = ui.input(|i| i.modifiers.alt);
    let eff_op = brush.active_sculpt_op(shift_held, alt_held);
    let ring_col = match eff_op {
        crate::terrain_tools::SculptOp::Raise => Color32::from_rgb(34, 197, 94),
        crate::terrain_tools::SculptOp::Lower => Color32::from_rgb(249, 115, 22),
        crate::terrain_tools::SculptOp::Smooth => Color32::from_rgb(56, 189, 248),
        crate::terrain_tools::SculptOp::Flatten(_) => Color32::from_rgb(234, 179, 8),
        crate::terrain_tools::SculptOp::GradeTrack => Color32::from_rgb(168, 85, 247),
        crate::terrain_tools::SculptOp::PaintBiome(_) => Color32::from_rgb(20, 184, 166),
        crate::terrain_tools::SculptOp::Reset => Color32::from_rgb(239, 68, 68),
    };

    let (_active_lid, active_y0, editable, active_layer_name) =
        crate::terrain_tools::active_layer_info(layout);
    if let Some(cw) = brush.cursor_world {
        let active_hf = crate::terrain_tools::active_layer_heightfield(layout);
        let y_base = active_y0 as f32;
        let sample_h = |wx: f64, wz: f64| -> f32 {
            active_hf
                .map(|hf| railbench_core::terrain::sample_heightfield_bilinear(hf, wx, wz))
                .unwrap_or(0.0)
        };
        const N: usize = 28;
        let mut outer = Vec::with_capacity(N + 1);
        let mut inner = Vec::with_capacity(N + 1);
        for i in 0..=N {
            let ang = (i as f64 / N as f64) * std::f64::consts::TAU;
            let ox = cw.x + ang.cos() * brush.radius;
            let oz = cw.y + ang.sin() * brush.radius;
            let oy = y_base + sample_h(ox, oz) + 2.0;
            if let Some(sp) = camera.world_to_screen(rect, [ox as f32, oy, oz as f32]) {
                outer.push(sp);
            }
            let ix = cw.x + ang.cos() * brush.radius * 0.35;
            let iz = cw.y + ang.sin() * brush.radius * 0.35;
            let iy = y_base + sample_h(ix, iz) + 2.5;
            if let Some(sp) = camera.world_to_screen(rect, [ix as f32, iy, iz as f32]) {
                inner.push(sp);
            }
        }
        if outer.len() >= 3 {
            painter.add(egui::Shape::line(outer, Stroke::new(2.4_f32, ring_col)));
        }
        if inner.len() >= 3 {
            painter.add(egui::Shape::line(
                inner,
                Stroke::new(
                    1.4_f32,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 175),
                ),
            ));
        }
        let ch = sample_h(cw.x, cw.y);
        if let Some(csp) = camera.world_to_screen(rect, [cw.x as f32, y_base + ch + 2.5, cw.y as f32]) {
            painter.circle_filled(csp, 4.0, ring_col);
            painter.circle_stroke(csp, 4.8, Stroke::new(1.2_f32, Color32::WHITE));
            let txt = format!("H: {:.0} mm · R: {:.0} mm", ch, brush.radius);
            let pill = Rect::from_min_size(csp + egui::vec2(14.0, -26.0), egui::vec2(146.0, 21.0));
            painter.rect_filled(pill, 6.0, Color32::from_rgba_unmultiplied(15, 23, 42, 225));
            painter.rect_stroke(pill, 6.0, Stroke::new(1.1_f32, ring_col), egui::StrokeKind::Inside);
            painter.text(
                pill.center(),
                egui::Align2::CENTER_CENTER,
                txt,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
    }

    // Floating Bottom-Center 3D Terrain Sculpt HUD Bar
    let bg = if dark_mode {
        Color32::from_rgba_unmultiplied(15, 23, 42, 236)
    } else {
        Color32::from_rgba_unmultiplied(248, 250, 252, 245)
    };
    let border = Color32::from_rgba_unmultiplied(34, 197, 94, 150);
    egui::Area::new(egui::Id::new("hud_3d_terrain_sculpt_bar"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
        .constrain_to(rect.shrink(6.0))
        .show(ui.ctx(), |ui| {
            Frame::new()
                .fill(bg)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(12, 7))
                .stroke(Stroke::new(1.2_f32, border))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let hud_title = if editable {
                            format!("{} · {active_layer_name}", if is_de { "3D GELAENDE" } else { "3D TERRAIN" })
                        } else {
                            format!("🔒 {active_layer_name}")
                        };
                        ui.label(
                            RichText::new(hud_title)
                                .strong()
                                .size(11.5)
                                .color(if editable {
                                    Color32::from_rgb(34, 197, 94)
                                } else {
                                    Color32::from_rgb(245, 158, 11)
                                }),
                        );
                        ui.separator();
                        let tools = [
                            (crate::terrain_tools::TerrainBrushMode::SculptRaise, if is_de { "Berg +" } else { "Raise" }),
                            (crate::terrain_tools::TerrainBrushMode::SculptLower, if is_de { "Tal -" } else { "Lower" }),
                            (crate::terrain_tools::TerrainBrushMode::SculptSmooth, if is_de { "Glaetten" } else { "Smooth" }),
                            (crate::terrain_tools::TerrainBrushMode::SculptFlatten, "Plateau"),
                            (crate::terrain_tools::TerrainBrushMode::SculptGrade, if is_de { "Damm" } else { "Grade" }),
                            (crate::terrain_tools::TerrainBrushMode::SculptBiome, if is_de { "Biom" } else { "Biome" }),
                            (crate::terrain_tools::TerrainBrushMode::SculptReset, if is_de { "Reset" } else { "Reset" }),
                        ];
                        for (m, lbl) in tools {
                            if ui.selectable_label(brush.mode == m, lbl).clicked() {
                                brush.mode = m;
                            }
                        }
                        ui.separator();
                        ui.label("R:");
                        ui.add(
                            egui::DragValue::new(&mut brush.radius)
                                .range(40.0..=600.0)
                                .speed(4.0)
                                .suffix(" mm"),
                        );
                        let mut pct = (brush.strength * 100.0).round();
                        if ui
                            .add(
                                egui::DragValue::new(&mut pct)
                                    .range(10.0..=100.0)
                                    .speed(2.0)
                                    .suffix("%"),
                            )
                            .changed()
                        {
                            brush.strength = (pct / 100.0).clamp(0.10, 1.0);
                        }
                        if matches!(
                            brush.mode,
                            crate::terrain_tools::TerrainBrushMode::SculptBiome
                                | crate::terrain_tools::TerrainBrushMode::SculptRaise
                        ) {
                            let biome_lbl = |b: u8| match (b, is_de) {
                                (1, true) => "Gras",
                                (1, false) => "Grass",
                                (2, true) => "Wald",
                                (2, false) => "Forest",
                                (3, true) => "Fels",
                                (3, false) => "Rock",
                                (4, true) => "Schnee",
                                (4, false) => "Snow",
                                (5, _) => "Sand",
                                (6, true) => "Erde",
                                (6, false) => "Dirt",
                                (7, true) => "Wasser",
                                (7, false) => "Water",
                                _ => "Auto",
                            };
                            egui::ComboBox::from_id_salt("hud3d_sculpt_biome")
                                .width(74.0)
                                .selected_text(biome_lbl(brush.biome))
                                .show_ui(ui, |ui| {
                                    for b in 0..=7_u8 {
                                        ui.selectable_value(&mut brush.biome, b, biome_lbl(b));
                                    }
                                });
                        }
                        ui.separator();
                        if ui.small_button(if is_de { "Fertig" } else { "Done" }).clicked() {
                            brush.set_active(false, None, None);
                        }
                    });
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{builtin_catalogs, create_piece, find_entry};
    use eframe::egui::Pos2;
    use railbench_core::types::Transform;

    #[test]
    fn screen_to_ray_and_world_to_screen_roundtrip_accurately() {
        let cam = Camera3D {
            target: [120.0, 20.0, -50.0],
            yaw: 0.7,
            pitch: 0.45,
            distance: 900.0,
        };
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(1200.0, 800.0));
        let world_pt = [260.0_f32, 35.0, 80.0];
        let scr = cam.world_to_screen(rect, world_pt).expect("point in front of camera");

        let (ro, rd) = cam.screen_to_ray(rect, scr);
        let hit = ray_intersect_y_plane(ro, rd, 35.0).expect("ray hits horizontal Y=35 plane");
        assert!(
            (hit[0] - world_pt[0]).abs() < 0.8 && (hit[2] - world_pt[2]).abs() < 0.8,
            "roundtrip mismatch: expected {world_pt:?}, got {hit:?}"
        );
    }

    #[test]
    fn ray_plane_intersection_and_3d_picking_selects_track_and_model() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let p0 = create_piece(
            "track-pick".into(),
            e24188,
            Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        );
        let layout = Layout {
            name: "PickTest".into(),
            scale: "H0".into(),
            pieces: vec![p0],
            models: Some(vec![railbench_core::types::Model3D {
                id: "m-station".into(),
                name: "Bahnhof".into(),
                category: "Buildings".into(),
                transform: Transform {
                    x: 94.0,
                    y: -140.0,
                    rotation: 0.0,
                },
                elevation: Some(0.0),
                scale: Some(1.0),
                layer_id: None,
                width: 140.0,
                depth: 80.0,
                height: 65.0,
                model_type: "builtin".into(),
                builtin_id: Some("station".into()),
                mountain_style: None,
                mountain_path: None,
                ground_terrain_type: None,
                file_data: None,
                file_name: None,
            }]),
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
            terrain_field: None,
        };

        let cam = Camera3D {
            target: [94.0, 10.0, 0.0],
            yaw: 0.4,
            pitch: 0.65,
            distance: 500.0,
        };
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let scr = cam.world_to_screen(rect, [94.0, 5.5, 0.0]).unwrap();
        let (ro, rd) = cam.screen_to_ray(rect, scr);
        let picked = pick_3d_object(ro, rd, &layout, &catalogs, true);
        assert_eq!(picked.as_deref(), Some("track-pick"));

        let scr_m = cam.world_to_screen(rect, [94.0, 25.0, -140.0]).unwrap();
        let (mro1, mrd1) = cam.screen_to_ray(rect, scr_m);
        assert_eq!(
            pick_3d_object(mro1, mrd1, &layout, &catalogs, true).as_deref(),
            Some("m-station")
        );

        let miss_scr = cam.world_to_screen(rect, [94.0, 5.5, 250.0]).unwrap();
        let (mro, mrd) = cam.screen_to_ray(rect, miss_scr);
        assert!(pick_3d_object(mro, mrd, &layout, &catalogs, true).is_none());
    }

    #[test]
    fn catalog_drop_preview_3d_snaps_to_open_connector() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let p0 = create_piece(
            "anchor-track".into(),
            e24188,
            Transform {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            },
            None,
        );
        let right_end = &p0.connectors[1];
        let right_x = right_end.x;
        let right_y = right_end.y;
        let layout = Layout {
            name: "Snap3DTest".into(),
            scale: "H0".into(),
            pieces: vec![p0],
            models: None,
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
            terrain_field: None,
        };

        let drop_near_right = Vec2 {
            x: right_x + 90.0,
            y: right_y + 18.0,
        };
        let (ghost, snap_pt, elev_mm) = compute_3d_catalog_drop_preview(
            e24188,
            drop_near_right,
            0.0,
            &layout,
            &catalogs,
            true,
            140.0,
        );
        let snap_xyz = snap_pt.expect("3D drop near open connector must magnetically snap");
        assert!((snap_xyz[0] - right_x as f32).abs() < 1e-2);
        assert!((snap_xyz[2] - right_y as f32).abs() < 1e-2);
        assert!((elev_mm - 0.0).abs() < 1e-3);
        assert!(
            ghost
                .connectors
                .iter()
                .any(|c| (c.x - right_x).hypot(c.y - right_y) < 1e-3),
            "snapped ghost connector must align exactly with target connector"
        );
    }

    #[test]
    fn rotation_handle_3d_projects_valid_point_for_selected_track() {
        let catalogs = builtin_catalogs();
        let e24188 = find_entry(&catalogs, "24188").unwrap();
        let p0 = create_piece(
            "t-rot3d".into(),
            e24188,
            Transform {
                x: 40.0,
                y: -20.0,
                rotation: 0.0,
            },
            None,
        );
        let layout = Layout {
            name: "Rot3DTest".into(),
            scale: "H0".into(),
            pieces: vec![p0],
            models: None,
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
            terrain_field: None,
        };
        let mut selected = HashSet::new();
        selected.insert("t-rot3d".to_string());
        let cam = Camera3D {
            target: [134.0, 10.0, -20.0],
            yaw: 0.35,
            pitch: 0.7,
            distance: 700.0,
        };
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 700.0));
        let edit = Edit3DState::default();
        let (piv, hdl, base_ang, init_rot) =
            compute_3d_rotation_handle_world(&layout, &selected, true, cam.distance, &edit)
                .expect("selected track produces a 3D rotation handle point");
        assert!(piv.iter().all(|v| v.is_finite()) && hdl.iter().all(|v| v.is_finite()));
        assert!((piv[0] - hdl[0]).hypot(piv[2] - hdl[2]) > 40.0);
        assert!((base_ang - 270.0).abs() < 1e-4);
        assert_eq!(init_rot, Some(0.0));
        assert!(cam.world_to_screen(rect, piv).is_some());
        assert!(cam.world_to_screen(rect, hdl).is_some());
    }
}
