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
    pub changed: bool,
    pub push_undo: bool,
    pub drop_catalog_at: Option<(Vec2, f64)>,
    pub quick_add_at: Option<(String, usize)>,
    pub rotate_delta: Option<f64>,
    pub copy_selected: bool,
    pub duplicate_selected: bool,
    pub select_connected: bool,
    pub select_layer: bool,
    pub move_to_layer: Option<String>,
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

#[inline]
fn aabb_intersects(aabb: (f64, f64, f64, f64), min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> bool {
    aabb.2 >= min_x && aabb.0 <= max_x && aabb.3 >= min_y && aabb.1 <= max_y
}

#[inline]
fn point_in_padded_aabb(aabb: (f64, f64, f64, f64), x: f64, y: f64, pad: f64) -> bool {
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
    branches: Vec<Vec<Vec2>>,
    hit_shape: PieceHitShape,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

struct CachedTriMesh {
    outer: Vec<Vec2>,
    holes: Vec<Vec<Vec2>>,
    tris: Vec<[Vec2; 3]>,
}

struct CachedBaseboard2D {
    id: String,
    name: Option<String>,
    style: String,
    show_dimensions: Option<bool>,
    bounds: BoardBounds,
    anchor: Vec2,
    mesh: CachedTriMesh,
    hole_tris: Vec<Vec<[Vec2; 3]>>,
}

struct CachedLayerPlate2D {
    style: String,
    name: Option<String>,
    centroid: Vec2,
    is_table_mode: bool,
    mesh: CachedTriMesh,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ModelTopMeshKey {
    kind_key: String,
    w_mm10: i32,
    d_mm10: i32,
    h_mm10: i32,
}

fn custom_model_content_key(m: &Model3D) -> String {
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

#[allow(clippy::too_many_arguments)]
pub fn draw_canvas_2d(
    ui: &mut egui::Ui,
    layout: &mut Layout,
    layout_version: u64,
    cache: &mut Canvas2DCache,
    catalogs: &[Catalog],
    camera: &mut Camera2D,
    selected: &mut HashSet<String>,
    dragging_active: &mut bool,
    drag_last_world: &mut Option<Vec2>,
    dragging_flex_handle: &mut Option<(String, usize)>,
    marquee_start: &mut Option<Pos2>,
    measure_points: &mut Option<(Vec2, Vec2)>,
    dragging_catalog: Option<&CatalogEntry>,
    dark_mode: bool,
    color_mode: ColorMode,
    show_labels: bool,
    show_connectors: bool,
    show_rulers: bool,
    snapping_enabled: bool,
    layer_protection: bool,
    power_sim_enabled: bool,
    power_sim: Option<&PowerSimResult>,
    measure_mode: bool,
    rotation_step: f64,
    terrain_brush: &mut TerrainBrushState,
    is_de: bool,
    next_piece_seq: &mut u64,
    active_board_id: &mut Option<String>,
    dragging_board: &mut Option<(String, Vec2, f64, f64)>,
    board_snap_guides: &mut Vec<BaseboardSnapGuide>,
    show_wood_plates: bool,
    sketcher: &mut BoardSketcherState,
    dragging_model_rot: &mut Option<String>,
    ramp_overlay: Option<(&mut crate::ramp_tools::RampSelectionState, f64, f64)>,
) -> CanvasInteraction {
    let mut interaction = CanvasInteraction::default();

    let avail = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Background (matches variables.css --canvas-bg)
    let bg = crate::theme::palette(dark_mode).canvas_bg;
    painter.rect_filled(rect, 0.0, bg);

    // Scroll wheel: Shift+Wheel adjusts terrain brush radius when active; otherwise zoom
    if response.hovered() {
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.01 {
            if terrain_brush.active && ui.input(|i| i.modifiers.shift) {
                terrain_brush.adjust_radius(if scroll_y > 0.0 { 20.0 } else { -20.0 });
            } else {
                let factor = (1.0 + scroll_y * 0.0018).clamp(0.75, 1.35);
                if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                    let before = camera.screen_to_world(rect, pointer);
                    camera.zoom = (camera.zoom * factor).clamp(0.04, 15.0);
                    camera.offset.x = pointer.x - rect.min.x - (before.x as f32) * camera.zoom;
                    camera.offset.y = pointer.y - rect.min.y - (before.y as f32) * camera.zoom;
                }
            }
        }
    }

    // Pan with middle or secondary mouse button; single secondary click exits terrain brush
    if response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        camera.offset += response.drag_delta();
    } else if terrain_brush.active && response.clicked_by(egui::PointerButton::Secondary) {
        terrain_brush.set_active(false, None, None);
    }

    cache.ensure_fresh(layout, catalogs, layout_version);

    let view_pad_world = (48.0 / camera.zoom.max(0.04)) as f64;
    let view_tl = camera.screen_to_world(rect, rect.min);
    let view_br = camera.screen_to_world(rect, rect.max);
    let view_min_x = view_tl.x.min(view_br.x) - view_pad_world;
    let view_max_x = view_tl.x.max(view_br.x) + view_pad_world;
    let view_min_y = view_tl.y.min(view_br.y) - view_pad_world;
    let view_max_y = view_tl.y.max(view_br.y) + view_pad_world;

    // 1. Millimeter Grid
    draw_grid(&painter, rect, camera, dark_mode);

    // 2. Baseboards & multi-table snap guides
    let board_header_rects = draw_baseboards(
        &painter,
        rect,
        camera,
        &cache.baseboards,
        active_board_id.as_deref(),
        board_snap_guides,
        dark_mode,
    );

    let active_layer_id = layout
        .active_layer_id
        .as_deref()
        .unwrap_or("layer-1")
        .to_string();

    // Build ordered list of visible layers from bottom (lowest height_mm) to top (highest height_mm)
    let mut visible_layers: Vec<(String, f64, Option<String>)> = match &layout.layers {
        Some(ls) if !ls.is_empty() => ls
            .iter()
            .filter(|l| l.visible)
            .map(|l| (l.id.clone(), l.height_mm.unwrap_or(0.0), l.color.clone()))
            .collect(),
        _ => vec![("layer-1".into(), 0.0, None)],
    };
    visible_layers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    if show_wood_plates {
        cache.ensure_layer_plates(layout, catalogs);
    }
    let models_slice: &[Model3D] = layout.models.as_deref().unwrap_or(&[]);

    // Render layer-by-layer from bottom to top:
    // within each layer: wood sub-plates -> ground terrain -> tracks -> buildings/models/mountains above tracks
    for (lid, _lh, lcolor) in &visible_layers {
        let is_active_layer = lid.as_str() == active_layer_id.as_str();

        if show_wood_plates {
            draw_layer_wood_plates(
                &painter,
                rect,
                camera,
                layout,
                cache,
                catalogs,
                lid,
                is_active_layer,
                lcolor.as_deref(),
                (view_min_x, view_min_y, view_max_x, view_max_y),
            );
        }

        // 2.5A. Ground terrain (lakes, rivers, meadows) renders BELOW tracks
        if let Some(m_indices) = cache.layer_ground_models.get(lid.as_str()) {
            for &mi in m_indices {
                if !aabb_intersects(
                    cache.model_bounds[mi],
                    view_min_x,
                    view_min_y,
                    view_max_x,
                    view_max_y,
                ) {
                    continue;
                }
                let m = &models_slice[mi];
                let is_sel = selected.contains(&m.id);
                draw_model_2d(
                    &painter,
                    rect,
                    camera,
                    m,
                    cache.model_top_meshes.get(mi).map(|arc| arc.as_ref()),
                    is_sel,
                    is_active_layer,
                    show_labels,
                );
            }
        }

        // 3. Tracks on this layer (pre-sorted by piece elevation; culled to viewport AABB)
        if let Some(p_indices) = cache.layer_piece_indices.get(lid.as_str()) {
            for &pi in p_indices {
                let pg = &cache.pieces[pi];
                if !aabb_intersects(
                    (pg.min_x, pg.min_y, pg.max_x, pg.max_y),
                    view_min_x,
                    view_min_y,
                    view_max_x,
                    view_max_y,
                ) {
                    continue;
                }
                let piece = &layout.pieces[pi];
                let is_sel = selected.contains(&piece.id);
                let entry = find_entry(catalogs, &piece.catalog_id);
                let piece_power = power_sim.and_then(|sim| sim.results.get(&piece.id));
                let special_top_mesh = if entry.map_or(false, |e| {
                    matches!(e.kind.as_str(), "turntable" | "transfer-table")
                }) {
                    cache
                        .piece_top_proto_cache
                        .get(&(piece.catalog_id.clone(), piece.has_tunnel == Some(true)))
                        .map(|arc| arc.as_ref())
                } else {
                    None
                };
                draw_track_piece(
                    &painter,
                    rect,
                    camera,
                    piece,
                    &pg.branches,
                    &layout.pieces,
                    &cache.piece_id_to_idx,
                    entry,
                    special_top_mesh,
                    is_sel,
                    is_active_layer,
                    color_mode,
                    lcolor.as_deref(),
                    show_labels,
                    power_sim_enabled,
                    piece_power,
                );
            }
        }

        // 3.5. Buildings, trees, signals, lamps & mountains render ABOVE tracks on this layer
        if let Some(m_indices) = cache.layer_top_models.get(lid.as_str()) {
            for &mi in m_indices {
                if !aabb_intersects(
                    cache.model_bounds[mi],
                    view_min_x,
                    view_min_y,
                    view_max_x,
                    view_max_y,
                ) {
                    continue;
                }
                let m = &models_slice[mi];
                let is_sel = selected.contains(&m.id);
                draw_model_2d(
                    &painter,
                    rect,
                    camera,
                    m,
                    cache.model_top_meshes.get(mi).map(|arc| arc.as_ref()),
                    is_sel,
                    is_active_layer,
                    show_labels,
                );
            }
        }
    }

    // Short-circuit warning markers & banner: ALWAYS visible on 2-wire short circuits (even when Power Overlay is off)
    if let Some(sim) = power_sim {
        let show_sc = sim.has_short_circuit && (power_sim_enabled || sim.is_two_wire);
        if show_sc {
            for sc in &sim.short_circuit_points {
                let sp = camera.world_to_screen(rect, sc.x, sc.y);
                if !rect.expand(24.0).contains(sp) {
                    continue;
                }
                painter.circle_filled(sp, 11.0, Color32::from_rgb(220, 38, 38));
                painter.circle_stroke(sp, 12.5, Stroke::new(2.0_f32, Color32::WHITE));
                painter.line_segment(
                    [Pos2::new(sp.x - 4.5, sp.y - 4.5), Pos2::new(sp.x + 4.5, sp.y + 4.5)],
                    Stroke::new(2.2_f32, Color32::WHITE),
                );
                painter.line_segment(
                    [Pos2::new(sp.x + 4.5, sp.y - 4.5), Pos2::new(sp.x - 4.5, sp.y + 4.5)],
                    Stroke::new(2.2_f32, Color32::WHITE),
                );
                let tag_pos = Pos2::new(sp.x, sp.y - 21.0);
                let tag_r = Rect::from_center_size(tag_pos, EguiVec2::new(56.0, 16.0));
                painter.rect_filled(tag_r, 4.0, Color32::from_rgb(153, 27, 27));
                painter.text(tag_pos, egui::Align2::CENTER_CENTER, "SHORT!", FontId::proportional(9.5), Color32::WHITE);
            }

            if sim.is_two_wire {
                let cnt = sim.short_circuit_points.len().max(1);
                let warn_msg = if is_de {
                    format!("⚠ 2-Leiter Kurzschluss ({cnt} Stelle(n)) – Trennstellen (Isolator) setzen!")
                } else {
                    format!("⚠ 2-Wire Short Circuit ({cnt} point(s)) – place Isolator tracks to break the loop!")
                };
                let banner = Rect::from_center_size(
                    Pos2::new(rect.center().x, rect.min.y + 36.0),
                    EguiVec2::new(470.0, 28.0),
                );
                painter.rect_filled(banner, 8.0, Color32::from_rgba_unmultiplied(153, 27, 27, 240));
                painter.rect_stroke(
                    banner,
                    8.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(252, 165, 165)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    banner.center(),
                    egui::Align2::CENTER_CENTER,
                    warn_msg,
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
        }
    }

    // 4. Connectors (snap points) & Connector '+' Quick-Add buttons
    let mut plus_buttons: Vec<(String, usize, Pos2, bool)> = Vec::new();
    let hover_pos = ui.input(|i| i.pointer.hover_pos());

    let mut hovered_open_conn: Option<(String, usize)> = None;
    if show_connectors && !terrain_brush.active && !measure_mode && dragging_catalog.is_none() {
        if let Some(hp) = hover_pos {
            let hit_r = (16.0 * camera.zoom.sqrt()).clamp(15.0, 24.0);
            let hp_world = camera.screen_to_world(rect, hp);
            let pad_world = (hit_r as f64 / camera.zoom.max(0.04) as f64) + 32.0;
            let mut best_d = f32::INFINITY;
            for (pidx, piece) in layout.pieces.iter().enumerate() {
                let editable = is_layer_editable_with_protection(
                    layout,
                    piece.layer_id.as_deref(),
                    layer_protection,
                );
                if !editable {
                    continue;
                }
                let pg = &cache.pieces[pidx];
                if !point_in_padded_aabb((pg.min_x, pg.min_y, pg.max_x, pg.max_y), hp_world.x, hp_world.y, pad_world) {
                    continue;
                }
                let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
                let lh = cache.layer_heights.get(lid).copied().unwrap_or(0.0);
                for (cidx, conn) in piece.connectors.iter().enumerate() {
                    if conn.connected_to.is_some() {
                        continue;
                    }
                    let conn_h = railbench_core::snapping::piece_connector_height(piece, cidx, lh);
                    let conn_sp = camera.world_to_screen(rect, conn.x, conn.y);
                    let rad = conn.angle.to_radians();
                    let plus_sp = camera.world_to_screen(
                        rect,
                        conn.x + 28.0 * rad.cos(),
                        conn.y + 28.0 * rad.sin(),
                    );
                    let dc = hp.distance(conn_sp);
                    let dp = hp.distance(plus_sp);
                    let d = dc.min(dp);
                    if d <= hit_r
                        && d < best_d
                        && !crate::building_snap::is_pos_near_any_door(
                            conn.x,
                            conn.y,
                            conn_h,
                            &cache.door_positions,
                            6.0,
                        )
                    {
                        best_d = d;
                        hovered_open_conn = Some((piece.id.clone(), cidx));
                    }
                }
            }
        }
    }

    if show_connectors {
        let conn_clip_plus = rect.expand(24.0 + 32.0 * camera.zoom);
        let conn_clip = rect.expand(12.0);
        for (pidx, piece) in layout.pieces.iter().enumerate() {
            let vis = piece
                .layer_id
                .as_deref()
                .and_then(|lid| cache.layer_flags.get(lid).copied())
                .map_or_else(|| is_layer_visible(layout, piece.layer_id.as_deref()), |(v, _)| v);
            let editable = is_layer_editable_with_protection(
                layout,
                piece.layer_id.as_deref(),
                layer_protection,
            );
            if !vis {
                continue;
            }
            let pg = &cache.pieces[pidx];
            if !aabb_intersects(
                (pg.min_x, pg.min_y, pg.max_x, pg.max_y),
                view_min_x,
                view_min_y,
                view_max_x,
                view_max_y,
            ) {
                continue;
            }
            let is_sel = selected.contains(&piece.id);
            let lid = piece.layer_id.as_deref().unwrap_or("layer-1");
            let lh = cache.layer_heights.get(lid).copied().unwrap_or(0.0);
            for (cidx, conn) in piece.connectors.iter().enumerate() {
                let sp = camera.world_to_screen(rect, conn.x, conn.y);
                let in_conn = conn_clip.contains(sp);
                let is_hovered_conn = hovered_open_conn
                    .as_ref()
                    .map_or(false, |(pid, ci)| pid == &piece.id && *ci == cidx);
                let may_draw_plus = editable
                    && conn.connected_to.is_none()
                    && (is_hovered_conn || (is_sel && piece.connectors.len() < 6));
                if !in_conn && (!may_draw_plus || !conn_clip_plus.contains(sp)) {
                    continue;
                }
                let conn_h = railbench_core::snapping::piece_connector_height(piece, cidx, lh);
                let docked_to_bldg = crate::building_snap::is_pos_near_any_door(
                    conn.x,
                    conn.y,
                    conn_h,
                    &cache.door_positions,
                    6.0,
                );
                if in_conn {
                    let r = (4.0 * camera.zoom).clamp(2.5, 6.5);
                    if conn.connected_to.is_some() || docked_to_bldg {
                        let fill = if docked_to_bldg {
                            Color32::from_rgb(2, 132, 199)
                        } else {
                            Color32::from_rgb(100, 116, 139)
                        };
                        painter.circle_filled(sp, r * 0.78, fill);
                    } else if cache.level_mismatch_conns.contains(&(pidx, cidx)) {
                        // Snap point has a matching connector in 2D that is on a different vertical level -> mark yellow
                        painter.circle_filled(sp, r * 1.15, Color32::from_rgb(234, 179, 8));
                        painter.circle_stroke(
                            sp,
                            r * 1.15 + 1.6,
                            Stroke::new(1.4_f32, Color32::from_rgb(250, 204, 21)),
                        );
                    } else {
                        painter.circle_filled(sp, r, Color32::from_rgb(22, 163, 74));
                        painter.circle_stroke(sp, r + 1.5, Stroke::new(1.0_f32, Color32::from_rgb(34, 197, 94)));
                    }
                }

                if may_draw_plus && !docked_to_bldg {
                    let rad = conn.angle.to_radians();
                    let plus_sp = camera.world_to_screen(
                        rect,
                        conn.x + 28.0 * rad.cos(),
                        conn.y + 28.0 * rad.sin(),
                    );
                    let badge_hov = hover_pos.map_or(false, |hp| hp.distance(plus_sp) <= 13.0);
                    plus_buttons.push((piece.id.clone(), cidx, plus_sp, badge_hov));
                    if rect.expand(24.0).contains(plus_sp) {
                        let dir_scr = plus_sp - sp;
                        let dir_len = dir_scr.length().max(1.0);
                        let ux = dir_scr.x / dir_len;
                        let uy = dir_scr.y / dir_len;
                        if dir_len > 14.0 {
                            painter.line_segment(
                                [
                                    Pos2::new(sp.x + ux * 5.0, sp.y + uy * 5.0),
                                    Pos2::new(plus_sp.x - ux * 8.0, plus_sp.y - uy * 8.0),
                                ],
                                Stroke::new(1.3_f32, Color32::from_rgb(56, 189, 248)),
                            );
                        }
                        painter.circle_filled(
                            plus_sp,
                            if badge_hov { 11.5 } else { 10.0 },
                            Color32::from_rgba_unmultiplied(2, 132, 199, if badge_hov { 95 } else { 52 }),
                        );
                        painter.circle_filled(
                            plus_sp,
                            8.0,
                            if badge_hov {
                                Color32::from_rgb(2, 132, 199)
                            } else {
                                Color32::from_rgb(14, 165, 233)
                            },
                        );
                        painter.circle_stroke(plus_sp, 8.0, Stroke::new(1.5_f32, Color32::WHITE));
                        let arm = 3.8_f32;
                        painter.line_segment(
                            [Pos2::new(plus_sp.x - arm, plus_sp.y), Pos2::new(plus_sp.x + arm, plus_sp.y)],
                            Stroke::new(1.8_f32, Color32::WHITE),
                        );
                        painter.line_segment(
                            [Pos2::new(plus_sp.x, plus_sp.y - arm), Pos2::new(plus_sp.x, plus_sp.y + arm)],
                            Stroke::new(1.8_f32, Color32::WHITE),
                        );
                        if badge_hov {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                    }
                }
            }
        }

        // Building door / trackside dock connectors (green = free, blue = docked same height, yellow = height mismatch)
        for &(dx, dy, door_state) in &cache.visible_doors {
            let sp = camera.world_to_screen(rect, dx, dy);
            if !conn_clip.contains(sp) {
                continue;
            }
            let r = (4.5 * camera.zoom).clamp(3.0, 7.0);
            match door_state {
                1 => {
                    painter.circle_filled(sp, r, Color32::from_rgb(2, 132, 199));
                    painter.circle_stroke(sp, r + 1.5, Stroke::new(1.2_f32, Color32::from_rgb(56, 189, 248)));
                }
                2 => {
                    painter.circle_filled(sp, r * 1.15, Color32::from_rgb(234, 179, 8));
                    painter.circle_stroke(
                        sp,
                        r * 1.15 + 1.6,
                        Stroke::new(1.4_f32, Color32::from_rgb(250, 204, 21)),
                    );
                }
                _ => {
                    painter.circle_filled(sp, r, Color32::from_rgb(22, 163, 74));
                    painter.circle_stroke(sp, r + 1.5, Stroke::new(1.2_f32, Color32::from_rgb(34, 197, 94)));
                }
            }
        }
    }

    // 4.5. Interactive Flex Bend & Shorten Handles (3 cyan interior bend points + 2 amber endpoint handles c[1], c[0])
    let selected_flex_handles: Option<(String, [Pos2; 5])> = if selected.len() == 1 {
        selected.iter().next().and_then(|sid| {
            let p = cache
                .piece_id_to_idx
                .get(sid.as_str())
                .and_then(|&idx| layout.pieces.get(idx))
                .or_else(|| layout.pieces.iter().find(|pc| &pc.id == sid))?;
            if !is_layer_editable_with_protection(layout, p.layer_id.as_deref(), layer_protection) {
                return None;
            }
            if crate::catalog::is_flex_id(catalogs, &p.catalog_id) {
                let wh = crate::catalog::flex_bend_handles_world(p)?;
                let mut sh = [Pos2::ZERO; 5];
                for i in 0..5 {
                    sh[i] = camera.world_to_screen(rect, wh[i].x, wh[i].y);
                    let (r, fill, stroke_col) = if i < 3 {
                        (6.5_f32, Color32::from_rgb(56, 189, 248), Color32::WHITE) // 3 interior bend points
                    } else {
                        let cidx = if i == 3 { 1 } else { 0 };
                        let is_conn = p.connectors.get(cidx).is_some_and(|c| c.connected_to.is_some());
                        if is_conn {
                            (7.0_f32, Color32::from_rgb(16, 185, 129), Color32::WHITE) // snapped endpoint (still draggable)
                        } else {
                            (7.5_f32, Color32::from_rgb(245, 158, 11), Color32::WHITE) // free endpoint length/bend/snap handle
                        }
                    };
                    painter.circle_filled(sh[i], r, fill);
                    painter.circle_stroke(sh[i], r + 1.2, Stroke::new(1.8_f32, stroke_col));
                }
                Some((p.id.clone(), sh))
            } else {
                None
            }
        })
    } else {
        None
    };

    // 4.6. Selected 3D Model rotation handle (top edge stem matching TS getModelRotationHandleWorld)
    let selected_model_rot_handle: Option<(String, Pos2)> =
        if !sketcher.active && !terrain_brush.active && !measure_mode && selected.len() == 1 {
            selected.iter().next().and_then(|sid| {
                let m = layout.models.as_ref()?.iter().find(|m| &m.id == sid)?;
                if !is_layer_editable_with_protection(layout, m.layer_id.as_deref(), layer_protection) {
                    return None;
                }
                if matches!(m.builtin_id.as_deref(), Some("mountain") | Some("ground-terrain")) {
                    return None;
                }
                let wh = crate::model_tools::get_model_rotation_handle_world(m);
                let cs = camera.world_to_screen(rect, m.transform.x, m.transform.y);
                let hs = camera.world_to_screen(rect, wh.x, wh.y);
                painter.line_segment(
                    [cs, hs],
                    Stroke::new(1.4_f32, Color32::from_rgb(56, 189, 248)),
                );
                painter.circle_filled(hs, 7.0, Color32::from_rgb(245, 158, 11));
                painter.circle_stroke(hs, 8.2, Stroke::new(1.6_f32, Color32::WHITE));
                Some((m.id.clone(), hs))
            })
        } else {
            None
        };

    // Terrain Brush cursor reticle & live ridge/ground stroke preview (matching renderer-overlays.ts)
    if terrain_brush.active {
        if response.hovered() {
            if let Some(sp) = hover_pos.filter(|p| rect.contains(*p)) {
                terrain_brush.cursor_world = Some(camera.screen_to_world(rect, sp));
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        let stroke_col = match terrain_brush.mode {
            TerrainBrushMode::Paint => Color32::from_rgb(34, 197, 94),
            TerrainBrushMode::Extend => Color32::from_rgb(14, 165, 233),
            TerrainBrushMode::Stamp => Color32::from_rgb(245, 158, 11),
            TerrainBrushMode::Erase => Color32::from_rgb(239, 68, 68),
            TerrainBrushMode::Ground => match terrain_brush.ground_type.as_str() {
                "water" => Color32::from_rgb(14, 165, 233),
                "sand" => Color32::from_rgb(214, 185, 124),
                "dirt" => Color32::from_rgb(133, 96, 62),
                _ => Color32::from_rgb(101, 163, 13),
            },
        };
        let r_scr = ((terrain_brush.radius as f32) * camera.zoom).max(8.0);
        if terrain_brush.stroke_points.len() >= 2 {
            let spts: Vec<Pos2> = terrain_brush
                .stroke_points
                .iter()
                .map(|p| camera.world_to_screen(rect, p.x, p.y))
                .collect();
            let base_band_col = if terrain_brush.mode == TerrainBrushMode::Ground {
                Color32::from_rgba_unmultiplied(stroke_col.r(), stroke_col.g(), stroke_col.b(), 110)
            } else {
                Color32::from_rgba_unmultiplied(68, 64, 60, 115)
            };
            painter.add(egui::Shape::line(spts.clone(), Stroke::new(r_scr * 2.0, base_band_col)));
            painter.add(egui::Shape::line(
                spts.clone(),
                Stroke::new(r_scr * 0.7, Color32::from_rgba_unmultiplied(248, 250, 252, 150)),
            ));
            for sp in spts {
                painter.circle_filled(sp, 4.5, stroke_col);
            }
        }
        if let Some(cw) = terrain_brush.cursor_world {
            let sp = camera.world_to_screen(rect, cw.x, cw.y);
            painter.circle_filled(
                sp,
                r_scr,
                Color32::from_rgba_unmultiplied(stroke_col.r(), stroke_col.g(), stroke_col.b(), 36),
            );
            painter.circle_stroke(sp, r_scr, Stroke::new(2.0_f32, stroke_col));
            if terrain_brush.mode != TerrainBrushMode::Erase {
                painter.circle_stroke(
                    sp,
                    r_scr * 0.35,
                    Stroke::new(1.3_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 140)),
                );
            }
        }
    }

    interaction::handle_canvas_interaction(
        ui,
        rect,
        &response,
        &painter,
        hover_pos,
        layout,
        cache,
        catalogs,
        camera,
        selected,
        dragging_active,
        drag_last_world,
        dragging_flex_handle,
        marquee_start,
        measure_points,
        snapping_enabled,
        layer_protection,
        measure_mode,
        terrain_brush,
        is_de,
        next_piece_seq,
        active_board_id,
        dragging_board,
        board_snap_guides,
        sketcher,
        dragging_model_rot,
        ramp_overlay,
        &active_layer_id,
        &board_header_rects,
        &plus_buttons,
        selected_flex_handles,
        selected_model_rot_handle,
        &mut interaction,
    );

    overlays_and_menu::draw_overlays_and_context_menu(
        ui,
        rect,
        &response,
        &painter,
        layout,
        cache,
        &active_layer_id,
        catalogs,
        camera,
        selected,
        measure_points,
        dragging_catalog,
        dark_mode,
        show_rulers,
        snapping_enabled,
        measure_mode,
        rotation_step,
        &mut interaction,
    );

    interaction
}

mod interaction;
mod overlays_and_menu;
mod cache;
#[allow(unused_imports)]
use cache::*;
#[allow(unused_imports)]
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
mod snapping;
#[allow(unused_imports)]
use snapping::*;
#[allow(unused_imports)]
pub use snapping::{snap_selected_pieces};
#[cfg(test)]
mod tests;
