use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2 as EguiVec2};
use railbench_core::geometry::{
    norm_angle_deg, rotate, sample_arc_between_connectors, sample_flex_spline_points,
};
use railbench_core::types::{
    BaseboardConfig, Catalog, CatalogEntry, Layout, Model3D, TrackPiece, Transform, Vec2,
};

use crate::board_tools::{
    self, BaseboardSnapGuide, BoardBounds, BoardSketcherState, SketchMode, TrackCutoutPolygon,
};
use crate::catalog::find_entry;
use crate::power_sim::{PiecePowerResult, PowerSimResult, PowerStatus};
use crate::terrain_tools::{evaluate_mountain_local, world_to_mountain_local, TerrainBrushMode, TerrainBrushState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Types,
    Layers,
    Off,
}

#[derive(Debug, Clone)]
pub struct Camera2D {
    pub offset: EguiVec2,
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            offset: EguiVec2::new(450.0, 320.0),
            zoom: 0.45,
        }
    }
}

impl Camera2D {
    pub fn world_to_screen(&self, rect: Rect, wx: f64, wy: f64) -> Pos2 {
        Pos2::new(
            rect.min.x + self.offset.x + (wx as f32) * self.zoom,
            rect.min.y + self.offset.y + (wy as f32) * self.zoom,
        )
    }

    pub fn screen_to_world(&self, rect: Rect, sp: Pos2) -> Vec2 {
        Vec2 {
            x: ((sp.x - rect.min.x - self.offset.x) / self.zoom) as f64,
            y: ((sp.y - rect.min.y - self.offset.y) / self.zoom) as f64,
        }
    }

    pub fn fit_layout(&mut self, rect: Rect, layout: &Layout) {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for p in &layout.pieces {
            for c in &p.connectors {
                min_x = min_x.min(c.x);
                max_x = max_x.max(c.x);
                min_y = min_y.min(c.y);
                max_y = max_y.max(c.y);
            }
        }
        for bb in board_tools::get_baseboards_slice(layout) {
            if !bb.enabled {
                continue;
            }
            for pt in board_tools::get_baseboard_polygon(&bb) {
                min_x = min_x.min(pt.x);
                max_x = max_x.max(pt.x);
                min_y = min_y.min(pt.y);
                max_y = max_y.max(pt.y);
            }
        }

        if !min_x.is_finite() || !max_x.is_finite() {
            self.zoom = 0.5;
            self.offset = EguiVec2::new(rect.width() * 0.5, rect.height() * 0.5);
            return;
        }

        let span_x = (max_x - min_x).max(200.0) + 180.0;
        let span_y = (max_y - min_y).max(200.0) + 180.0;
        let zx = rect.width() as f64 / span_x;
        let zy = rect.height() as f64 / span_y;
        self.zoom = (zx.min(zy) as f32).clamp(0.05, 4.0);
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        self.offset = EguiVec2::new(
            rect.width() * 0.5 - (cx as f32) * self.zoom,
            rect.height() * 0.5 - (cy as f32) * self.zoom,
        );
    }
}

#[derive(Default)]
pub struct CanvasInteraction {
    pub hover_world: Option<Vec2>,
    pub changed: bool,
    pub push_undo: bool,
    pub drop_catalog_at: Option<(Vec2, f64)>,
    pub drop_model_at: Option<Vec2>,
    pub quick_add_at: Option<(String, usize)>,
    pub trigger_uncoupler_piece: Option<String>,
    pub toggle_uncoupler_armed: Option<String>,
    pub rotate_delta: Option<f64>,
    pub copy_selected: bool,
    pub paste_clipboard: bool,
    pub duplicate_selected: bool,
    pub flip_selected: bool,
    pub toggle_group: bool,
    pub select_all: bool,
    pub fit_view: bool,
    pub close_track_gap: bool,
    pub generate_parallel_tracks: bool,
    pub select_connected: bool,
    pub select_layer: bool,
    pub move_to_layer: Option<String>,
    pub open_height_dialog: bool,
    pub auto_ramp: bool,
    pub open_replace_split: Option<String>,
    pub reset_flex: Option<String>,
    pub open_track_props: Option<String>,
    pub toggle_feeder: bool,
    pub toggle_isolator: bool,
    pub toggle_catenary: bool,
    pub toggle_catenary_feeder: bool,
    pub toggle_catenary_isolator: bool,
    pub toggle_lantern: bool,
    pub toggle_decoder: bool,
    pub toggle_signal: bool,
    pub cycle_signal_side: bool,
    pub cycle_signal_direction: bool,
    pub toggle_bridge: bool,
    pub toggle_tunnel: bool,
    pub delete_selected: bool,
}

pub struct HeightPanelOverlay<'a> {
    pub tab: u8,
    pub ramp_sel: &'a mut crate::ramp_tools::RampSelectionState,
    pub ramp_start_h: &'a mut f64,
    pub ramp_end_h: &'a mut f64,
    pub gen_pick_mode: &'a mut bool,
    pub gen_anchor_piece_id: &'a mut Option<String>,
    pub gen_conn_idx: &'a mut Option<usize>,
    pub gen_preview_pieces: &'a [TrackPiece],
    pub gen_start_h: f64,
    pub gen_target_h: f64,
}

#[derive(Debug, Clone)]
pub struct TrackRotationDrag {
    pub pivot: Vec2,
    pub start_pointer_angle_deg: f64,
    pub applied_delta_deg: f64,
    pub base_handle_angle_deg: f64,
    pub single_piece_id: Option<String>,
    pub initial_piece_rot_deg: f64,
}

/// Computes the world-space pivot, interactive rotation handle point, base handle angle,
/// single piece ID (when exactly one editable track piece is selected), and initial rotation
/// for any 2D selection containing at least one editable track piece.
pub fn compute_track_selection_rotation_handle(
    layout: &Layout,
    selected: &HashSet<String>,
    layer_protection: bool,
    zoom: f32,
    active_drag: Option<&TrackRotationDrag>,
) -> Option<(Vec2, Vec2, f64, Option<String>, f64)> {
    if selected.is_empty() {
        return None;
    }

    let editable_pieces: Vec<&TrackPiece> = layout
        .pieces
        .iter()
        .filter(|p| {
            selected.contains(&p.id)
                && is_layer_editable_with_protection(layout, p.layer_id.as_deref(), layer_protection)
        })
        .collect();
    if editable_pieces.is_empty() {
        return None;
    }

    let editable_models: Vec<&Model3D> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| {
                    selected.contains(&m.id)
                        && !matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain"))
                        && is_layer_editable_with_protection(
                            layout,
                            m.layer_id.as_deref(),
                            layer_protection,
                        )
                })
                .collect()
        })
        .unwrap_or_default();

    if editable_pieces.len() == 1 && editable_models.is_empty() {
        let p = editable_pieces[0];
        let pivot = if p.connectors.is_empty() {
            Vec2 {
                x: p.transform.x,
                y: p.transform.y,
            }
        } else {
            let n = p.connectors.len() as f64;
            Vec2 {
                x: p.connectors.iter().map(|c| c.x).sum::<f64>() / n,
                y: p.connectors.iter().map(|c| c.y).sum::<f64>() / n,
            }
        };
        let rot = p.transform.rotation;
        let rad = rot.to_radians();
        let dir = (rad.sin(), -rad.cos());
        let max_proj = p
            .connectors
            .iter()
            .map(|c| ((c.x - pivot.x) * dir.0 + (c.y - pivot.y) * dir.1).abs())
            .fold(0.0_f64, f64::max);
        let stem_dist = (max_proj + 34.0).max(46.0 / (zoom as f64).max(0.15));
        let handle_world = Vec2 {
            x: pivot.x + dir.0 * stem_dist,
            y: pivot.y + dir.1 * stem_dist,
        };
        let base_handle_angle_deg = norm_angle_deg(rot - 90.0);
        return Some((
            pivot,
            handle_world,
            base_handle_angle_deg,
            Some(p.id.clone()),
            rot,
        ));
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in &editable_pieces {
        if p.connectors.is_empty() {
            min_x = min_x.min(p.transform.x);
            max_x = max_x.max(p.transform.x);
            min_y = min_y.min(p.transform.y);
            max_y = max_y.max(p.transform.y);
        } else {
            for c in &p.connectors {
                min_x = min_x.min(c.x);
                max_x = max_x.max(c.x);
                min_y = min_y.min(c.y);
                max_y = max_y.max(c.y);
            }
        }
    }
    for m in &editable_models {
        min_x = min_x.min(m.transform.x);
        max_x = max_x.max(m.transform.x);
        min_y = min_y.min(m.transform.y);
        max_y = max_y.max(m.transform.y);
    }

    let pivot = active_drag.map(|d| d.pivot).unwrap_or(Vec2 {
        x: (min_x + max_x) * 0.5,
        y: (min_y + max_y) * 0.5,
    });
    let half_h = ((max_y - min_y) * 0.5).max(20.0);
    let stem_dist = (half_h + 34.0).max(52.0 / (zoom as f64).max(0.15));
    let ang_deg = active_drag
        .map(|d| d.base_handle_angle_deg + d.applied_delta_deg)
        .unwrap_or(-90.0);
    let rad = ang_deg.to_radians();
    let handle_world = Vec2 {
        x: pivot.x + rad.cos() * stem_dist,
        y: pivot.y + rad.sin() * stem_dist,
    };
    Some((pivot, handle_world, -90.0, None, 0.0))
}

/// Rotates all editable selected tracks and models rigidly by `delta_deg` around `pivot`.
pub fn rotate_selection_around_pivot(
    layout: &mut Layout,
    selected: &HashSet<String>,
    layer_protection: bool,
    pivot: Vec2,
    delta_deg: f64,
) -> bool {
    if selected.is_empty() || delta_deg.abs() <= 1e-9 {
        return false;
    }

    let editable_piece_ids: HashSet<String> = layout
        .pieces
        .iter()
        .filter(|p| {
            selected.contains(&p.id)
                && is_layer_editable_with_protection(layout, p.layer_id.as_deref(), layer_protection)
        })
        .map(|p| p.id.clone())
        .collect();
    let editable_model_ids: HashSet<String> = layout
        .models
        .as_ref()
        .map(|ms| {
            ms.iter()
                .filter(|m| {
                    selected.contains(&m.id)
                        && is_layer_editable_with_protection(
                            layout,
                            m.layer_id.as_deref(),
                            layer_protection,
                        )
                })
                .map(|m| m.id.clone())
                .collect()
        })
        .unwrap_or_default();

    if editable_piece_ids.is_empty() && editable_model_ids.is_empty() {
        return false;
    }

    for p in &mut layout.pieces {
        if !editable_piece_ids.contains(&p.id) {
            continue;
        }
        let rp = rotate(
            Vec2 {
                x: p.transform.x - pivot.x,
                y: p.transform.y - pivot.y,
            },
            delta_deg,
        );
        p.transform.x = pivot.x + rp.x;
        p.transform.y = pivot.y + rp.y;
        p.transform.rotation = norm_angle_deg(p.transform.rotation + delta_deg);
        for c in &mut p.connectors {
            let rc = rotate(
                Vec2 {
                    x: c.x - pivot.x,
                    y: c.y - pivot.y,
                },
                delta_deg,
            );
            c.x = pivot.x + rc.x;
            c.y = pivot.y + rc.y;
            c.angle = norm_angle_deg(c.angle + delta_deg);
            c.connected_to = None;
        }
    }
    if let Some(models) = &mut layout.models {
        for m in models {
            if !editable_model_ids.contains(&m.id) {
                continue;
            }
            let rm = rotate(
                Vec2 {
                    x: m.transform.x - pivot.x,
                    y: m.transform.y - pivot.y,
                },
                delta_deg,
            );
            m.transform.x = pivot.x + rm.x;
            m.transform.y = pivot.y + rm.y;
            m.transform.rotation = norm_angle_deg(m.transform.rotation + delta_deg);
        }
    }
    true
}

/// Snaps a cumulative drag rotation angle delta (in degrees) to the active toolbar `rotation_step`
/// unless free rotation (`Shift`) is held or `rotation_step <= 0`.
pub fn snap_drag_rotation_delta(raw_delta_deg: f64, rotation_step: f64, free_rot: bool) -> f64 {
    if free_rot || rotation_step <= 1e-6 {
        raw_delta_deg
    } else {
        (raw_delta_deg / rotation_step).round() * rotation_step
    }
}

/// Keeps the cumulative drag delta continuous across the ±180° pointer wrap boundary,
/// even for non-divisor fractional steps such as `5.7°` or `24.3°`.
pub fn unwrap_drag_angle_delta(
    cur_ang_deg: f64,
    start_ang_deg: f64,
    prev_applied_delta: f64,
) -> f64 {
    let mut diff = cur_ang_deg - start_ang_deg - prev_applied_delta;
    while diff > 180.0 {
        diff -= 360.0;
    }
    while diff <= -180.0 {
        diff += 360.0;
    }
    prev_applied_delta + diff
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TrackAccessoryButtonKind {
    Feeder,
    Isolator,
    Uncoupler,
}

#[derive(Debug, Clone)]
pub(super) struct TrackAccessoryButton {
    pub(super) piece_id: String,
    pub(super) kind: TrackAccessoryButtonKind,
    pub(super) screen_center: Pos2,
    pub(super) radius_px: f32,
    pub(super) active: bool,
}

pub(super) fn hit_test_accessory_button(
    buttons: &[TrackAccessoryButton],
    sp: Pos2,
) -> Option<&TrackAccessoryButton> {
    buttons
        .iter()
        .rev()
        .find(|b| sp.distance(b.screen_center) <= (b.radius_px + 3.0).max(11.0))
}

pub(super) fn toggle_track_accessory_on_piece(
    layout: &mut Layout,
    piece_id: &str,
    kind: TrackAccessoryButtonKind,
) -> bool {
    let Some(p) = layout.pieces.iter_mut().find(|pc| pc.id == piece_id) else {
        return false;
    };
    match kind {
        TrackAccessoryButtonKind::Feeder => {
            let next = p.power_feed != Some(true);
            p.power_feed = Some(next);
            true
        }
        TrackAccessoryButtonKind::Isolator => {
            let next = p.is_isolator != Some(true);
            p.is_isolator = Some(next);
            p.isolated_connector_index = if next {
                Some(p.isolated_connector_index.unwrap_or(0))
            } else {
                None
            };
            true
        }
        TrackAccessoryButtonKind::Uncoupler => false,
    }
}

/// Formats the live angle HUD readout for track rotation handles, preserving decimals for
/// fractional toolbar steps (`5.7°`, `7.5°`, `24.3°`) and free (`Shift`) rotation.
pub fn format_rotation_readout(deg: f64, signed: bool, free_rot: bool) -> String {
    let clean = if deg.abs() < 0.05 { 0.0 } else { deg };
    if !free_rot && (clean - clean.round()).abs() < 0.05 {
        let rounded = if clean.round().abs() < 0.05 {
            0.0
        } else {
            clean.round()
        };
        if signed {
            format!("{rounded:+.0}°")
        } else {
            format!("{rounded:.0}°")
        }
    } else if signed {
        format!("{clean:+.1}°")
    } else {
        format!("{clean:.1}°")
    }
}

#[inline]
pub(super) fn aabb_intersects(aabb: (f64, f64, f64, f64), min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> bool {
    aabb.2 >= min_x && aabb.0 <= max_x && aabb.3 >= min_y && aabb.1 <= max_y
}

#[inline]
pub(super) fn point_in_padded_aabb(aabb: (f64, f64, f64, f64), x: f64, y: f64, pad: f64) -> bool {
    x >= aabb.0 - pad && x <= aabb.2 + pad && y >= aabb.1 - pad && y <= aabb.3 + pad
}

#[derive(Clone, Copy)]
pub(crate) enum PieceHitShape {
    BranchesOnly,
    Circle { cx: f64, cy: f64, radius: f64 },
    OrientedRect {
        origin: Vec2,
        rot_deg: f64,
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    },
}

pub(crate) struct CachedPieceGeom {
    pub(super) branches: Vec<Vec<Vec2>>,
    pub(super) hit_shape: PieceHitShape,
    pub(super) min_x: f64,
    pub(super) min_y: f64,
    pub(super) max_x: f64,
    pub(super) max_y: f64,
}

pub(super) struct CachedTriMesh {
    pub(super) outer: Vec<Vec2>,
    pub(super) holes: Vec<Vec<Vec2>>,
    pub(super) tris: Vec<[Vec2; 3]>,
}

pub(super) struct CachedBaseboard2D {
    pub(super) id: String,
    pub(super) name: Option<String>,
    pub(super) style: String,
    pub(super) show_dimensions: Option<bool>,
    pub(super) bounds: BoardBounds,
    pub(super) anchor: Vec2,
    pub(super) mesh: CachedTriMesh,
    pub(super) hole_tris: Vec<Vec<[Vec2; 3]>>,
}

pub(super) struct CachedLayerPlate2D {
    pub(super) style: String,
    pub(super) name: Option<String>,
    pub(super) centroid: Vec2,
    pub(super) is_table_mode: bool,
    pub(super) mesh: CachedTriMesh,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct ModelTopMeshKey {
    pub(super) kind_key: String,
    pub(super) w_mm10: i32,
    pub(super) d_mm10: i32,
    pub(super) h_mm10: i32,
}

pub(super) fn custom_model_content_key(m: &Model3D) -> String {
    if let Some(fd) = m.file_data.as_deref() {
        let mut h = 0xcbf29ce484222325_u64;
        if let Some(fname) = m.file_name.as_deref() {
            for &b in fname.as_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
        h ^= 0xff;
        for &b in fd.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        format!("custom:{h:016x}:{}", fd.len())
    } else {
        format!("custom:{}:{}", m.id, m.file_name.as_deref().unwrap_or(""))
    }
}

#[derive(Clone, Default)]
pub(crate) struct CachedModelTopMesh2D {
    /// Local-space (unrotated, centered at origin) triangles sorted by 3D elevation ascending,
    /// with pre-lit top-down 3D viewer vertex colors (compact `f32` XZ pairs = 36 B/tri).
    pub(crate) tris: Vec<([[f32; 2]; 3], [Color32; 3])>,
}


mod draw;
pub use draw::draw_canvas_2d;
mod interaction;
mod overlays_and_menu;
mod cache;
#[allow(unused_imports)]
use cache::*;

pub use cache::{Canvas2DCache};
#[allow(unused_imports)]
pub(crate) use cache::{build_model_top_view_mesh_2d, build_special_piece_top_view_mesh_2d};
mod boards_and_grid;
#[allow(unused_imports)]
use boards_and_grid::*;
#[allow(unused_imports)]
pub use boards_and_grid::{parse_hex_color};
mod render_special;
#[allow(unused_imports)]
use render_special::*;
mod render_tracks;
#[allow(unused_imports)]
use render_tracks::*;
mod render_models;
#[allow(unused_imports)]
use render_models::*;
#[allow(unused_imports)]
pub use render_models::{is_layer_editable_with_protection};
mod render_vehicles;
mod snapping;
#[allow(unused_imports)]
use snapping::*;
#[allow(unused_imports)]
pub use snapping::{snap_selected_pieces};
mod terrain_maps;
#[cfg(test)]
mod tests;
