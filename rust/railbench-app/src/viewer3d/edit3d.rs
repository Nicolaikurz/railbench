use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, CornerRadius, FontId, Frame, Margin, Rect, Response, RichText, Stroke};
use railbench_core::geometry::{norm_angle_deg, rotate};
use railbench_core::snapping::{piece_connector_height, update_layout_connections};
use railbench_core::types::{Catalog, CatalogEntry, Elevation, Layout, Vec2};

use crate::catalog::{disconnect_cross_type_connections, find_entry, is_road_id, local_connectors};
use crate::export::sample_track_branches_world;
use crate::i18n::Lang;

use super::Camera3D;

#[derive(Debug, Clone, Default)]
pub struct Edit3DState {
    pub enabled: bool,
    pub dragging_selection: bool,
    pub dragging_vertical: bool,
    pub undo_pushed_for_drag: bool,
    pub drag_last_xz: Option<[f64; 2]>,
    pub drag_plane_y: f32,
    pub hovered_id: Option<String>,
    pub snap_preview_world: Option<[f32; 3]>,
}

pub struct Edit3DParams<'a> {
    pub state: &'a mut Edit3DState,
    pub layout: &'a mut Layout,
    pub catalogs: &'a [Catalog],
    pub selected: &'a mut HashSet<String>,
    pub dragging_catalog: Option<&'a CatalogEntry>,
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
    pub drop_catalog_at: Option<(CatalogEntry, Vec2, f64)>,
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
    snapping_enabled: bool,
    layer_protection: bool,
    _rotation_step: f64,
) -> (bool, Edit3DOutput) {
    let mut out = Edit3DOutput::default();
    if !edit.enabled {
        edit.dragging_selection = false;
        edit.dragging_vertical = false;
        edit.undo_pushed_for_drag = false;
        edit.hovered_id = None;
        edit.snap_preview_world = None;
        return (false, out);
    }

    let hover_pos = ui.input(|i| i.pointer.hover_pos()).or_else(|| {
        if dragging_catalog.is_some() {
            ui.input(|i| i.pointer.latest_pos())
        } else {
            None
        }
    });
    let layers = layer_info_map(layout);
    let active_lid = layout.active_layer_id.as_deref().unwrap_or("layer-1").to_string();
    let active_lh = layers.get(active_lid.as_str()).map(|v| v.0).unwrap_or(0.0) as f32;

    // 1. Handle Catalog Entry Drag-and-Drop directly onto the 3D plane!
    if let Some(entry) = dragging_catalog {
        if let Some(hp) = hover_pos {
            if rect.contains(hp) {
                let (ray_o, ray_d) = camera.screen_to_ray(rect, hp);
                if let Some(hit) = ray_intersect_y_plane(ray_o, ray_d, active_lh) {
                    edit.snap_preview_world = Some(hit);
                    if ui.input(|i| i.pointer.any_released()) {
                        out.drop_catalog_at = Some((
                            entry.clone(),
                            Vec2 {
                                x: hit[0] as f64,
                                y: hit[2] as f64,
                            },
                            32.0,
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
    // (`R` and `Delete` are handled cleanly once per frame by main.rs global shortcuts)
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

    // 4. Primary Click Selection
    let shift_held = ui.input(|i| i.modifiers.shift);
    let alt_held = ui.input(|i| i.modifiers.alt);
    if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(hp) = hover_pos {
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

    // 5. Primary Drag of Selected 3D Object(s) (or empty-space camera Orbit if not over selection)
    let mut consumed_drag = false;
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(hp) = hover_pos {
            let (ro, rd) = camera.screen_to_ray(rect, hp);
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

    if edit.dragging_selection && response.dragged_by(egui::PointerButton::Primary) {
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

/// Renders 3D projected selection wireframes, hover feedback, catalog drop ghost, XYZ gizmo triad,
/// and the bottom-center floating 3D Edit HUD bar.
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
    snapping_enabled: &mut bool,
    rotation_step: f64,
    train_mode_active: bool,
    dark_mode: bool,
    lang: Lang,
    out: &mut Edit3DOutput,
) -> bool {
    let mut exit_requested = false;
    if !edit.enabled {
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

    // 2. Catalog Drag Ghost Preview in 3D
    if let (Some(entry), Some(gp)) = (dragging_catalog, edit.snap_preview_world) {
        let len = entry.length.unwrap_or(188.3) as f32;
        if let (Some(a), Some(b)) = (
            camera.world_to_screen(rect, [gp[0] - len * 0.5, gp[1] + 6.0, gp[2]]),
            camera.world_to_screen(rect, [gp[0] + len * 0.5, gp[1] + 6.0, gp[2]]),
        ) {
            painter.line_segment([a, b], Stroke::new(4.0_f32, Color32::from_rgb(56, 189, 248)));
            painter.circle_filled(a, 5.0, Color32::from_rgb(250, 204, 21));
            painter.circle_filled(b, 5.0, Color32::from_rgb(250, 204, 21));
        }
        let locals = local_connectors(entry);
        let _ = locals;
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
    }

    // 4. Floating Bottom-Center 3D Edit HUD Bar (hidden while Trains 3D HUD is open)
    if !train_mode_active {
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
                            ui.label(
                                RichText::new(if is_de { "3D BEARBEITEN" } else { "3D EDIT MODE" })
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
                            ui.separator();
                            if ui.small_button(if is_de { "Fertig" } else { "Done" }).clicked() {
                                exit_requested = true;
                            }
                        });
                    });
            });
    }

    let _ = norm_angle_deg(0.0);
    exit_requested
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
}
